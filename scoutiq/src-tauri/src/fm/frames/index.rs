//! Reusable block index metadata (no decompressed payloads).

use crate::fm::types::CompressedBlockInfo;

#[derive(Debug, Clone, Default)]
pub struct BlockIndex {
    pub blocks: Vec<CompressedBlockInfo>,
}

impl BlockIndex {
    pub fn new(blocks: Vec<CompressedBlockInfo>) -> Self {
        Self { blocks }
    }

    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }
}
