//! `save_game_summary` section parser (read-only).
//!
//! This section is the load-screen summary FM itself renders, and is tiny
//! (~1–2 KB decompressed) regardless of how large the save is. It holds:
//!
//! ```text
//! LP(loaded competition name) …          // one per loaded league
//! 0E 01 LP(manager name) LP(club name)   // the human manager and their club
//! … packed in-game date …                // u32, year in the high u16
//! … squad shortlist …
//! <28-byte trailer>                      // current competition + its colours
//! ```
//!
//! Resolving the manager and club from here means never scanning the
//! multi-hundred-MB `game_db`, and never guessing from the filename. The
//! trailer is decoded by [`super::current_league`] and carries the club's
//! current primary competition.

use serde::{Deserialize, Serialize};

use super::current_league::{parse_current_league, CurrentLeague};
use super::types::{Evidence, EvidenceKind, ResolutionConfidence};

/// Marker preceding the human manager's person→club name pair.
const HUMAN_PAIR_MARKER: [u8; 2] = [0x0E, 0x01];
/// Plausible in-game years; FM26 starts in 2025 and saves can run decades.
const YEAR_MIN: u16 = 2020;
const YEAR_MAX: u16 = 2100;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSummary {
    pub manager_name: Option<String>,
    pub club_name: Option<String>,
    /// Club unique id when the summary carries it inline (not always present).
    pub club_id_hint: Option<u32>,
    pub in_game_year: Option<u16>,
    /// Leagues loaded in this save — context only, not the managed competition.
    pub loaded_competitions: Vec<String>,
    /// The managed club's current primary competition, from the section trailer.
    pub current_league: Option<CurrentLeague>,
    pub confidence: ResolutionConfidence,
    pub evidence: Vec<Evidence>,
}

fn rd_u32(d: &[u8], at: usize) -> Option<u32> {
    d.get(at..at + 4)
        .map(|s| u32::from_le_bytes(s.try_into().unwrap()))
}

fn is_name_byte(b: u8) -> bool {
    (0x20..=0x7e).contains(&b) || b >= 0x80
}

/// Length-prefixed UTF-8 name with sanity limits.
fn rd_lp_name(d: &[u8], at: usize, max: usize) -> Option<(String, usize)> {
    let len = rd_u32(d, at)? as usize;
    if len < 2 || len > max || at + 4 + len > d.len() {
        return None;
    }
    let b = &d[at + 4..at + 4 + len];
    if !b.iter().all(|&x| is_name_byte(x)) {
        return None;
    }
    let s = std::str::from_utf8(b).ok()?.to_string();
    if !s.chars().any(|c| c.is_alphabetic()) {
        return None;
    }
    Some((s, at + 4 + len))
}

fn person_like(s: &str) -> bool {
    s.contains(' ')
        && (5..=48).contains(&s.chars().count())
        && s.chars().any(|c| c.is_uppercase())
        && !s.contains(',')
        && !s.contains('/')
}

fn club_like(s: &str) -> bool {
    (2..=48).contains(&s.chars().count()) && s.chars().any(|c| c.is_alphabetic())
}

/// All `0E 01` person→club pairs, with the offset just past the club name.
fn find_pairs(d: &[u8]) -> Vec<(usize, String, String, usize)> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 2 < d.len() {
        if d[i..i + 2] == HUMAN_PAIR_MARKER {
            if let Some((person, n1)) = rd_lp_name(d, i + 2, 48) {
                if let Some((club, n2)) = rd_lp_name(d, n1, 48) {
                    if person_like(&person) && club_like(&club) {
                        out.push((i, person, club, n2));
                    }
                }
            }
        }
        i += 1;
    }
    out
}

/// Packed date: a u32 whose high u16 is the in-game year.
fn find_year(d: &[u8], from: usize, window: usize) -> Option<u16> {
    let end = (from + window).min(d.len());
    let mut i = from;
    while i + 4 <= end {
        let v = rd_u32(d, i)?;
        let year = (v >> 16) as u16;
        if (YEAR_MIN..=YEAR_MAX).contains(&year) {
            return Some(year);
        }
        i += 1;
    }
    None
}

/// Competition names appear before the manager pair; collect them for context.
fn collect_loaded_competitions(d: &[u8], before: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 4 < before {
        if let Some((name, next)) = rd_lp_name(d, i, 64) {
            if name.chars().any(|c| c.is_alphabetic()) && name.chars().count() >= 4 {
                out.push(name);
            }
            i = next;
            continue;
        }
        i += 1;
    }
    out
}

/// Parse a decompressed `save_game_summary` payload.
pub fn parse_save_summary(data: &[u8]) -> SaveSummary {
    let mut evidence = Vec::new();
    let pairs = find_pairs(data);

    if pairs.is_empty() {
        evidence.push(Evidence {
            kind: EvidenceKind::Absent,
            detail: "No 0E 01 human-manager→club pair in save_game_summary".into(),
            block_index: None,
            offset: None,
        });
        return SaveSummary {
            manager_name: None,
            club_name: None,
            club_id_hint: None,
            in_game_year: None,
            loaded_competitions: Vec::new(),
            current_league: None,
            confidence: ResolutionConfidence::Unknown,
            evidence,
        };
    }

    // More than one human manager (network save) is ambiguous — do not guess.
    if pairs.len() > 1 {
        evidence.push(Evidence {
            kind: EvidenceKind::Absent,
            detail: format!(
                "{} human-manager pairs found — ambiguous, refusing to guess",
                pairs.len()
            ),
            block_index: None,
            offset: None,
        });
        return SaveSummary {
            manager_name: None,
            club_name: None,
            club_id_hint: None,
            in_game_year: None,
            loaded_competitions: Vec::new(),
            current_league: None,
            confidence: ResolutionConfidence::Unknown,
            evidence,
        };
    }

    let (off, manager, club, after) = pairs.into_iter().next().unwrap();
    evidence.push(Evidence {
        kind: EvidenceKind::StructuralRecord,
        detail: "Unique 0E 01 person→club pair in save_game_summary".into(),
        block_index: None,
        offset: Some(off as u64),
    });
    evidence.push(Evidence {
        kind: EvidenceKind::RelationshipLink,
        detail: format!("Human manager '{manager}' → club '{club}'"),
        block_index: None,
        offset: Some(off as u64),
    });

    // A club id may sit immediately after the name. The field is optional (the
    // 26.0.x layout omits it), so it is accepted only when the packed date
    // follows it immediately — that adjacency, not the magnitude, is what tells
    // an id apart from the date itself. Ids are not bounded: user-created and
    // regenerated clubs carry very large ids (e.g. Salford City = 5_103_842).
    let club_id_hint =
        rd_u32(data, after).filter(|v| *v > 0 && find_year(data, after + 4, 4).is_some());
    if let Some(id) = club_id_hint {
        evidence.push(Evidence {
            kind: EvidenceKind::StructuralRecord,
            detail: format!("Club id {id} inline in summary (verified by adjacent date)"),
            block_index: None,
            offset: Some(after as u64),
        });
    }

    let in_game_year = find_year(data, after, 32);
    if let Some(y) = in_game_year {
        evidence.push(Evidence {
            kind: EvidenceKind::StructuralRecord,
            detail: format!("In-game year {y} from packed date after club name"),
            block_index: None,
            offset: Some(after as u64),
        });
    }

    let loaded_competitions = collect_loaded_competitions(data, off);

    let current_league = parse_current_league(data);
    match &current_league {
        Some(c) => evidence.extend(c.evidence(data.len())),
        None => evidence.push(Evidence {
            kind: EvidenceKind::Absent,
            detail: "save_game_summary trailer did not match the current-competition layout".into(),
            block_index: None,
            offset: None,
        }),
    }

    SaveSummary {
        manager_name: Some(manager),
        club_name: Some(club),
        club_id_hint,
        in_game_year,
        loaded_competitions,
        current_league,
        confidence: ResolutionConfidence::High,
        evidence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lp(s: &str) -> Vec<u8> {
        let mut v = (s.len() as u32).to_le_bytes().to_vec();
        v.extend_from_slice(s.as_bytes());
        v
    }

    fn summary(person: &str, club: &str, club_id: u32, year: u16) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend(lp("Eredivisie"));
        v.extend_from_slice(&HUMAN_PAIR_MARKER);
        v.extend(lp(person));
        v.extend(lp(club));
        v.extend_from_slice(&club_id.to_le_bytes());
        // Packed date: year in the high u16.
        v.extend_from_slice(&(((year as u32) << 16) | 0x1adf).to_le_bytes());
        v
    }

    #[test]
    fn parses_manager_club_id_and_year() {
        let d = summary("Niclas Krieck", "Bayer Leverkusen", 901, 2025);
        let s = parse_save_summary(&d);
        assert_eq!(s.manager_name.as_deref(), Some("Niclas Krieck"));
        assert_eq!(s.club_name.as_deref(), Some("Bayer Leverkusen"));
        assert_eq!(s.club_id_hint, Some(901));
        assert_eq!(s.in_game_year, Some(2025));
        assert_eq!(s.confidence, ResolutionConfidence::High);
        assert!(s.loaded_competitions.iter().any(|c| c == "Eredivisie"));
    }

    #[test]
    fn club_id_absent_when_field_missing() {
        // Date immediately follows the club name (observed in the VVV Venlo save).
        let mut d = Vec::new();
        d.extend_from_slice(&HUMAN_PAIR_MARKER);
        d.extend(lp("Niclas Krieck"));
        d.extend(lp("VVV-Venlo"));
        d.extend_from_slice(&((2025u32 << 16) | 0x5d27).to_le_bytes());
        let s = parse_save_summary(&d);
        assert_eq!(s.club_name.as_deref(), Some("VVV-Venlo"));
        assert_eq!(s.club_id_hint, None);
        assert_eq!(s.in_game_year, Some(2025));
    }

    #[test]
    fn multiple_humans_are_unresolved() {
        let mut d = summary("Alex One", "Club A", 100, 2025);
        d.extend(summary("Bea Two", "Club B", 101, 2025));
        let s = parse_save_summary(&d);
        assert!(s.manager_name.is_none());
        assert_eq!(s.confidence, ResolutionConfidence::Unknown);
    }

    #[test]
    fn never_derives_names_from_filename_bytes() {
        let d = b"Niclas Krieck - Lille.fm and Panathinaikos.fm VVV Venlo.fm";
        let s = parse_save_summary(d);
        assert!(s.manager_name.is_none());
    }
}
