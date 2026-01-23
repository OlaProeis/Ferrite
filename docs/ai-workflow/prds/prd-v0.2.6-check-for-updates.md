# PRD: Check for Updates

## Overview

Add a "Check for Updates" button in Settings that checks GitHub for newer versions and, if found, offers to download and install the update in one seamless flow. The app closes and the installer launches automatically.

**User flow:**
1. Click "Check for Updates"
2. If update found → prompt: "v0.2.7 available. Update now? (App will close, save your work)"
3. Click "Yes" → download with progress → app closes → installer launches

This maintains Ferrite's offline-first philosophy - no automatic checking, only triggered by explicit user action.

## User Experience

### Flow Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                                                                          │
│  Settings Panel                                                          │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │  About                                                              │ │
│  │                                                                     │ │
│  │  Ferrite v0.2.6                                                    │ │
│  │  [Check for Updates]                                               │ │
│  │                                                                     │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
                              │
                              │ Click
                              ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  Checking for updates...  [spinner]                                      │
└─────────────────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              │               │               │
              ▼               ▼               ▼
         Up to Date     Update Found       Error
              │               │               │
              ▼               ▼               ▼
         "You're on      Show dialog:    "Could not
          the latest     ┌──────────┐     check..."
          version!"      │ v0.2.7   │
                         │ available│
                         │          │
                         │ Update?  │
                         │ [No][Yes]│
                         └──────────┘
                              │
                              │ Yes
                              ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  Downloading update...  [████████░░░░░░░░]  12.3 / 18.5 MB              │
└─────────────────────────────────────────────────────────────────────────┘
                              │
                              │ Complete
                              ▼
                    ┌─────────────────┐
                    │ Launch installer│
                    │ Close app       │
                    └─────────────────┘
```

### Platform-Specific Behavior

| Platform | Behavior |
|----------|----------|
| **Windows MSI** | Download MSI → launch `msiexec` → app closes |
| **Windows Portable** | Download ZIP → open Downloads folder → show "Run the installer to complete" |
| **macOS** | Download tar.gz → open Downloads folder → show "Run the installer to complete" |
| **Linux tar.gz** | Download tar.gz → open Downloads folder → show "Extract and replace to complete" |
| **Linux packages** | Show message: "Update via your package manager (apt/dnf/pacman)" with link to release notes |

## Technical Implementation

### State Machine

```rust
#[derive(Debug, Clone)]
pub enum UpdateState {
    /// Ready to check
    Idle,
    
    /// Checking GitHub API
    Checking,
    
    /// Already on latest version
    UpToDate,
    
    /// Update found, showing confirmation dialog
    ConfirmUpdate {
        version: String,
        download_url: String,
        file_size: u64,
        release_notes: Option<String>,
    },
    
    /// User confirmed, downloading
    Downloading {
        version: String,
        progress_percent: f32,
        bytes_downloaded: u64,
        total_bytes: u64,
    },
    
    /// Download complete, launching installer
    Installing {
        version: String,
    },
    
    /// Error occurred
    Error(String),
    
    /// Linux package users - special case
    UsePackageManager {
        version: String,
        release_url: String,
    },
}
```

### New Module: `src/update.rs`

```rust
//! Update checking and installation.
//!
//! Checks GitHub Releases for updates and handles platform-specific installation.
//! Only activated by explicit user action (Check for Updates button).

use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

const GITHUB_API: &str = "https://api.github.com/repos/OlaProeis/Ferrite/releases/latest";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum InstallationType {
    WindowsMsi,
    WindowsPortable,
    MacOS,
    LinuxPackage,
    LinuxPortable,
}

impl InstallationType {
    pub fn detect() -> Self {
        #[cfg(target_os = "windows")]
        {
            if let Ok(exe) = std::env::current_exe() {
                let path = exe.to_string_lossy().to_lowercase();
                if path.contains("program files") {
                    return Self::WindowsMsi;
                }
            }
            Self::WindowsPortable
        }
        
        #[cfg(target_os = "macos")]
        { Self::MacOS }
        
        #[cfg(target_os = "linux")]
        {
            if let Ok(exe) = std::env::current_exe() {
                if exe.starts_with("/usr/bin") || exe.starts_with("/usr/local/bin") {
                    return Self::LinuxPackage;
                }
            }
            Self::LinuxPortable
        }
    }
    
    pub fn asset_name(&self) -> &'static str {
        match self {
            Self::WindowsMsi => "ferrite-windows-x64.msi",
            Self::WindowsPortable => "ferrite-portable-windows-x64.zip",
            Self::MacOS => {
                #[cfg(target_arch = "aarch64")]
                { "ferrite-macos-arm64.tar.gz" }
                #[cfg(not(target_arch = "aarch64"))]
                { "ferrite-macos-x64.tar.gz" }
            }
            Self::LinuxPackage | Self::LinuxPortable => "ferrite-linux-x64.tar.gz",
        }
    }
}

#[derive(Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub html_url: String,
    pub assets: Vec<GitHubAsset>,
    pub body: Option<String>,
}

#[derive(Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

// ============================================================================
// Version Comparison
// ============================================================================

/// Parse version string like "v0.2.6" or "0.2.6-hotfix.1" into components
fn parse_version(version: &str) -> Option<(u32, u32, u32)> {
    let v = version.strip_prefix('v').unwrap_or(version);
    let parts: Vec<&str> = v.split('-').next()?.split('.').collect();
    if parts.len() >= 3 {
        Some((
            parts[0].parse().ok()?,
            parts[1].parse().ok()?,
            parts[2].parse().ok()?,
        ))
    } else {
        None
    }
}

/// Check if `latest` is newer than `current`
pub fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_version(latest), parse_version(current)) {
        (Some((l_maj, l_min, l_patch)), Some((c_maj, c_min, c_patch))) => {
            (l_maj, l_min, l_patch) > (c_maj, c_min, c_patch)
        }
        _ => false,
    }
}

// ============================================================================
// GitHub API
// ============================================================================

/// Check GitHub for the latest release
pub fn check_for_update() -> Result<Option<GitHubRelease>, String> {
    let response = ureq::get(GITHUB_API)
        .set("User-Agent", &format!("Ferrite/{}", CURRENT_VERSION))
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("Network error: {e}"))?;
    
    let release: GitHubRelease = response
        .into_json()
        .map_err(|e| format!("Parse error: {e}"))?;
    
    if is_newer(&release.tag_name, CURRENT_VERSION) {
        Ok(Some(release))
    } else {
        Ok(None)
    }
}

// ============================================================================
// Download
// ============================================================================

pub struct DownloadProgress {
    pub bytes: u64,
    pub total: u64,
    pub percent: f32,
}

/// Download file to temp directory with progress updates
pub fn download_update(
    url: &str,
    filename: &str,
    progress_tx: mpsc::Sender<DownloadProgress>,
) -> Result<PathBuf, String> {
    let response = ureq::get(url)
        .set("User-Agent", &format!("Ferrite/{}", CURRENT_VERSION))
        .call()
        .map_err(|e| format!("Download failed: {e}"))?;
    
    let total = response
        .header("Content-Length")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    
    let temp_path = std::env::temp_dir().join(filename);
    let mut file = std::fs::File::create(&temp_path)
        .map_err(|e| format!("Cannot create file: {e}"))?;
    
    let mut reader = response.into_reader();
    let mut buffer = [0u8; 8192];
    let mut downloaded: u64 = 0;
    
    use std::io::{Read, Write};
    
    loop {
        let n = reader.read(&mut buffer).map_err(|e| format!("Read error: {e}"))?;
        if n == 0 { break; }
        
        file.write_all(&buffer[..n]).map_err(|e| format!("Write error: {e}"))?;
        downloaded += n as u64;
        
        let _ = progress_tx.send(DownloadProgress {
            bytes: downloaded,
            total,
            percent: if total > 0 { downloaded as f32 / total as f32 * 100.0 } else { 0.0 },
        });
    }
    
    Ok(temp_path)
}

// ============================================================================
// Installation
// ============================================================================

/// Launch the MSI installer and signal app to close
#[cfg(target_os = "windows")]
pub fn launch_msi_installer(msi_path: &Path) -> Result<(), String> {
    use std::process::Command;
    
    Command::new("msiexec")
        .args(["/i", &msi_path.to_string_lossy()])
        .spawn()
        .map_err(|e| format!("Failed to launch installer: {e}"))?;
    
    Ok(())
}

/// Move file to Downloads and open the folder
pub fn move_to_downloads_and_open(temp_path: &Path, filename: &str) -> Result<PathBuf, String> {
    let downloads = dirs::download_dir()
        .ok_or("Cannot find Downloads folder")?;
    
    let dest = downloads.join(filename);
    
    std::fs::rename(temp_path, &dest)
        .or_else(|_| std::fs::copy(temp_path, &dest).map(|_| ()))
        .map_err(|e| format!("Failed to move file: {e}"))?;
    
    // Open file manager with file selected
    #[cfg(target_os = "windows")]
    { let _ = std::process::Command::new("explorer").args(["/select,", &dest.to_string_lossy()]).spawn(); }
    
    #[cfg(target_os = "macos")]
    { let _ = std::process::Command::new("open").args(["-R", &dest.to_string_lossy()]).spawn(); }
    
    #[cfg(target_os = "linux")]
    { let _ = std::process::Command::new("xdg-open").arg(&downloads).spawn(); }
    
    Ok(dest)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_version_comparison() {
        assert!(is_newer("v0.2.7", "0.2.6"));
        assert!(is_newer("v0.3.0", "0.2.6"));
        assert!(is_newer("v1.0.0", "0.2.99"));
        assert!(!is_newer("v0.2.6", "0.2.6"));
        assert!(!is_newer("v0.2.5", "0.2.6"));
        assert!(!is_newer("v0.2.6-hotfix.1", "0.2.6"));
    }
}
```

### UI: Settings Panel

```rust
// In src/ui/settings.rs - render_about_section()

fn render_about_section(ui: &mut egui::Ui, state: &mut AppState, ctx: &egui::Context) {
    ui.heading(t!("settings.about"));
    ui.add_space(8.0);
    
    ui.label(format!("Ferrite v{}", env!("CARGO_PKG_VERSION")));
    ui.add_space(8.0);
    
    match &state.update_state {
        UpdateState::Idle | UpdateState::UpToDate | UpdateState::Error(_) => {
            if ui.button(t!("settings.check_for_updates")).clicked() {
                state.start_update_check();
            }
            
            if matches!(state.update_state, UpdateState::UpToDate) {
                ui.label(format!("✓ {}", t!("settings.up_to_date")));
            }
            
            if let UpdateState::Error(msg) = &state.update_state {
                ui.colored_label(egui::Color32::RED, format!("⚠ {}", msg));
            }
        }
        
        UpdateState::Checking => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(t!("settings.checking"));
            });
        }
        
        UpdateState::Downloading { progress_percent, bytes_downloaded, total_bytes, .. } => {
            ui.label(t!("settings.downloading"));
            ui.add(egui::ProgressBar::new(*progress_percent / 100.0).show_percentage());
            ui.small(format!("{:.1} / {:.1} MB", 
                *bytes_downloaded as f64 / 1_000_000.0,
                *total_bytes as f64 / 1_000_000.0));
        }
        
        UpdateState::Installing { version } => {
            ui.label(format!("{} {}...", t!("settings.installing"), version));
            ui.spinner();
        }
        
        UpdateState::UsePackageManager { version, release_url } => {
            ui.label(format!("🎉 {} {}", version, t!("settings.available")));
            ui.label(t!("settings.use_package_manager"));
            if ui.link(t!("settings.view_release")).clicked() {
                let _ = open::that(release_url);
            }
        }
        
        _ => {}
    }
    
    ui.add_space(8.0);
    if ui.small_button(t!("settings.view_all_releases")).clicked() {
        let _ = open::that("https://github.com/OlaProeis/Ferrite/releases");
    }
}
```

### UI: Update Confirmation Dialog

```rust
// Show when UpdateState::ConfirmUpdate

fn show_update_dialog(ctx: &egui::Context, state: &mut AppState) {
    if let UpdateState::ConfirmUpdate { version, file_size, .. } = &state.update_state {
        let version = version.clone();
        let size_mb = *file_size as f64 / 1_000_000.0;
        
        egui::Window::new(t!("update.title"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading(format!("🎉 {}", t!("update.available")));
                    ui.add_space(8.0);
                    
                    ui.label(format!("{} → {}", env!("CARGO_PKG_VERSION"), version));
                    ui.small(format!("{:.1} MB", size_mb));
                    
                    ui.add_space(16.0);
                    
                    ui.label(t!("update.will_close_app"));
                    ui.label(egui::RichText::new(t!("update.save_your_work"))
                        .color(egui::Color32::YELLOW));
                    
                    ui.add_space(16.0);
                    
                    ui.horizontal(|ui| {
                        if ui.button(t!("update.not_now")).clicked() {
                            state.update_state = UpdateState::Idle;
                        }
                        
                        ui.add_space(16.0);
                        
                        if ui.button(egui::RichText::new(t!("update.yes_update"))
                            .strong()).clicked() 
                        {
                            state.start_download();
                        }
                    });
                });
            });
    }
}
```

## Dependencies

```toml
# Cargo.toml
[dependencies]
ureq = { version = "2", default-features = false, features = ["tls"] }
```

## Translation Keys

```yaml
# locales/en.yaml
settings:
  about: "About"
  check_for_updates: "Check for Updates"
  checking: "Checking for updates..."
  up_to_date: "You're on the latest version!"
  downloading: "Downloading update..."
  installing: "Installing"
  available: "available"
  use_package_manager: "Please update using your package manager (apt, dnf, pacman)"
  view_release: "View Release Notes"
  view_all_releases: "View all releases"

update:
  title: "Update Available"
  available: "New Version Available!"
  will_close_app: "The app will close to install the update."
  save_your_work: "Please save your work before continuing."
  not_now: "Not Now"
  yes_update: "Yes, Update"
  download_complete: "Download complete. Please run the installer."
```

## Affected Files

| File | Changes |
|------|---------|
| `Cargo.toml` | Add `ureq` dependency |
| `src/update.rs` | New module |
| `src/state.rs` | Add `UpdateState` enum, update handling |
| `src/ui/settings.rs` | Add about/update section |
| `src/app.rs` | Poll update progress, handle app close for install |
| `src/main.rs` | Add `mod update;` |
| `locales/*.yaml` | Add translation keys |

## Success Criteria

1. **Single button** - "Check for Updates" does everything
2. **Clear prompt** - User sees version, size, and warning to save work
3. **Progress feedback** - Download shows progress bar with MB
4. **Windows MSI** - Installer launches, app closes automatically
5. **Other platforms** - File downloaded to Downloads, folder opens, clear instructions shown
6. **Linux packages** - Appropriate message to use package manager
7. **Error handling** - Network errors shown gracefully
8. **No automatic checks** - Only runs when user clicks button

## Platform Behavior Summary

| Platform | On "Yes, Update" |
|----------|------------------|
| **Windows MSI** | Download → Launch `msiexec /i file.msi` → App closes |
| **Windows Portable** | Download to Downloads → Open folder → Toast: "Run installer to complete" |
| **macOS** | Download to Downloads → Open Finder → Toast: "Extract and replace to complete" |
| **Linux tar.gz** | Download to Downloads → Open folder → Toast: "Extract and replace to complete" |
| **Linux packages** | (No download) → Message: "Use apt/dnf/pacman" → Link to release |

## Estimated Effort

| Task | Time |
|------|------|
| `ureq` dependency + module structure | 30 min |
| GitHub API + version comparison | 1-2 hours |
| Platform detection | 1 hour |
| Download with progress | 2 hours |
| Update confirmation dialog | 1-2 hours |
| Settings panel UI | 1-2 hours |
| Windows MSI launch + app close | 1 hour |
| Download-to-folder + open (other platforms) | 1 hour |
| Translation keys | 30 min |
| Testing | 3-4 hours |
| **Total** | **~2-3 days** |

## Out of Scope

- ❌ Automatic checking on startup
- ❌ Background/periodic checking
- ❌ Auto-replacement for portable versions (Phase 2)
- ❌ Delta updates
- ❌ Rollback mechanism
