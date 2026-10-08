# Line Ending Preservation (#174)

Per-tab line-ending metadata keeps Windows CRLF (and Unix LF) files from being normalized to LF on open, preview edit, or save.

## `LineEnding` enum

Defined in `src/state.rs`:

| Variant | Bytes | Use |
|---------|-------|-----|
| `Lf` | `\n` | Unix / default on non-Windows |
| `Crlf` | `\r\n` | Windows convention |

- **`platform_default()`** — CRLF on Windows (`cfg!(windows)`), LF elsewhere. Used for new/empty tabs and files with no line breaks.
- **`as_str()`** — `"\n"` or `"\r\n"` for rejoin.
- **`byte_len()`** — `1` (LF) or `2` (CRLF); used when computing cursor byte offsets after line ops.
- **`join_lines(lines)`** — Shared rejoin helper; line bodies must not include endings.
- **`split_lines(content)`** — Split into bodies **without** endings. Unlike `str::lines()`, keeps a final empty segment when content ends with an EOL so round-trip rejoin preserves a trailing newline.

## Detection policy

`LineEnding::detect_from_content` (preferred) / `detect_from_bytes` (low-level byte scan):

1. Empty input → `platform_default()`.
2. No `\n` or `\r\n` sequences → `platform_default()`.
3. Otherwise count standalone `\n` vs `\r\n` pairs (lone `\r` is skipped, not counted).
4. **Majority wins**; on a tie, the **first** line ending in the file wins.

Mixed-EOL files pick a single dominant ending; Ferrite does **not** invent a perfect mixed-EOL rewrite. Line ops in `src/app/line_ops.rs` rejoin the whole buffer with the tab’s stored ending — they normalize mixed files to that dominant ending rather than preserving per-line style.

### UTF-16 and other encodings

**Always detect from decoded text**, not raw file bytes. UTF-16LE CRLF is stored as `0D 00 0A 00` in the file; scanning raw bytes mis-counts bare `\n` (`0A 00`) and sets `Lf`, so Enter/line ops insert LF and save produces mixed EOLs.

All byte-based load paths decode first (BOM + chardetng), then call `detect_from_content(&content)`.

## Where `Tab.line_ending` is set

Field on `Tab` in `src/state.rs` (`pub line_ending: LineEnding`).

| Path | Detection source |
|------|------------------|
| `Tab::new()` | `platform_default()` |
| `Tab::with_file()` | `detect_from_content` |
| `Tab::with_file_bytes()` | `detect_from_content` (after decode) |
| `Tab::from_tab_info()` | `detect_from_content` |
| `Tab::from_tab_info_with_bytes()` | `detect_from_content` (after decode) |
| `Tab::finish_loading()` | `detect_from_content` (after decode) |
| `Tab::apply_external_disk_reload()` | `detect_from_content` (after decode) |
| Session recovery (`ResolvedContent::*`) | `detect_from_content` on restored buffer |

File open (`AppState::open_file_with_focus`) reads raw bytes via `std::fs::read` → `Tab::with_file_bytes_and_settings` — **no** `str::lines()` + `join("\n")` normalization on load.

External watcher reload in `src/app/file_ops.rs` calls `apply_external_disk_reload(bytes)`.

## Rewrite / rejoin paths

Buffer mutators that rebuild the document from lines must rejoin with the stored (or detected) ending:

| Area | Pattern |
|------|---------|
| `src/app/line_ops.rs` | Prefer **`tab.line_ending.join_lines`** (duplicate / move / delete line) |
| `src/markdown/editor.rs` | `detect_from_content(source)` + `split_lines` / `join_lines` (`update_source_range`, `update_code_block`, tables, links, …) |
| `src/markdown/formatting.rs` | `detect_from_content` + `join_lines`; `selected_lines_byte_range` excludes CRLF’s `\r` so splices keep `\r\n` in the suffix |
| `src/markdown/ast_ops.rs` | `rejoin_lines` helper → `detect_from_content` + `join_lines` for structural WYSIWYG edits |

Do **not** use `str::lines()` + `join("\n")` when writing back an open markdown buffer.

## Save path

`Tab::encode_content()` encodes `tab.content` bytes for the tab’s encoding (UTF-8 / UTF-16, BOM). `AppState::save_tab_by_id` / `save_active_tab_as` write those bytes with `std::fs::write` — **no** EOL normalization on save.

## Dirty-state guarantee

Opening a CRLF file without editing must not mark the tab modified solely from EOL normalization. Preview edits that only rejoin with the same ending must not flip the whole file to LF.

## Regression tests

| Location | Tests |
|----------|--------|
| `src/state.rs` | detect/majority/platform default; `join_lines` / `split_lines` round-trip; open CRLF not dirty; encode/save CRLF+LF; UTF-16LE CRLF on `with_file_bytes`, `from_tab_info_with_bytes`, `finish_loading` |
| `src/markdown/editor.rs` | `update_source_range` / `update_code_block` preserve CRLF and trailing EOL |
| `src/markdown/formatting.rs` | multiline list format preserves CRLF/LF; mid-document splice keeps surrounding CRLF |
| `src/markdown/ast_ops.rs` | indent / split list preserve CRLF/LF |

## Out of scope

Perfect mixed-EOL rewriting (per-line original endings); remote images; unrelated issues.

## Related

- Tab model: [`tab-system.md`](tab-system.md)
- Session restore: [`session-persistence.md`](session-persistence.md)
