# Local image assets & clipboard paste (#164)

Manual fixture for local image rendering path resolution. Uses the checked-in
sample at `test_md/assets/sample.png`. Open this file in **Rendered** or
**Split** view.

## Path variants that must all render

Plain relative path:

![plain](assets/sample.png)

MarkText-style dot-slash:

![dot-slash](./assets/sample.png)

Bare filename (resolves via the document's `assets/` folder):

![bare](sample.png)

Backslash separators (Windows-style, must still resolve):

![backslash](assets\sample.png)

Percent-encoded name (decodes to the same file — expected to fall back
gracefully if the literal name does not exist):

![encoded](assets/sample%2Epng)

## Remote URLs — must NOT be fetched (local-first policy)

![remote placeholder](https://example.com/does-not-load.png)

Expected: placeholder frame with alt text / "Web images not supported" — no
network request, no image.

## Missing file — graceful failure

![missing](assets/definitely-not-here.png)

Expected: "Image not found" style placeholder, no crash, no retry storm.

## What to check

- [ ] All four local variants above render the sample image.
- [ ] Remote URL shows placeholder only (verify no fetch).
- [ ] Missing file shows a placeholder, scrolling stays smooth.
- [ ] Drag-drop a new PNG into this document → saved under `assets/`, markdown inserted, renders immediately.
- [ ] Copy an image to the OS clipboard (Snipping Tool) → Ctrl+V in Raw → PNG saved under `assets/`, `![](assets/…)` inserted, renders in preview.
- [ ] Ctrl+Z after clipboard paste removes the markdown insert.
- [ ] Text paste still pastes text; image-URL smart paste unchanged.
