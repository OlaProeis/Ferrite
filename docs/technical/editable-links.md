# Editable Links in Rendered Mode

## Overview

Task 45 implements a hover-based link editing system in the rendered WYSIWYG view. This allows users to view and edit link text and URLs through a popup menu, while keeping links non-clickable to prevent accidental navigation during editing.

## Architecture

### Components

1. **`RenderedLinkState`** (`src/markdown/widgets.rs`)
   - Manages link editing state
   - Tracks popup open/closed state
   - Stores temporary edit values
   - Provides modification detection and commit/reset

2. **`RenderedLinkWidget`** (`src/markdown/widgets.rs`)
   - Renders link text with underline styling
   - Shows settings icon (⚙) on hover
   - Displays edit popup with text/URL fields and action buttons
   - Follows the EditableCodeBlock widget pattern

3. **`render_link`** (`src/markdown/editor.rs`)
   - Updated to use `RenderedLinkWidget`
   - Creates stable IDs based on line position and URL
   - Stores widget state in egui's memory
   - Triggers source updates on change

4. **`update_link_in_source`** (`src/markdown/editor.rs`)
   - Finds and replaces link syntax in markdown source
   - Handles various link formats with/without titles
   - Preserves surrounding content

## User Interaction

### Viewing Links

1. Links display with link color (blue) and underline styling
2. Links are **non-clickable** - clicking does nothing
3. Hovering shows a tooltip with the URL

### Editing Links

1. Hover over a link to reveal the **settings icon** (⚙)
2. Click the settings icon to open the **edit popup**
3. The popup contains:
   - **Text field**: Edit the display text
   - **URL field**: Edit the URL
   - **Open button** (🔗): Opens URL in browser (http/https only)
   - **Copy button** (📋): Copies URL to clipboard
4. **Click outside** the popup to close and save changes

### Workflow

```
Hover → Settings Icon → Click → Popup Opens → Edit → Click Outside → Changes Saved
```

### Important: No Separate Metadata

This implementation only edits the actual markdown content. **No separate metadata or data layer is created.** This ensures full compatibility with other applications and file formats.

### Autolink Support (Safe Mode)

Bare URLs (autolinks) like `https://example.com` are handled safely:

**For autolinks (bare URLs):**
- Text field is **hidden** - there's no separate text in the source
- Only the URL field is shown and editable
- Changes update only the URL in the source
- **No markdown syntax is injected** - a bare URL stays a bare URL

**For markdown links `[text](url)`:**
- Both Text and URL fields are shown and editable
- Changes update the markdown syntax directly

This safe approach ensures:
- Files are never corrupted by injecting markdown where it wasn't intended
- Works correctly for non-markdown files (JSON, plain text, etc.)
- User data is always respected

## Implementation Details

### State Management

```rust
pub struct RenderedLinkState {
    pub popup_open: bool,     // Is popup currently shown
    pub edit_text: String,    // Current text being edited
    pub edit_url: String,     // Current URL being edited
    original_text: String,    // For change detection
    original_url: String,     // For change detection
}
```

State is stored in egui's memory with a stable ID based on:
- Line number in source
- Original URL

### Hover Zone

To prevent flickering when moving between the link and settings icon, a unified hover zone is used:

```rust
let hover_zone = egui::Rect::from_min_max(
    link_rect.min,
    link_rect.max + egui::vec2(26.0, 0.0), // Extend to include button
);
```

The settings icon appears when the mouse is anywhere in this zone, not just over the link text.

### Click-Outside-to-Close

The popup closes automatically when clicking outside both the popup and the hover zone. Changes are committed when closing.

### Line Number Handling

The parser may return 0 for inline elements (like autolinks within paragraphs). The source update function handles this by treating 0 as line 1:

```rust
let effective_start = if start_line == 0 { 1 } else { start_line };
```

### Change Synchronization

When the user clicks "Done":
1. `RenderedLinkState::commit()` is called
2. `update_link_in_source()` replaces the old `[text](url)` with new values
3. Edit state is marked as modified
4. Changes propagate to markdown source

### URL Opening

- Only `http://` and `https://` URLs can be opened
- Uses the `open` crate for cross-platform browser opening
- Non-web URLs show disabled button with tooltip explanation

## Testing

### Unit Tests

Located in `src/markdown/widgets.rs` and `src/markdown/editor.rs`:

- `test_rendered_link_state_new`
- `test_rendered_link_state_modification_detection`
- `test_rendered_link_state_url_modification`
- `test_rendered_link_state_commit`
- `test_rendered_link_state_reset`
- `test_rendered_link_output_fields`
- `test_update_link_in_source_simple`
- `test_update_link_in_source_text_only`
- `test_update_link_in_source_url_only`
- `test_update_link_in_source_multiline`
- `test_update_link_in_source_preserves_other_lines`
- `test_update_link_in_source_multiple_links_same_line`

### Manual Testing

1. Open a markdown file with links in Rendered mode
2. Hover over a link → settings icon appears
3. Click icon → popup opens with correct values
4. Edit text/URL, click Done → link updates
5. Open Link button works for http/https URLs
6. Copy URL button copies to clipboard
7. Switch to Raw mode → changes preserved
8. Multiple links work independently

## Out of Scope

The following features were intentionally excluded:

- ❌ Ctrl+Click to open links
- ❌ Inline text editing (sync issues)
- ❌ Link validation/warnings
- ❌ Relative file navigation
- ❌ Image link previews
- ❌ Ctrl+K shortcut for creating links

## Related Files

- `src/markdown/widgets.rs` - RenderedLinkWidget, RenderedLinkState
- `src/markdown/editor.rs` - render_link, update_link_in_source
- `src/markdown/parser.rs` - Link node parsing (MarkdownNodeType::Link)

## Dependencies

- `open` crate - for opening URLs in browser
- `egui` - for UI components and state management
