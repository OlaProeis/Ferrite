A# GitHub HTML Subset Test

Manual fixture for **v0.3.1** GitHub HTML Phases 1–2. Open in **Rendered** or **Split** view and compare against the expectations below each section.

Docs: `docs/technical/markdown/github-html-subset.md`, `github-html-block-subset.md`.

---

## Phase 1 — `<div align>`

### Left (default)

<div align="left">

This paragraph should be **left-aligned** inside the div.

- Bullet one
- Bullet two

</div>

### Center

<div align="center">

This paragraph should be **center-aligned**.

*Italic* and `code` still render inline.

</div>

### Right

<div align="right">

This paragraph should be **right-aligned**.

</div>

### Adjacent divs

<div align="center">First centered block.</div>

<div align="right">Second block, right-aligned.</div>

Regular markdown paragraph after aligned divs — normal left alignment.

---

## Phase 1 — `<details>` / `<summary>`

### Closed by default

<details>
<summary>Click to expand (closed by default)</summary>

Hidden content appears here after you click the summary row.

- Nested list item
- Second item

```rust
// Code block inside details
fn main() {}
```

</details>

### Open by default

<details open>
<summary>Already expanded (`open` attribute)</summary>

This content should be **visible on first load** without clicking.

</details>

### Details with rich summary

<details>
<summary><strong>Bold summary</strong> — still clickable</summary>

Body text after a formatted summary.

</details>

### Back-to-back details

<details>
<summary>Section A</summary>

Content A.

</details>

<details open>
<summary>Section B (open)</summary>

Content B.

</details>

---

## Phase 1 — <br> line breaks

Standalone block break (blank line, then `<br>`, then more text):

<br>

Line after standalone `<br>` block.

Inline breaks in one paragraph:

Line one<br>Line two<br>Line three — three visual lines, one paragraph.

Hard break via markdown (for comparison):  
Second line via two trailing spaces.

---

## Phase 2 — <kbd>

Simple key cap: press <kbd>Ctrl</kbd> to start.

Chord: <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>P</kbd>

In a sentence with **bold** nearby: save with <kbd>Ctrl</kbd>+<kbd>S</kbd> before closing.

Arrow keys: <kbd>↑</kbd> <kbd>↓</kbd> <kbd>←</kbd> <kbd>→</kbd> (glyphs inside kbd boxes).

### Nested HTML inside `<kbd>` (unsupported — passthrough)

Expected: literal `«HTML»` or raw inline HTML text, **not** a styled key cap:

<kbd><b>Ctrl</b></kbd>

---

## Phase 2 — `<sup>` and `<sub>`

Chemistry: H<sub>2</sub>O and CO<sub>2</sub>.

Physics: E=mc<sup>2</sup> and x<sup>n</sup> + y<sup>n</sup> = z<sup>n</sup>.

Footnote style: word<sup>1</sup> and reference<sub>note</sub>.

Mixed in one line: H<sub>2</sub>SO<sub>4</sub> at 10<sup>−3</sup> mol/L.

---

## Phase 2 — sized `<img>`

Place a small local image at `test_md/assets/sample.png` (or adjust the path) to verify width/height. Markdown image for comparison:

![Markdown image](assets/sample.png)

HTML with explicit dimensions (120×40):

<img src="assets/sample.png" width="120" height="40" alt="Logo 120×40">

Larger dimensions (should scale down if wider than viewport):

<img src="assets/sample.png" width="400" height="200" alt="Wide logo">

HTML img with alt/title only (natural size when file exists):

<img src="assets/sample.png" alt="Sample" title="Tooltip title">

**If the file is missing:** broken-image indicator, no panic.

---

## Mixed markdown + HTML

<div align="center">

### Heading inside centered div

Press <kbd>Enter</kbd> after <kbd>Ctrl</kbd>+<kbd>S</kbd>.

Water: H<sub>2</sub>O — energy: E=mc<sup>2</sup>

</div>

> > [!NOTE]
  > Callout beside HTML — callout styling unchanged; HTML sections above/below still independent.

---

## Unsafe HTML (must stay passthrough / not execute)

Expected: `«HTML»` block indicator or escaped/raw text — **no** script execution, **no** iframe embed.

<script>alert('xss')</script>

<iframe src="https://example.com"></iframe>

<div onclick="alert(1)">Click me — event handler stripped/blocked</div>

<style>body { display: none; }</style>

---

## Unsupported tags (Phase 3+ — passthrough)

Expected: `«HTML»` or raw passthrough, not structured rendering.

<span style="color:red">Colored span</span>

<mark>Highlighted mark</mark>

<table><tr><td>HTML table cell</td></tr></table>

<b>HTML bold</b> vs **markdown bold**

---

## Preview lock regression (optional)

1. Lock the preview padlock on this tab.
2. Confirm: details expand/collapse still works (read-only navigation).
3. Confirm: no WYSIWYG edit on HTML blocks; switch to **Raw** to edit source.
4. Unlock — editing restored if you use rendered click-to-edit elsewhere.

---

## Quick checklist

| # | Check |
|---|--------|
| 1 | Left / center / right `<div align>` layout correct |
| 2 | `<details>` closed until clicked; `<details open>` expanded on load |
| 3 | `<br>` produces visible line breaks (block + inline) |
| 4 | `<kbd>` renders boxed monospace key caps |
| 5 | `<sup>` / `<sub>` raise/lower smaller text |
| 6 | `<img width height>` respects dimensions (local file) |
| 7 | `<script>`, `<iframe>`, `onclick` do not run |
| 8 | Unsupported tags show passthrough, app stable |
| 9 | Split view: preview matches rendered-only |