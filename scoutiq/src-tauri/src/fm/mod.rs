//! Football Manager save inspection (read-only).
//!
//! Generic FM26 engine pipeline:
//! Save → Container → Compression → Block Index → Structures → (future entities)
//!
//! Legacy heuristic modules remain for older commands but are not the primary path.

pub mod analyze;
pub mod cache;
pub mod compression;
pub mod container;
pub mod debug;
pub mod decompress;
pub mod entities;
pub mod errors;
pub mod forensic;
pub mod frames;
pub mod heuristics;
pub mod identity;
pub mod inspector;
pub mod metadata;
pub mod parser;
pub mod pipeline;
pub mod progress;
pub mod reader;
pub mod records;
pub mod strings;
pub mod structures;
pub mod types;

// Compatibility shim for older imports of container_types.
pub mod container_types {
    pub use crate::fm::types::*;

    /// Legacy alias used by older frontend code.
    pub type ContainerParseResult = crate::fm::types::FmEngineResult;
    pub type SaveDiffResult = crate::fm::types::FmSaveDiff;
    pub type DiagnosticExport = serde_json::Value;
    pub type StringEntry = crate::fm::types::FmStringEntry;
    pub type FrameInfo = crate::fm::types::CompressedBlockInfo;
    pub type FrameDiff = crate::fm::types::ChangedBlock;

    #[derive(Debug, Clone, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CalibratingMetadata {
        pub manager_name: Option<String>,
        pub club_name: Option<String>,
        pub club_id: Option<String>,
        pub in_game_date: Option<String>,
        pub season: Option<String>,
        pub competition_name: Option<String>,
        pub save_name: Option<String>,
        pub status: String,
        pub message: String,
    }

    impl CalibratingMetadata {
        pub fn new(save_name: Option<String>) -> Self {
            Self {
                manager_name: None,
                club_name: None,
                club_id: None,
                in_game_date: None,
                season: None,
                competition_name: None,
                save_name,
                status: "calibrating".into(),
                message: "Parser wird kalibriert".into(),
            }
        }
    }
}

pub use analyze::analyze_fm_save_path;
pub use container_types::CalibratingMetadata;
pub use debug::{compare_fm_saves, export_diagnostic_json, probe_known_value};
pub use entities::{ManagedClubResult, PARSER_ENGINE_VERSION, PARSER_SCHEMA_VERSION};
pub use heuristics::extract_fm_metadata_path;
pub use identity::{
    analyze_identity_from_engine, analyze_identity_from_engine_with_progress,
    analyze_save_identity, analyze_save_identity_with_progress, IdentityProgress,
    IDENTITY_PROGRESS_EVENT,
};
pub use inspector::inspect_fm_save_path;
pub use metadata::calibrating_panel;
pub use parser::{analyze_save, parse_container, AnalyzeOptions};
pub use pipeline::{
    emit_done, run_fm_analysis, validate_fm_save_path, FmQuickValidation, FmSaveAnalysis,
};
pub use types::{
    FmEngineResult, FmSaveAnalysisResult, FmSaveDiff, FmSaveInspection, FmSaveMetadata,
    KnownValueProbeResult, PARSER_STATUS,
};

/// Legacy names kept for lib.rs.
pub use compare_fm_saves as compare_saves;
pub use export_diagnostic_json as export_diagnostic;
