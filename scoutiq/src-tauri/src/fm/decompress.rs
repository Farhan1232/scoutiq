//! Bounded Zstd decompression for FM save metadata (read-only).
//!
//! Never decompresses unbounded frames into RAM. Each frame has a hard
//! output ceiling; incomplete / false-positive magics fail softly.

use std::io::{Cursor, Read};

use super::reader::{extract_all_strings, LocatedString};

const ZSTD_MAGIC: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];

/// Max decompressed bytes per frame.
pub const MAX_FRAME_OUT: usize = 2 * 1024 * 1024;
/// Max frames to attempt.
pub const MAX_FRAMES: usize = 6;
/// Max total decompressed payload retained for string mining.
pub const MAX_TOTAL_OUT: usize = 6 * 1024 * 1024;
/// How much of the scanned window we keep as compressed source.
pub const DECOMPRESS_WINDOW: usize = 2 * 1024 * 1024;

#[derive(Debug, Default)]
pub struct DecompressResult {
    pub frames_attempted: u32,
    pub frames_ok: u32,
    pub bytes_out: usize,
    pub strings: Vec<LocatedString>,
    pub notes: Vec<String>,
}

/// Find zstd magic offsets inside `data` (absolute offsets if `base_offset` applied).
pub fn find_zstd_offsets(data: &[u8], limit: usize) -> Vec<usize> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 4 <= data.len() && out.len() < limit {
        if data[i..i + 4] == ZSTD_MAGIC {
            out.push(i);
            i += 4;
        } else {
            i += 1;
        }
    }
    out
}

/// Try decompressing frames starting at known offsets within `window`.
pub fn decompress_frames_bounded(window: &[u8], offsets: &[usize]) -> DecompressResult {
    let mut result = DecompressResult::default();
    if window.is_empty() || offsets.is_empty() {
        result
            .notes
            .push("Keine Zstd-Frames im Scanfenster für Dekompression".into());
        return result;
    }

    let mut total_out = 0usize;
    let mut all_strings = Vec::new();

    for &offset in offsets.iter().take(MAX_FRAMES) {
        if total_out >= MAX_TOTAL_OUT {
            break;
        }
        if offset >= window.len() {
            continue;
        }
        result.frames_attempted += 1;
        let src = &window[offset..];
        let budget = (MAX_TOTAL_OUT - total_out).min(MAX_FRAME_OUT);
        match decompress_one(src, budget) {
            Some(decoded) if !decoded.is_empty() => {
                result.frames_ok += 1;
                total_out += decoded.len();
                let mut strings = extract_all_strings(&decoded);
                for s in &mut strings {
                    s.offset = offset.saturating_add(s.offset);
                    s.encoding = if s.encoding == "ascii" {
                        "zstd-ascii"
                    } else {
                        "zstd-utf16le"
                    };
                }
                all_strings.extend(strings);
                result.notes.push(format!(
                    "Zstd-Frame @{offset}: {} Bytes dekomprimiert",
                    decoded.len()
                ));
            }
            _ => {
                result.notes.push(format!(
                    "Zstd-Frame @{offset}: Dekompression übersprungen/fehlgeschlagen"
                ));
            }
        }
    }

    all_strings.sort_by_key(|s| s.offset);
    if all_strings.len() > 400 {
        all_strings.truncate(400);
    }
    result.bytes_out = total_out;
    result.strings = all_strings;
    if result.frames_ok == 0 {
        result
            .notes
            .push("Keine gültige Zstd-Dekompression — Heuristik bleibt auf Rohstrings".into());
    } else {
        result.notes.push(format!(
            "{} Zstd-Frame(s) gelesen, {} Strings aus Dekompression",
            result.frames_ok,
            result.strings.len()
        ));
    }
    result
}

pub fn decompress_one_frame(src: &[u8], max_out: usize) -> Option<Vec<u8>> {
    decompress_one(src, max_out)
}

fn decompress_one(src: &[u8], max_out: usize) -> Option<Vec<u8>> {
    if src.len() < 4 || src[..4] != ZSTD_MAGIC || max_out == 0 {
        return None;
    }
    let mut decoder = zstd::stream::read::Decoder::new(Cursor::new(src)).ok()?;

    let mut out = Vec::with_capacity(max_out.min(64 * 1024));
    let mut buf = [0u8; 32 * 1024];
    loop {
        if out.len() >= max_out {
            break;
        }
        let want = buf.len().min(max_out - out.len());
        match decoder.read(&mut buf[..want]) {
            Ok(0) => break,
            Ok(n) => out.extend_from_slice(&buf[..n]),
            Err(_) => {
                // Partial output still useful for string mining.
                break;
            }
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decompresses_simple_zstd_frame() {
        let plain = b"Hello Manager Club Season 2025/2026";
        let compressed = zstd::encode_all(&plain[..], 1).expect("compress");
        assert!(compressed.starts_with(&ZSTD_MAGIC));
        let offsets = find_zstd_offsets(&compressed, 4);
        assert_eq!(offsets, vec![0]);
        let result = decompress_frames_bounded(&compressed, &offsets);
        assert_eq!(result.frames_ok, 1);
        assert!(result
            .strings
            .iter()
            .any(|s| s.value.contains("Manager") || s.value.contains("Season")));
    }

    #[test]
    fn rejects_false_magic_softly() {
        let junk = [0x28u8, 0xB5, 0x2F, 0xFD, 0x00, 0x01, 0x02, 0x03];
        let result = decompress_frames_bounded(&junk, &[0]);
        assert_eq!(result.frames_ok, 0);
    }
}
