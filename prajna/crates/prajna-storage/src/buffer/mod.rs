//! L2 — Salience-Weighted Buffer Pool
//!
//! The buffer pool is the in-memory page cache that sits between the cognitive
//! engine and the block I/O layer. Every page access goes through the pool;
//! actual disk reads and writes only happen on cache misses and dirty evictions.
//!
//! # Eviction Policy
//!
//! Standard databases use LRU or Clock. Prajna uses **salience-weighted
//! eviction**: a frame's evictability is a function of both its recency and
//! the cognitive salience of the memory it contains.
//!
//! High-salience pages (identity kernels, strategic abstractions, frequently
//! reinforced episodes) stay in RAM longer than low-salience pages (raw event
//! logs, old session data) even if they were accessed less recently.
//!
//! Eviction score (higher = more evictable):
//!
//! ```text
//! score(frame) = age_secs / (cached_salience × tier_weight + ε)
//! ```
//!
//! Where `tier_weight` is 1.0 for tier 0 (hot) down to 0.01 for tier 6
//! (world models, essentially never evicted).
//!
//! # Current Status
//!
//! Phase 1 skeleton. The `insert` / `evict_one` / `get` interface is final;
//! the eviction policy is currently a simple unpinned-frame scan that will be
//! replaced with a salience-scored max-heap in Phase 1 Q3–Q4.

use std::collections::HashMap;

use crate::block::{PageId, RawPage};
use crate::error::StorageError;

// ── Configuration ────────────────────────────────────────────────────────────

/// Buffer pool construction parameters.
#[derive(Debug, Clone)]
pub struct BufferPoolConfig {
    /// Maximum number of page frames to hold in memory simultaneously.
    /// Each frame consumes `PAGE_SIZE` (16 KiB) of heap space.
    pub capacity: usize,

    /// Weight of cognitive salience in the eviction score (0.0–1.0).
    /// 0.0 → pure recency (LRU); 1.0 → pure salience.
    pub salience_weight: f32,
}

impl Default for BufferPoolConfig {
    fn default() -> Self {
        Self {
            capacity:        1024, // 16 MiB
            salience_weight: 0.5,
        }
    }
}

// ── Frame ────────────────────────────────────────────────────────────────────

/// A single buffer pool frame: one cached page plus its metadata.
pub struct Frame {
    pub page:             Box<RawPage>,
    /// Number of active callers holding a pin on this frame.
    /// A pinned frame must not be evicted.
    pub pin_count:        u32,
    /// True if the frame's page has been modified and not yet written back.
    pub dirty:            bool,
    /// Compression tier of the page's contents (0 = raw, 6 = world model).
    pub tier:             u8,
    /// Cached cognitive salience of the page; used for eviction scoring.
    /// Updated lazily when the salience engine runs.
    pub cached_salience:  f32,
    /// Unix timestamp (seconds) of the last access to this frame.
    pub last_accessed_at: i64,
}

impl Frame {
    /// Compute the eviction score for this frame at `now` (unix seconds).
    ///
    /// Higher score → more evictable. Pinned frames return `f32::NEG_INFINITY`
    /// and are never selected for eviction.
    pub fn eviction_score(&self, now: i64) -> f32 {
        if self.pin_count > 0 {
            return f32::NEG_INFINITY;
        }
        // +1 baseline ensures salience differentiates freshly-inserted frames
        // that were all accessed at the same second.
        let age_secs = (now - self.last_accessed_at).max(0) as f32 + 1.0;
        let tier_weight: f32 = match self.tier {
            0 => 1.00,
            1 => 0.80,
            2 => 0.50,
            3 => 0.30,
            4 => 0.10,
            5 => 0.03,
            _ => 0.01,
        };
        let salience_term = self.cached_salience * tier_weight + f32::EPSILON;
        age_secs / salience_term
    }
}

// ── BufferPool ───────────────────────────────────────────────────────────────

/// Salience-weighted in-memory page cache.
pub struct BufferPool {
    config: BufferPoolConfig,
    frames: HashMap<PageId, Frame>,
}

impl BufferPool {
    pub fn new(config: BufferPoolConfig) -> Self {
        Self {
            frames: HashMap::with_capacity(config.capacity),
            config,
        }
    }

    /// Number of frames the pool can hold.
    pub fn capacity(&self) -> usize {
        self.config.capacity
    }

    /// Number of frames currently in the pool.
    pub fn occupancy(&self) -> usize {
        self.frames.len()
    }

    pub fn is_full(&self) -> bool {
        self.frames.len() >= self.config.capacity
    }

    /// Return `true` if `page_id` is currently cached.
    pub fn contains(&self, page_id: PageId) -> bool {
        self.frames.contains_key(&page_id)
    }

    /// Borrow the frame for `page_id`, updating its last-accessed timestamp.
    pub fn get(&mut self, page_id: PageId) -> Option<&Frame> {
        if let Some(frame) = self.frames.get_mut(&page_id) {
            frame.last_accessed_at = now_unix();
            Some(frame)
        } else {
            None
        }
    }

    /// Mutably borrow the frame for `page_id`.
    pub fn get_mut(&mut self, page_id: PageId) -> Option<&mut Frame> {
        if let Some(frame) = self.frames.get_mut(&page_id) {
            frame.last_accessed_at = now_unix();
            Some(frame)
        } else {
            None
        }
    }

    /// Insert a freshly loaded page.
    ///
    /// If the pool is at capacity, one unpinned frame is evicted first.
    /// Returns the evicted `PageId` (if any) so the caller can write it
    /// back to disk if it was dirty.
    pub fn insert(
        &mut self,
        page_id:  PageId,
        page:     Box<RawPage>,
        salience: f32,
        tier:     u8,
    ) -> Result<Option<PageId>, StorageError> {
        let evicted = if self.is_full() {
            Some(self.evict_one()?)
        } else {
            None
        };

        self.frames.insert(
            page_id,
            Frame {
                page,
                pin_count:        0,
                dirty:            false,
                tier,
                cached_salience:  salience,
                last_accessed_at: now_unix(),
            },
        );

        Ok(evicted)
    }

    /// Pin a frame, preventing it from being evicted.
    pub fn pin(&mut self, page_id: PageId) {
        if let Some(f) = self.frames.get_mut(&page_id) {
            f.pin_count += 1;
        }
    }

    /// Unpin a frame, making it eligible for eviction again.
    pub fn unpin(&mut self, page_id: PageId) {
        if let Some(f) = self.frames.get_mut(&page_id) {
            f.pin_count = f.pin_count.saturating_sub(1);
        }
    }

    /// Mark a frame dirty (its page has been modified and needs a write-back).
    pub fn mark_dirty(&mut self, page_id: PageId) {
        if let Some(f) = self.frames.get_mut(&page_id) {
            f.dirty = true;
        }
    }

    /// Update the cached salience of a frame (called by the salience engine).
    pub fn update_salience(&mut self, page_id: PageId, salience: f32) {
        if let Some(f) = self.frames.get_mut(&page_id) {
            f.cached_salience = salience;
        }
    }

    /// Remove `page_id` from the pool unconditionally, returning its frame.
    pub fn remove(&mut self, page_id: PageId) -> Option<Frame> {
        self.frames.remove(&page_id)
    }

    /// Return all dirty, unpinned frames (for checkpoint / write-back).
    pub fn dirty_pages(&self) -> Vec<PageId> {
        self.frames
            .iter()
            .filter(|(_, f)| f.dirty && f.pin_count == 0)
            .map(|(id, _)| *id)
            .collect()
    }

    // ── Private ──────────────────────────────────────────────────────────────

    /// Select and remove the most-evictable unpinned frame.
    ///
    /// TODO (Phase 1 Q3): replace linear scan with a salience-scored max-heap.
    fn evict_one(&mut self) -> Result<PageId, StorageError> {
        let now = now_unix();

        let candidate = self
            .frames
            .iter()
            .filter(|(_, f)| f.pin_count == 0)
            .max_by(|(_, a), (_, b)| {
                a.eviction_score(now)
                    .partial_cmp(&b.eviction_score(now))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(id, _)| *id);

        candidate
            .map(|id| {
                self.frames.remove(&id);
                id
            })
            .ok_or(StorageError::BufferPoolExhausted)
    }
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::RawPage;

    fn pool(capacity: usize) -> BufferPool {
        BufferPool::new(BufferPoolConfig {
            capacity,
            ..Default::default()
        })
    }

    fn dummy_page() -> Box<RawPage> {
        RawPage::zeroed()
    }

    #[test]
    fn insert_and_get() {
        let mut p = pool(8);
        p.insert(0, dummy_page(), 0.5, 0).unwrap();
        assert!(p.contains(0));
        assert!(p.get(0).is_some());
    }

    #[test]
    fn evicts_when_full() {
        let mut p = pool(2);
        p.insert(0, dummy_page(), 0.1, 0).unwrap(); // low salience → evicted first
        p.insert(1, dummy_page(), 0.9, 0).unwrap(); // high salience
        let evicted = p.insert(2, dummy_page(), 0.5, 0).unwrap();
        assert!(evicted.is_some());
        // Page 0 should have been evicted (lower salience → higher eviction score).
        assert!(!p.contains(0));
        assert!(p.contains(1));
        assert!(p.contains(2));
    }

    #[test]
    fn pinned_frame_not_evicted() {
        let mut p = pool(1);
        p.insert(0, dummy_page(), 0.1, 0).unwrap();
        p.pin(0);
        let result = p.insert(1, dummy_page(), 0.5, 0);
        assert!(matches!(result, Err(StorageError::BufferPoolExhausted)));
    }

    #[test]
    fn dirty_pages_list() {
        let mut p = pool(4);
        p.insert(10, dummy_page(), 0.5, 0).unwrap();
        p.insert(11, dummy_page(), 0.5, 0).unwrap();
        p.mark_dirty(10);
        let dirty = p.dirty_pages();
        assert!(dirty.contains(&10));
        assert!(!dirty.contains(&11));
    }
}
