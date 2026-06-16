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

`VideoWebViewManager` (owned on `FerriteApp`) tracks active child WebViews keyed by `{tab_id}:{video_id}`.

Each rendered frame:

1. `push_video_webview_render_slot()` captures the parent `eframe::Frame` handle and calls `begin_frame()`.
2. For each visible trusted embed, `render_video_embed()` allocates a rect via `video_display_size()` and calls `sync_trusted_embed()` — create or `set_bounds` reposition.
3. `pop_video_webview_render_slot()` calls `end_frame()` and drops WebViews not seen that frame.

Coordinates: egui layer rect → global viewport via `Context::layer_transform_to_global`, then wry `LogicalPosition`/`LogicalSize`.

**Thread-local render slot:** wry `WebView` is not `Send`, so the manager cannot live in egui temp data. A UI-thread `thread_local` slot bridges `central_panel` and `video_render` during `MarkdownEditor::show`.

**Focus handling:** Child WebViews use `with_focused(false)`. `clear_all()`, stale removal, and `set_bounds` failure paths call `focus_parent()` so WebView2 does not retain HWND focus after teardown.

**Failure handling:** `create_child_webview` and `set_bounds` errors log a warning and return false; `render_video_embed` then draws the thumbnail fallback in the same rect. Failed creates are stored in `failed_embeds` (cleared on `clear_all()`). No panics, no `unwrap` on the hot path.

## Thumbnail Fallback

1. `youtube_thumbnail_url(info)` builds `https://img.youtube.com/vi/{id}/hqdefault.jpg` when `provider == YouTube` and `video_id` is non-empty.
2. First render: synchronous `ureq` fetch (10s timeout), decode via `image`, upload to egui `TextureHandle`.
3. Result cached in egui temp data keyed by thumbnail URL.
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

Run:

```bash
cargo test video_render
```

## Related

- [Video embed parsing](./video-embed-parsing.md)
- [Video embeds](./video-embeds.md) — architecture, pure-Rust vs native stack, security
- GitHub [#119](https://github.com/OlaProeis/Ferrite/issues/119)
- PRD §5.2 — [prd-v0.3.1.md](../../ai-workflow/prds/prd-v0.3.1.md)
