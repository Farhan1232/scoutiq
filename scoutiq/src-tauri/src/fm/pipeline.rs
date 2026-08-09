//! Staged, cancellable, one-pass FM save analysis (read-only).

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use super::decompress::{
    decompress_frames_bounded, find_zstd_offsets, DECOMPRESS_WINDOW, MAX_FRAMES,
};
use super::errors::{looks_like_cloud_path, map_io_error, FmError};
use super::forensic::{build_forensic_report, ForensicReport};
use super::heuristics::extract_fm_metadata_from_scan;
use super::progress::{AnalysisDiagnostics, AnalysisProgress, AnalysisStage};
use super::reader::{
    bytes_to_hex, file_name_from_path, is_supported_extension, LocatedString, HEADER_BYTES,
};
use super::types::{DetectedCompression, FmSaveInspection, FmSaveMetadata};

const CHUNK_SIZE: usize = 64 * 1024;
/// Retain only a small prefix for club-id proximity heuristics (not the whole file).
const PREFIX_KEEP: usize = 256 * 1024;
/// Metadata / structure heuristics live near the start — never read multi-GB saves end-to-end.
const SCAN_WINDOW_BYTES: u64 = 8 * 1024 * 1024;
const MAX_STRING_SAMPLES: usize = 48;
const MAX_STRING_LEN: usize = 120;
const MIN_STRING_LEN: usize = 4;
const MAGIC_OVERLAP: usize = 3;
/// Emit progress at least this often so the UI never looks frozen.
const PROGRESS_EMIT_MS: u64 = 100;
/// If zero bytes arrive within this window on a cloud path → clear local-unavailable error.
const CLOUD_STALL_MS: u64 = 8_000;
/// Absolute stall (any path) with zero bytes read — fail instead of hanging forever.
const READ_STALL_MS: u64 = 20_000;
const PROGRESS_EVENT: &str = "scoutiq://fm-progress";
const DONE_EVENT: &str = "scoutiq://fm-analysis-done";

const ZSTD_MAGIC: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];
const GZIP_MAGIC: [u8; 2] = [0x1F, 0x8B];
const ZIP_LOCAL: [u8; 4] = [0x50, 0x4B, 0x03, 0x04];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FmQuickValidation {
    pub file_name: String,
    pub file_path: String,
    pub file_size: u64,
    pub first_bytes_hex: String,
    pub supported_extension: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FmSaveAnalysis {
    pub inspection: FmSaveInspection,
    pub metadata: FmSaveMetadata,
    pub diagnostics: AnalysisDiagnostics,
    pub forensic: ForensicReport,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisDonePayload {
    pub ok: bool,
    pub analysis: Option<FmSaveAnalysis>,
    pub error: Option<String>,
}

/// Stage 1 — quick validation (openable, size, first 512 bytes).
/// Opens first; avoids `exists()` / standalone `metadata()` which can hang on iCloud.
pub fn validate_fm_save_path(path_str: &str) -> Result<FmQuickValidation, String> {
    let path = Path::new(path_str);
    if !is_supported_extension(path) {
        return Err(FmError::unsupported().into());
    }

    let mut file = File::open(path).map_err(|e| map_io_error(&e, path).message)?;
    let file_size = file.metadata().map(|m| m.len()).unwrap_or(0);
    let mut header = vec![0u8; HEADER_BYTES];
    let n = file
        .read(&mut header)
        .map_err(|e| map_io_error(&e, path).message)?;
    header.truncate(n);

    Ok(FmQuickValidation {
        file_name: file_name_from_path(path),
        file_path: path_str.to_string(),
        file_size,
        first_bytes_hex: bytes_to_hex(&header),
        supported_extension: true,
    })
}

/// Staged analysis with progress events. Checks `cancel` between chunks.
/// Scans at most [`SCAN_WINDOW_BYTES`] — enough for header, compression magic, and metadata strings.
pub fn run_fm_analysis(
    app: &AppHandle,
    path_str: &str,
    cancel: &Arc<AtomicBool>,
) -> Result<FmSaveAnalysis, String> {
    let started = Instant::now();
    let path = Path::new(path_str);
    let cloud_path = looks_like_cloud_path(path);

    emit_progress(
        app,
        AnalysisStage::OpeningFile,
        2,
        0,
        0,
        started.elapsed().as_millis() as u64,
    );
    check_cancel(cancel)?;

    if !is_supported_extension(path) {
        return Err(FmError::unsupported().into());
    }

    if cloud_path {
        emit_progress_msg(
            app,
            &AnalysisStage::OpeningFile,
            3,
            0,
            0,
            started.elapsed().as_millis() as u64,
            "Cloud-Datei — warte auf lokalen Inhalt…",
        );
    }

    // All blocking I/O runs on a helper thread; this thread only polls + cancels.
    let mut stream = FileStream::start(path_str)?;
    let file_size = wait_for_open(app, path, cancel, &started, cloud_path, &mut stream)?;
    let scan_limit = if file_size == 0 {
        SCAN_WINDOW_BYTES
    } else {
        file_size.min(SCAN_WINDOW_BYTES)
    };

    emit_progress(
        app,
        AnalysisStage::ReadingHeader,
        8,
        0,
        file_size,
        started.elapsed().as_millis() as u64,
    );
    check_cancel(cancel)?;

    let header = read_prefix_cancellable(
        app,
        path,
        cancel,
        &started,
        cloud_path,
        file_size,
        &mut stream,
        HEADER_BYTES,
        &AnalysisStage::ReadingHeader,
    )?;
    if header.is_empty() && file_size > 0 {
        return Err(FmError::corrupted().into());
    }

    emit_progress(
        app,
        AnalysisStage::ScanningStructure,
        12,
        header.len() as u64,
        file_size,
        started.elapsed().as_millis() as u64,
    );

    let mut scan = OnePassScan::new();
    scan.ingest(&header, 0);

    let mut bytes_read = header.len() as u64;
    let mut last_emit_pct = 12u8;
    let mut last_emit_at = started.elapsed().as_millis() as u64;

    while bytes_read < scan_limit {
        check_cancel(cancel)?;
        let want = ((scan_limit - bytes_read) as usize).min(CHUNK_SIZE);
        let chunk = match recv_chunk(
            app,
            path,
            cancel,
            &started,
            cloud_path,
            file_size,
            bytes_read,
            &mut stream,
            want,
            &AnalysisStage::ScanningStructure,
        )? {
            Some(c) => c,
            None => break,
        };
        let offset_base = bytes_read as usize;
        scan.ingest(&chunk, offset_base);
        bytes_read += chunk.len() as u64;

        let pct = progress_pct(bytes_read, scan_limit, 12, 82);
        let elapsed = started.elapsed().as_millis() as u64;
        let due_pct = pct >= last_emit_pct.saturating_add(1);
        let due_time = elapsed.saturating_sub(last_emit_at) >= PROGRESS_EMIT_MS;
        if due_pct || due_time || bytes_read >= scan_limit {
            last_emit_pct = pct;
            last_emit_at = elapsed;
            let stage = if pct < 40 {
                AnalysisStage::ScanningStructure
            } else if pct < 55 {
                AnalysisStage::DetectingCompression
            } else {
                AnalysisStage::ScanningStructure
            };
            emit_progress(app, stage, pct, bytes_read, file_size, elapsed);
        }
    }

    stream.stop();
    scan.finish();
    check_cancel(cancel)?;

    emit_progress_msg(
        app,
        &AnalysisStage::DetectingCompression,
        84,
        bytes_read,
        file_size,
        started.elapsed().as_millis() as u64,
        "Komprimierte Daten werden gelesen…",
    );
    check_cancel(cancel)?;

    // Bounded Zstd decompress from the kept scan window (never full save).
    let mut zstd_offsets = scan.zstd_offsets.clone();
    if zstd_offsets.is_empty() {
        zstd_offsets = find_zstd_offsets(&scan.decompress_window, MAX_FRAMES);
    }
    let decompressed = decompress_frames_bounded(&scan.decompress_window, &zstd_offsets);
    check_cancel(cancel)?;

    let forensic = build_forensic_report(
        &scan.decompress_window,
        file_size,
        bytes_read,
        &zstd_offsets,
    );
    check_cancel(cancel)?;

    emit_progress(
        app,
        AnalysisStage::DetectingCompression,
        86,
        bytes_read,
        file_size,
        started.elapsed().as_millis() as u64,
    );

    let compression = classify_compression(&header, &scan);
    let mut signatures = detect_header_signatures(&header);
    if scan.zstd_count > 0 && !signatures.iter().any(|s| s.contains("zstd")) {
        signatures.push(format!("zstd-frame×{}", scan.zstd_count));
    }
    if scan.gzip_count > 0 && !signatures.iter().any(|s| s.starts_with("gzip")) {
        signatures.push(format!("gzip×{}", scan.gzip_count));
    }
    if scan.zip_count > 0 && !signatures.iter().any(|s| s.starts_with("zip")) {
        signatures.push(format!("zip×{}", scan.zip_count));
    }
    if decompressed.frames_ok > 0 {
        signatures.push(format!(
            "zstd-decoded×{} ({} B)",
            decompressed.frames_ok, decompressed.bytes_out
        ));
    }

    emit_progress(
        app,
        AnalysisStage::ExtractingMetadata,
        90,
        bytes_read,
        file_size,
        started.elapsed().as_millis() as u64,
    );
    check_cancel(cancel)?;

    // Merge raw + decompressed strings for heuristics.
    let mut strings = scan.strings.clone();
    strings.extend(decompressed.strings.clone());
    strings.sort_by_key(|s| s.offset);
    strings.dedup_by(|a, b| a.value == b.value);

    let file_name = file_name_from_path(path);
    let mut metadata = extract_fm_metadata_from_scan(
        path_str,
        &file_name,
        file_size,
        &strings,
        &scan.prefix,
        scan.zstd_count,
        bytes_read,
    );
    metadata.notes.extend(decompressed.notes);
    metadata.notes.extend(forensic.log.summary_lines.clone());
    if file_size > bytes_read {
        metadata.notes.push(format!(
            "Scan auf erste {bytes_read} Bytes begrenzt (Datei {file_size} Bytes)"
        ));
    }

    let mut string_sample: Vec<String> = scan
        .strings
        .iter()
        .take(MAX_STRING_SAMPLES / 2)
        .map(|s| s.value.clone())
        .collect();
    for s in decompressed.strings.iter().take(MAX_STRING_SAMPLES / 2) {
        if !string_sample.iter().any(|x| x == &s.value) {
            string_sample.push(s.value.clone());
        }
    }

    let inspection = FmSaveInspection {
        file_name: file_name.clone(),
        file_path: path_str.to_string(),
        file_size,
        first_bytes_hex: bytes_to_hex(&header),
        detected_compression: compression.as_label().to_string(),
        zstd_frame_count: scan.zstd_count,
        readable_strings_sample: string_sample,
        errors: Vec::new(),
        detected_signatures: signatures,
    };

    let total_ms = started.elapsed().as_millis() as u64;
    let seconds = (total_ms as f64 / 1000.0).max(0.001);
    let read_speed = (bytes_read as f64 / (1024.0 * 1024.0)) / seconds;

    let diagnostics = AnalysisDiagnostics {
        file_size,
        bytes_scanned: bytes_read,
        total_scan_ms: total_ms,
        read_speed_mb_s: read_speed,
        stage: AnalysisStage::Finished.as_str().to_string(),
        chunk_size: CHUNK_SIZE as u64,
        string_samples: strings.len() as u32,
        zstd_frame_count: scan.zstd_count,
    };

    emit_progress(
        app,
        AnalysisStage::Finished,
        100,
        bytes_read,
        file_size,
        total_ms,
    );

    Ok(FmSaveAnalysis {
        inspection,
        metadata,
        diagnostics,
        forensic,
    })
}

pub fn emit_done(app: &AppHandle, result: Result<FmSaveAnalysis, String>) {
    let payload = match result {
        Ok(analysis) => AnalysisDonePayload {
            ok: true,
            analysis: Some(analysis),
            error: None,
        },
        Err(error) => AnalysisDonePayload {
            ok: false,
            analysis: None,
            error: Some(error),
        },
    };
    let _ = app.emit(DONE_EVENT, payload);
}

fn emit_progress(
    app: &AppHandle,
    stage: AnalysisStage,
    progress: u8,
    bytes_scanned: u64,
    file_size: u64,
    elapsed_ms: u64,
) {
    emit_progress_msg(
        app,
        &stage,
        progress,
        bytes_scanned,
        file_size,
        elapsed_ms,
        stage.message_de(),
    );
}

fn emit_progress_msg(
    app: &AppHandle,
    stage: &AnalysisStage,
    progress: u8,
    bytes_scanned: u64,
    file_size: u64,
    elapsed_ms: u64,
    message: &str,
) {
    let _ = app.emit(
        PROGRESS_EVENT,
        AnalysisProgress {
            stage: stage.as_str().to_string(),
            progress: progress.min(100),
            message: message.to_string(),
            bytes_scanned,
            file_size,
            elapsed_ms,
        },
    );
}

fn check_cancel(cancel: &Arc<AtomicBool>) -> Result<(), String> {
    if cancel.load(Ordering::SeqCst) {
        Err(FmError::cancelled().into())
    } else {
        Ok(())
    }
}

enum IoEvent {
    Opened { file_size: u64 },
    Chunk(Vec<u8>),
    Eof,
    Err(String),
}

enum IoCmd {
    Read { max: usize },
    Stop,
}

/// Owns the File on a dedicated thread so blocking cloud I/O never freezes cancel/progress.
struct FileStream {
    cmd_tx: Option<mpsc::Sender<IoCmd>>,
    event_rx: mpsc::Receiver<IoEvent>,
}

impl FileStream {
    fn start(path_str: &str) -> Result<Self, String> {
        let (cmd_tx, cmd_rx) = mpsc::channel::<IoCmd>();
        let (event_tx, event_rx) = mpsc::channel::<IoEvent>();
        let open_path = path_str.to_string();

        std::thread::Builder::new()
            .name("fm-io".into())
            .spawn(move || {
                let file = match File::open(&open_path) {
                    Ok(f) => f,
                    Err(err) => {
                        let path = Path::new(&open_path);
                        let _ = event_tx.send(IoEvent::Err(map_io_error(&err, path).message));
                        return;
                    }
                };
                let file_size = file.metadata().map(|m| m.len()).unwrap_or(0);
                if event_tx.send(IoEvent::Opened { file_size }).is_err() {
                    return;
                }

                let mut reader = BufReader::with_capacity(CHUNK_SIZE, file);
                while let Ok(cmd) = cmd_rx.recv() {
                    match cmd {
                        IoCmd::Stop => break,
                        IoCmd::Read { max } => {
                            let mut buf = vec![0u8; max];
                            match reader.read(&mut buf) {
                                Ok(0) => {
                                    let _ = event_tx.send(IoEvent::Eof);
                                    break;
                                }
                                Ok(n) => {
                                    buf.truncate(n);
                                    if event_tx.send(IoEvent::Chunk(buf)).is_err() {
                                        break;
                                    }
                                }
                                Err(err) => {
                                    let path = Path::new(&open_path);
                                    let _ = event_tx
                                        .send(IoEvent::Err(map_io_error(&err, path).message));
                                    break;
                                }
                            }
                        }
                    }
                }
            })
            .map_err(|_| String::from(FmError::unknown()))?;

        Ok(Self {
            cmd_tx: Some(cmd_tx),
            event_rx,
        })
    }

    fn request_read(&self, max: usize) -> Result<(), String> {
        self.cmd_tx
            .as_ref()
            .ok_or_else(|| String::from(FmError::unknown()))?
            .send(IoCmd::Read { max })
            .map_err(|_| String::from(FmError::open_failed()))
    }

    fn stop(&mut self) {
        if let Some(tx) = self.cmd_tx.take() {
            let _ = tx.send(IoCmd::Stop);
        }
    }
}

fn wait_for_open(
    app: &AppHandle,
    path: &Path,
    cancel: &Arc<AtomicBool>,
    started: &Instant,
    cloud_path: bool,
    stream: &mut FileStream,
) -> Result<u64, String> {
    loop {
        check_cancel(cancel)?;
        match stream.event_rx.recv_timeout(Duration::from_millis(150)) {
            Ok(IoEvent::Opened { file_size }) => return Ok(file_size),
            Ok(IoEvent::Err(msg)) => return Err(msg),
            Ok(IoEvent::Chunk(_)) | Ok(IoEvent::Eof) => {
                return Err(FmError::unknown().into());
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let elapsed = started.elapsed().as_millis() as u64;
                if cloud_path && elapsed >= CLOUD_STALL_MS {
                    return Err(FmError::cloud_unavailable(path).into());
                }
                if elapsed >= READ_STALL_MS {
                    return if cloud_path {
                        Err(FmError::cloud_unavailable(path).into())
                    } else {
                        Err(FmError::open_failed().into())
                    };
                }
                let msg = if cloud_path {
                    "Cloud-Datei — warte auf lokalen Inhalt…"
                } else {
                    AnalysisStage::OpeningFile.message_de()
                };
                emit_progress_msg(app, &AnalysisStage::OpeningFile, 3, 0, 0, elapsed, msg);
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(FmError::open_failed().into());
            }
        }
    }
}

fn read_prefix_cancellable(
    app: &AppHandle,
    path: &Path,
    cancel: &Arc<AtomicBool>,
    started: &Instant,
    cloud_path: bool,
    file_size: u64,
    stream: &mut FileStream,
    want: usize,
    stage: &AnalysisStage,
) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(want);
    while out.len() < want {
        let need = want - out.len();
        match recv_chunk(
            app,
            path,
            cancel,
            started,
            cloud_path,
            file_size,
            out.len() as u64,
            stream,
            need,
            stage,
        )? {
            Some(chunk) => out.extend_from_slice(&chunk),
            None => break,
        }
    }
    Ok(out)
}

fn recv_chunk(
    app: &AppHandle,
    path: &Path,
    cancel: &Arc<AtomicBool>,
    started: &Instant,
    cloud_path: bool,
    file_size: u64,
    bytes_so_far: u64,
    stream: &mut FileStream,
    want: usize,
    stage: &AnalysisStage,
) -> Result<Option<Vec<u8>>, String> {
    stream.request_read(want)?;
    let wait_started = Instant::now();
    loop {
        check_cancel(cancel)?;
        match stream.event_rx.recv_timeout(Duration::from_millis(150)) {
            Ok(IoEvent::Chunk(data)) => return Ok(Some(data)),
            Ok(IoEvent::Eof) => return Ok(None),
            Ok(IoEvent::Err(msg)) => return Err(msg),
            Ok(IoEvent::Opened { .. }) => continue,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let stall_ms = wait_started.elapsed().as_millis() as u64;
                let total_ms = started.elapsed().as_millis() as u64;
                if bytes_so_far == 0 && cloud_path && stall_ms >= CLOUD_STALL_MS {
                    return Err(FmError::cloud_unavailable(path).into());
                }
                if bytes_so_far == 0 && stall_ms >= READ_STALL_MS {
                    return if cloud_path {
                        Err(FmError::cloud_unavailable(path).into())
                    } else {
                        Err(FmError::open_failed().into())
                    };
                }
                let msg = if cloud_path && bytes_so_far == 0 {
                    "Cloud-Datei — Inhalt wird geladen (OneDrive/iCloud)…"
                } else {
                    stage.message_de()
                };
                emit_progress_msg(
                    app,
                    stage,
                    if bytes_so_far == 0 { 8 } else { 12 },
                    bytes_so_far,
                    file_size,
                    total_ms,
                    msg,
                );
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(FmError::open_failed().into());
            }
        }
    }
}

fn progress_pct(bytes_read: u64, file_size: u64, start: u8, end: u8) -> u8 {
    if file_size == 0 {
        return end;
    }
    let span = (end - start) as f64;
    let ratio = (bytes_read as f64 / file_size as f64).clamp(0.0, 1.0);
    start + (span * ratio) as u8
}

struct OnePassScan {
    prefix: Vec<u8>,
    /// Compressed source window for bounded Zstd decode (≤ DECOMPRESS_WINDOW).
    decompress_window: Vec<u8>,
    zstd_count: u64,
    zstd_offsets: Vec<usize>,
    gzip_count: u64,
    zip_count: u64,
    strings: Vec<LocatedString>,
    ascii_buf: Vec<u8>,
    ascii_start: Option<usize>,
    carry: Vec<u8>,
}

impl OnePassScan {
    fn new() -> Self {
        Self {
            prefix: Vec::with_capacity(PREFIX_KEEP.min(CHUNK_SIZE * 2)),
            decompress_window: Vec::with_capacity(DECOMPRESS_WINDOW.min(CHUNK_SIZE * 4)),
            zstd_count: 0,
            zstd_offsets: Vec::new(),
            gzip_count: 0,
            zip_count: 0,
            strings: Vec::new(),
            ascii_buf: Vec::new(),
            ascii_start: None,
            carry: Vec::new(),
        }
    }

    fn ingest(&mut self, chunk: &[u8], absolute_offset: usize) {
        if self.prefix.len() < PREFIX_KEEP {
            let need = PREFIX_KEEP - self.prefix.len();
            self.prefix
                .extend_from_slice(&chunk[..chunk.len().min(need)]);
        }
        if self.decompress_window.len() < DECOMPRESS_WINDOW {
            let need = DECOMPRESS_WINDOW - self.decompress_window.len();
            self.decompress_window
                .extend_from_slice(&chunk[..chunk.len().min(need)]);
        }

        // Magic detection with small carry across chunk boundaries.
        let mut window = self.carry.clone();
        let carry_len = window.len();
        window.extend_from_slice(chunk);
        let start_from = carry_len.saturating_sub(MAGIC_OVERLAP);
        self.count_magics(
            &window,
            start_from,
            absolute_offset.saturating_sub(carry_len),
        );
        let keep = MAGIC_OVERLAP.min(window.len());
        self.carry = window[window.len() - keep..].to_vec();

        // Streaming ASCII string samples only (bounded).
        if self.strings.len() < MAX_STRING_SAMPLES {
            self.collect_ascii(chunk, absolute_offset);
        }
    }

    fn finish(&mut self) {
        self.flush_ascii();
    }

    fn count_magics(&mut self, data: &[u8], start_from: usize, absolute_base: usize) {
        let from = start_from.min(data.len());
        let mut i = from;
        while i < data.len() {
            match data[i] {
                0x28 if i + 4 <= data.len() && data[i..i + 4] == ZSTD_MAGIC => {
                    self.zstd_count += 1;
                    let abs = absolute_base.saturating_add(i);
                    if self.zstd_offsets.len() < MAX_FRAMES && abs < DECOMPRESS_WINDOW {
                        self.zstd_offsets.push(abs);
                    }
                    i += 4;
                    continue;
                }
                0x1F if i + 2 <= data.len() && data[i..i + 2] == GZIP_MAGIC => {
                    self.gzip_count += 1;
                    i += 2;
                    continue;
                }
                0x50 if i + 4 <= data.len() && data[i..i + 4] == ZIP_LOCAL => {
                    self.zip_count += 1;
                    i += 4;
                    continue;
                }
                _ => {}
            }
            i += 1;
        }
    }

    fn collect_ascii(&mut self, chunk: &[u8], absolute_offset: usize) {
        for (i, &b) in chunk.iter().enumerate() {
            if matches!(b, 0x09 | 0x20..=0x7E) {
                if self.ascii_start.is_none() {
                    self.ascii_start = Some(absolute_offset + i);
                }
                if self.ascii_buf.len() < MAX_STRING_LEN {
                    self.ascii_buf.push(b);
                }
            } else {
                self.flush_ascii();
                if self.strings.len() >= MAX_STRING_SAMPLES {
                    return;
                }
            }
        }
    }

    fn flush_ascii(&mut self) {
        if self.strings.len() >= MAX_STRING_SAMPLES {
            self.ascii_buf.clear();
            self.ascii_start = None;
            return;
        }
        if self.ascii_buf.len() >= MIN_STRING_LEN {
            if let (Some(offset), Ok(s)) = (self.ascii_start, std::str::from_utf8(&self.ascii_buf))
            {
                let trimmed = s.trim();
                if !trimmed.is_empty() {
                    self.strings.push(LocatedString {
                        value: trimmed.to_string(),
                        offset,
                        encoding: "ascii",
                    });
                }
            }
        }
        self.ascii_buf.clear();
        self.ascii_start = None;
    }
}

fn detect_header_signatures(header: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    if header.starts_with(&ZSTD_MAGIC) {
        out.push("zstd-frame@0".into());
    }
    if header.starts_with(&GZIP_MAGIC) {
        out.push("gzip@0".into());
    }
    if header.starts_with(&ZIP_LOCAL) {
        out.push("zip-local@0".into());
    } else if header.starts_with(&[0x50, 0x4B]) {
        out.push("zip-pk@0".into());
    }
    if header.windows(2).any(|w| w == b"FM") {
        out.push("ascii:FM".into());
    }
    if out.is_empty() {
        out.push("keine offensichtliche Signatur im Header".into());
    }
    out
}

fn classify_compression(header: &[u8], scan: &OnePassScan) -> DetectedCompression {
    let header_zstd = header.starts_with(&ZSTD_MAGIC);
    let header_gzip = header.starts_with(&GZIP_MAGIC);
    let header_zip = header.starts_with(&ZIP_LOCAL) || header.starts_with(&[0x50, 0x4B]);
    let has_zstd = header_zstd || scan.zstd_count > 0;
    let has_gzip = header_gzip || scan.gzip_count > 0;
    let has_zip = header_zip || scan.zip_count > 0;
    let kinds = [has_zstd, has_gzip, has_zip].iter().filter(|&&v| v).count();
    match kinds {
        0 => DetectedCompression::None,
        1 if has_zstd => DetectedCompression::Zstd,
        1 if has_gzip => DetectedCompression::Gzip,
        1 if has_zip => DetectedCompression::Zip,
        n if n > 1 => DetectedCompression::Mixed,
        _ => DetectedCompression::Unknown,
    }
}
