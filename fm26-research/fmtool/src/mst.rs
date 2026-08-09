//! `mst.` section decoder — the numerically named `.scm` / `.apm` container family.
//!
//! Layout verified byte-for-byte across saves (all offsets are absolute within
//! the decompressed section):
//!
//! ```text
//! 0x00  03 01 "mst."            container magic (cf. the file's own "fmf.")
//! 0x08  u32                     format/build stamp (0x00021c3b in every save seen)
//! 0x18  u16                     in-game year
//! 0x30  u32                     global competition uid
//! 0x34  u32                     nation id
//! 0x38  u32                     stage/extra id
//! 0x3c  u32                     3
//! 0x40  u8 tag, LP(name), LP(short name)
//!       u8 0x02, 22×0xFF, 8×0x00
//!       u8 0x02, u32, u32, LP(stadium)
//!       reference groups (see `ReferenceGroup`)
//! ```
//!
//! Contents are a *match*: one competition, one stadium, one referee and a
//! localised injury string table. `.scm` sections are keyed by the first
//! reference in the first group; `.apm` sections repeat that key with a suffix.
//! They are therefore per-fixture caches, **not** competition membership lists.

use crate::resolve::{is_name_byte, rd_u32};

pub const MST_MAGIC: [u8; 6] = [0x03, 0x01, b'm', b's', b't', b'.'];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ext {
    Scm,
    Apm,
    Other,
}

impl Ext {
    pub fn parse(s: &str) -> Ext {
        match s {
            ".scm" => Ext::Scm,
            ".apm" => Ext::Apm,
            _ => Ext::Other,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Ext::Scm => ".scm",
            Ext::Apm => ".apm",
            Ext::Other => "?",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReferenceGroup {
    pub tag_or_kind: u8,
    pub references: Vec<u32>,
    pub start_offset: u64,
    pub end_offset: u64,
}

#[derive(Debug, Clone, Default)]
pub struct MstRecord {
    pub magic_ok: bool,
    pub format_stamp: Option<u32>,
    pub year: Option<u16>,
    pub competition_uid: Option<u32>,
    pub nation_id: Option<u32>,
    pub stage_id: Option<u32>,
    pub name: Option<String>,
    pub short_name: Option<String>,
    pub venue: Option<String>,
    pub venue_ids: Option<(u32, u32)>,
    pub reference_groups: Vec<ReferenceGroup>,
    pub evidence_offsets: Vec<(&'static str, u64)>,
}

fn read_lp(d: &[u8], at: usize) -> Option<(String, usize)> {
    let n = rd_u32(d, at)? as usize;
    if !(1..=200).contains(&n) || at + 4 + n > d.len() {
        return None;
    }
    let b = &d[at + 4..at + 4 + n];
    if !b.iter().all(|&x| is_name_byte(x)) {
        return None;
    }
    Some((String::from_utf8_lossy(b).to_string(), at + 4 + n))
}

/// Parse the fixed header and the reference groups that follow the venue name.
pub fn parse(d: &[u8]) -> MstRecord {
    let mut r = MstRecord::default();
    if d.len() < 0x48 || d[..6] != MST_MAGIC {
        return r;
    }
    r.magic_ok = true;
    r.format_stamp = rd_u32(d, 0x08);
    r.year = d.get(0x18..0x1a).map(|s| u16::from_le_bytes(s.try_into().unwrap()));
    r.competition_uid = rd_u32(d, 0x30);
    r.nation_id = rd_u32(d, 0x34);
    r.stage_id = rd_u32(d, 0x38);
    r.evidence_offsets.push(("competition_uid", 0x30));
    r.evidence_offsets.push(("nation_id", 0x34));

    // 0x40 is a single tag byte, then the two names back to back.
    let mut p = 0x41usize;
    if let Some((n, next)) = read_lp(d, p) {
        r.evidence_offsets.push(("name", p as u64));
        r.name = Some(n);
        p = next;
        if let Some((s, next2)) = read_lp(d, p) {
            r.evidence_offsets.push(("short_name", p as u64));
            r.short_name = Some(s);
            p = next2;
        }
    }

    // 0x02, 22 sentinel bytes, 8 zero bytes, 0x02, then two ids and the venue.
    if d.get(p) == Some(&0x02) {
        let q = p + 1 + 22 + 8;
        if d.get(q) == Some(&0x02) {
            if let (Some(a), Some(b)) = (rd_u32(d, q + 1), rd_u32(d, q + 5)) {
                r.venue_ids = Some((a, b));
                r.evidence_offsets.push(("venue_ids", (q + 1) as u64));
                if let Some((v, next)) = read_lp(d, q + 9) {
                    r.evidence_offsets.push(("venue", (q + 9) as u64));
                    r.venue = Some(v);
                    p = next;
                }
            }
        }
    }

    // Reference groups: `<u8 kind>` then a run of `02 <u32>` entries.
    // Stop at the referee's forename, which is a bare LP string.
    let mut guard = 0;
    while p < d.len() && guard < 32 {
        guard += 1;
        let kind = d[p];
        let start = p;
        let mut q = p + 1;
        let mut refs = Vec::new();
        while q + 5 <= d.len() && d[q] == 0x02 {
            refs.push(rd_u32(d, q + 1).unwrap_or(0));
            q += 5;
        }
        if refs.is_empty() {
            break;
        }
        r.reference_groups.push(ReferenceGroup {
            tag_or_kind: kind,
            references: refs,
            start_offset: start as u64,
            end_offset: q as u64,
        });
        // Trailing `00 00 01 <u32> <u32> 00` tail between groups, length varies;
        // advance to the next 0x02 run, bounded so a miss cannot run away.
        let mut adv = q;
        let limit = (q + 48).min(d.len());
        while adv + 5 <= limit && d[adv] != 0x02 {
            adv += 1;
        }
        if adv + 5 > limit {
            break;
        }
        p = adv.saturating_sub(1);
        if p <= start {
            break;
        }
    }
    r
}
