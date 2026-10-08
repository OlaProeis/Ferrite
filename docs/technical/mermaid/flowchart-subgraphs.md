# Flowchart Subgraph Support

This document describes the subgraph (cluster) support implemented in Task 18 for Ferrite's Mermaid flowchart rendering.

## Overview

Subgraphs allow grouping related nodes together in a flowchart with a visual container and optional title. This implementation supports:

- **Basic subgraphs**: `subgraph title` syntax
- **Named subgraphs**: `subgraph id [title]` syntax
- **Nested subgraphs**: Subgraphs within subgraphs
- **Direction overrides**: `direction TB/LR/etc` within subgraphs (parsed, not yet applied)

## Syntax Support

Subgraph headers resolve to an internal **id** (used for layout and nesting) and an optional **title** (shown in the container header). Ferrite supports four header forms:

| Header form | Parsed id | Parsed title |
|-------------|-----------|--------------|
| `subgraph id [Title]` | `id` (auto `subgraph_N` if id empty) | text inside `[…]` |
| `subgraph "Quoted Title"` / `'…'` | auto `subgraph_N` | quoted string |
| `subgraph SingleToken` | `SingleToken` | `SingleToken` |
| `subgraph Bare Multi Word Title` | auto `subgraph_N` | entire trailing string |

Bare multi-word text without `[brackets]` or quotes is **not** split into id + title. Only a single whitespace-delimited token doubles as both id and title. This matches mermaid.js and fixes [#165](https://github.com/OlaProeis/Ferrite/issues/165), where only the last token was previously used as the title.

### Explicit id and bracketed title

`subgraph id [Title]` — id from text before `[`, title from inside brackets.

```mermaid
flowchart TD
    subgraph sg1 [Service Layer]
        API[API Gateway]
        Auth[Auth Service]
    end
```

### Quoted title

`subgraph "Quoted Title"` or `subgraph 'Quoted Title'` — auto-generated id, quoted string as title.

```mermaid
flowchart TD
    subgraph "Production Environment"
        A[Node A]
    end
```

### Single-token header

`subgraph SingleToken` — the token is both id and title (no auto-id).

```mermaid
flowchart TD
    subgraph MyGroup
        A[Node A]
        B[Node B]
    end
```

### Bare multi-word title

`subgraph Bare Multi Word Title` — auto id (`subgraph_1`, `subgraph_2`, …) and the **full** remainder as title. CJK and mixed scripts are preserved:

```mermaid
flowchart TD
    subgraph 业务客户端 PEP
        A[Node A]
    end
```

```mermaid
flowchart TD
    subgraph My Group Name
        A[Node A]
    end
```

### Nested Subgraphs
```mermaid
flowchart TD
    subgraph Outer
        subgraph Inner
            A[Node A]
        end
        B[Node B]
    end
```

## Implementation Details

### AST Types

```rust
/// A subgraph (cluster) in a flowchart.
pub struct FlowSubgraph {
    /// Unique identifier for the subgraph
    pub id: String,
    /// Display title (may differ from id)
    pub title: Option<String>,
    /// IDs of nodes directly contained in this subgraph
    pub node_ids: Vec<String>,
    /// IDs of nested subgraphs
    pub child_subgraph_ids: Vec<String>,
    /// Optional direction override for this subgraph
    pub direction: Option<FlowDirection>,
}
```

### Parser

Header id/title resolution is implemented in `parse_subgraph_header()` (`src/markdown/mermaid/flowchart/parser.rs`):

1. Strip the `subgraph` keyword and leading whitespace.
2. **Bracket form** — id from text before `[`, title from inside brackets; empty id gets auto-id.
3. **Quoted form** — auto-id, title from quoted string.
4. **Single token** — token is both id and title (counter unchanged).
5. **Two or more tokens** — increment counter, id = `subgraph_{counter}`, title = full remainder string (including CJK).

Auto-ids use a shared `subgraph_counter` from `parse_flowchart()` so ids stay unique within a diagram.

Block structure uses a stack-based approach:

1. When `subgraph` keyword is encountered, push a new `SubgraphBuilder` onto the stack
2. Associate nodes/edges with the current (top of stack) subgraph
3. When `end` keyword is encountered, pop the builder and create the subgraph
4. Register nested subgraphs as children of their parent

See also [Flowchart Subgraph Header Parsing](./flowchart-subgraph-header-parsing.md) for the full parsing reference and unit tests.

### Layout

Subgraph bounding boxes are computed after node positions are determined:

1. For each subgraph (processing children before parents):
   - Calculate min/max bounds of all member nodes
   - Include bounds of nested subgraphs
   - Add padding around content
   - Add space for title at top

```rust
pub struct SubgraphLayout {
    /// Bounding box position (top-left corner)
    pub pos: Pos2,
    /// Bounding box size
    pub size: Vec2,
    /// Title to display (if any)
    pub title: Option<String>,
}
```

### Rendering

Subgraphs are rendered as the first layer (behind edges and nodes):

1. Draw semi-transparent rounded rectangle as background
2. Draw title text in top-left corner if present
3. Parent subgraphs drawn before children to layer correctly

## Configuration

Layout configuration for subgraphs:

| Parameter | Default | Description |
|-----------|---------|-------------|
| `subgraph_padding` | 15.0 | Padding around subgraph content |
| `subgraph_title_height` | 24.0 | Height reserved for title |

## Colors

Theme colors for subgraphs:

| Color | Dark Theme | Light Theme |
|-------|------------|-------------|
| `subgraph_fill` | rgba(60, 70, 90, 40) | rgba(200, 210, 230, 60) |
| `subgraph_stroke` | rgb(80, 100, 130) | rgb(150, 170, 200) |
| `subgraph_title` | rgb(160, 175, 195) | rgb(80, 95, 120) |

## Limitations

1. **Direction overrides**: Parsed but not yet applied to layout
2. **Edge to subgraph**: Edges connecting directly to subgraphs (not individual nodes) are not yet supported
3. **Subgraph styling**: Custom per-subgraph styling is not supported

## Future Enhancements

- Apply direction overrides within subgraphs
- Support edges to/from subgraph ids
- Custom styling per subgraph
- Collapse/expand subgraphs interactively

## Files

- **Implementation**: `src/markdown/mermaid.rs`
- **Key types**: `FlowSubgraph`, `SubgraphLayout`, `SubgraphBuilder`
- **Key functions**: `parse_subgraph_header()`, `compute_subgraph_layouts()`, `draw_subgraph()`
