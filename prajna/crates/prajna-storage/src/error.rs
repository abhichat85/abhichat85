use std::path::PathBuf;
use thiserror::Error;

use crate::block::PageId;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("page {0} not found")]
    PageNotFound(PageId),

    #[error("checksum mismatch on page {page_id}: stored {stored:#010x}, computed {computed:#010x}")]
    ChecksumMismatch {
        page_id: PageId,
        stored:  u32,
        computed: u32,
    },

    #[error("corrupted file at {}: {reason}", path.display())]
    CorruptedFile { path: PathBuf, reason: String },

    #[error("page is full")]
    PageFull,

    #[error("all buffer pool frames are pinned; cannot evict")]
    BufferPoolExhausted,

    #[error("invalid argument: {0}")]
    InvalidArgument(String),
}
