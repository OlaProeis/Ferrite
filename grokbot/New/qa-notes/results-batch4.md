# Ferrite v0.3.1 QA - Batch 4 (YouTube embeds: T2-36, VID-1..9, VID-section9..11, EXTRA-YT-1..6/8, EOL-7, EXTRA-YT-2)

Tester: Grok Bot. Launch logs: see Log column. Pre-step: Settings cleanup (EXTRA-SET-11/4 restart rows are in results-batch3.md; set-persist log logs/ferrite-debug-20261008-222853-set-persist.log). Settings > Reset All was run afterwards (status 'Settings reset to defaults'); Appearance afterwards: Theme Light, Language English, Default View Mode Raw (highlighted), Font Inter. Recent Files slider after Reset All not re-checked.

| ROW-ID | RESULT | observation | log | screenshot |
|---|---|---|---|---|
| T2-36 | FAIL (crash) | Fixture opened in Raw. Clicking the Rendered view button aborted Ferrite: exit 134 SIGABRT (panic) at 22:31:23. Panic: 'GDK has not been initialized. Call gdk::init or gtk::init first.' at gdk-0.18.2 display.rs:301 <- gdk::Display::default <- ferrite::markdown::video_render::create_child_webview <- render_video_embed <- render_node <- show_rendered_editor. Fires on the first render of the trusted embed, before any thumbnail or click. Reproduced: Rendered again 22:32:05 (retry log) and Split 22:32:51 (split log). Ctrl+P, resize, Ctrl+S and the Raw check were therefore not reachable. | logs/ferrite-debug-20261008-223058-T2-36.log (+ -223148-T2-36-retry.log, -223237-T2-36-split.log) | shots/T2-36-fail.png |
| VID-1 | FAIL (crash) | Minimal QA copy of section 1 alone (/tmp/vid-1-min.md, one trusted youtube.com/watch embed): Rendered view aborts with the same GDK panic (exit 134 at 22:35:37). No thumbnail, no play overlay, no WebView reached. | logs/ferrite-debug-20261008-223520-VID-1.log | shots/VID-1-fail-crash.png |
| VID-2 | BLOCKED (crash) | youtu.be section not run on its own. Every trusted embed goes through the render path that aborts (T2-36, VID-1). EXTRA-YT-6 typed youtu.be embed in Split also aborted (22:42:22). | logs/ferrite-debug-20261008-223957-EXTRA-YT-6.log | shots/EXTRA-YT-6-fail-crash.png |
| VID-3 | BLOCKED (crash) | width=480 and width=320 height=240 embeds never rendered: fixture and single-embed doc abort on first trusted render. Sizes not measured. | logs/ferrite-debug-20261008-223058-T2-36.log | shots/T2-36-fail.png |
| VID-4 | PASS | Section 5 vimeo as QA copy /tmp/vid-vimeo-only.md in Rendered: shows placeholder card 'External video (opens in browser)' with link https://vimeo.com/76979871. Log has 0 webview/create_child lines, no crash. Card deliberately not clicked (would open system browser). Same placeholder seen in Split (EXTRA-YT-6 doc). | logs/ferrite-debug-20261008-223312-VID-4.log | shots/VID-4-pass.png, shots/VID-4-cases.png |
| VID-5 | PARTIAL | No embed created (PASS for that part). But the bare URL and the <https://...> autolink in a list render as plain black text, with no link styling. A control doc with no video (/tmp/ctrl-plain.md) behaves the same for bare URLs and autolinks, while [markdown](link) renders blue. So this is not embed-specific, but the 'stays a Link node' expectation is not visible in this build. | logs/ferrite-debug-20261008-223312-VID-4.log, logs/ferrite-debug-20261008-223549-CTRL-plain.log | shots/VID-4-pass.png |
| VID-6 | BLOCKED (crash) | Drag-resize handle exists only on a rendered trusted embed; the fixture aborts on Rendered. Not run. | logs/ferrite-debug-20261008-223058-T2-36.log | shots/T2-36-fail.png |
| VID-7 | BLOCKED (crash) | Overlay test needs a playing video; not reachable. Ctrl+P, Find/Replace, unsaved-changes dialog and palette NOT tested over a player. | logs/ferrite-debug-20261008-223058-T2-36.log | none |
| VID-8 | BLOCKED (crash) | Needs a playing video (window drag / raw editing while playing). Not reached. | logs/ferrite-debug-20261008-223058-T2-36.log | none |
| VID-9 | BLOCKED (crash) | Needs a primary-window playing video before opening the second window. Not run. | none | none |
| VID-section9 | BLOCKED (crash) | Same-video x4 fixture aborts on Rendered. Yt-many (12 embeds, EXTRA-YT-4) also aborts on Rendered. Resize/player-follow not run. | logs/ferrite-debug-20261008-223923-EXTRA-YT-4.log | shots/EXTRA-YT-4-fail-crash.png |
| VID-section10 | BLOCKED (crash) | Wheel-over-player needs a live player. Not reached. | logs/ferrite-debug-20261008-223058-T2-36.log | none |
| VID-section11 | BLOCKED (crash) | Fullscreen needs a live player. Not reached. | logs/ferrite-debug-20261008-223058-T2-36.log | none |
| EXTRA-YT-1 | PARTIAL | Per-case table in the details section below. Bare URL/B/C cases (no WebView, no crash) PASS in a QA copy. All trusted A and D cases BLOCKED: any trusted embed in Rendered aborts the app (A1 confirmed as FAIL crash). | logs/ferrite-debug-20261008-223520-VID-1.log, logs/ferrite-debug-20261008-223312-VID-4.log | shots/VID-4-cases.png, shots/VID-4-pass.png |
| EXTRA-YT-4 | FAIL (crash) | yt-many.md (12 embeds): RSS at Raw open 207088 KB. Clicking Rendered aborted the app (exit 134 at 22:39:37), same GDK panic. Scroll, player-doubling and post-scroll RSS not reachable. | logs/ferrite-debug-20261008-223923-EXTRA-YT-4.log | shots/EXTRA-YT-4-fail-crash.png |
| EXTRA-YT-5 | BLOCKED (crash) | Needs a playing embed; the first step (entering Rendered) aborts the app. Not run. | logs/ferrite-debug-20261008-223058-T2-36.log | none |
| EXTRA-YT-3 | BLOCKED (crash) | No player ever rendered, so no players to follow window resize/Split/sidebar. Not run. | none | none |
| EXTRA-YT-6 | FAIL (crash) | In Split, /tmp/yt-live.md (vimeo embed) rendered as a placeholder with no crash. Typed edits in raw: a first shift+Home + typing attempt did not replace the line (text inserted after the vimeo embed; tester action, not a defect verdict). Then Ctrl+A and typing the single line {{video https://youtu.be/dQw4w9WgXcQ width=320}} char by char: the app aborted at 22:42:22 (exit 134) with the same GDK panic from create_child_webview via render_video_embed. Thumbnail fetch (ureq to img.youtube.com) logged right after the panic line. Width-by-typing not reached. | logs/ferrite-debug-20261008-223957-EXTRA-YT-6.log | shots/EXTRA-YT-6-live1.png, shots/EXTRA-YT-6-live2.png, shots/EXTRA-YT-6-fail-crash.png |
| EXTRA-YT-8 | BLOCKED (crash) | Needs a visible player for theme/font checks. Not reached. | none | none |
| EOL-7 | PARTIAL | /tmp/crlf-video.md opened in Raw (Rendered would abort). Typed x then BackSpace, Ctrl+S: log 'File saved successfully'. eol-check crlf: CRLF True (5 of 5 lines, bytes 0d0a). Drag-resize part NOT performed (needs Rendered). | logs/ferrite-debug-20261008-223753-EOL-7.log, logs/EOL-7-check.txt | none |
| EXTRA-YT-2 | FAIL (crash) | Offline: `sudo unshare -n` launch worked (netns with no network). A Session Recovery dialog appeared from the crashed EXTRA-YT-6 session; clicked Start Fresh. Clicking Rendered aborted the app (exit 134 at 22:43:28), same GDK panic. Placeholder/error-on-play/retry-storm checks not reachable; offline log has 0 ureq lines. | logs/ferrite-debug-20261008-224251-EXTRA-YT-2.log | shots/EXTRA-YT-2-fail-offline-crash.png |

## Details

### EXTRA-YT-1 URL matrix (per case: thumbnail / plays / size / toast / verdict)

| Case | Thumbnail? | Plays? | Size | Toast | Result |
|---|---|---|---|---|---|
| A1 watch?v= | no: app aborts on Rendered (VID-1 minimal doc) | no | - | none | FAIL (crash) |
| A2 youtu.be | not run alone; typed youtu.be embed aborted (EXTRA-YT-6) | - | - | - | BLOCKED (crash) |
| A3-A14 | not run; the matrix aborts on the first trusted case | - | - | - | BLOCKED (crash) |
| B1 youtube-nocookie | placeholder card 'External video (opens in browser)', no thumbnail | not clicked | n/a | none seen | PASS (no WebView) |
| B2 youtube.com/ (no id) | placeholder 'Open in browser' | not clicked | n/a | none seen | PASS (no WebView) |
| B3 playlist-only | placeholder 'Open in browser' | not clicked | n/a | none seen | PASS (no WebView) |
| B4 youtube.com.evil.example | placeholder 'External video' | not clicked | n/a | none seen | PASS (no WebView) |
| B5 vimeo | placeholder 'External video' | not clicked | n/a | none seen | PASS (no WebView) |
| C1 javascript: | literal text, plus a grey hatched bar | n/a | n/a | none | PASS (no embed) |
| C2 file:/// | literal text | n/a | n/a | none | PASS (no embed) |
| C3 notaurl | literal text | n/a | n/a | none | PASS (no embed) |
| C4 {{video}} | literal text, plus a grey hatched bar | n/a | n/a | none | PASS (no embed) |
| C5 {{VIDEO ...}} upper-case keyword | not run (trusted URL; aborts if recognised) | - | - | - | BLOCKED (crash) |
| C6 embed in list item | not run (trusted URL) | - | - | - | BLOCKED (crash) |
| C7 embed in blockquote | not run (trusted URL) | - | - | - | BLOCKED (crash) |
| C8 two embeds in one paragraph | not run (trusted URL) | - | - | - | BLOCKED (crash) |
| C9 trailing text | not run (trusted URL) | - | - | - | BLOCKED (crash) |
| D1-D8 (width=0/-5/abc/99999/height only/1x1/WIDTH+HEIGHT/autoplay) | not run (all trusted youtu.be) | - | - | - | BLOCKED (crash) |

Notes: B and C cases were run in QA copy /tmp/vid-vimeo-only.md (B1-B5, C1-C4 verbatim, plus one localhost case). Log count of webview/create_child lines: 0. Overview screenshot: shots/VID-4-cases.png (scrolled to C cases).

### Log tails (last 30 non-span lines) for each FAIL

#### logs/ferrite-debug-20261008-223058-T2-36.log
```
[2026-10-08T20:31:20Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:31:20Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1740.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:31:21Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:31:21Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:31:21Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1740.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:31:21Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:31:21Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:31:21Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1740.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:31:22Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:31:22Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:31:22Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1740.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:31:22Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:31:22Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:31:22Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1740.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:31:23Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:31:23Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:31:23Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1740.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:31:23Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:31:23Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:31:23Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1740.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:31:23Z DEBUG ferrite::app::title_bar] Title bar: Set view mode to Rendered

thread 'main' (407560) panicked at /home/box/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gdk-0.18.2/src/auto/display.rs:301:9:
GDK has not been initialized. Call `gdk::init` or `gtk::init` first.
stack backtrace:
[2026-10-08T20:31:23Z DEBUG ureq::stream] connecting to img.youtube.com:443 at 198.18.0.1:443
[2026-10-08T20:31:23Z DEBUG rustls::client::hs] No cached session for DnsName("img.youtube.com")
[2026-10-08T20:31:23Z DEBUG rustls::client::hs] Not resuming any session
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
=== Ferrite exited with code 134 SIGABRT - panic (release panic=abort) => FAIL at 22:31:23 CEST ===
```

#### logs/ferrite-debug-20261008-223148-T2-36-retry.log
```
[2026-10-08T20:32:04Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:32:04Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:32:04Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:32:04Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:32:04Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:32:04Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:32:04Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:32:04Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:32:04Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:32:05Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:32:05Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:32:05Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:32:05Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-08T20:32:05Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 0, y: 105 }) size=Physical(PhysicalSize { width: 1140, height: 641 })
[2026-10-08T20:32:05Z DEBUG winit::window] winit::Window::set_maximized; maximized=true
[2026-10-08T20:32:05Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:32:05Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:32:05Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1740.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:32:05Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-08T20:32:05Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 0, y: 105 }) size=Physical(PhysicalSize { width: 1780, height: 1041 })
[2026-10-08T20:32:05Z DEBUG ferrite::app::title_bar] Title bar: Set view mode to Rendered

thread 'main' (408904) panicked at /home/box/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gdk-0.18.2/src/auto/display.rs:301:9:
GDK has not been initialized. Call `gdk::init` or `gtk::init` first.
stack backtrace:
[2026-10-08T20:32:05Z DEBUG ureq::stream] connecting to img.youtube.com:443 at 198.18.0.1:443
[2026-10-08T20:32:05Z DEBUG rustls::client::hs] No cached session for DnsName("img.youtube.com")
[2026-10-08T20:32:05Z DEBUG rustls::client::hs] Not resuming any session
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
=== Ferrite exited with code 134 SIGABRT - panic (release panic=abort) => FAIL at 22:32:05 CEST ===
```

#### logs/ferrite-debug-20261008-223237-T2-36-split.log
```
[2026-10-08T20:32:50Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:32:50Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:32:50Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:32:50Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:32:50Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:32:50Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:32:50Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:32:50Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:32:50Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:32:50Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:32:50Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:32:50Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:32:50Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:32:50Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:32:50Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:32:50Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:32:50Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:32:51Z DEBUG ferrite::app::title_bar] Title bar: Set view mode to Split
[2026-10-08T20:32:51Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:32:51Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:32:51Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=526.3, max_wrap_width=840.0, effective=526.3

thread 'main' (410247) panicked at /home/box/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gdk-0.18.2/src/auto/display.rs:301:9:
GDK has not been initialized. Call `gdk::init` or `gtk::init` first.
stack backtrace:
[2026-10-08T20:32:51Z DEBUG ureq::stream] connecting to img.youtube.com:443 at 198.18.0.1:443
[2026-10-08T20:32:51Z DEBUG rustls::client::hs] No cached session for DnsName("img.youtube.com")
[2026-10-08T20:32:51Z DEBUG rustls::client::hs] Not resuming any session
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
=== Ferrite exited with code 134 SIGABRT - panic (release panic=abort) => FAIL at 22:32:51 CEST ===
```

#### logs/ferrite-debug-20261008-223520-VID-1.log
```
[2026-10-08T20:35:36Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:35:36Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:35:36Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:35:36Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:35:36Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:35:37Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:35:37Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:35:37Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:35:37Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:35:37Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:35:37Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:35:37Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:35:37Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:35:37Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:35:37Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:35:37Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:35:37Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:35:37Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:35:37Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:35:37Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:35:37Z DEBUG ferrite::app::title_bar] Title bar: Set view mode to Rendered

thread 'main' (414195) panicked at /home/box/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gdk-0.18.2/src/auto/display.rs:301:9:
GDK has not been initialized. Call `gdk::init` or `gtk::init` first.
stack backtrace:
[2026-10-08T20:35:37Z DEBUG ureq::stream] connecting to img.youtube.com:443 at 198.18.0.1:443
[2026-10-08T20:35:37Z DEBUG rustls::client::hs] No cached session for DnsName("img.youtube.com")
[2026-10-08T20:35:37Z DEBUG rustls::client::hs] Not resuming any session
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
=== Ferrite exited with code 134 SIGABRT - panic (release panic=abort) => FAIL at 22:35:37 CEST ===
```

#### logs/ferrite-debug-20261008-223923-EXTRA-YT-4.log
```
[2026-10-08T20:39:37Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:39:37Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:39:37Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:39:37Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:39:37Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:39:37Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:39:37Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:39:37Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:39:37Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:39:37Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:39:37Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:39:37Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:39:37Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:39:37Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:39:37Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:39:37Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:39:37Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:39:37Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:39:37Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-08T20:39:37Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 0, y: 105 }) size=Physical(PhysicalSize { width: 1140, height: 641 })
[2026-10-08T20:39:37Z DEBUG ferrite::app::title_bar] Title bar: Set view mode to Rendered

thread 'main' (422615) panicked at /home/box/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gdk-0.18.2/src/auto/display.rs:301:9:
GDK has not been initialized. Call `gdk::init` or `gtk::init` first.
stack backtrace:
[2026-10-08T20:39:37Z DEBUG ureq::stream] connecting to img.youtube.com:443 at 198.18.0.1:443
[2026-10-08T20:39:37Z DEBUG rustls::client::hs] No cached session for DnsName("img.youtube.com")
[2026-10-08T20:39:37Z DEBUG rustls::client::hs] Not resuming any session
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
=== Ferrite exited with code 134 SIGABRT - panic (release panic=abort) => FAIL at 22:39:37 CEST ===
```

#### logs/ferrite-debug-20261008-223957-EXTRA-YT-6.log
```
[2026-10-08T20:42:22Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 0, y: 105 }) size=Physical(PhysicalSize { width: 566, height: 641 })
[2026-10-08T20:42:22Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:42:22Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=526.3, max_wrap_width=840.0, effective=526.3
[2026-10-08T20:42:22Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:42:22Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=526.3, max_wrap_width=840.0, effective=526.3
[2026-10-08T20:42:22Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:42:22Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=526.3, max_wrap_width=840.0, effective=526.3
[2026-10-08T20:42:22Z DEBUG ferrite::editor::widget] EditorWidget: cursor sync changed: (0, 24) -> (0, 25)
[2026-10-08T20:42:22Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 0, y: 105 }) size=Physical(PhysicalSize { width: 566, height: 641 })
[2026-10-08T20:42:22Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:42:22Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=526.3, max_wrap_width=840.0, effective=526.3
[2026-10-08T20:42:22Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:42:22Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=526.3, max_wrap_width=840.0, effective=526.3
[2026-10-08T20:42:22Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 0, y: 105 }) size=Physical(PhysicalSize { width: 566, height: 641 })
[2026-10-08T20:42:22Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:42:22Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=526.3, max_wrap_width=840.0, effective=526.3
[2026-10-08T20:42:22Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:42:22Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=526.3, max_wrap_width=840.0, effective=526.3
[2026-10-08T20:42:22Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:42:22Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=526.3, max_wrap_width=840.0, effective=526.3
[2026-10-08T20:42:22Z DEBUG ferrite::editor::widget] EditorWidget: cursor sync changed: (0, 25) -> (0, 26)

thread 'main' (423503) panicked at /home/box/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gdk-0.18.2/src/auto/display.rs:301:9:
GDK has not been initialized. Call `gdk::init` or `gtk::init` first.
stack backtrace:
[2026-10-08T20:42:22Z DEBUG ureq::stream] connecting to img.youtube.com:443 at 198.18.0.1:443
[2026-10-08T20:42:22Z DEBUG rustls::client::hs] No cached session for DnsName("img.youtube.com")
[2026-10-08T20:42:22Z DEBUG rustls::client::hs] Not resuming any session
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
=== Ferrite exited with code 134 SIGABRT - panic (release panic=abort) => FAIL at 22:42:22 CEST ===
```

#### logs/ferrite-debug-20261008-224251-EXTRA-YT-2.log
```
[2026-10-08T20:43:27Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:43:27Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:43:27Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:43:27Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:43:27Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:43:27Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:43:27Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:43:27Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:43:27Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:43:27Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:43:28Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:43:28Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:43:28Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:43:28Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:43:28Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:43:28Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:43:28Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:43:28Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:43:28Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:43:28Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:43:28Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:43:28Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:43:28Z DEBUG ferrite::app::title_bar] Title bar: Set view mode to Rendered

thread 'main' (428360) panicked at /home/box/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gdk-0.18.2/src/auto/display.rs:301:9:
GDK has not been initialized. Call `gdk::init` or `gtk::init` first.
stack backtrace:
[2026-10-08T20:43:28Z WARN  ferrite::markdown::video_render] Failed to fetch video thumbnail 'https://img.youtube.com/vi/dQw4w9WgXcQ/hqdefault.jpg': https://img.youtube.com/vi/dQw4w9WgXcQ/hqdefault.jpg: Dns Failed: resolve dns name 'img.you
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
=== Ferrite exited with code 134 SIGABRT - panic (release panic=abort) => FAIL at 22:43:28 CEST ===
```

### Odd observations

- Settings Reset All re-enabled 'Restore previous session': later launches logged 'Restored tab' (tabs came back), and a crash left a Session Recovery dialog (Start Fresh used).
- Recent Files after relaunch read 'Remember last 10 files' (see EXTRA-SET-4-restart in results-batch3.md).
- Unclosed or invalid embeds (C1, C4, a half-typed {{video ...}}) show a grey hatched bar in Rendered/Split, next to the literal text.
- Bare URLs and <autolinks> are not link-styled (control doc without video shows the same), while [text](url) is.
- Repaint lag: the Rendered/Split pane often updates only after the mouse moves.
- Clicking the window title bar sometimes left the window in move mode (it followed the pointer); a click on the title-bar button at the top right toggled Zen Mode (log: 'Toggle Zen Mode') instead of closing.
- The panic is at the first render of a trusted embed; the code has no gtk::init/gdk::init call (rg over src/ for 'gtk::init|gdk::init|gtk_init' finds nothing), which matches the GDK message.

### Screenshots (under /workspace/ferrite-qa/shots/)

- T2-36-fail.png (crash terminal after Rendered abort), VID-1-fail-crash.png, EXTRA-YT-2-fail-offline-crash.png, EXTRA-YT-4-fail-crash.png, EXTRA-YT-6-fail-crash.png (crash terminal), EXTRA-YT-6-live1.png and -live2.png (live typing in Split), VID-4-pass.png (vimeo placeholder + bare-link rendering), VID-4-cases.png (B/C cases), set-persist-recent.png (Settings after relaunch, Recent Files=10).

### Final state

- Ferrite closed (EXTRA-YT-2 instance ended by crash; pgrep -x ferrite empty).
- Repo fixtures: none saved by me (all QA copies in /tmp or fixtures/ QA folder). git status --short in /workspace/ferrite-qa/ferrite:
```
?? test_md/zz_cjk_search.md
```
- Untracked test_md/zz_cjk_search.md was already present before this batch (created by an earlier session at 20:54); not touched by me.
