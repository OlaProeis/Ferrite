//! Markdown AST caching for performance optimization.
//!
//! Caches parsed `MarkdownDocument` results keyed by a blake3 content hash
//! to avoid re-parsing unchanged markdown on every frame in the rendered view.
//!
//! Follows the same pattern as `mermaid::cache::MermaidCacheManager`.

use std::collections::HashMap;
use std::sync::Mutex;

use super::parser::MarkdownDocument;
use crate::error::Result;

// ─────────────────────────────────────────────────────────────────────────────
// Content Hash
// ─────────────────────────────────────────────────────────────────────────────

/// Blake3 hash of markdown content, used as the cache key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ContentHash([u8; 32]);

impl ContentHash {
    fn of(content: &str) -> Self {
        Self(*blake3::hash(content.as_bytes()).as_bytes())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Cached Entry
// ─────────────────────────────────────────────────────────────────────────────

struct CachedAst {
    doc: MarkdownDocument,
    last_access: std::time::Instant,
}

impl CachedAst {
    fn new(doc: MarkdownDocument) -> Self {
        Self {
            doc,
            last_access: std::time::Instant::now(),
        }
    }

    fn touch(&mut self) {
        self.last_access = std::time::Instant::now();
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Cache Manager
// ─────────────────────────────────────────────────────────────────────────────

const MAX_ENTRIES: usize = 32;

struct MarkdownAstCache {
    entries: HashMap<ContentHash, CachedAst>,
}

impl MarkdownAstCache {
    fn new() -> Self {
        Self {
            entries: HashMap::with_capacity(MAX_ENTRIES),
        }
    }

    fn get(&mut self, hash: &ContentHash) -> Option<MarkdownDocument> {
        if let Some(entry) = self.entries.get_mut(hash) {
            entry.touch();
            Some(entry.doc.clone())
        } else {
            None
        }
    }

    fn insert(&mut self, hash: ContentHash, doc: MarkdownDocument) {
        if self.entries.len() >= MAX_ENTRIES {
            self.evict_lru();
        }
        self.entries.insert(hash, CachedAst::new(doc));
    }

    fn evict_lru(&mut self) {
        if let Some(oldest) = self
            .entries
            .iter()
            .min_by_key(|(_, v)| v.last_access)
            .map(|(k, _)| *k)
        {
            self.entries.remove(&oldest);
        }
    }

    fn clear(&mut self) {
        self.entries.clear();
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Global cache singleton
// ─────────────────────────────────────────────────────────────────────────────

static AST_CACHE: Mutex<Option<MarkdownAstCache>> = Mutex::new(None);

fn with_cache<F, R>(f: F) -> R
where
    F: FnOnce(&mut MarkdownAstCache) -> R,
{
    let mut guard = AST_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let cache = guard.get_or_insert_with(MarkdownAstCache::new);
    f(cache)
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Return a cached `MarkdownDocument` for `content`, parsing only on cache miss.
pub fn get_or_parse(content: &str) -> Result<MarkdownDocument> {
    let hash = ContentHash::of(content);

    if let Some(doc) = with_cache(|c| c.get(&hash)) {
        return Ok(doc);
    }

    let doc = super::parser::parse_markdown(content)?;
    with_cache(|c| c.insert(hash, doc.clone()));
    Ok(doc)
}

/// Clear the AST cache (e.g. when global settings change).
#[allow(dead_code)]
pub fn clear_ast_cache() {
    if let Ok(mut guard) = AST_CACHE.lock() {
        if let Some(cache) = guard.as_mut() {
            cache.clear();
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Block Height Cache
// ─────────────────────────────────────────────────────────────────────────────
//
// Per-block height cache keyed by blake3(block_source) + render-parameter hash.
// When the user edits one block in a large document, the ViewportCullingState
// (whole-document layout) is invalidated, triggering a measurement pass.  This
// cache lets the measurement pass skip rendering every unchanged block — only
// blocks whose source actually changed need a full egui render to measure.

const MAX_BLOCK_HEIGHT_ENTRIES: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct BlockHeightKey {
    content_hash: ContentHash,
    render_params_hash: u64,
}

struct CachedBlockHeight {
    height: f32,
    last_access: std::time::Instant,
}

impl CachedBlockHeight {
    fn new(height: f32) -> Self {
        Self {
            height,
            last_access: std::time::Instant::now(),
        }
    }

    fn touch(&mut self) {
        self.last_access = std::time::Instant::now();
    }
}

struct BlockHeightCache {
    entries: HashMap<BlockHeightKey, CachedBlockHeight>,
}

impl BlockHeightCache {
    fn new() -> Self {
        Self {
            entries: HashMap::with_capacity(MAX_BLOCK_HEIGHT_ENTRIES),
        }
    }

    fn get(&mut self, key: &BlockHeightKey) -> Option<f32> {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.touch();
            Some(entry.height)
        } else {
            None
        }
    }

    fn insert(&mut self, key: BlockHeightKey, height: f32) {
        if self.entries.len() >= MAX_BLOCK_HEIGHT_ENTRIES {
            self.evict_lru();
        }
        self.entries.insert(key, CachedBlockHeight::new(height));
    }

    fn evict_lru(&mut self) {
        if let Some(oldest) = self
            .entries
            .iter()
            .min_by_key(|(_, v)| v.last_access)
            .map(|(k, _)| *k)
        {
            self.entries.remove(&oldest);
        }
    }

    fn clear(&mut self) {
        self.entries.clear();
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Block Height Cache – global singleton
// ─────────────────────────────────────────────────────────────────────────────

static BLOCK_HEIGHT_CACHE: Mutex<Option<BlockHeightCache>> = Mutex::new(None);

fn with_block_cache<F, R>(f: F) -> R
where
    F: FnOnce(&mut BlockHeightCache) -> R,
{
    let mut guard = BLOCK_HEIGHT_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let cache = guard.get_or_insert_with(BlockHeightCache::new);
    f(cache)
}

// ─────────────────────────────────────────────────────────────────────────────
// Block Height Cache – public API
// ─────────────────────────────────────────────────────────────────────────────

/// Hash rendering parameters that affect block heights (width, font size).
/// Callers pass this alongside the block source to form a complete cache key.
pub fn render_params_hash(available_width: f32, font_size: f32) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    (available_width as u32).hash(&mut h);
    ((font_size * 100.0) as u32).hash(&mut h);
    h.finish()
}

/// Look up a cached block height by source content and render parameters.
pub fn get_block_height(block_source: &str, render_params: u64) -> Option<f32> {
    let key = BlockHeightKey {
        content_hash: ContentHash::of(block_source),
        render_params_hash: render_params,
    };
    with_block_cache(|c| c.get(&key))
}

/// Store a measured block height.
pub fn insert_block_height(block_source: &str, render_params: u64, height: f32) {
    let key = BlockHeightKey {
        content_hash: ContentHash::of(block_source),
        render_params_hash: render_params,
    };
    with_block_cache(|c| c.insert(key, height));
}

/// Clear the block height cache (e.g. when global rendering settings change).
#[allow(dead_code)]
pub fn clear_block_height_cache() {
    if let Ok(mut guard) = BLOCK_HEIGHT_CACHE.lock() {
        if let Some(cache) = guard.as_mut() {
            cache.clear();
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
pub(crate) fn block_height_cache_test_lock() -> std::sync::MutexGuard<'static, ()> {
    use std::sync::Mutex;
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_content_returns_cached_doc() {
        let md = "# Hello\n\nWorld";
        let doc1 = get_or_parse(md).unwrap();
        let doc2 = get_or_parse(md).unwrap();
        assert_eq!(doc1.root.children.len(), doc2.root.children.len());
    }

    #[test]
    fn different_content_parses_separately() {
        let doc_a = get_or_parse("# A").unwrap();
        let doc_b = get_or_parse("# B").unwrap();
        assert_eq!(doc_a.root.children.len(), 1);
        assert_eq!(doc_b.root.children.len(), 1);
    }

    #[test]
    fn content_hash_deterministic() {
        let h1 = ContentHash::of("hello");
        let h2 = ContentHash::of("hello");
        let h3 = ContentHash::of("world");
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
    }

    #[test]
    fn clear_cache_works() {
        let _ = get_or_parse("# test clear").unwrap();
        clear_ast_cache();
        // After clear, next call should re-parse (no panic)
        let _ = get_or_parse("# test clear").unwrap();
    }

    // ── Block height cache tests ─────────────────────────────────────────

    #[test]
    fn block_height_cache_hit() {
        let _lock = block_height_cache_test_lock();
        const SRC: &str = "block-height-cache-hit\n";
        clear_block_height_cache();
        let rp = render_params_hash(800.0, 14.0);
        insert_block_height(SRC, rp, 42.0);
        assert_eq!(get_block_height(SRC, rp), Some(42.0));
    }

    #[test]
    fn block_height_cache_miss_different_content() {
        let _lock = block_height_cache_test_lock();
        clear_block_height_cache();
        let rp = render_params_hash(800.0, 14.0);
        insert_block_height("block-height-miss-a\n", rp, 42.0);
        assert_eq!(get_block_height("block-height-miss-b\n", rp), None);
    }

    #[test]
    fn block_height_cache_miss_different_width() {
        let _lock = block_height_cache_test_lock();
        const SRC: &str = "block-height-miss-width\n";
        clear_block_height_cache();
        let rp1 = render_params_hash(800.0, 14.0);
        let rp2 = render_params_hash(600.0, 14.0);
        insert_block_height(SRC, rp1, 42.0);
        assert_eq!(get_block_height(SRC, rp2), None);
    }

    #[test]
    fn block_height_cache_miss_different_font_size() {
        let _lock = block_height_cache_test_lock();
        const SRC: &str = "block-height-miss-font-size\n";
        clear_block_height_cache();
        let rp1 = render_params_hash(800.0, 14.0);
        let rp2 = render_params_hash(800.0, 18.0);
        insert_block_height(SRC, rp1, 42.0);
        assert_eq!(get_block_height(SRC, rp2), None);
    }

    #[test]
    fn block_height_lru_eviction() {
        let _lock = block_height_cache_test_lock();
        clear_block_height_cache();
        let rp = render_params_hash(800.0, 14.0);
        // Fill the cache past capacity
        for i in 0..MAX_BLOCK_HEIGHT_ENTRIES + 10 {
            insert_block_height(&format!("block-height-test-{}", i), rp, i as f32);
        }
        // Most recent entries should still be present
        let last = MAX_BLOCK_HEIGHT_ENTRIES + 9;
        assert!(get_block_height(&format!("block-height-test-{}", last), rp).is_some());
    }

    #[test]
    fn clear_block_height_cache_works() {
        let _lock = block_height_cache_test_lock();
        const SRC: &str = "block-height-clear-test\n";
        clear_block_height_cache();
        let rp = render_params_hash(800.0, 14.0);
        insert_block_height(SRC, rp, 10.0);
        clear_block_height_cache();
        assert_eq!(get_block_height(SRC, rp), None);
    }
}
