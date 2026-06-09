//! Platform-specific functionality

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "macos")]
pub use macos::get_open_file_paths;

#[cfg(not(target_os = "macos"))]
pub fn get_open_file_paths() -> Vec<std::path::PathBuf> {
    Vec::new()
}

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
