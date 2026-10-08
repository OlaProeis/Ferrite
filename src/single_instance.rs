//! Single-instance application protocol.
//!
//! Ensures only one Ferrite process runs at a time. When a second instance is
//! launched (e.g., double-clicking a file in Explorer), it forwards the file
//! paths to the already-running process via a local TCP connection, then exits.
//! The primary process opens received paths in the last-focused document window.
//!
//! ## Protocol
//!
//! - **Lock file**: `{config_dir}/instance.lock` is exclusively locked with
//!   [`std::fs::File::try_lock`]. The primary writes its TCP port into that
//!   file and holds the handle for the process lifetime.
//! - **Port sidecar**: `{config_dir}/instance.port` mirrors the port so
//!   secondaries can read it on Windows, where an exclusive lock is mandatory
//!   and blocks reads of the locked file.
//! - **IPC**: The second instance connects to `127.0.0.1:{port}`, sends file
//!   paths as UTF-8 lines (one path per line), then waits for `OK\n`.
//! - **Background thread**: The primary instance runs a blocking accept loop on
//!   a background thread. Received paths are sent to the UI via a channel, and
//!   the UI thread is woken immediately via `ctx.request_repaint()`.

use crate::config::get_config_dir;
use crate::path_utils::normalize_path;
use log::{debug, error, info, warn};
use std::fs::{File, OpenOptions, TryLockError};
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

/// Name of the lock file stored in the config directory.
const LOCK_FILE_NAME: &str = "instance.lock";
/// Readable port mirror (Windows cannot read an exclusively locked file).
const PORT_FILE_NAME: &str = "instance.port";
/// Name of the pid file stored next to the lock file.
const PID_FILE_NAME: &str = "instance.pid";

/// Timeout for connecting to the existing instance (milliseconds).
const CONNECT_TIMEOUT_MS: u64 = 500;
/// How long a secondary waits for the primary to publish a port / accept.
const STARTUP_RETRY_BUDGET: Duration = Duration::from_secs(2);
const STARTUP_RETRY_SLEEP: Duration = Duration::from_millis(50);
/// How long a secondary waits for the `OK` handshake after sending paths.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(1);

/// Outcome of trying to become the single-instance primary.
pub enum InstanceAcquire {
    /// This process holds the lock and should run the UI.
    Primary(SingleInstanceListener),
    /// Paths were forwarded and acknowledged; this process should exit.
    Forwarded,
    /// Setting or `--new-instance`: no lock, no forwarding.
    Independent,
}

/// Payload delivered from the single-instance accept thread to the UI.
#[derive(Debug, Clone, Default)]
pub struct InstanceIncoming {
    pub paths: Vec<PathBuf>,
    /// True when the secondary instance sent `__FOCUS__` with no file paths.
    pub focus_only: bool,
}

/// Attempt to become the primary instance, or forward paths to the existing one.
///
/// When `skip_single_instance` is true (setting or `--new-instance`), no lock or
/// pid files are written and the caller should run independently.
pub fn try_acquire_instance(paths: &[PathBuf], skip_single_instance: bool) -> InstanceAcquire {
    if skip_single_instance {
        debug!("single-instance: independent (setting or --new-instance); no lock");
        return InstanceAcquire::Independent;
    }

    let Some(lock_path) = get_lock_file_path() else {
        warn!("Could not determine lock file path; proceeding as primary");
        return InstanceAcquire::Primary(create_listener(None));
    };

    if let Some(parent) = lock_path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            warn!("Failed to create config dir for instance lock: {e}");
            return InstanceAcquire::Primary(create_listener(None));
        }
    }

    let lock_file = match OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&lock_path)
    {
        Ok(f) => f,
        Err(e) => {
            warn!("Failed to open instance lock file: {e}; proceeding as primary");
            return InstanceAcquire::Primary(create_listener(None));
        }
    };

    match lock_file.try_lock() {
        Ok(()) => {
            debug!("single-instance: acquired exclusive lock; becoming primary");
            InstanceAcquire::Primary(create_listener(Some(lock_file)))
        }
        Err(TryLockError::WouldBlock) => {
            debug!("single-instance: lock held; attempting to forward paths");
            match try_forward_with_retry(paths) {
                ForwardResult::Success => {
                    debug!("single-instance: forwarded paths; handshake OK");
                    InstanceAcquire::Forwarded
                }
                ForwardResult::NoInstance => {
                    debug!(
                        "single-instance: no live instance (missing port or OK); becoming primary"
                    );
                    become_primary_after_stale(paths)
                }
            }
        }
        Err(TryLockError::Error(e)) => {
            warn!("Failed to lock instance file: {e}; proceeding as primary");
            InstanceAcquire::Primary(create_listener(None))
        }
    }
}

fn become_primary_after_stale(paths: &[PathBuf]) -> InstanceAcquire {
    remove_stale_instance_files();

    let Some(lock_path) = get_lock_file_path() else {
        return InstanceAcquire::Primary(create_listener(None));
    };

    let lock_file = match OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&lock_path)
    {
        Ok(f) => f,
        Err(_) => return InstanceAcquire::Primary(create_listener(None)),
    };

    match lock_file.try_lock() {
        Ok(()) => {
            debug!("single-instance: acquired lock after stale cleanup; becoming primary");
            InstanceAcquire::Primary(create_listener(Some(lock_file)))
        }
        Err(TryLockError::WouldBlock) => {
            debug!("single-instance: lock re-acquired by another process; retrying forward");
            match try_forward_with_retry(paths) {
                ForwardResult::Success => InstanceAcquire::Forwarded,
                ForwardResult::NoInstance => InstanceAcquire::Primary(create_listener(None)),
            }
        }
        Err(TryLockError::Error(_)) => InstanceAcquire::Primary(create_listener(None)),
    }
}

/// Create a TCP listener and publish the port while holding `lock_file`.
fn create_listener(lock_file: Option<File>) -> SingleInstanceListener {
    let listener = match TcpListener::bind("127.0.0.1:0") {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind single-instance listener: {}", e);
            return SingleInstanceListener::empty();
        }
    };

    let port = match listener.local_addr() {
        Ok(addr) => addr.port(),
        Err(e) => {
            error!("Failed to get listener address: {}", e);
            return SingleInstanceListener::empty();
        }
    };

    if let Some(mut lock_file) = lock_file {
        if let Err(e) = write_port_to_lock(&mut lock_file, port) {
            warn!("Failed to write port into instance lock file: {}", e);
        }
        if let Err(e) = write_port_sidecar(port) {
            warn!("Failed to write instance port sidecar: {}", e);
        }
        if let Err(e) = write_pid_file(std::process::id()) {
            warn!("Failed to write instance pid file: {}", e);
        }

        info!("Single-instance listener started on port {}", port);

        let (tx, rx) = mpsc::channel();
        let repaint_ctx: Arc<Mutex<Option<egui::Context>>> = Arc::new(Mutex::new(None));

        return SingleInstanceListener {
            receiver: rx,
            repaint_ctx: Arc::clone(&repaint_ctx),
            _accept_thread: Some(spawn_accept_thread(listener, tx, repaint_ctx)),
            _lock_file: Some(lock_file),
        };
    }

    info!(
        "Single-instance listener started on port {} (no lock)",
        port
    );

    let (tx, rx) = mpsc::channel();
    let repaint_ctx: Arc<Mutex<Option<egui::Context>>> = Arc::new(Mutex::new(None));

    SingleInstanceListener {
        receiver: rx,
        repaint_ctx: Arc::clone(&repaint_ctx),
        _accept_thread: Some(spawn_accept_thread(listener, tx, repaint_ctx)),
        _lock_file: None,
    }
}

/// Spawn a background thread that blocks on `accept()` and reads paths.
///
/// Wakes the UI thread immediately via `ctx.request_repaint()` when paths
/// arrive, bypassing idle repaint delays entirely.
fn spawn_accept_thread(
    listener: TcpListener,
    tx: mpsc::Sender<InstanceIncoming>,
    repaint_ctx: Arc<Mutex<Option<egui::Context>>>,
) -> std::thread::JoinHandle<()> {
    let _ = listener.set_nonblocking(false);

    std::thread::Builder::new()
        .name("single-instance-accept".into())
        .spawn(move || {
            loop {
                match listener.accept() {
                    Ok((stream, _addr)) => {
                        let message = read_message_from_stream(stream);
                        if message.focus_only || !message.paths.is_empty() {
                            if tx.send(message).is_err() {
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

/// Read file paths (and optional focus-only signal) from an accepted TCP stream.
///
/// After the message is read the primary replies `OK\n` so the secondary can
/// confirm delivery. Uses a short read timeout since all data arrives instantly
/// on localhost.
fn read_message_from_stream(stream: TcpStream) -> InstanceIncoming {
    let _ = stream.set_read_timeout(Some(Duration::from_millis(100)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));

    let mut reply = stream.try_clone().ok();
    let message = read_paths_from_stream(stream);
    if let Some(ref mut reply) = reply {
        let _ = reply.write_all(b"OK\n");
        let _ = reply.flush();
    }
    message
}

fn read_paths_from_stream(stream: TcpStream) -> InstanceIncoming {
    let mut paths = Vec::new();
    let mut focus_only = false;
    let reader = BufReader::new(stream);

    for line in reader.lines() {
        match line {
            Ok(line) => {
                let trimmed = line.trim().to_string();
                if trimmed.is_empty() {
                    continue;
                }
                if trimmed == "__FOCUS__" {
                    focus_only = true;
                    continue;
                }
                paths.push(PathBuf::from(trimmed));
            }
            Err(_) => break,
        }
    }

    InstanceIncoming { paths, focus_only }
}

fn read_published_port() -> Option<u16> {
    if let Some(path) = get_lock_file_path() {
        if let Some(port) = read_lock_port(&path) {
            return Some(port);
        }
    }
    if let Some(path) = get_port_file_path() {
        return read_lock_port(&path);
    }
    None
}

/// Read the port number from a lock/port file.
fn read_lock_port(lock_path: &Path) -> Option<u16> {
    let content = std::fs::read_to_string(lock_path).ok()?;
    content.trim().parse::<u16>().ok()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ForwardResult {
    Success,
    NoInstance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ForwardAttempt {
    Ok,
    ConnectFailed,
    NoHandshake,
}

fn try_forward_with_retry(paths: &[PathBuf]) -> ForwardResult {
    let deadline = Instant::now() + STARTUP_RETRY_BUDGET;
    loop {
        match read_published_port() {
            Some(port) => match try_forward_paths(port, paths) {
                ForwardAttempt::Ok => return ForwardResult::Success,
                ForwardAttempt::NoHandshake => {
                    debug!("single-instance: connected but missing OK handshake");
                    return ForwardResult::NoInstance;
                }
                ForwardAttempt::ConnectFailed => {
                    debug!("single-instance: port published but connect failed; retrying");
                }
            },
            None => {
                debug!("single-instance: port not yet written; retrying");
            }
        }

        if Instant::now() >= deadline {
            return ForwardResult::NoInstance;
        }
        std::thread::sleep(STARTUP_RETRY_SLEEP);
    }
}

/// Try to connect to an existing instance and forward file paths.
///
/// Returns [`ForwardAttempt::Ok`] only when the primary replies `OK`.
fn try_forward_paths(port: u16, paths: &[PathBuf]) -> ForwardAttempt {
    use std::net::SocketAddr;

    let addr: SocketAddr = ([127, 0, 0, 1], port).into();
    let timeout = Duration::from_millis(CONNECT_TIMEOUT_MS);

    let mut stream = match TcpStream::connect_timeout(&addr, timeout) {
        Ok(s) => s,
        Err(_) => return ForwardAttempt::ConnectFailed,
    };

    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_read_timeout(Some(HANDSHAKE_TIMEOUT));
    allow_primary_foreground_from_secondary();

    let forwarded = canonicalize_forward_paths(paths);
    for path in &forwarded {
        let line = format!("{}\n", path.display());
        if stream.write_all(line.as_bytes()).is_err() {
            return ForwardAttempt::ConnectFailed;
        }
    }

    if forwarded.is_empty() {
        if stream.write_all(b"__FOCUS__\n").is_err() {
            return ForwardAttempt::ConnectFailed;
        }
    }

    if stream.flush().is_err() {
        return ForwardAttempt::ConnectFailed;
    }
    let _ = stream.shutdown(Shutdown::Write);

    if wait_for_ok(&stream) {
        ForwardAttempt::Ok
    } else {
        ForwardAttempt::NoHandshake
    }
}

fn wait_for_ok(stream: &TcpStream) -> bool {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    match reader.read_line(&mut line) {
        Ok(0) => false,
        Ok(_) => line.trim() == "OK",
        Err(_) => false,
    }
}

/// Resolve a path the secondary is about to forward against *this* process CWD.
fn canonicalize_forward_path(p: &Path) -> PathBuf {
    let resolved = std::fs::canonicalize(p)
        .or_else(|_| std::env::current_dir().map(|d| d.join(p)))
        .unwrap_or_else(|_| p.to_path_buf());
    normalize_path(resolved)
}

fn canonicalize_forward_paths(paths: &[PathBuf]) -> Vec<PathBuf> {
    paths.iter().map(|p| canonicalize_forward_path(p)).collect()
}

fn write_port_to_lock(file: &mut File, port: u16) -> std::io::Result<()> {
    file.set_len(0)?;
    file.seek(SeekFrom::Start(0))?;
    write!(file, "{port}")?;
    file.flush()?;
    Ok(())
}

fn write_port_sidecar(port: u16) -> std::io::Result<()> {
    let port_path = get_port_file_path().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "Config dir not available")
    })?;

    if let Some(parent) = port_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(&port_path, port.to_string())
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

fn get_port_file_path() -> Option<PathBuf> {
    get_config_dir().ok().map(|dir| dir.join(PORT_FILE_NAME))
}

fn get_pid_file_path() -> Option<PathBuf> {
    get_config_dir().ok().map(|dir| dir.join(PID_FILE_NAME))
}

fn remove_stale_instance_files() {
    if let Some(lock_path) = get_lock_file_path() {
        let _ = std::fs::remove_file(&lock_path);
    }
    if let Some(port_path) = get_port_file_path() {
        let _ = std::fs::remove_file(&port_path);
    }
    if let Some(pid_path) = get_pid_file_path() {
        let _ = std::fs::remove_file(&pid_path);
    }
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
    receiver: mpsc::Receiver<InstanceIncoming>,
    repaint_ctx: Arc<Mutex<Option<egui::Context>>>,
    _accept_thread: Option<std::thread::JoinHandle<()>>,
    /// Held so the OS exclusive lock lives for the process lifetime.
    _lock_file: Option<File>,
}

impl SingleInstanceListener {
    /// Create a dummy listener that never receives anything.
    fn empty() -> Self {
        let (_tx, rx) = mpsc::channel();
        Self {
            receiver: rx,
            repaint_ctx: Arc::new(Mutex::new(None)),
            _accept_thread: None,
            _lock_file: None,
        }
    }

    /// Provide the egui context so the background thread can wake the UI
    /// immediately when paths arrive. Call once after the egui context is available.
    pub fn set_repaint_ctx(&self, ctx: egui::Context) {
        if let Ok(mut guard) = self.repaint_ctx.lock() {
            *guard = Some(ctx);
        }
    }

    /// Drain all pending messages from the background thread (non-blocking).
    pub fn poll(&self) -> Vec<InstanceIncoming> {
        let mut messages = Vec::new();
        while let Ok(message) = self.receiver.try_recv() {
            messages.push(message);
        }
        messages
    }
}

impl Drop for SingleInstanceListener {
    fn drop(&mut self) {
        // Release the OS lock first so cleanup can unlink the files on Windows.
        self._lock_file = None;
        if let Some(lock_path) = get_lock_file_path() {
            if lock_path.exists() {
                debug!("Cleaning up instance lock file");
                let _ = std::fs::remove_file(&lock_path);
            }
        }
        if let Some(port_path) = get_port_file_path() {
            if port_path.exists() {
                debug!("Cleaning up instance port file");
                let _ = std::fs::remove_file(&port_path);
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
    fn test_forward_to_nonexistent_port_returns_connect_failed() {
        let result = try_forward_paths(1, &[PathBuf::from("test.md")]);
        assert_eq!(result, ForwardAttempt::ConnectFailed);
    }

    #[test]
    fn test_read_message_parses_paths_and_focus_only() {
        use std::io::Write;
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
        let port = listener.local_addr().unwrap().port();

        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept test connection");
            stream
                .write_all(b"/tmp/a.md\n__FOCUS__\n")
                .expect("write test payload");
        });

        let stream = TcpStream::connect(("127.0.0.1", port)).expect("connect to test listener");
        let message = read_message_from_stream(stream);
        server.join().expect("server thread panicked");

        assert_eq!(message.paths, vec![PathBuf::from("/tmp/a.md")]);
        assert!(message.focus_only);
    }

    #[test]
    fn test_try_lock_exclusive_blocks_second_handle() {
        let path = std::env::temp_dir().join(format!(
            "ferrite_instance_lock_test_{}_{}.lock",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_file(&path);

        let first = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .expect("open first lock handle");
        first.try_lock().expect("first handle should acquire lock");

        let second = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .expect("open second lock handle");
        match second.try_lock() {
            Err(TryLockError::WouldBlock) => {}
            other => panic!("second handle should fail while first is held, got {other:?}"),
        }

        drop(first);

        second
            .try_lock()
            .expect("second handle should acquire lock after first is dropped");

        drop(second);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_read_message_ok_handshake_roundtrip() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
        let port = listener.local_addr().unwrap().port();

        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept test connection");
            let message = read_message_from_stream(stream);
            assert_eq!(message.paths, vec![PathBuf::from("/tmp/handshake.md")]);
            assert!(!message.focus_only);
        });

        let mut client = TcpStream::connect(("127.0.0.1", port)).expect("connect to test listener");
        client
            .write_all(b"/tmp/handshake.md\n")
            .expect("write paths");
        client.flush().expect("flush paths");
        let _ = client.shutdown(Shutdown::Write);
        client
            .set_read_timeout(Some(HANDSHAKE_TIMEOUT))
            .expect("set handshake timeout");
        assert!(wait_for_ok(&client), "primary should reply OK");
        server.join().expect("server thread panicked");
    }

    #[test]
    fn test_canonicalize_forward_path_relative_and_verbatim() {
        let resolved = canonicalize_forward_path(Path::new("./x.md"));
        let cwd = std::env::current_dir().expect("cwd");
        assert!(
            resolved.ends_with("x.md"),
            "expected ./x.md to resolve to a path ending in x.md, got {resolved:?}"
        );
        assert!(
            resolved.is_absolute() || resolved.starts_with(&cwd),
            "expected ./x.md to resolve against CWD {cwd:?}, got {resolved:?}"
        );
        if std::fs::canonicalize("./x.md").is_err() {
            assert_eq!(resolved, normalize_path(cwd.join("./x.md")));
        }

        let verbatim = PathBuf::from(r"\\?\C:\ferrite-test\file.md");
        let stripped = normalize_path(verbatim);
        #[cfg(windows)]
        {
            assert_eq!(stripped, PathBuf::from(r"C:\ferrite-test\file.md"));
            assert!(!stripped.to_string_lossy().starts_with(r"\\?\"));
        }
        #[cfg(not(windows))]
        {
            assert_eq!(stripped, PathBuf::from(r"\\?\C:\ferrite-test\file.md"));
        }
    }
}
