//! Tree Viewer for JSON, YAML, and TOML files
//!
//! This module provides a collapsible tree view for structured data files,
//! with syntax coloring, inline editing, and path copying.
//!
//! # Features
//! - Unified TreeNode representation for JSON, YAML, and TOML
//! - Collapsible tree structure with expand/collapse toggles
//! - Syntax coloring (keys: blue, strings: green, numbers: orange, null: gray)
//! - Inline editing for values
//! - Context menu with "Copy Path" (JSONPath format)
//! - Large file handling (>1MB warning, lazy expand)
//! - Error handling with raw fallback

// Allow dead code - this module has utility functions and fields for future extensibility
// - too_many_arguments: Rendering functions need many parameters for tree traversal
// - ptr_arg: Using &mut Vec for dynamic tree modification
#![allow(clippy::too_many_arguments)]
#![allow(clippy::ptr_arg)]
#![allow(dead_code)]

use eframe::egui::{self, Color32, RichText, ScrollArea, TextEdit, Ui, Vec2};
use log::warn;
use std::collections::HashMap;

// ─────────────────────────────────────────────────────────────────────────────
// File Type Detection
// ─────────────────────────────────────────────────────────────────────────────

/// Supported structured file types for tree viewing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuredFileType {
    Json,
    Yaml,
    Toml,
}

impl StructuredFileType {
    /// Detect file type from extension.
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "json" => Some(Self::Json),
            "yaml" | "yml" => Some(Self::Yaml),
            "toml" => Some(Self::Toml),
            _ => None,
        }
    }

    /// Get display name for the file type.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Json => "JSON",
            Self::Yaml => "YAML",
            Self::Toml => "TOML",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tree Node Model
// ─────────────────────────────────────────────────────────────────────────────

/// Unified tree node representation for all supported formats.
#[derive(Debug, Clone)]
pub enum TreeNode {
    /// Null value
    Null,
    /// Boolean value
    Bool(bool),
    /// Integer value
    Integer(i64),
    /// Floating point value
    Float(f64),
    /// String value
    String(String),
    /// Array of nodes
    Array(Vec<TreeNode>),
    /// Object/Map with string keys
    Object(Vec<(String, TreeNode)>),
}

impl TreeNode {
    /// Get a short type description for display.
    pub fn type_hint(&self) -> &'static str {
        match self {
            TreeNode::Null => "null",
            TreeNode::Bool(_) => "bool",
            TreeNode::Integer(_) => "int",
            TreeNode::Float(_) => "float",
            TreeNode::String(_) => "string",
            TreeNode::Array(arr) => {
                if arr.is_empty() {
                    "[]"
                } else {
                    "array"
                }
            }
            TreeNode::Object(obj) => {
                if obj.is_empty() {
                    "{}"
                } else {
                    "object"
                }
            }
        }
    }

    /// Check if this node is a container (array or object).
    pub fn is_container(&self) -> bool {
        matches!(self, TreeNode::Array(_) | TreeNode::Object(_))
    }

    /// Get the number of children for containers.
    pub fn child_count(&self) -> usize {
        match self {
            TreeNode::Array(arr) => arr.len(),
            TreeNode::Object(obj) => obj.len(),
            _ => 0,
        }
    }

    /// Convert back to JSON string for editing.
    pub fn to_json_string(&self) -> String {
        match self {
            TreeNode::Null => "null".to_string(),
            TreeNode::Bool(b) => b.to_string(),
            TreeNode::Integer(i) => i.to_string(),
            TreeNode::Float(f) => {
                if f.is_nan() {
                    "NaN".to_string()
                } else if f.is_infinite() {
                    if *f > 0.0 {
                        "Infinity".to_string()
                    } else {
                        "-Infinity".to_string()
                    }
                } else {
                    f.to_string()
                }
            }
            TreeNode::String(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
            TreeNode::Array(_) | TreeNode::Object(_) => "[...]".to_string(),
        }
    }

    /// Try to parse a value string and update this node.
    /// Returns true if parsing succeeded.
    pub fn try_update_from_string(&mut self, input: &str) -> bool {
        let trimmed = input.trim();

        // Try null
        if trimmed == "null" {
            *self = TreeNode::Null;
            return true;
        }

        // Try boolean
        if trimmed == "true" {
            *self = TreeNode::Bool(true);
            return true;
        }
        if trimmed == "false" {
            *self = TreeNode::Bool(false);
            return true;
        }

        // Try integer
        if let Ok(i) = trimmed.parse::<i64>() {
            *self = TreeNode::Integer(i);
            return true;
        }

        // Try float
        if let Ok(f) = trimmed.parse::<f64>() {
            *self = TreeNode::Float(f);
            return true;
        }

        // Try string (with or without quotes)
        if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
            let inner = &trimmed[1..trimmed.len() - 1];
            // Simple unescape
            let unescaped = inner.replace("\\\"", "\"").replace("\\\\", "\\");
            *self = TreeNode::String(unescaped);
            return true;
        }

        // Treat as unquoted string
        *self = TreeNode::String(trimmed.to_string());
        true
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Parsing
// ─────────────────────────────────────────────────────────────────────────────

/// Parse error with optional line number.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: Option<usize>,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(line) = self.line {
            write!(f, "Line {}: {}", line, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

/// Parse content into a TreeNode based on file type.
pub fn parse_structured_content(
    content: &str,
    file_type: StructuredFileType,
) -> Result<TreeNode, ParseError> {
    match file_type {
        StructuredFileType::Json => parse_json(content),
        StructuredFileType::Yaml => parse_yaml(content),
        StructuredFileType::Toml => parse_toml(content),
    }
}

/// Parse JSON content.
fn parse_json(content: &str) -> Result<TreeNode, ParseError> {
    let value: serde_json::Value = serde_json::from_str(content).map_err(|e| {
        // Try to extract line number from error
        let line = e.line();
        ParseError {
            message: e.to_string(),
            line: if line > 0 { Some(line) } else { None },
        }
    })?;
    Ok(json_to_tree(&value))
}

fn json_to_tree(value: &serde_json::Value) -> TreeNode {
    match value {
        serde_json::Value::Null => TreeNode::Null,
        serde_json::Value::Bool(b) => TreeNode::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                TreeNode::Integer(i)
            } else if let Some(f) = n.as_f64() {
                TreeNode::Float(f)
            } else {
                TreeNode::String(n.to_string())
            }
        }
        serde_json::Value::String(s) => TreeNode::String(s.clone()),
        serde_json::Value::Array(arr) => TreeNode::Array(arr.iter().map(json_to_tree).collect()),
        serde_json::Value::Object(obj) => TreeNode::Object(
            obj.iter()
                .map(|(k, v)| (k.clone(), json_to_tree(v)))
                .collect(),
        ),
    }
}

/// Parse YAML content.
fn parse_yaml(content: &str) -> Result<TreeNode, ParseError> {
    let value: serde_yaml::Value = serde_yaml::from_str(content).map_err(|e| {
        // serde_yaml errors include location info in the message
        ParseError {
            message: e.to_string(),
            line: None, // Location is in message
        }
    })?;
    Ok(yaml_to_tree(&value))
}

fn yaml_to_tree(value: &serde_yaml::Value) -> TreeNode {
    match value {
        serde_yaml::Value::Null => TreeNode::Null,
        serde_yaml::Value::Bool(b) => TreeNode::Bool(*b),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                TreeNode::Integer(i)
            } else if let Some(f) = n.as_f64() {
                TreeNode::Float(f)
            } else {
                TreeNode::String(n.to_string())
            }
        }
        serde_yaml::Value::String(s) => TreeNode::String(s.clone()),
        serde_yaml::Value::Sequence(seq) => TreeNode::Array(seq.iter().map(yaml_to_tree).collect()),
        serde_yaml::Value::Mapping(map) => TreeNode::Object(
            map.iter()
                .map(|(k, v)| {
                    let key = match k {
                        serde_yaml::Value::String(s) => s.clone(),
                        _ => format!("{:?}", k),
                    };
                    (key, yaml_to_tree(v))
                })
                .collect(),
        ),
        serde_yaml::Value::Tagged(tagged) => {
            // Handle tagged values by just using the value
            yaml_to_tree(&tagged.value)
        }
    }
}

/// Parse TOML content.
fn parse_toml(content: &str) -> Result<TreeNode, ParseError> {
    let value: toml::Value = toml::from_str(content).map_err(|e| {
        // toml errors may include span info
        ParseError {
            message: e.to_string(),
            line: None,
        }
    })?;
    Ok(toml_to_tree(&value))
}

fn toml_to_tree(value: &toml::Value) -> TreeNode {
    match value {
        toml::Value::String(s) => TreeNode::String(s.clone()),
        toml::Value::Integer(i) => TreeNode::Integer(*i),
        toml::Value::Float(f) => TreeNode::Float(*f),
        toml::Value::Boolean(b) => TreeNode::Bool(*b),
        toml::Value::Datetime(dt) => TreeNode::String(dt.to_string()),
        toml::Value::Array(arr) => TreeNode::Array(arr.iter().map(toml_to_tree).collect()),
        toml::Value::Table(table) => TreeNode::Object(
            table
                .iter()
                .map(|(k, v)| (k.clone(), toml_to_tree(v)))
                .collect(),
        ),
    }
}

/// Serialize TreeNode back to the original format.
pub fn serialize_tree(root: &TreeNode, file_type: StructuredFileType) -> Result<String, String> {
    match file_type {
        StructuredFileType::Json => serialize_to_json(root),
        StructuredFileType::Yaml => serialize_to_yaml(root),
        StructuredFileType::Toml => serialize_to_toml(root),
    }
}

fn tree_to_json_value(node: &TreeNode) -> serde_json::Value {
    match node {
        TreeNode::Null => serde_json::Value::Null,
        TreeNode::Bool(b) => serde_json::Value::Bool(*b),
        TreeNode::Integer(i) => serde_json::Value::Number((*i).into()),
        TreeNode::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        TreeNode::String(s) => serde_json::Value::String(s.clone()),
        TreeNode::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(tree_to_json_value).collect())
        }
        TreeNode::Object(obj) => serde_json::Value::Object(
            obj.iter()
                .map(|(k, v)| (k.clone(), tree_to_json_value(v)))
                .collect(),
        ),
    }
}

fn serialize_to_json(root: &TreeNode) -> Result<String, String> {
    let value = tree_to_json_value(root);
    serde_json::to_string_pretty(&value).map_err(|e| e.to_string())
}

fn tree_to_yaml_value(node: &TreeNode) -> serde_yaml::Value {
    match node {
        TreeNode::Null => serde_yaml::Value::Null,
        TreeNode::Bool(b) => serde_yaml::Value::Bool(*b),
        TreeNode::Integer(i) => serde_yaml::Value::Number((*i).into()),
        TreeNode::Float(f) => serde_yaml::Value::Number(serde_yaml::Number::from(*f)),
        TreeNode::String(s) => serde_yaml::Value::String(s.clone()),
        TreeNode::Array(arr) => {
            serde_yaml::Value::Sequence(arr.iter().map(tree_to_yaml_value).collect())
        }
        TreeNode::Object(obj) => {
            let map: serde_yaml::Mapping = obj
                .iter()
                .map(|(k, v)| (serde_yaml::Value::String(k.clone()), tree_to_yaml_value(v)))
                .collect();
            serde_yaml::Value::Mapping(map)
        }
    }
}

fn serialize_to_yaml(root: &TreeNode) -> Result<String, String> {
    let value = tree_to_yaml_value(root);
    serde_yaml::to_string(&value).map_err(|e| e.to_string())
}

fn tree_to_toml_value(node: &TreeNode) -> Option<toml::Value> {
    match node {
        // TOML doesn't support null
        TreeNode::Null => None,
        TreeNode::Bool(b) => Some(toml::Value::Boolean(*b)),
        TreeNode::Integer(i) => Some(toml::Value::Integer(*i)),
        TreeNode::Float(f) => Some(toml::Value::Float(*f)),
        TreeNode::String(s) => Some(toml::Value::String(s.clone())),
        TreeNode::Array(arr) => {
            let values: Vec<toml::Value> = arr.iter().filter_map(tree_to_toml_value).collect();
            Some(toml::Value::Array(values))
        }
        TreeNode::Object(obj) => {
            let mut table = toml::map::Map::new();
            for (k, v) in obj {
                if let Some(tv) = tree_to_toml_value(v) {
                    table.insert(k.clone(), tv);
                }
            }
            Some(toml::Value::Table(table))
        }
    }
}

fn serialize_to_toml(root: &TreeNode) -> Result<String, String> {
    // TOML requires root to be a table
    match root {
        TreeNode::Object(_) => {
            if let Some(value) = tree_to_toml_value(root) {
                toml::to_string_pretty(&value).map_err(|e| e.to_string())
            } else {
                Err("Failed to convert tree to TOML".to_string())
            }
        }
        _ => Err("TOML root must be a table/object".to_string()),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tree Viewer State
// ─────────────────────────────────────────────────────────────────────────────

/// State for the tree viewer widget.
#[derive(Debug, Clone, Default)]
pub struct TreeViewerState {
    /// Expanded paths (stored as dot-separated paths like "root.key.0")
    expanded: HashMap<String, bool>,
    /// Currently editing path (if any)
    editing_path: Option<String>,
    /// Edit buffer for inline editing
    edit_buffer: String,
    /// Whether editing has validation error
    edit_error: bool,
    /// Path copied to clipboard (for visual feedback)
    copied_path: Option<String>,
    /// Show raw view instead of tree
    pub show_raw: bool,
    /// Large file warning dismissed
    large_file_warning_dismissed: bool,
}

impl TreeViewerState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a path is expanded.
    pub fn is_expanded(&self, path: &str) -> bool {
        *self.expanded.get(path).unwrap_or(&true) // Default to expanded
    }

    /// Toggle expansion state.
    pub fn toggle_expanded(&mut self, path: &str) {
        let current = self.is_expanded(path);
        self.expanded.insert(path.to_string(), !current);
    }

    /// Expand all nodes.
    pub fn expand_all(&mut self) {
        self.expanded.clear(); // Will use default (expanded)
    }

    /// Collapse all nodes.
    pub fn collapse_all(&mut self, root: &TreeNode) {
        self.expanded.clear();
        self.collapse_recursive(root, "root");
    }

    fn collapse_recursive(&mut self, node: &TreeNode, path: &str) {
        match node {
            TreeNode::Array(arr) => {
                self.expanded.insert(path.to_string(), false);
                for (i, child) in arr.iter().enumerate() {
                    self.collapse_recursive(child, &format!("{}.{}", path, i));
                }
            }
            TreeNode::Object(obj) => {
                self.expanded.insert(path.to_string(), false);
                for (key, child) in obj {
                    self.collapse_recursive(child, &format!("{}.{}", path, key));
                }
            }
            _ => {}
        }
    }

    /// Start editing a value.
    pub fn start_editing(&mut self, path: &str, current_value: &TreeNode) {
        self.editing_path = Some(path.to_string());
        self.edit_buffer = match current_value {
            TreeNode::Null => "null".to_string(),
            TreeNode::Bool(b) => b.to_string(),
            TreeNode::Integer(i) => i.to_string(),
            TreeNode::Float(f) => f.to_string(),
            TreeNode::String(s) => s.clone(),
            _ => String::new(),
        };
        self.edit_error = false;
    }

    /// Cancel editing.
    pub fn cancel_editing(&mut self) {
        self.editing_path = None;
        self.edit_buffer.clear();
        self.edit_error = false;
    }

    /// Check if currently editing.
    pub fn is_editing(&self, path: &str) -> bool {
        self.editing_path.as_deref() == Some(path)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tree Viewer Colors
// ─────────────────────────────────────────────────────────────────────────────

/// Colors for tree viewer syntax highlighting.
#[derive(Debug, Clone)]
pub struct TreeViewerColors {
    pub key: Color32,
    pub string: Color32,
    pub number: Color32,
    pub boolean: Color32,
    pub null: Color32,
    pub bracket: Color32,
    pub background: Color32,
    pub error: Color32,
}

impl TreeViewerColors {
    pub fn dark() -> Self {
        Self {
            key: Color32::from_rgb(100, 180, 255),     // Blue
            string: Color32::from_rgb(150, 200, 130),  // Green
            number: Color32::from_rgb(255, 180, 100),  // Orange
            boolean: Color32::from_rgb(200, 130, 200), // Purple
            null: Color32::from_rgb(150, 150, 150),    // Gray
            bracket: Color32::from_rgb(180, 180, 180), // Light gray
            background: Color32::from_rgb(30, 30, 30),
            error: Color32::from_rgb(255, 100, 100), // Red
        }
    }

    pub fn light() -> Self {
        Self {
            key: Color32::from_rgb(0, 100, 180),      // Blue
            string: Color32::from_rgb(50, 120, 50),   // Green
            number: Color32::from_rgb(200, 100, 0),   // Orange
            boolean: Color32::from_rgb(150, 50, 150), // Purple
            null: Color32::from_rgb(120, 120, 120),   // Gray
            bracket: Color32::from_rgb(80, 80, 80),   // Dark gray
            background: Color32::from_rgb(255, 255, 255),
            error: Color32::from_rgb(200, 50, 50), // Red
        }
    }

    pub fn from_dark_mode(dark_mode: bool) -> Self {
        if dark_mode {
            Self::dark()
        } else {
            Self::light()
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tree Viewer Widget
// ─────────────────────────────────────────────────────────────────────────────

/// Output from the tree viewer widget.
#[derive(Debug, Clone)]
pub struct TreeViewerOutput {
    /// Whether the tree was modified (inline edit applied)
    pub changed: bool,
    /// New content if changed
    pub new_content: Option<String>,
    /// Whether user requested raw view toggle
    pub toggle_raw_requested: bool,
    /// Current scroll offset (for sync scrolling)
    pub scroll_offset: f32,
}

/// Large file threshold in bytes (1MB).
const LARGE_FILE_THRESHOLD: usize = 1_000_000;

/// Tree viewer widget.
pub struct TreeViewer<'a> {
    content: &'a mut String,
    file_type: StructuredFileType,
    state: &'a mut TreeViewerState,
    font_size: f32,
}

impl<'a> TreeViewer<'a> {
    pub fn new(
        content: &'a mut String,
        file_type: StructuredFileType,
        state: &'a mut TreeViewerState,
    ) -> Self {
        Self {
            content,
            file_type,
            state,
            font_size: 14.0,
        }
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub fn show(mut self, ui: &mut Ui) -> TreeViewerOutput {
        let colors = TreeViewerColors::from_dark_mode(ui.visuals().dark_mode);
        let mut output = TreeViewerOutput {
            changed: false,
            new_content: None,
            toggle_raw_requested: false,
            scroll_offset: 0.0,
        };

        // Large file warning
        let content_size = self.content.len();
        let is_large = content_size > LARGE_FILE_THRESHOLD;

        if is_large && !self.state.large_file_warning_dismissed && !self.state.show_raw {
            ui.horizontal(|ui| {
                ui.colored_label(
                    colors.error,
                    format!(
                        "⚠ Large file ({:.1} MB). Tree view may be slow.",
                        content_size as f64 / 1_000_000.0
                    ),
                );
                if ui.button("Dismiss").clicked() {
                    self.state.large_file_warning_dismissed = true;
                }
                if ui.button("Show Raw").clicked() {
                    self.state.show_raw = true;
                }
            });
            ui.separator();
        }

        // Toolbar
        ui.horizontal(|ui| {
            ui.label(RichText::new(self.file_type.display_name()).strong());
            ui.separator();

            if !self.state.show_raw {
                if ui.button("▼ Expand All").clicked() {
                    self.state.expand_all();
                }
                if ui.button("▶ Collapse All").clicked() {
                    // Need to parse to collapse
                    if let Ok(tree) = parse_structured_content(self.content, self.file_type) {
                        self.state.collapse_all(&tree);
                    }
                }
                ui.separator();
            }

            let raw_label = if self.state.show_raw {
                "📊 Tree View"
            } else {
                "📝 Raw View"
            };
            if ui.button(raw_label).clicked() {
                self.state.show_raw = !self.state.show_raw;
                output.toggle_raw_requested = true;
            }
        });
        ui.separator();

        // Content area
        if self.state.show_raw {
            // Raw view with syntax highlighting
            output.scroll_offset = self.show_raw_view(ui, &colors);
        } else {
            // Tree view
            match parse_structured_content(self.content, self.file_type) {
                Ok(mut tree) => {
                    let tree_output = self.show_tree_view(ui, &mut tree, &colors);
                    output.scroll_offset = tree_output.scroll_offset;
                    if tree_output.changed {
                        // Serialize back to content
                        match serialize_tree(&tree, self.file_type) {
                            Ok(new_content) => {
                                *self.content = new_content.clone();
                                output.changed = true;
                                output.new_content = Some(new_content);
                            }
                            Err(e) => {
                                warn!("Failed to serialize tree: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    // Parse error - show error and fall back to raw
                    self.show_parse_error(ui, &e, &colors);
                    output.scroll_offset = self.show_raw_view(ui, &colors);
                }
            }
        }

        output
    }

    fn show_parse_error(&self, ui: &mut Ui, error: &ParseError, colors: &TreeViewerColors) {
        ui.horizontal(|ui| {
            ui.colored_label(colors.error, "⚠ Parse Error:");
            ui.colored_label(colors.error, &error.message);
        });
        ui.separator();
    }

    fn show_raw_view(&self, ui: &mut Ui, _colors: &TreeViewerColors) -> f32 {
        // Use syntax highlighting if available, otherwise plain text
        let scroll_output = ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // For now, simple monospace display
                // TODO(enhancement): Integrate with syntect for syntax highlighting in tree viewer
                let text = self.content.as_str();
                ui.add(
                    TextEdit::multiline(&mut text.to_string())
                        .code_editor()
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .interactive(false),
                );
            });
        scroll_output.state.offset.y
    }

    fn show_tree_view(
        &mut self,
        ui: &mut Ui,
        tree: &mut TreeNode,
        colors: &TreeViewerColors,
    ) -> TreeViewerOutput {
        let mut output = TreeViewerOutput {
            changed: false,
            new_content: None,
            toggle_raw_requested: false,
            scroll_offset: 0.0,
        };

        let scroll_output = ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(4.0, 2.0);

                let changed = self.render_node(ui, tree, "root", "", colors, 0);
                output.changed = changed;
            });

        output.scroll_offset = scroll_output.state.offset.y;

        output
    }

    /// Render a single tree node recursively.
    /// Returns true if the node was modified.
    fn render_node(
        &mut self,
        ui: &mut Ui,
        node: &mut TreeNode,
        path: &str,
        key: &str,
        colors: &TreeViewerColors,
        depth: usize,
    ) -> bool {
        let indent = depth as f32 * 16.0;

        match node {
            TreeNode::Array(arr) => self.render_array(ui, arr, path, key, colors, depth, indent),
            TreeNode::Object(obj) => self.render_object(ui, obj, path, key, colors, depth, indent),
            _ => {
                // Leaf node
                self.render_leaf(ui, node, path, key, colors, indent)
            }
        }
    }

    fn render_array(
        &mut self,
        ui: &mut Ui,
        arr: &mut Vec<TreeNode>,
        path: &str,
        key: &str,
        colors: &TreeViewerColors,
        depth: usize,
        indent: f32,
    ) -> bool {
        let mut changed = false;
        let is_expanded = self.state.is_expanded(path);
        let count = arr.len();

        ui.horizontal(|ui| {
            ui.add_space(indent);

            // Expand/collapse toggle
            let toggle_text = if is_expanded { "▼" } else { "▶" };
            if ui.small_button(toggle_text).clicked() {
                self.state.toggle_expanded(path);
            }

            // Key if present
            if !key.is_empty() {
                ui.colored_label(colors.key, format!("{}:", key));
            }

            // Array indicator
            ui.colored_label(colors.bracket, format!("[{} items]", count));

            // Context menu
            self.add_context_menu(ui, path, colors);
        });

        // Children if expanded
        if is_expanded {
            for (i, child) in arr.iter_mut().enumerate() {
                let child_path = format!("{}.{}", path, i);
                let child_key = format!("[{}]", i);
                if self.render_node(ui, child, &child_path, &child_key, colors, depth + 1) {
                    changed = true;
                }
            }
        }

        changed
    }

    fn render_object(
        &mut self,
        ui: &mut Ui,
        obj: &mut Vec<(String, TreeNode)>,
        path: &str,
        key: &str,
        colors: &TreeViewerColors,
        depth: usize,
        indent: f32,
    ) -> bool {
        let mut changed = false;
        let is_expanded = self.state.is_expanded(path);
        let count = obj.len();

        ui.horizontal(|ui| {
            ui.add_space(indent);

            // Expand/collapse toggle
            let toggle_text = if is_expanded { "▼" } else { "▶" };
            if ui.small_button(toggle_text).clicked() {
                self.state.toggle_expanded(path);
            }

            // Key if present
            if !key.is_empty() {
                ui.colored_label(colors.key, format!("{}:", key));
            }

            // Object indicator
            ui.colored_label(colors.bracket, format!("{{...}} ({} keys)", count));

            // Context menu
            self.add_context_menu(ui, path, colors);
        });

        // Children if expanded
        if is_expanded {
            for (child_key, child_node) in obj.iter_mut() {
                let child_path = format!("{}.{}", path, child_key);
                if self.render_node(ui, child_node, &child_path, child_key, colors, depth + 1) {
                    changed = true;
                }
            }
        }

        changed
    }

    fn render_leaf(
        &mut self,
        ui: &mut Ui,
        node: &mut TreeNode,
        path: &str,
        key: &str,
        colors: &TreeViewerColors,
        indent: f32,
    ) -> bool {
        let mut changed = false;
        let is_editing = self.state.is_editing(path);

        ui.horizontal(|ui| {
            ui.add_space(indent);
            ui.add_space(20.0); // Space for alignment with containers

            // Key
            if !key.is_empty() {
                ui.colored_label(colors.key, format!("{}:", key));
            }

            if is_editing {
                // Editing mode
                let response = ui.add(
                    TextEdit::singleline(&mut self.state.edit_buffer)
                        .desired_width(200.0)
                        .font(egui::TextStyle::Monospace),
                );

                // Handle Enter/Escape
                if response.lost_focus() {
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        // Try to apply
                        let mut test_node = node.clone();
                        if test_node.try_update_from_string(&self.state.edit_buffer) {
                            *node = test_node;
                            changed = true;
                            self.state.cancel_editing();
                        } else {
                            self.state.edit_error = true;
                        }
                    } else {
                        // Escape or click away
                        self.state.cancel_editing();
                    }
                }

                if self.state.edit_error {
                    ui.colored_label(colors.error, "⚠ Invalid");
                }
            } else {
                // Display mode
                let (text, color) = self.format_leaf_value(node, colors);
                let label = ui.colored_label(color, &text);

                // Double-click to edit
                if label.double_clicked() && !node.is_container() {
                    self.state.start_editing(path, node);
                }

                // Context menu
                self.add_context_menu(ui, path, colors);
            }
        });

        changed
    }

    fn format_leaf_value(&self, node: &TreeNode, colors: &TreeViewerColors) -> (String, Color32) {
        match node {
            TreeNode::Null => ("null".to_string(), colors.null),
            TreeNode::Bool(b) => (b.to_string(), colors.boolean),
            TreeNode::Integer(i) => (i.to_string(), colors.number),
            TreeNode::Float(f) => (format!("{}", f), colors.number),
            TreeNode::String(s) => {
                // Truncate long strings
                let display = if s.len() > 100 {
                    format!("\"{}...\"", &s[..97])
                } else {
                    format!("\"{}\"", s)
                };
                (display, colors.string)
            }
            _ => ("???".to_string(), colors.null),
        }
    }

    fn add_context_menu(&mut self, ui: &mut Ui, path: &str, colors: &TreeViewerColors) {
        // Show "copied" feedback
        if self.state.copied_path.as_deref() == Some(path) {
            ui.colored_label(colors.string, "✓");
        }

        ui.menu_button("⋯", |ui| {
            if ui.button("📋 Copy Path").clicked() {
                let json_path = self.path_to_jsonpath(path);
                ui.output_mut(|o| o.copied_text = json_path);
                self.state.copied_path = Some(path.to_string());
                ui.close_menu();
            }
        });
    }

    /// Convert internal path to JSONPath format.
    fn path_to_jsonpath(&self, path: &str) -> String {
        let parts: Vec<&str> = path.split('.').collect();
        let mut result = String::from("$");

        for (i, part) in parts.iter().enumerate() {
            if i == 0 && *part == "root" {
                continue; // Skip the root prefix
            }

            // Check if it's an array index
            if part.starts_with('[') && part.ends_with(']') {
                result.push_str(part);
            } else if let Ok(_idx) = part.parse::<usize>() {
                result.push_str(&format!("[{}]", part));
            } else {
                // Object key
                if part.contains(' ') || part.contains('.') || part.contains('[') {
                    result.push_str(&format!("[\"{}\"]", part));
                } else {
                    result.push('.');
                    result.push_str(part);
                }
            }
        }

        if result == "$" {
            result = "$.".to_string();
        }

        result
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Check if a file path has a supported structured format extension.
pub fn is_structured_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .and_then(StructuredFileType::from_extension)
        .is_some()
}

/// Get the structured file type from a path.
pub fn get_structured_file_type(path: &std::path::Path) -> Option<StructuredFileType> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .and_then(StructuredFileType::from_extension)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_type_detection() {
        assert_eq!(
            StructuredFileType::from_extension("json"),
            Some(StructuredFileType::Json)
        );
        assert_eq!(
            StructuredFileType::from_extension("JSON"),
            Some(StructuredFileType::Json)
        );
        assert_eq!(
            StructuredFileType::from_extension("yaml"),
            Some(StructuredFileType::Yaml)
        );
        assert_eq!(
            StructuredFileType::from_extension("yml"),
            Some(StructuredFileType::Yaml)
        );
        assert_eq!(
            StructuredFileType::from_extension("toml"),
            Some(StructuredFileType::Toml)
        );
        assert_eq!(StructuredFileType::from_extension("md"), None);
    }

    #[test]
    fn test_parse_json() {
        let json = r#"{"name": "test", "count": 42, "enabled": true}"#;
        let result = parse_json(json);
        assert!(result.is_ok());

        let tree = result.unwrap();
        match tree {
            TreeNode::Object(obj) => {
                assert_eq!(obj.len(), 3);
            }
            _ => panic!("Expected object"),
        }
    }

    #[test]
    fn test_parse_yaml() {
        let yaml = "name: test\ncount: 42\nenabled: true";
        let result = parse_yaml(yaml);
        assert!(result.is_ok());

        let tree = result.unwrap();
        match tree {
            TreeNode::Object(obj) => {
                assert_eq!(obj.len(), 3);
            }
            _ => panic!("Expected object"),
        }
    }

    #[test]
    fn test_parse_toml() {
        let toml = r#"
name = "test"
count = 42
enabled = true
"#;
        let result = parse_toml(toml);
        assert!(result.is_ok());

        let tree = result.unwrap();
        match tree {
            TreeNode::Object(obj) => {
                assert_eq!(obj.len(), 3);
            }
            _ => panic!("Expected object"),
        }
    }

    #[test]
    fn test_parse_json_error() {
        let invalid_json = r#"{"name": "test",}"#;
        let result = parse_json(invalid_json);
        assert!(result.is_err());
    }

    #[test]
    fn test_tree_node_update() {
        let mut node = TreeNode::String("hello".to_string());
        assert!(node.try_update_from_string("42"));
        match node {
            TreeNode::Integer(i) => assert_eq!(i, 42),
            _ => panic!("Expected integer"),
        }

        assert!(node.try_update_from_string("true"));
        match node {
            TreeNode::Bool(b) => assert!(b),
            _ => panic!("Expected boolean"),
        }
    }

    #[test]
    fn test_serialize_roundtrip_json() {
        let json = r#"{"name":"test","count":42}"#;
        let tree = parse_json(json).unwrap();
        let serialized = serialize_to_json(&tree).unwrap();
        let reparsed = parse_json(&serialized).unwrap();

        // Should match structure
        match (&tree, &reparsed) {
            (TreeNode::Object(a), TreeNode::Object(b)) => {
                assert_eq!(a.len(), b.len());
            }
            _ => panic!("Structures don't match"),
        }
    }

    #[test]
    fn test_is_structured_file() {
        use std::path::Path;

        assert!(is_structured_file(Path::new("config.json")));
        assert!(is_structured_file(Path::new("config.yaml")));
        assert!(is_structured_file(Path::new("config.yml")));
        assert!(is_structured_file(Path::new("config.toml")));
        assert!(!is_structured_file(Path::new("readme.md")));
        assert!(!is_structured_file(Path::new("main.rs")));
    }

    #[test]
    fn test_state_expand_collapse() {
        let mut state = TreeViewerState::new();

        assert!(state.is_expanded("root")); // Default expanded

        state.toggle_expanded("root");
        assert!(!state.is_expanded("root"));

        state.toggle_expanded("root");
        assert!(state.is_expanded("root"));
    }
}
