//! Shared entity-resolution types.

use serde::{Deserialize, Serialize};

/// Bump when cached identity results become incompatible.
pub const PARSER_ENGINE_VERSION: &str = "0.4.0";
pub const PARSER_SCHEMA_VERSION: u32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResolutionConfidence {
    Confirmed,
    High,
    Medium,
    Low,
    Unknown,
}

impl ResolutionConfidence {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Confirmed => "confirmed",
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
            Self::Unknown => "unknown",
        }
    }

    /// Normal UI may present Confirmed/High as factual.
    pub fn is_user_facing(self) -> bool {
        matches!(self, Self::Confirmed | Self::High)
    }

    /// Medium may show with an explicit "possibly detected" label.
    pub fn is_possible(self) -> bool {
        matches!(self, Self::Medium)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EvidenceKind {
    StructuralRecord,
    RelationshipLink,
    HeaderField,
    IndexReference,
    HeuristicString,
    Absent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Evidence {
    pub kind: EvidenceKind,
    pub detail: String,
    pub block_index: Option<usize>,
    pub offset: Option<u64>,
}

impl Evidence {
    pub fn structural(detail: impl Into<String>) -> Self {
        Self {
            kind: EvidenceKind::StructuralRecord,
            detail: detail.into(),
            block_index: None,
            offset: None,
        }
    }

    pub fn absent(detail: impl Into<String>) -> Self {
        Self {
            kind: EvidenceKind::Absent,
            detail: detail.into(),
            block_index: None,
            offset: None,
        }
    }

    pub fn header(detail: impl Into<String>) -> Self {
        Self {
            kind: EvidenceKind::HeaderField,
            detail: detail.into(),
            block_index: None,
            offset: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EntityId {
    Club(u64),
    Person(u64),
    Competition(u64),
    Nation(u64),
    Contract(u64),
    Fixture(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityRef {
    pub id: EntityId,
    pub display_name: Option<String>,
    pub short_name: Option<String>,
    pub confidence: ResolutionConfidence,
}
