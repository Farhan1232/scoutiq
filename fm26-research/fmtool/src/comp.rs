//! Division table extraction + club->division co-occurrence tallying.

use std::collections::HashMap;

use crate::resolve::{is_name_byte, rd_u32};

pub const LVLF: [u8; 4] = [0x6c, 0x76, 0x6c, 0x66]; // "lvlf"
pub const SENT: [u8; 4] = [0x60, 0x79, 0xfe, 0xff];

#[derive(Debug, Clone)]
pub struct Division {
    pub name: String,
    pub ids: [u32; 3],
    pub offset: u64,
}

/// `lvlf 01 01 <sent> LP(name) xx 02 03 00 00 00 a b c` -> division entry.
pub fn scan_divisions(buf: &[u8], base: u64, out: &mut Vec<Division>) {
    let mut i = 0usize;
    while i + 10 < buf.len() {
        if buf[i..i + 4] != LVLF || buf[i + 4] != 0x01 || buf[i + 5] != 0x01 {
            i += 1;
            continue;
        }
        if buf[i + 6..i + 10] != SENT {
            i += 1;
            continue;
        }
        let at = i + 10;
        let Some(len) = rd_u32(buf, at) else { break };
        let len = len as usize;
        if len < 2 || len > 90 || at + 4 + len + 22 > buf.len() {
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
        // Expect `xx 02` then `03 00 00 00` then three u32 ids.
        let p = at + 4 + len;
        if buf[p + 1] != 0x02 || rd_u32(buf, p + 2) != Some(3) {
            i += 1;
            continue;
        }
        let (Some(a), Some(b), Some(c)) = (
            rd_u32(buf, p + 6),
            rd_u32(buf, p + 10),
            rd_u32(buf, p + 14),
        ) else {
            i += 1;
            continue;
        };
        out.push(Division {
            name: name.to_string(),
            ids: [a, b, c],
            offset: base + i as u64,
        });
        i += 1;
    }
}

/// Tally division ids that appear within `win` bytes of any occurrence of `club_id`.
pub fn tally_near(
    buf: &[u8],
    club_id: u32,
    idmap: &HashMap<u32, (String, usize)>,
    win: usize,
    tally: &mut HashMap<(String, usize), usize>,
) {
    let cb = club_id.to_le_bytes();
    let mut i = 0usize;
    while i + 4 <= buf.len() {
        if buf[i..i + 4] != cb {
            i += 1;
            continue;
        }
        let lo = i.saturating_sub(win);
        let hi = (i + 4 + win).min(buf.len());
        let mut j = lo;
        while j + 4 <= hi {
            if j != i {
                if let Some(v) = rd_u32(buf, j) {
                    if let Some((name, slot)) = idmap.get(&v) {
                        *tally.entry((name.clone(), *slot)).or_insert(0) += 1;
                    }
                }
            }
            j += 1;
        }
        i += 1;
    }
}
