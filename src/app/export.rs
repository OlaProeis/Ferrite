//! Export operations for the Ferrite application.
//!
//! This module contains handlers for HTML export and copy-as-HTML.

use super::FerriteApp;
use crate::export::{copy_html_to_clipboard, generate_html_document};
use crate::files::dialogs::detect_linux_desktop;
use eframe::egui;
use log::{debug, info, warn};
use rust_i18n::t;

impl FerriteApp {
    pub(crate) fn handle_export_html(&mut self, ctx: &egui::Context) {
        // Get the active tab content
        let Some(tab) = self.state.active_tab() else {
            let time = self.get_app_time();
            self.state
                .show_toast(t!("notification.no_document_export").to_string(), time, 2.0);
            return;
        };

        let content = tab.content.clone();
        let source_path = tab.path.clone();

        // Determine initial directory and default filename
        let initial_dir = source_path
            .as_ref()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf())
            .or_else(|| self.state.settings.last_export_directory.clone())
            .or_else(|| {
                self.state
                    .settings
                    .recent_files
                    .first()
                    .and_then(|p| p.parent())
                    .map(|p| p.to_path_buf())
            });

        let default_name = source_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .and_then(|s| s.to_str())
            .map(|s| format!("{}.html", s))
            .unwrap_or_else(|| "exported.html".to_string());

        // Get current theme colors
        let theme_colors = self.theme_manager.colors(ctx);

        // Open save dialog for HTML
        let filter = rfd::FileDialog::new()
            .add_filter("HTML Files", &["html", "htm"])
            .set_file_name(&default_name);

        let filter = if let Some(dir) = initial_dir.as_ref() {
            filter.set_directory(dir)
        } else {
            filter
        };

        let save_result = filter.save_file();

        let path = match save_result {
            Some(p) => p,
            None => {
                let (desktop_env, requires_portal) = detect_linux_desktop();
                if requires_portal {
                    debug!(
                        "Export save dialog returned None on {} (portal-requiring desktop). \
                         If no dialog appeared, check xdg-desktop-portal installation.",
                        desktop_env.as_deref().unwrap_or("unknown")
                    );
                } else {
                    debug!("Export save dialog cancelled");
                }
                return;
            }
        };

        // Get document title
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Exported Document");

        // Generate HTML with paragraph indentation setting
        match generate_html_document(
            &content,
            Some(title),
            &theme_colors,
            true,
            self.state.settings.paragraph_indent,
        ) {
            Ok(html) => {
                // Write to file
                match std::fs::write(&path, html) {
                    Ok(()) => {
                        info!("Exported HTML to: {}", path.display());

                        // Update last export directory
                        if let Some(parent) = path.parent() {
                            self.state.settings.last_export_directory = Some(parent.to_path_buf());
                            self.state.mark_settings_dirty();
                        }

                        let time = self.get_app_time();
                        self.state.show_toast(
                            t!(
                                "notification.exported_to",
                                path = path.display().to_string()
                            )
                            .to_string(),
                            time,
                            2.5,
                        );

                        // Optionally open the file
                        if self.state.settings.open_after_export {
                            if let Err(e) = open::that(&path) {
                                warn!("Failed to open exported file: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to write HTML file: {}", e);
                        let time = self.get_app_time();
                        self.state.show_toast(
                            t!("notification.export_failed", error = e.to_string()).to_string(),
                            time,
                            3.0,
                        );
                    }
                }
            }
            Err(e) => {
                warn!("Failed to generate HTML: {}", e);
                let time = self.get_app_time();
                self.state
                    .show_toast(format!("Export failed: {}", e), time, 3.0);
            }
        }
    }

    /// Handle copying the current document as HTML to clipboard.
    pub(crate) fn handle_copy_as_html(&mut self) {
        // Get the active tab content
        let Some(tab) = self.state.active_tab() else {
            let time = self.get_app_time();
            self.state
                .show_toast(t!("notification.no_document_copy").to_string(), time, 2.0);
            return;
        };

        let content = tab.content.clone();

        // Copy HTML to clipboard
        match copy_html_to_clipboard(&content) {
            Ok(()) => {
                info!("Copied HTML to clipboard");
                let time = self.get_app_time();
                self.state
                    .show_toast(t!("notification.html_copied").to_string(), time, 2.0);
            }
            Err(e) => {
                warn!("Failed to copy HTML to clipboard: {}", e);
                let time = self.get_app_time();
                self.state.show_toast(
                    t!("notification.copy_failed", error = e.to_string()).to_string(),
                    time,
                    3.0,
                );
            }
        }
    }
}
