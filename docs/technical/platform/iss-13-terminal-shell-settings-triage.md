# ISS-13 — Terminal shell selector vs Settings (triage)

Linux QA (Grok Bot) filed **ISS-13** because minimal-release checklist row **29** listed **“terminal shell”** among 0.3.0 Settings controls. Settings search for `shell` only hits code-execution allowlist and “Enable Integrated Terminal”; there is no default-shell picker in the settings registry.

## Outcome

**Not a product gap for v0.3.1.** `prd-v0.3.0` never required a shell selector in Settings. It covers:

- Integrated terminal regression (TRM-* in [`v0.3.0-regression-matrix.md`](./v0.3.0-regression-matrix.md))
- Code-block Run spawning the **platform default** shell (§5.9 in [`prd-v0.3.0.md`](../../ai-workflow/prds/prd-v0.3.0.md))

Shell **type** for new integrated terminals is chosen in the **terminal panel UI**, not Settings:

- Windows: **+** (default PowerShell) and right-click menu → PowerShell, CMD, WSL (`src/ui/terminal_panel.rs`, `ShellType` in `src/terminal/pty.rs`)
- Unix: `$SHELL` / login shell via `ShellType::Default`

A future “default shell in Settings” would be a separate feature; it is out of scope for the v0.3.1 QA-closeout PRD.

## Checklist fix

Row **29** in [`v0.3.1-minimal-release-test.md`](./v0.3.1-minimal-release-test.md) now expects **integrated terminal settings** (enable, font, startup command, etc.) and explicitly excludes a Settings shell picker.

## QA report

[`grokbot/New/qa-report-v0.3.1-linux.md`](../../../grokbot/New/qa-report-v0.3.1-linux.md): row 29 marked **PASS (ISS-13 triaged)**; ISS-13 severity **triaged — not a defect (stale checklist)**.
