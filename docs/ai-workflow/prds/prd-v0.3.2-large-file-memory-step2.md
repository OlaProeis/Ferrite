# Ferrite — Large-file memory, step 2 (PRD)

> **Status:** Scheduled for v0.3.2. Not started.
> **Version target:** v0.3.2, after step 1. Do this before the FerriteEditor crate extraction freezes a two-buffer public API.
> **Consumers:** Human review, then a cyclopsctl task queue.
> **Depends on:** [`prd-v0.3.2-large-file-memory-step1.md`](prd-v0.3.2-large-file-memory-step1.md). Large raw undo must already record `EditOperation`s without a full-string diff.
> **Hard constraint:** The rope (`TextBuffer`) is the only resident copy of a document. A `String` the size of the file exists only for the duration of save, export, or an explicit full copy, then it is dropped.

## 1. Overview

Step 1 leaves two full copies: `Tab.content: String` and `FerriteEditor`'s rope. [`architecture.md`](../../technical/editor/architecture.md) has called that the wrong layout since the custom editor landed. The rope-only target in that doc is this PRD.

**Honest budget.** One ropey rope is a bit larger than the file, because nodes are ~1 KB and are not packed solid. For an 80 MB file the buffer should land around **100–130 MB**, on top of the idle process (about 50 MB working set at startup, less after Windows trims it). "+80 MB for an 80 MB file" is not the acceptance bar. Shipping the crate with both copies still resident would publish the wrong API.

### Why the rope, not the String

A single `String` would sit closer to 1× the file. Inserting one character at the start would memmove the whole buffer. The rope exists so an edit is O(log n) in the file size. Step 2 keeps the rope and deletes the long-lived string. It does not replace ropey.

### Why this shape

| Choice | Decision |
|--------|----------|
| Source of truth | `FerriteEditor`'s `TextBuffer` for any tab the raw editor has opened. |
| `Tab.content` | Gone as a long-lived field, or reduced to a cache that is **empty** whenever the rope is resident. Prefer removing the field over a cache that silently fills back up. |
| Who may allocate a full `String` | Save, export, "copy entire document", and tests. Each drops the string before returning. Per-frame UI reads slices. |
| Order | Step 1 first. This PRD does not re-design undo. |
| Crate extraction | The FerriteEditor crate's public buffer type is `TextBuffer`. It does not take or return a second owned document string on each frame. |

## 2. Goals

- Opening `test_md/very_large_test_data.csv` (86,169,227 bytes) in Raw increases commit size by about the rope only: roughly **+100–130 MB** over the idle process, not +160 MB and not +320 MB.
- After the first frame settles, no `String` or `Vec<u8>` the size of the file remains. The decode buffer and any `to_string()` used to build the rope are dropped.
- Save writes the same bytes as today (encoding, BOM, line endings preserved).
- Find, spellcheck, the CSV row index, outline, stats, and rendered markdown read ranges or lines from the rope. They do not keep a second full copy.
- Undo from step 1 still works, against the rope alone.
- Small files behave the same to the user. They just stop carrying a twin string.

## 3. Non-goals

- **A new text data structure.** Stay on ropey. No mmap, no gap buffer.
- **Virtual scrolling, galley cache, or uniform-height mode.** Those already bound the paint cost. Do not redo them.
- **Rendered WYSIWYG rewrite.** The true-WYSIWYG PRD is v0.4.0+. This PRD only stops that view from holding its own full `String` beside the rope.
- **Changing the 1 MB large-file feature cuts** (minimap off, auto-close off, fixed gutter width). Those stay.
- **Promising 1.00× RAM** in the README, changelog, or architecture doc.

## 4. Current state (after step 1)

Step 1's end state is the input here:

- Large raw edits are `EditOperation`s plus a ranged splice into `Tab.content`. There is no `pending_undo_snapshot` and no per-frame full-file hash on that path.
- `Tab.content` is still the string every non-editor feature reads: save (`encode_content`), find, search-in-files, spellcheck tokenization, CSV `build_csv_row_index`, markdown parse, outline, document stats, session/recovery snapshots, line-ending detect.
- The rope is rebuilt from that string on open (`Rope::from_str`) and on external resync (`set_content`).
- `FerriteEditor` for a tab stays allocated in `FerriteEditorStorage` when the user switches to Rendered, which is why Rendered did not free RAM.

Call sites that `.clone()` `tab.content` or take `&tab.content` as a full-document `&str` are the migration list. Step 1's PRD names the hot ones on the raw frame. This step has to cover the rest, including save and session.

## 5. Functional requirements

### 5.1 One allocation on open

**Problem.** `fs::read` produces a `Vec<u8>`, decode produces a `String`, `Rope::from_str` copies the string. Step 1 stopped the snapshot and the verify `to_string()`. The string is still kept.

**Behavior.**

- Decode straight into the rope, or decode to a `String`, build the rope, and drop the `String` before the first frame returns. For a large file the dropped decode buffer must not be stored in `original_bytes` (already true) or in `Tab.content`.
- `content_version` and the editor sync cache still move when the rope changes. They do not need a parallel string to do that.
- Line ending and encoding detection run on the decode buffer **before** it is dropped, and the results stay on the tab as they do now (`LineEnding`, `current_encoding`, `had_bom`).

### 5.2 Readers take rope ranges

**Problem.** Most features take `&String` or clone it because that was the easy handle.

**Behavior.**

- Add the smallest read API on `TextBuffer` the callers actually need: line slice, byte or char range, full iterator of lines, and a `with_text(|s: &str| ...)` that borrows a temporary string only when a caller truly needs a contiguous `&str` (CSV index build, markdown parse of a dirty document). `with_text` drops that temporary before it returns. It is not a cache field.
- Save and export use one `to_string()` (or an encode-from-rope path), write the bytes, and drop the string.
- Session and recovery snapshots may serialize the rope to disk. They must not keep a second in-memory copy of the open document after the write.
- Spellcheck, find, and stats already bound their work to a viewport or a version counter. Point that work at rope slices so a viewport pass does not materialize the file.
- The CSV lazy index may scan once via `with_text` on first paint, store the `Vec<u64>` offsets, and release the temporary. It must not store the document text again.

### 5.3 Writes go to the rope only

**Behavior.**

- Raw edits from step 1 apply to the rope and record `EditOperation`s. Delete the ranged splice into `Tab.content`.
- Rendered and split commits build the same kind of rope edit (the replacement text and the source range they already compute) and apply it to the rope. They stop writing a parallel `String`.
- External reload replaces the rope (`set_content` or a rope rebuild) and bumps `content_version`.
- `is_modified` stays a version or hash of the rope against the last-saved hash. Large files already avoid storing `original_content`. Keep that. Do not hash the whole rope on every frame. Hash on save, on a debounced idle point, or compare `content_version` to the version at last save.

### 5.4 The editor crate boundary

**Behavior.**

- When the FerriteEditor crate is extracted, this layout is the one that moves. `ui()` reads and writes `TextBuffer`. The app does not push a full `String` in and pull a full `String` out each frame.
- If crate extraction is scheduled in the same release and this PRD slips, the crate work waits. Extracting the two-buffer split publishes the bug.

## 6. Acceptance

Release build. Record **Commit size** and working set. Working set may sit lower after the window goes idle. Commit size is the pass/fail number.

| Check | Pass |
|-------|------|
| Idle, no file | Record it. |
| Open the 82 MB CSV in Raw, do not type | Commit size is idle plus about 100–130 MB. A second ~80 MB plateau fails. |
| Type, Ctrl+Z, save, reopen | Bytes on disk match, undo works, line endings and encoding match the pre-change behavior on a CRLF and a UTF-16 fixture. |
| Rendered view of a normal markdown note | Edit, undo, and save still round-trip. |
| CSV grid on the large file | Row index appears, scrolling works, no second copy of the file shows up in commit size. |
| Find in a large raw file | Next/prev hits the match without a lasting extra full-file allocation. |

## 7. Tests

- A large-buffer test opens a rope-backed tab and asserts there is no stored `String` of that length afterward (the tab field is absent, or a test-only resident-byte counter stays under a small ceiling).
- Save → read back matches the rope, including CRLF and a non-UTF-8 encoding fixture already covered by the line-ending and encoding tests.
- Undo/redo from step 1 still passes without a `Tab.content` splice.
- One rendered commit (paragraph or heading) updates the rope and save output without a parallel document string.
- Existing small-file widget tests are updated to read the rope, not retargeted to a weaker assertion.

Do not assert process RSS in CI.

## 8. Docs

Replace the "current / wrong" diagram in [`architecture.md`](../../technical/editor/architecture.md) with the rope-only layout, and set the memory row in [`ferrite-editor.md`](../../technical/editor/ferrite-editor.md) to the measured commit-size delta for the 82 MB CSV. README and the v0.2.6 changelog notes stay worded as they are after the 2026-11-12 correction until this number is real. Then one sentence can say a large file costs about the size of its rope, not the size of the file three times over.
