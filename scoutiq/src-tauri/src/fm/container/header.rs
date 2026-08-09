//! Parsed outer FMF header (read-only).

use serde::{Deserialize, Serialize};

use super::types::{FMF_DOT_HEADER_LEN, FMF_DOT_SIGNATURE, FMF_SIGNATURE};
use crate::fm::types::ContainerType;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FmfHeaderInfo {
    pub container: ContainerType,
    /// Offset of the first candidate Zstd frame (validated later by decoder).
    pub first_zstd_offset: Option<u64>,
    pub header_len: u64,
    pub notes: Vec<String>,
}

/// Parse the outer header from the first bytes of a save.
pub fn parse_fmf_header(header: &[u8]) -> FmfHeaderInfo {
    let mut notes = Vec::new();

    // Preferred FM26 layout: 02 01 "fmf." ... 00 03 28 B5 2F FD
    if header.len() >= 30
        && header[0] == 0x02
        && header[1] == 0x01
        && &header[2..6] == FMF_DOT_SIGNATURE
    {
        notes.push("Detected FM26-style \\x02\\x01fmf. outer header".into());
        let mut first_zstd_offset = None;
        if header.len() >= 30 && header[26..30] == [0x28, 0xB5, 0x2F, 0xFD] {
            first_zstd_offset = Some(FMF_DOT_HEADER_LEN);
            notes.push(format!(
                "Zstd magic at offset {FMF_DOT_HEADER_LEN} (after outer header)"
            ));
        } else if let Some(rel) = find_zstd(&header[6..]) {
            first_zstd_offset = Some(6 + rel as u64);
            notes.push(format!(
                "Zstd magic at offset {}",
                first_zstd_offset.unwrap()
            ));
        }
        return FmfHeaderInfo {
            container: ContainerType::Fmf,
            first_zstd_offset,
            header_len: first_zstd_offset.unwrap_or(FMF_DOT_HEADER_LEN),
            notes,
        };
    }

    if header
        .windows(FMF_SIGNATURE.len())
        .any(|w| w == FMF_SIGNATURE)
        || header
            .windows(FMF_DOT_SIGNATURE.len())
            .any(|w| w == FMF_DOT_SIGNATURE)
    {
        notes.push("FMF-like signature present (non-canonical layout)".into());
        let first_zstd_offset = find_zstd(header).map(|i| i as u64);
        return FmfHeaderInfo {
            container: ContainerType::Fmf,
            first_zstd_offset,
            header_len: first_zstd_offset.unwrap_or(0),
            notes,
        };
    }

    notes.push("No FMF signature in header".into());
    FmfHeaderInfo {
        container: ContainerType::Unknown,
        first_zstd_offset: find_zstd(header).map(|i| i as u64),
        header_len: 0,
        notes,
    }
}

fn find_zstd(data: &[u8]) -> Option<usize> {
    data.windows(4).position(|w| w == [0x28, 0xB5, 0x2F, 0xFD])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_real_fm26_header_shape() {
        let mut h = vec![0u8; 32];
        h[0] = 0x02;
        h[1] = 0x01;
        h[2..6].copy_from_slice(b"fmf.");
        h[26..30].copy_from_slice(&[0x28, 0xB5, 0x2F, 0xFD]);
        let info = parse_fmf_header(&h);
        assert_eq!(info.container, ContainerType::Fmf);
        assert_eq!(info.first_zstd_offset, Some(26));
    }
}
