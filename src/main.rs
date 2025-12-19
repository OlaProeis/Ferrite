// Hide console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Ferrite - Main Entry Point
//!
//! A fast, lightweight text editor for Markdown, JSON, and more. Built with Rust and egui.

mod app;
mod config;
mod editor;
mod error;
mod export;
mod files;
mod fonts;
mod markdown;
mod preview;
mod state;
mod theme;
mod ui;
mod workspaces;

use app::FerriteApp;
use config::load_config;
use log::info;
use ui::get_app_icon;

// Note: Native window decorations are disabled for custom title bar styling.
// This provides consistent appearance across all platforms (Windows, macOS, Linux).

/// Application name constant.
const APP_NAME: &str = "Ferrite";

fn main() -> eframe::Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("Starting {}", APP_NAME);

    // Load settings to get window configuration
    let settings = load_config();
    let window_size = &settings.window_size;

    info!(
        "Window configuration: {}x{}, maximized: {}",
        window_size.width, window_size.height, window_size.maximized
    );

    // Load application icon
    let app_icon = get_app_icon();
    if app_icon.is_some() {
        info!("Application icon loaded successfully");
    }

    // Configure the native window options with custom title bar (no native decorations)
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title(APP_NAME)
        .with_decorations(false) // Custom title bar - no native window decorations
        .with_inner_size([window_size.width, window_size.height])
        .with_min_inner_size([400.0, 300.0]);

    // Set application icon if available
    if let Some(icon) = app_icon {
        viewport = viewport.with_icon(icon);
    }

    // Apply position if saved
    let viewport = if let (Some(x), Some(y)) = (window_size.x, window_size.y) {
        viewport.with_position([x, y])
    } else {
        viewport
    };

    // Apply maximized state
    let viewport = if window_size.maximized {
        viewport.with_maximized(true)
    } else {
        viewport
    };

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    // Run the application
    eframe::run_native(
        APP_NAME,
        native_options,
        Box::new(|cc| {
            // Configure egui visuals based on theme (basic setup)
            // Full theme support will be implemented in a later task
            Ok(Box::new(FerriteApp::new(cc)))
        }),
    )
}
