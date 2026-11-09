# Find scroll-to-match & rendered arrow navigation (#175, #170)

Manual fixture. The word **ferrite** appears once in each numbered section so
Find next/prev must scroll the viewport between matches. The document is long
enough that matches are off-screen. Also use this file for Arrow Up/Down block
navigation in Rendered view.

## Section 1

The ferrite crystal sat on the shelf.

Paragraph without the keyword. Lorem ipsum dolor sit amet, consectetur
adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna
aliqua.

## Section 2

Another paragraph. Ut enim ad minim veniam, quis nostrud exercitation ullamco
laboris nisi ut aliquip ex ea commodo consequat.

Deep in the mine they found ferrite deposits.

- List item one
- List item two
- List item three

## Section 3

Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore
eu fugiat nulla pariatur.

> A quoted line for block navigation testing.

The word ferrite appears here again.

```rust
// A code block between matches (arrow nav should skip through or over it
// without getting stuck)
fn main() {
    println!("hello");
}
```

## Section 4

Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia
deserunt mollit anim id est laborum.

Multi-line paragraph for caret-boundary testing: line one of the paragraph
wraps and continues to line two of the paragraph and continues further to line
three so the caret has several visual rows to move through before leaving the
block at the first or last row.

Ferrite is mentioned in this fourth section.

## Section 5

Sed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium
doloremque laudantium.

| Table | Block |
|-------|-------|
| nav   | test  |

Final ferrite match at the bottom of the document.

## What to check — Find (#175)

- [ ] Rendered: Ctrl+F → `ferrite` → Enter/F3 cycles matches and the **viewport scrolls** to each.
- [ ] Shift+F3 / previous also scrolls.
- [ ] Split: preview pane scrolls to matches; raw find still works.
- [ ] Ctrl+F with an existing query **selects all** query text (typing replaces, does not append).

## What to check — Arrows (#170)

- [ ] Rendered: click a paragraph → ArrowDown/ArrowUp moves between blocks (headings, paragraphs, list items, quote).
- [ ] Inside a multi-line paragraph edit, arrows move the caret by visual row; at the first/last row the caret leaves to the adjacent block.
- [ ] Preview lock ON → arrows still navigate, edits stay blocked.
- [ ] Split: raw-pane arrows unchanged; preview arrows navigate.
