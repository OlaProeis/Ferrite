//! Mermaid diagram caching for performance optimization.
//!
//! This module provides caching for parsed AST and layout results to avoid
//! re-parsing and re-laying-out unchanged diagrams on every frame.
//!
//! # Cache Key
//!
//! The cache key includes:
//! - Source code hash (blake3)
//! - Font size (rounded to avoid cache misses from minor variations)
//! - Available width (rounded similarly)
//!
//! # Cache Invalidation
//!
//! Entries are invalidated when:
//! - Source code changes (hash mismatch)
//! - Font size changes significantly
//! - Available width changes significantly
//! - LRU eviction when cache is full

use std::collections::HashMap;

use super::flowchart::{Flowchart, FlowchartLayout};

// ─────────────────────────────────────────────────────────────────────────────
// Cache Key
// ─────────────────────────────────────────────────────────────────────────────

/// A cache key for Mermaid diagram rendering.
///
/// The key is a combination of the source hash and rendering parameters
/// that affect layout (font size, available width).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CacheKey {
    /// Blake3 hash of the source code
    source_hash: [u8; 32],
    /// Font size rounded to nearest 0.5
    font_size_x2: u32,
    /// Available width rounded to nearest 10 pixels
    width_div10: u32,
}

impl CacheKey {
    /// Create a new cache key from source and rendering parameters.
    pub fn new(source: &str, font_size: f32, available_width: f32) -> Self {
        let hash = blake3::hash(source.as_bytes());
        Self {
            source_hash: *hash.as_bytes(),
            // Round font_size to nearest 0.5 (multiply by 2, round, store as u32)
            font_size_x2: (font_size * 2.0).round() as u32,
            // Round width to nearest 10 pixels
            width_div10: (available_width / 10.0).round() as u32,
        }
    }

    /// Get the source hash for debugging.
    #[allow(dead_code)]
    pub fn source_hash(&self) -> &[u8; 32] {
        &self.source_hash
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Cached Flowchart
// ─────────────────────────────────────────────────────────────────────────────

/// Cached flowchart data including parsed AST and layout.
#[derive(Debug, Clone)]
pub struct CachedFlowchart {
    /// The parsed flowchart AST
    pub flowchart: Flowchart,
    /// The computed layout
    pub layout: FlowchartLayout,
    /// Last access time (for LRU eviction)
    last_access: std::time::Instant,
}

impl CachedFlowchart {
    /// Create a new cached flowchart entry.
    pub fn new(flowchart: Flowchart, layout: FlowchartLayout) -> Self {
        Self {
            flowchart,
            layout,
            last_access: std::time::Instant::now(),
        }
    }

    /// Update the last access time.
    pub fn touch(&mut self) {
        self.last_access = std::time::Instant::now();
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Cache Manager
// ─────────────────────────────────────────────────────────────────────────────

/// Maximum number of cached flowcharts.
const DEFAULT_MAX_ENTRIES: usize = 50;

/// Cache manager for Mermaid diagrams.
///
/// Provides O(1) lookup and LRU eviction when the cache is full.
///
/// # Example
///
/// ```ignore
/// use crate::markdown::mermaid::cache::{MermaidCacheManager, CacheKey};
///
/// let mut cache = MermaidCacheManager::new();
/// let key = CacheKey::new("flowchart TD\n  A --> B", 14.0, 400.0);
///
/// if let Some(cached) = cache.get_flowchart(&key) {
///     // Use cached flowchart and layout
/// } else {
///     // Parse and layout, then cache
///     let flowchart = parse_flowchart(source)?;
///     let layout = layout_flowchart(&flowchart, ...);
///     cache.insert_flowchart(key, flowchart, layout);
/// }
/// ```
#[derive(Debug)]
pub struct MermaidCacheManager {
    /// Flowchart cache: key -> cached data
    flowcharts: HashMap<CacheKey, CachedFlowchart>,
    /// Maximum number of entries
    max_entries: usize,
    /// Cache statistics
    stats: CacheStats,
}

/// Cache statistics for monitoring performance.
#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Number of evictions
    pub evictions: u64,
}

impl CacheStats {
    /// Get the hit rate as a percentage.
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f64 / total as f64) * 100.0
        }
    }
}

impl Default for MermaidCacheManager {
    fn default() -> Self {
        Self::new()
    }
}

impl MermaidCacheManager {
    /// Create a new cache manager with default capacity.
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_MAX_ENTRIES)
    }

    /// Create a new cache manager with specified capacity.
    pub fn with_capacity(max_entries: usize) -> Self {
        Self {
            flowcharts: HashMap::with_capacity(max_entries),
            max_entries,
            stats: CacheStats::default(),
        }
    }

    /// Get a cached flowchart if available.
    ///
    /// Updates the last access time on hit.
    pub fn get_flowchart(&mut self, key: &CacheKey) -> Option<&CachedFlowchart> {
        if let Some(entry) = self.flowcharts.get_mut(key) {
            entry.touch();
            self.stats.hits += 1;
            // Return immutable reference (need to re-borrow)
            self.flowcharts.get(key)
        } else {
            self.stats.misses += 1;
            None
        }
    }

    /// Insert a flowchart into the cache.
    ///
    /// If the cache is full, evicts the least recently used entry.
    pub fn insert_flowchart(
        &mut self,
        key: CacheKey,
        flowchart: Flowchart,
        layout: FlowchartLayout,
    ) {
        // Evict if at capacity
        if self.flowcharts.len() >= self.max_entries {
            self.evict_lru();
        }

        let cached = CachedFlowchart::new(flowchart, layout);
        self.flowcharts.insert(key, cached);
    }

    /// Clear the entire cache.
    ///
    /// Useful when theme or global settings change.
    pub fn clear(&mut self) {
        self.flowcharts.clear();
        log::debug!("Mermaid cache cleared");
    }

    /// Get the number of cached entries.
    pub fn len(&self) -> usize {
        self.flowcharts.len()
    }

    /// Maximum number of entries before LRU eviction.
    pub fn max_entries(&self) -> usize {
        self.max_entries
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.flowcharts.is_empty()
    }

    /// Get cache statistics.
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    /// Evict the least recently used entry.
    fn evict_lru(&mut self) {
        if let Some(oldest_key) = self
            .flowcharts
            .iter()
            .min_by_key(|(_, v)| v.last_access)
            .map(|(k, _)| *k)
        {
            self.flowcharts.remove(&oldest_key);
            self.stats.evictions += 1;
            log::trace!("Evicted LRU cache entry");
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_creation() {
        let key1 = CacheKey::new("flowchart TD\n  A --> B", 14.0, 400.0);
        let key2 = CacheKey::new("flowchart TD\n  A --> B", 14.0, 400.0);
        let key3 = CacheKey::new("flowchart TD\n  A --> C", 14.0, 400.0);

        // Same source should produce same key
        assert_eq!(key1, key2);

        // Different source should produce different key
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_cache_key_font_size_rounding() {
        // Font sizes within 0.25 should round to same value
        let key1 = CacheKey::new("test", 14.0, 400.0);
        let key2 = CacheKey::new("test", 14.2, 400.0);
        let key3 = CacheKey::new("test", 14.5, 400.0);

        // 14.0 and 14.2 round to 14.0 (x2 = 28)
        assert_eq!(key1.font_size_x2, key2.font_size_x2);
        // 14.5 rounds to 14.5 (x2 = 29)
        assert_ne!(key1.font_size_x2, key3.font_size_x2);
    }

    #[test]
    fn test_cache_key_width_rounding() {
        // Widths within 5 pixels should round to same value
        let key1 = CacheKey::new("test", 14.0, 400.0);
        let key2 = CacheKey::new("test", 14.0, 404.0);
        let key3 = CacheKey::new("test", 14.0, 410.0);

        // 400 and 404 round to 40
        assert_eq!(key1.width_div10, key2.width_div10);
        // 410 rounds to 41
        assert_ne!(key1.width_div10, key3.width_div10);
    }

    #[test]
    fn test_cache_manager_basic() {
        let mut cache = MermaidCacheManager::new();
        let key = CacheKey::new("flowchart TD\n  A --> B", 14.0, 400.0);

        // Cache should be empty initially
        assert!(cache.get_flowchart(&key).is_none());
        assert_eq!(cache.stats().misses, 1);

        // Insert a flowchart
        let flowchart = Flowchart::default();
        let layout = FlowchartLayout::default();
        cache.insert_flowchart(key, flowchart, layout);

        // Should now be cached
        assert!(cache.get_flowchart(&key).is_some());
        assert_eq!(cache.stats().hits, 1);
    }

    #[test]
    fn test_cache_manager_eviction() {
        let mut cache = MermaidCacheManager::with_capacity(2);

        let key1 = CacheKey::new("source1", 14.0, 400.0);
        let key2 = CacheKey::new("source2", 14.0, 400.0);
        let key3 = CacheKey::new("source3", 14.0, 400.0);

        // Insert two entries
        cache.insert_flowchart(key1, Flowchart::default(), FlowchartLayout::default());
        cache.insert_flowchart(key2, Flowchart::default(), FlowchartLayout::default());

        assert_eq!(cache.len(), 2);

        // Access key1 to make it more recent
        cache.get_flowchart(&key1);

        // Insert third entry - should evict key2 (LRU)
        cache.insert_flowchart(key3, Flowchart::default(), FlowchartLayout::default());

        assert_eq!(cache.len(), 2);
        assert!(cache.get_flowchart(&key1).is_some());
        assert!(cache.get_flowchart(&key3).is_some());
        // key2 should have been evicted
        assert_eq!(cache.stats().evictions, 1);
    }

    #[test]
    fn test_cache_manager_clear() {
        let mut cache = MermaidCacheManager::new();
        let key = CacheKey::new("test", 14.0, 400.0);

        cache.insert_flowchart(key, Flowchart::default(), FlowchartLayout::default());
        assert_eq!(cache.len(), 1);

        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_stats_hit_rate() {
        let mut stats = CacheStats::default();

        // No operations - 0% hit rate
        assert_eq!(stats.hit_rate(), 0.0);

        // 3 hits, 1 miss - 75% hit rate
        stats.hits = 3;
        stats.misses = 1;
        assert!((stats.hit_rate() - 75.0).abs() < 0.01);
    }
}
