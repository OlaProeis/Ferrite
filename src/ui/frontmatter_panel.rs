//! Frontmatter Panel Component
//!
//! Side panel for visually editing YAML frontmatter in markdown files.
//! Displays key-value pairs as form fields with type-aware widgets
//! (text, tags/lists, booleans). Supports bidirectional sync with the
//! raw editor.

use crate::ui::phosphor_icons::{phosphor_rich_text, TRASH, WARNING, X};
use eframe::egui::{self, Color32, RichText, ScrollArea, Ui, Vec2};

const FIELD_SPACING: f32 = 8.0;

// ─────────────────────────────────────────────────────────────────────────────
// Frontmatter value types
// ─────────────────────────────────────────────────────────────────────────────

/// A single value in the frontmatter form.
#[derive(Debug, Clone, PartialEq)]
pub enum FrontmatterValue {
    String(String),
    Bool(bool),
    Number(String),
    List(Vec<String>),
    /// Nested mapping (rendered read-only as YAML text)
    Mapping(String),
    Null,
}

/// One key-value entry in the parsed frontmatter.
#[derive(Debug, Clone)]
pub struct FrontmatterField {
    pub key: String,
    pub value: FrontmatterValue,
}

// ─────────────────────────────────────────────────────────────────────────────
// Parsing helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Extract raw YAML frontmatter from markdown content.
/// Returns `Some((yaml_body, byte_end))` where `byte_end` is the byte offset
/// of the closing `---\n` (exclusive), or `None` if no frontmatter is present.
pub fn extract_frontmatter(content: &str) -> Option<(String, usize)> {
    let trimmed = content.trim_start_matches('\u{feff}'); // strip BOM
    let bom_offset = content.len() - trimmed.len();

    if !trimmed.starts_with("---") {
        return None;
    }

    let after_open = &trimmed[3..];
    if !after_open.starts_with('\n') && !after_open.starts_with("\r\n") {
        return None;
    }

    let body_start = if after_open.starts_with("\r\n") { 5 } else { 4 };
    let body = &trimmed[body_start..];

    for (i, _) in body.match_indices("\n---") {
        let after = &body[i + 4..];
        if after.is_empty() || after.starts_with('\n') || after.starts_with("\r\n") {
            let yaml_body = body[..i].to_string();
            let end = bom_offset + body_start + i + 4;
            let end = if after.starts_with("\r\n") {
                end + 2
            } else if after.starts_with('\n') {
                end + 1
            } else {
                end
            };
            return Some((yaml_body, end));
        }
    }

    None
}

/// Parse a YAML string into a list of frontmatter fields, preserving key order.
pub fn parse_frontmatter_fields(yaml: &str) -> Vec<FrontmatterField> {
    let value: serde_norway::Value = match serde_norway::from_str(yaml) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mapping = match value {
        serde_norway::Value::Mapping(m) => m,
        _ => return Vec::new(),
    };

    mapping
        .into_iter()
        .map(|(k, v)| {
            let key = match k {
                serde_norway::Value::String(s) => s,
                other => format!("{:?}", other),
            };
            FrontmatterField {
                key,
                value: yaml_to_field_value(v),
            }
        })
        .collect()
}

fn yaml_to_field_value(v: serde_norway::Value) -> FrontmatterValue {
    match v {
        serde_norway::Value::String(s) => FrontmatterValue::String(s),
        serde_norway::Value::Bool(b) => FrontmatterValue::Bool(b),
        serde_norway::Value::Number(n) => FrontmatterValue::Number(n.to_string()),
        serde_norway::Value::Null => FrontmatterValue::Null,
        serde_norway::Value::Sequence(seq) => {
            let items: Vec<String> = seq
                .into_iter()
                .map(|item| match item {
                    serde_norway::Value::String(s) => s,
                    serde_norway::Value::Number(n) => n.to_string(),
                    serde_norway::Value::Bool(b) => b.to_string(),
                    other => serde_norway::to_string(&other)
                        .unwrap_or_default()
                        .trim()
                        .to_string(),
                })
                .collect();
            FrontmatterValue::List(items)
        }
        serde_norway::Value::Mapping(_) | serde_norway::Value::Tagged(_) => {
            let text = serde_norway::to_string(&v).unwrap_or_default();
            FrontmatterValue::Mapping(text.trim().to_string())
        }
    }
}

/// Serialize frontmatter fields back to YAML text (without the `---` delimiters).
pub fn serialize_frontmatter_fields(fields: &[FrontmatterField]) -> String {
    if fields.is_empty() {
        return String::new();
    }

    let mut lines = Vec::new();
    for field in fields {
        match &field.value {
            FrontmatterValue::String(s) => {
                if s.contains('\n')
                    || s.contains(':')
                    || s.contains('#')
                    || s.starts_with(' ')
                    || s.ends_with(' ')
                    || s.contains('"')
                    || s.is_empty()
                {
                    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
                    lines.push(format!("{}: \"{}\"", field.key, escaped));
                } else {
                    lines.push(format!("{}: {}", field.key, s));
                }
            }
            FrontmatterValue::Bool(b) => {
                lines.push(format!("{}: {}", field.key, b));
            }
            FrontmatterValue::Number(n) => {
                lines.push(format!("{}: {}", field.key, n));
            }
            FrontmatterValue::Null => {
                lines.push(format!("{}:", field.key));
            }
            FrontmatterValue::List(items) => {
                if items.is_empty() {
                    lines.push(format!("{}: []", field.key));
                } else {
                    lines.push(format!("{}:", field.key));
                    for item in items {
                        if item.contains(':') || item.contains('#') || item.is_empty() {
                            let escaped = item.replace('\\', "\\\\").replace('"', "\\\"");
                            lines.push(format!("  - \"{}\"", escaped));
                        } else {
                            lines.push(format!("  - {}", item));
                        }
                    }
                }
            }
            FrontmatterValue::Mapping(raw) => {
                lines.push(format!("{}:", field.key));
                for line in raw.lines() {
                    lines.push(format!("  {}", line));
                }
            }
        }
    }
    lines.join("\n")
}

/// Replace frontmatter in content or insert new frontmatter.
/// Returns the new full content string.
pub fn replace_frontmatter_in_content(content: &str, fields: &[FrontmatterField]) -> String {
    let yaml = serialize_frontmatter_fields(fields);

    if let Some((_old_yaml, end_offset)) = extract_frontmatter(content) {
        let new_fm = format!("---\n{}\n---\n", yaml);
        format!("{}{}", new_fm, &content[end_offset..])
    } else if !yaml.is_empty() {
        format!("---\n{}\n---\n\n{}", yaml, content)
    } else {
        content.to_string()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Panel output
// ─────────────────────────────────────────────────────────────────────────────

/// Actions emitted by the frontmatter panel content.
#[derive(Debug, Clone, Default)]
pub struct FrontmatterPanelOutput {
    /// The user edited frontmatter; new full-document content to apply.
    pub new_content: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Panel widget
// ─────────────────────────────────────────────────────────────────────────────

/// Cached state for the frontmatter panel to avoid re-parsing every frame.
pub struct FrontmatterPanel {
    fields: Vec<FrontmatterField>,
    /// `(tab_id, content_version)` of the last parse.
    ///
    /// `content_version` alone is **not** a safe cache key: it's a per-tab
    /// counter that starts at 0 for every new tab, so two different tabs
    /// frequently collide on the same version (e.g. both unedited). Pairing
    /// it with the stable `Tab.id` ensures cache invalidation on tab switch
    /// while keeping per-frame cost O(1).
    cached_key: Option<(usize, u64)>,
    new_key_buf: String,
    new_tag_bufs: Vec<String>,
    has_frontmatter: bool,
    parse_error: Option<String>,
    /// The current document content, stored so `show_content` can reference it
    current_content: String,
}

impl FrontmatterPanel {
    pub fn new() -> Self {
        Self {
            fields: Vec::new(),
            cached_key: None,
            new_key_buf: String::new(),
            new_tag_bufs: Vec::new(),
            has_frontmatter: false,
            parse_error: None,
            current_content: String::new(),
        }
    }

    /// Re-parse frontmatter from content if the active tab or its content
    /// changed. Gated by `(tab_id, content_version)` — see [`cached_key`].
    ///
    /// [`cached_key`]: FrontmatterPanel::cached_key
    pub fn update_from_content_versioned(
        &mut self,
        content: &str,
        tab_id: usize,
        content_version: u64,
    ) {
        let key = (tab_id, content_version);
        if self.cached_key == Some(key) {
            return;
        }
        self.cached_key = Some(key);
        self.current_content.clear();
        self.current_content.push_str(content);

        match extract_frontmatter(content) {
            Some((yaml, _end)) => {
                let fields = parse_frontmatter_fields(&yaml);
                if fields.is_empty() && !yaml.trim().is_empty() {
                    self.parse_error = Some("Invalid YAML frontmatter".to_string());
                    self.has_frontmatter = true;
                    self.fields.clear();
                } else {
                    self.parse_error = None;
                    self.has_frontmatter = true;
                    self.fields = fields;
                }
                self.new_tag_bufs.resize(self.fields.len(), String::new());
            }
            None => {
                self.has_frontmatter = false;
                self.fields.clear();
                self.new_tag_bufs.clear();
                self.parse_error = None;
            }
        }
    }

    /// Render frontmatter content inside a parent UI (e.g. outline panel tab).
    pub fn show_content(&mut self, ui: &mut Ui, is_dark: bool) -> FrontmatterPanelOutput {
        let mut output = FrontmatterPanelOutput::default();

        let muted_color = if is_dark {
            Color32::from_rgb(140, 140, 140)
        } else {
            Color32::from_rgb(120, 120, 120)
        };
        let label_color = if is_dark {
            Color32::from_rgb(170, 170, 170)
        } else {
            Color32::from_rgb(80, 80, 80)
        };

        let content = self.current_content.clone();

        // Wrap content in a frame with inner margin so it doesn't hug the panel edge
        egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(8, 4))
            .show(ui, |ui| {
                if !self.has_frontmatter {
                    self.render_empty_state(ui, &content, muted_color, &mut output);
                } else if let Some(ref err) = self.parse_error.clone() {
                    ui.horizontal(|ui| {
                        ui.label(
                            phosphor_rich_text(WARNING, 12.0)
                                .color(Color32::from_rgb(220, 160, 60)),
                        );
                        ui.label(
                            RichText::new(err.as_str()).color(Color32::from_rgb(220, 160, 60)),
                        );
                    });
                } else {
                    self.render_fields(
                        ui,
                        &content,
                        is_dark,
                        label_color,
                        muted_color,
                        &mut output,
                    );
                }
            });

        output
    }

    fn render_empty_state(
        &mut self,
        ui: &mut Ui,
        content: &str,
        muted_color: Color32,
        output: &mut FrontmatterPanelOutput,
    ) {
        ui.add_space(16.0);
        ui.label(
            RichText::new("No frontmatter detected")
                .color(muted_color)
                .italics(),
        );
        ui.add_space(8.0);
        ui.label(
            RichText::new("Add YAML frontmatter between --- delimiters at the top of your file.")
                .color(muted_color)
                .small(),
        );
        ui.add_space(12.0);

        if ui.button("Add frontmatter").clicked() {
            let default_fields = vec![
                FrontmatterField {
                    key: "title".to_string(),
                    value: FrontmatterValue::String(String::new()),
                },
                FrontmatterField {
                    key: "date".to_string(),
                    value: FrontmatterValue::String(
                        chrono::Local::now().format("%Y-%m-%d").to_string(),
                    ),
                },
                FrontmatterField {
                    key: "tags".to_string(),
                    value: FrontmatterValue::List(Vec::new()),
                },
            ];
            let new_content = replace_frontmatter_in_content(content, &default_fields);
            output.new_content = Some(new_content);
            self.cached_key = None; // force re-parse next frame
        }
    }

    fn render_fields(
        &mut self,
        ui: &mut Ui,
        content: &str,
        is_dark: bool,
        label_color: Color32,
        muted_color: Color32,
        output: &mut FrontmatterPanelOutput,
    ) {
        let mut changed = false;
        let mut remove_idx: Option<usize> = None;

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                for (idx, field) in self.fields.iter_mut().enumerate() {
                    ui.push_id(idx, |ui| {
                        // Field label with delete button
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(&field.key)
                                    .strong()
                                    .size(12.0)
                                    .color(label_color),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .small_button(
                                            phosphor_rich_text(TRASH, 11.0).color(muted_color),
                                        )
                                        .on_hover_text("Remove field")
                                        .clicked()
                                    {
                                        remove_idx = Some(idx);
                                    }
                                },
                            );
                        });

                        // Field value widget
                        match &mut field.value {
                            FrontmatterValue::String(s) => {
                                let resp = ui.add(
                                    egui::TextEdit::singleline(s)
                                        .desired_width(ui.available_width())
                                        .hint_text("value"),
                                );
                                if resp.changed() {
                                    changed = true;
                                }
                            }
                            FrontmatterValue::Bool(b) => {
                                if ui.checkbox(b, "").changed() {
                                    changed = true;
                                }
                            }
                            FrontmatterValue::Number(n) => {
                                let resp = ui.add(
                                    egui::TextEdit::singleline(n)
                                        .desired_width(ui.available_width())
                                        .hint_text("number"),
                                );
                                if resp.changed() {
                                    changed = true;
                                }
                            }
                            FrontmatterValue::Null => {
                                ui.label(
                                    RichText::new("null").color(muted_color).italics().small(),
                                );
                            }
                            FrontmatterValue::List(items) => {
                                changed |= Self::render_tag_list(
                                    ui,
                                    items,
                                    idx,
                                    &mut self.new_tag_bufs,
                                    is_dark,
                                    muted_color,
                                );
                            }
                            FrontmatterValue::Mapping(raw) => {
                                ui.add(
                                    egui::TextEdit::multiline(raw)
                                        .desired_width(ui.available_width())
                                        .desired_rows(3)
                                        .code_editor(),
                                );
                            }
                        }

                        ui.add_space(FIELD_SPACING);
                    });
                }

                // ── Add new field ──
                ui.separator();
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.new_key_buf)
                            .desired_width(ui.available_width() - 60.0)
                            .hint_text("new key…"),
                    );
                    let enter_pressed =
                        resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if (ui.button("Add").clicked() || enter_pressed)
                        && !self.new_key_buf.trim().is_empty()
                    {
                        let key = self.new_key_buf.trim().to_string();
                        self.fields.push(FrontmatterField {
                            key,
                            value: FrontmatterValue::String(String::new()),
                        });
                        self.new_key_buf.clear();
                        self.new_tag_bufs.push(String::new());
                        changed = true;
                    }
                });
            });

        // Handle field removal
        if let Some(idx) = remove_idx {
            self.fields.remove(idx);
            if idx < self.new_tag_bufs.len() {
                self.new_tag_bufs.remove(idx);
            }
            changed = true;
        }

        // Sync edits back to content
        if changed {
            let new_content = replace_frontmatter_in_content(content, &self.fields);
            output.new_content = Some(new_content);
            self.cached_key = None; // force re-parse to keep in sync
        }
    }

    /// Render a tag/list field as pill-style chips with an inline add input.
    fn render_tag_list(
        ui: &mut Ui,
        items: &mut Vec<String>,
        field_idx: usize,
        new_tag_bufs: &mut Vec<String>,
        is_dark: bool,
        muted_color: Color32,
    ) -> bool {
        let mut changed = false;
        let mut remove_tag: Option<usize> = None;

        if new_tag_bufs.len() <= field_idx {
            new_tag_bufs.resize(field_idx + 1, String::new());
        }

        // Tag pills
        ui.horizontal_wrapped(|ui| {
            for (tag_idx, tag) in items.iter().enumerate() {
                let chip_bg = if is_dark {
                    Color32::from_rgb(55, 65, 80)
                } else {
                    Color32::from_rgb(215, 228, 240)
                };
                let chip_text = if is_dark {
                    Color32::from_rgb(180, 200, 220)
                } else {
                    Color32::from_rgb(40, 60, 80)
                };

                let frame = egui::Frame::new()
                    .fill(chip_bg)
                    .corner_radius(egui::CornerRadius::same(10))
                    .inner_margin(egui::Margin::symmetric(8, 2));

                frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                        ui.label(RichText::new(tag).size(11.5).color(chip_text));
                        if ui
                            .small_button(phosphor_rich_text(X, 11.0).color(muted_color))
                            .on_hover_text("Remove tag")
                            .clicked()
                        {
                            remove_tag = Some(tag_idx);
                        }
                    });
                });
            }
        });

        // Remove tag
        if let Some(idx) = remove_tag {
            items.remove(idx);
            changed = true;
        }

        // Add new tag
        ui.horizontal(|ui| {
            let buf = &mut new_tag_bufs[field_idx];
            let resp = ui.add(
                egui::TextEdit::singleline(buf)
                    .desired_width(ui.available_width() - 40.0)
                    .hint_text("add tag…"),
            );
            let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if (ui.small_button("+").clicked() || enter) && !buf.trim().is_empty() {
                items.push(buf.trim().to_string());
                buf.clear();
                changed = true;
            }
        });

        changed
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_basic_frontmatter() {
        let content = "---\ntitle: Hello\ndate: 2024-01-01\n---\n\n# Heading\n";
        let (yaml, end) = extract_frontmatter(content).unwrap();
        assert_eq!(yaml, "title: Hello\ndate: 2024-01-01");
        assert_eq!(&content[end..], "\n# Heading\n");
    }

    #[test]
    fn no_frontmatter() {
        let content = "# Just a heading\n\nSome text.";
        assert!(extract_frontmatter(content).is_none());
    }

    #[test]
    fn parse_string_fields() {
        let yaml = "title: My Post\nauthor: John";
        let fields = parse_frontmatter_fields(yaml);
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].key, "title");
        assert!(matches!(&fields[0].value, FrontmatterValue::String(s) if s == "My Post"));
    }

    #[test]
    fn parse_list_field() {
        let yaml = "tags:\n  - rust\n  - egui";
        let fields = parse_frontmatter_fields(yaml);
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].key, "tags");
        if let FrontmatterValue::List(items) = &fields[0].value {
            assert_eq!(items, &["rust", "egui"]);
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn parse_bool_field() {
        let yaml = "draft: true";
        let fields = parse_frontmatter_fields(yaml);
        assert_eq!(fields.len(), 1);
        assert!(matches!(&fields[0].value, FrontmatterValue::Bool(true)));
    }

    #[test]
    fn roundtrip_serialize() {
        let fields = vec![
            FrontmatterField {
                key: "title".to_string(),
                value: FrontmatterValue::String("Test".to_string()),
            },
            FrontmatterField {
                key: "tags".to_string(),
                value: FrontmatterValue::List(vec!["a".to_string(), "b".to_string()]),
            },
            FrontmatterField {
                key: "draft".to_string(),
                value: FrontmatterValue::Bool(false),
            },
        ];
        let yaml = serialize_frontmatter_fields(&fields);
        let reparsed = parse_frontmatter_fields(&yaml);
        assert_eq!(reparsed.len(), 3);
        assert_eq!(reparsed[0].key, "title");
        assert_eq!(reparsed[2].key, "draft");
    }

    #[test]
    fn replace_existing_frontmatter() {
        let content = "---\ntitle: Old\n---\n\n# Body\n";
        let fields = vec![FrontmatterField {
            key: "title".to_string(),
            value: FrontmatterValue::String("New".to_string()),
        }];
        let result = replace_frontmatter_in_content(content, &fields);
        assert!(result.starts_with("---\ntitle: New\n---\n"));
        assert!(result.contains("# Body"));
    }

    #[test]
    fn insert_frontmatter_into_plain_content() {
        let content = "# No frontmatter\n";
        let fields = vec![FrontmatterField {
            key: "title".to_string(),
            value: FrontmatterValue::String("Added".to_string()),
        }];
        let result = replace_frontmatter_in_content(content, &fields);
        assert!(result.starts_with("---\ntitle: Added\n---\n"));
        assert!(result.contains("# No frontmatter"));
    }

    /// Regression: switching between two tabs that share the same
    /// `content_version` (very common — every freshly-opened tab starts
    /// at version 0) must still re-parse. Pre-fix this caused the panel
    /// to stay stuck on the previous tab's frontmatter / stale content
    /// and could splice the previous tab's body into the active tab if
    /// the user clicked "Add frontmatter".
    #[test]
    fn cache_invalidates_on_tab_switch_with_matching_version() {
        let mut panel = FrontmatterPanel::new();

        // Tab A (id=0, version=0): no frontmatter.
        let tab_a = "# Plain README\n\nNo frontmatter here.\n";
        panel.update_from_content_versioned(tab_a, 0, 0);
        assert!(!panel.has_frontmatter);
        assert_eq!(panel.current_content, tab_a);

        // Tab B (id=1, version=0): has frontmatter. Same version as Tab A.
        let tab_b = "---\ntitle: Hello\n---\n\n# Body\n";
        panel.update_from_content_versioned(tab_b, 1, 0);
        assert!(
            panel.has_frontmatter,
            "tab switch with matching version must invalidate the cache"
        );
        assert_eq!(panel.fields.len(), 1);
        assert_eq!(panel.fields[0].key, "title");
        assert_eq!(panel.current_content, tab_b);

        // Switching back to Tab A also re-parses.
        panel.update_from_content_versioned(tab_a, 0, 0);
        assert!(!panel.has_frontmatter);
        assert_eq!(panel.current_content, tab_a);
    }

    /// Edits on the same tab still hit the cache: only version bumps
    /// (or tab switches) trigger a re-parse.
    #[test]
    fn cache_skips_reparse_when_key_unchanged() {
        let mut panel = FrontmatterPanel::new();
        let content = "---\ntitle: A\n---\n\n# Body\n";

        panel.update_from_content_versioned(content, 7, 3);
        assert!(panel.has_frontmatter);
        assert_eq!(panel.fields[0].key, "title");

        // Simulate a stale view of the same content (same key) — should
        // be a no-op even if we pass garbage as content.
        panel.update_from_content_versioned("garbage", 7, 3);
        assert_eq!(
            panel.current_content, content,
            "matching (tab_id, version) must skip re-parse and keep state"
        );
        assert_eq!(panel.fields[0].key, "title");
    }
}
