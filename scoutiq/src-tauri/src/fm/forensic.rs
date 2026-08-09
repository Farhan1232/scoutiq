//! Forensic debug structures for FM26 save reverse-engineering (read-only).

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use serde::Serialize;

use super::decompress::{decompress_one_frame, find_zstd_offsets, DECOMPRESS_WINDOW, MAX_FRAMES};
use super::errors::map_io_error;
use super::reader::bytes_to_hex;

const CONTEXT_RADIUS: usize = 64;
const MAX_CATALOG_STRINGS: usize = 400;
const MAX_PROBE_MATCHES: usize = 40;
const ENTROPY_BLOCK: usize = 64 * 1024;
const MIN_STR: usize = 4;
const MAX_STR: usize = 96;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForensicStringHit {
    pub offset: u64,
    pub value: String,
    pub length: u32,
    pub encoding: String,
    pub source: String,
    pub context_before_hex: String,
    pub context_after_hex: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegerCandidate {
    pub offset: u64,
    pub width: u8,
    pub endian: String,
    pub signed: bool,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeMatch {
    pub offset: u64,
    pub encoding: String,
    pub source: String,
    pub matched_value: String,
    pub context_hex: String,
    pub nearby_strings: Vec<String>,
    pub nearby_integers: Vec<IntegerCandidate>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepeatedPatternGroup {
    pub value: String,
    pub occurrences: u32,
    pub offsets: Vec<u64>,
    pub similar_context_count: u32,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntropyBlock {
    pub offset: u64,
    pub size: u64,
    pub entropy: f32,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressedBlockInfo {
    pub offset: u64,
    pub magic: String,
    pub decompressed_bytes: u64,
    pub success: bool,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EncodingStats {
    pub ascii: u32,
    pub utf8: u32,
    pub utf16_le: u32,
    pub utf16_be: u32,
    pub from_raw: u32,
    pub from_zstd: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForensicLog {
    pub bytes_scanned: u64,
    pub file_size: u64,
    pub strings_by_encoding: EncodingStats,
    pub compressed_blocks_detected: u32,
    pub decompressed_blocks_processed: u32,
    pub decompressed_bytes_total: u64,
    pub catalog_entries: u32,
    pub repeated_groups: u32,
    pub entropy_blocks: u32,
    pub promising_offsets: Vec<u64>,
    pub summary_lines: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForensicReport {
    pub strings: Vec<ForensicStringHit>,
    pub repeated_patterns: Vec<RepeatedPatternGroup>,
    pub entropy_map: Vec<EntropyBlock>,
    pub compressed_blocks: Vec<CompressedBlockInfo>,
    pub log: ForensicLog,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeResult {
    pub query: String,
    pub query_type: String,
    pub matches: Vec<ProbeMatch>,
    pub encodings_hit: Vec<String>,
    pub notes: Vec<String>,
}

pub fn empty_forensic_report() -> ForensicReport {
    ForensicReport {
        strings: Vec::new(),
        repeated_patterns: Vec::new(),
        entropy_map: Vec::new(),
        compressed_blocks: Vec::new(),
        log: ForensicLog {
            bytes_scanned: 0,
            file_size: 0,
            strings_by_encoding: EncodingStats {
                ascii: 0,
                utf8: 0,
                utf16_le: 0,
                utf16_be: 0,
                from_raw: 0,
                from_zstd: 0,
            },
            compressed_blocks_detected: 0,
            decompressed_blocks_processed: 0,
            decompressed_bytes_total: 0,
            catalog_entries: 0,
            repeated_groups: 0,
            entropy_blocks: 0,
            promising_offsets: Vec::new(),
            summary_lines: Vec::new(),
        },
    }
}

/// Build forensic catalog from the scanned window + optional decompressed slices.
pub fn build_forensic_report(
    window: &[u8],
    file_size: u64,
    bytes_scanned: u64,
    zstd_offsets: &[usize],
) -> ForensicReport {
    let mut strings = Vec::new();
    let mut stats = EncodingStats {
        ascii: 0,
        utf8: 0,
        utf16_le: 0,
        utf16_be: 0,
        from_raw: 0,
        from_zstd: 0,
    };

    collect_all_encodings(window, 0, "raw", &mut strings, &mut stats);

    let mut compressed_blocks = Vec::new();
    let mut offsets = zstd_offsets.to_vec();
    if offsets.is_empty() {
        offsets = find_zstd_offsets(window, MAX_FRAMES);
    }
    let mut decompressed_ok = 0u32;
    let mut decompressed_bytes = 0u64;

    for &off in offsets.iter().take(MAX_FRAMES) {
        if off >= window.len() {
            continue;
        }
        let src = &window[off..];
        match decompress_one_frame(src, 2 * 1024 * 1024) {
            Some(decoded) if !decoded.is_empty() => {
                decompressed_ok += 1;
                decompressed_bytes += decoded.len() as u64;
                compressed_blocks.push(CompressedBlockInfo {
                    offset: off as u64,
                    magic: "zstd".into(),
                    decompressed_bytes: decoded.len() as u64,
                    success: true,
                    note: format!("Frame @{off} → {} Bytes", decoded.len()),
                });
                // Stream string extraction without keeping full decoded blob beyond this loop.
                collect_all_encodings(&decoded, off as u64, "zstd", &mut strings, &mut stats);
            }
            _ => {
                compressed_blocks.push(CompressedBlockInfo {
                    offset: off as u64,
                    magic: "zstd".into(),
                    decompressed_bytes: 0,
                    success: false,
                    note: format!("Frame @{off}: Dekompression fehlgeschlagen/übersprungen"),
                });
            }
        }
    }

    // Cap catalog — prefer longer / more informative strings.
    strings.sort_by(|a, b| {
        b.length
            .cmp(&a.length)
            .then_with(|| a.offset.cmp(&b.offset))
    });
    strings.dedup_by(|a, b| a.value == b.value && a.encoding == b.encoding);
    if strings.len() > MAX_CATALOG_STRINGS {
        strings.truncate(MAX_CATALOG_STRINGS);
    }
    // Re-sort by offset for UI browsing.
    strings.sort_by_key(|s| s.offset);

    let repeated_patterns = find_repeated_patterns(&strings, window);
    let entropy_map = build_entropy_map(window);
    let promising = promising_offsets(&strings, &repeated_patterns);

    let mut summary = Vec::new();
    summary.push(format!(
        "Strings: ascii={} utf8={} utf16le={} utf16be={} (raw={} zstd={})",
        stats.ascii, stats.utf8, stats.utf16_le, stats.utf16_be, stats.from_raw, stats.from_zstd
    ));
    summary.push(format!(
        "Kompression: {} Blöcke erkannt, {} dekomprimiert ({} Bytes Output)",
        compressed_blocks.len(),
        decompressed_ok,
        decompressed_bytes
    ));
    summary.push(format!(
        "Katalog: {} Einträge, {} Wiederholungsgruppen, {} Entropy-Blöcke",
        strings.len(),
        repeated_patterns.len(),
        entropy_map.len()
    ));
    if !promising.is_empty() {
        summary.push(format!(
            "Vielversprechende Offsets: {}",
            promising
                .iter()
                .take(12)
                .map(|o| o.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    ForensicReport {
        log: ForensicLog {
            bytes_scanned,
            file_size,
            strings_by_encoding: stats,
            compressed_blocks_detected: compressed_blocks.len() as u32,
            decompressed_blocks_processed: decompressed_ok,
            decompressed_bytes_total: decompressed_bytes,
            catalog_entries: strings.len() as u32,
            repeated_groups: repeated_patterns.len() as u32,
            entropy_blocks: entropy_map.len() as u32,
            promising_offsets: promising,
            summary_lines: summary,
        },
        strings,
        repeated_patterns,
        entropy_map,
        compressed_blocks,
    }
}

/// Targeted probe for a known manager/club/competition/free-text value.
pub fn probe_known_value(window: &[u8], query: &str, query_type: &str) -> ProbeResult {
    let q = query.trim();
    let mut notes = Vec::new();
    if q.is_empty() {
        return ProbeResult {
            query: q.into(),
            query_type: query_type.into(),
            matches: Vec::new(),
            encodings_hit: Vec::new(),
            notes: vec!["Leerer Suchwert".into()],
        };
    }
    if window.is_empty() {
        notes.push("Kein Scanfenster verfügbar — Spielstand zuerst analysieren".into());
        return ProbeResult {
            query: q.into(),
            query_type: query_type.into(),
            matches: Vec::new(),
            encodings_hit: Vec::new(),
            notes,
        };
    }

    let mut matches = Vec::new();
    let mut encodings_hit = Vec::new();

    // Raw window searches.
    search_needle(window, 0, "raw", q, &mut matches, &mut encodings_hit);

    // Decompress frames and search without retaining all output at once.
    let offsets = find_zstd_offsets(window, MAX_FRAMES);
    for &off in &offsets {
        if off >= window.len() {
            continue;
        }
        if let Some(decoded) = decompress_one_frame(&window[off..], 2 * 1024 * 1024) {
            search_needle(
                &decoded,
                off as u64,
                "zstd",
                q,
                &mut matches,
                &mut encodings_hit,
            );
        }
    }

    matches.truncate(MAX_PROBE_MATCHES);
    encodings_hit.sort();
    encodings_hit.dedup();

    if matches.is_empty() {
        notes.push(format!(
            "„{q}“ nicht im Scanfenster / dekomprimierten Frames gefunden"
        ));
    } else {
        notes.push(format!("{} Treffer für „{q}“", matches.len()));
    }

    ProbeResult {
        query: q.into(),
        query_type: query_type.into(),
        matches,
        encodings_hit,
        notes,
    }
}

fn collect_all_encodings(
    data: &[u8],
    base_offset: u64,
    source: &str,
    out: &mut Vec<ForensicStringHit>,
    stats: &mut EncodingStats,
) {
    for hit in extract_ascii_hits(data, base_offset, source) {
        bump_stats(stats, &hit);
        out.push(hit);
    }
    for hit in extract_utf8_hits(data, base_offset, source) {
        bump_stats(stats, &hit);
        out.push(hit);
    }
    for hit in extract_utf16_hits(data, base_offset, source, true) {
        bump_stats(stats, &hit);
        out.push(hit);
    }
    for hit in extract_utf16_hits(data, base_offset, source, false) {
        bump_stats(stats, &hit);
        out.push(hit);
    }
}

fn bump_stats(stats: &mut EncodingStats, hit: &ForensicStringHit) {
    match hit.encoding.as_str() {
        "ascii" => stats.ascii += 1,
        "utf8" => stats.utf8 += 1,
        "utf16le" => stats.utf16_le += 1,
        "utf16be" => stats.utf16_be += 1,
        _ => {}
    }
    if hit.source == "raw" {
        stats.from_raw += 1;
    } else {
        stats.from_zstd += 1;
    }
}

fn extract_ascii_hits(data: &[u8], base: u64, source: &str) -> Vec<ForensicStringHit> {
    let mut out = Vec::new();
    let mut start = None;
    let mut buf = Vec::new();
    for (i, &b) in data.iter().enumerate() {
        if matches!(b, 0x20..=0x7E | 0x09) {
            if start.is_none() {
                start = Some(i);
            }
            if buf.len() < MAX_STR {
                buf.push(b);
            }
        } else if let Some(s) = start.take() {
            push_if_ok(&mut out, data, base, s, &buf, "ascii", source);
            buf.clear();
        }
    }
    if let Some(s) = start {
        push_if_ok(&mut out, data, base, s, &buf, "ascii", source);
    }
    out
}

fn extract_utf8_hits(data: &[u8], base: u64, source: &str) -> Vec<ForensicStringHit> {
    // Multi-byte UTF-8 runs that are not pure ASCII (ASCII already covered).
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < data.len() {
        if data[i] < 0x80 {
            i += 1;
            continue;
        }
        let start = i;
        let mut end = i;
        while end < data.len() && end - start < MAX_STR * 2 {
            match std::str::from_utf8(&data[start..=end]) {
                Ok(s) if s.chars().all(|c| !c.is_control() || c == '\t') => {
                    end += 1;
                }
                Ok(_) | Err(_) => break,
            }
        }
        if end > start {
            if let Ok(s) = std::str::from_utf8(&data[start..end]) {
                let trimmed = s.trim();
                if trimmed.chars().count() >= MIN_STR && trimmed.bytes().any(|b| b >= 0x80) {
                    out.push(make_hit(data, base, start, trimmed, "utf8", source));
                }
            }
            i = end.max(start + 1);
        } else {
            i += 1;
        }
    }
    out
}

fn extract_utf16_hits(
    data: &[u8],
    base: u64,
    source: &str,
    little: bool,
) -> Vec<ForensicStringHit> {
    let mut out = Vec::new();
    if data.len() < 2 {
        return out;
    }
    let enc = if little { "utf16le" } else { "utf16be" };
    let mut i = 0usize;
    while i + 1 < data.len() {
        let mut chars = Vec::new();
        let start = i;
        while i + 1 < data.len() && chars.len() < MAX_STR {
            let code = if little {
                u16::from_le_bytes([data[i], data[i + 1]])
            } else {
                u16::from_be_bytes([data[i], data[i + 1]])
            };
            let printable = (0x20..=0x7E).contains(&code)
                || code == 0x09
                || (0xC0..=0xFF).contains(&code)
                || (0x100..=0x024F).contains(&code);
            if printable {
                chars.push(char::from_u32(code as u32).unwrap_or('?'));
                i += 2;
            } else {
                break;
            }
        }
        if chars.len() >= MIN_STR {
            let value: String = chars.into_iter().collect();
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                out.push(make_hit(data, base, start, trimmed, enc, source));
            }
        }
        i += 2;
    }
    out
}

fn push_if_ok(
    out: &mut Vec<ForensicStringHit>,
    data: &[u8],
    base: u64,
    start: usize,
    buf: &[u8],
    encoding: &str,
    source: &str,
) {
    if buf.len() < MIN_STR {
        return;
    }
    if let Ok(s) = std::str::from_utf8(buf) {
        let trimmed = s.trim();
        if !trimmed.is_empty() {
            out.push(make_hit(data, base, start, trimmed, encoding, source));
        }
    }
}

fn make_hit(
    data: &[u8],
    base: u64,
    local_start: usize,
    value: &str,
    encoding: &str,
    source: &str,
) -> ForensicStringHit {
    let before_start = local_start.saturating_sub(CONTEXT_RADIUS);
    let after_start =
        (local_start + value.len().min(data.len().saturating_sub(local_start))).min(data.len());
    let after_end = (after_start + CONTEXT_RADIUS).min(data.len());
    ForensicStringHit {
        offset: base + local_start as u64,
        value: value.to_string(),
        length: value.chars().count() as u32,
        encoding: encoding.into(),
        source: source.into(),
        context_before_hex: bytes_to_hex(&data[before_start..local_start]),
        context_after_hex: bytes_to_hex(&data[after_start..after_end]),
    }
}

fn search_needle(
    data: &[u8],
    base: u64,
    source: &str,
    query: &str,
    matches: &mut Vec<ProbeMatch>,
    encodings_hit: &mut Vec<String>,
) {
    if matches.len() >= MAX_PROBE_MATCHES {
        return;
    }
    // ASCII / UTF-8 bytes
    let q_bytes = query.as_bytes();
    if !q_bytes.is_empty() {
        for (i, window) in data.windows(q_bytes.len()).enumerate() {
            if window.eq_ignore_ascii_case(q_bytes) || window == q_bytes {
                push_probe_match(
                    data,
                    base,
                    i,
                    query,
                    "utf8/ascii",
                    source,
                    matches,
                    encodings_hit,
                );
                if matches.len() >= MAX_PROBE_MATCHES {
                    return;
                }
            }
        }
    }
    // UTF-16 LE / BE encodings of the query
    let utf16: Vec<u16> = query.encode_utf16().collect();
    let le: Vec<u8> = utf16.iter().flat_map(|c| c.to_le_bytes()).collect();
    let be: Vec<u8> = utf16.iter().flat_map(|c| c.to_be_bytes()).collect();
    for (label, needle) in [("utf16le", &le[..]), ("utf16be", &be[..])] {
        if needle.is_empty() {
            continue;
        }
        for (i, window) in data.windows(needle.len()).enumerate() {
            if window == needle {
                push_probe_match(data, base, i, query, label, source, matches, encodings_hit);
                if matches.len() >= MAX_PROBE_MATCHES {
                    return;
                }
            }
        }
    }
}

fn push_probe_match(
    data: &[u8],
    base: u64,
    local: usize,
    query: &str,
    encoding: &str,
    source: &str,
    matches: &mut Vec<ProbeMatch>,
    encodings_hit: &mut Vec<String>,
) {
    let abs = base + local as u64;
    let ctx_start = local.saturating_sub(CONTEXT_RADIUS);
    let ctx_end = (local + query.len().max(16) + CONTEXT_RADIUS).min(data.len());
    let nearby = nearby_readable(data, local, 8);
    let ints = interpret_integers_near(data, local);

    encodings_hit.push(encoding.into());
    matches.push(ProbeMatch {
        offset: abs,
        encoding: encoding.into(),
        source: source.into(),
        matched_value: query.into(),
        context_hex: bytes_to_hex(&data[ctx_start..ctx_end]),
        nearby_strings: nearby,
        nearby_integers: ints,
    });
}

fn nearby_readable(data: &[u8], center: usize, limit: usize) -> Vec<String> {
    let start = center.saturating_sub(256);
    let end = (center + 256).min(data.len());
    let slice = &data[start..end];
    let mut out = Vec::new();
    for hit in extract_ascii_hits(slice, 0, "raw") {
        if !out.contains(&hit.value) {
            out.push(hit.value);
        }
        if out.len() >= limit {
            break;
        }
    }
    out
}

fn interpret_integers_near(data: &[u8], center: usize) -> Vec<IntegerCandidate> {
    let mut out = Vec::new();
    let start = center.saturating_sub(32);
    let end = (center + 32).min(data.len());
    let mut i = start;
    while i + 8 <= end && out.len() < 24 {
        let abs = i as u64;
        if i + 2 <= data.len() {
            let le = u16::from_le_bytes([data[i], data[i + 1]]);
            let be = u16::from_be_bytes([data[i], data[i + 1]]);
            out.push(int_cand(abs, 16, "le", false, le.to_string()));
            out.push(int_cand(abs, 16, "be", false, be.to_string()));
            out.push(int_cand(abs, 16, "le", true, (le as i16).to_string()));
        }
        if i + 4 <= data.len() {
            let le = u32::from_le_bytes(data[i..i + 4].try_into().unwrap_or([0; 4]));
            let be = u32::from_be_bytes(data[i..i + 4].try_into().unwrap_or([0; 4]));
            out.push(int_cand(abs, 32, "le", false, le.to_string()));
            out.push(int_cand(abs, 32, "be", false, be.to_string()));
            out.push(int_cand(abs, 32, "le", true, (le as i32).to_string()));
            // Rough timestamp/date heuristic notes via value only.
            if (1_000_000_000..2_200_000_000).contains(&le) {
                out.push(int_cand(
                    abs,
                    32,
                    "le",
                    false,
                    format!("{le} (mögl. unix-ts)"),
                ));
            }
        }
        if i + 8 <= data.len() {
            let le = u64::from_le_bytes(data[i..i + 8].try_into().unwrap_or([0; 8]));
            let be = u64::from_be_bytes(data[i..i + 8].try_into().unwrap_or([0; 8]));
            out.push(int_cand(abs, 64, "le", false, le.to_string()));
            out.push(int_cand(abs, 64, "be", false, be.to_string()));
        }
        i += 2;
    }
    out
}

fn int_cand(offset: u64, width: u8, endian: &str, signed: bool, value: String) -> IntegerCandidate {
    IntegerCandidate {
        offset,
        width,
        endian: endian.into(),
        signed,
        value,
    }
}

fn find_repeated_patterns(
    strings: &[ForensicStringHit],
    window: &[u8],
) -> Vec<RepeatedPatternGroup> {
    let mut map: HashMap<&str, Vec<u64>> = HashMap::new();
    for s in strings {
        if s.value.len() < 5 {
            continue;
        }
        map.entry(s.value.as_str()).or_default().push(s.offset);
    }
    let mut groups = Vec::new();
    for (value, offsets) in map {
        if offsets.len() < 2 {
            continue;
        }
        let mut similar = 0u32;
        if offsets.len() >= 2 {
            let ctxs: Vec<_> = offsets
                .iter()
                .take(6)
                .filter_map(|off| {
                    let o = *off as usize;
                    if o >= window.len() {
                        return None;
                    }
                    let a = o.saturating_sub(8);
                    let b = (o + value.len() + 8).min(window.len());
                    Some(bytes_to_hex(&window[a..b]))
                })
                .collect();
            if ctxs.len() >= 2 {
                let first = &ctxs[0];
                similar = ctxs.iter().filter(|c| *c == first).count() as u32;
            }
        }
        groups.push(RepeatedPatternGroup {
            value: value.into(),
            occurrences: offsets.len() as u32,
            offsets: offsets.clone(),
            similar_context_count: similar,
            note: if similar >= 2 {
                "Ähnliche Byte-Nachbarschaft — mögliches Record-Feld".into()
            } else {
                "Mehrfachvorkommen, Kontexte unterschiedlich".into()
            },
        });
    }
    groups.sort_by(|a, b| b.occurrences.cmp(&a.occurrences));
    groups.truncate(40);
    groups
}

fn build_entropy_map(data: &[u8]) -> Vec<EntropyBlock> {
    let mut out = Vec::new();
    let mut offset = 0usize;
    while offset < data.len() {
        let end = (offset + ENTROPY_BLOCK).min(data.len());
        let slice = &data[offset..end];
        let ent = shannon_entropy(slice);
        let kind = if ent < 3.5 {
            "textartig"
        } else if ent > 7.2 {
            "komprimiert/hoch"
        } else {
            "gemischt"
        };
        out.push(EntropyBlock {
            offset: offset as u64,
            size: (end - offset) as u64,
            entropy: ent,
            kind: kind.into(),
        });
        offset = end;
    }
    out
}

fn shannon_entropy(data: &[u8]) -> f32 {
    if data.is_empty() {
        return 0.0;
    }
    let mut counts = [0u32; 256];
    for &b in data {
        counts[b as usize] += 1;
    }
    let len = data.len() as f32;
    let mut ent = 0.0f32;
    for c in counts {
        if c == 0 {
            continue;
        }
        let p = c as f32 / len;
        ent -= p * p.log2();
    }
    ent
}

fn promising_offsets(strings: &[ForensicStringHit], groups: &[RepeatedPatternGroup]) -> Vec<u64> {
    let mut out = Vec::new();
    for g in groups
        .iter()
        .filter(|g| g.similar_context_count >= 2)
        .take(10)
    {
        out.extend(g.offsets.iter().take(3));
    }
    for s in strings.iter().filter(|s| {
        let l = s.value.to_ascii_lowercase();
        l.contains("manager")
            || l.contains("club")
            || l.contains("season")
            || l.contains("liga")
            || l.contains("verein")
    }) {
        out.push(s.offset);
    }
    out.sort_unstable();
    out.dedup();
    out.truncate(32);
    out
}

/// Load a bounded read-only window for probing (never the whole save).
pub fn load_probe_window(path_str: &str) -> Result<Vec<u8>, String> {
    let path = Path::new(path_str);
    let mut file = File::open(path).map_err(|e| map_io_error(&e, path).message)?;
    let mut buf = vec![0u8; DECOMPRESS_WINDOW];
    let n = file
        .read(&mut buf)
        .map_err(|e| map_io_error(&e, path).message)?;
    buf.truncate(n);
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_ascii_and_utf16_in_window() {
        let mut data = b"....Manager Name....".to_vec();
        let utf16: Vec<u8> = "Club"
            .encode_utf16()
            .flat_map(|c| c.to_le_bytes())
            .collect();
        data.extend_from_slice(&utf16);
        let report = build_forensic_report(&data, data.len() as u64, data.len() as u64, &[]);
        assert!(report.log.strings_by_encoding.ascii > 0);
        assert!(report.strings.iter().any(|s| s.value.contains("Manager")));
    }

    #[test]
    fn probe_finds_utf16_club() {
        let utf16: Vec<u8> = "TSV 1860"
            .encode_utf16()
            .flat_map(|c| c.to_le_bytes())
            .collect();
        let mut data = vec![0u8; 32];
        data.extend_from_slice(&utf16);
        data.extend_from_slice(&[0u8; 16]);
        let result = probe_known_value(&data, "TSV 1860", "club");
        assert!(!result.matches.is_empty());
        assert!(result.encodings_hit.iter().any(|e| e.contains("utf16")));
    }
}
