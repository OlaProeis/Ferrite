# Ferrite — Large-file memory, step 1 (PRD)

> **Status:** Scheduled for v0.3.2. Not started.
> **Version target:** v0.3.2. Ship before step 2. Step 2 assumes the undo path in this PRD.
> **Consumers:** Human review, then a cyclopsctl task queue.
> **Pair:** [`prd-v0.3.2-large-file-memory-step2.md`](prd-v0.3.2-large-file-memory-step2.md) (one resident rope). This PRD does not do that.

## 1. Overview

Opening `test_md/very_large_test_data.csv` (86,169,227 bytes, about 82 MiB) in Raw view moved Ferrite from about 50 MB to about 370 MB working set on Windows (2026-11-12). Switching to Rendered did not change that number. The grid is not the cost. Raw view is already holding the file several times, and it keeps those copies when the view changes.

Virtual scrolling is fine. Visible lines are what get a galley. The waste is full-buffer copies that exist before the first paint finishes, and a full-file hash on every later frame.

**This step removes the copies that are pure overhead on the raw path.** After it, a large file still lives twice (the `String` on the tab, and the rope). That is the known "current" state in [`architecture.md`](../../technical/editor/architecture.md). One copy is step 2.

### Why this shape

| Choice | Decision |
|--------|----------|
| Threshold | Existing `LARGE_FILE_THRESHOLD` in `src/state.rs` (1_000_000 bytes). Do not add a second cutoff. |
| Who changes | The raw editor path only (`EditorWidget::show_inner`). Rendered and Split markdown keep today's snapshot undo. |
| Undo model | `EditHistory` already stores `EditOperation::{Insert, Delete}` with just the edited text. Large raw files must record those ops at the edit, not by diffing two full strings. |
| What "done" looks like | Opening the 82 MB CSV in Raw adds about two copies (string + rope), on the order of +160–220 MB over the idle process, not ~+320 MB. Typing one character does not allocate another full-file `String`. Ctrl+Z restores that character. |

A result of "+80 MB for an 80 MB file" is out of scope. ropey chunks are larger than the bytes they hold. Step 2's honest target is about +100–130 MB of buffer.

## 2. Goals

- A file at or above `LARGE_FILE_THRESHOLD`, opened in Raw and not edited, does not keep `pending_undo_snapshot`.
- The raw editor does not run blake3 (or any other hash) over the whole `Tab.content` on frames where the large file did not change.
- Creating the `FerriteEditor` from `Tab.content` does not turn the new rope back into a `String` to check that it matches.
- A later real change (reload, undo, rendered write-back) still updates the rope via `set_content` when `content_version` or length says the tab moved.
- Typing, delete, paste, and indent on a large raw file push one undo step and splice `Tab.content` by range. They do not `Rope::to_string()` the whole buffer and they do not `compute_edit_ops` on two full strings.
- Ctrl+Z / Ctrl+Y on that file restores and redoes the edit. Typing-run coalescing (the ~700 ms merge) still applies to single-character inserts.
- Files under the threshold keep the current snapshot path, including rendered and split markdown undo.

## 3. Non-goals

- **Removing `Tab.content`.** Both the string and the rope stay. Step 2 removes the string.
- **Dropping the rope when the user switches to Rendered.** The editor stays in `FerriteEditorStorage`. Rendered staying flat is expected. Do not evict the editor on view switch in this step.
- **A new buffer type** (gap buffer, piece table, mmap).
- **Changing rendered or split undo** for files under the threshold. A large markdown file edited in Rendered may still take the snapshot. Say so in a code comment. Step 2 covers every view.
- **Rewriting the README claim down to 1×.** Docs updated with this PRD already say step 1 leaves two copies.

## 4. Current state (verified in code, 2026-11-12)

### 4.1 Three resident copies, plus a spike

`Tab::with_file_bytes` (`src/state.rs`) decodes the file into `Tab.content`. For a large file it does **not** keep `original_content` or `original_bytes`. Good.

The first raw frame then does all of the following:

1. `EditorWidget::show_inner` (`src/editor/widget.rs`) calls `Tab::prepare_undo_snapshot_hashed` **before** any edit. That blake3-hashes the whole string and, on the first call, `clone`s it into `pending_undo_snapshot`. The clone stays for the life of the tab. Later frames hash the whole string again even when nothing changed (`src/state.rs`, `prepare_undo_snapshot_hashed`).
2. `take_ferrite_editor_from_storage` builds `FerriteEditor::from_string`, which copies every byte into a rope (`TextBuffer::from_string` → `Rope::from_str`).
3. `editor_needs_content_sync` returns `needs_sync = true` with reason `"no_editor"` because the editor did not exist yet. The sync block then calls `editor.buffer().to_string()` and hashes that, to compare it with the string the rope was just built from.

So an 82 MB open holds the tab string, the undo snapshot, and the rope, and allocates a fourth string for the compare. The fourth string is temporary. Windows often still shows it in the working set right after the open.

Switching to Rendered does not drop the rope or the snapshot. The CSV lazy index is about 4 MB (one `u64` per row) and is not the 370 MB.

### 4.2 How a keystroke records undo today

When the rope is dirty, `show_inner` does:

```text
tab.content = editor.buffer().to_string()
tab.record_external_edit_from_snapshot_coalesced(...)
```

`record_external_edit_from_snapshot_coalesced` diffs `pending_undo_snapshot` against the new full string (`compute_edit_ops` in `src/editor/ferrite/history.rs`). The editor already knows the kind of edit (`note_external_undo_edit` / `take_external_undo_recording`) but not the inserted or deleted slice. The slice is recovered by the full-buffer diff.

`EditHistory` itself is cheap: an `Insert` or `Delete` stores only that slice, and large files cap groups at `LARGE_FILE_MAX_UNDO_GROUPS` (200).

### 4.3 What must keep working

- Small-file raw undo, including coalesced typing and the anomalous-shrink path (`record_anomalous_buffer_shrink_from_snapshot`).
- Rendered and split undo for normal markdown (`prepare_undo_snapshot_hashed` in `src/markdown/editor.rs`, `src/markdown/rendered_session.rs`, and the rendered branches of `src/app/central_panel.rs`).
- External reload and `content_version` resync (`editor_needs_content_sync` reason `"version_changed"`). The rope must still pick up a new `Tab.content`. It must not get there by printing the old rope to a string first.
- CRLF: splices into `Tab.content` go through the tab's `LineEnding` rules already used by rope edits. Do not join with `"\n"` only.

## 5. Functional requirements

### 5.1 No baseline clone, no per-frame full hash (large raw only)

**Problem.** `prepare_undo_snapshot_hashed` exists so a later diff can see the pre-edit text. On a large raw file the pre-edit text is the rope, and the diff never needs a second `String`.

**Behavior.**

- In `EditorWidget::show_inner`, if `tab.content.len() >= LARGE_FILE_THRESHOLD`, do not call `prepare_undo_snapshot_hashed`.
- Do not add a different full-buffer hash in its place.
- Leave `pending_undo_snapshot` as `None` for that tab unless some other, non-raw path already filled it. If a previous small-file session left one and the buffer then grew past the threshold, drop it on the next large raw frame.
- Files under the threshold call `prepare_undo_snapshot_hashed` exactly as today.

### 5.2 Record the edit from the rope, splice the string

**Problem.** The dirty-frame path replaces `Tab.content` with `buffer().to_string()` and then diffs two full strings. That brings the extra copies back on the first keystroke.

**Behavior.**

- When a large raw edit commits, the editor hands the tab an `EditOperation` (or a short list of them) built from the mutation it just applied: char position, and the inserted or deleted text. The rope mutation sites (`insert` / `remove` and the indent helpers) are where that text is already known. Capture it there. Do not rediscover it with `compute_edit_ops`.
- Push those ops through `EditHistory::record_with_meta` so coalescing, `break_before`, and the 200-group cap stay in one place.
- Update `Tab.content` with a ranged splice (`replace_range` / `insert_str` / `drain` on the existing `String`) using the same char offsets. Peak extra allocation is the size of the edit, not the file.
- `content_version` still bumps, so the title dirty mark, autosave, and the editor sync cache see the edit.
- The anomalous-shrink guard stays. If it can only work by comparing two full strings, skip that compare for large files and log once. Do not reintroduce the snapshot to keep the guard.

### 5.3 Do not materialize the rope on create or on sync

**Problem.** `"no_editor"` forces `needs_sync`, and the sync block hashes `buffer().to_string()` even though `from_string` just copied `Tab.content`.

**Behavior.**

- After creating the editor from `Tab.content`, store length, `content_version`, and the content hash taken from **the string**, not from a rope `to_string()`.
- For a large file, `editor_needs_content_sync` reason `"no_editor"` must not enter the `buffer().to_string()` compare.
- When a large file really does need a resync (`version_changed`, length change, reload), call `set_content(&tab.content)` directly. Do not hash the old rope first.
- Small files keep the current compare.

### 5.4 Undo still round-trips

**Behavior.**

- Ctrl+Z applies the stored `EditOperation` to the rope and applies the inverse splice to `Tab.content`, or applies it to the rope and splices the string from that same op. Both buffers match after undo and after redo.
- One typed character is one undo step after the coalesce window, or one step for a coalesced run, matching small files.
- Paste of a multi-kilobyte clip is one step. The op holds the clip, not the whole file.

## 6. Acceptance

Measured on a release build, Windows Task Manager, **Commit size** as well as working set (working set alone drops when Windows trims cold pages).

| Check | Pass |
|-------|------|
| Idle process, no file | Record the number. Do not require 20 MB. |
| Open `test_md/very_large_test_data.csv` in Raw, do not type, let the number settle | Commit size is about two copies over idle (string + rope, rope a bit over the file). A jump of ~300 MB or more fails. |
| Switch to Rendered and back | No third full copy appears. The rope from Raw may still be resident. |
| Type one character, Ctrl+Z, Ctrl+Y | Character appears, disappears, reappears. A heap counter or test double proves no full-file `String` alloc on that keystroke. |
| File under 1 MB, raw and rendered | Existing undo tests pass unchanged. |

## 7. Tests

Targeted, not a full `cargo test` as the deliverable of each edit:

- A large-content fixture (build a `String` past `LARGE_FILE_THRESHOLD` in the test; do not require the 82 MB CSV in CI) asserts `pending_undo_snapshot` stays `None` across a raw `show` frame.
- An edit on that fixture records an `EditOperation` whose stored text is the edit, not the file, and `Tab.content` equals the rope afterward.
- Undo and redo on that fixture restore both sides.
- A regression test that the create/sync path for a large buffer does not call a full rope `to_string()`. A test hook or a counter on the conversion is enough. Do not snapshot process RSS in CI.
- Existing history and widget undo tests still pass for small buffers.

## 8. Docs

When the code lands, update [`architecture.md`](../../technical/editor/architecture.md) and [`ferrite-editor.md`](../../technical/editor/ferrite-editor.md) so they describe two resident copies, and point step 2 at the remaining string. Do not claim 1× RAM.
