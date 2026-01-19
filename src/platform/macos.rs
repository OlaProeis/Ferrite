//! macOS-specific functionality for handling Apple Events (Open With, file associations)
//!
//! When a user opens a file with Ferrite via Finder's "Open With" menu or by double-clicking
//! a file associated with the app, macOS sends an Apple Event rather than passing the file
//! path as a command-line argument.
//!
//! ## Current Limitations
//!
//! Full "Open With" support is not yet implemented because:
//! - eframe/winit controls the NSApplication event loop
//! - Apple Events for file opening require access to delegate methods
//! - eframe does not currently expose winit's `Event::Opened` events
//!
//! The app bundle includes file type associations (CFBundleDocumentTypes), so Ferrite
//! appears in the "Open With" menu. However, files opened this way won't actually open
//! until eframe adds support.
//!
//! Workaround: Use the command line: `ferrite /path/to/file.md`
//!
//! See: https://github.com/rust-windowing/winit/issues/1751

use std::path::PathBuf;

/// Retrieves file paths that were passed to the app via macOS "Open With" functionality.
///
/// Currently returns an empty Vec due to eframe limitations.
/// Files should be opened via command line arguments instead.
pub fn get_open_file_paths() -> Vec<PathBuf> {
    Vec::new()
}

/// Placeholder for initializing macOS-specific functionality.
/// Currently a no-op due to eframe/winit event loop conflicts.
pub fn init_app_delegate() {
    // No-op: Setting an NSApplicationDelegate before eframe starts
    // causes crashes in the event loop initialization.
    //
    // This function exists to maintain API compatibility and can be
    // implemented once eframe exposes file open events.
}
