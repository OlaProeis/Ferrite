# Ferrite v0.3.1 QA - Batch 8 (Spellcheck + fonts)

Tester: Grok Bot. Binary: `/workspace/ferrite-qa/ferrite/target/release/ferrite`. Launcher: `/workspace/ferrite-qa/run-ferrite.sh --tag <ROW> <file-or-folder>`. Screenshots: `shots/`. Logs: `logs/`.
Pre-run config: `spellcheck_enabled: false`, `rendered_font_family: null`, `terminal_font_family: null`.
Log-to-row map: SPL-1 instance `ferrite-debug-20261009-020610-SPL-1.log` (02:06-02:17) covered SPL-1 toggle, SPL-2, SPL-3, SPL-5, SPL-6, SPL-7. Instance `ferrite-debug-20261009-021724-SPL-4.log` (02:17-02:24) covered SPL-4 relaunch, SPL-8, SPL-1 re-check, FNT-1, FNT-2 (set). Instance `ferrite-debug-20261009-022442-FNT-2.log` (02:24-02:30) covered FNT-2 persistence, FNT-3, FNT-4.

| ROW-ID | RESULT | observation | log | screenshot |
|---|---|---|---|---|
| SPL-1 | PASS | Default is OFF: pre-run config `spellcheck_enabled: false`; Settings > Editor > Spellcheck was unchecked. Turned ON (Settings > Editor > Spellcheck). The transient 'Loading dictionary...' status text was NOT seen: the first pass did not record it, and a re-check (screenshot ~0.5 s after the toggle) showed no such text (dictionary had already loaded this session, so it may not reload). Noted, not verified. | logs/ferrite-debug-20261009-020610-SPL-1.log (toggle); re-check in logs/ferrite-debug-20261009-021724-SPL-4.log | none for toggle; shots/SPL-1-*: none (no loading text captured) |
| SPL-2 | FAIL | Pasted `This is a speling mistaek in Ferrite.` into a new Untitled doc (Raw). `speling` squiggled (hint tooltip "[hint/spell] Unknown word: speling"). `mistaek` NOT squiggled (zoomed check; it was not in user-words.txt yet). `This` and `is` not squiggled. `Ferrite` not clearly squiggled in this shot (tooltip covers the line). | logs/ferrite-debug-20261009-020610-SPL-1.log | shots/SPL-2-fail.png (copy of SPL-2-pass.png, misnamed at capture) |
| SPL-3 | BLOCKED | Right-click `speling` -> suggestion list did not give a usable `spelling` entry: the "[hint/spell] Unknown word" tooltip overlapped the context menu. Clicks on the item either fell through (caret placed, tooltip shown) or closed the menu; one stray click ran a context-menu Paste of clipboard text into the doc (undone with Ctrl+Z). Down-arrow + Enter did not select. ~4 attempts, not reliably reachable. Replacement and the Ctrl+Z restore were NOT verified. | logs/ferrite-debug-20261009-020610-SPL-1.log | shots/SPL-3-check.png (02:07:23) |
| SPL-4 | PARTIAL | `mistaek` was written to `~/.config/ferrite/spellcheck/user-words.txt` at 02:07:22, during SPL-3 troubleshooting. I did not deliberately choose "Add word" and could not pinpoint the click, so the "squiggle disappears immediately on Add word" step is not verified as a deliberate action. Persistence check: after Alt+F4 (Don't Save) and relaunch (instance SPL-4), pasting the sentence into a new doc: `speling` squiggled, `Ferrite` squiggled, `mistaek` NOT squiggled. `cat ~/.config/ferrite/spellcheck/user-words.txt` = `mistaek` (one line). File left as is. | logs/ferrite-debug-20261009-021724-SPL-4.log | shots/SPL-4-pass-relaunch.png |
| SPL-5 | FAIL | New doc with separate lines: fenced block (```` ``` ````, `speling inside fence`), `` `speling` inline code ``, `https://exmaple.com/pth`, `[ok](http://speling.com)`, `camelCaseWord`, `ALLCAPS`, `abc123`. No squiggle on the fenced `speling`, inline-code `speling`, the URL, `camelCaseWord`, `ALLCAPS`, `abc123`. BUT `ok` in the link text `[ok](...)` IS squiggled in Raw (zoomed). Expected none. | logs/ferrite-debug-20261009-020610-SPL-1.log | shots/SPL-5-fail.png (copy of SPL-5-pass.png, misnamed at capture) |
| SPL-6 | PASS | Same doc, Split (Ctrl+E cycled to Split; no clicks into text). Rendered/preview pane: no squiggles. Inline code `speling` plain, URL plain, `ok` is a blue underlined link (not wavy), camelCaseWord/ALLCAPS/abc123 plain. Rendered-only view also checked (shot). Raw pane still shows the `ok` squiggle, which is the SPL-5 issue. | logs/ferrite-debug-20261009-020610-SPL-1.log | shots/SPL-6-rendered-pass.png, shots/SPL-6-split-pass.png |
| SPL-7 | PARTIAL | `test_scroll_perf.md` in Raw, mouse-wheel scrolled (before shot + 2 wheel steps, lines ~340-370). No freeze and no visible stutter. Smoothness is only an impression; this software-rendered box does not measure it. Squiggles appear only in the visible viewport. BUT squiggles are drawn at the far right edge of the text column (about one wrap-width away from the word, e.g. `viewport`), not under the misspelled word. Misplaced squiggles = fail on placement. | logs/ferrite-debug-20261009-020610-SPL-1.log | shots/SPL-7-before-scroll.png, shots/SPL-7-scroll1.png, shots/SPL-7-scroll2.png |
| SPL-8 | PASS | Spellcheck turned OFF: squiggles on `This is a speling mistaek in Ferrite.` gone at once, no crash/hang. After relaunch, Settings > Recently Changed shows Spellcheck unchecked. | logs/ferrite-debug-20261009-021724-SPL-4.log | shots/SPL-8-pass-off.png |
| FNT-1 | PASS | Default Rendered view font = "Same as editor" (checkbox ticked). Unticked and chose built-in `JetBrains Mono`: Rendered view of `fixtures/t1-multibyte.md` changed from proportional (Inter) to monospace. Raw view of the same doc: same proportional font, same glyphs and positions (visual compare; the before-shot had a Settings-panel transition overlay, so a raw pixel diff is not clean). | logs/ferrite-debug-20261009-021724-SPL-4.log | shots/FNT-1-settings-untick.png, FNT-1-settings-jbmono.png, FNT-1-rendered-before.png, FNT-1-rendered-after-jbmono.png, FNT-1-raw-before.png, FNT-1-raw-after.png |
| FNT-2 | PASS | Rendered font -> Custom System Font -> `DejaVu Serif`: rendered pane switched to serif immediately. Quit (Alt+F4, exit 0) and relaunch: Settings still shows Custom System Font = DejaVu Serif, rendered pane still serif; config `rendered_font_family: {"custom": "DejaVu Serif"}`. Then set back to "Same as editor" (config `rendered_font_family: null` after quit). | logs/ferrite-debug-20261009-021724-SPL-4.log (set), logs/ferrite-debug-20261009-022442-FNT-2.log (persisted) | shots/FNT-2-dejavu-serif-set.png, FNT-2-rendered-dejavu-serif.png |
| FNT-3 | PASS | Terminal Font default = "Default (JetBrains Mono)" (recorded). Changed to Custom System Font -> `DejaVu Sans Mono`. Terminal panel (Ctrl+`) drew in DejaVu Sans Mono: `ls -la` columns aligned, `echo gjpqy` shows g j p q y with full descenders (no clipping). Side-by-side with JetBrains Mono restored: glyph shapes differ subtly (DejaVu heavier/wider) at 14px. Set back to Default (JetBrains Mono); config `terminal_font_family: null`. | logs/ferrite-debug-20261009-022442-FNT-2.log | shots/FNT-3-dejavu-sans-mono-set.png, FNT-3-dejavu-sans-mono-terminal.png, FNT-3-jbmono-terminal.png |
| FNT-4 | PARTIAL | Exact command `printf '\ue0b0 \uf015 \ue62b\n'` printed the literal text `\ue0b0 \uf015 \ue62b`, so no PUA codepoints were emitted. Cause: the Ferrite terminal's shell is `/bin/sh` (dash, `echo $0` = `/bin/sh`); dash printf does not expand `\u`/`\x` escapes (the same command works in plain bash in the Shell tool). Equivalent emitters that did run: dash-compatible octal `printf '\356\202\260 \357\200\225 \356\230\253\n'` and `python3 -c "print('\ue0b0 \uf015 \ue62b')"`. Output: a powerline arrow glyph, a tofu box (for \uf015), and an icon glyph (\ue62b). No toast, no crash, no panic. The log shows repeated non-fatal WARN `epaint::text::fonts: Failed to find replacement characters '◻' or '?'. Will use empty glyph.` Nerd Font install: SKIPPED (optional, not installed). | logs/ferrite-debug-20261009-022442-FNT-2.log | shots/FNT-4-nerdfont-pua.png, FNT-4-nerdfont-octal.png, FNT-4-nerdfont-python.png |

## Notes
- Rows not reached: none. All 12 rows (SPL-1..SPL-8, FNT-1..FNT-4) were run.
- Unintended side effect (SPL-3): a stray context-menu Paste inserted clipboard text into an Untitled doc (undone with Ctrl+Z). The `mistaek` dictionary entry (SPL-4) was also created unintentionally at 02:07:22. Left as is, per instructions.
- The untitled docs with content were not saved; Ferrite exited cleanly (code 0) with recovery data kept.

## FAIL log evidence (last 30 non-span lines of logs/ferrite-debug-20261009-020610-SPL-1.log; covers SPL-2 and SPL-5)

```
[2026-10-09T00:17:16Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-09T00:17:16Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-09T00:17:16Z INFO  ferrite::state] Settings saved
[2026-10-09T00:17:16Z INFO  ferrite::state] AppState shutdown complete
[2026-10-09T00:17:16Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T00:17:16Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T00:17:16Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1202.4, max_wrap_width=840.0, effective=840.0
[2026-10-09T00:17:16Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-10-09T00:17:16Z INFO  ferrite::app] Application exiting
[2026-10-09T00:17:16Z DEBUG ferrite::config::session] Saved recovery content for tab 57 (path=None, hash=None)
[2026-10-09T00:17:16Z DEBUG ferrite::config::session] Saved recovery content for tab 58 (path=None, hash=None)
[2026-10-09T00:17:16Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-09T00:17:16Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=31, clean_shutdown=true
[2026-10-09T00:17:16Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-10-09T00:17:16Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-10-09T00:17:16Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (1 windows, 31 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-09T00:17:16Z INFO  ferrite::app] Session state saved for next startup
[2026-10-09T00:17:16Z DEBUG ferrite::config::session] Removed session lock file
[2026-10-09T00:17:16Z DEBUG ferrite::workspaces::persistence] Saved workspace state to "/workspace/ferrite-qa/ferrite/.ferrite/state.json" (3 recent files, 2 expanded paths)
[2026-10-09T00:17:16Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-09T00:17:16Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-09T00:17:16Z INFO  ferrite::state] Settings saved
[2026-10-09T00:17:16Z INFO  ferrite::state] AppState shutdown complete
[2026-10-09T00:17:16Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-10-09T00:17:16Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-10-09T00:17:16Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-10-09T00:17:16Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T00:17:16Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-10-09T00:17:16Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 02:17:16 CEST ===
```

## End of batch 8 state
- Spellcheck: OFF (config `spellcheck_enabled: false`; Settings checkbox unchecked).
- Rendered view font: "Same as editor" (config `rendered_font_family: null`).
- Terminal font: "Default (JetBrains Mono)" (config `terminal_font_family: null`).
- Ferrite quit with Alt+F4: `=== Ferrite exited with code 0 clean exit at 02:30:24 CEST ===`. `pgrep -a -x ferrite` is empty.

`git -C /workspace/ferrite-qa/ferrite status --short` (after batch 8):

```
```
