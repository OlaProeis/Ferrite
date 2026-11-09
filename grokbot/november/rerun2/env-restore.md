# Ferrite QA env restore after box restart — 2026-11-09 11:48 CET (Europe/Oslo)

| Item | Result |
|---|---|
| HEAD | `9effd76` on `0.3.1-experimental` (clean tree; no source edits) |
| `cargo build --release --locked` | **PASS** — log `/workspace/ferrite-qa/logs/rerun2-rebuild.log` |
| Binary | `/workspace/ferrite-qa/ferrite/target/release/ferrite` — 46626640 bytes, `ferrite 0.3.1`, mtime 11:48 CET |
| `cargo test` | **not re-run** (per request) |
| scrot | 1.12.1-1 |
| xclip | 0.13-4 |
| gdb | 16.3-1 |
| ImageMagick `convert` | 7.1.1-43 (pkg `imagemagick` 8:7.1.1.43+dfsg1-1+deb13u14) |
| make | 4.4.1-2 (needed for jemalloc; was missing after restart) |
| pkg-config | 1.8.1 — resolves glib-2.0 / gtk+-3.0 / webkit2gtk-4.1 / libsoup-3.0 |
| libxkbcommon-x11-0 / libxcb-xkb1 | still installed |
| libwebkit2gtk-4.1-dev | left **half-configured (iU)** as instructed; runtime `libwebkit2gtk-4.1-0` **2.54.0-1~deb13u3** |
| Harness | `/workspace/ferrite-qa/run-ferrite.sh`, `shot.sh`, `prep-fixtures.sh`, `eol-check.sh` (survived restart) |
| Fixtures | `/tmp` eol fixtures via `prep-fixtures.sh`; `/tmp/ferrite-rerun2/` recreated from `/workspace/ferrite-qa/fixtures/` |
| Smoke | DISPLAY=:2 launch stayed up 10s, no panic; exit 143 after SIGTERM; shot `shots/R2-SMOKE-pass.png` |

Notes: apt http→proxy 500 still broken; packages restored via `curl` + `dpkg -i` from deb.debian.org / debian-security. First rebuild attempts failed on missing `pkg-config` then missing `make`; final rebuild OK in ~3m33s.
- Cleared stale ~/.config/ferrite/instance.{lock,pid,port} after smoke SIGTERM (pointed at dead pid).
