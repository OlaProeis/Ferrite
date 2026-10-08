# Flowchart Subgraph Header Parsing

Bare multi-word and CJK subgraph titles are parsed to match mermaid.js: the full trailing text becomes the display title with an auto-generated id.

## Mermaid syntax

| Header form | Parsed id | Parsed title |
|-------------|-----------|--------------|
| `subgraph SingleToken` | `SingleToken` | `SingleToken` |
| `subgraph My Group Name` | `subgraph_N` | `My Group Name` |
| `subgraph 业务客户端 PEP` | `subgraph_N` | `业务客户端 PEP` |
| `subgraph id [Bracketed Title]` | `id` | `Bracketed Title` |
| `subgraph "Quoted Title"` | `subgraph_N` | `Quoted Title` |

Bare multi-word text without `[brackets]` or quotes is **not** split into id + title. Only a single whitespace-delimited token doubles as both id and title.

## Implementation

`parse_subgraph_header()` in `src/markdown/mermaid/flowchart/parser.rs`:

1. Strip the `subgraph` keyword and leading whitespace from the line.
2. Bracket form `id [title]` — id from text before `[`, title from inside brackets; empty id gets auto-id.
3. Quoted form — auto-id, title from quoted string.
4. Single token — token is both id and title (counter unchanged).
5. Two or more tokens — increment counter, id = `subgraph_{counter}`, title = full remainder string; the first token is recorded as a former id.

Auto-ids use the shared `subgraph_counter` passed from `parse_flowchart()` so ids stay unique within a diagram.

`warn_former_first_token_subgraph_edges` emits a `FlowchartWarning` when an edge endpoint equals that former first-token id (documents that still link `业务客户端` after `subgraph 业务客户端 PEP`). Use `subgraph id [Title]` to keep a stable id.

## Tests

Unit tests in `parser.rs` (`mod tests`):

- `parse_subgraph_header_bare_multi_word_cjk_title`
- `parse_subgraph_header_bare_multi_word_title`
- `parse_subgraph_header_single_token`
- `parse_subgraph_header_bracketed_title`
- `parse_subgraph_header_quoted_title`
- `edge_endpoint_matching_former_first_token_subgraph_id_warns`

## Related

- [Flowchart Subgraphs](./flowchart-subgraphs.md) — subgraph AST, layout, and rendering
- [Flowchart Subgraph Title Width Fix](./flowchart-subgraph-title.md) — title width in layout
