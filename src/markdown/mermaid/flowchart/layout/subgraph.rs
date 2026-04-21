//! Subgraph layout engine for computing internal subgraph positions.

use std::collections::HashMap;

use egui::{Pos2, Vec2};

use super::super::types::FlowDirection;
use super::config::FlowLayoutConfig;
use super::graph::FlowGraph;

/// Result of laying out a subgraph's internal contents.
#[derive(Debug, Clone)]
pub(crate) struct SubgraphInternalLayout {
    /// Positions of nodes relative to subgraph origin (0,0)
    pub node_positions: HashMap<usize, Pos2>,
    /// Bounding box size (including padding and title)
    pub bounding_size: Vec2,
    /// Content size (without padding)
    pub content_size: Vec2,
}

/// Engine for laying out subgraph contents independently.
pub(crate) struct SubgraphLayoutEngine<'a> {
    /// Reference to the full graph
    graph: &'a FlowGraph,
    /// Layout configuration
    config: &'a FlowLayoutConfig,
    /// Flow direction
    direction: FlowDirection,
    /// Available width for layout
    #[allow(dead_code)]
    available_width: f32,
}

impl<'a> SubgraphLayoutEngine<'a> {
    pub fn new(
        graph: &'a FlowGraph,
        config: &'a FlowLayoutConfig,
        direction: FlowDirection,
        available_width: f32,
    ) -> Self {
        Self {
            graph,
            config,
            direction,
            available_width,
        }
    }

    /// Layout a single subgraph's internal contents.
    /// Returns positions relative to (0, 0) origin.
    pub fn layout_subgraph(
        &self,
        subgraph_id: &str,
        child_subgraph_layouts: &HashMap<String, SubgraphInternalLayout>,
    ) -> Option<SubgraphInternalLayout> {
        let (node_indices, child_ids) = self.graph.subgraph_info.get(subgraph_id)?;

        if node_indices.is_empty() && child_ids.is_empty() {
            return None;
        }

        // Get the effective direction for this subgraph (use override if present)
        let effective_direction = self
            .graph
            .subgraph_directions
            .get(subgraph_id)
            .copied()
            .unwrap_or(self.direction);

        // Collect nodes that directly belong to this subgraph (not nested children)
        let direct_nodes: Vec<usize> = node_indices
            .iter()
            .filter(|&&idx| {
                self.graph
                    .node_subgraph
                    .get(idx)
                    .and_then(|s| s.as_ref())
                    .map(|s| s == subgraph_id)
                    .unwrap_or(false)
            })
            .copied()
            .collect();

        // Build set of all nodes in this subgraph (for edge filtering)
        let all_subgraph_nodes: std::collections::HashSet<usize> =
            node_indices.iter().copied().collect();

        // Find internal edges (both endpoints in subgraph)
        let back_edge_set: std::collections::HashSet<(usize, usize)> =
            self.graph.back_edges.iter().cloned().collect();

        let mut internal_edges: Vec<(usize, usize)> = Vec::new();
        for &from in &direct_nodes {
            if let Some(targets) = self.graph.outgoing.get(from) {
                for &to in targets {
                    if all_subgraph_nodes.contains(&to) && !back_edge_set.contains(&(from, to)) {
                        internal_edges.push((from, to));
                    }
                }
            }
        }

        // If we only have direct nodes (no nested subgraphs), do simple layout
        if child_ids.is_empty() {
            return self.layout_simple_subgraph(
                &direct_nodes,
                &internal_edges,
                effective_direction,
            );
        }

        // Complex case: we have child subgraphs that act as "super-nodes"
        self.layout_hierarchical_subgraph(
            subgraph_id,
            &direct_nodes,
            child_ids,
            child_subgraph_layouts,
            effective_direction,
        )
    }

    /// Layout a subgraph that contains only direct nodes (no nested subgraphs).
    fn layout_simple_subgraph(
        &self,
        nodes: &[usize],
        edges: &[(usize, usize)],
        direction: FlowDirection,
    ) -> Option<SubgraphInternalLayout> {
        if nodes.is_empty() {
            return None;
        }

        let is_horizontal = matches!(
            direction,
            FlowDirection::LeftRight | FlowDirection::RightLeft
        );

        // Assign layers within subgraph
        let layers = self.assign_internal_layers(nodes, edges);

        // Compute positions within subgraph
        let (node_positions, content_size) =
            self.compute_internal_positions(nodes, &layers, is_horizontal);

        // Add padding and title height for bounding box
        let padding = self.config.subgraph_padding;
        let title_height = self.config.subgraph_title_height;

        let bounding_size = Vec2::new(
            content_size.x + padding * 2.0,
            content_size.y + padding * 2.0 + title_height,
        );

        Some(SubgraphInternalLayout {
            node_positions,
            bounding_size,
            content_size,
        })
    }

    /// Assign layers to nodes within a subgraph using longest-path.
    fn assign_internal_layers(&self, nodes: &[usize], edges: &[(usize, usize)]) -> Vec<Vec<usize>> {
        if nodes.is_empty() {
            return Vec::new();
        }

        let node_set: std::collections::HashSet<usize> = nodes.iter().copied().collect();
        let local_idx: HashMap<usize, usize> = nodes
            .iter()
            .enumerate()
            .map(|(i, &node)| (node, i))
            .collect();

        let n = nodes.len();

        // Build in-degree for internal edges
        let mut in_degree = vec![0usize; n];
        for &(from, to) in edges {
            if let (Some(&_from_local), Some(&to_local)) =
                (local_idx.get(&from), local_idx.get(&to))
            {
                in_degree[to_local] += 1;
            }
        }

        // Longest-path layer assignment
        let mut node_layers = vec![0usize; n];
        let mut queue: std::collections::VecDeque<usize> = std::collections::VecDeque::new();

        // Start with nodes that have no internal predecessors
        for (local_i, &deg) in in_degree.iter().enumerate() {
            if deg == 0 {
                queue.push_back(local_i);
            }
        }

        // If all nodes have predecessors (cycle), start from first
        if queue.is_empty() && !nodes.is_empty() {
            queue.push_back(0);
        }

        // Safety limit to prevent infinite loops on malformed input
        let max_iterations = n * n + 100;
        let mut iteration = 0;

        while let Some(local_i) = queue.pop_front() {
            iteration += 1;
            if iteration > max_iterations {
                return vec![nodes.to_vec()];
            }

            let node = nodes[local_i];
            let current_layer = node_layers[local_i];

            for &(from, to) in edges {
                if from == node {
                    if let Some(&to_local) = local_idx.get(&to) {
                        if node_set.contains(&to) {
                            node_layers[to_local] = node_layers[to_local].max(current_layer + 1);

                            in_degree[to_local] = in_degree[to_local].saturating_sub(1);
                            if in_degree[to_local] == 0 {
                                queue.push_back(to_local);
                            }
                        }
                    }
                }
            }
        }

        // Build layers structure
        let max_layer = node_layers.iter().copied().max().unwrap_or(0);
        let mut layers: Vec<Vec<usize>> = vec![Vec::new(); max_layer + 1];

        for (local_i, &layer) in node_layers.iter().enumerate() {
            layers[layer].push(nodes[local_i]);
        }

        layers
    }

    /// Compute positions for nodes within a subgraph.
    /// Returns (node_positions, content_size).
    fn compute_internal_positions(
        &self,
        _nodes: &[usize],
        layers: &[Vec<usize>],
        is_horizontal: bool,
    ) -> (HashMap<usize, Pos2>, Vec2) {
        let mut positions: HashMap<usize, Pos2> = HashMap::new();

        if layers.is_empty() {
            return (positions, Vec2::ZERO);
        }

        let spacing = &self.config.node_spacing;
        let padding = self.config.subgraph_padding;
        let title_height = self.config.subgraph_title_height;

        // Calculate layer sizes
        let mut layer_main_sizes: Vec<f32> = Vec::new();
        let mut layer_cross_sizes: Vec<f32> = Vec::new();

        for layer in layers {
            let main_size: f32 = layer
                .iter()
                .map(|&idx| {
                    let size = self.graph.node_sizes[idx];
                    if is_horizontal {
                        size.x
                    } else {
                        size.y
                    }
                })
                .fold(0.0_f32, f32::max);

            let cross_size: f32 = layer
                .iter()
                .map(|&idx| {
                    let size = self.graph.node_sizes[idx];
                    if is_horizontal {
                        size.y
                    } else {
                        size.x
                    }
                })
                .sum::<f32>()
                + (layer.len().saturating_sub(1)) as f32
                    * if is_horizontal { spacing.y } else { spacing.x };

            layer_main_sizes.push(main_size);
            layer_cross_sizes.push(cross_size);
        }

        let max_cross_size = layer_cross_sizes.iter().copied().fold(0.0_f32, f32::max);

        // Position nodes layer by layer
        let mut current_main = padding + title_height;
        let mut max_extent = Vec2::ZERO;

        for (layer_idx, layer) in layers.iter().enumerate() {
            let layer_cross = layer_cross_sizes[layer_idx];
            let start_cross = padding + (max_cross_size - layer_cross) / 2.0;
            let mut current_cross = start_cross;

            for &node_idx in layer {
                let size = self.graph.node_sizes[node_idx];

                let pos = if is_horizontal {
                    Pos2::new(current_main, current_cross)
                } else {
                    Pos2::new(current_cross, current_main)
                };

                positions.insert(node_idx, pos);

                max_extent.x = max_extent.x.max(pos.x + size.x);
                max_extent.y = max_extent.y.max(pos.y + size.y);

                current_cross += if is_horizontal {
                    size.y + spacing.y
                } else {
                    size.x + spacing.x
                };
            }

            current_main +=
                layer_main_sizes[layer_idx] + if is_horizontal { spacing.x } else { spacing.y };
        }

        // Content size is the extent minus the padding/title we added
        let content_size = Vec2::new(
            max_extent.x - padding,
            max_extent.y - padding - title_height,
        );

        (positions, content_size.max(Vec2::ZERO))
    }

    /// Layout a subgraph that contains nested child subgraphs.
    fn layout_hierarchical_subgraph(
        &self,
        _subgraph_id: &str,
        direct_nodes: &[usize],
        child_ids: &[String],
        child_layouts: &HashMap<String, SubgraphInternalLayout>,
        direction: FlowDirection,
    ) -> Option<SubgraphInternalLayout> {
        // For hierarchical layouts, we treat child subgraphs as large "virtual nodes"
        // and layout them alongside direct nodes.

        // Collect sizes: direct nodes + child subgraph bounding boxes
        let mut all_sizes: Vec<(usize, Vec2, bool)> = Vec::new();

        for &node_idx in direct_nodes {
            all_sizes.push((node_idx, self.graph.node_sizes[node_idx], false));
        }

        let is_horizontal = matches!(
            direction,
            FlowDirection::LeftRight | FlowDirection::RightLeft
        );

        let spacing = &self.config.node_spacing;
        let padding = self.config.subgraph_padding;
        let title_height = self.config.subgraph_title_height;

        let mut positions: HashMap<usize, Pos2> = HashMap::new();
        let mut current_main = padding + title_height;
        let mut max_cross: f32 = 0.0;

        // First pass: compute max cross size
        for &node_idx in direct_nodes {
            let size = self.graph.node_sizes[node_idx];
            let cross = if is_horizontal { size.y } else { size.x };
            max_cross = max_cross.max(cross);
        }
        for child_id in child_ids {
            if let Some(child_layout) = child_layouts.get(child_id) {
                let cross = if is_horizontal {
                    child_layout.bounding_size.y
                } else {
                    child_layout.bounding_size.x
                };
                max_cross = max_cross.max(cross);
            }
        }

        // Second pass: position items
        for &node_idx in direct_nodes {
            let size = self.graph.node_sizes[node_idx];
            let cross = if is_horizontal { size.y } else { size.x };
            let offset_cross = padding + (max_cross - cross) / 2.0;

            let pos = if is_horizontal {
                Pos2::new(current_main, offset_cross)
            } else {
                Pos2::new(offset_cross, current_main)
            };

            positions.insert(node_idx, pos);

            current_main += if is_horizontal { size.x } else { size.y };
            current_main += if is_horizontal { spacing.x } else { spacing.y };
        }

        // Calculate content size
        let content_size = if is_horizontal {
            Vec2::new(current_main - padding - title_height, max_cross)
        } else {
            Vec2::new(max_cross, current_main - padding - title_height)
        };

        let bounding_size = Vec2::new(
            content_size.x + padding * 2.0,
            content_size.y + padding * 2.0 + title_height,
        );

        Some(SubgraphInternalLayout {
            node_positions: positions,
            bounding_size,
            content_size,
        })
    }
}
