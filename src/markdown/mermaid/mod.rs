//! Native Mermaid Diagram Rendering
//!
//! This module provides native rendering of MermaidJS diagrams without external
//! dependencies. Diagrams are parsed and rendered directly using egui primitives.
//!
// Note: Many structs/fields in this module are defined for future diagram types
// or represent parsed data that isn't fully utilized yet. Suppressing dead_code
// warnings at the module level since this is actively evolving diagram code.
#![allow(dead_code)]
//!
//! # Supported Diagram Types
//!
//! - **Flowchart** (TD, TB, LR, RL, BT) - Nodes and edges with various shapes
//! - **Sequence Diagram** - Participants and message flows
//! - **Pie Chart** - Simple pie charts with labels
//! - **State Diagram** - State machines with transitions
//! - **Mindmap** - Hierarchical mind maps
//! - **Class Diagram** - UML class diagrams
//! - **ER Diagram** - Entity-relationship diagrams
//! - **Git Graph** - Git commit history visualization
//! - **Gantt Chart** - Project timeline charts
//! - **Timeline** - Event timelines
//! - **User Journey** - User experience journey maps
//!
//! # Architecture
//!
//! Each diagram type is implemented in its own submodule with:
//! 1. AST types for the parsed diagram structure
//! 2. Parser function to convert source text to AST
//! 3. Renderer function to draw the diagram using egui
//!
//! # Performance
//!
//! Flowchart diagrams use AST and layout caching to avoid re-parsing and
//! re-laying-out unchanged diagrams on every frame. The cache key includes:
//! - Source code hash (blake3)
//! - Font size (rounded to nearest 0.5)
//! - Available width (rounded to nearest 10 pixels)
//!
//! # Example
//!
//! ```ignore
//! use crate::markdown::mermaid::{render_mermaid_diagram, RenderResult};
//!
//! let source = "flowchart TD\n  A[Start] --> B[End]";
//! match render_mermaid_diagram(ui, source, dark_mode, font_size) {
//!     RenderResult::Success => println!("Rendered successfully"),
//!     RenderResult::ParseError(e) => println!("Parse error: {}", e),
//!     RenderResult::Unsupported(t) => println!("Unsupported type: {}", t),
//! }
//! ```

mod cache;
mod class_diagram;
mod er_diagram;
mod flowchart;
mod frontmatter;
mod gantt;
mod git_graph;
mod journey;
mod mindmap;
mod pie;
mod sequence;
mod state;
mod templates;
mod text;
mod timeline;
mod utils;
mod validation;

pub use templates::{mermaid_kind_menu_label, snippet_fenced_block, MermaidTemplateKind};
pub use validation::{compute_mermaid_diagnostics, validate_mermaid_source, MermaidError};

use egui::{Color32, FontId, Ui};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Mutex;

// Re-export cache types for external use
pub use cache::{CacheKey, CacheStats, MermaidCacheManager};

pub use text::EstimatedTextMeasurer;

// Re-export frontmatter types
use frontmatter::parse_frontmatter;

// Re-export text measurement utilities
pub use text::EguiTextMeasurer;

// ─────────────────────────────────────────────────────────────────────────────
// Global Diagram Cache
// ─────────────────────────────────────────────────────────────────────────────

/// Global cache for Mermaid diagram rendering.
///
/// Uses a Mutex for thread-safe access. The cache persists across frames
/// to avoid re-parsing and re-laying-out unchanged diagrams.
static DIAGRAM_CACHE: Mutex<Option<MermaidCacheManager>> = Mutex::new(None);

/// Get or initialize the global diagram cache.
fn with_cache<F, R>(f: F) -> R
where
    F: FnOnce(&mut MermaidCacheManager) -> R,
{
    let mut guard = DIAGRAM_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let cache = guard.get_or_insert_with(MermaidCacheManager::new);
    f(cache)
}

/// Clear the global diagram cache.
///
/// Call this when theme or global font settings change.
pub fn clear_diagram_cache() {
    if let Ok(mut guard) = DIAGRAM_CACHE.lock() {
        if let Some(cache) = guard.as_mut() {
            cache.clear();
        }
    }
}

/// Get cache statistics for monitoring.
pub fn get_cache_stats() -> Option<CacheStats> {
    DIAGRAM_CACHE
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(|c| c.stats().clone()))
}

// Re-export flowchart types and functions (includes types needed for HTML SVG export).
pub use flowchart::{
    layout_flowchart, parse_flowchart, render_flowchart, ArrowHead, EdgeStyle, Flowchart,
    FlowchartColors, FlowchartLayout, NodeLayout, NodeShape, NodeStyle,
};

// Internal imports for render_mermaid_diagram function
use class_diagram::{parse_class_diagram, render_class_diagram};
use er_diagram::{parse_er_diagram, render_er_diagram};
use gantt::{parse_gantt_chart, render_gantt_chart};
use git_graph::{parse_git_graph, render_git_graph};
use journey::{parse_user_journey, render_user_journey};
use mindmap::{parse_mindmap, render_mindmap};
use pie::{parse_pie_chart, render_pie_chart};
use sequence::{parse_sequence_diagram, render_sequence_diagram};
use state::{parse_state_diagram, render_state_diagram};
use timeline::{parse_timeline, render_timeline};

// Re-export types used in tests

// Re-export text measurer for tests

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Mermaid diagram text without optional YAML frontmatter (for parsers / export).
#[inline]
pub fn strip_frontmatter(source: &str) -> &str {
    parse_frontmatter(source).1
}

/// Result of attempting to render a mermaid diagram.
#[derive(Debug, Clone)]
pub enum RenderResult {
    /// Successfully rendered.
    Success,
    /// Parse error with message.
    ParseError(String),
    /// Diagram type not yet supported.
    Unsupported(String),
}

/// Render a mermaid diagram to the UI.
///
/// Returns a RenderResult indicating success or failure.
pub fn render_mermaid_diagram(
    ui: &mut Ui,
    source: &str,
    dark_mode: bool,
    font_size: f32,
) -> RenderResult {
    let _diag = crate::diag::SlowScope::new("mermaid::render_mermaid_diagram", 32);
    let source = source.trim();
    if source.is_empty() {
        return RenderResult::ParseError("Empty diagram source".to_string());
    }

    // Parse YAML frontmatter if present
    let (frontmatter, diagram_source) = parse_frontmatter(source);

    // Render title from frontmatter if present
    if let Some(ref fm) = frontmatter {
        if let Some(ref title) = fm.title {
            render_diagram_title(ui, title, dark_mode, font_size);
        }
    }

    // Use the diagram source (with frontmatter stripped) for type detection
    let first_line = diagram_source
        .lines()
        .map(|l| l.trim())
        .find(|l| !l.is_empty() && !l.starts_with("%%"))
        .unwrap_or("")
        .to_lowercase();

    if first_line.starts_with("flowchart") || first_line.starts_with("graph") {
        // Use cached flowchart if available
        let available_width = ui.available_width();
        let cache_key = CacheKey::new(diagram_source, font_size, available_width);

        // Try to get from cache first
        let cached = with_cache(|cache| {
            cache
                .get_flowchart(&cache_key)
                .map(|c| (c.flowchart.clone(), c.layout.clone()))
        });

        if let Some((flowchart, layout)) = cached {
            // Cache hit - just render using cached data
            let result = catch_unwind(AssertUnwindSafe(|| {
                let colors = if dark_mode {
                    FlowchartColors::dark()
                } else {
                    FlowchartColors::light()
                };
                render_flowchart(ui, &flowchart, &layout, &colors, font_size);
                RenderResult::Success
            }));

            match result {
                Ok(render_result) => return render_result,
                Err(panic_info) => {
                    let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                        format!("Internal error: {}", s)
                    } else if let Some(s) = panic_info.downcast_ref::<String>() {
                        format!("Internal error: {}", s)
                    } else {
                        "Internal error during flowchart rendering".to_string()
                    };
                    return RenderResult::ParseError(msg);
                }
            }
        }

        // Cache miss - parse, layout, cache, and render
        crate::diag::event(
            "mermaid_cache_miss",
            format!(
                "flowchart layout {:.0}px wide, {} chars",
                available_width,
                diagram_source.len()
            ),
        );
        let parse_result = parse_flowchart(diagram_source);

        match parse_result {
            Ok(flowchart) => {
                // Wrap layout and rendering in panic handler
                let result = catch_unwind(AssertUnwindSafe(|| {
                    let colors = if dark_mode {
                        FlowchartColors::dark()
                    } else {
                        FlowchartColors::light()
                    };
                    let text_measurer = EguiTextMeasurer::new(ui);
                    let layout =
                        layout_flowchart(&flowchart, available_width, font_size, &text_measurer);

                    // Cache the result for future frames
                    with_cache(|cache| {
                        cache.insert_flowchart(cache_key, flowchart.clone(), layout.clone());
                    });

                    render_flowchart(ui, &flowchart, &layout, &colors, font_size);
                    RenderResult::Success
                }));

                match result {
                    Ok(render_result) => render_result,
                    Err(panic_info) => {
                        let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                            format!("Internal error: {}", s)
                        } else if let Some(s) = panic_info.downcast_ref::<String>() {
                            format!("Internal error: {}", s)
                        } else {
                            "Internal error during flowchart rendering".to_string()
                        };
                        RenderResult::ParseError(msg)
                    }
                }
            }
            Err(e) => RenderResult::ParseError(e),
        }
    } else if first_line.starts_with("sequencediagram") {
        match parse_sequence_diagram(diagram_source) {
            Ok(diagram) => {
                render_sequence_diagram(ui, &diagram, dark_mode, font_size);
                RenderResult::Success
            }
            Err(e) => RenderResult::ParseError(e),
        }
    } else if first_line.starts_with("pie") {
        match parse_pie_chart(diagram_source) {
            Ok(chart) => {
                render_pie_chart(ui, &chart, dark_mode, font_size);
                RenderResult::Success
            }
            Err(e) => RenderResult::ParseError(e),
        }
    } else if first_line.starts_with("statediagram") {
        match parse_state_diagram(diagram_source) {
            Ok(diagram) => {
                render_state_diagram(ui, &diagram, dark_mode, font_size);
                RenderResult::Success
            }
            Err(e) => RenderResult::ParseError(e),
        }
    } else if first_line.starts_with("mindmap") {
        match parse_mindmap(diagram_source) {
            Ok(mindmap) => {
                render_mindmap(ui, &mindmap, dark_mode, font_size);
                RenderResult::Success
            }
            Err(e) => RenderResult::ParseError(e),
        }
    } else if first_line.starts_with("classdiagram") {
        match parse_class_diagram(diagram_source) {
            Ok(diagram) => {
                render_class_diagram(ui, &diagram, dark_mode, font_size);
                RenderResult::Success
            }
            Err(e) => RenderResult::ParseError(e),
        }
    } else if first_line.starts_with("erdiagram") {
        match parse_er_diagram(diagram_source) {
            Ok(diagram) => {
                render_er_diagram(ui, &diagram, dark_mode, font_size);
                RenderResult::Success
            }
            Err(e) => RenderResult::ParseError(e),
        }
    } else if first_line.starts_with("gantt") {
        match parse_gantt_chart(diagram_source) {
            Ok(chart) => {
                render_gantt_chart(ui, &chart, dark_mode, font_size);
                RenderResult::Success
            }
            Err(e) => RenderResult::ParseError(e),
        }
    } else if first_line.starts_with("gitgraph") {
        match parse_git_graph(diagram_source) {
            Ok(graph) => {
                render_git_graph(ui, &graph, dark_mode, font_size);
                RenderResult::Success
            }
            Err(e) => RenderResult::ParseError(e),
        }
    } else if first_line.starts_with("timeline") {
        match parse_timeline(diagram_source) {
            Ok(timeline) => {
                render_timeline(ui, &timeline, dark_mode, font_size);
                RenderResult::Success
            }
            Err(e) => RenderResult::ParseError(e),
        }
    } else if first_line.starts_with("journey") {
        match parse_user_journey(diagram_source) {
            Ok(journey) => {
                render_user_journey(ui, &journey, dark_mode, font_size);
                RenderResult::Success
            }
            Err(e) => RenderResult::ParseError(e),
        }
    } else {
        RenderResult::ParseError(format!("Unknown diagram type: {}", first_line))
    }
}

/// Render a diagram title above the diagram.
///
/// The title is displayed in a slightly larger font, centered above the diagram
/// with appropriate spacing.
fn render_diagram_title(ui: &mut Ui, title: &str, dark_mode: bool, font_size: f32) {
    let title_font_size = font_size * 1.3; // Slightly larger than diagram text
    let title_color = if dark_mode {
        Color32::from_rgb(220, 220, 220)
    } else {
        Color32::from_rgb(40, 40, 40)
    };

    ui.vertical_centered(|ui| {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(title)
                .font(FontId::proportional(title_font_size))
                .color(title_color)
                .strong(),
        );
        ui.add_space(8.0);
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::mermaid::flowchart::parser::{
        parse_direction, parse_edge_line_full, parse_node_from_text,
    };
    use crate::markdown::mermaid::flowchart::{FlowDirection, NodeShape, NodeStyle};
    use crate::markdown::mermaid::text::{EstimatedTextMeasurer, TextMeasurer};

    #[test]
    fn test_parse_simple_flowchart() {
        let source = "flowchart TD\n  A[Start] --> B[End]";
        let result = parse_flowchart(source);
        assert!(result.is_ok());
        let flowchart = result.unwrap();
        assert_eq!(flowchart.nodes.len(), 2);
        assert_eq!(flowchart.edges.len(), 1);
    }

    #[test]
    fn test_parse_direction() {
        assert_eq!(parse_direction("flowchart TD"), FlowDirection::TopDown);
        assert_eq!(parse_direction("flowchart LR"), FlowDirection::LeftRight);
        assert_eq!(parse_direction("flowchart BT"), FlowDirection::BottomUp);
        assert_eq!(parse_direction("flowchart RL"), FlowDirection::RightLeft);
    }

    #[test]
    fn test_parse_node_shapes() {
        let rect = parse_node_from_text("A[Text]").unwrap();
        assert_eq!(rect.2, NodeShape::Rectangle);

        let round = parse_node_from_text("B(Text)").unwrap();
        assert_eq!(round.2, NodeShape::RoundRect);

        let diamond = parse_node_from_text("C{Decision}").unwrap();
        assert_eq!(diamond.2, NodeShape::Diamond);

        let circle = parse_node_from_text("D((Circle))").unwrap();
        assert_eq!(circle.2, NodeShape::Circle);

        let trap = parse_node_from_text("E[/Trap\\]").unwrap();
        assert_eq!(trap.0, "E");
        assert_eq!(trap.1, "Trap");
        assert_eq!(trap.2, NodeShape::Trapezoid);

        let trap_inv = parse_node_from_text("F[\\TrapInv/]").unwrap();
        assert_eq!(trap_inv.0, "F");
        assert_eq!(trap_inv.1, "TrapInv");
        assert_eq!(trap_inv.2, NodeShape::TrapezoidInv);

        let para = parse_node_from_text("G[/Para/]").unwrap();
        assert_eq!(para.0, "G");
        assert_eq!(para.1, "Para");
        assert_eq!(para.2, NodeShape::Parallelogram);

        let dbl = parse_node_from_text("H(((Ring)))").unwrap();
        assert_eq!(dbl.0, "H");
        assert_eq!(dbl.1, "Ring");
        assert_eq!(dbl.2, NodeShape::DoubleCircle);
    }

    #[test]
    fn test_parse_edge_with_label() {
        let result = parse_edge_line_full("A -->|Yes| B");
        assert!(result.is_some());
        let (nodes, edges) = result.unwrap();
        assert_eq!(nodes.len(), 2);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].label, Some("Yes".to_string()));
    }

    #[test]
    fn test_parse_chained_edges() {
        // Test: A --> B --> C should create 3 nodes and 2 edges
        let result = parse_edge_line_full("a[Chapter 1] --> b[Chapter 2] --> c[Chapter 3]");
        assert!(result.is_some());
        let (nodes, edges) = result.unwrap();
        assert_eq!(nodes.len(), 3);
        assert_eq!(edges.len(), 2);

        // Check nodes
        assert_eq!(nodes[0].0, "a"); // id
        assert_eq!(nodes[0].1, "Chapter 1"); // label
        assert_eq!(nodes[1].0, "b");
        assert_eq!(nodes[1].1, "Chapter 2");
        assert_eq!(nodes[2].0, "c");
        assert_eq!(nodes[2].1, "Chapter 3");

        // Check edges
        assert_eq!(edges[0].from, "a");
        assert_eq!(edges[0].to, "b");
        assert_eq!(edges[1].from, "b");
        assert_eq!(edges[1].to, "c");
    }

    #[test]
    fn test_parse_chained_edges_with_labels() {
        // Test: A -->|Yes| B -->|No| C
        let result = parse_edge_line_full("A -->|Yes| B -->|No| C");
        assert!(result.is_some());
        let (nodes, edges) = result.unwrap();
        assert_eq!(nodes.len(), 3);
        assert_eq!(edges.len(), 2);
        assert_eq!(edges[0].label, Some("Yes".to_string()));
        assert_eq!(edges[1].label, Some("No".to_string()));
    }

    #[test]
    fn test_parse_flowchart_with_chained_edges() {
        // Test the full flowchart parsing with chained edges
        let source = r#"flowchart LR
            a[Chapter 1] --> b[Chapter 2] --> c[Chapter 3]
            c-->d[Using Ledger]
            c-->e[Using Trezor]
            d-->f[Chapter 4]
            e-->f"#;

        let result = parse_flowchart(source);
        assert!(result.is_ok());
        let flowchart = result.unwrap();
        assert_eq!(flowchart.nodes.len(), 6); // a, b, c, d, e, f
        assert_eq!(flowchart.edges.len(), 6); // a->b, b->c, c->d, c->e, d->f, e->f
        assert_eq!(flowchart.direction, FlowDirection::LeftRight);
    }

    #[test]
    fn test_parse_decision_tree_with_chained_edges() {
        // Test case from task: coffee machine troubleshooting diagram
        // This tests chained edges with labels and diamond (decision) nodes
        let source = r#"graph TD
            A[Coffee machine not working] --> B{Machine has power?}
            B -->|No| H[Plug in and turn on]
            B -->|Yes| C[Out of beans or water?] -->|Yes| G[Refill beans and water]
            C -->|No| D{Filter warning?} -->|Yes| I[Replace or clean filter]
            D -->|No| F[Send for repair]"#;

        let result = parse_flowchart(source);
        assert!(result.is_ok());
        let flowchart = result.unwrap();

        // Should have nodes: A, B, H, C, G, D, I, F = 8 nodes
        assert_eq!(flowchart.nodes.len(), 8);

        // Should have edges:
        // A->B, B->H, B->C, C->G, C->D, D->I, D->F = 7 edges
        assert_eq!(flowchart.edges.len(), 7);

        // Verify direction
        assert_eq!(flowchart.direction, FlowDirection::TopDown);

        // Verify some node shapes
        let b_node = flowchart.nodes.iter().find(|n| n.id == "B").unwrap();
        assert_eq!(b_node.shape, NodeShape::Diamond); // {Decision} syntax

        let a_node = flowchart.nodes.iter().find(|n| n.id == "A").unwrap();
        assert_eq!(a_node.shape, NodeShape::Rectangle);
        assert_eq!(a_node.label, "Coffee machine not working");
    }

    #[test]
    fn test_parse_multiple_edges() {
        let source = r#"flowchart TD
            A[Start] --> B{Decision}
            B -->|Yes| C[Great!]
            B -->|No| D[Debug]
            D --> E[Fix]
            E --> B
            C --> F[End]"#;

        let result = parse_flowchart(source);
        assert!(result.is_ok());
        let flowchart = result.unwrap();
        assert_eq!(flowchart.nodes.len(), 6); // A, B, C, D, E, F
        assert_eq!(flowchart.edges.len(), 6);
    }

    #[test]
    fn test_layout_produces_valid_positions() {
        let source = "flowchart TD\n  A[Start] --> B[End]";
        let flowchart = parse_flowchart(source).unwrap();
        let text_measurer = EstimatedTextMeasurer::new();
        let layout = layout_flowchart(&flowchart, 400.0, 14.0, &text_measurer);

        assert_eq!(layout.nodes.len(), 2);
        assert!(layout.nodes.contains_key("A"));
        assert!(layout.nodes.contains_key("B"));

        // In TD layout, B should be below A
        let a_pos = layout.nodes.get("A").unwrap().pos;
        let b_pos = layout.nodes.get("B").unwrap().pos;
        assert!(b_pos.y > a_pos.y);
    }

    #[test]
    fn test_layout_left_right_direction() {
        let source = "flowchart LR\n  A[Start] --> B[End]";
        let flowchart = parse_flowchart(source).unwrap();
        assert_eq!(flowchart.direction, FlowDirection::LeftRight);

        let text_measurer = EstimatedTextMeasurer::new();
        let layout = layout_flowchart(&flowchart, 800.0, 14.0, &text_measurer);

        let a_pos = layout.nodes.get("A").unwrap().pos;
        let b_pos = layout.nodes.get("B").unwrap().pos;

        // In LR layout, B should be to the right of A (larger x)
        println!("LR layout: A.pos = {:?}, B.pos = {:?}", a_pos, b_pos);
        assert!(
            b_pos.x > a_pos.x,
            "In LR layout, B should be to the right of A. A.x={}, B.x={}",
            a_pos.x,
            b_pos.x
        );
        // Y coordinates should be similar (same vertical level)
        assert!(
            (a_pos.y - b_pos.y).abs() < 50.0,
            "In LR layout, A and B should be at similar Y levels"
        );
    }

    #[test]
    fn test_layout_right_left_direction() {
        let source = "flowchart RL\n  A[Start] --> B[End]";
        let flowchart = parse_flowchart(source).unwrap();
        assert_eq!(flowchart.direction, FlowDirection::RightLeft);

        let text_measurer = EstimatedTextMeasurer::new();
        let layout = layout_flowchart(&flowchart, 800.0, 14.0, &text_measurer);

        let a_pos = layout.nodes.get("A").unwrap().pos;
        let b_pos = layout.nodes.get("B").unwrap().pos;

        // In RL layout, B should be to the left of A (smaller x)
        println!("RL layout: A.pos = {:?}, B.pos = {:?}", a_pos, b_pos);
        assert!(
            b_pos.x < a_pos.x,
            "In RL layout, B should be to the left of A. A.x={}, B.x={}",
            a_pos.x,
            b_pos.x
        );
    }

    #[test]
    fn test_layout_bottom_top_direction() {
        let source = "flowchart BT\n  A[Start] --> B[End]";
        let flowchart = parse_flowchart(source).unwrap();
        assert_eq!(flowchart.direction, FlowDirection::BottomUp);

        let text_measurer = EstimatedTextMeasurer::new();
        let layout = layout_flowchart(&flowchart, 400.0, 14.0, &text_measurer);

        let a_pos = layout.nodes.get("A").unwrap().pos;
        let b_pos = layout.nodes.get("B").unwrap().pos;

        // In BT layout, B should be above A (smaller y)
        println!("BT layout: A.pos = {:?}, B.pos = {:?}", a_pos, b_pos);
        assert!(
            b_pos.y < a_pos.y,
            "In BT layout, B should be above A. A.y={}, B.y={}",
            a_pos.y,
            b_pos.y
        );
    }

    #[test]
    fn test_layout_lr_complex_diagram() {
        // Test a complex diagram with multiple layers and branching
        let source = r#"flowchart LR
            A[Start] --> B{Decision}
            B -->|Yes| C[Process 1]
            B -->|No| D[Process 2]
            C --> E[End]
            D --> E"#;

        let flowchart = parse_flowchart(source).unwrap();
        assert_eq!(flowchart.direction, FlowDirection::LeftRight);
        assert_eq!(flowchart.nodes.len(), 5);

        let text_measurer = EstimatedTextMeasurer::new();
        let layout = layout_flowchart(&flowchart, 800.0, 14.0, &text_measurer);

        let a_pos = layout.nodes.get("A").unwrap().pos;
        let b_pos = layout.nodes.get("B").unwrap().pos;
        let c_pos = layout.nodes.get("C").unwrap().pos;
        let d_pos = layout.nodes.get("D").unwrap().pos;
        let e_pos = layout.nodes.get("E").unwrap().pos;

        println!("LR complex layout:");
        println!("  A = {:?}", a_pos);
        println!("  B = {:?}", b_pos);
        println!("  C = {:?}", c_pos);
        println!("  D = {:?}", d_pos);
        println!("  E = {:?}", e_pos);

        // Layer 0: A
        // Layer 1: B
        // Layer 2: C, D (branch)
        // Layer 3: E

        // For LR: x increases as we go through layers
        assert!(b_pos.x > a_pos.x, "B should be to the right of A");
        assert!(c_pos.x > b_pos.x, "C should be to the right of B");
        assert!(d_pos.x > b_pos.x, "D should be to the right of B");
        assert!(e_pos.x > c_pos.x, "E should be to the right of C");
        assert!(e_pos.x > d_pos.x, "E should be to the right of D");

        // C and D should be in the same layer (same x) but different y
        assert!(
            (c_pos.x - d_pos.x).abs() < 10.0,
            "C and D should be in same layer (same x)"
        );
        assert!(
            (c_pos.y - d_pos.y).abs() > 10.0,
            "C and D should be at different y positions (branching)"
        );
    }

    #[test]
    fn test_layout_td_complex_diagram() {
        // Same diagram but TD direction
        let source = r#"flowchart TD
            A[Start] --> B{Decision}
            B -->|Yes| C[Process 1]
            B -->|No| D[Process 2]
            C --> E[End]
            D --> E"#;

        let flowchart = parse_flowchart(source).unwrap();
        assert_eq!(flowchart.direction, FlowDirection::TopDown);

        let text_measurer = EstimatedTextMeasurer::new();
        let layout = layout_flowchart(&flowchart, 800.0, 14.0, &text_measurer);

        let a_pos = layout.nodes.get("A").unwrap().pos;
        let b_pos = layout.nodes.get("B").unwrap().pos;
        let c_pos = layout.nodes.get("C").unwrap().pos;
        let d_pos = layout.nodes.get("D").unwrap().pos;
        let e_pos = layout.nodes.get("E").unwrap().pos;

        println!("TD complex layout:");
        println!("  A = {:?}", a_pos);
        println!("  B = {:?}", b_pos);
        println!("  C = {:?}", c_pos);
        println!("  D = {:?}", d_pos);
        println!("  E = {:?}", e_pos);

        // For TD: y increases as we go through layers
        assert!(b_pos.y > a_pos.y, "B should be below A");
        assert!(c_pos.y > b_pos.y, "C should be below B");
        assert!(d_pos.y > b_pos.y, "D should be below B");
        assert!(e_pos.y > c_pos.y, "E should be below C");
        assert!(e_pos.y > d_pos.y, "E should be below D");

        // C and D should be in the same layer (same y) but different x
        assert!(
            (c_pos.y - d_pos.y).abs() < 10.0,
            "C and D should be in same layer (same y)"
        );
        assert!(
            (c_pos.x - d_pos.x).abs() > 10.0,
            "C and D should be at different x positions (branching)"
        );

        // Mermaid convention: FIRST-declared edge target goes LEFT
        // B -->|Yes| C is declared first, B -->|No| D is declared second
        // So C should be to the LEFT of D (smaller x)
        assert!(
            c_pos.x < d_pos.x,
            "C (first branch 'Yes') should be LEFT of D (second branch 'No'). C.x={}, D.x={}",
            c_pos.x,
            d_pos.x
        );
    }

    #[test]
    fn test_layout_coffee_machine_all_nodes() {
        // Test that all 8 nodes get layout positions (verifies no missing nodes)
        let source = r#"graph TD
            A[Coffee machine not working] --> B{Machine has power?}
            B -->|No| H[Plug in and turn on]
            B -->|Yes| C[Out of beans or water?] -->|Yes| G[Refill beans and water]
            C -->|No| D{Filter warning?} -->|Yes| I[Replace or clean filter]
            D -->|No| F[Send for repair]"#;

        let flowchart = parse_flowchart(source).unwrap();
        assert_eq!(flowchart.nodes.len(), 8, "Should have 8 nodes");

        let text_measurer = EstimatedTextMeasurer::new();
        let layout = layout_flowchart(&flowchart, 800.0, 14.0, &text_measurer);

        println!(
            "Coffee machine layout ({} nodes in layout):",
            layout.nodes.len()
        );

        // Verify ALL 8 nodes are in the layout
        let expected_nodes = ["A", "B", "C", "D", "F", "G", "H", "I"];
        for node_id in expected_nodes {
            assert!(
                layout.nodes.contains_key(node_id),
                "Node '{}' should be in layout",
                node_id
            );
            let pos = layout.nodes.get(node_id).unwrap();
            println!("  {} = pos {:?}, size {:?}", node_id, pos.pos, pos.size);
        }

        assert_eq!(layout.nodes.len(), 8, "Layout should have all 8 nodes");

        // Verify branch ordering for decision node B:
        // B -->|No| H is declared first, B -->|Yes| C is declared second
        // So H should be LEFT of C
        let h_pos = layout.nodes.get("H").unwrap().pos;
        let c_pos = layout.nodes.get("C").unwrap().pos;
        assert!(
            h_pos.x < c_pos.x,
            "H (first branch 'No') should be LEFT of C (second branch 'Yes'). H.x={}, C.x={}",
            h_pos.x,
            c_pos.x
        );

        // Regression guard: siblings on the same layer (matched by y position)
        // must not overlap horizontally. Previously the barycenter post-pass
        // pulled C onto H and D onto G; the layer-overlap safety net now
        // guarantees a minimum gap between every adjacent sibling pair.
        let mut by_layer: std::collections::BTreeMap<i32, Vec<(&str, f32, f32)>> =
            std::collections::BTreeMap::new();
        for id in expected_nodes {
            let n = layout.nodes.get(id).unwrap();
            let key = n.pos.y.round() as i32;
            by_layer
                .entry(key)
                .or_default()
                .push((id, n.pos.x, n.pos.x + n.size.x));
        }
        for (y, mut row) in by_layer {
            if row.len() < 2 {
                continue;
            }
            row.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            for window in row.windows(2) {
                let (left_id, _, left_right) = window[0];
                let (right_id, right_left, _) = window[1];
                let gap = right_left - left_right;
                assert!(
                    gap >= 0.0,
                    "siblings {left_id} (right={left_right}) and {right_id} \
                     (left={right_left}) overlap at y={y} (gap={gap})"
                );
            }
        }
    }

    #[test]
    fn test_layout_chapter_flow_lr() {
        // Test case from task: Chapter flow diagram with LR direction
        let source = r#"flowchart LR
            a[Chapter 1] --> b[Chapter 2] --> c[Chapter 3]
            c-->d[Using Ledger]
            c-->e[Using Trezor]
            d-->f[Chapter 4]
            e-->f"#;

        let flowchart = parse_flowchart(source).unwrap();
        assert_eq!(flowchart.nodes.len(), 6, "Should have 6 nodes");
        assert_eq!(flowchart.direction, FlowDirection::LeftRight);

        let text_measurer = EstimatedTextMeasurer::new();
        let layout = layout_flowchart(&flowchart, 800.0, 14.0, &text_measurer);

        println!(
            "Chapter flow LR layout ({} nodes in layout):",
            layout.nodes.len()
        );

        // Verify ALL 6 nodes are in the layout
        let expected_nodes = ["a", "b", "c", "d", "e", "f"];
        for node_id in expected_nodes {
            assert!(
                layout.nodes.contains_key(node_id),
                "Node '{}' should be in layout",
                node_id
            );
            let pos = layout.nodes.get(node_id).unwrap();
            println!("  {} = pos {:?}, size {:?}", node_id, pos.pos, pos.size);
        }

        assert_eq!(layout.nodes.len(), 6, "Layout should have all 6 nodes");

        // Verify layer ordering: a -> b -> c -> d,e -> f
        let a_pos = layout.nodes.get("a").unwrap().pos;
        let b_pos = layout.nodes.get("b").unwrap().pos;
        let c_pos = layout.nodes.get("c").unwrap().pos;
        let d_pos = layout.nodes.get("d").unwrap().pos;
        let e_pos = layout.nodes.get("e").unwrap().pos;
        let f_pos = layout.nodes.get("f").unwrap().pos;

        // In LR layout, x increases through layers
        assert!(b_pos.x > a_pos.x, "b should be right of a");
        assert!(c_pos.x > b_pos.x, "c should be right of b");
        assert!(d_pos.x > c_pos.x, "d should be right of c");
        assert!(e_pos.x > c_pos.x, "e should be right of c");
        assert!(f_pos.x > d_pos.x, "f should be right of d");
        assert!(f_pos.x > e_pos.x, "f should be right of e");

        // d and e should be in same layer (same x)
        assert!(
            (d_pos.x - e_pos.x).abs() < 10.0,
            "d and e should be in same layer"
        );

        // Branch ordering: c-->d declared first, c-->e declared second
        // In LR, d (first) should be TOP (smaller y), e (second) should be BOTTOM (larger y)
        assert!(
            d_pos.y < e_pos.y,
            "d (first branch) should be above e (second branch). d.y={}, e.y={}",
            d_pos.y,
            e_pos.y
        );
    }

    #[test]
    fn test_text_measurer_trait() {
        let measurer = EstimatedTextMeasurer::new();

        // Test basic measurement
        let size = measurer.measure("Hello", 14.0);
        assert!(size.width > 0.0);
        assert!(size.height > 0.0);

        // Longer text should have greater width
        let size_longer = measurer.measure("Hello World", 14.0);
        assert!(size_longer.width > size.width);

        // Test row height
        let row_height = measurer.row_height(14.0);
        assert!(row_height > 0.0);
    }

    #[test]
    fn test_semicolon_stripping() {
        // Test: A-->B; should create nodes A and B (not 'B;')
        let result = parse_edge_line_full("A-->B;");
        assert!(result.is_some());
        let (nodes, edges) = result.unwrap();
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].0, "A");
        assert_eq!(nodes[1].0, "B"); // Should NOT be "B;"
        assert_eq!(edges.len(), 1);
    }

    #[test]
    fn test_semicolon_in_header() {
        // Test: graph TD; header should parse correctly
        let source = "graph TD;\n  A-->B;";
        let result = parse_flowchart(source);
        assert!(result.is_ok());
        let flowchart = result.unwrap();
        assert_eq!(flowchart.direction, FlowDirection::TopDown);
        assert_eq!(flowchart.nodes.len(), 2);
        assert_eq!(flowchart.nodes[0].id, "A");
        assert_eq!(flowchart.nodes[1].id, "B");
    }

    #[test]
    fn test_semicolon_with_labels() {
        // Test: semicolons with labeled nodes
        let source = "graph TD;\n  A[Start]-->B[End];";
        let result = parse_flowchart(source);
        assert!(result.is_ok());
        let flowchart = result.unwrap();
        assert_eq!(flowchart.nodes.len(), 2);
        assert_eq!(flowchart.nodes[0].label, "Start");
        assert_eq!(flowchart.nodes[1].label, "End");
    }

    #[test]
    fn test_ampersand_simple() {
        // Test: A & B --> C should create edges A→C and B→C
        let result = parse_edge_line_full("A & B --> C");
        assert!(result.is_some());
        let (nodes, edges) = result.unwrap();
        assert_eq!(nodes.len(), 3, "Should have 3 nodes: A, B, C");
        assert_eq!(edges.len(), 2, "Should have 2 edges: A→C and B→C");

        // Check nodes
        let node_ids: Vec<&str> = nodes.iter().map(|(id, _, _)| id.as_str()).collect();
        assert!(node_ids.contains(&"A"));
        assert!(node_ids.contains(&"B"));
        assert!(node_ids.contains(&"C"));

        // Check edges
        assert!(edges.iter().any(|e| e.from == "A" && e.to == "C"));
        assert!(edges.iter().any(|e| e.from == "B" && e.to == "C"));
    }

    #[test]
    fn test_ampersand_target() {
        // Test: A --> B & C should create edges A→B and A→C
        let result = parse_edge_line_full("A --> B & C");
        assert!(result.is_some());
        let (nodes, edges) = result.unwrap();
        assert_eq!(nodes.len(), 3, "Should have 3 nodes: A, B, C");
        assert_eq!(edges.len(), 2, "Should have 2 edges: A→B and A→C");

        // Check edges
        assert!(edges.iter().any(|e| e.from == "A" && e.to == "B"));
        assert!(edges.iter().any(|e| e.from == "A" && e.to == "C"));
    }

    #[test]
    fn test_ampersand_both_sides() {
        // Test: A & B --> C & D should create 4 edges
        let result = parse_edge_line_full("A & B --> C & D");
        assert!(result.is_some());
        let (nodes, edges) = result.unwrap();
        assert_eq!(nodes.len(), 4, "Should have 4 nodes: A, B, C, D");
        assert_eq!(edges.len(), 4, "Should have 4 edges: A→C, A→D, B→C, B→D");

        // Check all 4 edges exist
        assert!(edges.iter().any(|e| e.from == "A" && e.to == "C"));
        assert!(edges.iter().any(|e| e.from == "A" && e.to == "D"));
        assert!(edges.iter().any(|e| e.from == "B" && e.to == "C"));
        assert!(edges.iter().any(|e| e.from == "B" && e.to == "D"));
    }

    #[test]
    fn test_ampersand_in_flowchart() {
        // Test full flowchart with ampersand syntax
        let source = r#"flowchart TD
            A & B --> C & D"#;

        let result = parse_flowchart(source);
        assert!(result.is_ok());
        let flowchart = result.unwrap();
        assert_eq!(flowchart.nodes.len(), 4);
        assert_eq!(flowchart.edges.len(), 4);
    }

    #[test]
    fn test_ampersand_with_semicolon() {
        // Test: A & B --> C & D; (combining both syntaxes)
        let result = parse_edge_line_full("A & B --> C & D;");
        assert!(result.is_some());
        let (nodes, edges) = result.unwrap();
        assert_eq!(nodes.len(), 4);
        assert_eq!(edges.len(), 4);

        // Make sure D doesn't have a semicolon
        let d_node = nodes.iter().find(|(id, _, _)| id == "D");
        assert!(d_node.is_some(), "Node D should exist");
        assert!(
            !d_node.unwrap().0.contains(';'),
            "Node D should not contain semicolon"
        );
    }

    #[test]
    fn test_truncate_with_ellipsis() {
        let measurer = EstimatedTextMeasurer::new();

        // Text that fits should not be truncated
        let short_text = "Hi";
        let result = measurer.truncate_with_ellipsis(short_text, 14.0, 100.0);
        assert_eq!(result, short_text);

        // Long text should be truncated
        let long_text = "This is a very long label that should be truncated";
        let result = measurer.truncate_with_ellipsis(long_text, 14.0, 50.0);
        assert!(result.len() < long_text.len());
        assert!(result.ends_with('…'));
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // classDef and class directive tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_classdef_basic() {
        let source = r#"flowchart TD
            A[Start] --> B[Process]
            classDef green fill:#9f6,stroke:#333,stroke-width:2px
            class A green"#;

        let result = parse_flowchart(source);
        assert!(result.is_ok());
        let flowchart = result.unwrap();

        // Check that classDef was parsed
        assert!(
            flowchart.class_defs.contains_key("green"),
            "Should have 'green' class defined"
        );
        let green_style = flowchart.class_defs.get("green").unwrap();

        // Check fill color (#9f6 -> RGB(153, 255, 102))
        assert!(green_style.fill.is_some(), "Should have fill color");
        let fill = green_style.fill.unwrap();
        assert_eq!(fill.r(), 153, "Fill red component");
        assert_eq!(fill.g(), 255, "Fill green component");
        assert_eq!(fill.b(), 102, "Fill blue component");

        // Check stroke color (#333 -> RGB(51, 51, 51))
        assert!(green_style.stroke.is_some(), "Should have stroke color");
        let stroke = green_style.stroke.unwrap();
        assert_eq!(stroke.r(), 51);
        assert_eq!(stroke.g(), 51);
        assert_eq!(stroke.b(), 51);

        // Check stroke width
        assert_eq!(green_style.stroke_width, Some(2.0));

        // Check that class assignment was parsed
        assert!(
            flowchart.node_classes.contains_key("A"),
            "Node A should have class assigned"
        );
        assert_eq!(flowchart.node_classes.get("A"), Some(&"green".to_string()));
    }

    #[test]
    fn test_classdef_multiple_classes() {
        let source = r#"flowchart TD
            A --> B --> C --> D
            classDef red fill:#f00,stroke:#000
            classDef blue fill:#00f
            class A red
            class B,C blue"#;

        let result = parse_flowchart(source);
        assert!(result.is_ok());
        let flowchart = result.unwrap();

        // Check both class definitions exist
        assert!(flowchart.class_defs.contains_key("red"));
        assert!(flowchart.class_defs.contains_key("blue"));

        // Check class assignments
        assert_eq!(flowchart.node_classes.get("A"), Some(&"red".to_string()));
        assert_eq!(flowchart.node_classes.get("B"), Some(&"blue".to_string()));
        assert_eq!(flowchart.node_classes.get("C"), Some(&"blue".to_string()));
        assert!(
            !flowchart.node_classes.contains_key("D"),
            "D should have no class"
        );
    }

    #[test]
    fn test_classdef_hex_color_formats() {
        // Test different hex color formats
        let source = r#"flowchart TD
            A --> B --> C
            classDef long fill:#99ff66
            classDef alpha fill:#99ff66dd"#;

        let result = parse_flowchart(source);
        assert!(result.is_ok());
        let flowchart = result.unwrap();

        // #99ff66 -> (153, 255, 102)
        let long = flowchart.class_defs.get("long").unwrap();
        assert!(long.fill.is_some());
        let long_fill = long.fill.unwrap();
        assert_eq!(long_fill.r(), 153); // 0x99
        assert_eq!(long_fill.g(), 255); // 0xff
        assert_eq!(long_fill.b(), 102); // 0x66

        // #99ff66dd - check alpha exists and is approximately correct
        // Note: egui's Color32 uses premultiplied alpha internally, so RGB values
        // may be modified when alpha < 255. We just verify the color was parsed.
        let alpha = flowchart.class_defs.get("alpha").unwrap();
        assert!(alpha.fill.is_some());
        let alpha_fill = alpha.fill.unwrap();
        assert_eq!(alpha_fill.a(), 221); // 0xdd - alpha is preserved
                                         // RGB values will be premultiplied by alpha internally, so we don't assert exact values
    }

    #[test]
    fn test_classdef_short_hex() {
        // Test 3-char hex color format separately
        let source = r#"flowchart TD
            A --> B
            classDef short fill:#f00"#;

        let result = parse_flowchart(source);
        assert!(result.is_ok());
        let flowchart = result.unwrap();

        // #f00 -> #ff0000 (255, 0, 0) - pure red
        let short = flowchart.class_defs.get("short").unwrap();
        assert!(short.fill.is_some());
        let fill = short.fill.unwrap();
        // 0xf * 17 = 15 * 17 = 255
        assert_eq!(fill.r(), 255);
        assert_eq!(fill.g(), 0);
        assert_eq!(fill.b(), 0);
    }

    #[test]
    fn test_classdef_stroke_width_formats() {
        let source = r#"flowchart TD
            A --> B --> C
            classDef withPx stroke-width:4px
            classDef withoutPx stroke-width:3"#;

        let result = parse_flowchart(source);
        assert!(result.is_ok());
        let flowchart = result.unwrap();

        let with_px = flowchart.class_defs.get("withPx").unwrap();
        assert_eq!(with_px.stroke_width, Some(4.0));

        let without_px = flowchart.class_defs.get("withoutPx").unwrap();
        assert_eq!(without_px.stroke_width, Some(3.0));
    }

    #[test]
    fn test_classdef_undefined_class_reference() {
        // Using a class that doesn't exist should not crash
        let source = r#"flowchart TD
            A --> B
            class A nonexistent"#;

        let result = parse_flowchart(source);
        assert!(result.is_ok());
        let flowchart = result.unwrap();

        // Node A references nonexistent class
        assert_eq!(
            flowchart.node_classes.get("A"),
            Some(&"nonexistent".to_string())
        );
        // But no classDef for it
        assert!(!flowchart.class_defs.contains_key("nonexistent"));
    }

    #[test]
    fn test_classdef_with_semicolons() {
        let source = r#"graph TD;
            A-->B;
            classDef green fill:#9f6;
            class A green;"#;

        let result = parse_flowchart(source);
        assert!(result.is_ok());
        let flowchart = result.unwrap();

        // Should still parse classDef correctly despite semicolons in source
        assert!(flowchart.class_defs.contains_key("green"));
        assert!(flowchart.node_classes.contains_key("A"));
    }

    #[test]
    fn test_class_assignment_multiple_nodes() {
        let source = r#"flowchart TD
            A --> B --> C --> D --> E
            classDef highlight fill:#ff0
            class A,B,C highlight"#;

        let result = parse_flowchart(source);
        assert!(result.is_ok());
        let flowchart = result.unwrap();

        // All three nodes should have the highlight class
        assert_eq!(
            flowchart.node_classes.get("A"),
            Some(&"highlight".to_string())
        );
        assert_eq!(
            flowchart.node_classes.get("B"),
            Some(&"highlight".to_string())
        );
        assert_eq!(
            flowchart.node_classes.get("C"),
            Some(&"highlight".to_string())
        );
        assert!(!flowchart.node_classes.contains_key("D"));
        assert!(!flowchart.node_classes.contains_key("E"));
    }

    #[test]
    fn test_classdef_partial_style() {
        // Only fill, no stroke
        let source = r#"flowchart TD
            A --> B
            classDef onlyFill fill:#f00"#;

        let result = parse_flowchart(source);
        assert!(result.is_ok());
        let flowchart = result.unwrap();

        let style = flowchart.class_defs.get("onlyFill").unwrap();
        assert!(style.fill.is_some());
        assert!(style.stroke.is_none());
        assert!(style.stroke_width.is_none());
    }

    #[test]
    fn test_classdef_color_text_field() {
        let source = r#"flowchart TD
            A --> B
            classDef themed fill:#222,stroke:#888,color:#0f0"#;
        let flowchart = parse_flowchart(source).unwrap();
        let st = flowchart.class_defs.get("themed").unwrap();
        assert!(st.fill.is_some());
        assert!(st.color.is_some());
        assert_eq!(st.color.unwrap(), egui::Color32::from_rgb(0, 255, 0));
    }

    #[test]
    fn test_style_directive_round_trip() {
        let source = r#"flowchart TD
            A[One] --> B[Two]
            style A fill:#ff0000,stroke:#00ff00,stroke-width:3px,color:#0000ff"#;
        let fc = parse_flowchart(source).unwrap();
        let st = fc.node_styles.get("A").expect("style A");
        assert_eq!(st.fill, Some(egui::Color32::from_rgb(255, 0, 0)));
        assert_eq!(st.stroke, Some(egui::Color32::from_rgb(0, 255, 0)));
        assert_eq!(st.stroke_width, Some(3.0));
        assert_eq!(st.color, Some(egui::Color32::from_rgb(0, 0, 255)));
    }

    #[test]
    fn test_style_overrides_class_per_field() {
        let source = r#"flowchart TD
            A[Node] --> B
            classDef c fill:#00ff00,stroke:#000000,stroke-width:1px,color:#ffffff
            class A c
            style A fill:#ff0000,color:#111111"#;
        let fc = parse_flowchart(source).unwrap();
        let class_st = fc.class_defs.get("c").unwrap();
        let merged = NodeStyle::merge_class_and_inline(Some(class_st), fc.node_styles.get("A"));
        let m = merged.expect("merged");
        assert_eq!(m.fill, Some(egui::Color32::from_rgb(255, 0, 0)));
        assert_eq!(m.stroke, Some(egui::Color32::BLACK));
        assert_eq!(m.stroke_width, Some(1.0));
        assert_eq!(m.color, Some(egui::Color32::from_rgb(17, 17, 17)));
    }

    #[test]
    fn test_existing_shapes_still_parse() {
        assert_eq!(
            parse_node_from_text("S([Stadium])").unwrap().2,
            NodeShape::Stadium
        );
        assert_eq!(
            parse_node_from_text("Cyl[(Cyl)]").unwrap().2,
            NodeShape::Cylinder
        );
        assert_eq!(
            parse_node_from_text("Sub[[Sub]]").unwrap().2,
            NodeShape::Subroutine
        );
        assert_eq!(
            parse_node_from_text("Hx{{Hex}}").unwrap().2,
            NodeShape::Hexagon
        );
        assert_eq!(
            parse_node_from_text("P[/Slash/]").unwrap().2,
            NodeShape::Parallelogram
        );
        assert_eq!(
            parse_node_from_text("Q>Flag]").unwrap().2,
            NodeShape::Asymmetric
        );
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Subgraph layout tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_subgraph_nodes_clustered() {
        // Test that nodes in a subgraph are assigned to consecutive layers
        // Note: Nodes must be FIRST defined inside the subgraph to be associated with it
        let source = r#"flowchart TD
            subgraph Group
                A[Start Process] --> B[Middle]
                B --> C[End Process]
            end
            Entry --> A
            C --> Exit"#;

        let flowchart = parse_flowchart(source).unwrap();
        assert_eq!(flowchart.nodes.len(), 5); // Entry, A, B, C, Exit
        assert_eq!(flowchart.subgraphs.len(), 1);

        // Verify subgraph contains A, B, C
        let group = flowchart
            .subgraphs
            .iter()
            .find(|s| s.id == "Group")
            .unwrap();
        assert!(
            group.node_ids.contains(&"A".to_string()),
            "A should be in Group"
        );
        assert!(
            group.node_ids.contains(&"B".to_string()),
            "B should be in Group"
        );
        assert!(
            group.node_ids.contains(&"C".to_string()),
            "C should be in Group"
        );

        let text_measurer = EstimatedTextMeasurer::new();
        let layout = layout_flowchart(&flowchart, 800.0, 14.0, &text_measurer);

        // All nodes should be laid out
        assert_eq!(layout.nodes.len(), 5);

        // Get positions
        let entry_pos = layout.nodes.get("Entry").unwrap().pos;
        let a_pos = layout.nodes.get("A").unwrap().pos;
        let b_pos = layout.nodes.get("B").unwrap().pos;
        let c_pos = layout.nodes.get("C").unwrap().pos;
        let exit_pos = layout.nodes.get("Exit").unwrap().pos;

        println!("Subgraph test positions:");
        println!("  Entry: {:?}", entry_pos);
        println!("  A: {:?}", a_pos);
        println!("  B: {:?}", b_pos);
        println!("  C: {:?}", c_pos);
        println!("  Exit: {:?}", exit_pos);

        // In TD layout, y increases through layers
        // Expected: Entry -> A -> B -> C -> Exit
        assert!(a_pos.y > entry_pos.y, "A should be below Entry");
        assert!(b_pos.y > a_pos.y, "B should be below A");
        assert!(c_pos.y > b_pos.y, "C should be below B");
        assert!(exit_pos.y > c_pos.y, "Exit should be below C");

        // Subgraph bounding box should exist
        assert!(
            layout.subgraphs.contains_key("Group"),
            "Subgraph 'Group' should have a layout"
        );
        let sg_layout = layout.subgraphs.get("Group").unwrap();

        println!(
            "  Subgraph bounds: pos={:?}, size={:?}",
            sg_layout.pos, sg_layout.size
        );

        // Subgraph bounding box should contain A, B, C but not Entry or Exit
        // The bounding box includes padding above for title
        assert!(
            sg_layout.pos.y <= a_pos.y,
            "Subgraph top should be at or above A"
        );
        assert!(
            sg_layout.pos.y + sg_layout.size.y >= c_pos.y,
            "Subgraph bottom should be at or below C"
        );

        // Entry should be above the subgraph
        assert!(
            entry_pos.y < sg_layout.pos.y
                || entry_pos.y + layout.nodes.get("Entry").unwrap().size.y
                    < sg_layout.pos.y + sg_layout.size.y / 2.0,
            "Entry should be above the subgraph content"
        );
    }

    #[test]
    fn test_subgraph_with_external_connections() {
        // Test a subgraph where nodes have edges to external nodes
        let source = r#"flowchart TD
            External1 --> A
            subgraph Process
                A --> B
                B --> C
            end
            C --> External2
            External1 --> External2"#;

        let flowchart = parse_flowchart(source).unwrap();

        let text_measurer = EstimatedTextMeasurer::new();
        let layout = layout_flowchart(&flowchart, 800.0, 14.0, &text_measurer);

        // Get positions of subgraph nodes
        let a_pos = layout.nodes.get("A").unwrap().pos;
        let b_pos = layout.nodes.get("B").unwrap().pos;
        let c_pos = layout.nodes.get("C").unwrap().pos;

        println!("External connections test:");
        println!("  A: {:?}", a_pos);
        println!("  B: {:?}", b_pos);
        println!("  C: {:?}", c_pos);

        // Subgraph nodes A, B, C should be in consecutive layers
        // Calculate layer differences based on y-positions (assuming consistent spacing)
        let layer_height = b_pos.y - a_pos.y; // Reference height

        // B should be exactly one layer below A
        assert!(
            (b_pos.y - a_pos.y).abs() < layer_height * 1.5,
            "B should be close to one layer below A"
        );

        // C should be exactly one layer below B
        assert!(
            (c_pos.y - b_pos.y - layer_height).abs() < layer_height * 0.5,
            "C should be one layer below B"
        );
    }

    #[test]
    fn test_multiple_subgraphs() {
        // Test multiple separate subgraphs
        let source = r#"flowchart TD
            Start --> A1
            Start --> B1
            subgraph GroupA
                A1 --> A2
            end
            subgraph GroupB
                B1 --> B2
            end
            A2 --> End
            B2 --> End"#;

        let flowchart = parse_flowchart(source).unwrap();
        assert_eq!(flowchart.subgraphs.len(), 2);

        let text_measurer = EstimatedTextMeasurer::new();
        let layout = layout_flowchart(&flowchart, 800.0, 14.0, &text_measurer);

        // Both subgraphs should have layouts
        assert!(
            layout.subgraphs.contains_key("GroupA"),
            "GroupA should have layout"
        );
        assert!(
            layout.subgraphs.contains_key("GroupB"),
            "GroupB should have layout"
        );

        // Get positions
        let a1_pos = layout.nodes.get("A1").unwrap().pos;
        let a2_pos = layout.nodes.get("A2").unwrap().pos;
        let b1_pos = layout.nodes.get("B1").unwrap().pos;
        let b2_pos = layout.nodes.get("B2").unwrap().pos;

        println!("Multiple subgraphs test:");
        println!("  A1: {:?}, A2: {:?}", a1_pos, a2_pos);
        println!("  B1: {:?}, B2: {:?}", b1_pos, b2_pos);

        // Within each subgraph, nodes should be in consecutive layers
        assert!(a2_pos.y > a1_pos.y, "A2 should be below A1");
        assert!(b2_pos.y > b1_pos.y, "B2 should be below B1");
    }

    #[test]
    fn test_subgraph_internal_layout() {
        // Test that subgraph contents are laid out together and the bounding box
        // correctly encompasses all contained nodes
        let source = r#"flowchart TD
            Start --> A
            subgraph Process[Processing Pipeline]
                A[Input] --> B[Transform]
                B --> C[Output]
            end
            C --> End"#;

        let flowchart = parse_flowchart(source).unwrap();
        assert_eq!(flowchart.subgraphs.len(), 1);

        let text_measurer = EstimatedTextMeasurer::new();
        let layout = layout_flowchart(&flowchart, 800.0, 14.0, &text_measurer);

        // Subgraph should have a layout
        let sg_layout = layout
            .subgraphs
            .get("Process")
            .expect("Process subgraph should have layout");

        // Get node positions and sizes
        let a_layout = layout.nodes.get("A").unwrap();
        let b_layout = layout.nodes.get("B").unwrap();
        let c_layout = layout.nodes.get("C").unwrap();

        println!("Subgraph internal layout test:");
        println!(
            "  Subgraph: pos={:?}, size={:?}",
            sg_layout.pos, sg_layout.size
        );
        println!(
            "  A: {:?}, B: {:?}, C: {:?}",
            a_layout.pos, b_layout.pos, c_layout.pos
        );

        // Subgraph bounding box should encompass all its nodes (with padding)
        // The subgraph pos includes padding, so node left edges should be >= subgraph left + small margin
        assert!(
            a_layout.pos.x >= sg_layout.pos.x,
            "A should be inside subgraph (x)"
        );
        assert!(
            c_layout.pos.x + c_layout.size.x <= sg_layout.pos.x + sg_layout.size.x,
            "C should be inside subgraph (right edge)"
        );
        assert!(
            c_layout.pos.y + c_layout.size.y <= sg_layout.pos.y + sg_layout.size.y,
            "C should be inside subgraph (bottom edge)"
        );

        // Title should be present
        assert_eq!(sg_layout.title.as_deref(), Some("Processing Pipeline"));

        // Verify subgraph bounding box is reasonable (not too large)
        assert!(
            sg_layout.size.x < 300.0,
            "Subgraph width should be reasonable"
        );
        assert!(
            sg_layout.size.y < 400.0,
            "Subgraph height should be reasonable"
        );

        // Nodes inside subgraph should be in consecutive layers (vertically aligned for TD)
        assert!(b_layout.pos.y > a_layout.pos.y, "B should be below A");
        assert!(c_layout.pos.y > b_layout.pos.y, "C should be below B");
    }

    #[test]
    fn test_nested_subgraph_layout() {
        // Test nested subgraphs - outer contains inner, both have proper bounds
        let source = r#"flowchart TB
            subgraph outer[Outer Container]
                subgraph inner[Inner Container]
                    A[Node A] --> B[Node B]
                end
                B --> C[Node C]
            end"#;

        let flowchart = parse_flowchart(source).unwrap();
        assert_eq!(flowchart.subgraphs.len(), 2, "Should have 2 subgraphs");

        let text_measurer = EstimatedTextMeasurer::new();
        let layout = layout_flowchart(&flowchart, 800.0, 14.0, &text_measurer);

        // Both subgraphs should have layouts
        let inner_layout = layout
            .subgraphs
            .get("inner")
            .expect("inner subgraph should have layout");
        let outer_layout = layout
            .subgraphs
            .get("outer")
            .expect("outer subgraph should have layout");

        // Get node positions
        let a_layout = layout.nodes.get("A").unwrap();
        let b_layout = layout.nodes.get("B").unwrap();
        let c_layout = layout.nodes.get("C").unwrap();

        println!("Nested subgraph test:");
        println!(
            "  Inner: pos={:?}, size={:?}",
            inner_layout.pos, inner_layout.size
        );
        println!(
            "  Outer: pos={:?}, size={:?}",
            outer_layout.pos, outer_layout.size
        );
        println!(
            "  A: {:?}, B: {:?}, C: {:?}",
            a_layout.pos, b_layout.pos, c_layout.pos
        );

        // Inner subgraph should contain A and B
        assert!(
            a_layout.pos.x >= inner_layout.pos.x,
            "A should be inside inner (x)"
        );
        assert!(
            b_layout.pos.x >= inner_layout.pos.x,
            "B should be inside inner (x)"
        );

        // Outer subgraph should contain inner subgraph AND C
        assert!(
            inner_layout.pos.x >= outer_layout.pos.x,
            "Inner left edge should be >= outer left edge"
        );
        assert!(
            inner_layout.pos.x + inner_layout.size.x <= outer_layout.pos.x + outer_layout.size.x,
            "Inner right edge should be <= outer right edge"
        );
        assert!(
            c_layout.pos.x >= outer_layout.pos.x,
            "C should be inside outer (x)"
        );

        // Titles should be present
        assert_eq!(inner_layout.title.as_deref(), Some("Inner Container"));
        assert_eq!(outer_layout.title.as_deref(), Some("Outer Container"));
    }

    #[test]
    fn test_subgraph_title_width_expansion() {
        // Test that subgraph width expands to fit long titles
        // This verifies the fix for title truncation issue
        let source = r#"flowchart TD
            subgraph veryLongTitle[This Is A Very Long Subgraph Title That Should Not Be Truncated]
                A[Small] --> B[Node]
            end"#;

        let flowchart = parse_flowchart(source).unwrap();
        assert_eq!(flowchart.subgraphs.len(), 1);

        let text_measurer = EstimatedTextMeasurer::new();
        let layout = layout_flowchart(&flowchart, 800.0, 14.0, &text_measurer);

        let sg_layout = layout
            .subgraphs
            .get("veryLongTitle")
            .expect("subgraph should have layout");

        // Measure the title width using the same measurer
        let title = sg_layout.title.as_ref().expect("title should be present");
        let title_size = text_measurer.measure(title, 14.0);

        // Title padding: 12px left + 12px right = 24px
        let min_required_width = title_size.width + 24.0;

        println!("Title width expansion test:");
        println!("  Title: '{}'", title);
        println!("  Title text width: {}", title_size.width);
        println!("  Min required subgraph width: {}", min_required_width);
        println!("  Actual subgraph width: {}", sg_layout.size.x);

        // The subgraph width should be at least as wide as the title + padding
        assert!(
            sg_layout.size.x >= min_required_width,
            "Subgraph width ({}) should be >= title width + padding ({})",
            sg_layout.size.x,
            min_required_width
        );
    }

    #[test]
    fn test_subgraph_short_title_no_extra_expansion() {
        // Test that subgraphs with short titles are not expanded beyond content width
        // (unless the title itself requires it)
        let source = r#"flowchart TD
            subgraph sg[A]
                A[Small] --> B[Node]
            end"#;

        let flowchart = parse_flowchart(source).unwrap();
        let text_measurer = EstimatedTextMeasurer::new();
        let layout = layout_flowchart(&flowchart, 800.0, 14.0, &text_measurer);

        let sg_layout = layout
            .subgraphs
            .get("sg")
            .expect("subgraph should have layout");

        // Measure the short title
        let title = sg_layout.title.as_ref().expect("title should be present");
        let title_size = text_measurer.measure(title, 14.0);
        let min_width_for_title = title_size.width + 24.0;

        println!("Short title test:");
        println!("  Title: '{}'", title);
        println!("  Title text width: {}", title_size.width);
        println!("  Min width for title: {}", min_width_for_title);
        println!("  Actual subgraph width: {}", sg_layout.size.x);

        // The subgraph should be at least as wide as the title requires
        assert!(
            sg_layout.size.x >= min_width_for_title,
            "Subgraph width ({}) should be >= title min width ({})",
            sg_layout.size.x,
            min_width_for_title
        );

        // For a short title like "A", the content width should dominate,
        // so the subgraph width should be significantly larger than the title min width
        assert!(
            sg_layout.size.x > min_width_for_title + 20.0,
            "With short title, content width should dominate (subgraph: {}, title min: {})",
            sg_layout.size.x,
            min_width_for_title
        );
    }

    #[test]
    fn test_fc_83a_layout_has_all_nodes() {
        let source = r#"graph TD
    linkStyle default interpolate basis
    A[Enter Chart Definition] --> B(Preview)
    B --> C{decide}
    C --> D[Keep]
    C --> E[Edit Definition]
    E --> B
    D --> F[Save Image and Code]
    F --> B"#;

        let flowchart = parse_flowchart(source).unwrap();
        assert_eq!(flowchart.nodes.len(), 6, "should parse 6 nodes");

        let text_measurer = EstimatedTextMeasurer::new();
        let layout = layout_flowchart(&flowchart, 800.0, 14.0, &text_measurer);

        for id in ["A", "B", "C", "D", "E", "F"] {
            assert!(
                layout.nodes.contains_key(id),
                "node {id} missing from layout"
            );
        }

        let d = layout.nodes.get("D").unwrap();
        let e = layout.nodes.get("E").unwrap();
        let f = layout.nodes.get("F").unwrap();
        let c = layout.nodes.get("C").unwrap();

        let center_x = |id: &str| {
            let n = layout.nodes.get(id).unwrap();
            n.pos.x + n.size.x / 2.0
        };

        assert!(
            (center_x("A") - center_x("B")).abs() < 1.0,
            "B should align with A on the main axis"
        );
        assert!(
            center_x("C") < center_x("B") - 5.0,
            "decide should sit at branch barycenter (left of Preview), got C={} B={}",
            center_x("C"),
            center_x("B")
        );
        assert!(
            (center_x("C") - center_x("F")).abs() > 5.0,
            "F stays on the main axis; decide shifts toward branches"
        );
        assert!(
            (center_x("A") - center_x("F")).abs() < 1.0,
            "F should align with the main axis"
        );
        assert!(center_x("D") < center_x("C"), "D should be left of C");
        assert!(center_x("E") > center_x("C"), "E should be right of C");

        let min_x = layout
            .nodes
            .values()
            .map(|n| n.pos.x)
            .fold(f32::INFINITY, f32::min);
        assert!(
            (min_x - 20.0).abs() < 1.0,
            "layout should start at margin, not centered in available_width; min_x={min_x}"
        );

        assert!(d.pos.y > c.pos.y, "D should be below C");
        assert!(f.pos.y > d.pos.y, "F should be below D");
        assert!(
            f.pos.y + f.size.y <= layout.total_size.y,
            "F should fit within total_size height (f_bottom={}, total_h={})",
            f.pos.y + f.size.y,
            layout.total_size.y
        );
    }
}
