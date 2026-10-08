# Video Embed Rendering

## Overview

Ferrite v0.3.1 renders `MarkdownNodeType::VideoEmbed` nodes in the WYSIWYG rendered view with two paths:

1. **Primary (trusted YouTube):** wry child WebView positioned over the embed rect each frame, loading a local relay page that embeds YouTube via iframe.
2. **Fallback (mandatory):** YouTube CDN thumbnail + play overlay → system browser; also used when WebView creation/positioning fails or the embed is untrusted.

Parsing and AST types are documented in [video-embed-parsing.md](./video-embed-parsing.md). Architecture, security, and lifecycle are documented in [video-embeds.md](./video-embeds.md).

## Render Paths

| Condition | UI |
|-----------|-----|
| Trusted YouTube with valid `video_id`, WebView succeeds | Inline wry child WebView at embed rect (relay page → `youtube-nocookie.com` iframe); rect sized by `video_display_size()` |
| Trusted YouTube, WebView fails | Thumbnail fallback (same as below) |
| YouTube with valid `video_id` (fallback path) | Fetch `img.youtube.com/vi/<id>/hqdefault.jpg`, scale to pane width, play overlay, click → `open::that(url)` |
| Thumbnail fetch/decode failure | Text frame: “Video thumbnail unavailable” + clickable URL |
| Non-YouTube or missing video ID | Text frame: hint + clickable URL (untrusted embeds never get WebView) |

## Relay page flow

Direct navigation to `https://www.youtube.com/embed/{id}` inside a WebView triggers YouTube **Error 153**. Ferrite instead:

```
Markdown render
  → sync_trusted_embed()
    → create_child_webview()
      → with_custom_protocol("ferrite-video", serve_youtube_embed_relay)
      → with_url("ferrite-video://localhost/embed?v={id}")
      → HTML served in-process:
           <iframe src="https://www.youtube-nocookie.com/embed/{id}?…&origin={location.origin}">
```

`provider_embed_url()` / `provider_relay_page_url()` return the relay URL (`ferrite-video://localhost/embed?v=…`), not the YouTube embed URL.

## Security Gate

The **single gate** for the WebView path is `VideoWebViewManager::is_webview_eligible(info)`:

- Requires `VideoEmbedInfo.trusted == true` (set only by the allowlist in `video_embed.rs`).
- Requires a validated YouTube `video_id` (charset-checked before HTML generation).

Untrusted embeds never reach `WebViewBuilder::build_as_child`. The iframe `src` is constructed only from validated IDs; user watch URLs are not used as WebView top-level navigation.

## Key Files

| File | Purpose |
|------|---------|
| `src/markdown/video_render.rs` | `VideoWebViewManager`, relay protocol, `render_video_embed()`, thumbnail fetch + WebView sync |
| `src/markdown/video_embed.rs` | Allowlist, `format_video_embed_source`, `rewrite_video_embed_dimensions` |
| `src/markdown/editor.rs` | `MarkdownNodeType::VideoEmbed` render arm; resize commit → source + `EditState` |
| `src/app/mod.rs` | `FerriteApp.video_webview_manager` |
| `src/app/central_panel.rs` | `push_video_webview_render_slot` / `pop_video_webview_render_slot` around rendered editor show |
| `locales/en.yaml` | `markdown.video_embed.*` user-facing strings |

## Display sizing

`video_display_size(info, available_width)` in `video_render.rs` allocates the egui rect (and WebView bounds follow via `set_bounds`):

| `VideoEmbedInfo` dimensions | Display size |
|-----------------------------|--------------|
| Neither set | Full pane width, 16:9 (`EMBED_ASPECT_RATIO`) |
| `width` only | Explicit width; height = width × 9/16 |
| `height` only | Explicit height; width = height ÷ 9/16 |
| Both set | Exact width × height |

When the target width exceeds `ui.available_width()`, scale down proportionally (same pattern as image embeds). Thumbnail fallback uses the same rect.

Syntax for explicit dimensions: see [video-embed-parsing.md](./video-embed-parsing.md).

## Drag-resize (source write-back)

When preview is unlocked, each embed shows a bottom-right drag handle (`Sense::drag()` in `render_video_embed`).

| Phase | Behaviour |
|-------|-----------|
| Hover / drag | Pending size stored in egui temp data keyed by `(source_line, url)`; layout rect updates live |
| Drag release | `VideoEmbedResizeCommit { width, height }` returned; editor calls `rewrite_video_embed_dimensions()` and `mark_line_modified` |
| After commit | Pending size kept until AST `width`/`height` match (avoids flicker before `rebuild_markdown`) |

**WebView interaction:** Child HWNDs sit above egui and would block the handle. While the handle is hovered or a drag is active, `try_render_webview_overlay` is skipped for that embed so the thumbnail underlay receives input.

**Source helpers** (`video_embed.rs`):

- `format_video_embed_source(url, width, height)` — builds `{{video URL width=N height=N}}`
- `rewrite_video_embed_dimensions(source, line, info, width, height)` — replaces the source line (clamp `1..=8192`)

Drag-resize always writes both `width` and `height`. Minimum drag size: 160×90 logical px. Disabled when preview-locked (`VideoEmbedResizeContext::enabled == false`).

## WebView Manager

`VideoWebViewManager` (owned on `FerriteApp`) tracks active child WebViews keyed by `{tab_id}:{video_id}:{source_line}` (`embed_webview_key()`).

**Key uniqueness (regression):** the key *must* include the source line. A document can embed the same video several times (e.g. the `VID-*` test fixture uses one video ID for four embeds); keying by video ID alone made all occurrences share one WebView, whose bounds were re-set once per occurrence per frame — the single HWND visibly jumped between embed slots (flicker / "doubling"). Line-based keys stay stable while scrolling; edits that shift lines above an embed recreate its WebView (player restarts), which is the accepted trade-off.

Each rendered frame:

1. `push_video_webview_render_slot()` captures the parent `eframe::Frame` handle and calls `begin_frame()` (which also drains fullscreen IPC events).
2. For each visible trusted embed, `render_video_embed()` allocates a rect via `video_display_size()` and calls `sync_trusted_embed()` — create, or `set_bounds` reposition **only when the screen rect actually changed** (`rects_approx_eq`, 0.1 px epsilon; avoids per-frame native churn).
3. `pop_video_webview_render_slot()` calls `end_frame()`; unsynced WebViews are hidden immediately and destroyed only after `WEBVIEW_STALE_GRACE` (2.5 s) — WebView2 creation costs ~100 ms, and destroying on the first unseen frame made scroll-back stutter and restarted playback.

Coordinates: egui layer rect → global viewport via `Context::layer_transform_to_global`, then wry `LogicalPosition`/`LogicalSize`.

**Partial visibility (window-region clipping, Windows):** the child HWND cannot be clipped by egui scroll areas, so previously the WebView was hidden unless the embed was *fully* inside the pane — scrolling constantly swapped player↔thumbnail. Now `sync_container_region()` applies `SetWindowRgn` on the WebView2 container (container-local **physical** px via `compute_container_region`), so partially scrolled embeds keep showing the live player, clipped to the pane. On platforms without region support, the fully-visible gating remains (`region_clipping_supported()`).

**Fullscreen (window-fullscreen):** the relay page listens for `fullscreenchange` (fullscreen propagates from the player iframe to the top document) and reports `fullscreen:on/off` via `window.ipc.postMessage`; `with_ipc_handler` queues the event and the manager applies it in `begin_frame`. While an embed is fullscreen its WebView bounds are `ctx.viewport_rect()` (covers the app window — not the OS screen), the region clip is removed, stale-drop is exempted, and the wheel hook steps aside (`set_video_fullscreen_active`) so the player owns wheel input and the document does not scroll underneath. ESC exits via the browser's own fullscreen handling → `off` event → bounds restore next frame.

**Thread-local render slot:** wry `WebView` is not `Send`, so the manager cannot live in egui temp data. A UI-thread `thread_local` slot bridges `central_panel` and `video_render` during `MarkdownEditor::show`.

**Focus handling:**

- Child WebViews use `with_focused(false)`, but WebView2 can still grab focus during creation — which happens exactly when an embed scrolls fully into view. `sync_trusted_embed` calls `focus_parent()` immediately after a successful create so scrolling/shortcuts keep working.
- `end_frame()` yields focus back to Ferrite **edge-triggered** (tracked by `focus_yielded`): once when the pointer leaves the embed rects, not every frame. Per-frame `focus_parent()` calls disrupted in-flight scrolling.
- `set_visible` is applied only on state transitions (tracked per `ActiveWebView.visible`); the hidden→shown/shown→hidden edges also drive `focus_parent()`.
- `clear_all()`, stale removal, and `set_bounds` failure paths go through `drop_webview_entry()`, which calls `focus_parent()` and unregisters the container HWND from the wheel hook.

**Wheel-hook HWND hygiene:** `install_wheel_forwarding` returns the WebView2 container HWND; the manager stores it per entry and calls `unregister_webview_container()` on teardown. Windows reuses HWND values — stale entries in the hook's container set could swallow wheel events over unrelated windows, and stale subclass records prevented re-subclassing reused HWNDs (scroll silently breaking after embeds were destroyed/recreated).

**Wheel forwarding (`video_webview_input.rs`):** the low-level mouse hook queues wheel events (delta + modifiers + **cursor screen position**) and swallows them; `drain_pending_wheel_into_egui` injects synthetic `PointerMoved` + `MouseWheel` events each frame. The position injection is mandatory: while the cursor is over a child WebView HWND the main window gets `WM_MOUSELEAVE`, egui's pointer goes `None`, and a bare wheel event has no scroll-area target — scroll silently stopped the moment the cursor entered a video. The hook wakes the event loop via `request_video_repaint()` (a stored `egui::Context`), since the swallowed event never reaches the main window.

**Failure handling:** `create_child_webview` and `set_bounds` errors log a warning and return false; `render_video_embed` then draws the thumbnail fallback in the same rect. Failed creates are stored in `failed_embeds` (cleared on `clear_all()`). No panics, no `unwrap` on the hot path.

## Thumbnail Fallback

1. `youtube_thumbnail_url(info)` builds `https://img.youtube.com/vi/{id}/hqdefault.jpg` when `provider == YouTube` and `video_id` is non-empty.
2. First render inserts a `Loading` cache entry and spawns a background thread (`spawn_thumbnail_fetch`, `ureq` with 10s timeout, decode via `image`, upload to egui `TextureHandle`, then `request_repaint`). A dark placeholder with the play affordance renders while loading — the fetch must never block the UI thread.
3. Result cached in egui temp data keyed by thumbnail URL (`Loading` → `Loaded`/`Failed`).
4. Failed loads cache `Failed` and show text fallback — no retry storm.

## Interaction

- Thumbnail click calls `open::that(&info.url)` and sets `link_click_consumed_this_frame`.
- Play overlay: semi-transparent dim + circle + triangle via `ui.painter()`.
- Hover tooltip: `markdown.video_embed.play_tooltip` (trusted) or `untrusted_hint`.
- Drag-resize handle: bottom-right corner grip; `ResizeNwSe` cursor; writes `width`/`height` to source on release (see above).

## i18n Keys

```yaml
markdown:
  video_embed:
    play_tooltip: "Play video in browser"
    open_in_browser: "Open in browser"
    thumbnail_failed: "Video thumbnail unavailable"
    untrusted_hint: "External video (opens in browser)"
```

## Tests

Unit tests in `src/markdown/video_render.rs`:

- `youtube_thumbnail_url_*`, `provider_embed_url_*`, `video_display_size_*`, `clamp_display_size_scales_down_wide_rect`
- `format_video_embed_source_*`, `rewrite_video_embed_dimensions_*` (in `video_embed.rs`)
- `relay_html_includes_video_id`, `video_id_from_relay_uri_parses_query`
- `untrusted_embed_never_webview_eligible`, `webview_gate_blocks_untrusted_before_constructor`
- `force_fallback_skips_webview_path`, `clear_all_empties_manager_state`
- `embed_webview_key_unique_per_source_line` (same-video-multiple-times regression)
- `rects_approx_eq_tolerates_sub_epsilon_jitter`, `end_frame_focus_yield_is_edge_triggered`

Run:

```bash
cargo test video_render
```

## Related

- [Video embed parsing](./video-embed-parsing.md)
- [Video embeds](./video-embeds.md) — architecture, pure-Rust vs native stack, security
- GitHub [#119](https://github.com/OlaProeis/Ferrite/issues/119)
- PRD §5.2 — [prd-v0.3.1.md](../../ai-workflow/prds/prd-v0.3.1.md)
