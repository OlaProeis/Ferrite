//! Background worker thread for language server processes and async LSP I/O.
//!
//! The UI thread owns [`LspManager`] and sends commands via an `mpsc` channel; the worker
//! reports [`LspManagerEvent`] (status changes, diagnostics, spawn failures) on another channel.
//! The worker manages child processes with exponential backoff restart, performs the LSP
//! `initialize` / `initialized` handshake, reads JSON-RPC from server stdout, and routes
//! `textDocument/publishDiagnostics` to the UI thread.

use crate::lsp::detection::LspServerSpec;
use crate::lsp::state::{DiagnosticEntry, DiagnosticSeverity, ServerStatus};
use crate::lsp::transport::{read_lsp_message, write_lsp_message, MessageReader};
use log::{debug, error, info, warn};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const BACKOFF_INITIAL: Duration = Duration::from_secs(1);
const BACKOFF_MAX: Duration = Duration::from_secs(30);
const BACKOFF_RESET_AFTER: Duration = Duration::from_secs(60);
const HEALTH_CHECK_INTERVAL: Duration = Duration::from_millis(200);

/// Commands sent from the UI thread to the LSP worker.
#[derive(Debug)]
pub enum LspCommand {
    Start {
        server_key: String,
        spec: LspServerSpec,
        workspace_root: Option<PathBuf>,
    },
    Stop {
        server_key: String,
    },
    StopAll,
    Restart {
        server_key: String,
    },
    /// Notify the server that a document was opened.
    DidOpen {
        server_key: String,
        uri: String,
        language_id: String,
        version: i64,
        text: String,
    },
    /// Notify the server that a document changed (full sync).
    DidChange {
        server_key: String,
        uri: String,
        version: i64,
        text: String,
    },
    /// Notify the server that a document was closed.
    DidClose {
        server_key: String,
        uri: String,
    },
    Shutdown,
}

/// Events emitted by the worker for the UI thread to drain (e.g. each frame).
#[derive(Debug, Clone)]
pub enum LspManagerEvent {
    StatusChanged {
        server_key: String,
        status: ServerStatus,
    },
    SpawnFailed {
        server_key: String,
        program: String,
        error: String,
    },
    /// Diagnostics for a file (replaces previous diagnostics for that URI).
    Diagnostics {
        server_key: String,
        path: PathBuf,
        diagnostics: Vec<DiagnosticEntry>,
    },
}

struct ActiveServer {
    child: Child,
    stdin: ChildStdin,
    spec: LspServerSpec,
    workspace_root: Option<PathBuf>,
    started_at: Instant,
    backoff: Duration,
    next_request_id: i64,
    initialized: bool,
}

impl ActiveServer {
    fn next_id(&mut self) -> i64 {
        let id = self.next_request_id;
        self.next_request_id += 1;
        id
    }

    fn send(&mut self, msg: &Value) -> bool {
        match write_lsp_message(&mut self.stdin, msg) {
            Ok(()) => true,
            Err(e) => {
                warn!("LSP write error: {}", e);
                false
            }
        }
    }
}

/// Owns channels and the worker join handle; cheap to pass inside [`crate::state::AppState`].
#[derive(Debug)]
pub struct LspManager {
    cmd_tx: mpsc::Sender<LspCommand>,
    event_rx: mpsc::Receiver<LspManagerEvent>,
    join: Option<thread::JoinHandle<()>>,
}

impl LspManager {
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let join = thread::spawn(move || worker_main(cmd_rx, event_tx));
        Self {
            cmd_tx,
            event_rx,
            join: Some(join),
        }
    }

    pub fn start_server(
        &self,
        server_key: impl Into<String>,
        spec: LspServerSpec,
        workspace_root: Option<PathBuf>,
    ) {
        let _ = self.cmd_tx.send(LspCommand::Start {
            server_key: server_key.into(),
            spec,
            workspace_root,
        });
    }

    pub fn stop_server(&self, server_key: impl Into<String>) {
        let _ = self.cmd_tx.send(LspCommand::Stop {
            server_key: server_key.into(),
        });
    }

    pub fn stop_all_servers(&self) {
        let _ = self.cmd_tx.send(LspCommand::StopAll);
    }

    pub fn restart_server(&self, server_key: impl Into<String>) {
        let _ = self.cmd_tx.send(LspCommand::Restart {
            server_key: server_key.into(),
        });
    }

    pub fn did_open(
        &self,
        server_key: impl Into<String>,
        uri: String,
        language_id: String,
        version: i64,
        text: String,
    ) {
        let _ = self.cmd_tx.send(LspCommand::DidOpen {
            server_key: server_key.into(),
            uri,
            language_id,
            version,
            text,
        });
    }

    pub fn did_change(
        &self,
        server_key: impl Into<String>,
        uri: String,
        version: i64,
        text: String,
    ) {
        let _ = self.cmd_tx.send(LspCommand::DidChange {
            server_key: server_key.into(),
            uri,
            version,
            text,
        });
    }

    pub fn did_close(&self, server_key: impl Into<String>, uri: String) {
        let _ = self.cmd_tx.send(LspCommand::DidClose {
            server_key: server_key.into(),
            uri,
        });
    }

    /// Non-blocking drain of pending events (call from the main/UI loop).
    pub fn poll_events(&mut self) -> Vec<LspManagerEvent> {
        let mut out = Vec::new();
        while let Ok(ev) = self.event_rx.try_recv() {
            out.push(ev);
        }
        out
    }
}

impl Default for LspManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for LspManager {
    fn drop(&mut self) {
        let _ = self.cmd_tx.send(LspCommand::Shutdown);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Worker internals
// ─────────────────────────────────────────────────────────────────────────────

fn send_status(tx: &mpsc::Sender<LspManagerEvent>, server_key: &str, status: ServerStatus) {
    let _ = tx.send(LspManagerEvent::StatusChanged {
        server_key: server_key.to_string(),
        status,
    });
}

fn spawn_pipe_drain(mut pipe: impl Read + Send + 'static) {
    thread::spawn(move || {
        let mut buf = [0u8; 8192];
        loop {
            match pipe.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
        }
    });
}

/// Messages arriving from a server stdout reader thread.
enum StdoutMsg {
    JsonRpc(Value),
    Eof,
    Error(String),
}

/// Spawn a background thread that reads JSON-RPC from server stdout and forwards
/// parsed messages via `msg_tx`. The worker thread polls `msg_rx` non-blocking.
fn spawn_stdout_reader(
    stdout: std::process::ChildStdout,
    msg_tx: mpsc::Sender<(String, StdoutMsg)>,
    server_key: String,
) {
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut acc = MessageReader::new();
        loop {
            match read_lsp_message(&mut reader, &mut acc) {
                Ok(val) => {
                    if msg_tx
                        .send((server_key.clone(), StdoutMsg::JsonRpc(val)))
                        .is_err()
                    {
                        break;
                    }
                }
                Err(e) => {
                    let msg = format!("{}", e);
                    if msg.contains("EOF") || msg.contains("eof") {
                        let _ = msg_tx.send((server_key.clone(), StdoutMsg::Eof));
                    } else {
                        let _ = msg_tx.send((server_key.clone(), StdoutMsg::Error(msg)));
                    }
                    break;
                }
            }
        }
    });
}

fn spawn_server(spec: &LspServerSpec) -> Result<Child, String> {
    let mut cmd = Command::new(&spec.program);
    cmd.args(&spec.args);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    cmd.spawn().map_err(|e| format!("{e}"))
}

fn try_start(
    server_key: &str,
    spec: LspServerSpec,
    workspace_root: Option<PathBuf>,
    backoff: Duration,
    active: &mut HashMap<String, ActiveServer>,
    event_tx: &mpsc::Sender<LspManagerEvent>,
    stdout_tx: &mpsc::Sender<(String, StdoutMsg)>,
) -> bool {
    send_status(event_tx, server_key, ServerStatus::Starting);
    match spawn_server(&spec) {
        Ok(mut child) => {
            let stdin = child.stdin.take().expect("stdin was piped");
            let stdout = child.stdout.take().expect("stdout was piped");

            // Drain stderr to prevent blocking
            if let Some(err) = child.stderr.take() {
                spawn_pipe_drain(err);
            }

            // Spawn stdout reader thread
            spawn_stdout_reader(stdout, stdout_tx.clone(), server_key.to_string());

            info!("LSP server started: {} ({})", server_key, spec.program);
            let mut server = ActiveServer {
                child,
                stdin,
                spec,
                workspace_root,
                started_at: Instant::now(),
                backoff,
                next_request_id: 1,
                initialized: false,
            };

            send_status(event_tx, server_key, ServerStatus::Initializing);
            send_initialize(&mut server);

            active.insert(server_key.to_string(), server);
            true
        }
        Err(e) => {
            warn!("LSP spawn failed for {}: {}", server_key, e);
            let _ = event_tx.send(LspManagerEvent::SpawnFailed {
                server_key: server_key.to_string(),
                program: spec.program.clone(),
                error: e.clone(),
            });
            send_status(event_tx, server_key, ServerStatus::Error(e));
            false
        }
    }
}

fn stop_all(active: &mut HashMap<String, ActiveServer>, event_tx: &mpsc::Sender<LspManagerEvent>) {
    let keys: Vec<String> = active.keys().cloned().collect();
    for key in keys {
        if let Some(mut s) = active.remove(&key) {
            send_shutdown(&mut s);
        }
        send_status(event_tx, &key, ServerStatus::Disconnected);
    }
}

fn send_shutdown(server: &mut ActiveServer) {
    let id = server.next_id();
    let _ = server.send(&json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "shutdown",
        "params": null
    }));
    let _ = server.send(&json!({
        "jsonrpc": "2.0",
        "method": "exit"
    }));
    let _ = server.child.kill();
    let _ = server.child.wait();
}

fn send_initialize(server: &mut ActiveServer) {
    let id = server.next_id();
    let root_uri = server
        .workspace_root
        .as_ref()
        .map(|p| format!("file:///{}", p.display().to_string().replace('\\', "/")))
        .unwrap_or_default();

    let params = json!({
        "processId": std::process::id(),
        "capabilities": {
            "textDocument": {
                "publishDiagnostics": {
                    "relatedInformation": false
                },
                "synchronization": {
                    "didSave": true,
                    "willSave": false,
                    "willSaveWaitUntil": false,
                    "dynamicRegistration": false
                }
            }
        },
        "rootUri": if root_uri.is_empty() { Value::Null } else { Value::String(root_uri.clone()) },
        "workspaceFolders": if root_uri.is_empty() {
            Value::Null
        } else {
            json!([{ "uri": root_uri, "name": "workspace" }])
        }
    });

    server.send(&json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "initialize",
        "params": params
    }));
}

fn send_initialized(server: &mut ActiveServer) {
    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    }));
}

fn send_did_open(
    server: &mut ActiveServer,
    uri: &str,
    language_id: &str,
    version: i64,
    text: &str,
) {
    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": uri,
                "languageId": language_id,
                "version": version,
                "text": text
            }
        }
    }));
}

fn send_did_change(server: &mut ActiveServer, uri: &str, version: i64, text: &str) {
    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": { "uri": uri, "version": version },
            "contentChanges": [{ "text": text }]
        }
    }));
}

fn send_did_close(server: &mut ActiveServer, uri: &str) {
    server.send(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didClose",
        "params": {
            "textDocument": { "uri": uri }
        }
    }));
}

// ─────────────────────────────────────────────────────────────────────────────
// Notification handling
// ─────────────────────────────────────────────────────────────────────────────

fn handle_server_message(
    server_key: &str,
    msg: &Value,
    active: &mut HashMap<String, ActiveServer>,
    event_tx: &mpsc::Sender<LspManagerEvent>,
) {
    // Check for error responses
    if let Some(err) = msg.get("error") {
        let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(0);
        let emsg = err
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown");
        warn!(
            "LSP [{}] error response (code {}): {}",
            server_key, code, emsg
        );
        return;
    }

    // Check if this is a response to our `initialize` request
    if msg.get("id").is_some() && msg.get("result").is_some() {
        if let Some(result) = msg.get("result") {
            if result.get("capabilities").is_some() {
                if let Some(server) = active.get_mut(server_key) {
                    if !server.initialized {
                        server.initialized = true;
                        send_initialized(server);
                        send_status(event_tx, server_key, ServerStatus::Ready);
                        info!("LSP server {} initialized successfully", server_key);
                    }
                }
                return;
            }
        }
    }

    let method = msg.get("method").and_then(|m| m.as_str());
    match method {
        Some("textDocument/publishDiagnostics") => {
            if let Some(params) = msg.get("params") {
                handle_publish_diagnostics(server_key, params, event_tx);
            }
        }
        Some("window/logMessage") | Some("window/showMessage") => {
            if let Some(params) = msg.get("params") {
                let message = params.get("message").and_then(|m| m.as_str()).unwrap_or("");
                debug!("LSP [{}]: {}", server_key, message);
            }
        }
        _ => {
            // Ignore unknown notifications / responses
        }
    }
}

fn handle_publish_diagnostics(
    server_key: &str,
    params: &Value,
    event_tx: &mpsc::Sender<LspManagerEvent>,
) {
    let uri = match params.get("uri").and_then(|u| u.as_str()) {
        Some(u) => u,
        None => return,
    };

    let path = uri_to_path(uri);
    let diags_json = params.get("diagnostics").and_then(|d| d.as_array());

    let diagnostics: Vec<DiagnosticEntry> = diags_json
        .map(|arr| arr.iter().filter_map(|d| parse_diagnostic(d)).collect())
        .unwrap_or_default();

    debug!(
        "LSP [{}] publishDiagnostics: {} → {} diagnostic(s)",
        server_key,
        path.display(),
        diagnostics.len()
    );

    let _ = event_tx.send(LspManagerEvent::Diagnostics {
        server_key: server_key.to_string(),
        path,
        diagnostics,
    });
}

fn parse_diagnostic(val: &Value) -> Option<DiagnosticEntry> {
    let range = val.get("range")?;
    let start = range.get("start")?;
    let end = range.get("end")?;

    Some(DiagnosticEntry {
        start_line: start.get("line")?.as_u64()? as usize,
        start_col: start.get("character")?.as_u64()? as usize,
        end_line: end.get("line")?.as_u64()? as usize,
        end_col: end.get("character")?.as_u64()? as usize,
        severity: DiagnosticSeverity::from_lsp(
            val.get("severity").and_then(|s| s.as_u64()).unwrap_or(1),
        ),
        message: val
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("")
            .to_string(),
        source: val
            .get("source")
            .and_then(|s| s.as_str())
            .map(|s| s.to_string()),
    })
}

fn uri_to_path(uri: &str) -> PathBuf {
    let raw = if let Some(rest) = uri.strip_prefix("file:///") {
        #[cfg(windows)]
        {
            PathBuf::from(rest.replace('/', "\\"))
        }
        #[cfg(not(windows))]
        {
            PathBuf::from(format!("/{}", rest))
        }
    } else if let Some(rest) = uri.strip_prefix("file://") {
        PathBuf::from(rest)
    } else {
        PathBuf::from(uri)
    };
    // Normalize so drive letter casing and \\?\ prefix match Tab::path
    super::normalize_lsp_path(&raw)
}

// ─────────────────────────────────────────────────────────────────────────────
// Worker main loop
// ─────────────────────────────────────────────────────────────────────────────

struct PendingRestart {
    spec: LspServerSpec,
    workspace_root: Option<PathBuf>,
    backoff: Duration,
    restart_at: Instant,
}

fn worker_main(cmd_rx: mpsc::Receiver<LspCommand>, event_tx: mpsc::Sender<LspManagerEvent>) {
    let mut active: HashMap<String, ActiveServer> = HashMap::new();
    let mut pending_restarts: HashMap<String, PendingRestart> = HashMap::new();

    // Channel for JSON-RPC messages from all server stdout reader threads
    let (stdout_tx, stdout_rx) = mpsc::channel::<(String, StdoutMsg)>();

    loop {
        // 1. Process UI commands (non-blocking after first blocking wait)
        match cmd_rx.recv_timeout(HEALTH_CHECK_INTERVAL) {
            Ok(cmd) => match cmd {
                LspCommand::Shutdown => break,
                LspCommand::Start {
                    server_key,
                    spec,
                    workspace_root,
                } => {
                    pending_restarts.remove(&server_key);
                    if let Some(mut prev) = active.remove(&server_key) {
                        send_shutdown(&mut prev);
                    }
                    try_start(
                        &server_key,
                        spec,
                        workspace_root,
                        BACKOFF_INITIAL,
                        &mut active,
                        &event_tx,
                        &stdout_tx,
                    );
                }
                LspCommand::Stop { server_key } => {
                    pending_restarts.remove(&server_key);
                    if let Some(mut s) = active.remove(&server_key) {
                        send_shutdown(&mut s);
                    }
                    send_status(&event_tx, &server_key, ServerStatus::Disconnected);
                }
                LspCommand::StopAll => {
                    pending_restarts.clear();
                    stop_all(&mut active, &event_tx);
                }
                LspCommand::Restart { server_key } => {
                    pending_restarts.remove(&server_key);
                    let info = active
                        .get(&server_key)
                        .map(|s| (s.spec.clone(), s.workspace_root.clone()));
                    if let Some((spec, ws)) = info {
                        if let Some(mut s) = active.remove(&server_key) {
                            send_shutdown(&mut s);
                        }
                        try_start(
                            &server_key,
                            spec,
                            ws,
                            BACKOFF_INITIAL,
                            &mut active,
                            &event_tx,
                            &stdout_tx,
                        );
                    } else {
                        warn!("LSP restart: unknown server {}", server_key);
                    }
                }
                LspCommand::DidOpen {
                    server_key,
                    uri,
                    language_id,
                    version,
                    text,
                } => {
                    if let Some(server) = active.get_mut(&server_key) {
                        if server.initialized {
                            debug!(
                                "LSP [{}] sending didOpen: {} (v{})",
                                server_key, uri, version
                            );
                            send_did_open(server, &uri, &language_id, version, &text);
                        } else {
                            warn!(
                                "LSP [{}] didOpen skipped: server not initialized",
                                server_key
                            );
                        }
                    }
                }
                LspCommand::DidChange {
                    server_key,
                    uri,
                    version,
                    text,
                } => {
                    if let Some(server) = active.get_mut(&server_key) {
                        if server.initialized {
                            debug!(
                                "LSP [{}] sending didChange: {} (v{}, {} bytes)",
                                server_key,
                                uri,
                                version,
                                text.len()
                            );
                            send_did_change(server, &uri, version, &text);
                        }
                    }
                }
                LspCommand::DidClose { server_key, uri } => {
                    if let Some(server) = active.get_mut(&server_key) {
                        if server.initialized {
                            debug!("LSP [{}] sending didClose: {}", server_key, uri);
                            send_did_close(server, &uri);
                        }
                    }
                }
            },
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }

        // 2. Drain all pending stdout messages from server reader threads
        while let Ok((key, msg)) = stdout_rx.try_recv() {
            match msg {
                StdoutMsg::JsonRpc(val) => {
                    handle_server_message(&key, &val, &mut active, &event_tx);
                }
                StdoutMsg::Eof => {
                    debug!("LSP stdout EOF for {}", key);
                }
                StdoutMsg::Error(e) => {
                    error!("LSP stdout read error for {}: {}", key, e);
                }
            }
        }

        // 3. Health-check: detect crashed servers and schedule restarts
        let mut crashed: Vec<(String, LspServerSpec, Option<PathBuf>, Duration)> = Vec::new();
        for (key, server) in active.iter_mut() {
            match server.child.try_wait() {
                Ok(Some(_status)) => {
                    let uptime = server.started_at.elapsed();
                    let next_backoff = if uptime >= BACKOFF_RESET_AFTER {
                        BACKOFF_INITIAL
                    } else {
                        (server.backoff * 2).min(BACKOFF_MAX)
                    };
                    warn!(
                        "LSP server {} ({}) exited after {:.1}s, will restart in {:.1}s",
                        key,
                        server.spec.program,
                        uptime.as_secs_f64(),
                        next_backoff.as_secs_f64()
                    );
                    crashed.push((
                        key.clone(),
                        server.spec.clone(),
                        server.workspace_root.clone(),
                        next_backoff,
                    ));
                }
                Ok(None) => {}
                Err(e) => {
                    warn!("LSP health check error for {}: {}", key, e);
                }
            }
        }

        for (key, spec, ws, backoff) in crashed {
            active.remove(&key);
            send_status(
                &event_tx,
                &key,
                ServerStatus::Error("server crashed".into()),
            );
            pending_restarts.insert(
                key,
                PendingRestart {
                    spec,
                    workspace_root: ws,
                    backoff,
                    restart_at: Instant::now() + backoff,
                },
            );
        }

        // 4. Process pending restarts whose backoff has elapsed
        let now = Instant::now();
        let ready: Vec<String> = pending_restarts
            .iter()
            .filter(|(_, pr)| now >= pr.restart_at)
            .map(|(k, _)| k.clone())
            .collect();

        for key in ready {
            if let Some(pr) = pending_restarts.remove(&key) {
                info!(
                    "LSP auto-restart: {} (backoff was {:.1}s)",
                    key,
                    pr.backoff.as_secs_f64()
                );
                try_start(
                    &key,
                    pr.spec,
                    pr.workspace_root,
                    pr.backoff,
                    &mut active,
                    &event_tx,
                    &stdout_tx,
                );
            }
        }
    }

    // Clean shutdown
    for (_, mut s) in active.drain() {
        send_shutdown(&mut s);
    }
}
