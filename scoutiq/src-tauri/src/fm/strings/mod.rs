//! Generic string extraction and index (one-pass friendly).

pub mod extractor;
pub mod index;

pub use extractor::{extract_strings_one_pass, ExtractConfig};
pub use index::StringIndex;
