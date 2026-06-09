//! Video embed parsing for markdown documents.
//!
//! Recognizes `{{video URL}}` syntax and bare YouTube URLs in standalone paragraphs.
//! Trusted domains (YouTube / youtu.be) may use interactive WebView overlays; others
//! fall back to thumbnail-only rendering (handled in a later task).

use super::parser::{
    MarkdownNode, MarkdownNodeType, VideoEmbedInfo, VideoProvider,
};

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
    parse_video_embed_url_with_source(raw_url, raw_url.trim().to_string())
}

fn parse_video_embed_url_with_source(raw_url: &str, source_text: String) -> Option<VideoEmbedInfo> {
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
                source_text,
            });
        }
    }

    Some(VideoEmbedInfo {
        provider: VideoProvider::Unknown,
        video_id: None,
        url: url_string,
        trusted,
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

/// Extract `{{video URL}}` URL from braced syntax, if present.
fn parse_braced_video_syntax(text: &str) -> Option<&str> {
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
    Some(inner)
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

/// If `node` is a video embed paragraph, return parsed embed metadata.
pub fn try_parse_video_paragraph(node: &MarkdownNode) -> Option<VideoEmbedInfo> {
    if !matches!(node.node_type, MarkdownNodeType::Paragraph) {
        return None;
    }

    let source_text = paragraph_source_text(node);

    if let Some(url_str) = parse_braced_video_syntax(&source_text).map(str::to_string) {
        return parse_video_embed_url_with_source(&url_str, source_text);
    }

    let bare_url = extract_bare_youtube_url(node)?;
    let info = parse_video_embed_url_with_source(&bare_url, source_text)?;
    if info.provider == VideoProvider::YouTube && info.video_id.is_some() {
        Some(info)
    } else {
        None
    }
}

fn extract_bare_youtube_url(node: &MarkdownNode) -> Option<String> {
    match node.children.len() {
        0 => None,
        1 => match &node.children[0].node_type {
            MarkdownNodeType::Text(text) => {
                let trimmed = text.trim();
                if trimmed.is_empty() || trimmed.contains('\n') {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            }
            MarkdownNodeType::Link { url, .. } => Some(url.trim().to_string()),
            _ => None,
        },
        _ => {
            if node
                .children
                .iter()
                .any(|child| !matches!(child.node_type, MarkdownNodeType::Text(_)))
            {
                return None;
            }
            let combined = node.text_content();
            let trimmed = combined.trim();
            if trimmed.is_empty() || trimmed.contains('\n') {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
    }
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
    fn document_parses_bare_youtube_url_paragraph() {
        let doc = parse_markdown("https://youtu.be/xyz").unwrap();
        let node = &doc.root.children[0];
        assert!(matches!(node.node_type, MarkdownNodeType::VideoEmbed(_)));
        if let MarkdownNodeType::VideoEmbed(info) = &node.node_type {
            assert_eq!(info.video_id.as_deref(), Some("xyz"));
            assert_eq!(info.source_text, "https://youtu.be/xyz");
        } else {
            panic!("expected VideoEmbed node");
        }
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
}
