//! Inner `tad.` container helpers (decompressed payload).

use crate::fm::container::types::TAD_DOT_SIGNATURE;
use crate::fm::entities::types::Evidence;

#[derive(Debug, Clone)]
pub struct TadInfo {
    pub is_tad: bool,
    pub game_version_string: Option<String>,
    pub evidence: Vec<Evidence>,
}

pub fn inspect_tad(data: &[u8]) -> TadInfo {
    let mut evidence = Vec::new();
    if data.len() >= 6 && data[0] == 0x03 && data[1] == 0x01 && &data[2..6] == TAD_DOT_SIGNATURE {
        evidence.push(Evidence::structural(
            "Inner container \\x03\\x01tad. confirmed after Zstd decompress",
        ));
        let game_version_string = extract_version_string(data);
        if let Some(ref v) = game_version_string {
            evidence.push(Evidence::header(format!("tad. version string '{v}'")));
        }
        return TadInfo {
            is_tad: true,
            game_version_string,
            evidence,
        };
    }
    evidence.push(Evidence::absent(
        "No tad. marker at start of decompressed payload",
    ));
    TadInfo {
        is_tad: false,
        game_version_string: None,
        evidence,
    }
}

fn extract_version_string(data: &[u8]) -> Option<String> {
    // Observed: ASCII like "26.0.0+0" / "26.2.0+0" near tad. head.
    let window = &data[..data.len().min(64)];
    let start = window.windows(4).position(|w| {
        w[0].is_ascii_digit() && w[1].is_ascii_digit() && w[2] == b'.' && w[3].is_ascii_digit()
    })?;
    let slice = &window[start..];
    let end = slice
        .iter()
        .position(|&b| b == 0 || !(b.is_ascii_graphic() || b == b'+' || b == b'.'))
        .unwrap_or(slice.len());
    let s = std::str::from_utf8(&slice[..end]).ok()?.to_string();
    if s.len() >= 3 {
        Some(s)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_tad_and_version() {
        let mut data = vec![0x03, 0x01];
        data.extend_from_slice(b"tad.");
        data.extend_from_slice(&[0x2E, 0x00, 0x08, 0x00, 0x00, 0x00]);
        data.extend_from_slice(b"26.2.0+0\0 mf.");
        let info = inspect_tad(&data);
        assert!(info.is_tad);
        assert_eq!(info.game_version_string.as_deref(), Some("26.2.0+0"));
    }
}
