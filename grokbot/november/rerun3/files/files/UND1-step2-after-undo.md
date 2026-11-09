# Rendered copy spacing (#162) & Split-view undo (#167)

Manual fixture for two v0.3.1 fixes that share a code-block setup.

## Part A — Copy from Rendered view (#162)

Switch to **Rendered**, select the code block content below, copy, and paste
into Notepad. Every line must paste **byte-identical** — especially no extra
space in `mkdir -p` (the historical bug produced `mkdir - p`).

```bash
sudo mkdir -p /opt/stacks/notesnook
sudo mkdir -p /srv/docker-data/notesnook/mongo
sudo mkdir -p /srv/docker-data/notesnook/minio
sudo mkdir -p /srv/backups-local/notesnook
```

Also copy this syntax-highlighted block (multiple token colors per line):

```rust
let x: Vec<String> = items.iter().map(|s| s.to_string()).collect();
```

And this inline-formatted paragraph — copy the whole line and verify no extra
spaces appear around the styled spans: normal **bold** *italic* `code` end.

## Part B — Split-view undo on code blocks (#167)

1. Switch to **Split** view.
2. In the **right** (preview) pane, click **Edit** on the code block below and
   append a new line of text at the end.
3. Click outside to commit the block.
4. Press **Ctrl+Z** once.

Expected: exactly the appended line is removed; the rest of the document is
untouched. The historical bug applied the undo at wrong offsets and mangled
unrelated text.

```python
def placeholder():
    value = 1
    return value
```

Text after the code block that must never be corrupted by undo:
SENTINEL-LINE-1 unique text that should survive all undo operations.
SENTINEL-LINE-2 more unique text to detect offset corruption.

## What to check

- [ ] A: bash block pastes with `mkdir -p` intact (no inserted spaces).
- [ ] A: rust block pastes identical (token boundaries add no spaces).
- [ ] A: formatted paragraph copies without extra spaces between spans.
- [ ] B: Ctrl+Z after a preview code-block edit removes only that edit.
- [ ] B: SENTINEL lines intact after multiple undo/redo cycles.
- [ ] B: repeat in Rendered (non-split) mode — same clean undo.
