//! Build save-identity analysis from FMF → Zstd frames (no invented metadata).

use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::fm::compression::zstd::{decompress_zstd_frame_bounded, MAX_FRAME_IN};
use crate::fm::container::directory::{parse_directory, read_section, stream_section, FmDirectory};
use crate::fm::container::header::parse_fmf_header;
use crate::fm::container::reader::{open_file_info, read_slice};
use crate::fm::container::tad::inspect_tad;
use crate::fm::entities::club_record::{
    collect_club_windows, find_club_record_with_hint, matches_uid,
};
use crate::fm::entities::managed_club::resolve_managed_club;
use crate::fm::entities::managed_club::resolve_managed_club_with_summary;
use crate::fm::entities::{
    build_catalogue, detect_game_version, resolve_human_manager_club, scan_name_records,
    CompetitionCatalogue, EntityRegistry, NameRecord, RelationshipGraph, PARSER_ENGINE_VERSION,
    PARSER_SCHEMA_VERSION,
};
use crate::fm::types::{
    FmEngineResult, FmSaveAnalysisResult, IdentityConfidenceMap, ParserMetadata,
};

const IDENTITY_MAX_OUT: usize = 32 * 1024 * 1024;

/// Section holding the entity database (names, ids, club records).
const SEC_GAME_DB: &str = "game_db";
/// Section holding the load-screen summary (manager, club, date, trailer).
const SEC_SUMMARY: &str = "save_game_summary";
/// Bytes carried from one streamed chunk to the next so a record straddling a
/// chunk boundary is still seen whole.
const CHUNK_OVERLAP: usize = 1024;
/// Cap on club-name windows retained from one save. Each is ~600 bytes, so even
/// a pathological save costs tens of kilobytes rather than the whole `game_db`.
const MAX_CLUB_WINDOWS: usize = 64;

pub const IDENTITY_PROGRESS_EVENT: &str = "scoutiq://fm-identity-progress";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityProgress {
    pub stage: String,
    pub progress: u32,
    pub message: String,
    pub frames_scanned: u64,
    pub frames_total: u64,
}

pub fn analyze_identity_from_engine(engine: &FmEngineResult) -> FmSaveAnalysisResult {
    analyze_identity_from_engine_with_progress(engine, None)
}

pub fn analyze_identity_from_engine_with_progress(
    engine: &FmEngineResult,
    app: Option<&AppHandle>,
) -> FmSaveAnalysisResult {
    match analyze_identity_from_path_with_progress(&engine.file.file_path, app) {
        Ok(r) => r,
        Err(_) => {
            let game_version = detect_game_version(&engine.file);
            let managed =
                resolve_managed_club(engine, &EntityRegistry::new(), &RelationshipGraph::new());
            wrap_result(
                engine.file.clone(),
                game_version,
                managed,
                vec!["Fallback identity path (engine index only)".into()],
            )
        }
    }
}

pub fn analyze_save_identity(path: &str) -> Result<FmSaveAnalysisResult, String> {
    analyze_identity_from_path_with_progress(path, None)
}

pub fn analyze_save_identity_with_progress(
    path: &str,
    app: &AppHandle,
) -> Result<FmSaveAnalysisResult, String> {
    analyze_identity_from_path_with_progress(path, Some(app))
}

fn emit_progress(app: Option<&AppHandle>, progress: IdentityProgress) {
    if let Some(app) = app {
        let _ = app.emit(IDENTITY_PROGRESS_EVENT, progress);
    }
}

fn analyze_identity_from_path_with_progress(
    path_str: &str,
    app: Option<&AppHandle>,
) -> Result<FmSaveAnalysisResult, String> {
    emit_progress(
        app,
        IdentityProgress {
            stage: "opening".into(),
            progress: 2,
            message: "Spielstand wird geöffnet …".into(),
            frames_scanned: 0,
            frames_total: 0,
        },
    );

    let file = open_file_info(path_str).map_err(|e| e.message)?;
    let header_bytes = {
        let path = Path::new(path_str);
        read_slice(path, 0, 64).map_err(|e| e.message)?
    };
    let fmf = parse_fmf_header(&header_bytes);
    let mut warnings = fmf.notes.clone();

    let mut game_version = detect_game_version(&file);
    let zstd_off = fmf
        .first_zstd_offset
        .ok_or_else(|| "Containerstruktur konnte nicht gelesen werden.".to_string())?;

    let path = Path::new(path_str);
    let file_len = file.file_size as usize;
    let compressed =
        read_slice(path, zstd_off, MAX_FRAME_IN.min(file_len)).map_err(|e| e.message)?;

    emit_progress(
        app,
        IdentityProgress {
            stage: "decompress".into(),
            progress: 8,
            message: "Container wird entpackt …".into(),
            frames_scanned: 0,
            frames_total: 0,
        },
    );

    let outcome = decompress_zstd_frame_bounded(&compressed, IDENTITY_MAX_OUT);
    if !outcome.ok {
        return Err(outcome
            .error
            .unwrap_or_else(|| "Komprimierter Datenblock konnte nicht entpackt werden.".into()));
    }

    let mut payload = outcome.decompressed;
    if payload.len() > IDENTITY_MAX_OUT {
        payload.truncate(IDENTITY_MAX_OUT);
        warnings.push(format!(
            "Identity scan truncated to {IDENTITY_MAX_OUT} decompressed bytes"
        ));
    }

    let tad = inspect_tad(&payload);
    warnings.extend(tad.evidence.iter().map(|e| e.detail.clone()));
    if let Some(v) = &tad.game_version_string {
        game_version.game = Some("Football Manager".into());
        if let Some(major) = v.split('.').next().and_then(|s| s.parse::<u32>().ok()) {
            game_version.major_version = Some(major);
        }
        game_version
            .evidence
            .push(crate::fm::entities::Evidence::header(format!(
                "Game version from tad.: {v}"
            )));
    }

    emit_progress(
        app,
        IdentityProgress {
            stage: "club_db".into(),
            progress: 15,
            message: "Vereinsdatenbank wird gelesen …".into(),
            frames_scanned: 0,
            frames_total: 0,
        },
    );

    // The section directory is parsed once and reused: `save_game_summary` and
    // `game_db` are both reached through it, so nothing below re-reads the file
    // from the start or scans it for frame magic.
    let directory = parse_directory(path).ok();
    if directory.is_none() {
        warnings.push("Abschnittsverzeichnis nicht lesbar — Spielklasse bleibt offen".into());
    }

    // `save_game_summary`. Two things need it: the competition trailer is
    // positional, so it only means anything at the end of that exact section;
    // and the `0E 01` manager→club pair lives there, which the bounded
    // front-of-file decompression above does not always reach (the TSV 1860
    // München save is one where it does not).
    let summary_section = directory
        .as_ref()
        .and_then(|d| read_summary_section(path, d));
    if summary_section.is_none() {
        warnings.push("save_game_summary section not readable — Spielklasse bleibt offen".into());
    }
    let pair_source: &[u8] = summary_section.as_deref().unwrap_or(&payload);

    // The managed club's display name, needed as the needle for the club-record
    // search below. It comes from the summary's own `0E 01` record, never the
    // filename.
    let manager_hit = resolve_human_manager_club(pair_source);
    let short_hint = manager_hit.as_ref().map(|h| h.club_name.clone());
    let uid_hint = manager_hit.as_ref().and_then(|h| h.club_id_hint);

    emit_progress(
        app,
        IdentityProgress {
            stage: "competitions".into(),
            progress: 40,
            message: "Wettbewerbe werden gelesen …".into(),
            frames_scanned: 0,
            frames_total: 0,
        },
    );

    // One streamed pass over `game_db` produces both the competition-name
    // catalogue and the club-record windows. `game_db` is the only section
    // touched, it is never held in memory whole, and no other section is
    // decompressed.
    let scan = match directory.as_ref() {
        Some(dir) => scan_game_db(path, dir, short_hint.as_deref(), uid_hint, app),
        None => GameDbScan::default(),
    };
    warnings.extend(scan.notes);
    let catalogue = scan.catalogue;
    if catalogue.is_none() {
        warnings.push("Wettbewerbsnamen nicht lesbar — nur Wettbewerbs-ID".into());
    }

    // Club-record sources, cheapest first. The front-of-file `tad.` payload is
    // already decompressed and bounded at 8 MB, so it stays in play.
    let mut club_refs: Vec<&[u8]> = scan.club_windows.iter().map(|v| v.as_slice()).collect();
    club_refs.push(&payload);
    let managed = resolve_managed_club_with_summary(
        pair_source,
        &club_refs,
        summary_section.as_deref(),
        catalogue.as_ref(),
    );

    if managed.club_name.is_none() {
        warnings.push("Verein noch nicht strukturell auflösbar".into());
    }
    if managed.competition_name.is_none() {
        warnings.push("Spielklasse noch nicht strukturell verifiziert".into());
    }

    emit_progress(
        app,
        IdentityProgress {
            stage: "done".into(),
            progress: 100,
            message: "Identität aufgelöst".into(),
            frames_scanned: scan.chunks_done,
            frames_total: scan.chunks_total,
        },
    );

    Ok(wrap_result(file, game_version, managed, warnings))
}

/// Decompressed `save_game_summary`, located through the save's own section
/// directory. Returns `None` when the section is absent or will not decompress.
fn read_summary_section(path: &Path, dir: &FmDirectory) -> Option<Vec<u8>> {
    let entry = dir.section(SEC_SUMMARY)?.clone();
    read_section(path, dir, &entry).ok()
}

/// What one streamed pass over `game_db` yields.
#[derive(Default)]
struct GameDbScan {
    /// `competition uid → display name`, built from the name table.
    catalogue: Option<CompetitionCatalogue>,
    /// Small byte windows around each `LP(short) == managed club name` match,
    /// carrying enough context for the club-record parser and nothing more.
    club_windows: Vec<Vec<u8>>,
    notes: Vec<String>,
    chunks_done: u64,
    chunks_total: u64,
}

/// Stream `game_db` once, extracting everything the identity result needs from
/// it: the competition-name catalogue and the managed club's record.
///
/// This replaces an earlier design that read the entire save into memory with
/// `fs::read`, scanned it for Zstd frame magic and decompressed up to 80 frames
/// regardless of what they held. On a 640 MB save that cost ~700 MB of resident
/// memory; the section directory makes it unnecessary, because `game_db` can be
/// decompressed on its own in bounded chunks and no other section is touched.
///
/// Neither the competition uid nor the `save_game_summary` trailer depends on
/// anything here: a total failure costs the competition's *name* and the club's
/// *record*, never the uid.
fn scan_game_db(
    path: &Path,
    dir: &FmDirectory,
    short_name: Option<&str>,
    uid_hint: Option<u32>,
    app: Option<&AppHandle>,
) -> GameDbScan {
    let Some(gdb) = dir.section(SEC_GAME_DB).cloned() else {
        return GameDbScan {
            notes: vec!["game_db section absent — Vereinsdatensatz nicht lesbar".into()],
            ..Default::default()
        };
    };

    let chunk_size = 4u64 << 20;
    let chunks_total = gdb.decompressed_size.div_ceil(chunk_size).max(1);
    let needle = short_name.map(|s| s.as_bytes().to_vec());

    let mut records: Vec<NameRecord> = Vec::new();
    let mut club_windows: Vec<Vec<u8>> = Vec::new();
    let mut tail: Vec<u8> = Vec::new();
    let mut chunks_done = 0u64;
    let mut truncated = false;
    // Set once a window has been seen that parses to the club the summary names
    // by uid. Nothing later can beat an exact uid match, so the search stops —
    // the same early exit the old frame loop had, without its whole-file read.
    let mut club_settled = false;

    let streamed = stream_section(path, dir, &gdb, &mut |abs, chunk| {
        let carried = tail.len();
        let base = abs - carried as u64;
        let mut buf = std::mem::take(&mut tail);
        buf.extend_from_slice(chunk);

        scan_name_records(&buf, base, &mut records);
        if let Some(n) = needle.as_deref() {
            if !club_settled && club_windows.len() < MAX_CLUB_WINDOWS {
                let first_new = club_windows.len();
                collect_club_windows(&buf, carried, n, &mut club_windows, MAX_CLUB_WINDOWS);
                if club_windows.len() >= MAX_CLUB_WINDOWS {
                    truncated = true;
                }
                if let (Some(hint), Some(name)) = (uid_hint, short_name) {
                    club_settled = club_windows[first_new..].iter().any(|w| {
                        find_club_record_with_hint(w, name, Some(hint))
                            .is_some_and(|r| matches_uid(r.club_id, hint))
                    });
                }
            }
        }

        let keep = CHUNK_OVERLAP.min(buf.len());
        tail = buf[buf.len() - keep..].to_vec();
        chunks_done += 1;

        emit_progress(
            app,
            IdentityProgress {
                stage: "club_db".into(),
                // 40 → 95 across the section, leaving room either side.
                progress: 40 + (55 * chunks_done / chunks_total.max(1)).min(55) as u32,
                message: "Vereinsdatenbank wird gelesen …".into(),
                frames_scanned: chunks_done,
                frames_total: chunks_total,
            },
        );
    });

    let mut notes = Vec::new();
    if streamed.is_err() {
        notes.push("game_db konnte nicht vollständig entpackt werden".into());
    }
    if truncated {
        notes.push(format!(
            "Club-name matches capped at {MAX_CLUB_WINDOWS}; later occurrences not examined"
        ));
    }
    notes.push(format!(
        "game_db streamed in {chunks_done} chunks ({} name records, {} club-name windows)",
        records.len(),
        club_windows.len()
    ));

    records.sort_by_key(|r| r.offset);
    records.dedup_by_key(|r| r.offset);

    GameDbScan {
        catalogue: Some(build_catalogue(&records)),
        club_windows,
        notes,
        chunks_done,
        chunks_total,
    }
}

/// Assemble the user-facing result: parser metadata, per-field confidence
/// labels, and the managed-club block filtered through [`ManagedClubResult::for_ui`]
/// so nothing below user-facing confidence is presented as fact.
fn wrap_result(
    file: crate::fm::types::FmFileInfo,
    game_version: crate::fm::entities::GameVersionInfo,
    managed: crate::fm::entities::ManagedClubResult,
    warnings: Vec<String>,
) -> FmSaveAnalysisResult {
    let confidence = IdentityConfidenceMap {
        manager: managed.manager_confidence.as_str().into(),
        club: managed.club_confidence.as_str().into(),
        competition: managed.competition_confidence.as_str().into(),
        season: managed.season_confidence.as_str().into(),
        date: managed.date_confidence.as_str().into(),
        overall: managed.confidence.as_str().into(),
    };
    let parser_info = ParserMetadata {
        engine_version: PARSER_ENGINE_VERSION.to_string(),
        game_version: game_version.major_version.map(|v| v.to_string()),
        schema_version: PARSER_SCHEMA_VERSION,
    };
    FmSaveAnalysisResult {
        file_info: file,
        parser_info,
        game_version,
        managed: managed.for_ui(),
        warnings,
        confidence,
    }
}

#[cfg(test)]
mod live_save_tests {
    use super::*;
    use std::path::PathBuf;

    /// Live checks against real saves. Point `SCOUTIQ_TEST_SAVES` at a directory
    /// of `.fm` files:
    ///
    /// ```text
    /// SCOUTIQ_TEST_SAVES=/path/to/saves cargo test --release --lib -- --nocapture
    /// ```
    ///
    /// Without it the checks skip *loudly*. They were previously keyed on
    /// absolute paths from one developer's machine, so on every other machine
    /// they passed vacuously while appearing to verify real saves.
    fn saves() -> Vec<PathBuf> {
        let Ok(dir) = std::env::var("SCOUTIQ_TEST_SAVES") else {
            return Vec::new();
        };
        let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "fm"))
            .collect();
        out.sort();
        out
    }

    /// Ground truth per club uid: the competition uids that club may legitimately
    /// be in across the calibration saves. Macclesfield has three saves spanning
    /// a promotion, so both League One (13) and the Championship (12) are valid —
    /// which of the two applies is the save's business, not the test's.
    fn expected_competitions(club_id: u64) -> Option<&'static [u32]> {
        match club_id {
            901 => Some(&[22]),               // Bayer Leverkusen  — Bundesliga
            933 => Some(&[23]),               // MSV Duisburg      — 2. Bundesliga
            955 => Some(&[91_107_111]),       // TSV 1860 München  — Regionalliga Bayern
            5_103_842 => Some(&[14]),         // Salford City      — EFL League Two
            2_000_095_665 => Some(&[13, 12]), // Macclesfield — League One → Championship
            _ => None,
        }
    }

    #[test]
    fn resolves_manager_and_club_on_calibration_saves() {
        let saves = saves();
        if saves.is_empty() {
            eprintln!(
                "SKIPPED live save checks — set SCOUTIQ_TEST_SAVES=<dir of .fm saves> to run them"
            );
            return;
        }
        for path in saves {
            let p = path.to_string_lossy().into_owned();
            let r = analyze_save_identity(&p).unwrap_or_else(|e| panic!("{p}: {e:?}"));
            assert!(r.managed.manager_name.is_some(), "no manager for {p}");
            assert!(r.managed.club_name.is_some(), "no club for {p}");
            assert!(
                r.managed.competition_id.is_some(),
                "no competition for {p} — the summary trailer should always carry one"
            );
            let Some(club) = r.managed.club_id else {
                continue;
            };
            let Some(allowed) = expected_competitions(club) else {
                continue;
            };
            let got = r.managed.competition_id.unwrap() as u32;
            assert!(
                allowed.contains(&got),
                "{p}: club {club} resolved to competition {got}, expected one of {allowed:?}"
            );
        }
    }

    /// Competition uids are the solved result and must not move when the name
    /// layer changes. Every save is checked, and every save must yield a uid.
    ///
    /// This deliberately asserts on the *uid* only: whether a display name is
    /// available is a separate concern that must never feed back into it.
    #[test]
    fn competition_uids_are_unchanged_by_name_resolution() {
        use crate::fm::entities::resolve_save;
        let saves = saves();
        if saves.is_empty() {
            eprintln!("SKIPPED uid regression check — set SCOUTIQ_TEST_SAVES");
            return;
        }
        let mut checked = 0usize;
        for path in &saves {
            let p = path.display().to_string();
            let shallow = resolve_save(path, false).unwrap_or_else(|e| panic!("{p}: {e:?}"));
            let deep = resolve_save(path, true).unwrap_or_else(|e| panic!("{p}: {e:?}"));
            assert!(shallow.competition_id.is_some(), "{p}: no competition uid");
            assert_eq!(
                shallow.competition_id, deep.competition_id,
                "{p}: uid changed between shallow and deep resolution"
            );
            let Some(club) = deep.club_id.map(u64::from) else {
                continue;
            };
            if let Some(allowed) = expected_competitions(club) {
                let uid = deep.competition_id.unwrap();
                assert!(
                    allowed.contains(&uid),
                    "{p}: club {club} → competition {uid}, expected one of {allowed:?}"
                );
                checked += 1;
            }
        }
        assert!(checked > 0, "no ground-truth save matched");
    }

    /// A save that names a competition itself must keep its own name, and the
    /// source must say so. `EFL League Two` / `Regionalliga Bayern` are recorded
    /// in the save; nothing may substitute a catalogue default for them.
    #[test]
    fn save_embedded_names_are_preferred_and_labelled() {
        use crate::fm::entities::{resolve_save, CompetitionNameSource};
        let saves = saves();
        if saves.is_empty() {
            eprintln!("SKIPPED name-source check — set SCOUTIQ_TEST_SAVES");
            return;
        }
        for path in saves {
            let p = path.display().to_string();
            let r = resolve_save(&path, true).unwrap_or_else(|e| panic!("{p}: {e:?}"));
            let Some(uid) = r.competition_id else {
                continue;
            };
            // Whatever the source, a uid never disappears because of naming.
            assert!(r.competition_id.is_some(), "{p}: uid lost");
            match r.competition_name_source {
                CompetitionNameSource::SaveShortName | CompetitionNameSource::SaveFullName => {
                    assert!(
                        r.competition_name.is_some(),
                        "{p}: source claims a save name but none was returned"
                    );
                }
                CompetitionNameSource::Unavailable => {
                    assert!(r.competition_name.is_none(), "{p}: name without a source");
                }
                _ => {}
            }
            // Known save-embedded names must come from the save, not a default.
            if matches!(uid, 13 | 14 | 91_107_111) {
                assert_eq!(
                    r.competition_name_source,
                    CompetitionNameSource::SaveShortName,
                    "{p}: uid {uid} should be named by the save itself"
                );
            }
        }
    }

    /// Directory-driven resolver: manager, club and competition straight from
    /// `save_game_summary`, with no `game_db` scan.
    #[test]
    fn resolves_current_league_from_the_section_directory() {
        use crate::fm::entities::resolve_save;
        let saves = saves();
        if saves.is_empty() {
            eprintln!("SKIPPED live save checks — set SCOUTIQ_TEST_SAVES");
            return;
        }
        for path in saves {
            let p = path.display().to_string();
            let r = resolve_save(&path, false).unwrap_or_else(|e| panic!("{p}: {e:?}"));
            assert!(r.manager_name.is_some(), "no manager for {p}");
            assert!(r.club_name.is_some(), "no club for {p}");
            let comp = r
                .competition_id
                .unwrap_or_else(|| panic!("no competition uid for {p}"));
            let Some(club) = r.club_id.map(u64::from) else {
                continue;
            };
            let Some(allowed) = expected_competitions(club) else {
                continue;
            };
            assert!(
                allowed.contains(&comp),
                "{p}: club {club} resolved to competition {comp}, expected one of {allowed:?}"
            );
        }
    }

    /// The same club, same team, two league assignments: the resolver must follow
    /// the save rather than remember an answer. Runs only when the directory
    /// holds more than one save for a club whose ground truth spans a promotion.
    #[test]
    fn tracks_a_club_across_a_division_change() {
        use crate::fm::entities::resolve_save;
        let saves = saves();
        if saves.is_empty() {
            eprintln!("SKIPPED cross-save check — set SCOUTIQ_TEST_SAVES");
            return;
        }
        let mut seen: std::collections::HashMap<u64, std::collections::BTreeSet<u32>> =
            Default::default();
        for path in saves {
            let Ok(r) = resolve_save(&path, false) else {
                continue;
            };
            if let (Some(club), Some(comp)) = (r.club_id, r.competition_id) {
                seen.entry(club as u64).or_default().insert(comp);
            }
        }
        let Some(macc) = seen.get(&2_000_095_665) else {
            eprintln!("SKIPPED — no Macclesfield saves in SCOUTIQ_TEST_SAVES");
            return;
        };
        assert!(
            macc.iter().all(|c| [12u32, 13].contains(c)),
            "Macclesfield resolved outside its known divisions: {macc:?}"
        );
        if macc.len() > 1 {
            assert!(
                macc.contains(&13) && macc.contains(&12),
                "expected both League One and the Championship across the saves, got {macc:?}"
            );
        }
    }

    /// The three Macclesfield saves are League One, League One, Championship.
    /// Asserted as a multiset rather than a sequence: ordering them would mean
    /// ranking saves by filename, which the resolver must never do, and the
    /// middle save carries the null fixture date so it cannot be ordered by that
    /// either.
    #[test]
    fn macclesfield_saves_are_two_league_one_and_one_championship() {
        use crate::fm::entities::resolve_save;
        let saves = saves();
        if saves.is_empty() {
            eprintln!("SKIPPED Macclesfield check — set SCOUTIQ_TEST_SAVES");
            return;
        }
        // Keyed on the save name the container itself carries, so the same save
        // copied into the directory twice counts once. Duplicates must agree.
        let mut by_save: std::collections::BTreeMap<String, u32> = Default::default();
        for path in saves {
            let Ok(r) = resolve_save(&path, false) else {
                continue;
            };
            if r.club_id != Some(2_000_095_665) {
                continue;
            }
            let Some(c) = r.competition_id else { continue };
            if let Some(prev) = by_save.insert(r.save_name.clone(), c) {
                assert_eq!(
                    prev, c,
                    "two copies of the same save disagree on the league"
                );
            }
        }
        if by_save.len() < 3 {
            eprintln!(
                "SKIPPED — {} distinct Macclesfield saves present, need 3",
                by_save.len()
            );
            return;
        }
        let mut uids: Vec<u32> = by_save.values().copied().collect();
        uids.sort_unstable();
        let ones = uids.iter().filter(|&&c| c == 13).count();
        let champs = uids.iter().filter(|&&c| c == 12).count();
        assert_eq!(ones, 2, "expected two League One saves, got {uids:?}");
        assert_eq!(champs, 1, "expected one Championship save, got {uids:?}");
        assert_eq!(ones + champs, uids.len(), "unexpected division in {uids:?}");
    }
}

#[cfg(test)]
mod live_league_probe {
    use crate::fm::compression::zstd::{decompress_zstd_frame_bounded, MAX_FRAME_IN};
    use crate::fm::container::header::parse_fmf_header;
    use crate::fm::container::reader::read_slice;
    use crate::fm::entities::club_record::{find_club_record, read_lp_name};
    use crate::fm::entities::resolve_human_manager_club;
    use std::collections::HashMap;
    use std::path::Path;

    /// Whole-file frame enumeration. The production path no longer does this —
    /// it reaches `game_db` through the section directory — but this diagnostic
    /// deliberately looks at *every* frame, so it keeps its own copy.
    fn find_zstd_offsets(data: &[u8]) -> Vec<usize> {
        use crate::fm::compression::zstd::ZSTD_MAGIC;
        let mut out = Vec::new();
        let mut i = 0usize;
        while i + 4 <= data.len() {
            if data[i..i + 4] == ZSTD_MAGIC {
                out.push(i);
                i += 4;
            } else {
                i += 1;
            }
        }
        out
    }

    fn decompress_at(data: &[u8], off: usize, max_out: usize) -> Option<Vec<u8>> {
        if off >= data.len() {
            return None;
        }
        let outcome = decompress_zstd_frame_bounded(&data[off..], max_out);
        (outcome.ok && !outcome.decompressed.is_empty()).then_some(outcome.decompressed)
    }

    fn primary(path: &Path) -> Option<Vec<u8>> {
        let header = read_slice(path, 0, 64).ok()?;
        let fmf = parse_fmf_header(&header);
        let zstd_off = fmf.first_zstd_offset?;
        let file_len = std::fs::metadata(path).ok()?.len() as usize;
        let compressed = read_slice(path, zstd_off, MAX_FRAME_IN.min(file_len)).ok()?;
        Some(decompress_zstd_frame_bounded(&compressed, 32 * 1024 * 1024).decompressed)
    }

    fn is_marker(m: &[u8]) -> bool {
        m.len() == 9
            && m[2] == 0
            && m[3] == 0
            && m[4] == 0x03
            && m[5] == 0
            && m[6] == 0
            && m[7] == 0
            && (m[1] == 0x02 || m[1] == 0x03)
    }

    fn build_comps(raw: &[u8], offs: &[usize]) -> HashMap<u32, (String, u8)> {
        let mut map = HashMap::new();
        for (fi, &off) in offs.iter().enumerate().take(5000) {
            if fi % 3 != 0 {
                continue;
            }
            let Some(chunk) = decompress_at(raw, off, 1536 * 1024) else {
                continue;
            };
            let mut i = 0usize;
            while i + 25 < chunk.len() {
                let m = &chunk[i + 8..i + 17];
                if !is_marker(m) {
                    i += 1;
                    continue;
                }
                let id = u32::from_le_bytes(chunk[i..i + 4].try_into().unwrap());
                if id == 0 || id > 200_000 {
                    i += 1;
                    continue;
                }
                if let Some((full, n1)) = read_lp_name(&chunk, i + 17) {
                    let l = full.to_ascii_lowercase();
                    let ok = (l.contains("liga")
                        || l.contains("league")
                        || l.contains("division")
                        || l.contains("bundes")
                        || l.contains("premier")
                        || l.contains("championship")
                        || l.contains("serie")
                        || l.contains("eredivisie")
                        || l.contains("ligue"))
                        && !l.contains("pokal")
                        && !l.contains("cup")
                        && !l.contains("u19")
                        && !l.contains("jugend")
                        && !l.contains("reserve");
                    if ok {
                        let short = read_lp_name(&chunk, n1)
                            .map(|(s, _)| s)
                            .unwrap_or(full.clone());
                        map.entry(id).or_insert((short, m[8]));
                    }
                }
                i += 1;
            }
        }
        map
    }

    #[test]
    #[ignore]
    fn diagnose_gap8_assignment() {
        let base = "/Users/niclaskrieck/Library/CloudStorage/OneDrive-Persönlich/Dokumente/Sports Interactive/Football Manager 26/games";
        let saves = [
            ("Lev", format!("{base}/Niclas Krieck – Bayer Leverkusen.fm")),
            ("Dui", format!("{base}/Niclas Krieck – MSV Duisburg.fm")),
            ("Mac", format!("{base}/Niclas Krieck – Macclesfield.fm")),
            (
                "Stu",
                format!("{base}/Niclas Krieck – VfB Stuttgart (v02).fm"),
            ),
            ("Her", format!("{base}/Niclas Krieck – Hertha BSC (v02).fm")),
        ];
        for (label, path) in &saves {
            if !Path::new(path).is_file() {
                continue;
            }
            let data = primary(Path::new(path)).unwrap();
            let hit = resolve_human_manager_club(&data).unwrap();
            let rec = find_club_record(&data, &hit.club_name).unwrap();
            let club = rec.club_id as u32;
            eprintln!("\n===== {label} {} uid={} =====", hit.club_name, club);
            let raw = std::fs::read(path).unwrap();
            let offs = find_zstd_offsets(&raw);
            let comps = build_comps(&raw, &offs);
            eprintln!("  comps loaded: {}", comps.len());
            for (id, (n, t)) in &comps {
                let l = n.to_ascii_lowercase();
                if l == "bundesliga"
                    || l == "2. bundesliga"
                    || l == "3. liga"
                    || l.contains("national league")
                    || l == "premier league"
                {
                    eprintln!("  entity id={id} tier={t} {n}");
                }
            }

            let club_b = club.to_le_bytes();
            // fixed gaps after club_uid: +4,+8,+12,+16,+20,+24
            let mut gap_scores: HashMap<(i32, u32), usize> = HashMap::new();
            let mut samples: HashMap<i32, String> = HashMap::new();
            for (fi, &off) in offs.iter().enumerate().take(5000) {
                if fi % 2 == 1 {
                    continue;
                }
                let Some(chunk) = decompress_at(&raw, off, 1536 * 1024) else {
                    continue;
                };
                let mut i = 0usize;
                while i + 28 < chunk.len() {
                    if chunk[i..i + 4] != club_b {
                        i += 1;
                        continue;
                    }
                    for gap in [4i32, 8, 12, 16, 20, 24, -4, -8, -12] {
                        let j = i as i32 + gap;
                        if j < 0 || (j as usize) + 4 > chunk.len() {
                            continue;
                        }
                        let j = j as usize;
                        let v = u32::from_le_bytes(chunk[j..j + 4].try_into().unwrap());
                        if comps.contains_key(&v) {
                            *gap_scores.entry((gap, v)).or_default() += 1;
                            samples.entry(gap).or_insert_with(|| {
                                let s = i.saturating_sub(8);
                                let e = (i + 32).min(chunk.len());
                                format!(
                                    "f{fi}+{i}: {}",
                                    chunk[s..e]
                                        .iter()
                                        .map(|b| format!("{b:02x}"))
                                        .collect::<Vec<_>>()
                                        .join(" ")
                                )
                            });
                        }
                    }
                    i += 4;
                }
            }
            let mut ranked: Vec<_> = gap_scores.into_iter().collect();
            ranked.sort_by(|a, b| b.1.cmp(&a.1));
            eprintln!("  fixed-gap club→comp:");
            for ((gap, id), c) in ranked.iter().take(15) {
                let (name, tier) = comps.get(id).cloned().unwrap_or_default();
                eprintln!("    gap={gap:+} id={id} tier={tier} ×{c} {name}");
            }
            for gap in [8i32, 4, 12, 16] {
                if let Some(s) = samples.get(&gap) {
                    eprintln!("  sample gap={gap}: {s}");
                }
            }
        }
    }
}

/// Read-only safety: a `.fm` save must come out of a full parse byte-identical.
///
/// The parser only ever calls `File::open`, but "we never wrote to it" is a
/// claim about every code path, including the container walk, the streamed
/// `game_db` pass and the Zstd decoders. This checks the property itself —
/// size, modification time and SHA-256 before and after
/// [`analyze_save_identity`] — rather than the intent behind it.
///
/// ```text
/// SCOUTIQ_TEST_SAVES=/path/to/saves cargo test --release --lib -- --nocapture
/// ```
///
/// Without that variable the check skips loudly instead of passing vacuously.
#[cfg(test)]
mod read_only_safety {
    use super::*;
    use std::fs::File;
    use std::io::Read;
    use std::path::{Path, PathBuf};

    fn saves() -> Vec<PathBuf> {
        let Ok(dir) = std::env::var("SCOUTIQ_TEST_SAVES") else {
            return Vec::new();
        };
        let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "fm"))
            .collect();
        out.sort();
        out
    }

    /// `(size, modified-unix-nanos, sha-256)`.
    fn fingerprint(path: &Path) -> (u64, u128, String) {
        let meta = std::fs::metadata(path).expect("metadata");
        let modified = meta
            .modified()
            .expect("mtime")
            .duration_since(std::time::UNIX_EPOCH)
            .expect("epoch")
            .as_nanos();
        let mut f = File::open(path).expect("open");
        let mut hasher = Sha256::new();
        let mut buf = vec![0u8; 1 << 20];
        loop {
            let n = f.read(&mut buf).expect("read");
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        (meta.len(), modified, hasher.finish())
    }

    #[test]
    fn parsing_a_save_leaves_the_file_untouched() {
        let saves = saves();
        if saves.is_empty() {
            eprintln!("SKIPPED read-only check — set SCOUTIQ_TEST_SAVES=<dir of .fm saves>");
            return;
        }
        for path in &saves {
            let p = path.display().to_string();
            let before = fingerprint(path);

            // The full production path, not a narrowed helper.
            let parsed = analyze_save_identity(&p);
            assert!(parsed.is_ok(), "{p}: parse failed: {:?}", parsed.err());

            let after = fingerprint(path);
            assert_eq!(before.0, after.0, "{p}: file size changed");
            assert_eq!(before.1, after.1, "{p}: modification time changed");
            assert_eq!(
                before.2, after.2,
                "{p}: SHA-256 changed — the save was rewritten"
            );
            eprintln!("read-only OK: {} bytes, sha256 {}", after.0, &after.2[..16]);
        }
    }

    /// A file opened read-only cannot be written through, and the parser must
    /// not need write access to begin with.
    #[test]
    fn parses_a_save_whose_file_is_write_protected() {
        let Some(path) = saves().into_iter().next() else {
            eprintln!("SKIPPED write-protection check — set SCOUTIQ_TEST_SAVES");
            return;
        };
        // The calibration saves ship read-only (mode 444); confirm the parser
        // is happy with that rather than silently relying on write permission.
        let meta = std::fs::metadata(&path).expect("metadata");
        let readonly = meta.permissions().readonly();
        let r = analyze_save_identity(&path.display().to_string());
        assert!(r.is_ok(), "parse failed on a read-only file: {:?}", r.err());
        eprintln!(
            "parsed {} (permissions readonly = {readonly})",
            path.display()
        );
    }

    #[test]
    fn sha256_matches_the_published_test_vectors() {
        let mut h = Sha256::new();
        h.update(b"abc");
        assert_eq!(
            h.finish(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );

        let mut h = Sha256::new();
        h.update(b"");
        assert_eq!(
            h.finish(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );

        // Spans a block boundary and exercises the streaming path.
        let mut h = Sha256::new();
        h.update(b"abcdbcdecdefdefgefghfghighijhi");
        h.update(b"jkijkljklmklmnlmnomnopnopq");
        assert_eq!(
            h.finish(),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );

        // One megabyte fed in uneven pieces must equal one flat update.
        let data: Vec<u8> = (0..1_000_000u32).map(|i| (i % 251) as u8).collect();
        let mut flat = Sha256::new();
        flat.update(&data);
        let mut split = Sha256::new();
        for part in data.chunks(7919) {
            split.update(part);
        }
        assert_eq!(flat.finish(), split.finish());
    }

    /// Minimal FIPS 180-4 SHA-256. Written out rather than pulled in as a
    /// dependency: it exists only to prove a file did not change, and the
    /// vectors above keep it honest.
    struct Sha256 {
        h: [u32; 8],
        block: [u8; 64],
        used: usize,
        len: u64,
    }

    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    impl Sha256 {
        fn new() -> Self {
            Self {
                h: [
                    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c,
                    0x1f83d9ab, 0x5be0cd19,
                ],
                block: [0u8; 64],
                used: 0,
                len: 0,
            }
        }

        fn update(&mut self, data: &[u8]) {
            self.len = self.len.wrapping_add(data.len() as u64);
            self.absorb(data);
        }

        /// Buffer and compress without counting the bytes — padding uses this.
        fn absorb(&mut self, mut data: &[u8]) {
            if self.used > 0 {
                let take = (64 - self.used).min(data.len());
                self.block[self.used..self.used + take].copy_from_slice(&data[..take]);
                self.used += take;
                data = &data[take..];
                if self.used == 64 {
                    let b = self.block;
                    self.compress(&b);
                    self.used = 0;
                }
            }
            while data.len() >= 64 {
                let (head, rest) = data.split_at(64);
                let mut b = [0u8; 64];
                b.copy_from_slice(head);
                self.compress(&b);
                data = rest;
            }
            if !data.is_empty() {
                self.block[..data.len()].copy_from_slice(data);
                self.used = data.len();
            }
        }

        fn finish(mut self) -> String {
            let bits = self.len.wrapping_mul(8);
            let rem = ((self.len + 1) % 64) as usize;
            let zeros = if rem <= 56 { 56 - rem } else { 120 - rem };
            let mut tail = Vec::with_capacity(1 + zeros + 8);
            tail.push(0x80u8);
            tail.extend(std::iter::repeat_n(0u8, zeros));
            tail.extend_from_slice(&bits.to_be_bytes());
            self.absorb(&tail);
            debug_assert_eq!(self.used, 0, "padding must land on a block boundary");
            self.h.iter().map(|w| format!("{w:08x}")).collect()
        }

        fn compress(&mut self, block: &[u8; 64]) {
            let mut w = [0u32; 64];
            for i in 0..16 {
                w[i] = u32::from_be_bytes(block[i * 4..i * 4 + 4].try_into().unwrap());
            }
            for i in 16..64 {
                let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
                let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
                w[i] = w[i - 16]
                    .wrapping_add(s0)
                    .wrapping_add(w[i - 7])
                    .wrapping_add(s1);
            }
            let (mut a, mut b, mut c, mut d) = (self.h[0], self.h[1], self.h[2], self.h[3]);
            let (mut e, mut f, mut g, mut hh) = (self.h[4], self.h[5], self.h[6], self.h[7]);
            for i in 0..64 {
                let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
                let ch = (e & f) ^ ((!e) & g);
                let t1 = hh
                    .wrapping_add(s1)
                    .wrapping_add(ch)
                    .wrapping_add(K[i])
                    .wrapping_add(w[i]);
                let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
                let maj = (a & b) ^ (a & c) ^ (b & c);
                let t2 = s0.wrapping_add(maj);
                hh = g;
                g = f;
                f = e;
                e = d.wrapping_add(t1);
                d = c;
                c = b;
                b = a;
                a = t1.wrapping_add(t2);
            }
            for (slot, v) in self.h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
                *slot = slot.wrapping_add(v);
            }
        }
    }
}
