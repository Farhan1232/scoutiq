//! Read-only FM26 save container explorer (directory-aware).
//! Opens saves with File::open only — never for writing.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use zstd_safe::{DCtx, InBuffer, OutBuffer};
mod clubtable;
mod comp;
mod mst;
mod resolve;

const ZSTD_MAGIC: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];
const FMF_SIG: [u8; 6] = [0x02, 0x01, b'f', b'm', b'f', b'.'];
const OUT_CHUNK: usize = 4 << 20;

#[derive(Debug, Clone)]
struct Section {
    name: String,
    ext: String,
    offset: u64,
    comp: u64,
    decomp: u64,
}

fn read_at(f: &mut File, off: u64, len: usize) -> std::io::Result<Vec<u8>> {
    f.seek(SeekFrom::Start(off))?;
    let mut v = vec![0u8; len];
    f.read_exact(&mut v)?;
    Ok(v)
}

/// Decompress a run of concatenated zstd frames from `src`, up to `max_out` bytes.
fn zstd_decompress_all(src: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    let mut chunk = vec![0u8; OUT_CHUNK];
    while pos + 4 <= src.len() && src[pos..pos + 4] == ZSTD_MAGIC {
        let mut dctx = DCtx::create();
        let mut inb = InBuffer::around(&src[pos..]);
        loop {
            let written;
            let done;
            {
                let mut outb = OutBuffer::around(&mut chunk[..]);
                match dctx.decompress_stream(&mut outb, &mut inb) {
                    Ok(hint) => {
                        written = outb.pos();
                        done = hint == 0;
                    }
                    Err(_) => {
                        written = outb.pos();
                        done = true;
                    }
                }
            }
            if written > 0 {
                out.extend_from_slice(&chunk[..written]);
            }
            if done {
                break;
            }
            if inb.pos() == inb.src.len() && written == 0 {
                break;
            }
        }
        if inb.pos() == 0 {
            break;
        }
        pos += inb.pos();
    }
    out
}

fn rd_u32(d: &[u8], at: usize) -> Option<u32> {
    d.get(at..at + 4)
        .map(|s| u32::from_le_bytes(s.try_into().unwrap()))
}
fn rd_u64(d: &[u8], at: usize) -> Option<u64> {
    d.get(at..at + 8)
        .map(|s| u64::from_le_bytes(s.try_into().unwrap()))
}
fn rd_lp(d: &[u8], at: usize) -> Option<(String, usize)> {
    let len = rd_u32(d, at)? as usize;
    if len > 4096 || at + 4 + len > d.len() {
        return None;
    }
    let s = String::from_utf8_lossy(&d[at + 4..at + 4 + len]).to_string();
    Some((s, at + 4 + len))
}

/// Header length = offset of first zstd magic after the `02 01 fmf.` signature.
fn header_len(head: &[u8]) -> Option<u64> {
    if head.len() < 6 || head[..6] != FMF_SIG {
        return None;
    }
    head.windows(4).position(|w| w == ZSTD_MAGIC).map(|p| p as u64)
}

/// Locate + parse the trailing section directory by scanning backwards for `02 01 fmf.`.
fn load_directory(path: &str) -> (u64, String, Vec<Section>) {
    let mut f = File::open(path).expect("open read-only");
    let file_len = f.metadata().expect("meta").len();

    let head = read_at(&mut f, 0, 64).expect("head");
    let main_hdr = header_len(&head).expect("no fmf. main header");

    // Scan the tail for the last valid directory container.
    let tail_len = (4 << 20).min(file_len) as usize;
    let tail_start = file_len - tail_len as u64;
    let tail = read_at(&mut f, tail_start, tail_len).expect("tail");

    let mut cand: Vec<usize> = Vec::new();
    for i in 0..tail.len().saturating_sub(6) {
        if tail[i..i + 6] == FMF_SIG {
            cand.push(i);
        }
    }

    for &i in cand.iter().rev() {
        let sub = &tail[i..];
        let Some(hl) = header_len(sub) else { continue };
        let raw = zstd_decompress_all(&sub[hl as usize..]);
        if raw.len() < 8 {
            continue;
        }
        let Some((save_name, mut p)) = rd_lp(&raw, 0) else {
            continue;
        };
        let Some(count) = rd_u32(&raw, p) else { continue };
        if count == 0 || count > 10_000 {
            continue;
        }
        p += 4;
        let mut sections = Vec::new();
        let mut ok = true;
        for _ in 0..count {
            let Some((name, p1)) = rd_lp(&raw, p) else {
                ok = false;
                break;
            };
            let Some((ext, p2)) = rd_lp(&raw, p1) else {
                ok = false;
                break;
            };
            let (Some(offset), Some(comp), Some(decomp)) =
                (rd_u64(&raw, p2), rd_u64(&raw, p2 + 8), rd_u64(&raw, p2 + 16))
            else {
                ok = false;
                break;
            };
            sections.push(Section { name, ext, offset, comp, decomp });
            p = p2 + 24 + 16;
        }
        if ok && !sections.is_empty() {
            return (main_hdr, save_name, sections);
        }
    }
    panic!("no valid section directory found");
}

fn section_bytes(path: &str, main_hdr: u64, s: &Section) -> Vec<u8> {
    let mut f = File::open(path).expect("open read-only");
    let raw = read_at(&mut f, main_hdr + s.offset, s.comp as usize).expect("section read");
    zstd_decompress_all(&raw)
}

/// Stream-decompress a section, handing chunks to `sink` (never materialised).
fn stream_section(path: &str, main_hdr: u64, s: &Section, sink: &mut dyn FnMut(u64, &[u8])) {
    let mut f = File::open(path).expect("open read-only");
    f.seek(SeekFrom::Start(main_hdr + s.offset)).expect("seek");

    let mut remaining = s.comp as usize;
    let mut inbuf = vec![0u8; 1 << 20];
    let mut chunk = vec![0u8; OUT_CHUNK];
    let mut dctx = DCtx::create();
    let mut abs: u64 = 0;
    let mut carry: Vec<u8> = Vec::new();

    while remaining > 0 {
        let want = remaining.min(inbuf.len());
        let mut got = vec![0u8; want];
        f.read_exact(&mut got).expect("read");
        remaining -= want;

        let mut src = std::mem::take(&mut carry);
        src.extend_from_slice(&got);
        let mut inb = InBuffer::around(&src[..]);

        loop {
            let written;
            let finished;
            {
                let mut outb = OutBuffer::around(&mut chunk[..]);
                match dctx.decompress_stream(&mut outb, &mut inb) {
                    Ok(hint) => {
                        written = outb.pos();
                        finished = hint == 0;
                    }
                    Err(_) => {
                        written = outb.pos();
                        finished = true;
                    }
                }
            }
            if written > 0 {
                sink(abs, &chunk[..written]);
                abs += written as u64;
            }
            if finished {
                // Frame ended; a section may hold several concatenated frames.
                if inb.pos() < inb.src.len() {
                    dctx = DCtx::create();
                    continue;
                }
                break;
            }
            if inb.pos() == inb.src.len() {
                break;
            }
        }
        if inb.pos() < src.len() {
            carry = src[inb.pos()..].to_vec();
        }
    }
}

fn cmd_find(path: &str, want: &str, needle: &str, ctx: usize, limit: usize) {
    let (hdr, _, secs) = load_directory(path);
    let s = secs.iter().find(|s| s.name == want).expect("section not found");
    let nb = needle.as_bytes();
    let keep = ctx + nb.len() + 64;
    let mut tail: Vec<u8> = Vec::new();
    let mut hits = 0usize;

    stream_section(path, hdr, s, &mut |abs, chunk| {
        if hits >= limit {
            return;
        }
        let base = abs - tail.len() as u64;
        let mut buf = std::mem::take(&mut tail);
        buf.extend_from_slice(chunk);

        let mut i = 0usize;
        while i + nb.len() <= buf.len() {
            if &buf[i..i + nb.len()] == nb {
                let lo = i.saturating_sub(ctx);
                let hi = (i + nb.len() + ctx).min(buf.len());
                let hex: Vec<String> =
                    buf[lo..hi].iter().map(|b| format!("{:02x}", b)).collect();
                let asc: String = buf[lo..hi]
                    .iter()
                    .map(|&b| if (0x20..0x7f).contains(&b) { b as char } else { '.' })
                    .collect();
                println!(
                    "HIT off={} pre={}\n  hex {}\n  asc |{}|",
                    base + i as u64,
                    i - lo,
                    hex.join(" "),
                    asc
                );
                hits += 1;
                if hits >= limit {
                    break;
                }
            }
            i += 1;
        }
        let k = keep.min(buf.len());
        tail = buf[buf.len() - k..].to_vec();
    });
    println!("hits={}", hits);
}

/// Dump a byte range of a decompressed section to a file.
fn cmd_slice(path: &str, want: &str, start: u64, len: usize, out: &str) {
    let (hdr, _, secs) = load_directory(path);
    let s = secs.iter().find(|s| s.name == want).expect("section not found");
    let mut buf: Vec<u8> = Vec::new();
    let end = start + len as u64;
    stream_section(path, hdr, s, &mut |abs, chunk| {
        let c0 = abs;
        let c1 = abs + chunk.len() as u64;
        if c1 <= start || c0 >= end {
            return;
        }
        let lo = start.saturating_sub(c0) as usize;
        let hi = (end.min(c1) - c0) as usize;
        buf.extend_from_slice(&chunk[lo..hi]);
    });
    std::fs::write(out, &buf).expect("write");
    println!("sliced {} bytes from {}@{} -> {}", buf.len(), want, start, out);
}

/// Stream a section with `overlap` bytes of carry so records spanning chunks survive.
fn stream_overlapped(
    path: &str,
    hdr: u64,
    s: &Section,
    overlap: usize,
    f: &mut dyn FnMut(u64, &[u8]),
) {
    let mut tail: Vec<u8> = Vec::new();
    stream_section(path, hdr, s, &mut |abs, chunk| {
        let base = abs - tail.len() as u64;
        let mut buf = std::mem::take(&mut tail);
        buf.extend_from_slice(chunk);
        f(base, &buf);
        let k = overlap.min(buf.len());
        tail = buf[buf.len() - k..].to_vec();
    });
}

/// Dump byte windows around every length-prefixed occurrence of `name` in a section.
/// The LP framing (u32 len == name len, then the bytes) filters out prose mentions,
/// leaving actual record fields.
fn cmd_namewin(path: &str, want: &str, name: &str, radius: usize, limit: usize, prefix: &str) {
    let (hdr, _, secs) = load_directory(path);
    let s = secs.iter().find(|s| s.name == want).expect("section not found");
    let nb = name.as_bytes();
    let mut pat = (nb.len() as u32).to_le_bytes().to_vec();
    pat.extend_from_slice(nb);

    let keep = 2 * radius + pat.len() + 64;
    let mut seen: std::collections::HashSet<u64> = std::collections::HashSet::new();
    let mut n = 0usize;

    stream_overlapped(path, hdr, s, keep, &mut |base, buf| {
        if n >= limit {
            return;
        }
        let mut i = 0usize;
        while i + pat.len() <= buf.len() {
            if buf[i..i + pat.len()] == pat[..]
                && i >= radius
                && i + pat.len() + radius <= buf.len()
                && seen.insert(base + i as u64)
            {
                let out = format!("{prefix}_{n}.bin");
                std::fs::write(&out, &buf[i - radius..i + pat.len() + radius]).expect("write");
                println!("hit {} off={} lp_at={} -> {}", n, base + i as u64, radius, out);
                n += 1;
                if n >= limit {
                    break;
                }
            }
            i += 1;
        }
    });
    println!("windows={}", n);
}

/// Map where the self-describing tagged encoding lives, by counting
/// `<4 tag chars> 01 <known type>` hits per bucket.
fn cmd_tagdens(path: &str, want: &str, bucket_bits: u32, thresh: usize) {
    let (hdr, _, secs) = load_directory(path);
    let s = secs.iter().find(|s| s.name == want).expect("section not found");
    let types: [bool; 256] = {
        let mut t = [false; 256];
        for b in [0x01u8, 0x02, 0x03, 0x0a, 0x0b, 0x0f, 0x11, 0x12, 0x18, 0x1a] {
            t[b as usize] = true;
        }
        t
    };
    let is_tag = |b: u8| (0x30..0x7b).contains(&b) || b == 0x20 || b == 0x5f;

    let mut buckets: std::collections::BTreeMap<u64, usize> = std::collections::BTreeMap::new();
    stream_overlapped(path, hdr, s, 16, &mut |base, buf| {
        for i in 0..buf.len().saturating_sub(6) {
            if buf[i + 4] == 0x01
                && types[buf[i + 5] as usize]
                && is_tag(buf[i])
                && is_tag(buf[i + 1])
                && is_tag(buf[i + 2])
                && is_tag(buf[i + 3])
            {
                *buckets.entry((base + i as u64) >> bucket_bits).or_insert(0) += 1;
            }
        }
    });

    let size = 1u64 << bucket_bits;
    let mut run: Option<(u64, u64, usize)> = None;
    println!("section={} decomp={} bucket={}", s.name, s.decomp, size);
    for (&k, &v) in buckets.iter() {
        if v < thresh {
            continue;
        }
        match run {
            Some((a, b, tot)) if k <= b + 2 => run = Some((a, k, tot + v)),
            Some((a, b, tot)) => {
                println!("  {:>12} .. {:>12}  ({:>7.2} MB) hits={}", a * size, (b + 1) * size, ((b + 1 - a) * size) as f64 / 1e6, tot);
                run = Some((k, k, v));
            }
            None => run = Some((k, k, v)),
        }
    }
    if let Some((a, b, tot)) = run {
        println!("  {:>12} .. {:>12}  ({:>7.2} MB) hits={}", a * size, (b + 1) * size, ((b + 1 - a) * size) as f64 / 1e6, tot);
    }
}

/// Name-table records: `duni 01 0f <u32 id><u32 id> prty 01 02 <class> nwvl 01 1a <lp>`.
/// Tags are stored byte-reversed, so `duni` is written "inud" and so on.
const T_DUNI: [u8; 6] = [b'i', b'n', b'u', b'd', 0x01, 0x0f];
const T_PRTY: [u8; 6] = [b'y', b't', b'r', b'p', 0x01, 0x02];
const T_NWVL: [u8; 6] = [b'l', b'v', b'w', b'n', 0x01, 0x1a];

fn cmd_names(path: &str, filter: &str) {
    let (hdr, _, secs) = load_directory(path);
    let gdb = secs.iter().find(|s| s.name == "game_db").expect("no game_db");
    let mut seen: std::collections::HashSet<u64> = std::collections::HashSet::new();

    stream_overlapped(path, hdr, gdb, 4096, &mut |base, buf| {
        let mut i = 0usize;
        while i + 32 <= buf.len() {
            if buf[i..i + 6] != T_DUNI {
                i += 1;
                continue;
            }
            let Some(id) = rd_u32(buf, i + 6) else { break };
            let p = i + 14;
            if buf.get(p..p + 6) != Some(&T_PRTY[..]) {
                i += 1;
                continue;
            }
            let class: String = buf[p + 6..p + 10].iter().rev().map(|&b| b as char).collect();
            let q = p + 10;
            if buf.get(q..q + 6) != Some(&T_NWVL[..]) {
                i += 1;
                continue;
            }
            let Some((name, next)) = rd_lp(buf, q + 6) else {
                i += 1;
                continue;
            };
            if (filter.is_empty() || class == filter) && seen.insert(base + i as u64) {
                println!("{}\t{}\t{}", class, id, name);
            }
            i = next;
        }
    });
}

/// Tally, per byte offset relative to every occurrence of `anchor`, how often
/// `target` appears. The offset that scores across saves is the real field.
fn cmd_relscan(path: &str, anchor: u32, target: u32, radius: usize) {
    let (hdr, _, secs) = load_directory(path);
    let gdb = secs.iter().find(|s| s.name == "game_db").expect("no game_db");
    let ab = anchor.to_le_bytes();
    let t32 = target.to_le_bytes();
    let t16 = (target as u16).to_le_bytes();

    let mut c32 = vec![0usize; 2 * radius + 1];
    let mut c16 = vec![0usize; 2 * radius + 1];
    let mut anchors = 0usize;

    stream_overlapped(path, hdr, gdb, 2 * radius + 8, &mut |_, buf| {
        let mut i = radius;
        while i + radius + 4 <= buf.len() {
            if buf[i..i + 4] == ab {
                anchors += 1;
                for r in 0..=2 * radius {
                    let at = i + r - radius;
                    if buf[at..at + 4] == t32 {
                        c32[r] += 1;
                    }
                    if buf[at..at + 2] == t16 {
                        c16[r] += 1;
                    }
                }
            }
            i += 1;
        }
    });

    println!("anchor={anchor} target={target} anchor_hits={anchors}");
    let mut v: Vec<(usize, isize, usize, usize)> = (0..=2 * radius)
        .map(|r| (r, r as isize - radius as isize, c32[r], c16[r]))
        .collect();
    v.sort_by_key(|x| std::cmp::Reverse(x.2 * 4 + x.3));
    for (_, rel, a, b) in v.into_iter().take(14) {
        println!("  rel={rel:>+5}  u32={a:<7} u16={b}");
    }
}

/// Competition-registration entries: `01 <kind> <u32 id> <u32 id> 00 00`.
/// Tally which ids appear per kind, to see which slot carries the senior league.
fn cmd_regscan(path: &str, top: usize) {
    let (hdr, _, secs) = load_directory(path);
    let gdb = secs.iter().find(|s| s.name == "game_db").expect("no game_db");
    let mut tally: std::collections::HashMap<(u8, u32), usize> = std::collections::HashMap::new();

    stream_overlapped(path, hdr, gdb, 32, &mut |_, buf| {
        let mut i = 0usize;
        while i + 12 <= buf.len() {
            if buf[i] == 0x01 && (1..=8).contains(&buf[i + 1]) {
                let a = rd_u32(buf, i + 2).unwrap();
                let b = rd_u32(buf, i + 6).unwrap();
                if a != 0 && a == b && buf[i + 10] == 0 && buf[i + 11] == 0 {
                    *tally.entry((buf[i + 1], a)).or_insert(0) += 1;
                    i += 12;
                    continue;
                }
            }
            i += 1;
        }
    });

    let mut by_kind: std::collections::BTreeMap<u8, Vec<(u32, usize)>> = Default::default();
    for ((k, id), n) in tally {
        by_kind.entry(k).or_default().push((id, n));
    }
    for (k, mut v) in by_kind {
        v.sort_by_key(|x| std::cmp::Reverse(x.1));
        let total: usize = v.iter().map(|x| x.1).sum();
        println!("kind={k} distinct={} total={total}", v.len());
        for (id, n) in v.into_iter().take(top) {
            println!("    id={id:<12} n={n}");
        }
    }
}

/// Collect every club unique id from the name table (`Cnam`/`Csnm` records).
fn club_uids(path: &str, hdr: u64, gdb: &Section) -> std::collections::HashSet<u32> {
    let mut out = std::collections::HashSet::new();
    stream_overlapped(path, hdr, gdb, 4096, &mut |_, buf| {
        let mut i = 0usize;
        while i + 32 <= buf.len() {
            if buf[i..i + 6] != T_DUNI {
                i += 1;
                continue;
            }
            let Some(id) = rd_u32(buf, i + 6) else { break };
            let p = i + 14;
            if buf.get(p..p + 6) != Some(&T_PRTY[..]) {
                i += 1;
                continue;
            }
            let class: String = buf[p + 6..p + 10].iter().rev().map(|&b| b as char).collect();
            if class == "Cnam" || class == "Csnm" {
                out.insert(id);
            }
            i = p + 10;
        }
    });
    out
}

/// Find runs of club ids at a constant stride — league tables, squad lists and
/// competition team lists all take this shape.
fn cmd_teamarray(path: &str, min_run: usize, limit: usize, want: u32) {
    let (hdr, _, secs) = load_directory(path);
    let gdb = secs.iter().find(|s| s.name == "game_db").expect("no game_db");
    let clubs = club_uids(path, hdr, gdb);
    println!("club_uids={}", clubs.len());

    let strides: Vec<usize> = (1..=24).map(|k| k * 4).collect();
    let mut found = 0usize;

    stream_overlapped(path, hdr, gdb, 24 * 4 * 64 + 64, &mut |base, buf| {
        if found >= limit {
            return;
        }
        let mut i = 0usize;
        while i + 8 <= buf.len() {
            let Some(v) = rd_u32(buf, i) else { break };
            if !clubs.contains(&v) {
                i += 1;
                continue;
            }
            for &s in &strides {
                let mut n = 1usize;
                let mut seen = vec![v];
                while i + n * s + 4 <= buf.len() {
                    let Some(x) = rd_u32(buf, i + n * s) else { break };
                    if !clubs.contains(&x) || seen.contains(&x) {
                        break;
                    }
                    seen.push(x);
                    n += 1;
                }
                if n >= min_run && (want == 0 || seen.contains(&want)) {
                    println!(
                        "run off={} stride={} n={} ids={:?}",
                        base + i as u64,
                        s,
                        n,
                        &seen[..n.min(24)]
                    );
                    found += 1;
                    if found >= limit {
                        return;
                    }
                    i += n * s;
                    break;
                }
            }
            i += 1;
        }
    });
    println!("runs={found}");
}

/// Per-section: how often `a` occurs, and how often `b` sits within `radius` of it.
/// Narrows which section actually stores the club/competition binding.
fn cmd_secprox(path: &str, a: u32, b: u32, radius: usize) {
    let (hdr, _, secs) = load_directory(path);
    let ab = a.to_le_bytes();
    let b32 = b.to_le_bytes();
    let b16 = (b as u16).to_le_bytes();

    println!("{:<34} {:>9} {:>9} {:>9}", "section", "a_hits", "near32", "near16");
    for s in &secs {
        if s.decomp == 0 {
            continue;
        }
        let mut ah = 0usize;
        let mut n32 = 0usize;
        let mut n16 = 0usize;
        stream_overlapped(path, hdr, s, 2 * radius + 8, &mut |_, buf| {
            let mut i = radius;
            while i + radius + 4 <= buf.len() {
                if buf[i..i + 4] == ab {
                    ah += 1;
                    let lo = i - radius;
                    let hi = i + radius;
                    let mut f32 = false;
                    let mut f16 = false;
                    for j in lo..=hi {
                        if buf[j..j + 4] == b32 {
                            f32 = true;
                        }
                        if buf[j..j + 2] == b16 {
                            f16 = true;
                        }
                    }
                    if f32 {
                        n32 += 1;
                    }
                    if f16 {
                        n16 += 1;
                    }
                }
                i += 1;
            }
        });
        if ah > 0 {
            println!("{:<34} {:>9} {:>9} {:>9}", s.name, ah, n32, n16);
        }
    }
}

/// Take every u32 near the club's compact record and score it by how often it
/// sits next to the competition id. The club's internal id should stand out.
fn cmd_internal(path: &str, club: &str, comp: u32, win: usize, radius: usize) {
    let (hdr, _, secs) = load_directory(path);
    let gdb = secs.iter().find(|s| s.name == "game_db").expect("no game_db");
    let nb = club.as_bytes();
    let mut pat = (nb.len() as u32).to_le_bytes().to_vec();
    pat.extend_from_slice(nb);

    // Pass 1: candidate ids drawn from the neighbourhood of each name occurrence.
    let mut cands: std::collections::HashSet<u32> = std::collections::HashSet::new();
    stream_overlapped(path, hdr, gdb, 2 * win + 64, &mut |_, buf| {
        let mut i = win;
        while i + pat.len() + win <= buf.len() {
            if buf[i..i + pat.len()] == pat[..] {
                for j in (i - win)..(i + pat.len() + win) {
                    if let Some(v) = rd_u32(buf, j) {
                        if v > 0 && v != 0xFFFF_FFFF {
                            cands.insert(v);
                        }
                    }
                }
            }
            i += 1;
        }
    });
    println!("candidates={}", cands.len());

    // Pass 2: score each candidate by proximity to the competition id.
    let c32 = comp.to_le_bytes();
    let c16 = (comp as u16).to_le_bytes();
    let mut total: std::collections::HashMap<u32, usize> = Default::default();
    let mut near: std::collections::HashMap<u32, usize> = Default::default();

    stream_overlapped(path, hdr, gdb, 2 * radius + 8, &mut |_, buf| {
        let mut i = radius;
        while i + radius + 4 <= buf.len() {
            let v = rd_u32(buf, i).unwrap();
            if cands.contains(&v) {
                *total.entry(v).or_insert(0) += 1;
                let mut hit = false;
                for j in (i - radius)..=(i + radius) {
                    if buf[j..j + 4] == c32 || buf[j..j + 2] == c16 {
                        hit = true;
                        break;
                    }
                }
                if hit {
                    *near.entry(v).or_insert(0) += 1;
                }
            }
            i += 1;
        }
    });

    let mut v: Vec<(u32, usize, usize)> = near
        .into_iter()
        .map(|(id, n)| (id, n, *total.get(&id).unwrap_or(&0)))
        .filter(|x| x.1 >= 4)
        .collect();
    v.sort_by(|a, b| (b.1 * 1000 / b.2.max(1)).cmp(&(a.1 * 1000 / a.2.max(1))).then(b.1.cmp(&a.1)));
    println!("{:>12} {:>8} {:>8} {:>7}", "id", "near", "total", "ratio");
    for (id, n, t) in v.into_iter().take(20) {
        println!("{:>12} {:>8} {:>8} {:>6.1}%", id, n, t, 100.0 * n as f64 / t as f64);
    }
}

/// Inverse lookup: sites where a competition id sits among many club ids —
/// i.e. a league table or a competition's team list.
fn cmd_compteams(path: &str, comp: u32, radius: usize, min_clubs: usize, limit: usize) {
    let (hdr, _, secs) = load_directory(path);
    let gdb = secs.iter().find(|s| s.name == "game_db").expect("no game_db");
    let clubs = club_uids(path, hdr, gdb);
    println!("club_uids={} comp={}", clubs.len(), comp);

    let cb = comp.to_le_bytes();
    let mut found = 0usize;
    stream_overlapped(path, hdr, gdb, 2 * radius + 8, &mut |base, buf| {
        if found >= limit {
            return;
        }
        let mut i = radius;
        while i + radius + 4 <= buf.len() {
            if buf[i..i + 4] == cb {
                let mut ids: Vec<u32> = Vec::new();
                let mut j = i - radius;
                while j + 4 <= i + radius {
                    let v = rd_u32(buf, j).unwrap();
                    if clubs.contains(&v) && !ids.contains(&v) {
                        ids.push(v);
                    }
                    j += 1;
                }
                if ids.len() >= min_clubs {
                    println!(
                        "site off={} clubs={} ids={:?}",
                        base + i as u64,
                        ids.len(),
                        &ids[..ids.len().min(26)]
                    );
                    found += 1;
                    if found >= limit {
                        return;
                    }
                    i += radius;
                }
            }
            i += 1;
        }
    });
    println!("sites={found}");
}

/// Dump raw context wherever `a` and `b` sit within `radius` bytes of each other.
fn cmd_proxdump(path: &str, a: u32, b: u32, radius: usize, ctx: usize, limit: usize) {
    let (hdr, _, secs) = load_directory(path);
    let gdb = secs.iter().find(|s| s.name == "game_db").expect("no game_db");
    let ab = a.to_le_bytes();
    let bb = b.to_le_bytes();
    let mut n = 0usize;

    stream_overlapped(path, hdr, gdb, 2 * (radius + ctx) + 16, &mut |base, buf| {
        if n >= limit {
            return;
        }
        let mut i = radius + ctx;
        while i + radius + ctx + 4 <= buf.len() {
            if buf[i..i + 4] == ab {
                let mut at: Option<usize> = None;
                for j in (i - radius)..=(i + radius) {
                    if buf[j..j + 4] == bb {
                        at = Some(j);
                        break;
                    }
                }
                if let Some(j) = at {
                    let lo = i.min(j) - ctx;
                    let hi = (i.max(j) + 4 + ctx).min(buf.len());
                    let hex: Vec<String> = buf[lo..hi].iter().map(|x| format!("{:02x}", x)).collect();
                    println!(
                        "site off={} a_at={} b_at={} delta={}\n  {}",
                        base + i as u64,
                        i - lo,
                        j - lo,
                        j as isize - i as isize,
                        hex.join(" ")
                    );
                    n += 1;
                    if n >= limit {
                        return;
                    }
                    i += radius;
                }
            }
            i += 1;
        }
    });
    println!("sites={n}");
}

/// Every competition unique id, from the name table.
fn comp_uids(path: &str, hdr: u64, gdb: &Section) -> std::collections::HashSet<u32> {
    let mut out = std::collections::HashSet::new();
    stream_overlapped(path, hdr, gdb, 4096, &mut |_, buf| {
        let mut i = 0usize;
        while i + 32 <= buf.len() {
            if buf[i..i + 6] != T_DUNI {
                i += 1;
                continue;
            }
            let Some(id) = rd_u32(buf, i + 6) else { break };
            let p = i + 14;
            if buf.get(p..p + 6) != Some(&T_PRTY[..]) {
                i += 1;
                continue;
            }
            let class: String = buf[p + 6..p + 10].iter().rev().map(|&b| b as char).collect();
            if class == "cnam" || class == "csnm" || class == "c3ln" {
                out.insert(id);
            }
            i = p + 10;
        }
    });
    out
}

/// At each byte offset relative to the club id, tally values that are real
/// competition ids. The division field should dominate at one fixed offset.
fn cmd_divscan(path: &str, club: u32, radius: usize) {
    let (hdr, _, secs) = load_directory(path);
    let gdb = secs.iter().find(|s| s.name == "game_db").expect("no game_db");
    let comps = comp_uids(path, hdr, gdb);
    let cb = club.to_le_bytes();

    let mut tally: std::collections::HashMap<(isize, u32, bool), usize> = Default::default();
    let mut anchors = 0usize;

    stream_overlapped(path, hdr, gdb, 2 * radius + 8, &mut |_, buf| {
        let mut i = radius;
        while i + radius + 4 <= buf.len() {
            if buf[i..i + 4] == cb {
                anchors += 1;
                for r in 0..=2 * radius {
                    let at = i + r - radius;
                    let rel = r as isize - radius as isize;
                    let v32 = rd_u32(buf, at).unwrap();
                    if v32 != club && comps.contains(&v32) {
                        *tally.entry((rel, v32, true)).or_insert(0) += 1;
                    }
                    let v16 = u16::from_le_bytes([buf[at], buf[at + 1]]) as u32;
                    if v16 > 0 && v16 != club && comps.contains(&v16) {
                        *tally.entry((rel, v16, false)).or_insert(0) += 1;
                    }
                }
            }
            i += 1;
        }
    });

    println!("club={club} anchors={anchors} known_comps={}", comps.len());
    let mut v: Vec<_> = tally.into_iter().collect();
    v.sort_by_key(|x| std::cmp::Reverse(x.1));
    for ((rel, id, is32), n) in v.into_iter().take(16) {
        println!(
            "  rel={rel:>+4} {:<3} comp={id:<10} n={n}  ({:.0}% of anchors)",
            if is32 { "u32" } else { "u16" },
            100.0 * n as f64 / anchors.max(1) as f64
        );
    }
}

/// Route 2: FM models Club and Team separately, and fixtures/tables key on Team.
/// Pass 1 gathers ids that sit near the club id (Team-id candidates); pass 2 scores
/// each by how often it sits near the competition id.
fn cmd_teamid(path: &str, club: u32, comp: u32, win: usize, radius: usize, keep: usize) {
    let (hdr, _, secs) = load_directory(path);
    let gdb = secs.iter().find(|s| s.name == "game_db").expect("no game_db");
    let cb = club.to_le_bytes();

    let mut near_club: std::collections::HashMap<u32, usize> = Default::default();
    let mut anchors = 0usize;
    stream_overlapped(path, hdr, gdb, 2 * win + 8, &mut |_, buf| {
        let mut i = win;
        while i + win + 4 <= buf.len() {
            if buf[i..i + 4] == cb {
                anchors += 1;
                let mut j = i - win;
                while j + 4 <= i + win {
                    let v = rd_u32(buf, j).unwrap();
                    // Aligned to the club id, and not a padding/sentinel word.
                    let bytes = v.to_le_bytes();
                    let ff = bytes.iter().filter(|&&b| b == 0xFF).count();
                    let degenerate = ff >= 2 || v == club || v == comp || v == 0;
                    if (j as isize - i as isize) % 4 == 0 && !degenerate && v < 0x8000_0000 {
                        *near_club.entry(v).or_insert(0) += 1;
                    }
                    j += 4;
                }
            }
            i += 1;
        }
    });

    let mut cands: Vec<(u32, usize)> = near_club.into_iter().filter(|x| x.1 >= 3).collect();
    cands.sort_by_key(|x| std::cmp::Reverse(x.1));
    cands.truncate(keep);
    let set: std::collections::HashSet<u32> = cands.iter().map(|x| x.0).collect();
    println!("club={club} anchors={anchors} candidates={}", set.len());

    let c32 = comp.to_le_bytes();
    let c16 = (comp as u16).to_le_bytes();
    let small = comp < 65536;
    let mut total: std::collections::HashMap<u32, usize> = Default::default();
    let mut near: std::collections::HashMap<u32, usize> = Default::default();

    stream_overlapped(path, hdr, gdb, 2 * radius + 8, &mut |_, buf| {
        let mut i = radius;
        while i + radius + 4 <= buf.len() {
            let v = rd_u32(buf, i).unwrap();
            if set.contains(&v) {
                *total.entry(v).or_insert(0) += 1;
                let mut hit = false;
                let mut j = i - radius;
                while j + 4 <= i + radius {
                    if buf[j..j + 4] == c32 || (small && buf[j..j + 2] == c16) {
                        hit = true;
                        break;
                    }
                    j += 1;
                }
                if hit {
                    *near.entry(v).or_insert(0) += 1;
                }
            }
            i += 1;
        }
    });

    let mut v: Vec<(u32, usize, usize)> = near
        .into_iter()
        .map(|(id, n)| (id, n, *total.get(&id).unwrap_or(&0)))
        .collect();
    v.retain(|x| x.1 >= 8 && x.2 <= 400_000);
    v.sort_by(|a, b| (b.1 * 10_000 / b.2.max(1)).cmp(&(a.1 * 10_000 / a.2.max(1))).then(b.1.cmp(&a.1)));
    println!("{:>12} {:>8} {:>9} {:>8}", "cand_id", "near", "total", "ratio");
    for (id, n, t) in v.into_iter().take(18) {
        println!("{:>12} {:>8} {:>9} {:>7.1}%", id, n, t, 100.0 * n as f64 / t.max(1) as f64);
    }
}

/// Club ids straight from the compact club table, so custom-database clubs
/// (editor ids like 2000095665) are included — the name table misses those.
/// Record shape: `u32 row | u32 id | u32 id | 00 | 4x u32 | u32 | 6 bytes | LP(long name)`,
/// so the long name's length field sits exactly 35 bytes after the id pair.
fn club_ids_compact(path: &str, hdr: u64, gdb: &Section) -> std::collections::HashSet<u32> {
    let mut out = std::collections::HashSet::new();
    stream_overlapped(path, hdr, gdb, 128, &mut |_, buf| {
        let mut i = 0usize;
        while i + 110 <= buf.len() {
            let Some(a) = rd_u32(buf, i) else { break };
            if a == 0 || a == 0xFFFF_FFFF || buf[i + 8] != 0 {
                i += 1;
                continue;
            }
            if rd_u32(buf, i + 4) != Some(a) {
                i += 1;
                continue;
            }
            // Four id words follow: exactly one sentinel, three equal nation ids.
            let q: Vec<u32> = (0..4).map(|k| rd_u32(buf, i + 9 + 4 * k).unwrap_or(0)).collect();
            let sentinels = q.iter().filter(|&&v| v == 0xFFFF_FFFF).count();
            let nat: Vec<u32> = q.iter().copied().filter(|&v| v != 0xFFFF_FFFF).collect();
            let nation_ok = sentinels == 1
                && nat.len() == 3
                && nat[0] == nat[1]
                && nat[1] == nat[2]
                && (1..10_000).contains(&nat[0]);
            if !nation_ok {
                i += 1;
                continue;
            }
            // Long name then short name, both length-prefixed.
            let lp = |at: usize| -> Option<usize> {
                let n = rd_u32(buf, at)? as usize;
                if !(2..=64).contains(&n) || at + 4 + n > buf.len() {
                    return None;
                }
                let s = &buf[at + 4..at + 4 + n];
                if s.iter().all(|&b| b >= 0x20 && b != 0x7f) && s.iter().any(|b| b.is_ascii_alphabetic())
                {
                    Some(at + 4 + n)
                } else {
                    None
                }
            };
            if let Some(after_long) = lp(i + 35) {
                if lp(after_long).is_some() {
                    out.insert(a);
                    i = after_long;
                    continue;
                }
            }
            i += 1;
        }
    });
    out
}

/// A league table is recognisable by shape alone: the managed club sitting among
/// ~20 other club ids. Report each such cohort so it can be compared across saves.
fn cmd_cohort(path: &str, club: u32, radius: usize, min_n: usize, limit: usize) {
    let (hdr, _, secs) = load_directory(path);
    let gdb = secs.iter().find(|s| s.name == "game_db").expect("no game_db");
    let clubs = club_ids_compact(path, hdr, gdb);
    println!("compact_club_ids={} anchor={}", clubs.len(), club);
    if !clubs.contains(&club) {
        println!("NOTE: anchor not in compact club set");
    }

    let cb = club.to_le_bytes();
    let mut found = 0usize;
    stream_overlapped(path, hdr, gdb, 2 * radius + 16, &mut |base, buf| {
        if found >= limit {
            return;
        }
        let mut i = radius;
        while i + radius + 4 <= buf.len() {
            if buf[i..i + 4] == cb {
                let mut ids: Vec<u32> = Vec::new();
                let mut j = i - radius;
                while j + 4 <= i + radius {
                    let v = rd_u32(buf, j).unwrap();
                    if v != club && clubs.contains(&v) && !ids.contains(&v) {
                        ids.push(v);
                    }
                    j += 4;
                }
                if ids.len() >= min_n {
                    ids.sort_unstable();
                    println!("site off={} n={} ids={:?}", base + i as u64, ids.len(), ids);
                    found += 1;
                    if found >= limit {
                        return;
                    }
                    i += radius;
                }
            }
            i += 1;
        }
    });
    println!("sites={found}");
}

/// Dump context wherever the u16 at `anchor + rel` equals `val`.
fn cmd_relsite(path: &str, anchor: u32, rel: isize, val: u16, ctx: usize, limit: usize) {
    let (hdr, _, secs) = load_directory(path);
    let gdb = secs.iter().find(|s| s.name == "game_db").expect("no game_db");
    let ab = anchor.to_le_bytes();
    let vb = val.to_le_bytes();
    let pad = ctx + rel.unsigned_abs() + 16;
    let mut n = 0usize;

    stream_overlapped(path, hdr, gdb, 2 * pad, &mut |base, buf| {
        if n >= limit {
            return;
        }
        let mut i = pad;
        while i + pad <= buf.len() {
            if buf[i..i + 4] == ab {
                let at = (i as isize + rel) as usize;
                if buf[at..at + 2] == vb {
                    let lo = i.min(at) - ctx;
                    let hi = (i.max(at) + 4 + ctx).min(buf.len());
                    println!(
                        "site off={} anchor_at={}\n  {}",
                        base + i as u64,
                        i - lo,
                        buf[lo..hi].iter().map(|x| format!("{:02x}", x)).collect::<Vec<_>>().join(" ")
                    );
                    n += 1;
                    if n >= limit {
                        return;
                    }
                }
            }
            i += 1;
        }
    });
    println!("sites={n}");
}

fn cmd_resolve(path: &str) {
    let (hdr, save_name, secs) = load_directory(path);
    println!("save_name = {save_name}");

    // 1. Manager -> club, from the tiny save_game_summary section.
    let sgs = secs
        .iter()
        .find(|s| s.name == "save_game_summary")
        .expect("no save_game_summary");
    let sgs_data = section_bytes(path, hdr, sgs);
    let pairs = resolve::find_manager_pair(&sgs_data);
    if pairs.is_empty() {
        println!("NO MANAGER PAIR");
        return;
    }
    for (off, person, club, _) in &pairs {
        println!("manager_pair @{off}: '{person}' -> '{club}'");
    }
    let (_, _manager, club_short, _) = pairs[0].clone();

    // 2. Club record(s) in game_db.
    let gdb = secs.iter().find(|s| s.name == "game_db").expect("no game_db");
    let mut hits: Vec<resolve::ClubHit> = Vec::new();
    stream_overlapped(path, hdr, gdb, 512, &mut |base, buf| {
        resolve::scan_club_records(buf, base, &club_short, &mut hits);
    });
    hits.sort_by_key(|h| h.offset);
    hits.dedup_by_key(|h| h.offset);
    println!("club_record_hits = {}", hits.len());
    let mut ids: Vec<u32> = Vec::new();
    for h in hits.iter().take(8) {
        println!(
            "  off={} uid={} sec={} full='{}'",
            h.offset, h.uid, h.secondary, h.full
        );
        if h.uid > 0 {
            ids.push(h.uid);
        }
        if h.secondary > 0 {
            ids.push(h.secondary);
        }
    }
    ids.sort_unstable();
    ids.dedup();
    println!("candidate_ids = {ids:?}");

    // 3. For each candidate id, tally names that follow it.
    for id in ids.iter().take(4) {
        let mut tally = resolve::LinkTally::default();
        stream_overlapped(path, hdr, gdb, 256, &mut |_, buf| {
            resolve::scan_links(buf, *id, &mut tally, 24);
        });
        let mut v: Vec<_> = tally.names.into_iter().collect();
        v.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));
        println!("--- id {id}: top followed-by names ---");
        for (name, (count, marker, gap)) in v.into_iter().take(12) {
            let mh: Vec<String> = marker.iter().map(|b| format!("{:02x}", b)).collect();
            println!("   {count:>7}  gap={gap:<3} [{}]  {name}", mh.join(" "));
        }
    }
}

fn cmd_divs(path: &str, club_id: Option<u32>) {
    let (hdr, _, secs) = load_directory(path);
    let gdb = secs.iter().find(|s| s.name == "game_db").expect("no game_db");

    let mut divs: Vec<comp::Division> = Vec::new();
    stream_overlapped(path, hdr, gdb, 512, &mut |base, buf| {
        comp::scan_divisions(buf, base, &mut divs);
    });
    divs.sort_by_key(|d| d.offset);
    divs.dedup_by_key(|d| d.offset);
    println!("division_records = {}", divs.len());

    let mut idmap: std::collections::HashMap<u32, (String, usize)> =
        std::collections::HashMap::new();
    for d in &divs {
        for (slot, id) in d.ids.iter().enumerate() {
            if *id > 0 {
                idmap.entry(*id).or_insert((d.name.clone(), slot));
            }
        }
    }
    println!("distinct_ids = {}", idmap.len());
    for d in divs.iter().take(8) {
        println!("  {:?} ids={:?}", d.name, d.ids);
    }

    let Some(cid) = club_id else { return };
    let mut tally = std::collections::HashMap::new();
    stream_overlapped(path, hdr, gdb, 512, &mut |_, buf| {
        comp::tally_near(buf, cid, &idmap, 64, &mut tally);
    });
    let mut v: Vec<_> = tally.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    println!("--- divisions co-occurring with club {cid} ---");
    for ((name, slot), n) in v.into_iter().take(15) {
        println!("   {n:>8}  slot={slot}  {name}");
    }
}

/// Per-section counts of an id and a name -> narrows where the link lives.
/// Structural search for the club's competition-entry record:
///   <u16 comp_id> 00 00 00 00 <null date> 00 <null date> <null date> 00 00 00 00 <u32 club_id>
/// where a null date is 01 00 6C 07 (1/0/1900). The competition id sits 23
/// bytes before the club id; the null-date block is what identifies the record.
fn cmd_csig(path: &str, club: u32) {
    const NULLDATE: [u8; 4] = [0x01, 0x00, 0x6c, 0x07];
    let (hdr, _, secs) = load_directory(path);
    let gdb = secs.iter().find(|s| s.name == "game_db").expect("game_db");
    let cb = club.to_le_bytes();
    let mut found: std::collections::HashMap<u16, usize> = std::collections::HashMap::new();

    stream_overlapped(path, hdr, gdb, 256, &mut |base, buf| {
        let mut i = 32usize;
        while i + 4 <= buf.len() {
            if buf[i..i + 4] == cb
                && buf[i - 27..i - 23] == [0x10, 0, 0, 0]
                && buf[i - 17..i - 13] == NULLDATE
                && buf[i - 12..i - 8] == NULLDATE
                && buf[i - 8..i - 4] == NULLDATE
                && buf[i - 4..i] == [0, 0, 0, 0]
            {
                let comp = u16::from_le_bytes(buf[i - 23..i - 21].try_into().unwrap());
                *found.entry(comp).or_insert(0) += 1;
                println!("  record @{} -> competition_id={}", base + i as u64, comp);
            }
            i += 1;
        }
    });
    let mut v: Vec<_> = found.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    println!("club {club}: competition ids {:?}", v);
}

/// Print the sites where `comp` sits exactly `d` bytes from `club` (u16 field).
fn cmd_dsite(path: &str, club: u32, comp: u32, d: i64, ctx: usize) {
    let (hdr, _, secs) = load_directory(path);
    let gdb = secs.iter().find(|s| s.name == "game_db").expect("game_db");
    let cb = club.to_le_bytes();
    let c16 = (comp as u16).to_le_bytes();
    stream_overlapped(path, hdr, gdb, ctx * 2 + 128, &mut |base, buf| {
        let n = buf.len() as i64;
        let mut i = 0i64;
        while i + 4 <= n {
            if buf[i as usize..i as usize + 4] == cb {
                let p = i + d;
                if p >= 0
                    && p + 2 <= n
                    && buf[p as usize..p as usize + 2] == c16
                    && i - (ctx as i64) >= 0
                    && i + (ctx as i64) <= n
                {
                    let lo = (i - ctx as i64) as usize;
                    let hi = (i + ctx as i64) as usize;
                    let hex: Vec<String> =
                        buf[lo..hi].iter().map(|b| format!("{:02x}", b)).collect();
                    println!("SITE club_off={} (club at pre={})\n   {}", base + i as u64, ctx, hex.join(" "));
                }
            }
            i += 1;
        }
    });
}

fn fnv(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Three-way structural diff. A field that is the club's competition must read
/// `va` in save A and `vb` in BOTH save B and save C (two independent later
/// states of the same career), with byte-identical surrounding context.
/// Requiring the third save kills matches that coincide only in one pair.
fn cmd_xdiff3(pa: &str, pb: &str, pc: &str, va: u32, vb: u32, width: usize, ctx: usize) {
    let ov = ctx * 2 + width + 8;
    let hash_at = |buf: &[u8], i: usize| -> u64 {
        let mut v = Vec::with_capacity(ctx * 2);
        v.extend_from_slice(&buf[i - ctx..i]);
        v.extend_from_slice(&buf[i + width..i + width + ctx]);
        fnv(&v)
    };
    let val_at = |buf: &[u8], i: usize| -> u32 {
        match width {
            2 => u16::from_le_bytes(buf[i..i + 2].try_into().unwrap()) as u32,
            _ => u32::from_le_bytes(buf[i..i + 4].try_into().unwrap()),
        }
    };
    let distinctive = |buf: &[u8], i: usize| -> bool {
        buf[i - ctx..i]
            .iter()
            .chain(buf[i + width..i + width + ctx].iter())
            .filter(|&&b| b != 0)
            .count()
            >= ctx / 2
    };

    // Pass A: contexts holding va, unique within A.
    let (ha, _, sa) = load_directory(pa);
    let ga = sa.iter().find(|s| s.name == "game_db").expect("game_db A");
    let mut counts: std::collections::HashMap<u64, u32> = std::collections::HashMap::new();
    stream_overlapped(pa, ha, ga, ov, &mut |_, buf| {
        if buf.len() < ov { return; }
        let mut i = ctx;
        while i + width + ctx <= buf.len() {
            if val_at(buf, i) == va && distinctive(buf, i) {
                *counts.entry(hash_at(buf, i)).or_insert(0) += 1;
            }
            i += 1;
        }
    });
    let s1: std::collections::HashSet<u64> =
        counts.iter().filter(|(_, &c)| c == 1).map(|(&h, _)| h).collect();
    println!("A: {} unique contexts holding {}", s1.len(), va);

    // Pass B: keep those that also appear in B holding vb.
    let (hb, _, sb) = load_directory(pb);
    let gb = sb.iter().find(|s| s.name == "game_db").expect("game_db B");
    let mut s2: std::collections::HashSet<u64> = std::collections::HashSet::new();
    stream_overlapped(pb, hb, gb, ov, &mut |_, buf| {
        if buf.len() < ov { return; }
        let mut i = ctx;
        while i + width + ctx <= buf.len() {
            if val_at(buf, i) == vb && distinctive(buf, i) {
                let h = hash_at(buf, i);
                if s1.contains(&h) { s2.insert(h); }
            }
            i += 1;
        }
    });
    println!("B: {} survive holding {}", s2.len(), vb);

    // Pass C: the third state must agree too.
    let (hc, _, sc) = load_directory(pc);
    let gc = sc.iter().find(|s| s.name == "game_db").expect("game_db C");
    let mut hits = 0usize;
    stream_overlapped(pc, hc, gc, ov, &mut |base, buf| {
        if buf.len() < ov || hits >= 25 { return; }
        let mut i = ctx;
        while i + width + ctx <= buf.len() {
            if val_at(buf, i) == vb && distinctive(buf, i) && s2.contains(&hash_at(buf, i)) {
                let lo = i - ctx;
                let hi = i + width + ctx;
                let hex: Vec<String> = buf[lo..hi].iter().map(|b| format!("{:02x}", b)).collect();
                println!("SURVIVOR C_off={} field_at_pre={}\n   {}", base + i as u64, ctx, hex.join(" "));
                hits += 1;
                if hits >= 25 { break; }
            }
            i += 1;
        }
    });
    println!("survivors={}", hits);
}

/// Cross-save structural diff: find positions holding `va` in save A and `vb`
/// in save B with byte-identical surrounding context. Offsets shift between
/// saves, so the context hash — not the offset — is the anchor.
fn cmd_xdiff(pa: &str, pb: &str, va: u32, vb: u32, width: usize, ctx: usize) {
    let read_ctx = |buf: &[u8], i: usize| -> u64 {
        let mut v = Vec::with_capacity(ctx * 2);
        v.extend_from_slice(&buf[i - ctx..i]);
        v.extend_from_slice(&buf[i + width..i + width + ctx]);
        fnv(&v)
    };
    let matches = |buf: &[u8], i: usize, val: u32| -> bool {
        match width {
            2 => u16::from_le_bytes(buf[i..i + 2].try_into().unwrap()) as u32 == val,
            _ => u32::from_le_bytes(buf[i..i + 4].try_into().unwrap()) == val,
        }
    };

    // A context of mostly zero padding matches everywhere; require real content.
    let distinctive = |buf: &[u8], i: usize| -> bool {
        let nz = buf[i - ctx..i]
            .iter()
            .chain(buf[i + width..i + width + ctx].iter())
            .filter(|&&b| b != 0)
            .count();
        nz >= ctx / 2
    };

    let (ha, _, sa) = load_directory(pa);
    let ga = sa.iter().find(|s| s.name == "game_db").expect("game_db A");
    let mut counts: std::collections::HashMap<u64, u32> = std::collections::HashMap::new();
    let ov = ctx * 2 + width + 8;
    stream_overlapped(pa, ha, ga, ov, &mut |_, buf| {
        if buf.len() < ov {
            return;
        }
        let mut i = ctx;
        while i + width + ctx <= buf.len() {
            if matches(buf, i, va) && distinctive(buf, i) {
                *counts.entry(read_ctx(buf, i)).or_insert(0) += 1;
            }
            i += 1;
        }
    });
    // Keep only contexts that are unique in A, so a match identifies one site.
    let set: std::collections::HashSet<u64> =
        counts.iter().filter(|(_, &c)| c == 1).map(|(&h, _)| h).collect();
    println!(
        "save A: {} distinctive contexts holding {} ({} unique)",
        counts.len(),
        va,
        set.len()
    );

    let (hb, _, sb) = load_directory(pb);
    let gb = sb.iter().find(|s| s.name == "game_db").expect("game_db B");
    let mut hits = 0usize;
    stream_overlapped(pb, hb, gb, ov, &mut |base, buf| {
        if buf.len() < ov || hits >= 40 {
            return;
        }
        let mut i = ctx;
        while i + width + ctx <= buf.len() {
            if matches(buf, i, vb) && distinctive(buf, i) && set.contains(&read_ctx(buf, i)) {
                let lo = i - ctx;
                let hi = i + width + ctx;
                let hex: Vec<String> = buf[lo..hi].iter().map(|b| format!("{:02x}", b)).collect();
                println!("MATCH B_off={} pre={}\n   {}", base + i as u64, ctx, hex.join(" "));
                hits += 1;
                if hits >= 40 {
                    break;
                }
            }
            i += 1;
        }
    });
    println!("matches={}", hits);
}

/// Histogram of byte offsets at which `comp_id` sits relative to `club_id`.
/// A delta shared across saves is the club->competition field.
fn cmd_delta(path: &str, club_id: u32, comp_id: u32, win: i64) {
    let (hdr, _, secs) = load_directory(path);
    let gdb = secs.iter().find(|s| s.name == "game_db").expect("no game_db");
    let cb = club_id.to_le_bytes();
    let c16 = (comp_id as u16).to_le_bytes();
    let c32 = comp_id.to_le_bytes();
    let mut tally: std::collections::HashMap<(&str, i64), usize> = std::collections::HashMap::new();
    let mut occurrences = 0usize;

    stream_overlapped(path, hdr, gdb, (win as usize) * 2 + 32, &mut |_, buf| {
        let n = buf.len() as i64;
        let mut i = 0i64;
        while i + 4 <= n {
            if buf[i as usize..i as usize + 4] == cb {
                occurrences += 1;
                let mut d = -win;
                while d <= win {
                    let p = i + d;
                    if p >= 0 && p + 2 <= n && buf[p as usize..p as usize + 2] == c16 {
                        *tally.entry(("u16", d)).or_insert(0) += 1;
                    }
                    if p >= 0 && p + 4 <= n && buf[p as usize..p as usize + 4] == c32 {
                        *tally.entry(("u32", d)).or_insert(0) += 1;
                    }
                    d += 1;
                }
            }
            i += 1;
        }
    });

    let mut v: Vec<_> = tally.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    println!("# club {club_id} occurrences={occurrences} comp {comp_id}");
    for ((kind, d), n) in v {
        println!("D {kind} {d} {n}");
    }
}

fn cmd_findhex(path: &str, want: &str, hexpat: &str, ctx: usize, limit: usize) {
    let pat: Vec<u8> = (0..hexpat.len() / 2)
        .map(|i| u8::from_str_radix(&hexpat[i * 2..i * 2 + 2], 16).unwrap())
        .collect();
    let (hdr, _, secs) = load_directory(path);
    let s = secs.iter().find(|s| s.name == want).expect("section not found");
    let mut hits = 0usize;
    stream_overlapped(path, hdr, s, ctx * 2 + pat.len() + 16, &mut |base, buf| {
        if hits >= limit {
            return;
        }
        let mut i = 0usize;
        while i + pat.len() <= buf.len() {
            if buf[i..i + pat.len()] == pat[..] {
                let lo = i.saturating_sub(ctx);
                let hi = (i + pat.len() + ctx).min(buf.len());
                let hex: Vec<String> = buf[lo..hi].iter().map(|b| format!("{:02x}", b)).collect();
                println!("HIT off={} pre={}\n  hex {}", base + i as u64, i - lo, hex.join(" "));
                hits += 1;
                if hits >= limit {
                    break;
                }
            }
            i += 1;
        }
    });
    println!("hits={}", hits);
}

fn cmd_findid(path: &str, want: &str, id: u32, ctx: usize, limit: usize) {
    let (hdr, _, secs) = load_directory(path);
    let s = secs.iter().find(|s| s.name == want).expect("section not found");
    let idb = id.to_le_bytes();
    let mut hits = 0usize;
    stream_overlapped(path, hdr, s, ctx * 2 + 16, &mut |base, buf| {
        if hits >= limit {
            return;
        }
        let mut i = 0usize;
        while i + 4 <= buf.len() {
            if buf[i..i + 4] == idb {
                let lo = i.saturating_sub(ctx);
                let hi = (i + 4 + ctx).min(buf.len());
                let hex: Vec<String> = buf[lo..hi].iter().map(|b| format!("{:02x}", b)).collect();
                let asc: String = buf[lo..hi]
                    .iter()
                    .map(|&b| if (0x20..0x7f).contains(&b) { b as char } else { '.' })
                    .collect();
                println!("HIT off={} pre={}\n  hex {}\n  asc |{}|", base + i as u64, i - lo, hex.join(" "), asc);
                hits += 1;
                if hits >= limit {
                    break;
                }
            }
            i += 1;
        }
    });
    println!("hits={}", hits);
}

fn cmd_sweep(path: &str, id: u32, name: &str) {
    let (hdr, _, secs) = load_directory(path);
    let idb = id.to_le_bytes();
    let nb = name.as_bytes();
    println!("{:<40} {:>12} {:>8} {:>8}", "section", "decomp", "id", "name");
    for s in &secs {
        if s.comp == 0 {
            continue;
        }
        let mut idc = 0usize;
        let mut nc = 0usize;
        stream_overlapped(path, hdr, s, 256, &mut |_, buf| {
            let mut i = 0usize;
            while i + 4 <= buf.len() {
                if buf[i..i + 4] == idb {
                    idc += 1;
                }
                if i + nb.len() <= buf.len() && &buf[i..i + nb.len()] == nb {
                    nc += 1;
                }
                i += 1;
            }
        });
        if idc > 0 || nc > 0 {
            println!("{:<40} {:>12} {:>8} {:>8}", s.name, s.decomp, idc, nc);
        }
    }
}

/// Windows where both ids occur within `win` bytes -> candidate membership arrays.
fn cmd_pair(path: &str, id1: u32, id2: u32, win: u64) {
    let (hdr, _, secs) = load_directory(path);
    let gdb = secs.iter().find(|s| s.name == "game_db").expect("no game_db");
    let (b1, b2) = (id1.to_le_bytes(), id2.to_le_bytes());
    let mut o1: Vec<u64> = Vec::new();
    let mut o2: Vec<u64> = Vec::new();
    stream_overlapped(path, hdr, gdb, 256, &mut |base, buf| {
        let mut i = 0usize;
        while i + 4 <= buf.len() {
            if buf[i..i + 4] == b1 {
                o1.push(base + i as u64);
            } else if buf[i..i + 4] == b2 {
                o2.push(base + i as u64);
            }
            i += 1;
        }
    });
    o1.sort_unstable();
    o1.dedup();
    o2.sort_unstable();
    o2.dedup();
    println!("id{} count={}  id{} count={}", id1, o1.len(), id2, o2.len());
    let mut pairs: Vec<(u64, u64, u64)> = Vec::new();
    for &a in &o1 {
        let k = o2.partition_point(|&x| x < a);
        for c in [k.checked_sub(1), Some(k)].into_iter().flatten() {
            if let Some(&b) = o2.get(c) {
                let d = if b > a { b - a } else { a - b };
                if d <= win {
                    pairs.push((d, a, b));
                }
            }
        }
    }
    pairs.sort();
    println!("co-occurrences within {win}: {}", pairs.len());
    for (d, a, b) in pairs.iter().take(15) {
        println!("   dist={d:<6} id1@{a}  id2@{b}");
    }
}

/// Where does `id` sit relative to occurrences of `name`?
fn cmd_probe(path: &str, id: u32, name: &str) {
    let (hdr, _, secs) = load_directory(path);
    let gdb = secs.iter().find(|s| s.name == "game_db").expect("no game_db");
    let nb = name.as_bytes();

    // Pass 1: offsets of the competition name string.
    let mut name_offs: Vec<u64> = Vec::new();
    stream_overlapped(path, hdr, gdb, 256, &mut |base, buf| {
        let mut i = 0usize;
        while i + nb.len() <= buf.len() {
            if &buf[i..i + nb.len()] == nb {
                name_offs.push(base + i as u64);
            }
            i += 1;
        }
    });
    name_offs.sort_unstable();
    name_offs.dedup();
    println!("name '{}' occurrences: {}", name, name_offs.len());

    // Pass 2: offsets of the id, tracking the closest pairing.
    let idb = id.to_le_bytes();
    let mut best: Option<(u64, u64, u64)> = None;
    let mut id_count = 0usize;
    stream_overlapped(path, hdr, gdb, 256, &mut |base, buf| {
        let mut i = 0usize;
        while i + 4 <= buf.len() {
            if buf[i..i + 4] == idb {
                id_count += 1;
                let o = base + i as u64;
                let k = name_offs.partition_point(|&x| x < o);
                for c in [k.checked_sub(1), Some(k)].into_iter().flatten() {
                    if let Some(&no) = name_offs.get(c) {
                        let d = if no > o { no - o } else { o - no };
                        if best.map(|b| d < b.0).unwrap_or(true) {
                            best = Some((d, o, no));
                        }
                    }
                }
            }
            i += 1;
        }
    });
    println!("id {} occurrences: {}", id, id_count);
    match best {
        Some((d, o, no)) => println!("closest: distance={} id_off={} name_off={}", d, o, no),
        None => println!("no pairing"),
    }
}

fn cmd_linkscan(path: &str, id: u32) {
    let (hdr, _, secs) = load_directory(path);
    let gdb = secs.iter().find(|s| s.name == "game_db").expect("no game_db");
    let mut tally = resolve::LinkTally::default();
    stream_overlapped(path, hdr, gdb, 256, &mut |_, buf| {
        resolve::scan_links(buf, id, &mut tally, 24);
    });
    let mut v: Vec<_> = tally.names.into_iter().collect();
    v.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));
    println!("--- id {id} ---");
    for (name, (count, marker, gap)) in v.into_iter().take(15) {
        let mh: Vec<String> = marker.iter().map(|b| format!("{:02x}", b)).collect();
        println!("   {count:>7}  gap={gap:<3} [{}]  {name}", mh.join(" "));
    }
}

fn cmd_dir(path: &str) {
    let (hdr, name, secs) = load_directory(path);
    println!("main_header_len={hdr}");
    println!("save_name={name}");
    println!("sections={}", secs.len());
    for s in &secs {
        println!(
            "{:<40} off={:>12} comp={:>11} decomp={:>12} ext={}",
            s.name, s.offset, s.comp, s.decomp, s.ext
        );
    }
}

fn cmd_sect(path: &str, want: &str, out: &str) {
    let (hdr, _, secs) = load_directory(path);
    let s = secs.iter().find(|s| s.name == want).expect("section not found");
    let data = section_bytes(path, hdr, s);
    std::fs::write(out, &data).expect("write");
    println!(
        "section={} declared_decomp={} actual={} -> {}",
        s.name,
        s.decomp,
        data.len(),
        out
    );
}

/// Walk the compact team table: `<u32 team_ref> <u32 club_uid> <u32 club_uid> 0a 00 …`.
/// The pair is the same one the `mst.` match records name, which is what makes
/// this a decoded record rather than a proximity guess.
fn cmd_teamtable(path: &str, want_club: u32, want_team: u32, dump: usize) {
    let (hdr, _, secs) = load_directory(path);
    let gdb = secs.iter().find(|s| s.name == "game_db").expect("no game_db");
    let mut rows: Vec<(u64, u32, u32, u8, u16, u32, u32, u32, u32, u16)> = Vec::new();

    stream_overlapped(path, hdr, gdb, 1 << 16, &mut |base, buf| {
        let mut i = 0usize;
        while i + 48 <= buf.len() {
            let a = u32::from_le_bytes(buf[i..i + 4].try_into().unwrap());
            if a == 0 || a > 20_000_000 {
                i += 1;
                continue;
            }
            let b = u32::from_le_bytes(buf[i + 4..i + 8].try_into().unwrap());
            let c = u32::from_le_bytes(buf[i + 8..i + 12].try_into().unwrap());
            if b == 0 || b == u32::MAX || b != c || buf[i + 12] != 0x0a || buf[i + 13] != 0x00 {
                i += 1;
                continue;
            }
            let f1 = buf[i + 14];
            let f2 = u16::from_le_bytes(buf[i + 15..i + 17].try_into().unwrap());
            let g1 = u32::from_le_bytes(buf[i + 17..i + 21].try_into().unwrap());
            let g2 = u32::from_le_bytes(buf[i + 21..i + 25].try_into().unwrap());
            let h1 = u32::from_le_bytes(buf[i + 29..i + 33].try_into().unwrap());
            let h2 = u32::from_le_bytes(buf[i + 33..i + 37].try_into().unwrap());
            let n = u16::from_le_bytes(buf[i + 46..i + 48].try_into().unwrap());
            rows.push((base + i as u64, a, b, f1, f2, g1, g2, h1, h2, n));
            i += 12;
        }
    });
    rows.sort_by_key(|r| r.0);
    rows.dedup_by_key(|r| r.0);
    println!("team_records={}", rows.len());
    let mut shown = 0;
    for r in &rows {
        if want_club != 0 && r.2 != want_club {
            continue;
        }
        if want_team != 0 && r.1 != want_team {
            continue;
        }
        if want_club == 0 && want_team == 0 && shown >= dump {
            break;
        }
        println!(
            "off={} team_ref={} club_uid={} f1={} f2={} g1={} g2={} h1={} h2={} players={}",
            r.0, r.1, r.2, r.3, r.4, r.5, r.6, r.7, r.8, r.9
        );
        shown += 1;
    }
    println!("shown={shown}");
}

/// Collect `team_ref -> club_uid` from the compact team table.
fn team_map(path: &str, hdr: u64, gdb: &Section) -> std::collections::HashMap<u32, u32> {
    let mut m = std::collections::HashMap::new();
    stream_overlapped(path, hdr, gdb, 1 << 16, &mut |_, buf| {
        let mut i = 0usize;
        while i + 48 <= buf.len() {
            let a = u32::from_le_bytes(buf[i..i + 4].try_into().unwrap());
            if a == 0 || a > 20_000_000 {
                i += 1;
                continue;
            }
            let b = u32::from_le_bytes(buf[i + 4..i + 8].try_into().unwrap());
            let c = u32::from_le_bytes(buf[i + 8..i + 12].try_into().unwrap());
            if b == 0 || b == u32::MAX || b != c || buf[i + 12] != 0x0a || buf[i + 13] != 0x00 {
                i += 1;
                continue;
            }
            m.insert(a, b);
            i += 12;
        }
    });
    m
}

/// Constant-stride runs of distinct team refs — the shape a league table or a
/// competition's participant array takes. Reported only when the run contains
/// `want`, so the output is anchored on a known team rather than fished for.
fn cmd_teamruns(path: &str, want: u32, min_run: usize, limit: usize, section: &str) {
    let (hdr, _, secs) = load_directory(path);
    let gdb = secs.iter().find(|s| s.name == "game_db").expect("no game_db");
    let teams = team_map(path, hdr, gdb);
    println!("team_refs={} want={want}", teams.len());
    let sec = secs.iter().find(|s| s.name == section).expect("section");

    let strides: Vec<usize> = (1..=32).map(|k| k * 4).collect();
    let maxspan = 32 * 4 * 64 + 64;
    let mut found = 0usize;
    stream_overlapped(path, hdr, sec, maxspan, &mut |base, buf| {
        if found >= limit {
            return;
        }
        let mut i = 0usize;
        while i + 8 <= buf.len() {
            let Some(v) = rd_u32(buf, i) else { break };
            if !teams.contains_key(&v) {
                i += 1;
                continue;
            }
            for &s in &strides {
                let mut seen = vec![v];
                let mut n = 1usize;
                while i + n * s + 4 <= buf.len() {
                    let Some(x) = rd_u32(buf, i + n * s) else { break };
                    if !teams.contains_key(&x) || seen.contains(&x) {
                        break;
                    }
                    seen.push(x);
                    n += 1;
                }
                if n >= min_run && seen.contains(&want) {
                    println!("run off={} stride={} n={} teams={:?}", base + i as u64, s, n, seen);
                    found += 1;
                    if found >= limit {
                        return;
                    }
                    i += n * s;
                    break;
                }
            }
            i += 1;
        }
    });
    println!("runs={found}");
}

/// Find byte windows in which *every* listed id occurs. Used to test whether a
/// competition's known participants are stored together as one array.
fn cmd_cluster(path: &str, ids: &str, win: u64, section: &str, limit: usize) {
    let (hdr, _, secs) = load_directory(path);
    let sec = secs.iter().find(|s| s.name == section).expect("section");
    let want: Vec<u32> = ids.split(',').filter_map(|s| s.trim().parse().ok()).collect();
    let pats: Vec<[u8; 4]> = want.iter().map(|v| v.to_le_bytes()).collect();

    let mut offs: Vec<Vec<u64>> = vec![Vec::new(); want.len()];
    stream_overlapped(path, hdr, sec, 16, &mut |base, buf| {
        let mut i = 0usize;
        while i + 4 <= buf.len() {
            let w = &buf[i..i + 4];
            for (k, p) in pats.iter().enumerate() {
                if w == p {
                    offs[k].push(base + i as u64);
                }
            }
            i += 1;
        }
    });
    for (k, v) in offs.iter_mut().enumerate() {
        v.sort_unstable();
        v.dedup();
        println!("id {} occurrences {}", want[k], v.len());
    }

    // Walk the rarest id's sites; a site qualifies when every other id has an
    // occurrence within `win` bytes of it.
    let (rarest, _) = offs
        .iter()
        .enumerate()
        .min_by_key(|(_, v)| v.len())
        .map(|(i, v)| (i, v.len()))
        .unwrap();
    let mut found = 0usize;
    for &o in &offs[rarest] {
        let lo = o.saturating_sub(win);
        let hi = o + win;
        let mut near: Vec<(u32, u64)> = Vec::new();
        let mut all = true;
        for (k, v) in offs.iter().enumerate() {
            let idx = v.partition_point(|&x| x < lo);
            match v.get(idx) {
                Some(&x) if x <= hi => near.push((want[k], x)),
                _ => {
                    all = false;
                    break;
                }
            }
        }
        if all {
            near.sort_by_key(|x| x.1);
            println!("cluster around {o}: {near:?}");
            found += 1;
            if found >= limit {
                break;
            }
        }
    }
    println!("clusters={found}");
}

/// For each `a:b` pair, tally the byte delta at which `b` follows `a`, then
/// report the deltas that hold for *every* pair. One pair proves nothing; a
/// delta shared by many independent pairs is a record field.
fn cmd_pairdelta(path: &str, pairs: &str, win: i64, section: &str) {
    let (hdr, _, secs) = load_directory(path);
    let sec = secs.iter().find(|s| s.name == section).expect("section");
    let parsed: Vec<(u32, u32)> = pairs
        .split(',')
        .filter_map(|p| {
            let (a, b) = p.split_once(':')?;
            Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
        })
        .collect();
    println!("pairs={} window=±{win} section={section}", parsed.len());

    // delta -> (pairs that show it, total hits, one example offset)
    let mut per_delta: std::collections::HashMap<i64, (Vec<(u32, u32)>, usize, u64)> =
        Default::default();

    for &(a, b) in &parsed {
        let ab = a.to_le_bytes();
        let bb = b.to_le_bytes();
        let mut seen: std::collections::HashMap<i64, (usize, u64)> = Default::default();
        stream_overlapped(path, hdr, sec, (win as usize) * 2 + 32, &mut |base, buf| {
            let n = buf.len() as i64;
            let mut i = 0i64;
            while i + 4 <= n {
                if buf[i as usize..i as usize + 4] == ab {
                    let mut d = -win;
                    while d <= win {
                        let p = i + d;
                        if p >= 0 && p + 4 <= n && buf[p as usize..p as usize + 4] == bb {
                            let e = seen.entry(d).or_insert((0, base + i as u64));
                            e.0 += 1;
                        }
                        d += 1;
                    }
                }
                i += 1;
            }
        });
        for (d, (n, off)) in seen {
            let e = per_delta.entry(d).or_insert((Vec::new(), 0, off));
            e.0.push((a, b));
            e.1 += n;
        }
        println!("  pair {a}:{b} done");
    }

    let mut v: Vec<_> = per_delta.into_iter().collect();
    v.sort_by_key(|(_, (ps, n, _))| std::cmp::Reverse((ps.len(), *n)));
    println!("{:>7} {:>7} {:>8}  example_off", "delta", "pairs", "hits");
    for (d, (ps, n, off)) in v.into_iter().take(20) {
        println!("{:>7} {:>7} {:>8}  {}", d, ps.len(), n, off);
    }
}

/// Numeric prefix of a `<key>_<time>[_<n>]` section name.
fn section_key(name: &str) -> Option<u32> {
    name.split('_').next().and_then(|s| s.parse().ok())
}

/// Enumerate every `.scm`/`.apm` section and decode its `mst.` record.
fn cmd_compsections(path: &str) {
    let (hdr, save, secs) = load_directory(path);
    println!("save={save}");
    println!(
        "{:<26} {:>7} {:>4} {:>9} {:>10} {:>10} {:>5} {:>6} {:>4}  {}",
        "section", "key", "ext", "comp", "decomp", "comp_uid", "nat", "stage", "yr", "name / venue / groups"
    );
    for s in &secs {
        let ext = mst::Ext::parse(&s.ext);
        if ext == mst::Ext::Other {
            continue;
        }
        let data = section_bytes(path, hdr, s);
        let r = mst::parse(&data);
        println!(
            "{:<26} {:>7} {:>4} {:>9} {:>10} {:>10} {:>5} {:>6} {:>4}  {} | {} | {}",
            s.name,
            section_key(&s.name).map(|k| k.to_string()).unwrap_or_else(|| "-".into()),
            ext.as_str(),
            s.comp,
            s.decomp,
            r.competition_uid.map(|v| v.to_string()).unwrap_or_else(|| "-".into()),
            r.nation_id.map(|v| v.to_string()).unwrap_or_else(|| "-".into()),
            r.stage_id.map(|v| v.to_string()).unwrap_or_else(|| "-".into()),
            r.year.map(|v| v.to_string()).unwrap_or_else(|| "-".into()),
            r.name.clone().unwrap_or_default(),
            r.short_name.clone().unwrap_or_default(),
            r.venue.clone().unwrap_or_default(),
        );
        println!(
            "    magic_ok={} stamp={:?} venue_ids={:?} evidence={:?}",
            r.magic_ok, r.format_stamp, r.venue_ids, r.evidence_offsets
        );
        for g in &r.reference_groups {
            println!(
                "    group kind=0x{:02x} n={} [{}..{}] refs={:?}",
                g.tag_or_kind,
                g.references.len(),
                g.start_offset,
                g.end_offset,
                g.references
            );
        }
    }
}

/// Walk the compact club table across `game_db` and report every record.
fn cmd_clubtable(path: &str, filter: &str, want_id: u32) {
    let (hdr, _, secs) = load_directory(path);
    let gdb = secs.iter().find(|s| s.name == "game_db").expect("no game_db");

    // Overlap comfortably exceeds one club record so every record is complete
    // in at least one window; duplicates are removed by offset afterwards.
    const OVERLAP: usize = 1 << 20;
    let mut rows: Vec<clubtable::ClubRow> = Vec::new();
    stream_overlapped(path, hdr, gdb, OVERLAP, &mut |base, buf| {
        let mut found: Vec<clubtable::ClubRow> = Vec::new();
        clubtable::scan_club_rows(buf, base, &mut found);
        // A record's span ends where the next record begins; the last one in the
        // window has no successor here, so it is left for the next window.
        for k in 0..found.len().saturating_sub(1) {
            let mut r = found[k].clone();
            r.end = found[k + 1].offset;
            let from = (r.body_at - base) as usize;
            let to = (r.end - base) as usize;
            r.body_len = to.saturating_sub(from);
            r.regs = clubtable::scan_registrations(buf, base, from, to);
            rows.push(r);
        }
    });
    rows.sort_by_key(|r| r.offset);
    rows.dedup_by_key(|r| r.offset);

    let mut shown = 0usize;
    for r in &rows {
        let hit = (want_id != 0 && r.compact_id == want_id)
            || (!filter.is_empty()
                && (r.full.contains(filter) || r.short.contains(filter)));
        if want_id == 0 && filter.is_empty() {
            // Summary mode.
        } else if !hit {
            continue;
        }
        shown += 1;
        println!(
            "club off={} id={} nation={} body=[{}..{}) len={} full='{}' short='{}'",
            r.offset, r.compact_id, r.nation_id, r.body_at, r.end, r.body_len, r.full, r.short
        );
        for g in &r.regs {
            println!(
                "    reg kind={} comp_uid={} second={} @{}",
                g.kind, g.comp_uid, g.second, g.offset
            );
        }
    }
    println!("club_records={} shown={}", rows.len(), shown);
}

/// Dump one club record: every aligned u32 with its relative offset.
fn cmd_clubrefs(path: &str, want_id: u32, max_words: usize) {
    let (hdr, _, secs) = load_directory(path);
    let gdb = secs.iter().find(|s| s.name == "game_db").expect("no game_db");
    const OVERLAP: usize = 1 << 20;
    let mut out: Option<(clubtable::ClubRow, Vec<u8>)> = None;
    stream_overlapped(path, hdr, gdb, OVERLAP, &mut |base, buf| {
        if out.is_some() {
            return;
        }
        let mut found: Vec<clubtable::ClubRow> = Vec::new();
        clubtable::scan_club_rows(buf, base, &mut found);
        for k in 0..found.len().saturating_sub(1) {
            if found[k].compact_id != want_id {
                continue;
            }
            let mut r = found[k].clone();
            r.end = found[k + 1].offset;
            let from = (r.offset - base) as usize;
            let to = (r.end - base) as usize;
            r.regs = clubtable::scan_registrations(buf, base, (r.body_at - base) as usize, to);
            out = Some((r, buf[from..to].to_vec()));
            return;
        }
    });
    let Some((r, bytes)) = out else {
        println!("club id {want_id} not found");
        return;
    };
    println!(
        "club off={} id={} nation={} record_len={} full='{}' short='{}'",
        r.offset,
        r.compact_id,
        r.nation_id,
        bytes.len(),
        r.full,
        r.short
    );
    println!("body_at={} (rel {})", r.body_at, r.body_at - r.offset);
    for g in &r.regs {
        println!(
            "  reg kind={} comp_uid={} second={} abs={} rel={}",
            g.kind,
            g.comp_uid,
            g.second,
            g.offset,
            g.offset - r.offset
        );
    }
    println!("  --- aligned u32 words (rel, abs, value) ---");
    let mut i = 0usize;
    let mut n = 0usize;
    while i + 4 <= bytes.len() && n < max_words {
        let v = u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap());
        if v != 0 && v != u32::MAX {
            println!("  rel={:<6} abs={:<12} u32={}", i, r.offset + i as u64, v);
            n += 1;
        }
        i += 4;
    }
}

/// Offsets only — a cheap way to see where an id lives without hex spam.
fn cmd_idoff(path: &str, want: &str, id: u32, limit: usize) {
    let (hdr, _, secs) = load_directory(path);
    let s = secs.iter().find(|s| s.name == want).expect("section not found");
    let idb = id.to_le_bytes();
    let mut offs: Vec<u64> = Vec::new();
    stream_overlapped(path, hdr, s, 16, &mut |base, buf| {
        let mut i = 0usize;
        while i + 4 <= buf.len() {
            if buf[i..i + 4] == idb {
                offs.push(base + i as u64);
            }
            i += 1;
        }
    });
    offs.sort_unstable();
    offs.dedup();
    println!("id={id} section={want} hits={}", offs.len());
    for o in offs.iter().take(limit) {
        println!("  {o}");
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    match a.get(1).map(|s| s.as_str()) {
        Some("dir") => cmd_dir(&a[2]),
        Some("compsections") => cmd_compsections(&a[2]),
        Some("teamruns") => cmd_teamruns(&a[2], a[3].parse().unwrap(), a.get(4).and_then(|x| x.parse().ok()).unwrap_or(8), a.get(5).and_then(|x| x.parse().ok()).unwrap_or(20), a.get(6).map(|s| s.as_str()).unwrap_or("game_db")),
        Some("teamtable") => cmd_teamtable(&a[2], a.get(3).and_then(|x| x.parse().ok()).unwrap_or(0), a.get(4).and_then(|x| x.parse().ok()).unwrap_or(0), a.get(5).and_then(|x| x.parse().ok()).unwrap_or(40)),
        Some("cluster") => cmd_cluster(&a[2], &a[3], a.get(4).and_then(|x| x.parse().ok()).unwrap_or(512), a.get(5).map(|s| s.as_str()).unwrap_or("game_db"), a.get(6).and_then(|x| x.parse().ok()).unwrap_or(20)),
        Some("pairdelta") => cmd_pairdelta(
            &a[2],
            &a[3],
            a.get(4).and_then(|x| x.parse().ok()).unwrap_or(64),
            a.get(5).map(|s| s.as_str()).unwrap_or("game_db"),
        ),
        Some("clubtable") => cmd_clubtable(
            &a[2],
            a.get(3).map(|s| s.as_str()).unwrap_or(""),
            a.get(4).and_then(|x| x.parse().ok()).unwrap_or(0),
        ),
        Some("clubrefs") => cmd_clubrefs(
            &a[2],
            a[3].parse().unwrap(),
            a.get(4).and_then(|x| x.parse().ok()).unwrap_or(400),
        ),
        Some("idoff") => cmd_idoff(
            &a[2],
            &a[3],
            a[4].parse().unwrap(),
            a.get(5).and_then(|x| x.parse().ok()).unwrap_or(60),
        ),
        Some("sect") => cmd_sect(&a[2], &a[3], &a[4]),
        Some("relsite") => cmd_relsite(&a[2], a[3].parse().unwrap(), a[4].parse().unwrap(), a[5].parse().unwrap(), a[6].parse().unwrap(), a[7].parse().unwrap()),
        Some("cohort") => cmd_cohort(&a[2], a[3].parse().unwrap(), a[4].parse().unwrap(), a[5].parse().unwrap(), a[6].parse().unwrap()),
        Some("teamid") => cmd_teamid(&a[2], a[3].parse().unwrap(), a[4].parse().unwrap(), a[5].parse().unwrap(), a[6].parse().unwrap(), a.get(7).and_then(|x| x.parse().ok()).unwrap_or(400)),
        Some("divscan") => cmd_divscan(&a[2], a[3].parse().unwrap(), a[4].parse().unwrap()),
        Some("proxdump") => cmd_proxdump(&a[2], a[3].parse().unwrap(), a[4].parse().unwrap(), a[5].parse().unwrap(), a[6].parse().unwrap(), a[7].parse().unwrap()),
        Some("compteams") => cmd_compteams(&a[2], a[3].parse().unwrap(), a[4].parse().unwrap(), a[5].parse().unwrap(), a[6].parse().unwrap()),
        Some("internal") => cmd_internal(&a[2], &a[3], a[4].parse().unwrap(), a[5].parse().unwrap(), a[6].parse().unwrap()),
        Some("secprox") => cmd_secprox(&a[2], a[3].parse().unwrap(), a[4].parse().unwrap(), a[5].parse().unwrap()),
        Some("teamarray") => cmd_teamarray(&a[2], a[3].parse().unwrap(), a[4].parse().unwrap(), a.get(5).and_then(|x| x.parse().ok()).unwrap_or(0)),
        Some("regscan") => cmd_regscan(&a[2], a.get(3).and_then(|x| x.parse().ok()).unwrap_or(6)),
        Some("names") => cmd_names(&a[2], a.get(3).map(|s| s.as_str()).unwrap_or("")),
        Some("relscan") => cmd_relscan(&a[2], a[3].parse().unwrap(), a[4].parse().unwrap(), a[5].parse().unwrap()),
        Some("tagdens") => cmd_tagdens(&a[2], &a[3], a[4].parse().unwrap(), a[5].parse().unwrap()),
        Some("namewin") => cmd_namewin(
            &a[2],
            &a[3],
            &a[4],
            a[5].parse().unwrap(),
            a[6].parse().unwrap(),
            &a[7],
        ),
        Some("resolve") => cmd_resolve(&a[2]),
        Some("linkscan") => cmd_linkscan(&a[2], a[3].parse().unwrap()),
        Some("probe") => cmd_probe(&a[2], a[3].parse().unwrap(), &a[4]),
        Some("divs") => cmd_divs(&a[2], a.get(3).and_then(|x| x.parse().ok())),
        Some("csig") => cmd_csig(&a[2], a[3].parse().unwrap()),
        Some("dsite") => cmd_dsite(&a[2], a[3].parse().unwrap(), a[4].parse().unwrap(), a[5].parse().unwrap(), a[6].parse().unwrap()),
        Some("xdiff3") => cmd_xdiff3(&a[2], &a[3], &a[4], a[5].parse().unwrap(), a[6].parse().unwrap(), a[7].parse().unwrap(), a[8].parse().unwrap()),
        Some("xdiff") => cmd_xdiff(&a[2], &a[3], a[4].parse().unwrap(), a[5].parse().unwrap(), a[6].parse().unwrap(), a[7].parse().unwrap()),
        Some("delta") => cmd_delta(&a[2], a[3].parse().unwrap(), a[4].parse().unwrap(), a[5].parse().unwrap()),
        Some("findhex") => cmd_findhex(&a[2], &a[3], &a[4], a[5].parse().unwrap(), a[6].parse().unwrap()),
        Some("findid") => cmd_findid(&a[2], &a[3], a[4].parse().unwrap(), a[5].parse().unwrap(), a[6].parse().unwrap()),
        Some("sweep") => cmd_sweep(&a[2], a[3].parse().unwrap(), &a[4]),
        Some("pair") => cmd_pair(&a[2], a[3].parse().unwrap(), a[4].parse().unwrap(), a[5].parse().unwrap()),
        Some("slice") => cmd_slice(&a[2], &a[3], a[4].parse().unwrap(), a[5].parse().unwrap(), &a[6]),
        Some("find") => {
            let ctx = a.get(5).and_then(|s| s.parse().ok()).unwrap_or(48);
            let limit = a.get(6).and_then(|s| s.parse().ok()).unwrap_or(10);
            cmd_find(&a[2], &a[3], &a[4], ctx, limit)
        }
        _ => eprintln!("usage: fmtool dir|sect|find ..."),
    }
}
