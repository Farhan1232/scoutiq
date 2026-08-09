//! FM26 entity name table (read-only).
//!
//! `game_db` stores display names in self-describing tagged records. Field tags
//! are 4 ASCII bytes held little-endian, so they read reversed on disk
//! (`manC` on disk = `Cnam`, `mnsC` = `Csnm`, `srev` = `vers`):
//!
//! ```text
//! "inud" 01 0F <u32 id> <u32 id> "ytrp" 01 02 <tag> "lvwn" 01 1A LP(name)
//! ```
//!
//! Both `id` fields carry the entity's unique id, so a club's numeric id can be
//! recovered from its name without positional guessing.
//!
//! This matters: the previous club-id heuristic read the id block *preceding*
//! the club's name strings, which belongs to the neighbouring record. In the
//! Leverkusen save that yielded 900 (Alemannia Aachen) instead of 901
//! (Bayer Leverkusen), so every downstream id lookup was querying the wrong club.

const T_INUD: [u8; 4] = *b"inud";
const T_YTRP: [u8; 4] = *b"ytrp";
const T_LVWN: [u8; 4] = *b"lvwn";

/// Name-kind tag as stored on disk (reversed 4CC).
pub const TAG_FULL_NAME: [u8; 4] = *b"manC";
pub const TAG_SHORT_NAME: [u8; 4] = *b"mnsC";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameRecord {
    pub id: u32,
    /// Raw on-disk tag; compare against [`TAG_FULL_NAME`] / [`TAG_SHORT_NAME`].
    pub tag: [u8; 4],
    pub name: String,
    pub offset: u64,
}

impl NameRecord {
    pub fn is_short_name(&self) -> bool {
        // Case differs between record families (`mnsC` / `mnsc`).
        self.tag.eq_ignore_ascii_case(&TAG_SHORT_NAME)
    }

    pub fn is_full_name(&self) -> bool {
        self.tag.eq_ignore_ascii_case(&TAG_FULL_NAME)
    }
}

fn rd_u32(d: &[u8], at: usize) -> Option<u32> {
    d.get(at..at + 4)
        .map(|s| u32::from_le_bytes(s.try_into().unwrap()))
}

fn is_name_byte(b: u8) -> bool {
    (0x20..=0x7e).contains(&b) || b >= 0x80
}

/// Minimum bytes a complete name record occupies before the string payload.
const RECORD_PREFIX: usize = 4 + 2 + 4 + 4 + 4 + 2 + 4 + 4 + 2;

/// Scan `buf` for name records, appending to `out`.
/// `base` is the offset of `buf[0]` within the decompressed section.
pub fn scan_name_records(buf: &[u8], base: u64, out: &mut Vec<NameRecord>) {
    let mut i = 0usize;
    while i + RECORD_PREFIX + 4 <= buf.len() {
        if buf[i..i + 4] != T_INUD || buf[i + 4] != 0x01 || buf[i + 5] != 0x0F {
            i += 1;
            continue;
        }
        let (Some(id1), Some(id2)) = (rd_u32(buf, i + 6), rd_u32(buf, i + 10)) else {
            i += 1;
            continue;
        };
        // Both id slots must agree, guarding against a false `inud` match.
        if id1 == 0 || id1 != id2 {
            i += 1;
            continue;
        }
        if buf[i + 14..i + 18] != T_YTRP || buf[i + 18] != 0x01 || buf[i + 19] != 0x02 {
            i += 1;
            continue;
        }
        let tag: [u8; 4] = buf[i + 20..i + 24].try_into().unwrap();
        if buf[i + 24..i + 28] != T_LVWN || buf[i + 28] != 0x01 || buf[i + 29] != 0x1A {
            i += 1;
            continue;
        }
        let at = i + 30;
        let Some(len) = rd_u32(buf, at).map(|v| v as usize) else {
            i += 1;
            continue;
        };
        if len < 2 || len > 96 || at + 4 + len > buf.len() {
            i += 1;
            continue;
        }
        let nb = &buf[at + 4..at + 4 + len];
        if !nb.iter().all(|&b| is_name_byte(b)) {
            i += 1;
            continue;
        }
        let Ok(name) = std::str::from_utf8(nb) else {
            i += 1;
            continue;
        };
        if !name.chars().any(|c| c.is_alphabetic()) {
            i += 1;
            continue;
        }
        out.push(NameRecord {
            id: id1,
            tag,
            name: name.to_string(),
            offset: base + i as u64,
        });
        i += RECORD_PREFIX + len;
    }
}

fn unique_id(mut ids: Vec<u32>) -> Option<u32> {
    ids.sort_unstable();
    ids.dedup();
    match ids.len() {
        1 => Some(ids[0]),
        _ => None,
    }
}

/// Unique id for `club_name`, tried in order of decreasing strictness:
/// exact short name, exact full name, then a full name that begins with the
/// summary name (Salford City FC records only a full name, "Salford City
/// Football Club", while the summary says "Salford").
/// Returns `None` when absent or ambiguous — never a guess.
pub fn club_id_for_name(records: &[NameRecord], club_name: &str) -> Option<u32> {
    let exact_short = records
        .iter()
        .filter(|r| r.is_short_name() && r.name == club_name)
        .map(|r| r.id)
        .collect::<Vec<_>>();
    if let Some(id) = unique_id(exact_short) {
        return Some(id);
    }

    let exact_full = records
        .iter()
        .filter(|r| r.is_full_name() && r.name == club_name)
        .map(|r| r.id)
        .collect::<Vec<_>>();
    if let Some(id) = unique_id(exact_full) {
        return Some(id);
    }

    // Word-boundary prefix only, so "Salford" cannot match "Salfordians".
    let prefix = format!("{club_name} ");
    let prefixed = records
        .iter()
        .filter(|r| r.is_full_name() && r.name.starts_with(&prefix))
        .map(|r| r.id)
        .collect::<Vec<_>>();
    unique_id(prefixed)
}

/// Longest full-name variant recorded for `id`.
pub fn full_name_for_id(records: &[NameRecord], id: u32) -> Option<String> {
    records
        .iter()
        .filter(|r| r.id == id && r.is_full_name())
        .map(|r| r.name.clone())
        .max_by_key(|n| n.chars().count())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: u32, tag: &[u8; 4], name: &str) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&T_INUD);
        v.extend_from_slice(&[0x01, 0x0F]);
        v.extend_from_slice(&id.to_le_bytes());
        v.extend_from_slice(&id.to_le_bytes());
        v.extend_from_slice(&T_YTRP);
        v.extend_from_slice(&[0x01, 0x02]);
        v.extend_from_slice(tag);
        v.extend_from_slice(&T_LVWN);
        v.extend_from_slice(&[0x01, 0x1A]);
        v.extend_from_slice(&(name.len() as u32).to_le_bytes());
        v.extend_from_slice(name.as_bytes());
        v
    }

    #[test]
    fn reads_club_id_from_short_name() {
        let mut d = vec![0u8; 16];
        d.extend(record(901, &TAG_SHORT_NAME, "Bayer Leverkusen"));
        d.extend(record(901, &TAG_FULL_NAME, "Bayer 04 Leverkusen Fußball"));
        d.extend(record(900, &TAG_SHORT_NAME, "Alemannia Aachen"));

        let mut recs = Vec::new();
        scan_name_records(&d, 0, &mut recs);
        assert_eq!(recs.len(), 3);
        assert_eq!(club_id_for_name(&recs, "Bayer Leverkusen"), Some(901));
        assert_eq!(club_id_for_name(&recs, "Alemannia Aachen"), Some(900));
        assert_eq!(
            full_name_for_id(&recs, 901).as_deref(),
            Some("Bayer 04 Leverkusen Fußball")
        );
    }

    #[test]
    fn mismatched_id_slots_are_rejected() {
        let mut d = Vec::new();
        d.extend_from_slice(&T_INUD);
        d.extend_from_slice(&[0x01, 0x0F]);
        d.extend_from_slice(&901u32.to_le_bytes());
        d.extend_from_slice(&902u32.to_le_bytes());
        d.extend_from_slice(&T_YTRP);
        d.extend_from_slice(&[0x01, 0x02]);
        d.extend_from_slice(&TAG_SHORT_NAME);
        d.extend_from_slice(&T_LVWN);
        d.extend_from_slice(&[0x01, 0x1A]);
        d.extend_from_slice(&4u32.to_le_bytes());
        d.extend_from_slice(b"Test");

        let mut recs = Vec::new();
        scan_name_records(&d, 0, &mut recs);
        assert!(recs.is_empty());
    }

    #[test]
    fn ambiguous_name_yields_none() {
        let mut d = Vec::new();
        d.extend(record(10, &TAG_SHORT_NAME, "United"));
        d.extend(record(11, &TAG_SHORT_NAME, "United"));
        let mut recs = Vec::new();
        scan_name_records(&d, 0, &mut recs);
        assert_eq!(club_id_for_name(&recs, "United"), None);
    }
}
