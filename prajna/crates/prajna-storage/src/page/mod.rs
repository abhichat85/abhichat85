//! L1 — Embedding-Aware Slotted Page Format
//!
//! Prajna's page format is purpose-built for cognitive workloads. The key
//! architectural decision: separate the embedding vectors from the record
//! payloads into two distinct regions within each page.
//!
//! # Page Layout
//!
//! ```text
//! ┌──────────────────────────────────────────────┐  offset 0
//! │  PageHeader (64 bytes)                        │
//! │  checksum · type · tier · lsn · slot_count…  │
//! ├──────────────────────────────────────────────┤  offset 64
//! │  Embedding Region (32-byte aligned)           │
//! │  vec[0]: f32 × dim                           │
//! │  vec[1]: f32 × dim                           │
//! │  …                                           │
//! │  (SIMD-friendly; all vectors contiguous)      │
//! ├──────────────────────────────────────────────┤  embedding_region_end
//! │  Record Region    ↓ grows forward            │
//! │  record[0], record[1], …                     │
//! │                                              │
//! │            free space                        │
//! │                                              │
//! │  … slot[1], slot[0]  ↑ grows backward        │
//! ├──────────────────────────────────────────────┤
//! │  Slot Directory (SlotEntry × slot_count)     │
//! └──────────────────────────────────────────────┘  offset PAGE_SIZE
//! ```
//!
//! # Why Two Separate Regions?
//!
//! A scan that only needs to rank memories by vector similarity should not
//! pay the cost of fetching record payloads. By isolating embeddings into a
//! contiguous, 32-byte-aligned region, we enable:
//!
//! - **SIMD dot-product over an entire page's embeddings** without touching
//!   the record region (AVX2: 8 × f32 per cycle).
//! - **Prefetching** the embedding region into L1 before entering the inner
//!   loop of the similarity scan.
//! - **Selective I/O**: a future compressed-column layout could store only
//!   the embedding region on fast NVMe and the payload region on slower media.

use std::mem;

use bytemuck::{Pod, Zeroable};
use static_assertions::const_assert_eq;

use crate::block::{PageId, PAGE_SIZE};

// ── PageHeader ───────────────────────────────────────────────────────────────

/// The 64-byte header that begins every Prajna page.
///
/// Fields are ordered to avoid padding under `repr(C)`, giving a naturally
/// aligned, 64-byte layout that is also `Pod` (safe to cast to/from bytes).
///
/// Offsets (bytes from page start):
///
/// ```text
///  0.. 4   checksum
///  4.. 5   page_type
///  5.. 6   compression_tier
///  6.. 8   flags
///  8..16   page_id
/// 16..24   lsn
/// 24..28   slot_count
/// 28..32   free_start
/// 32..36   free_end
/// 36..40   embedding_dim
/// 40..44   embedding_count
/// 44..48   embedding_region_end
/// 48..64   _reserved
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct PageHeader {
    /// CRC32c over bytes `4..PAGE_SIZE`. Written by `RawPage::seal()`.
    pub checksum:             u32,
    /// Discriminant from `PageType`. u8 to avoid padding waste.
    pub page_type:            u8,
    /// Memory compression tier (0 = raw event, 6 = world model).
    pub compression_tier:     u8,
    /// Reserved bitflags (e.g. dirty, overflow-chained, pinned).
    pub flags:                u16,
    /// The `PageId` of this page within its `BlockFile`.
    pub page_id:              u64,
    /// Log Sequence Number of the last WAL record that modified this page.
    pub lsn:                  u64,
    /// Number of live + dead slots in the slot directory.
    pub slot_count:           u32,
    /// Byte offset of the first free byte in the record region.
    pub free_start:           u32,
    /// Byte offset of the last byte of free space (exclusive; slot dir grows down from here).
    pub free_end:             u32,
    /// Dimensionality of stored embeddings. 0 when the page carries no vectors.
    pub embedding_dim:        u32,
    /// Number of embeddings written into the embedding region so far.
    pub embedding_count:      u32,
    /// Byte offset where the embedding region ends (= start of record region).
    pub embedding_region_end: u32,
    pub _reserved:            [u8; 16],
}

const_assert_eq!(mem::size_of::<PageHeader>(), 64);

pub const PAGE_HEADER_SIZE: usize = mem::size_of::<PageHeader>();

// ── PageType ─────────────────────────────────────────────────────────────────

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageType {
    Free            = 0,
    /// Cognitive data page: MemoryObjects with embedding vectors.
    CognitiveData   = 1,
    /// Internal node of the unified CogTree index.
    CogTreeInternal = 2,
    /// Leaf node of the unified CogTree index.
    CogTreeLeaf     = 3,
    /// Overflow page for large content blobs (TOAST-style chaining).
    Overflow        = 4,
    /// Write-ahead log segment page.
    Wal             = 5,
    /// Free Space Map bitmap page.
    FreeSpaceMap    = 6,
    /// Agent identity kernel page.
    IdentityKernel  = 7,
}

// ── SlotEntry ────────────────────────────────────────────────────────────────

/// A 12-byte slot directory entry that locates a record within the page.
///
/// The slot directory grows downward from `PAGE_SIZE` toward `free_end`.
/// Slot 0's entry is at `PAGE_SIZE - SLOT_ENTRY_SIZE`, slot 1's entry is
/// at `PAGE_SIZE - 2*SLOT_ENTRY_SIZE`, and so on.
///
/// A slot with `length == 0` is a dead (deleted) slot.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SlotEntry {
    /// Byte offset of the record from the beginning of the page.
    pub offset: u32,
    /// Byte length of the record. 0 means the slot is dead.
    pub length: u32,
    /// Index into the embedding region for this record's vector.
    /// `u32::MAX` (`INVALID_EMBEDDING_IDX`) means no embedding.
    pub embedding_idx: u32,
}

pub const SLOT_ENTRY_SIZE: usize = mem::size_of::<SlotEntry>();
const_assert_eq!(SLOT_ENTRY_SIZE, 12);

pub const INVALID_EMBEDDING_IDX: u32 = u32::MAX;

// ── Embedding region helpers ─────────────────────────────────────────────────

/// Alignment for the start of each embedding vector (AVX2 = 32 bytes).
pub const EMBEDDING_ALIGN: usize = 32;

/// Byte size of the embedding region for `count` vectors of `dim` f32 components,
/// rounded up to `EMBEDDING_ALIGN`.
pub fn embedding_region_size(dim: u32, count: u32) -> usize {
    let raw = dim as usize * count as usize * mem::size_of::<f32>();
    raw.next_multiple_of(EMBEDDING_ALIGN)
}

/// Maximum number of embeddings that fit in the embedding region of a fresh page,
/// given `dim` components and reserving `record_reserve` bytes for the record region.
pub fn max_embeddings(dim: u32, record_reserve: usize) -> u32 {
    let available = PAGE_SIZE
        .saturating_sub(PAGE_HEADER_SIZE)
        .saturating_sub(record_reserve);
    let bytes_per_vec = dim as usize * mem::size_of::<f32>();
    if bytes_per_vec == 0 {
        return 0;
    }
    (available / bytes_per_vec) as u32
}

// ── SlottedPage ──────────────────────────────────────────────────────────────

/// A mutable view over a `[u8; PAGE_SIZE]` buffer that enforces Prajna's
/// slotted page invariants.
///
/// `SlottedPage` borrows a page buffer mutably; the caller retains ownership
/// of the underlying bytes (typically via `RawPage`). All structural mutations
/// go through this type.
pub struct SlottedPage<'a> {
    data: &'a mut [u8; PAGE_SIZE],
}

impl<'a> SlottedPage<'a> {
    pub fn new(data: &'a mut [u8; PAGE_SIZE]) -> Self {
        Self { data }
    }

    // ── Header access ────────────────────────────────────────────────────────

    pub fn header(&self) -> &PageHeader {
        bytemuck::from_bytes(&self.data[..PAGE_HEADER_SIZE])
    }

    pub fn header_mut(&mut self) -> &mut PageHeader {
        bytemuck::from_bytes_mut(&mut self.data[..PAGE_HEADER_SIZE])
    }

    // ── Initialization ───────────────────────────────────────────────────────

    /// Zero-initialise the page and write a valid header.
    ///
    /// `max_emb` is the maximum number of embedding slots to reserve. Pass 0
    /// for pages that carry no vectors (e.g. WAL, FSM).
    pub fn init(
        &mut self,
        page_id:   PageId,
        page_type: PageType,
        tier:      u8,
        emb_dim:   u32,
        max_emb:   u32,
    ) {
        self.data.fill(0);

        let emb_region_bytes = embedding_region_size(emb_dim, max_emb);
        let emb_region_end   = PAGE_HEADER_SIZE + emb_region_bytes;

        let h = self.header_mut();
        h.page_type             = page_type as u8;
        h.compression_tier      = tier;
        h.page_id               = page_id;
        h.embedding_dim         = emb_dim;
        h.embedding_region_end  = emb_region_end as u32;
        h.free_start            = emb_region_end as u32;
        h.free_end              = PAGE_SIZE as u32;
    }

    // ── Embedding region ─────────────────────────────────────────────────────

    /// Return the embedding vector at index `idx`, or `None` if out of range.
    pub fn embedding(&self, idx: u32) -> Option<&[f32]> {
        let h = self.header();
        if idx >= h.embedding_count {
            return None;
        }
        let dim    = h.embedding_dim as usize;
        let offset = PAGE_HEADER_SIZE + idx as usize * dim * mem::size_of::<f32>();
        let bytes  = &self.data[offset..offset + dim * mem::size_of::<f32>()];
        Some(bytemuck::cast_slice(bytes))
    }

    /// Append an embedding vector into the region and return its index.
    ///
    /// Returns `None` if the embedding region is full or `emb.len() != dim`.
    pub fn push_embedding(&mut self, emb: &[f32]) -> Option<u32> {
        let h = self.header();
        if emb.len() != h.embedding_dim as usize {
            return None;
        }
        let idx    = h.embedding_count;
        let dim    = h.embedding_dim as usize;
        let offset = PAGE_HEADER_SIZE + idx as usize * dim * mem::size_of::<f32>();
        let end    = offset + dim * mem::size_of::<f32>();

        if end > h.embedding_region_end as usize {
            return None; // region exhausted
        }

        let bytes: &[u8] = bytemuck::cast_slice(emb);
        self.data[offset..end].copy_from_slice(bytes);
        self.header_mut().embedding_count = idx + 1;
        Some(idx)
    }

    /// Return an immutable slice over the entire embedding region,
    /// interpreted as a flat array of f32. Useful for SIMD batch scoring.
    pub fn embedding_region_flat(&self) -> &[f32] {
        let h   = self.header();
        let end = PAGE_HEADER_SIZE + h.embedding_count as usize
            * h.embedding_dim as usize
            * mem::size_of::<f32>();
        bytemuck::cast_slice(&self.data[PAGE_HEADER_SIZE..end])
    }

    // ── Slot directory ───────────────────────────────────────────────────────

    /// Return the `SlotEntry` at `slot_idx`, or `None` if out of range.
    pub fn slot(&self, slot_idx: u32) -> Option<SlotEntry> {
        let h = self.header();
        if slot_idx >= h.slot_count {
            return None;
        }
        let slot_end    = PAGE_SIZE - slot_idx as usize * SLOT_ENTRY_SIZE;
        let slot_start  = slot_end - SLOT_ENTRY_SIZE;
        Some(*bytemuck::from_bytes(&self.data[slot_start..slot_end]))
    }

    /// Return the record bytes for `slot_idx`, or `None` if deleted/absent.
    pub fn record(&self, slot_idx: u32) -> Option<&[u8]> {
        let slot = self.slot(slot_idx)?;
        if slot.length == 0 {
            return None; // dead slot
        }
        let off = slot.offset as usize;
        Some(&self.data[off..off + slot.length as usize])
    }

    // ── Mutation ─────────────────────────────────────────────────────────────

    /// Insert a record, optionally associating it with an embedding vector.
    ///
    /// If `embedding` is provided, it is appended to the embedding region and
    /// linked to the new slot via `embedding_idx`. If the record or embedding
    /// does not fit, returns `Err(StorageError::PageFull)`.
    ///
    /// Returns the slot index assigned to the inserted record.
    pub fn insert(
        &mut self,
        record:    &[u8],
        embedding: Option<&[f32]>,
    ) -> Result<u32, crate::error::StorageError> {
        let emb_idx = if let Some(emb) = embedding {
            self.push_embedding(emb)
                .ok_or(crate::error::StorageError::PageFull)?
        } else {
            INVALID_EMBEDDING_IDX
        };

        // Copy the values we need before any mutable borrow of self.data.
        let (slot_count, free_start, free_end) = {
            let h = self.header();
            (h.slot_count, h.free_start as usize, h.free_end as usize)
        };

        let new_free_start = free_start + record.len();
        let new_free_end   = free_end.saturating_sub(SLOT_ENTRY_SIZE);

        if new_free_start > new_free_end {
            return Err(crate::error::StorageError::PageFull);
        }

        // Write the record into the record region.
        self.data[free_start..free_start + record.len()].copy_from_slice(record);

        // Write the slot entry at the top of the slot directory.
        let slot = SlotEntry {
            offset:        free_start as u32,
            length:        record.len() as u32,
            embedding_idx: emb_idx,
        };
        let slot_bytes: &[u8] = bytemuck::bytes_of(&slot);
        self.data[new_free_end..new_free_end + SLOT_ENTRY_SIZE]
            .copy_from_slice(slot_bytes);

        // Update header.
        let h = self.header_mut();
        h.free_start = new_free_start as u32;
        h.free_end   = new_free_end as u32;
        h.slot_count = slot_count + 1;

        Ok(slot_count)
    }

    /// Mark slot `slot_idx` as dead (logical delete). Space is not reclaimed
    /// until a future compaction pass.
    pub fn delete(&mut self, slot_idx: u32) -> bool {
        if slot_idx >= self.header().slot_count {
            return false;
        }
        let slot_end   = PAGE_SIZE - slot_idx as usize * SLOT_ENTRY_SIZE;
        let slot_start = slot_end - SLOT_ENTRY_SIZE;
        // Zero the length field (bytes 4..8 of the SlotEntry).
        self.data[slot_start + 4..slot_start + 8].fill(0);
        true
    }

    // ── Space accounting ─────────────────────────────────────────────────────

    /// Bytes available for new records (not counting the embedding region).
    pub fn free_bytes(&self) -> usize {
        let h = self.header();
        (h.free_end as usize).saturating_sub(h.free_start as usize)
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::RawPage;

    fn fresh_data_page(emb_dim: u32, max_emb: u32) -> Box<RawPage> {
        let mut raw = RawPage::zeroed();
        let mut sp  = SlottedPage::new(&mut raw.data);
        sp.init(42, PageType::CognitiveData, 0, emb_dim, max_emb);
        raw
    }

    #[test]
    fn header_size_is_64() {
        assert_eq!(PAGE_HEADER_SIZE, 64);
    }

    #[test]
    fn insert_record_without_embedding() {
        let mut raw = fresh_data_page(0, 0);
        let mut sp  = SlottedPage::new(&mut raw.data);

        let data = b"hello cognitive world";
        let slot = sp.insert(data, None).unwrap();
        assert_eq!(slot, 0);
        assert_eq!(sp.header().slot_count, 1);
        assert_eq!(sp.record(0).unwrap(), data);
    }

    #[test]
    fn insert_multiple_records() {
        let mut raw = fresh_data_page(0, 0);
        let mut sp  = SlottedPage::new(&mut raw.data);

        sp.insert(b"memory one", None).unwrap();
        sp.insert(b"memory two", None).unwrap();
        sp.insert(b"memory three", None).unwrap();

        assert_eq!(sp.header().slot_count, 3);
        assert_eq!(sp.record(1).unwrap(), b"memory two");
    }

    #[test]
    fn insert_record_with_embedding() {
        let dim: u32 = 4;
        let mut raw  = fresh_data_page(dim, 8);
        let mut sp   = SlottedPage::new(&mut raw.data);

        let emb: Vec<f32> = vec![0.1, 0.2, 0.3, 0.4];
        let slot = sp.insert(b"episodic memory", Some(&emb)).unwrap();

        assert_eq!(slot, 0);
        let se = sp.slot(0).unwrap();
        assert_ne!(se.embedding_idx, INVALID_EMBEDDING_IDX);

        let stored = sp.embedding(se.embedding_idx).unwrap();
        for (a, b) in stored.iter().zip(emb.iter()) {
            assert!((a - b).abs() < 1e-7);
        }
    }

    #[test]
    fn embedding_region_is_contiguous() {
        let dim: u32 = 3;
        let mut raw  = fresh_data_page(dim, 4);
        let mut sp   = SlottedPage::new(&mut raw.data);

        let e0 = vec![1.0_f32, 2.0, 3.0];
        let e1 = vec![4.0_f32, 5.0, 6.0];
        sp.insert(b"r0", Some(&e0)).unwrap();
        sp.insert(b"r1", Some(&e1)).unwrap();

        let flat = sp.embedding_region_flat();
        assert_eq!(&flat[0..3], &[1.0, 2.0, 3.0]);
        assert_eq!(&flat[3..6], &[4.0, 5.0, 6.0]);
    }

    #[test]
    fn delete_marks_slot_dead() {
        let mut raw = fresh_data_page(0, 0);
        let mut sp  = SlottedPage::new(&mut raw.data);

        sp.insert(b"alive", None).unwrap();
        sp.insert(b"to be deleted", None).unwrap();
        sp.insert(b"also alive", None).unwrap();

        sp.delete(1);
        assert!(sp.record(1).is_none());
        assert!(sp.record(0).is_some());
        assert!(sp.record(2).is_some());
    }

    #[test]
    fn page_full_error() {
        // A page with zero embedding capacity and a tiny free region.
        let mut raw = RawPage::zeroed();
        let mut sp  = SlottedPage::new(&mut raw.data);
        // Reserve almost the entire page for the embedding region by setting
        // free_start near free_end directly (white-box test of the invariant).
        sp.init(1, PageType::CognitiveData, 0, 0, 0);

        // Insert records until the page is full.
        let record = vec![0u8; 1024];
        let mut count = 0;
        loop {
            match sp.insert(&record, None) {
                Ok(_)  => count += 1,
                Err(crate::error::StorageError::PageFull) => break,
                Err(e) => panic!("unexpected error: {e}"),
            }
            if count > PAGE_SIZE { panic!("infinite loop guard"); }
        }
        assert!(count > 0, "should have fit at least one record");
    }
}
