# Ferrite v0.3.1 re-run — automated gate (Linux)

| Field | Value |
|---|---|
| Date (Europe/Oslo) | 2026-11-09 10:37 CET |
| Host | Debian 13 box (Xvfb QA environment) |
| Repo | `/workspace/ferrite-qa/ferrite` (`OlaProeis/Ferrite`) |
| Branch | `0.3.1-experimental` |
| HEAD | `9effd76` — `docs(qa): point GrokBot re-run gate at 221b6ae+` |
| Code commit under test | `221b6ae` (reachable from HEAD; HEAD only updates the re-run doc) |
| `git status --short` | clean (empty) |
| Toolchain | `rustc 1.92.0 (ded5c06cf 2025-12-08)` / `cargo 1.92.0 (344c4567c 2025-10-21)` |
| Launcher binary | `/workspace/ferrite-qa/ferrite/target/release/ferrite` (from `run-ferrite.sh`) |

## 1. Release build — **OK**

```text
cargo build --release --locked
→ Finished `release` profile [optimized] target(s) in 3m 38s
BUILD_EXIT=0
```

- Log: `/workspace/ferrite-qa/logs/rerun2-build.log`
- Binary: `/workspace/ferrite-qa/ferrite/target/release/ferrite`
- Size: 46 626 640 bytes
- mtime (Europe/Oslo): **2026-11-09 10:32:12 CET**
- `--version`: `ferrite 0.3.1`

Note: first build attempt failed because `pkg-config` and GTK/WebKit `-dev` packages were missing from the box. They were installed from Debian `.deb`s (apt mirror was returning 500 via the sandbox proxy). Package configure is still partially broken due to runtime vs `-dev` version skew, but `pkg-config` resolves `glib-2.0`, `gtk+-3.0`, `webkit2gtk-4.1`, and `libsoup-3.0`, and the release link succeeded.

## 2. `cargo test --locked` — **OK**

```text
Running unittests src/main.rs (target/debug/deps/ferrite-…)
running 2230 tests
test result: ok. 2228 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 1.75s
TEST_EXIT=0
```

| Binary / suite | Passed | Failed | Ignored |
|---|---:|---:|---:|
| `ferrite` unittests (`src/main.rs`) | 2228 | 0 | 2 |
| **Total** | **2228** | **0** | **2** |

Ignored tests (intentional `#[ignore]`):

- `editor::ferrite::buffer::tests::test_large_file`
- `editor::ferrite::history::tests::test_large_buffer_performance`

Failures: **none**.

Log: `/workspace/ferrite-qa/logs/rerun2-cargo-test.log`

### Comparison to Windows

| Platform | Passed | Failed | Ignored |
|---|---:|---:|---:|
| Windows (reported) | 2239 | (not stated) | (not stated) |
| Linux (this run) | 2228 | 0 | 2 |

Delta: Linux is **11 fewer passed** than the Windows figure (2239). Likely `#[cfg(windows)]` / platform-only tests (several exist under `path_utils`, `single_instance`, `fonts`, `terminal`, `lsp`). No Linux failures.

## 3. `cargo clippy --locked --release` — **OK** (warnings only)

```text
warning: `ferrite` (bin "ferrite") generated 321 warnings
Finished `release` profile [optimized] target(s) in 38.35s
CLIPPY_EXIT=0
```

| Metric | Count |
|---|---:|
| `error:` lines | 0 |
| `warning:` lines (raw log matches) | 323 |
| Clippy summary for `ferrite` bin | **321 warnings** (198 auto-fixable suggestions) |
| Exit code | 0 |

Also noted: future-incompat for dependency `ashpd v0.8.1` (`cargo report future-incompatibilities --id 1`).

Top warning themes (by message): unnecessary `to_string`, borrowed expression implements traits, too many arguments, manual prefix strip, collapsible `if`, `map_or` simplify, derivable `impl`, complex types, etc. Hygiene only — not treated as a gate failure.

Log: `/workspace/ferrite-qa/logs/rerun2-clippy.log`

## 4. Gate verdict

| Check | Result |
|---|---|
| Correct commit / clean tree | PASS (`9effd76`, clean) |
| `cargo build --release --locked` | **PASS** |
| `cargo test --locked` | **PASS** (2228 / 0 / 2) |
| `cargo clippy --locked --release` | **PASS** (0 errors, 321 warnings) |

**Automated gate: PASS.** Ready for GUI re-run per `grokbot/New/rerun-v0.3.1-round2.md` (Tier 1 ISS-1/2/5/7, then Round 1 / Round 2 tables).
