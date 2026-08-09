//! Combined FM save analysis — compatibility helpers.

use super::pipeline::FmSaveAnalysis;
use super::reader::{read_save_sample, INSPECT_SAMPLE_BYTES};

/// Sync helper used by legacy commands (small sample, no progress events).
pub fn analyze_fm_save_path(path_str: &str) -> Result<FmSaveAnalysis, String> {
    let sample = read_save_sample(path_str, INSPECT_SAMPLE_BYTES)?;
    let inspection = super::inspector::inspect_from_sample(&sample);
    let metadata = super::heuristics::extract_fm_metadata_from_sample(&sample);
    Ok(FmSaveAnalysis {
        inspection,
        metadata,
        diagnostics: super::progress::AnalysisDiagnostics {
            file_size: sample.file_size,
            bytes_scanned: sample.bytes.len() as u64,
            total_scan_ms: 0,
            read_speed_mb_s: 0.0,
            stage: "finished".into(),
            chunk_size: 0,
            string_samples: 0,
            zstd_frame_count: 0,
        },
        forensic: super::forensic::empty_forensic_report(),
    })
}
