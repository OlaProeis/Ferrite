//! Runtime module snapshot for the Stats panel (loaded fonts, caches, terminals).

use crate::fonts;
use crate::markdown::mermaid::get_cache_snapshot;

/// Aggregated read-only runtime module state for the Stats panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeModulesInfo {
    /// Keys of lazily loaded CJK/complex-script font families (e.g. `CJK_JP`, `Arabic`).
    pub loaded_font_names: Vec<String>,
    /// Number of entries in the Mermaid blake3-keyed diagram cache.
    pub mermaid_cache_entries: usize,
    /// Maximum Mermaid cache entries before LRU eviction.
    pub mermaid_cache_max_entries: usize,
    /// Active terminal tab/session count.
    pub terminal_session_count: usize,
}

impl RuntimeModulesInfo {
    /// Collect current runtime module state. `terminal_session_count` comes from the terminal manager.
    pub fn collect(terminal_session_count: usize) -> Self {
        let cache = get_cache_snapshot();
        Self {
            loaded_font_names: fonts::get_loaded_runtime_font_names(),
            mermaid_cache_entries: cache.entry_count,
            mermaid_cache_max_entries: cache.max_entries,
            terminal_session_count,
        }
    }

    /// Format Mermaid cache occupancy for display (`entries / max`).
    pub fn format_mermaid_cache_size(&self) -> String {
        format!(
            "{} / {}",
            self.mermaid_cache_entries, self.mermaid_cache_max_entries
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::mermaid::MermaidCacheSnapshot;

    #[test]
    fn test_collect_sets_terminal_count() {
        let info = RuntimeModulesInfo::collect(3);
        assert_eq!(info.terminal_session_count, 3);
    }

    #[test]
    fn test_format_mermaid_cache_size() {
        let info = RuntimeModulesInfo {
            loaded_font_names: Vec::new(),
            mermaid_cache_entries: 2,
            mermaid_cache_max_entries: 50,
            terminal_session_count: 0,
        };
        assert_eq!(info.format_mermaid_cache_size(), "2 / 50");
    }

    #[test]
    fn test_runtime_modules_font_list_aggregation() {
        let info = RuntimeModulesInfo {
            loaded_font_names: vec!["CJK_JP".to_string(), "Arabic".to_string()],
            mermaid_cache_entries: 1,
            mermaid_cache_max_entries: 50,
            terminal_session_count: 1,
        };
        assert_eq!(info.loaded_font_names.len(), 2);
        assert!(info.loaded_font_names.contains(&"CJK_JP".to_string()));
        assert!(info.loaded_font_names.contains(&"Arabic".to_string()));
    }

    #[test]
    fn test_empty_cache_snapshot_defaults() {
        let cache = MermaidCacheSnapshot::default();
        assert_eq!(cache.entry_count, 0);
        assert_eq!(cache.max_entries, 0);
    }
}
