# Mermaid v0.3.1 Test Fixture

Used by the v0.3.1 manual test checklist (`MMD-*`). Covers the second-wave
additions: diagram popup viewer, FC-83b icon labels, `linkStyle interpolate`,
and subgraph-title width stability (#159). For `@pos` use
[`test_pos_hints.md`](test_pos_hints.md); for git graphs use
[`test_git_graphs.md`](test_git_graphs.md).

## 1. Popup viewer (click / Ctrl+Click / popup button)

```mermaid
flowchart TD
    A[Start] --> B{Big decision with a long label}
    B -->|Yes| C[Process step one]
    B -->|No| D[Process step two]
    C --> E[Converge]
    D --> E
    E --> F[Finish]
```

**Expect:** hovering shows an *Open diagram in popup* button; clicking the
diagram (or Ctrl+Click, or the button) opens the zoom/pan popup. In the popup:
scroll/buttons zoom, **Hand** mode pans, **Select** mode does not, reset button
restores fit, close (X) dismisses. Dark/light follows the app theme.

## 2. FC-83b — Font Awesome icon prefixes stripped

```mermaid
flowchart LR
    A[fa:fa-spinner Loading] --> B[fab:fa-github Repo]
    B --> C[fa:fa-check Done]
```

**Expect:** labels render as **Loading**, **Repo**, **Done** — the `fa:fa-*` /
`fab:fa-*` prefix is stripped (glyph not drawn), no literal `fa:fa-` text.

## 3. linkStyle interpolate basis (smooth curves)

```mermaid
flowchart LR
    A --> B
    A --> C
    B --> D
    C --> D
    linkStyle 0,1,2,3 interpolate basis
```

**Expect:** each of the four edges is a smooth S-curve: it leaves its source
node and enters its target node horizontally (the `LR` flow direction) and
bends in between. Compare with the same diagram without the `linkStyle` line,
where the same edges are straight diagonal lines.

Note: a straight chain such as `A --> B --> C --> D` in `LR` has no bends at
all, so `interpolate basis` cannot change its appearance — in Mermaid either.
Use a diagram with fan-out/fan-in (as above) to verify curve rendering.

## 4. Subgraph title width stability (#159)

```mermaid
flowchart TB
    subgraph S1[This subgraph title is intentionally much wider than its nodes]
        N1[a] --> N2[b]
    end
    subgraph S2[Short]
        M1[node one] --> M2[node two] --> M3[node three]
    end
    S1 --> S2
```

**Expect:** each subgraph box is wide enough for its title; the width is
**stable** across frames (no flicker/oscillation) when the title is the widest
element. Open in the popup (test 1) and confirm it stays stable while zooming.
