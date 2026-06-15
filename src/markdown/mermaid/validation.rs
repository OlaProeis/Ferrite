//! Parse-time validation and structured error reporting for Mermaid diagrams.
//!
//! Wraps the existing per-diagram parsers with a uniform [`MermaidError`] that
//! carries a 1-indexed line number (best-effort), the human-readable message
//! and an optional [`hint`] for common mistakes.
//!
//! Two entry points:
//! - [`validate_mermaid_source`] — parse-only, used for editor squiggles and
//!   the warning header in the rendered widget.
//! - [`compute_mermaid_diagnostics`] — walks a markdown document, validates
//!   every fenced ```mermaid``` block, and returns LSP-style
//!   [`DiagnosticEntry`] values pointing at the offending source lines.

use crate::lsp::state::{DiagnosticEntry, DiagnosticSeverity};

use super::frontmatter::parse_frontmatter;
use super::{
    class_diagram::parse_class_diagram, er_diagram::parse_er_diagram, flowchart::parse_flowchart,
    gantt::parse_gantt_chart, git_graph::parse_git_graph, journey::parse_user_journey,
    mindmap::parse_mindmap, pie::parse_pie_chart, sequence::parse_sequence_diagram,
    state::parse_state_diagram, timeline::parse_timeline,
};

// ─────────────────────────────────────────────────────────────────────────────
// MermaidError
// ─────────────────────────────────────────────────────────────────────────────

/// A structured Mermaid validation error.
///
/// `line` is 1-indexed within the diagram source (i.e. line 1 is the diagram
/// header). When the underlying parser does not report a line number, the
/// header line (1) is used as a reasonable fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MermaidError {
    /// Human-readable error message (without any "Line N:" prefix).
    pub message: String,
    /// 1-indexed line within the diagram source.
    pub line: usize,
    /// Optional friendly hint about how to fix the issue.
    pub hint: Option<String>,
}

impl MermaidError {
    /// Build a `MermaidError` from a raw parser message string. Extracts the
    /// `Line N:` prefix when present and adds a hint for common mistakes.
    pub fn from_message(source: &str, message: impl Into<String>) -> Self {
        let raw: String = message.into();
        let line = extract_line_from_message(&raw).unwrap_or(1);
        let stripped = strip_line_prefix(&raw).unwrap_or(raw);
        let hint = derive_hint(source, &stripped);
        Self {
            message: stripped,
            line,
            hint,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Public validation API
// ─────────────────────────────────────────────────────────────────────────────

/// Validate a Mermaid diagram source. Returns `Ok(())` if the source parses
/// cleanly, otherwise a structured [`MermaidError`].
///
/// This routine performs parsing only — it never touches the renderer, so it
/// is safe to call from non-UI code (for example, the central panel's editor
/// diagnostics path).
pub fn validate_mermaid_source(source: &str) -> Result<(), MermaidError> {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Err(MermaidError::from_message(source, "Empty diagram source"));
    }

    // Strip optional YAML frontmatter (`---` … `---`).
    let (_, body) = parse_frontmatter(trimmed);

    let first_line = body
        .lines()
        .map(|l| l.trim())
        .find(|l| !l.is_empty() && !l.starts_with("%%"))
        .unwrap_or("")
        .to_lowercase();

    let result: Result<(), String> =
        if first_line.starts_with("flowchart") || first_line.starts_with("graph") {
            parse_flowchart(body).and_then(|flowchart| {
                if let Some(w) = flowchart.warnings.first() {
                    Err(format!("Line {}: {}", w.line, w.message))
                } else {
                    Ok(())
                }
            })
        } else if first_line.starts_with("sequencediagram") {
            parse_sequence_diagram(body).map(|_| ())
        } else if first_line.starts_with("pie") {
            parse_pie_chart(body).map(|_| ())
        } else if first_line.starts_with("statediagram") {
            parse_state_diagram(body).map(|_| ())
        } else if first_line.starts_with("mindmap") {
            parse_mindmap(body).map(|_| ())
        } else if first_line.starts_with("classdiagram") {
            parse_class_diagram(body).map(|_| ())
        } else if first_line.starts_with("erdiagram") {
            parse_er_diagram(body).map(|_| ())
        } else if first_line.starts_with("gantt") {
            parse_gantt_chart(body).map(|_| ())
        } else if first_line.starts_with("gitgraph") {
            parse_git_graph(body).and_then(|graph| {
                if let Some(w) = graph.warnings.first() {
                    Err(format!("Line {}: {}", w.line, w.message))
                } else {
                    Ok(())
                }
            })
        } else if first_line.starts_with("timeline") {
            parse_timeline(body).map(|_| ())
        } else if first_line.starts_with("journey") {
            parse_user_journey(body).map(|_| ())
        } else if first_line.is_empty() {
            Err("Missing diagram header".to_string())
        } else {
            Err(format!("Unknown diagram type: {}", first_line))
        };

    result.map_err(|msg| MermaidError::from_message(source, msg))
}

// ─────────────────────────────────────────────────────────────────────────────
// Diagnostic computation
// ─────────────────────────────────────────────────────────────────────────────

/// Walk the markdown document, validate every ```mermaid``` fenced block and
/// emit a [`DiagnosticEntry`] for each block that fails to parse.
///
/// `DiagnosticEntry` lines are 0-indexed (matching the LSP/editor convention).
pub fn compute_mermaid_diagnostics(content: &str) -> Vec<DiagnosticEntry> {
    let Ok(doc) = crate::markdown::cache::get_or_parse(content) else {
        return Vec::new();
    };

    let mut diagnostics = Vec::new();
    collect_mermaid_diagnostics(&doc.root, &mut diagnostics);
    diagnostics
}

fn collect_mermaid_diagnostics(
    node: &crate::markdown::parser::MarkdownNode,
    out: &mut Vec<DiagnosticEntry>,
) {
    use crate::markdown::parser::MarkdownNodeType;

    if let MarkdownNodeType::CodeBlock {
        language, literal, ..
    } = &node.node_type
    {
        if language.eq_ignore_ascii_case("mermaid") {
            if let Err(err) = validate_mermaid_source(literal) {
                // Map the parser's 1-indexed line within the diagram body to
                // a 0-indexed editor line. The fence opener `\`\`\`mermaid` is
                // node.start_line (1-indexed); the body's first line is at
                // node.start_line + 1 (1-indexed) → start_line (0-indexed).
                let body_first_line_0 = node.start_line; // node.start_line is 1-indexed
                let err_line_0 = body_first_line_0 + err.line.saturating_sub(1);

                // Clamp to the closing fence (one line before end_line).
                let body_last_line_0 = node.end_line.saturating_sub(2);
                let line = err_line_0.min(body_last_line_0.max(body_first_line_0));

                let mut message = format!("Mermaid: {}", err.message);
                if let Some(hint) = err.hint {
                    message.push_str(" — ");
                    message.push_str(&hint);
                }

                out.push(DiagnosticEntry {
                    start_line: line,
                    start_col: 0,
                    end_line: line,
                    end_col: usize::MAX, // renderer clamps to line length
                    severity: DiagnosticSeverity::Warning,
                    message,
                    source: Some("mermaid".to_string()),
                });
            }
        }
    }

    for child in &node.children {
        collect_mermaid_diagnostics(child, out);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Extract the first `Line N` mention anywhere in the message.
fn extract_line_from_message(msg: &str) -> Option<usize> {
    let lower = msg.to_lowercase();
    let idx = lower.find("line ")?;
    let after = &msg[idx + 5..];
    let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

/// Strip a leading `Line N: ` prefix when present.
fn strip_line_prefix(msg: &str) -> Option<String> {
    let rest = msg.strip_prefix("Line ")?;
    let pos = rest.find(": ")?;
    let n_str = &rest[..pos];
    if n_str.chars().all(|c| c.is_ascii_digit()) {
        Some(rest[pos + 2..].to_string())
    } else {
        None
    }
}

/// Derive a friendly hint for common Mermaid authoring mistakes.
fn derive_hint(source: &str, message: &str) -> Option<String> {
    let lower = message.to_lowercase();

    if lower.contains("expected 'flowchart'")
        || lower.contains("empty flowchart")
        || lower.contains("missing diagram header")
    {
        return Some(
            "Add a diagram header like `flowchart TD` or `graph LR` on the first line.".to_string(),
        );
    }

    if lower.contains("unknown diagram type") {
        return Some(
            "Supported types: flowchart, sequenceDiagram, classDiagram, stateDiagram, \
             erDiagram, gantt, pie, mindmap, timeline, journey, gitGraph."
                .to_string(),
        );
    }

    if lower.contains("@pos") {
        return Some(
            "Use `%% @pos <node_id> <x> <y>` with an existing node id and numeric coordinates."
                .to_string(),
        );
    }

    if lower.contains("'else' without matching 'alt'")
        || lower.contains("else can only be used inside")
    {
        return Some("Wrap the `else` branch in `alt … else … end`.".to_string());
    }

    if lower.contains("'and' without matching 'par'")
        || lower.contains("and can only be used inside")
    {
        return Some("Use `and` only inside `par … and … end` blocks.".to_string());
    }

    if let Some(unbalanced) = find_unbalanced_bracket(source) {
        return Some(format!(
            "Unmatched `{}` — every opening bracket needs a matching closing bracket.",
            unbalanced
        ));
    }

    let opens = source
        .lines()
        .filter(|l| l.trim_start().to_lowercase().starts_with("subgraph"))
        .count();
    let ends = source
        .lines()
        .filter(|l| l.trim().eq_ignore_ascii_case("end"))
        .count();
    if opens > ends {
        return Some(format!(
            "Missing `end` — {} subgraph(s) opened but only {} closed.",
            opens, ends
        ));
    }

    None
}

/// Returns the bracket character that is unbalanced, if any. Counts each
/// pair (`[]`, `()`, `{}`) independently so that one type's imbalance does
/// not mask another.
fn find_unbalanced_bracket(source: &str) -> Option<char> {
    let pairs = [('[', ']'), ('(', ')'), ('{', '}')];
    for (open, close) in pairs {
        let opens = source.chars().filter(|c| *c == open).count();
        let closes = source.chars().filter(|c| *c == close).count();
        if opens != closes {
            return Some(if opens > closes { open } else { close });
        }
    }
    None
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_source_is_invalid() {
        let err = validate_mermaid_source("").unwrap_err();
        assert!(err.message.to_lowercase().contains("empty"));
        assert_eq!(err.line, 1);
    }

    #[test]
    fn unknown_diagram_is_invalid_with_hint() {
        let err = validate_mermaid_source("doodleDiagram\n  A").unwrap_err();
        assert!(err.message.to_lowercase().contains("unknown diagram type"));
        assert!(err
            .hint
            .as_deref()
            .unwrap_or("")
            .contains("Supported types"));
    }

    #[test]
    fn valid_flowchart_passes() {
        assert!(validate_mermaid_source("flowchart TD\n  A --> B").is_ok());
    }

    #[test]
    fn flowchart_missing_header_is_invalid() {
        // First non-empty line is just nodes — should fail.
        let err = validate_mermaid_source("A --> B\nC --> D").unwrap_err();
        assert!(err.message.to_lowercase().contains("unknown diagram type"));
    }

    #[test]
    fn unmatched_bracket_produces_hint() {
        let src = "flowchart TD\n  A[Start --> B[End]";
        let _err_or_ok = validate_mermaid_source(src);
        // The bracket-balance heuristic also runs against the source, so it
        // should at least surface a `[` hint when consulted directly.
        let hint = derive_hint(src, "Some unrelated parse error");
        assert!(
            hint.unwrap_or_default().contains('['),
            "expected unbalanced [ hint"
        );
    }

    #[test]
    fn unclosed_subgraph_produces_hint() {
        let src = "flowchart TD\nsubgraph Foo\n  A --> B";
        let hint = derive_hint(src, "some failure");
        let hint = hint.unwrap_or_default();
        assert!(hint.contains("subgraph") || hint.contains("end"));
    }

    #[test]
    fn extracts_line_from_sequence_error() {
        // Sequence parser produces "Line 4: 'else' without matching 'alt' block"
        let src = "sequenceDiagram\n  participant A\n  participant B\n  else what now";
        let err = validate_mermaid_source(src).unwrap_err();
        assert_eq!(err.line, 4);
        assert!(!err.message.starts_with("Line "));
        assert!(err
            .hint
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .contains("alt"));
    }

    #[test]
    fn message_strips_line_prefix() {
        let err = MermaidError::from_message("flowchart TD", "Line 7: some explanation");
        assert_eq!(err.line, 7);
        assert_eq!(err.message, "some explanation");
    }

    #[test]
    fn diagnostics_emitted_for_invalid_mermaid_block() {
        let md = "# Header\n\n```mermaid\ndoodleDiagram\n  A\n```\n\nText.";
        let diags = compute_mermaid_diagnostics(md);
        assert_eq!(diags.len(), 1, "expected exactly one mermaid diagnostic");
        let d = &diags[0];
        assert_eq!(d.severity, DiagnosticSeverity::Warning);
        assert_eq!(d.source.as_deref(), Some("mermaid"));
        assert!(d.message.to_lowercase().contains("mermaid"));
        // Diagnostic line should fall on `doodleDiagram` (line 4 in 1-indexed,
        // line 3 in 0-indexed).
        assert_eq!(d.start_line, 3);
    }

    #[test]
    fn diagnostics_skipped_for_valid_mermaid_block() {
        let md = "```mermaid\nflowchart TD\n  A --> B\n```";
        let diags = compute_mermaid_diagnostics(md);
        assert!(diags.is_empty());
    }

    #[test]
    fn gitgraph_unknown_statement_surfaces_validation_warning() {
        let src = "gitGraph\n  commit\n  reset HEAD~1\n  commit";
        let err = validate_mermaid_source(src).unwrap_err();
        assert_eq!(err.line, 3);
        assert!(err.message.contains("Unknown gitGraph statement"));
    }

    #[test]
    fn gitgraph_cherry_pick_unknown_id_surfaces_validation_warning() {
        let src = "gitGraph\n  commit\n  cherry-pick id: \"missing\"";
        let err = validate_mermaid_source(src).unwrap_err();
        assert_eq!(err.line, 3);
        assert!(err.message.contains("unknown commit id"));
    }

    #[test]
    fn valid_gitgraph_passes() {
        assert!(validate_mermaid_source("gitGraph\n  commit id: \"a1\"").is_ok());
    }

    #[test]
    fn flowchart_link_style_interpolate_basis_validates() {
        let source = r#"graph TD
    linkStyle default interpolate basis
    A --> B
"#;
        assert!(validate_mermaid_source(source).is_ok());
    }

    #[test]
    fn diagnostics_only_for_mermaid_language_tag() {
        let md = "```rust\nfn main() {}\n```\n\n```mermaid\nflowchart TD\n  A --> B\n```";
        let diags = compute_mermaid_diagnostics(md);
        assert!(diags.is_empty());
    }
}
