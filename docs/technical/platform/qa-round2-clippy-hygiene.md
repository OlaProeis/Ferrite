# QA round 2 — Clippy hygiene (OpenOptions + egui tests)

Small fixes from GrokBot round 2 close-out (PRD §5.6 observations): satisfy `clippy::suspicious_open_options` on the single-instance lock file and stop using deprecated `egui::Context::run` in unit tests.

## Single-instance lock `OpenOptions`

File: `src/single_instance.rs`.

Clippy requires an explicit `.truncate(..)` whenever `create(true)` is used (or when opening without create, to document intent).

| Call site | Options | Rationale |
|-----------|---------|-----------|
| `try_acquire_instance`, `become_primary_after_stale` | `.read(true).write(true).create(true).truncate(true)` | Fresh lock file when becoming primary; matches `test_try_lock_exclusive_blocks_second_handle` first handle. |
| Test second handle | `.read(true).write(true).truncate(false)` | Opens the existing lock path without truncating while the first handle holds the lock. |

Lock protocol overview: [single-instance.md](./single-instance.md).

## egui unit tests: `run_ui` instead of `run`

Deprecated API: `ctx.run(RawInput, |ctx| { ... })`.

Replacement: `let _ = ctx.run_ui(raw, |ctx| { ... })` with the same closure body and assertions.

Migrated tests (task 33):

| Module | Test |
|--------|------|
| `src/markdown/widgets.rs` | `test_table_force_commit_take_is_one_shot` |
| `src/markdown/editor.rs` | `remember_arrow_nav_sel_does_not_reenter_context_lock`, `rendered_line_mappings_sorted_when_culled_past_first_screen` |

Shared pattern elsewhere: `src/markdown/mermaid/measure.rs` (`run_ui` helper + `CentralPanel::show_inside`). New tests should prefer `show_inside` over deprecated `CentralPanel::show`; migrating those call sites is separate from the `Context::run` removal.

## Verification

```bash
cargo clippy --bin ferrite   # no suspicious_open_options; no deprecated Context::run in touched tests
cargo test --bin ferrite single_instance
cargo test --bin ferrite widgets::
```
