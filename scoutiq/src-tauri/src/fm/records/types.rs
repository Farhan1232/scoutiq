use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordPatternHint {
    pub block_index: usize,
    pub offset: u64,
    pub stride: Option<u32>,
    pub evidence: Vec<String>,
}
