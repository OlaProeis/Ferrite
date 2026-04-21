//! Sequence diagram parsing and rendering.

use egui::{Color32, FontId, Pos2, Rect, Rounding, Stroke, Ui, Vec2};
use std::collections::HashMap;

use super::text::{EguiTextMeasurer, TextMeasurer};
use super::utils::draw_dashed_line;

/// A participant in a sequence diagram.
#[derive(Debug, Clone)]
pub struct Participant {
    pub id: String,
    pub label: String,
    pub is_actor: bool,
}

/// Type of message arrow in sequence diagram.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MessageType {
    #[default]
    Solid, // ->>
    SolidOpen,  // ->
    Dotted,     // -->>
    DottedOpen, // -->
}

/// A message between participants.
#[derive(Debug, Clone)]
pub struct Message {
    pub from: String,
    pub to: String,
    pub label: String,
    pub message_type: MessageType,
    /// Activate the target participant when this message is sent
    pub activate_target: bool,
    /// Deactivate the target participant when this message is sent
    pub deactivate_target: bool,
}

/// Position for a note in a sequence diagram.
#[derive(Debug, Clone)]
pub enum NotePosition {
    LeftOf(String),
    RightOf(String),
    Over(Vec<String>),
}

/// A note in a sequence diagram.
#[derive(Debug, Clone)]
pub struct SeqNote {
    pub position: NotePosition,
    pub text: String,
}

/// Type of control-flow block in sequence diagram.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeqBlockKind {
    Loop,
    Alt,
    Opt,
    Par,
}

impl SeqBlockKind {
    fn from_keyword(kw: &str) -> Option<Self> {
        match kw {
            "loop" => Some(Self::Loop),
            "alt" => Some(Self::Alt),
            "opt" => Some(Self::Opt),
            "par" => Some(Self::Par),
            _ => None,
        }
    }

    fn display_name(&self) -> &'static str {
        match self {
            Self::Loop => "loop",
            Self::Alt => "alt",
            Self::Opt => "opt",
            Self::Par => "par",
        }
    }
}

/// A segment within a control-flow block (e.g., alt branches, par branches).
#[derive(Debug, Clone)]
pub struct SeqBlockSegment {
    pub segment_label: Option<String>,
    pub statements: Vec<SeqStatement>,
}

/// A control-flow block in a sequence diagram (loop, alt, opt, par).
#[derive(Debug, Clone)]
pub struct SeqBlock {
    pub kind: SeqBlockKind,
    pub label: String,
    pub segments: Vec<SeqBlockSegment>,
}

/// A statement in a sequence diagram - message, block, note, or activation directive.
#[derive(Debug, Clone)]
pub enum SeqStatement {
    Message(Message),
    Block(SeqBlock),
    Note(SeqNote),
    Activate(String),
    Deactivate(String),
}

/// A parsed sequence diagram.
#[derive(Debug, Clone, Default)]
pub struct SequenceDiagram {
    pub participants: Vec<Participant>,
    pub statements: Vec<SeqStatement>,
}

/// Helper struct for building control-flow blocks during parsing.
struct SeqBlockBuilder {
    kind: SeqBlockKind,
    label: String,
    segments: Vec<SeqBlockSegment>,
    current_segment_label: Option<String>,
    current_segment_statements: Vec<SeqStatement>,
}

impl SeqBlockBuilder {
    fn new(kind: SeqBlockKind, label: String) -> Self {
        Self {
            kind,
            label,
            segments: Vec::new(),
            current_segment_label: None,
            current_segment_statements: Vec::new(),
        }
    }

    fn start_new_segment(&mut self, label: Option<String>) {
        if !self.current_segment_statements.is_empty() || self.current_segment_label.is_some() {
            self.segments.push(SeqBlockSegment {
                segment_label: self.current_segment_label.take(),
                statements: std::mem::take(&mut self.current_segment_statements),
            });
        }
        self.current_segment_label = label;
    }

    fn add_statement(&mut self, stmt: SeqStatement) {
        self.current_segment_statements.push(stmt);
    }

    fn finalize(mut self) -> SeqBlock {
        self.segments.push(SeqBlockSegment {
            segment_label: self.current_segment_label,
            statements: self.current_segment_statements,
        });

        SeqBlock {
            kind: self.kind,
            label: self.label,
            segments: self.segments,
        }
    }
}

/// Parse mermaid sequence diagram source.
pub fn parse_sequence_diagram(source: &str) -> Result<SequenceDiagram, String> {
    let mut diagram = SequenceDiagram::default();
    let mut participant_map: HashMap<String, usize> = HashMap::new();
    let mut block_stack: Vec<SeqBlockBuilder> = Vec::new();
    let lines: Vec<&str> = source.lines().skip(1).collect();

    for (line_num, line) in lines.iter().enumerate() {
        let line = line.trim();

        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        // Parse participant declaration
        if line.starts_with("participant ") || line.starts_with("actor ") {
            let is_actor = line.starts_with("actor ");
            let rest = if is_actor { &line[6..] } else { &line[12..] };
            let rest = rest.trim();

            let (id, label) = if let Some(as_pos) = rest.find(" as ") {
                let id = rest[..as_pos].trim().to_string();
                let label = rest[as_pos + 4..].trim().to_string();
                (id, label)
            } else {
                (rest.to_string(), rest.to_string())
            };

            if !participant_map.contains_key(&id) {
                participant_map.insert(id.clone(), diagram.participants.len());
                diagram.participants.push(Participant {
                    id,
                    label,
                    is_actor,
                });
            }
            continue;
        }

        // Parse control-flow block keywords
        let first_word = line.split_whitespace().next().unwrap_or("");

        if let Some(kind) = SeqBlockKind::from_keyword(first_word) {
            let label = line[first_word.len()..].trim().to_string();
            block_stack.push(SeqBlockBuilder::new(kind, label));
            continue;
        }

        if first_word == "else" {
            if let Some(builder) = block_stack.last_mut() {
                if builder.kind == SeqBlockKind::Alt {
                    let label = line[4..].trim();
                    let label = if label.is_empty() {
                        None
                    } else {
                        Some(label.to_string())
                    };
                    builder.start_new_segment(label);
                } else {
                    return Err(format!(
                        "Line {}: 'else' can only be used inside 'alt' blocks",
                        line_num + 2
                    ));
                }
            } else {
                return Err(format!(
                    "Line {}: 'else' without matching 'alt' block",
                    line_num + 2
                ));
            }
            continue;
        }

        if first_word == "and" {
            if let Some(builder) = block_stack.last_mut() {
                if builder.kind == SeqBlockKind::Par {
                    let label = line[3..].trim();
                    let label = if label.is_empty() {
                        None
                    } else {
                        Some(label.to_string())
                    };
                    builder.start_new_segment(label);
                } else {
                    return Err(format!(
                        "Line {}: 'and' can only be used inside 'par' blocks",
                        line_num + 2
                    ));
                }
            } else {
                return Err(format!(
                    "Line {}: 'and' without matching 'par' block",
                    line_num + 2
                ));
            }
            continue;
        }

        if first_word == "end" {
            if let Some(builder) = block_stack.pop() {
                let block = builder.finalize();
                let stmt = SeqStatement::Block(block);

                if let Some(parent) = block_stack.last_mut() {
                    parent.add_statement(stmt);
                } else {
                    diagram.statements.push(stmt);
                }
            } else {
                return Err(format!(
                    "Line {}: 'end' without matching block opener",
                    line_num + 2
                ));
            }
            continue;
        }

        // Parse activate/deactivate commands
        if first_word == "activate" {
            let participant_id = line[8..].trim().to_string();
            if !participant_id.is_empty() {
                if !participant_map.contains_key(&participant_id) {
                    participant_map.insert(participant_id.clone(), diagram.participants.len());
                    diagram.participants.push(Participant {
                        id: participant_id.clone(),
                        label: participant_id.clone(),
                        is_actor: false,
                    });
                }

                let stmt = SeqStatement::Activate(participant_id);
                if let Some(builder) = block_stack.last_mut() {
                    builder.add_statement(stmt);
                } else {
                    diagram.statements.push(stmt);
                }
            }
            continue;
        }

        if first_word == "deactivate" {
            let participant_id = line[10..].trim().to_string();
            if !participant_id.is_empty() {
                let stmt = SeqStatement::Deactivate(participant_id);
                if let Some(builder) = block_stack.last_mut() {
                    builder.add_statement(stmt);
                } else {
                    diagram.statements.push(stmt);
                }
            }
            continue;
        }

        // Parse Note syntax
        if let Some(note) = parse_sequence_note(line) {
            match &note.position {
                NotePosition::LeftOf(id) | NotePosition::RightOf(id) => {
                    if !participant_map.contains_key(id) {
                        participant_map.insert(id.clone(), diagram.participants.len());
                        diagram.participants.push(Participant {
                            id: id.clone(),
                            label: id.clone(),
                            is_actor: false,
                        });
                    }
                }
                NotePosition::Over(ids) => {
                    for id in ids {
                        if !participant_map.contains_key(id) {
                            participant_map.insert(id.clone(), diagram.participants.len());
                            diagram.participants.push(Participant {
                                id: id.clone(),
                                label: id.clone(),
                                is_actor: false,
                            });
                        }
                    }
                }
            }

            let stmt = SeqStatement::Note(note);
            if let Some(builder) = block_stack.last_mut() {
                builder.add_statement(stmt);
            } else {
                diagram.statements.push(stmt);
            }
            continue;
        }

        // Parse message
        if let Some(msg) = parse_sequence_message(line) {
            for id in [&msg.from, &msg.to] {
                if !participant_map.contains_key(id) {
                    participant_map.insert(id.clone(), diagram.participants.len());
                    diagram.participants.push(Participant {
                        id: id.clone(),
                        label: id.clone(),
                        is_actor: false,
                    });
                }
            }

            let stmt = SeqStatement::Message(msg);

            if let Some(builder) = block_stack.last_mut() {
                builder.add_statement(stmt);
            } else {
                diagram.statements.push(stmt);
            }
        }
    }

    if let Some(builder) = block_stack.last() {
        return Err(format!(
            "Unclosed '{}' block at end of diagram",
            builder.kind.display_name()
        ));
    }

    if diagram.participants.is_empty() {
        return Err("No participants found in sequence diagram".to_string());
    }

    Ok(diagram)
}

fn parse_sequence_message(line: &str) -> Option<Message> {
    let arrow_patterns = [
        ("-->>+", MessageType::Dotted, true, false),
        ("-->>-", MessageType::Dotted, false, true),
        ("->>+", MessageType::Solid, true, false),
        ("->>-", MessageType::Solid, false, true),
        ("-->+", MessageType::DottedOpen, true, false),
        ("-->-", MessageType::DottedOpen, false, true),
        ("->+", MessageType::SolidOpen, true, false),
        ("->-", MessageType::SolidOpen, false, true),
        ("-->>", MessageType::Dotted, false, false),
        ("->>", MessageType::Solid, false, false),
        ("-->", MessageType::DottedOpen, false, false),
        ("->", MessageType::SolidOpen, false, false),
    ];

    for (pattern, msg_type, is_activate, is_deactivate) in arrow_patterns {
        if let Some(arrow_pos) = line.find(pattern) {
            let from = line[..arrow_pos].trim();
            let rest = &line[arrow_pos + pattern.len()..];

            let (to, label) = if let Some(colon_pos) = rest.find(':') {
                let to = rest[..colon_pos].trim();
                let label = rest[colon_pos + 1..].trim();
                (to, label)
            } else {
                (rest.trim(), "")
            };

            if !from.is_empty() && !to.is_empty() {
                return Some(Message {
                    from: from.to_string(),
                    to: to.to_string(),
                    label: label.to_string(),
                    message_type: msg_type,
                    activate_target: is_activate,
                    deactivate_target: is_deactivate,
                });
            }
        }
    }

    None
}

fn parse_sequence_note(line: &str) -> Option<SeqNote> {
    let line_lower = line.to_lowercase();

    if !line_lower.starts_with("note ") {
        return None;
    }

    let rest = &line[5..];
    let colon_pos = rest.find(':')?;
    let position_part = rest[..colon_pos].trim();
    let text = rest[colon_pos + 1..].trim().to_string();

    let position_lower = position_part.to_lowercase();

    let position = if position_lower.starts_with("left of ") {
        let participant = position_part[8..].trim().to_string();
        NotePosition::LeftOf(participant)
    } else if position_lower.starts_with("right of ") {
        let participant = position_part[9..].trim().to_string();
        NotePosition::RightOf(participant)
    } else if position_lower.starts_with("over ") {
        let participants_str = position_part[5..].trim();
        let participants: Vec<String> = participants_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if participants.is_empty() {
            return None;
        }
        NotePosition::Over(participants)
    } else {
        return None;
    };

    Some(SeqNote { position, text })
}

// ─────────────────────────────────────────────────────────────────────────────
// Sequence Diagram Renderer
// ─────────────────────────────────────────────────────────────────────────────

struct SeqColors {
    bg: Color32,
    stroke: Color32,
    text: Color32,
    lifeline: Color32,
    actor: Color32,
    block_loop: Color32,
    block_alt: Color32,
    block_opt: Color32,
    block_par: Color32,
    block_stroke: Color32,
    block_text: Color32,
    activation_fill: Color32,
    activation_stroke: Color32,
    note_fill: Color32,
    note_stroke: Color32,
    note_text: Color32,
}

impl SeqColors {
    fn new(dark_mode: bool) -> Self {
        if dark_mode {
            Self {
                bg: Color32::from_rgb(45, 55, 72),
                stroke: Color32::from_rgb(100, 140, 180),
                text: Color32::from_rgb(220, 230, 240),
                lifeline: Color32::from_rgb(80, 100, 120),
                actor: Color32::from_rgb(100, 160, 220),
                block_loop: Color32::from_rgba_unmultiplied(100, 180, 100, 30),
                block_alt: Color32::from_rgba_unmultiplied(180, 140, 100, 30),
                block_opt: Color32::from_rgba_unmultiplied(100, 140, 180, 30),
                block_par: Color32::from_rgba_unmultiplied(180, 100, 180, 30),
                block_stroke: Color32::from_rgb(120, 140, 160),
                block_text: Color32::from_rgb(200, 210, 220),
                activation_fill: Color32::from_rgb(70, 90, 110),
                activation_stroke: Color32::from_rgb(100, 140, 180),
                note_fill: Color32::from_rgb(80, 80, 60),
                note_stroke: Color32::from_rgb(140, 140, 100),
                note_text: Color32::from_rgb(220, 220, 200),
            }
        } else {
            Self {
                bg: Color32::from_rgb(240, 245, 250),
                stroke: Color32::from_rgb(100, 140, 180),
                text: Color32::from_rgb(30, 40, 50),
                lifeline: Color32::from_rgb(180, 190, 200),
                actor: Color32::from_rgb(50, 120, 180),
                block_loop: Color32::from_rgba_unmultiplied(100, 180, 100, 40),
                block_alt: Color32::from_rgba_unmultiplied(220, 180, 100, 40),
                block_opt: Color32::from_rgba_unmultiplied(100, 140, 220, 40),
                block_par: Color32::from_rgba_unmultiplied(200, 100, 200, 40),
                block_stroke: Color32::from_rgb(100, 120, 140),
                block_text: Color32::from_rgb(40, 50, 60),
                activation_fill: Color32::from_rgb(200, 220, 240),
                activation_stroke: Color32::from_rgb(100, 140, 180),
                note_fill: Color32::from_rgb(255, 255, 220),
                note_stroke: Color32::from_rgb(180, 180, 140),
                note_text: Color32::from_rgb(60, 60, 40),
            }
        }
    }

    fn block_fill(&self, kind: &SeqBlockKind) -> Color32 {
        match kind {
            SeqBlockKind::Loop => self.block_loop,
            SeqBlockKind::Alt => self.block_alt,
            SeqBlockKind::Opt => self.block_opt,
            SeqBlockKind::Par => self.block_par,
        }
    }
}

fn count_statement_slots(statements: &[SeqStatement]) -> usize {
    let mut count = 0;
    for stmt in statements {
        match stmt {
            SeqStatement::Message(_) => count += 1,
            SeqStatement::Note(_) => count += 1,
            SeqStatement::Activate(_) | SeqStatement::Deactivate(_) => {}
            SeqStatement::Block(block) => {
                count += 1;
                for segment in &block.segments {
                    count += count_statement_slots(&segment.statements);
                }
                count += 1;
            }
        }
    }
    count
}

struct SeqLayout {
    min_participant_width: f32,
    participant_height: f32,
    participant_spacing: f32,
    message_height: f32,
    margin: f32,
    lifeline_extend: f32,
    participant_padding: f32,
    block_padding: f32,
    block_label_height: f32,
    activation_width: f32,
    activation_offset: f32,
    note_width: f32,
    note_padding: f32,
    note_corner_size: f32,
}

impl Default for SeqLayout {
    fn default() -> Self {
        Self {
            min_participant_width: 80.0,
            participant_height: 40.0,
            participant_spacing: 50.0,
            message_height: 40.0,
            margin: 20.0,
            lifeline_extend: 30.0,
            participant_padding: 24.0,
            block_padding: 8.0,
            block_label_height: 20.0,
            activation_width: 10.0,
            activation_offset: 4.0,
            note_width: 100.0,
            note_padding: 8.0,
            note_corner_size: 8.0,
        }
    }
}

/// Render a sequence diagram to the UI.
pub fn render_sequence_diagram(
    ui: &mut Ui,
    diagram: &SequenceDiagram,
    dark_mode: bool,
    font_size: f32,
) {
    if diagram.participants.is_empty() {
        return;
    }

    let layout = SeqLayout::default();
    let colors = SeqColors::new(dark_mode);

    let participant_widths: HashMap<String, f32> = {
        let text_measurer = EguiTextMeasurer::new(ui);
        diagram
            .participants
            .iter()
            .map(|p| {
                let text_size = text_measurer.measure(&p.label, font_size);
                let width = (text_size.width * 1.15 + layout.participant_padding)
                    .max(layout.min_participant_width);
                (p.id.clone(), width)
            })
            .collect()
    };

    let total_participants_width: f32 = participant_widths.values().sum();
    let total_width = layout.margin * 2.0
        + total_participants_width
        + (diagram.participants.len().saturating_sub(1)) as f32 * layout.participant_spacing;

    let total_slots = count_statement_slots(&diagram.statements);
    let total_height = layout.margin * 2.0
        + layout.participant_height
        + total_slots as f32 * layout.message_height
        + layout.lifeline_extend;

    let (response, painter) =
        ui.allocate_painter(Vec2::new(total_width, total_height), egui::Sense::hover());
    let offset = response.rect.min.to_vec2();

    let mut participant_x: HashMap<String, f32> = HashMap::new();
    let mut current_x = layout.margin;

    for participant in &diagram.participants {
        let width = participant_widths
            .get(&participant.id)
            .copied()
            .unwrap_or(layout.min_participant_width);
        participant_x.insert(participant.id.clone(), current_x + width / 2.0);
        current_x += width + layout.participant_spacing;
    }

    let lifeline_left = diagram
        .participants
        .first()
        .and_then(|p| participant_x.get(&p.id))
        .copied()
        .unwrap_or(layout.margin);
    let lifeline_right = diagram
        .participants
        .last()
        .and_then(|p| {
            let x = participant_x.get(&p.id)?;
            let w = participant_widths.get(&p.id)?;
            Some(x + w / 2.0)
        })
        .unwrap_or(total_width - layout.margin);

    // Draw lifelines
    let lifeline_start_y = layout.margin + layout.participant_height;
    let lifeline_end_y = total_height - layout.margin;

    for participant in &diagram.participants {
        if let Some(&x) = participant_x.get(&participant.id) {
            painter.line_segment(
                [
                    Pos2::new(x, lifeline_start_y) + offset,
                    Pos2::new(x, lifeline_end_y) + offset,
                ],
                Stroke::new(1.0, colors.lifeline),
            );
        }
    }

    // Draw participants
    for participant in &diagram.participants {
        if let Some(&center_x) = participant_x.get(&participant.id) {
            let width = participant_widths
                .get(&participant.id)
                .copied()
                .unwrap_or(layout.min_participant_width);
            let rect = Rect::from_center_size(
                Pos2::new(center_x, layout.margin + layout.participant_height / 2.0) + offset,
                Vec2::new(width, layout.participant_height),
            );

            if participant.is_actor {
                draw_actor(
                    &painter,
                    center_x,
                    &rect,
                    offset,
                    &colors,
                    font_size,
                    &participant.label,
                );
            } else {
                painter.rect(
                    rect,
                    Rounding::same(4.0),
                    colors.bg,
                    Stroke::new(2.0, colors.stroke),
                );
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    &participant.label,
                    FontId::proportional(font_size),
                    colors.text,
                );
            }
        }
    }

    // Draw statements
    let mut current_y = layout.margin + layout.participant_height + layout.message_height / 2.0;
    let mut activation_state: HashMap<String, ActivationState> = HashMap::new();

    draw_statements(
        &painter,
        &diagram.statements,
        &participant_x,
        &mut current_y,
        offset,
        &layout,
        &colors,
        font_size,
        lifeline_left,
        lifeline_right,
        0,
        &mut activation_state,
    );

    // Draw remaining activations
    let diagram_end_y = current_y;
    for (participant_id, state) in activation_state.iter() {
        if let Some(&x) = participant_x.get(participant_id) {
            for (i, &start_y) in state.start_ys.iter().enumerate() {
                let depth_offset = i as f32 * layout.activation_offset;
                draw_activation_box(
                    &painter,
                    x + depth_offset,
                    start_y,
                    diagram_end_y,
                    offset,
                    &layout,
                    &colors,
                );
            }
        }
    }
}

fn draw_actor(
    painter: &egui::Painter,
    center_x: f32,
    rect: &Rect,
    offset: Vec2,
    colors: &SeqColors,
    font_size: f32,
    label: &str,
) {
    let head_y = rect.top() + 10.0;
    let body_y = head_y + 15.0;
    let legs_y = body_y + 12.0;

    painter.circle_stroke(
        Pos2::new(center_x + offset.x, head_y),
        6.0,
        Stroke::new(2.0, colors.actor),
    );
    painter.line_segment(
        [
            Pos2::new(center_x + offset.x, head_y + 6.0),
            Pos2::new(center_x + offset.x, body_y),
        ],
        Stroke::new(2.0, colors.actor),
    );
    painter.line_segment(
        [
            Pos2::new(center_x - 10.0 + offset.x, head_y + 12.0),
            Pos2::new(center_x + 10.0 + offset.x, head_y + 12.0),
        ],
        Stroke::new(2.0, colors.actor),
    );
    painter.line_segment(
        [
            Pos2::new(center_x + offset.x, body_y),
            Pos2::new(center_x - 8.0 + offset.x, legs_y),
        ],
        Stroke::new(2.0, colors.actor),
    );
    painter.line_segment(
        [
            Pos2::new(center_x + offset.x, body_y),
            Pos2::new(center_x + 8.0 + offset.x, legs_y),
        ],
        Stroke::new(2.0, colors.actor),
    );
    painter.text(
        Pos2::new(center_x + offset.x, rect.bottom() - 2.0),
        egui::Align2::CENTER_BOTTOM,
        label,
        FontId::proportional(font_size - 2.0),
        colors.text,
    );
}

fn draw_message(
    painter: &egui::Painter,
    message: &Message,
    participant_x: &HashMap<String, f32>,
    y: f32,
    offset: Vec2,
    colors: &SeqColors,
    font_size: f32,
) {
    if let (Some(&from_x), Some(&to_x)) = (
        participant_x.get(&message.from),
        participant_x.get(&message.to),
    ) {
        let from_pos = Pos2::new(from_x + offset.x, y);
        let to_pos = Pos2::new(to_x + offset.x, y);

        let stroke = Stroke::new(1.5, colors.stroke);

        if matches!(
            message.message_type,
            MessageType::Dotted | MessageType::DottedOpen
        ) {
            draw_dashed_line(painter, from_pos, to_pos, stroke, 5.0, 3.0);
        } else {
            painter.line_segment([from_pos, to_pos], stroke);
        }

        let is_solid_head = matches!(
            message.message_type,
            MessageType::Solid | MessageType::Dotted
        );
        let dir = (to_pos - from_pos).normalized();
        let arrow_size = 8.0;
        let perp = Vec2::new(-dir.y, dir.x);

        let arrow_tip = to_pos;
        let arrow_left = to_pos - dir * arrow_size + perp * (arrow_size * 0.4);
        let arrow_right = to_pos - dir * arrow_size - perp * (arrow_size * 0.4);

        if is_solid_head {
            painter.add(egui::Shape::convex_polygon(
                vec![arrow_tip, arrow_left, arrow_right],
                colors.stroke,
                Stroke::NONE,
            ));
        } else {
            painter.line_segment([arrow_tip, arrow_left], stroke);
            painter.line_segment([arrow_tip, arrow_right], stroke);
        }

        if !message.label.is_empty() {
            let label_pos = Pos2::new((from_x + to_x) / 2.0 + offset.x, y - 8.0);
            painter.text(
                label_pos,
                egui::Align2::CENTER_BOTTOM,
                &message.label,
                FontId::proportional(font_size - 2.0),
                colors.text,
            );
        }
    }
}

#[derive(Default)]
struct ActivationState {
    start_ys: Vec<f32>,
    depth: usize,
}

#[allow(clippy::too_many_arguments)]
fn draw_statements(
    painter: &egui::Painter,
    statements: &[SeqStatement],
    participant_x: &HashMap<String, f32>,
    current_y: &mut f32,
    offset: Vec2,
    layout: &SeqLayout,
    colors: &SeqColors,
    font_size: f32,
    lifeline_left: f32,
    lifeline_right: f32,
    depth: usize,
    activation_state: &mut HashMap<String, ActivationState>,
) {
    for stmt in statements {
        match stmt {
            SeqStatement::Message(message) => {
                let y = *current_y + offset.y;

                if message.activate_target {
                    let state = activation_state.entry(message.to.clone()).or_default();
                    state.start_ys.push(*current_y);
                    state.depth += 1;
                }

                draw_message(
                    painter,
                    message,
                    participant_x,
                    y,
                    offset,
                    colors,
                    font_size,
                );
                *current_y += layout.message_height;

                if message.deactivate_target {
                    if let Some(state) = activation_state.get_mut(&message.to) {
                        if let Some(start_y) = state.start_ys.pop() {
                            if let Some(&x) = participant_x.get(&message.to) {
                                let depth_offset =
                                    state.depth.saturating_sub(1) as f32 * layout.activation_offset;
                                draw_activation_box(
                                    painter,
                                    x + depth_offset,
                                    start_y,
                                    *current_y,
                                    offset,
                                    layout,
                                    colors,
                                );
                            }
                            state.depth = state.depth.saturating_sub(1);
                        }
                    }
                }
            }
            SeqStatement::Note(note) => {
                let y = *current_y + offset.y;
                draw_note(
                    painter,
                    note,
                    participant_x,
                    y,
                    offset,
                    layout,
                    colors,
                    font_size,
                );
                *current_y += layout.message_height;
            }
            SeqStatement::Activate(participant_id) => {
                let state = activation_state.entry(participant_id.clone()).or_default();
                state.start_ys.push(*current_y);
                state.depth += 1;
            }
            SeqStatement::Deactivate(participant_id) => {
                if let Some(state) = activation_state.get_mut(participant_id) {
                    if let Some(start_y) = state.start_ys.pop() {
                        if let Some(&x) = participant_x.get(participant_id) {
                            let depth_offset =
                                state.depth.saturating_sub(1) as f32 * layout.activation_offset;
                            draw_activation_box(
                                painter,
                                x + depth_offset,
                                start_y,
                                *current_y,
                                offset,
                                layout,
                                colors,
                            );
                        }
                        state.depth = state.depth.saturating_sub(1);
                    }
                }
            }
            SeqStatement::Block(block) => {
                draw_block(
                    painter,
                    block,
                    participant_x,
                    current_y,
                    offset,
                    layout,
                    colors,
                    font_size,
                    lifeline_left,
                    lifeline_right,
                    depth,
                    activation_state,
                );
            }
        }
    }
}

fn draw_activation_box(
    painter: &egui::Painter,
    x: f32,
    start_y: f32,
    end_y: f32,
    offset: Vec2,
    layout: &SeqLayout,
    colors: &SeqColors,
) {
    let rect = Rect::from_min_max(
        Pos2::new(x - layout.activation_width / 2.0, start_y) + offset,
        Pos2::new(x + layout.activation_width / 2.0, end_y) + offset,
    );

    painter.rect(
        rect,
        Rounding::same(2.0),
        colors.activation_fill,
        Stroke::new(1.5, colors.activation_stroke),
    );
}

fn draw_note(
    painter: &egui::Painter,
    note: &SeqNote,
    participant_x: &HashMap<String, f32>,
    y: f32,
    offset: Vec2,
    layout: &SeqLayout,
    colors: &SeqColors,
    font_size: f32,
) {
    let (note_x, note_width) = match &note.position {
        NotePosition::LeftOf(participant) => {
            if let Some(&x) = participant_x.get(participant) {
                (
                    x - layout.note_width - layout.participant_spacing / 2.0,
                    layout.note_width,
                )
            } else {
                return;
            }
        }
        NotePosition::RightOf(participant) => {
            if let Some(&x) = participant_x.get(participant) {
                (x + layout.participant_spacing / 2.0, layout.note_width)
            } else {
                return;
            }
        }
        NotePosition::Over(participants) => {
            if participants.is_empty() {
                return;
            }

            let xs: Vec<f32> = participants
                .iter()
                .filter_map(|id| participant_x.get(id).copied())
                .collect();

            if xs.is_empty() {
                return;
            }

            let min_x = xs.iter().copied().fold(f32::INFINITY, f32::min);
            let max_x = xs.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            let width = (max_x - min_x).max(layout.note_width);
            let center_x = (min_x + max_x) / 2.0;

            (center_x - width / 2.0, width)
        }
    };

    let note_height = layout.message_height - layout.note_padding;
    let note_rect = Rect::from_min_size(
        Pos2::new(note_x, y - note_height / 2.0) + offset,
        Vec2::new(note_width, note_height),
    );

    let corner = layout.note_corner_size;
    let points = vec![
        note_rect.left_top(),
        Pos2::new(note_rect.right() - corner, note_rect.top()),
        Pos2::new(note_rect.right(), note_rect.top() + corner),
        note_rect.right_bottom(),
        note_rect.left_bottom(),
    ];

    painter.add(egui::Shape::convex_polygon(
        points,
        colors.note_fill,
        Stroke::new(1.0, colors.note_stroke),
    ));

    painter.line_segment(
        [
            Pos2::new(note_rect.right() - corner, note_rect.top()),
            Pos2::new(note_rect.right() - corner, note_rect.top() + corner),
        ],
        Stroke::new(1.0, colors.note_stroke),
    );
    painter.line_segment(
        [
            Pos2::new(note_rect.right() - corner, note_rect.top() + corner),
            Pos2::new(note_rect.right(), note_rect.top() + corner),
        ],
        Stroke::new(1.0, colors.note_stroke),
    );

    painter.text(
        note_rect.center(),
        egui::Align2::CENTER_CENTER,
        &note.text,
        FontId::proportional(font_size - 2.0),
        colors.note_text,
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_block(
    painter: &egui::Painter,
    block: &SeqBlock,
    participant_x: &HashMap<String, f32>,
    current_y: &mut f32,
    offset: Vec2,
    layout: &SeqLayout,
    colors: &SeqColors,
    font_size: f32,
    lifeline_left: f32,
    lifeline_right: f32,
    depth: usize,
    activation_state: &mut HashMap<String, ActivationState>,
) {
    let mut block_height = layout.block_label_height;
    for (i, segment) in block.segments.iter().enumerate() {
        let segment_slots = count_statement_slots(&segment.statements);
        block_height += segment_slots as f32 * layout.message_height;
        if i < block.segments.len() - 1 {
            block_height += layout.block_label_height;
        }
    }
    block_height += layout.block_padding * 2.0;

    let inset = depth as f32 * layout.block_padding * 2.0;
    let block_left = lifeline_left - layout.block_padding * 4.0 + inset;
    let block_right = lifeline_right + layout.block_padding * 4.0 - inset;
    let block_width = block_right - block_left;

    let block_top_y = *current_y - layout.message_height / 2.0 + layout.block_padding;

    let block_rect = Rect::from_min_size(
        Pos2::new(block_left, block_top_y) + offset,
        Vec2::new(block_width, block_height),
    );

    let fill_color = colors.block_fill(&block.kind);
    painter.rect(
        block_rect,
        Rounding::same(4.0),
        fill_color,
        Stroke::new(1.5, colors.block_stroke),
    );

    let label_text = block.kind.display_name().to_string();
    let label_with_bracket = if block.label.is_empty() {
        label_text
    } else {
        format!("{} [{}]", label_text, block.label)
    };

    let label_rect = Rect::from_min_size(
        Pos2::new(block_left, block_top_y) + offset,
        Vec2::new(
            label_with_bracket.len() as f32 * 7.0 + 12.0,
            layout.block_label_height,
        ),
    );

    painter.rect_filled(label_rect, Rounding::ZERO, colors.block_stroke);

    painter.text(
        label_rect.left_center() + Vec2::new(6.0, 0.0),
        egui::Align2::LEFT_CENTER,
        &label_with_bracket,
        FontId::proportional(font_size - 3.0),
        Color32::WHITE,
    );

    *current_y += layout.block_label_height;

    for (i, segment) in block.segments.iter().enumerate() {
        if i > 0 {
            let sep_y = *current_y - layout.message_height / 2.0 + offset.y;
            painter.line_segment(
                [
                    Pos2::new(block_left + offset.x, sep_y),
                    Pos2::new(block_right + offset.x, sep_y),
                ],
                Stroke::new(1.0, colors.block_stroke),
            );

            let segment_label_text = if let Some(label) = &segment.segment_label {
                Some(format!("[{}]", label))
            } else {
                match block.kind {
                    SeqBlockKind::Alt => Some("[else]".to_string()),
                    SeqBlockKind::Par => Some("[and]".to_string()),
                    _ => None,
                }
            };

            if let Some(text) = segment_label_text {
                painter.text(
                    Pos2::new(block_left + 10.0 + offset.x, sep_y + 2.0),
                    egui::Align2::LEFT_TOP,
                    &text,
                    FontId::proportional(font_size - 3.0),
                    colors.block_text,
                );
            }

            *current_y += layout.block_label_height;
        }

        draw_statements(
            painter,
            &segment.statements,
            participant_x,
            current_y,
            offset,
            layout,
            colors,
            font_size,
            lifeline_left,
            lifeline_right,
            depth + 1,
            activation_state,
        );
    }

    *current_y += layout.block_padding * 2.0;
}
