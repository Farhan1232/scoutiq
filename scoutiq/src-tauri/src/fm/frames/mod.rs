//! Frame / block index and one-pass processing.

pub mod index;
pub mod processor;
pub mod types;

pub use index::BlockIndex;
pub use processor::{process_block, ProcessOutcome};
pub use types::fnv1a_64;
