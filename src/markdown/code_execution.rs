//! Execute fenced code from the rendered markdown preview (explicit user action only).
//!
//! Public surface:
//!
//! * [`CodeExecutionUi`] — settings snapshot used by the preview to gate the
//!   Run button and pass the working directory.
//! * [`spawn_run`] — spawn a background worker that executes a code snippet
//!   for the chosen language and streams output into a [`RunHandle`].
//! * [`RunHandle`] / [`RunState`] — shared state polled per frame by the
//!   inline output panel in [`crate::markdown::widgets::EditableCodeBlock`].
//! * [`run_snippet`] — synchronous helper retained for tests and any caller
//!   that wants the combined output as a single string.
//!
//! ANSI byte streams are parsed in the UI layer via
//! [`crate::markdown::ansi_render`] so the inline panel does not duplicate
//! terminal emulation.

use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui;
#[cfg(test)]
use crate::ui::phosphor_icons::{CHECK, X};

/// [`crate::markdown::MarkdownEditor`] stores the current snapshot at this id for
/// [`crate::markdown::widgets::EditableCodeBlock`].
pub(crate) fn code_execution_ctx_id() -> egui::Id {
    egui::Id::new("ferrite_markdown_code_execution_ctx")
}

fn code_execution_toasts_id() -> egui::Id {
    egui::Id::new("ferrite_code_exec_toasts")
}

/// Settings snapshot for gating and running fenced code blocks.
#[derive(Clone, Debug)]
pub struct CodeExecutionUi {
    /// Master toggle (`Settings.enable_code_execution`).
    pub enable: bool,
    /// Persists as [`crate::config::Settings::code_execution_consent_acknowledged`].
    pub consent_acknowledged: bool,
    pub allow_shell: bool,
    pub allow_python: bool,
    pub timeout_secs: u32,
    /// When true, render output inline below the block; otherwise fall back
    /// to the legacy toast-only completion notification.
    pub show_inline_output: bool,
    /// Working directory for the subprocess (typically the current file's folder).
    pub working_directory: Option<PathBuf>,
}

impl CodeExecutionUi {
    pub fn disabled() -> Self {
        Self {
            enable: false,
            consent_acknowledged: false,
            allow_shell: false,
            allow_python: false,
            timeout_secs: 30,
            show_inline_output: true,
            working_directory: None,
        }
    }

    pub fn from_settings(settings: &crate::config::Settings) -> Self {
        Self {
            enable: settings.enable_code_execution,
            consent_acknowledged: settings.code_execution_consent_acknowledged,
            allow_shell: settings.allow_shell,
            allow_python: settings.allow_python,
            timeout_secs: settings.code_execution_timeout_secs,
            show_inline_output: settings.code_execution_show_inline_output,
            working_directory: None,
        }
    }

    /// Full snapshot for preview / split view (file directory as cwd when available).
    pub fn from_settings_with_workdir(
        settings: &crate::config::Settings,
        working_directory: Option<PathBuf>,
    ) -> Self {
        let mut s = Self::from_settings(settings);
        s.working_directory = working_directory;
        s
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunnableKind {
    Shell,
    Python,
}

pub fn classify_language(lang: &str) -> Option<RunnableKind> {
    match lang.trim().to_ascii_lowercase().as_str() {
        "bash" | "sh" | "shell" | "zsh" | "pwsh" | "powershell" | "ps1" | "cmd" | "bat"
        | "batch" => Some(RunnableKind::Shell),
        "python" | "python3" | "py" => Some(RunnableKind::Python),
        _ => None,
    }
}

pub fn run_button_visible(ctx: &CodeExecutionUi, language: &str) -> bool {
    let allowed_lang = match classify_language(language) {
        Some(RunnableKind::Shell) => ctx.allow_shell,
        Some(RunnableKind::Python) => ctx.allow_python,
        None => return false,
    };
    if !allowed_lang {
        return false;
    }
    if ctx.enable {
        return true;
    }
    !ctx.consent_acknowledged
}

fn pending_consent_dialog_id() -> egui::Id {
    egui::Id::new("ferrite_code_exec_pending_consent_v1")
}

/// Queue opening the consent dialog (picked up in [`crate::app::dialogs::FerriteApp::render_dialogs`]).
pub fn push_pending_code_execution_consent(
    ctx: &egui::Context,
    pending: crate::state::PendingCodeRun,
) {
    ctx.memory_mut(|mem| {
        mem.data.insert_temp(pending_consent_dialog_id(), pending);
    });
}

pub fn take_pending_code_execution_consent(
    ctx: &egui::Context,
) -> Option<crate::state::PendingCodeRun> {
    ctx.memory_mut(|mem| {
        let id = pending_consent_dialog_id();
        let got = mem.data.get_temp::<crate::state::PendingCodeRun>(id);
        if got.is_some() {
            mem.data.remove::<crate::state::PendingCodeRun>(id);
        }
        got
    })
}

pub fn push_code_execution_toast(ctx: &egui::Context, message: String) {
    ctx.data_mut(|d| {
        let q: &mut Vec<String> =
            d.get_temp_mut_or_insert_with(code_execution_toasts_id(), Vec::new);
        q.push(message);
    });
}

/// Called from [`crate::app::FerriteApp::update`] to surface completion toasts.
pub fn drain_code_execution_toasts(ctx: &egui::Context) -> Vec<String> {
    ctx.data_mut(|d| {
        let q: &mut Vec<String> =
            d.get_temp_mut_or_insert_with(code_execution_toasts_id(), Vec::new);
        std::mem::take(q)
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Streaming output state
// ─────────────────────────────────────────────────────────────────────────────

/// High-level lifecycle of a single Run invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunStatus {
    Running,
    Completed {
        exit_code: Option<i32>,
    },
    Failed {
        message: String,
    },
    TimedOut,
    /// User cancelled the run via the inline output panel's Stop button.
    Cancelled,
}

impl RunStatus {
    pub fn is_running(&self) -> bool {
        matches!(self, RunStatus::Running)
    }

    /// Status glyph for UI display (Phosphor icons; running uses a separate spinner).
    #[cfg(test)]
    pub fn glyph(&self) -> &'static str {
        match self {
            RunStatus::Running => "…",
            RunStatus::Completed { exit_code: Some(0) } => CHECK,
            RunStatus::Completed { .. }
            | RunStatus::Failed { .. }
            | RunStatus::TimedOut
            | RunStatus::Cancelled => X,
        }
    }
}

/// Live mutable state for a single run, shared between worker thread and UI.
pub struct RunState {
    pub status: RunStatus,
    /// Raw stdout bytes received so far. Parsed for ANSI in the UI layer.
    pub stdout: Vec<u8>,
    /// Raw stderr bytes received so far. Parsed for ANSI in the UI layer.
    pub stderr: Vec<u8>,
    pub started_at: Instant,
    pub finished_at: Option<Instant>,
    /// Configured timeout for this run; surfaced to the UI to format
    /// "Timed out after Ns" without re-reading settings.
    pub timeout_secs: u32,
    /// Cooperative cancellation flag polled by the worker thread. Lives
    /// behind its own `Arc` so the worker can check it without locking the
    /// outer `Mutex<RunState>` and contending with the UI thread.
    pub cancel: Arc<AtomicBool>,
}

impl RunState {
    fn new(timeout_secs: u32) -> Self {
        Self {
            status: RunStatus::Running,
            stdout: Vec::new(),
            stderr: Vec::new(),
            started_at: Instant::now(),
            finished_at: None,
            timeout_secs,
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.finished_at
            .map(|f| f.saturating_duration_since(self.started_at))
            .unwrap_or_else(|| self.started_at.elapsed())
    }

    /// True once cancellation has been requested (worker may not have
    /// observed the flag yet).
    pub fn cancel_requested(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
}

/// Cheaply-cloneable handle to live run state.
pub type RunHandle = Arc<Mutex<RunState>>;

/// Request cancellation for an in-flight run. Idempotent; safe to call from
/// the UI thread. The worker observes the flag inside its `wait_child` loop
/// (`<= 100 ms` poll cadence) and kills the spawned child.
pub fn cancel(handle: &RunHandle) {
    if let Ok(state) = handle.lock() {
        state.cancel.store(true, Ordering::Relaxed);
    }
}

/// Spawn a code-execution worker and return a handle that the UI polls.
///
/// The worker runs to completion (or the configured timeout) and never blocks
/// the UI thread. `egui_ctx` is requested-repainted so the UI updates when the
/// worker finishes.
pub fn spawn_run(
    code: String,
    fence_lang: String,
    working_directory: Option<PathBuf>,
    timeout: Duration,
    egui_ctx: egui::Context,
) -> RunHandle {
    let timeout_secs = timeout.as_secs().min(u32::MAX as u64) as u32;
    let handle: RunHandle = Arc::new(Mutex::new(RunState::new(timeout_secs)));
    let worker_handle = Arc::clone(&handle);

    thread::spawn(move || {
        let result = run_snippet_inner(
            &code,
            &fence_lang,
            working_directory.as_deref(),
            timeout,
            Some(&worker_handle),
        );

        if let Ok(mut state) = worker_handle.lock() {
            state.finished_at = Some(Instant::now());
            state.status = match result {
                Ok(exit_code) => RunStatus::Completed {
                    exit_code: Some(exit_code),
                },
                Err(RunError::TimedOut) => RunStatus::TimedOut,
                Err(RunError::Cancelled) => RunStatus::Cancelled,
                Err(RunError::Spawn(msg)) | Err(RunError::Io(msg)) => {
                    RunStatus::Failed { message: msg }
                }
            };
        }
        egui_ctx.request_repaint();
    });

    handle
}

/// Synchronous helper: run a snippet and return the combined output string.
///
/// Test-only blocking API; production uses [`spawn_run`].
#[cfg(test)]
pub fn run_snippet(
    code: &str,
    fence_lang: &str,
    working_directory: Option<&Path>,
    timeout: Duration,
) -> Result<String, String> {
    let timeout_secs = timeout.as_secs().min(u32::MAX as u64) as u32;
    let handle: RunHandle = Arc::new(Mutex::new(RunState::new(timeout_secs)));
    let res = run_snippet_inner(code, fence_lang, working_directory, timeout, Some(&handle));
    let state = handle.lock().map_err(|e| e.to_string())?;
    let mut combined = String::new();
    if !state.stdout.is_empty() {
        combined.push_str(&String::from_utf8_lossy(&state.stdout));
    }
    if !state.stderr.is_empty() {
        if !combined.is_empty() && !combined.ends_with('\n') {
            combined.push('\n');
        }
        combined.push_str("[stderr]\n");
        combined.push_str(&String::from_utf8_lossy(&state.stderr));
    }
    drop(state);

    match res {
        Ok(0) => Ok(if combined.is_empty() {
            "(no output)".into()
        } else {
            combined
        }),
        Ok(code) => Err(if combined.is_empty() {
            format!("Exited with code {code}.")
        } else {
            format!("Exited with code {code}.\n{combined}")
        }),
        Err(RunError::TimedOut) => Err("Process timed out.".into()),
        Err(RunError::Cancelled) => Err("Run cancelled by user.".into()),
        Err(RunError::Spawn(msg)) | Err(RunError::Io(msg)) => Err(msg),
    }
}

#[derive(Debug)]
enum RunError {
    Spawn(String),
    Io(String),
    TimedOut,
    Cancelled,
}

fn run_snippet_inner(
    code: &str,
    fence_lang: &str,
    working_directory: Option<&Path>,
    timeout: Duration,
    handle: Option<&RunHandle>,
) -> Result<i32, RunError> {
    let lang = fence_lang.trim().to_ascii_lowercase();
    let kind = classify_language(&lang)
        .ok_or_else(|| RunError::Spawn("Unsupported language for run.".to_string()))?;
    let cwd = working_directory.unwrap_or_else(|| Path::new("."));

    match kind {
        RunnableKind::Shell => run_shell(&lang, code, cwd, timeout, handle),
        RunnableKind::Python => run_python(code, cwd, timeout, handle),
    }
}

struct TempScript {
    path: PathBuf,
}

impl TempScript {
    fn new(suffix: &str) -> std::io::Result<(Self, PathBuf)> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("ferrite_code_{nanos}{suffix}"));
        Ok((Self { path: path.clone() }, path))
    }
}

impl Drop for TempScript {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Choose the interpreter list per fence + platform.
fn shell_interpreters(lang: &str) -> Vec<&'static str> {
    match lang {
        "zsh" => vec!["zsh"],
        "sh" => vec!["sh"],
        "pwsh" | "powershell" | "ps1" => vec!["pwsh", "powershell"],
        "cmd" | "bat" | "batch" => vec!["cmd"],
        // "bash" / "shell" / generic: prefer platform default, then fall back
        _ => {
            if cfg!(target_os = "windows") {
                vec!["bash", "pwsh", "powershell", "cmd", "sh"]
            } else {
                vec!["bash", "sh"]
            }
        }
    }
}

fn shell_suffix(exe: &str) -> &'static str {
    match exe {
        "pwsh" | "powershell" => ".ps1",
        "cmd" => ".bat",
        _ => ".sh",
    }
}

fn shell_args(exe: &str, script: &Path) -> Vec<std::ffi::OsString> {
    match exe {
        "pwsh" | "powershell" => vec![
            "-NoLogo".into(),
            "-NoProfile".into(),
            "-NonInteractive".into(),
            "-ExecutionPolicy".into(),
            "Bypass".into(),
            "-File".into(),
            script.as_os_str().to_owned(),
        ],
        "cmd" => vec!["/C".into(), script.as_os_str().to_owned()],
        _ => vec![script.as_os_str().to_owned()],
    }
}

fn run_shell(
    lang: &str,
    code: &str,
    cwd: &Path,
    timeout: Duration,
    handle: Option<&RunHandle>,
) -> Result<i32, RunError> {
    let interpreters = shell_interpreters(lang);

    let mut last_io_err: Option<String> = None;
    for exe in interpreters {
        let (_guard, path) =
            TempScript::new(shell_suffix(exe)).map_err(|e| RunError::Spawn(e.to_string()))?;
        std::fs::write(&path, code).map_err(|e| RunError::Spawn(e.to_string()))?;
        let args = shell_args(exe, &path);

        let mut cmd = Command::new(exe);
        cmd.args(&args);
        cmd.current_dir(cwd);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        suppress_console_window(&mut cmd);

        match cmd.spawn() {
            Ok(child) => return wait_child(child, timeout, handle),
            Err(e) if e.kind() == ErrorKind::NotFound => {
                last_io_err = Some(format!("Interpreter '{exe}' not found."));
                continue;
            }
            Err(e) => return Err(RunError::Spawn(format!("Failed to spawn {exe}: {e}"))),
        }
    }

    Err(RunError::Spawn(last_io_err.unwrap_or_else(|| {
        "No shell interpreter found (install Git Bash, WSL, or PowerShell).".into()
    })))
}

fn run_python(
    code: &str,
    cwd: &Path,
    timeout: Duration,
    handle: Option<&RunHandle>,
) -> Result<i32, RunError> {
    let (_guard, path) = TempScript::new(".py").map_err(|e| RunError::Spawn(e.to_string()))?;
    std::fs::write(&path, code).map_err(|e| RunError::Spawn(e.to_string()))?;

    let candidates: &[&str] = if cfg!(windows) {
        &["python", "py", "python3"]
    } else {
        &["python3", "python"]
    };

    for exe in candidates {
        let mut cmd = Command::new(exe);
        if exe == &"py" {
            cmd.arg("-3");
        }
        cmd.arg(&path);
        cmd.current_dir(cwd);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        suppress_console_window(&mut cmd);

        match cmd.spawn() {
            Ok(child) => return wait_child(child, timeout, handle),
            Err(e) if e.kind() == ErrorKind::NotFound => continue,
            Err(e) => return Err(RunError::Spawn(format!("Failed to spawn {exe}: {e}"))),
        }
    }
    Err(RunError::Spawn("Python was not found in PATH.".into()))
}

#[cfg(target_os = "windows")]
fn suppress_console_window(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(target_os = "windows"))]
fn suppress_console_window(_cmd: &mut Command) {}

fn wait_child(
    mut child: std::process::Child,
    timeout: Duration,
    handle: Option<&RunHandle>,
) -> Result<i32, RunError> {
    // Spawn dedicated reader threads so blocking `read()` on the piped streams
    // never starves the main loop's `try_wait` / timeout check. Each thread
    // owns its pipe and pushes bytes into the shared `RunState`.
    let stdout_thread = child.stdout.take().map(|pipe| {
        let h = handle.cloned();
        thread::spawn(move || drain_pipe(pipe, h.as_ref(), false))
    });
    let stderr_thread = child.stderr.take().map(|pipe| {
        let h = handle.cloned();
        thread::spawn(move || drain_pipe(pipe, h.as_ref(), true))
    });

    // Take a cheap clone of the cancel flag so the polling loop can observe
    // user-initiated stop requests without locking the outer mutex on every
    // tick. Reader threads are joined (blocking on `read` returning 0) only
    // after the child is reaped, which closes their pipes.
    let cancel_flag: Option<Arc<AtomicBool>> =
        handle.and_then(|h| h.lock().ok().map(|state| Arc::clone(&state.cancel)));

    let start = Instant::now();
    let join_readers = move || {
        if let Some(t) = stdout_thread {
            let _ = t.join();
        }
        if let Some(t) = stderr_thread {
            let _ = t.join();
        }
    };

    loop {
        if cancel_flag
            .as_ref()
            .is_some_and(|c| c.load(Ordering::Relaxed))
        {
            let _ = child.kill();
            let _ = child.wait();
            join_readers();
            return Err(RunError::Cancelled);
        }

        if start.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            join_readers();
            return Err(RunError::TimedOut);
        }

        match child.try_wait() {
            Ok(Some(status)) => {
                join_readers();
                return Ok(status.code().unwrap_or(-1));
            }
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(e) => return Err(RunError::Io(e.to_string())),
        }
    }
}

fn drain_pipe<R: Read>(mut pipe: R, handle: Option<&RunHandle>, is_stderr: bool) {
    let mut buf = [0u8; 4096];
    loop {
        match pipe.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => push_chunk(handle, &buf[..n], is_stderr),
            Err(_) => break,
        }
    }
}

fn push_chunk(handle: Option<&RunHandle>, bytes: &[u8], is_stderr: bool) {
    let Some(h) = handle else { return };
    if let Ok(mut state) = h.lock() {
        if is_stderr {
            state.stderr.extend_from_slice(bytes);
        } else {
            state.stdout.extend_from_slice(bytes);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::phosphor_icons::{CHECK, X};

    #[test]
    fn classify_normalizes_case() {
        assert_eq!(classify_language("  PYTHON "), Some(RunnableKind::Python));
        assert_eq!(classify_language("Bash"), Some(RunnableKind::Shell));
        assert_eq!(classify_language("PowerShell"), Some(RunnableKind::Shell));
        assert_eq!(classify_language("ps1"), Some(RunnableKind::Shell));
        assert_eq!(classify_language("batch"), Some(RunnableKind::Shell));
        assert_eq!(classify_language("rust"), None);
    }

    #[test]
    fn visibility_respects_flags() {
        let mut s = CodeExecutionUi::disabled();
        s.enable = true;
        s.allow_shell = true;
        assert!(run_button_visible(&s, "sh"));
        s.allow_shell = false;
        assert!(!run_button_visible(&s, "sh"));
    }

    #[test]
    fn run_visible_when_disabled_until_consent() {
        let mut s = CodeExecutionUi::disabled();
        s.allow_shell = true;
        assert!(run_button_visible(&s, "bash"));
        s.consent_acknowledged = true;
        assert!(!run_button_visible(&s, "bash"));
    }

    #[test]
    fn run_hidden_when_disabled_after_consent_without_master() {
        let mut s = CodeExecutionUi::disabled();
        s.allow_shell = true;
        s.consent_acknowledged = true;
        assert!(!run_button_visible(&s, "bash"));
    }

    #[test]
    fn status_glyphs() {
        assert_eq!(RunStatus::Running.glyph(), "…");
        assert_eq!(RunStatus::Completed { exit_code: Some(0) }.glyph(), CHECK);
        assert_eq!(RunStatus::Completed { exit_code: Some(2) }.glyph(), X);
        assert_eq!(RunStatus::TimedOut.glyph(), X);
        assert_eq!(RunStatus::Cancelled.glyph(), X);
    }

    #[test]
    fn cancel_flips_state_flag() {
        let handle: RunHandle = Arc::new(Mutex::new(RunState::new(30)));
        assert!(!handle.lock().unwrap().cancel_requested());
        cancel(&handle);
        assert!(handle.lock().unwrap().cancel_requested());
        // Idempotent: a second call is a no-op.
        cancel(&handle);
        assert!(handle.lock().unwrap().cancel_requested());
    }

    #[test]
    fn run_state_records_timeout_secs() {
        let s = RunState::new(45);
        assert_eq!(s.timeout_secs, 45);
        assert!(matches!(s.status, RunStatus::Running));
    }

    #[test]
    fn cancelled_status_is_terminal() {
        let cancelled = RunStatus::Cancelled;
        assert!(!cancelled.is_running());
        assert!(!matches!(cancelled, RunStatus::Completed { exit_code: Some(0) }));
    }
}
