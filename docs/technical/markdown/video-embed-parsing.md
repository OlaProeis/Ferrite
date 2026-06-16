# Video Embed Parsing

## Overview

Ferrite v0.3.1 adds AST support for embedded video syntax in markdown. Parsing runs as a post-processing pass after comrak conversion and wikilink extraction. **Rendering** (wry WebView overlay + thumbnail fallback) is documented in [video-embed-rendering.md](./video-embed-rendering.md).

## Supported Syntax

| Form | Example | AST result |
|------|---------|------------|
| Braced | `{{video https://youtube.com/watch?v=abc}}` | `VideoEmbed` (YouTube if allowlisted) |
| Braced + width | `{{video https://youtu.be/abc width=640}}` | `VideoEmbed` with `width: Some(640)` |
| Braced + both | `{{video https://youtu.be/abc width=640 height=360}}` | `VideoEmbed` with explicit dimensions |

Optional `width` / `height` params use `key=value` tokens after the URL (whitespace-separated). Unknown keys are ignored; invalid values are skipped. Parsed dimensions are clamped to `1..=8192`. The full braced line is stored verbatim in `source_text` for round-trip (including unknown params like `autoplay=1`).

Bare YouTube/youtu.be URLs in a paragraph stay normal paragraphs with comrak autolink `Link` nodes — they do **not** become `VideoEmbed`. Braced non-YouTube URLs become `VideoEmbed` with `trusted: false` (no WebView path).

## Key Files

| File | Purpose |
|------|---------|
| `src/markdown/parser.rs` | `VideoProvider`, `VideoEmbedInfo`, `MarkdownNodeType::VideoEmbed` |
| `src/markdown/video_embed.rs` | URL parsing, `parse_braced_video_content()` (URL + params), allowlist, `extract_video_embeds()` post-pass |
| `src/markdown/widgets.rs` | `serialize_node` round-trip via `source_text` |

## AST Types

```rust
pub enum VideoProvider { YouTube, Unknown }

pub struct VideoEmbedInfo {
    pub provider: VideoProvider,
    pub video_id: Option<String>,   // YouTube ID when provider is YouTube
    pub url: String,               // Normalized http/https URL
    pub trusted: bool,             // Allowlisted domain → WebView-eligible
    pub width: Option<u32>,        // From `width=N` param (clamped)
    pub height: Option<u32>,       // From `height=N` param (clamped)
    pub source_text: String,       // Original markdown (round-trip preserved)
}

// MarkdownNodeType variant:
VideoEmbed(VideoEmbedInfo)
```

## Parse Pipeline

Called from `parse_markdown_with_options()` after `extract_wikilinks()`:

1. Walk block-level children recursively.
2. For each `Paragraph`, try `try_parse_video_paragraph()` (braced `{{video …}}` only).
3. On match, replace paragraph with a leaf `VideoEmbed` node (same line span).

## URL Handling

- Parsed with the `url` crate; only `http` / `https` accepted.
- **Trusted hosts** (WebView allowlist): `youtube.com`, `www.youtube.com`, `m.youtube.com`, `music.youtube.com`, `youtu.be`, `www.youtu.be`.
- YouTube ID extraction: `watch?v=`, `youtu.be/<id>`, `/embed/`, `/shorts/`, `/v/`.

Public helper for downstream rendering:

```rust
pub fn parse_video_embed_url(raw_url: &str) -> Option<VideoEmbedInfo>
```

Re-exported from `crate::markdown`.

## Round-Trip

- `VideoEmbedInfo.source_text` stores the exact paragraph source.
- `MarkdownNode::text_content()` returns `source_text` for embed nodes.
- `widgets::serialize_node` writes `source_text` unchanged — raw/rendered switches do not rewrite markdown.

## Security

- No arbitrary iframe URLs on the trusted path — domain allowlist gates `trusted: true`.
- Non-allowlisted braced URLs parse as `VideoEmbed` but `trusted: false` (thumbnail/text fallback only; see [video-embed-rendering.md](./video-embed-rendering.md)).
- `javascript:` and other schemes rejected at parse time (`None` from URL parser path).

## Tests

Unit tests in `src/markdown/video_embed.rs` (`#[cfg(test)]`):

- Braced YouTube watch + youtu.be URLs
- `width` / `width+height` param parsing, clamping, invalid/unknown key handling
- Bare YouTube paragraph stays paragraph (autolink `Link`, not `VideoEmbed`)
- Braced Vimeo → untrusted embed
- Mixed paragraph text + URL stays paragraph

Run (when test harness compiles):

```bash
cargo test video_embed
```

## Related

- PRD §5.3 — [prd-v0.3.1.md](../../ai-workflow/prds/prd-v0.3.1.md)
- GitHub [#119](https://github.com/OlaProeis/Ferrite/issues/119)
