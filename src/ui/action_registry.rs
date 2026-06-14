//! Shared metadata for small context menus.
//!
//! App code owns side effects; this module only renders action rows and returns
//! the clicked action id.

use crate::config::{KeyboardShortcuts, ShortcutCommand};
use eframe::egui::{self, RichText, Ui};
use rust_i18n::t;

/// Stable ids for context-menu actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContextActionId {
    NewTab,
    CloseTab,
    CopyPath,
    RevealInExplorer,
}

/// The surface requesting actions.
#[derive(Debug, Clone, Copy)]
pub enum ActionContext {
    Tab { has_file_path: bool },
}

/// Context-menu action metadata.
#[derive(Debug, Clone)]
pub struct ActionDefinition {
    pub id: ContextActionId,
    pub display_name: String,
    pub group: u8,
    pub shortcut_command: Option<ShortcutCommand>,
}

/// Registry for context-menu actions.
pub struct ActionRegistry;

impl ActionRegistry {
    pub fn actions_for(context: ActionContext) -> Vec<ActionDefinition> {
        match context {
            ActionContext::Tab { has_file_path } => {
                let mut actions = vec![
                    ActionDefinition::new(
                        ContextActionId::NewTab,
                        localized_label(t!("tab.new_tab").to_string(), "New Tab"),
                        0,
                        Some(ShortcutCommand::NewTab),
                    ),
                    ActionDefinition::new(
                        ContextActionId::CloseTab,
                        localized_label(t!("tab.close").to_string(), "Close Tab"),
                        0,
                        Some(ShortcutCommand::CloseTab),
                    ),
                ];

                if has_file_path {
                    actions.extend([
                        ActionDefinition::new(ContextActionId::CopyPath, "Copy File Path", 1, None),
                        ActionDefinition::new(
                            ContextActionId::RevealInExplorer,
                            localized_label(
                                t!("tab.reveal_in_explorer").to_string(),
                                "Reveal in Explorer",
                            ),
                            1,
                            None,
                        ),
                    ]);
                }

                actions
            }
        }
    }
}

impl ActionDefinition {
    fn new(
        id: ContextActionId,
        display_name: impl Into<String>,
        group: u8,
        shortcut_command: Option<ShortcutCommand>,
    ) -> Self {
        Self {
            id,
            display_name: display_name.into(),
            group,
            shortcut_command,
        }
    }
}

/// Render a context menu with optional right-aligned shortcut hints.
pub fn render_action_menu_with_shortcuts(
    ui: &mut Ui,
    actions: &[ActionDefinition],
    shortcuts: Option<&KeyboardShortcuts>,
) -> Option<ContextActionId> {
    let mut clicked = None;
    let mut last_group = None;

    for action in actions {
        if last_group.is_some() && last_group != Some(action.group) {
            ui.separator();
        }
        last_group = Some(action.group);

        let shortcut_label = shortcuts
            .and_then(|shortcuts| action.shortcut_command.map(|cmd| shortcuts.get(cmd)))
            .filter(|binding| binding.has_modifiers())
            .map(|binding| binding.display_string());

        let row = ui
            .horizontal(|ui| {
                ui.set_min_width(210.0);
                ui.label(&action.display_name);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(shortcut_label) = &shortcut_label {
                        ui.label(
                            RichText::new(shortcut_label)
                                .small()
                                .monospace()
                                .color(ui.visuals().weak_text_color()),
                        );
                    }
                });
            })
            .response;

        let response = ui.interact(
            row.rect,
            ui.id().with(("context_action", action.id)),
            egui::Sense::click(),
        );

        if response.clicked() {
            clicked = Some(action.id);
            ui.close();
            break;
        }
    }

    clicked
}

fn localized_label(label: String, fallback: &'static str) -> String {
    if label.trim().is_empty() {
        fallback.to_string()
    } else {
        label
    }
}
