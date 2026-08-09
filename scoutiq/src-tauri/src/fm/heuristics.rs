//! Phase-2 metadata heuristics for FM saves (read-only).
//!
//! Values are only returned when contextual evidence exists.
//! Unreliable guesses are left as `None` / `unknown`.

use std::collections::HashMap;

use super::reader::{extract_all_strings, read_save_sample, LocatedString, METADATA_SAMPLE_BYTES};
use super::types::{
    FmSaveMetadata, FmSaveMetadataFields, MetadataConfidence, MetadataConfidenceMap, MetadataField,
};

const ZSTD_MAGIC: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];

const NOISE: &[&str] = &[
    "football manager",
    "sports interactive",
    "sigames",
    "copyright",
    "microsoft",
    "windows",
    "steam",
    "epic",
    "null",
    "true",
    "false",
    "error",
    "debug",
    "http",
    "https",
    "www.",
    ".dll",
    ".exe",
    "zstd",
    "zlib",
];

const MANAGER_MARKERS: &[&str] = &[
    "manager",
    "head coach",
    "trainer",
    "gaffer",
    "human manager",
];

const CLUB_MARKERS: &[&str] = &["club", "team", "fc ", " verein", "afc", "cf "];

const COMPETITION_MARKERS: &[&str] = &[
    "league",
    "liga",
    "cup",
    "pokal",
    "championship",
    "premier",
    "bundesliga",
    "serie a",
    "la liga",
    "eredivisie",
    "competition",
];

#[derive(Debug, Clone)]
struct Candidate {
    value: String,
    score: i32,
    evidence: Vec<String>,
}

pub fn extract_fm_metadata_path(path_str: &str) -> FmSaveMetadata {
    let sample = match read_save_sample(path_str, METADATA_SAMPLE_BYTES) {
        Ok(s) => s,
        Err(msg) => {
            return empty_metadata(None, vec![msg], Vec::new());
        }
    };
    extract_fm_metadata_from_sample(&sample)
}

pub fn extract_fm_metadata_from_sample(sample: &super::reader::SaveSample) -> FmSaveMetadata {
    let strings = extract_all_strings(&sample.bytes);
    extract_fm_metadata_from_scan(
        &sample.path,
        &sample.file_name,
        sample.file_size,
        &strings,
        &sample.bytes,
        count_zstd_magic(&sample.bytes) as u64,
        sample.bytes.len() as u64,
    )
}

/// Metadata heuristics from a completed one-pass scan (no second file read).
pub fn extract_fm_metadata_from_scan(
    path_str: &str,
    file_name: &str,
    file_size: u64,
    strings: &[LocatedString],
    prefix: &[u8],
    zstd_count: u64,
    bytes_scanned: u64,
) -> FmSaveMetadata {
    let mut notes = Vec::new();
    let errors: Vec<String> = Vec::new();

    let save_name_field = MetadataField::with_value(
        file_name.to_string(),
        MetadataConfidence::Confirmed,
        vec![
            "aus Dateipfad übernommen".into(),
            format!("Pfad: {path_str}"),
        ],
    );

    if zstd_count > 0 {
        notes.push(format!(
            "Zstd-Magic {zstd_count}× erkannt — begrenzte Dekompression für Metadaten aktiv"
        ));
    } else {
        notes.push("Keine Zstd-Magic im Scan — Heuristik auf Rohdaten/Strings".into());
    }

    notes.push(format!(
        "One-Pass: {bytes_scanned} Bytes von {file_size} gescannt; {} Strings (begrenzt)",
        strings.len()
    ));

    let date_field = detect_in_game_date(strings);
    let season_field = detect_season(strings, &date_field);
    let manager_field = detect_manager(strings);
    let club_field = detect_club(strings, &manager_field);
    // Filename is NOT published as club — only forensic probe / developer notes.
    if let Some(from_name) = club_from_filename(file_name) {
        if let Some(name) = &from_name.value {
            notes.push(format!(
                "Dateiname deutet auf Verein „{name}“ — nicht als Metadatum übernommen (fehlende Struktur-Evidenz)"
            ));
        }
    }
    let club_id_field = detect_club_id(strings, &club_field, prefix);
    let competition_field = detect_competition(strings, &club_field);

    FmSaveMetadata {
        manager_name: publish(&manager_field),
        club_name: publish(&club_field),
        club_id: publish(&club_id_field),
        in_game_date: publish(&date_field),
        season: publish(&season_field),
        competition_name: publish(&competition_field),
        save_name: publish(&save_name_field),
        confidence: MetadataConfidenceMap {
            manager_name: manager_field.confidence,
            club_name: club_field.confidence,
            club_id: club_id_field.confidence,
            in_game_date: date_field.confidence,
            season: season_field.confidence,
            competition_name: competition_field.confidence,
            save_name: save_name_field.confidence,
        },
        fields: FmSaveMetadataFields {
            manager_name: manager_field,
            club_name: club_field,
            club_id: club_id_field,
            in_game_date: date_field,
            season: season_field,
            competition_name: competition_field,
            save_name: save_name_field,
        },
        notes,
        errors,
    }
}

fn count_zstd_magic(data: &[u8]) -> usize {
    let mut count = 0usize;
    let mut i = 0usize;
    while i + 4 <= data.len() {
        if data[i..i + 4] == ZSTD_MAGIC {
            count += 1;
            i += 4;
        } else {
            i += 1;
        }
    }
    count
}

fn empty_metadata(
    save_name: Option<String>,
    errors: Vec<String>,
    notes: Vec<String>,
) -> FmSaveMetadata {
    let save_field = match save_name {
        Some(name) => MetadataField::with_value(
            name.clone(),
            MetadataConfidence::Confirmed,
            vec!["aus Dateipfad übernommen".into()],
        ),
        None => MetadataField::unknown(),
    };
    FmSaveMetadata {
        manager_name: None,
        club_name: None,
        club_id: None,
        in_game_date: None,
        season: None,
        competition_name: None,
        save_name: save_field.value.clone(),
        confidence: MetadataConfidenceMap {
            manager_name: MetadataConfidence::Unknown,
            club_name: MetadataConfidence::Unknown,
            club_id: MetadataConfidence::Unknown,
            in_game_date: MetadataConfidence::Unknown,
            season: MetadataConfidence::Unknown,
            competition_name: MetadataConfidence::Unknown,
            save_name: save_field.confidence,
        },
        fields: FmSaveMetadataFields {
            manager_name: MetadataField::unknown(),
            club_name: MetadataField::unknown(),
            club_id: MetadataField::unknown(),
            in_game_date: MetadataField::unknown(),
            season: MetadataField::unknown(),
            competition_name: MetadataField::unknown(),
            save_name: save_field,
        },
        notes,
        errors,
    }
}

/// Only expose values that are at least `likely`. Uncertain stays in debug fields only.
fn publish(field: &MetadataField) -> Option<String> {
    match field.confidence {
        MetadataConfidence::Confirmed | MetadataConfidence::Likely => field.value.clone(),
        MetadataConfidence::Uncertain | MetadataConfidence::Unknown => None,
    }
}

fn is_dedicated_marker(value: &str, markers: &[&str]) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    markers.iter().any(|marker| {
        let m = marker.trim();
        if m.is_empty() {
            return false;
        }
        lower == m
            || lower == format!("{m}:")
            || lower == format!("the {m}")
            || lower == format!("current {m}")
            || lower == format!("human {m}")
            || (lower.starts_with(m)
                && lower.len() <= m.len() + 6
                && lower
                    .chars()
                    .nth(m.len())
                    .is_none_or(|c| matches!(c, ':' | ' ' | '\t')))
    })
}

fn is_noise(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    if lower.len() < 3 {
        return true;
    }
    if NOISE.iter().any(|n| lower.contains(n)) {
        return true;
    }
    // Mostly digits / punctuation only
    let alpha = lower.chars().filter(|c| c.is_alphabetic()).count();
    alpha < 2
}

fn looks_like_person_name(s: &str) -> bool {
    let parts: Vec<_> = s.split_whitespace().collect();
    if parts.len() < 2 || parts.len() > 4 {
        return false;
    }
    parts.iter().all(|p| {
        let chars: Vec<char> = p.chars().collect();
        !chars.is_empty()
            && chars[0].is_uppercase()
            && chars
                .iter()
                .all(|c| c.is_alphabetic() || *c == '-' || *c == '\'')
            && p.len() >= 2
            && p.len() <= 24
    })
}

fn looks_like_club_name(s: &str) -> bool {
    let trimmed = s.trim();
    if trimmed.len() < 3 || trimmed.len() > 48 {
        return false;
    }
    if is_noise(trimmed) {
        return false;
    }
    if looks_like_date(trimmed) || looks_like_season(trimmed) {
        return false;
    }
    // Must contain letters; allow digits (e.g. TSV 1860 München)
    let letters = trimmed.chars().filter(|c| c.is_alphabetic()).count();
    if letters < 3 {
        return false;
    }
    // Reject all-caps codes shorter than 5 (IDs / enums)
    if trimmed.len() < 5
        && trimmed
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
    {
        return false;
    }
    true
}

pub fn looks_like_date(s: &str) -> bool {
    detect_date_in_text(s).is_some()
}

pub fn looks_like_season(s: &str) -> bool {
    detect_season_in_text(s).is_some()
}

pub fn detect_date_in_text(s: &str) -> Option<String> {
    // DD/MM/YYYY or DD.MM.YYYY or DD-MM-YYYY
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + 9 < bytes.len() {
        if bytes[i].is_ascii_digit() {
            if let Some(end) = match_dmy(&s[i..]) {
                let candidate = &s[i..i + end];
                if validate_dmy(candidate) {
                    return Some(normalize_date(candidate));
                }
            }
            if let Some(end) = match_ymd(&s[i..]) {
                let candidate = &s[i..i + end];
                if validate_ymd(candidate) {
                    return Some(normalize_date(candidate));
                }
            }
        }
        i += 1;
    }
    None
}

fn match_dmy(s: &str) -> Option<usize> {
    // D(D) sep M(M) sep YYYY
    let b = s.as_bytes();
    let mut i = 0;
    let mut digits = 0;
    while i < b.len() && b[i].is_ascii_digit() && digits < 2 {
        i += 1;
        digits += 1;
    }
    if digits == 0 || i >= b.len() || !matches!(b[i], b'/' | b'.' | b'-') {
        return None;
    }
    let sep = b[i];
    i += 1;
    digits = 0;
    while i < b.len() && b[i].is_ascii_digit() && digits < 2 {
        i += 1;
        digits += 1;
    }
    if digits == 0 || i >= b.len() || b[i] != sep {
        return None;
    }
    i += 1;
    digits = 0;
    while i < b.len() && b[i].is_ascii_digit() && digits < 4 {
        i += 1;
        digits += 1;
    }
    if digits != 4 {
        return None;
    }
    Some(i)
}

fn match_ymd(s: &str) -> Option<usize> {
    let b = s.as_bytes();
    if b.len() < 10 {
        return None;
    }
    if !(b[0].is_ascii_digit()
        && b[1].is_ascii_digit()
        && b[2].is_ascii_digit()
        && b[3].is_ascii_digit())
    {
        return None;
    }
    if !matches!(b[4], b'/' | b'.' | b'-') {
        return None;
    }
    let sep = b[4];
    let mut i = 5;
    let mut digits = 0;
    while i < b.len() && b[i].is_ascii_digit() && digits < 2 {
        i += 1;
        digits += 1;
    }
    if digits == 0 || i >= b.len() || b[i] != sep {
        return None;
    }
    i += 1;
    digits = 0;
    while i < b.len() && b[i].is_ascii_digit() && digits < 2 {
        i += 1;
        digits += 1;
    }
    if digits == 0 {
        return None;
    }
    Some(i)
}

fn validate_dmy(s: &str) -> bool {
    let Some(sep) = s.chars().find(|c| matches!(c, '/' | '.' | '-')) else {
        return false;
    };
    let parts: Vec<_> = s.split(sep).collect();
    if parts.len() != 3 {
        return false;
    }
    let d: u32 = parts[0].parse().unwrap_or(0);
    let m: u32 = parts[1].parse().unwrap_or(0);
    let y: u32 = parts[2].parse().unwrap_or(0);
    (1..=31).contains(&d) && (1..=12).contains(&m) && (1990..=2100).contains(&y)
}

fn validate_ymd(s: &str) -> bool {
    let Some(sep) = s.chars().find(|c| matches!(c, '/' | '.' | '-')) else {
        return false;
    };
    let parts: Vec<_> = s.split(sep).collect();
    if parts.len() != 3 {
        return false;
    }
    let y: u32 = parts[0].parse().unwrap_or(0);
    let m: u32 = parts[1].parse().unwrap_or(0);
    let d: u32 = parts[2].parse().unwrap_or(0);
    (1..=31).contains(&d) && (1..=12).contains(&m) && (1990..=2100).contains(&y)
}

fn normalize_date(s: &str) -> String {
    s.to_string()
}

pub fn detect_season_in_text(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut i = 0;
    while i + 6 < b.len() {
        if b[i].is_ascii_digit()
            && b.get(i + 1).is_some_and(|c| c.is_ascii_digit())
            && b.get(i + 2).is_some_and(|c| c.is_ascii_digit())
            && b.get(i + 3).is_some_and(|c| c.is_ascii_digit())
            && matches!(b.get(i + 4), Some(b'/' | b'-'))
            && b.get(i + 5).is_some_and(|c| c.is_ascii_digit())
            && b.get(i + 6).is_some_and(|c| c.is_ascii_digit())
        {
            let year: u32 = std::str::from_utf8(&b[i..i + 4])
                .ok()
                .and_then(|x| x.parse().ok())
                .unwrap_or(0);
            let yy: u32 = std::str::from_utf8(&b[i + 5..i + 7])
                .ok()
                .and_then(|x| x.parse().ok())
                .unwrap_or(0);
            if (1990..=2100).contains(&year) && yy == (year + 1) % 100 {
                return Some(format!("{year}/{:02}", yy));
            }
        }
        i += 1;
    }
    None
}

fn score_confidence(score: i32) -> MetadataConfidence {
    if score >= 8 {
        MetadataConfidence::Confirmed
    } else if score >= 5 {
        MetadataConfidence::Likely
    } else if score >= 3 {
        MetadataConfidence::Uncertain
    } else {
        MetadataConfidence::Unknown
    }
}

fn pick_best(mut candidates: Vec<Candidate>) -> MetadataField {
    if candidates.is_empty() {
        return MetadataField::unknown();
    }
    candidates.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.value.cmp(&b.value)));
    let best = &candidates[0];
    // Require clear winner for confirmed/likely — if top two are close and different, downgrade.
    let mut confidence = score_confidence(best.score);
    if candidates.len() > 1 {
        let second = &candidates[1];
        if second.value != best.value && best.score - second.score < 2 {
            confidence = MetadataConfidence::Uncertain;
            let mut evidence = best.evidence.clone();
            evidence.push(format!(
                "Konkurrent „{}“ mit ähnlichem Score ({})",
                second.value, second.score
            ));
            return MetadataField {
                value: Some(best.value.clone()),
                confidence,
                evidence,
            };
        }
    }

    if confidence == MetadataConfidence::Unknown {
        return MetadataField {
            value: None,
            confidence,
            evidence: best.evidence.clone(),
        };
    }

    MetadataField {
        value: Some(best.value.clone()),
        confidence,
        evidence: best.evidence.clone(),
    }
}

fn detect_in_game_date(strings: &[LocatedString]) -> MetadataField {
    let mut counts: HashMap<String, Candidate> = HashMap::new();
    for (idx, s) in strings.iter().enumerate() {
        if let Some(date) = detect_date_in_text(&s.value) {
            let entry = counts.entry(date.clone()).or_insert_with(|| Candidate {
                value: date.clone(),
                score: 0,
                evidence: Vec::new(),
            });
            entry.score += 2;
            entry.evidence.push(format!(
                "Datumsmuster in String @{} ({})",
                s.offset, s.encoding
            ));

            // Nearby season / date keywords boost confidence.
            for j in idx.saturating_sub(2)..=(idx + 2).min(strings.len().saturating_sub(1)) {
                let neigh = strings[j].value.to_ascii_lowercase();
                if neigh.contains("date")
                    || neigh.contains("datum")
                    || neigh.contains("season")
                    || neigh.contains("saison")
                    || neigh.contains("game")
                {
                    entry.score += 2;
                    entry.evidence.push(format!(
                        "nahe Schlüsselwort „{}“ @{}",
                        strings[j].value, strings[j].offset
                    ));
                }
            }
        }
    }
    pick_best(counts.into_values().collect())
}

fn detect_season(strings: &[LocatedString], date_field: &MetadataField) -> MetadataField {
    let mut counts: HashMap<String, Candidate> = HashMap::new();
    for (idx, s) in strings.iter().enumerate() {
        if let Some(season) = detect_season_in_text(&s.value) {
            let entry = counts.entry(season.clone()).or_insert_with(|| Candidate {
                value: season.clone(),
                score: 0,
                evidence: Vec::new(),
            });
            entry.score += 3;
            entry.evidence.push(format!(
                "Saisonmuster in String @{} ({})",
                s.offset, s.encoding
            ));
            for j in idx.saturating_sub(2)..=(idx + 2).min(strings.len().saturating_sub(1)) {
                let neigh = strings[j].value.to_ascii_lowercase();
                if neigh.contains("season") || neigh.contains("saison") {
                    entry.score += 2;
                    entry.evidence.push(format!(
                        "nahe Schlüsselwort „{}“ @{}",
                        strings[j].value, strings[j].offset
                    ));
                }
            }
        }
    }

    // Derive season from date only as uncertain helper evidence — not automatic confirmation.
    if counts.is_empty() {
        if let Some(date) = &date_field.value {
            if let Some(year) = extract_year(date) {
                let season = format!("{year}/{:02}", (year + 1) % 100);
                return MetadataField {
                    value: Some(season),
                    confidence: MetadataConfidence::Uncertain,
                    evidence: vec![
                        format!("aus Datum „{date}“ abgeleitet (nicht direkt gefunden)"),
                        "Ableitung allein reicht nicht für likely/confirmed".into(),
                    ],
                };
            }
        }
        return MetadataField::unknown();
    }

    pick_best(counts.into_values().collect())
}

fn extract_year(date: &str) -> Option<u32> {
    for part in date.split(|c| matches!(c, '/' | '.' | '-')) {
        if part.len() == 4 {
            if let Ok(y) = part.parse::<u32>() {
                if (1990..=2100).contains(&y) {
                    return Some(y);
                }
            }
        }
    }
    None
}

/// Derive a likely club label from FM's default save naming: "Club Name (v02).fm".
fn club_from_filename(file_name: &str) -> Option<MetadataField> {
    let stem = file_name
        .trim_end_matches(".fm")
        .trim_end_matches(".FM")
        .trim_end_matches(".sav")
        .trim_end_matches(".SAV");
    let mut name = stem.to_string();
    // Strip trailing version suffixes like " (v02)" repeated by FM autosave.
    loop {
        let Some(idx) = name.rfind(" (") else {
            break;
        };
        let tail = &name[idx + 2..];
        let is_version = tail.starts_with('v')
            || tail.starts_with('V')
            || tail.chars().next().is_some_and(|c| c.is_ascii_digit());
        if is_version && name[idx..].ends_with(')') {
            name.truncate(idx);
        } else {
            break;
        }
    }
    let name = name.trim();
    if !looks_like_club_name(name) {
        return None;
    }
    Some(MetadataField::with_value(
        name.to_string(),
        MetadataConfidence::Likely,
        vec![
            "aus Speichernamen abgeleitet".into(),
            format!("Dateiname: {file_name}"),
        ],
    ))
}

fn detect_manager(strings: &[LocatedString]) -> MetadataField {
    let mut candidates: Vec<Candidate> = Vec::new();

    for (idx, s) in strings.iter().enumerate() {
        let lower = s.value.to_ascii_lowercase();
        let is_marker = is_dedicated_marker(&s.value, MANAGER_MARKERS)
            || MANAGER_MARKERS
                .iter()
                .any(|m| lower == *m || lower.starts_with(&format!("{m} ")));
        if !is_marker {
            continue;
        }

        // Look at neighbouring strings for a person name — do not use the marker itself.
        for j in idx.saturating_sub(3)..=(idx + 3).min(strings.len().saturating_sub(1)) {
            if j == idx {
                continue;
            }
            let name = strings[j].value.trim();
            if !looks_like_person_name(name) || is_noise(name) {
                continue;
            }
            let distance = idx.abs_diff(j);
            let mut score = 4 - distance as i32;
            let mut evidence = vec![
                format!("nahe Manager-Marker „{}“ @{}", s.value, s.offset),
                format!(
                    "Name-Kandidat „{name}“ @{} ({})",
                    strings[j].offset, strings[j].encoding
                ),
            ];
            // Repeated identical name nearby
            let repeats = strings.iter().filter(|x| x.value == name).count();
            if repeats >= 2 {
                score += 2;
                evidence.push(format!("Name kommt {repeats}× in der Stichprobe vor"));
            }
            candidates.push(Candidate {
                value: name.to_string(),
                score,
                evidence,
            });
        }
    }

    pick_best(merge_candidates(candidates))
}

fn detect_club(strings: &[LocatedString], manager: &MetadataField) -> MetadataField {
    let mut candidates: Vec<Candidate> = Vec::new();

    for (idx, s) in strings.iter().enumerate() {
        if !is_dedicated_marker(&s.value, CLUB_MARKERS) {
            continue;
        }

        for j in idx.saturating_sub(4)..=(idx + 4).min(strings.len().saturating_sub(1)) {
            if j == idx {
                continue;
            }
            let name = strings[j].value.trim();
            if !looks_like_club_name(name) {
                continue;
            }
            // Avoid taking person names as club unless they also look like clubs with FC etc.
            if looks_like_person_name(name) && !name.to_ascii_lowercase().contains("fc") {
                continue;
            }

            let distance = idx.abs_diff(j);
            let mut score = 4 - distance as i32;
            let mut evidence = vec![
                format!("nahe Club-Marker „{}“ @{}", s.value, s.offset),
                format!(
                    "Club-Kandidat „{name}“ @{} ({})",
                    strings[j].offset, strings[j].encoding
                ),
            ];

            let repeats = strings.iter().filter(|x| x.value == name).count();
            if repeats >= 2 {
                score += 2;
                evidence.push(format!("Name kommt {repeats}× vor"));
            }
            if repeats >= 3 {
                score += 1;
            }

            // Proximity to detected manager boosts club confidence.
            if let Some(mgr) = &manager.value {
                if let Some(midx) = strings.iter().position(|x| x.value == *mgr) {
                    if midx.abs_diff(j) <= 6 {
                        score += 3;
                        evidence.push("nahe erkanntem Manager-Eintrag".into());
                    }
                }
            }

            candidates.push(Candidate {
                value: name.to_string(),
                score,
                evidence,
            });
        }
    }

    // Extra: club-like tokens that appear often AND near a date/season (still need markers for likely).
    // We intentionally do NOT promote high-frequency club-like strings without markers —
    // that would be "first/most common club name" guessing.

    pick_best(merge_candidates(candidates))
}

fn detect_club_id(strings: &[LocatedString], club: &MetadataField, data: &[u8]) -> MetadataField {
    let Some(club_name) = &club.value else {
        return MetadataField {
            value: None,
            confidence: MetadataConfidence::Unknown,
            evidence: vec!["kein zuverlässiger Vereinsname — Club-ID nicht ableitbar".into()],
        };
    };

    let club_offsets: Vec<usize> = strings
        .iter()
        .filter(|s| &s.value == club_name)
        .map(|s| s.offset)
        .collect();

    if club_offsets.is_empty() {
        return MetadataField::unknown();
    }

    let mut id_candidates: HashMap<String, Candidate> = HashMap::new();

    for &off in &club_offsets {
        // Prefix buffer may be shorter than absolute file offsets from a full scan.
        if off >= data.len() {
            continue;
        }
        let window_start = off.saturating_sub(32);
        let window_end = (off + club_name.len() + 32).min(data.len());
        let window = &data[window_start..window_end];

        // ASCII digit runs 3–8 length near the club name.
        let mut i = 0;
        while i < window.len() {
            if window[i].is_ascii_digit() {
                let start = i;
                while i < window.len() && window[i].is_ascii_digit() {
                    i += 1;
                }
                let len = i - start;
                if (3..=8).contains(&len) {
                    let id = String::from_utf8_lossy(&window[start..i]).into_owned();
                    let entry = id_candidates
                        .entry(id.clone())
                        .or_insert_with(|| Candidate {
                            value: id.clone(),
                            score: 0,
                            evidence: Vec::new(),
                        });
                    entry.score += 2;
                    entry.evidence.push(format!(
                        "Ziffernfolge „{id}“ nahe Club „{club_name}“ (Fenster @{window_start})"
                    ));
                }
            } else {
                i += 1;
            }
        }
    }

    let mut field = pick_best(id_candidates.into_values().collect());
    // Club IDs are structural guesses without schema — never above uncertain/likely.
    if field.confidence == MetadataConfidence::Confirmed {
        field.confidence = MetadataConfidence::Likely;
        field
            .evidence
            .push("auf likely begrenzt: keine bestätigte FM-ID-Schema-Zuordnung".into());
    }
    if field.value.is_some()
        && field.confidence == MetadataConfidence::Likely
        && field.evidence.len() < 2
    {
        field.confidence = MetadataConfidence::Uncertain;
    }
    field
}

fn detect_competition(strings: &[LocatedString], club: &MetadataField) -> MetadataField {
    let mut candidates: Vec<Candidate> = Vec::new();

    for (idx, s) in strings.iter().enumerate() {
        let lower = s.value.to_ascii_lowercase();
        let marker_hit = COMPETITION_MARKERS.iter().any(|m| lower.contains(m));
        if !marker_hit || is_noise(&s.value) {
            continue;
        }
        // The string itself may be the competition name if long enough and not only a marker word.
        if s.value.len() >= 8 && looks_like_club_name(&s.value) {
            let mut score = 3;
            let mut evidence = vec![format!(
                "Wettbewerbs-ähnlicher String „{}“ @{}",
                s.value, s.offset
            )];
            if let Some(club_name) = &club.value {
                if let Some(cidx) = strings.iter().position(|x| &x.value == club_name) {
                    if cidx.abs_diff(idx) <= 8 {
                        score += 2;
                        evidence.push("nahe erkanntem Verein".into());
                    }
                }
            }
            candidates.push(Candidate {
                value: s.value.clone(),
                score,
                evidence,
            });
        }
    }

    let mut field = pick_best(merge_candidates(candidates));
    // Competitions are especially easy to mis-identify — cap at likely.
    if field.confidence == MetadataConfidence::Confirmed {
        field.confidence = MetadataConfidence::Likely;
        field
            .evidence
            .push("auf likely begrenzt: Wettbewerb ohne Schema-Bestätigung".into());
    }
    field
}

fn merge_candidates(candidates: Vec<Candidate>) -> Vec<Candidate> {
    let mut map: HashMap<String, Candidate> = HashMap::new();
    for c in candidates {
        let entry = map.entry(c.value.clone()).or_insert_with(|| Candidate {
            value: c.value.clone(),
            score: 0,
            evidence: Vec::new(),
        });
        entry.score += c.score;
        for e in c.evidence {
            if !entry.evidence.contains(&e) {
                entry.evidence.push(e);
            }
        }
    }
    map.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fm::reader::LocatedString;

    fn loc(value: &str, offset: usize) -> LocatedString {
        LocatedString {
            value: value.to_string(),
            offset,
            encoding: "ascii",
        }
    }

    #[test]
    fn detects_dmy_and_ymd_dates() {
        assert_eq!(
            detect_date_in_text("played on 12/08/2026 end"),
            Some("12/08/2026".into())
        );
        assert_eq!(
            detect_date_in_text("stamp 2026-08-12 ok"),
            Some("2026-08-12".into())
        );
        assert_eq!(detect_date_in_text("no date here"), None);
        assert_eq!(detect_date_in_text("99/99/2026"), None);
    }

    #[test]
    fn detects_season_patterns() {
        assert_eq!(
            detect_season_in_text("Saison 2025/26"),
            Some("2025/26".into())
        );
        assert_eq!(
            detect_season_in_text("2024-25 league"),
            Some("2024/25".into())
        );
        assert_eq!(detect_season_in_text("2025/28"), None); // not consecutive
    }

    #[test]
    fn confidence_scoring_thresholds() {
        assert_eq!(score_confidence(9), MetadataConfidence::Confirmed);
        assert_eq!(score_confidence(5), MetadataConfidence::Likely);
        assert_eq!(score_confidence(3), MetadataConfidence::Uncertain);
        assert_eq!(score_confidence(1), MetadataConfidence::Unknown);
    }

    #[test]
    fn manager_requires_marker_context() {
        let strings = vec![loc("Alice Example", 10), loc("Bob Example", 40)];
        let field = detect_manager(&strings);
        assert!(field.value.is_none());
        assert_eq!(field.confidence, MetadataConfidence::Unknown);
    }

    #[test]
    fn manager_with_marker_can_be_detected() {
        let strings = vec![
            loc("Human Manager", 0),
            loc("Niclas Krieck", 20),
            loc("Niclas Krieck", 80),
        ];
        let field = detect_manager(&strings);
        assert_eq!(field.value.as_deref(), Some("Niclas Krieck"));
        assert!(matches!(
            field.confidence,
            MetadataConfidence::Likely
                | MetadataConfidence::Confirmed
                | MetadataConfidence::Uncertain
        ));
        assert!(!field.evidence.is_empty());
    }

    #[test]
    fn club_not_chosen_from_frequency_alone() {
        let strings = vec![
            loc("TSV 1860 München", 0),
            loc("TSV 1860 München", 40),
            loc("TSV 1860 München", 80),
            loc("Other Club Name", 120),
        ];
        let field = detect_club(&strings, &MetadataField::unknown());
        // No club markers → must not invent a club from frequency.
        assert!(field.value.is_none());
        assert_eq!(field.confidence, MetadataConfidence::Unknown);
    }

    #[test]
    fn club_with_marker_and_manager_proximity() {
        let strings = vec![
            loc("Manager", 0),
            loc("Niclas Krieck", 10),
            loc("Club", 30),
            loc("TSV 1860 Muenchen", 40),
            loc("TSV 1860 Muenchen", 90),
        ];
        let manager = detect_manager(&strings);
        let club = detect_club(&strings, &manager);
        assert_eq!(club.value.as_deref(), Some("TSV 1860 Muenchen"));
        assert!(!club.evidence.is_empty());
    }

    #[test]
    fn publish_hides_uncertain_values() {
        let uncertain = MetadataField {
            value: Some("Maybe Club".into()),
            confidence: MetadataConfidence::Uncertain,
            evidence: vec!["schwach".into()],
        };
        assert_eq!(publish(&uncertain), None);

        let likely =
            MetadataField::with_value("Likely Club", MetadataConfidence::Likely, vec!["ok".into()]);
        assert_eq!(publish(&likely).as_deref(), Some("Likely Club"));
    }

    #[test]
    fn club_from_fm_autosave_filename() {
        let field = club_from_filename("MSV Duisburg (v02) (v02) (v02) (v02).fm")
            .expect("club from filename");
        assert_eq!(field.value.as_deref(), Some("MSV Duisburg"));
        assert_eq!(field.confidence, MetadataConfidence::Likely);
        assert_eq!(publish(&field).as_deref(), Some("MSV Duisburg"));
    }
}
