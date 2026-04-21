//! Git graph diagram parsing and rendering.

use egui::{Color32, FontId, Pos2, Rect, Stroke, Ui, Vec2};
use std::collections::HashMap;

/// A commit in a git graph.
#[derive(Debug, Clone)]
pub struct GitCommit {
    pub id: String,
    pub branch: String,
    pub message: Option<String>,
    pub is_merge: bool,
    pub merge_from: Option<String>,
}

/// A branch in a git graph.
#[derive(Debug, Clone)]
pub struct GitBranch {
    pub name: String,
    pub color_idx: usize,
}

/// A git graph.
#[derive(Debug, Clone)]
pub struct GitGraph {
    pub commits: Vec<GitCommit>,
    pub branches: Vec<GitBranch>,
}

/// Parse a git graph from source.
pub fn parse_git_graph(source: &str) -> Result<GitGraph, String> {
    let mut commits: Vec<GitCommit> = Vec::new();
    let mut branches: Vec<GitBranch> = vec![GitBranch {
        name: "main".to_string(),
        color_idx: 0,
    }];
    let mut current_branch = "main".to_string();
    let mut commit_counter = 0;

    for line in source.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        let line_lower = line.to_lowercase();

        // Parse commit
        if line_lower.starts_with("commit") {
            commit_counter += 1;
            let id = if line.contains("id:") {
                // commit id: "abc123"
                line.split("id:")
                    .nth(1)
                    .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                    .unwrap_or_else(|| format!("c{}", commit_counter))
            } else {
                format!("c{}", commit_counter)
            };

            let message = if line.contains("msg:") {
                line.split("msg:")
                    .nth(1)
                    .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
            } else {
                None
            };

            commits.push(GitCommit {
                id,
                branch: current_branch.clone(),
                message,
                is_merge: false,
                merge_from: None,
            });
        }
        // Parse branch creation
        else if line_lower.starts_with("branch") {
            let name = line[6..].trim().to_string();
            if !branches.iter().any(|b| b.name == name) {
                branches.push(GitBranch {
                    name: name.clone(),
                    color_idx: branches.len(),
                });
            }
            current_branch = name;
        }
        // Parse checkout
        else if line_lower.starts_with("checkout") {
            let name = line[8..].trim().to_string();
            if !branches.iter().any(|b| b.name == name) {
                branches.push(GitBranch {
                    name: name.clone(),
                    color_idx: branches.len(),
                });
            }
            current_branch = name;
        }
        // Parse merge
        else if line_lower.starts_with("merge") {
            commit_counter += 1;
            let rest = line[5..].trim();
            let (merge_from, id) = if rest.contains("id:") {
                let parts: Vec<&str> = rest.split("id:").collect();
                let from = parts[0].trim().to_string();
                let id = parts[1]
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string();
                (from, id)
            } else {
                (rest.to_string(), format!("m{}", commit_counter))
            };

            commits.push(GitCommit {
                id,
                branch: current_branch.clone(),
                message: Some(format!("Merge {}", merge_from)),
                is_merge: true,
                merge_from: Some(merge_from),
            });
        }
    }

    if commits.is_empty() {
        return Err("No commits found in git graph".to_string());
    }

    Ok(GitGraph { commits, branches })
}

/// Render a git graph to the UI.
pub fn render_git_graph(ui: &mut Ui, graph: &GitGraph, dark_mode: bool, font_size: f32) {
    let margin = 30.0_f32;
    let commit_radius = 8.0_f32;
    let commit_spacing = 50.0_f32;
    let branch_spacing = 60.0_f32; // Wider spacing between branches
    let label_width = 140.0_f32; // Wider label area

    // Branch colors
    let branch_colors = if dark_mode {
        vec![
            Color32::from_rgb(100, 180, 100), // green (main)
            Color32::from_rgb(100, 150, 220), // blue
            Color32::from_rgb(220, 160, 100), // orange
            Color32::from_rgb(180, 100, 180), // purple
            Color32::from_rgb(220, 100, 100), // red
            Color32::from_rgb(100, 200, 200), // cyan
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
    let text_color = if dark_mode {
        Color32::from_rgb(220, 230, 240)
    } else {
        Color32::from_rgb(30, 40, 50)
    };
    let line_bg = if dark_mode {
        Color32::from_rgb(50, 55, 65)
    } else {
        Color32::from_rgb(240, 245, 250)
    };

    // Calculate branch positions
    let mut branch_x: HashMap<String, f32> = HashMap::new();
    for (i, branch) in graph.branches.iter().enumerate() {
        branch_x.insert(
            branch.name.clone(),
            margin + label_width + i as f32 * branch_spacing,
        );
    }

    let total_width =
        margin * 2.0 + label_width + graph.branches.len() as f32 * branch_spacing + 50.0;
    let total_height = margin * 2.0 + graph.commits.len() as f32 * commit_spacing;

    let (response, painter) = ui.allocate_painter(
        Vec2::new(total_width.max(300.0), total_height.max(100.0)),
        egui::Sense::hover(),
    );
    let offset = response.rect.min.to_vec2();

    // Track last commit position per branch for drawing lines
    let mut last_commit_pos: HashMap<String, Pos2> = HashMap::new();

    // Draw commits
    for (i, commit) in graph.commits.iter().enumerate() {
        let x = branch_x
            .get(&commit.branch)
            .copied()
            .unwrap_or(margin + label_width);
        let y = margin + i as f32 * commit_spacing;
        let pos = Pos2::new(x, y) + offset;

        let branch = graph.branches.iter().find(|b| b.name == commit.branch);
        let color = branch_colors[branch.map(|b| b.color_idx).unwrap_or(0) % branch_colors.len()];

        // Draw line from previous commit on same branch
        if let Some(prev_pos) = last_commit_pos.get(&commit.branch) {
            painter.line_segment([*prev_pos, pos], Stroke::new(3.0, color));
        }

        // Draw merge line
        if let Some(ref merge_from) = commit.merge_from {
            if let Some(merge_pos) = last_commit_pos.get(merge_from) {
                let merge_color = graph
                    .branches
                    .iter()
                    .find(|b| &b.name == merge_from)
                    .map(|b| branch_colors[b.color_idx % branch_colors.len()])
                    .unwrap_or(color);

                // Draw curved merge line
                let mid_y = (merge_pos.y + pos.y) / 2.0;
                let ctrl1 = Pos2::new(merge_pos.x, mid_y);
                let ctrl2 = Pos2::new(pos.x, mid_y);

                // Approximate bezier with line segments
                painter.line_segment([*merge_pos, ctrl1], Stroke::new(2.0, merge_color));
                painter.line_segment([ctrl1, ctrl2], Stroke::new(2.0, merge_color));
                painter.line_segment([ctrl2, pos], Stroke::new(2.0, merge_color));
            }
        }

        // Draw commit circle
        if commit.is_merge {
            // Merge commit - filled circle with border
            painter.circle_filled(pos, commit_radius, color);
            painter.circle_stroke(
                pos,
                commit_radius,
                Stroke::new(
                    2.0,
                    if dark_mode {
                        Color32::WHITE
                    } else {
                        Color32::BLACK
                    },
                ),
            );
        } else {
            // Regular commit - filled circle
            painter.circle_filled(pos, commit_radius, color);
        }

        // Draw commit label
        let label = commit.message.as_ref().unwrap_or(&commit.id);
        let label_bg_rect = Rect::from_min_size(
            Pos2::new(offset.x + margin - 5.0, pos.y - font_size * 0.5 - 2.0),
            Vec2::new(label_width - 10.0, font_size + 4.0),
        );
        painter.rect_filled(label_bg_rect, 3.0, line_bg);
        painter.text(
            Pos2::new(offset.x + margin, pos.y),
            egui::Align2::LEFT_CENTER,
            label,
            FontId::proportional(font_size - 2.0),
            text_color,
        );

        last_commit_pos.insert(commit.branch.clone(), pos);
    }

    // Draw branch labels at top
    for branch in &graph.branches {
        if let Some(&x) = branch_x.get(&branch.name) {
            let color = branch_colors[branch.color_idx % branch_colors.len()];
            let pos = Pos2::new(x, margin - 15.0) + offset;
            painter.text(
                pos,
                egui::Align2::CENTER_BOTTOM,
                &branch.name,
                FontId::proportional(font_size - 2.0),
                color,
            );
        }
    }
}
