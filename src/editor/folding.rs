//! Fold detection algorithms for code folding.
//!
//! This module implements fold region detection for different content types:
//! - Markdown headings, code blocks, and lists
//! - Indentation-based folding for JSON/YAML

use crate::state::{FileType, FoldKind, FoldRegion, FoldState};

/// Detect all foldable regions in a document.
///
/// This function analyzes the document content and returns fold regions
/// based on the file type and enabled fold kinds.
pub fn detect_fold_regions(
    content: &str,
    file_type: FileType,
    fold_headings: bool,
    fold_code_blocks: bool,
    fold_lists: bool,
    fold_indentation: bool,
) -> FoldState {
    let mut fold_state = FoldState::new();
    let lines: Vec<&str> = content.lines().collect();

    if lines.is_empty() {
        fold_state.mark_clean();
        return fold_state;
    }

    match file_type {
        FileType::Markdown => {
            // Detect Markdown-specific folds
            if fold_headings {
                detect_markdown_headings(&lines, &mut fold_state);
            }
            if fold_code_blocks {
                detect_markdown_code_blocks(&lines, &mut fold_state);
            }
            if fold_lists {
                detect_markdown_lists(&lines, &mut fold_state);
            }
        }
        FileType::Json | FileType::Yaml | FileType::Toml => {
            // Use indentation-based folding for structured files
            if fold_indentation {
                detect_indentation_folds(&lines, &mut fold_state);
            }
        }
        FileType::Csv | FileType::Tsv | FileType::Image | FileType::Pdf => {
            // Tabular/image/PDF files don't support folding (flat structure)
        }
        FileType::Unknown => {
            // For unknown files, try indentation-based folding
            if fold_indentation {
                detect_indentation_folds(&lines, &mut fold_state);
            }
        }
    }

    fold_state.mark_clean();
    fold_state
}

// ─────────────────────────────────────────────────────────────────────────────
// Markdown Folding
// ─────────────────────────────────────────────────────────────────────────────

/// Detect markdown heading levels (ATX style: #, ##, ###, etc.)
fn get_heading_level(line: &str) -> Option<u8> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }

    let hash_count = trimmed.chars().take_while(|&c| c == '#').count();
    if hash_count > 6 {
        return None;
    }

    // Must have space after hashes or be just hashes (for some parsers)
    let after_hashes = &trimmed[hash_count..];
    if after_hashes.is_empty() || after_hashes.starts_with(' ') {
        return Some(hash_count as u8);
    }

    None
}

/// Detect markdown headings and their content as fold regions.
///
/// A heading fold region starts at the heading line and ends at:
/// - The line before the next heading of same or higher level
/// - Or the end of the document
fn detect_markdown_headings(lines: &[&str], fold_state: &mut FoldState) {
    let mut headings: Vec<(usize, u8, String)> = Vec::new();

    // First pass: find all headings
    for (line_num, line) in lines.iter().enumerate() {
        if let Some(level) = get_heading_level(line) {
            // Extract heading text for preview
            let text = line
                .trim_start_matches('#')
                .trim()
                .chars()
                .take(50)
                .collect::<String>();
            headings.push((line_num, level, text));
        }
    }

    // Second pass: determine fold regions for each heading
    for (i, (start_line, level, preview_text)) in headings.iter().enumerate() {
        // Find the end of this heading's content
        let end_line = if i + 1 < headings.len() {
            // Look for the next heading of same or higher (lower number) level
            let mut found_end = None;
            for j in (i + 1)..headings.len() {
                let (next_line, next_level, _) = &headings[j];
                if *next_level <= *level {
                    // Found a heading of same or higher level
                    found_end = Some(next_line.saturating_sub(1));
                    break;
                }
            }
            // If no same/higher level heading found, fold until next heading
            found_end.unwrap_or_else(|| {
                if i + 1 < headings.len() {
                    headings[i + 1].0.saturating_sub(1)
                } else {
                    lines.len().saturating_sub(1)
                }
            })
        } else {
            // Last heading - fold until end of document
            lines.len().saturating_sub(1)
        };

        // Only create fold if there's content to fold (more than just the heading line)
        if end_line > *start_line {
            // Skip trailing empty lines
            let mut actual_end = end_line;
            while actual_end > *start_line
                && lines
                    .get(actual_end)
                    .map(|l| l.trim().is_empty())
                    .unwrap_or(true)
            {
                actual_end -= 1;
            }

            if actual_end > *start_line {
                let region = FoldRegion::with_preview(
                    fold_state.next_id(),
                    *start_line,
                    actual_end,
                    FoldKind::Heading(*level),
                    preview_text.clone(),
                );
                fold_state.add_region(region);
            }
        }
    }
}

/// Check if a line is a fenced code block delimiter (``` or ~~~)
fn is_code_fence(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("```") {
        Some("```")
    } else if trimmed.starts_with("~~~") {
        Some("~~~")
    } else {
        None
    }
}

/// Detect fenced code blocks as fold regions.
fn detect_markdown_code_blocks(lines: &[&str], fold_state: &mut FoldState) {
    let mut in_fence = false;
    let mut fence_char: &str = "";
    let mut start_line = 0;
    let mut preview_text = String::new();

    for (line_num, line) in lines.iter().enumerate() {
        if let Some(fence) = is_code_fence(line) {
            if !in_fence {
                // Opening fence
                in_fence = true;
                fence_char = fence;
                start_line = line_num;
                // Extract language hint for preview
                let after_fence = line.trim_start().trim_start_matches(fence).trim();
                preview_text = if after_fence.is_empty() {
                    "code".to_string()
                } else {
                    after_fence.chars().take(20).collect()
                };
            } else if line.trim_start().starts_with(fence_char) {
                // Closing fence (must match opening fence type)
                in_fence = false;

                // Create fold region if there's content
                if line_num > start_line {
                    let region = FoldRegion::with_preview(
                        fold_state.next_id(),
                        start_line,
                        line_num,
                        FoldKind::CodeBlock,
                        format!("```{}", preview_text),
                    );
                    fold_state.add_region(region);
                }
            }
        }
    }
}

/// Get the indentation level of a list item (counting leading spaces/tabs).
fn get_list_indent(line: &str) -> Option<usize> {
    let trimmed = line.trim_start();

    // Check for unordered list markers: -, *, +
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
        return Some(line.len() - trimmed.len());
    }

    // Check for ordered list markers: 1. 2. etc.
    let mut chars = trimmed.chars().peekable();
    let mut has_digit = false;
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            has_digit = true;
            chars.next();
        } else {
            break;
        }
    }

    if has_digit {
        if let Some('.') = chars.next() {
            if let Some(' ') = chars.next() {
                return Some(line.len() - trimmed.len());
            }
        }
    }

    None
}

/// Detect nested list hierarchies as fold regions.
fn detect_markdown_lists(lines: &[&str], fold_state: &mut FoldState) {
    let mut list_starts: Vec<(usize, usize, String)> = Vec::new(); // (line, indent, preview)

    for (line_num, line) in lines.iter().enumerate() {
        if let Some(indent) = get_list_indent(line) {
            // This is a list item
            let preview = line.trim().chars().take(40).collect::<String>();

            // Check for nested content following this list item
            let mut end_line = line_num;
            let mut has_nested = false;

            for (future_idx, future_line) in lines.iter().enumerate().skip(line_num + 1) {
                let future_trimmed = future_line.trim();

                if future_trimmed.is_empty() {
                    // Empty line might be within list continuation
                    continue;
                }

                let future_indent = future_line.len() - future_trimmed.len();

                // Check if this is a nested list item or continuation
                if let Some(nested_indent) = get_list_indent(future_line) {
                    if nested_indent > indent {
                        // Nested list item
                        has_nested = true;
                        end_line = future_idx;
                    } else if nested_indent <= indent {
                        // Same or parent level list item - end here
                        break;
                    }
                } else if future_indent > indent {
                    // Continuation content (indented text)
                    has_nested = true;
                    end_line = future_idx;
                } else {
                    // Not part of this list item
                    break;
                }
            }

            // Only create fold if there's nested content
            if has_nested && end_line > line_num {
                let region = FoldRegion::with_preview(
                    fold_state.next_id(),
                    line_num,
                    end_line,
                    FoldKind::List,
                    preview.clone(),
                );
                list_starts.push((line_num, indent, preview));
                fold_state.add_region(region);
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Indentation-Based Folding (JSON/YAML/TOML)
// ─────────────────────────────────────────────────────────────────────────────

/// Get the indentation level of a line (number of leading spaces).
fn get_indentation(line: &str) -> usize {
    line.chars().take_while(|&c| c == ' ' || c == '\t').count()
}

/// Detect indentation-based fold regions.
///
/// Creates fold regions for lines that have more-indented content following them.
fn detect_indentation_folds(lines: &[&str], fold_state: &mut FoldState) {
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        // Skip empty lines
        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        let current_indent = get_indentation(line);

        // Look ahead to see if there's more-indented content
        let mut end_line = i;
        let mut found_nested = false;

        for j in (i + 1)..lines.len() {
            let future_line = lines[j];
            let future_trimmed = future_line.trim();

            // Skip empty lines but don't end the fold
            if future_trimmed.is_empty() {
                continue;
            }

            let future_indent = get_indentation(future_line);

            if future_indent > current_indent {
                // More indented - part of this fold
                found_nested = true;
                end_line = j;
            } else {
                // Same or less indented - end of fold
                break;
            }
        }

        // Create fold region if there's nested content
        if found_nested && end_line > i {
            let preview = trimmed.chars().take(50).collect::<String>();
            let region = FoldRegion::with_preview(
                fold_state.next_id(),
                i,
                end_line,
                FoldKind::Indentation,
                preview,
            );
            fold_state.add_region(region);
        }

        i += 1;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heading_level_detection() {
        assert_eq!(get_heading_level("# Heading 1"), Some(1));
        assert_eq!(get_heading_level("## Heading 2"), Some(2));
        assert_eq!(get_heading_level("### Heading 3"), Some(3));
        assert_eq!(get_heading_level("###### Heading 6"), Some(6));
        assert_eq!(get_heading_level("####### Too many"), None);
        assert_eq!(get_heading_level("Not a heading"), None);
        assert_eq!(get_heading_level("#NoSpace"), None);
        assert_eq!(get_heading_level("  ## Indented heading"), Some(2));
    }

    #[test]
    fn test_code_fence_detection() {
        assert_eq!(is_code_fence("```rust"), Some("```"));
        assert_eq!(is_code_fence("~~~python"), Some("~~~"));
        assert_eq!(is_code_fence("```"), Some("```"));
        assert_eq!(is_code_fence("  ```"), Some("```"));
        assert_eq!(is_code_fence("not a fence"), None);
    }

    #[test]
    fn test_list_indent_detection() {
        assert_eq!(get_list_indent("- Item"), Some(0));
        assert_eq!(get_list_indent("  - Nested"), Some(2));
        assert_eq!(get_list_indent("* Asterisk"), Some(0));
        assert_eq!(get_list_indent("1. Numbered"), Some(0));
        assert_eq!(get_list_indent("  10. Nested numbered"), Some(2));
        assert_eq!(get_list_indent("Not a list"), None);
    }

    #[test]
    fn test_markdown_heading_folds() {
        let content = "# Title\n\nSome content\n\n## Section\n\nMore content\n\n# Another";
        let fold_state =
            detect_fold_regions(content, FileType::Markdown, true, false, false, false);

        // Should have folds for "Title" section (lines 0-3) and "Section" (lines 4-6)
        assert!(!fold_state.is_empty());

        let regions = fold_state.regions();
        // Check that we have heading folds
        assert!(regions
            .iter()
            .any(|r| matches!(r.kind, FoldKind::Heading(_))));
    }

    #[test]
    fn test_code_block_folds() {
        let content = "Text\n\n```rust\nfn main() {}\n```\n\nMore text";
        let fold_state =
            detect_fold_regions(content, FileType::Markdown, false, true, false, false);

        let regions = fold_state.regions();
        assert_eq!(regions.len(), 1);
        assert!(matches!(regions[0].kind, FoldKind::CodeBlock));
        assert_eq!(regions[0].start_line, 2);
        assert_eq!(regions[0].end_line, 4);
    }

    #[test]
    fn test_indentation_folds_json() {
        let content = "{\n  \"key\": {\n    \"nested\": true\n  }\n}";
        let fold_state = detect_fold_regions(content, FileType::Json, false, false, false, true);

        assert!(!fold_state.is_empty());
        let regions = fold_state.regions();
        assert!(regions
            .iter()
            .any(|r| matches!(r.kind, FoldKind::Indentation)));
    }
}
