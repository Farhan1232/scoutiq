use crate::fm::types::{StructureCandidate, StructureStats, StructureType};

pub fn structure_stats(items: &[StructureCandidate]) -> StructureStats {
    let mut stats = StructureStats {
        string_tables: 0,
        record_tables: 0,
        index_tables: 0,
        internal_files: 0,
        unknown: 0,
    };
    for s in items {
        match s.structure_type {
            StructureType::StringTable => stats.string_tables += 1,
            StructureType::RecordTable | StructureType::EntityTable => stats.record_tables += 1,
            StructureType::IndexTable | StructureType::ReferenceTable => stats.index_tables += 1,
            StructureType::InternalFile => stats.internal_files += 1,
            StructureType::UnknownRepeatedStructure => stats.unknown += 1,
        }
    }
    stats
}
