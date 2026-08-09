//! Directory-driven `Human Manager → Managed Club → Competition` resolution.
//!
//! Nothing here is keyed on a club, league, nation or filename: the save's own
//! section directory is read, the manager/club pair comes from the game's
//! `save_game_summary` record, and the club's numeric id comes from the
//! `game_db` name table. Promotions, relegations, lower leagues and custom
//! databases therefore need no special handling — the values are whatever the
//! save says they are at that moment.
//!
//! Saves are opened read-only and never modified.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::club_record::{collect_club_windows, resolve_club_record};
use super::competition_name::{
    resolve_competition_name, CompetitionNameSource, StandardCompetitionCatalogue,
};
use super::competition_table::{build_catalogue, CompetitionCatalogue};
use super::name_table::{club_id_for_name, full_name_for_id, scan_name_records, NameRecord};
use super::save_summary::{parse_save_summary, SaveSummary};
use super::types::{Evidence, EvidenceKind, ResolutionConfidence};
use crate::fm::container::directory::{parse_directory, read_section, stream_section, FmDirectory};
use crate::fm::errors::FmError;

/// Section holding the load-screen summary (manager, club, date).
const SEC_SUMMARY: &str = "save_game_summary";
/// Section holding the entity database (names, ids, relationships).
const SEC_GAME_DB: &str = "game_db";
/// Carry between streamed chunks so records spanning a boundary still parse.
const CHUNK_OVERLAP: usize = 1024;
/// Cap on club-name windows kept while streaming; each is ~600 bytes.
const MAX_CLUB_WINDOWS: usize = 64;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveResolution {
    pub save_name: String,
    pub section_count: usize,

    pub manager_name: Option<String>,
    pub club_name: Option<String>,
    pub club_full_name: Option<String>,
    pub club_id: Option<u32>,
    pub in_game_year: Option<u16>,

    pub competition_name: Option<String>,
    pub competition_id: Option<u32>,
    /// Where `competition_name` came from. `Unavailable` means the uid stands on
    /// its own — it never means the uid is in doubt.
    pub competition_name_source: CompetitionNameSource,
    /// Day-of-year and year of the fixture the competition reading applies to,
    /// or `None` between seasons (FM writes its 1/1900 null date there).
    pub competition_as_of: Option<(u16, u16)>,
    /// Competition ids → names read from this save (empty unless `deep`).
    /// Names the id the trailer selects; without `deep` the id stands alone.
    pub competition_catalogue: CompetitionCatalogue,

    /// Leagues loaded in this save — context, not the managed competition.
    pub loaded_competitions: Vec<String>,

    pub manager_confidence: ResolutionConfidence,
    pub club_confidence: ResolutionConfidence,
    pub competition_confidence: ResolutionConfidence,
    pub evidence: Vec<Evidence>,
    pub relationship_summary: String,
}

/// Resolve without touching `game_db` — manager and club only. Milliseconds.
pub fn resolve_summary_only(path: &Path) -> Result<(FmDirectory, SaveSummary), FmError> {
    let dir = parse_directory(path)?;
    let entry = dir
        .section(SEC_SUMMARY)
        .ok_or_else(FmError::container)?
        .clone();
    let bytes = read_section(path, &dir, &entry)?;
    Ok((dir, parse_save_summary(&bytes)))
}

/// Full resolution. `deep` additionally streams `game_db` to recover the club's
/// numeric id and canonical full name from the name table.
pub fn resolve_save(path: &Path, deep: bool) -> Result<SaveResolution, FmError> {
    let (dir, summary) = resolve_summary_only(path)?;

    let mut evidence = summary.evidence.clone();
    evidence.push(Evidence {
        kind: EvidenceKind::StructuralRecord,
        detail: format!(
            "Section directory parsed: {} sections, save '{}'",
            dir.sections.len(),
            dir.save_name
        ),
        block_index: None,
        offset: None,
    });

    let mut club_id = summary.club_id_hint;
    let mut club_full_name = None;
    let mut competition_catalogue = CompetitionCatalogue::default();

    if deep {
        if let (Some(club), Some(gdb)) = (summary.club_name.as_deref(), dir.section(SEC_GAME_DB)) {
            let mut records: Vec<NameRecord> = Vec::new();
            let mut club_windows: Vec<Vec<u8>> = Vec::new();
            let mut tail: Vec<u8> = Vec::new();
            let needle = club.as_bytes().to_vec();
            stream_section(path, &dir, gdb, &mut |abs, chunk| {
                let carried = tail.len();
                let base = abs - carried as u64;
                let mut buf = std::mem::take(&mut tail);
                buf.extend_from_slice(chunk);
                scan_name_records(&buf, base, &mut records);
                // Cheap in the same pass, and the only way a save whose name
                // table will not parse can still yield a club uid.
                collect_club_windows(&buf, carried, &needle, &mut club_windows, MAX_CLUB_WINDOWS);
                let keep = CHUNK_OVERLAP.min(buf.len());
                tail = buf[buf.len() - keep..].to_vec();
            })?;

            records.sort_by_key(|r| r.offset);
            records.dedup_by_key(|r| r.offset);
            evidence.push(Evidence {
                kind: EvidenceKind::StructuralRecord,
                detail: format!("Name table: {} records indexed from game_db", records.len()),
                block_index: None,
                offset: None,
            });

            competition_catalogue = build_catalogue(&records);
            evidence.push(Evidence {
                kind: EvidenceKind::StructuralRecord,
                detail: format!(
                    "Competition catalogue: {} competitions named from game_db",
                    competition_catalogue.len()
                ),
                block_index: None,
                offset: None,
            });

            match club_id_for_name(&records, club) {
                Some(id) => {
                    club_full_name = full_name_for_id(&records, id);
                    if club_id.is_some_and(|hint| hint != id) {
                        evidence.push(Evidence {
                            kind: EvidenceKind::Absent,
                            detail: format!(
                                "Summary club-id hint {:?} disagrees with name table {id}; \
                                 using the name table",
                                club_id
                            ),
                            block_index: None,
                            offset: None,
                        });
                    }
                    club_id = Some(id);
                    evidence.push(Evidence {
                        kind: EvidenceKind::RelationshipLink,
                        detail: format!("Club '{club}' resolved to unique id {id}"),
                        block_index: None,
                        offset: None,
                    });
                }
                None => {
                    evidence.push(Evidence {
                        kind: EvidenceKind::Absent,
                        detail: format!("No unambiguous name-table id for club '{club}'"),
                        block_index: None,
                        offset: None,
                    });
                    // Older saves (the VVV-Venlo one is FM 26.0.0) use a name
                    // table layout this parser yields nothing from, and carry no
                    // inline uid either. The club table is still walkable, so the
                    // club's own record supplies the uid — but only when it is
                    // unambiguous, and never over an id already established.
                    if club_id.is_none() {
                        let refs: Vec<&[u8]> = club_windows.iter().map(|w| w.as_slice()).collect();
                        match resolve_club_record(&refs, club, None) {
                            Some(rec) => {
                                club_id = Some(rec.club_id as u32);
                                club_full_name = Some(rec.full_name.clone());
                                evidence.push(Evidence {
                                    kind: EvidenceKind::RelationshipLink,
                                    detail: format!(
                                        "Club '{club}' resolved to id {} from its club-table \
                                         record ({} name-field occurrences, all in agreement)",
                                        rec.club_id,
                                        club_windows.len()
                                    ),
                                    block_index: None,
                                    offset: Some(rec.offset as u64),
                                });
                            }
                            None if !club_windows.is_empty() => evidence.push(Evidence {
                                kind: EvidenceKind::Absent,
                                detail: format!(
                                    "{} club-table records answer to '{club}' and disagree on \
                                     the uid — refusing to choose",
                                    club_windows.len()
                                ),
                                block_index: None,
                                offset: None,
                            }),
                            None => {}
                        }
                    }
                }
            }
        }
    }

    // The managed club's current competition comes from the summary trailer
    // (see `current_league`). In deep mode the catalogue must also know the id:
    // a uid absent from this save's own competition names is not trustworthy,
    // so it is reported at Medium rather than presented as fact.
    let competition_id = summary.current_league.as_ref().map(|c| c.competition_uid);
    let competition_as_of = summary
        .current_league
        .as_ref()
        .filter(|c| !c.fixture_date.is_null())
        .map(|c| (c.fixture_date.day_of_year, c.fixture_date.year));
    // Display name only — a name lookup can never change or reject the uid.
    let standard = StandardCompetitionCatalogue::load_default();
    if let Some(err) = &standard.load_error {
        evidence.push(Evidence {
            kind: EvidenceKind::Absent,
            detail: format!("Standard competition catalogue unavailable: {err}"),
            block_index: None,
            offset: None,
        });
    }
    let display = competition_id
        .map(|id| resolve_competition_name(id, &competition_catalogue, Some(&standard)));
    let competition_name = display.as_ref().and_then(|d| d.display_name.clone());
    let competition_name_source = display
        .as_ref()
        .map(|d| d.source)
        .unwrap_or(CompetitionNameSource::Unavailable);
    if let Some(d) = &display {
        evidence.push(Evidence {
            kind: EvidenceKind::StructuralRecord,
            detail: format!(
                "Competition {} display name {:?} from {}",
                d.competition_uid,
                d.display_name,
                d.source.as_str()
            ),
            block_index: None,
            offset: None,
        });
    }

    // Confidence in the *uid*, cross-checked against the save's own name table
    // only. The standard catalogue is display data and is deliberately not
    // allowed to vouch for a uid.
    let competition_confidence = match (competition_id, deep) {
        (None, _) => ResolutionConfidence::Unknown,
        (Some(_), false) => ResolutionConfidence::High,
        (Some(id), true) => {
            if competition_catalogue.is_empty() {
                ResolutionConfidence::High
            } else if competition_catalogue.knows(id) {
                ResolutionConfidence::Confirmed
            } else {
                evidence.push(Evidence {
                    kind: EvidenceKind::Absent,
                    detail: format!(
                        "Competition uid {id} is not in this save's competition catalogue"
                    ),
                    block_index: None,
                    offset: None,
                });
                ResolutionConfidence::Medium
            }
        }
    };
    if competition_id.is_none() {
        evidence.push(Evidence {
            kind: EvidenceKind::Absent,
            detail: "No current-competition trailer in save_game_summary — left unresolved".into(),
            block_index: None,
            offset: None,
        });
    }

    let club_confidence = if club_id.is_some() {
        ResolutionConfidence::High
    } else {
        summary.confidence
    };

    // A verified uid whose display name this save does not carry is still a
    // result: report the id rather than discarding the edge over a missing label.
    let competition_label = competition_name
        .clone()
        .unwrap_or_else(|| match competition_id {
            Some(id) => format!("uid {id}"),
            None => "unresolved".to_string(),
        });
    let relationship_summary = format!(
        "Manager('{}') → Club('{}') → Competition('{}')",
        summary.manager_name.as_deref().unwrap_or("?"),
        summary.club_name.as_deref().unwrap_or("?"),
        competition_label
    );

    Ok(SaveResolution {
        save_name: dir.save_name.clone(),
        section_count: dir.sections.len(),
        manager_name: summary.manager_name.clone(),
        club_name: summary.club_name.clone(),
        club_full_name,
        club_id,
        in_game_year: summary.in_game_year,
        competition_name,
        competition_id,
        competition_name_source,
        competition_as_of,
        competition_catalogue,
        loaded_competitions: summary.loaded_competitions.clone(),
        manager_confidence: summary.confidence,
        club_confidence,
        competition_confidence,
        evidence,
        relationship_summary,
    })
}
