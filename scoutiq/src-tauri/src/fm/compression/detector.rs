//! Generic compression signature detection.
//! Magics are candidates until a decoder validates them.

use crate::fm::types::CompressionType;

pub const ZSTD_MAGIC: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];
pub const GZIP_MAGIC: [u8; 2] = [0x1F, 0x8B];
pub const ZLIB_CMF_LOW: u8 = 0x78;
pub const ZIP_LOCAL: [u8; 4] = [0x50, 0x4B, 0x03, 0x04];
pub const ZIP_EOCD: [u8; 4] = [0x50, 0x4B, 0x05, 0x06];

#[derive(Debug, Clone)]
pub struct CompressionCandidate {
    pub offset: u64,
    pub compression_type: CompressionType,
}

/// Detect compression type at the start of `data` (absolute offset for reporting).
pub fn detect_at(data: &[u8], absolute_offset: u64) -> Option<CompressionCandidate> {
    if data.len() >= 4 && data[..4] == ZSTD_MAGIC {
        return Some(CompressionCandidate {
            offset: absolute_offset,
            compression_type: CompressionType::Zstandard,
        });
    }
    if data.len() >= 2 && data[..2] == GZIP_MAGIC {
        return Some(CompressionCandidate {
            offset: absolute_offset,
            compression_type: CompressionType::Gzip,
        });
    }
    if data.len() >= 4 && data[..4] == ZIP_LOCAL {
        return Some(CompressionCandidate {
            offset: absolute_offset,
            compression_type: CompressionType::Zip,
        });
    }
    if data.len() >= 4 && data[..4] == ZIP_EOCD {
        return Some(CompressionCandidate {
            offset: absolute_offset,
            compression_type: CompressionType::Zip,
        });
    }
    // zlib: CMF=0x78, common FLG values; treat as candidate only.
    if data.len() >= 2 && data[0] == ZLIB_CMF_LOW {
        let flg = data[1];
        if matches!(flg, 0x01 | 0x5E | 0x9C | 0xDA) {
            return Some(CompressionCandidate {
                offset: absolute_offset,
                compression_type: CompressionType::Zlib,
            });
        }
    }
    None
}

/// Scan a buffer for compression magics (relative offsets).
pub fn scan_candidates(data: &[u8], limit: usize) -> Vec<(usize, CompressionType)> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < data.len() && out.len() < limit {
        if let Some(c) = detect_at(&data[i..], i as u64) {
            out.push((i, c.compression_type));
            i += match c.compression_type {
                CompressionType::Zstandard | CompressionType::Zip => 4,
                CompressionType::Gzip | CompressionType::Zlib => 2,
                _ => 1,
            };
        } else {
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_zstd_magic() {
        let mut buf = vec![0u8; 16];
        buf[4..8].copy_from_slice(&ZSTD_MAGIC);
        let hits = scan_candidates(&buf, 10);
        assert!(hits
            .iter()
            .any(|(o, t)| *o == 4 && *t == CompressionType::Zstandard));
    }

    #[test]
    fn detects_gzip_magic() {
        let data = [0x1F, 0x8B, 0x08, 0x00];
        let c = detect_at(&data, 0).expect("gzip");
        assert_eq!(c.compression_type, CompressionType::Gzip);
    }
}
