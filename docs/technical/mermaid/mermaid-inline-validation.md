# Mermaid Inline Validation

Parse-time validation for fenced ```` ```mermaid ```` blocks. Surfaces
structured errors in the rendered diagram area, draws warning squiggles in
the raw editor, and keeps the last successfully rendered diagram visible
while the user fixes a transient typo.

Implements task **#71** (depends on #57 — egui 0.31 upgrade).

## Goals

- Catch parser failures before the user has to switch views.
- Tell the user **which line** broke and **how to fix it**.
- Never blank a working diagram on a small breaking edit.

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│  src/markdown/mermaid/validation.rs                              │
│                                                                  │
│  ┌────────────────────────────┐    ┌─────────────────────────┐   │
│  │ validate_mermaid_source()  │    │ MermaidError            │   │
│  │   → Result<(), Mermaid…>   │───▶│  • message              │   │
│  │   parse-only, no rendering │    │  • line (1-indexed)     │   │
│  └────────────────────────────┘    │  • hint (optional)      │   │
│              │                     └─────────────────────────┘   │
│              ▼                                                   │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │ compute_mermaid_diagnostics(content) → Vec<DiagnosticEntry>│  │
│  │   walks the markdown AST, validates every mermaid block,   │  │
│  │   maps body-relative lines → editor line numbers           │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
              │                                 │
              ▼                                 ▼
   markdown/widgets.rs                 app/central_panel.rs
   ─ MermaidBlock widget               ─ merges with LSP diagnostics
   ─ warning header banner             ─ feeds FerriteEditor.diagnostics
   ─ last-good fallback render         ─ wavy underlines in raw view
```

## Public API (`crate::markdown::mermaid`)

| Symbol | Purpose |
|---|---|
| `MermaidError { message, line, hint }` | Structured validation error |
| `validate_mermaid_source(&str) -> Result<(), MermaidError>` | Parse-only validation |
| `compute_mermaid_diagnostics(&str) -> Vec<DiagnosticEntry>` | All mermaid errors in a markdown document |

`compute_mermaid_diagnostics` is also re-exported from
`crate::markdown::compute_mermaid_diagnostics` for the central panel.

## Parser-error normalization

Many existing diagram parsers already prefix their messages with
`Line N:` (e.g. the sequence-diagram parser). `MermaidError::from_message`:

1. Extracts the first `Line N` mention into `MermaidError::line` (1-indexed
   within the diagram body, defaulting to **1** when no line is reported —
   reasonable because the header is usually the culprit).
2. Strips a redundant `Line N: ` prefix so the warning header doesn't
   duplicate it.
3. Runs `derive_hint` to attach a short suggestion when one of the common
   mistake patterns matches.

The per-diagram parsers themselves keep their `Result<T, String>`
signatures; the validation module owns the normalization.

**Panic-freedom:** `validate_mermaid_source` is not wrapped in
`catch_unwind`. Parsers must not panic on half-typed or swapped brackets;
use `parse_util::slice_between` (see
[`parser-safe-slicing.md`](parser-safe-slicing.md)). Release builds abort
on panic, so a parser slice bug kills the app during typing in Split view.

## Hint heuristics

`derive_hint` matches the parser message and the source for common mistakes
and returns a short suggestion:

| Trigger | Suggestion |
|---|---|
| `Expected 'flowchart'` / `Empty flowchart` / `Missing diagram header` | "Add a diagram header like `flowchart TD` or `graph LR` …" |
| `Unknown diagram type` | Lists supported types |
| `'else' without matching 'alt'` | Wrap in `alt … else … end` |
| `'and' without matching 'par'` | Wrap in `par … and … end` |
| Unbalanced `[]`, `()`, or `{}` | Names the unbalanced bracket |
| `subgraph` count > `end` count | Reports counts and asks for missing `end` |

## Rendered widget — warning header + last-good fallback

`MermaidBlockData` gained two preserved-across-frames fields:

- `last_good_source: Option<String>` — last source that successfully rendered.
- `last_error: Option<MermaidError>` — most recent validation error.

`render_mermaid_block` (in `markdown/editor.rs`) preserves both fields
when the underlying markdown literal changes, so a small edit that breaks
the parse keeps the previous good render available.

`MermaidBlock::show` flow:

1. Validate the current source (parse-only, no rendering).
2. **Valid** → render normally; on render success, update
   `last_good_source` and clear `last_error`.
3. **Invalid** → store the error, draw a yellow warning header banner
   (`show_validation_warning`) with `Line N:`, the message, and the hint.
   Below the banner:
   - if `last_good_source` is set, render that source so the user keeps
     visual context;
   - otherwise, fall back to showing the syntax-highlighted source so the
     user can fix the issue.

The original `show_render_error` helper is retained but no longer on the
hot path; `#![allow(dead_code)]` at the file level lets it stay until any
caller in HTML export needs it again.

## Raw editor — wavy squiggles

`compute_mermaid_diagnostics(content)` walks the cached markdown AST
(`crate::markdown::cache::get_or_parse`) and emits a `DiagnosticEntry`
(reuses the LSP types in `crate::lsp::state`) for every fenced
`mermaid` block whose body fails to validate.

Line mapping:

```
fence opener (```mermaid)       → MarkdownNode.start_line   (1-indexed)
body line N (1-indexed)         → start_line + N            (1-indexed)
                                → start_line + N - 1        (0-indexed,
                                  matching DiagnosticEntry)
```

The diagnostic is clamped to the closing fence to guard against
parser-line numbers that overshoot the body. Severity is always
`Warning` and `source = Some("mermaid")` so other diagnostic consumers can
distinguish it from LSP output.

`app/central_panel.rs` appends these to `tab_diagnostics` (the same vec
already fed to LSP squiggles) for both the single-pane and split-view
editors, gated to markdown files. `FerriteEditor::render_diagnostic_squiggles`
draws the wavy underline as it does for LSP diagnostics — **no new
rendering code** required.

## Performance

- Validation reuses the per-diagram parsers, which are O(lines). For
  flowcharts the existing AST cache avoids re-parsing identical sources
  on the next frame.
- `compute_mermaid_diagnostics` calls `markdown::cache::get_or_parse`,
  which is content-hashed via blake3 — re-parsing only happens when the
  markdown content changes.
- Each frame currently re-runs the diagnostic walk. For very large
  documents with many blocks, this could be cached by `content_version`
  in a follow-up; the current walk is bounded by the AST's code-block
  count and is cheap in practice.

## Tests

`src/markdown/mermaid/validation.rs` ships 11 unit tests covering:

- Empty source / unknown diagram type / missing header
- Sequence-diagram error line extraction
- `Line N:` prefix stripping
- Bracket-balance and unclosed-subgraph hints
- `compute_mermaid_diagnostics` behaviour for valid / invalid /
  non-mermaid fenced blocks (including correct line mapping)

Run with:

```bash
cargo test --bin ferrite markdown::mermaid::validation::
```

## Files touched

| File | Change |
|---|---|
| `src/markdown/mermaid/validation.rs` | **New** — `MermaidError`, validation, diagnostic computation, hints, tests |
| `src/markdown/mermaid/mod.rs` | Register module + re-export public API |
| `src/markdown/mod.rs` | Re-export `compute_mermaid_diagnostics` |
| `src/markdown/widgets.rs` | `MermaidBlockData.last_good_source / last_error`, `show_validation_warning`, new render flow |
| `src/markdown/editor.rs` | Preserve last-good state when literal changes |
| `src/app/central_panel.rs` | Merge mermaid diagnostics into editor diagnostics (single + split views) |
| `locales/en.yaml` | New `mermaid.warning_line` key |

## Future work

- Per-`content_version` cache for `compute_mermaid_diagnostics` if profiling
  shows the AST walk is hot on huge documents.
- Column ranges (currently whole-line). Most parsers don't track columns
  yet; would need parser-level changes to expose them.
- "Quick fix" hover actions to apply hints automatically.
