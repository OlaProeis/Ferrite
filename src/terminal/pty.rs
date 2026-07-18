//! Pseudo-terminal (PTY) handling for terminal emulation.
//!
//! This module provides cross-platform PTY management using the `portable-pty` crate.
//! It handles spawning shell processes and communicating with them.

use portable_pty::{native_pty_system, CommandBuilder, PtyPair, PtySize};
use std::io::{Read, Write};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;

use serde::{Deserialize, Serialize};

/// Which Windows shell binary we are launching (for UTF-8 startup args).
#[cfg(windows)]
enum WindowsLaunchShell {
    PowerShell,
    Cmd,
    Wsl,
}

/// Configure the child console for UTF-8 so PSReadLine/cmd echo CJK paste/input
/// correctly instead of `?` while the shell still receives valid UTF-8 bytes.
#[cfg(windows)]
fn apply_windows_utf8_init(cmd: &mut CommandBuilder, shell: WindowsLaunchShell) {
    match shell {
        WindowsLaunchShell::PowerShell => {
            cmd.args([
                "-NoLogo",
                "-NoExit",
                "-Command",
                "[Console]::InputEncoding = [Console]::OutputEncoding = (New-Object System.Text.UTF8Encoding $false); \
                 $OutputEncoding = [Console]::OutputEncoding; chcp 65001 | Out-Null",
            ]);
        }
        WindowsLaunchShell::Cmd => {
            cmd.args(["/K", "chcp 65001>nul"]);
        }
        WindowsLaunchShell::Wsl => {
            // WSL defaults to UTF-8; leave locale to the distro profile.
        }
    }
}

#[cfg(windows)]
fn is_windows_powershell_available() -> bool {
    std::path::Path::new("C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe").exists()
}

/// Shell type for terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShellType {
    /// PowerShell (Windows)
    PowerShell,
    /// Command Prompt (Windows)
    Cmd,
    /// Windows Subsystem for Linux
    Wsl,
    /// Default system shell (Unix)
    Default,
}

/// PTY wrapper for terminal communication.
pub struct TerminalPty {
    /// Writer to send data to the PTY
    writer: Box<dyn Write + Send>,
    /// Receiver for data from the PTY (read in background thread)
    reader_rx: Receiver<Vec<u8>>,
    /// Channel to signal the reader thread to stop
    _stop_tx: Sender<()>,
    /// PTY pair (kept for resize operations)
    pty_pair: PtyPair,
    /// Whether the child process is still running
    child_running: bool,
    /// Child process handle
    child: Box<dyn portable_pty::Child + Send>,
}

impl TerminalPty {
    /// Create a new PTY with the given size, shell type, and optional working directory.
    pub fn new(
        cols: u16,
        rows: u16,
        shell_type: ShellType,
        working_dir: Option<std::path::PathBuf>,
    ) -> Result<Self, String> {
        // Get the native PTY system
        let pty_system = native_pty_system();

        // Create PTY pair with specified size
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("Failed to open PTY: {}", e))?;

        // Build the shell command based on platform and type
        let cmd = Self::build_shell_command(shell_type, working_dir);

        // Spawn the shell process
        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("Failed to spawn shell: {}", e))?;

        // Get writer for the master side
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| format!("Failed to get PTY writer: {}", e))?;

        // Get reader for the master side
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| format!("Failed to get PTY reader: {}", e))?;

        // Create channels for communication
        let (data_tx, data_rx) = mpsc::channel::<Vec<u8>>();
        let (stop_tx, stop_rx) = mpsc::channel::<()>();

        // Spawn background thread to read from PTY
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                // Check if we should stop
                match stop_rx.try_recv() {
                    Ok(_) | Err(TryRecvError::Disconnected) => break,
                    Err(TryRecvError::Empty) => {}
                }

                // Read from PTY (non-blocking would be better, but portable-pty doesn't support it well)
                match reader.read(&mut buf) {
                    Ok(0) => {
                        // EOF - process has exited
                        break;
                    }
                    Ok(n) => {
                        if data_tx.send(buf[..n].to_vec()).is_err() {
                            // Receiver dropped
                            break;
                        }
                    }
                    Err(e) => {
                        log::debug!("PTY read error: {}", e);
                        break;
                    }
                }
            }
            log::debug!("PTY reader thread exiting");
        });

        Ok(Self {
            writer,
            reader_rx: data_rx,
            _stop_tx: stop_tx,
            pty_pair: pair,
            child_running: true,
            child,
        })
    }

    /// Get the process ID of the child process.
    pub fn pid(&self) -> u32 {
        self.child.process_id().unwrap_or(0)
    }

    /// Build the shell command for the specified shell type.
    fn build_shell_command(
        shell_type: ShellType,
        working_dir: Option<std::path::PathBuf>,
    ) -> CommandBuilder {
        #[cfg(windows)]
        let mut cmd = Self::build_windows_shell_command(shell_type);

        #[cfg(not(windows))]
        let mut cmd = Self::build_unix_shell_command();

        // Set working directory: use provided path, fall back to current dir, then home
        if let Some(dir) = working_dir {
            cmd.cwd(dir);
        } else if let Ok(cwd) = std::env::current_dir() {
            cmd.cwd(cwd);
        } else if let Some(home) = dirs::home_dir() {
            cmd.cwd(home);
        }

        // Set TERM environment variable for proper terminal behavior
        cmd.env("TERM", "xterm-256color");

        // Set COLORTERM for true color support
        cmd.env("COLORTERM", "truecolor");

        cmd
    }

    #[cfg(windows)]
    fn build_windows_shell_command(shell_type: ShellType) -> CommandBuilder {
        let (mut cmd, win_shell) = match shell_type {
            ShellType::PowerShell => (
                CommandBuilder::new("powershell.exe"),
                WindowsLaunchShell::PowerShell,
            ),
            ShellType::Cmd => {
                let shell = std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string());
                (CommandBuilder::new(shell), WindowsLaunchShell::Cmd)
            }
            ShellType::Wsl => (CommandBuilder::new("wsl.exe"), WindowsLaunchShell::Wsl),
            ShellType::Default => {
                if is_windows_powershell_available() {
                    (
                        CommandBuilder::new("powershell.exe"),
                        WindowsLaunchShell::PowerShell,
                    )
                } else {
                    let shell = std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string());
                    (CommandBuilder::new(shell), WindowsLaunchShell::Cmd)
                }
            }
        };
        apply_windows_utf8_init(&mut cmd, win_shell);
        cmd
    }

    #[cfg(not(windows))]
    fn build_unix_shell_command() -> CommandBuilder {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        let mut cmd = CommandBuilder::new(&shell);

        // Add -l flag for login shell behavior (loads profile)
        if shell.contains("bash") || shell.contains("zsh") {
            cmd.arg("-l");
        }

        cmd
    }

    /// Write data to the PTY (send to shell).
    pub fn write(&mut self, data: &[u8]) -> Result<(), String> {
        self.writer
            .write_all(data)
            .map_err(|e| format!("Failed to write to PTY: {}", e))?;
        self.writer
            .flush()
            .map_err(|e| format!("Failed to flush PTY: {}", e))?;
        Ok(())
    }

    /// Read available data from the PTY (non-blocking).
    /// Returns Ok(Some(data)) if data is available, Ok(None) if no data.
    pub fn read(&mut self) -> Result<Option<Vec<u8>>, String> {
        match self.reader_rx.try_recv() {
            Ok(data) => Ok(Some(data)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => {
                self.child_running = false;
                Err("PTY reader disconnected".to_string())
            }
        }
    }

    /// Check if the child process is still running.
    pub fn is_running(&self) -> bool {
        self.child_running
    }

    /// Resize the PTY.
    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<(), String> {
        self.pty_pair
            .master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("Failed to resize PTY: {}", e))
    }
}

impl Drop for TerminalPty {
    fn drop(&mut self) {
        // Stop signal is sent automatically when _stop_tx is dropped
        // Attempt graceful child process cleanup
        if self.child_running {
            match self.child.kill() {
                Ok(()) => log::debug!("Terminal child process killed on drop"),
                Err(e) => log::debug!("Terminal child process may have already exited: {}", e),
            }
        }
        // Wait briefly for child to clean up (non-blocking check)
        match self.child.try_wait() {
            Ok(Some(status)) => log::debug!("Terminal child exited with status: {:?}", status),
            Ok(None) => {
                log::debug!("Terminal child still running after kill, will be cleaned up by OS")
            }
            Err(e) => log::debug!("Could not check terminal child status: {}", e),
        }
        log::debug!("TerminalPty dropped, resources released");
    }
}
