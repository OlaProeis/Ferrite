//! Theme Manager for Ferrite
//!
//! This module provides centralized theme management, handling theme switching,
//! persistence, and application of themes to the egui context.

// Allow dead code - manager has comprehensive theme methods for future use
#![allow(dead_code)]

//! # Usage
//!
//! ```ignore
//! use crate::theme::ThemeManager;
//! use crate::config::Theme;
//!
//! // Create manager with initial theme
//! let mut manager = ThemeManager::new(Theme::Dark);
//!
//! // Apply theme to egui context
//! manager.apply(&ctx);
//!
//! // Switch themes
//! manager.set_theme(Theme::Light);
//! manager.apply(&ctx);
//!
//! // Toggle between light/dark
//! manager.toggle();
//! manager.apply(&ctx);
//! ```

use eframe::egui::{Context, Visuals};
use log::{debug, info};

use super::{dark, light, ThemeColors};
use crate::config::Theme;

// ─────────────────────────────────────────────────────────────────────────────
// Theme Manager
// ─────────────────────────────────────────────────────────────────────────────

/// Manages theme state and applies themes to the egui context.
///
/// The ThemeManager centralizes all theme-related operations:
/// - Storing the current theme preference
/// - Converting Theme enum to egui Visuals
/// - Applying themes to the egui context
/// - Handling System theme detection
#[derive(Debug, Clone)]
pub struct ThemeManager {
    /// Current theme setting (Light, Dark, or System)
    current_theme: Theme,
    /// Cached visuals for the current theme
    cached_visuals: Option<Visuals>,
    /// Whether the theme needs to be reapplied
    needs_apply: bool,
    /// Last detected system dark mode state (for System theme)
    last_system_dark_mode: Option<bool>,
}

impl ThemeManager {
    /// Create a new ThemeManager with the given initial theme.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let manager = ThemeManager::new(Theme::Dark);
    /// ```
    pub fn new(theme: Theme) -> Self {
        info!("ThemeManager initialized with theme: {:?}", theme);
        Self {
            current_theme: theme,
            cached_visuals: None,
            needs_apply: true,
            last_system_dark_mode: None,
        }
    }

    /// Get the current theme setting.
    pub fn current_theme(&self) -> Theme {
        self.current_theme
    }

    /// Set the theme and mark for reapplication.
    ///
    /// This doesn't apply the theme immediately - call `apply()` to update the UI.
    pub fn set_theme(&mut self, theme: Theme) {
        if self.current_theme != theme {
            info!("Theme changed from {:?} to {:?}", self.current_theme, theme);
            self.current_theme = theme;
            self.cached_visuals = None;
            self.needs_apply = true;
        }
    }

    /// Toggle between Light and Dark themes.
    ///
    /// If System is currently selected, switches to Dark.
    /// If Light, switches to Dark.
    /// If Dark, switches to Light.
    ///
    /// Returns the new theme.
    pub fn toggle(&mut self) -> Theme {
        let new_theme = match self.current_theme {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Light,
            Theme::System => Theme::Dark, // System -> Dark as starting point
        };
        self.set_theme(new_theme);
        new_theme
    }

    /// Cycle between Light and Dark themes only (skips System).
    ///
    /// Returns the new theme.
    pub fn cycle(&mut self) -> Theme {
        let new_theme = match self.current_theme {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Light,
            Theme::System => Theme::Light, // If somehow on System, go to Light
        };
        self.set_theme(new_theme);
        new_theme
    }

    /// Check if the theme needs to be reapplied.
    ///
    /// This is true after `set_theme()` is called, or when using System theme
    /// and the system preference may have changed.
    pub fn needs_apply(&self) -> bool {
        self.needs_apply
    }

    /// Apply the current theme to the egui context.
    ///
    /// This should be called once per frame if `needs_apply()` returns true,
    /// or unconditionally for simplicity.
    ///
    /// For System theme, this checks the current system preference and updates
    /// accordingly.
    pub fn apply(&mut self, ctx: &Context) {
        let visuals = self.get_or_create_visuals(ctx);
        ctx.set_visuals(visuals);
        self.needs_apply = false;
        debug!("Applied theme: {:?}", self.current_theme);
    }

    /// Apply the theme only if needed (theme changed or system preference changed).
    ///
    /// This is more efficient than `apply()` as it only updates egui when necessary.
    /// Returns `true` if the theme was applied.
    pub fn apply_if_needed(&mut self, ctx: &Context) -> bool {
        // For System theme, check if system preference changed
        if self.current_theme == Theme::System {
            let current_system_dark = ctx.style().visuals.dark_mode;
            if self.last_system_dark_mode != Some(current_system_dark) {
                self.last_system_dark_mode = Some(current_system_dark);
                self.needs_apply = true;
                debug!("System dark mode changed to: {}", current_system_dark);
            }
        }

        if self.needs_apply {
            self.apply(ctx);
            true
        } else {
            false
        }
    }

    /// Get the visuals for the current theme, creating them if necessary.
    fn get_or_create_visuals(&mut self, ctx: &Context) -> Visuals {
        if let Some(ref visuals) = self.cached_visuals {
            return visuals.clone();
        }

        let visuals = match self.current_theme {
            Theme::Light => light::create_light_visuals(),
            Theme::Dark => dark::create_dark_visuals(),
            Theme::System => {
                // Follow system preference
                let system_dark = ctx.style().visuals.dark_mode;
                self.last_system_dark_mode = Some(system_dark);
                if system_dark {
                    dark::create_dark_visuals()
                } else {
                    light::create_light_visuals()
                }
            }
        };

        self.cached_visuals = Some(visuals.clone());
        visuals
    }

    /// Get the current theme colors.
    ///
    /// This returns the `ThemeColors` for the effective theme (resolving System
    /// to the actual light/dark variant).
    pub fn colors(&self, ctx: &Context) -> ThemeColors {
        ThemeColors::from_theme(self.current_theme, &ctx.style().visuals)
    }

    /// Check if the current effective theme is dark.
    ///
    /// For System theme, this returns the actual system preference.
    pub fn is_dark(&self, ctx: &Context) -> bool {
        match self.current_theme {
            Theme::Dark => true,
            Theme::Light => false,
            Theme::System => ctx.style().visuals.dark_mode,
        }
    }

    /// Get a display label for the current theme.
    pub fn label(&self) -> &'static str {
        match self.current_theme {
            Theme::Light => "Light",
            Theme::Dark => "Dark",
            Theme::System => "System",
        }
    }

    /// Get an icon for the current theme.
    pub fn icon(&self) -> &'static str {
        match self.current_theme {
            Theme::Light => "☀",
            Theme::Dark => "🌙",
            Theme::System => "💻",
        }
    }

    /// Get a tooltip describing the current theme.
    pub fn tooltip(&self, ctx: &Context) -> String {
        match self.current_theme {
            Theme::Light => "Light theme".to_string(),
            Theme::Dark => "Dark theme".to_string(),
            Theme::System => {
                let effective = if ctx.style().visuals.dark_mode {
                    "dark"
                } else {
                    "light"
                };
                format!("System theme (currently {})", effective)
            }
        }
    }

    /// Force refresh the theme (invalidates cache).
    ///
    /// Call this if external factors (like syntax theme changes) require
    /// the theme to be recomputed.
    pub fn refresh(&mut self) {
        self.cached_visuals = None;
        self.needs_apply = true;
        debug!("Theme refresh requested");
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new(Theme::default())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_manager_new() {
        let manager = ThemeManager::new(Theme::Dark);
        assert_eq!(manager.current_theme(), Theme::Dark);
        assert!(manager.needs_apply());
    }

    #[test]
    fn test_theme_manager_default() {
        let manager = ThemeManager::default();
        // Default theme is Light (from Theme::default())
        assert_eq!(manager.current_theme(), Theme::Light);
    }

    #[test]
    fn test_theme_manager_set_theme() {
        let mut manager = ThemeManager::new(Theme::Light);

        manager.set_theme(Theme::Dark);
        assert_eq!(manager.current_theme(), Theme::Dark);
        assert!(manager.needs_apply());
    }

    #[test]
    fn test_theme_manager_set_same_theme() {
        let mut manager = ThemeManager::new(Theme::Light);
        manager.needs_apply = false; // Simulate already applied

        manager.set_theme(Theme::Light); // Same theme
        assert!(!manager.needs_apply()); // Should not need reapply
    }

    #[test]
    fn test_theme_manager_toggle() {
        let mut manager = ThemeManager::new(Theme::Light);

        let new_theme = manager.toggle();
        assert_eq!(new_theme, Theme::Dark);
        assert_eq!(manager.current_theme(), Theme::Dark);

        let new_theme = manager.toggle();
        assert_eq!(new_theme, Theme::Light);
    }

    #[test]
    fn test_theme_manager_toggle_from_system() {
        let mut manager = ThemeManager::new(Theme::System);

        let new_theme = manager.toggle();
        assert_eq!(new_theme, Theme::Dark); // System -> Dark
    }

    #[test]
    fn test_theme_manager_cycle() {
        let mut manager = ThemeManager::new(Theme::Light);

        // Cycle should only toggle between Light and Dark (skip System)
        assert_eq!(manager.cycle(), Theme::Dark);
        assert_eq!(manager.cycle(), Theme::Light);
        assert_eq!(manager.cycle(), Theme::Dark);

        // If on System, cycle should go to Light
        manager.set_theme(Theme::System);
        assert_eq!(manager.cycle(), Theme::Light);
    }

    #[test]
    fn test_theme_manager_labels() {
        let mut manager = ThemeManager::new(Theme::Light);
        assert_eq!(manager.label(), "Light");
        assert_eq!(manager.icon(), "☀");

        manager.set_theme(Theme::Dark);
        assert_eq!(manager.label(), "Dark");
        assert_eq!(manager.icon(), "🌙");

        manager.set_theme(Theme::System);
        assert_eq!(manager.label(), "System");
        assert_eq!(manager.icon(), "💻");
    }

    #[test]
    fn test_theme_manager_refresh() {
        let mut manager = ThemeManager::new(Theme::Light);
        manager.needs_apply = false;
        manager.cached_visuals = Some(Visuals::light());

        manager.refresh();

        assert!(manager.needs_apply());
        assert!(manager.cached_visuals.is_none());
    }

    #[test]
    fn test_theme_manager_clone() {
        let manager = ThemeManager::new(Theme::Dark);
        let cloned = manager.clone();

        assert_eq!(cloned.current_theme(), Theme::Dark);
    }
}
