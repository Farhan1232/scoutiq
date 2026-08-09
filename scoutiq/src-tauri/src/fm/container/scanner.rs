//! One streaming pass: index compression candidates without assuming block ends.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::fm::compression::detector::scan_candidates;
use crate::fm::container::reader::FmStreamReader;
use crate::fm::errors::FmError;
use crate::fm::types::{CompressedBlockInfo, CompressionType, ValidationStatus};

const CHUNK: usize = 256 * 1024;
const MAGIC_OVERLAP: usize = 4;
/// Soft cap so 1GB+ saves stay interactive; decoder still validates each candidate.
pub const MAX_BLOCK_CANDIDATES: usize = 128;

pub fn index_compression_candidates(
    path: &Path,
    cancel: Option<&AtomicBool>,
) -> Result<(Vec<CompressedBlockInfo>, u64), FmError> {
    let mut stream = FmStreamReader::open(path)?;
    let mut blocks = Vec::new();
    let mut carry = Vec::new();
    let mut absolute: u64 = 0;
    let mut bytes_processed = 0u64;
    let mut buf = vec![0u8; CHUNK];
    let mut seen = std::collections::BTreeSet::new();

    loop {
        if cancel.map(|c| c.load(Ordering::SeqCst)).unwrap_or(false) {
            return Err(FmError::cancelled());
        }
        let n = stream.read_chunk(&mut buf)?;
        if n == 0 {
            break;
        }
        bytes_processed += n as u64;

        let mut window = carry.clone();
        let carry_len = window.len();
        window.extend_from_slice(&buf[..n]);

        let start_from = carry_len.saturating_sub(MAGIC_OVERLAP);
        let base = absolute.saturating_sub(carry_len as u64);
        let locals = scan_candidates(&window[start_from..], MAX_BLOCK_CANDIDATES);

        for (rel, ctype) in locals {
            let abs = base + (start_from + rel) as u64;
            if !seen.insert(abs) {
                continue;
            }
            if blocks.len() >= MAX_BLOCK_CANDIDATES {
                break;
            }
            blocks.push(CompressedBlockInfo {
                index: blocks.len(),
                compression_type: ctype,
                file_offset: abs,
                compressed_size: None,
                decompressed_size: None,
                validation_status: ValidationStatus::Candidate,
                error: None,
                content_digest: None,
                signatures: Vec::new(),
                string_count: 0,
            });
        }

        let keep = MAGIC_OVERLAP.min(window.len());
        carry = window[window.len() - keep..].to_vec();
        absolute += n as u64;

        if blocks.len() >= MAX_BLOCK_CANDIDATES {
            break;
        }
    }

    // Prefer Zstandard candidates first for processing order (generic priority).
    blocks.sort_by_key(|b| {
        (
            match b.compression_type {
                CompressionType::Zstandard => 0,
                CompressionType::Gzip => 1,
                CompressionType::Zlib => 2,
                CompressionType::Zip => 3,
                _ => 9,
            },
            b.file_offset,
        )
    });
    for (i, b) in blocks.iter_mut().enumerate() {
        b.index = i;
    }

    Ok((blocks, bytes_processed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn indexes_zstd_candidates_without_full_read() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "scoutiq_idx_{}.fm",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let payload = b"index-test";
        let frame = zstd::encode_all(&payload[..], 1).unwrap();
        {
            let mut f = File::create(&path).unwrap();
            f.write_all(b"FMF\0\0\0\0").unwrap();
            f.write_all(&frame).unwrap();
            f.write_all(&[0u8; 2000]).unwrap();
        }
        let (blocks, bytes) = index_compression_candidates(&path, None).unwrap();
        assert!(bytes > 0);
        assert!(blocks
            .iter()
            .any(|b| b.compression_type == CompressionType::Zstandard));
        assert!(blocks
            .iter()
            .all(|b| b.validation_status == ValidationStatus::Candidate));
        let _ = std::fs::remove_file(&path);
    }
}
