# Quick note workflow (ephemeral untitled tabs)

Default-on editor mode (`Settings.quick_note_workflow`, **Settings → Files**): reduces friction for pathless “scratch” tabs. Turn off in settings to restore the classic save confirmation for new tabs.

## Behavior

- **Quit:** Modified untitled tabs (no file path) do not block exit. Saved files on disk still use the normal prompt when modified.
- **Close tab:** Modified untitled tabs with content still show the unsaved-changes dialog (Save / Don't save / Cancel). Empty untitled tabs close silently.
- **Close window (app keeps running):** Same rule as closing a tab. The window's tabs are purged and nothing restores them later, so a modified untitled tab with content prompts (custom ×, native ×, Alt+F4). Only closing the **last** window is an app exit.
- **Persistence:** Unsaved text is **not** written to a normal file path; it is still captured by the existing session pipeline (`session.json`, `session.recovery.json`, and per-tab files under the config dir `recovery/`). Requires **Restore previous session on startup** so the next launch reloads those buffers.
- **Display names:** `Tab.untitled_display_name` holds an optional label. The tab strip shows it instead of “Untitled”. **Double-click** an untitled document tab to open the rename dialog. The name is stored in session metadata (`SessionTabState.display_title` without a `*` suffix).
- **Save As:** Assigning a path clears `untitled_display_name` on save (real file name comes from disk).

## Code touchpoints

- `Tab::should_prompt_to_save(&Settings, SavePromptContext)` — returns `false` for pathless tabs when `quick_note_workflow` is true **only** for `SavePromptContext::AppExit`.
- `AppState::close_tab()` — uses `SavePromptContext::TabClose` so scratch tabs with content still prompt.
- `AppState::has_unsaved_changes()` — uses `AppExit` so `request_exit()` does not block on scratch tabs.
- `AppState::window_has_unsaved_changes()` / `request_close_window()` — use `TabClose` (QA SES-3); `unsaved_changes_prompt` takes the context so the "notes kept for the next launch" line only appears in the exit dialog.
- `FerriteApp::handle_save_all_modified_tabs(ctx, window_scope)` — `TabClose` when scoped to a window, `AppExit` for the exit dialog.
- `AppState::resolve_tab_content` — pathless tabs with `has_unsaved_content == false` restore as empty buffers (supports multiple empty untitled tabs in session).
- `AppState::capture_session_state` — uses `Tab::persisted_session_display_title()` for stable session titles (no trailing `*`).

## Caveats

- This is the same in **dev** (`cargo run`) and **installed** builds: persistence uses your Ferrite [config directory](./config-persistence.md), not the project folder.
- On **clean exit**, the app saves `session.json` **and** per-tab bodies under `recovery/`. Older versions deleted `recovery/` on exit, which broke restoring pathless tabs; that is fixed by keeping recovery content and only removing the crash snapshot file (`session.recovery.json`).
- Closing a modified untitled tab via **Don't save** discards that buffer from the open session; **Save** runs Save As. Other open scratch tabs remain in the session for the next run.
- Turning **Quick note workflow** off restores the original save prompts for new tabs; existing session data is unchanged.
