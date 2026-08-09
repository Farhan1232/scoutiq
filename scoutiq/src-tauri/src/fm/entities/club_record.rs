//! Club DB record: repeated unique-id header + length-prefixed full/short names.
//!
//! Verified FM26 layout before `LP(full) LP(short)`:
//!
//! ```text
//! club_uid | club_uid | 0x00 | secondary_block | next_u32 | flags… | LP(full) | LP(short)
//! ```
//!
//! `secondary_block` is one of:
//! - Pattern A: `sec | 0xFFFFFFFF | sec | sec`
//! - Pattern B: `0xFFFFFFFF | sec | sec | sec`
//!
//! In both patterns `sec` is a **nation** id (Germany=145, England=139), not the
//! club. The club's own uid comes only from the `uid | uid | 0x00` header, and a
//! record without that header is rejected — reporting `sec` as the club id was a
//! bug that mislabelled every custom-database club as its country.
//!
//! `division_tier` is `(next_u32 >> 8) & 0xff` and is **not** a division tier.
//! It is retained only as an opaque hint: across the Macclesfield promotion pair
//! it is byte-identical (217 for the managed club, 4 for the base-database one)
//! and Salford's evaluates to 54. Do not use it to infer a league.

use super::types::{Evidence, EvidenceKind};

#[derive(Debug, Clone)]
pub struct ClubRecordHit {
    pub club_id: u64,
    /// Id used for club→competition link scans (may differ from `club_id`).
    pub link_id: u64,
    /// Pattern A/B secondary id from the FF block (may be nation-scoped).
    pub secondary_id: u64,
    pub full_name: String,
    pub short_name: String,
    /// Division depth hint from the club record (`None` if unavailable).
    pub division_tier: Option<u8>,
    /// True when Pattern A `sec` differs from `club_uid` (nation-scoped secondary).
    pub nation_scoped_secondary: bool,
    pub offset: usize,
    pub evidence: Vec<Evidence>,
}

/// Find club record matching the manager-pair short name.
pub fn find_club_record(data: &[u8], short_name: &str) -> Option<ClubRecordHit> {
    find_club_record_with_hint(data, short_name, None)
}

/// Lead-in kept before a club-name match: enough for `LP(full)` (≤120 bytes)
/// and the 64-byte id-header window [`try_parse_at_short`] walks backwards over.
pub const CLUB_WINDOW_BACK: usize = 512;
/// Bytes kept after a club-name match.
pub const CLUB_WINDOW_FWD: usize = 64;

/// Cut a small window around every occurrence of `needle` preceded by its own
/// u32 length — the shape of a club record's `LP(short)` field.
///
/// This exists so a caller streaming `game_db` never has to hold the section in
/// memory: it keeps ~600 bytes per candidate instead of hundreds of megabytes,
/// and the windows parse through [`find_club_record_with_hint`] unchanged.
///
/// `carried` is how many leading bytes of `buf` the caller already examined as
/// the previous chunk's tail. A match lying wholly inside them was emitted then
/// and is skipped now, so a record on a chunk boundary is neither lost nor
/// duplicated. Collection stops once `out` reaches `cap`.
pub fn collect_club_windows(
    buf: &[u8],
    carried: usize,
    needle: &[u8],
    out: &mut Vec<Vec<u8>>,
    cap: usize,
) {
    if needle.len() < 2 || out.len() >= cap {
        return;
    }
    // First-byte test before the slice compare: this runs over every byte of
    // `game_db`, so the inner loop has to stay a single load and branch.
    let first = needle[0];
    let mut i = 4usize;
    while i + needle.len() <= buf.len() {
        if buf[i] != first || &buf[i..i + needle.len()] != needle {
            i += 1;
            continue;
        }
        // The length prefix immediately before the name is what makes this a
        // record field rather than the same text appearing in running data.
        let declared = u32::from_le_bytes(buf[i - 4..i].try_into().unwrap()) as usize;
        if declared == needle.len() && i + needle.len() > carried {
            let start = i.saturating_sub(CLUB_WINDOW_BACK);
            let end = (i + needle.len() + CLUB_WINDOW_FWD).min(buf.len());
            out.push(buf[start..end].to_vec());
            if out.len() >= cap {
                return;
            }
        }
        i += 1;
    }
}

/// Find the club record for `short_name`, preferring the one whose id matches
/// `expected_uid`.
///
/// Club names are not unique — a save built on a custom database can hold
/// several clubs sharing a display name, so the first name match is not
/// necessarily the managed one. The hint comes from the `0E 01` summary record
/// (see [`super::human_manager`]).
///
/// **Two id spaces.** The hint is a *name-table* uid, while this record carries
/// the *compact-table* id, and `name_table_uid = compact_id + 1`. Verified:
/// Aachen 899/900, Leverkusen 900/901, Duisburg 932/933, Salford 5103841/5103842.
/// A hint therefore matches on either value; comparing only for equality makes
/// the hint silently never fire.
pub fn find_club_record_with_hint(
    data: &[u8],
    short_name: &str,
    expected_uid: Option<u32>,
) -> Option<ClubRecordHit> {
    let needle = short_name.as_bytes();
    if needle.len() < 2 {
        return None;
    }

    let mut first: Option<ClubRecordHit> = None;
    let mut i = 0usize;
    while i + needle.len() <= data.len() {
        if &data[i..i + needle.len()] != needle {
            i += 1;
            continue;
        }
        if i < 4 {
            i += 1;
            continue;
        }
        let short_len = match data.get(i - 4..i).and_then(|s| <[u8; 4]>::try_from(s).ok()) {
            Some(b) => u32::from_le_bytes(b) as usize,
            None => {
                i += 1;
                continue;
            }
        };
        if short_len != needle.len() {
            i += 1;
            continue;
        }

        if let Some(hit) = try_parse_at_short(data, i, short_name) {
            match expected_uid {
                // Without a hint the first structurally valid record wins.
                None => return Some(hit),
                Some(want) if matches_uid(hit.club_id, want) => return Some(hit),
                Some(_) => {
                    if first.is_none() {
                        first = Some(hit);
                    }
                }
            }
        }
        i += 1;
    }
    // Hint given but never matched — fall back rather than losing the club.
    first
}

/// A name-table uid matches a compact-table id either directly or off by one
/// (`name_table_uid = compact_id + 1`).
pub fn matches_uid(compact_id: u64, name_table_uid: u32) -> bool {
    let want = name_table_uid as u64;
    compact_id == want || compact_id + 1 == want
}

/// Every structurally valid club record in `data` carrying `short_name`.
pub fn club_record_candidates(data: &[u8], short_name: &str) -> Vec<ClubRecordHit> {
    let needle = short_name.as_bytes();
    let mut out = Vec::new();
    if needle.len() < 2 {
        return out;
    }
    let first = needle[0];
    let mut i = 4usize;
    while i + needle.len() <= data.len() {
        if data[i] != first || &data[i..i + needle.len()] != needle {
            i += 1;
            continue;
        }
        let declared = u32::from_le_bytes(data[i - 4..i].try_into().unwrap()) as usize;
        if declared == needle.len() {
            if let Some(hit) = try_parse_at_short(data, i, short_name) {
                out.push(hit);
            }
        }
        i += 1;
    }
    out
}

/// Pick the managed club's record out of several candidate buffers.
///
/// **With a uid hint** the hint decides: the search prefers the record whose id
/// matches it and otherwise yields the first, so a caller can see a namesake and
/// reject it explicitly (that is how the custom Macclesfield club is told apart
/// from the base-database "Macclesfield Town").
///
/// **Without a hint** a display name is not an identifier — a save may hold two
/// clubs sharing one. A record is adopted only when every candidate that parses
/// agrees on the same club id; a genuine collision returns `None` rather than
/// the first match, because picking one would be a guess.
pub fn resolve_club_record(
    buffers: &[&[u8]],
    short_name: &str,
    hint: Option<u32>,
) -> Option<ClubRecordHit> {
    if hint.is_some() {
        return buffers
            .iter()
            .find_map(|b| find_club_record_with_hint(b, short_name, hint));
    }
    let mut chosen: Option<ClubRecordHit> = None;
    for b in buffers {
        for cand in club_record_candidates(b, short_name) {
            match &chosen {
                None => chosen = Some(cand),
                Some(c) if c.club_id == cand.club_id => {}
                // Two different clubs answer to this name: refuse to choose.
                Some(_) => return None,
            }
        }
    }
    chosen
}

fn try_parse_at_short(data: &[u8], short_at: usize, short_name: &str) -> Option<ClubRecordHit> {
    for full_len in 3..120 {
        let Some(full_start) = short_at.checked_sub(4 + full_len) else {
            continue;
        };
        let Some(pret) = full_start.checked_sub(4) else {
            continue;
        };
        let Some(len_slice) = data.get(pret..full_start) else {
            continue;
        };
        let Ok(len_bytes) = <[u8; 4]>::try_from(len_slice) else {
            continue;
        };
        let declared = u32::from_le_bytes(len_bytes) as usize;
        if declared != full_len {
            continue;
        }
        let Some(bytes) = data.get(full_start..short_at - 4) else {
            continue;
        };
        if !bytes.iter().all(|&b| is_name_byte(b)) {
            continue;
        }
        let full = decode_name(bytes);
        if full.contains(" II") || full.contains(" Reserve") {
            continue;
        }
        if !names_related(&full, short_name) {
            continue;
        }

        // Wider window: club_uid pair sits before the FF secondary block.
        let win_start = pret.saturating_sub(64);
        let win = &data[win_start..pret];
        if let Some(ids) = find_club_ids(win) {
            let abs_after = win_start + ids.pattern_end;
            let division_tier = data
                .get(abs_after..abs_after + 4)
                .and_then(|s| <[u8; 4]>::try_from(s).ok())
                .map(|b| {
                    let next = u32::from_le_bytes(b);
                    ((next >> 8) & 0xff) as u8
                });
            return Some(ClubRecordHit {
                club_id: ids.club_id as u64,
                link_id: ids.link_id as u64,
                secondary_id: ids.secondary_id as u64,
                full_name: full,
                short_name: short_name.to_string(),
                division_tier,
                nation_scoped_secondary: ids.nation_scoped_secondary,
                offset: pret,
                evidence: vec![
                    Evidence {
                        kind: EvidenceKind::StructuralRecord,
                        detail: format!(
                            "Club DB record: id={} link_id={} secondary={} tier_hint={:?} nation_scoped={} LP(full)/LP(short)",
                            ids.club_id,
                            ids.link_id,
                            ids.secondary_id,
                            division_tier,
                            ids.nation_scoped_secondary
                        ),
                        block_index: None,
                        offset: Some(pret as u64),
                    },
                    Evidence {
                        kind: EvidenceKind::RelationshipLink,
                        detail: "Club id header (uid·uid·0·secondary FF-block)".into(),
                        block_index: None,
                        offset: Some(pret as u64),
                    },
                ],
            });
        }
    }
    None
}

#[derive(Debug, Clone, Copy)]
struct ClubIds {
    club_id: u32,
    link_id: u32,
    secondary_id: u32,
    nation_scoped_secondary: bool,
    pattern_end: usize,
}

/// Nation ids are small; club ids are not bounded — editor/custom-database
/// clubs use ids far above the base-database range (e.g. 2000095665).
const MAX_NATION_ID: u32 = 500_000;

fn is_entity_id(v: u32) -> bool {
    v > 0 && v != u32::MAX
}

/// Returns ids and end offset within `win` after the 16-byte secondary FF block.
///
/// The club's own uid always comes from the `uid | uid | 0x00` header preceding
/// the FF block. If that header is missing the record is rejected outright: the
/// secondary slot holds a *nation* id, and returning it as the club id silently
/// mislabels every custom-database club as its country.
fn find_club_ids(win: &[u8]) -> Option<ClubIds> {
    if win.len() < 16 {
        return None;
    }
    for i in 0..=win.len() - 16 {
        let a = u32::from_le_bytes(win[i..i + 4].try_into().unwrap());
        let b = u32::from_le_bytes(win[i + 4..i + 8].try_into().unwrap());
        let c = u32::from_le_bytes(win[i + 8..i + 12].try_into().unwrap());
        let d = u32::from_le_bytes(win[i + 12..i + 16].try_into().unwrap());

        // Pattern A: sec | FFFFFFFF | sec | sec
        let secondary = if a > 0 && a < MAX_NATION_ID && b == u32::MAX && c == a && d == a {
            Some(a)
        // Pattern B: FFFFFFFF | sec | sec | sec
        } else if a == u32::MAX && b > 0 && b < MAX_NATION_ID && c == b && d == b {
            Some(b)
        } else {
            None
        };

        let Some(secondary) = secondary else { continue };
        let Some(club_id) = find_preceding_uid(win, i) else {
            continue;
        };

        return Some(ClubIds {
            club_id,
            link_id: club_id,
            secondary_id: secondary,
            nation_scoped_secondary: club_id != secondary,
            pattern_end: i + 16,
        });
    }
    None
}

/// `club_uid | club_uid | 0x00` immediately before the secondary FF block.
fn find_preceding_uid(win: &[u8], secondary_at: usize) -> Option<u32> {
    // Prefer exact `uid uid 00` packing (9 bytes) right before secondary.
    if secondary_at >= 9 {
        let at = secondary_at - 9;
        let u0 = u32::from_le_bytes(win[at..at + 4].try_into().unwrap());
        let u1 = u32::from_le_bytes(win[at + 4..at + 8].try_into().unwrap());
        let pad = win[at + 8];
        if is_entity_id(u0) && u0 == u1 && pad == 0 {
            return Some(u0);
        }
    }
    // Fallback: `uid uid` with 4-byte alignment in the 24 bytes before secondary.
    let start = secondary_at.saturating_sub(24);
    let mut j = start;
    while j + 8 <= secondary_at {
        let u0 = u32::from_le_bytes(win[j..j + 4].try_into().unwrap());
        let u1 = u32::from_le_bytes(win[j + 4..j + 8].try_into().unwrap());
        if is_entity_id(u0) && u0 == u1 {
            return Some(u0);
        }
        j += 1;
    }
    None
}

fn names_related(full: &str, short: &str) -> bool {
    if full == short || full.contains(short) {
        return true;
    }
    let tokens: Vec<_> = short
        .split_whitespace()
        .filter(|t| t.chars().count() >= 3)
        .collect();
    if tokens.is_empty() {
        return false;
    }
    tokens.iter().filter(|t| full.contains(*t)).count() > 0
}

pub fn is_name_byte(b: u8) -> bool {
    matches!(b, 0x20..=0x7E) || b >= 0x80
}

pub fn decode_name(bytes: &[u8]) -> String {
    // FM26 club/competition LPs are typically UTF-8 (e.g. "Fußball" = C3 9F).
    // Latin-1-per-byte decoding produces classic mojibake like "FuÃball".
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    bytes
        .iter()
        .map(|&b| {
            if b < 0x80 {
                b as char
            } else {
                char::from_u32(b as u32).unwrap_or('\u{FFFD}')
            }
        })
        .collect()
}

pub fn read_lp_name(data: &[u8], at: usize) -> Option<(String, usize)> {
    if at + 4 > data.len() {
        return None;
    }
    let len = u32::from_le_bytes(data[at..at + 4].try_into().ok()?) as usize;
    if !(2..=90).contains(&len) || at + 4 + len > data.len() {
        return None;
    }
    let bytes = &data[at + 4..at + 4 + len];
    if !bytes.iter().all(|&b| is_name_byte(b)) {
        return None;
    }
    let s = decode_name(bytes);
    if !s.chars().any(|c| c.is_alphabetic()) {
        return None;
    }
    Some((s, at + 4 + len))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pattern A with the `uid uid 00` header present.
    #[test]
    fn finds_id_ff_id_id_pattern() {
        let mut data = Vec::new();
        data.extend_from_slice(&[0u8; 8]);
        let uid = 933u32;
        let nation = 145u32;
        data.extend_from_slice(&uid.to_le_bytes());
        data.extend_from_slice(&uid.to_le_bytes());
        data.push(0x00);
        data.extend_from_slice(&nation.to_le_bytes());
        data.extend_from_slice(&u32::MAX.to_le_bytes());
        data.extend_from_slice(&nation.to_le_bytes());
        data.extend_from_slice(&nation.to_le_bytes());
        data.extend_from_slice(&0x231u32.to_le_bytes());
        data.extend_from_slice(&[0x02, 0x08, 0x00, 0x10, 0x01, 0x00]);
        let full = b"Testverein Duisburg 04";
        let short = b"Duisburg 04";
        data.extend_from_slice(&(full.len() as u32).to_le_bytes());
        data.extend_from_slice(full);
        data.extend_from_slice(&(short.len() as u32).to_le_bytes());
        data.extend_from_slice(short);

        let hit = find_club_record(&data, "Duisburg 04").expect("club");
        assert_eq!(hit.club_id, 933);
        assert_eq!(hit.secondary_id, 145);
        assert!(hit.nation_scoped_secondary);
    }

    /// Pattern B with the `uid uid 00` header present.
    #[test]
    fn finds_ff_id_id_id_pattern() {
        let mut data = Vec::new();
        data.extend_from_slice(&[0u8; 8]);
        let uid = 677u32;
        let nation = 139u32;
        data.extend_from_slice(&uid.to_le_bytes());
        data.extend_from_slice(&uid.to_le_bytes());
        data.push(0x00);
        data.extend_from_slice(&u32::MAX.to_le_bytes());
        data.extend_from_slice(&nation.to_le_bytes());
        data.extend_from_slice(&nation.to_le_bytes());
        data.extend_from_slice(&nation.to_le_bytes());
        data.extend_from_slice(&0x432u32.to_le_bytes());
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x10, 0x00, 0x00]);
        let full = b"Macclesfield Town";
        let short = b"Macclesfield";
        data.extend_from_slice(&(full.len() as u32).to_le_bytes());
        data.extend_from_slice(full);
        data.extend_from_slice(&(short.len() as u32).to_le_bytes());
        data.extend_from_slice(short);

        let hit = find_club_record(&data, "Macclesfield").expect("club");
        assert_eq!(hit.club_id, 677);
        assert_eq!(hit.secondary_id, 139);
        assert_eq!(hit.full_name, "Macclesfield Town");
    }

    /// A nation id in the secondary slot must never be reported as the club.
    #[test]
    fn record_without_uid_header_is_rejected() {
        let mut data = Vec::new();
        data.extend_from_slice(&[0u8; 16]);
        let nation = 139u32;
        data.extend_from_slice(&u32::MAX.to_le_bytes());
        data.extend_from_slice(&nation.to_le_bytes());
        data.extend_from_slice(&nation.to_le_bytes());
        data.extend_from_slice(&nation.to_le_bytes());
        data.extend_from_slice(&0x432u32.to_le_bytes());
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x10, 0x00, 0x00]);
        let full = b"Macclesfield Town";
        let short = b"Macclesfield";
        data.extend_from_slice(&(full.len() as u32).to_le_bytes());
        data.extend_from_slice(full);
        data.extend_from_slice(&(short.len() as u32).to_le_bytes());
        data.extend_from_slice(short);

        assert!(find_club_record(&data, "Macclesfield").is_none());
    }

    #[test]
    fn prefers_club_uid_over_nation_secondary() {
        let mut data = Vec::new();
        data.extend_from_slice(&[0u8; 8]);
        let uid = 900u32;
        let nation = 145u32;
        data.extend_from_slice(&uid.to_le_bytes());
        data.extend_from_slice(&uid.to_le_bytes());
        data.push(0x00);
        data.extend_from_slice(&nation.to_le_bytes());
        data.extend_from_slice(&u32::MAX.to_le_bytes());
        data.extend_from_slice(&nation.to_le_bytes());
        data.extend_from_slice(&nation.to_le_bytes());
        data.extend_from_slice(&0x21bu32.to_le_bytes());
        data.extend_from_slice(&[0x02, 0x0a, 0x01, 0x10, 0x00, 0x00]);
        let full = "Bayer 04 Leverkusen Fußball".as_bytes();
        let short = b"Bayer Leverkusen";
        data.extend_from_slice(&(full.len() as u32).to_le_bytes());
        data.extend_from_slice(full);
        data.extend_from_slice(&(short.len() as u32).to_le_bytes());
        data.extend_from_slice(short);

        let hit = find_club_record(&data, "Bayer Leverkusen").expect("club");
        assert_eq!(hit.club_id, 900);
        assert_eq!(hit.link_id, 900);
        assert_eq!(hit.secondary_id, 145);
        assert!(hit.nation_scoped_secondary);
        assert_eq!(hit.full_name, "Bayer 04 Leverkusen Fußball");
        assert_eq!(hit.division_tier, Some(2));
    }

    /// Two clubs share a display name; the uid hint must pick the managed one.
    #[test]
    fn hint_selects_the_managed_club_among_name_collisions() {
        fn club(uid: u32, nation: u32, full: &str, short: &str) -> Vec<u8> {
            let mut d = vec![0u8; 8];
            d.extend_from_slice(&uid.to_le_bytes());
            d.extend_from_slice(&uid.to_le_bytes());
            d.push(0x00);
            d.extend_from_slice(&nation.to_le_bytes());
            d.extend_from_slice(&u32::MAX.to_le_bytes());
            d.extend_from_slice(&nation.to_le_bytes());
            d.extend_from_slice(&nation.to_le_bytes());
            d.extend_from_slice(&0x432u32.to_le_bytes());
            d.extend_from_slice(&[0x00, 0x00, 0x00, 0x10, 0x00, 0x00]);
            d.extend_from_slice(&(full.len() as u32).to_le_bytes());
            d.extend_from_slice(full.as_bytes());
            d.extend_from_slice(&(short.len() as u32).to_le_bytes());
            d.extend_from_slice(short.as_bytes());
            d
        }

        // Base-database club first, then the custom-database club that is managed.
        let mut data = club(677, 139, "Macclesfield Town", "Macclesfield");
        data.extend(club(2_000_095_665, 139, "Macclesfield FC", "Macclesfield"));

        // No hint: first match wins (the base-database club).
        assert_eq!(
            find_club_record(&data, "Macclesfield").unwrap().club_id,
            677
        );
        // Exact hint: the managed club, despite its id exceeding the old bound.
        let hit =
            find_club_record_with_hint(&data, "Macclesfield", Some(2_000_095_665)).expect("club");
        assert_eq!(hit.club_id, 2_000_095_665);
        assert_eq!(hit.full_name, "Macclesfield FC");
        // Name-table uid is compact_id + 1, so the +1 form must match too.
        let hit =
            find_club_record_with_hint(&data, "Macclesfield", Some(2_000_095_666)).expect("club");
        assert_eq!(hit.club_id, 2_000_095_665);
        // Unmatched hint still returns something rather than losing the club.
        assert_eq!(
            find_club_record_with_hint(&data, "Macclesfield", Some(12345))
                .unwrap()
                .club_id,
            677
        );
    }

    /// The real-save case: the hint is a name-table uid one above the record id.
    #[test]
    fn hint_matches_name_table_uid_one_above_compact_id() {
        // Duisburg: compact record 932, summary hint 933.
        assert!(matches_uid(932, 933));
        assert!(matches_uid(932, 932));
        // Leverkusen 900/901, Salford 5103841/5103842.
        assert!(matches_uid(900, 901));
        assert!(matches_uid(5_103_841, 5_103_842));
        // Not a free pass in the other direction.
        assert!(!matches_uid(933, 932));
        assert!(!matches_uid(900, 903));
    }

    /// Salford's uid is 5103841 — far above the old `< 500_000` bound.
    #[test]
    fn accepts_club_ids_above_the_old_bound() {
        let mut data = vec![0u8; 8];
        let uid = 5_103_841u32;
        let nation = 139u32;
        data.extend_from_slice(&uid.to_le_bytes());
        data.extend_from_slice(&uid.to_le_bytes());
        data.push(0x00);
        data.extend_from_slice(&nation.to_le_bytes());
        data.extend_from_slice(&u32::MAX.to_le_bytes());
        data.extend_from_slice(&nation.to_le_bytes());
        data.extend_from_slice(&nation.to_le_bytes());
        data.extend_from_slice(&0x36c2u32.to_le_bytes());
        data.extend_from_slice(&[0x00, 0x0a, 0x01, 0x10, 0x00, 0x00]);
        let full = b"Salford City Football Club";
        let short = b"Salford";
        data.extend_from_slice(&(full.len() as u32).to_le_bytes());
        data.extend_from_slice(full);
        data.extend_from_slice(&(short.len() as u32).to_le_bytes());
        data.extend_from_slice(short);

        let hit = find_club_record(&data, "Salford").expect("club");
        assert_eq!(hit.club_id, 5_103_841);
        assert_eq!(hit.secondary_id, 139);
    }

    #[test]
    fn decodes_utf8_club_names() {
        let bytes = b"Bayer 04 Leverkusen Fu\xc3\x9fball";
        assert_eq!(decode_name(bytes), "Bayer 04 Leverkusen Fußball");
    }
}
