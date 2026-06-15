//! Git graph painting: lanes, connectors, commit dots, and labels.

use egui::{
    Align2, Color32, CornerRadius, FontId, Painter, Pos2, Rect, Sense, Stroke, StrokeKind, Ui,
    Vec2,
};

use super::layout::{layout_git_graph, GitGraphBranchLine, GitGraphLayout, GitGraphLayoutConfig};
use super::{GitCommitKind, GitGraph, GitGraphOrientation};
use crate::markdown::mermaid::text::{EguiTextMeasurer, TextMeasurer};
use crate::markdown::mermaid::utils::draw_dashed_line;

/// Branch / label colors for git graph rendering.
struct GitGraphColors {
    branch: Vec<Color32>,
    text: Color32,
    label_bg: Color32,
    dot_outline: Color32,
    tag_bg: Color32,
    tag_text: Color32,
}

impl GitGraphColors {
    fn new(dark_mode: bool) -> Self {
        let branch = if dark_mode {
            vec![
                Color32::from_rgb(100, 180, 100),
                Color32::from_rgb(100, 150, 220),
                Color32::from_rgb(220, 160, 100),
                Color32::from_rgb(180, 100, 180),
                Color32::from_rgb(220, 100, 100),
                Color32::from_rgb(100, 200, 200),
            ]
        } else {
            vec![
                Color32::from_rgb(60, 140, 60),
                Color32::from_rgb(60, 110, 180),
                Color32::from_rgb(180, 120, 60),
                Color32::from_rgb(140, 60, 140),
                Color32::from_rgb(180, 60, 60),
                Color32::from_rgb(60, 160, 160),
            ]
        };

        Self {
            branch,
            text: if dark_mode {
                Color32::from_rgb(220, 230, 240)
            } else {
                Color32::from_rgb(30, 40, 50)
            },
            label_bg: if dark_mode {
                Color32::from_rgb(50, 55, 65)
            } else {
                Color32::from_rgb(240, 245, 250)
            },
            dot_outline: if dark_mode {
                Color32::WHITE
            } else {
                Color32::BLACK
            },
            tag_bg: if dark_mode {
                Color32::from_rgb(70, 75, 90)
            } else {
                Color32::from_rgb(230, 235, 245)
            },
            tag_text: if dark_mode {
                Color32::from_rgb(240, 240, 250)
            } else {
                Color32::from_rgb(40, 50, 70)
            },
        }
    }

    fn branch_color(&self, color_idx: usize) -> Color32 {
        self.branch[color_idx % self.branch.len()]
    }
}

struct LabelHover {
    rect: Rect,
    tooltip: String,
}

struct PreparedCommitLabel {
    display: String,
    tooltip: Option<String>,
    local_pos: Pos2,
    align: Align2,
    text_dims: Vec2,
}

struct PreparedBranchLabel {
    display: String,
    tooltip: Option<String>,
    local_anchor: Pos2,
    align: Align2,
    text_dims: Vec2,
    color: Color32,
}

/// Render a git graph to the UI using the lane layout from task 6.
pub fn render_git_graph(ui: &mut Ui, graph: &GitGraph, dark_mode: bool, font_size: f32) {
    let config = GitGraphLayoutConfig::default();
    let layout = layout_git_graph(graph, config);
    let colors = GitGraphColors::new(dark_mode);
    let label_font = font_size - 2.0;

    let (commit_labels, branch_labels) = {
        let text_measurer = EguiTextMeasurer::new(ui);
        let max_label_width = config.commit_spacing * 0.85;
        let max_branch_width = config.margin - 6.0;

        let commit_labels: Vec<PreparedCommitLabel> = layout
            .commits
            .iter()
            .map(|commit_layout| {
                let commit = &graph.commits[commit_layout.commit_index];
                let label_text = commit.message.as_ref().unwrap_or(&commit.id);
                let display =
                    text_measurer.truncate_with_ellipsis(label_text, label_font, max_label_width);
                let tooltip = if display == *label_text {
                    None
                } else {
                    Some(label_text.clone())
                };
                let (local_pos, align) = commit_label_placement(
                    commit_layout.pos,
                    commit_layout.lane,
                    graph.orientation,
                    &config,
                    label_font,
                );
                let text_size = text_measurer.measure(&display, label_font);
                PreparedCommitLabel {
                    display,
                    tooltip,
                    local_pos,
                    align,
                    text_dims: Vec2::new(text_size.width, text_size.height),
                }
            })
            .collect();

        let branch_labels: Vec<PreparedBranchLabel> = graph
            .branches
            .iter()
            .filter_map(|branch| {
                layout.branch_lanes.get(&branch.name)?;
                let display =
                    text_measurer.truncate_with_ellipsis(&branch.name, label_font, max_branch_width);
                let tooltip = if display == branch.name {
                    None
                } else {
                    Some(branch.name.clone())
                };
                let lane = *layout.branch_lanes.get(&branch.name).unwrap_or(&0);
                let lane_y = config.margin + lane as f32 * config.lane_spacing;
                let (local_anchor, align) = match graph.orientation {
                    GitGraphOrientation::Lr => (Pos2::new(4.0, lane_y), Align2::LEFT_CENTER),
                    GitGraphOrientation::Tb => (Pos2::new(lane_y, 4.0), Align2::CENTER_TOP),
                };
                let text_size = text_measurer.measure(&display, label_font);
                Some(PreparedBranchLabel {
                    display,
                    tooltip,
                    local_anchor,
                    align,
                    text_dims: Vec2::new(text_size.width, text_size.height),
                    color: colors.branch_color(branch.color_idx),
                })
            })
            .collect();

        (commit_labels, branch_labels)
    };

    let alloc_size = Vec2::new(
        layout.bounds.x.max(300.0),
        layout.bounds.y.max(100.0),
    );

    let (response, painter) = ui.allocate_painter(alloc_size, Sense::hover());
    let offset = response.rect.min.to_vec2();

    let mut hovers: Vec<LabelHover> = Vec::new();

    draw_branch_lines(
        &painter,
        graph,
        &layout,
        &colors,
        offset,
        config.commit_radius,
    );
    draw_merge_connectors(
        &painter,
        &layout,
        graph,
        &colors,
        offset,
        config.commit_radius,
    );
    draw_cherry_pick_connectors(
        &painter,
        &layout,
        graph,
        &colors,
        offset,
        config.commit_radius,
    );

    for (commit_layout, label) in layout.commits.iter().zip(commit_labels.iter()) {
        let commit = &graph.commits[commit_layout.commit_index];
        let branch = graph.branches.iter().find(|b| b.name == commit.branch);
        let color = colors.branch_color(branch.map(|b| b.color_idx).unwrap_or(0));
        let pos = commit_layout.pos + offset;

        draw_commit_dot(
            &painter,
            pos,
            config.commit_radius,
            commit,
            color,
            &colors,
        );

        if let Some(ref tag) = commit.tag {
            draw_tag_label(&painter, pos, tag, config.commit_radius, label_font, &colors);
        }

        let label_pos = label.local_pos + offset;
        let label_rect = label_rect(label_pos, label.text_dims, label.align);

        if let Some(tooltip) = &label.tooltip {
            hovers.push(LabelHover {
                rect: label_rect,
                tooltip: tooltip.clone(),
            });
        }

        painter.rect_filled(label_rect, 3.0, colors.label_bg);
        painter.text(
            label_pos,
            label.align,
            &label.display,
            FontId::proportional(label_font),
            colors.text,
        );
    }

    for label in &branch_labels {
        let anchor = label.local_anchor + offset;
        let rect = match graph.orientation {
            GitGraphOrientation::Lr => Rect::from_min_size(
                Pos2::new(anchor.x, anchor.y - label.text_dims.y * 0.5),
                label.text_dims,
            ),
            GitGraphOrientation::Tb => Rect::from_min_size(
                Pos2::new(anchor.x - label.text_dims.x * 0.5, anchor.y),
                label.text_dims,
            ),
        };

        if let Some(tooltip) = &label.tooltip {
            hovers.push(LabelHover {
                rect: rect.intersect(response.rect),
                tooltip: tooltip.clone(),
            });
        }

        painter.text(
            anchor,
            label.align,
            &label.display,
            FontId::proportional(label_font),
            label.color,
        );
    }

    for (idx, hover) in hovers.into_iter().enumerate() {
        let id = ui.id().with("git_commit_label").with(idx);
        ui.interact(hover.rect, id, Sense::hover())
            .on_hover_text(&hover.tooltip);
    }
}

fn branch_line_color(
    graph: &GitGraph,
    colors: &GitGraphColors,
    line: &GitGraphBranchLine,
) -> Color32 {
    graph
        .branches
        .iter()
        .find(|b| b.name == line.branch)
        .map(|b| colors.branch_color(b.color_idx))
        .unwrap_or(colors.branch_color(0))
}

fn lane_y_for_branch(graph: &GitGraph, layout: &GitGraphLayout, branch: &str) -> f32 {
    layout
        .commits
        .iter()
        .find(|c| graph.commits[c.commit_index].branch == branch)
        .map(|c| c.pos.y)
        .unwrap_or(0.0)
}

fn draw_branch_lines(
    painter: &Painter,
    graph: &GitGraph,
    layout: &GitGraphLayout,
    colors: &GitGraphColors,
    offset: Vec2,
    radius: f32,
) {
    let stroke_width = 3.0;

    for line in &layout.branch_lines {
        let color = branch_line_color(graph, colors, line);
        let lane_y = lane_y_for_branch(graph, layout, &line.branch);

        let commits_on_branch: Vec<Pos2> = layout
            .commits
            .iter()
            .filter(|c| graph.commits[c.commit_index].branch == line.branch)
            .map(|c| c.pos)
            .collect();

        if let (Some(first), Some(last)) = (commits_on_branch.first(), commits_on_branch.last()) {
            let start = Pos2::new(first.x, lane_y) + offset;
            let end = Pos2::new(last.x, lane_y) + offset;
            if (end.x - start.x).abs() > f32::EPSILON {
                painter.line_segment([start, end], Stroke::new(stroke_width, color));
            }
        }

        if let (Some(branch_off), Some(first)) = (line.branch_off, commits_on_branch.first()) {
            let from = shorten_toward(branch_off + offset, *first + offset, radius);
            let to = shorten_toward(*first + offset, branch_off + offset, radius);
            draw_lane_connector(painter, from, to, color, 2.0);
        }
    }
}

fn draw_merge_connectors(
    painter: &Painter,
    layout: &GitGraphLayout,
    graph: &GitGraph,
    colors: &GitGraphColors,
    offset: Vec2,
    radius: f32,
) {
    for merge in &layout.merge_connectors {
        let merge_color = graph
            .branches
            .iter()
            .find(|b| b.name == merge.source_branch)
            .map(|b| colors.branch_color(b.color_idx))
            .unwrap_or(colors.branch_color(0));

        let target = layout
            .commits
            .iter()
            .find(|c| c.commit_index == merge.target_commit_index)
            .map(|c| c.pos + offset)
            .unwrap_or(merge.target_pos + offset);

        let source = merge.source_pos + offset;
        let from = shorten_toward(source, target, radius);
        let to = shorten_toward(target, source, radius);
        draw_lane_connector(painter, from, to, merge_color, 2.0);
    }
}

fn draw_cherry_pick_connectors(
    painter: &Painter,
    layout: &GitGraphLayout,
    graph: &GitGraph,
    colors: &GitGraphColors,
    offset: Vec2,
    radius: f32,
) {
    for cp in &layout.cherry_pick_connectors {
        let commit = &graph.commits[cp.target_commit_index];
        let branch = graph.branches.iter().find(|b| b.name == commit.branch);
        let color = colors.branch_color(branch.map(|b| b.color_idx).unwrap_or(0));

        let source = cp.source_pos + offset;
        let target = cp.target_pos + offset;
        let from = shorten_toward(source, target, radius);
        let to = shorten_toward(target, source, radius);

        draw_dashed_line(
            painter,
            from,
            to,
            Stroke::new(2.0, color),
            5.0,
            3.0,
        );
    }
}

/// Three-segment connector: horizontal bar at mid-lane (LR) or mid-sequence (TB).
fn draw_lane_connector(
    painter: &Painter,
    from: Pos2,
    to: Pos2,
    color: Color32,
    width: f32,
) {
    let stroke = Stroke::new(width, color);
    let mid = Pos2::new((from.x + to.x) * 0.5, (from.y + to.y) * 0.5);
    let ctrl1 = Pos2::new(from.x, mid.y);
    let ctrl2 = Pos2::new(to.x, mid.y);
    painter.line_segment([from, ctrl1], stroke);
    painter.line_segment([ctrl1, ctrl2], stroke);
    painter.line_segment([ctrl2, to], stroke);
}

fn shorten_toward(from: Pos2, to: Pos2, distance: f32) -> Pos2 {
    let delta = to - from;
    let len = delta.length();
    if len <= distance {
        return to;
    }
    from + delta / len * (len - distance)
}

fn draw_commit_dot(
    painter: &Painter,
    pos: Pos2,
    radius: f32,
    commit: &super::GitCommit,
    color: Color32,
    colors: &GitGraphColors,
) {
    if commit.is_merge {
        painter.circle_filled(pos, radius, color);
        painter.circle_stroke(
            pos,
            radius,
            Stroke::new(2.0, colors.dot_outline),
        );
        return;
    }

    match commit.kind {
        GitCommitKind::Normal => {
            painter.circle_filled(pos, radius, color);
        }
        GitCommitKind::Reverse => {
            painter.circle_filled(pos, radius, color);
            let arm = radius * 0.65;
            let stroke = Stroke::new(2.0, colors.dot_outline);
            painter.line_segment(
                [Pos2::new(pos.x - arm, pos.y - arm), Pos2::new(pos.x + arm, pos.y + arm)],
                stroke,
            );
            painter.line_segment(
                [Pos2::new(pos.x - arm, pos.y + arm), Pos2::new(pos.x + arm, pos.y - arm)],
                stroke,
            );
        }
        GitCommitKind::Highlight => {
            painter.circle_stroke(pos, radius, Stroke::new(3.5, color));
            painter.circle_filled(pos, radius * 0.45, color);
        }
    }
}

fn draw_tag_label(
    painter: &Painter,
    dot_pos: Pos2,
    tag: &str,
    radius: f32,
    font_size: f32,
    colors: &GitGraphColors,
) {
    let padding = Vec2::new(6.0, 2.0);
    let text_size = Vec2::new(tag.len() as f32 * font_size * 0.55, font_size + 2.0);
    let size = text_size + padding * 2.0;
    let min = Pos2::new(dot_pos.x + radius + 4.0, dot_pos.y - radius - size.y - 2.0);
    let rect = Rect::from_min_size(min, size);

    painter.rect_filled(rect, CornerRadius::same(4), colors.tag_bg);
    painter.rect_stroke(
        rect,
        CornerRadius::same(4),
        Stroke::new(1.0, colors.tag_text.gamma_multiply(0.35)),
        StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        tag,
        FontId::proportional(font_size - 1.0),
        colors.tag_text,
    );
}

fn commit_label_placement(
    dot_pos: Pos2,
    lane: usize,
    orientation: GitGraphOrientation,
    config: &GitGraphLayoutConfig,
    font_size: f32,
) -> (Pos2, Align2) {
    let gap = config.commit_radius + font_size * 0.35;
    match orientation {
        GitGraphOrientation::Lr => {
            if lane % 2 == 0 {
                (
                    Pos2::new(dot_pos.x, dot_pos.y + gap),
                    Align2::CENTER_TOP,
                )
            } else {
                (
                    Pos2::new(dot_pos.x, dot_pos.y - gap),
                    Align2::CENTER_BOTTOM,
                )
            }
        }
        GitGraphOrientation::Tb => {
            if lane % 2 == 0 {
                (
                    Pos2::new(dot_pos.x + gap, dot_pos.y),
                    Align2::LEFT_CENTER,
                )
            } else {
                (
                    Pos2::new(dot_pos.x - gap, dot_pos.y),
                    Align2::RIGHT_CENTER,
                )
            }
        }
    }
}

fn label_rect(anchor: Pos2, text_size: Vec2, align: Align2) -> Rect {
    let pad = Vec2::new(4.0, 2.0);
    let size = text_size + pad * 2.0;
    let min = match align {
        Align2::CENTER_TOP => Pos2::new(anchor.x - size.x * 0.5, anchor.y - pad.y),
        Align2::CENTER_BOTTOM => Pos2::new(anchor.x - size.x * 0.5, anchor.y - size.y + pad.y),
        Align2::LEFT_CENTER => Pos2::new(anchor.x - pad.x, anchor.y - size.y * 0.5),
        Align2::RIGHT_CENTER => Pos2::new(anchor.x - size.x + pad.x, anchor.y - size.y * 0.5),
        _ => Pos2::new(anchor.x - size.x * 0.5, anchor.y - size.y * 0.5),
    };
    Rect::from_min_size(min, size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::mermaid::git_graph::parse_git_graph;

    #[test]
    fn layout_drives_nonzero_bounds_for_typical_graph() {
        let graph = parse_git_graph(
            "gitGraph\n  commit\n  branch develop\n  commit\n  checkout main\n  merge develop",
        )
        .unwrap();
        let layout = layout_git_graph(&graph, GitGraphLayoutConfig::default());
        assert!(layout.bounds.x > 100.0);
        assert!(layout.bounds.y > 50.0);
        assert_eq!(layout.merge_connectors.len(), 1);
        assert_eq!(layout.branch_lines.len(), 2);
    }

    #[test]
    fn shorten_toward_stops_before_target() {
        let from = Pos2::new(0.0, 0.0);
        let to = Pos2::new(10.0, 0.0);
        let shortened = shorten_toward(from, to, 2.0);
        assert!((shortened.x - 8.0).abs() < f32::EPSILON);
    }

    #[test]
    fn commit_label_placement_alternates_by_lane() {
        let config = GitGraphLayoutConfig::default();
        let dot = Pos2::new(80.0, 30.0);
        let (below, _) = commit_label_placement(dot, 0, GitGraphOrientation::Lr, &config, 12.0);
        let (above, _) = commit_label_placement(dot, 1, GitGraphOrientation::Lr, &config, 12.0);
        assert!(below.y > dot.y);
        assert!(above.y < dot.y);
    }
}
