//! Streaming FM save inspector — research/debug only.
//! Does not decompress or parse player structures.
//! Original save files are opened read-only and never modified.

use std::path::Path;

use super::reader::{
    bytes_to_hex, empty_inspection, extract_ascii_strings, read_save_sample, HEADER_BYTES,
    INSPECT_SAMPLE_BYTES, MAX_STRING_DISPLAY,
};
use super::types::{DetectedCompression, FmSaveInspection};

const ZSTD_MAGIC: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];
const GZIP_MAGIC: [u8; 2] = [0x1F, 0x8B];
const ZIP_LOCAL: [u8; 4] = [0x50, 0x4B, 0x03, 0x04];
const ZIP_PREFIX: [u8; 2] = [0x50, 0x4B];
const MAX_STRING_SAMPLES: usize = 48;

pub fn inspect_fm_save_path(path_str: &str) -> FmSaveInspection {
    let path = Path::new(path_str);

    let sample = match read_save_sample(path_str, INSPECT_SAMPLE_BYTES) {
        Ok(s) => s,
        Err(msg) => {
            return empty_inspection(path, vec![msg]);
        }
    };

    inspect_from_sample(&sample)
}

pub fn inspect_from_sample(sample: &super::reader::SaveSample) -> FmSaveInspection {
    let mut errors = sample.errors.clone();
    let header_len = sample.bytes.len().min(HEADER_BYTES);
    let header = &sample.bytes[..header_len];
    let first_bytes_hex = bytes_to_hex(header);
    let mut signatures = detect_header_signatures(header);

    let scan = analyze_sample(&sample.bytes);

    if scan.zstd_count > 0 && !signatures.iter().any(|s| s.contains("zstd")) {
        signatures.push(format!("zstd-frame×{}", scan.zstd_count));
    }
    if scan.gzip_hits > 0 && !signatures.iter().any(|s| s.starts_with("gzip")) {
        signatures.push(format!("gzip×{}", scan.gzip_hits));
    }
    if scan.zip_hits > 0 && !signatures.iter().any(|s| s.starts_with("zip")) {
        signatures.push(format!("zip×{}", scan.zip_hits));
    }

    let compression = classify_compression(header, &scan);

    if sample.file_size > 50 * 1024 * 1024 {
        signatures.push(format!(
            "Hinweis: große Datei ({:.1} MB) — nur erste {} KB gescannt",
            sample.file_size as f64 / (1024.0 * 1024.0),
            INSPECT_SAMPLE_BYTES / 1024
        ));
    }

    if sample.bytes.is_empty() && errors.is_empty() {
        errors.push("Spielstand konnte nicht analysiert werden".into());
    }

    FmSaveInspection {
        file_name: sample.file_name.clone(),
        file_path: sample.path.clone(),
        file_size: sample.file_size,
        first_bytes_hex,
        detected_compression: compression.as_label().to_string(),
        zstd_frame_count: scan.zstd_count,
        readable_strings_sample: scan.strings,
        errors,
        detected_signatures: signatures,
    }
}

#[derive(Default)]
struct ScanResult {
    zstd_count: u64,
    gzip_hits: u64,
    zip_hits: u64,
    strings: Vec<String>,
}

fn analyze_sample(sample: &[u8]) -> ScanResult {
    let mut result = ScanResult::default();
    count_magics(sample, &mut result);
    let extracted = extract_ascii_strings(sample, 6, MAX_STRING_DISPLAY);
    result.strings = extracted
        .into_iter()
        .take(MAX_STRING_SAMPLES)
        .map(|s| s.value)
        .collect();
    result
}

fn detect_header_signatures(header: &[u8]) -> Vec<String> {
    let mut out = Vec::new();

    if header.starts_with(&ZSTD_MAGIC) {
        out.push("zstd-frame@0".into());
    }
    if header.starts_with(&GZIP_MAGIC) {
        out.push("gzip@0".into());
    }
    if header.starts_with(&ZIP_LOCAL) {
        out.push("zip-local@0".into());
    } else if header.starts_with(&ZIP_PREFIX) {
        out.push("zip-pk@0".into());
    }

    if header.windows(2).any(|w| w == b"FM") {
        out.push("ascii:FM".into());
    }
    if contains_ascii_ci(header, b"SIGames") || contains_ascii_ci(header, b"Sports Interactive") {
        out.push("ascii:SportsInteractive".into());
    }
    if contains_ascii_ci(header, b"zlib") {
        out.push("ascii:zlib".into());
    }
    if contains_ascii_ci(header, b"zstd") {
        out.push("ascii:zstd".into());
    }

    if out.is_empty() {
        out.push("keine offensichtliche Signatur im Header".into());
    }

    out
}

fn contains_ascii_ci(hay: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || hay.len() < needle.len() {
        return false;
    }
    hay.windows(needle.len()).any(|w| {
        w.iter()
            .zip(needle.iter())
            .all(|(a, b)| a.to_ascii_lowercase() == b.to_ascii_lowercase())
    })
}

fn classify_compression(header: &[u8], scan: &ScanResult) -> DetectedCompression {
    let header_zstd = header.starts_with(&ZSTD_MAGIC);
    let header_gzip = header.starts_with(&GZIP_MAGIC);
    let header_zip = header.starts_with(&ZIP_LOCAL) || header.starts_with(&ZIP_PREFIX);

    let has_zstd = header_zstd || scan.zstd_count > 0;
    let has_gzip = header_gzip || scan.gzip_hits > 0;
    let has_zip = header_zip || scan.zip_hits > 0;

    let kinds = [has_zstd, has_gzip, has_zip].iter().filter(|&&v| v).count();

    match kinds {
        0 => DetectedCompression::None,
        1 if has_zstd => DetectedCompression::Zstd,
        1 if has_gzip => DetectedCompression::Gzip,
        1 if has_zip => DetectedCompression::Zip,
        n if n > 1 => DetectedCompression::Mixed,
        _ => DetectedCompression::Unknown,
    }
}

fn count_magics(data: &[u8], result: &mut ScanResult) {
    let mut offset = 0usize;
    while offset < data.len() {
        match data[offset] {
            0x28 => {
                if offset + 4 <= data.len() && data[offset..offset + 4] == ZSTD_MAGIC {
                    result.zstd_count += 1;
                    offset += 4;
                    continue;
                }
            }
            0x1F => {
                if offset + 2 <= data.len() && data[offset..offset + 2] == GZIP_MAGIC {
                    result.gzip_hits += 1;
                    offset += 2;
                    continue;
                }
            }
            0x50 => {
                if offset + 4 <= data.len() && data[offset..offset + 4] == ZIP_LOCAL {
                    result.zip_hits += 1;
                    offset += 4;
                    continue;
                }
            }
            _ => {}
        }
        offset += 1;
    }
}
