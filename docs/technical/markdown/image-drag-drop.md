# Image Drag & Drop

> **Current reference:** [`local-image-assets.md`](local-image-assets.md) — drop, paste, path resolution, and Rendered preview (#164).

## Overview

Ferrite supports dragging and dropping image files directly into the editor. Dropped images are automatically:
1. Copied to an `assets/` folder
2. Renamed with a unique timestamp-based name
3. Inserted as a markdown image link at the cursor position

## Supported Formats

- PNG (`.png`)
- JPEG (`.jpg`, `.jpeg`)
- GIF (`.gif`)
- WebP (`.webp`)

## How It Works

### Drop Detection

When files are dropped onto the Ferrite window, the application categorizes them:
- **Folders**: Opened as workspace
- **Images**: Processed via the image handler
- **Documents**: Opened in new tabs (markdown, csv, json, etc.)

Images take priority over documents when mixed files are dropped.

### Asset Directory Resolution

The assets directory is determined in this priority order:
1. **Document directory**: If the active document is saved, uses `<document-dir>/assets/`
2. **Workspace root**: If in workspace mode, uses `<workspace>/assets/`
3. **Current directory**: Falls back to `./assets/`

### Unique Filename Generation

Dropped images are renamed to prevent collisions:
```
YYYYMMDD-HHMMSS-originalname.ext
```

Example: `20260116-143025-photo.png`

### Markdown Link Insertion

After copying the image, a markdown image link is inserted at the cursor position:
```markdown
![](assets/20260116-143025-photo.png)
```

The cursor is then positioned after the inserted link.

## Implementation Details

### Key Functions

Located in `src/app/file_ops.rs` and `src/path_utils.rs`:

- `is_supported_image()` — Supported image extension check
- `assets_dir_for_paths()` / `get_assets_dir()` — Target `assets/` directory
- `generate_unique_image_filename()` — Timestamp-based unique filename
- `handle_dropped_image()` — Copy to assets + insert markdown
- `handle_dropped_files()` — Main drop event handler (categorizes and dispatches)
- `resolve_local_image_path()` — Rendered preview path resolution (see [`local-image-assets.md`](local-image-assets.md))

### Undo Support

Image insertions are fully undoable. The operation records the edit state before insertion, allowing Ctrl+Z to remove the inserted link.

### Toast Notification

A toast notification confirms successful image insertion:
- Single image: "Image added to assets"
- Multiple images: "N images added to assets"

### Error Handling

Errors are displayed via the error dialog:
- Failed to create assets directory
- Failed to copy image file

## Usage Tips

1. **Unsaved documents**: If the document hasn't been saved yet, images go to the workspace root's assets folder. Save your document first for images to be stored relative to it.

2. **Multiple images**: Dropping multiple images inserts them all sequentially at the cursor position, each with a unique timestamp.

3. **Existing assets folder**: If `assets/` already exists, images are added to it without affecting existing files.

## Related Features

- Smart paste (Ctrl+V) for image URLs creates inline markdown links
- Preview mode renders markdown images inline
