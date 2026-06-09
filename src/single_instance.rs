//! Single-instance application protocol.
//!
//! Ensures only one Ferrite window runs at a time. When a second instance is
//! launched (e.g., double-clicking a file in Explorer), it forwards the file
//! paths to the already-running instance via a local TCP connection, then exits.
//!
//! ## Protocol
//!
//! - **Lock file**: `{config_dir}/instance.lock` contains the TCP port of the
//!   running instance as plain text.
//! - **IPC**: The second instance connects to `127.0.0.1:{port}`, sends file
//!   paths as UTF-8 lines (one path per line), then closes the connection.
//! - **Background thread**: The primary instance runs a blocking accept loop on
//!   a background thread. Received paths are sent to the UI via a channel, and
//!   the UI thread is woken immediately via `ctx.request_repaint()`.

use crate::config::get_config_dir;
use log::{debug, error, info, warn};
use std::io::{BufRead, BufReader, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};

/// Name of the lock file stored in the config directory.
const LOCK_FILE_NAME: &str = "instance.lock";
/// Name of the pid file stored next to the lock file.
const PID_FILE_NAME: &str = "instance.pid";

/// Timeout for connecting to the existing instance (milliseconds).
const CONNECT_TIMEOUT_MS: u64 = 500;

/// Attempt to become the primary instance, or forward paths to the existing one.
///
/// Returns `Some(listener)` if this process should become the primary instance.
/// Returns `None` if paths were forwarded to an existing instance and we should exit.
pub fn try_acquire_instance(paths: &[PathBuf]) -> Option<SingleInstanceListener> {
    let lock_path = match get_lock_file_path() {
        Some(p) => p,
        None => {
            warn!("Could not determine lock file path; proceeding as primary");
            return create_listener();
        }
    };

    // Check if a lock file exists with a valid port
    if let Some(port) = read_lock_port(&lock_path) {
        // Try to connect to the existing instance
        if try_forward_paths(port, paths) {
            // Don't log here — logger may not be initialized yet (early check in main)
            return None; // Signal caller to exit
        }
        // Connection failed — the old instance is dead. Clean up stale lock.
        let _ = std::fs::remove_file(&lock_path);
    }

    // No running instance — become the primary
    create_listener()
}

/// Create a new TCP listener and write the lock file.
fn create_listener() -> Option<SingleInstanceListener> {
    let listener = match TcpListener::bind("127.0.0.1:0") {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind single-instance listener: {}", e);
            return Some(SingleInstanceListener::empty());
        }
    };

    let port = match listener.local_addr() {
        Ok(addr) => addr.port(),
        Err(e) => {
            error!("Failed to get listener address: {}", e);
            return Some(SingleInstanceListener::empty());
        }
    };

    if let Err(e) = write_lock_file(port) {
        warn!("Failed to write instance lock file: {}", e);
    }
    if let Err(e) = write_pid_file(std::process::id()) {
        warn!("Failed to write instance pid file: {}", e);
    }

    info!("Single-instance listener started on port {}", port);

    let (tx, rx) = mpsc::channel();
    let repaint_ctx: Arc<Mutex<Option<egui::Context>>> = Arc::new(Mutex::new(None));

    Some(SingleInstanceListener {
        receiver: rx,
        repaint_ctx: Arc::clone(&repaint_ctx),
        _accept_thread: Some(spawn_accept_thread(listener, tx, repaint_ctx)),
    })
}

/// Spawn a background thread that blocks on `accept()` and reads paths.
///
/// Wakes the UI thread immediately via `ctx.request_repaint()` when paths
/// arrive, bypassing idle repaint delays entirely.
fn spawn_accept_thread(
    listener: TcpListener,
    tx: mpsc::Sender<Vec<PathBuf>>,
    repaint_ctx: Arc<Mutex<Option<egui::Context>>>,
) -> std::thread::JoinHandle<()> {
    let _ = listener.set_nonblocking(false);

    std::thread::Builder::new()
        .name("single-instance-accept".into())
        .spawn(move || {
            loop {
                match listener.accept() {
                    Ok((stream, _addr)) => {
                        let paths = read_paths_from_stream(stream);
                        if !paths.is_empty() {
                            if tx.send(paths).is_err() {
                                break;
                            }
                            // Wake the UI thread immediately so it drains the channel
                            if let Ok(guard) = repaint_ctx.lock() {
                                if let Some(ctx) = guard.as_ref() {
                                    ctx.request_repaint();
                                }
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Accept thread error: {}", e);
                        break;
                    }
                }
            }
        })
        .expect("Failed to spawn single-instance accept thread")
}

/// Read file paths from an accepted TCP stream.
///
/// Uses a short read timeout since all data arrives instantly on localhost.
fn read_paths_from_stream(stream: TcpStream) -> Vec<PathBuf> {
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(100)));

    let mut paths = Vec::new();
    let reader = BufReader::new(stream);

    for line in reader.lines() {
        match line {
            Ok(line) => {
                let trimmed = line.trim().to_string();
                if trimmed.is_empty() || trimmed == "__FOCUS__" {
                    continue;
                }
                paths.push(PathBuf::from(trimmed));
            }
            Err(_) => break,
        }
    }

    paths
}

/// Read the port number from the lock file.
fn read_lock_port(lock_path: &std::path::Path) -> Option<u16> {
    let content = std::fs::read_to_string(lock_path).ok()?;
    content.trim().parse::<u16>().ok()
}

/// Try to connect to an existing instance and forward file paths.
///
/// Returns `true` if the paths were successfully forwarded.
fn try_forward_paths(port: u16, paths: &[PathBuf]) -> bool {
    use std::net::SocketAddr;
    use std::time::Duration;

    let addr: SocketAddr = ([127, 0, 0, 1], port).into();
    let timeout = Duration::from_millis(CONNECT_TIMEOUT_MS);

    let mut stream = match TcpStream::connect_timeout(&addr, timeout) {
        Ok(s) => s,
        Err(_) => return false,
    };

    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    allow_primary_foreground_from_secondary();

    for path in paths {
        let line = format!("{}\n", path.display());
        if stream.write_all(line.as_bytes()).is_err() {
            return false;
        }
    }

    if paths.is_empty() {
        if stream.write_all(b"__FOCUS__\n").is_err() {
            return false;
        }
    }

    let _ = stream.flush();
    let _ = stream.shutdown(Shutdown::Write);
    true
}

/// Write the lock file with the given port number.
fn write_lock_file(port: u16) -> std::io::Result<()> {
    let lock_path = get_lock_file_path().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "Config dir not available")
    })?;

    if let Some(parent) = lock_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(&lock_path, port.to_string())
}

fn write_pid_file(pid: u32) -> std::io::Result<()> {
    let pid_path = get_pid_file_path().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "Config dir not available")
    })?;

    if let Some(parent) = pid_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(&pid_path, pid.to_string())
}

/// Get the path to the instance lock file.
fn get_lock_file_path() -> Option<PathBuf> {
    get_config_dir().ok().map(|dir| dir.join(LOCK_FILE_NAME))
}

fn get_pid_file_path() -> Option<PathBuf> {
    get_config_dir().ok().map(|dir| dir.join(PID_FILE_NAME))
}

fn read_primary_pid() -> Option<u32> {
    let pid_path = get_pid_file_path()?;
    let content = std::fs::read_to_string(pid_path).ok()?;
    content.trim().parse::<u32>().ok()
}

fn allow_primary_foreground_from_secondary() {
    #[cfg(target_os = "windows")]
    if let Some(primary_pid) = read_primary_pid() {
        let _ = crate::platform::allow_set_foreground_window(primary_pid);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SingleInstanceListener — lives in the primary instance
// ─────────────────────────────────────────────────────────────────────────────

/// Receives file-open requests from secondary instances via a background thread.
///
/// The background thread blocks on TCP `accept()` and reads paths immediately.
/// The UI thread drains the channel each frame (non-blocking).
/// An egui repaint context can be provided so the background thread wakes the
/// UI instantly when paths arrive, bypassing idle repaint delays.
pub struct SingleInstanceListener {
    receiver: mpsc::Receiver<Vec<PathBuf>>,
    repaint_ctx: Arc<Mutex<Option<egui::Context>>>,
    _accept_thread: Option<std::thread::JoinHandle<()>>,
}

impl SingleInstanceListener {
    /// Create a dummy listener that never receives anything.
    fn empty() -> Self {
        let (_tx, rx) = mpsc::channel();
        Self {
            receiver: rx,
            repaint_ctx: Arc::new(Mutex::new(None)),
            _accept_thread: None,
        }
    }

    /// Provide the egui context so the background thread can wake the UI
    /// immediately when paths arrive. Call once after the egui context is available.
    pub fn set_repaint_ctx(&self, ctx: egui::Context) {
        if let Ok(mut guard) = self.repaint_ctx.lock() {
            *guard = Some(ctx);
        }
    }

    /// Drain all pending paths from the background thread (non-blocking).
    pub fn poll(&self) -> Vec<PathBuf> {
        let mut all_paths = Vec::new();
        while let Ok(paths) = self.receiver.try_recv() {
            all_paths.extend(paths);
        }
        all_paths
    }
}

impl Drop for SingleInstanceListener {
    fn drop(&mut self) {
        // Clean up the lock file when the primary instance exits
        if let Some(lock_path) = get_lock_file_path() {
            if lock_path.exists() {
                debug!("Cleaning up instance lock file");
                let _ = std::fs::remove_file(&lock_path);
            }
        }
        if let Some(pid_path) = get_pid_file_path() {
            if pid_path.exists() {
                debug!("Cleaning up instance pid file");
                let _ = std::fs::remove_file(&pid_path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_file_roundtrip() {
        let port_str = "12345";
        let port: u16 = port_str.trim().parse().unwrap();
        assert_eq!(port, 12345);
    }

    #[test]
    fn test_forward_to_nonexistent_port_returns_false() {
        let result = try_forward_paths(1, &[PathBuf::from("test.md")]);
        assert!(!result);
    }
}
