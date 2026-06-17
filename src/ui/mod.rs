//! UI components for Ferrite
//!
//! This module contains reusable UI widgets and components.
//!
mod about;
mod action_registry;
mod backlinks_panel;
mod command_palette;
mod dialogs;
mod docked_sidebar;
mod file_index_progress;
mod file_tree;
pub mod format_toolbar;
mod frontmatter_panel;
mod icons;
mod mermaid_popup;
mod nav_buttons;
mod outline_panel;
pub mod phosphor_icons;
mod pipeline;
mod productivity_panel;
mod quick_switcher;
mod ribbon;
mod runtime_modules;
mod search;
mod settings;
mod terminal_panel;
mod view_segment;
mod welcome;
mod window;

pub use about::AboutPanel;
pub use action_registry::{
    render_action_menu_with_shortcuts, ActionContext, ActionRegistry, ContextActionId,
};
pub use backlinks_panel::BacklinksPanel;
pub use command_palette::CommandPalette;
pub use dialogs::{FileOperationDialog, FileOperationResult, GoToLineDialog, GoToLineResult};
pub use file_index_progress::file_index_progress_ui;
pub use file_tree::{FileTreeContextAction, FileTreePanel};
pub use format_toolbar::{side_panel_toggle_strip, FormatToolbar};
pub use frontmatter_panel::{
    parse_frontmatter_fields, FrontmatterField, FrontmatterPanel, FrontmatterValue,
};
pub use icons::{get_app_icon, load_app_logo_texture};
pub use mermaid_popup::MermaidPopupState;
pub use nav_buttons::{
    render_markdown_cheatsheet, render_nav_buttons, set_overlay_blocks_nav_buttons, NavAction,
};
pub use outline_panel::{OutlinePanel, OutlinePanelTab};
pub use pipeline::{PipelinePanel, TabPipelineState};
pub use productivity_panel::ProductivityPanel;
pub use quick_switcher::QuickSwitcher;
pub use ribbon::{markdown_cheatsheet_trigger_rect, Ribbon, RibbonAction};
pub use runtime_modules::RuntimeModulesInfo;
pub use search::{SearchNavigationTarget, SearchPanel};
pub use settings::SettingsPanel;
pub use terminal_panel::{FloatingWindow, TerminalPanel, TerminalPanelState};
pub use view_segment::{TitleBarButton, ViewModeSegment, ViewSegmentAction};
pub use welcome::WelcomePanel;
pub use window::{
    apply_window_chrome, center_panel_in_viewport, constrain_rect_to_viewport,
    consume_clicks_in_resize_zones, handle_window_resize, search_panel_constraints,
    PanelConstraints, WindowResizeState,
};
