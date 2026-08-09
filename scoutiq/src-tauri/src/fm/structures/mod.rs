//! Generic structure discovery (no entity conversion from low confidence).

pub mod detector;
pub mod types;

pub use detector::detect_structures;
pub use types::structure_stats;
