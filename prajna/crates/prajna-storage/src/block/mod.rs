//! L0 — Block I/O
//!
//! The lowest layer of the Prajna storage stack. Abstracts a file as a linear
//! address space of fixed-size pages, with CRC32c checksums on every page.
//!
//! Design notes:
//!
//! - Page size is 16 KB (vs PostgreSQL's 8 KB). The larger size accommodates
//!   the embedding region that every cognitive data page carries. At 1536-dim
//!   float32 (OpenAI ada-002 size), a single embedding is ~6 KB; a 16 KB page
//!   fits two embeddings plus record metadata comfortably.
//!
//! - The `RawPage` buffer is `#[repr(align(4096))]`. This alignment satisfies
//!   the kernel's O_DIRECT requirement (Linux: buffer must be aligned to the
//!   logical sector size, which is always ≤ 4096). Direct I/O will be enabled
//!   in a later phase to bypass the kernel page cache entirely, since Prajna
//!   manages its own buffer pool with salience-aware eviction.
//!
//! - Checksums use CRC32c, the same algorithm as PostgreSQL. It is hardware-
//!   accelerated on all modern x86_64 and ARM64 CPUs via the `crc32c` crate.
//!   The checksum field occupies bytes 0–3 of every page; the checksum is
//!   computed over bytes 4..PAGE_SIZE (excluding the field itself).

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use crate::error::StorageError;

// ── Constants ────────────────────────────────────────────────────────────────

/// On-disk page size: 16 KiB.
pub const PAGE_SIZE: usize = 16 * 1024;

pub const PAGE_SIZE_U64: u64 = PAGE_SIZE as u64;

/// Sentinel value for an unset or null page reference.
pub const INVALID_PAGE_ID: PageId = u64::MAX;

// ── Types ────────────────────────────────────────────────────────────────────

/// Logical address of a page within a `BlockFile`.
///
/// Page IDs are stable and never reused within a file; free-page recycling
/// is handled by the Free Space Map layer above.
pub type PageId = u64;

// ── RawPage ──────────────────────────────────────────────────────────────────

/// A fixed-size, heap-allocated page buffer.
///
/// The `align(4096)` attribute is load-bearing: it allows this buffer to be
/// used with O_DIRECT I/O, which requires the buffer address to be aligned to
/// the device's logical sector size (always a divisor of 4096).
#[repr(C, align(4096))]
#[derive(Debug)]
pub struct RawPage {
    pub data: [u8; PAGE_SIZE],
}

impl RawPage {
    /// Allocate a zeroed page on the heap with the required alignment.
    pub fn zeroed() -> Box<Self> {
        // SAFETY: all-zero bytes are a valid bit pattern for [u8; N].
        unsafe { Box::new_zeroed().assume_init() }
    }

    /// Compute the CRC32c checksum of the page payload (bytes 4..PAGE_SIZE).
    ///
    /// The first four bytes are the stored checksum field and are excluded from
    /// the computation so that the checksum can be written without invalidating
    /// itself.
    #[inline]
    pub fn compute_checksum(&self) -> u32 {
        crc32c::crc32c(&self.data[4..])
    }

    /// Read the checksum stored in bytes 0–3 (little-endian).
    #[inline]
    pub fn stored_checksum(&self) -> u32 {
        u32::from_le_bytes(self.data[0..4].try_into().unwrap())
    }

    /// Compute the current checksum and write it into bytes 0–3.
    pub fn seal(&mut self) {
        let cs = self.compute_checksum();
        self.data[0..4].copy_from_slice(&cs.to_le_bytes());
    }

    /// Return `true` if the stored checksum matches the computed checksum.
    pub fn verify(&self) -> bool {
        self.stored_checksum() == self.compute_checksum()
    }
}

// ── BlockFile ────────────────────────────────────────────────────────────────

/// A file-backed linear address space of `PAGE_SIZE`-byte pages.
///
/// `BlockFile` is the only component in Prajna that performs actual
/// filesystem I/O. Everything above it — the buffer pool, WAL, indexes,
/// cognitive engine — is pure in-memory logic that eventually calls
/// `read_page` and `write_page` here.
pub struct BlockFile {
    file:       File,
    path:       std::path::PathBuf,
    page_count: u64,
}

impl BlockFile {
    /// Open (or create) a block file at `path`.
    ///
    /// Returns an error if the file size is not a multiple of `PAGE_SIZE`,
    /// which would indicate truncation or corruption.
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .map_err(StorageError::Io)?;

        let len = file.metadata().map_err(StorageError::Io)?.len();

        if len % PAGE_SIZE_U64 != 0 {
            return Err(StorageError::CorruptedFile {
                path: path.to_path_buf(),
                reason: format!(
                    "file length {len} is not a multiple of page size {PAGE_SIZE}"
                ),
            });
        }

        Ok(Self {
            file,
            path: path.to_path_buf(),
            page_count: len / PAGE_SIZE_U64,
        })
    }

    /// Read the page at `page_id`, verifying its checksum.
    pub fn read_page(&mut self, page_id: PageId) -> Result<Box<RawPage>, StorageError> {
        if page_id >= self.page_count {
            return Err(StorageError::PageNotFound(page_id));
        }

        self.file
            .seek(SeekFrom::Start(page_id * PAGE_SIZE_U64))
            .map_err(StorageError::Io)?;

        let mut page = RawPage::zeroed();
        self.file
            .read_exact(&mut page.data)
            .map_err(StorageError::Io)?;

        if !page.verify() {
            return Err(StorageError::ChecksumMismatch {
                page_id,
                stored:   page.stored_checksum(),
                computed: page.compute_checksum(),
            });
        }

        Ok(page)
    }

    /// Write `page` to `page_id`.
    ///
    /// The caller must have called `page.seal()` to write a valid checksum
    /// before calling this method. `read_page` will reject pages with bad
    /// checksums, so forgetting to seal is caught on the next read.
    pub fn write_page(&mut self, page_id: PageId, page: &RawPage) -> Result<(), StorageError> {
        if page_id >= self.page_count {
            return Err(StorageError::PageNotFound(page_id));
        }

        self.file
            .seek(SeekFrom::Start(page_id * PAGE_SIZE_U64))
            .map_err(StorageError::Io)?;

        self.file.write_all(&page.data).map_err(StorageError::Io)?;

        Ok(())
    }

    /// Extend the file by one page, zero-filled, and return its new `PageId`.
    pub fn allocate_page(&mut self) -> Result<PageId, StorageError> {
        let page_id = self.page_count;

        self.file
            .seek(SeekFrom::Start(page_id * PAGE_SIZE_U64))
            .map_err(StorageError::Io)?;

        // Write a full zeroed page to extend the file atomically.
        self.file
            .write_all(&[0u8; PAGE_SIZE])
            .map_err(StorageError::Io)?;

        self.page_count += 1;
        Ok(page_id)
    }

    /// Flush all pending writes to durable storage (`fsync`).
    pub fn sync(&self) -> Result<(), StorageError> {
        self.file.sync_all().map_err(StorageError::Io)
    }

    pub fn page_count(&self) -> u64 {
        self.page_count
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn open_tmp() -> (NamedTempFile, BlockFile) {
        let tmp = NamedTempFile::new().unwrap();
        let bf  = BlockFile::open(tmp.path()).unwrap();
        (tmp, bf)
    }

    #[test]
    fn empty_file_has_zero_pages() {
        let (_tmp, bf) = open_tmp();
        assert_eq!(bf.page_count(), 0);
    }

    #[test]
    fn allocate_returns_sequential_ids() {
        let (_tmp, mut bf) = open_tmp();
        assert_eq!(bf.allocate_page().unwrap(), 0);
        assert_eq!(bf.allocate_page().unwrap(), 1);
        assert_eq!(bf.allocate_page().unwrap(), 2);
        assert_eq!(bf.page_count(), 3);
    }

    #[test]
    fn write_and_read_back() {
        let (_tmp, mut bf) = open_tmp();
        let id = bf.allocate_page().unwrap();

        let mut page = RawPage::zeroed();
        page.data[64]  = 0xDE;
        page.data[65]  = 0xAD;
        page.data[100] = 0xBE;
        page.seal();

        bf.write_page(id, &page).unwrap();

        let back = bf.read_page(id).unwrap();
        assert_eq!(back.data[64],  0xDE);
        assert_eq!(back.data[65],  0xAD);
        assert_eq!(back.data[100], 0xBE);
    }

    #[test]
    fn detects_checksum_mismatch() {
        let (_tmp, mut bf) = open_tmp();
        let id = bf.allocate_page().unwrap();

        let mut page = RawPage::zeroed();
        page.seal();
        // Corrupt one byte after sealing.
        page.data[200] = 0xFF;

        bf.write_page(id, &page).unwrap();

        let err = bf.read_page(id).unwrap_err();
        assert!(matches!(err, StorageError::ChecksumMismatch { .. }));
    }

    #[test]
    fn read_out_of_range_returns_not_found() {
        let (_tmp, mut bf) = open_tmp();
        let err = bf.read_page(99).unwrap_err();
        assert!(matches!(err, StorageError::PageNotFound(99)));
    }

    #[test]
    fn file_survives_reopen() {
        let tmp = NamedTempFile::new().unwrap();

        let written = {
            let mut bf = BlockFile::open(tmp.path()).unwrap();
            let id = bf.allocate_page().unwrap();
            let mut page = RawPage::zeroed();
            page.data[500] = 0x42;
            page.seal();
            bf.write_page(id, &page).unwrap();
            bf.sync().unwrap();
            id
        };

        // Re-open and verify the data survived.
        let mut bf = BlockFile::open(tmp.path()).unwrap();
        assert_eq!(bf.page_count(), 1);
        let back = bf.read_page(written).unwrap();
        assert_eq!(back.data[500], 0x42);
    }
}
