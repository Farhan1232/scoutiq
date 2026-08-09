//! Shared FM engine types (generic, save-agnostic).
//! Legacy inspection/metadata types remain for older commands.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Legacy inspection / metadata (kept for sync commands)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MetadataConfidence {
    Confirmed,
    Likely,
    Uncertain,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataField {
    pub value: Option<String>,
    pub confidence: MetadataConfidence,
    pub evidence: Vec<String>,
}

impl MetadataField {
    pub fn unknown() -> Self {
        Self {
            value: None,
            confidence: MetadataConfidence::Unknown,
            evidence: Vec::new(),
        }
    }

    pub fn with_value(
        value: impl Into<String>,
        confidence: MetadataConfidence,
        evidence: Vec<String>,
    ) -> Self {
        Self {
            value: Some(value.into()),
            confidence,
            evidence,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FmSaveMetadataFields {
    pub manager_name: MetadataField,
    pub club_name: MetadataField,
    pub club_id: MetadataField,
    pub in_game_date: MetadataField,
    pub season: MetadataField,
    pub competition_name: MetadataField,
    pub save_name: MetadataField,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataConfidenceMap {
    pub manager_name: MetadataConfidence,
    pub club_name: MetadataConfidence,
    pub club_id: MetadataConfidence,
    pub in_game_date: MetadataConfidence,
    pub season: MetadataConfidence,
    pub competition_name: MetadataConfidence,
    pub save_name: MetadataConfidence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FmSaveMetadata {
    pub manager_name: Option<String>,
    pub club_name: Option<String>,
    pub club_id: Option<String>,
    pub in_game_date: Option<String>,
    pub season: Option<String>,
    pub competition_name: Option<String>,
    pub save_name: Option<String>,
    pub confidence: MetadataConfidenceMap,
    pub fields: FmSaveMetadataFields,
    pub notes: Vec<String>,
    pub errors: Vec<String>,
}

/// High-level compression guess used by the legacy pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DetectedCompression {
    None,
    Zstd,
    Gzip,
    Zip,
    Mixed,
    Unknown,
}

impl DetectedCompression {
    pub fn as_label(&self) -> &'static str {
        match self {
            Self::None => "keine",
            Self::Zstd => "zstd",
            Self::Gzip => "gzip",
            Self::Zip => "zip",
            Self::Mixed => "gemischt",
            Self::Unknown => "unbekannt",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FmSaveInspection {
    pub file_name: String,
    pub file_path: String,
    pub file_size: u64,
    pub first_bytes_hex: String,
    pub detected_compression: String,
    pub zstd_frame_count: u64,
    pub readable_strings_sample: Vec<String>,
    pub errors: Vec<String>,
    pub detected_signatures: Vec<String>,
}

// ---------------------------------------------------------------------------
// Engine enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContainerType {
    Fmf,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CompressionType {
    Zstandard,
    Gzip,
    Zlib,
    Deflate,
    Zip,
    HighEntropy,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ValidationStatus {
    Candidate,
    Valid,
    Invalid,
    Processed,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StringEncoding {
    Ascii,
    Utf8,
    Utf16Le,
    Utf16Be,
}

impl StringEncoding {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ascii => "ascii",
            Self::Utf8 => "utf8",
            Self::Utf16Le => "utf16le",
            Self::Utf16Be => "utf16be",
        }
    }
}

impl From<StringEncoding> for String {
    fn from(value: StringEncoding) -> Self {
        value.as_str().into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StructureType {
    StringTable,
    RecordTable,
    IndexTable,
    ReferenceTable,
    InternalFile,
    EntityTable,
    UnknownRepeatedStructure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EntityType {
    Club,
    Person,
    Player,
    Staff,
    Competition,
    Nation,
    Contract,
    Transfer,
    Finance,
    Fixture,
}

// ---------------------------------------------------------------------------
// File / block / string / structure
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FmFileInfo {
    pub file_name: String,
    pub file_path: String,
    pub file_size: u64,
    pub header_hex: String,
    pub detected_container: Option<ContainerType>,
    pub modified_unix: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressedBlockInfo {
    pub index: usize,
    pub compression_type: CompressionType,
    pub file_offset: u64,
    pub compressed_size: Option<u64>,
    pub decompressed_size: Option<u64>,
    pub validation_status: ValidationStatus,
    pub error: Option<String>,
    pub content_digest: Option<String>,
    pub signatures: Vec<String>,
    pub string_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FmStringEntry {
    pub value: String,
    pub encoding: StringEncoding,
    pub block_index: usize,
    pub decompressed_offset: u64,
    pub source_file_offset: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureCandidate {
    pub block_index: usize,
    pub offset: u64,
    pub structure_type: StructureType,
    pub confidence: Confidence,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NearbyInteger {
    pub offset: u64,
    pub width: u8,
    pub endian: String,
    pub signed: bool,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnownValueMatch {
    pub searched_value: String,
    pub block_index: usize,
    pub source_file_offset: u64,
    pub decompressed_offset: u64,
    pub encoding: StringEncoding,
    pub nearby_strings: Vec<String>,
    pub context_before_hex: String,
    pub context_after_hex: String,
    pub nearby_integers: Vec<NearbyInteger>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnownValueProbeResult {
    pub query: String,
    pub query_type: String,
    pub matches: Vec<KnownValueMatch>,
    pub blocks_searched: u32,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangedBlock {
    pub index_a: Option<usize>,
    pub index_b: Option<usize>,
    pub file_offset_a: Option<u64>,
    pub file_offset_b: Option<u64>,
    pub status: String,
    pub digest_a: Option<String>,
    pub digest_b: Option<String>,
    pub decompressed_size_a: Option<u64>,
    pub decompressed_size_b: Option<u64>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangedRegion {
    pub block_index: Option<usize>,
    pub offset: u64,
    pub length: u64,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FmSaveDiff {
    pub file_a_size: u64,
    pub file_b_size: u64,
    pub blocks_a: usize,
    pub blocks_b: usize,
    pub path_a: String,
    pub path_b: String,
    pub changed_blocks: Vec<ChangedBlock>,
    pub changed_regions: Vec<ChangedRegion>,
    pub unchanged_count: u32,
    pub only_in_a: u32,
    pub only_in_b: u32,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceDiagnostics {
    pub elapsed_ms: u64,
    pub bytes_processed: u64,
    pub decompressed_bytes: u64,
    pub blocks_processed: u32,
    pub mb_per_s: f64,
    pub cache_hit: bool,
    pub errors: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockStats {
    pub candidates: u32,
    pub validated: u32,
    pub decompressed: u32,
    pub errors: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureStats {
    pub string_tables: u32,
    pub record_tables: u32,
    pub index_tables: u32,
    pub internal_files: u32,
    pub unknown: u32,
}

/// Primary engine analysis result (developer / RE). No invented game metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FmEngineResult {
    pub file: FmFileInfo,
    pub blocks: Vec<CompressedBlockInfo>,
    pub block_stats: BlockStats,
    pub structures: Vec<StructureCandidate>,
    pub structure_stats: StructureStats,
    pub string_index: Vec<FmStringEntry>,
    pub string_entries: u32,
    pub performance: PerformanceDiagnostics,
    pub parser_status: String,
    pub analysis_status: String,
    pub notes: Vec<String>,
    pub from_cache: bool,
}

pub const PARSER_STATUS: &str = "Reverse Engineering / Developer Build";
pub const ANALYSIS_CALIBRATING: &str = "calibrating";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParserMetadata {
    pub engine_version: String,
    pub game_version: Option<String>,
    pub schema_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityConfidenceMap {
    pub manager: String,
    pub club: String,
    pub competition: String,
    pub season: String,
    pub date: String,
    pub overall: String,
}

/// User-facing save identity (no invented fields).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FmSaveAnalysisResult {
    pub file_info: FmFileInfo,
    pub parser_info: ParserMetadata,
    pub game_version: crate::fm::entities::GameVersionInfo,
    pub managed: crate::fm::entities::ManagedClubResult,
    pub warnings: Vec<String>,
    pub confidence: IdentityConfidenceMap,
}

/// Progress payload for Tauri events.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineProgress {
    pub stage: String,
    pub progress: u8,
    pub current_block: u32,
    pub total_blocks: u32,
    pub bytes_processed: u64,
    pub total_bytes: u64,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineStage {
    OpeningFile,
    ReadingHeader,
    DetectingContainer,
    IndexingBlocks,
    ValidatingCompression,
    DecompressingBlocks,
    DiscoveringStructures,
    BuildingStringIndex,
    Finished,
}

impl EngineStage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpeningFile => "opening_file",
            Self::ReadingHeader => "reading_header",
            Self::DetectingContainer => "detecting_container",
            Self::IndexingBlocks => "indexing_blocks",
            Self::ValidatingCompression => "validating_compression",
            Self::DecompressingBlocks => "decompressing_blocks",
            Self::DiscoveringStructures => "discovering_structures",
            Self::BuildingStringIndex => "building_string_index",
            Self::Finished => "finished",
        }
    }

    pub fn message_de(self) -> &'static str {
        match self {
            Self::OpeningFile => "Spielstand wird geöffnet",
            Self::ReadingHeader | Self::DetectingContainer => "Container wird analysiert",
            Self::IndexingBlocks | Self::ValidatingCompression => {
                "Komprimierte Datenblöcke werden indexiert"
            }
            Self::DecompressingBlocks => "Datenblöcke werden entpackt",
            Self::DiscoveringStructures | Self::BuildingStringIndex => {
                "Interne Strukturen werden gesucht"
            }
            Self::Finished => "Analyse abgeschlossen",
        }
    }

    pub fn progress_pct(self) -> u8 {
        match self {
            Self::OpeningFile => 5,
            Self::ReadingHeader => 10,
            Self::DetectingContainer => 15,
            Self::IndexingBlocks => 30,
            Self::ValidatingCompression => 40,
            Self::DecompressingBlocks => 70,
            Self::DiscoveringStructures => 85,
            Self::BuildingStringIndex => 95,
            Self::Finished => 100,
        }
    }
}
