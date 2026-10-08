//! Arrow Up/Down navigation between rendered (WYSIWYG) blocks (#170).
//!
//! ## WYSIWYG model
//! - Inside an active click-to-edit / session TextEdit, ArrowUp/Down move the caret
//!   on visual rows (egui TextEdit).
//! - When the caret is already on the **first** visual row, ArrowUp leaves to the
//!   previous navigable block (caret at end).
//! - When the caret is already on the **last** visual row, ArrowDown leaves to the
//!   next navigable block (caret at start).
//! - Single-line blocks (headings, short paragraphs) are always on the first/last
//!   row, so Up/Down move between blocks immediately.
//! - Preview lock: navigation (focus target + scroll) still runs; edits stay gated
//!   by existing preview-lock paths.
//! - Tables / CSV keep their own cell arrow handling — skip when `TableCell` is active.
//! - Raw and Split raw panes are unchanged: callers must only consume keys when the
//!   rendered pane owns focus.

use crate::markdown::parser::{MarkdownNode, MarkdownNodeType};
use crate::markdown::rendered_session::BlockRef;
use eframe::egui::text::CCursor;
use eframe::egui::Galley;

/// Vertical direction for block-to-block navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArrowNavDir {
    #[default]
    Up,
    Down,
}

/// True when `cursor_index` lies on the first visual row of `galley`.
#[allow(dead_code)] // kept for galley-based callers / docs of the WYSIWYG model
pub fn caret_on_first_visual_row(galley: &Galley, cursor_index: usize) -> bool {
    if galley.rows.is_empty() {
        return true;
    }
    let cursor = CCursor::new(cursor_index);
    galley.layout_from_cursor(cursor).row == 0
}

/// True when `cursor_index` lies on the last visual row of `galley`.
#[allow(dead_code)]
pub fn caret_on_last_visual_row(galley: &Galley, cursor_index: usize) -> bool {
    let n = galley.rows.len();
    if n == 0 {
        return true;
    }
    let cursor = CCursor::new(cursor_index);
    galley.layout_from_cursor(cursor).row + 1 >= n
}

/// Whether ArrowUp/Down at this caret should leave the current block.
#[allow(dead_code)]
pub fn should_leave_block_on_arrow(galley: &Galley, cursor_index: usize, dir: ArrowNavDir) -> bool {
    match dir {
        ArrowNavDir::Up => caret_on_first_visual_row(galley, cursor_index),
        ArrowNavDir::Down => caret_on_last_visual_row(galley, cursor_index),
    }
}

/// Fallback when no prior-frame galley row cache exists (first focus frame).
///
/// Treats char index 0 as first-row and `chars().count()` as last-row — correct for
/// single-line widgets; slightly conservative for wrapped multi-line until a galley
/// row is cached.
pub fn should_leave_block_on_arrow_char_fallback(
    text_char_len: usize,
    cursor_index: usize,
    dir: ArrowNavDir,
) -> bool {
    match dir {
        ArrowNavDir::Up => cursor_index == 0,
        ArrowNavDir::Down => cursor_index >= text_char_len,
    }
}

/// Index of `current` in `blocks`, or the closest block by source line.
pub fn index_of_navigable(blocks: &[BlockRef], current: BlockRef) -> Option<usize> {
    if let Some(i) = blocks.iter().position(|b| *b == current) {
        return Some(i);
    }
    let line = block_nav_line(current);
    blocks
        .iter()
        .enumerate()
        .filter(|(_, b)| block_nav_line(**b) <= line)
        .map(|(i, _)| i)
        .next_back()
        .or_else(|| {
            blocks
                .iter()
                .enumerate()
                .find(|(_, b)| block_nav_line(**b) >= line)
                .map(|(i, _)| i)
        })
}

/// Next/previous navigable block. `current == None` → first (Down) or last (Up).
pub fn adjacent_navigable(
    blocks: &[BlockRef],
    current: Option<BlockRef>,
    dir: ArrowNavDir,
) -> Option<BlockRef> {
    if blocks.is_empty() {
        return None;
    }
    let Some(cur) = current else {
        return Some(match dir {
            ArrowNavDir::Down => blocks[0],
            ArrowNavDir::Up => *blocks.last().unwrap(),
        });
    };
    let idx = index_of_navigable(blocks, cur)?;
    match dir {
        ArrowNavDir::Down => blocks.get(idx + 1).copied(),
        ArrowNavDir::Up => idx.checked_sub(1).map(|i| blocks[i]),
    }
}

fn block_nav_line(block: BlockRef) -> usize {
    match block {
        BlockRef::Heading { line, .. }
        | BlockRef::Paragraph { line }
        | BlockRef::ListItem { line, .. }
        | BlockRef::FormattedParagraph { line, .. }
        | BlockRef::FormattedListItem { line, .. } => line,
        BlockRef::TableCell { table_line, .. } => table_line,
    }
}

fn paragraph_has_inline_formatting(node: &MarkdownNode) -> bool {
    node.children.iter().any(|c| {
        matches!(
            c.node_type,
            MarkdownNodeType::Link { .. }
                | MarkdownNodeType::Wikilink { .. }
                | MarkdownNodeType::Strong
                | MarkdownNodeType::Emphasis
                | MarkdownNodeType::Strikethrough
                | MarkdownNodeType::Code(_)
                | MarkdownNodeType::Image { .. }
                | MarkdownNodeType::LineBreak
        )
    })
}

fn push_paragraph_block(out: &mut Vec<BlockRef>, node: &MarkdownNode) {
    if paragraph_has_inline_formatting(node) {
        out.push(BlockRef::FormattedParagraph {
            line: node.start_line,
            structural: false,
        });
    } else {
        out.push(BlockRef::Paragraph {
            line: node.start_line,
        });
    }
}

fn collect_list_items(out: &mut Vec<BlockRef>, list_node: &MarkdownNode) {
    for (idx, child) in list_node.children.iter().enumerate() {
        let should = matches!(
            &child.node_type,
            MarkdownNodeType::Item | MarkdownNodeType::TaskItem { .. }
        );
        if !should {
            continue;
        }

        let para_node = child
            .children
            .iter()
            .find(|c| matches!(c.node_type, MarkdownNodeType::Paragraph));

        let has_inline = para_node
            .map(paragraph_has_inline_formatting)
            .unwrap_or(false);

        if has_inline {
            if let Some(para) = para_node {
                out.push(BlockRef::FormattedListItem {
                    line: para.start_line,
                    item: idx as u32,
                    structural: false,
                });
            }
        } else if let Some(para) = para_node {
            let text = para.text_content();
            if !text.is_empty() {
                out.push(BlockRef::ListItem {
                    line: para.start_line,
                    item: idx as u32,
                });
            }
        } else {
            let text: String = child
                .children
                .iter()
                .filter(|c| {
                    !matches!(
                        c.node_type,
                        MarkdownNodeType::List { .. } | MarkdownNodeType::TaskItem { .. }
                    )
                })
                .map(|c| c.text_content())
                .collect::<Vec<_>>()
                .join("");
            if !text.is_empty() {
                out.push(BlockRef::ListItem {
                    line: child.start_line,
                    item: idx as u32,
                });
            }
        }

        for nested in child
            .children
            .iter()
            .filter(|c| matches!(c.node_type, MarkdownNodeType::List { .. }))
        {
            collect_list_items(out, nested);
        }
    }
}

fn collect_from_node(out: &mut Vec<BlockRef>, node: &MarkdownNode) {
    match &node.node_type {
        MarkdownNodeType::Heading { .. } => {
            out.push(BlockRef::Heading {
                line: node.start_line,
                structural: false,
            });
        }
        MarkdownNodeType::Paragraph => {
            // Skip entity-only / HTML-inline paragraphs that are display-only.
            let has_html_inline = node.children.iter().any(|c| {
                matches!(
                    c.node_type,
                    MarkdownNodeType::HtmlInline(_)
                        | MarkdownNodeType::Kbd
                        | MarkdownNodeType::Superscript
                        | MarkdownNodeType::Subscript
                )
            });
            if !has_html_inline {
                push_paragraph_block(out, node);
            }
        }
        MarkdownNodeType::List { .. } => collect_list_items(out, node),
        MarkdownNodeType::BlockQuote
        | MarkdownNodeType::Callout { .. }
        | MarkdownNodeType::AlignedDiv { .. }
        | MarkdownNodeType::Details { .. }
        | MarkdownNodeType::Document => {
            for child in &node.children {
                collect_from_node(out, child);
            }
        }
        // Tables keep in-widget arrow nav; code/mermaid/hr/video are not session blocks.
        _ => {}
    }
}

/// Document-order list of session-backed navigable blocks (matches non-structural render path).
///
/// O(nodes) AST walk — no layout. Safe to call once per frame.
pub fn collect_navigable_blocks(root: &MarkdownNode) -> Vec<BlockRef> {
    let mut out = Vec::new();
    collect_from_node(&mut out, root);
    out
}

/// Map a navigable block's source line to a top-level culling Y for scroll-into-view.
pub fn scroll_y_for_source_line(
    block_line_ranges: &[(usize, usize)],
    block_start_y: &[f32],
    source_line: usize,
) -> Option<f32> {
    if block_line_ranges.len() != block_start_y.len() || source_line == 0 {
        return None;
    }
    let idx = block_line_ranges
        .iter()
        .position(|&(s, e)| s <= source_line && source_line <= e)?;
    block_start_y.get(idx).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::parser::parse_markdown;

    #[test]
    fn adjacent_none_current_picks_ends() {
        let blocks = vec![
            BlockRef::Paragraph { line: 1 },
            BlockRef::Paragraph { line: 3 },
            BlockRef::Paragraph { line: 5 },
        ];
        assert_eq!(
            adjacent_navigable(&blocks, None, ArrowNavDir::Down),
            Some(BlockRef::Paragraph { line: 1 })
        );
        assert_eq!(
            adjacent_navigable(&blocks, None, ArrowNavDir::Up),
            Some(BlockRef::Paragraph { line: 5 })
        );
    }

    #[test]
    fn adjacent_moves_and_clamps() {
        let blocks = vec![
            BlockRef::Heading {
                line: 1,
                structural: false,
            },
            BlockRef::Paragraph { line: 3 },
            BlockRef::Paragraph { line: 5 },
        ];
        let h = BlockRef::Heading {
            line: 1,
            structural: false,
        };
        assert_eq!(
            adjacent_navigable(&blocks, Some(h), ArrowNavDir::Down),
            Some(BlockRef::Paragraph { line: 3 })
        );
        assert_eq!(adjacent_navigable(&blocks, Some(h), ArrowNavDir::Up), None);
        assert_eq!(
            adjacent_navigable(
                &blocks,
                Some(BlockRef::Paragraph { line: 5 }),
                ArrowNavDir::Down
            ),
            None
        );
        assert_eq!(
            adjacent_navigable(
                &blocks,
                Some(BlockRef::Paragraph { line: 5 }),
                ArrowNavDir::Up
            ),
            Some(BlockRef::Paragraph { line: 3 })
        );
    }

    #[test]
    fn char_fallback_boundaries() {
        assert!(should_leave_block_on_arrow_char_fallback(
            5,
            0,
            ArrowNavDir::Up
        ));
        assert!(!should_leave_block_on_arrow_char_fallback(
            5,
            2,
            ArrowNavDir::Up
        ));
        assert!(should_leave_block_on_arrow_char_fallback(
            5,
            5,
            ArrowNavDir::Down
        ));
        assert!(!should_leave_block_on_arrow_char_fallback(
            5,
            4,
            ArrowNavDir::Down
        ));
    }

    #[test]
    fn collect_multi_paragraph_and_heading() {
        let doc = parse_markdown("# Title\n\nAlpha\n\nBeta\n\n**bold** gamma\n").unwrap();
        let blocks = collect_navigable_blocks(&doc.root);
        assert_eq!(
            blocks,
            vec![
                BlockRef::Heading {
                    line: 1,
                    structural: false,
                },
                BlockRef::Paragraph { line: 3 },
                BlockRef::Paragraph { line: 5 },
                BlockRef::FormattedParagraph {
                    line: 7,
                    structural: false,
                },
            ]
        );
    }

    #[test]
    fn collect_list_items_in_order() {
        let doc = parse_markdown("- one\n- two\n\nPara\n").unwrap();
        let blocks = collect_navigable_blocks(&doc.root);
        assert!(
            blocks
                .iter()
                .any(|b| matches!(b, BlockRef::ListItem { .. })),
            "expected list items: {blocks:?}"
        );
        assert!(blocks
            .iter()
            .any(|b| matches!(b, BlockRef::Paragraph { line: 4 })));
        // List items come before the trailing paragraph.
        let last = blocks.last().copied();
        assert_eq!(last, Some(BlockRef::Paragraph { line: 4 }));
    }

    #[test]
    fn collect_formatted_list_item_uses_child_index() {
        // Ordered lists display 1-based numbers; BlockRef.item must stay 0-based child idx
        // so arrow nav matches the live render path.
        let doc = parse_markdown("1. **bold** one\n2. two\n").unwrap();
        let blocks = collect_navigable_blocks(&doc.root);
        assert!(
            blocks.iter().any(|b| {
                matches!(
                    b,
                    BlockRef::FormattedListItem {
                        item: 0,
                        structural: false,
                        ..
                    }
                )
            }),
            "expected FormattedListItem item=0, got {blocks:?}"
        );
    }

    #[test]
    fn scroll_y_maps_contained_line() {
        let ranges = vec![(1, 1), (3, 5), (7, 7)];
        let ys = vec![0.0, 40.0, 120.0];
        assert_eq!(scroll_y_for_source_line(&ranges, &ys, 4), Some(40.0));
        assert_eq!(scroll_y_for_source_line(&ranges, &ys, 7), Some(120.0));
        assert_eq!(scroll_y_for_source_line(&ranges, &ys, 99), None);
    }
}
