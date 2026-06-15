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

/// [`crate::markdown::MarkdownEditor`] stores the current snapshot at this id for
/// [`crate::markdown::widgets::EditableCodeBlock`].
pub(crate) fn code_execution_ctx_id() -> egui::Id {
    egui::Id::new("ferrite_markdown_code_execution_ctx")
}

/// Stable key for per-block run handles in egui temp storage.
///
/// Keys off the fenced source (`language` + `code`) so edits above the block
/// do not orphan in-flight output; content edits intentionally get a new key.
pub(crate) fn code_run_state_key(code: &str, language: &str) -> egui::Id {
    let mut input = String::with_capacity(language.len().saturating_add(1).saturating_add(code.len()));
    input.push_str(language);
    input.push('\n');
    input.push_str(code);
    let hash = blake3::hash(input.as_bytes());
    egui::Id::new(("ferrite_code_run", *hash.as_bytes()))
}

/// Plain-text run output for clipboard / fence insertion (ANSI stripped).
///
/// Stderr is prefixed with the same `── stderr ──` heading used by the inline
/// output panel.
pub(crate) fn format_run_output_plain(stdout: &[u8], stderr: &[u8]) -> String {
    let mut out = String::new();
    if !stdout.is_empty() {
        out.push_str(&strip_ansi_plain(&String::from_utf8_lossy(stdout)));
    }
    if !stderr.is_empty() {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&format!(
            "── {} ──\n",
            rust_i18n::t!("widgets.code_block.run_stderr_heading")
        ));
        out.push_str(&strip_ansi_plain(&String::from_utf8_lossy(stderr)));
    }
    out
}

/// Best-effort ANSI escape stripping for clipboard / fence insertion.
fn strip_ansi_plain(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if let Some(&next) = chars.peek() {
                if next == '[' {
                    chars.next();
                    while let Some(&cc) = chars.peek() {
                        chars.next();
                        if cc.is_ascii_alphabetic() {
                            break;
                        }
                    }
                    continue;
                } else if next == ']' {
                    chars.next();
                    while let Some(&cc) = chars.peek() {
                        chars.next();
                        if cc == '\x07' {
                            break;
                        }
                    }
                    continue;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
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
        use crate::ui::phosphor_icons::{CHECK, X};
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

/// How a temp script is passed to the child process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellDispatchKind {
    /// `program script.sh` (POSIX shells).
    PosixScript,
    /// `pwsh|powershell -File script.ps1`
    PowerShellFile,
    /// `cmd /C script.bat`
    CmdBatch,
    /// `wsl.exe bash script.sh` (Windows WSL fallback).
    WslBash,
}

/// Ordered shell interpreter candidate for a fenced language.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ShellDispatch {
    program: String,
    script_suffix: &'static str,
    kind: ShellDispatchKind,
}

impl ShellDispatch {
    fn new(program: &str, script_suffix: &'static str, kind: ShellDispatchKind) -> Self {
        Self {
            program: program.to_string(),
            script_suffix,
            kind,
        }
    }
}

const GIT_BASH_CANDIDATES: &[&str] = &[
    r"C:\Program Files\Git\bin\bash.exe",
    r"C:\Program Files\Git\usr\bin\bash.exe",
];

const WSL_LAUNCHER: &str = r"C:\Windows\System32\wsl.exe";

/// Windows POSIX/bash dispatch: PATH `bash`, Git Bash installs, then WSL.
fn windows_posix_bash_chain() -> Vec<ShellDispatch> {
    let mut chain = vec![ShellDispatch::new("bash", ".sh", ShellDispatchKind::PosixScript)];
    for path in GIT_BASH_CANDIDATES {
        chain.push(ShellDispatch::new(path, ".sh", ShellDispatchKind::PosixScript));
    }
    chain.push(ShellDispatch::new(
        WSL_LAUNCHER,
        ".sh",
        ShellDispatchKind::WslBash,
    ));
    chain
}

/// Ordered interpreter candidates for a fence language (before availability checks).
fn shell_dispatch_chain(lang: &str) -> Vec<ShellDispatch> {
    shell_dispatch_chain_for(lang, cfg!(windows))
}

/// Platform-parameterized chain for unit tests (`windows` simulates Windows dispatch).
fn shell_dispatch_chain_for(lang: &str, windows: bool) -> Vec<ShellDispatch> {
    match lang {
        "pwsh" | "powershell" | "ps1" => vec![
            ShellDispatch::new("pwsh", ".ps1", ShellDispatchKind::PowerShellFile),
            ShellDispatch::new(
                "powershell",
                ".ps1",
                ShellDispatchKind::PowerShellFile,
            ),
        ],
        "cmd" | "bat" | "batch" => vec![ShellDispatch::new(
            "cmd",
            ".bat",
            ShellDispatchKind::CmdBatch,
        )],
        "zsh" if windows => {
            let mut chain = vec![ShellDispatch::new("zsh", ".sh", ShellDispatchKind::PosixScript)];
            chain.extend(windows_posix_bash_chain());
            chain
        }
        "zsh" => vec![
            ShellDispatch::new("zsh", ".sh", ShellDispatchKind::PosixScript),
            ShellDispatch::new("sh", ".sh", ShellDispatchKind::PosixScript),
        ],
        "sh" if windows => {
            let mut chain = vec![ShellDispatch::new("sh", ".sh", ShellDispatchKind::PosixScript)];
            chain.extend(windows_posix_bash_chain());
            chain
        }
        "sh" => vec![ShellDispatch::new("sh", ".sh", ShellDispatchKind::PosixScript)],
        // `bash`, `shell`, and any other POSIX-style fence
        _ if windows => windows_posix_bash_chain(),
        _ => vec![
            ShellDispatch::new("bash", ".sh", ShellDispatchKind::PosixScript),
            ShellDispatch::new("sh", ".sh", ShellDispatchKind::PosixScript),
        ],
    }
}

/// Returns candidates whose `program` passes `is_available` (injectable for tests).
fn filter_available_dispatches(
    chain: &[ShellDispatch],
    is_available: &dyn Fn(&str) -> bool,
) -> Vec<ShellDispatch> {
    chain
        .iter()
        .filter(|d| is_available(&d.program))
        .cloned()
        .collect()
}

fn is_posix_fence(lang: &str) -> bool {
    matches!(lang, "bash" | "sh" | "shell" | "zsh")
}

fn shell_dispatch_exhausted_error(lang: &str) -> String {
    if cfg!(windows) && is_posix_fence(lang) {
        rust_i18n::t!("widgets.code_block.run_posix_shell_missing_windows").to_string()
    } else {
        match lang {
            "zsh" => rust_i18n::t!("widgets.code_block.run_shell_missing_zsh").to_string(),
            "bash" | "shell" => {
                rust_i18n::t!("widgets.code_block.run_shell_missing_bash").to_string()
            }
            "sh" => rust_i18n::t!("widgets.code_block.run_shell_missing_sh").to_string(),
            "pwsh" | "powershell" | "ps1" => {
                rust_i18n::t!("widgets.code_block.run_shell_missing_powershell").to_string()
            }
            "cmd" | "bat" | "batch" => {
                rust_i18n::t!("widgets.code_block.run_shell_missing_cmd").to_string()
            }
            _ => rust_i18n::t!("widgets.code_block.run_shell_missing_generic").to_string(),
        }
    }
}

fn configure_shell_command(dispatch: &ShellDispatch, script: &Path, cwd: &Path) -> Command {
    let mut cmd = Command::new(&dispatch.program);
    match dispatch.kind {
        ShellDispatchKind::PosixScript => {
            cmd.arg(script);
        }
        ShellDispatchKind::PowerShellFile => {
            cmd.args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ]);
            cmd.arg(script);
        }
        ShellDispatchKind::CmdBatch => {
            cmd.arg("/C");
            cmd.arg(script);
        }
        ShellDispatchKind::WslBash => {
            cmd.arg("bash");
            cmd.arg(script);
        }
    }
    cmd.current_dir(cwd);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    suppress_console_window(&mut cmd);
    cmd
}

fn run_shell(
    lang: &str,
    code: &str,
    cwd: &Path,
    timeout: Duration,
    handle: Option<&RunHandle>,
) -> Result<i32, RunError> {
    let chain = shell_dispatch_chain(lang);

    let mut last_io_err: Option<String> = None;
    for dispatch in &chain {
        let (_guard, path) = TempScript::new(dispatch.script_suffix)
            .map_err(|e| RunError::Spawn(e.to_string()))?;
        std::fs::write(&path, code).map_err(|e| RunError::Spawn(e.to_string()))?;

        let mut cmd = configure_shell_command(dispatch, &path, cwd);

        match cmd.spawn() {
            Ok(child) => return wait_child(child, timeout, handle),
            Err(e) if e.kind() == ErrorKind::NotFound => {
                last_io_err = Some(
                    rust_i18n::t!(
                        "widgets.code_block.run_interpreter_not_found",
                        program = dispatch.program
                    )
                    .to_string(),
                );
                continue;
            }
            Err(e) => {
                return Err(RunError::Spawn(
                    rust_i18n::t!(
                        "widgets.code_block.run_interpreter_spawn_failed",
                        program = dispatch.program,
                        error = e.to_string()
                    )
                    .to_string(),
                ));
            }
        }
    }

    Err(RunError::Spawn(
        last_io_err.unwrap_or_else(|| shell_dispatch_exhausted_error(lang)),
    ))
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

    fn chain_programs(lang: &str, windows: bool) -> Vec<String> {
        shell_dispatch_chain_for(lang, windows)
            .into_iter()
            .map(|d| d.program)
            .collect()
    }

    fn chain_suffixes(lang: &str, windows: bool) -> Vec<&'static str> {
        shell_dispatch_chain_for(lang, windows)
            .into_iter()
            .map(|d| d.script_suffix)
            .collect()
    }

    #[test]
    fn windows_bash_chain_never_uses_powershell_or_cmd() {
        let chain = shell_dispatch_chain_for("bash", true);
        assert!(
            chain
                .iter()
                .all(|d| d.script_suffix == ".sh" && d.kind != ShellDispatchKind::PowerShellFile),
            "bash on Windows must only use POSIX script dispatch: {chain:?}"
        );
        let programs = chain_programs("bash", true);
        assert_eq!(programs[0], "bash");
        assert!(programs.contains(&GIT_BASH_CANDIDATES[0].to_string()));
        assert!(programs.contains(&WSL_LAUNCHER.to_string()));
        assert!(!programs.iter().any(|p| p == "pwsh" || p == "powershell" || p == "cmd"));
    }

    #[test]
    fn windows_shell_fence_matches_bash_chain() {
        let bash = chain_programs("bash", true);
        let shell = chain_programs("shell", true);
        assert_eq!(bash, shell);
    }

    #[test]
    fn unix_bash_falls_back_to_sh() {
        let chain = shell_dispatch_chain_for("bash", false);
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].program, "bash");
        assert_eq!(chain[1].program, "sh");
        assert!(chain_suffixes("bash", false).iter().all(|s| *s == ".sh"));
    }

    #[test]
    fn unix_zsh_falls_back_to_sh() {
        let chain = shell_dispatch_chain_for("zsh", false);
        assert_eq!(chain[0].program, "zsh");
        assert_eq!(chain[1].program, "sh");
    }

    #[test]
    fn unix_sh_has_no_fallback() {
        let chain = shell_dispatch_chain_for("sh", false);
        assert_eq!(chain.len(), 1);
        assert_eq!(chain[0].program, "sh");
    }

    #[test]
    fn windows_zsh_tries_zsh_then_posix_bash_chain() {
        let programs = chain_programs("zsh", true);
        assert_eq!(programs[0], "zsh");
        assert!(programs.contains(&"bash".to_string()));
        assert!(!programs.iter().any(|p| p == "pwsh" || p == "powershell"));
    }

    #[test]
    fn filter_available_dispatches_respects_mock_availability() {
        let chain = shell_dispatch_chain_for("bash", true);
        let available = |name: &str| name == GIT_BASH_CANDIDATES[0];
        let filtered = filter_available_dispatches(&chain, &available);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].program, GIT_BASH_CANDIDATES[0]);
    }

    #[test]
    fn filter_available_dispatches_unix_zsh_without_zsh() {
        let chain = shell_dispatch_chain_for("zsh", false);
        let available = |name: &str| name == "sh";
        let filtered = filter_available_dispatches(&chain, &available);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].program, "sh");
    }

    #[test]
    fn windows_bash_absent_leaves_no_powershell_fallback() {
        let chain = shell_dispatch_chain_for("bash", true);
        let available = |_name: &str| false;
        let filtered = filter_available_dispatches(&chain, &available);
        assert!(filtered.is_empty());
        assert!(
            !chain
                .iter()
                .any(|d| d.kind == ShellDispatchKind::PowerShellFile)
        );
    }

    #[test]
    fn powershell_chain_uses_ps1_suffix_only() {
        let suffixes = chain_suffixes("powershell", true);
        assert_eq!(suffixes, vec![".ps1", ".ps1"]);
    }

    #[test]
    fn posix_exhausted_error_windows_mentions_git_bash() {
        let msg = shell_dispatch_exhausted_error("bash");
        if cfg!(windows) {
            assert!(msg.contains("Git Bash") || msg.contains("WSL"));
        } else {
            // On Unix CI the Windows-specific key still resolves to the English string.
            assert!(!msg.is_empty());
        }
    }

    #[test]
    fn zsh_exhausted_error_mentions_sh_fallback() {
        let msg = shell_dispatch_exhausted_error("zsh");
        assert!(msg.contains("sh"));
    }

    #[test]
    fn code_run_state_key_stable_when_block_moves_lines() {
        let code = "sleep 2 && echo hi";
        let lang = "bash";
        let key_at_line_5 = code_run_state_key(code, lang);
        let key_at_line_12 = code_run_state_key(code, lang);
        assert_eq!(key_at_line_5, key_at_line_12);
    }

    #[test]
    fn code_run_state_key_changes_when_content_changes() {
        let lang = "bash";
        let key_a = code_run_state_key("echo a", lang);
        let key_b = code_run_state_key("echo b", lang);
        assert_ne!(key_a, key_b);
    }

    #[test]
    fn code_run_state_key_changes_when_language_changes() {
        let code = "echo hi";
        let key_bash = code_run_state_key(code, "bash");
        let key_python = code_run_state_key(code, "python");
        assert_ne!(key_bash, key_python);
    }

    #[test]
    fn format_run_output_plain_prefixes_stderr_like_panel() {
        let stdout = b"hello\n";
        let stderr = b"warn\n";
        let plain = format_run_output_plain(stdout, stderr);
        assert!(plain.starts_with("hello\n"));
        assert!(
            plain.contains("── stderr ──\nwarn"),
            "stderr block should use panel heading prefix, got: {plain:?}"
        );
    }

    #[test]
    fn format_run_output_plain_stderr_only() {
        let plain = format_run_output_plain(b"", b"oops");
        assert_eq!(plain, "── stderr ──\noops");
    }
}
