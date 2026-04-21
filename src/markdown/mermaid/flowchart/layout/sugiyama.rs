//! Sugiyama-style layered graph layout algorithm.
//!
//! Implements the core layout algorithm with:
//! - Cycle detection and back-edge handling
//! - Layer assignment with longest-path algorithm
//! - Subgraph-aware layer clustering
//! - Edge crossing minimization using barycenter heuristic
//! - Coordinate assignment for all flow directions

use std::collections::HashMap;

use egui::{Pos2, Vec2};

use super::super::types::*;
use super::config::FlowLayoutConfig;
use super::graph::FlowGraph;
use super::subgraph::{SubgraphInternalLayout, SubgraphLayoutEngine};

/// Sugiyama-style layered graph layout algorithm.
pub(crate) struct SugiyamaLayout {
    pub graph: FlowGraph,
    direction: FlowDirection,
    config: FlowLayoutConfig,
    available_width: f32,
    /// Assigned layer for each node (indexed by node index)
    node_layers: Vec<usize>,
    /// Nodes in each layer, ordered for crossing minimization
    layers: Vec<Vec<usize>>,
}

impl SugiyamaLayout {
    pub fn new(
        graph: FlowGraph,
        direction: FlowDirection,
        config: FlowLayoutConfig,
        available_width: f32,
    ) -> Self {
        let n = graph.node_count();
        SugiyamaLayout {
            graph,
            direction,
            config,
            available_width,
            node_layers: vec![0; n],
            layers: Vec::new(),
        }
    }

    /// Run the complete layout algorithm with subgraph-aware positioning.
    pub fn compute(mut self) -> FlowchartLayout {
        if self.graph.node_count() == 0 {
            return FlowchartLayout::default();
        }

        // Step 0: Layout subgraphs inside-out and compute their bounding boxes
        let subgraph_layouts = self.layout_subgraphs_inside_out();

        // Store original node sizes before replacing with super-node sizes
        let original_sizes = self.graph.node_sizes.clone();

        // Step 1: Detect cycles and mark back-edges
        self.detect_cycles_and_mark_back_edges();

        // Step 2: Assign layers using longest-path algorithm
        self.assign_layers();

        // Step 3: Build initial layer structure
        self.build_layers();

        // Step 4: Reduce edge crossings
        self.reduce_crossings();

        // Step 5: Assign coordinates (using original sizes for actual placement)
        self.graph.node_sizes = original_sizes;
        self.assign_coordinates_with_subgraphs(&subgraph_layouts)
    }

    /// Layout all subgraphs from innermost to outermost.
    fn layout_subgraphs_inside_out(&mut self) -> HashMap<String, SubgraphInternalLayout> {
        let mut layouts: HashMap<String, SubgraphInternalLayout> = HashMap::new();

        if self.graph.subgraph_info.is_empty() {
            return layouts;
        }

        let subgraph_order = self.get_subgraph_processing_order();

        let engine = SubgraphLayoutEngine::new(
            &self.graph,
            &self.config,
            self.direction,
            self.available_width,
        );

        for subgraph_id in &subgraph_order {
            if let Some(layout) = engine.layout_subgraph(subgraph_id, &layouts) {
                layouts.insert(subgraph_id.clone(), layout);
            }
        }

        layouts
    }

    /// Get subgraph IDs in processing order (children before parents).
    fn get_subgraph_processing_order(&self) -> Vec<String> {
        let mut result: Vec<String> = Vec::new();
        let mut processed: std::collections::HashSet<String> = std::collections::HashSet::new();
        let subgraph_ids: Vec<String> = self.graph.subgraph_info.keys().cloned().collect();

        fn process_subgraph(
            id: &str,
            info: &HashMap<String, (Vec<usize>, Vec<String>)>,
            result: &mut Vec<String>,
            processed: &mut std::collections::HashSet<String>,
        ) {
            if processed.contains(id) {
                return;
            }

            if let Some((_, child_ids)) = info.get(id) {
                for child_id in child_ids {
                    process_subgraph(child_id, info, result, processed);
                }
            }

            result.push(id.to_string());
            processed.insert(id.to_string());
        }

        for id in &subgraph_ids {
            process_subgraph(id, &self.graph.subgraph_info, &mut result, &mut processed);
        }

        result
    }

    /// Detect cycles using DFS and mark back-edges.
    fn detect_cycles_and_mark_back_edges(&mut self) {
        let n = self.graph.node_count();
        let mut visited = vec![false; n];
        let mut in_stack = vec![false; n];
        let mut back_edges = Vec::new();

        for start in 0..n {
            if !visited[start] {
                self.dfs_find_back_edges(start, &mut visited, &mut in_stack, &mut back_edges);
            }
        }

        self.graph.back_edges = back_edges;
    }

    fn dfs_find_back_edges(
        &self,
        node: usize,
        visited: &mut [bool],
        in_stack: &mut [bool],
        back_edges: &mut Vec<(usize, usize)>,
    ) {
        visited[node] = true;
        in_stack[node] = true;

        for &neighbor in &self.graph.outgoing[node] {
            if !visited[neighbor] {
                self.dfs_find_back_edges(neighbor, visited, in_stack, back_edges);
            } else if in_stack[neighbor] {
                back_edges.push((node, neighbor));
            }
        }

        in_stack[node] = false;
    }

    /// Assign layers using longest-path algorithm with subgraph awareness.
    fn assign_layers(&mut self) {
        let n = self.graph.node_count();

        let back_edge_set: std::collections::HashSet<(usize, usize)> =
            self.graph.back_edges.iter().cloned().collect();

        let mut effective_incoming: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (from_idx, targets) in self.graph.outgoing.iter().enumerate() {
            for &to_idx in targets {
                if !back_edge_set.contains(&(from_idx, to_idx)) {
                    effective_incoming[to_idx].push(from_idx);
                }
            }
        }

        // Phase 1: Standard longest-path layer assignment
        let mut in_degree: Vec<usize> = effective_incoming.iter().map(|v| v.len()).collect();
        let mut queue: std::collections::VecDeque<usize> = std::collections::VecDeque::new();

        for (idx, &deg) in in_degree.iter().enumerate() {
            if deg == 0 {
                queue.push_back(idx);
                self.node_layers[idx] = 0;
            }
        }

        if queue.is_empty() && n > 0 {
            queue.push_back(0);
            self.node_layers[0] = 0;
            in_degree[0] = 0;
        }

        while let Some(node) = queue.pop_front() {
            let current_layer = self.node_layers[node];

            for &neighbor in &self.graph.outgoing[node] {
                if !back_edge_set.contains(&(node, neighbor)) {
                    self.node_layers[neighbor] = self.node_layers[neighbor].max(current_layer + 1);

                    in_degree[neighbor] = in_degree[neighbor].saturating_sub(1);
                    if in_degree[neighbor] == 0 {
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        // Handle any remaining nodes (disconnected or in complex cycles)
        for idx in 0..n {
            if in_degree[idx] > 0 {
                let max_pred_layer = effective_incoming[idx]
                    .iter()
                    .filter(|&&pred| in_degree[pred] == 0 || self.node_layers[pred] > 0)
                    .map(|&pred| self.node_layers[pred])
                    .max()
                    .unwrap_or(0);
                self.node_layers[idx] = max_pred_layer + 1;
            }
        }

        // Phase 2: Cluster subgraph nodes to consecutive layers
        self.cluster_subgraph_layers(&back_edge_set);
    }

    /// Adjust layer assignments to keep subgraph nodes in consecutive layers.
    fn cluster_subgraph_layers(
        &mut self,
        back_edge_set: &std::collections::HashSet<(usize, usize)>,
    ) {
        let subgraph_ids: Vec<String> = self.graph.subgraph_info.keys().cloned().collect();

        for subgraph_id in &subgraph_ids {
            if let Some((node_indices, _)) = self.graph.subgraph_info.get(subgraph_id) {
                if node_indices.is_empty() {
                    continue;
                }

                let subgraph_nodes: Vec<usize> = node_indices
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

                if subgraph_nodes.is_empty() {
                    continue;
                }

                let min_layer = subgraph_nodes
                    .iter()
                    .map(|&idx| self.node_layers[idx])
                    .min()
                    .unwrap_or(0);

                let relative_layers =
                    self.compute_subgraph_relative_layers(&subgraph_nodes, back_edge_set);

                for (&node_idx, &rel_layer) in subgraph_nodes.iter().zip(relative_layers.iter()) {
                    self.node_layers[node_idx] = min_layer + rel_layer;
                }
            }
        }

        self.ensure_layer_constraints();
    }

    /// Compute relative layers for nodes within a subgraph based on internal edges.
    fn compute_subgraph_relative_layers(
        &self,
        nodes: &[usize],
        back_edge_set: &std::collections::HashSet<(usize, usize)>,
    ) -> Vec<usize> {
        let n = nodes.len();
        if n == 0 {
            return Vec::new();
        }

        let node_set: std::collections::HashSet<usize> = nodes.iter().copied().collect();

        let local_idx: HashMap<usize, usize> = nodes
            .iter()
            .enumerate()
            .map(|(i, &node)| (node, i))
            .collect();

        let mut in_degree = vec![0usize; n];
        for &node in nodes {
            for &pred in &self.graph.incoming[node] {
                if node_set.contains(&pred) && !back_edge_set.contains(&(pred, node)) {
                    in_degree[local_idx[&node]] += 1;
                }
            }
        }

        let mut relative_layers = vec![0usize; n];
        let mut queue: std::collections::VecDeque<usize> = std::collections::VecDeque::new();

        for (local_i, &deg) in in_degree.iter().enumerate() {
            if deg == 0 {
                queue.push_back(local_i);
            }
        }

        if queue.is_empty() && !nodes.is_empty() {
            queue.push_back(0);
        }

        while let Some(local_i) = queue.pop_front() {
            let node = nodes[local_i];
            let current_layer = relative_layers[local_i];

            for &succ in &self.graph.outgoing[node] {
                if let Some(&succ_local) = local_idx.get(&succ) {
                    if !back_edge_set.contains(&(node, succ)) {
                        relative_layers[succ_local] =
                            relative_layers[succ_local].max(current_layer + 1);

                        in_degree[succ_local] = in_degree[succ_local].saturating_sub(1);
                        if in_degree[succ_local] == 0 {
                            queue.push_back(succ_local);
                        }
                    }
                }
            }
        }

        relative_layers
    }

    /// Ensure layer constraints are satisfied after subgraph clustering.
    fn ensure_layer_constraints(&mut self) {
        let n = self.graph.node_count();
        let back_edge_set: std::collections::HashSet<(usize, usize)> =
            self.graph.back_edges.iter().cloned().collect();

        for _ in 0..n {
            let mut changed = false;
            for node in 0..n {
                for &pred in &self.graph.incoming[node] {
                    if !back_edge_set.contains(&(pred, node)) {
                        let min_layer = self.node_layers[pred] + 1;
                        if self.node_layers[node] < min_layer {
                            self.node_layers[node] = min_layer;
                            changed = true;
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }
    }

    /// Build the layers structure from node_layers assignments.
    fn build_layers(&mut self) {
        let max_layer = self.node_layers.iter().copied().max().unwrap_or(0);
        self.layers = vec![Vec::new(); max_layer + 1];

        for (node_idx, &layer) in self.node_layers.iter().enumerate() {
            self.layers[layer].push(node_idx);
        }

        let all_edge_positions: HashMap<usize, usize> = (0..self.graph.node_count())
            .map(|node| (node, self.get_min_incoming_edge_position(node)))
            .collect();

        for layer in &mut self.layers {
            layer.sort_by(|&a, &b| {
                let pos_a = all_edge_positions.get(&a).copied().unwrap_or(a);
                let pos_b = all_edge_positions.get(&b).copied().unwrap_or(b);
                pos_a.cmp(&pos_b)
            });
        }
    }

    /// Get the minimum position of a node in any predecessor's outgoing edge list.
    fn get_min_incoming_edge_position(&self, node: usize) -> usize {
        let mut min_pos = usize::MAX;
        for &pred in &self.graph.incoming[node] {
            if let Some(pos) = self.graph.outgoing[pred].iter().position(|&n| n == node) {
                min_pos = min_pos.min(pos);
            }
        }
        if min_pos == usize::MAX {
            node
        } else {
            min_pos
        }
    }

    /// Reduce edge crossings using the barycenter heuristic.
    fn reduce_crossings(&mut self) {
        let back_edge_set: std::collections::HashSet<(usize, usize)> =
            self.graph.back_edges.iter().cloned().collect();

        for _ in 0..self.config.crossing_reduction_iterations {
            for layer_idx in 1..self.layers.len() {
                self.order_layer_by_barycenter(layer_idx, true, &back_edge_set);
            }
            for layer_idx in (0..self.layers.len().saturating_sub(1)).rev() {
                self.order_layer_by_barycenter(layer_idx, false, &back_edge_set);
            }
        }
    }

    /// Order a single layer using barycenter of connected nodes in adjacent layer.
    fn order_layer_by_barycenter(
        &mut self,
        layer_idx: usize,
        use_predecessors: bool,
        back_edge_set: &std::collections::HashSet<(usize, usize)>,
    ) {
        let adjacent_layer_idx = if use_predecessors {
            layer_idx.saturating_sub(1)
        } else {
            (layer_idx + 1).min(self.layers.len().saturating_sub(1))
        };

        if adjacent_layer_idx == layer_idx {
            return;
        }

        let adjacent_positions: HashMap<usize, usize> = self.layers[adjacent_layer_idx]
            .iter()
            .enumerate()
            .map(|(pos, &node)| (node, pos))
            .collect();

        let current_positions: HashMap<usize, usize> = self.layers[layer_idx]
            .iter()
            .enumerate()
            .map(|(pos, &node)| (node, pos))
            .collect();

        let mut barycenters: Vec<(usize, f32)> = Vec::new();

        for &node in &self.layers[layer_idx] {
            let neighbors: Vec<usize> = if use_predecessors {
                self.graph.incoming[node]
                    .iter()
                    .filter(|&&pred| !back_edge_set.contains(&(pred, node)))
                    .copied()
                    .collect()
            } else {
                self.graph.outgoing[node]
                    .iter()
                    .filter(|&&succ| !back_edge_set.contains(&(node, succ)))
                    .copied()
                    .collect()
            };

            let barycenter = if neighbors.is_empty() {
                current_positions.get(&node).copied().unwrap_or(node) as f32
            } else {
                let sum: f32 = neighbors
                    .iter()
                    .filter_map(|n| adjacent_positions.get(n))
                    .map(|&pos| pos as f32)
                    .sum();
                let count = neighbors
                    .iter()
                    .filter(|n| adjacent_positions.contains_key(n))
                    .count();
                if count > 0 {
                    sum / count as f32
                } else {
                    current_positions.get(&node).copied().unwrap_or(node) as f32
                }
            };

            barycenters.push((node, barycenter));
        }

        let edge_positions: HashMap<usize, usize> = barycenters
            .iter()
            .map(|&(node, _)| (node, self.get_min_incoming_edge_position(node)))
            .collect();

        barycenters.sort_by(|a, b| match a.1.partial_cmp(&b.1) {
            Some(std::cmp::Ordering::Equal) | None => {
                let pos_a = edge_positions.get(&a.0).copied().unwrap_or(a.0);
                let pos_b = edge_positions.get(&b.0).copied().unwrap_or(b.0);
                pos_a.cmp(&pos_b)
            }
            Some(ord) => ord,
        });

        self.layers[layer_idx] = barycenters.into_iter().map(|(node, _)| node).collect();
    }

    /// Assign final coordinates to all nodes.
    fn assign_coordinates_with_subgraphs(
        self,
        _subgraph_layouts: &HashMap<String, SubgraphInternalLayout>,
    ) -> FlowchartLayout {
        let is_horizontal = matches!(
            self.direction,
            FlowDirection::LeftRight | FlowDirection::RightLeft
        );
        let is_reversed = matches!(
            self.direction,
            FlowDirection::BottomUp | FlowDirection::RightLeft
        );

        let mut layout = FlowchartLayout::default();
        let margin = self.config.margin;

        let mut layer_cross_sizes: Vec<f32> = Vec::new();
        for layer in &self.layers {
            let mut size: f32 = 0.0;
            for &node_idx in layer {
                let node_size = self.graph.node_sizes[node_idx];
                size += if is_horizontal {
                    node_size.y
                } else {
                    node_size.x
                };
            }
            size += (layer.len().saturating_sub(1)) as f32
                * if is_horizontal {
                    self.config.node_spacing.y
                } else {
                    self.config.node_spacing.x
                };
            layer_cross_sizes.push(size);
        }
        let max_cross_size = layer_cross_sizes.iter().copied().fold(0.0_f32, f32::max);

        let layer_main_sizes: Vec<f32> = self
            .layers
            .iter()
            .map(|layer| {
                layer
                    .iter()
                    .map(|&idx| {
                        let size = self.graph.node_sizes[idx];
                        if is_horizontal {
                            size.x
                        } else {
                            size.y
                        }
                    })
                    .fold(0.0_f32, f32::max)
            })
            .collect();

        // Position nodes layer by layer
        let mut current_main = margin;
        let mut max_x: f32 = 0.0;
        let mut max_y: f32 = 0.0;

        for (layer_idx, layer) in self.layers.iter().enumerate() {
            let layer_cross_size = layer_cross_sizes[layer_idx];

            let start_cross = if is_horizontal {
                margin + (max_cross_size - layer_cross_size) / 2.0
            } else {
                (self.available_width - layer_cross_size).max(margin * 2.0) / 2.0
            };

            let mut current_cross = start_cross;

            for &node_idx in layer {
                let node_id = &self.graph.node_ids[node_idx];
                let size = self.graph.node_sizes[node_idx];

                let pos = if is_horizontal {
                    Pos2::new(current_main, current_cross)
                } else {
                    Pos2::new(current_cross, current_main)
                };

                layout
                    .nodes
                    .insert(node_id.clone(), NodeLayout { pos, size });

                max_x = max_x.max(pos.x + size.x);
                max_y = max_y.max(pos.y + size.y);

                current_cross += if is_horizontal {
                    size.y + self.config.node_spacing.y
                } else {
                    size.x + self.config.node_spacing.x
                };
            }

            current_main += layer_main_sizes[layer_idx]
                + if is_horizontal {
                    self.config.node_spacing.x
                } else {
                    self.config.node_spacing.y
                };
        }

        // Handle reversed directions (BT, RL)
        if is_reversed {
            let total = if is_horizontal { max_x } else { max_y };
            for node_layout in layout.nodes.values_mut() {
                if is_horizontal {
                    node_layout.pos.x = total - node_layout.pos.x - node_layout.size.x + margin;
                } else {
                    node_layout.pos.y = total - node_layout.pos.y - node_layout.size.y + margin;
                }
            }
        }

        // Convert back-edge indices to node IDs
        for &(from_idx, to_idx) in &self.graph.back_edges {
            let from_id = self.graph.node_ids[from_idx].clone();
            let to_id = self.graph.node_ids[to_idx].clone();
            layout.back_edges.insert((from_id, to_id));
        }

        layout.total_size = Vec2::new(max_x + margin, max_y + margin);
        layout
    }
}
