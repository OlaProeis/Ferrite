//! Platform-specific functionality

pub mod primary_selection;

#[allow(unused_imports)] // `set_primary` / decision are Linux-only call sites
pub use primary_selection::{get_primary, primary_publish_decision, set_primary};

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "macos")]
pub use macos::{bind_egui_context, get_open_file_paths, take_opened_files};

/// Non-macOS stub: no Apple Events / Finder Open With path reception.
#[cfg(not(target_os = "macos"))]
pub fn get_open_file_paths() -> Vec<std::path::PathBuf> {
    Vec::new()
}

/// Non-macOS stub mirroring [`get_open_file_paths`].
#[cfg(not(target_os = "macos"))]
pub fn take_opened_files() -> Vec<std::path::PathBuf> {
    Vec::new()
}

/// Non-macOS stub: no Open With egui wake / AE handler to install.
#[cfg(not(target_os = "macos"))]
pub fn bind_egui_context(_ctx: egui::Context) {}

#[cfg(target_os = "windows")]
pub fn allow_set_foreground_window(process_id: u32) -> bool {
    extern "system" {
        fn AllowSetForegroundWindow(dw_process_id: u32) -> i32;
    }

    unsafe { AllowSetForegroundWindow(process_id) != 0 }
}

#[cfg(not(target_os = "windows"))]
pub fn allow_set_foreground_window(_process_id: u32) -> bool {
    false
}
