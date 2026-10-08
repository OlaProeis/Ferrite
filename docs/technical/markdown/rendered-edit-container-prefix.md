# Rendered Edit — Container Prefix Model

Click-to-edit in Rendered/Split seeds session buffers **without** blockquote / list markers and re-attaches the original prefix on commit. One helper owns that split so headings, paragraphs, lists, and `> [!NOTE]` callout bodies do not lose or double `> `.

**Related:** [Source range replacement](./rendered-edit-source-range.md), [Headings](./rendered-edit-session-headings.md), [Plain paragraphs & lists](./rendered-edit-session-paragraphs-lists.md), [GitHub callouts](./github-callouts.md)

## Problem

Editing a heading or paragraph inside a `>` blockquote (including every `> [!NOTE]` callout body) rewrote source wrongly:

| Block | Bad commit |
|-------|------------|
| `> ## Sub` | `# Sub` (quote dropped, level reset to H1) |
| `> quoted one` | `> > quoted one` (prefix doubled every commit) |
| Culled multi-line paragraph | In-frame commit with no `edit_state` node used a single-line span and left stale trailing lines |

## Canonical split

`split_container_prefix(line) -> (prefix, content)` in `src/markdown/editor.rs`:

- **Prefix:** leading indent + `>` markers (and the optional space after each) + optional list/task/ordered marker
- **Content:** the rest of the line
- Plain indented paragraphs (no quote / list marker) keep an empty prefix so leading spaces stay editable

`extract_line_prefix` is an alias. Unit shapes: `> `, `> > `, `  - `, `> - [ ] `, `1. `.

## Seed (no prefix)

Every line of the block is stripped before it enters the session buffer:

| Path | Behaviour |
|------|-----------|
| `extract_paragraph_content` | Maps each source line through `split_container_prefix` |
| `seed_session_block_text` | Headings use AST `text_content()` (or `#` strip after prefix); paragraphs use `extract_paragraph_content` |
| TextEdit `source_text` | Same extract — formatted and plain paragraphs |

Example: `> quoted one` seeds `quoted one`.

## Commit (re-attach prefix)

| Block | Write-back |
|-------|------------|
| Heading | `format!("{prefix}{}", format_heading(text, level))` via `update_source_line`. `prefix` from the original source line. `level` from `heading_level_for_commit`: Heading AST node via `find_block_node_for_ref`, else `#` count **after** the prefix |
| Paragraph / list | `update_source_range` prefixes the first committed line with the original marker; continuation lines reuse the **quote** part of the prefix (`continuation_prefix`), not indent-only. List markers hang as two spaces |

Rendered commits still do **not** bump `source_epoch` (RS-7).

## Culled in-frame commits

Viewport-culled blocks may have no `edit_state` node. `ast_end_line_for_session_block` prefers `edit_state`, then `find_block_node_for_ref(...).end_line` from `cache::get_or_parse` (same as `flush_rendered_edit_session` / `seed_edit_state_for_active_block`). Shrinking a 3-line paragraph to one line then removes the old trailing lines.

## Out of scope

List-marker renumbering, rendered arrow navigation, HTML entity caret mapping, `widgets.rs`.

## Tests

Round-trips in `src/markdown/editor.rs` (`seed_mutate_commit`):

- `> ## Sub` → `> ## Subx`
- `> quoted one\n>\n> quoted two` (first paragraph) → `> quoted onex\n>\n> quoted two`
- `> [!NOTE]\n> body` (body) → `> [!NOTE]\n> bodyx`
- `> > deep` → `> > deepx`
- `> - item` → `> - itemx`
- Plain `- item` and a plain paragraph unchanged

Regression: 3-line paragraph at line 200 of a 400-line doc, committed via `write_session_block_to_source` with an empty `EditState`, removes lines 201–202.

Existing `seed_session_block_text_*` (blockquote paragraph seed is `quoted one`) and RS-1…RS-7 still apply.

## Manual QA

`v0.3.1-minimal-release-test.md` rows **17**, **18**, **18b** (edit inside `> [!NOTE]`, Escape, check Raw).
