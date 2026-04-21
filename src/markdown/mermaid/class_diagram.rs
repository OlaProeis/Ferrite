//! Class diagram parsing and rendering.

use egui::{Color32, FontId, Pos2, Rect, Rounding, Stroke, Ui, Vec2};
use std::collections::HashMap;

use super::text::{EguiTextMeasurer, TextMeasurer};
use super::utils::draw_dashed_line;

/// Visibility modifier for class members.
#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    Public,    // +
    Private,   // -
    Protected, // #
    Package,   // ~
}

/// A member of a class (attribute or method).
#[derive(Debug, Clone)]
pub struct ClassMember {
    pub visibility: Visibility,
    pub name: String,
    pub member_type: String, // return type or attribute type
    pub is_method: bool,
}

/// A class in the diagram.
#[derive(Debug, Clone)]
pub struct Class {
    pub id: String,
    pub name: String,
    pub stereotype: Option<String>, // <<interface>>, <<abstract>>, etc.
    pub attributes: Vec<ClassMember>,
    pub methods: Vec<ClassMember>,
}

/// Relationship type between classes.
#[derive(Debug, Clone, PartialEq)]
pub enum ClassRelationType {
    Inheritance, // --|>
    Composition, // *--
    Aggregation, // o--
    Association, // --
    Dependency,  // ..>
    Realization, // ..|>
}

/// A relationship between classes.
#[derive(Debug, Clone)]
pub struct ClassRelation {
    pub from: String,
    pub to: String,
    pub relation_type: ClassRelationType,
    pub label: Option<String>,
    pub from_cardinality: Option<String>,
    pub to_cardinality: Option<String>,
}

/// A class diagram.
#[derive(Debug, Clone)]
pub struct ClassDiagram {
    pub classes: Vec<Class>,
    pub relations: Vec<ClassRelation>,
}

/// Parse a class diagram from source.
pub fn parse_class_diagram(source: &str) -> Result<ClassDiagram, String> {
    let mut classes: Vec<Class> = Vec::new();
    let mut relations: Vec<ClassRelation> = Vec::new();
    let mut current_class: Option<Class> = None;

    for line in source.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        // Check for class definition start: class ClassName { or class ClassName
        if line.starts_with("class ") {
            // Save previous class if any
            if let Some(c) = current_class.take() {
                classes.push(c);
            }

            let rest = line[6..].trim();
            let (name, stereotype) = if rest.contains("~") {
                // class Animal~T~ for generics
                let parts: Vec<&str> = rest.splitn(2, '~').collect();
                (parts[0].trim().to_string(), None)
            } else if rest.contains("<<") && rest.contains(">>") {
                // class Interface <<interface>>
                let start = rest.find("<<").unwrap();
                let end = rest.find(">>").unwrap();
                let name = rest[..start]
                    .trim()
                    .trim_end_matches('{')
                    .trim()
                    .to_string();
                let stereo = rest[start + 2..end].trim().to_string();
                (name, Some(stereo))
            } else {
                (rest.trim_end_matches('{').trim().to_string(), None)
            };

            current_class = Some(Class {
                id: name.clone(),
                name,
                stereotype,
                attributes: Vec::new(),
                methods: Vec::new(),
            });
            continue;
        }

        // Check for class definition end
        if line == "}" {
            if let Some(c) = current_class.take() {
                classes.push(c);
            }
            continue;
        }

        // Check for member definition inside class
        if current_class.is_some() && !line.contains("--") && !line.contains("..") {
            if let Some(member) = parse_class_member(line) {
                if let Some(ref mut c) = current_class {
                    if member.is_method {
                        c.methods.push(member);
                    } else {
                        c.attributes.push(member);
                    }
                }
            }
            continue;
        }

        // Check for relationship
        if let Some(relation) = parse_class_relation(line) {
            // Ensure classes exist
            for class_id in [&relation.from, &relation.to] {
                if !classes.iter().any(|c| &c.id == class_id)
                    && current_class
                        .as_ref()
                        .map(|c| &c.id != class_id)
                        .unwrap_or(true)
                {
                    classes.push(Class {
                        id: class_id.clone(),
                        name: class_id.clone(),
                        stereotype: None,
                        attributes: Vec::new(),
                        methods: Vec::new(),
                    });
                }
            }
            relations.push(relation);
        }
    }

    // Save last class
    if let Some(c) = current_class {
        classes.push(c);
    }

    if classes.is_empty() {
        return Err("No classes found in diagram".to_string());
    }

    Ok(ClassDiagram { classes, relations })
}

fn parse_class_member(line: &str) -> Option<ClassMember> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    let (visibility, rest) = if line.starts_with('+') {
        (Visibility::Public, &line[1..])
    } else if line.starts_with('-') {
        (Visibility::Private, &line[1..])
    } else if line.starts_with('#') {
        (Visibility::Protected, &line[1..])
    } else if line.starts_with('~') {
        (Visibility::Package, &line[1..])
    } else {
        (Visibility::Public, line)
    };

    let is_method = rest.contains('(');

    // Parse type and name
    let (name, member_type) = if is_method {
        // method() ReturnType or method(): ReturnType
        let paren_idx = rest.find('(')?;
        let name_part = rest[..paren_idx].trim().to_string();
        let type_part = if rest.contains(':') {
            rest.rsplit(':').next().unwrap_or("void").trim().to_string()
        } else if rest.contains(')') {
            let after_paren = rest.rfind(')')?;
            rest[after_paren + 1..].trim().to_string()
        } else {
            "void".to_string()
        };
        (name_part, type_part)
    } else {
        // attribute: Type or Type attribute
        if rest.contains(':') {
            let parts: Vec<&str> = rest.splitn(2, ':').collect();
            (
                parts[0].trim().to_string(),
                parts.get(1).map(|s| s.trim()).unwrap_or("").to_string(),
            )
        } else {
            (rest.trim().to_string(), String::new())
        }
    };

    Some(ClassMember {
        visibility,
        name,
        member_type,
        is_method,
    })
}

fn parse_class_relation(line: &str) -> Option<ClassRelation> {
    // Patterns: A --|> B, A *-- B, A o-- B, A -- B, A ..> B, A ..|> B
    // With optional labels: A "label" --|> "label2" B

    let relation_patterns = [
        ("--|>", ClassRelationType::Inheritance),
        ("<|--", ClassRelationType::Inheritance),
        ("..|>", ClassRelationType::Realization),
        ("<|..", ClassRelationType::Realization),
        ("*--", ClassRelationType::Composition),
        ("--*", ClassRelationType::Composition),
        ("o--", ClassRelationType::Aggregation),
        ("--o", ClassRelationType::Aggregation),
        ("..>", ClassRelationType::Dependency),
        ("<..", ClassRelationType::Dependency),
        ("--", ClassRelationType::Association),
        ("..", ClassRelationType::Dependency),
    ];

    for (pattern, rel_type) in &relation_patterns {
        if line.contains(pattern) {
            let parts: Vec<&str> = line.split(pattern).collect();
            if parts.len() >= 2 {
                let from = parts[0]
                    .trim()
                    .trim_matches('"')
                    .split_whitespace()
                    .last()?
                    .to_string();
                let to = parts[1]
                    .trim()
                    .trim_matches('"')
                    .split_whitespace()
                    .next()?
                    .to_string();

                // Extract label if present (between quotes after relation)
                let label = if parts[1].contains(':') {
                    Some(parts[1].split(':').last()?.trim().to_string())
                } else {
                    None
                };

                return Some(ClassRelation {
                    from,
                    to,
                    relation_type: rel_type.clone(),
                    label,
                    from_cardinality: None,
                    to_cardinality: None,
                });
            }
        }
    }
    None
}

/// Render a class diagram to the UI.
pub fn render_class_diagram(ui: &mut Ui, diagram: &ClassDiagram, dark_mode: bool, font_size: f32) {
    let margin = 30.0_f32;
    let class_min_width = 120.0_f32;
    let member_height = font_size + 4.0;
    let header_height = font_size + 10.0;
    let spacing = Vec2::new(60.0, 50.0);
    let member_font_size = font_size - 2.0;
    let text_width_factor = 1.15;
    let name_padding = 24.0;
    let member_padding = 24.0;

    // Pre-measure class names and member text
    let class_sizes: HashMap<String, Vec2> = {
        let text_measurer = EguiTextMeasurer::new(ui);
        diagram
            .classes
            .iter()
            .map(|class| {
                // Measure class name
                let name_size = text_measurer.measure(&class.name, font_size);
                let name_width = name_size.width * text_width_factor + name_padding;

                // Measure all members (attributes and methods)
                let max_member_width = class
                    .attributes
                    .iter()
                    .chain(class.methods.iter())
                    .map(|m| {
                        let member_text = format!("{}: {}", m.name, m.member_type);
                        let size = text_measurer.measure(&member_text, member_font_size);
                        size.width * text_width_factor + member_padding
                    })
                    .fold(0.0_f32, f32::max);

                let width = name_width.max(max_member_width).max(class_min_width);
                let height = header_height
                    + (class.attributes.len().max(1) as f32 * member_height)
                    + (class.methods.len().max(1) as f32 * member_height)
                    + 10.0;

                (class.id.clone(), Vec2::new(width, height))
            })
            .collect()
    };

    // Layout classes in grid
    let classes_per_row = 3.max((diagram.classes.len() as f32).sqrt().ceil() as usize);
    let mut class_pos: HashMap<String, Pos2> = HashMap::new();
    let mut max_x = 0.0_f32;
    let mut max_y = 0.0_f32;
    let mut row_height = 0.0_f32;
    let mut x = margin;
    let mut y = margin;

    for (i, class) in diagram.classes.iter().enumerate() {
        let size = class_sizes
            .get(&class.id)
            .copied()
            .unwrap_or(Vec2::new(class_min_width, 80.0));

        if i > 0 && i % classes_per_row == 0 {
            x = margin;
            y += row_height + spacing.y;
            row_height = 0.0;
        }

        class_pos.insert(class.id.clone(), Pos2::new(x, y));
        max_x = max_x.max(x + size.x);
        max_y = max_y.max(y + size.y);
        row_height = row_height.max(size.y);
        x += size.x + spacing.x;
    }

    let total_width = max_x + margin;
    let total_height = max_y + margin;

    // Colors
    let (class_fill, class_stroke, header_fill, text_color, line_color) = if dark_mode {
        (
            Color32::from_rgb(40, 48, 60),
            Color32::from_rgb(100, 140, 180),
            Color32::from_rgb(55, 70, 90),
            Color32::from_rgb(220, 230, 240),
            Color32::from_rgb(120, 150, 180),
        )
    } else {
        (
            Color32::from_rgb(255, 255, 255),
            Color32::from_rgb(100, 140, 180),
            Color32::from_rgb(230, 240, 250),
            Color32::from_rgb(30, 40, 50),
            Color32::from_rgb(100, 130, 160),
        )
    };

    let (response, painter) = ui.allocate_painter(
        Vec2::new(total_width.max(300.0), total_height.max(100.0)),
        egui::Sense::hover(),
    );
    let offset = response.rect.min.to_vec2();

    // Draw relations first
    for relation in &diagram.relations {
        if let (Some(&from_pos), Some(&to_pos)) =
            (class_pos.get(&relation.from), class_pos.get(&relation.to))
        {
            let from_size = class_sizes
                .get(&relation.from)
                .copied()
                .unwrap_or(Vec2::new(100.0, 80.0));
            let to_size = class_sizes
                .get(&relation.to)
                .copied()
                .unwrap_or(Vec2::new(100.0, 80.0));

            let from_center = from_pos + from_size / 2.0 + offset;
            let to_center = to_pos + to_size / 2.0 + offset;

            let dir = (to_center - from_center).normalized();
            let start = from_center + dir * (from_size.x / 2.0).min(from_size.y / 2.0);
            let end = to_center - dir * (to_size.x / 2.0).min(to_size.y / 2.0);

            // Draw line based on type
            let is_dashed = matches!(
                relation.relation_type,
                ClassRelationType::Dependency | ClassRelationType::Realization
            );

            if is_dashed {
                draw_dashed_line(&painter, start, end, Stroke::new(1.5, line_color), 6.0, 4.0);
            } else {
                painter.line_segment([start, end], Stroke::new(1.5, line_color));
            }

            // Draw arrow/decoration at end
            let arrow_size = 10.0;
            let perp = Vec2::new(-dir.y, dir.x);

            match relation.relation_type {
                ClassRelationType::Inheritance | ClassRelationType::Realization => {
                    // Empty triangle
                    let tip = end;
                    let left = end - dir * arrow_size + perp * (arrow_size * 0.5);
                    let right = end - dir * arrow_size - perp * (arrow_size * 0.5);
                    painter.add(egui::Shape::convex_polygon(
                        vec![tip, left, right],
                        if dark_mode {
                            Color32::from_rgb(40, 48, 60)
                        } else {
                            Color32::WHITE
                        },
                        Stroke::new(1.5, line_color),
                    ));
                }
                ClassRelationType::Composition => {
                    // Filled diamond at start
                    let diamond_size = 8.0;
                    let d_center = start + dir * diamond_size;
                    painter.add(egui::Shape::convex_polygon(
                        vec![
                            start,
                            d_center + perp * (diamond_size * 0.5),
                            start + dir * diamond_size * 2.0,
                            d_center - perp * (diamond_size * 0.5),
                        ],
                        line_color,
                        Stroke::NONE,
                    ));
                }
                ClassRelationType::Aggregation => {
                    // Empty diamond at start
                    let diamond_size = 8.0;
                    let d_center = start + dir * diamond_size;
                    painter.add(egui::Shape::convex_polygon(
                        vec![
                            start,
                            d_center + perp * (diamond_size * 0.5),
                            start + dir * diamond_size * 2.0,
                            d_center - perp * (diamond_size * 0.5),
                        ],
                        if dark_mode {
                            Color32::from_rgb(40, 48, 60)
                        } else {
                            Color32::WHITE
                        },
                        Stroke::new(1.5, line_color),
                    ));
                }
                ClassRelationType::Dependency => {
                    // Simple arrow
                    let left = end - dir * arrow_size + perp * (arrow_size * 0.4);
                    let right = end - dir * arrow_size - perp * (arrow_size * 0.4);
                    painter.line_segment([left, end], Stroke::new(1.5, line_color));
                    painter.line_segment([right, end], Stroke::new(1.5, line_color));
                }
                ClassRelationType::Association => {
                    // Simple line, no decoration
                }
            }

            // Draw label
            if let Some(label) = &relation.label {
                let mid = Pos2::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);
                painter.text(
                    mid,
                    egui::Align2::CENTER_CENTER,
                    label,
                    FontId::proportional(font_size - 2.0),
                    text_color,
                );
            }
        }
    }

    // Draw classes
    for class in &diagram.classes {
        if let Some(&pos) = class_pos.get(&class.id) {
            let size = class_sizes
                .get(&class.id)
                .copied()
                .unwrap_or(Vec2::new(100.0, 80.0));
            let rect = Rect::from_min_size(pos + offset, size);

            // Draw box
            painter.rect(
                rect,
                Rounding::same(4.0),
                class_fill,
                Stroke::new(2.0, class_stroke),
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

            // Draw stereotype
            let mut text_y = rect.min.y + 4.0;
            if let Some(stereo) = &class.stereotype {
                painter.text(
                    Pos2::new(rect.center().x, text_y + font_size * 0.35),
                    egui::Align2::CENTER_CENTER,
                    format!("<<{}>>", stereo),
                    FontId::proportional(font_size - 3.0),
                    text_color.gamma_multiply(0.7),
                );
                text_y += font_size * 0.7;
            }

            // Draw class name
            painter.text(
                Pos2::new(rect.center().x, text_y + font_size * 0.5),
                egui::Align2::CENTER_CENTER,
                &class.name,
                FontId::proportional(font_size),
                text_color,
            );

            // Draw separator after header
            let sep_y = rect.min.y + header_height;
            painter.line_segment(
                [Pos2::new(rect.min.x, sep_y), Pos2::new(rect.max.x, sep_y)],
                Stroke::new(1.0, class_stroke),
            );

            // Draw attributes
            let mut y = sep_y + 4.0;
            for attr in &class.attributes {
                let vis_char = match attr.visibility {
                    Visibility::Public => "+",
                    Visibility::Private => "-",
                    Visibility::Protected => "#",
                    Visibility::Package => "~",
                };
                let text = if attr.member_type.is_empty() {
                    format!("{} {}", vis_char, attr.name)
                } else {
                    format!("{} {}: {}", vis_char, attr.name, attr.member_type)
                };
                painter.text(
                    Pos2::new(rect.min.x + 6.0, y + member_height * 0.4),
                    egui::Align2::LEFT_CENTER,
                    text,
                    FontId::proportional(font_size - 2.0),
                    text_color,
                );
                y += member_height;
            }
            if class.attributes.is_empty() {
                y += member_height;
            }

            // Draw separator before methods
            painter.line_segment(
                [Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)],
                Stroke::new(1.0, class_stroke),
            );
            y += 4.0;

            // Draw methods
            for method in &class.methods {
                let vis_char = match method.visibility {
                    Visibility::Public => "+",
                    Visibility::Private => "-",
                    Visibility::Protected => "#",
                    Visibility::Package => "~",
                };
                let text = if method.member_type.is_empty() || method.member_type == "void" {
                    format!("{} {}()", vis_char, method.name)
                } else {
                    format!("{} {}(): {}", vis_char, method.name, method.member_type)
                };
                painter.text(
                    Pos2::new(rect.min.x + 6.0, y + member_height * 0.4),
                    egui::Align2::LEFT_CENTER,
                    text,
                    FontId::proportional(font_size - 2.0),
                    text_color,
                );
                y += member_height;
            }
        }
    }
}
