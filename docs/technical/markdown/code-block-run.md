# Code block Run (preview)

The **Run** control on fenced code blocks appears in **Rendered** and **Split** preview when code execution is allowed by settings. It sits in the code-block header alongside Copy and Edit.

## Behaviour

- **Languages:**
  - **Shell:** `bash`, `sh`, `shell`, `zsh`, `pwsh`, `powershell`, `ps1`, `cmd`, `bat`, `batch` — fence language is compared case-insensitively after trim.
  - **Python:** `python`, `python3`, `py`. Detection prefers `python3` on Unix and `python` then `py` on Windows.
  - **Not runnable:** other fences (e.g. `csharp`, `rust`, `js`) do not show **Run** — only the shell and Python families above are implemented.
- **Gating:** Uses `enable_code_execution`, `allow_shell`, `allow_python`, `code_execution_consent_acknowledged`, `code_execution_timeout_secs`, and `code_execution_show_inline_output` — see [Code execution settings](../config/code-execution-settings.md). **Run** can appear while the master toggle is still off until consent is recorded (first modal or enabling from Settings); see [Code execution consent dialog](./code-execution-consent-dialog.md). If only Python blocks show **Run**, enable **Allow shell blocks** under Settings → Editor → Code execution (or use a supported shell fence — `csharp` / `rust` never show **Run** until those runners exist).
- **Working directory:** The open file’s parent folder when the tab has a path; otherwise the process default.

## Copy-paste examples (Windows-friendly)

Use these fenced languages so **Run** appears (with shell/Python allowed in Settings):

**PowerShell** (`powershell`, `pwsh`, or `ps1`):

````markdown
```powershell
Write-Output "Hello from PowerShell"
Get-Location
```
````

**Command Prompt** (`cmd` or `bat`):

````markdown
```cmd
echo Hello from CMD
cd
```
````

**Python** (stdout only if you print or call code that prints):

````markdown
```python
print("Hello from Python")
```
````

PowerShell runs with `-ExecutionPolicy Bypass` for the generated temp `.ps1` so default machine policies are less likely to block one-off snippets.

## Shell interpreter dispatch

`run_shell` in [`code_execution.rs`](../../../src/markdown/code_execution.rs) builds an ordered **`ShellDispatch`** chain per fence language. Each candidate writes a temp script with the matching suffix (`.sh`, `.ps1`, or `.bat`) and spawns only interpreters compatible with that script type — POSIX fences never fall back to PowerShell or CMD on Windows.

| Fence language | Windows chain (in order) | Unix chain (in order) | Temp suffix |
|----------------|--------------------------|------------------------|-------------|
| `bash`, `shell` | `bash` → Git Bash (`Program Files\Git\bin\bash.exe`, `...\usr\bin\bash.exe`) → `wsl.exe bash` | `bash` → `sh` | `.sh` |
| `sh` | `sh` → same POSIX/bash chain as above | `sh` | `.sh` |
| `zsh` | `zsh` → same POSIX/bash chain as above | `zsh` → `sh` | `.sh` |
| `pwsh`, `powershell`, `ps1` | `pwsh` → `powershell` | same | `.ps1` |
| `cmd`, `bat`, `batch` | `cmd` | same | `.bat` |

**Windows POSIX gotcha:** If no `bash`, Git Bash, or WSL is available, POSIX fences fail with an actionable i18n error (`run_posix_shell_missing_windows`) — bash source is **not** written to a `.ps1` file. Use `powershell` / `pwsh` / `cmd` fences on plain Windows, or install Git Bash or WSL.

**Unix fallback:** `zsh` and `bash`/`shell` fall back to `sh` when the requested interpreter is absent. `sh` tries only `sh`.

Spawn helpers: `shell_dispatch_chain` / `shell_dispatch_chain_for` (testable), `configure_shell_command`, `shell_dispatch_exhausted_error`. Error strings: `locales/en.yaml` → `widgets.code_block.run_interpreter_*`, `run_shell_missing_*`, `run_posix_shell_missing_windows`.

## Inline output panel

Each Run launches a background worker via [`spawn_run`](../../../src/markdown/code_execution.rs); the UI thread polls a shared [`RunHandle`] (`Arc<Mutex<RunState>>`) once per frame. The widget renders a transient output panel directly below the code block with:

- **Status header:** rotating Braille spinner + "Running" while live, then ✓ "Exit 0", ✗ "Exit N", "Timed out after Ns", "Stopped by user", or "Failed".
- **Elapsed time** (`123ms` / `4.2s` / `1m 12s`).
- **Stdout** and (if non-empty) a separated **stderr** section.
- **ANSI colors** parsed via [`ansi_render`](../../../src/markdown/ansi_render.rs), which wraps `vte::Parser` with a small `Perform` adapter and reuses [`crate::terminal::AnsiColor`] / `TerminalTheme::ferrite_dark|light` so colors match the integrated terminal.
- **Windows line endings:** Many CLIs (including Python’s `print` on Windows) emit **CRLF** (`\r\n`). The parser treats a lone **carriage return** as “clear this line” (for progress-style `\r` overwrites). Before parsing, [`ansi_render::parse`](../../../src/markdown/ansi_render.rs) therefore **normalizes `\r\n` → `\n`**, so the panel shows the same text as **Copy** / raw bytes. Standalone `\r` (not followed by `\n`) is unchanged.
- **Live action:** **Stop** (kills the running child via the cancellation token; the slot becomes Dismiss once the run is no longer `Running`).
- **While running with no bytes yet:** italic *Waiting for output…* placeholder (`widgets.code_block.run_waiting_for_output`) instead of a blank scroll area.
- **Post-run actions:** **Copy** (clipboard, ANSI-stripped), **Insert as block** (appends a fenced ` ```output ` block right after the source), **Dismiss** (clears the panel state).

When `code_execution_show_inline_output` is **off**, the panel is hidden and completion (including timeouts and user cancellations) falls back to the legacy toast notification (one-shot per run, routed through `format_completion_toast`).

## Run state keying & plain-text export

Per-block run handles live in egui temp storage under keys derived from [`code_run_state_key`](../../../src/markdown/code_execution.rs) — **blake3** hash of `language + "\n" + code`, not the block’s source line number. Inserting or deleting lines **above** a fence therefore does **not** orphan in-flight or completed inline output. Editing the fence body or language intentionally starts a fresh key (prior panel state is not carried over).

| Suffix | Purpose |
|--------|---------|
| *(base hash id)* | Stable identity for the block’s current source |
| `.with("run_handle")` | Live [`RunHandle`] while a worker is attached |
| `.with("run_toast_emitted")` | One-shot toast guard when inline output is disabled |

The consent-dialog path stores the same base key on [`PendingCodeRun::run_state_key`](../../../src/state.rs) so a deferred Run after **Enable & Run** attaches to the correct block.

**Copy** and **Insert as block** both call [`format_run_output_plain`](../../../src/markdown/code_execution.rs): stdout first (ANSI stripped), then stderr under the same `── stderr ──` heading used in the on-screen panel (`widgets.code_block.run_stderr_heading`). Toasts use the same helper via `format_completion_toast`.

## Threading & cancellation model

`spawn_run` spawns one worker thread per Run. The worker:

1. Writes a temp script (`ferrite_code_<nanos>.sh|.ps1|.bat|.py`) into `std::env::temp_dir`, with cross-platform interpreter selection.
2. Spawns the child process with `Stdio::piped()` for stdout/stderr; on Windows we set `CREATE_NO_WINDOW` to avoid a console flash.
3. Spawns dedicated **reader threads** for stdout and stderr so blocking `read()` never starves the timeout/`try_wait` loop. Each reader pushes raw bytes into the shared `RunState`. Killing the child closes the pipes, which lets `read()` return `Ok(0)` and the readers join cleanly — no zombie threads.
4. **Polls the cancellation token** (`RunState.cancel: Arc<AtomicBool>`) every ~20 ms inside `wait_child`. The worker holds its own clone of the `Arc<AtomicBool>` so it never has to lock the outer `Mutex<RunState>` to check the flag. When the flag flips it kills the child, joins reader threads, and returns `RunError::Cancelled`.
5. On exit (or timeout, or cancellation), records `RunStatus::Completed { exit_code }` / `TimedOut` / `Cancelled` / `Failed { message }` and calls `egui_ctx.request_repaint()` so the UI reflects the final state immediately.

The cancellation token is exposed to the UI through the small `code_execution::cancel(&RunHandle)` helper. The UI thread calls it from the **Stop** button handler in `EditableCodeBlock::show`. The button is disabled (`add_enabled(false, …)`) once `cancel_requested` is true, so a double-click cannot enqueue a second cancellation. The repaint cadence stays at `request_repaint_after(80 ms)` while a run is in progress, which keeps the spinner rotating and the elapsed-time label fresh.

`RunState.timeout_secs` is captured at spawn time and surfaced in `RunSnapshot` so the panel can render `Timed out after Ns` without re-reading settings. Toast fallback uses the same value via `format_completion_toast`.

ANSI parsing happens in the UI layer (`ansi_render::parse`, including CRLF normalization — see `normalize_crlf` in [`ansi_render.rs`](../../../src/markdown/ansi_render.rs)) so the worker stays focused on transport. This keeps the rendered output consistent with whatever theme is active and avoids duplicating SGR handling already covered by `terminal/handler.rs`.

## Code map

| Area | Location |
|------|----------|
| Runner, gating helpers, `code_run_state_key`, `format_run_output_plain`, `spawn_run`, `cancel`, `RunHandle`, `RunStatus`, cancel token | `src/markdown/code_execution.rs` |
| ANSI parser + renderer (`AnsiLine`, `AnsiSegment`, `render_lines`; `parse` normalizes CRLF) | `src/markdown/ansi_render.rs` |
| Run button, Stop button + inline output panel | `src/markdown/widgets.rs` — `EditableCodeBlock::show`, `render_run_output_panel`, `run_status_label`, `running_spinner_frame` |
| Insert-as-fenced-block handler | `src/markdown/editor.rs` — `render_code_block`, `insert_output_block_after` |
| Settings snapshot into preview | `src/markdown/editor.rs` — `MarkdownEditor::show_rendered_editor` (`code_execution_ctx_id`) |
| Build `CodeExecutionUi` + cwd | `src/app/central_panel.rs` — `CodeExecutionUi::from_settings_with_workdir` |
| Consent → deferred Run (same run-state key) | `src/app/dialogs.rs` — `PendingCodeRun.run_state_key` |
| Toast drain (fallback path) | `src/app/mod.rs` (after `render_ui`), `drain_code_execution_toasts` |
| Strings | `locales/en.yaml` — `widgets.code_block.run_*` (incl. `run_stop`, `run_status_cancelled`, `run_interpreter_*`, `run_shell_missing_*`, parameterised `run_status_timed_out`), `settings.editor.code_execution_*` |
| Shell interpreter dispatch | `src/markdown/code_execution.rs` — `ShellDispatch`, `shell_dispatch_chain`, `windows_posix_bash_chain`, `configure_shell_command` |

## Known limitations

Manual regression on Windows passed using [`test_md/test_code_execution.md`](../../../test_md/test_code_execution.md). Remaining edge cases — see [ROADMAP.md](../../../ROADMAP.md) (*Executable code blocks — hardening*).

| Limitation | Impact | Workaround |
|------------|--------|------------|
| **Run state resets on fence edit** | Changing the fenced source or language starts a new blake3 key; prior inline output is not shown for the edited block. | Expected — **Run** again after editing the snippet. |
| **Structural edits above the fence** | Line shifts no longer orphan output (hash keying), but unrelated blocks with identical source+language share the same key. | Rare in practice; edit one character or **Dismiss** if panels collide. |

## Validation

- `cargo test --bin ferrite markdown::ansi_render` covers the SGR parser (plain text, basic colors, 256-color, truecolor, bold/reset, carriage return rewrite, **CRLF vs bare `\r`**, empty input, trailing newline).
- `cargo test --bin ferrite markdown::code_execution` covers language classification (incl. `pwsh`), Run-button visibility flags, status glyph mapping (incl. `Cancelled`), `RunState.timeout_secs` capture, the idempotent `cancel(&RunHandle)` helper, shell interpreter dispatch (Windows POSIX chain excludes PowerShell/cmd; Unix `zsh`/`bash` → `sh` fallback; mocked availability filtering), **`code_run_state_key` stability** (same source → same key regardless of line position; content/language changes → new key), and **`format_run_output_plain` stderr heading** parity with the inline panel.
- Manual: full checklist in [`test_md/test_code_execution.md`](../../../test_md/test_code_execution.md). Spot checks:
  - Enable Settings → Editor → Code execution; open a markdown file containing shell or python fences; click Run and verify the inline panel reports stdout/stderr with colors and an accurate exit indicator.
  - Long-running snippet (`sleep 60`, `while True: pass`): click **Stop** and confirm the panel transitions to `Stopped by user` within ~100 ms, the spinner stops rotating, and the UI scrolls/interacts normally throughout.
  - Lower the timeout (e.g. 5s), run an infinite loop, and confirm the panel reads `Timed out after 5s` once the worker reaps the child.
  - Toggle `code_execution_show_inline_output` off and repeat both flows; the toast must read `Run failed: Stopped by user` / `Run failed: Timed out after Ns`.
