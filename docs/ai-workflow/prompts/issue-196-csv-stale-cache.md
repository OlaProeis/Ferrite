# Fix GitHub issue #196 (Ferrite 0.3.1)

Paste this whole file into a new chat. Fix only this issue. Do not commit unless asked.

## Issue

https://github.com/OlaProeis/Ferrite/issues/196

CSV table view can throw away a Raw-view edit.

On a CSV bigger than 8 KB, the table cache uses `hash_content_bytes`, which hashes the length plus the first and last 4 KB. A same-length edit in the middle leaves that hash unchanged, so the cached rows stay stale. The next cell commit serializes those rows back over `tab.content`, and the Raw edit is gone.

Cell editing is disabled at `LARGE_FILE_THRESHOLD` (1_000_000 bytes). The data loss is the full-parse path between 8 KB and that limit.

## Repo

- Path: `G:\DEV\markDownNotepad`
- Branch: `0.3.1-experimental`
- The working tree already has unrelated edits. Do not revert them, do not stash them, and do not commit them.

## What to change

File: `src/markdown/csv_viewer.rs`

- `hash_content_bytes` (about line 630) is the sampled hash. `test_hash_content_bytes_large` says a missed middle change is acceptable for that function. Leave that function and that test alone.
- `blake3_content_hash` (about line 646) already hashes the whole buffer. `show_raw_view` uses it.
- The table path does not. Around line 1752, `needs_rebuild` is computed with `hash_content_bytes`. Around line 1177, `apply_pending_cell_commit` stores that same sampled hash after writing the serialized rows.

Use `blake3_content_hash` for both of those. They must stay on the same function, or the frame after a commit will disagree with the cache.

The lazy path for files at or above `LARGE_FILE_THRESHOLD` also stores `content_hash` (about line 1784) and does not write rows back (editing is disabled). You can leave the sampled hash there. Do not leave it on any path that can serialize `cached_data` over the document.

## Tests

Add a regression test next to the existing csv hash tests:

- Build a CSV larger than 8192 bytes.
- Change one digit in the middle without changing the length.
- The hash the table cache uses must change.
- A cell commit on top of that edit must keep the middle change. If a full UI test is awkward, test the hash plus the serialize/commit helper directly and say what you did not run.

`cargo test --bin ferrite` for the csv viewer tests you touched. Fix anything you break. Do not weaken `test_hash_content_bytes_large`.

## Docs

Add a short note under `docs/technical/` (files or markdown, whichever matches the csv viewer docs) and one bullet in `docs/index.md`. State that the table cache uses the full-content hash so a mid-file Raw edit survives the next cell commit. #196.

## Leave alone

- Do not switch the release profile off `panic = "abort"`.
- Do not edit issue #197 or #198 (encoding). Another chat may be doing those.
- Do not comment on GitHub.
- Do not mark tasks done.
