//! UI components for Ferrite
//!
//! This module contains reusable UI widgets and components.
//!
mod welcome;
mod about;
mod backlinks_panel;
mod dialogs;
mod file_tree;
mod icons;
mod nav_buttons;
mod outline_panel;
mod pipeline;
mod productivity_panel;
mod quick_switcher;
mod ribbon;
mod search;
mod settings;
mod terminal_panel;
mod view_segment;
mod window;

pub use welcome::WelcomePanel;
pub use about::AboutPanel;
pub use backlinks_panel::{ BacklinksPanel, BacklinksPanelOutput };
pub use dialogs::{ FileOperationDialog, FileOperationResult, GoToLineDialog, GoToLineResult };
pub use file_tree::{ FileTreeContextAction, FileTreePanel };
pub use icons::{ get_app_icon, load_app_logo_texture };
pub use nav_buttons::{ render_nav_buttons, NavAction };
pub use outline_panel::{ OutlinePanel, OutlinePanelTab };
pub use pipeline::{ PipelinePanel, TabPipelineState };
pub use productivity_panel::{ AutoSave, PomodoroTimer, ProductivityPanel, Task };
pub use quick_switcher::QuickSwitcher;
pub use ribbon::{ Ribbon, RibbonAction };
pub use search::{ SearchNavigationTarget, SearchPanel };
pub use settings::SettingsPanel;
pub use terminal_panel::{ TerminalPanel, TerminalPanelState, FloatingWindow };
pub use view_segment::{ TitleBarButton, ViewModeSegment, ViewSegmentAction };
pub use window::{
    center_panel_in_viewport,
    constrain_rect_to_viewport,
    handle_window_resize,
    search_panel_constraints,
    PanelConstraints,
    WindowResizeState,
};
