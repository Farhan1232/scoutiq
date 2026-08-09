//! Game / save-format version hints from container header (not filename).

use serde::{Deserialize, Serialize};

use crate::fm::container::types::{FMF_DOT_SIGNATURE, FMF_SIGNATURE};
use crate::fm::entities::types::Evidence;
use crate::fm::types::FmFileInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameVersionInfo {
    pub game: Option<String>,
    pub major_version: Option<u32>,
    pub save_format_version: Option<u32>,
    pub evidence: Vec<Evidence>,
}

/// Derive coarse game/format hints from verified header bytes only.
pub fn detect_game_version(file: &FmFileInfo) -> GameVersionInfo {
    let mut evidence = Vec::new();
    let mut game = None;
    // Never narrowed here: the major version comes from the `tad.` version
    // string in `identity`, not from the outer header.
    let major_version = None;
    let mut save_format_version = None;

    let hex = file.header_hex.replace(' ', "");
    let bytes = hex_to_bytes(&hex);

    if bytes
        .windows(FMF_DOT_SIGNATURE.len())
        .any(|w| w == FMF_DOT_SIGNATURE)
        || bytes.windows(FMF_SIGNATURE.len()).any(|w| {
            // case-insensitive FMF
            w.eq_ignore_ascii_case(FMF_SIGNATURE)
        })
    {
        evidence.push(Evidence::header("FMF container signature present"));
        game = Some("Football Manager".into());
        evidence.push(Evidence::header(
            "Game family inferred from FMF container — major from tad. when available",
        ));
    } else {
        evidence.push(Evidence::absent(
            "No FMF signature in first header bytes — game version unknown",
        ));
    }

    // Look for small integers near the start that often encode format revisions.
    // These remain candidates until validated across multiple FM26 saves.
    if bytes.len() >= 16 {
        let candidates = [
            u16::from_le_bytes([bytes[4], bytes[5]]) as u32,
            u16::from_le_bytes([bytes[8], bytes[9]]) as u32,
            u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
        ];
        for c in candidates {
            if (1..=64).contains(&c) {
                save_format_version = Some(c);
                evidence.push(Evidence::header(format!(
                    "Candidate save_format_version={c} near header (unconfirmed)"
                )));
                break;
            }
        }
    }

    // Do not invent FM26 specifically without deterministic header field mapping.
    let _ = &major_version;

    GameVersionInfo {
        game,
        major_version,
        save_format_version,
        evidence,
    }
}

fn hex_to_bytes(hex: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let mut chars = hex.chars().filter(|c| c.is_ascii_hexdigit());
    while let (Some(a), Some(b)) = (chars.next(), chars.next()) {
        if let Ok(v) = u8::from_str_radix(&format!("{a}{b}"), 16) {
            out.push(v);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fm::types::ContainerType;

    #[test]
    fn detects_fmf_family_without_inventing_major() {
        let mut header = vec![0u8; 32];
        header[0..3].copy_from_slice(b"FMF");
        let hex = header
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");
        let info = detect_game_version(&FmFileInfo {
            file_name: "any.fm".into(),
            file_path: "/tmp/any.fm".into(),
            file_size: 32,
            header_hex: hex,
            detected_container: Some(ContainerType::Fmf),
            modified_unix: None,
        });
        assert_eq!(info.game.as_deref(), Some("Football Manager"));
        assert!(info.major_version.is_none());
    }

    #[test]
    fn does_not_use_filename_for_version() {
        let info = detect_game_version(&FmFileInfo {
            file_name: "MSV_Duisburg_FM26.fm".into(),
            file_path: "/x/MSV_Duisburg_FM26.fm".into(),
            file_size: 10,
            header_hex: "00 01 02".into(),
            detected_container: Some(ContainerType::Unknown),
            modified_unix: None,
        });
        assert!(info.game.is_none());
        assert!(info.major_version.is_none());
    }
}
