# Local image assets — drag-drop, paste, and Rendered preview (#164 / #182)

End-to-end workflow for **local** images: copy files into an `assets/` folder beside the document, insert `![](assets/…)` markdown, and display them in Rendered/Split preview. Remote `http://` / `https://` URLs are **not** fetched — preview shows a placeholder.

## Where `assets/` lives

`assets_dir_for_paths(doc_path, workspace_root)` in `src/path_utils.rs` is the **resolution** helper (preview + tests):

| Priority | Location |
|----------|----------|
| 1 | `<document-parent>/assets/` when the active tab has a saved path |
| 2 | `<workspace-root>/assets/` when in workspace mode |
| 3 | `./assets/` relative to the process working directory (preview of already-written relative links only) |

**Writes** (drop / clipboard paste) must use `assets_dir_for_write`. It returns `None` for a pathless tab with no workspace so Ferrite **does not** create `assets/` in the process CWD. Callers toast `image_paste.save_document_first` and abort.

## Drag-drop

When image files are dropped on the Ferrite window (`handle_dropped_files` → `handle_dropped_image` in `src/app/file_ops.rs`):

1. Supported extensions: `.png`, `.jpg`, `.jpeg`, `.gif`, `.webp`
2. `flush_active_rendered_session` runs first so a drop during click-to-edit does not orphan the active block
3. File is copied via `copy_dropped_image_to_assets` → `unique_asset_dest_path` (UTC timestamp name `YYYYMMDD-HHMMSS-<stem>.<ext>` via `chrono`, then `-1`, `-2`, … if that name exists). Markdown links the **filename actually written**
4. `![](assets/<filename>)` is inserted at the caret (undo removes the markdown insert; file on disk remains)

Images take priority over document opens when mixed files are dropped. Folders still open as workspaces.

## Clipboard paste

See [`clipboard-image-paste.md`](clipboard-image-paste.md) — same `assets_dir_for_write` / `unique_asset_dest_path` / `insert_asset_image_markdown` helpers.

## Rendered preview path resolution

`render_image` in `src/markdown/editor.rs` resolves image URLs via `resolve_local_image_path` (not generic link resolution). Context comes from `WikilinkContext` (`current_dir` = document parent, `workspace_root`).

Resolution order in `resolve_local_image_path`:

1. Skip `http://` / `https://` (caller shows “Web images not supported”)
2. Percent-decode (`%20` → space) and normalize MarkText-style paths (`./assets/…`, backslashes → `/`)
3. Standard local path resolution (absolute, `file://`, relative to document dir, then workspace)
4. CWD fallback for already-written relative `assets/…` (untitled / no workspace preview only)
5. **Bare filename fallback** — `![](photo.png)` → `<assets-dir>/photo.png` (MarkText migration / drop naming)

### Reporter cases (#182)

These resolve for preview when the file exists **beside the document** (not only under `assets/`):

| Markdown | Resolves to |
|----------|-------------|
| `![x](image.png)` | `<doc-dir>/image.png` |
| `![x](./image.png)` | same (`./` stripped, then relative to document dir) |
| `![x](assets/photo.png)` / `./assets/photo.png` / `.\assets\photo.png` | `<doc-dir>/assets/photo.png` |
| `![x](20260101-image.png)` (bare drop-style name, file only under `assets/`) | `<doc-dir>/assets/20260101-image.png` |

Reply-ready: Ferrite preview follows the same relative path as the markdown. A file next to the `.md` is valid; `assets/` is used when the name is only there (MarkText / drop naming). Remote `http(s)` images are still placeholders — no network fetch.

## Texture cache and failed loads

`render_image` uses egui temp data:

| Cache | Key / contents | Refresh |
|-------|----------------|---------|
| Success | `ImageLoadResult::Loaded` keyed by path + **mtime + len** | `metadata()` at most every **500 ms** per path so an overwritten `assets/sample.png` updates within a second |
| Failure | `ImageLoadResult::Failed { msg, mtime, at }` | Retry only when `metadata().modified()` changes or after a **2 s** backoff. `log::warn` **once** per path (no 60×/s spam) |

## Table cells (known limitation)

Images inside table cells are **not** rendered as textures. The cell layout shows alt text plus a small image icon; hover tooltip is the URL. Full inline images in table cells and headings are deferred to v0.3.2.

## Key code

| Piece | Location |
|-------|----------|
| `assets_dir_for_paths`, `assets_dir_for_write`, `resolve_local_image_path` | `src/path_utils.rs` |
| Drop / paste / `unique_asset_dest_path` / `insert_asset_image_markdown` | `src/app/file_ops.rs` |
| Fail + texture cache, `render_image` | `src/markdown/editor.rs` |
| Table-cell icon + alt + tooltip | `src/markdown/widgets.rs` — `append_table_cell_image_fallback` |
| Wikilink / image context | `src/app/central_panel.rs` — `WikilinkContext` per tab |

## Tests

- `path_utils::tests` — `resolve_local_image_path_*` (beside-doc / `./image.png`), `assets_dir_for_write_refuses_untitled_without_workspace`
- `markdown::editor::tests` — `failed_image_cache_*`, metadata throttle
- `markdown::widgets::tests` — table-cell icon/alt (not raw `![](…)`), click map
- `app::file_ops::image_assets_tests` — `unique_asset_dest_path_used_by_drop_yields_distinct_names`, `untitled_without_workspace_does_not_write_cwd_assets`

## Related docs

- [`clipboard-image-paste.md`](clipboard-image-paste.md) — OS clipboard image bytes
- [`smart-paste.md`](smart-paste.md) — URL link/image smart paste (no remote download)
- [`image-drag-drop.md`](image-drag-drop.md) — legacy overview (see this doc for current behaviour)

## Manual QA

- Saved `.md` + drag-drop PNG → file under `<doc-dir>/assets/`, markdown inserted, **visible in Rendered**
- Two same-named PNGs from different folders → two files, two distinct links
- Untitled tab + paste/drop image → toast `image_paste.save_document_first`, nothing written under CWD `assets/`
- Missing image: placeholder, no per-frame `warn` under `--log-level debug`
- Replace `assets/sample.png` on disk → preview updates within a second
- `![x](image.png)` / `./image.png` beside the document renders
- Table cell `![alt](url)` → icon + alt + URL tooltip, not raw markdown
- `![x](https://example.com/a.png)` still shows placeholder, no network fetch
