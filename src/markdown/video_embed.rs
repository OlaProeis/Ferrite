//! Video embed parsing for markdown documents.
//!
//! Recognizes explicit `{{video URL}}` syntax in standalone paragraphs.
//! Trusted domains (YouTube / youtu.be) may use interactive WebView overlays; others
//! fall back to thumbnail-only rendering (handled in a later task).

use super::parser::{
    MarkdownNode, MarkdownNodeType, VideoEmbedInfo, VideoProvider,
};

const MIN_VIDEO_DIMENSION: u32 = 1;
const MAX_VIDEO_DIMENSION: u32 = 8192;

const TRUSTED_VIDEO_HOSTS: &[&str] = &[
    "youtube.com",
    "www.youtube.com",
    "m.youtube.com",
    "music.youtube.com",
    "youtu.be",
    "www.youtu.be",
];

/// Parse a URL string into video embed metadata.
///
/// Returns `None` when the URL is missing, uses a non-http(s) scheme, or has no host.
pub fn parse_video_embed_url(raw_url: &str) -> Option<VideoEmbedInfo> {
    parse_video_embed_url_with_source(raw_url, raw_url.trim().to_string(), None, None)
}

fn parse_video_embed_url_with_source(
    raw_url: &str,
    source_text: String,
    width: Option<u32>,
    height: Option<u32>,
) -> Option<VideoEmbedInfo> {
    let parsed = url::Url::parse(raw_url.trim()).ok()?;
    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return None;
    }

    let host = parsed.host_str()?.to_ascii_lowercase();
    let trusted = is_trusted_video_host(&host);
    let url_string = parsed.as_str().to_string();

    if trusted {
        if let Some(video_id) = extract_youtube_video_id(&parsed) {
            return Some(VideoEmbedInfo {
                provider: VideoProvider::YouTube,
                video_id: Some(video_id),
                url: url_string,
                trusted: true,
                width,
                height,
                source_text,
            });
        }
    }

    Some(VideoEmbedInfo {
        provider: VideoProvider::Unknown,
        video_id: None,
        url: url_string,
        trusted,
        width,
        height,
        source_text,
    })
}

fn is_trusted_video_host(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    TRUSTED_VIDEO_HOSTS.iter().any(|&trusted| host == trusted)
}

fn extract_youtube_video_id(parsed: &url::Url) -> Option<String> {
    let host = parsed.host_str()?.to_ascii_lowercase();

    if host == "youtu.be" || host == "www.youtu.be" {
        let id = parsed.path().trim_start_matches('/');
        if !id.is_empty() && !id.contains('/') {
            return Some(id.to_string());
        }
        return None;
    }

    if host.ends_with("youtube.com") {
        for (key, value) in parsed.query_pairs() {
            if key == "v" && !value.is_empty() {
                return Some(value.into_owned());
            }
        }

        let path = parsed.path();
        for prefix in ["/embed/", "/shorts/", "/v/"] {
            if let Some(rest) = path.strip_prefix(prefix) {
                let id = rest.split('/').next().filter(|id| !id.is_empty())?;
                return Some(id.to_string());
            }
        }
    }

    None
}

/// Parsed inner content of `{{video URL [key=value …]}}`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct BracedVideoContent {
    url: String,
    width: Option<u32>,
    height: Option<u32>,
}

fn parse_video_dimension_value(raw: &str) -> Option<u32> {
    let parsed = raw.parse::<u32>().ok()?;
    if parsed == 0 {
        return None;
    }
    Some(parsed.clamp(MIN_VIDEO_DIMENSION, MAX_VIDEO_DIMENSION))
}

/// Extract URL and optional `width`/`height` params from braced video syntax.
fn parse_braced_video_content(text: &str) -> Option<BracedVideoContent> {
    let trimmed = text.trim();
    if !trimmed.starts_with("{{video") || !trimmed.ends_with("}}") {
        return None;
    }
    let inner = trimmed
        .strip_prefix("{{video")?
        .strip_suffix("}}")?
        .trim();
    if inner.is_empty() {
        return None;
    }

    let mut parts = inner.split_whitespace();
    let url = parts.next()?.to_string();
    let mut width = None;
    let mut height = None;

    for part in parts {
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };
        match key.to_ascii_lowercase().as_str() {
            "width" => width = parse_video_dimension_value(value),
            "height" => height = parse_video_dimension_value(value),
            _ => {}
        }
    }

    Some(BracedVideoContent {
        url,
        width,
        height,
    })
}

/// Reconstruct paragraph inline text without normalizing line breaks to spaces.
fn paragraph_source_text(node: &MarkdownNode) -> String {
    let mut output = String::new();
    for child in &node.children {
        match &child.node_type {
            MarkdownNodeType::Text(text) => output.push_str(text),
            MarkdownNodeType::Link { url, .. } => output.push_str(url),
            MarkdownNodeType::SoftBreak => output.push(' '),
            MarkdownNodeType::LineBreak => output.push('\n'),
            _ => output.push_str(&child.text_content()),
        }
    }
    output
}

/// Build `{{video URL [width=N] [height=N]}}` source syntax.
pub fn format_video_embed_source(url: &str, width: Option<u32>, height: Option<u32>) -> String {
    let mut inner = url.to_string();
    if let Some(w) = width {
        inner.push_str(&format!(" width={w}"));
    }
    if let Some(h) = height {
        inner.push_str(&format!(" height={h}"));
    }
    format!("{{{{video {inner}}}}}")
}

/// Replace a video embed line in the markdown source with new explicit dimensions.
pub fn rewrite_video_embed_dimensions(
    source: &mut String,
    line: usize,
    info: &VideoEmbedInfo,
    width: u32,
    height: u32,
) -> bool {
    if line == 0 {
        return false;
    }
    let width = width.clamp(MIN_VIDEO_DIMENSION, MAX_VIDEO_DIMENSION);
    let height = height.clamp(MIN_VIDEO_DIMENSION, MAX_VIDEO_DIMENSION);
    let new_line = format_video_embed_source(&info.url, Some(width), Some(height));

    let mut lines: Vec<String> = source.lines().map(str::to_string).collect();
    if line > lines.len() {
        return false;
    }
    let had_trailing_nl = source.ends_with('\n');
    lines[line - 1] = new_line;
    let mut rebuilt = lines.join("\n");
    if had_trailing_nl && !rebuilt.is_empty() {
        rebuilt.push('\n');
    }
    *source = rebuilt;
    true
}

/// If `node` is a video embed paragraph, return parsed embed metadata.
pub fn try_parse_video_paragraph(node: &MarkdownNode) -> Option<VideoEmbedInfo> {
    if !matches!(node.node_type, MarkdownNodeType::Paragraph) {
        return None;
    }

    let source_text = paragraph_source_text(node);

    let content = parse_braced_video_content(&source_text)?;
    parse_video_embed_url_with_source(&content.url, source_text, content.width, content.height)
}

/// Walk the AST and replace video embed paragraphs with `VideoEmbed` nodes.
pub fn extract_video_embeds(node: &mut MarkdownNode) {
    for child in &mut node.children {
        extract_video_embeds(child);
    }

    let old_children = std::mem::take(&mut node.children);
    let mut new_children = Vec::with_capacity(old_children.len());

    for child in old_children {
        if let Some(info) = try_parse_video_paragraph(&child) {
            new_children.push(MarkdownNode {
                node_type: MarkdownNodeType::VideoEmbed(info),
                children: Vec::new(),
                start_line: child.start_line,
                end_line: child.end_line,
            });
        } else {
            new_children.push(child);
        }
    }

    node.children = new_children;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::parser::{parse_markdown, MarkdownNodeType};

    #[test]
    fn parse_braced_youtube_watch_url() {
        let info = parse_video_embed_url("https://youtube.com/watch?v=abc123XYZ_-").unwrap();
        assert_eq!(info.provider, VideoProvider::YouTube);
        assert_eq!(info.video_id.as_deref(), Some("abc123XYZ_-"));
        assert!(info.trusted);
    }

    #[test]
    fn parse_braced_youtu_be_url() {
        let info = parse_video_embed_url("https://youtu.be/xyz789").unwrap();
        assert_eq!(info.provider, VideoProvider::YouTube);
        assert_eq!(info.video_id.as_deref(), Some("xyz789"));
        assert!(info.trusted);
    }

    #[test]
    fn parse_non_youtube_url_is_untrusted() {
        let info = parse_video_embed_url("https://vimeo.com/123456").unwrap();
        assert_eq!(info.provider, VideoProvider::Unknown);
        assert!(info.video_id.is_none());
        assert!(!info.trusted);
    }

    #[test]
    fn parse_rejects_non_http_scheme() {
        assert!(parse_video_embed_url("javascript:alert(1)").is_none());
    }

    #[test]
    fn document_parses_braced_video_syntax() {
        let doc = parse_markdown("{{video https://youtube.com/watch?v=abc}}").unwrap();
        let node = &doc.root.children[0];
        assert!(matches!(node.node_type, MarkdownNodeType::VideoEmbed(_)));
        if let MarkdownNodeType::VideoEmbed(info) = &node.node_type {
            assert_eq!(info.provider, VideoProvider::YouTube);
            assert_eq!(info.video_id.as_deref(), Some("abc"));
            assert_eq!(
                info.source_text,
                "{{video https://youtube.com/watch?v=abc}}"
            );
        } else {
            panic!("expected VideoEmbed node");
        }
    }

    #[test]
    fn document_bare_youtube_url_stays_paragraph() {
        let doc = parse_markdown("https://youtu.be/xyz").unwrap();
        let node = &doc.root.children[0];
        assert!(matches!(node.node_type, MarkdownNodeType::Paragraph));
        assert!(
            !matches!(node.node_type, MarkdownNodeType::VideoEmbed(_)),
            "bare YouTube URL must not auto-embed"
        );
        let has_link = node
            .children
            .iter()
            .any(|child| matches!(child.node_type, MarkdownNodeType::Link { .. }));
        assert!(has_link, "bare URL should be a clickable link");
    }

    #[test]
    fn bare_non_youtube_url_stays_paragraph() {
        let doc = parse_markdown("https://vimeo.com/123456").unwrap();
        assert!(!matches!(
            doc.root.children[0].node_type,
            MarkdownNodeType::VideoEmbed(_)
        ));
    }

    #[test]
    fn braced_non_youtube_url_becomes_untrusted_embed() {
        let doc = parse_markdown("{{video https://vimeo.com/123456}}").unwrap();
        let node = &doc.root.children[0];
        if let MarkdownNodeType::VideoEmbed(info) = &node.node_type {
            assert_eq!(info.provider, VideoProvider::Unknown);
            assert!(!info.trusted);
        } else {
            panic!("expected VideoEmbed node");
        }
    }

    #[test]
    fn video_embed_in_paragraph_with_other_text_stays_paragraph() {
        let doc = parse_markdown("Watch this: https://youtu.be/xyz").unwrap();
        assert!(matches!(
            doc.root.children[0].node_type,
            MarkdownNodeType::Paragraph
        ));
    }

    #[test]
    fn braced_video_width_param_parsed() {
        let source = "{{video https://youtube.com/watch?v=abc width=640}}";
        let doc = parse_markdown(source).unwrap();
        if let MarkdownNodeType::VideoEmbed(info) = &doc.root.children[0].node_type {
            assert_eq!(info.width, Some(640));
            assert_eq!(info.height, None);
            assert_eq!(info.source_text, source);
        } else {
            panic!("expected VideoEmbed node");
        }
    }

    #[test]
    fn braced_video_width_and_height_params_parsed() {
        let source = "{{video https://youtube.com/watch?v=abc width=640 height=360}}";
        let doc = parse_markdown(source).unwrap();
        if let MarkdownNodeType::VideoEmbed(info) = &doc.root.children[0].node_type {
            assert_eq!(info.width, Some(640));
            assert_eq!(info.height, Some(360));
            assert_eq!(info.source_text, source);
        } else {
            panic!("expected VideoEmbed node");
        }
    }

    #[test]
    fn braced_video_width_only_keeps_16_9_height_at_render() {
        let content = parse_braced_video_content(
            "{{video https://youtube.com/watch?v=abc width=400}}",
        )
        .unwrap();
        assert_eq!(content.width, Some(400));
        assert_eq!(content.height, None);
    }

    #[test]
    fn braced_video_dimension_clamped_to_max() {
        let content =
            parse_braced_video_content("{{video https://youtu.be/xyz width=99999}}").unwrap();
        assert_eq!(content.width, Some(MAX_VIDEO_DIMENSION));
    }

    #[test]
    fn braced_video_invalid_dimension_ignored() {
        let content = parse_braced_video_content(
            "{{video https://youtu.be/xyz width=abc height=0}}",
        )
        .unwrap();
        assert_eq!(content.width, None);
        assert_eq!(content.height, None);
        assert_eq!(content.url, "https://youtu.be/xyz");
    }

    #[test]
    fn format_video_embed_source_width_height() {
        assert_eq!(
            format_video_embed_source("https://youtu.be/abc", Some(640), Some(360)),
            "{{video https://youtu.be/abc width=640 height=360}}"
        );
    }

    #[test]
    fn format_video_embed_source_url_only() {
        assert_eq!(
            format_video_embed_source("https://youtu.be/abc", None, None),
            "{{video https://youtu.be/abc}}"
        );
    }

    #[test]
    fn rewrite_video_embed_dimensions_updates_source_line() {
        let info = parse_video_embed_url("https://youtube.com/watch?v=abc").unwrap();
        let mut source = "{{video https://youtube.com/watch?v=abc}}\n\nNext line".to_string();
        assert!(rewrite_video_embed_dimensions(&mut source, 1, &info, 800, 450));
        assert_eq!(
            source,
            "{{video https://youtube.com/watch?v=abc width=800 height=450}}\n\nNext line"
        );
    }

    #[test]
    fn braced_video_unknown_params_ignored() {
        let source = "{{video https://youtu.be/xyz autoplay=1 width=320 foo=bar}}";
        let content = parse_braced_video_content(source).unwrap();
        assert_eq!(content.width, Some(320));
        assert_eq!(content.height, None);
        let doc = parse_markdown(source).unwrap();
        if let MarkdownNodeType::VideoEmbed(info) = &doc.root.children[0].node_type {
            assert_eq!(info.source_text, source);
        } else {
            panic!("expected VideoEmbed node");
        }
    }
}
