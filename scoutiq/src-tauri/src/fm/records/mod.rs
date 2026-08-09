//! Record pattern discovery — generic only; no entity parsers yet.

pub mod detector;
pub mod types;

pub use detector::discover_record_patterns;
pub use types::RecordPatternHint;
