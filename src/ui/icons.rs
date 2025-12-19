//! Icon loading utilities for Ferrite
//!
//! This module provides helper functions to load PNG icons and convert them
//! to `egui::IconData` for use as window icons.

use eframe::egui;
use image::GenericImageView;
use std::sync::Arc;

/// Default icon PNG embedded at compile time (256x256 for good quality)
/// Falls back gracefully if the icon file doesn't exist during development.
#[cfg(feature = "bundle-icon")]
const EMBEDDED_ICON: &[u8] = include_bytes!("../../assets/icons/icon_256.png");

/// Load icon data from PNG bytes.
///
/// Converts PNG image data into `egui::IconData` suitable for window icons.
/// The PNG should be a square image (recommended sizes: 64, 128, 256 pixels).
///
/// # Arguments
///
/// * `png_data` - Raw PNG file bytes
///
/// # Returns
///
/// `Some(IconData)` on success, `None` if the PNG couldn't be decoded.
///
/// # Example
///
/// ```rust,ignore
/// let icon_bytes = include_bytes!("../assets/icons/icon_256.png");
/// if let Some(icon) = load_icon_from_png(icon_bytes) {
///     native_options.viewport = native_options.viewport.with_icon(icon);
/// }
/// ```
pub fn load_icon_from_png(png_data: &[u8]) -> Option<egui::IconData> {
    let image = image::load_from_memory(png_data).ok()?;
    let rgba = image.to_rgba8();
    let (width, height) = image.dimensions();

    Some(egui::IconData {
        rgba: rgba.into_raw(),
        width,
        height,
    })
}

/// Load icon from a file path.
///
/// Attempts to load a PNG icon from the filesystem. Useful for development
/// when icons aren't embedded yet.
///
/// # Arguments
///
/// * `path` - Path to the PNG file
///
/// # Returns
///
/// `Some(IconData)` on success, `None` if the file couldn't be read or decoded.
#[allow(dead_code)]
pub fn load_icon_from_file(path: &std::path::Path) -> Option<egui::IconData> {
    let data = std::fs::read(path).ok()?;
    load_icon_from_png(&data)
}

/// Get the application icon for use in native window options.
///
/// This function attempts to load the application icon in the following order:
/// 1. Embedded icon (if `bundle-icon` feature is enabled)
/// 2. Icon from assets directory (development fallback)
/// 3. None (graceful degradation)
///
/// # Returns
///
/// An `Arc<egui::IconData>` if an icon could be loaded, otherwise `None`.
pub fn get_app_icon() -> Option<Arc<egui::IconData>> {
    // Try embedded icon first (release builds with bundle-icon feature)
    #[cfg(feature = "bundle-icon")]
    if let Some(icon) = load_icon_from_png(EMBEDDED_ICON) {
        log::info!("Loaded embedded application icon");
        return Some(Arc::new(icon));
    }

    // Development fallback: try loading from assets directory
    let icon_paths = [
        "assets/icons/icon_256.png",
        "assets/icons/icon_128.png",
        "assets/icons/icon_64.png",
    ];

    for path in &icon_paths {
        let path = std::path::Path::new(path);
        if path.exists() {
            if let Some(icon) = load_icon_from_file(path) {
                log::info!("Loaded application icon from: {}", path.display());
                return Some(Arc::new(icon));
            }
        }
    }

    log::debug!("No application icon found, using default");
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_icon_from_png_invalid_data() {
        let invalid_data = b"not a png file";
        assert!(load_icon_from_png(invalid_data).is_none());
    }

    #[test]
    fn test_load_icon_from_file_nonexistent() {
        let path = std::path::Path::new("nonexistent_icon.png");
        assert!(load_icon_from_file(path).is_none());
    }
}
