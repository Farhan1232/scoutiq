//! Entity registry, relationships, and managed-club resolution.
//!
//! Deterministic Club/Competition display requires verified record schemas.
//! Until then resolvers return Unknown — never invented or filename-based clubs.

pub mod club_record;
pub mod competition_name;
pub mod competition_table;
pub mod current_league;
pub mod game_version;
pub mod human_manager;
pub mod managed_club;
pub mod name_table;
pub mod registry;
pub mod relationships;
pub mod save_resolver;
pub mod save_summary;
pub mod types;

pub use competition_name::{
    resolve_competition_name, CompetitionDisplayName, CompetitionNameSource,
    StandardCompetitionCatalogue,
};
pub use competition_table::{build_catalogue, CompetitionCatalogue};
pub use current_league::{parse_current_league, CurrentLeague, PackedDate};
pub use game_version::{detect_game_version, GameVersionInfo};
pub use human_manager::{resolve_human_manager_club, HumanManagerClubHit};
pub use managed_club::{
    resolve_managed_club, resolve_managed_club_from_buffers, resolve_managed_club_from_payload,
    ManagedClubResult,
};
pub use name_table::{club_id_for_name, full_name_for_id, scan_name_records, NameRecord};
pub use registry::EntityRegistry;
pub use relationships::{RelationKind, Relationship, RelationshipGraph};
pub use save_resolver::{resolve_save, SaveResolution};
pub use save_summary::{parse_save_summary, SaveSummary};
pub use types::{
    EntityId, EntityRef, Evidence, EvidenceKind, ResolutionConfidence, PARSER_ENGINE_VERSION,
    PARSER_SCHEMA_VERSION,
};
