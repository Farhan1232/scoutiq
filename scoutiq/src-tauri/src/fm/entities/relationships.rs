//! Lightweight relationship graph for deterministic resolution.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RelationKind {
    Manages,
    PlaysIn,
    LocatedIn,
    ContractedTo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Relationship {
    pub kind: RelationKind,
    pub from_id: u64,
    pub to_id: u64,
    pub note: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipGraph {
    pub edges: Vec<Relationship>,
}

impl RelationshipGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, rel: Relationship) {
        self.edges.push(rel);
    }

    pub fn managed_club_id(&self, manager_id: u64) -> Option<u64> {
        self.edges.iter().find_map(|e| {
            if e.kind == RelationKind::Manages && e.from_id == manager_id {
                Some(e.to_id)
            } else {
                None
            }
        })
    }

    pub fn competition_for_club(&self, club_id: u64) -> Option<u64> {
        self.edges.iter().find_map(|e| {
            if e.kind == RelationKind::PlaysIn && e.from_id == club_id {
                Some(e.to_id)
            } else {
                None
            }
        })
    }

    pub fn nation_for_club(&self, club_id: u64) -> Option<u64> {
        self.edges.iter().find_map(|e| {
            if e.kind == RelationKind::LocatedIn && e.from_id == club_id {
                Some(e.to_id)
            } else {
                None
            }
        })
    }
}
