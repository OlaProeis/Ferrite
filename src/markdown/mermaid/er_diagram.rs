//! Entity-Relationship diagram parsing and rendering.

use egui::{Color32, FontId, Pos2, Rect, Rounding, Stroke, Ui, Vec2};
use std::collections::HashMap;

use super::text::{EguiTextMeasurer, TextMeasurer};
use super::utils::draw_dashed_line;

/// An entity in an ER diagram.
#[derive(Debug, Clone)]
pub struct Entity {
    pub name: String,
    pub attributes: Vec<EntityAttribute>,
}

/// An attribute of an entity.
#[derive(Debug, Clone)]
pub struct EntityAttribute {
    pub name: String,
    pub attr_type: String,
    pub is_pk: bool, // Primary key
    pub is_fk: bool, // Foreign key
}

/// Cardinality in a relationship.
#[derive(Debug, Clone, PartialEq)]
pub enum Cardinality {
    ZeroOrOne,  // |o or o|
    ExactlyOne, // ||
    ZeroOrMore, // }o or o{
    OneOrMore,  // }|  or |{
}

/// A relationship between entities.
#[derive(Debug, Clone)]
pub struct ERRelation {
    pub from: String,
    pub to: String,
    pub from_cardinality: Cardinality,
    pub to_cardinality: Cardinality,
    pub label: Option<String>,
    pub identifying: bool, // solid vs dashed line
}

/// An ER diagram.
#[derive(Debug, Clone)]
pub struct ERDiagram {
    pub entities: Vec<Entity>,
    pub relations: Vec<ERRelation>,
}

/// Parse an ER diagram from source.
pub fn parse_er_diagram(source: &str) -> Result<ERDiagram, String> {
    let mut entities: Vec<Entity> = Vec::new();
    let mut relations: Vec<ERRelation> = Vec::new();
    let mut current_entity: Option<Entity> = None;

    for line in source.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        // Check for entity definition
        if line.ends_with('{') {
            if let Some(e) = current_entity.take() {
                entities.push(e);
            }
            let name = line.trim_end_matches('{').trim().to_string();
            current_entity = Some(Entity {
                name,
                attributes: Vec::new(),
            });
            continue;
        }

        // Check for entity end
        if line == "}" {
            if let Some(e) = current_entity.take() {
                entities.push(e);
            }
            continue;
        }

        // Check for attribute inside entity
        if current_entity.is_some()
            && !line.contains("||")
            && !line.contains("}")
            && !line.contains("{")
        {
            if let Some(attr) = parse_er_attribute(line) {
                if let Some(ref mut e) = current_entity {
                    e.attributes.push(attr);
                }
            }
            continue;
        }

        // Check for relationship
        if let Some(relation) = parse_er_relation(line) {
            // Ensure entities exist
            for entity_name in [&relation.from, &relation.to] {
                if !entities.iter().any(|e| &e.name == entity_name)
                    && current_entity
                        .as_ref()
                        .map(|e| &e.name != entity_name)
                        .unwrap_or(true)
                {
                    entities.push(Entity {
                        name: entity_name.clone(),
                        attributes: Vec::new(),
                    });
                }
            }
            relations.push(relation);
        }
    }

    // Save last entity
    if let Some(e) = current_entity {
        entities.push(e);
    }

    if entities.is_empty() {
        return Err("No entities found in diagram".to_string());
    }

    Ok(ERDiagram {
        entities,
        relations,
    })
}

fn parse_er_attribute(line: &str) -> Option<EntityAttribute> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    // Format: type name PK/FK or type name "comment"
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    let attr_type = parts[0].to_string();
    let name = parts[1].to_string();
    let is_pk = parts
        .get(2)
        .map(|s| s.to_uppercase() == "PK")
        .unwrap_or(false);
    let is_fk = parts
        .get(2)
        .map(|s| s.to_uppercase() == "FK")
        .unwrap_or(false);

    Some(EntityAttribute {
        name,
        attr_type,
        is_pk,
        is_fk,
    })
}

fn parse_er_relation(line: &str) -> Option<ERRelation> {
    // Patterns: ENTITY1 ||--o{ ENTITY2 : "label"
    // Cardinality markers: || (exactly one), |o (zero or one), }| (one or more), }o (zero or more)

    let relation_patterns = [
        (
            "||--||",
            Cardinality::ExactlyOne,
            Cardinality::ExactlyOne,
            true,
        ),
        (
            "||--|{",
            Cardinality::ExactlyOne,
            Cardinality::OneOrMore,
            true,
        ),
        (
            "||--o{",
            Cardinality::ExactlyOne,
            Cardinality::ZeroOrMore,
            true,
        ),
        (
            "||--o|",
            Cardinality::ExactlyOne,
            Cardinality::ZeroOrOne,
            true,
        ),
        (
            "|o--||",
            Cardinality::ZeroOrOne,
            Cardinality::ExactlyOne,
            true,
        ),
        (
            "|o--|{",
            Cardinality::ZeroOrOne,
            Cardinality::OneOrMore,
            true,
        ),
        (
            "|o--o{",
            Cardinality::ZeroOrOne,
            Cardinality::ZeroOrMore,
            true,
        ),
        (
            "|o--o|",
            Cardinality::ZeroOrOne,
            Cardinality::ZeroOrOne,
            true,
        ),
        (
            "}|--||",
            Cardinality::OneOrMore,
            Cardinality::ExactlyOne,
            true,
        ),
        (
            "}|--|{",
            Cardinality::OneOrMore,
            Cardinality::OneOrMore,
            true,
        ),
        (
            "}|--o{",
            Cardinality::OneOrMore,
            Cardinality::ZeroOrMore,
            true,
        ),
        (
            "}|--o|",
            Cardinality::OneOrMore,
            Cardinality::ZeroOrOne,
            true,
        ),
        (
            "}o--||",
            Cardinality::ZeroOrMore,
            Cardinality::ExactlyOne,
            true,
        ),
        (
            "}o--|{",
            Cardinality::ZeroOrMore,
            Cardinality::OneOrMore,
            true,
        ),
        (
            "}o--o{",
            Cardinality::ZeroOrMore,
            Cardinality::ZeroOrMore,
            true,
        ),
        (
            "}o--o|",
            Cardinality::ZeroOrMore,
            Cardinality::ZeroOrOne,
            true,
        ),
        // Dashed variants (non-identifying)
        (
            "||..||",
            Cardinality::ExactlyOne,
            Cardinality::ExactlyOne,
            false,
        ),
        (
            "||..|{",
            Cardinality::ExactlyOne,
            Cardinality::OneOrMore,
            false,
        ),
        (
            "||..o{",
            Cardinality::ExactlyOne,
            Cardinality::ZeroOrMore,
            false,
        ),
        (
            "||..o|",
            Cardinality::ExactlyOne,
            Cardinality::ZeroOrOne,
            false,
        ),
    ];

    for (pattern, from_card, to_card, identifying) in &relation_patterns {
        if line.contains(pattern) {
            let parts: Vec<&str> = line.split(pattern).collect();
            if parts.len() >= 2 {
                let from = parts[0].trim().to_string();
                let rest = parts[1].trim();

                // Extract entity name and label
                let (to, label) = if rest.contains(':') {
                    let label_parts: Vec<&str> = rest.splitn(2, ':').collect();
                    (
                        label_parts[0].trim().to_string(),
                        Some(label_parts[1].trim().trim_matches('"').to_string()),
                    )
                } else {
                    (rest.to_string(), None)
                };

                return Some(ERRelation {
                    from,
                    to,
                    from_cardinality: from_card.clone(),
                    to_cardinality: to_card.clone(),
                    label,
                    identifying: *identifying,
                });
            }
        }
    }
    None
}

/// Render an ER diagram to the UI.
pub fn render_er_diagram(ui: &mut Ui, diagram: &ERDiagram, dark_mode: bool, font_size: f32) {
    let margin = 30.0_f32;
    let entity_min_width = 140.0_f32;
    let attr_height = font_size + 4.0;
    let header_height = font_size + 12.0;
    let spacing = Vec2::new(80.0, 60.0);
    let attr_font_size = font_size - 2.0;
    let text_width_factor = 1.15;
    let name_padding = 30.0;
    let attr_padding = 30.0;
    let label_font_size = font_size - 2.0;

    // Pre-measure entity sizes and relation labels
    let entity_sizes: HashMap<String, Vec2> = {
        let text_measurer = EguiTextMeasurer::new(ui);
        diagram
            .entities
            .iter()
            .map(|entity| {
                // Measure entity name
                let name_size = text_measurer.measure(&entity.name, font_size);
                let name_width = name_size.width * text_width_factor + name_padding;

                // Measure all attributes
                let max_attr_width = entity
                    .attributes
                    .iter()
                    .map(|a| {
                        let attr_text = format!("{} {}", a.attr_type, a.name);
                        let size = text_measurer.measure(&attr_text, attr_font_size);
                        size.width * text_width_factor + attr_padding
                    })
                    .fold(0.0_f32, f32::max);

                let width = name_width.max(max_attr_width).max(entity_min_width);
                let height =
                    header_height + entity.attributes.len().max(1) as f32 * attr_height + 10.0;

                (entity.name.clone(), Vec2::new(width, height))
            })
            .collect()
    };

    // Pre-measure relation labels
    let relation_labels: HashMap<usize, (String, Vec2)> = {
        let text_measurer = EguiTextMeasurer::new(ui);
        diagram
            .relations
            .iter()
            .enumerate()
            .filter_map(|(idx, rel)| {
                rel.label.as_ref().map(|label| {
                    let size = text_measurer.measure(label, label_font_size);
                    let label_size =
                        Vec2::new(size.width * text_width_factor + 16.0, size.height + 8.0);
                    (idx, (label.clone(), label_size))
                })
            })
            .collect()
    };

    // Layout entities
    let entities_per_row = 3.max((diagram.entities.len() as f32).sqrt().ceil() as usize);
    let mut entity_pos: HashMap<String, Pos2> = HashMap::new();
    let mut max_x = 0.0_f32;
    let mut max_y = 0.0_f32;
    let mut row_height = 0.0_f32;
    let mut x = margin;
    let mut y = margin;

    for (i, entity) in diagram.entities.iter().enumerate() {
        let size = entity_sizes
            .get(&entity.name)
            .copied()
            .unwrap_or(Vec2::new(entity_min_width, 80.0));

        if i > 0 && i % entities_per_row == 0 {
            x = margin;
            y += row_height + spacing.y;
            row_height = 0.0;
        }

        entity_pos.insert(entity.name.clone(), Pos2::new(x, y));
        max_x = max_x.max(x + size.x);
        max_y = max_y.max(y + size.y);
        row_height = row_height.max(size.y);
        x += size.x + spacing.x;
    }

    let total_width = max_x + margin;
    let total_height = max_y + margin;

    // Colors
    let (entity_fill, entity_stroke, header_fill, text_color, line_color, pk_color) = if dark_mode {
        (
            Color32::from_rgb(40, 48, 60),
            Color32::from_rgb(100, 160, 140),
            Color32::from_rgb(50, 70, 65),
            Color32::from_rgb(220, 230, 240),
            Color32::from_rgb(100, 150, 130),
            Color32::from_rgb(220, 180, 80),
        )
    } else {
        (
            Color32::from_rgb(255, 255, 255),
            Color32::from_rgb(60, 140, 100),
            Color32::from_rgb(220, 245, 230),
            Color32::from_rgb(30, 40, 50),
            Color32::from_rgb(60, 120, 90),
            Color32::from_rgb(200, 150, 50),
        )
    };

    let (response, painter) = ui.allocate_painter(
        Vec2::new(total_width.max(300.0), total_height.max(100.0)),
        egui::Sense::hover(),
    );
    let offset = response.rect.min.to_vec2();

    // Draw relations first
    for (rel_idx, relation) in diagram.relations.iter().enumerate() {
        if let (Some(&from_pos), Some(&to_pos)) =
            (entity_pos.get(&relation.from), entity_pos.get(&relation.to))
        {
            let from_size = entity_sizes
                .get(&relation.from)
                .copied()
                .unwrap_or(Vec2::new(100.0, 80.0));
            let to_size = entity_sizes
                .get(&relation.to)
                .copied()
                .unwrap_or(Vec2::new(100.0, 80.0));

            let from_center = from_pos + from_size / 2.0 + offset;
            let to_center = to_pos + to_size / 2.0 + offset;

            let dir = (to_center - from_center).normalized();
            let start = from_center + dir * (from_size.x / 2.0).min(from_size.y / 2.0);
            let end = to_center - dir * (to_size.x / 2.0).min(to_size.y / 2.0);

            // Draw line
            if relation.identifying {
                painter.line_segment([start, end], Stroke::new(1.5, line_color));
            } else {
                draw_dashed_line(&painter, start, end, Stroke::new(1.5, line_color), 6.0, 4.0);
            }

            // Draw cardinality markers
            let marker_size = 8.0;
            let perp = Vec2::new(-dir.y, dir.x);

            // From side marker
            draw_cardinality_marker(
                &painter,
                start,
                dir,
                perp,
                &relation.from_cardinality,
                marker_size,
                line_color,
            );

            // To side marker
            draw_cardinality_marker(
                &painter,
                end,
                -dir,
                -perp,
                &relation.to_cardinality,
                marker_size,
                line_color,
            );

            // Draw label using pre-measured size
            if let Some((label_text, label_size)) = relation_labels.get(&rel_idx) {
                let mid = Pos2::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);
                let label_bg = if dark_mode {
                    Color32::from_rgb(35, 40, 50)
                } else {
                    Color32::from_rgb(255, 255, 255)
                };
                let label_rect = Rect::from_center_size(mid, *label_size);
                painter.rect_filled(label_rect, 2.0, label_bg);
                painter.text(
                    mid,
                    egui::Align2::CENTER_CENTER,
                    label_text,
                    FontId::proportional(label_font_size),
                    text_color,
                );
            }
        }
    }

    // Draw entities
    for entity in &diagram.entities {
        if let Some(&pos) = entity_pos.get(&entity.name) {
            let size = entity_sizes
                .get(&entity.name)
                .copied()
                .unwrap_or(Vec2::new(100.0, 80.0));
            let rect = Rect::from_min_size(pos + offset, size);

            // Draw box
            painter.rect(
                rect,
                Rounding::same(4.0),
                entity_fill,
                Stroke::new(2.0, entity_stroke),
            );

            // Draw header
            let header_rect = Rect::from_min_size(rect.min, Vec2::new(size.x, header_height));
            painter.rect_filled(
                header_rect,
                Rounding {
                    nw: 4.0,
                    ne: 4.0,
                    sw: 0.0,
                    se: 0.0,
                },
                header_fill,
            );

            // Draw entity name
            painter.text(
                Pos2::new(rect.center().x, rect.min.y + header_height / 2.0),
                egui::Align2::CENTER_CENTER,
                &entity.name,
                FontId::proportional(font_size),
                text_color,
            );

            // Draw separator
            let sep_y = rect.min.y + header_height;
            painter.line_segment(
                [Pos2::new(rect.min.x, sep_y), Pos2::new(rect.max.x, sep_y)],
                Stroke::new(1.0, entity_stroke),
            );

            // Draw attributes
            let mut y = sep_y + 4.0;
            for attr in &entity.attributes {
                let color = if attr.is_pk { pk_color } else { text_color };
                let prefix = if attr.is_pk {
                    "🔑 "
                } else if attr.is_fk {
                    "🔗 "
                } else {
                    ""
                };
                let text = format!("{}{} {}", prefix, attr.attr_type, attr.name);

                painter.text(
                    Pos2::new(rect.min.x + 8.0, y + attr_height * 0.4),
                    egui::Align2::LEFT_CENTER,
                    text,
                    FontId::proportional(font_size - 2.0),
                    color,
                );
                y += attr_height;
            }
        }
    }
}

fn draw_cardinality_marker(
    painter: &egui::Painter,
    pos: Pos2,
    dir: Vec2,
    perp: Vec2,
    cardinality: &Cardinality,
    size: f32,
    color: Color32,
) {
    match cardinality {
        Cardinality::ExactlyOne => {
            // Two vertical lines ||
            let p1a = pos + perp * size * 0.4;
            let p1b = pos - perp * size * 0.4;
            let p2a = pos + dir * 4.0 + perp * size * 0.4;
            let p2b = pos + dir * 4.0 - perp * size * 0.4;
            painter.line_segment([p1a, p1b], Stroke::new(2.0, color));
            painter.line_segment([p2a, p2b], Stroke::new(2.0, color));
        }
        Cardinality::ZeroOrOne => {
            // Circle and line |o
            let line_p1 = pos + perp * size * 0.4;
            let line_p2 = pos - perp * size * 0.4;
            painter.line_segment([line_p1, line_p2], Stroke::new(2.0, color));
            painter.circle_stroke(pos + dir * 8.0, 4.0, Stroke::new(2.0, color));
        }
        Cardinality::ZeroOrMore => {
            // Circle and crow's foot }o
            painter.circle_stroke(pos + dir * 4.0, 4.0, Stroke::new(2.0, color));
            let foot_start = pos + dir * 12.0;
            painter.line_segment(
                [foot_start, pos + perp * size * 0.5],
                Stroke::new(1.5, color),
            );
            painter.line_segment(
                [foot_start, pos - perp * size * 0.5],
                Stroke::new(1.5, color),
            );
            painter.line_segment([foot_start, pos], Stroke::new(1.5, color));
        }
        Cardinality::OneOrMore => {
            // Line and crow's foot }|
            let line_p1 = pos + perp * size * 0.4;
            let line_p2 = pos - perp * size * 0.4;
            painter.line_segment([line_p1, line_p2], Stroke::new(2.0, color));
            let foot_start = pos + dir * 8.0;
            painter.line_segment(
                [foot_start, pos + perp * size * 0.5],
                Stroke::new(1.5, color),
            );
            painter.line_segment(
                [foot_start, pos - perp * size * 0.5],
                Stroke::new(1.5, color),
            );
            painter.line_segment([foot_start, pos], Stroke::new(1.5, color));
        }
    }
}
