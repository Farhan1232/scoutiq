//! Generic FM26 parsing engine orchestrator.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use tauri::{AppHandle, Emitter};

use crate::fm::cache::{build_cache_key, global_cache};
use crate::fm::container::reader::open_file_info;
use crate::fm::container::scanner::index_compression_candidates;
use crate::fm::frames::processor::process_block;
use crate::fm::strings::index::StringIndex;
use crate::fm::structures::types::structure_stats;
use crate::fm::types::{
    BlockStats, EngineProgress, EngineStage, FmEngineResult, PerformanceDiagnostics,
    ValidationStatus, ANALYSIS_CALIBRATING, PARSER_STATUS,
};

#[derive(Clone, Default)]
pub struct AnalyzeOptions<'a> {
    pub use_cache: bool,
    pub emit_progress: bool,
    pub cancel: Option<&'a AtomicBool>,
    pub app: Option<&'a AppHandle>,
}

fn emit_progress(
    opts: &AnalyzeOptions<'_>,
    stage: EngineStage,
    current_block: u32,
    total_blocks: u32,
    bytes_processed: u64,
    total_bytes: u64,
) {
    if !opts.emit_progress {
        return;
    }
    let Some(app) = opts.app else {
        return;
    };
    let payload = EngineProgress {
        stage: stage.as_str().into(),
        progress: stage.progress_pct(),
        current_block,
        total_blocks,
        bytes_processed,
        total_bytes,
        message: stage.message_de().into(),
    };
    let _ = app.emit("scoutiq://fm-progress", payload);
}

fn check_cancel(opts: &AnalyzeOptions<'_>) -> Result<(), String> {
    if opts
        .cancel
        .map(|c| c.load(Ordering::SeqCst))
        .unwrap_or(false)
    {
        Err(crate::fm::errors::FmError::cancelled().into())
    } else {
        Ok(())
    }
}

/// Primary generic analysis entry point.
pub fn analyze_save(path_str: &str, opts: AnalyzeOptions<'_>) -> Result<FmEngineResult, String> {
    let started = Instant::now();

    emit_progress(&opts, EngineStage::OpeningFile, 0, 0, 0, 0);
    check_cancel(&opts)?;

    emit_progress(&opts, EngineStage::ReadingHeader, 0, 0, 0, 0);
    let file = open_file_info(path_str).map_err(|e| e.message)?;
    check_cancel(&opts)?;

    emit_progress(
        &opts,
        EngineStage::DetectingContainer,
        0,
        0,
        0,
        file.file_size,
    );

    if opts.use_cache {
        let key = build_cache_key(
            &file.file_path,
            file.file_size,
            file.modified_unix,
            &file.header_hex,
        );
        if let Some(cached) = global_cache().get(&key) {
            emit_progress(
                &opts,
                EngineStage::Finished,
                cached.block_stats.decompressed,
                cached.blocks.len() as u32,
                cached.performance.bytes_processed,
                file.file_size,
            );
            return Ok(cached);
        }
    }

    emit_progress(&opts, EngineStage::IndexingBlocks, 0, 0, 0, file.file_size);
    let path = std::path::Path::new(path_str);
    let (candidates, bytes_processed) =
        index_compression_candidates(path, opts.cancel).map_err(|e| e.message)?;
    check_cancel(&opts)?;

    let total_blocks = candidates.len() as u32;
    emit_progress(
        &opts,
        EngineStage::ValidatingCompression,
        0,
        total_blocks,
        bytes_processed,
        file.file_size,
    );

    let mut blocks = Vec::new();
    let mut string_index = StringIndex::new();
    let mut structures = Vec::new();
    let mut decompressed_bytes = 0u64;
    let mut errors = 0u32;
    let mut validated = 0u32;
    let mut decompressed = 0u32;

    emit_progress(
        &opts,
        EngineStage::DecompressingBlocks,
        0,
        total_blocks,
        bytes_processed,
        file.file_size,
    );

    for (i, candidate) in candidates.into_iter().enumerate() {
        check_cancel(&opts)?;
        match process_block(path, candidate) {
            Ok(outcome) => {
                match outcome.block.validation_status {
                    ValidationStatus::Processed | ValidationStatus::Valid => {
                        validated += 1;
                        if outcome.decompressed_len > 0 {
                            decompressed += 1;
                        }
                    }
                    ValidationStatus::Error | ValidationStatus::Invalid => {
                        errors += 1;
                    }
                    _ => {}
                }
                decompressed_bytes += outcome.decompressed_len;
                string_index.extend(outcome.strings);
                structures.extend(outcome.structures);
                blocks.push(outcome.block);
            }
            Err(e) => {
                errors += 1;
                // Soft-continue: one bad block must not abort the whole engine.
                let _ = e;
            }
        }
        if i % 4 == 0 || i + 1 == total_blocks as usize {
            emit_progress(
                &opts,
                EngineStage::DecompressingBlocks,
                (i + 1) as u32,
                total_blocks,
                bytes_processed,
                file.file_size,
            );
        }
    }

    emit_progress(
        &opts,
        EngineStage::DiscoveringStructures,
        total_blocks,
        total_blocks,
        bytes_processed,
        file.file_size,
    );
    emit_progress(
        &opts,
        EngineStage::BuildingStringIndex,
        total_blocks,
        total_blocks,
        bytes_processed,
        file.file_size,
    );

    let elapsed_ms = started.elapsed().as_millis() as u64;
    let mb_per_s = if elapsed_ms > 0 {
        (bytes_processed as f64 / 1_048_576.0) / (elapsed_ms as f64 / 1000.0)
    } else {
        0.0
    };

    let structure_stats = structure_stats(&structures);
    let string_entries = string_index.len() as u32;
    let string_index = string_index.into_entries();

    let mut notes = vec![
        PARSER_STATUS.into(),
        "Keine Spiel-Metadaten geraten — nur Container-/Strukturindex.".into(),
        format!(
            "{} Block-Kandidaten, {} entpackt, {} Strings indiziert",
            blocks.len(),
            decompressed,
            string_entries
        ),
    ];
    if blocks.is_empty() {
        notes.push("Keine komprimierten Kandidaten gefunden (oder Limit)".into());
    }

    let result = FmEngineResult {
        file: file.clone(),
        blocks,
        block_stats: BlockStats {
            candidates: total_blocks,
            validated,
            decompressed,
            errors,
        },
        structures,
        structure_stats,
        string_index,
        string_entries,
        performance: PerformanceDiagnostics {
            elapsed_ms,
            bytes_processed,
            decompressed_bytes,
            blocks_processed: total_blocks,
            mb_per_s,
            cache_hit: false,
            errors,
        },
        parser_status: PARSER_STATUS.into(),
        analysis_status: ANALYSIS_CALIBRATING.into(),
        notes,
        from_cache: false,
    };

    if opts.use_cache {
        let key = build_cache_key(
            &file.file_path,
            file.file_size,
            file.modified_unix,
            &file.header_hex,
        );
        global_cache().put(key, result.clone());
    }

    emit_progress(
        &opts,
        EngineStage::Finished,
        total_blocks,
        total_blocks,
        bytes_processed,
        file.file_size,
    );

    Ok(result)
}

/// Compatibility alias used by Tauri `parse_fm_container`.
pub fn parse_container(path_str: &str) -> Result<FmEngineResult, String> {
    analyze_save(
        path_str,
        AnalyzeOptions {
            use_cache: true,
            emit_progress: false,
            cancel: None,
            app: None,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::sync::atomic::AtomicBool;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn cancellation_stops_cleanly() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "scoutiq_cancel_{}.fm",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        {
            let mut f = File::create(&path).unwrap();
            f.write_all(b"FMF").unwrap();
            f.write_all(&vec![0u8; 50_000]).unwrap();
        }
        let cancel = AtomicBool::new(true);
        let err = analyze_save(
            path.to_str().unwrap(),
            AnalyzeOptions {
                use_cache: false,
                emit_progress: false,
                cancel: Some(&cancel),
                app: None,
            },
        )
        .unwrap_err();
        assert!(err.contains("abgebrochen"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn malformed_file_does_not_panic() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "scoutiq_bad_{}.fm",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        {
            let mut f = File::create(&path).unwrap();
            // Fake zstd magic without valid frame.
            f.write_all(&[0x28, 0xB5, 0x2F, 0xFD, 1, 2, 3, 4]).unwrap();
        }
        let r = analyze_save(
            path.to_str().unwrap(),
            AnalyzeOptions {
                use_cache: false,
                ..Default::default()
            },
        )
        .expect("should return soft result");
        assert!(r.block_stats.candidates >= 1);
        let _ = std::fs::remove_file(&path);
    }
}
