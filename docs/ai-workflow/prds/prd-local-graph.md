# Ferrite — Local Graph (PRD)

> **Status:** Feature PRD. Not scheduled inside v0.3.2.
> **Version target:** v0.3.3 candidate. v0.3.2 is already committed to LSP, the FerriteEditor crate, the Mermaid crate, and GitHub HTML Phase 3 (`ROADMAP.md`). This feature waits for the release after that.
> **Consumers:** Human review, then a cyclopsctl task queue once a release slot exists.
> **Hard constraint:** Native egui painting only. No `egui_graphs`, no `fdg-sim`, no Graphviz, no WebView.

## 1. Overview

Obsidian's graph view draws each note as a circle and each internal link as a line. The full-vault picture is occasionally useful for hubs, clusters, and notes nothing links to. The view people keep open is the **local graph**: the note in front of them, plus the notes one or two hops away.

Ferrite already has half of that data. `BacklinkIndex` (`src/state.rs`) scans a workspace for `[[wikilinks]]` and local `[text](note.md)` links, and `BacklinksPanel` lists the files that point at the active note. What is missing is the forward direction, a small subgraph around the active note, and a drawing of it.

**Theme:** "What else touches this note?" A local graph beside the editor, built from the link index Ferrite already maintains, laid out once and then left still.

### Pillars

1. **Local graph (headline)** — the active note in the center, direct links on the first ring, one optional second hop.
2. **Same links the backlinks panel already trusts** — wikilinks and local markdown links, plus a forward index so outgoing links are visible too.
3. **Still when it is done** — positions are computed when the note, depth, or filters change, then painting is static. No per-frame physics.
4. **A global graph only after the local one is real** — and only with a hard size cap, a static layout, and orphans hidden.

### Why this shape

| Choice | Decision |
|--------|----------|
| First view | Local graph. Depth 1 by default, depth 2 optional. Depth stops at 2. |
| Layout | Concentric rings, sorted by display name so the picture does not shuffle. |
| Simulation | None in v1. A force layout is a later option for the global graph, and it runs to completion off the UI thread, then stops. |
| Widget crate | None. `egui_graphs` 0.32 needs egui 0.36; Ferrite is on egui 0.34. Its repulsion is O(n²) and it does not provide depth, filters, or "open this note." `fdg-sim` 0.9.1 last shipped in 2022. |
| Node identity | A workspace markdown file, keyed by its path. Link *resolution* stays the stem match `BacklinkIndex` uses today. |
| Where it lives | A **Graph** tab on the outline side panel, plus a command-palette command. |

Distance on the rings is layout, not similarity. Two circles sit near each other because they share a hop count, not because the notes are about the same thing. Only a line means a link.

---

## 2. Goals

- From the active markdown file in a workspace, the user can open a graph of that file and the notes it links to or that link to it.
- Clicking a note circle opens that file, the same way a backlink row does.
- Hovering a circle highlights its lines and shows the file name.
- The user can include or drop incoming links, outgoing links, and links to notes that do not exist yet, and can add one extra hop.
- Changing notes, saving a file, or toggling a filter updates the graph. Once drawn, it does not keep animating.
- The graph uses the workspace link index. It does not re-read the workspace on every frame.
- A workspace with no links, a single file with no workspace, and a missing note each have a clear empty or ghost state.

## 3. Non-goals

- **No full-vault graph in v1.** Tier C describes it so the local design does not block it. It does not ship with the local graph.
- **No force simulation, no time-lapse, no tag nodes, no attachment nodes, no color groups, no search box inside the graph.** Those are Obsidian display options. They are not required to answer "what else touches this?"
- **No drag-to-pin layout, no writing positions back to the note.**
- **No new link syntax.** The graph reads the links Ferrite already extracts. It does not add block references, embeds-as-nodes, or a new wikilink resolver.
- **No path-aware wikilink resolution.** Two files that share a stem still resolve to one key, matching `normalize_filename` today. Fixing that changes the backlinks panel too and is a separate change.
- **No graph for CSV, terminal, settings, or other non-markdown tabs.**
- **No `egui_graphs` or other graph crate**, including after an egui bump, unless a later PRD says the crate replaces the painter and the rest of this behavior stays in Ferrite.

---

## 4. Current state (verified in code, 2026-11-12)

### 4.1 Backlink index

`BacklinkIndex` (`src/state.rs`) is a `HashMap` from a normalized stem to the files that link to it.

- `extract_links_from_content` collects `[[target]]`, `[[target|display]]` (text before `|`), and `[text](….md)` / `.markdown` links that are not `http`, `https`, or `#` anchors. The markdown-link path keeps only the file name.
- `normalize_filename` trims, lowercases, and strips a trailing `.md` / `.markdown`.
- The index stores incoming links only. There is no "this file links out to these stems" map.
- Workspaces of about 50 files or fewer call `scan_on_demand` when the active file changes. Larger workspaces call `build_from_files` once and `update_file` on save (`src/app/navigation.rs`, `refresh_backlinks`).
- Matching is stem-only. `notes/a/readme.md` and `notes/b/readme.md` are one key.
- The extractor does not skip fenced code or inline code, and it does not strip `#heading` or `#^block` from a wikilink. `[[Note#Section]]` is stored as `note#section`, which does not match a file named `Note`.

### 4.2 Backlinks panel

`BacklinksPanel` (`src/ui/backlinks_panel.rs`) is a list inside `OutlinePanelTab::Backlinks` (`src/ui/outline_panel.rs`). A click sets `OutlinePanelOutput.backlink_navigate_to`. With no panel, the tab shows `outline.backlinks_unavailable`.

### 4.3 Commands and settings

Commands live in `src/app/commands.rs` and dispatch from `src/app/central_panel.rs`. New settings fields go through `src/config/settings.rs` with `#[serde(default)]` so an older `config.json` still loads (`from_json_tolerant`).

---

## 5. Functional requirements — Tier A (must ship)

### 5.1 Link graph beside the backlink index

**Problem.** Incoming links are indexed. A graph needs both directions, a stable file node, and cleaner targets than the raw extractor output.

**Behavior.**

- Add a forward map next to `BacklinkIndex`: source path → outgoing targets. Build it in the same scan as the reverse map (`build_from_files`, `update_file`, and the on-demand path). Do not keep a second full-workspace walk.
- A **node** is either:
  - a markdown file in the workspace (identity = absolute path, label = file stem), or
  - an **unresolved** target (identity = the normalized stem, no path).
- A **target** is normalized in this order: take the text before `|`, drop a `#…` or `#^…` suffix, then `normalize_filename`. `[[Note#Section|alias]]` resolves as `note`.
- Ignore a target whose stem, after that cleanup, has an extension other than `md` / `markdown` or no note-like stem (so `![[diagram.png]]` and `[[image.png]]` are not nodes).
- Skip `[[…]]` and markdown links that sit inside a fenced code block. A line-oriented fence tracker is enough (` ``` ` / `~~~` toggles). Inline code spans are best-effort; a missed span is acceptable.
- Collapse duplicate edges from the same source to the same target. A self-link (a file linking to its own stem) is dropped.
- When several workspace files share a stem, every one of them is a node, and an edge to that stem attaches to **each** of them. The label shows the parent folder when the stem is ambiguous (`readme` → `a/readme`, `b/readme`). This keeps today's resolver and stops two files from drawing as one circle.

**Key files.** `src/state.rs` (`BacklinkIndex`, `extract_links_from_content`, `normalize_filename`). New graph types can live in `src/graph/` (`mod.rs` for the subgraph query, `layout.rs` for rings) so `state.rs` does not grow by a painter.

**Acceptance.**

1. Given notes `A → B`, `C → B`, `B → D`, a query for `B` returns incoming `A` and `C` and outgoing `D`.
2. `[[B#Heading|see]]`, `[[B]]`, and `[x](B.md)` are one edge to `B`.
3. A wikilink inside a ``` fence does not create an edge. The same text outside the fence does.
4. `![[shot.png]]` creates no node.
5. Saving `A` updates only `A`'s outgoing edges and the reverse entries whose source is `A`, matching `update_file`.
6. Two files named `readme.md` stay two nodes, and a link to `[[readme]]` draws a line to both.

**Docs.** `docs/technical/ui/local-graph.md` covers the index shape and the heading-strip rule. Update `docs/index.md` with one bullet.

### 5.2 Local subgraph

**Problem.** The index is the whole workspace. The view is one neighborhood.

**Behavior.**

- Center = the active tab's file, when that tab is a markdown file in the open workspace.
- **Depth 1:** the center plus nodes with a direct incoming or outgoing edge.
- **Depth 2:** also the nodes one more hop out from those neighbors. Edges between a depth-2 node and a node outside the set are not drawn.
- Direction filters apply before the hop walk:
  - **Incoming** off → do not walk edges that point at the current node from elsewhere.
  - **Outgoing** off → do not walk edges that leave the current node.
  - At depth 2 the filters apply to the center only. Neighbor-to-neighbor edges are not drawn in v1 (see non-goals and Tier B).
- **Unresolved** off → drop ghost nodes. If that leaves a depth-2 node with no path back to the center, drop it too.
- Cap the drawn set at **200 nodes**. Fill the center, then depth 1 (name order), then depth 2 (name order). If the cap cuts the set, the panel shows a short count ("showing 200") and does not keep expanding.
- Rebuild the subgraph when the center path changes, the depth or filters change, or the index update touches the center or a node already in the set. Otherwise keep the previous result.

**Acceptance.**

1. Depth 1 around `B` in the example above is `{A, B, C, D}` and no further notes those four link to.
2. Depth 2 includes notes linked from `A`, `C`, or `D`, and not notes two hops beyond those.
3. Incoming off, outgoing on, depth 1 around `B` is `{B, D}`.
4. A 201st node is omitted and the cap message is visible.
5. Switching from `B` to `A` rebuilds. An unrelated file save does not.

### 5.3 Concentric layout

**Problem.** The subgraph needs positions that stay put and stay readable in a side panel.

**Behavior.**

- Place the center at the middle of the graph rect.
- Place depth-1 nodes evenly on a circle, depth-2 nodes evenly on a larger circle. Sort each ring by label (case-insensitive) so the order is stable.
- Draw a straight line for each edge in the subgraph. No arrows in v1.
- If a ring has no nodes, skip it. A center with no links draws the center alone and the empty-state line from §5.5.
- Labels are the display stem (or `folder/stem` when ambiguous). Draw a label when it fits without crossing the next label on that ring; otherwise show it on hover only. The center label is always drawn.
- The layout function is pure: nodes in, positions out. No time step, no stored velocity.
- Colors come from the active theme. Center uses the accent. Existing files use the primary text color. Unresolved nodes use the muted color and a dashed or hollow circle. Incoming and outgoing lines may share one color in v1; they must both remain visible on the dark and light themes.

**Acceptance.**

1. The same subgraph produces the same positions every time.
2. Depth 1 nodes sit on one radius, depth 2 on a larger radius, center in the middle.
3. An unresolved node is visually distinct from a file node in both themes.
4. With 30 nodes on a ring, labels that would overlap are hidden until hover, and the center label stays.

### 5.4 Graph tab and command

**Problem.** The picture needs a place to live that matches how Ferrite already shows structure.

**Behavior.**

- Add `OutlinePanelTab::Graph` beside Backlinks. The tab shows the local graph for the active markdown file.
- Command palette entry **Open local graph** (category with the other panel commands in `src/app/commands.rs`). It focuses the outline side panel, selects the Graph tab, and does nothing else.
- **Click** a file node opens that path through the same navigation the backlinks panel uses (`backlink_navigate_to` or the existing open-file path). Clicking the center does not reload the file.
- **Click** an unresolved node does nothing in v1 except select it (hover already shows the name). Creating the note is Tier B.
- **Hover** highlights that node's lines and shows the full label in a tooltip, including "note does not exist" for a ghost.
- **Drag** pans. **Scroll** zooms, clamped so a ring cannot invert or grow without bound (for example 0.5×–2.5× relative to the fitted view). Releasing the pointer leaves the pan where it is. No inertia.
- On first open, and when the center changes, fit the rings inside the panel with padding. Pan and zoom persist for the session while the same center stays open, and reset on center change.
- Depth control: a two-step control, **1** and **2**, default **1**. Toggles: **Incoming**, **Outgoing**, **Unresolved**. All three default on.
- Persist those four values on `Settings` with `#[serde(default)]` so a missing field uses the defaults. Toggles live on the graph tab, not as a new Settings search section.
- Locale keys for the tab label, command name, toggles, empty states, cap message, and ghost tooltip. Add them to every shipped `locales/*.yaml` file, following `Language::all()`.

**Key files.** `src/ui/outline_panel.rs`, new `src/ui/graph_panel.rs`, `src/app/commands.rs`, `src/app/central_panel.rs` (dispatch), `src/config/settings.rs`, `locales/*.yaml`.

**Acceptance.**

1. **Open local graph** reveals the outline panel on the Graph tab.
2. Clicking a neighbor opens that file and moves the center to it on the next rebuild.
3. Incoming off removes lines that only exist as backlinks, and the picture updates without an app restart.
4. Wheel zoom and drag pan do not move the editor caret or scroll the document behind the panel.
5. Restarting Ferrite restores the last depth and the three toggles.
6. An old `config.json` without the new fields still loads, and the graph uses the defaults.
7. The new strings exist in every shipped locale file. English is the source text; other locales may start as the English string.

### 5.5 Empty and blocked states

**Behavior.**

- No workspace folder, or the active tab is not a workspace markdown file: the tab shows one line that a workspace note is required. No graph, no scan.
- Workspace note with no links after filters: draw the center node and one line, "no links".
- Index not built yet for a large workspace: show the center and "indexing…" until `is_built`, then draw. Do not block the UI thread with a fresh full scan caused by opening the tab. Opening the tab uses the index `refresh_backlinks` already maintains.
- The Graph tab is available in single-file mode and shows the workspace-required line. It does not invent links from the one open buffer.

**Acceptance.**

1. Opening the tab with no folder shows the workspace line and does not read arbitrary directories.
2. A note that neither links nor is linked shows only itself and "no links".
3. Opening the tab does not call `build_from_files` if the index is already built.

---

## 6. Tier B (ship if Tier A is in and the release still has room)

### 6.1 Neighbor links

A toggle **Links between neighbors**, default off. When on, depth 1 draws edges whose both ends are in the depth-1 set. Depth 2 may draw edges between visible nodes that are not the center. The toggle is session-persisted with the others.

### 6.2 Create a missing note

Clicking an unresolved node offers **Create note** (button or context row). It creates `<stem>.md` in the same directory as the center file, opens it, and lets the next index update turn the ghost into a file node. No create action for an ambiguous stem.

### 6.3 Arrows

A toggle **Arrows**, default off. Arrowhead points from the file that contains the link toward the target. Incoming and outgoing stay one data model.

---

## 7. Tier C (separate follow-up, not part of the v1 task queue)

### 7.1 Global graph

A second command, **Open vault graph**, draws every markdown file in the workspace.

- Orphans (no incoming and no outgoing) are hidden by default, with a toggle to show them.
- Color nodes by their parent folder, one color per folder, stable for the session. No user-authored color groups.
- If the workspace has more than **400** markdown files, do not draw it. Show the count and point the user at the local graph.
- Layout runs on a worker: a simple Fruchterman–Reingold (or the concentric layout grouped by folder, if that is the smaller code path) for a bounded number of steps, then the positions are frozen and the UI only paints. Cancel the worker if the user closes the view or switches workspace.
- Still no tags, attachments, time-lapse, or search syntax.
- This tier gets its own task breakdown after Tier A has shipped. Do not start it inside the local-graph queue.

---

## 8. Performance and memory

- The UI thread paints lines and circles. It does not integrate forces.
- The local layout is O(n) in the capped subgraph (n ≤ 200) and runs on a change, not per frame.
- The index stores paths and stems, not file bodies. File bodies are read inside the existing scan and dropped.
- Painting a still graph requests no extra repaint. A hover, pan, or zoom requests one, the same way other panels do.
- Opening the Graph tab must not add a workspace walk beyond `refresh_backlinks`.

---

## 9. Testing

Automated, targeted:

- Extractor tests next to the existing `test_extract_wikilinks_from_content` cases: alias, `#heading`, fenced code, image embed, markdown link, self-link, duplicate edge.
- Subgraph tests: depth 1, depth 2, incoming off, outgoing off, unresolved off, 200-node cap, ambiguous stem.
- Layout test: deterministic positions for a fixed node list; depth-2 radius greater than depth-1 radius; center at the origin of the layout space.

Manual, once the tab exists:

- Workspace of about 20 linked notes: depth 1 and 2, each toggle, click-through, hover, pan, zoom, dark and light.
- One note with no links. One `[[missing]]` ghost, toggled off and on.
- Two files with the same stem. A wikilink inside a code fence that must not appear.
- Save a note that adds a link and confirm the open graph updates.
- Open the tab with no workspace.

No screenshot harness and no soak run. `cargo test` filters cover the extractor, subgraph, and layout modules.

---

## 10. Suggested task split

When this is parsed into a queue, use this order. Each item is one parent task.

1. **Link graph data** — forward map, target cleanup, fence skip, ambiguous-stem nodes, tests (§5.1).
2. **Subgraph query** — depth, direction, unresolved, cap, tests (§5.2).
3. **Ring layout** — pure function plus tests (§5.3).
4. **Graph tab** — paint, hover, click-to-open, pan/zoom, fit-on-center-change, empty states (§5.4, §5.5).
5. **Command, settings, locales** — palette command, persisted toggles, locale keys, tolerant-load default (§5.4).
6. **Docs** — `docs/technical/ui/local-graph.md` and the `docs/index.md` bullet.

Tier B items are separate tasks after 1–6, only if the release slot remains. Tier C is not in this split.
