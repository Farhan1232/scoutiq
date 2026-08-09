//! Known-value probe, save diff, diagnostic export (RE only).

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::fm::compression::zstd::{decompress_zstd_frame, MAX_FRAME_IN};
use crate::fm::container::reader::read_slice;
use crate::fm::parser::{analyze_save, AnalyzeOptions};
use crate::fm::reader::bytes_to_hex;
use crate::fm::strings::extractor::extract_strings_one_pass;
use crate::fm::types::{
    ChangedBlock, ChangedRegion, CompressionType, FmEngineResult, FmSaveDiff, KnownValueMatch,
    KnownValueProbeResult, NearbyInteger, StringEncoding, ValidationStatus, PARSER_STATUS,
};

const CONTEXT: usize = 128;
const MAX_MATCHES: usize = 40;

pub fn probe_known_value(
    path_str: &str,
    query: &str,
    query_type: &str,
) -> Result<KnownValueProbeResult, String> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(KnownValueProbeResult {
            query: q.into(),
            query_type: query_type.into(),
            matches: Vec::new(),
            blocks_searched: 0,
            notes: vec!["Leerer Suchwert".into()],
        });
    }

    let analyzed = analyze_save(
        path_str,
        AnalyzeOptions {
            use_cache: true,
            emit_progress: false,
            cancel: None,
            app: None,
        },
    )?;

    let path = Path::new(path_str);
    let mut matches = Vec::new();
    let mut blocks_searched = 0u32;
    let mut notes = Vec::new();

    for block in analyzed.blocks.iter().filter(|b| {
        b.compression_type == CompressionType::Zstandard
            && matches!(
                b.validation_status,
                ValidationStatus::Processed | ValidationStatus::Valid
            )
    }) {
        let slice = read_slice(path, block.file_offset, MAX_FRAME_IN).map_err(|e| e.message)?;
        let outcome = decompress_zstd_frame(&slice);
        blocks_searched += 1;
        if !outcome.ok {
            continue;
        }
        search_decoded(
            &outcome.decompressed,
            q,
            block.index,
            block.file_offset,
            &mut matches,
        );
        drop(outcome.decompressed);
        if matches.len() >= MAX_MATCHES {
            notes.push("Treffer-Limit erreicht".into());
            break;
        }
    }

    if matches.is_empty() {
        notes.push(format!(
            "„{q}“ in {blocks_searched} entpackten Blöcken nicht gefunden"
        ));
    } else {
        notes.push(format!("{} Treffer für „{q}“", matches.len()));
    }

    Ok(KnownValueProbeResult {
        query: q.into(),
        query_type: query_type.into(),
        matches,
        blocks_searched,
        notes,
    })
}

fn search_decoded(
    data: &[u8],
    query: &str,
    block_index: usize,
    source_file_offset: u64,
    matches: &mut Vec<KnownValueMatch>,
) {
    // ASCII / UTF-8 bytes
    let q_bytes = query.as_bytes();
    find_bytes(
        data,
        q_bytes,
        block_index,
        source_file_offset,
        StringEncoding::Utf8,
        matches,
    );

    // UTF-16LE
    let mut q16le = Vec::new();
    for c in query.encode_utf16() {
        q16le.extend_from_slice(&c.to_le_bytes());
    }
    find_bytes(
        data,
        &q16le,
        block_index,
        source_file_offset,
        StringEncoding::Utf16Le,
        matches,
    );

    // UTF-16BE
    let mut q16be = Vec::new();
    for c in query.encode_utf16() {
        q16be.extend_from_slice(&c.to_be_bytes());
    }
    find_bytes(
        data,
        &q16be,
        block_index,
        source_file_offset,
        StringEncoding::Utf16Be,
        matches,
    );
}

fn find_bytes(
    data: &[u8],
    needle: &[u8],
    block_index: usize,
    source_file_offset: u64,
    encoding: StringEncoding,
    matches: &mut Vec<KnownValueMatch>,
) {
    if needle.is_empty() || matches.len() >= MAX_MATCHES {
        return;
    }
    let mut start = 0usize;
    while let Some(rel) = find_subslice(&data[start..], needle) {
        let off = start + rel;
        push_match(
            data,
            off,
            needle.len(),
            block_index,
            source_file_offset,
            encoding,
            matches,
        );
        start = off + needle.len().max(1);
        if matches.len() >= MAX_MATCHES {
            break;
        }
    }
}

fn find_subslice(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn push_match(
    data: &[u8],
    off: usize,
    len: usize,
    block_index: usize,
    source_file_offset: u64,
    encoding: StringEncoding,
    matches: &mut Vec<KnownValueMatch>,
) {
    let before_start = off.saturating_sub(CONTEXT);
    let after_end = (off + len + CONTEXT).min(data.len());
    let before = &data[before_start..off];
    let after = &data[off + len..after_end];

    let nearby = extract_strings_one_pass(
        &data[before_start..after_end],
        block_index,
        source_file_offset,
    )
    .into_iter()
    .map(|s| s.value)
    .take(12)
    .collect::<Vec<_>>();

    let ints = nearby_integers(data, off);

    let raw = &data[off..off + len.min(data.len().saturating_sub(off))];
    let searched = match encoding {
        StringEncoding::Utf16Le | StringEncoding::Utf16Be => {
            // Prefer reconstructing from UTF-16 code units when possible.
            let mut u16s = Vec::new();
            let mut i = 0;
            while i + 1 < raw.len() {
                let code = if encoding == StringEncoding::Utf16Le {
                    u16::from_le_bytes([raw[i], raw[i + 1]])
                } else {
                    u16::from_be_bytes([raw[i], raw[i + 1]])
                };
                u16s.push(code);
                i += 2;
            }
            String::from_utf16_lossy(&u16s)
        }
        _ => String::from_utf8_lossy(raw).into_owned(),
    };

    matches.push(KnownValueMatch {
        searched_value: searched,
        block_index,
        source_file_offset,
        decompressed_offset: off as u64,
        encoding,
        nearby_strings: nearby,
        context_before_hex: bytes_to_hex(before),
        context_after_hex: bytes_to_hex(after),
        nearby_integers: ints,
    });
}

fn nearby_integers(data: &[u8], at: usize) -> Vec<NearbyInteger> {
    let mut out = Vec::new();
    let base = at.saturating_sub(8);
    if base + 1 <= data.len() {
        out.push(NearbyInteger {
            offset: base as u64,
            width: 8,
            endian: "n/a".into(),
            signed: false,
            value: data[base].to_string(),
        });
    }
    for (endian, le) in [("le", true), ("be", false)] {
        if base + 2 <= data.len() {
            let v = if le {
                u16::from_le_bytes([data[base], data[base + 1]])
            } else {
                u16::from_be_bytes([data[base], data[base + 1]])
            };
            out.push(NearbyInteger {
                offset: base as u64,
                width: 16,
                endian: endian.into(),
                signed: false,
                value: v.to_string(),
            });
        }
        if base + 4 <= data.len() {
            let bytes = [data[base], data[base + 1], data[base + 2], data[base + 3]];
            let u = if le {
                u32::from_le_bytes(bytes)
            } else {
                u32::from_be_bytes(bytes)
            };
            let i = if le {
                i32::from_le_bytes(bytes)
            } else {
                i32::from_be_bytes(bytes)
            };
            out.push(NearbyInteger {
                offset: base as u64,
                width: 32,
                endian: endian.into(),
                signed: false,
                value: u.to_string(),
            });
            out.push(NearbyInteger {
                offset: base as u64,
                width: 32,
                endian: endian.into(),
                signed: true,
                value: i.to_string(),
            });
        }
        if base + 8 <= data.len() {
            let bytes = [
                data[base],
                data[base + 1],
                data[base + 2],
                data[base + 3],
                data[base + 4],
                data[base + 5],
                data[base + 6],
                data[base + 7],
            ];
            let u = if le {
                u64::from_le_bytes(bytes)
            } else {
                u64::from_be_bytes(bytes)
            };
            out.push(NearbyInteger {
                offset: base as u64,
                width: 64,
                endian: endian.into(),
                signed: false,
                value: u.to_string(),
            });
        }
    }
    out
}

pub fn compare_fm_saves(path_a: &str, path_b: &str) -> Result<FmSaveDiff, String> {
    let opts = AnalyzeOptions {
        use_cache: true,
        emit_progress: false,
        cancel: None,
        app: None,
    };
    let a = analyze_save(path_a, opts.clone())?;
    let b = analyze_save(path_b, opts)?;

    let digests_a: Vec<_> = a
        .blocks
        .iter()
        .filter_map(|bl| {
            bl.content_digest
                .as_ref()
                .map(|d| (bl.index, bl.file_offset, d.clone(), bl.decompressed_size))
        })
        .collect();
    let digests_b: Vec<_> = b
        .blocks
        .iter()
        .filter_map(|bl| {
            bl.content_digest
                .as_ref()
                .map(|d| (bl.index, bl.file_offset, d.clone(), bl.decompressed_size))
        })
        .collect();

    let mut changed_blocks = Vec::new();
    let mut unchanged = 0u32;
    let mut only_a = 0u32;
    let mut only_b = 0u32;
    let max = digests_a.len().max(digests_b.len());

    for i in 0..max {
        match (digests_a.get(i), digests_b.get(i)) {
            (Some(x), Some(y)) if x.2 == y.2 && x.3 == y.3 => {
                unchanged += 1;
            }
            (Some(x), Some(y)) => {
                changed_blocks.push(ChangedBlock {
                    index_a: Some(x.0),
                    index_b: Some(y.0),
                    file_offset_a: Some(x.1),
                    file_offset_b: Some(y.1),
                    status: "changed".into(),
                    digest_a: Some(x.2.clone()),
                    digest_b: Some(y.2.clone()),
                    decompressed_size_a: x.3,
                    decompressed_size_b: y.3,
                    note: "Fingerprint unterschiedlich".into(),
                });
            }
            (Some(x), None) => {
                only_a += 1;
                changed_blocks.push(ChangedBlock {
                    index_a: Some(x.0),
                    index_b: None,
                    file_offset_a: Some(x.1),
                    file_offset_b: None,
                    status: "only_a".into(),
                    digest_a: Some(x.2.clone()),
                    digest_b: None,
                    decompressed_size_a: x.3,
                    decompressed_size_b: None,
                    note: "Nur in Save A".into(),
                });
            }
            (None, Some(y)) => {
                only_b += 1;
                changed_blocks.push(ChangedBlock {
                    index_a: None,
                    index_b: Some(y.0),
                    file_offset_a: None,
                    file_offset_b: Some(y.1),
                    status: "only_b".into(),
                    digest_a: None,
                    digest_b: Some(y.2.clone()),
                    decompressed_size_a: None,
                    decompressed_size_b: y.3,
                    note: "Nur in Save B".into(),
                });
            }
            (None, None) => {}
        }
    }

    // Coarse region hints from changed block offsets (not full-file byte diff).
    let changed_regions: Vec<ChangedRegion> = changed_blocks
        .iter()
        .filter(|c| c.status == "changed")
        .filter_map(|c| {
            Some(ChangedRegion {
                block_index: c.index_a.or(c.index_b),
                offset: c.file_offset_a.or(c.file_offset_b)?,
                length: c.decompressed_size_a.or(c.decompressed_size_b).unwrap_or(0),
                note: "Geänderter Block (Fingerprint)".into(),
            })
        })
        .collect();

    Ok(FmSaveDiff {
        file_a_size: a.file.file_size,
        file_b_size: b.file.file_size,
        blocks_a: a.blocks.len(),
        blocks_b: b.blocks.len(),
        path_a: path_a.into(),
        path_b: path_b.into(),
        changed_blocks,
        changed_regions,
        unchanged_count: unchanged,
        only_in_a: only_a,
        only_in_b: only_b,
        notes: vec![
            "Vergleich über Block-Fingerprints (kein Full-File-Diff)".into(),
            PARSER_STATUS.into(),
        ],
    })
}

pub fn export_diagnostic_json(
    result: &FmEngineResult,
    probe: Option<&KnownValueProbeResult>,
) -> serde_json::Value {
    let generated_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default();

    serde_json::json!({
        "version": 2,
        "parserStatus": PARSER_STATUS,
        "generatedAt": generated_at,
        "file": result.file,
        "blockStats": result.block_stats,
        "blocks": result.blocks.iter().map(|b| serde_json::json!({
            "index": b.index,
            "compressionType": b.compression_type,
            "fileOffset": b.file_offset,
            "compressedSize": b.compressed_size,
            "decompressedSize": b.decompressed_size,
            "validationStatus": b.validation_status,
            "contentDigest": b.content_digest,
            "signatures": b.signatures,
            "error": b.error,
        })).collect::<Vec<_>>(),
        "structures": result.structures,
        "structureStats": result.structure_stats,
        "stringSample": result.string_index.iter().take(100).collect::<Vec<_>>(),
        "performance": result.performance,
        "probe": probe,
        "notes": result.notes,
    })
}
