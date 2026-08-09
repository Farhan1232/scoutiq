//! Container signature constants (evidence-based across FM26 desktop saves).

/// Classic uppercase marker (legacy / soft detect).
pub const FMF_SIGNATURE: &[u8] = b"FMF";

/// Observed FM26 desktop outer header: `02 01 66 6D 66 2E` → `\x02\x01fmf.`
pub const FMF_DOT_SIGNATURE: &[u8] = b"fmf.";

/// Inner decompressed container marker: `03 01 74 61 64 2E` → `\x03\x01tad.`
pub const TAD_DOT_SIGNATURE: &[u8] = b"tad.";

/// Bytes before the first Zstd frame in verified FM26 `fmf.` saves.
pub const FMF_DOT_HEADER_LEN: u64 = 26;
