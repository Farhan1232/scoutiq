//! One-pass string extraction: ASCII, UTF-8, UTF-16LE, UTF-16BE.

use crate::fm::types::{FmStringEntry, StringEncoding};

#[derive(Debug, Clone, Copy)]
pub struct ExtractConfig {
    pub min_len: usize,
    pub max_len: usize,
    pub max_per_block: usize,
}

impl Default for ExtractConfig {
    fn default() -> Self {
        Self {
            min_len: 4,
            max_len: 120,
            max_per_block: 200,
        }
    }
}

pub fn extract_strings_one_pass(
    data: &[u8],
    block_index: usize,
    source_file_offset: u64,
) -> Vec<FmStringEntry> {
    extract_with_config(
        data,
        block_index,
        source_file_offset,
        ExtractConfig::default(),
    )
}

pub fn extract_with_config(
    data: &[u8],
    block_index: usize,
    source_file_offset: u64,
    cfg: ExtractConfig,
) -> Vec<FmStringEntry> {
    let mut out = Vec::new();
    extract_ascii_utf8(data, block_index, source_file_offset, cfg, &mut out);
    extract_utf16(data, block_index, source_file_offset, true, cfg, &mut out);
    extract_utf16(data, block_index, source_file_offset, false, cfg, &mut out);
    out.truncate(cfg.max_per_block);
    out
}

fn looks_like_garbage(s: &str) -> bool {
    if s.chars().count() < 2 {
        return true;
    }
    let alnum = s.chars().filter(|c| c.is_alphanumeric()).count();
    alnum * 4 < s.chars().count()
}

fn push(
    out: &mut Vec<FmStringEntry>,
    value: &str,
    encoding: StringEncoding,
    block_index: usize,
    decompressed_offset: u64,
    source_file_offset: u64,
    cfg: ExtractConfig,
) {
    if out.len() >= cfg.max_per_block {
        return;
    }
    let t = value.trim();
    if t.chars().count() < cfg.min_len || looks_like_garbage(t) {
        return;
    }
    out.push(FmStringEntry {
        value: t.to_string(),
        encoding,
        block_index,
        decompressed_offset,
        source_file_offset,
    });
}

fn decode_utf8_char(data: &[u8]) -> Option<(char, usize)> {
    if data.is_empty() {
        return None;
    }
    let w = match data[0] {
        0x00..=0x7F => 1,
        0xC2..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF4 => 4,
        _ => return None,
    };
    if data.len() < w {
        return None;
    }
    let s = std::str::from_utf8(&data[..w]).ok()?;
    let ch = s.chars().next()?;
    Some((ch, w))
}

fn is_printable_char(ch: char) -> bool {
    matches!(ch, '\t' | ' '..='~') || (ch as u32) >= 0xA0
}

fn extract_ascii_utf8(
    data: &[u8],
    block_index: usize,
    source_file_offset: u64,
    cfg: ExtractConfig,
    out: &mut Vec<FmStringEntry>,
) {
    let mut i = 0usize;
    while i < data.len() && out.len() < cfg.max_per_block {
        let Some((ch0, len0)) = decode_utf8_char(&data[i..]) else {
            i += 1;
            continue;
        };
        if !is_printable_char(ch0) {
            i += len0;
            continue;
        }

        let start = i;
        let mut s = String::new();
        let mut has_non_ascii = false;
        while i < data.len() && s.chars().count() < cfg.max_len {
            let Some((ch, len)) = decode_utf8_char(&data[i..]) else {
                break;
            };
            if !is_printable_char(ch) {
                break;
            }
            if !ch.is_ascii() {
                has_non_ascii = true;
            }
            s.push(ch);
            i += len;
        }

        if s.chars().count() >= cfg.min_len {
            let enc = if has_non_ascii {
                StringEncoding::Utf8
            } else {
                StringEncoding::Ascii
            };
            push(
                out,
                &s,
                enc,
                block_index,
                start as u64,
                source_file_offset,
                cfg,
            );
        } else if i == start {
            i += 1;
        }
    }
}

fn extract_utf16(
    data: &[u8],
    block_index: usize,
    source_file_offset: u64,
    little: bool,
    cfg: ExtractConfig,
    out: &mut Vec<FmStringEntry>,
) {
    if data.len() < 2 {
        return;
    }
    let enc = if little {
        StringEncoding::Utf16Le
    } else {
        StringEncoding::Utf16Be
    };
    let mut i = 0usize;
    while i + 1 < data.len() && out.len() < cfg.max_per_block {
        let mut chars = Vec::new();
        let start = i;
        while i + 1 < data.len() && chars.len() < cfg.max_len {
            let code = if little {
                u16::from_le_bytes([data[i], data[i + 1]])
            } else {
                u16::from_be_bytes([data[i], data[i + 1]])
            };
            let ok = (0x20..=0x7E).contains(&code)
                || code == 0x09
                || (0xC0..=0xFF).contains(&code)
                || (0x100..=0x024F).contains(&code);
            if ok {
                chars.push(char::from_u32(code as u32).unwrap_or('?'));
                i += 2;
            } else {
                break;
            }
        }
        if chars.len() >= cfg.min_len {
            let value: String = chars.into_iter().collect();
            push(
                out,
                &value,
                enc,
                block_index,
                start as u64,
                source_file_offset,
                cfg,
            );
        }
        i += 2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_ascii() {
        let data = b"xxxxHelloClubyyyy";
        let s = extract_strings_one_pass(data, 0, 100);
        assert!(s
            .iter()
            .any(|e| e.value.contains("HelloClub") && e.encoding == StringEncoding::Ascii));
    }

    #[test]
    fn extracts_utf8() {
        let data = "____München____".as_bytes();
        let s = extract_strings_one_pass(data, 1, 0);
        assert!(s
            .iter()
            .any(|e| e.value.contains("München") && e.encoding == StringEncoding::Utf8));
    }

    #[test]
    fn extracts_utf16le() {
        let mut data = Vec::new();
        for c in "TestClub".encode_utf16() {
            data.extend_from_slice(&c.to_le_bytes());
        }
        let s = extract_strings_one_pass(&data, 0, 0);
        assert!(s
            .iter()
            .any(|e| e.value.contains("TestClub") && e.encoding == StringEncoding::Utf16Le));
    }

    #[test]
    fn extracts_utf16be() {
        let mut data = Vec::new();
        for c in "TestClub".encode_utf16() {
            data.extend_from_slice(&c.to_be_bytes());
        }
        let s = extract_strings_one_pass(&data, 0, 0);
        assert!(s
            .iter()
            .any(|e| e.value.contains("TestClub") && e.encoding == StringEncoding::Utf16Be));
    }
}
