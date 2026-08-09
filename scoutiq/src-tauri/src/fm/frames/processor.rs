//! Validate and process one compressed block; drop bulk data after analysis.

use std::path::Path;

use crate::fm::compression::zstd::{decompress_zstd_frame, MAX_FRAME_IN};
use crate::fm::container::reader::read_slice;
use crate::fm::errors::FmError;
use crate::fm::frames::types::digest_hex;
use crate::fm::strings::extractor::extract_strings_one_pass;
use crate::fm::structures::detector::detect_structures;
use crate::fm::types::{
    CompressedBlockInfo, CompressionType, FmStringEntry, StructureCandidate, ValidationStatus,
};

pub struct ProcessOutcome {
    pub block: CompressedBlockInfo,
    pub strings: Vec<FmStringEntry>,
    pub structures: Vec<StructureCandidate>,
    pub decompressed_len: u64,
}

/// Process a candidate block: decoder validates boundary; analysis runs once.
pub fn process_block(
    path: &Path,
    mut block: CompressedBlockInfo,
) -> Result<ProcessOutcome, FmError> {
    match block.compression_type {
        CompressionType::Zstandard => process_zstd(path, block),
        other => {
            block.validation_status = ValidationStatus::Invalid;
            block.error = Some(format!(
                "Kompression {:?} noch nicht entpackt (nur erkannt)",
                other
            ));
            Ok(ProcessOutcome {
                block,
                strings: Vec::new(),
                structures: Vec::new(),
                decompressed_len: 0,
            })
        }
    }
}

fn process_zstd(path: &Path, mut block: CompressedBlockInfo) -> Result<ProcessOutcome, FmError> {
    let slice = read_slice(path, block.file_offset, MAX_FRAME_IN)?;
    let outcome = decompress_zstd_frame(&slice);

    if !outcome.ok {
        block.validation_status = ValidationStatus::Error;
        block.compressed_size = if outcome.compressed_consumed > 0 {
            Some(outcome.compressed_consumed)
        } else {
            None
        };
        block.error = outcome
            .error
            .or_else(|| Some("Komprimierter Datenblock konnte nicht entpackt werden.".into()));
        return Ok(ProcessOutcome {
            block,
            strings: Vec::new(),
            structures: Vec::new(),
            decompressed_len: 0,
        });
    }

    block.validation_status = ValidationStatus::Valid;
    block.compressed_size = Some(outcome.compressed_consumed);
    block.decompressed_size = Some(outcome.decompressed.len() as u64);
    block.content_digest = Some(digest_hex(&outcome.decompressed));

    let strings = extract_strings_one_pass(&outcome.decompressed, block.index, block.file_offset);
    let structures = detect_structures(&outcome.decompressed, block.index);
    let signatures = collect_generic_signatures(&outcome.decompressed);
    block.signatures = signatures;
    block.string_count = strings.len() as u32;
    block.validation_status = ValidationStatus::Processed;

    let decompressed_len = outcome.decompressed.len() as u64;
    // Explicitly drop bulk bytes — only metadata retained by caller.
    drop(outcome.decompressed);

    Ok(ProcessOutcome {
        block,
        strings,
        structures,
        decompressed_len,
    })
}

fn collect_generic_signatures(data: &[u8]) -> Vec<String> {
    let mut sigs = Vec::new();
    let sample: String = data
        .iter()
        .take(4096)
        .map(|&b| {
            if matches!(b, 0x20..=0x7E) {
                (b as char).to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect();

    // Generic file-like / table-like markers — not entity labels.
    for needle in [
        ".dat", ".bin", "table", "index", "string", "fmdb", "savegame",
    ] {
        if sample.contains(needle) {
            sigs.push(format!("ascii:{needle}"));
        }
    }

    if data.len() >= 64 {
        let hdr = &data[..4];
        let mut repeats = 0u32;
        let mut i = 0usize;
        while i + 4 <= data.len().min(4096) {
            if &data[i..i + 4] == hdr {
                repeats += 1;
            }
            i += 16;
        }
        if repeats >= 4 {
            sigs.push(format!("repeat-header×{repeats}"));
        }
    }

    sigs.sort();
    sigs.dedup();
    sigs
}
