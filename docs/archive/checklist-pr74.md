# Pre-Merge Review Checklist for PR #74

**Version:** v0.5.0 Swiss Army Knife  
**Date:** February 3, 2026  
**Reviewer:** OlaProeis  
**Status:** Testing in `test-pr-74` branch

---

## Executive Summary

PR #74 introduces significant functionality (integrated terminal, productivity hub, async foundation) but increases baseline memory usage from ~75MB to ~300MB (4x increase). Before merging into the 0.2.6.1 release, we need to investigate performance implications, add debugging capabilities, refine UI integration, and validate production readiness.

---

## Critical Issues to Address

### 1. Memory Usage Investigation (HIGH PRIORITY)

**Current Observation:**

- Baseline memory: ~75MB (pre-PR)
- Current memory: ~300MB (post-PR)
- **4x increase is unacceptable for production**

**Investigation Tasks:**

- [ ] **Profile memory allocation with tooling**
  - Run with `cargo instruments` (macOS) or `heaptrack` (Linux) or Windows Performance Analyzer
  - Identify top memory consumers (PTY buffers? Screen buffer? Task storage?)
  - Generate flame graph of allocations
- [ ] **Check terminal screen buffer size**
  - Current scrollback limit (unlimited?)
  - Propose: Cap at 10,000 lines per terminal with circular buffer
  - Verify buffer is deallocated when terminal tab closes
- [ ] **Review Tokio runtime overhead**
  - Is runtime spawning excessive threads?
  - Check `tokio::runtime::Builder` configuration
  - Consider: `.worker_threads(2)` instead of default multi-thread
- [ ] **Audit PTY handle leaks**
  - Verify `portable-pty` handles are properly dropped
  - Check for lingering file descriptors on Windows
  - Test: Create/close 50 terminals rapidly, monitor memory
- [ ] **Test workspace with large task lists**
  - Create 1,000+ tasks in `.ferrite/tasks.json`
  - Measure memory impact
  - Implement pagination if needed (show first 100, "Load more...")

**Acceptance Criteria:**

- Memory usage ≤ 150MB baseline (2x increase acceptable, 4x is not)
- No memory leaks after creating/destroying 100 terminal tabs
- Document memory bounds in README

---

### 2. Lazy Loading & Initialization Optimization

**Current Implementation:**

- Workers lazy-load on first panel visibility ✅
- PTY spawns immediately on terminal open ✅
- Screen buffers allocated upfront ❓

**Optimization Tasks:**

- [ ] **Defer PTY spawning until terminal tab is active**
  - Don't spawn shell process for background tabs
  - Spawn on-demand when tab receives focus
  - Expected savings: ~10-20MB per unused terminal
- [ ] **Lazy-load productivity hub data**
  - Don't parse `tasks.json` until Ctrl+Shift+H pressed
  - Don't load notes from disk until panel shown
  - Benchmark: Startup time before/after
- [ ] **Reduce VTE parser allocations**
  - Check if `vte` crate buffers are resizable
  - Consider fixed-size circular buffer for ANSI sequences
- [ ] **Profile cold start time**
  - Measure time from launch to first frame
  - Target: <500ms on mid-range hardware
  - Identify blocking operations during init

**Acceptance Criteria:**

- App launches in <500ms (cold start)
- Memory only increases when panels are actually used
- No upfront cost for features user doesn't enable

---

### 3. Metrics & Diagnostics Panel (NEW FEATURE)

**Rationale:**

- No visibility into background worker health
- Can't diagnose performance issues without instrumentation
- Need real-time stats for debugging memory/CPU

**Implementation Plan:**

- [ ] **Create `DiagnosticsPanel` struct**
  - Toggle with `Ctrl+Shift+D` or toolbar icon
  - Show in floating window (similar to Productivity Hub)
- [ ] **Track worker metrics**

  ```rust
  struct WorkerMetrics {
      spawn_time: Instant,
      commands_processed: u64,
      responses_sent: u64,
      last_activity: Instant,
      status: WorkerStatus, // Idle, Active, Error
  }
  ```

- [ ] **Display terminal statistics**
  - Active terminals count
  - Total PTY processes
  - Screen buffer sizes (lines × bytes)
  - Aggregate memory estimate
- [ ] **Show productivity hub stats**
  - Task count (total, completed, pending)
  - Notes count and total size (KB)
  - Last auto-save timestamp
- [ ] **Add system resource monitoring**
  - Current app memory usage (from OS)
  - Frame time (ms per frame)
  - Repaint requests per second
  - CPU usage (if available via sysinfo crate)
- [ ] **Log export functionality**
  - "Export Diagnostics" button
  - Save to `.ferrite/diagnostics-{timestamp}.json`
  - Include all metrics + git commit hash

**UI Mockup:**

```
┌─ Diagnostics ─────────────────────────┐
│ Memory: 147 MB                        │
│ Frame Time: 8.3ms (120 FPS)           │
│                                       │
│ Workers:                              │
│  -  Echo Worker: Active (23 cmds)     │
│  -  AI Worker: Not Loaded             │
│                                       │
│ Terminals:                            │
│  -  Active Tabs: 3                    │
│  -  Buffer Size: 42 MB                │
│  -  Total Lines: 8,247                │
│                                       │
│ Productivity:                         │
│  -  Tasks: 12 (5 completed)           │
│  -  Notes: 3 files (8.2 KB)           │
│  -  Last Save: 2s ago                 │
│                                       │
│ [Export Diagnostics] [Clear Logs]     │
└───────────────────────────────────────┘
```

**Acceptance Criteria:**

- Panel shows real-time updates (refresh every 500ms)
- Export functionality works without crashing
- Metrics accurate within ±5%

---

### 4. UI Polish & Integration Review

**Current Issues:**

- View menu feels tacked on (not integrated with existing UI patterns)
- Floating windows may conflict with workspace layouts
- Panel toggles need better discoverability
- "Coming Soon" items pollute the UI

**UI Redesign Tasks:**

#### 4.1 Remove View Menu

- [ ] Remove the View menu bar
- [ ] Delete View menu implementation
- [ ] Current panel toggles need new home
- [ ] Remove "Coming Soon" placeholders (AI Assistant, Database Tools, SSH Sessions)

#### 4.2 Integrate Panels into Existing UI

**Option A: Toolbar Icons (Recommended)**

- Add toolbar icons next to existing buttons
- 📋 Tasks icon (Productivity Hub toggle)
- 📊 Diagnostics icon (when implemented)
- Keep existing >_ terminal toggle
- Use tooltips to show keyboard shortcuts

**Option B: Status Bar Integration**

- Add clickable status bar sections (bottom of window)
- Left section: Terminal indicator (click to toggle)
- Center section: Task progress (5/12 tasks) — click to open hub
- Right section: System stats (memory/CPU) — click for diagnostics

**Option C: Command Palette**

- Implement Ctrl+Shift+P command palette (VS Code style)
- Searchable list of all commands
- Type "tasks", "terminal", "productivity"
- Shows keyboard shortcuts next to commands
- Fuzzy search for quick access

**Option D: Sidebar Panel Switcher**

- Add vertical icon bar on left/right edge
- File tree icon (existing)
- Terminal icon
- Tasks/Productivity icon
- Diagnostics icon
- Click to show/hide panels in dedicated area

#### 4.3 Panel Layout Strategy

**Current:** Floating windows for Productivity Hub (feels disconnected)

**Proposed Improvements:**

- Dockable panels — Drag panels to dock at bottom/sides
- Tab groups — Terminal and Tasks share bottom panel with tabs
- Persistent layout — Remember panel positions per workspace
- Quick toggle animations — Smooth slide-in/out transitions

**Layout Mockup (Bottom Panel with Tabs):**

```
┌─ Editor Area ─────────────────────────────┐
│                                           │
│     [Your Code Here]                      │
│                                           │
├─ Bottom Panel ────────────────────────────┤
│ [Terminal] [Tasks] [Diagnostics]          │
│ ┌────────────────────────────────────────┐│
│ │ $ cargo build                          ││
│ │ Compiling ferrite v0.2.6.1             ││
│ │                                        ││
│ └────────────────────────────────────────┘│
└───────────────────────────────────────────┘
```

#### 4.4 Keyboard Shortcut Consistency

- [ ] Review all shortcuts for conflicts
  - Current: Ctrl+` (terminal), Ctrl+Shift+H (productivity)
  - Proposed: Standardize pattern (Ctrl+Shift+T, Ctrl+Shift+P, Ctrl+Shift+D)
  - Document in UI (tooltip hints)
- [ ] Add shortcut customization
  - Settings panel for rebinding keys
  - Export/import keybinding profiles
  - Conflict detection and warnings

#### 4.5 Visual Consistency

- [ ] Audit panel styling
  - Use consistent padding, borders, colors across panels
  - Match existing Ferrite design language
  - Dark/light theme support for new panels
- [ ] Icon design
  - Create consistent icon set (or use existing library like Lucide)
  - Same size/style for toolbar and panel tabs
  - Support high-DPI displays
- [ ] Typography hierarchy
  - Panel titles use same font as editor tabs
  - Section headers consistent size
  - Monospace for terminal, sans-serif for tasks

#### 4.6 Accessibility

- [ ] Keyboard navigation
  - Tab through all interactive elements
  - Arrow keys for list navigation (tasks, terminals)
  - Escape to close panels/dialogs
- [ ] Screen reader support
  - Announce panel state changes ("Terminal opened")
  - Label all buttons and inputs
  - Semantic HTML for egui widgets

**Acceptance Criteria:**

- View menu completely removed
- Panel toggles accessible via toolbar OR command palette OR status bar
- No "Coming Soon" items visible in UI
- All panels follow consistent design patterns
- Keyboard shortcuts documented and conflict-free
- Layout persists across restarts

---

### 5. Error Logging & Monitoring

**Current Gap:**

- Workspace sync failures are silent
- PTY spawn errors not surfaced to user
- Task corruption recovery happens invisibly

**Implementation Tasks:**

- [ ] **Add structured logging framework**
  - Use `tracing` crate (already popular in Rust ecosystem)
  - Configure levels: ERROR, WARN, INFO, DEBUG
  - Log to `.ferrite/logs/ferrite.log` with rotation
- [ ] **Log critical events**
  - Terminal spawn failures (with shell path attempted)
  - Workspace sync errors (file not found, permission denied)
  - Task/note save failures (disk full, permissions)
  - Worker crashes or panics
- [ ] **Show errors in UI**
  - Add status bar notification area (bottom-right corner)
  - Toast notifications for errors (auto-dismiss after 5s)
  - "View Logs" button opens log viewer panel
- [ ] **Add error recovery hints**

  ```
  ❌ Failed to spawn terminal: PowerShell not found
  💡 Try setting custom shell in Settings → Terminal → Shell Path
  ```

**Acceptance Criteria:**

- All errors logged to file with timestamps
- User sees actionable error messages (not just "failed")
- Logs don't grow unbounded (rotate at 10MB)

---

### 6. Performance Testing & Benchmarks

**Test Scenarios:**

- [ ] **Stress test: 20 concurrent terminals**
  - Open 20 terminal tabs
  - Run `ping localhost` in each (continuous output)
  - Measure: CPU usage, memory, frame drops
  - Expected: <30% CPU, <500MB RAM, ≥30 FPS
- [ ] **Stress test: Large task list**
  - Create `.ferrite/tasks.json` with 5,000 tasks
  - Open Productivity Hub
  - Measure: Load time, UI responsiveness, memory
  - Expected: <2s load, no UI freezing
- [ ] **Stress test: Rapid panel toggling**
  - Script to toggle Productivity Hub 100 times
  - Check for memory leaks (before/after delta)
  - Expected: No memory growth >10MB
- [ ] **Real-world workflow simulation**
  - Open project with 50 files
  - Open 3 terminals running build/test/server
  - Edit files while terminals output logs
  - Use Productivity Hub intermittently
  - Duration: 30 minutes
  - Expected: Stable memory, no crashes, responsive UI
- [ ] **Cross-platform validation**
  - Test on Windows 10/11
  - Test on Linux (Ubuntu 22.04+)
  - Test on macOS (if available)
  - Verify PTY behavior, shell detection, file paths

**Acceptance Criteria:**

- No crashes during stress tests
- Memory usage stays within documented bounds
- Frame rate ≥30 FPS during heavy terminal output

---

### 7. Code Quality & Architecture Review

**Review Tasks:**

- [ ] **Audit error handling**
  - All `unwrap()` calls replaced with proper error handling
  - PTY failures handled gracefully (don't panic app)
  - File I/O uses `Result<T, E>` with user-facing errors
- [ ] **Review thread safety**
  - All shared state uses `Arc<Mutex<>>` or channels correctly
  - No data races possible (verify with `cargo clippy`)
  - No deadlocks in worker shutdown path
- [ ] **Check for blocking operations on main thread**
  - File I/O for tasks/notes happens on background thread?
  - PTY reads/writes are non-blocking?
  - No `std::thread::sleep` on main thread
- [ ] **Validate settings migration**
  - Old configs without new fields work correctly
  - New fields have sensible defaults
  - Settings don't get corrupted on version mismatch
- [ ] **Security considerations**
  - Terminal doesn't execute arbitrary code from workspace files
  - Task input sanitized (no script injection)
  - File paths validated (no directory traversal)

**Acceptance Criteria:**

- Zero `unwrap()` in hot paths
- `cargo clippy` passes with no warnings
- Manual code review by second developer (you + wolverin0)

---

### 8. Documentation Gaps

**Missing Documentation:**

- [ ] **User-facing docs**
  - README section: "Integrated Terminal Usage"
  - README section: "Productivity Hub Guide"
  - Keyboard shortcuts reference (create `SHORTCUTS.md`)
  - Memory usage expectations documented
- [ ] **Developer docs**
  - Architecture diagram for async worker pattern
  - Comment on PTY lifecycle (spawn → read loop → cleanup)
  - Explain workspace-scoped storage design
  - Add `CONTRIBUTING.md` with PR guidelines
- [ ] **Troubleshooting guide**
  - "Terminal won't spawn" → Check shell path
  - "Tasks not persisting" → Check `.ferrite/` permissions
  - "High memory usage" → Link to diagnostics panel
  - "Sound not working" → Check OS audio settings

**Acceptance Criteria:**

- New user can use terminal without reading code
- Developers can extend worker pattern without asking questions
- Common issues have documented solutions

---

### 9. Release Readiness Checklist

**Before merging to master:**

- [ ] Memory investigation complete (findings documented)
- [ ] Diagnostics panel implemented (or tracked in separate issue)
- [ ] UI polish complete — View menu removed
- [ ] Panel integration redesigned (toolbar/command palette/status bar)
- [ ] Performance benchmarks pass acceptance criteria
- [ ] Error logging framework in place
- [ ] Cross-platform testing complete (Windows + Linux minimum)
- [ ] Code review approved by maintainer (you)
- [ ] User-facing documentation updated
- [ ] `CHANGELOG.md` entry written
- [ ] Version bumped to 0.2.6.1 in `Cargo.toml`

**Known Issues to Track:**

- [ ] Create GitHub issue: "Memory optimization for terminal buffers"
- [ ] Create GitHub issue: "Diagnostics panel implementation"
- [ ] Create GitHub issue: "UI redesign — Remove View menu, integrate panels"
- [ ] Create GitHub issue: "Terminal theme selection UI"
- [ ] Create GitHub issue: "Productivity Hub task pagination (>1000 items)"
- [ ] Create GitHub issue: "Command palette implementation"

---

## Decision Framework

| Criteria | Status | Blocker? |
|----------|--------|----------|
| Memory ≤ 150MB baseline | 🔴 Failed (300MB) | YES |
| No crashes in stress test | 🟡 Unknown | YES |
| UI integration complete | 🔴 View menu needs removal | YES |
| Error logging exists | 🔴 Missing | NO |
| Cross-platform tested | 🟡 Unknown | YES |
| Code review approved | 🟡 Pending | YES |
| Documentation updated | 🔴 Missing | NO |

**Recommendation:**

- ❌ **Do not merge yet** — Memory blocker and UI redesign must be resolved
- ✅ **Continue testing** — Document findings in GitHub issue
- ✅ **Engage wolverin0** — Collaborate on optimization and UI redesign plan

---

## Contact & Next Steps

- Schedule call with wolverin0 — Discuss memory findings and UI redesign
- Create tracking issues — One per section above
- Set merge deadline — Align with 0.2.6.1 release date
- Document decision — Update PR with "Merge blocked by #XYZ"

---

**Document Version:** 1.0  
**Last Updated:** February 3, 2026  
**Next Review:** After memory investigation complete
