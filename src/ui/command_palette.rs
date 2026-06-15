//! Command palette overlay for quick command execution.
//!
//! Provides an Alt+Space searchable command launcher that shows recent
//! commands first, with fuzzy search across all available actions.

#![allow(clippy::collapsible_if)]

use crate::app::commands::{all_palette_commands, PaletteCommand};
use crate::config::{KeyboardShortcuts, ShortcutCommand};
use crate::ui::icons::phosphor_rich_text;
use eframe::egui::{self, Color32, Key, LayerId, Order, Pos2, RichText, Sense};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::collections::VecDeque;

const MAX_RESULTS: usize = 15;
const MAX_RECENT: usize = 20;

/// Output from the command palette for one frame.
#[derive(Debug, Default)]
pub struct CommandPaletteOutput {
    /// Command selected by the user to execute.
    pub selected_command: Option<ShortcutCommand>,
    /// Whether the palette was closed this frame.
    pub closed: bool,
    /// Screen rect of the overlay panel (for native WebView occlusion).
    pub screen_rect: Option<egui::Rect>,
}

/// Persistent state for the command palette.
pub struct CommandPalette {
    is_open: bool,
    query: String,
    selected_index: usize,
    matcher: SkimMatcherV2,
    /// Most recently executed commands (front = most recent).
    recent_commands: VecDeque<ShortcutCommand>,
    /// Cached full command list (built once).
    all_commands: Vec<PaletteCommand>,
    /// Last known mouse position — hover only updates selection when mouse moves.
    last_mouse_pos: Option<Pos2>,
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandPalette {
    pub fn new() -> Self {
        Self {
            is_open: false,
            query: String::new(),
            selected_index: 0,
            matcher: SkimMatcherV2::default(),
            recent_commands: VecDeque::with_capacity(MAX_RECENT),
            all_commands: all_palette_commands(),
            last_mouse_pos: None,
        }
    }

    pub fn is_open(&self) -> bool {
        self.is_open
    }

    pub fn open(&mut self) {
        self.is_open = true;
        self.query.clear();
        self.selected_index = 0;
        self.last_mouse_pos = None;
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.query.clear();
        self.selected_index = 0;
    }

    pub fn toggle(&mut self) {
        if self.is_open {
            self.close();
        } else {
            self.open();
        }
    }

    /// Record a command as recently used (moves it to the front).
    pub fn record_recent(&mut self, cmd: ShortcutCommand) {
        self.recent_commands.retain(|c| *c != cmd);
        self.recent_commands.push_front(cmd);
        if self.recent_commands.len() > MAX_RECENT {
            self.recent_commands.pop_back();
        }
    }

    /// Get recent commands list for persistence.
    pub fn recent_commands(&self) -> &VecDeque<ShortcutCommand> {
        &self.recent_commands
    }

    /// Restore recent commands from persisted data.
    pub fn set_recent_commands(&mut self, recent: VecDeque<ShortcutCommand>) {
        self.recent_commands = recent;
    }

    /// Render the command palette and return output for this frame.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        shortcuts: &KeyboardShortcuts,
        is_dark: bool,
    ) -> CommandPaletteOutput {
        let mut output = CommandPaletteOutput::default();

        if !self.is_open {
            return output;
        }

        let results = self.filter_commands();

        // Theme colors (mirrors QuickSwitcher)
        let bg_color = if is_dark {
            Color32::from_rgb(35, 35, 40)
        } else {
            Color32::from_rgb(255, 255, 255)
        };
        let border_color = if is_dark {
            Color32::from_rgb(80, 80, 90)
        } else {
            Color32::from_rgb(180, 180, 190)
        };
        let text_color = if is_dark {
            Color32::from_rgb(220, 220, 220)
        } else {
            Color32::from_rgb(40, 40, 40)
        };
        let secondary_color = if is_dark {
            Color32::from_rgb(140, 140, 150)
        } else {
            Color32::from_rgb(100, 100, 110)
        };
        let selected_bg = if is_dark {
            Color32::from_rgb(55, 65, 85)
        } else {
            Color32::from_rgb(220, 230, 245)
        };
        let hover_bg = if is_dark {
            Color32::from_rgb(45, 50, 60)
        } else {
            Color32::from_rgb(235, 240, 248)
        };
        let shortcut_bg = if is_dark {
            Color32::from_rgb(50, 50, 58)
        } else {
            Color32::from_rgb(230, 232, 238)
        };
        let category_color = if is_dark {
            Color32::from_rgb(100, 140, 200)
        } else {
            Color32::from_rgb(60, 100, 160)
        };

        // Detect actual mouse movement (not just sitting still over an item)
        let mouse_moved = ctx.input(|i| {
            let pos = i.pointer.hover_pos();
            let moved = pos != self.last_mouse_pos && pos.is_some();
            if pos.is_some() {
                self.last_mouse_pos = pos;
            }
            moved
        });

        // Keyboard handling
        ctx.input(|i| {
            if i.key_pressed(Key::Escape) {
                output.closed = true;
            }
            if i.key_pressed(Key::ArrowDown) && !results.is_empty() {
                self.selected_index = (self.selected_index + 1) % results.len();
            }
            if i.key_pressed(Key::ArrowUp) && !results.is_empty() {
                self.selected_index = if self.selected_index == 0 {
                    results.len() - 1
                } else {
                    self.selected_index - 1
                };
            }
            if i.key_pressed(Key::Enter) {
                if let Some(result) = results.get(self.selected_index) {
                    output.selected_command = Some(result.command.command);
                    output.closed = true;
                }
            }
        });

        // Overlay
        let area_response = egui::Area::new(egui::Id::new("command_palette_overlay"))
            .anchor(egui::Align2::CENTER_TOP, [0.0, 100.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(bg_color)
                    .stroke(egui::Stroke::new(1.0, border_color))
                    .corner_radius(8)
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 4],
                        blur: 12,
                        spread: 0,
                        color: Color32::from_black_alpha(60),
                    })
                    .show(ui, |ui| {
                        ui.set_width(520.0);
                        ui.add_space(8.0);

                        // Search input
                        ui.horizontal(|ui| {
                            ui.add_space(12.0);
                            ui.label(RichText::new(">").size(16.0).strong().color(category_color));
                            ui.add_space(4.0);

                            let response = ui.add(
                                egui::TextEdit::singleline(&mut self.query)
                                    .hint_text("Type a command...")
                                    .frame(egui::Frame::NONE)
                                    .desired_width(470.0)
                                    .font(egui::TextStyle::Body),
                            );
                            response.request_focus();

                            if response.changed() {
                                self.selected_index = 0;
                            }
                            ui.add_space(8.0);
                        });

                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);

                        // Results
                        if results.is_empty() {
                            ui.horizontal(|ui| {
                                ui.add_space(16.0);
                                ui.label(
                                    RichText::new("No matching commands")
                                        .color(secondary_color)
                                        .italics(),
                                );
                            });
                            ui.add_space(8.0);
                        } else {
                            let mut prev_category: Option<&str> = None;

                            for (idx, result) in results.iter().enumerate() {
                                let is_selected = idx == self.selected_index;

                                // Category header (only when query is empty, grouped view)
                                if self.query.is_empty() {
                                    let cat = result.command.category();
                                    if prev_category != Some(cat) {
                                        if prev_category.is_some() {
                                            ui.add_space(4.0);
                                        }
                                        ui.horizontal(|ui| {
                                            ui.add_space(12.0);
                                            ui.label(
                                                RichText::new(cat)
                                                    .color(category_color)
                                                    .small()
                                                    .strong(),
                                            );
                                        });
                                        ui.add_space(2.0);
                                        prev_category = Some(cat);
                                    }
                                }

                                // Row content
                                let row_response = ui
                                    .horizontal(|ui| {
                                        ui.add_space(16.0);

                                        // Icon
                                        ui.label(phosphor_rich_text(result.command.icon, 14.0));
                                        ui.add_space(8.0);

                                        // Command name
                                        ui.label(
                                            RichText::new(result.command.label())
                                                .color(text_color)
                                                .strong(),
                                        );

                                        // Recent indicator
                                        if result.is_recent && !self.query.is_empty() {
                                            ui.add_space(4.0);
                                            ui.label(
                                                RichText::new("recent")
                                                    .color(secondary_color)
                                                    .small()
                                                    .italics(),
                                            );
                                        }

                                        // Shortcut hint (right-aligned, only if meaningful)
                                        let binding = shortcuts.get(result.command.command);
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.add_space(16.0);
                                                if binding.has_modifiers() {
                                                    let shortcut_str = binding.display_string();
                                                    egui::Frame::new()
                                                        .fill(shortcut_bg)
                                                        .corner_radius(3)
                                                        .inner_margin(egui::Margin::symmetric(6, 2))
                                                        .show(ui, |ui| {
                                                            ui.label(
                                                                RichText::new(shortcut_str)
                                                                    .color(secondary_color)
                                                                    .small()
                                                                    .family(
                                                                        egui::FontFamily::Monospace,
                                                                    ),
                                                            );
                                                        });
                                                }
                                            },
                                        );
                                    })
                                    .response;

                                // Interaction overlay
                                let row_rect = row_response.rect.expand2(egui::vec2(8.0, 2.0));
                                let response = ui.interact(
                                    row_rect,
                                    ui.id().with(("palette_row", idx)),
                                    Sense::click(),
                                );

                                if response.hovered() && mouse_moved {
                                    self.selected_index = idx;
                                }

                                let show_highlight = is_selected || response.hovered();
                                if show_highlight {
                                    let bg_layer = LayerId::new(
                                        Order::Background,
                                        ui.id().with(("palette_bg", idx)),
                                    );
                                    ui.ctx().layer_painter(bg_layer).rect_filled(
                                        row_rect,
                                        4.0,
                                        if is_selected { selected_bg } else { hover_bg },
                                    );
                                }

                                if response.clicked() {
                                    output.selected_command = Some(result.command.command);
                                    output.closed = true;
                                }

                                ui.add_space(2.0);
                            }
                            ui.add_space(4.0);
                        }

                        // Footer hints
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.add_space(12.0);
                            ui.label(
                                RichText::new("↑↓ Navigate  ⏎ Execute  Esc Close")
                                    .color(secondary_color)
                                    .small(),
                            );
                        });
                        ui.add_space(6.0);
                    });
            });
        let area_id = egui::Id::new("command_palette_overlay");
        output.screen_rect = ctx.memory(|m| m.area_rect(area_id)).or_else(|| {
            Some(crate::markdown::video_render::egui_rect_to_screen(
                ctx,
                area_response.response.layer_id,
                area_response.response.rect,
            ))
        });
        if let Some(rect) = output.screen_rect {
            output.screen_rect = Some(
                rect.expand(crate::markdown::video_render::VIDEO_OCCLUDER_MARGIN),
            );
        }

        if output.closed {
            self.close();
        }

        output
    }

    /// Filter and score commands based on the query.
    fn filter_commands(&self) -> Vec<PaletteResult> {
        if self.query.is_empty() {
            return self.default_results();
        }

        let mut scored: Vec<(usize, i64)> = Vec::new();

        for (idx, cmd) in self.all_commands.iter().enumerate() {
            let search_text = format!("{} {}", cmd.category(), cmd.label());
            if let Some(score) = self.matcher.fuzzy_match(&search_text, &self.query) {
                let is_recent = self.recent_commands.contains(&cmd.command);
                let boosted = if is_recent { score + 100 } else { score };
                scored.push((idx, boosted));
            }
        }

        scored.sort_by(|a, b| b.1.cmp(&a.1));

        scored
            .into_iter()
            .take(MAX_RESULTS)
            .map(|(idx, _score)| {
                let cmd = &self.all_commands[idx];
                PaletteResult {
                    command: cmd.clone(),
                    is_recent: self.recent_commands.contains(&cmd.command),
                }
            })
            .collect()
    }

    /// Default results when query is empty: recent commands first, then by category.
    fn default_results(&self) -> Vec<PaletteResult> {
        let mut results: Vec<PaletteResult> = Vec::new();

        // Recent commands first
        for recent_cmd in &self.recent_commands {
            if let Some(cmd) = self.all_commands.iter().find(|c| c.command == *recent_cmd) {
                results.push(PaletteResult {
                    command: cmd.clone(),
                    is_recent: true,
                });
                if results.len() >= MAX_RESULTS {
                    return results;
                }
            }
        }

        // Fill remaining with common commands (not already listed)
        let common = [
            ShortcutCommand::Save,
            ShortcutCommand::SaveAs,
            ShortcutCommand::Open,
            ShortcutCommand::New,
            ShortcutCommand::Find,
            ShortcutCommand::FindReplace,
            ShortcutCommand::ToggleViewMode,
            ShortcutCommand::OpenSettings,
            ShortcutCommand::ToggleTerminal,
            ShortcutCommand::ExportHtml,
            ShortcutCommand::ExportPdf,
            ShortcutCommand::PrintPreview,
            ShortcutCommand::ToggleZenMode,
            ShortcutCommand::CycleTheme,
            ShortcutCommand::GoToLine,
            ShortcutCommand::ToggleOutline,
            ShortcutCommand::SearchInFiles,
        ];

        for sc in common {
            if results.len() >= MAX_RESULTS {
                break;
            }
            if results.iter().any(|r| r.command.command == sc) {
                continue;
            }
            if let Some(cmd) = self.all_commands.iter().find(|c| c.command == sc) {
                results.push(PaletteResult {
                    command: cmd.clone(),
                    is_recent: false,
                });
            }
        }

        results
    }
}

/// A single result row in the command palette.
struct PaletteResult {
    command: PaletteCommand,
    is_recent: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_palette_new() {
        let palette = CommandPalette::new();
        assert!(!palette.is_open());
        assert!(!palette.all_commands.is_empty());
    }

    #[test]
    fn test_command_palette_toggle() {
        let mut palette = CommandPalette::new();
        assert!(!palette.is_open());
        palette.toggle();
        assert!(palette.is_open());
        palette.toggle();
        assert!(!palette.is_open());
    }

    #[test]
    fn test_record_recent() {
        let mut palette = CommandPalette::new();
        palette.record_recent(ShortcutCommand::Save);
        palette.record_recent(ShortcutCommand::Open);
        palette.record_recent(ShortcutCommand::Save);

        assert_eq!(palette.recent_commands.len(), 2);
        assert_eq!(palette.recent_commands[0], ShortcutCommand::Save);
        assert_eq!(palette.recent_commands[1], ShortcutCommand::Open);
    }

    #[test]
    fn test_filter_empty_query() {
        let palette = CommandPalette::new();
        let results = palette.filter_commands();
        assert!(!results.is_empty());
        assert!(results.len() <= MAX_RESULTS);
    }
}
