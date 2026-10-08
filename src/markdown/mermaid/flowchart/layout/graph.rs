//! Internal graph representation for layout algorithms.

use std::collections::HashMap;

use crate::markdown::mermaid::text::TextMeasurer;

use super::super::types::*;
use super::config::FlowLayoutConfig;

/// Internal graph representation for layout algorithms.
#[derive(Debug)]
pub(crate) struct FlowGraph {
    /// Node IDs in order
    pub node_ids: Vec<String>,
    /// Map from node ID to index (kept for potential future edge routing enhancements)
    #[allow(dead_code)]
    pub id_to_index: HashMap<String, usize>,
    /// Node sizes (indexed by node index)
    pub node_sizes: Vec<egui::Vec2>,
    /// Outgoing edges: node_index -> Vec<target_index>
    pub outgoing: Vec<Vec<usize>>,
    /// Incoming edges: node_index -> Vec<source_index>
    pub incoming: Vec<Vec<usize>>,
    /// Back-edges detected during cycle breaking (source, target)
    pub back_edges: Vec<(usize, usize)>,
    /// Subgraph membership: node_index -> Option<subgraph_id>
    /// None means the node is not in any subgraph
    pub node_subgraph: Vec<Option<String>>,
    /// Subgraph info: subgraph_id -> (node_indices, child_subgraph_ids)
    pub subgraph_info: HashMap<String, (Vec<usize>, Vec<String>)>,
    /// Subgraph direction overrides: subgraph_id -> direction
    pub subgraph_directions: HashMap<String, FlowDirection>,
}

impl FlowGraph {
    /// Build graph from flowchart AST with text measurement.
    pub fn from_flowchart(
        flowchart: &Flowchart,
        font_size: f32,
        text_measurer: &(impl TextMeasurer + ?Sized),
        config: &FlowLayoutConfig,
    ) -> Self {
        let n = flowchart.nodes.len();
        let mut node_ids = Vec::with_capacity(n);
        let mut id_to_index = HashMap::with_capacity(n);
        let mut node_sizes = Vec::with_capacity(n);
        let mut outgoing = vec![Vec::new(); n];
        let mut incoming = vec![Vec::new(); n];

        // Build node index mapping and compute sizes
        for (idx, node) in flowchart.nodes.iter().enumerate() {
            node_ids.push(node.id.clone());
            id_to_index.insert(node.id.clone(), idx);

            // Measure text and compute node size
            let text_size = text_measurer.measure(&node.label, font_size);
            let adjusted_width = text_size.width * config.text_width_factor;

            let (text_width, text_height) =
                if adjusted_width + config.node_padding.x * 2.0 > config.max_node_width {
                    let wrap_width = config.max_node_width - config.node_padding.x * 2.0;
                    let wrapped = text_measurer.measure_wrapped(&node.label, font_size, wrap_width);
                    (wrapped.width * config.text_width_factor, wrapped.height)
                } else {
                    (adjusted_width, text_size.height)
                };

            let size = egui::Vec2::new(
                (text_width + config.node_padding.x * 2.0).max(80.0),
                (text_height + config.node_padding.y * 2.0).max(40.0),
            );
            node_sizes.push(size);
        }

        // Build adjacency lists
        for edge in &flowchart.edges {
            if let (Some(&from_idx), Some(&to_idx)) =
                (id_to_index.get(&edge.from), id_to_index.get(&edge.to))
            {
                outgoing[from_idx].push(to_idx);
                incoming[to_idx].push(from_idx);
            }
        }

        // Build subgraph membership mapping
        let mut node_subgraph: Vec<Option<String>> = vec![None; n];
        let mut subgraph_info: HashMap<String, (Vec<usize>, Vec<String>)> = HashMap::new();
        let mut subgraph_directions: HashMap<String, FlowDirection> = HashMap::new();

        for subgraph in &flowchart.subgraphs {
            let mut node_indices = Vec::new();
            for node_id in &subgraph.node_ids {
                if let Some(&idx) = id_to_index.get(node_id) {
                    node_indices.push(idx);
                    if node_subgraph[idx].is_none() {
                        node_subgraph[idx] = Some(subgraph.id.clone());
                    }
                }
            }
            subgraph_info.insert(
                subgraph.id.clone(),
                (node_indices, subgraph.child_subgraph_ids.clone()),
            );
            if let Some(direction) = subgraph.direction {
                subgraph_directions.insert(subgraph.id.clone(), direction);
            }
        }

        FlowGraph {
            node_ids,
            id_to_index,
            node_sizes,
            outgoing,
            incoming,
            back_edges: Vec::new(),
            node_subgraph,
            subgraph_info,
            subgraph_directions,
        }
    }

    pub fn node_count(&self) -> usize {
        self.node_ids.len()
    }
}
