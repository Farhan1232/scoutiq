//! Deterministic human-manager → current-club resolver for FM26 `tad.` payloads.
//!
//! Verified across multiple independent saves:
//!
//! ```text
//! 0E 01 | LP(manager name) | LP(club name) | u32 club_uid | u32 date | u32 counter
//! ```
//!
//! The trailing `club_uid` is the managed club's unique id and is what makes it
//! possible to pick the *right* club record when several share a display name
//! (e.g. a custom-database club alongside the base-database one). It is absent
//! on older patches — FM 26.0.6 puts the date directly after the club name — so
//! it is exposed as a hint, not a guarantee.
//!
//! This does NOT hardcode any club or manager names.

use crate::fm::entities::types::{Evidence, EvidenceKind, ResolutionConfidence};

/// In-game years live in the high half of the date word; a value in this range
/// means we are looking at the date, i.e. the save carries no inline club id.
const PLAUSIBLE_YEARS: std::ops::RangeInclusive<u32> = 1900..=2200;

#[derive(Debug, Clone)]
pub struct HumanManagerClubHit {
    pub offset: usize,
    pub manager_name: String,
    pub club_name: String,
    /// Managed club's unique id when the payload carries it inline.
    pub club_id_hint: Option<u32>,
    pub confidence: ResolutionConfidence,
    pub evidence: Vec<Evidence>,
}

/// Find human manager → club pairs using the structural `0E 01` record.
pub fn resolve_human_manager_club(data: &[u8]) -> Option<HumanManagerClubHit> {
    let pairs = find_pairs(data);
    if pairs.is_empty() {
        return None;
    }

    // Across calibration saves the human-managed club yields exactly one pair.
    if pairs.len() == 1 {
        let (offset, manager_name, club_name, after_club) = pairs.into_iter().next().unwrap();
        let club_id_hint = read_club_id_hint(data, after_club);
        let mut evidence = vec![
            Evidence {
                kind: EvidenceKind::StructuralRecord,
                detail: "Unique 0E 01 length-prefixed person→club pair in tad. payload".into(),
                block_index: None,
                offset: Some(offset as u64),
            },
            Evidence {
                kind: EvidenceKind::RelationshipLink,
                detail: "Human manager name immediately followed by current club name".into(),
                block_index: None,
                offset: Some(offset as u64),
            },
        ];
        evidence.push(match club_id_hint {
            Some(id) => Evidence {
                kind: EvidenceKind::RelationshipLink,
                detail: format!("Managed club unique id {id} read inline after the club name"),
                block_index: None,
                offset: Some(after_club as u64),
            },
            None => Evidence {
                kind: EvidenceKind::Absent,
                detail: "No inline club id after the club name (older patch layout)".into(),
                block_index: None,
                offset: Some(after_club as u64),
            },
        });
        return Some(HumanManagerClubHit {
            offset,
            manager_name,
            club_name,
            club_id_hint,
            confidence: ResolutionConfidence::High,
            evidence,
        });
    }

    // Multiple pairs: do not guess — leave unresolved for UI.
    None
}

/// Returns `(marker offset, manager, club, offset just past the club name)`.
fn find_pairs(data: &[u8]) -> Vec<(usize, String, String, usize)> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 2 < data.len() {
        if data[i] == 0x0E && data[i + 1] == 0x01 {
            if let Some((person, next)) = read_lp_name(data, i + 2) {
                if let Some((club, after)) = read_lp_name(data, next) {
                    if person_like(&person) && club_like(&club) {
                        out.push((i, person, club, after));
                    }
                }
            }
        }
        i += 1;
    }
    out
}

/// The word after the club name is the club's unique id — unless this patch
/// writes the date there instead, which the in-game year gives away.
fn read_club_id_hint(data: &[u8], at: usize) -> Option<u32> {
    let bytes = data.get(at..at + 4)?;
    let v = u32::from_le_bytes(bytes.try_into().ok()?);
    if v == 0 || v == u32::MAX {
        return None;
    }
    if PLAUSIBLE_YEARS.contains(&(v >> 16)) {
        return None;
    }
    Some(v)
}

/// Length-prefixed UTF-8 name.
///
/// Bytes at or above `0x80` are accepted and left to `from_utf8` to validate:
/// FM stores these names as UTF-8, so an ASCII-only filter silently loses every
/// club and manager with an accented character — "TSV 1860 München" (`c3 bc`)
/// among them, which made the whole save resolve to no manager at all.
fn read_lp_name(data: &[u8], at: usize) -> Option<(String, usize)> {
    if at + 4 > data.len() {
        return None;
    }
    let len = u32::from_le_bytes(data[at..at + 4].try_into().ok()?) as usize;
    if !(2..=64).contains(&len) || at + 4 + len > data.len() {
        return None;
    }
    let bytes = &data[at + 4..at + 4 + len];
    if !bytes
        .iter()
        .all(|&b| (0x20..=0x7E).contains(&b) || b >= 0x80)
    {
        return None;
    }
    let s = std::str::from_utf8(bytes).ok()?.to_string();
    Some((s, at + 4 + len))
}

fn person_like(s: &str) -> bool {
    s.contains(' ')
        && (5..=48).contains(&s.chars().count())
        && s.chars().any(|c| c.is_uppercase())
        && s.chars().filter(|c| c.is_alphabetic()).count() >= 5
        && !s.contains(',')
        && !s.contains('/')
}

fn club_like(s: &str) -> bool {
    (3..=48).contains(&s.chars().count())
        && s.chars().any(|c| c.is_alphabetic())
        && !s.contains(',')
        && !s.eq_ignore_ascii_case("ENTRY")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synth_pair(person: &str, club: &str) -> Vec<u8> {
        let mut v = vec![0x0E, 0x01];
        let pb = person.as_bytes();
        v.extend_from_slice(&(pb.len() as u32).to_le_bytes());
        v.extend_from_slice(pb);
        let cb = club.as_bytes();
        v.extend_from_slice(&(cb.len() as u32).to_le_bytes());
        v.extend_from_slice(cb);
        v
    }

    #[test]
    fn unique_pair_is_high_confidence() {
        let mut data = vec![0u8; 32];
        data.extend(synth_pair("Alex Manager", "Example FC"));
        let hit = resolve_human_manager_club(&data).expect("hit");
        assert_eq!(hit.manager_name, "Alex Manager");
        assert_eq!(hit.club_name, "Example FC");
        assert_eq!(hit.confidence, ResolutionConfidence::High);
    }

    #[test]
    fn multiple_pairs_yield_none() {
        let mut data = synth_pair("Alex Manager", "Example FC");
        data.extend(synth_pair("Other Person", "Other Club"));
        assert!(resolve_human_manager_club(&data).is_none());
    }

    /// Club and manager names are UTF-8; an ASCII-only reader dropped the pair
    /// entirely and the save resolved to no manager at all.
    #[test]
    fn accepts_non_ascii_club_and_manager_names() {
        let mut data = vec![0u8; 8];
        data.extend(synth_pair("Niclas Krieck", "TSV 1860 München"));
        let hit = resolve_human_manager_club(&data).expect("hit");
        assert_eq!(hit.club_name, "TSV 1860 München");

        let mut data = vec![0u8; 8];
        data.extend(synth_pair("José Peñarroja", "Fußball Club"));
        let hit = resolve_human_manager_club(&data).expect("hit");
        assert_eq!(hit.manager_name, "José Peñarroja");
        assert_eq!(hit.club_name, "Fußball Club");
    }

    /// Invalid UTF-8 must still be rejected rather than lossily accepted.
    #[test]
    fn rejects_invalid_utf8_names() {
        let mut data = vec![0u8; 8];
        data.extend_from_slice(&[0x0E, 0x01]);
        data.extend_from_slice(&12u32.to_le_bytes());
        data.extend_from_slice(b"Alex Man\xffger");
        data.extend_from_slice(&10u32.to_le_bytes());
        data.extend_from_slice(b"Example FC");
        assert!(resolve_human_manager_club(&data).is_none());
    }

    #[test]
    fn does_not_use_filename_bytes() {
        // Noise that looks like a filename must not create a pair without 0E 01.
        let data = b"Niclas Krieck - Lille.fm and Panathinaikos.fm VVV Venlo.fm";
        assert!(resolve_human_manager_club(data).is_none());
    }
}
