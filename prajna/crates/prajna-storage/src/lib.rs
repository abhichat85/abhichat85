//! `prajna-storage` — the Prajna cognitive storage engine.
//!
//! This crate implements the lowest storage layers of the Prajna stack:
//!
//! | Layer | Module           | Responsibility                              |
//! |-------|------------------|---------------------------------------------|
//! | L0    | [`block`]        | File-backed page I/O with CRC32c checksums  |
//! | L1    | [`page`]         | Embedding-aware slotted page format         |
//! | L2    | [`buffer`]       | Salience-weighted in-memory page cache      |
//! | L3    | [`wal`]          | Write-ahead log for crash recovery          |
//! | —     | [`error`]        | Unified error type                          |
//!
//! Higher layers (free-space map, indexes, query engine, cognitive compiler)
//! will be added in subsequent phases. See `ARCHITECTURE.md` for the full map.

pub mod block;
pub mod buffer;
pub mod error;
pub mod page;
pub mod wal;
