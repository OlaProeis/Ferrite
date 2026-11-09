# Ferrite v0.3.1 rerun 2 (221b6ae) - Batch B: CPY-3 redo, ISS-2, ISS-5, ISS-7

| ROW-ID | RESULT | observation | prior | log | screenshot |
|---|---|---|---|---|---|
| CPY-3 | FAIL | Selected the rendered inline paragraph and copied with Ctrl+C; `DISPLAY=:2 xclip -selection clipboard -o` returned the source Markdown, including `**bold**`, `*italic*`, `` `code` ``, and a newline between the source lines. Expected rendered text without markup: `And this inline-formatted paragraph — copy the whole line and verify no extra spaces appear around the styled spans: normal bold italic code end.` Comparison failed. No hang/crash. | Prior `results-batch9.md`: FAIL (hang during selection; clipboard check not reached). | `logs/ferrite-debug-20261109-110716-R2-CPY-3.log` | `shots/R2-CPY-3-fail.png` |
| T2-36 | FAIL | Rendered mode no longer aborted; no `GDK has not been initialized` in the log. Embed #1 initially showed a blank white area; clicking it opened the YouTube player with `Feil 153` (Error 153), not playback. Ctrl+P did not bring up an overlay; resize/source-update and playing-video overlay steps not reached. | Prior `results-batch4.md`: FAIL (crash, exit 134 GDK panic). | `logs/ferrite-debug-20261109-110845-R2-T2-36.log` | `shots/R2-T2-36-pass.png` (before click), `shots/R2-T2-36-fail.png` (Error 153) |
| VID-1 | FAIL | Isolated trusted `youtube.com/watch` embed rendered into a badly clipped/blank area; within ~6 s a YouTube panel appeared with `Feil 153` (Error 153), no playable thumbnail/player. Ferrite remained running; no GDK panic in log. | Prior `results-batch4.md`: FAIL (GDK abort, exit 134). | `logs/ferrite-debug-20261109-110956-R2-VID-1.log` | `shots/R2-VID-1-fail.png` |
| EXTRA-YT-2 | FAIL | Ran the documented `sudo unshare -n` offline launch. Rendered fixture displayed large black WebView rectangles instead of thumbnail/placeholder; click on embed produced no visible error/toast. Switching to Raw worked, so editing UI remained responsive. Log contains one `ureq` mention and no `GDK has not been initialized`/panic (no retry storm observed). | Prior `results-batch4.md`: FAIL (GDK abort, exit 134; offline placeholder checks unreachable). | `logs/ferrite-debug-20261109-111035-R2-EXTRA-YT-2.log` | `shots/R2-EXTRA-YT-2-fail.png` |
| EXTRA-YT-4 | FAIL | Opened the 12-embed `yt-many.md`, switched to Rendered, scrolled top→bottom→top 3×, and attempted to start embed 1. Ferrite stayed responsive and did not crash; thumbnails/player areas resolved to YouTube `Feil 153` errors (seen on embeds 1–4), not usable players. RSS after run: 286,380 KB. No GDK/panic lines. | Prior `results-batch4.md`: FAIL (exit 134 on Rendered; scrolling unreachable). | `logs/ferrite-debug-20261109-111123-R2-EXTRA-YT-4.log` | `shots/R2-EXTRA-YT-4-fail.png` |
| EXTRA-YT-6 | PARTIAL | In Split, the initial `youtu.be` embed rendered as thumbnail/grey WebView overlap and did not crash. YouTube playback was unavailable (Error 153 on other trusted embeds). Two char-by-char replacement attempts via xdotool did not replace the selected source; they appended another `{{video …}}` instead. No GDK panic occurred, but breaking/fixing the live URL and width update could not be reliably verified. | Prior `results-batch4.md`: FAIL (crash, exit 134 during trusted embed edit). | `logs/ferrite-debug-20261109-111333-R2-EXTRA-YT-6b.log` | `shots/R2-EXTRA-YT-6-partial.png` |
| EXTRA-YT-7 | PARTIAL | The prescribed bare-URL document rendered plain links, not embeds, so I wrapped the same two URLs in `{{video …}}` to exercise webviews. The two webviews initially overlapped/painted black; one showed a thumbnail for `jNQXAC9IVRw`, then the player reported Error 153. The dirty Reload confirmation and Search-in-Files dialog both visibly drew above the thumbnail/webview. Ctrl+, opened a separate Settings tab rather than an overlay, so Settings-on-player stacking and live playback could not be verified. No GDK panic. | Prior `results-batch12.md`: FAIL (GDK abort, exit 134, twice). | `logs/ferrite-debug-20261109-111452-R2-EXTRA-YT-7.log` | `shots/R2-EXTRA-YT-7-pass-search.png`, `shots/R2-EXTRA-YT-7-pass-reload.png`, `shots/R2-EXTRA-YT-7-partial.png` |
| VID-10 | PASS | `ldd` confirms `libwebkit2gtk-4.1.so.0`, `libgtk-3.so.0`, and `libsoup-3.0.so.0`. Rendered trusted embed paths in T2-36 and VID-1 ran without the former GDK initialization abort (though YouTube itself returned Error 153; recorded separately). | Prior `results-batch4.md`: FAIL (GDK abort on first Rendered). | `logs/ferrite-debug-20261109-110845-R2-T2-36.log`, `logs/ferrite-debug-20261109-110956-R2-VID-1.log`; `ldd` output | `shots/R2-T2-36-pass.png` |
| T1-14 | FAIL | Created `/tmp/red.png` with ImageMagick and set clipboard via `DISPLAY=:2 xclip -selection clipboard -t image/png`; verified `TARGETS image/png`. Clicked after `日本語テキスト ééé` in saved `/tmp/ferrite-rerun2/paste.md`, Ctrl+V: no image ref, no PNG/`assets/`, no visible toast. Log reports arboard `incorrect type received from clipboard`. | Prior `results-batch2.md`: FAIL (same arboard clipboard type error; keyboard paste path did not reach image handler). | `logs/ferrite-debug-20261109-111926-R2-T1-14.log` | `shots/R2-T1-14-fail.png` |
| IMG-X1 | FAIL | With PNG clipboard target `image/png` verified, Ctrl+V in a new Untitled tab did nothing and showed no “save the document first” toast. No new `/tmp/assets/` or launcher-cwd `assets/` appeared; existing repo `ferrite/assets/` unchanged. Log again reports arboard `incorrect type received from clipboard`, so paste never reached untitled-document handling. | Prior `results-batch7.md`: FAIL (no toast; arboard rejected image before image-paste path). | `logs/ferrite-debug-20261109-111926-R2-T1-14.log` | `shots/R2-IMG-X1-fail.png` |
| IMG-X2 | FAIL | On saved `/tmp/ferrite-rerun2/paste.md`, attempted two Ctrl+V pastes quickly with `image/png` still the verified clipboard target. Neither inserted Markdown or a file; `/tmp/ferrite-rerun2/assets/` does not exist. No overwrite collision could be tested. Same arboard `incorrect type received from clipboard` error as T1-14. | Prior `results-batch7.md`: FAIL (image paste rejected before handler; no new files). | `logs/ferrite-debug-20261109-111926-R2-T1-14.log` | `shots/R2-IMG-X2-fail.png` |
| IMG-4 | BLOCKED-by-input/environment | Undo-after-paste could not be exercised: the preceding saved-doc PNG paste never inserted Markdown (`T1-14` / `IMG-X2`), so there was no image-paste undo entry to remove. | Prior `results-batch10.md`: BLOCKED (same clipboard paste failure). | `logs/ferrite-debug-20261109-111926-R2-T1-14.log` | `shots/R2-T1-14-fail.png` |
| IMG-7 | BLOCKED-by-input/environment | Enabled the preview lock in Split (tooltip confirmed lock control), focused Raw, and Ctrl+V with clipboard `image/png`: no Markdown/ref or PNG was inserted; no toast. The same arboard `incorrect type received from clipboard` error prevented checking the allowed Raw paste vs preview mutation gate. | Prior `results-batch10.md`: BLOCKED (same clipboard failure prevented the Raw-vs-preview paste check). | `logs/ferrite-debug-20261109-111926-R2-T1-14.log` | `shots/R2-IMG-7-blocked.png` |
| T1-21 | PARTIAL | In Split/Raw, typed the opener character-by-character at 150 ms: the saved source starts exactly ` ```mermaid` (10 chars), not reversed, and preview rendered a flowchart without crash/hang. The later exact sample was not fully reproduced: xdotool produced `subgraph 业 PEP` (missing the second CJK character) and auto-close appended `]}` to `subgraph a] [b`, with the closing fence on that line. So the fix-critical opener/diagram passed, but exact malformed-input source check is partial. | Prior `results-batch2.md`: FAIL (opener expanded to many backticks + `diamrem`; no crash). | `logs/ferrite-debug-20261109-112248-R2-T1-21.log` | `shots/R2-T1-21-partial.png` |
| MMD-12 | PASS | Incrementally typed a Mermaid block in Split/Raw; opening fence remained ordered, intermediate states did not crash/hang, and the completed flowchart rendered. The malformed-input lines were separately noted under T1-21. | Prior result not separately listed; same prior T1-21 corrupted-opener failure in `results-batch2.md`. | `logs/ferrite-debug-20261109-112248-R2-T1-21.log` | `shots/R2-MMD-12-pass.png` |

## FAIL log tails (last ~30 non-span lines)

### CPY-3 — `logs/ferrite-debug-20261109-110716-R2-CPY-3.log`
```text
[2026-11-09T10:08:20Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:08:20Z DEBUG ferrite::editor::widget] Syncing 8 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:08:20Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:08:20Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:08:20Z DEBUG ferrite::editor::widget] Syncing 8 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:08:20Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:08:20Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:08:20Z DEBUG ferrite::editor::widget] Syncing 8 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:08:20Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:08:20Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:08:20Z DEBUG ferrite::editor::widget] Syncing 8 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:08:20Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:08:20Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:08:20Z DEBUG ferrite::editor::widget] Syncing 8 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:08:20Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:08:21Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:08:21Z DEBUG ferrite::editor::widget] Syncing 8 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:08:21Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:08:21Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:08:21Z DEBUG ferrite::editor::widget] Syncing 8 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:08:21Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:08:21Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:08:21Z DEBUG ferrite::editor::widget] Syncing 8 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:08:21Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:08:21Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:08:21Z DEBUG ferrite::editor::widget] Syncing 8 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:08:21Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:08:21Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:08:21Z DEBUG ferrite::editor::widget] Syncing 8 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:08:21Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0

```

### T2-36 — last ~30 non-span log lines
```text
[2026-11-09T10:08:53Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:08:53Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:08:53Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:08:53Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:08:54Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:08:54Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:08:54Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:08:54Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 250, y: 183 }) size=Physical(PhysicalSize { width: 1530, height: 963 })
[2026-11-09T10:08:54Z DEBUG ferrite::app::title_bar] Title bar: Set view mode to Rendered
[2026-11-09T10:08:54Z DEBUG ureq::stream] connecting to img.youtube.com:443 at 198.18.0.1:443
[2026-11-09T10:08:54Z DEBUG rustls::client::hs] No cached session for DnsName("img.youtube.com")
[2026-11-09T10:08:54Z DEBUG rustls::client::hs] Not resuming any session

(ferrite:493301): dbind-WARNING **: 11:08:54.438: AT-SPI: Error retrieving accessibility bus address: org.freedesktop.DBus.Error.ServiceUnknown: The name org.a11y.Bus was not provided by any .service files
[2026-11-09T10:08:54Z INFO  ferrite::markdown::video_render::linux_gtk] GTK initialised on X11 for WebKitGTK video embeds
[2026-11-09T10:08:54Z DEBUG rustls::client::hs] Using ciphersuite TLS13_AES_256_GCM_SHA384
[2026-11-09T10:08:54Z DEBUG rustls::client::tls13] Not resuming
[2026-11-09T10:08:54Z DEBUG rustls::client::tls13] TLS1.3 encrypted extensions: ServerExtensions { unknown_extensions: {}, .. }
[2026-11-09T10:08:54Z DEBUG rustls::client::hs] ALPN protocol is None
[2026-11-09T10:08:54Z DEBUG ureq::stream] created stream: Stream(RustlsStream)
[2026-11-09T10:08:54Z DEBUG ureq::unit] sending request GET https://img.youtube.com/vi/dQw4w9WgXcQ/hqdefault.jpg
[2026-11-09T10:08:54Z DEBUG ureq::unit] writing prelude: GET /vi/dQw4w9WgXcQ/hqdefault.jpg HTTP/1.1
    Host: img.youtube.com
    User-Agent: ureq/2.12.1
    Accept: */*
[2026-11-09T10:08:54Z DEBUG ureq::response] Streaming body until content-length: 21011
[2026-11-09T10:08:54Z DEBUG ureq::unit] response 200 to GET https://img.youtube.com/vi/dQw4w9WgXcQ/hqdefault.jpg
[2026-11-09T10:08:54Z DEBUG ureq::stream] dropping stream: Stream(RustlsStream)
[2026-11-09T10:08:54Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=false
Failed to create hard link from /home/box/.cache/ferrite/WebKitCache/Version 17/Blobs/DC9F3F5B563DAAD33942B8D55BACE2BED7C8B227 to /home/box/.cache/ferrite/WebKitCache/Version 17/Records/2E2D8F4ED5B89FB354A1731D4268E39E3657674A/Resource/C4F2E64EB7BA45FFFB6CDCCCEB0477D7A08B82E7-blob

```

### VID-1 — last ~30 non-span log lines
```text
[2026-11-09T10:10:03Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:10:03Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:10:03Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:10:03Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:10:03Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:10:03Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:10:03Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:10:03Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:10:03Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 250, y: 209 }) size=Physical(PhysicalSize { width: 1530, height: 937 })
[2026-11-09T10:10:03Z DEBUG ferrite::app::title_bar] Title bar: Set view mode to Rendered
[2026-11-09T10:10:03Z DEBUG ureq::stream] connecting to img.youtube.com:443 at 198.18.0.1:443
[2026-11-09T10:10:03Z DEBUG rustls::client::hs] No cached session for DnsName("img.youtube.com")
[2026-11-09T10:10:03Z DEBUG rustls::client::hs] Not resuming any session

(ferrite:495216): dbind-WARNING **: 11:10:03.950: AT-SPI: Error retrieving accessibility bus address: org.freedesktop.DBus.Error.ServiceUnknown: The name org.a11y.Bus was not provided by any .service files
[2026-11-09T10:10:03Z INFO  ferrite::markdown::video_render::linux_gtk] GTK initialised on X11 for WebKitGTK video embeds
[2026-11-09T10:10:03Z DEBUG rustls::client::hs] Using ciphersuite TLS13_AES_256_GCM_SHA384
[2026-11-09T10:10:03Z DEBUG rustls::client::tls13] Not resuming
[2026-11-09T10:10:03Z DEBUG rustls::client::tls13] TLS1.3 encrypted extensions: ServerExtensions { unknown_extensions: {}, .. }
[2026-11-09T10:10:03Z DEBUG rustls::client::hs] ALPN protocol is None
[2026-11-09T10:10:03Z DEBUG ureq::stream] created stream: Stream(RustlsStream)
[2026-11-09T10:10:03Z DEBUG ureq::unit] sending request GET https://img.youtube.com/vi/dQw4w9WgXcQ/hqdefault.jpg
[2026-11-09T10:10:03Z DEBUG ureq::unit] writing prelude: GET /vi/dQw4w9WgXcQ/hqdefault.jpg HTTP/1.1
    Host: img.youtube.com
    User-Agent: ureq/2.12.1
    Accept: */*
[2026-11-09T10:10:03Z DEBUG ureq::response] Streaming body until content-length: 21011
[2026-11-09T10:10:03Z DEBUG ureq::unit] response 200 to GET https://img.youtube.com/vi/dQw4w9WgXcQ/hqdefault.jpg
[2026-11-09T10:10:03Z DEBUG ureq::stream] dropping stream: Stream(RustlsStream)
[2026-11-09T10:10:04Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=false

```

### EXTRA-YT-2 — last ~30 non-span log lines
```text
[2026-11-09T10:11:06Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:11:06Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:11:06Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:11:06Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:11:06Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:11:06Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:11:06Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:11:06Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:11:06Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:11:06Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:11:06Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:11:06Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:11:06Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:11:06Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:11:06Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:11:07Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:11:07Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:11:07Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:11:07Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:11:07Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:11:07Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:11:07Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:11:07Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:11:07Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:11:07Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:11:07Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:11:07Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:11:07Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:11:07Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:11:07Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0

```

### EXTRA-YT-4 — last ~30 non-span log lines
```text
[2026-11-09T10:11:48Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-11-09T10:11:48Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-11-09T10:11:48Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-11-09T10:11:48Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-11-09T10:11:48Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-11-09T10:11:48Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-11-09T10:11:48Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-11-09T10:11:48Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-11-09T10:11:48Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-11-09T10:11:48Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-11-09T10:11:48Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-11-09T10:11:48Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-11-09T10:11:48Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-11-09T10:11:49Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-11-09T10:11:49Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-11-09T10:11:49Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-11-09T10:11:49Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-11-09T10:11:49Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-11-09T10:11:49Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-11-09T10:11:49Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-11-09T10:11:49Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true

(ferrite:498001): Gdk-CRITICAL **: 11:11:49.640: _gdk_frame_clock_freeze: assertion 'GDK_IS_FRAME_CLOCK (clock)' failed

(ferrite:498001): Gdk-CRITICAL **: 11:11:49.641: _gdk_frame_clock_thaw: assertion 'GDK_IS_FRAME_CLOCK (clock)' failed
[2026-11-09T10:11:49Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-11-09T10:11:49Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-11-09T10:11:49Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-11-09T10:11:49Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-11-09T10:11:56Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true

```

### T1-14 — last ~30 non-span log lines
```text
[2026-11-09T10:20:13Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:20:13Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:20:13Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:20:13Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:20:13Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:20:13Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:20:13Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:20:13Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:20:13Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:20:13Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:20:13Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:20:13Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:20:13Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:20:13Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:20:13Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:20:13Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:20:13Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:20:13Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:20:13Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:20:13Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:20:13Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:20:13Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:20:13Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:20:13Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:20:13Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:20:13Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:20:13Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:20:13Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:20:13Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:20:13Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0

```

### IMG-X2 — last ~30 non-span log lines
```text
[2026-11-09T10:20:39Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:20:39Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:20:39Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:20:39Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:20:39Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:20:39Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:20:39Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:20:39Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:20:39Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:20:39Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:20:39Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:20:39Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:20:40Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:20:40Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:20:40Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:20:40Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:20:40Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:20:40Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:20:40Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:20:40Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:20:40Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:20:40Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:20:40Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:20:40Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:20:40Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:20:40Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:20:40Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:20:40Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:20:40Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-11-09T10:20:40Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0

```

### IMG-X1 — last ~30 non-span log lines
```text
[2026-11-09T10:21:22Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:21:22Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:21:22Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:21:22Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:21:22Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:21:22Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:21:22Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:21:22Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:21:22Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:21:22Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:21:23Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:21:23Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:21:23Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:21:23Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:21:23Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:21:23Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:21:23Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:21:23Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:21:23Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:21:23Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:21:23Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:21:23Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:21:23Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:21:23Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:21:23Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:21:23Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:21:23Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:21:23Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0
[2026-11-09T10:21:23Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-11-09T10:21:23Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1490.3, max_wrap_width=840.0, effective=840.0

```

## End checks
- Ferrite process: none (`pgrep -a -x ferrite` returned empty).
- `git -C /workspace/ferrite-qa/ferrite status --short`: empty (clean).
- Rows not reached: none.
