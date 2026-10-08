# Mermaid parser-safe delimiter slicing

Half-typed or swapped brackets while editing a mermaid fence in Split view
must not panic. Release builds use `panic = abort`, and
`validate_mermaid_source` (editor squiggles / warning banner) is **not**
wrapped in `catch_unwind`.

## Helper

`slice_between` in `src/markdown/mermaid/parse_util.rs`:

```rust
fn slice_between<'a>(text: &'a str, open: &str, close: &str)
    -> Option<(&'a str, &'a str)>
```

Returns `(before, inner)` only when both delimiters are present **and**
`start + open.len() <= end` (`find` for `open`, `rfind` for `close`).
Otherwise `None`. Callers treat `None` exactly as “delimiters absent” and
fall through to the next node shape or the plain-id path.

Do **not** slice with independent `find` / `rfind` results. Inputs such as
`A}x{ --> B`, `subgraph a] [b`, and `A}}x{{ --> B` invert the range and
used to abort the process.

## Where it is applied

- Flowchart node shapes and `subgraph id [title]` headers
  (`src/markdown/mermaid/flowchart/parser.rs`)
- Class-diagram `<<stereotype>>`
- Mindmap `root ((…))` / `root (…)` wrapping
- Empty edge labels (`||`) use an exclusive slice (`text[1..end_pos + 1]`)
  so `1..=0` cannot invert

Layout/render `catch_unwind` in `render_mermaid_diagram` remains a
debug-friendly guard. Parse itself is unwrapped; release relies on
parser panic-freedom.

## Tests

Flowchart parser:

- `mangled_subgraph_brackets_do_not_panic`
- `mangled_diamond_brackets_do_not_panic`
- `mangled_hexagon_brackets_do_not_panic`
- `empty_edge_label_delimiters_do_not_panic`

Corpus (`mermaid_mangled_input_corpus_does_not_panic` in
`src/markdown/mermaid/mod.rs`): every ```` ```mermaid ```` fence in
`test_md/test_mermaid_*.md` and `test_md/test_flowcharts.md` is exercised
via `validate_mermaid_source` plus the matching `parse_*` on (a) every
newline-truncated prefix, (b) each individual deletion of
`[ ] { } ( ) | "`, (c) swapped bracket pairs. Each call is wrapped in
`catch_unwind` **inside the test only**.

## Manual QA

Minimal-release-test row 21: type a mermaid fence character by character
in Split view; a release build must not close.

## Related

- [`mermaid-inline-validation.md`](mermaid-inline-validation.md) —
  `validate_mermaid_source` (unguarded parse path)
- [`flowchart-crash-prevention.md`](flowchart-crash-prevention.md) —
  layout iteration limits and render `catch_unwind`
