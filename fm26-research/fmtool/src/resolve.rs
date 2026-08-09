//! Experiment: manager -> club -> competition resolution measurement.

use std::collections::HashMap;

pub fn rd_u32(d: &[u8], at: usize) -> Option<u32> {
    d.get(at..at + 4)
        .map(|s| u32::from_le_bytes(s.try_into().unwrap()))
}

pub fn is_name_byte(b: u8) -> bool {
    (0x20..=0x7e).contains(&b) || b >= 0x80
}

/// Length-prefixed UTF-8 name.
pub fn rd_lp_name(d: &[u8], at: usize, max: usize) -> Option<(String, usize)> {
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

/// Find the manager -> club pair (`0E 01` + LP person + LP club).
pub fn find_manager_pair(d: &[u8]) -> Vec<(usize, String, String, usize)> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 2 < d.len() {
        if d[i] == 0x0E && d[i + 1] == 0x01 {
            if let Some((person, n1)) = rd_lp_name(d, i + 2, 48) {
                if let Some((club, n2)) = rd_lp_name(d, n1, 48) {
                    if person.contains(' ') && person.len() >= 5 {
                        out.push((i, person, club, n2));
                    }
                }
            }
        }
        i += 1;
    }
    out
}

/// Club-record candidates: LP(full) LP(short) with an id header before them.
pub struct ClubHit {
    pub uid: u32,
    pub secondary: u32,
    pub full: String,
    pub offset: u64,
    pub pre: Vec<u8>,
}

pub fn scan_club_records(buf: &[u8], base: u64, short: &str, out: &mut Vec<ClubHit>) {
    let nb = short.as_bytes();
    if nb.len() < 2 {
        return;
    }
    let mut i = 4usize;
    while i + nb.len() <= buf.len() {
        if &buf[i..i + nb.len()] != nb {
            i += 1;
            continue;
        }
        // Require exact LP framing for the short name.
        if rd_u32(buf, i - 4) != Some(nb.len() as u32) {
            i += 1;
            continue;
        }
        // Walk back to find LP(full) immediately preceding.
        for full_len in 3..120usize {
            let Some(fs) = i.checked_sub(4 + full_len + 4) else { break };
            if rd_u32(buf, fs) != Some(full_len as u32) {
                continue;
            }
            let fb = &buf[fs + 4..fs + 4 + full_len];
            if fs + 4 + full_len != i - 4 || !fb.iter().all(|&b| is_name_byte(b)) {
                continue;
            }
            let Ok(full) = std::str::from_utf8(fb) else { continue };
            let lo = fs.saturating_sub(48);
            let pre = buf[lo..fs].to_vec();
            let (uid, secondary) = extract_ids(&pre);
            out.push(ClubHit {
                uid,
                secondary,
                full: full.to_string(),
                offset: base + fs as u64,
                pre,
            });
            break;
        }
        i += 1;
    }
}

/// `uid uid 00 | sec FF sec sec` (A) or `uid uid 00 | FF sec sec sec` (B).
fn extract_ids(pre: &[u8]) -> (u32, u32) {
    if pre.len() < 16 {
        return (0, 0);
    }
    for i in 0..=pre.len() - 16 {
        let a = rd_u32(pre, i).unwrap();
        let b = rd_u32(pre, i + 4).unwrap();
        let c = rd_u32(pre, i + 8).unwrap();
        let d = rd_u32(pre, i + 12).unwrap();
        let sec_a = a > 0 && a < 500_000 && b == u32::MAX && c == a && d == a;
        let sec_b = a == u32::MAX && b > 0 && b < 500_000 && c == b && d == b;
        if sec_a || sec_b {
            let sec = if sec_a { a } else { b };
            let mut uid = 0u32;
            if i >= 9 {
                let u0 = rd_u32(pre, i - 9).unwrap_or(0);
                let u1 = rd_u32(pre, i - 5).unwrap_or(0);
                if u0 > 0 && u0 < 500_000 && u0 == u1 && pre[i - 1] == 0 {
                    uid = u0;
                }
            }
            return (uid, sec);
        }
    }
    (0, 0)
}

/// Tally competition-name candidates that follow an occurrence of `id`.
#[derive(Default)]
pub struct LinkTally {
    pub names: HashMap<String, (usize, Vec<u8>, usize)>,
}

pub fn scan_links(buf: &[u8], id: u32, tally: &mut LinkTally, lookahead: usize) {
    let idb = id.to_le_bytes();
    let mut i = 0usize;
    while i + 4 + lookahead < buf.len() {
        if buf[i..i + 4] != idb {
            i += 1;
            continue;
        }
        for gap in 0..lookahead {
            let at = i + 4 + gap;
            if let Some((name, _)) = rd_lp_name(buf, at, 64) {
                if name.len() >= 4 {
                    let marker = buf[i + 4..at].to_vec();
                    let e = tally
                        .names
                        .entry(name)
                        .or_insert((0, marker.clone(), gap));
                    e.0 += 1;
                }
                break;
            }
        }
        i += 1;
    }
}
