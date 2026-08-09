//! FMF / container layer — streamed read-only access.

pub mod directory;
pub mod header;
pub mod reader;
pub mod scanner;
pub mod tad;
pub mod types;

pub use directory::{parse_directory, read_section, stream_section, FmDirectory, SectionEntry};
pub use header::{parse_fmf_header, FmfHeaderInfo};
pub use reader::{open_file_info, read_slice, FmStreamReader};
pub use scanner::index_compression_candidates;
pub use tad::{inspect_tad, TadInfo};
pub use types::{FMF_DOT_HEADER_LEN, FMF_DOT_SIGNATURE, FMF_SIGNATURE, TAD_DOT_SIGNATURE};
