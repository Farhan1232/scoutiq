//! Bounded string index metadata (no entity assumptions).

use crate::fm::types::FmStringEntry;

const DEFAULT_CAP: usize = 800;

#[derive(Debug, Clone, Default)]
pub struct StringIndex {
    entries: Vec<FmStringEntry>,
    cap: usize,
}

impl StringIndex {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            cap: DEFAULT_CAP,
        }
    }

    pub fn with_cap(cap: usize) -> Self {
        Self {
            entries: Vec::new(),
            cap,
        }
    }

    pub fn extend(&mut self, items: impl IntoIterator<Item = FmStringEntry>) {
        for item in items {
            if self.entries.len() >= self.cap {
                break;
            }
            self.entries.push(item);
        }
    }

    pub fn entries(&self) -> &[FmStringEntry] {
        &self.entries
    }

    pub fn into_entries(self) -> Vec<FmStringEntry> {
        self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}
