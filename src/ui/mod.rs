//! UI components for Ferrite
//!
//! This module contains reusable UI widgets and components.

mod about;
mod dialogs;
mod file_tree;
mod icons;
mod outline_panel;
mod quick_switcher;
mod ribbon;
mod search;
mod settings;
mod window;

pub use about::AboutPanel;
pub use dialogs::{FileOperationDialog, FileOperationResult};
pub use file_tree::{FileTreeContextAction, FileTreePanel};
pub use icons::get_app_icon;
pub use outline_panel::OutlinePanel;
pub use quick_switcher::QuickSwitcher;
pub use ribbon::{Ribbon, RibbonAction};
pub use search::SearchPanel;
pub use settings::SettingsPanel;
pub use window::{handle_window_resize, WindowResizeState};
