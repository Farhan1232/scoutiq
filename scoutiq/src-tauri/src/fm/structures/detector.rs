//! Detect repeated layouts / file-like names / possible tables generically.

use crate::fm::types::{Confidence, StructureCandidate, StructureType};

const MAX_CANDIDATES: usize = 32;

pub fn detect_structures(data: &[u8], block_index: usize) -> Vec<StructureCandidate> {
    let mut out = Vec::new();
    detect_file_like(data, block_index, &mut out);
    detect_repeated_fixed_width(data, block_index, &mut out);
    detect_count_pointer_pattern(data, block_index, &mut out);
    out.truncate(MAX_CANDIDATES);
    out
}

fn detect_file_like(data: &[u8], block_index: usize, out: &mut Vec<StructureCandidate>) {
    let sample: String = data
        .iter()
        .take(8192)
        .map(|&b| {
            if matches!(b, 0x20..=0x7E) {
                b as char
            } else {
                '\0'
            }
        })
        .collect();
    let lower = sample.to_ascii_lowercase();
    for (needle, sty) in [
        (".dat", StructureType::InternalFile),
        (".bin", StructureType::InternalFile),
        ("table", StructureType::RecordTable),
        ("index", StructureType::IndexTable),
        ("string", StructureType::StringTable),
    ] {
        if let Some(pos) = lower.find(needle) {
            out.push(StructureCandidate {
                block_index,
                offset: pos as u64,
                structure_type: sty,
                confidence: Confidence::Low,
                evidence: vec![format!(
                    "ascii marker '{needle}' at decompressed offset {pos}"
                )],
            });
        }
    }
}

fn detect_repeated_fixed_width(data: &[u8], block_index: usize, out: &mut Vec<StructureCandidate>) {
    if data.len() < 256 {
        return;
    }
    // Probe common record strides.
    for stride in [8usize, 12, 16, 20, 24, 32, 48, 64] {
        let window = &data[..data.len().min(4096)];
        if window.len() < stride * 8 {
            continue;
        }
        let first = &window[..4];
        let mut same_prefix = 0u32;
        let mut i = 0usize;
        while i + 4 <= window.len() {
            if &window[i..i + 4] == first {
                same_prefix += 1;
            }
            i += stride;
        }
        if same_prefix >= 8 {
            out.push(StructureCandidate {
                block_index,
                offset: 0,
                structure_type: StructureType::UnknownRepeatedStructure,
                confidence: Confidence::Medium,
                evidence: vec![format!(
                    "repeating {stride}-byte stride with shared 4-byte prefix ×{same_prefix}"
                )],
            });
            break;
        }
    }
}

fn detect_count_pointer_pattern(
    data: &[u8],
    block_index: usize,
    out: &mut Vec<StructureCandidate>,
) {
    if data.len() < 16 {
        return;
    }
    // Look for small LE counts followed by plausible absolute-ish offsets.
    let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if (2..=4096).contains(&count) {
        let ptr = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        if ptr as usize > 8 && (ptr as usize) < data.len() {
            out.push(StructureCandidate {
                block_index,
                offset: 0,
                structure_type: StructureType::IndexTable,
                confidence: Confidence::Low,
                evidence: vec![format!(
                    "possible count({count})+pointer({ptr}) at block start"
                )],
            });
        }
    }
}
