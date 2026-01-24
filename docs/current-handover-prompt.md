# Handover: Custom Text Editor Planning (v0.2.6)

## Rules
- Never auto-update this file - only update when explicitly requested
- Complete entire task before requesting next instruction
- Run `cargo build` / `cargo check` after changes to verify code compiles
- Follow existing code patterns and conventions
- Update task status via Task Master when starting (`in-progress`) and completing (`done`)
- Use Context7 MCP tool to fetch library documentation when needed
- Document by feature (e.g., `memory-optimization.md`), not by task
- Update `docs/index.md` when adding new documentation
- **Use MCP tools** for Task Master operations, not CLI
- **Avoid `git diff`** - causes disconnections

---

## Current Task

**Custom Text Editor Architecture Planning - v0.2.6**

- **Status**: PLANNING
- **Priority**: critical
- **Goal**: Design and plan a custom text editor widget to replace egui's TextEdit

### Why This Is Needed

egui's `TextEdit` widget is fundamentally incompatible with large files. It creates a **Galley** structure that stores layout information (positions, glyphs, metrics) for **every character** in the file. For a 4MB file, this alone uses ~500MB-1GB RAM.

**No amount of optimization on Ferrite's side can fix this** — we must replace the underlying text widget.

See GitHub Issue: [#45](https://github.com/OlaProeis/Ferrite/issues/45)

---

## Planning Phase Goals

### 1. Research & Analysis
- [ ] Read existing custom editor plan: `docs/technical/planning/custom-editor-widget-plan.md`
- [ ] Analyze current `src/editor/widget.rs` to understand what features we need to preserve
- [ ] Research egui's low-level drawing primitives (Painter, Galley, etc.)
- [ ] Research `ropey` crate for rope-based text buffer
- [ ] Look at how other Rust editors handle virtual scrolling (if any)

### 2. Architecture Design
- [ ] Define the `FerriteEditor` widget interface
- [ ] Design the text buffer (ropey integration)
- [ ] Plan virtual scrolling implementation (only render visible lines)
- [ ] Plan cursor/selection handling without full document layout
- [ ] Plan syntax highlighting integration (per-line, visible only)
- [ ] Plan undo/redo with rope-based buffer

### 3. Create Implementation Plan
- [ ] Break down into incremental milestones
- [ ] Identify what can be done in parallel vs sequential
- [ ] Estimate complexity of each component
- [ ] Document the plan in `docs/technical/planning/`

---

## Key Architectural Questions

1. **Text Storage**: How to integrate `ropey` with our existing Tab/content model?
2. **Rendering**: Line-by-line galley creation vs caching strategies?
3. **Scrolling**: How to track scroll position by line number, not pixels?
4. **Selection**: How to handle selection spanning off-screen content?
5. **IME/Input**: How does egui handle text input at low level?
6. **Compatibility**: How to maintain feature parity with current editor?

---

## Existing Documentation to Review

| File | Purpose |
|------|---------|
| `docs/technical/planning/custom-editor-widget-plan.md` | Existing high-level plan |
| `src/editor/widget.rs` | Current EditorWidget implementation |
| `src/editor/mod.rs` | Editor module structure |
| `src/state.rs` | Tab struct, content management |
| `src/markdown/syntax.rs` | Syntax highlighting |

---

## Memory Target

**Before (current):** 4MB file uses ~500MB-1GB (egui Galley)  
**After (target):** 4MB file uses ~10-20MB (content + visible lines only)

---

## Environment
- **Project**: Ferrite (Markdown editor)
- **Language**: Rust
- **GUI Framework**: egui 0.28
- **Version**: v0.2.6 (planned)

---

## Related Links
- [Custom Editor Plan](docs/technical/planning/custom-editor-widget-plan.md)
- [Memory Optimization](docs/technical/planning/memory-optimization.md)
- [egui docs](https://docs.rs/egui/latest/egui/)
- [ropey docs](https://docs.rs/ropey/latest/ropey/)
