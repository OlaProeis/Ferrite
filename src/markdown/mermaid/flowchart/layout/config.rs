//! Layout configuration for flowchart diagrams.

use egui::Vec2;

/// Configuration for flowchart layout.
#[derive(Debug, Clone)]
pub(crate) struct FlowLayoutConfig {
    pub node_padding: Vec2,
    pub node_spacing: Vec2,
    pub max_node_width: f32,
    pub text_width_factor: f32,
    pub margin: f32,
    pub crossing_reduction_iterations: usize,
    /// Padding around subgraph content
    pub subgraph_padding: f32,
    /// Height reserved for subgraph title
    pub subgraph_title_height: f32,
    /// Extra margin between nested subgraph boundaries
    pub nested_subgraph_margin: f32,
}

impl FlowLayoutConfig {
    /// Scale cross-axis spacing when a layer has many siblings (e.g. `&` fan-out).
    pub fn adaptive_cross_spacing(&self, base_spacing: f32, layer_len: usize) -> f32 {
        if layer_len > 4 {
            let factor = (1.0 + 0.1 * (layer_len - 4) as f32).min(2.0);
            base_spacing * factor
        } else {
            base_spacing
        }
    }

    /// Use more crossing-reduction passes on dense graphs.
    pub fn crossing_iterations_for(&self, edge_count: usize) -> usize {
        if edge_count > 30 {
            self.crossing_reduction_iterations.max(18)
        } else if edge_count > 15 {
            self.crossing_reduction_iterations.max(12)
        } else {
            self.crossing_reduction_iterations
        }
    }
}
