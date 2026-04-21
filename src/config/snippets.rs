//! Snippets/Abbreviation System for Ferrite
//!
//! This module provides user-defined text expansions with built-in date/time snippets.
//! Users can type a trigger word followed by space/tab, and it expands to the full text.
//!
//! ## Built-in Snippets
//!
//! - `;date` → Current date (YYYY-MM-DD)
//! - `;time` → Current time (HH:MM)
//! - `;datetime` → Current date and time (YYYY-MM-DD HH:MM)
//! - `;now` → ISO 8601 timestamp
//!
//! ## Custom Snippets
//!
//! Custom snippets are stored in `~/.config/ferrite/snippets.json` (or platform equivalent)
//! in JSON format: `{ "trigger": "expansion text" }`

use chrono::Local;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

// ─────────────────────────────────────────────────────────────────────────────
// Snippet Configuration
// ─────────────────────────────────────────────────────────────────────────────

/// Configuration for the snippets/abbreviation system.
///
/// Stores user-defined text expansions and settings for the snippet feature.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SnippetConfig {
    /// Whether snippets expansion is enabled
    pub enabled: bool,
    /// User-defined snippets (trigger → expansion)
    pub snippets: HashMap<String, String>,
}

impl Default for SnippetConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            snippets: HashMap::new(),
        }
    }
}

impl SnippetConfig {
    /// Create a new empty snippet configuration.
    #[allow(dead_code)] // Public API for future settings UI
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the expansion for a built-in snippet trigger.
    ///
    /// Built-in snippets are evaluated at expansion time to get current values:
    /// - `;date` → Current date (YYYY-MM-DD)
    /// - `;time` → Current time (HH:MM)
    /// - `;datetime` → Current date and time (YYYY-MM-DD HH:MM)
    /// - `;now` → ISO 8601 timestamp
    ///
    /// Returns `None` if the trigger is not a built-in snippet.
    pub fn builtin_expansion(trigger: &str) -> Option<String> {
        let now = Local::now();
        match trigger {
            ";date" => Some(now.format("%Y-%m-%d").to_string()),
            ";time" => Some(now.format("%H:%M").to_string()),
            ";datetime" => Some(now.format("%Y-%m-%d %H:%M").to_string()),
            ";now" => Some(now.to_rfc3339()),
            _ => None,
        }
    }

    /// Get the expansion for a trigger (checks built-in first, then user-defined).
    ///
    /// Returns `None` if the trigger is not found or snippets are disabled.
    pub fn get_expansion(&self, trigger: &str) -> Option<String> {
        if !self.enabled {
            return None;
        }

        // Check built-in snippets first
        if let Some(expansion) = Self::builtin_expansion(trigger) {
            return Some(expansion);
        }

        // Check user-defined snippets
        self.snippets.get(trigger).cloned()
    }

    /// Add or update a user-defined snippet.
    #[allow(dead_code)] // Public API for future settings UI
    pub fn set_snippet(&mut self, trigger: String, expansion: String) {
        self.snippets.insert(trigger, expansion);
    }

    /// Remove a user-defined snippet.
    ///
    /// Returns the removed expansion if it existed.
    #[allow(dead_code)] // Public API for future settings UI
    pub fn remove_snippet(&mut self, trigger: &str) -> Option<String> {
        self.snippets.remove(trigger)
    }

    /// Check if a trigger is a built-in snippet.
    #[allow(dead_code)] // Public API for future settings UI
    pub fn is_builtin(trigger: &str) -> bool {
        matches!(trigger, ";date" | ";time" | ";datetime" | ";now")
    }

    /// Get all built-in snippet triggers.
    #[allow(dead_code)] // Public API for future settings UI
    pub fn builtin_triggers() -> &'static [&'static str] {
        &[";date", ";time", ";datetime", ";now"]
    }

    /// Get descriptions for built-in snippets.
    #[allow(dead_code)] // Public API for future settings UI
    pub fn builtin_description(trigger: &str) -> Option<&'static str> {
        match trigger {
            ";date" => Some("Current date (YYYY-MM-DD)"),
            ";time" => Some("Current time (HH:MM)"),
            ";datetime" => Some("Current date and time (YYYY-MM-DD HH:MM)"),
            ";now" => Some("ISO 8601 timestamp"),
            _ => None,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Snippet Manager (Loading/Saving)
// ─────────────────────────────────────────────────────────────────────────────

/// Manages loading and saving of snippet configuration.
#[derive(Debug, Clone)]
pub struct SnippetManager {
    /// The current snippet configuration
    pub config: SnippetConfig,
    /// Path to the snippets config file
    config_path: PathBuf,
    /// Last modified time of the config file (for change detection)
    last_modified: Option<std::time::SystemTime>,
}

impl Default for SnippetManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SnippetManager {
    /// Create a new snippet manager and load configuration from disk.
    pub fn new() -> Self {
        let config_path = Self::config_path();
        let mut manager = Self {
            config: SnippetConfig::default(),
            config_path,
            last_modified: None,
        };
        manager.load();
        manager
    }

    /// Get the path to the snippets configuration file.
    ///
    /// Uses the centralized config directory (supports portable mode):
    /// - Portable: `<exe_dir>/portable/snippets.json`
    /// - Linux: `~/.config/ferrite/snippets.json`
    /// - macOS: `~/Library/Application Support/ferrite/snippets.json`
    /// - Windows: `%APPDATA%\ferrite\snippets.json`
    pub fn config_path() -> PathBuf {
        use crate::config::persistence::get_config_dir;
        get_config_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("snippets.json")
    }

    /// Load snippet configuration from disk.
    ///
    /// If the file doesn't exist, uses default (empty) configuration.
    /// If the file is malformed, logs an error and uses default configuration.
    pub fn load(&mut self) {
        if !self.config_path.exists() {
            debug!(
                "Snippets config file not found at {:?}, using defaults",
                self.config_path
            );
            return;
        }

        match fs::read_to_string(&self.config_path) {
            Ok(content) => match serde_json::from_str::<SnippetConfig>(&content) {
                Ok(config) => {
                    info!(
                        "Loaded {} custom snippets from {:?}",
                        config.snippets.len(),
                        self.config_path
                    );
                    self.config = config;
                    self.last_modified = fs::metadata(&self.config_path)
                        .ok()
                        .and_then(|m| m.modified().ok());
                }
                Err(e) => {
                    error!(
                        "Failed to parse snippets config at {:?}: {}",
                        self.config_path, e
                    );
                    // Keep default config
                }
            },
            Err(e) => {
                error!(
                    "Failed to read snippets config at {:?}: {}",
                    self.config_path, e
                );
                // Keep default config
            }
        }
    }

    /// Save snippet configuration to disk.
    ///
    /// Creates the config directory if it doesn't exist.
    #[allow(dead_code)] // Public API for future settings UI
    pub fn save(&mut self) -> Result<(), String> {
        // Ensure config directory exists
        if let Some(parent) = self.config_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| {
                    format!("Failed to create config directory {:?}: {}", parent, e)
                })?;
            }
        }

        let content = serde_json::to_string_pretty(&self.config)
            .map_err(|e| format!("Failed to serialize snippets config: {}", e))?;

        fs::write(&self.config_path, &content).map_err(|e| {
            format!(
                "Failed to write snippets config to {:?}: {}",
                self.config_path, e
            )
        })?;

        self.last_modified = fs::metadata(&self.config_path)
            .ok()
            .and_then(|m| m.modified().ok());

        info!(
            "Saved {} custom snippets to {:?}",
            self.config.snippets.len(),
            self.config_path
        );
        Ok(())
    }

    /// Check if the config file has been modified externally and reload if needed.
    ///
    /// Returns `true` if the config was reloaded.
    #[allow(dead_code)] // Public API for hot-reload feature
    pub fn check_and_reload(&mut self) -> bool {
        if !self.config_path.exists() {
            return false;
        }

        let current_modified = fs::metadata(&self.config_path)
            .ok()
            .and_then(|m| m.modified().ok());

        if current_modified != self.last_modified {
            debug!("Snippets config file changed, reloading");
            self.load();
            true
        } else {
            false
        }
    }

    /// Get the expansion for a trigger.
    pub fn get_expansion(&self, trigger: &str) -> Option<String> {
        self.config.get_expansion(trigger)
    }

    /// Check if snippets are enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Enable or disable snippets.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
    }

    /// Add or update a custom snippet.
    #[allow(dead_code)] // Public API for future settings UI
    pub fn set_snippet(&mut self, trigger: String, expansion: String) {
        self.config.set_snippet(trigger, expansion);
    }

    /// Remove a custom snippet.
    #[allow(dead_code)] // Public API for future settings UI
    pub fn remove_snippet(&mut self, trigger: &str) -> Option<String> {
        self.config.remove_snippet(trigger)
    }

    /// Get an iterator over custom snippets.
    #[allow(dead_code)] // Public API for future settings UI
    pub fn custom_snippets(&self) -> impl Iterator<Item = (&String, &String)> {
        self.config.snippets.iter()
    }

    /// Get the number of custom snippets.
    #[allow(dead_code)] // Public API for future settings UI
    pub fn custom_snippet_count(&self) -> usize {
        self.config.snippets.len()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Snippet Trigger Detection
// ─────────────────────────────────────────────────────────────────────────────

/// Result of checking for a snippet trigger.
#[derive(Debug, Clone)]
pub struct SnippetMatch {
    /// Starting position of the trigger in the text (byte offset)
    pub start: usize,
    /// Ending position of the trigger in the text (byte offset)
    pub end: usize,
    /// The trigger text that was matched
    pub trigger: String,
    /// The expansion text to replace it with
    pub expansion: String,
}

/// Find a snippet trigger word immediately before the cursor.
///
/// This function looks backwards from the cursor position to find a word
/// that matches a snippet trigger. Words are delimited by whitespace.
///
/// # Arguments
///
/// * `text` - The text content
/// * `cursor_byte_pos` - The cursor position (byte offset)
/// * `manager` - The snippet manager to check for triggers
///
/// # Returns
///
/// `Some(SnippetMatch)` if a trigger was found, `None` otherwise.
pub fn find_trigger_at_cursor(
    text: &str,
    cursor_byte_pos: usize,
    manager: &SnippetManager,
) -> Option<SnippetMatch> {
    if !manager.is_enabled() {
        return None;
    }

    // Cursor must be within text bounds
    if cursor_byte_pos > text.len() {
        return None;
    }

    // Get the text before cursor
    let before_cursor = &text[..cursor_byte_pos];

    // Find the start of the current word (look backwards for whitespace)
    let word_start = before_cursor
        .rfind(|c: char| c.is_whitespace())
        .map(|pos| pos + 1) // Move past the whitespace character
        .unwrap_or(0); // If no whitespace found, word starts at beginning

    // Extract the word
    let word = &before_cursor[word_start..];

    // Skip empty words
    if word.is_empty() {
        return None;
    }

    // Check if it's a valid trigger
    if let Some(expansion) = manager.get_expansion(word) {
        Some(SnippetMatch {
            start: word_start,
            end: cursor_byte_pos,
            trigger: word.to_string(),
            expansion,
        })
    } else {
        None
    }
}

/// Apply a snippet expansion to text content.
///
/// Replaces the trigger text with the expansion text and returns the new content
/// and the new cursor position (at the end of the expansion).
///
/// # Arguments
///
/// * `text` - The original text content
/// * `snippet_match` - The snippet match to apply
///
/// # Returns
///
/// A tuple of (new_content, new_cursor_byte_pos).
pub fn apply_snippet(text: &str, snippet_match: &SnippetMatch) -> (String, usize) {
    let mut new_content = String::with_capacity(
        text.len() - (snippet_match.end - snippet_match.start) + snippet_match.expansion.len(),
    );

    // Add text before trigger
    new_content.push_str(&text[..snippet_match.start]);

    // Add expansion
    new_content.push_str(&snippet_match.expansion);

    // Calculate new cursor position (at end of expansion)
    let new_cursor = snippet_match.start + snippet_match.expansion.len();

    // Add text after trigger
    if snippet_match.end < text.len() {
        new_content.push_str(&text[snippet_match.end..]);
    }

    (new_content, new_cursor)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snippet_config_default() {
        let config = SnippetConfig::default();
        assert!(config.enabled);
        assert!(config.snippets.is_empty());
    }

    #[test]
    fn test_builtin_snippets() {
        // Test that built-in snippets return Some
        assert!(SnippetConfig::builtin_expansion(";date").is_some());
        assert!(SnippetConfig::builtin_expansion(";time").is_some());
        assert!(SnippetConfig::builtin_expansion(";datetime").is_some());
        assert!(SnippetConfig::builtin_expansion(";now").is_some());

        // Test that non-built-in returns None
        assert!(SnippetConfig::builtin_expansion("custom").is_none());
        assert!(SnippetConfig::builtin_expansion(";notreal").is_none());
    }

    #[test]
    fn test_builtin_date_format() {
        let expansion = SnippetConfig::builtin_expansion(";date").unwrap();
        // Should be in YYYY-MM-DD format
        assert_eq!(expansion.len(), 10);
        assert!(expansion.chars().nth(4) == Some('-'));
        assert!(expansion.chars().nth(7) == Some('-'));
    }

    #[test]
    fn test_builtin_time_format() {
        let expansion = SnippetConfig::builtin_expansion(";time").unwrap();
        // Should be in HH:MM format
        assert_eq!(expansion.len(), 5);
        assert!(expansion.chars().nth(2) == Some(':'));
    }

    #[test]
    fn test_custom_snippet() {
        let mut config = SnippetConfig::default();
        config.set_snippet("sig".to_string(), "Best regards,\nJohn".to_string());

        assert_eq!(
            config.get_expansion("sig"),
            Some("Best regards,\nJohn".to_string())
        );
    }

    #[test]
    fn test_custom_snippet_removal() {
        let mut config = SnippetConfig::default();
        config.set_snippet("test".to_string(), "Test expansion".to_string());

        let removed = config.remove_snippet("test");
        assert_eq!(removed, Some("Test expansion".to_string()));
        assert!(config.get_expansion("test").is_none());
    }

    #[test]
    fn test_disabled_snippets() {
        let mut config = SnippetConfig::default();
        config.enabled = false;
        config.set_snippet("test".to_string(), "expansion".to_string());

        // Should return None when disabled
        assert!(config.get_expansion("test").is_none());
        assert!(config.get_expansion(";date").is_none());
    }

    #[test]
    fn test_is_builtin() {
        assert!(SnippetConfig::is_builtin(";date"));
        assert!(SnippetConfig::is_builtin(";time"));
        assert!(SnippetConfig::is_builtin(";datetime"));
        assert!(SnippetConfig::is_builtin(";now"));
        assert!(!SnippetConfig::is_builtin("custom"));
        assert!(!SnippetConfig::is_builtin(";custom"));
    }

    #[test]
    fn test_find_trigger_at_cursor() {
        let manager = SnippetManager::new();

        // Test with built-in trigger
        let text = "Hello ;date";
        let cursor = text.len();
        let result = find_trigger_at_cursor(text, cursor, &manager);
        assert!(result.is_some());
        let m = result.unwrap();
        assert_eq!(m.trigger, ";date");
        assert_eq!(m.start, 6);
        assert_eq!(m.end, 11);

        // Test with text and trigger
        let text = "Start ;time more";
        let cursor = 11; // Just after ;time
        let result = find_trigger_at_cursor(text, cursor, &manager);
        assert!(result.is_some());
        assert_eq!(result.unwrap().trigger, ";time");

        // Test with non-trigger word
        let text = "Hello world";
        let cursor = text.len();
        let result = find_trigger_at_cursor(text, cursor, &manager);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_trigger_at_start_of_line() {
        let manager = SnippetManager::new();

        // Trigger at start of text
        let text = ";date";
        let cursor = text.len();
        let result = find_trigger_at_cursor(text, cursor, &manager);
        assert!(result.is_some());
        let m = result.unwrap();
        assert_eq!(m.start, 0);
        assert_eq!(m.end, 5);
    }

    #[test]
    fn test_apply_snippet() {
        let text = "Today is ;date and time is ;time.";
        let snippet = SnippetMatch {
            start: 9,
            end: 14,
            trigger: ";date".to_string(),
            expansion: "2026-01-16".to_string(),
        };

        let (new_text, new_cursor) = apply_snippet(text, &snippet);
        assert_eq!(new_text, "Today is 2026-01-16 and time is ;time.");
        assert_eq!(new_cursor, 19); // End of expansion
    }

    #[test]
    fn test_apply_snippet_at_end() {
        let text = "Current time: ;time";
        let snippet = SnippetMatch {
            start: 14,
            end: 19,
            trigger: ";time".to_string(),
            expansion: "12:30".to_string(),
        };

        let (new_text, new_cursor) = apply_snippet(text, &snippet);
        assert_eq!(new_text, "Current time: 12:30");
        assert_eq!(new_cursor, 19);
    }

    #[test]
    fn test_apply_multiline_snippet() {
        let text = "Hello sig";
        let snippet = SnippetMatch {
            start: 6,
            end: 9,
            trigger: "sig".to_string(),
            expansion: "Best regards,\nJohn".to_string(),
        };

        let (new_text, new_cursor) = apply_snippet(text, &snippet);
        assert_eq!(new_text, "Hello Best regards,\nJohn");
        assert_eq!(new_cursor, 24);
    }

    #[test]
    fn test_serialization() {
        let mut config = SnippetConfig::default();
        config.set_snippet("sig".to_string(), "Signature".to_string());
        config.enabled = true;

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: SnippetConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.enabled, config.enabled);
        assert_eq!(deserialized.snippets.len(), 1);
        assert_eq!(
            deserialized.snippets.get("sig"),
            Some(&"Signature".to_string())
        );
    }
}
