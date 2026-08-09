//! Progress events for FM save analysis (emitted to React).

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisStage {
    OpeningFile,
    ReadingHeader,
    ScanningStructure,
    DetectingCompression,
    ExtractingMetadata,
    Finished,
}

impl AnalysisStage {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OpeningFile => "opening_file",
            Self::ReadingHeader => "reading_header",
            Self::ScanningStructure => "scanning_structure",
            Self::DetectingCompression => "detecting_compression",
            Self::ExtractingMetadata => "extracting_metadata",
            Self::Finished => "finished",
        }
    }

    pub fn message_de(&self) -> &'static str {
        match self {
            Self::OpeningFile => "Datei wird geöffnet",
            Self::ReadingHeader => "Header wird gelesen",
            Self::ScanningStructure => "Spielstandstruktur wird untersucht",
            Self::DetectingCompression => "Kompression wird erkannt",
            Self::ExtractingMetadata => "Metadaten werden extrahiert",
            Self::Finished => "Analyse abgeschlossen",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisProgress {
    pub stage: String,
    pub progress: u8,
    pub message: String,
    pub bytes_scanned: u64,
    pub file_size: u64,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisDiagnostics {
    pub file_size: u64,
    pub bytes_scanned: u64,
    pub total_scan_ms: u64,
    pub read_speed_mb_s: f64,
    pub stage: String,
    pub chunk_size: u64,
    pub string_samples: u32,
    pub zstd_frame_count: u64,
}
