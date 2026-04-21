//! YAML frontmatter parsing for Mermaid diagrams.
//!
//! Mermaid supports YAML frontmatter between `---` markers at the start of diagrams.
//! This allows specifying diagram titles and configuration options.
//!
//! # Example
//!
//! ```text
//! ---
//! title: My Flowchart
//! config:
//!   theme: dark
//! ---
//! flowchart TD
//!     A --> B --> C
//! ```

use serde::Deserialize;

/// Parsed YAML frontmatter from a Mermaid diagram.
#[derive(Debug, Clone, Default)]
pub struct MermaidFrontmatter {
    /// Diagram title to display above the diagram
    pub title: Option<String>,
    /// Configuration options
    pub config: Option<MermaidConfig>,
}

/// Configuration options from frontmatter.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct MermaidConfig {
    /// Theme name: "default", "dark", "forest", "neutral", etc.
    pub theme: Option<String>,
    // Future: Add more config options as needed
}

/// Intermediate struct for serde deserialization.
#[derive(Deserialize)]
struct FrontmatterYaml {
    title: Option<String>,
    config: Option<MermaidConfig>,
}

/// Extract and parse YAML frontmatter from mermaid source.
///
/// Returns the parsed frontmatter (if any) and the remaining diagram source.
/// If no frontmatter is present or parsing fails, returns (None, original_source).
///
/// # Arguments
///
/// * `source` - The complete Mermaid diagram source
///
/// # Returns
///
/// A tuple of (Option<MermaidFrontmatter>, remaining_source)
///
/// # Example
///
/// ```ignore
/// let source = "---\ntitle: Test\n---\nflowchart TD\n  A --> B";
/// let (frontmatter, diagram_source) = parse_frontmatter(source);
/// assert_eq!(frontmatter.as_ref().unwrap().title, Some("Test".to_string()));
/// assert!(diagram_source.starts_with("flowchart"));
/// ```
pub fn parse_frontmatter(source: &str) -> (Option<MermaidFrontmatter>, &str) {
    let source = source.trim();

    // Frontmatter must start with ---
    if !source.starts_with("---") {
        return (None, source);
    }

    // Find the end of the opening --- line
    let after_dashes = &source[3..];

    // Skip any spaces/tabs after --- and find the newline
    let after_opening = after_dashes.trim_start_matches([' ', '\t']);

    // Must have a newline after opening ---
    let (newline_len, after_newline) = if after_opening.starts_with("\r\n") {
        (2, &after_opening[2..])
    } else if after_opening.starts_with('\n') {
        (1, &after_opening[1..])
    } else {
        // No newline after opening ---, invalid frontmatter
        return (None, source);
    };
    let _ = newline_len; // silence unused warning

    // Find the closing --- by scanning line by line
    let mut yaml_end = None;
    let mut pos = 0;

    for line in after_newline.lines() {
        if line.trim() == "---" {
            yaml_end = Some(pos);
            break;
        }
        // Account for the line content + newline
        // lines() strips line endings, so we need to figure out what was there
        pos += line.len();
        // Check what kind of newline follows this line in the original
        let remaining = &after_newline[pos..];
        if remaining.starts_with("\r\n") {
            pos += 2;
        } else if remaining.starts_with('\n') {
            pos += 1;
        }
        // If no newline, we're at the end of the string
    }

    let yaml_end = match yaml_end {
        Some(idx) => idx,
        None => return (None, source),
    };

    // Extract YAML content (everything before the closing ---)
    let yaml_content = &after_newline[..yaml_end];

    // Find where the diagram starts (after the closing --- and newline)
    let after_closing = &after_newline[yaml_end..];
    // Skip the --- and any trailing spaces
    let after_closing = after_closing.strip_prefix("---").unwrap_or(after_closing);
    let after_closing = after_closing.trim_start_matches([' ', '\t']);

    // Skip the newline after closing ---
    let diagram_source = if after_closing.starts_with("\r\n") {
        &after_closing[2..]
    } else if after_closing.starts_with('\n') {
        &after_closing[1..]
    } else {
        after_closing
    };

    // Parse YAML
    match serde_yaml::from_str::<FrontmatterYaml>(yaml_content) {
        Ok(parsed) => {
            let frontmatter = MermaidFrontmatter {
                title: parsed.title,
                config: parsed.config,
            };
            (Some(frontmatter), diagram_source)
        }
        Err(_e) => {
            // YAML parse error - return original source without frontmatter
            // This allows the diagram to still render
            #[cfg(debug_assertions)]
            log::debug!("Failed to parse Mermaid frontmatter YAML: {}", _e);
            (None, source)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_frontmatter() {
        let source = "flowchart TD\n  A --> B";
        let (fm, remaining) = parse_frontmatter(source);
        assert!(fm.is_none());
        assert_eq!(remaining, source);
    }

    #[test]
    fn test_simple_frontmatter_with_title() {
        let source = "---\ntitle: My Chart\n---\nflowchart TD\n  A --> B";
        let (fm, remaining) = parse_frontmatter(source);

        assert!(fm.is_some());
        let fm = fm.unwrap();
        assert_eq!(fm.title, Some("My Chart".to_string()));
        assert!(remaining.starts_with("flowchart"));
    }

    #[test]
    fn test_frontmatter_with_config() {
        let source =
            "---\ntitle: Dark Theme Chart\nconfig:\n  theme: dark\n---\nflowchart LR\n  A --> B";
        let (fm, remaining) = parse_frontmatter(source);

        assert!(fm.is_some());
        let fm = fm.unwrap();
        assert_eq!(fm.title, Some("Dark Theme Chart".to_string()));
        assert!(fm.config.is_some());
        assert_eq!(fm.config.unwrap().theme, Some("dark".to_string()));
        assert!(remaining.starts_with("flowchart"));
    }

    #[test]
    fn test_frontmatter_title_only() {
        let source = "---\ntitle: Simple Title\n---\npie\n  \"A\" : 30\n  \"B\" : 70";
        let (fm, remaining) = parse_frontmatter(source);

        assert!(fm.is_some());
        assert_eq!(fm.unwrap().title, Some("Simple Title".to_string()));
        assert!(remaining.starts_with("pie"));
    }

    #[test]
    fn test_frontmatter_config_only() {
        let source = "---\nconfig:\n  theme: forest\n---\nsequenceDiagram\n  A->>B: Hello";
        let (fm, remaining) = parse_frontmatter(source);

        assert!(fm.is_some());
        let fm = fm.unwrap();
        assert!(fm.title.is_none());
        assert!(fm.config.is_some());
        assert_eq!(fm.config.unwrap().theme, Some("forest".to_string()));
    }

    #[test]
    fn test_invalid_yaml_frontmatter() {
        // Invalid YAML should return original source
        let source = "---\n  invalid: yaml: content:\n---\nflowchart TD\n  A --> B";
        let (fm, remaining) = parse_frontmatter(source);

        // Should fail gracefully
        assert!(fm.is_none());
        assert_eq!(remaining, source);
    }

    #[test]
    fn test_no_closing_delimiter() {
        let source = "---\ntitle: Unclosed\nflowchart TD\n  A --> B";
        let (fm, remaining) = parse_frontmatter(source);

        // No closing --- means no frontmatter
        assert!(fm.is_none());
        assert_eq!(remaining, source);
    }

    #[test]
    fn test_empty_frontmatter() {
        // Empty YAML between delimiters
        let source = "---\n---\nflowchart TD\n  A --> B";
        let (fm, remaining) = parse_frontmatter(source);

        assert!(fm.is_some());
        let fm = fm.unwrap();
        assert!(fm.title.is_none());
        assert!(fm.config.is_none());
        assert!(remaining.starts_with("flowchart"));
    }

    #[test]
    fn test_frontmatter_with_unknown_keys() {
        // Unknown keys should be silently ignored
        let source = "---\ntitle: Test\nunknown_key: some_value\nanother_unknown:\n  nested: value\n---\nflowchart TD\n  A --> B";
        let (fm, remaining) = parse_frontmatter(source);

        assert!(fm.is_some());
        assert_eq!(fm.unwrap().title, Some("Test".to_string()));
        assert!(remaining.starts_with("flowchart"));
    }

    #[test]
    fn test_frontmatter_with_quoted_title() {
        let source = "---\ntitle: \"Quoted: Title\"\n---\nflowchart TD\n  A --> B";
        let (fm, remaining) = parse_frontmatter(source);

        assert!(fm.is_some());
        assert_eq!(fm.unwrap().title, Some("Quoted: Title".to_string()));
        assert!(remaining.starts_with("flowchart"));
    }

    #[test]
    fn test_frontmatter_windows_line_endings() {
        let source = "---\r\ntitle: Windows\r\n---\r\nflowchart TD\r\n  A --> B";
        let (fm, remaining) = parse_frontmatter(source);

        assert!(fm.is_some());
        assert_eq!(fm.unwrap().title, Some("Windows".to_string()));
        // The remaining should contain flowchart (may have \r\n at start)
        assert!(remaining.trim().starts_with("flowchart"));
    }
}
