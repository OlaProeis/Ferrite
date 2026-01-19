//! Platform-specific functionality

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "macos")]
pub use macos::get_open_file_paths;

#[cfg(not(target_os = "macos"))]
pub fn get_open_file_paths() -> Vec<std::path::PathBuf> {
    Vec::new()
}
