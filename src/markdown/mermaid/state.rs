//! State diagram parsing and rendering.

use egui::{Color32, FontId, Pos2, Rect, Rounding, Stroke, Ui, Vec2};
use std::collections::HashMap;

use super::text::{EguiTextMeasurer, TextMeasurer};

/// A state in a state diagram, supporting composite/nested states.
#[derive(Debug, Clone)]
pub struct State {
    pub id: String,
    pub label: String,
    pub is_start: bool,
    pub is_end: bool,
    /// Child states for composite states (empty for simple states)
    pub children: Vec<State>,
    /// Internal transitions within this composite state
    pub internal_transitions: Vec<Transition>,
    /// Parent state ID if this is a nested state
    pub parent: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            id: String::new(),
            label: String::new(),
            is_start: false,
            is_end: false,
            children: Vec::new(),
            internal_transitions: Vec::new(),
            parent: None,
        }
    }
}

impl State {
    /// Create a new simple state
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        let id = id.into();
        let label = label.into();
        Self {
            id,
            label,
            ..Default::default()
        }
    }

    /// Check if this is a composite (has children)
    pub fn is_composite(&self) -> bool {
        !self.children.is_empty()
    }
}

/// The kind of transition based on hierarchy relationship.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionKind {
    /// Both source and target are in the same scope
    Internal,
    /// Transition enters a composite state from outside
    Enter,
    /// Transition exits a composite state to outside
    Exit,
    /// Transition crosses between different branches of the hierarchy
    CrossHierarchy,
}

impl Default for TransitionKind {
    fn default() -> Self {
        Self::Internal
    }
}

/// A transition between states.
#[derive(Debug, Clone)]
pub struct Transition {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    /// The kind of transition (computed during normalization)
    pub kind: TransitionKind,
    /// Source state's enclosing composite (None if top-level)
    pub source_scope: Option<String>,
    /// Target state's enclosing composite (None if top-level)
    pub target_scope: Option<String>,
    /// Lowest common ancestor composite for source and target
    pub lca_scope: Option<String>,
}

impl Transition {
    /// Create a new transition with default kind
    pub fn new(from: String, to: String, label: Option<String>) -> Self {
        Self {
            from,
            to,
            label,
            kind: TransitionKind::Internal,
            source_scope: None,
            target_scope: None,
            lca_scope: None,
        }
    }
}

/// Configuration options for state diagram layout.
#[derive(Debug, Clone)]
pub struct StateDiagramConfig {
    pub composite_padding: f32,
    pub header_height: f32,
    pub child_spacing_x: f32,
    pub child_spacing_y: f32,
    pub min_state_width: f32,
    pub state_height: f32,
    pub spacing_x: f32,
    pub spacing_y: f32,
    pub margin: f32,
    pub orthogonal_cross_routing: bool,
    pub prefer_horizontal_anchors: bool,
}

impl Default for StateDiagramConfig {
    fn default() -> Self {
        Self {
            composite_padding: 20.0,
            header_height: 28.0,
            child_spacing_x: 80.0,
            child_spacing_y: 56.0,
            min_state_width: 80.0,
            state_height: 36.0,
            spacing_x: 100.0,
            spacing_y: 70.0,
            margin: 40.0,
            orthogonal_cross_routing: true,
            prefer_horizontal_anchors: true,
        }
    }
}

/// A parsed state diagram.
#[derive(Debug, Clone, Default)]
pub struct StateDiagram {
    pub states: Vec<State>,
    pub transitions: Vec<Transition>,
    pub config: StateDiagramConfig,
}

/// Parse mermaid state diagram source.
pub fn parse_state_diagram(source: &str) -> Result<StateDiagram, String> {
    let lines: Vec<&str> = source.lines().skip(1).collect();
    let mut diagram = StateDiagram::default();
    let mut idx = 0;

    parse_state_diagram_block(
        &lines,
        &mut idx,
        &mut diagram.states,
        &mut diagram.transitions,
        None,
    )?;

    if diagram.states.is_empty() {
        return Err("No states found in state diagram".to_string());
    }

    normalize_state_diagram(&mut diagram);

    Ok(diagram)
}

fn normalize_state_diagram(diagram: &mut StateDiagram) {
    let mut ancestry_map: HashMap<String, Vec<String>> = HashMap::new();
    build_ancestry_map(&diagram.states, &[], &mut ancestry_map);

    for transition in &mut diagram.transitions {
        normalize_transition(transition, &ancestry_map);
    }

    normalize_internal_transitions(&mut diagram.states, &ancestry_map);
}

fn build_ancestry_map(
    states: &[State],
    current_ancestry: &[String],
    ancestry_map: &mut HashMap<String, Vec<String>>,
) {
    for state in states {
        ancestry_map.insert(state.id.clone(), current_ancestry.to_vec());

        if state.is_composite() {
            let mut child_ancestry = current_ancestry.to_vec();
            child_ancestry.push(state.id.clone());
            build_ancestry_map(&state.children, &child_ancestry, ancestry_map);
        }
    }
}

fn normalize_internal_transitions(
    states: &mut [State],
    ancestry_map: &HashMap<String, Vec<String>>,
) {
    for state in states {
        for transition in &mut state.internal_transitions {
            normalize_transition(transition, ancestry_map);
        }
        normalize_internal_transitions(&mut state.children, ancestry_map);
    }
}

fn normalize_transition(transition: &mut Transition, ancestry_map: &HashMap<String, Vec<String>>) {
    let source_ancestry = ancestry_map
        .get(&transition.from)
        .cloned()
        .unwrap_or_default();
    let target_ancestry = ancestry_map
        .get(&transition.to)
        .cloned()
        .unwrap_or_default();

    transition.source_scope = source_ancestry.last().cloned();
    transition.target_scope = target_ancestry.last().cloned();
    transition.lca_scope = find_lca(&source_ancestry, &target_ancestry);

    transition.kind = if transition.source_scope == transition.target_scope {
        TransitionKind::Internal
    } else if source_ancestry.is_empty() && !target_ancestry.is_empty() {
        TransitionKind::Enter
    } else if !source_ancestry.is_empty() && target_ancestry.is_empty() {
        TransitionKind::Exit
    } else if is_ancestor_of(&source_ancestry, &transition.to) {
        TransitionKind::Enter
    } else if is_ancestor_of(&target_ancestry, &transition.from) {
        TransitionKind::Exit
    } else {
        TransitionKind::CrossHierarchy
    };
}

fn find_lca(ancestry_a: &[String], ancestry_b: &[String]) -> Option<String> {
    let mut lca = None;
    for (a, b) in ancestry_a.iter().zip(ancestry_b.iter()) {
        if a == b {
            lca = Some(a.clone());
        } else {
            break;
        }
    }
    lca
}

fn is_ancestor_of(ancestry: &[String], state_id: &str) -> bool {
    ancestry.iter().any(|a| a == state_id)
}

fn parse_state_diagram_block(
    lines: &[&str],
    idx: &mut usize,
    states: &mut Vec<State>,
    transitions: &mut Vec<Transition>,
    parent_id: Option<&str>,
) -> Result<(), String> {
    let mut state_map: HashMap<String, usize> = HashMap::new();

    for (i, state) in states.iter().enumerate() {
        state_map.insert(state.id.clone(), i);
    }

    while *idx < lines.len() {
        let line = lines[*idx].trim();
        *idx += 1;

        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        if line == "}" {
            return Ok(());
        }

        // Parse composite state
        if line.starts_with("state ") && line.ends_with('{') {
            let rest = line[6..].trim_end_matches('{').trim();

            let (state_id, state_label) = if let Some(as_pos) = rest.find(" as ") {
                let label = rest[..as_pos].trim().trim_matches('"').to_string();
                let id = rest[as_pos + 4..].trim().to_string();
                (id, label)
            } else {
                let id = rest.trim_matches('"').to_string();
                (id.clone(), id)
            };

            let mut composite = State {
                id: state_id.clone(),
                label: state_label,
                parent: parent_id.map(|s| s.to_string()),
                ..Default::default()
            };

            parse_state_diagram_block(
                lines,
                idx,
                &mut composite.children,
                &mut composite.internal_transitions,
                Some(&state_id),
            )?;

            for child in &mut composite.children {
                child.parent = Some(state_id.clone());
            }

            let state_idx = states.len();
            state_map.insert(state_id.clone(), state_idx);
            states.push(composite);
            continue;
        }

        // Parse transitions
        if line.contains("-->") {
            if let Some(arrow_pos) = line.find("-->") {
                let from_part = line[..arrow_pos].trim();
                let rest = &line[arrow_pos + 3..];

                let (to_part, label) = if let Some(colon_pos) = rest.find(':') {
                    (
                        rest[..colon_pos].trim(),
                        Some(rest[colon_pos + 1..].trim().to_string()),
                    )
                } else {
                    (rest.trim(), None)
                };

                let from_id = if from_part == "[*]" {
                    "__start__".to_string()
                } else {
                    from_part.to_string()
                };
                let to_id = if to_part == "[*]" {
                    "__end__".to_string()
                } else {
                    to_part.to_string()
                };

                for (id, is_start, is_end) in [
                    (&from_id, from_part == "[*]", false),
                    (&to_id, false, to_part == "[*]"),
                ] {
                    if !state_map.contains_key(id) {
                        let state_label = if is_start {
                            "●".to_string()
                        } else if is_end {
                            "◉".to_string()
                        } else {
                            id.clone()
                        };
                        let state_idx = states.len();
                        state_map.insert(id.clone(), state_idx);
                        states.push(State {
                            id: id.clone(),
                            label: state_label,
                            is_start,
                            is_end,
                            parent: parent_id.map(|s| s.to_string()),
                            ..Default::default()
                        });
                    }
                }

                transitions.push(Transition::new(from_id, to_id, label));
            }
            continue;
        }

        // Parse simple state definition
        if line.starts_with("state ") {
            let rest = line[6..].trim();
            if let Some(as_pos) = rest.find(" as ") {
                let label = rest[..as_pos].trim().trim_matches('"').to_string();
                let id = rest[as_pos + 4..].trim().to_string();
                if !state_map.contains_key(&id) {
                    let state_idx = states.len();
                    state_map.insert(id.clone(), state_idx);
                    states.push(State {
                        id: id.clone(),
                        label,
                        parent: parent_id.map(|s| s.to_string()),
                        ..Default::default()
                    });
                }
            } else {
                let id = rest.trim_matches('"').to_string();
                if !state_map.contains_key(&id) {
                    let state_idx = states.len();
                    state_map.insert(id.clone(), state_idx);
                    states.push(State {
                        id: id.clone(),
                        label: id.clone(),
                        parent: parent_id.map(|s| s.to_string()),
                        ..Default::default()
                    });
                }
            }
        }
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// State Diagram Renderer
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct StateLayout {
    center: Pos2,
    size: Vec2,
    bounds: Rect,
    is_composite: bool,
}

struct StateDiagramColors {
    state_fill: Color32,
    state_stroke: Color32,
    composite_fill: Color32,
    composite_title_bg: Color32,
    text_color: Color32,
    arrow_color: Color32,
    cross_arrow_color: Color32,
    boundary_arrow_color: Color32,
    start_color: Color32,
    label_bg: Color32,
}

impl StateDiagramColors {
    fn new(dark_mode: bool) -> Self {
        if dark_mode {
            Self {
                state_fill: Color32::from_rgb(45, 55, 72),
                state_stroke: Color32::from_rgb(100, 140, 180),
                composite_fill: Color32::from_rgba_unmultiplied(40, 50, 65, 200),
                composite_title_bg: Color32::from_rgb(55, 70, 90),
                text_color: Color32::from_rgb(220, 230, 240),
                arrow_color: Color32::from_rgb(120, 150, 180),
                cross_arrow_color: Color32::from_rgb(180, 120, 150),
                boundary_arrow_color: Color32::from_rgb(150, 180, 120),
                start_color: Color32::from_rgb(80, 180, 120),
                label_bg: Color32::from_rgb(35, 40, 50),
            }
        } else {
            Self {
                state_fill: Color32::from_rgb(240, 245, 250),
                state_stroke: Color32::from_rgb(100, 140, 180),
                composite_fill: Color32::from_rgba_unmultiplied(235, 240, 250, 220),
                composite_title_bg: Color32::from_rgb(220, 230, 245),
                text_color: Color32::from_rgb(30, 40, 50),
                arrow_color: Color32::from_rgb(100, 130, 160),
                cross_arrow_color: Color32::from_rgb(160, 100, 130),
                boundary_arrow_color: Color32::from_rgb(100, 140, 100),
                start_color: Color32::from_rgb(50, 150, 80),
                label_bg: Color32::from_rgb(255, 255, 255),
            }
        }
    }

    fn arrow_color_for_kind(&self, kind: TransitionKind) -> Color32 {
        match kind {
            TransitionKind::Internal => self.arrow_color,
            TransitionKind::Enter | TransitionKind::Exit => self.boundary_arrow_color,
            TransitionKind::CrossHierarchy => self.cross_arrow_color,
        }
    }
}

/// Render a state diagram to the UI.
pub fn render_state_diagram(ui: &mut Ui, diagram: &StateDiagram, dark_mode: bool, font_size: f32) {
    if diagram.states.is_empty() {
        return;
    }

    let colors = StateDiagramColors::new(dark_mode);
    let config = &diagram.config;
    let label_font_size = font_size - 2.0;
    let state_padding = Vec2::new(24.0, 12.0);
    let min_state_width = config.min_state_width;
    let state_height = config.state_height;
    let spacing_x = config.spacing_x;
    let spacing_y = config.spacing_y;
    let margin = config.margin;
    let composite_padding = config.composite_padding;
    let title_bar_height = config.header_height;

    fn collect_all_states(states: &[State]) -> Vec<&State> {
        let mut result = Vec::new();
        for state in states {
            result.push(state);
            result.extend(collect_all_states(&state.children));
        }
        result
    }

    let all_states = collect_all_states(&diagram.states);

    let state_widths: HashMap<String, f32> = {
        let text_measurer = EguiTextMeasurer::new(ui);
        all_states
            .iter()
            .filter(|s| !s.is_start && !s.is_end)
            .map(|state| {
                let text_size = text_measurer.measure(&state.label, font_size);
                let width = (text_size.width * 1.15 + state_padding.x).max(min_state_width);
                (state.id.clone(), width)
            })
            .collect()
    };

    fn collect_all_transitions<'a>(
        states: &'a [State],
        transitions: &'a [Transition],
    ) -> Vec<&'a Transition> {
        let mut result: Vec<&'a Transition> = transitions.iter().collect();
        for state in states {
            result.extend(state.internal_transitions.iter());
            result.extend(collect_all_transitions(
                &state.children,
                &state.internal_transitions,
            ));
        }
        result
    }

    let all_transitions = collect_all_transitions(&diagram.states, &diagram.transitions);

    let transition_labels: HashMap<(String, String), (String, Vec2)> = {
        let text_measurer = EguiTextMeasurer::new(ui);
        all_transitions
            .iter()
            .filter_map(|trans| {
                trans.label.as_ref().map(|label| {
                    let text_size = text_measurer.measure(label, label_font_size);
                    let label_padding = Vec2::new(24.0, 10.0);
                    let size = Vec2::new(
                        text_size.width * 1.15 + label_padding.x,
                        text_size.height + label_padding.y,
                    );
                    (
                        (trans.from.clone(), trans.to.clone()),
                        (label.clone(), size),
                    )
                })
            })
            .collect()
    };

    let mut state_layouts: HashMap<String, StateLayout> = HashMap::new();

    fn layout_states(
        states: &[State],
        transitions: &[Transition],
        state_widths: &HashMap<String, f32>,
        min_state_width: f32,
        state_height: f32,
        spacing_x: f32,
        spacing_y: f32,
        composite_padding: f32,
        title_bar_height: f32,
        start_pos: Pos2,
        layouts: &mut HashMap<String, StateLayout>,
    ) -> Vec2 {
        if states.is_empty() {
            return Vec2::ZERO;
        }

        let mut composite_sizes: HashMap<String, Vec2> = HashMap::new();
        for state in states {
            if state.is_composite() {
                let child_size = layout_states(
                    &state.children,
                    &state.internal_transitions,
                    state_widths,
                    min_state_width,
                    state_height,
                    spacing_x * 0.8,
                    spacing_y * 0.8,
                    composite_padding,
                    title_bar_height,
                    Pos2::ZERO,
                    layouts,
                );
                let comp_width = (child_size.x + composite_padding * 2.0).max(
                    state_widths
                        .get(&state.id)
                        .copied()
                        .unwrap_or(min_state_width)
                        + composite_padding * 2.0,
                );
                let comp_height = child_size.y + composite_padding * 2.0 + title_bar_height;
                composite_sizes.insert(state.id.clone(), Vec2::new(comp_width, comp_height));
            }
        }

        let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
        let mut incoming: HashMap<String, Vec<String>> = HashMap::new();

        for state in states {
            outgoing.insert(state.id.clone(), Vec::new());
            incoming.insert(state.id.clone(), Vec::new());
        }

        for trans in transitions {
            if states.iter().any(|s| s.id == trans.from) && states.iter().any(|s| s.id == trans.to)
            {
                if let Some(out) = outgoing.get_mut(&trans.from) {
                    out.push(trans.to.clone());
                }
                if let Some(inc) = incoming.get_mut(&trans.to) {
                    inc.push(trans.from.clone());
                }
            }
        }

        let mut layers: Vec<Vec<String>> = Vec::new();
        let mut state_layer: HashMap<String, usize> = HashMap::new();
        let mut remaining: Vec<String> = states.iter().map(|s| s.id.clone()).collect();

        while !remaining.is_empty() {
            let layer_states: Vec<String> = remaining
                .iter()
                .filter(|id| {
                    incoming.get(*id).map_or(true, |inc| {
                        inc.iter().all(|from| state_layer.contains_key(from))
                    })
                })
                .cloned()
                .collect();

            if layer_states.is_empty() {
                for id in remaining.drain(..) {
                    let idx = layers.len();
                    state_layer.insert(id.clone(), idx);
                    if layers.len() <= idx {
                        layers.push(Vec::new());
                    }
                    layers[idx].push(id);
                }
                break;
            }

            let idx = layers.len();
            layers.push(layer_states.clone());
            for id in &layer_states {
                state_layer.insert(id.clone(), idx);
                remaining.retain(|r| r != id);
            }
        }

        let mut max_x = 0.0_f32;
        let mut max_y = 0.0_f32;

        for (layer_idx, layer) in layers.iter().enumerate() {
            let layer_max_width = layer
                .iter()
                .map(|id| {
                    if let Some(comp_size) = composite_sizes.get(id) {
                        comp_size.x
                    } else {
                        state_widths.get(id).copied().unwrap_or(min_state_width)
                    }
                })
                .fold(min_state_width, f32::max);

            let x = start_pos.x
                + layer_idx as f32 * (layer_max_width + spacing_x)
                + layer_max_width / 2.0;

            let mut current_y = start_pos.y;

            for id in layer {
                let state = states.iter().find(|s| s.id == *id).unwrap();
                let (width, height) = if let Some(comp_size) = composite_sizes.get(id) {
                    (comp_size.x, comp_size.y)
                } else if state.is_start || state.is_end {
                    (24.0, 24.0)
                } else {
                    (
                        state_widths.get(id).copied().unwrap_or(min_state_width),
                        state_height,
                    )
                };

                let center_y = current_y + height / 2.0;
                let center = Pos2::new(x, center_y);

                layouts.insert(
                    id.clone(),
                    StateLayout {
                        center,
                        size: Vec2::new(width, height),
                        bounds: Rect::from_center_size(center, Vec2::new(width, height)),
                        is_composite: state.is_composite(),
                    },
                );

                if state.is_composite() {
                    let child_offset = Vec2::new(
                        center.x - width / 2.0 + composite_padding,
                        center.y - height / 2.0 + title_bar_height + composite_padding,
                    );
                    reposition_children(&state.children, child_offset, layouts);
                }

                current_y += height + spacing_y;
                max_x = max_x.max(x + width / 2.0);
                max_y = max_y.max(current_y - spacing_y);
            }
        }

        Vec2::new(max_x - start_pos.x, max_y - start_pos.y)
    }

    fn reposition_children(
        children: &[State],
        offset: Vec2,
        layouts: &mut HashMap<String, StateLayout>,
    ) {
        for child in children {
            if let Some(layout) = layouts.get_mut(&child.id) {
                layout.center = layout.center + offset;
                layout.bounds = Rect::from_center_size(layout.center, layout.size);
            }
            reposition_children(&child.children, offset, layouts);
        }
    }

    let total_size = layout_states(
        &diagram.states,
        &diagram.transitions,
        &state_widths,
        min_state_width,
        state_height,
        spacing_x,
        spacing_y,
        composite_padding,
        title_bar_height,
        Pos2::new(margin, margin),
        &mut state_layouts,
    );

    let total_width = (total_size.x + margin * 2.0).max(300.0);
    let total_height = (total_size.y + margin * 2.0).max(100.0);

    let (response, painter) =
        ui.allocate_painter(Vec2::new(total_width, total_height), egui::Sense::hover());
    let offset = response.rect.min.to_vec2();

    fn draw_states(
        states: &[State],
        layouts: &HashMap<String, StateLayout>,
        painter: &egui::Painter,
        offset: Vec2,
        colors: &StateDiagramColors,
        font_size: f32,
        title_bar_height: f32,
        dark_mode: bool,
    ) {
        for state in states {
            if let Some(layout) = layouts.get(&state.id) {
                let center = layout.center + offset;
                let bounds = layout.bounds.translate(offset);

                if state.is_start || state.is_end {
                    let radius = if state.is_start { 10.0 } else { 14.0 };
                    painter.circle_filled(
                        center,
                        radius,
                        if state.is_start {
                            colors.start_color
                        } else {
                            colors.state_stroke
                        },
                    );
                    if state.is_end {
                        painter.circle_filled(
                            center,
                            7.0,
                            if dark_mode {
                                Color32::from_rgb(30, 35, 45)
                            } else {
                                Color32::WHITE
                            },
                        );
                    }
                } else if state.is_composite() {
                    painter.rect(
                        bounds,
                        Rounding::same(10.0),
                        colors.composite_fill,
                        Stroke::new(2.0, colors.state_stroke),
                    );

                    let title_rect = Rect::from_min_size(
                        bounds.min,
                        Vec2::new(bounds.width(), title_bar_height),
                    );
                    painter.rect(
                        title_rect,
                        Rounding {
                            nw: 10.0,
                            ne: 10.0,
                            sw: 0.0,
                            se: 0.0,
                        },
                        colors.composite_title_bg,
                        Stroke::NONE,
                    );

                    let title_center = Pos2::new(title_rect.center().x, title_rect.center().y);
                    painter.text(
                        title_center,
                        egui::Align2::CENTER_CENTER,
                        &state.label,
                        FontId::proportional(font_size),
                        colors.text_color,
                    );

                    painter.line_segment(
                        [
                            Pos2::new(bounds.min.x, bounds.min.y + title_bar_height),
                            Pos2::new(bounds.max.x, bounds.min.y + title_bar_height),
                        ],
                        Stroke::new(1.0, colors.state_stroke),
                    );

                    draw_states(
                        &state.children,
                        layouts,
                        painter,
                        offset,
                        colors,
                        font_size,
                        title_bar_height,
                        dark_mode,
                    );
                } else {
                    painter.rect(
                        bounds,
                        Rounding::same(8.0),
                        colors.state_fill,
                        Stroke::new(2.0, colors.state_stroke),
                    );
                    painter.text(
                        center,
                        egui::Align2::CENTER_CENTER,
                        &state.label,
                        FontId::proportional(font_size),
                        colors.text_color,
                    );
                }
            }
        }
    }

    fn compute_anchor_point(
        layout: &StateLayout,
        target: Pos2,
        offset: Vec2,
        is_composite: bool,
        header_height: f32,
    ) -> Pos2 {
        let center = layout.center + offset;
        let bounds = layout.bounds.translate(offset);

        if layout.size.x == layout.size.y && layout.size.x < 30.0 {
            let dir = (target - center).normalized();
            return center + dir * 12.0;
        }

        let dx = target.x - center.x;
        let dy = target.y - center.y;
        let abs_dx = dx.abs();
        let abs_dy = dy.abs();

        let half_w = layout.size.x / 2.0;
        let half_h = layout.size.y / 2.0;

        if abs_dx / half_w > abs_dy / half_h {
            if dx > 0.0 {
                let y = center
                    .y
                    .max(
                        bounds.min.y
                            + if is_composite {
                                header_height + 5.0
                            } else {
                                0.0
                            },
                    )
                    .min(bounds.max.y - 5.0);
                Pos2::new(
                    bounds.max.x,
                    y.clamp(bounds.min.y + 5.0, bounds.max.y - 5.0),
                )
            } else {
                let y = center
                    .y
                    .max(
                        bounds.min.y
                            + if is_composite {
                                header_height + 5.0
                            } else {
                                0.0
                            },
                    )
                    .min(bounds.max.y - 5.0);
                Pos2::new(
                    bounds.min.x,
                    y.clamp(bounds.min.y + 5.0, bounds.max.y - 5.0),
                )
            }
        } else if dy > 0.0 {
            Pos2::new(
                center.x.clamp(bounds.min.x + 5.0, bounds.max.x - 5.0),
                bounds.max.y,
            )
        } else {
            let min_y = if is_composite {
                bounds.min.y + header_height
            } else {
                bounds.min.y
            };
            Pos2::new(
                center.x.clamp(bounds.min.x + 5.0, bounds.max.x - 5.0),
                min_y,
            )
        }
    }

    fn draw_transitions(
        transitions: &[Transition],
        layouts: &HashMap<String, StateLayout>,
        transition_labels: &HashMap<(String, String), (String, Vec2)>,
        painter: &egui::Painter,
        offset: Vec2,
        colors: &StateDiagramColors,
        font_size: f32,
        config: &StateDiagramConfig,
    ) {
        for trans in transitions {
            if let (Some(from_layout), Some(to_layout)) =
                (layouts.get(&trans.from), layouts.get(&trans.to))
            {
                let arrow_color = colors.arrow_color_for_kind(trans.kind);

                let from_center = from_layout.center + offset;
                let to_center = to_layout.center + offset;

                let start = compute_anchor_point(
                    from_layout,
                    to_center,
                    offset,
                    from_layout.is_composite,
                    config.header_height,
                );
                let end = compute_anchor_point(
                    to_layout,
                    from_center,
                    offset,
                    to_layout.is_composite,
                    config.header_height,
                );

                let use_orthogonal =
                    config.orthogonal_cross_routing && trans.kind == TransitionKind::CrossHierarchy;

                if use_orthogonal
                    && (start.x - end.x).abs() > 20.0
                    && (start.y - end.y).abs() > 20.0
                {
                    let mid_x = (start.x + end.x) / 2.0;
                    let elbow1 = Pos2::new(mid_x, start.y);
                    let elbow2 = Pos2::new(mid_x, end.y);

                    painter.line_segment([start, elbow1], Stroke::new(1.5, arrow_color));
                    painter.line_segment([elbow1, elbow2], Stroke::new(1.5, arrow_color));
                    painter.line_segment([elbow2, end], Stroke::new(1.5, arrow_color));

                    let final_dir = (end - elbow2).normalized();
                    let arrow_size = 8.0;
                    let perp = Vec2::new(-final_dir.y, final_dir.x);
                    let arrow_left = end - final_dir * arrow_size + perp * (arrow_size * 0.4);
                    let arrow_right = end - final_dir * arrow_size - perp * (arrow_size * 0.4);
                    painter.add(egui::Shape::convex_polygon(
                        vec![end, arrow_left, arrow_right],
                        arrow_color,
                        Stroke::NONE,
                    ));

                    if let Some((label_text, label_size)) =
                        transition_labels.get(&(trans.from.clone(), trans.to.clone()))
                    {
                        let label_pos = Pos2::new(mid_x, (elbow1.y + elbow2.y) / 2.0);
                        let label_rect = Rect::from_center_size(label_pos, *label_size);
                        painter.rect_filled(label_rect, 3.0, colors.label_bg);
                        painter.text(
                            label_pos,
                            egui::Align2::CENTER_CENTER,
                            label_text,
                            FontId::proportional(font_size - 2.0),
                            colors.text_color,
                        );
                    }
                } else {
                    let dir = (end - start).normalized();

                    painter.line_segment([start, end], Stroke::new(1.5, arrow_color));

                    let arrow_size = 8.0;
                    let perp = Vec2::new(-dir.y, dir.x);
                    let arrow_left = end - dir * arrow_size + perp * (arrow_size * 0.4);
                    let arrow_right = end - dir * arrow_size - perp * (arrow_size * 0.4);
                    painter.add(egui::Shape::convex_polygon(
                        vec![end, arrow_left, arrow_right],
                        arrow_color,
                        Stroke::NONE,
                    ));

                    if let Some((label_text, label_size)) =
                        transition_labels.get(&(trans.from.clone(), trans.to.clone()))
                    {
                        let mid = Pos2::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);
                        let label_pos = mid - Vec2::new(0.0, 14.0);
                        let label_rect = Rect::from_center_size(label_pos, *label_size);
                        painter.rect_filled(label_rect, 3.0, colors.label_bg);
                        painter.text(
                            label_pos,
                            egui::Align2::CENTER_CENTER,
                            label_text,
                            FontId::proportional(font_size - 2.0),
                            colors.text_color,
                        );
                    }
                }
            }
        }
    }

    fn draw_all_internal_transitions(
        states: &[State],
        layouts: &HashMap<String, StateLayout>,
        transition_labels: &HashMap<(String, String), (String, Vec2)>,
        painter: &egui::Painter,
        offset: Vec2,
        colors: &StateDiagramColors,
        font_size: f32,
        config: &StateDiagramConfig,
    ) {
        for state in states {
            draw_transitions(
                &state.internal_transitions,
                layouts,
                transition_labels,
                painter,
                offset,
                colors,
                font_size,
                config,
            );
            draw_all_internal_transitions(
                &state.children,
                layouts,
                transition_labels,
                painter,
                offset,
                colors,
                font_size,
                config,
            );
        }
    }

    draw_states(
        &diagram.states,
        &state_layouts,
        &painter,
        offset,
        &colors,
        font_size,
        title_bar_height,
        dark_mode,
    );
    draw_transitions(
        &diagram.transitions,
        &state_layouts,
        &transition_labels,
        &painter,
        offset,
        &colors,
        label_font_size,
        config,
    );
    draw_all_internal_transitions(
        &diagram.states,
        &state_layouts,
        &transition_labels,
        &painter,
        offset,
        &colors,
        label_font_size,
        config,
    );
}
