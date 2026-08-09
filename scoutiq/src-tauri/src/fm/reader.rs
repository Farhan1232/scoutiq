//! Read-only helpers for Football Manager save files.
//! Never opens the file for writing.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use super::types::FmSaveInspection;

pub const HEADER_BYTES: usize = 512;
/// Keep first open tiny — large FM saves + iCloud hydrate slowly.
pub const INSPECT_SAMPLE_BYTES: usize = 8 * 1024;
pub const METADATA_SAMPLE_BYTES: usize = 8 * 1024;
pub const MIN_STRING_LEN: usize = 4;
pub const MAX_STRING_DISPLAY: usize = 120;

#[derive(Debug, Clone)]
pub struct LocatedString {
    pub value: String,
    pub offset: usize,
    pub encoding: &'static str,
}

#[derive(Debug, Clone)]
pub struct SaveSample {
    pub path: String,
    pub file_name: String,
    pub file_size: u64,
    pub bytes: Vec<u8>,
    pub errors: Vec<String>,
}

pub fn file_name_from_path(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unbekannt")
        .to_string()
}

pub fn is_supported_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| {
            let lower = ext.to_ascii_lowercase();
            lower == "fm" || lower == "sav"
        })
        .unwrap_or(false)
}

pub fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn empty_inspection(path: &Path, errors: Vec<String>) -> FmSaveInspection {
    FmSaveInspection {
        file_name: file_name_from_path(path),
        file_path: path.to_string_lossy().into_owned(),
        file_size: 0,
        first_bytes_hex: String::new(),
        detected_compression: "unbekannt".into(),
        zstd_frame_count: 0,
        readable_strings_sample: Vec::new(),
        errors,
        detected_signatures: Vec::new(),
    }
}

/// Read a small prefix of the save (legacy sync path). Prefer `pipeline::run_fm_analysis`.
pub fn read_save_sample(path_str: &str, max_bytes: usize) -> Result<SaveSample, String> {
    use super::errors::map_io_error;

    let path = Path::new(path_str);
    let mut errors = Vec::new();

    if !is_supported_extension(path) {
        errors.push("Nicht unterstütztes Dateiformat".into());
    }

    let mut file = File::open(path).map_err(|e| map_io_error(&e, path).message)?;

    // Prefer stream_len when available; fall back without failing the read.
    let file_size = file.metadata().map(|m| m.len()).unwrap_or(0);

    let want = if file_size > 0 {
        (file_size as usize).min(max_bytes)
    } else {
        max_bytes
    };

    let mut bytes = vec![0u8; want];
    let n = file
        .read(&mut bytes)
        .map_err(|e| map_io_error(&e, path).message)?;
    bytes.truncate(n);

    // If metadata failed, report at least the bytes we actually read.
    let file_size = if file_size > 0 { file_size } else { n as u64 };

    Ok(SaveSample {
        path: path_str.to_string(),
        file_name: file_name_from_path(path),
        file_size,
        bytes,
        errors,
    })
}

/// Extract printable ASCII runs with offsets.
pub fn extract_ascii_strings(data: &[u8], min_len: usize, max_len: usize) -> Vec<LocatedString> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    let mut current = Vec::<u8>::new();

    for (i, &b) in data.iter().enumerate() {
        if matches!(b, 0x09 | 0x20..=0x7E) {
            if start.is_none() {
                start = Some(i);
            }
            if current.len() < max_len {
                current.push(b);
            }
        } else if let Some(offset) = start.take() {
            if current.len() >= min_len {
                if let Ok(s) = std::str::from_utf8(&current) {
                    let trimmed = s.trim();
                    if !trimmed.is_empty() {
                        out.push(LocatedString {
                            value: trimmed.to_string(),
                            offset,
                            encoding: "ascii",
                        });
                    }
                }
            }
            current.clear();
        }
    }

    if let Some(offset) = start {
        if current.len() >= min_len {
            if let Ok(s) = std::str::from_utf8(&current) {
                let trimmed = s.trim();
                if !trimmed.is_empty() {
                    out.push(LocatedString {
                        value: trimmed.to_string(),
                        offset,
                        encoding: "ascii",
                    });
                }
            }
        }
    }

    out
}

/// Extract UTF-16LE printable runs (common in Windows/FM-related blobs).
pub fn extract_utf16le_strings(data: &[u8], min_len: usize, max_len: usize) -> Vec<LocatedString> {
    let mut out = Vec::new();
    if data.len() < 2 {
        return out;
    }

    let mut i = 0usize;
    while i + 1 < data.len() {
        let mut chars = Vec::<char>::new();
        let start = i;
        while i + 1 < data.len() {
            let code = u16::from_le_bytes([data[i], data[i + 1]]);
            // ASCII + Latin-1 letters (German umlauts etc. in FM UTF-16 blobs)
            let printable =
                (0x20..=0x7E).contains(&code) || code == 0x09 || (0xC0..=0xFF).contains(&code);
            if printable {
                chars.push(char::from_u32(code as u32).unwrap_or('?'));
                i += 2;
                if chars.len() >= max_len {
                    break;
                }
            } else {
                break;
            }
        }

        if chars.len() >= min_len {
            let value: String = chars.into_iter().collect();
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                out.push(LocatedString {
                    value: trimmed.to_string(),
                    offset: start,
                    encoding: "utf16le",
                });
            }
        }

        i += 2;
    }

    out
}

pub fn extract_all_strings(data: &[u8]) -> Vec<LocatedString> {
    let mut all = extract_ascii_strings(data, MIN_STRING_LEN, MAX_STRING_DISPLAY);
    all.extend(extract_utf16le_strings(
        data,
        MIN_STRING_LEN,
        MAX_STRING_DISPLAY,
    ));
    all.sort_by_key(|s| s.offset);
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_ascii_strings_with_offsets() {
        let data = b"xxxxManager Name\0\0Club ABCDEF\0yy";
        let found = extract_ascii_strings(data, 4, 64);
        let values: Vec<_> = found.iter().map(|s| s.value.as_str()).collect();
        assert!(values.iter().any(|v| v.contains("Manager")));
        assert!(values.iter().any(|v| v.contains("Club")));
        assert!(found.iter().all(|s| s.encoding == "ascii"));
    }

    #[test]
    fn extracts_utf16le_strings() {
        let mut data = Vec::new();
        for c in ['B', 'a', 'y', 'e', 'r', 'n'] {
            data.push(c as u8);
            data.push(0);
        }
        data.extend_from_slice(&[0, 0]);
        let found = extract_utf16le_strings(&data, 4, 64);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].value, "Bayern");
        assert_eq!(found[0].encoding, "utf16le");
    }
}
