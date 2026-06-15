//! Flowchart parsing functions.
//!
//! Converts Mermaid flowchart source text into a `Flowchart` AST.
//! Handles node shapes, edge styles, subgraphs, chained edges,
//! classDef/class directives, and linkStyle directives.

use egui::{Color32, Pos2};
use std::collections::{HashMap, HashSet};

use super::types::*;

// ─────────────────────────────────────────────────────────────────────────────
// Parser
// ─────────────────────────────────────────────────────────────────────────────

/// Parse mermaid flowchart source into a Flowchart AST.
pub fn parse_flowchart(source: &str) -> Result<Flowchart, String> {
    let mut flowchart = Flowchart::default();
    let lines: Vec<&str> = source.lines().collect();
    let mut node_map: HashMap<String, usize> = HashMap::new();
    let mut line_idx = 0;

    // Parse header line (skip comments and empty lines)
    let mut found_header = false;
    while line_idx < lines.len() {
        let header_trimmed = lines[line_idx].trim();
        line_idx += 1;

        // Skip empty lines and comments
        if header_trimmed.is_empty() || header_trimmed.starts_with("%%") {
            continue;
        }
        let header_lower = header_trimmed.to_lowercase();
        if header_lower.starts_with("flowchart") || header_lower.starts_with("graph") {
            flowchart.direction = parse_direction(&header_lower);
            found_header = true;
            break;
        } else {
            return Err("Expected 'flowchart' or 'graph' declaration".to_string());
        }
    }

    if !found_header {
        return Err("Empty flowchart source".to_string());
    }

    // Parse body with subgraph support
    let mut subgraph_stack: Vec<SubgraphBuilder> = Vec::new();
    let mut subgraph_counter = 0;

    while line_idx < lines.len() {
        let line = lines[line_idx].trim();
        line_idx += 1;

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        let line_lower = line.to_lowercase();

        // Parse classDef directive: classDef className fill:#fff,stroke:#000,stroke-width:2px
        if line_lower.starts_with("classdef ") {
            if let Some((class_name, style)) = parse_class_def(line) {
                flowchart.class_defs.insert(class_name, style);
            }
            continue;
        }

        // Parse class directive: class nodeId1,nodeId2 className
        if line_lower.starts_with("class ") {
            parse_class_assignment(line, &mut flowchart.node_classes);
            continue;
        }

        // Parse linkStyle directive: linkStyle <index|default> <css-properties>
        if line_lower.starts_with("linkstyle ") {
            parse_link_style(line, &mut flowchart);
            continue;
        }

        // style nodeId … (per-node CSS-like properties)
        if line_lower.starts_with("style ") {
            parse_style_directive(line, &mut flowchart.node_styles);
            continue;
        }

        // Interactive links — not rendered
        if line_lower.starts_with("click ") {
            continue;
        }

        // Check for subgraph start
        if line_lower.starts_with("subgraph") {
            let (id, title) = parse_subgraph_header(line, &mut subgraph_counter);
            subgraph_stack.push(SubgraphBuilder {
                id: id.clone(),
                title,
                node_ids: Vec::new(),
                child_subgraph_ids: Vec::new(),
                direction: None,
            });
            continue;
        }

        // Check for subgraph end
        if line_lower == "end" {
            if let Some(builder) = subgraph_stack.pop() {
                let subgraph = FlowSubgraph {
                    id: builder.id.clone(),
                    title: builder.title,
                    node_ids: builder.node_ids,
                    child_subgraph_ids: builder.child_subgraph_ids,
                    direction: builder.direction,
                };

                // Register this subgraph as a child of the parent (if any)
                if let Some(parent) = subgraph_stack.last_mut() {
                    parent.child_subgraph_ids.push(builder.id.clone());
                }

                flowchart.subgraphs.push(subgraph);
            }
            continue;
        }

        // Check for direction override inside subgraph
        if !subgraph_stack.is_empty() && line_lower.starts_with("direction") {
            if let Some(current) = subgraph_stack.last_mut() {
                current.direction = Some(parse_direction(&line_lower));
            }
            continue;
        }

        // Try to parse as edge (contains arrow) - use the full parser for chained edges
        if let Some((nodes, edges)) = parse_edge_line_full(line) {
            for (id, label, shape) in nodes {
                if let Some(&idx) = node_map.get(&id) {
                    // Node exists - update if new definition has more info
                    let existing = &mut flowchart.nodes[idx];
                    // Only update and associate with subgraph if this is a NEW definition
                    // (has label content beyond just the ID). Plain references like "C --> E"
                    // where C was already defined elsewhere should NOT add C to this subgraph.
                    if label != id && existing.label == existing.id {
                        existing.label = label;
                        existing.shape = shape;

                        // Only associate with current subgraph when actually defining the node
                        if let Some(current) = subgraph_stack.last_mut() {
                            if !current.node_ids.contains(&id) {
                                current.node_ids.push(id);
                            }
                        }
                    }
                    // Note: Plain references to existing nodes don't add them to the current subgraph
                } else {
                    node_map.insert(id.clone(), flowchart.nodes.len());
                    flowchart.nodes.push(FlowNode {
                        id: id.clone(),
                        label,
                        shape,
                    });

                    // Associate with current subgraph if any
                    if let Some(current) = subgraph_stack.last_mut() {
                        current.node_ids.push(id);
                    }
                }
            }
            // Add all edges from the chain
            for e in edges {
                flowchart.edges.push(e);
            }
        } else if let Some(node) = parse_node_definition(line) {
            // Standalone node definition
            if let Some(&idx) = node_map.get(&node.id) {
                // Node exists - update if new definition has more info
                let existing = &mut flowchart.nodes[idx];
                if node.label != node.id && existing.label == existing.id {
                    existing.label = node.label;
                    existing.shape = node.shape;
                }

                // Associate with current subgraph if node appears inside it
                if let Some(current) = subgraph_stack.last_mut() {
                    if !current.node_ids.contains(&node.id) {
                        current.node_ids.push(node.id);
                    }
                }
            } else {
                let id = node.id.clone();
                node_map.insert(id.clone(), flowchart.nodes.len());
                flowchart.nodes.push(node);

                // Associate with current subgraph if any
                if let Some(current) = subgraph_stack.last_mut() {
                    current.node_ids.push(id);
                }
            }
        }
    }

    // Handle any unclosed subgraphs (close them at end of diagram)
    while let Some(builder) = subgraph_stack.pop() {
        let subgraph = FlowSubgraph {
            id: builder.id.clone(),
            title: builder.title,
            node_ids: builder.node_ids,
            child_subgraph_ids: builder.child_subgraph_ids,
            direction: builder.direction,
        };

        if let Some(parent) = subgraph_stack.last_mut() {
            parent.child_subgraph_ids.push(builder.id.clone());
        }

        flowchart.subgraphs.push(subgraph);
    }

    collect_position_hints(source, &node_map, &mut flowchart);

    Ok(flowchart)
}

/// Result of attempting to parse a `%% @pos` comment line.
enum PosHintParse {
    NotPosHint,
    Malformed(String),
    Parsed { node_id: String, x: f32, y: f32 },
}

/// Parse `%% @pos <node_id> <x> <y>` from a trimmed comment line.
fn parse_pos_hint_line(line: &str) -> PosHintParse {
    let trimmed = line.trim();
    if !trimmed.starts_with("%%") {
        return PosHintParse::NotPosHint;
    }

    let after_comment = trimmed[2..].trim();
    if !after_comment.to_lowercase().starts_with("@pos") {
        return PosHintParse::NotPosHint;
    }

    let rest = after_comment[4..].trim();
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.len() != 3 {
        return PosHintParse::Malformed(
            "Malformed @pos hint: expected '%% @pos <node_id> <x> <y>'".to_string(),
        );
    }

    let node_id = parts[0].to_string();
    let x = match parts[1].parse::<f32>() {
        Ok(v) => v,
        Err(_) => {
            return PosHintParse::Malformed(format!(
                "Invalid x coordinate '{}' in @pos hint for node '{node_id}'",
                parts[1]
            ));
        }
    };
    let y = match parts[2].parse::<f32>() {
        Ok(v) => v,
        Err(_) => {
            return PosHintParse::Malformed(format!(
                "Invalid y coordinate '{}' in @pos hint for node '{node_id}'",
                parts[2]
            ));
        }
    };

    PosHintParse::Parsed { node_id, x, y }
}

/// Collect `%% @pos` hints from the full source and validate against parsed nodes.
fn collect_position_hints(
    source: &str,
    node_map: &HashMap<String, usize>,
    flowchart: &mut Flowchart,
) {
    let mut seen_nodes: HashSet<String> = HashSet::new();

    for (idx, line) in source.lines().enumerate() {
        let line_no = idx + 1;
        match parse_pos_hint_line(line) {
            PosHintParse::NotPosHint => {}
            PosHintParse::Malformed(message) => {
                flowchart.warnings.push(FlowchartWarning { line: line_no, message });
            }
            PosHintParse::Parsed { node_id, x, y } => {
                if !node_map.contains_key(&node_id) {
                    flowchart.warnings.push(FlowchartWarning {
                        line: line_no,
                        message: format!("Unknown node id '{node_id}' in @pos hint"),
                    });
                    continue;
                }
                if seen_nodes.contains(&node_id) {
                    flowchart.warnings.push(FlowchartWarning {
                        line: line_no,
                        message: format!("Duplicate @pos hint for node '{node_id}'"),
                    });
                    continue;
                }
                seen_nodes.insert(node_id.clone());
                flowchart
                    .position_hints
                    .insert(node_id, Pos2::new(x, y));
            }
        }
    }
}

/// Helper struct for building subgraphs during parsing.
struct SubgraphBuilder {
    id: String,
    title: Option<String>,
    node_ids: Vec<String>,
    child_subgraph_ids: Vec<String>,
    direction: Option<FlowDirection>,
}

/// Parse subgraph header line to extract id and title.
/// Supports: `subgraph title` and `subgraph id [title]`
fn parse_subgraph_header(line: &str, counter: &mut usize) -> (String, Option<String>) {
    let rest = line
        .trim_start_matches(|c: char| c.is_ascii_alphabetic())
        .trim_start(); // Remove "subgraph" and leading whitespace

    if rest.is_empty() {
        // No id or title, generate id
        *counter += 1;
        return (format!("subgraph_{}", counter), None);
    }

    // Check if rest contains brackets (explicit title)
    if let Some(bracket_start) = rest.find('[') {
        if let Some(bracket_end) = rest.rfind(']') {
            let id = rest[..bracket_start].trim().to_string();
            let title = rest[bracket_start + 1..bracket_end].trim().to_string();
            let id = if id.is_empty() {
                *counter += 1;
                format!("subgraph_{}", counter)
            } else {
                id
            };
            return (id, Some(title));
        }
    }

    // Check for quoted title
    if rest.starts_with('"') || rest.starts_with('\'') {
        let quote = rest.chars().next().unwrap();
        if let Some(end_quote) = rest[1..].find(quote) {
            let title = rest[1..end_quote + 1].to_string();
            *counter += 1;
            return (format!("subgraph_{}", counter), Some(title));
        }
    }

    // Check if first token looks like an ID (alphanumeric, no spaces)
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    if tokens.len() == 1 {
        let token = tokens[0].to_string();
        return (token.clone(), Some(token));
    } else if tokens.len() >= 2 {
        // First token is ID, rest is title
        let id = tokens[0].to_string();
        let title = tokens[1..].join(" ");
        return (id, Some(title));
    }

    // Fallback: generate ID, use rest as title
    *counter += 1;
    (format!("subgraph_{}", counter), Some(rest.to_string()))
}

pub(crate) fn parse_direction(header: &str) -> FlowDirection {
    // Strip trailing semicolon from header (e.g., "graph TD;")
    let header = strip_trailing_semicolon(header);
    let parts: Vec<&str> = header.split_whitespace().collect();
    if parts.len() > 1 {
        // Strip any trailing semicolon from the direction part too
        let direction = strip_trailing_semicolon(parts[1]);
        match direction.to_uppercase().as_str() {
            "TD" | "TB" => FlowDirection::TopDown,
            "BT" => FlowDirection::BottomUp,
            "LR" => FlowDirection::LeftRight,
            "RL" => FlowDirection::RightLeft,
            _ => FlowDirection::TopDown,
        }
    } else {
        FlowDirection::TopDown
    }
}

/// Parse `fill:…,stroke:…` style fragments used by `classDef` and `style nodeId`.
fn parse_node_style_properties(properties_str: &str) -> NodeStyle {
    let mut style = NodeStyle::default();

    for prop in properties_str.split(',') {
        let prop = prop.trim();
        if let Some(colon_pos) = prop.find(':') {
            let key = prop[..colon_pos].trim().to_lowercase();
            let value = prop[colon_pos + 1..].trim();

            match key.as_str() {
                "fill" => style.fill = parse_css_color(value),
                "stroke" => style.stroke = parse_css_color(value),
                "stroke-width" => style.stroke_width = parse_stroke_width(value),
                "color" => style.color = parse_css_color(value),
                _ => {}
            }
        }
    }

    style
}

/// `style nodeId fill:#fff,stroke:#000,stroke-width:2px,color:#333`
fn parse_style_directive(line: &str, node_styles: &mut HashMap<String, NodeStyle>) {
    let rest = if line.to_lowercase().starts_with("style ") {
        line[6..].trim()
    } else {
        return;
    };

    if rest.is_empty() {
        return;
    }

    let mut parts = rest.splitn(2, char::is_whitespace);
    let node_id = parts.next().unwrap_or("").trim();
    if node_id.is_empty() {
        return;
    }

    let properties_str = parts.next().unwrap_or("").trim();
    let patch = parse_node_style_properties(properties_str);

    node_styles
        .entry(node_id.to_string())
        .and_modify(|existing| existing.apply_overlay(&patch))
        .or_insert(patch);
}

/// Parse a classDef directive: `classDef className fill:#fff,stroke:#000,stroke-width:2px`
/// Returns (class_name, NodeStyle) on success.
fn parse_class_def(line: &str) -> Option<(String, NodeStyle)> {
    // Remove "classDef " prefix (case-insensitive)
    let rest = if line.to_lowercase().starts_with("classdef ") {
        &line[9..] // len("classdef ") = 9
    } else {
        return None;
    };

    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }

    // Split into class name and style properties
    let mut parts = rest.splitn(2, char::is_whitespace);
    let class_name = parts.next()?.trim().to_string();
    let properties_str = parts.next().unwrap_or("").trim();

    if class_name.is_empty() {
        return None;
    }

    Some((class_name, parse_node_style_properties(properties_str)))
}

/// Parse a class assignment directive: `class nodeId1,nodeId2 className`
/// or inline syntax: `class nodeId className`
fn parse_class_assignment(line: &str, node_classes: &mut HashMap<String, String>) {
    // Remove "class " prefix (case-insensitive)
    let rest = if line.to_lowercase().starts_with("class ") {
        &line[6..] // len("class ") = 6
    } else {
        return;
    };

    let rest = rest.trim();
    if rest.is_empty() {
        return;
    }

    // Split into node IDs and class name
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    if tokens.len() < 2 {
        return;
    }

    let class_name = tokens.last().unwrap().trim().to_string();

    // Everything before the class name is node IDs (comma-separated)
    let node_ids_str = tokens[..tokens.len() - 1].join(" ");

    // Parse node IDs (can be comma-separated: "A,B,C" or "A, B, C")
    for node_id in node_ids_str.split(',') {
        let node_id = node_id.trim();
        if !node_id.is_empty() {
            node_classes.insert(node_id.to_string(), class_name.clone());
        }
    }
}

/// Parse a CSS color value (hex format).
/// Supports: #RGB, #RRGGBB, #RRGGBBAA
fn parse_css_color(value: &str) -> Option<Color32> {
    let value = value.trim();

    if !value.starts_with('#') {
        return None;
    }

    let hex = &value[1..];

    match hex.len() {
        // #RGB -> #RRGGBB
        3 => {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            Some(Color32::from_rgb(r, g, b))
        }
        // #RRGGBB
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Color32::from_rgb(r, g, b))
        }
        // #RRGGBBAA
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
            Some(Color32::from_rgba_unmultiplied(r, g, b, a))
        }
        _ => None,
    }
}

/// Parse stroke-width value, e.g., "2px", "1.5px", "2"
fn parse_stroke_width(value: &str) -> Option<f32> {
    let value = value.trim();
    let num_str = value.strip_suffix("px").unwrap_or(value);
    num_str.parse::<f32>().ok()
}

/// Parse a linkStyle directive and update the flowchart.
fn parse_link_style(line: &str, flowchart: &mut Flowchart) {
    let content = if line.len() > 10 { &line[10..] } else { return };
    let content = content.trim();

    let (index_part, css_part) = match content.find(char::is_whitespace) {
        Some(pos) => {
            let (idx, css) = content.split_at(pos);
            (idx.trim(), css.trim())
        }
        None => return,
    };

    let mut style = LinkStyle::default();
    for property in css_part.split(',') {
        let property = property.trim();
        if let Some((key, value)) = property.split_once(':') {
            let key = key.trim().to_lowercase();
            let value = value.trim();

            match key.as_str() {
                "stroke" => {
                    style.stroke = parse_css_color(value);
                }
                "stroke-width" => {
                    style.stroke_width = parse_stroke_width(value);
                }
                _ => {}
            }
        }
    }

    let tokens: Vec<&str> = css_part
        .split(|c: char| c == ',' || c.is_whitespace())
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .collect();
    for window in tokens.windows(2) {
        if window[0].eq_ignore_ascii_case("interpolate") && window[1].eq_ignore_ascii_case("basis")
        {
            style.interpolate_basis = true;
            break;
        }
    }

    if index_part.eq_ignore_ascii_case("default") {
        flowchart.default_link_style = Some(style);
    } else if let Ok(index) = index_part.parse::<usize>() {
        flowchart.link_styles.insert(index, style);
    }
}

/// Arrow pattern definition for parsing edges.
/// Ordered by length (longest first) to ensure correct matching.
const ARROW_PATTERNS: &[(&str, EdgeStyle, ArrowHead, ArrowHead)] = &[
    // 4+ char patterns first
    ("<-->", EdgeStyle::Solid, ArrowHead::Arrow, ArrowHead::Arrow),
    (
        "o--o",
        EdgeStyle::Solid,
        ArrowHead::Circle,
        ArrowHead::Circle,
    ),
    ("x--x", EdgeStyle::Solid, ArrowHead::Cross, ArrowHead::Cross),
    ("--->", EdgeStyle::Solid, ArrowHead::None, ArrowHead::Arrow),
    ("-.->", EdgeStyle::Dotted, ArrowHead::None, ArrowHead::Arrow),
    // 3 char patterns
    ("-->", EdgeStyle::Solid, ArrowHead::None, ArrowHead::Arrow),
    ("---", EdgeStyle::Solid, ArrowHead::None, ArrowHead::None),
    ("-.-", EdgeStyle::Dotted, ArrowHead::None, ArrowHead::None),
    ("==>", EdgeStyle::Thick, ArrowHead::None, ArrowHead::Arrow),
    ("===", EdgeStyle::Thick, ArrowHead::None, ArrowHead::None),
    ("--o", EdgeStyle::Solid, ArrowHead::None, ArrowHead::Circle),
    ("--x", EdgeStyle::Solid, ArrowHead::None, ArrowHead::Cross),
];

/// Find the first arrow pattern in the given text, returning its position, length, and style info.
fn find_arrow_pattern(
    text: &str,
) -> Option<(usize, &'static str, EdgeStyle, ArrowHead, ArrowHead)> {
    let mut best_match: Option<(usize, &'static str, EdgeStyle, ArrowHead, ArrowHead)> = None;

    for &(pattern, style, arrow_start, arrow_end) in ARROW_PATTERNS {
        if let Some(pos) = text.find(pattern) {
            let dominated = best_match.map_or(false, |(best_pos, best_pat, _, _, _)| {
                pos > best_pos || (pos == best_pos && pattern.len() <= best_pat.len())
            });
            if !dominated {
                best_match = Some((pos, pattern, style, arrow_start, arrow_end));
            }
        }
    }

    best_match
}

/// Parse an edge segment: extracts the label (if any) after the arrow and returns the remaining text.
fn parse_edge_label(text: &str) -> (Option<String>, &str) {
    let text = text.trim();

    // Check for label syntax: |label|
    if text.starts_with('|') {
        if let Some(end_pos) = text[1..].find('|') {
            let label = text[1..=end_pos].trim();
            let rest = text[end_pos + 2..].trim();
            return (Some(clean_label(label)), rest);
        }
    }

    (None, text)
}

/// Extract dash-style edge label from node text.
fn extract_dash_label(node_text: &str) -> (&str, Option<String>) {
    let text = node_text.trim();

    let label_start_patterns = ["-- ", "-. ", "== "];

    let shape_closers = [']', ')', '}', '|'];
    let last_closer_pos = shape_closers.iter().filter_map(|&c| text.rfind(c)).max();

    if let Some(closer_pos) = last_closer_pos {
        let after_closer = &text[closer_pos + 1..];

        for pattern in &label_start_patterns {
            if after_closer.starts_with(pattern) {
                let label = after_closer[pattern.len()..].trim();
                let node_part = &text[..=closer_pos];
                log::trace!(
                    "extract_dash_label: found dash label, node='{}', label='{}'",
                    node_part,
                    label
                );
                return (node_part, Some(clean_label(label)));
            }
        }

        for pattern_start in ["--", "-.", "=="] {
            if after_closer.starts_with(pattern_start) {
                let rest = &after_closer[pattern_start.len()..];
                if rest.is_empty() || rest.starts_with(char::is_whitespace) {
                    let label = rest.trim();
                    let node_part = &text[..=closer_pos];
                    log::trace!(
                        "extract_dash_label: found dash label (variant), node='{}', label='{}'",
                        node_part,
                        label
                    );
                    return (node_part, Some(clean_label(label)));
                }
            }
        }
    }

    (text, None)
}

/// Strip trailing semicolon from a string.
fn strip_trailing_semicolon(s: &str) -> &str {
    s.strip_suffix(';').unwrap_or(s).trim_end()
}

/// Split node text by ampersand, handling the `A & B` syntax.
fn split_by_ampersand(text: &str) -> Vec<&str> {
    let has_shape_marker =
        text.contains('[') || text.contains('(') || text.contains('{') || text.contains('>');

    if has_shape_marker {
        if let Some(amp_pos) = text.find('&') {
            let first_marker = [
                text.find('['),
                text.find('('),
                text.find('{'),
                text.find('>'),
            ]
            .into_iter()
            .flatten()
            .min();

            if let Some(marker_pos) = first_marker {
                if amp_pos < marker_pos {
                    let ids_part = &text[..marker_pos];
                    let _shape_part = &text[marker_pos..];

                    let ids: Vec<&str> = ids_part.split('&').map(|s| s.trim()).collect();
                    if ids.len() > 1 {
                        return ids;
                    }
                }
            }
        }
        return vec![text];
    }

    if text.contains('&') {
        text.split('&')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        vec![text]
    }
}

/// Parse a line that may contain chained edges, returning all nodes and all edges.
pub(crate) fn parse_edge_line_full(
    line: &str,
) -> Option<(Vec<(String, String, NodeShape)>, Vec<FlowEdge>)> {
    let line = strip_trailing_semicolon(line.trim());

    if find_arrow_pattern(line).is_none() {
        return None;
    }

    let mut all_nodes: Vec<(String, String, NodeShape)> = Vec::new();
    let mut all_edges: Vec<FlowEdge> = Vec::new();
    let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    let mut prev_node_ids: Vec<String> = Vec::new();
    let mut remaining = line;

    while !remaining.is_empty() {
        if let Some((arrow_pos, pattern, style, arrow_start, arrow_end)) =
            find_arrow_pattern(remaining)
        {
            let raw_node_text = remaining[..arrow_pos].trim();
            let after_arrow = &remaining[arrow_pos + pattern.len()..];
            let (pipe_label, rest_after_label) = parse_edge_label(after_arrow);

            let (node_text, dash_label) = extract_dash_label(raw_node_text);

            let label = pipe_label.or(dash_label);

            let from_ids: Vec<String> = if !node_text.is_empty() {
                let node_parts = split_by_ampersand(node_text);
                let mut ids = Vec::new();
                for part in node_parts {
                    if let Some((id, node_label, shape)) = parse_node_from_text(part) {
                        if !seen_ids.contains(&id) {
                            seen_ids.insert(id.clone());
                            all_nodes.push((id.clone(), node_label, shape));
                        }
                        ids.push(id);
                    }
                }
                ids
            } else {
                prev_node_ids.clone()
            };

            let next_segment = rest_after_label.trim();

            let target_end = find_arrow_pattern(next_segment)
                .map(|(pos, _, _, _, _)| pos)
                .unwrap_or(next_segment.len());

            let target_text = strip_trailing_semicolon(next_segment[..target_end].trim());

            let target_parts = split_by_ampersand(target_text);
            let mut to_ids: Vec<String> = Vec::new();

            for part in target_parts {
                if let Some((to_id, to_label, to_shape)) = parse_node_from_text(part) {
                    if !seen_ids.contains(&to_id) {
                        seen_ids.insert(to_id.clone());
                        all_nodes.push((to_id.clone(), to_label, to_shape));
                    }
                    to_ids.push(to_id);
                }
            }

            for from in &from_ids {
                for to in &to_ids {
                    all_edges.push(FlowEdge {
                        from: from.clone(),
                        to: to.clone(),
                        label: label.clone(),
                        style,
                        arrow_start,
                        arrow_end,
                    });
                }
            }

            prev_node_ids = to_ids;

            remaining = &next_segment[target_end..];
        } else {
            break;
        }
    }

    if all_nodes.is_empty() {
        return None;
    }

    Some((all_nodes, all_edges))
}

pub(crate) fn parse_node_from_text(text: &str) -> Option<(String, String, NodeShape)> {
    let text = strip_trailing_semicolon(text.trim());
    if text.is_empty() {
        return None;
    }

    log::trace!("parse_node_from_text: input='{}'", text);

    // Stadium: ([text])
    if text.contains("([") && text.contains("])") {
        if let Some(start) = text.find("([") {
            let id = text[..start].trim();
            let id = if id.is_empty() {
                &text[..start.max(1)]
            } else {
                id
            };
            if let Some(end) = text.find("])") {
                let label = text[start + 2..end].trim();
                return Some((extract_id(id, text), clean_label(label), NodeShape::Stadium));
            }
        }
    }

    // Double circle: (((text)))
    if text.contains("(((") {
        if let Some(start) = text.find("(((") {
            let id = text[..start].trim();
            if let Some(rel) = text[start..].find(")))") {
                let end = start + rel;
                let label = text[start + 3..end].trim();
                return Some((
                    extract_id(id, text),
                    clean_label(label),
                    NodeShape::DoubleCircle,
                ));
            }
        }
    }

    // Circle: ((text))
    if text.contains("((") && text.contains("))") {
        if let Some(start) = text.find("((") {
            let id = text[..start].trim();
            if let Some(end) = text.find("))") {
                let label = text[start + 2..end].trim();
                return Some((extract_id(id, text), clean_label(label), NodeShape::Circle));
            }
        }
    }

    // Cylinder: [(text)]
    if text.contains("[(") && text.contains(")]") {
        if let Some(start) = text.find("[(") {
            let id = text[..start].trim();
            if let Some(end) = text.find(")]") {
                let label = text[start + 2..end].trim();
                return Some((
                    extract_id(id, text),
                    clean_label(label),
                    NodeShape::Cylinder,
                ));
            }
        }
    }

    // Subroutine: [[text]]
    if text.contains("[[") && text.contains("]]") {
        if let Some(start) = text.find("[[") {
            let id = text[..start].trim();
            if let Some(end) = text.find("]]") {
                let label = text[start + 2..end].trim();
                return Some((
                    extract_id(id, text),
                    clean_label(label),
                    NodeShape::Subroutine,
                ));
            }
        }
    }

    // Hexagon: {{text}}
    if text.contains("{{") && text.contains("}}") {
        if let Some(start) = text.find("{{") {
            let id = text[..start].trim();
            if let Some(end) = text.find("}}") {
                let label = text[start + 2..end].trim();
                return Some((extract_id(id, text), clean_label(label), NodeShape::Hexagon));
            }
        }
    }

    // Diamond: {text}
    if text.contains('{') && text.contains('}') && !text.contains("{{") {
        if let Some(start) = text.find('{') {
            let id = text[..start].trim();
            if let Some(end) = text.rfind('}') {
                let label = text[start + 1..end].trim();
                return Some((extract_id(id, text), clean_label(label), NodeShape::Diamond));
            }
        }
    }

    // Inverted trapezoid: [\text/]
    if let Some(start) = text.find("[\\") {
        let id = text[..start].trim();
        let after_open = &text[start + 2..];
        if let Some(rel) = after_open.find("/]") {
            let label = after_open[..rel].trim();
            return Some((
                extract_id(id, text),
                clean_label(label),
                NodeShape::TrapezoidInv,
            ));
        }
    }

    // Trapezoid [/text\] or parallelogram [/text/]
    if let Some(start) = text.find("[/") {
        let id = text[..start].trim();
        let after_open = &text[start + 2..];
        if let Some(rel) = after_open.find("\\]") {
            let label = after_open[..rel].trim();
            return Some((
                extract_id(id, text),
                clean_label(label),
                NodeShape::Trapezoid,
            ));
        }
        if let Some(rel) = after_open.rfind("/]") {
            let label = after_open[..rel].trim();
            return Some((
                extract_id(id, text),
                clean_label(label),
                NodeShape::Parallelogram,
            ));
        }
    }

    // Round rect: (text)
    if text.contains('(')
        && text.contains(')')
        && !text.contains("((")
        && !text.contains("([")
        && !text.contains("[(")
    {
        if let Some(start) = text.find('(') {
            let id = text[..start].trim();
            if let Some(end) = text.rfind(')') {
                let label = text[start + 1..end].trim();
                return Some((
                    extract_id(id, text),
                    clean_label(label),
                    NodeShape::RoundRect,
                ));
            }
        }
    }

    // Rectangle: [text]
    if text.contains('[')
        && text.contains(']')
        && !text.contains("[[")
        && !text.contains("[(")
        && !text.contains("([")
        && !text.contains("[/")
        && !text.contains("[\\")
    {
        if let Some(start) = text.find('[') {
            let id = text[..start].trim();
            if let Some(end) = text.rfind(']') {
                let label = text[start + 1..end].trim();
                return Some((
                    extract_id(id, text),
                    clean_label(label),
                    NodeShape::Rectangle,
                ));
            }
        }
    }

    // Asymmetric: >text]
    if text.contains('>') && text.contains(']') {
        if let Some(start) = text.find('>') {
            if let Some(end) = text.rfind(']') {
                if start < end {
                    let id = text[..start].trim();
                    let label = text[start + 1..end].trim();
                    log::debug!(
                        "Asymmetric shape detected: id='{}', label='{}', text='{}'",
                        id,
                        label,
                        text
                    );
                    return Some((
                        extract_id(id, text),
                        clean_label(label),
                        NodeShape::Asymmetric,
                    ));
                }
            }
        }
    }

    // Just an ID (no shape specified)
    let id = strip_trailing_semicolon(text.split_whitespace().next().unwrap_or(text));
    log::trace!(
        "parse_node_from_text: no shape matched, defaulting to Rectangle for id='{}', text='{}'",
        id,
        text
    );
    Some((id.to_string(), id.to_string(), NodeShape::Rectangle))
}

fn extract_id(id: &str, full_text: &str) -> String {
    if id.is_empty() {
        full_text
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect()
    } else {
        id.to_string()
    }
}

/// Clean up label text by converting HTML line breaks to newlines and stripping
/// Font Awesome icon prefixes (`fa:fa-*`, `fab:fa-*`) from the start of the label.
fn clean_label(label: &str) -> String {
    let with_breaks = label
        .replace("<br/>", "\n")
        .replace("<br>", "\n")
        .replace("<br />", "\n");
    strip_font_awesome_icon_prefixes(&with_breaks)
}

/// Strip leading `fa:fa-<name>` / `fab:fa-<name>` tokens (Mermaid inline icon syntax).
/// Only prefixes at the start of the label are removed; mid-label tokens are kept.
fn strip_font_awesome_icon_prefixes(label: &str) -> String {
    let mut text = label.to_string();
    loop {
        let trimmed = text.trim_start();
        let Some(rest) = strip_one_font_awesome_icon_prefix(trimmed) else {
            break;
        };
        text = rest.trim_start().to_string();
    }
    text
}

/// If `text` begins with `fa:fa-` or `fab:fa-` followed by an icon name, return the remainder.
fn strip_one_font_awesome_icon_prefix(text: &str) -> Option<&str> {
    const PREFIXES: [&str; 2] = ["fa:fa-", "fab:fa-"];
    for prefix in PREFIXES {
        if let Some(after_prefix) = text.strip_prefix(prefix) {
            let icon_len = after_prefix
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
                .count();
            if icon_len > 0 {
                return Some(&after_prefix[icon_len..]);
            }
        }
    }
    None
}

fn parse_node_definition(line: &str) -> Option<FlowNode> {
    parse_node_from_text(line).map(|(id, label, shape)| FlowNode { id, label, shape })
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::pos2;

    #[test]
    fn pos_hint_valid_stored_in_map() {
        let source = r#"flowchart TD
%% @pos A 100 50
A --> B
%% @pos B 200 150
"#;
        let flowchart = parse_flowchart(source).unwrap();
        assert!(flowchart.warnings.is_empty());
        assert_eq!(flowchart.position_hints.len(), 2);
        assert_eq!(flowchart.position_hints.get("A"), Some(&pos2(100.0, 50.0)));
        assert_eq!(flowchart.position_hints.get("B"), Some(&pos2(200.0, 150.0)));
    }

    #[test]
    fn pos_hint_unknown_node_id_warns() {
        let source = r#"flowchart TD
%% @pos Missing 10 20
A --> B
"#;
        let flowchart = parse_flowchart(source).unwrap();
        assert_eq!(flowchart.position_hints.len(), 0);
        assert_eq!(flowchart.warnings.len(), 1);
        assert_eq!(flowchart.warnings[0].line, 2);
        assert!(flowchart
            .warnings[0]
            .message
            .contains("Unknown node id 'Missing'"));
    }

    #[test]
    fn pos_hint_malformed_coords_warns() {
        let source = r#"flowchart TD
A --> B
%% @pos A foo 20
"#;
        let flowchart = parse_flowchart(source).unwrap();
        assert_eq!(flowchart.position_hints.len(), 0);
        assert_eq!(flowchart.warnings.len(), 1);
        assert!(flowchart.warnings[0].message.contains("Invalid x coordinate"));
    }

    #[test]
    fn pos_hint_malformed_token_count_warns() {
        let source = r#"flowchart TD
A --> B
%% @pos A 10
"#;
        let flowchart = parse_flowchart(source).unwrap();
        assert_eq!(flowchart.position_hints.len(), 0);
        assert_eq!(flowchart.warnings.len(), 1);
        assert!(flowchart
            .warnings[0]
            .message
            .contains("Malformed @pos hint"));
    }

    #[test]
    fn pos_hint_duplicate_warns_and_keeps_first() {
        let source = r#"flowchart TD
%% @pos A 10 20
A --> B
%% @pos A 99 99
"#;
        let flowchart = parse_flowchart(source).unwrap();
        assert_eq!(flowchart.position_hints.len(), 1);
        assert_eq!(flowchart.position_hints.get("A"), Some(&pos2(10.0, 20.0)));
        assert_eq!(flowchart.warnings.len(), 1);
        assert_eq!(flowchart.warnings[0].line, 4);
        assert!(flowchart
            .warnings[0]
            .message
            .contains("Duplicate @pos hint"));
    }

    #[test]
    fn pos_hint_ignores_regular_comments() {
        let source = r#"flowchart TD
%% just a note
A --> B
"#;
        let flowchart = parse_flowchart(source).unwrap();
        assert!(flowchart.position_hints.is_empty());
        assert!(flowchart.warnings.is_empty());
    }

    #[test]
    fn clean_label_strips_fa_prefix() {
        assert_eq!(clean_label("fa:fa-car Car"), "Car");
    }

    #[test]
    fn clean_label_strips_fab_prefix() {
        assert_eq!(clean_label("fab:fa-github GitHub"), "GitHub");
    }

    #[test]
    fn clean_label_does_not_strip_mid_label_fa_prefix() {
        assert_eq!(clean_label("Hello fa:fa-car world"), "Hello fa:fa-car world");
    }

    #[test]
    fn clean_label_without_fa_prefix_unchanged() {
        assert_eq!(clean_label("Plain label"), "Plain label");
    }

    #[test]
    fn clean_label_strips_multiple_leading_fa_prefixes() {
        assert_eq!(
            clean_label("fa:fa-box fa:fa-arrow-right Create an order"),
            "Create an order"
        );
    }

    #[test]
    fn clean_label_strips_fa_prefix_from_edge_label() {
        let source = r#"flowchart TD
    A -->|fa:fa-car Car| B
"#;
        let flowchart = parse_flowchart(source).unwrap();
        assert_eq!(flowchart.edges[0].label.as_deref(), Some("Car"));
    }

    #[test]
    fn clean_label_strips_fa_prefix_from_fc_83b_node() {
        let source = r#"flowchart TD
    C -->|Three| F[fa:fa-car Car]
"#;
        let flowchart = parse_flowchart(source).unwrap();
        let node_f = flowchart
            .nodes
            .iter()
            .find(|n| n.id == "F")
            .expect("node F");
        assert_eq!(node_f.label, "Car");
        let edge = flowchart
            .edges
            .iter()
            .find(|e| e.to == "F")
            .expect("edge to F");
        assert_eq!(edge.label.as_deref(), Some("Three"));
    }

    #[test]
    fn link_style_default_interpolate_basis_parsed() {
        let source = r#"graph TD
    linkStyle default interpolate basis
    A --> B
"#;
        let flowchart = parse_flowchart(source).unwrap();
        assert!(flowchart.warnings.is_empty());
        let style = flowchart
            .default_link_style
            .as_ref()
            .expect("default link style");
        assert!(style.interpolate_basis);
    }

    #[test]
    fn link_style_index_interpolate_basis_parsed() {
        let source = r#"flowchart TD
    A --> B --> C
    linkStyle 1 interpolate basis, stroke:#f00
"#;
        let flowchart = parse_flowchart(source).unwrap();
        let style = flowchart.link_styles.get(&1).expect("edge 1 style");
        assert!(style.interpolate_basis);
        assert!(style.stroke.is_some());
    }
}
