# Session Handover

## Environment

- **Project:** Ferrite (markdown editor, Rust + egui)
- **Tech Stack:** Rust 2021 (toolchain **1.92** via `rust-toolchain.toml`), **egui/eframe 0.34.2** (glow backend on Windows)
- **Context file:** Always read [`docs/ai-context.md`](./ai-context.md) first — project rules, architecture, conventions.
- **Branch:** v0.3.1 work on `master` tag tasks

## Handover Rules

- **SCOPE:** Implement **task 15 only** — CSV rendered editing MVP.
- Run `cargo check` / `cargo build` after changes.
- Full `cargo test` may fail due to pre-existing unrelated test compile errors on this branch — fix only if your changes touch those modules.
- Document behaviour changes in [`docs/technical/viewers/csv-viewer.md`](./technical/viewers/csv-viewer.md) when done (task requirement).

---

## Current Task: 15 — Implement CSV rendered editing MVP with size gating and undo support

**Priority:** high | **Complexity:** 7 | **Dependencies:** none | **Status:** pending

**Goal:** Enable inline cell editing in the CSV table (Rendered view). Double-click enters edit mode; Enter commits, Escape cancels. Commits update `tab.content` via `csv::Writer`, integrate undo via `prepare_undo_snapshot_hashed`, and mark the tab dirty. Files **≥ 1 MB** show a banner and disable rendered editing (Raw view only).

### Requirements

1. **Edit mode** — Double-click cell → egui text field overlay; Enter commits, Escape cancels.
2. **Commit path** — Update in-memory model; serialize back to `tab.content` with `csv::Writer` (correct quoting/escaping); call `Tab::prepare_undo_snapshot_hashed()` before mutation; mark tab modified.
3. **Size gating** — `< 1 MB`: editing enabled. `≥ 1 MB`: banner in viewer directing user to Raw view; no cell edits.
4. **Keyboard** — Basic arrow-key grid navigation when not in edit mode.
5. **Docs** — Extend `docs/technical/viewers/csv-viewer.md` with rendered-editing section.

### Suggested subtask breakdown (optional expand)

| # | Area |
|---|------|
| 1 | Cell edit mode UI (double-click, text field, Enter/Escape) |
| 2 | In-memory cell update + `csv::Writer` → `tab.content` |
| 3 | Undo via `prepare_undo_snapshot_hashed` + dirty flag |
| 4 | 1 MB size gate + banner UX |
| 5 | Arrow-key navigation in grid |
| 6 | Tests + manual checks (special chars, undo/redo, save/reload) |

### Pseudo-code (commit)

```rust
fn commit_cell_edit(tab: &mut Tab, model: &mut CsvModel, row: usize, col: usize, new_value: String) {
    tab.prepare_undo_snapshot_hashed();
    model.set_cell(row, col, new_value);
    let mut writer = csv::Writer::from_writer(Vec::new());
    for r in model.rows() {
        writer.write_record(r)?;
    }
    tab.content = String::from_utf8(writer.into_inner()?)?;
    tab.record_edit_from_snapshot(/* ... */);
}
```

### Test strategy (from task)

- Unit tests: `set_cell` + serialization round-trip; quoting preserved.
- Fixtures: ~0.5 MB (editable) vs ~2 MB (banner, no edit).
- Manual: double-click, Enter/Escape, undo/redo, save/reload, commas in quoted fields.

---

## Key Files (task 15)

| File | Purpose |
|------|---------|
| `src/markdown/csv_viewer.rs` | `CsvViewer`, `CsvViewerState`, table rendering, `LARGE_FILE_THRESHOLD` (1 MB), lazy vs full parse |
| `src/app/central_panel.rs` | Wires `CsvViewer::new(...).show(ui)` for tabular tabs (~lines 1845, 2152) |
| `src/state.rs` | `Tab::prepare_undo_snapshot_hashed()`, `record_edit_from_snapshot()`, `LARGE_FILE_THRESHOLD`, tab dirty/save |
| `docs/technical/viewers/csv-viewer.md` | Update with rendered-editing behaviour |

**Existing constants:** `csv_viewer.rs` already defines `LARGE_FILE_THRESHOLD: usize = 1_000_000` and branches on `is_large` for lazy parsing — reuse for edit gating.

**Note:** Central panel comments say CSV viewer is currently **read-only** — task 15 makes rendered table cells editable.

---

## Verification baseline

```powershell
cargo build
cargo check
# Targeted tests when harness compiles:
cargo test csv_viewer
```

- **Manual:** Open small CSV → double-click cell → edit → undo → save → reload. Open ≥1 MB CSV → confirm banner, no edit.

---

## Model Selection

| Complexity | Recommendation |
|------------|----------------|
| **7** | **Strong implementation model.** Touches viewer widget state, content serialization, undo integration, and size gating. Careful with `csv::Writer` quoting and invalidating `CsvViewerState` caches after edits. |

---

## Task Master

```bash
task-master show 15 --json
task-master set-status --id=15 --status=in-progress
# When complete:
task-master set-status --id=15 --status=done
```

Prefer **MCP tools** (`get_task`, `set_task_status`) when available.
