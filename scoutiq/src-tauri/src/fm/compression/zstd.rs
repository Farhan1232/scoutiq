//! Bounded Zstandard decode — never shells out.

use std::io::{Cursor, Read};

pub const ZSTD_MAGIC: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];
pub const MAX_FRAME_OUT: usize = 4 * 1024 * 1024;
pub const MAX_FRAME_IN: usize = 8 * 1024 * 1024;

#[derive(Debug)]
pub struct ZstdDecodeOutcome {
    pub ok: bool,
    pub decompressed: Vec<u8>,
    pub compressed_consumed: u64,
    pub error: Option<String>,
}

/// Decompress one Zstd frame starting at `src[0]` (must begin with magic).
/// Boundaries come from the decoder (bytes consumed), not the next magic.
pub fn decompress_zstd_frame(src: &[u8]) -> ZstdDecodeOutcome {
    decompress_zstd_frame_bounded(src, MAX_FRAME_OUT)
}

/// Decompress one Zstd frame with a custom output bound.
pub fn decompress_zstd_frame_bounded(src: &[u8], max_out: usize) -> ZstdDecodeOutcome {
    if src.len() < 4 || src[..4] != ZSTD_MAGIC {
        return ZstdDecodeOutcome {
            ok: false,
            decompressed: Vec::new(),
            compressed_consumed: 0,
            error: Some("Kein Zstd-Magic am Offset".into()),
        };
    }

    let feed = &src[..src.len().min(MAX_FRAME_IN)];
    let mut cursor = Cursor::new(feed);
    let start = cursor.position();

    let mut decoder = match zstd::stream::read::Decoder::new(&mut cursor) {
        Ok(d) => d,
        Err(e) => {
            return ZstdDecodeOutcome {
                ok: false,
                decompressed: Vec::new(),
                compressed_consumed: 0,
                error: Some(format!("Decoder-Init: {e}")),
            };
        }
    };

    let mut out = Vec::new();
    let mut buf = [0u8; 64 * 1024];
    let mut decode_err: Option<String> = None;

    loop {
        if out.len() >= max_out {
            break;
        }
        let want = buf.len().min(max_out - out.len());
        match decoder.read(&mut buf[..want]) {
            Ok(0) => break,
            Ok(n) => out.extend_from_slice(&buf[..n]),
            Err(e) => {
                decode_err = Some(format!("Decode: {e}"));
                break;
            }
        }
    }

    let compressed_consumed = cursor.position().saturating_sub(start);

    if out.is_empty() {
        ZstdDecodeOutcome {
            ok: false,
            decompressed: out,
            compressed_consumed,
            error: decode_err
                .or_else(|| Some("Komprimierter Datenblock konnte nicht entpackt werden.".into())),
        }
    } else {
        ZstdDecodeOutcome {
            ok: true,
            decompressed: out,
            compressed_consumed,
            error: decode_err,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_false_magic() {
        let r = decompress_zstd_frame(&[0, 1, 2, 3, 4, 5]);
        assert!(!r.ok);
    }

    #[test]
    fn decompresses_simple_frame() {
        let payload = b"ScoutIQ FM26 generic parser test payload";
        let compressed = zstd::encode_all(&payload[..], 1).expect("encode");
        assert!(compressed.starts_with(&ZSTD_MAGIC));
        let r = decompress_zstd_frame(&compressed);
        assert!(r.ok, "{:?}", r.error);
        assert_eq!(r.decompressed, payload);
        assert!(r.compressed_consumed > 0);
    }
}
