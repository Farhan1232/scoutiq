//! Compression detection and codecs (in-process only).

pub mod detector;
pub mod zstd;

pub use detector::{detect_at, scan_candidates, CompressionCandidate};
pub use zstd::{
    decompress_zstd_frame, decompress_zstd_frame_bounded, MAX_FRAME_IN, MAX_FRAME_OUT, ZSTD_MAGIC,
};
