//! L3 — Write-Ahead Log (WAL)
//!
//! The WAL provides crash-recovery durability for the Prajna storage engine.
//! Every page mutation must be logged here **before** the buffer pool flushes
//! the dirty page to the data file. On crash, `Wal::recover` replays those
//! records to restore the data file to a consistent state.
//!
//! # On-disk format
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────┐
//! │  File header (32 B, at offset 0)                        │
//! │    magic[4]  version[2]  pad[2]                         │
//! │    checkpoint_lsn[8]  checkpoint_off[8]                 │
//! │    header_crc[4]  reserved[4]                           │
//! ├──────────────────────────────────────────────────────────┤
//! │  Record 0                                               │
//! │    lsn[8]  kind[1]  pad[3]  payload_len[4]  (16 B)     │
//! │    payload[payload_len]                                  │
//! │    record_crc[4]                                        │
//! ├──────────────────────────────────────────────────────────┤
//! │  Record 1 …                                             │
//! └──────────────────────────────────────────────────────────┘
//! ```
//!
//! **PageWrite payload**: `page_id[8] · page_data[16384]` → 16392 bytes
//! **Checkpoint payload**: empty → 0 bytes
//!
//! # Crash safety
//!
//! A partial write at the tail of the file (caused by a crash mid-record)
//! is detected by a CRC mismatch. `Wal::recover` stops at the first such
//! record and treats everything after it as garbage — this is the standard
//! ARIES truncation rule.
//!
//! # Recovery (Phase 1)
//!
//! `Wal::recover` always scans from the beginning of the log. Phase 2 will
//! honour the checkpoint offset stored in the header to skip already-applied
//! history and enable WAL segment truncation.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::block::{PageId, RawPage, PAGE_SIZE};
use crate::error::StorageError;

// ── Public types ──────────────────────────────────────────────────────────────

/// Log Sequence Number — monotonically increasing record identifier.
pub type Lsn = u64;

/// Sentinel for an unset LSN (no checkpoint yet, invalid reference, etc.).
pub const INVALID_LSN: Lsn = u64::MAX;

/// A single page image recovered from the WAL during crash recovery.
pub struct RecoveredPage {
    /// LSN of the WAL record that last wrote this page image.
    pub lsn:     Lsn,
    pub page_id: PageId,
    pub page:    Box<RawPage>,
}

// ── Private constants ─────────────────────────────────────────────────────────

const WAL_MAGIC:    [u8; 4] = *b"PWL1";
const WAL_VERSION:  u16     = 1;
const HEADER_SIZE:  u64     = 32;

const KIND_PAGE_WRITE: u8 = 0;
const KIND_CHECKPOINT: u8 = 1;

// ── Wal ───────────────────────────────────────────────────────────────────────

/// Write-ahead log.
///
/// Open with [`Wal::create`] for a brand-new log or [`Wal::open`] to resume
/// an existing one. For crash recovery without constructing a `Wal`, call the
/// associated function [`Wal::recover`].
pub struct Wal {
    file:           File,
    path:           PathBuf,
    next_lsn:       Lsn,
    /// LSN of the last written checkpoint record. `INVALID_LSN` = none yet.
    checkpoint_lsn: Lsn,
    /// File offset at which the last checkpoint record starts.
    checkpoint_off: u64,
}

impl Wal {
    // ── Construction ─────────────────────────────────────────────────────────

    /// Create a brand-new WAL file at `path`.
    ///
    /// Returns an error if the file already exists.
    pub fn create(path: &Path) -> Result<Self, StorageError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(StorageError::Io)?;

        let mut wal = Self {
            file,
            path:           path.to_path_buf(),
            next_lsn:       0,
            checkpoint_lsn: INVALID_LSN,
            checkpoint_off: INVALID_LSN,
        };
        wal.write_header()?;
        Ok(wal)
    }

    /// Open an existing WAL file.
    ///
    /// Validates the header, then scans all records to find the highest valid
    /// LSN (handles the case where the engine crashed before flushing
    /// `next_lsn`). The file pointer is left at the end, ready for appending.
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(StorageError::Io)?;

        let (checkpoint_lsn, checkpoint_off) = Self::read_header_inner(&mut file, path)?;

        let next_lsn = Self::scan_max_lsn(&mut file)
            .map(|max| max.saturating_add(1))
            .unwrap_or(0);

        file.seek(SeekFrom::End(0)).map_err(StorageError::Io)?;

        Ok(Self {
            file,
            path: path.to_path_buf(),
            next_lsn,
            checkpoint_lsn,
            checkpoint_off,
        })
    }

    // ── Writing ──────────────────────────────────────────────────────────────

    /// Log a full page image for `page_id`.
    ///
    /// Returns the LSN assigned to this record. The caller should store the
    /// returned LSN in the page header's `lsn` field before flushing the page
    /// to the data file.
    ///
    /// Call [`Wal::sync`] after this (and before the data-file write) to
    /// guarantee that the record is durable.
    pub fn log_page_write(
        &mut self,
        page_id: PageId,
        page:    &RawPage,
    ) -> Result<Lsn, StorageError> {
        let lsn        = self.next_lsn;
        self.next_lsn += 1;

        let payload_len = (8 + PAGE_SIZE) as u32;
        let prefix      = Self::build_prefix(lsn, KIND_PAGE_WRITE, payload_len);
        let id_bytes    = page_id.to_le_bytes();

        let crc = {
            let h = crc32c::crc32c(&prefix);
            let h = crc32c::crc32c_append(h, &id_bytes);
            crc32c::crc32c_append(h, &page.data)
        };

        self.file.seek(SeekFrom::End(0)).map_err(StorageError::Io)?;
        self.file.write_all(&prefix).map_err(StorageError::Io)?;
        self.file.write_all(&id_bytes).map_err(StorageError::Io)?;
        self.file.write_all(&page.data).map_err(StorageError::Io)?;
        self.file.write_all(&crc.to_le_bytes()).map_err(StorageError::Io)?;

        Ok(lsn)
    }

    /// Write a checkpoint record and update the file header.
    ///
    /// The caller must flush all dirty pages to the data file **before**
    /// calling this so that recovery can safely start from this checkpoint.
    pub fn checkpoint(&mut self) -> Result<Lsn, StorageError> {
        let lsn        = self.next_lsn;
        self.next_lsn += 1;

        let prefix = Self::build_prefix(lsn, KIND_CHECKPOINT, 0);
        let crc    = crc32c::crc32c(&prefix);

        let off = self.file.seek(SeekFrom::End(0)).map_err(StorageError::Io)?;
        self.file.write_all(&prefix).map_err(StorageError::Io)?;
        self.file.write_all(&crc.to_le_bytes()).map_err(StorageError::Io)?;

        self.checkpoint_lsn = lsn;
        self.checkpoint_off = off;
        self.write_header()?;

        Ok(lsn)
    }

    /// Flush all buffered WAL writes to durable storage (`fsync`).
    pub fn sync(&self) -> Result<(), StorageError> {
        self.file.sync_all().map_err(StorageError::Io)
    }

    /// LSN that will be assigned to the next record.
    pub fn next_lsn(&self) -> Lsn {
        self.next_lsn
    }

    /// LSN of the last written checkpoint. `INVALID_LSN` means no checkpoint.
    pub fn checkpoint_lsn(&self) -> Lsn {
        self.checkpoint_lsn
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    // ── Recovery ─────────────────────────────────────────────────────────────

    /// Scan the WAL at `path` and return the last image of every page that
    /// was written since the beginning of the log.
    ///
    /// Records are returned sorted by LSN. The caller applies them to the
    /// data file in that order to reconstruct a consistent post-crash state.
    ///
    /// A CRC mismatch at the tail of the file is treated as a crash-truncated
    /// record (not an error) — scanning stops there.
    pub fn recover(path: &Path) -> Result<Vec<RecoveredPage>, StorageError> {
        let mut file = OpenOptions::new()
            .read(true)
            .open(path)
            .map_err(StorageError::Io)?;

        Self::read_header_inner(&mut file, path)?;

        file.seek(SeekFrom::Start(HEADER_SIZE)).map_err(StorageError::Io)?;

        let mut pages: HashMap<PageId, (Lsn, Box<RawPage>)> = HashMap::new();

        loop {
            match Self::read_next_record(&mut file) {
                Ok((lsn, KIND_PAGE_WRITE, payload)) => {
                    if payload.len() < 8 + PAGE_SIZE {
                        return Err(StorageError::CorruptedFile {
                            path:   path.to_path_buf(),
                            reason: format!("WAL PageWrite LSN {lsn}: payload too short"),
                        });
                    }
                    let page_id = u64::from_le_bytes(payload[0..8].try_into().unwrap());
                    let mut page = RawPage::zeroed();
                    page.data.copy_from_slice(&payload[8..8 + PAGE_SIZE]);
                    pages.insert(page_id, (lsn, page));
                }
                Ok((_, KIND_CHECKPOINT, _)) => {}
                Ok((lsn, kind, _)) => {
                    return Err(StorageError::CorruptedFile {
                        path:   path.to_path_buf(),
                        reason: format!("WAL LSN {lsn}: unknown record kind {kind}"),
                    });
                }
                Err(RecordReadError::Eof | RecordReadError::Truncated) => break,
                Err(RecordReadError::Io(e)) => return Err(e),
            }
        }

        let mut result: Vec<RecoveredPage> = pages
            .into_iter()
            .map(|(page_id, (lsn, page))| RecoveredPage { lsn, page_id, page })
            .collect();

        result.sort_unstable_by_key(|r| r.lsn);
        Ok(result)
    }

    // ── Private ──────────────────────────────────────────────────────────────

    /// Build the 16-byte record prefix: `lsn[8] · kind[1] · pad[3] · payload_len[4]`.
    fn build_prefix(lsn: Lsn, kind: u8, payload_len: u32) -> [u8; 16] {
        let mut p = [0u8; 16];
        p[0..8].copy_from_slice(&lsn.to_le_bytes());
        p[8] = kind;
        // p[9..12] = zero pad
        p[12..16].copy_from_slice(&payload_len.to_le_bytes());
        p
    }

    /// Write (or overwrite) the 32-byte file header at offset 0.
    ///
    /// Leaves the file pointer at the end so subsequent appends are correct.
    fn write_header(&mut self) -> Result<(), StorageError> {
        let mut buf = [0u8; HEADER_SIZE as usize];
        buf[0..4].copy_from_slice(&WAL_MAGIC);
        buf[4..6].copy_from_slice(&WAL_VERSION.to_le_bytes());
        // buf[6..8] = zero pad
        buf[8..16].copy_from_slice(&self.checkpoint_lsn.to_le_bytes());
        buf[16..24].copy_from_slice(&self.checkpoint_off.to_le_bytes());
        let crc = crc32c::crc32c(&buf[0..24]);
        buf[24..28].copy_from_slice(&crc.to_le_bytes());
        // buf[28..32] = zero reserved

        self.file.seek(SeekFrom::Start(0)).map_err(StorageError::Io)?;
        self.file.write_all(&buf).map_err(StorageError::Io)?;
        self.file.seek(SeekFrom::End(0)).map_err(StorageError::Io)?;
        Ok(())
    }

    /// Read and validate the 32-byte file header.
    /// Returns `(checkpoint_lsn, checkpoint_off)`.
    fn read_header_inner(
        file: &mut File,
        path: &Path,
    ) -> Result<(Lsn, u64), StorageError> {
        let mut buf = [0u8; HEADER_SIZE as usize];
        file.seek(SeekFrom::Start(0)).map_err(StorageError::Io)?;
        file.read_exact(&mut buf).map_err(|e| StorageError::CorruptedFile {
            path:   path.to_path_buf(),
            reason: format!("cannot read WAL header: {e}"),
        })?;

        if buf[0..4] != WAL_MAGIC {
            return Err(StorageError::CorruptedFile {
                path:   path.to_path_buf(),
                reason: format!("invalid WAL magic: {:?}", &buf[0..4]),
            });
        }

        let stored   = u32::from_le_bytes(buf[24..28].try_into().unwrap());
        let computed = crc32c::crc32c(&buf[0..24]);
        if stored != computed {
            return Err(StorageError::CorruptedFile {
                path:   path.to_path_buf(),
                reason: format!(
                    "WAL header CRC mismatch: stored={stored:#010x}, computed={computed:#010x}"
                ),
            });
        }

        let checkpoint_lsn = u64::from_le_bytes(buf[8..16].try_into().unwrap());
        let checkpoint_off = u64::from_le_bytes(buf[16..24].try_into().unwrap());

        Ok((checkpoint_lsn, checkpoint_off))
    }

    /// Scan all valid records from `HEADER_SIZE` and return the maximum LSN
    /// seen. Stops at the first CRC error or EOF (crash-truncation boundary).
    fn scan_max_lsn(file: &mut File) -> Option<Lsn> {
        file.seek(SeekFrom::Start(HEADER_SIZE)).ok()?;
        let mut max: Option<Lsn> = None;
        loop {
            match Self::read_next_record(file) {
                Ok((lsn, _, _)) => {
                    max = Some(max.map_or(lsn, |m: Lsn| m.max(lsn)));
                }
                Err(_) => break,
            }
        }
        max
    }

    /// Read one complete record from the current file position.
    ///
    /// Returns `(lsn, kind, payload)` on success. On EOF before the prefix,
    /// returns `Err(Eof)`. On truncation (partial read or CRC mismatch),
    /// returns `Err(Truncated)`. On real I/O errors, returns `Err(Io(_))`.
    fn read_next_record(
        file: &mut File,
    ) -> Result<(Lsn, u8, Vec<u8>), RecordReadError> {
        // Read 16-byte prefix.
        let mut prefix = [0u8; 16];
        match file.read_exact(&mut prefix) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(RecordReadError::Eof);
            }
            Err(e) => return Err(RecordReadError::Io(StorageError::Io(e))),
        }

        let lsn         = u64::from_le_bytes(prefix[0..8].try_into().unwrap());
        let kind        = prefix[8];
        let payload_len = u32::from_le_bytes(prefix[12..16].try_into().unwrap()) as usize;

        // Read payload.
        let mut payload = vec![0u8; payload_len];
        file.read_exact(&mut payload).map_err(|e| {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                RecordReadError::Truncated
            } else {
                RecordReadError::Io(StorageError::Io(e))
            }
        })?;

        // Read + verify CRC.
        let mut crc_buf = [0u8; 4];
        file.read_exact(&mut crc_buf).map_err(|e| {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                RecordReadError::Truncated
            } else {
                RecordReadError::Io(StorageError::Io(e))
            }
        })?;

        let stored   = u32::from_le_bytes(crc_buf);
        let computed = crc32c::crc32c_append(crc32c::crc32c(&prefix), &payload);
        if stored != computed {
            return Err(RecordReadError::Truncated);
        }

        Ok((lsn, kind, payload))
    }
}

// ── Internal error type for record reads ─────────────────────────────────────

enum RecordReadError {
    /// Clean EOF before the start of a record.
    Eof,
    /// Partial record or CRC mismatch — crash truncation boundary.
    Truncated,
    /// Genuine I/O failure.
    Io(StorageError),
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::RawPage;

    /// Helper: create a WAL at a fresh temp path, return (path, wal).
    fn fresh(suffix: &str) -> (PathBuf, Wal) {
        let dir  = std::env::temp_dir();
        let path = dir.join(format!("prajna_wal_test_{suffix}_{}.wal",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()));
        let wal = Wal::create(&path).unwrap();
        (path, wal)
    }

    fn page_with_byte(byte: u8) -> Box<RawPage> {
        let mut p = RawPage::zeroed();
        p.data[100] = byte;
        p
    }

    #[test]
    fn create_starts_at_lsn_zero() {
        let (path, wal) = fresh("create");
        assert_eq!(wal.next_lsn(), 0);
        assert_eq!(wal.checkpoint_lsn(), INVALID_LSN);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn log_page_write_returns_sequential_lsns() {
        let (path, mut wal) = fresh("seq");
        let lsn0 = wal.log_page_write(0, &page_with_byte(0xAA)).unwrap();
        let lsn1 = wal.log_page_write(1, &page_with_byte(0xBB)).unwrap();
        assert_eq!(lsn0, 0);
        assert_eq!(lsn1, 1);
        assert_eq!(wal.next_lsn(), 2);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn recover_empty_wal_returns_no_pages() {
        let (path, _wal) = fresh("empty");
        let recovered = Wal::recover(&path).unwrap();
        assert!(recovered.is_empty());
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn recover_single_page_write() {
        let (path, mut wal) = fresh("single");
        let mut page = RawPage::zeroed();
        page.data[42] = 0xDE;
        page.data[43] = 0xAD;
        wal.log_page_write(7, &page).unwrap();
        drop(wal);

        let recovered = Wal::recover(&path).unwrap();
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].page_id, 7);
        assert_eq!(recovered[0].page.data[42], 0xDE);
        assert_eq!(recovered[0].page.data[43], 0xAD);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn recover_last_write_wins_for_same_page() {
        let (path, mut wal) = fresh("overwrite");
        let mut page_v1 = RawPage::zeroed();
        page_v1.data[10] = 0x01;
        let mut page_v2 = RawPage::zeroed();
        page_v2.data[10] = 0x02;

        wal.log_page_write(5, &page_v1).unwrap();
        wal.log_page_write(5, &page_v2).unwrap();
        drop(wal);

        let recovered = Wal::recover(&path).unwrap();
        assert_eq!(recovered.len(), 1, "only one entry per page_id");
        assert_eq!(recovered[0].page.data[10], 0x02, "latest write wins");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn recover_multiple_pages() {
        let (path, mut wal) = fresh("multi");
        for id in 0..4u64 {
            wal.log_page_write(id, &page_with_byte(id as u8)).unwrap();
        }
        drop(wal);

        let recovered = Wal::recover(&path).unwrap();
        assert_eq!(recovered.len(), 4);

        let mut ids: Vec<PageId> = recovered.iter().map(|r| r.page_id).collect();
        ids.sort_unstable();
        assert_eq!(ids, [0, 1, 2, 3]);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn recover_result_is_sorted_by_lsn() {
        let (path, mut wal) = fresh("sorted");
        wal.log_page_write(10, &page_with_byte(1)).unwrap();
        wal.log_page_write(20, &page_with_byte(2)).unwrap();
        wal.log_page_write(30, &page_with_byte(3)).unwrap();
        drop(wal);

        let recovered = Wal::recover(&path).unwrap();
        let lsns: Vec<Lsn> = recovered.iter().map(|r| r.lsn).collect();
        let mut sorted = lsns.clone();
        sorted.sort_unstable();
        assert_eq!(lsns, sorted, "recovery result must be sorted by LSN");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn checkpoint_updates_header_and_lsn() {
        let (path, mut wal) = fresh("ckpt");
        wal.log_page_write(0, &page_with_byte(0)).unwrap();
        let ckpt_lsn = wal.checkpoint().unwrap();
        assert_eq!(ckpt_lsn, 1); // lsn 0 was the page write
        assert_eq!(wal.checkpoint_lsn(), 1);
        drop(wal);

        // Reopen and verify checkpoint_lsn survived.
        let wal2 = Wal::open(&path).unwrap();
        assert_eq!(wal2.checkpoint_lsn(), 1);
        assert_eq!(wal2.next_lsn(), 2); // 0 + 1 = two records used

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn open_resumes_next_lsn_correctly() {
        let (path, mut wal) = fresh("resume");
        wal.log_page_write(0, &page_with_byte(1)).unwrap();
        wal.log_page_write(1, &page_with_byte(2)).unwrap();
        wal.log_page_write(2, &page_with_byte(3)).unwrap();
        drop(wal);

        let wal2 = Wal::open(&path).unwrap();
        // Three records written (LSNs 0, 1, 2) → next should be 3.
        assert_eq!(wal2.next_lsn(), 3);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn crash_truncated_tail_is_silently_ignored() {
        let (path, mut wal) = fresh("truncate");
        wal.log_page_write(0, &page_with_byte(0xAA)).unwrap();
        drop(wal);

        // Append garbage to simulate a crash mid-write.
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        file.write_all(&[0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x00, 0x00, 0x01]).unwrap();
        drop(file);

        // Recovery should succeed and return only the valid record.
        let recovered = Wal::recover(&path).unwrap();
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].page.data[100], 0xAA);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn sync_is_callable_without_error() {
        let (path, wal) = fresh("sync");
        wal.sync().unwrap();
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn recover_with_checkpoint_record_included() {
        let (path, mut wal) = fresh("ckpt_recover");
        wal.log_page_write(0, &page_with_byte(0x11)).unwrap();
        wal.checkpoint().unwrap();
        wal.log_page_write(1, &page_with_byte(0x22)).unwrap();
        drop(wal);

        let recovered = Wal::recover(&path).unwrap();
        // Checkpoint records carry no page data — only two page records.
        assert_eq!(recovered.len(), 2);
        std::fs::remove_file(&path).ok();
    }
}
