//! Compact club-table walker + the club's counted competition-registration array.
//!
//! Verified FM26 record shape (see `scoutiq/src-tauri/src/fm/entities/club_record.rs`):
//!
//! ```text
//! uid | uid | 0x00 | <4 words: one 0xFFFFFFFF + three equal nation ids>
//!     | next_u32 | 6 flag bytes | LP(full name) | LP(short name) | body…
//! ```
//!
//! The body ends where the next record's header begins, so records are walked in
//! order and each one's span is closed by its successor. Everything reported for
//! a club therefore comes from inside that club's own byte range.
//!
//! Inside the body sits a counted array of competition registrations:
//!
//! ```text
//! <u8 count> then count × ( 0x01 <u8 kind> <u32 comp_uid> <u32 comp_uid|0> 0x00 0x00 )
//! ```
//!
//! `kind` is a team slot, not a competition class: 1/2 are the U19 sides, 4 the
//! reserve team, 5 the third team. The senior team is deliberately absent — see
//! the findings notes.

use crate::resolve::{is_name_byte, rd_u32};

#[derive(Debug, Clone)]
pub struct Registration {
    pub kind: u8,
    pub comp_uid: u32,
    pub second: u32,
    pub offset: u64,
}

#[derive(Debug, Clone)]
pub struct ClubRow {
    pub offset: u64,
    pub compact_id: u32,
    pub nation_id: u32,
    pub full: String,
    pub short: String,
    /// Offset just past `LP(short)`, where the record body starts.
    pub body_at: u64,
    /// Exclusive end of this record, i.e. the next record's start.
    pub end: u64,
    pub regs: Vec<Registration>,
    /// Every counted `(0x02, id)` style reference group found in the body.
    pub body_len: usize,
}

/// Parse a club record header at `i`, returning the row and the body offset.
pub fn parse_club_header(buf: &[u8], base: u64, i: usize) -> Option<ClubRow> {
    let a = rd_u32(buf, i)?;
    if a == 0 || a == u32::MAX {
        return None;
    }
    if rd_u32(buf, i + 4)? != a || *buf.get(i + 8)? != 0 {
        return None;
    }
    let q: Vec<u32> = (0..4).map(|k| rd_u32(buf, i + 9 + 4 * k)).collect::<Option<_>>()?;
    let sentinels = q.iter().filter(|&&v| v == u32::MAX).count();
    let nat: Vec<u32> = q.iter().copied().filter(|&v| v != u32::MAX).collect();
    if sentinels != 1 || nat.len() != 3 || nat[0] != nat[1] || nat[1] != nat[2] {
        return None;
    }
    if !(1..10_000).contains(&nat[0]) {
        return None;
    }
    // next_u32 (4) + 6 flag bytes, then the two length-prefixed names.
    let (full, after_full) = read_lp(buf, i + 35)?;
    let (short, after_short) = read_lp(buf, after_full)?;
    Some(ClubRow {
        offset: base + i as u64,
        compact_id: a,
        nation_id: nat[0],
        full,
        short,
        body_at: base + after_short as u64,
        end: base + after_short as u64,
        regs: Vec::new(),
        body_len: 0,
    })
}

fn read_lp(buf: &[u8], at: usize) -> Option<(String, usize)> {
    let n = rd_u32(buf, at)? as usize;
    if !(2..=90).contains(&n) || at + 4 + n > buf.len() {
        return None;
    }
    let b = &buf[at + 4..at + 4 + n];
    if !b.iter().all(|&x| is_name_byte(x)) || !b.iter().any(|x| x.is_ascii_alphabetic()) {
        return None;
    }
    Some((String::from_utf8_lossy(b).to_string(), at + 4 + n))
}

/// Walk a buffer, emitting every club record header found, in order.
pub fn scan_club_rows(buf: &[u8], base: u64, out: &mut Vec<ClubRow>) {
    let mut i = 0usize;
    while i + 120 <= buf.len() {
        match parse_club_header(buf, base, i) {
            Some(row) => {
                i = (row.body_at - base) as usize;
                out.push(row);
            }
            None => i += 1,
        }
    }
}

/// Competition registrations inside `[from, to)` of the buffer.
pub fn scan_registrations(buf: &[u8], base: u64, from: usize, to: usize) -> Vec<Registration> {
    let mut out = Vec::new();
    let mut j = from;
    let hi = to.min(buf.len());
    while j + 12 <= hi {
        if buf[j] == 0x01 {
            let kind = buf[j + 1];
            if (1..=12).contains(&kind) {
                if let (Some(a), Some(b)) = (rd_u32(buf, j + 2), rd_u32(buf, j + 6)) {
                    if a > 100 && a != u32::MAX && (b == a || b == 0)
                        && buf[j + 10] == 0
                        && buf[j + 11] == 0
                    {
                        out.push(Registration {
                            kind,
                            comp_uid: a,
                            second: b,
                            offset: base + j as u64,
                        });
                        j += 12;
                        continue;
                    }
                }
            }
        }
        j += 1;
    }
    out
}
