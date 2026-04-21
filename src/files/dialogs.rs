//! Native file dialog integration using the rfd crate
//!
//! This module provides functions to open native file picker dialogs
//! for opening and saving files, and for opening workspace folders.
//!
//! On Linux, rfd uses xdg-desktop-portal by default (via its built-in ashpd
//! backend). In Flatpak sandboxes, portal-based dialogs grant the app access
//! to user-selected paths without requiring broad filesystem permissions.

use log::debug;
use rfd::FileDialog;
use rust_i18n::t;
use std::path::PathBuf;

/// Result type for file dialog operations that may fail due to portal issues.
#[derive(Debug, Clone)]
pub enum DialogResult<T> {
    /// User selected a path/file(s)
    Success(T),
    /// User cancelled the dialog
    Cancelled,
    /// Dialog failed (likely portal issue on Linux)
    Failed {
        is_portal_error: bool,
        desktop_env: Option<String>,
    },
}

impl<T> DialogResult<T> {
    /// Convert to Option, discarding error information
    pub fn ok(self) -> Option<T> {
        match self {
            DialogResult::Success(t) => Some(t),
            _ => None,
        }
    }

    /// Check if this is a portal-related failure
    pub fn is_portal_failure(&self) -> bool {
        matches!(
            self,
            DialogResult::Failed {
                is_portal_error: true,
                ..
            }
        )
    }
}

/// File extension filters for supported file types.
const MARKDOWN_EXTENSIONS: &[&str] = &["md", "markdown", "mdown", "mkd", "mkdn"];
const JSON_EXTENSIONS: &[&str] = &["json", "jsonc"];
const YAML_EXTENSIONS: &[&str] = &["yaml", "yml"];
const TOML_EXTENSIONS: &[&str] = &["toml"];
const TEXT_EXTENSIONS: &[&str] = &["txt", "text"];
const CSV_EXTENSIONS: &[&str] = &["csv", "tsv"];

/// Combined filter for all commonly edited file types (default filter).
/// Includes markdown, text, and data files that Ferrite supports.
const SUPPORTED_EXTENSIONS: &[&str] = &[
    "md", "markdown", "mdown", "mkd", "mkdn", // Markdown
    "txt", "text", // Plain text
    "json", "jsonc", // JSON
    "yaml", "yml",  // YAML
    "toml", // TOML
    "csv", "tsv", // Tabular data
];

/// Returns true when running inside a Flatpak sandbox.
pub fn is_flatpak() -> bool {
    std::env::var("FLATPAK_ID").is_ok()
}

/// Returns true when running on Linux (any flavor).
pub fn is_linux() -> bool {
    cfg!(target_os = "linux")
}

/// Detects the current Linux desktop environment from environment variables.
/// Returns a tuple of (desktop_name, requires_portal) where requires_portal
/// indicates if this DE typically needs xdg-desktop-portal for file dialogs.
pub fn detect_linux_desktop() -> (Option<String>, bool) {
    if !is_linux() {
        return (None, false);
    }

    // Check XDG_CURRENT_DESKTOP first (most reliable)
    if let Ok(desktop) = std::env::var("XDG_CURRENT_DESKTOP") {
        let desktop_lower = desktop.to_lowercase();

        // Desktop environments that require xdg-desktop-portal
        let requires_portal = matches!(
            desktop_lower.as_str(),
            "hyprland"
                | "sway"
                | "i3"
                | "bspwm"
                | "dwm"
                | "awesomewm"
                | "xmonad"
                | "qtile"
                | "river"
                | "niri"
                | "cosmic"
                | "wayfire"
                | "labwc"
                | " Weston" // Some compositors might not have full portal support
        );

        // Desktop environments with native file dialog support
        let has_native = matches!(
            desktop_lower.as_str(),
            "gnome"
                | "kde"
                | "plasma"
                | "xfce"
                | "mate"
                | "cinnamon"
                | "x-cinnamon"
                | "lxde"
                | "lxqt"
                | "budgie"
        );

        return (Some(desktop), requires_portal || !has_native);
    }

    // Fallback: check DESKTOP_SESSION
    if let Ok(session) = std::env::var("DESKTOP_SESSION") {
        let session_lower = session.to_lowercase();
        let requires_portal = session_lower.contains("hyprland")
            || session_lower.contains("sway")
            || session_lower.contains("i3");
        return (Some(session), requires_portal);
    }

    // Unknown desktop on Linux - assume it might need portal
    (None, true)
}

/// Returns a human-readable name for the current Linux distro based on common files.
pub fn detect_linux_distro() -> Option<String> {
    // Try /etc/os-release first (standard)
    if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if line.starts_with("NAME=") {
                return line
                    .trim_start_matches("NAME=")
                    .trim_matches('"')
                    .to_string()
                    .into();
            }
            if line.starts_with("ID=") {
                return line
                    .trim_start_matches("ID=")
                    .trim_matches('"')
                    .to_string()
                    .into();
            }
        }
    }

    // Fallback to other common files
    for path in &[
        "/etc/arch-release",
        "/etc/debian_version",
        "/etc/fedora-release",
    ] {
        if std::path::Path::new(path).exists() {
            return match *path {
                "/etc/arch-release" => Some("arch".to_string()),
                "/etc/debian_version" => Some("debian".to_string()),
                "/etc/fedora-release" => Some("fedora".to_string()),
                _ => None,
            };
        }
    }

    None
}

/// Returns the install command for xdg-desktop-portal packages based on distro
/// and desktop environment. Cinnamon/Mint recommends xapp/gtk backends instead of wlr.
pub fn portal_install_instructions(desktop_env: Option<&str>) -> (&'static str, Vec<&'static str>) {
    let distro = detect_linux_distro().unwrap_or_default();

    let is_cinnamon = desktop_env
        .map(|d| {
            let d = d.to_lowercase();
            d == "cinnamon" || d == "x-cinnamon"
        })
        .unwrap_or(false);

    let portal_backends: &[&str] = if is_cinnamon {
        &["xdg-desktop-portal-xapp", "xdg-desktop-portal-gtk"]
    } else {
        &["xdg-desktop-portal-hyprland", "xdg-desktop-portal-wlr"]
    };

    let mut packages = vec!["xdg-desktop-portal"];
    packages.extend_from_slice(portal_backends);

    let cmd = match distro.as_str() {
        "arch" | "manjaro" | "endeavouros" | "garuda" => "pacman -S",
        "debian" | "ubuntu" | "pop" | "mint" | "elementary" => "apt install",
        "fedora" | "nobara" => "dnf install",
        "opensuse" | "suse" => "zypper install",
        _ => "<package-manager> install",
    };

    (cmd, packages)
}

/// Resolve initial directory for a file dialog, with Flatpak-aware fallback.
///
/// In Flatpak, the xdg-desktop-portal file chooser needs a navigable starting
/// directory. Without one, the portal may fail silently or start in an
/// inaccessible sandbox-internal path. We fall back to `$HOME` (which the
/// portal can translate) to ensure the dialog always opens at a usable location.
fn resolve_initial_dir(initial_dir: Option<&PathBuf>) -> Option<PathBuf> {
    if let Some(dir) = initial_dir {
        if dir.is_dir() {
            return Some(dir.clone());
        }
        debug!("Provided initial_dir does not exist: {}", dir.display());
    }

    // Fallback: use $HOME so the portal dialog has a navigable starting point.
    // This is especially important in Flatpak where the default may be
    // a sandbox-internal path the user can't browse from.
    if let Some(home) = dirs::home_dir() {
        if home.is_dir() {
            return Some(home);
        }
    }

    None
}

/// Opens a native folder picker dialog for selecting a workspace folder.
///
/// Uses xdg-desktop-portal automatically on Linux (rfd's default backend).
/// In Flatpak, the portal grants sandbox access to the selected directory.
/// Returns `DialogResult::Success(PathBuf)` if a folder was selected.
pub fn open_folder_dialog(initial_dir: Option<&PathBuf>) -> DialogResult<PathBuf> {
    let effective_dir = resolve_initial_dir(initial_dir);

    let mut dialog = FileDialog::new().set_title(&t!("file_dialog.open_workspace").to_string());

    if let Some(dir) = effective_dir.as_ref() {
        dialog = dialog.set_directory(dir);
    }

    let result = dialog.pick_folder();

    match result {
        Some(path) => DialogResult::Success(path),
        None => {
            // rfd's sync API returns None for both user cancellation and portal failure.
            // Always treat as cancellation to avoid false-positive error dialogs.
            if is_flatpak() {
                debug!(
                    "Folder dialog returned None in Flatpak (initial_dir: {:?}). \
                     This may be a portal/sandbox issue or the user cancelled.",
                    initial_dir
                );
            }

            let (desktop_env, requires_portal) = detect_linux_desktop();
            if is_linux() && requires_portal {
                debug!(
                    "Folder dialog returned None on {} (portal-requiring desktop). \
                     If no dialog appeared, check xdg-desktop-portal installation.",
                    desktop_env.as_deref().unwrap_or("unknown")
                );
            }
            DialogResult::Cancelled
        }
    }
}

/// Opens a native file dialog for selecting multiple files.
///
/// Supports Markdown, JSON, YAML, TOML, CSV/TSV, and plain text files.
/// The default filter shows all supported file types.
/// Returns `DialogResult::Success(Vec<PathBuf>)` if files were selected.
pub fn open_multiple_files_dialog(initial_dir: Option<&PathBuf>) -> DialogResult<Vec<PathBuf>> {
    let effective_dir = resolve_initial_dir(initial_dir);

    let mut dialog = FileDialog::new()
        .set_title(&t!("file_dialog.open_files").to_string())
        .add_filter(
            &t!("file_dialog.filter.supported").to_string(),
            SUPPORTED_EXTENSIONS,
        )
        .add_filter(
            &t!("file_dialog.filter.markdown").to_string(),
            MARKDOWN_EXTENSIONS,
        )
        .add_filter(&t!("file_dialog.filter.text").to_string(), TEXT_EXTENSIONS)
        .add_filter(&t!("file_dialog.filter.json").to_string(), JSON_EXTENSIONS)
        .add_filter(&t!("file_dialog.filter.yaml").to_string(), YAML_EXTENSIONS)
        .add_filter(&t!("file_dialog.filter.toml").to_string(), TOML_EXTENSIONS)
        .add_filter(
            &t!("file_dialog.filter.csv_tsv").to_string(),
            CSV_EXTENSIONS,
        )
        .add_filter(&t!("file_dialog.filter.all").to_string(), &["*"]);

    if let Some(dir) = effective_dir.as_ref() {
        dialog = dialog.set_directory(dir);
    }

    let result = dialog.pick_files();

    match result {
        Some(paths) if !paths.is_empty() => DialogResult::Success(paths),
        _ => {
            let (desktop_env, requires_portal) = detect_linux_desktop();
            if is_linux() && requires_portal {
                debug!(
                    "File open dialog returned None on {} (portal-requiring desktop). \
                     If no dialog appeared, check xdg-desktop-portal installation.",
                    desktop_env.as_deref().unwrap_or("unknown")
                );
            }
            DialogResult::Cancelled
        }
    }
}

/// Opens a native save dialog for saving a file.
///
/// Returns `DialogResult::Success(PathBuf)` if a location was selected.
pub fn save_file_dialog(
    initial_dir: Option<&PathBuf>,
    default_name: Option<&str>,
) -> DialogResult<PathBuf> {
    let effective_dir = resolve_initial_dir(initial_dir);

    let mut dialog = FileDialog::new()
        .set_title(&t!("file_dialog.save_file").to_string())
        .add_filter(
            &t!("file_dialog.filter.supported").to_string(),
            SUPPORTED_EXTENSIONS,
        )
        .add_filter(
            &t!("file_dialog.filter.markdown").to_string(),
            MARKDOWN_EXTENSIONS,
        )
        .add_filter(&t!("file_dialog.filter.text").to_string(), TEXT_EXTENSIONS)
        .add_filter(&t!("file_dialog.filter.json").to_string(), JSON_EXTENSIONS)
        .add_filter(&t!("file_dialog.filter.yaml").to_string(), YAML_EXTENSIONS)
        .add_filter(&t!("file_dialog.filter.toml").to_string(), TOML_EXTENSIONS)
        .add_filter(
            &t!("file_dialog.filter.csv_tsv").to_string(),
            CSV_EXTENSIONS,
        )
        .add_filter(&t!("file_dialog.filter.all").to_string(), &["*"]);

    if let Some(dir) = effective_dir.as_ref() {
        dialog = dialog.set_directory(dir);
    }

    if let Some(name) = default_name {
        dialog = dialog.set_file_name(name);
    }

    let result = dialog.save_file();

    match result {
        Some(path) => DialogResult::Success(path),
        None => {
            let (desktop_env, requires_portal) = detect_linux_desktop();
            if is_linux() && requires_portal {
                debug!(
                    "Save dialog returned None on {} (portal-requiring desktop). \
                     If no dialog appeared, check xdg-desktop-portal installation.",
                    desktop_env.as_deref().unwrap_or("unknown")
                );
            }
            DialogResult::Cancelled
        }
    }
}
