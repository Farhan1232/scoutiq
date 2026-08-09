//! Sparse entity registry — resolvable by ID once records are parsed.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::types::{EntityId, EntityRef};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityRegistry {
    pub clubs: HashMap<u64, EntityRef>,
    pub persons: HashMap<u64, EntityRef>,
    pub competitions: HashMap<u64, EntityRef>,
    pub nations: HashMap<u64, EntityRef>,
    pub contracts: HashMap<u64, EntityRef>,
    pub fixtures: HashMap<u64, EntityRef>,
}

impl EntityRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_club(&self, id: u64) -> Option<&EntityRef> {
        self.clubs.get(&id)
    }

    pub fn get_person(&self, id: u64) -> Option<&EntityRef> {
        self.persons.get(&id)
    }

    pub fn get_competition(&self, id: u64) -> Option<&EntityRef> {
        self.competitions.get(&id)
    }

    pub fn get_nation(&self, id: u64) -> Option<&EntityRef> {
        self.nations.get(&id)
    }

    pub fn insert(&mut self, entity: EntityRef) {
        match entity.id {
            EntityId::Club(id) => {
                self.clubs.insert(id, entity);
            }
            EntityId::Person(id) => {
                self.persons.insert(id, entity);
            }
            EntityId::Competition(id) => {
                self.competitions.insert(id, entity);
            }
            EntityId::Nation(id) => {
                self.nations.insert(id, entity);
            }
            EntityId::Contract(id) => {
                self.contracts.insert(id, entity);
            }
            EntityId::Fixture(id) => {
                self.fixtures.insert(id, entity);
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.clubs.is_empty()
            && self.persons.is_empty()
            && self.competitions.is_empty()
            && self.nations.is_empty()
    }
}
