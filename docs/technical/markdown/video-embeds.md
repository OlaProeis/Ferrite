# Video Embeds

## Overview

Ferrite v0.3.1 embeds video in the WYSIWYG rendered markdown view via a two-path renderer:

1. **Primary (trusted YouTube):** wry child WebView positioned over the embed rect, loading a local relay page that hosts a YouTube iframe.
2. **Fallback (mandatory):** YouTube CDN thumbnail + play overlay → system browser; also used when WebView creation/positioning fails or the embed is untrusted.

Syntax and AST types are documented in [video-embed-parsing.md](./video-embed-parsing.md). Low-level render-path details also appear in [video-embed-rendering.md](./video-embed-rendering.md).

## Is it pure Rust?

**No — Rust orchestrates native components.** The feature is implemented in Rust (`video_render.rs`, `video_embed.rs`, `central_panel.rs`), but playback depends on platform WebView stacks:

| Layer | Technology |
|-------|------------|
| UI layout / lifecycle | **egui** (Rust) |
| WebView wrapper | **wry** 0.55 (Rust crate) |
| Windows WebView engine | **WebView2** (Chromium-based system component) |
| macOS / Linux | **WKWebView** / **WebKitGTK** (via wry) |
| Thumbnail fallback | **ureq** + **image** (Rust) → egui texture |
| Relay HTML | Static template string generated in Rust, served via wry custom protocol |

There is no separate Node/Electron runtime and no bundled Chromium — on Windows the OS-provided WebView2 runtime loads YouTube's iframe player.

## Render Paths

| Condition | UI |
|-----------|-----|
| Trusted YouTube with valid `video_id`, WebView succeeds, embed visible in preview pane | Inline wry child WebView at clipped embed rect (relay page → `youtube-nocookie.com` iframe) |
| Trusted YouTube, WebView fails | Thumbnail fallback (same as below) |
| Embed scrolled off-screen or outside preview pane clip | WebView destroyed; thumbnail fallback drawn when the block is in the render viewport |
| YouTube with valid `video_id` (fallback path) | Fetch `img.youtube.com/vi/<id>/hqdefault.jpg`, scale to pane width, play overlay, click → `open::that(url)` |
| Thumbnail fetch/decode failure | Text frame: “Video thumbnail unavailable” + clickable URL |
| Non-YouTube or missing video ID | Text frame: hint + clickable URL (untrusted embeds never get WebView) |

## Relay page (YouTube Error 153 fix)

YouTube **rejects top-level navigation** to `https://www.youtube.com/embed/{id}` inside a WebView (Error 153 — “Video player configuration error”). The embed player is only valid inside an iframe on a host page with a recognizable origin and Referer policy.

Ferrite's fix:

1. Child WebView navigates to `ferrite-video://localhost/embed?v={video_id}`.
2. On Windows, wry maps that to `https://ferrite-video.localhost/…` (WebView2 custom-protocol workaround).
3. A Rust `with_custom_protocol` handler serves a minimal HTML relay page (no network fetch for the host document).
4. The relay page sets `<meta name="referrer" content="strict-origin-when-cross-origin">` and creates an iframe pointing at `https://www.youtube-nocookie.com/embed/{id}` with `origin=location.origin`.

The user's raw watch URL is **never** loaded as the WebView's top-level document.

## Security Model

Trust and WebView eligibility are separate gates:

| Gate | Location | Rule |
|------|----------|------|
| **Parse allowlist** | `video_embed.rs` → `TRUSTED_VIDEO_HOSTS` | Only allowlisted YouTube hosts set `VideoEmbedInfo.trusted = true` |
| **WebView eligibility** | `video_render.rs` → `VideoWebViewManager::is_webview_eligible()` | Requires `trusted == true` **and** a validated YouTube `video_id` |
| **Video ID validation** | `video_render.rs` → `is_valid_youtube_video_id()` | ASCII alphanumeric + `_` + `-` only before insertion into relay HTML |
| **Navigation scope** | `create_child_webview()` | Top-level URL is always the fixed `ferrite-video://` relay; iframe `src` is always `youtube-nocookie.com/embed/{validated_id}` |
| **Navigation allowlist** | `is_allowed_webview_navigation()` | `with_navigation_handler` blocks (and logs) any navigation that is not the relay page (`ferrite-video://` or its `{http,https}://ferrite-video.localhost` Windows mapping) or `https://www.youtube-nocookie.com/` (WKWebView routes iframe navigations through the same handler) |
| **New-window guard** | `create_child_webview()` | `with_new_window_req_handler` returns `Deny` for all popups; `http(s)` popup URLs (e.g. "Watch on YouTube") open in the **system browser** via `open::that` |

Untrusted embeds never reach `WebViewBuilder::build_as_child`. No arbitrary URL iframes. Non-allowlisted braced URLs parse as `VideoEmbed` with `trusted: false` and use thumbnail/text fallback only.

### Safety notes

- **Deliberate trade-off:** Inline playback runs YouTube's third-party JavaScript inside WebView2/WebKit (same as a browser tab). This is required for the official embed player.
- **Mitigations:** Host allowlist, strict video-ID charset gate, no user-controlled top-level WebView URLs, WebView destroyed on tab/view-mode change, `with_focused(false)` + `focus_parent()` on teardown to avoid HWND focus hijack on Windows.
- **Fallback path:** Thumbnail click opens the original watch URL in the **system browser** via `open::that`, not inside Ferrite's WebView.
- **Not a sandbox escape:** The WebView is a child overlay bounded to the embed rect and preview-pane clip; it does not execute markdown or user HTML.

## WebView Path (wry)

`VideoWebViewManager` (owned on `FerriteApp`) tracks active child WebViews keyed by `{tab_id}:{video_id}`.

Each rendered-markdown frame:

1. `central_panel.rs` calls `push_video_webview_render_slot()` with the preview-pane clip rect and parent `eframe::Frame` handle; `begin_frame()` clears the active-embed set.
2. For each visible trusted embed, `render_video_embed()` allocates a 16:9 rect and calls `sync_trusted_embed()` — create or `set_bounds` reposition.
3. `pop_video_webview_render_slot()` calls `end_frame()` and drops WebViews not seen that frame.

Coordinates: egui layer rect → global viewport via `Context::layer_transform_to_global`, then wry `LogicalPosition`/`LogicalSize`.

**Thread-local render slot:** wry `WebView` is not `Send`, so the manager cannot live in egui temp data. A UI-thread `thread_local` slot bridges `central_panel` and `video_render` during `MarkdownEditor::show`.

**Failure handling:** `create_child_webview` and `set_bounds` errors log a warning and return false; `render_video_embed` then draws the thumbnail fallback in the same rect. Failed create attempts are recorded in `failed_embeds` to avoid per-frame recreate storms. No panics, no `unwrap` on the hot path.

## Focus handling (WebView2 input lock)

**Not by design:** a playing embed must not block the rest of Ferrite. On Windows, WebView2 child HWNDs can retain focus after you click the YouTube player, which prevents the custom title bar from receiving drags and the split-view raw pane from receiving keyboard input until the WebView is destroyed (e.g. tab switch).

Each frame, `end_frame()` calls `focus_parent()` on active embed WebViews when:

- The pointer is **not** over any synced embed rect, or
- The pointer is over a **focus-priority** Ferrite rect (split raw pane, minimap, splitter, format bar, or title-bar band).

Pointer over the embed rect keeps WebView focus so YouTube controls still work. Video continues playing when focus returns to the parent window.

## Modal / overlay z-order

**Not by design:** WebView2 child HWNDs are always composited above the egui/glow surface. Foreground egui UI (unsaved-changes dialog, quick switcher, command palette, find/replace, export dialogs, etc.) would otherwise appear **under** the video.

At the end of each frame, `VideoWebViewManager::apply_foreground_occlusion()` hides (`set_visible(false)`) only embed WebViews whose screen rect **intersects** a collected foreground overlay rect (unsaved-changes dialog, quick switcher panel, command palette, etc.). Overlay rects are converted to screen space with `egui_rect_to_screen` (same transform as embed positioning); a small margin accounts for panel shadows. Small overlays that do not overlap the video leave playback visible. When overlap ends, the next sync frame shows the WebView again without reloading the embed.

**Ordering invariant:** `apply_foreground_occlusion()` runs at the end of `render_central_panel()`, *after* `end_frame()` and after all overlays have rendered and pushed their occluder rects. `embed_screen_rects_this_frame` is therefore **kept alive across `end_frame()`** and only cleared at the start of the next rendered frame in `begin_frame()` (and in `clear_all()`). Clearing it in `end_frame()` was a regression: the late occlusion pass saw an empty rect map, every intersection test returned false, and `set_visible(true)` re-surfaced the video above modals each frame. Guarded by the `embed_rects_survive_end_frame_for_late_occlusion_pass` unit test.

## Overlay Lifecycle

Lifecycle is managed in `central_panel.rs` and `video_render.rs`:

| Event | Behavior |
|-------|----------|
| **Scroll off-screen** | Embed rect intersected with `ui.clip_rect()` and preview-pane clip; if not visible (`is_rect_visible`) or too small, WebView is not synced → dropped at `end_frame` |
| **Preview pane clip** | Split view passes `right_rect`; rendered-only mode passes `ui.clip_rect()` — WebView bounds never extend outside the preview pane |
| **Tab switch** | Inactive tab's embed keys are not seen → `end_frame` drops them; if the new tab is not rendered markdown, `clear_all()` runs at end of frame |
| **Rendered ↔ Raw** | Raw/Split-left frames never push the render slot → `clear_all()` destroys all WebViews (no orphaned overlays) |
| **Special/viewer tabs** | Same as Raw — `clear_all()` at frame end |
| **Return to rendered view** | Next frame re-syncs visible embeds and recreates WebViews as needed |

`VideoWebViewManager::clear_all()` calls `focus_parent()` on each active WebView before drop, then clears `failed_embeds` and the embed rect map. Invoked when `video_webview_frame_active` is false at the end of a **primary-viewport** `render_central_panel()` pass.

## Multi-window (primary viewport only)

**Inline WebView playback exists only in the primary window.** Secondary document windows (child viewports, see [multi-window.md](../platform/multi-window.md)) render video embeds via the thumbnail fallback. Three reasons:

1. `VideoWebViewParent::from_frame(frame)` exposes the **primary** window handle only — a WebView created during a secondary pass would be parented to (and painted over) the primary window.
2. Secondary passes reuse `render_central_panel()` with the shared `VideoWebViewManager`. Without gating, each secondary pass ran `begin_frame`/`end_frame` (dropping the primary's WebViews as "stale") or `clear_all()` (when its tab isn't rendered markdown) — destroying and recreating the player **every frame**, so the video never loaded ("player disappears when a second window is open").
3. Occluder rects from secondary viewports are in that window's coordinate space and would produce spurious intersections against primary embed rects.

Gating: `render_central_panel()` computes `is_primary_viewport = ctx.viewport_id() == ViewportId::ROOT`; the render slot, `apply_foreground_occlusion()`, and `clear_all()` only run when it is true. `FerriteApp::push_video_occluder_rect()` ignores pushes while `state.working_window_id != PRIMARY_WINDOW_ID` (reset after `render_secondary_document_windows()`).

## Thumbnail Fallback

1. `youtube_thumbnail_url(info)` builds `https://img.youtube.com/vi/{id}/hqdefault.jpg` when `provider == YouTube` and `video_id` is non-empty.
2. First render: synchronous `ureq` fetch (10s timeout), decode via `image`, upload to egui `TextureHandle`.
3. Result cached in egui temp data keyed by thumbnail URL.
4. Failed loads cache `Failed` and show text fallback — no retry storm.

Thumbnail click calls `open::that(&info.url)` and sets `link_click_consumed_this_frame`.

## Preview Lock

Video playback is a **read action** — it is not gated by preview lock. WebView sync and thumbnail interaction proceed regardless of edit-lock state on the preview pane.

## Key Files

| File | Purpose |
|------|---------|
| `src/markdown/video_embed.rs` | Allowlist (`TRUSTED_VIDEO_HOSTS`) — sole source of `trusted: true` |
| `src/markdown/video_render.rs` | `VideoWebViewManager`, relay protocol, `render_video_embed()`, thumbnail fetch, clip/visibility gates |
| `src/markdown/editor.rs` | `MarkdownNodeType::VideoEmbed` render arm |
| `src/app/mod.rs` | `FerriteApp.video_webview_manager`, occluder push helpers (primary-window gated) |
| `src/app/central_panel.rs` | `push`/`pop` render slot, pane clip rect, primary-viewport gate, `clear_all()` lifecycle |
| `src/app/windows.rs` | Secondary viewports — never touch the WebView manager; resets `working_window_id` |
| `locales/en.yaml` | `markdown.video_embed.*` user-facing strings |

## Tests

Unit tests in `src/markdown/video_render.rs`:

- `youtube_thumbnail_url_*`, `provider_embed_url_*` (relay page URL)
- `relay_html_includes_video_id`, `video_id_from_relay_uri_parses_query`
- `untrusted_embed_never_webview_eligible`, `webview_gate_blocks_untrusted_before_constructor`
- `force_fallback_skips_webview_path`, `clear_all_empties_manager_state`
- `embed_rects_survive_end_frame_for_late_occlusion_pass`, `begin_frame_resets_embed_rects`, `occluder_intersection_respects_margin` (modal z-order / occlusion)
- `navigation_allowlist_permits_relay_and_player_only` (WebView navigation lockdown)

Run:

```bash
cargo test video_render
```

## Related

- [Video embed parsing](./video-embed-parsing.md)
- [Video embed rendering](./video-embed-rendering.md)
- GitHub [#119](https://github.com/OlaProeis/Ferrite/issues/119)
- PRD §5.2 / §5.3 — [prd-v0.3.1.md](../../ai-workflow/prds/prd-v0.3.1.md)
