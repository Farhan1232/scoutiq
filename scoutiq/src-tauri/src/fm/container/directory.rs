//! FM26 section directory (read-only).
//!
//! An `.fm` save is laid out as:
//!
//! ```text
//! [ outer header: 02 01 "fmf." … ][ zstd frame ][ zstd frame ] … [ directory container ]
//! ```
//!
//! Every logical section (`game_db`, `save_game_summary`, `humans`, …) is one or
//! more concatenated Zstd frames. A **directory** is appended at the very end of
//! the file as its own `02 01 "fmf."` container. Decompressed, it reads:
//!
//! ```text
//! LP(save_name) | u32 entry_count | entry × entry_count
//!
//! entry := LP(name) LP(ext) u64 offset u64 compressed_size u64 decompressed_size u64 _ u64 _
//! ```
//!
//! `offset` is relative to the end of the outer header, so
//! `file_offset = header_len + offset`.
//!
//! This replaces blind whole-file frame scanning: any section can be located and
//! decompressed on its own without touching the rest of a multi-hundred-MB save.
//! Saves are opened with [`File::open`] only and are never written to.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::fm::errors::{map_io_error, FmError};

const ZSTD_MAGIC: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];
const FMF_SIG: [u8; 6] = [0x02, 0x01, b'f', b'm', b'f', b'.'];
/// Trailing window searched for the directory container.
const TAIL_SCAN: u64 = 4 * 1024 * 1024;
/// Guards against a corrupt count driving a huge allocation.
const MAX_ENTRIES: u32 = 10_000;
/// Per-entry trailer following the three size/offset u64s.
const ENTRY_TRAILER: usize = 16;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionEntry {
    pub name: String,
    pub ext: String,
    /// Offset relative to the end of the outer header.
    pub offset: u64,
    pub compressed_size: u64,
    pub decompressed_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FmDirectory {
    /// Length of the outer header; section offsets are relative to it.
    pub header_len: u64,
    /// Save name as stored in the container (not the filename).
    pub save_name: String,
    pub sections: Vec<SectionEntry>,
}

impl FmDirectory {
    pub fn section(&self, name: &str) -> Option<&SectionEntry> {
        self.sections.iter().find(|s| s.name == name)
    }

    pub fn file_offset(&self, s: &SectionEntry) -> u64 {
        self.header_len + s.offset
    }
}

fn rd_u32(d: &[u8], at: usize) -> Option<u32> {
    d.get(at..at + 4)
        .map(|s| u32::from_le_bytes(s.try_into().unwrap()))
}

fn rd_u64(d: &[u8], at: usize) -> Option<u64> {
    d.get(at..at + 8)
        .map(|s| u64::from_le_bytes(s.try_into().unwrap()))
}

/// Length-prefixed UTF-8 string (u32 length).
fn rd_lp(d: &[u8], at: usize, max: usize) -> Option<(String, usize)> {
    let len = rd_u32(d, at)? as usize;
    if len > max || at + 4 + len > d.len() {
        return None;
    }
    let s = String::from_utf8_lossy(&d[at + 4..at + 4 + len]).into_owned();
    Some((s, at + 4 + len))
}

/// Header length = offset of the first Zstd magic after the `02 01 fmf.` signature.
fn header_len(head: &[u8]) -> Option<u64> {
    if head.len() < FMF_SIG.len() || head[..FMF_SIG.len()] != FMF_SIG {
        return None;
    }
    head.windows(4)
        .position(|w| w == ZSTD_MAGIC)
        .map(|p| p as u64)
}

/// Decompress every concatenated Zstd frame in `src`, stopping at the first non-frame.
fn decompress_frames(src: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    while pos + 4 <= src.len() && src[pos..pos + 4] == ZSTD_MAGIC {
        let Ok(dec) = zstd::stream::read::Decoder::new(&src[pos..]) else {
            break;
        };
        let mut dec = dec.single_frame();
        let mut buf = Vec::new();
        if dec.read_to_end(&mut buf).is_err() && buf.is_empty() {
            break;
        }
        out.extend_from_slice(&buf);
        // Re-locate the next frame; the decoder does not report bytes consumed.
        match src[pos + 4..]
            .windows(4)
            .position(|w| w == ZSTD_MAGIC)
            .map(|p| pos + 4 + p)
        {
            Some(next) => pos = next,
            None => break,
        }
    }
    out
}

/// Parse the trailing section directory. Opens `path` read-only.
pub fn parse_directory(path: &Path) -> Result<FmDirectory, FmError> {
    let mut f = File::open(path).map_err(|e| map_io_error(&e, path))?;
    let file_len = f.metadata().map_err(|e| map_io_error(&e, path))?.len();
    if file_len < 32 {
        return Err(FmError::container());
    }

    let mut head = vec![0u8; 64.min(file_len as usize)];
    f.read_exact(&mut head)
        .map_err(|e| map_io_error(&e, path))?;
    let hdr = header_len(&head).ok_or_else(FmError::container)?;

    let tail_len = TAIL_SCAN.min(file_len);
    let tail_start = file_len - tail_len;
    f.seek(SeekFrom::Start(tail_start))
        .map_err(|e| map_io_error(&e, path))?;
    let mut tail = vec![0u8; tail_len as usize];
    f.read_exact(&mut tail)
        .map_err(|e| map_io_error(&e, path))?;

    // The directory is the last valid `fmf.` container in the file.
    let mut starts: Vec<usize> = Vec::new();
    for i in 0..tail.len().saturating_sub(FMF_SIG.len()) {
        if tail[i..i + FMF_SIG.len()] == FMF_SIG {
            starts.push(i);
        }
    }

    for &i in starts.iter().rev() {
        let sub = &tail[i..];
        let Some(hl) = header_len(sub) else { continue };
        let raw = decompress_frames(&sub[hl as usize..]);
        if let Some(dir) = parse_directory_bytes(&raw, hdr) {
            return Ok(dir);
        }
    }
    Err(FmError::container())
}

/// Parse a decompressed directory payload. Exposed for tests.
pub fn parse_directory_bytes(raw: &[u8], header_len: u64) -> Option<FmDirectory> {
    let (save_name, mut p) = rd_lp(raw, 0, 512)?;
    let count = rd_u32(raw, p)?;
    if count == 0 || count > MAX_ENTRIES {
        return None;
    }
    p += 4;

    let mut sections = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let (name, p1) = rd_lp(raw, p, 256)?;
        let (ext, p2) = rd_lp(raw, p1, 32)?;
        let offset = rd_u64(raw, p2)?;
        let compressed_size = rd_u64(raw, p2 + 8)?;
        let decompressed_size = rd_u64(raw, p2 + 16)?;
        if name.is_empty() {
            return None;
        }
        sections.push(SectionEntry {
            name,
            ext,
            offset,
            compressed_size,
            decompressed_size,
        });
        p = p2 + 24 + ENTRY_TRAILER;
    }

    Some(FmDirectory {
        header_len,
        save_name,
        sections,
    })
}

/// Open a decoder over exactly one section's compressed extent.
/// The decoder spans concatenated frames, so a section split across several
/// frames decodes as one continuous stream.
fn section_decoder(
    path: &Path,
    dir: &FmDirectory,
    entry: &SectionEntry,
) -> Result<zstd::Decoder<'static, std::io::BufReader<std::io::Take<File>>>, FmError> {
    let mut f = File::open(path).map_err(|e| map_io_error(&e, path))?;
    f.seek(SeekFrom::Start(dir.file_offset(entry)))
        .map_err(|e| map_io_error(&e, path))?;
    zstd::Decoder::new(f.take(entry.compressed_size)).map_err(|_| FmError::decompress())
}

/// Fully decompress one section. Intended for small sections such as
/// `save_game_summary`; use [`stream_section`] for `game_db`-sized payloads.
pub fn read_section(
    path: &Path,
    dir: &FmDirectory,
    entry: &SectionEntry,
) -> Result<Vec<u8>, FmError> {
    let mut dec = section_decoder(path, dir, entry)?;
    let mut out = Vec::with_capacity(entry.decompressed_size.min(1 << 24) as usize);
    dec.read_to_end(&mut out)
        .map_err(|_| FmError::decompress())?;
    Ok(out)
}

/// Stream a section's decompressed bytes to `sink` in fixed-size chunks, so a
/// multi-hundred-MB section is never held in memory at once.
/// `sink` receives `(offset_within_section, chunk)`.
pub fn stream_section(
    path: &Path,
    dir: &FmDirectory,
    entry: &SectionEntry,
    sink: &mut dyn FnMut(u64, &[u8]),
) -> Result<(), FmError> {
    const CHUNK: usize = 4 << 20;

    let mut dec = section_decoder(path, dir, entry)?;
    let mut buf = vec![0u8; CHUNK];
    let mut abs = 0u64;
    loop {
        let n = dec.read(&mut buf).map_err(|_| FmError::decompress())?;
        if n == 0 {
            break;
        }
        sink(abs, &buf[..n]);
        abs += n as u64;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lp(s: &str) -> Vec<u8> {
        let mut v = (s.len() as u32).to_le_bytes().to_vec();
        v.extend_from_slice(s.as_bytes());
        v
    }

    fn entry(name: &str, off: u64, comp: u64, dec: u64) -> Vec<u8> {
        let mut v = lp(name);
        v.extend(lp(".dat"));
        v.extend_from_slice(&off.to_le_bytes());
        v.extend_from_slice(&comp.to_le_bytes());
        v.extend_from_slice(&dec.to_le_bytes());
        v.extend_from_slice(&[0u8; ENTRY_TRAILER]);
        v
    }

    #[test]
    fn parses_directory_payload() {
        let mut raw = lp("Niclas Krieck – Bayer Leverkusen");
        raw.extend_from_slice(&2u32.to_le_bytes());
        raw.extend(entry("game_info", 0, 2653, 7305));
        raw.extend(entry("game_db", 8058, 83_012_511, 299_187_822));

        let dir = parse_directory_bytes(&raw, 26).expect("directory");
        assert_eq!(dir.save_name, "Niclas Krieck – Bayer Leverkusen");
        assert_eq!(dir.sections.len(), 2);
        let db = dir.section("game_db").expect("game_db");
        assert_eq!(db.compressed_size, 83_012_511);
        assert_eq!(db.decompressed_size, 299_187_822);
        // Section offsets are relative to the outer header.
        assert_eq!(dir.file_offset(db), 8084);
    }

    #[test]
    fn rejects_absurd_entry_count() {
        let mut raw = lp("save");
        raw.extend_from_slice(&999_999u32.to_le_bytes());
        assert!(parse_directory_bytes(&raw, 26).is_none());
    }

    #[test]
    fn header_len_finds_zstd_after_signature() {
        let mut h = vec![0u8; 32];
        h[0] = 0x02;
        h[1] = 0x01;
        h[2..6].copy_from_slice(b"fmf.");
        h[26..30].copy_from_slice(&ZSTD_MAGIC);
        assert_eq!(header_len(&h), Some(26));
    }

    #[test]
    fn header_len_rejects_foreign_file() {
        assert_eq!(header_len(b"PK\x03\x04not-an-fm-save"), None);
    }
}
