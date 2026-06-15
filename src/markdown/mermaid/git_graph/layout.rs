//! Git graph lane layout: branches as lanes, commits as sequence columns.

use egui::{Pos2, Vec2};
use std::collections::{HashMap, HashSet};

use super::{GitGraph, GitGraphOrientation};

/// Spacing constants for git graph layout.
#[derive(Debug, Clone, Copy)]
pub struct GitGraphLayoutConfig {
    pub margin: f32,
    pub commit_spacing: f32,
    pub lane_spacing: f32,
    pub commit_radius: f32,
}

impl Default for GitGraphLayoutConfig {
    fn default() -> Self {
        Self {
            margin: 30.0,
            commit_spacing: 50.0,
            lane_spacing: 60.0,
            commit_radius: 8.0,
        }
    }
}

/// Layout data for a single commit dot.
#[derive(Debug, Clone, PartialEq)]
pub struct GitGraphCommitLayout {
    /// Index into `GitGraph::commits`.
    pub commit_index: usize,
    /// Declaration-order sequence index (0-based).
    pub sequence: usize,
    /// Lane index for the commit's branch.
    pub lane: usize,
    /// Dot center in layout coordinates.
    pub pos: Pos2,
}

/// Horizontal branch polyline extent (first → last commit, plus branch-off).
#[derive(Debug, Clone, PartialEq)]
pub struct GitGraphBranchLine {
    pub branch: String,
    pub lane: usize,
    /// Start of the branch line (branch-off on parent, or first commit).
    pub start: Pos2,
    /// End at the branch's last commit.
    pub end: Pos2,
    /// Divergence point on the parent branch, if any.
    pub branch_off: Option<Pos2>,
    pub parent_branch: Option<String>,
}

/// Merge connector from source branch tip to merge commit.
#[derive(Debug, Clone, PartialEq)]
pub struct GitGraphMergeConnector {
    pub target_commit_index: usize,
    pub source_branch: String,
    pub source_pos: Pos2,
    pub target_pos: Pos2,
}

/// Cherry-pick connector from source commit to cherry-pick commit.
#[derive(Debug, Clone, PartialEq)]
pub struct GitGraphCherryPickConnector {
    pub target_commit_index: usize,
    pub source_commit_index: usize,
    pub source_pos: Pos2,
    pub target_pos: Pos2,
}

/// Complete layout output for a git graph.
#[derive(Debug, Clone, PartialEq)]
pub struct GitGraphLayout {
    pub commits: Vec<GitGraphCommitLayout>,
    pub branch_lines: Vec<GitGraphBranchLine>,
    pub merge_connectors: Vec<GitGraphMergeConnector>,
    pub cherry_pick_connectors: Vec<GitGraphCherryPickConnector>,
    pub branch_lanes: HashMap<String, usize>,
    pub bounds: Vec2,
}

/// Assign lane indices: main/first branch defaults to lane 0, then declaration order,
/// with explicit `order:` overriding the default slot.
pub fn assign_branch_lanes(branches: &[super::GitBranch]) -> HashMap<String, usize> {
    let mut lanes: HashMap<String, usize> = HashMap::new();
    let mut occupied: HashSet<usize> = HashSet::new();

    for branch in branches {
        if let Some(order) = branch.order {
            let lane = order as usize;
            lanes.insert(branch.name.clone(), lane);
            occupied.insert(lane);
        }
    }

    let mut next_lane = 0;
    for branch in branches {
        if lanes.contains_key(&branch.name) {
            continue;
        }
        while occupied.contains(&next_lane) {
            next_lane += 1;
        }
        lanes.insert(branch.name.clone(), next_lane);
        occupied.insert(next_lane);
        next_lane += 1;
    }

    lanes
}

fn lr_coords(seq: usize, lane: usize, config: &GitGraphLayoutConfig) -> (f32, f32) {
    let x = config.margin + seq as f32 * config.commit_spacing;
    let y = config.margin + lane as f32 * config.lane_spacing;
    (x, y)
}

fn apply_orientation(x: f32, y: f32, orientation: GitGraphOrientation) -> Pos2 {
    match orientation {
        GitGraphOrientation::Lr => Pos2::new(x, y),
        GitGraphOrientation::Tb => Pos2::new(y, x),
    }
}

fn commit_pos(
    seq: usize,
    lane: usize,
    orientation: GitGraphOrientation,
    config: &GitGraphLayoutConfig,
) -> Pos2 {
    let (x, y) = lr_coords(seq, lane, config);
    apply_orientation(x, y, orientation)
}

/// Compute lane layout for a parsed git graph.
pub fn layout_git_graph(graph: &GitGraph, config: GitGraphLayoutConfig) -> GitGraphLayout {
    let branch_lanes = assign_branch_lanes(&graph.branches);

    let mut commit_layouts: Vec<GitGraphCommitLayout> = Vec::with_capacity(graph.commits.len());
    let mut first_commit_on_branch: HashMap<String, usize> = HashMap::new();
    let mut last_commit_on_branch: HashMap<String, (usize, Pos2)> = HashMap::new();

    for (commit_index, commit) in graph.commits.iter().enumerate() {
        let sequence = commit_index;
        let lane = *branch_lanes.get(&commit.branch).unwrap_or(&0);
        let pos = commit_pos(sequence, lane, graph.orientation, &config);

        first_commit_on_branch
            .entry(commit.branch.clone())
            .or_insert(commit_index);

        last_commit_on_branch.insert(commit.branch.clone(), (commit_index, pos));

        commit_layouts.push(GitGraphCommitLayout {
            commit_index,
            sequence,
            lane,
            pos,
        });
    }

    let mut branch_lines: Vec<GitGraphBranchLine> = Vec::new();
    for branch in &graph.branches {
        let lane = *branch_lanes.get(&branch.name).unwrap_or(&0);
        let commits_on_branch: Vec<(usize, Pos2)> = commit_layouts
            .iter()
            .filter(|c| graph.commits[c.commit_index].branch == branch.name)
            .map(|c| (c.commit_index, c.pos))
            .collect();

        if commits_on_branch.is_empty() {
            continue;
        }

        let first_idx = commits_on_branch[0].0;
        let last_pos = commits_on_branch.last().map(|(_, p)| *p).unwrap_or(Pos2::ZERO);

        let (branch_off, parent_branch) = if first_idx > 0 {
            let prev = &commit_layouts[first_idx - 1];
            let parent = graph.commits[prev.commit_index].branch.clone();
            (Some(prev.pos), Some(parent))
        } else {
            (None, None)
        };

        let start = branch_off.unwrap_or(commits_on_branch[0].1);

        branch_lines.push(GitGraphBranchLine {
            branch: branch.name.clone(),
            lane,
            start,
            end: last_pos,
            branch_off,
            parent_branch,
        });
    }

    let mut merge_connectors: Vec<GitGraphMergeConnector> = Vec::new();
    for layout in &commit_layouts {
        let commit = &graph.commits[layout.commit_index];
        if !commit.is_merge {
            continue;
        }
        let Some(source_branch) = commit.merge_from.as_ref() else {
            continue;
        };

        let source_pos = last_commit_on_branch_before(
            source_branch,
            layout.sequence,
            graph,
            &commit_layouts,
        );

        if let Some(source_pos) = source_pos {
            merge_connectors.push(GitGraphMergeConnector {
                target_commit_index: layout.commit_index,
                source_branch: source_branch.clone(),
                source_pos,
                target_pos: layout.pos,
            });
        }
    }

    let mut cherry_pick_connectors: Vec<GitGraphCherryPickConnector> = Vec::new();
    for layout in &commit_layouts {
        let commit = &graph.commits[layout.commit_index];
        if !commit.is_cherry_pick {
            continue;
        }
        let Some(source_id) = commit.cherry_pick_from_id.as_ref() else {
            continue;
        };

        if let Some((source_index, source_layout)) = commit_layouts
            .iter()
            .find(|c| graph.commits[c.commit_index].id == *source_id)
            .map(|c| (c.commit_index, c))
        {
            cherry_pick_connectors.push(GitGraphCherryPickConnector {
                target_commit_index: layout.commit_index,
                source_commit_index: source_index,
                source_pos: source_layout.pos,
                target_pos: layout.pos,
            });
        }
    }

    let max_lane = branch_lanes.values().copied().max().unwrap_or(0);
    let max_seq = graph.commits.len().saturating_sub(1);
    let bounds = compute_bounds(max_seq, max_lane, &config);

    GitGraphLayout {
        commits: commit_layouts,
        branch_lines,
        merge_connectors,
        cherry_pick_connectors,
        branch_lanes,
        bounds,
    }
}

fn last_commit_on_branch_before(
    branch: &str,
    before_sequence: usize,
    graph: &GitGraph,
    layouts: &[GitGraphCommitLayout],
) -> Option<Pos2> {
    layouts
        .iter()
        .filter(|l| {
            l.sequence < before_sequence && graph.commits[l.commit_index].branch == branch
        })
        .max_by_key(|l| l.sequence)
        .map(|l| l.pos)
}

fn compute_bounds(max_seq: usize, max_lane: usize, config: &GitGraphLayoutConfig) -> Vec2 {
    let (lr_x, lr_y) = lr_coords(max_seq, max_lane, config);
    let pad = config.margin + config.commit_radius;
    Vec2::new(lr_x + pad, lr_y + pad)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::mermaid::git_graph::parse_git_graph;

    fn layout(source: &str) -> GitGraphLayout {
        let graph = parse_git_graph(source).expect("parse");
        layout_git_graph(&graph, GitGraphLayoutConfig::default())
    }

    #[test]
    fn lane_assignment_declaration_order() {
        let graph = parse_git_graph(
            "gitGraph\n  commit\n  branch develop\n  commit\n  branch feature\n  commit",
        )
        .unwrap();
        let lanes = assign_branch_lanes(&graph.branches);
        assert_eq!(lanes.get("main"), Some(&0));
        assert_eq!(lanes.get("develop"), Some(&1));
        assert_eq!(lanes.get("feature"), Some(&2));
    }

    #[test]
    fn lane_assignment_with_order_override() {
        let graph = parse_git_graph(
            "gitGraph\n  commit\n  branch develop order: 2\n  commit\n  branch feature\n  commit",
        )
        .unwrap();
        let lanes = assign_branch_lanes(&graph.branches);
        assert_eq!(lanes.get("main"), Some(&0));
        assert_eq!(lanes.get("develop"), Some(&2));
        assert_eq!(lanes.get("feature"), Some(&1));
    }

    #[test]
    fn sequence_indices_follow_declaration_order() {
        let layout = layout(
            "gitGraph\n  commit id: \"a\"\n  branch develop\n  commit id: \"b\"\n  checkout main\n  merge develop",
        );
        assert_eq!(layout.commits.len(), 3);
        assert_eq!(layout.commits[0].sequence, 0);
        assert_eq!(layout.commits[1].sequence, 1);
        assert_eq!(layout.commits[2].sequence, 2);
    }

    #[test]
    fn lr_positions_use_sequence_for_x_and_lane_for_y() {
        let config = GitGraphLayoutConfig::default();
        let layout = layout_git_graph(
            &parse_git_graph(
                "gitGraph\n  commit\n  branch develop\n  commit\n  checkout main\n  commit",
            )
            .unwrap(),
            config,
        );

        let main_lane = *layout.branch_lanes.get("main").unwrap();
        let develop_lane = *layout.branch_lanes.get("develop").unwrap();
        assert!(develop_lane > main_lane);

        let c0 = &layout.commits[0];
        let c1 = &layout.commits[1];
        assert!((c0.pos.x - config.margin).abs() < f32::EPSILON);
        assert!((c0.pos.y - (config.margin + main_lane as f32 * config.lane_spacing)).abs()
            < f32::EPSILON);
        assert!((c1.pos.x - (config.margin + config.commit_spacing)).abs() < f32::EPSILON);
        assert!((c1.pos.y - (config.margin + develop_lane as f32 * config.lane_spacing)).abs()
            < f32::EPSILON);
    }

    #[test]
    fn tb_transposes_lr_coordinates() {
        let config = GitGraphLayoutConfig::default();
        let source = "gitGraph TB:\n  commit\n  branch develop\n  commit";
        let graph = parse_git_graph(source).unwrap();
        let lr_graph = GitGraph {
            orientation: GitGraphOrientation::Lr,
            ..graph.clone()
        };
        let tb_layout = layout_git_graph(&graph, config);
        let lr_layout = layout_git_graph(&lr_graph, config);

        for (tb, lr) in tb_layout.commits.iter().zip(lr_layout.commits.iter()) {
            assert!((tb.pos.x - lr.pos.y).abs() < f32::EPSILON);
            assert!((tb.pos.y - lr.pos.x).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn merge_connector_from_source_branch_tip_to_target() {
        let layout = layout(
            "gitGraph\n  commit\n  branch develop\n  commit\n  checkout main\n  merge develop",
        );
        assert_eq!(layout.merge_connectors.len(), 1);
        let merge = &layout.merge_connectors[0];
        let develop_tip = layout
            .commits
            .iter()
            .find(|c| c.sequence == 1)
            .expect("develop commit")
            .pos;

        assert_eq!(merge.source_pos, develop_tip);
        assert_eq!(merge.target_pos, layout.commits[2].pos);
        assert_eq!(merge.target_commit_index, 2);
        assert_eq!(merge.source_branch, "develop");
    }

    #[test]
    fn cherry_pick_connector_links_source_and_target() {
        let layout = layout(
            "gitGraph\n  commit id: \"base\"\n  branch feature\n  cherry-pick id: \"base\"",
        );
        assert_eq!(layout.cherry_pick_connectors.len(), 1);
        let cp = &layout.cherry_pick_connectors[0];
        assert_eq!(cp.source_pos, layout.commits[0].pos);
        assert_eq!(cp.target_pos, layout.commits[1].pos);
    }

    #[test]
    fn branch_line_has_branch_off_on_parent() {
        let layout = layout("gitGraph\n  commit\n  branch develop\n  commit");
        let develop_line = layout
            .branch_lines
            .iter()
            .find(|l| l.branch == "develop")
            .expect("develop branch line");
        assert_eq!(develop_line.branch_off, Some(layout.commits[0].pos));
        assert_eq!(develop_line.start, layout.commits[0].pos);
        assert_eq!(develop_line.end, layout.commits[1].pos);
        assert_eq!(develop_line.parent_branch.as_deref(), Some("main"));
    }

    #[test]
    fn bounds_grow_with_lanes_and_commits() {
        let config = GitGraphLayoutConfig::default();
        let small = layout_git_graph(
            &parse_git_graph("gitGraph\n  commit").unwrap(),
            config,
        );
        let large = layout_git_graph(
            &parse_git_graph(
                "gitGraph\n  commit\n  branch a\n  commit\n  branch b\n  commit\n  commit",
            )
            .unwrap(),
            config,
        );
        assert!(large.bounds.x > small.bounds.x);
        assert!(large.bounds.y > small.bounds.y);
    }
}
