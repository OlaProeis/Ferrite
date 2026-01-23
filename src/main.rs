// Hide console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Ferrite - Main Entry Point
//!
//! A fast, lightweight text editor for Markdown, JSON, and more. Built with Rust and egui.

// ============================================================================
// Global Memory Allocator Configuration
// ============================================================================
// Use high-performance allocators to reduce heap fragmentation and memory usage.
// - Windows: mimalloc - Microsoft's compact, fast allocator
// - Unix (Linux/macOS): jemalloc - battle-tested allocator from Facebook/Meta
//
// These allocators are enabled by the "high-perf-alloc" feature (default on).
// Disable with: cargo build --no-default-features --features bundle-icon
// ============================================================================

#[cfg(all(feature = "high-perf-alloc", target_os = "windows"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(all(feature = "high-perf-alloc", unix))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

// Initialize internationalization system
// The locales directory contains YAML files for each supported language (e.g., en.yaml, zh.yaml)
// Usage: rust_i18n::t!("menu.file.open") returns the translated string
rust_i18n::i18n!("locales");

mod app;
mod config;
mod editor;
mod error;
mod export;
mod files;
mod fonts;
mod markdown;
mod path_utils;
mod platform;
mod preview;
mod state;
mod string_utils;
mod theme;
mod ui;
mod vcs;
mod workspaces;

use app::FerriteApp;
use clap::Parser;
use config::{load_config, LogLevel};
use log::info;
use rust_i18n::{set_locale, t};
use std::path::PathBuf;
use ui::get_app_icon;

/// Ferrite - A fast, lightweight text editor for Markdown, JSON, and more.
#[derive(Parser, Debug)]
#[command(name = "ferrite", version, about, long_about = None)]
struct Cli {
    /// Files or directory to open on startup.
    ///
    /// Pass one or more file paths to open them as tabs.
    /// Pass a directory path to open it as a workspace.
    #[arg(value_name = "PATH")]
    paths: Vec<PathBuf>,

    /// Set the log level for debugging.
    ///
    /// Overrides the log_level setting in config.json.
    /// Valid values: debug, info, warn, error, off
    #[arg(long, value_name = "LEVEL", value_parser = parse_log_level)]
    log_level: Option<LogLevel>,
}

/// Parse a log level string into a LogLevel enum.
fn parse_log_level(s: &str) -> Result<LogLevel, String> {
    match s.to_lowercase().as_str() {
        "debug" => Ok(LogLevel::Debug),
        "info" => Ok(LogLevel::Info),
        "warn" | "warning" => Ok(LogLevel::Warn),
        "error" => Ok(LogLevel::Error),
        "off" | "none" => Ok(LogLevel::Off),
        _ => Err(format!(
            "Invalid log level '{}'. Valid values: debug, info, warn, error, off",
            s
        )),
    }
}

// Note: Native window decorations are disabled for custom title bar styling.
// This provides consistent appearance across all platforms (Windows, macOS, Linux).

/// Application name constant.
const APP_NAME: &str = "Ferrite";

fn main() -> eframe::Result<()> {
    // Initialize macOS app delegate FIRST, before anything else
    // This must happen very early to catch Apple Events for "Open With" functionality
    #[cfg(target_os = "macos")]
    platform::macos::init_app_delegate();

    // Parse CLI arguments first (before logging, so --help/--version work without config)
    let cli = Cli::parse();

    // Load settings to get configuration (including log level and language)
    let settings = load_config();

    // Apply saved language setting for i18n
    set_locale(settings.language.locale_code());

    // Determine effective log level: CLI > config > default (Warn)
    let effective_log_level = cli.log_level.unwrap_or(settings.log_level);

    // Initialize logging with the effective log level
    env_logger::Builder::new()
        .filter_level(effective_log_level.to_level_filter())
        .init();

    info!("Starting {}", APP_NAME);
    info!(
        "Language: {} ({})",
        settings.language.native_name(),
        settings.language.locale_code()
    );
    info!("i18n initialized: {}", t!("app.name")); // Test i18n system
    info!(
        "Log level: {} (source: {})",
        effective_log_level.display_name(),
        if cli.log_level.is_some() {
            "CLI flag"
        } else {
            "config"
        }
    );

    // Log CLI paths if provided
    if !cli.paths.is_empty() {
        info!("CLI paths provided: {:?}", cli.paths);
    }
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
        .with_app_id("ferrite") // Set app_id for Wayland (helps with alt-tab and taskbar on Linux)
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
        // Enable vsync to limit frame rate to display refresh (reduces CPU usage)
        vsync: true,
        // Use run-on-demand event loop for better power efficiency
        run_and_return: true,
        ..Default::default()
    };

    // Combine CLI paths with any paths received via macOS Apple Events ("Open With")
    let mut initial_paths = cli.paths;
    let apple_event_paths = platform::get_open_file_paths();
    if !apple_event_paths.is_empty() {
        info!("Received paths via Apple Event: {:?}", apple_event_paths);
        initial_paths.extend(apple_event_paths);
    }

    // Run the application
    eframe::run_native(
        APP_NAME,
        native_options,
        Box::new(move |cc| {
            // Configure egui visuals based on theme (basic setup)
            // Full theme support will be implemented in a later task
            let mut app = FerriteApp::new(cc);

            // Open files/directories from CLI arguments and Apple Events
            app.open_initial_paths(initial_paths);

            Ok(Box::new(app))
        }),
    )
}
