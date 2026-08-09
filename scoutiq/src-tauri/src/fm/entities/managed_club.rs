//! Resolve human manager → club structurally.
//!
//! Primary path (FM26 tad. payload): unique `0E 01` length-prefixed person→club pair.
//! Club DB record supplies numeric id + full name.
//! The competition comes from the `save_game_summary` trailer decoded by
//! [`super::current_league`], and only when that whole section is available —
//! the trailer is positional, so it cannot be read out of a buffer that merely
//! contains the section. Never uses the filename.

use serde::{Deserialize, Serialize};

use super::club_record::resolve_club_record;
use super::competition_name::{
    resolve_competition_name, CompetitionNameSource, StandardCompetitionCatalogue,
};
use super::competition_table::CompetitionCatalogue;
use super::current_league::parse_current_league;
use super::human_manager::resolve_human_manager_club;
use super::registry::EntityRegistry;
use super::relationships::RelationshipGraph;
use super::types::{Evidence, ResolutionConfidence};
use crate::fm::types::FmEngineResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedClubResult {
    pub manager_id: Option<u64>,
    pub manager_name: Option<String>,

    pub club_id: Option<u64>,
    pub club_name: Option<String>,
    pub club_short_name: Option<String>,
    pub club_logo_id: Option<u64>,
    pub club_external_id: Option<String>,

    pub competition_id: Option<u64>,
    pub competition_name: Option<String>,
    pub competition_short_name: Option<String>,
    /// Where `competition_name` came from, so the UI can present a default name
    /// differently from one the save itself carries. Independent of the uid.
    pub competition_name_source: CompetitionNameSource,

    pub nation_id: Option<u64>,
    pub nation_name: Option<String>,

    pub season: Option<String>,
    pub in_game_date: Option<String>,

    pub confidence: ResolutionConfidence,
    pub manager_confidence: ResolutionConfidence,
    pub club_confidence: ResolutionConfidence,
    pub competition_confidence: ResolutionConfidence,
    pub season_confidence: ResolutionConfidence,
    pub date_confidence: ResolutionConfidence,

    pub evidence: Vec<Evidence>,
    pub relationship_summary: String,
}

impl ManagedClubResult {
    pub fn unknown(evidence: Vec<Evidence>) -> Self {
        Self {
            manager_id: None,
            manager_name: None,
            club_id: None,
            club_name: None,
            club_short_name: None,
            club_logo_id: None,
            club_external_id: None,
            competition_id: None,
            competition_name: None,
            competition_short_name: None,
            competition_name_source: CompetitionNameSource::Unavailable,
            nation_id: None,
            nation_name: None,
            season: None,
            in_game_date: None,
            confidence: ResolutionConfidence::Unknown,
            manager_confidence: ResolutionConfidence::Unknown,
            club_confidence: ResolutionConfidence::Unknown,
            competition_confidence: ResolutionConfidence::Unknown,
            season_confidence: ResolutionConfidence::Unknown,
            date_confidence: ResolutionConfidence::Unknown,
            evidence,
            relationship_summary: "Manager → Club → Competition (nicht aufgelöst)".into(),
        }
    }

    pub fn for_ui(&self) -> Self {
        let mut out = self.clone();
        if !out.manager_confidence.is_user_facing() && !out.manager_confidence.is_possible() {
            out.manager_name = None;
            out.manager_id = None;
        }
        if !out.club_confidence.is_user_facing() && !out.club_confidence.is_possible() {
            out.club_name = None;
            out.club_short_name = None;
            out.club_id = None;
        }
        if !out.competition_confidence.is_user_facing() && !out.competition_confidence.is_possible()
        {
            out.competition_name = None;
            out.competition_short_name = None;
            out.competition_id = None;
        }
        if !out.season_confidence.is_user_facing() && !out.season_confidence.is_possible() {
            out.season = None;
        }
        if !out.date_confidence.is_user_facing() && !out.date_confidence.is_possible() {
            out.in_game_date = None;
        }
        out
    }
}

/// Resolve from a decompressed `tad.` payload (manager→club short name).
pub fn resolve_managed_club_from_payload(data: &[u8]) -> ManagedClubResult {
    resolve_managed_club_from_buffers(data, &[])
}

/// Resolve using the primary payload plus optional extra decompressed frames
/// (club DB + competition link tables live in later Zstd frames).
pub fn resolve_managed_club_from_buffers(
    primary: &[u8],
    extra_frames: &[&[u8]],
) -> ManagedClubResult {
    resolve_managed_club_with_summary(primary, extra_frames, None, None)
}

/// As [`resolve_managed_club_from_buffers`], but with the exact decompressed
/// `save_game_summary` section, whose 28-byte trailer names the club's current
/// competition. `catalogue` supplies the display name when the caller has
/// already read the name table; the uid resolves with or without it.
pub fn resolve_managed_club_with_summary(
    primary: &[u8],
    extra_frames: &[&[u8]],
    summary_section: Option<&[u8]>,
    catalogue: Option<&CompetitionCatalogue>,
) -> ManagedClubResult {
    let mut evidence = Vec::new();
    evidence.push(Evidence::structural(format!(
        "Scanning {} primary + {} extra frame bytes for manager→club→competition",
        primary.len(),
        extra_frames.iter().map(|f| f.len()).sum::<usize>()
    )));

    let Some(hit) = resolve_human_manager_club(primary) else {
        evidence.push(Evidence::absent(
            "No unique 0E 01 human-manager→club pair found in payload",
        ));
        evidence.push(Evidence::absent(
            "Filename is never used for club/competition resolution",
        ));
        return ManagedClubResult::unknown(evidence);
    };
    evidence.extend(hit.evidence);

    let mut club_id = None;
    let mut club_name = Some(hit.club_name.clone());
    let mut club_short = Some(hit.club_name.clone());
    let mut club_confidence = hit.confidence;

    // Prefer club DB record (full name + numeric id) from primary or extras.
    // The uid hint from the 0E 01 record disambiguates clubs that share a name
    // (base-database vs custom-database entries).
    // Without a hint the record is adopted only when every candidate agrees on
    // one club id — see [`resolve_club_record`].
    let uid_hint = hit.club_id_hint;
    let mut sources: Vec<&[u8]> = Vec::with_capacity(1 + extra_frames.len());
    sources.push(primary);
    sources.extend_from_slice(extra_frames);
    let club_db = resolve_club_record(&sources, &hit.club_name, uid_hint);
    let mut division_tier = None;
    let mut link_id: Option<u32> = None;
    let mut secondary_id: Option<u32> = None;
    let mut nation_scoped = false;
    if let Some(rec) = club_db {
        // The `0E 01` record's uid is authoritative — it is the club the human
        // manages. A record found by name may belong to a *different* club that
        // happens to share the display name (a base-database entry alongside the
        // custom-database one), so its fields are adopted only when its id
        // agrees with the hint.
        let record_is_the_managed_club = match uid_hint {
            Some(hint) => super::club_record::matches_uid(rec.club_id, hint),
            None => true,
        };
        if record_is_the_managed_club {
            link_id = Some(rec.link_id as u32);
            secondary_id = Some(rec.secondary_id as u32);
            club_name = Some(rec.full_name);
            club_short = Some(rec.short_name);
            club_confidence = ResolutionConfidence::High;
            division_tier = rec.division_tier;
            nation_scoped = rec.nation_scoped_secondary;
            evidence.extend(rec.evidence);
        } else {
            evidence.push(Evidence::absent(format!(
                "Club record {} shares the name but not the managed uid {} — record ignored",
                rec.club_id,
                uid_hint.unwrap_or_default()
            )));
        }
        // Report the name-table uid so this agrees with `save_resolver`; the
        // compact-table id is one lower (see `club_record`).
        club_id = match uid_hint {
            Some(hint) => Some(hint as u64),
            None => Some(rec.club_id),
        };
        let _ = secondary_id; // retained for future competition-id resolution
    } else if let Some(hint) = uid_hint {
        club_id = Some(hint as u64);
        evidence.push(Evidence::structural(format!(
            "Managed club uid {hint} from the summary record; no club record in the scanned frames"
        )));
    }

    // Competition: the `save_game_summary` trailer. It is not derived from the
    // club record — the club record carries no division at any alignment (see
    // `current_league`), so nothing here is inferred from tier hints or ids that
    // happen to sit near the club.
    let current = summary_section.and_then(parse_current_league);
    let mut competition_id: Option<u64> = None;
    let mut competition_name: Option<String> = None;
    let mut competition_short: Option<String> = None;
    let mut competition_name_source = CompetitionNameSource::Unavailable;
    let mut competition_confidence = ResolutionConfidence::Unknown;

    match &current {
        Some(c) => {
            competition_id = Some(c.competition_uid as u64);
            // Confidence in the uid, decided before any name is looked up.
            competition_confidence = match catalogue {
                Some(cat) if cat.knows(c.competition_uid) => ResolutionConfidence::Confirmed,
                _ => ResolutionConfidence::High,
            };
            evidence.extend(c.evidence(summary_section.map(|s| s.len()).unwrap_or(0)));

            // Display name only, and only after the uid is settled.
            let empty = CompetitionCatalogue::default();
            let standard = StandardCompetitionCatalogue::load_default();
            let display = resolve_competition_name(
                c.competition_uid,
                catalogue.unwrap_or(&empty),
                Some(&standard),
            );
            competition_name = display.display_name.clone();
            competition_short = display
                .short_name
                .clone()
                .or_else(|| display.display_name.clone());
            competition_name_source = display.source;
            evidence.push(Evidence::structural(format!(
                "Competition {} display name {:?} from {}",
                display.competition_uid,
                display.display_name,
                display.source.as_str()
            )));
        }
        None => evidence.push(Evidence::absent(
            "No save_game_summary trailer available — competition left unresolved",
        )),
    }

    let _ = (link_id, division_tier, nation_scoped, extra_frames);

    if club_id.is_none() {
        evidence.push(Evidence::absent("Club numeric id not found"));
    }

    let uid_label;
    let comp_label = match competition_short.as_deref().or(competition_name.as_deref()) {
        Some(n) => n,
        None => match competition_id {
            // A verified uid without a display name is still a result.
            Some(id) => {
                uid_label = format!("uid {id}");
                &uid_label
            }
            None => "unknown",
        },
    };
    let relationship_summary = format!(
        "Manager('{}') → Club('{}') → Competition('{}')",
        hit.manager_name,
        club_short.as_deref().unwrap_or("?"),
        comp_label
    );

    let overall = if competition_confidence.is_user_facing() {
        ResolutionConfidence::High
    } else {
        hit.confidence
    };

    ManagedClubResult {
        manager_id: None,
        manager_name: Some(hit.manager_name),
        club_id,
        club_name,
        club_short_name: club_short,
        club_logo_id: None,
        club_external_id: None,
        competition_id,
        competition_name,
        competition_short_name: competition_short,
        competition_name_source,
        nation_id: None,
        nation_name: None,
        season: None,
        in_game_date: None,
        confidence: overall,
        manager_confidence: hit.confidence,
        club_confidence,
        competition_confidence,
        season_confidence: ResolutionConfidence::Unknown,
        date_confidence: ResolutionConfidence::Unknown,
        evidence,
        relationship_summary,
    }
    .for_ui()
}

/// Legacy path from engine index + empty registry (kept for compatibility).
pub fn resolve_managed_club(
    engine: &FmEngineResult,
    registry: &EntityRegistry,
    graph: &RelationshipGraph,
) -> ManagedClubResult {
    let mut evidence = Vec::new();
    evidence.push(Evidence::structural(format!(
        "Engine indexed {} blocks, {} structures, {} strings",
        engine.blocks.len(),
        engine.structures.len(),
        engine.string_entries
    )));

    if !registry.is_empty() {
        if let Some((manager_id, person)) = registry.persons.iter().next() {
            if let Some(club_id) = graph.managed_club_id(*manager_id) {
                if let Some(club) = registry.get_club(club_id) {
                    let competition_id = graph.competition_for_club(club_id);
                    let competition = competition_id.and_then(|id| registry.get_competition(id));
                    return ManagedClubResult {
                        manager_id: Some(*manager_id),
                        manager_name: person.display_name.clone(),
                        club_id: Some(club_id),
                        club_name: club.display_name.clone(),
                        club_short_name: club.short_name.clone(),
                        club_logo_id: None,
                        club_external_id: None,
                        competition_id,
                        competition_name: competition.and_then(|c| c.display_name.clone()),
                        competition_short_name: competition.and_then(|c| c.short_name.clone()),
                        competition_name_source: CompetitionNameSource::Unavailable,
                        nation_id: graph.nation_for_club(club_id),
                        nation_name: None,
                        season: None,
                        in_game_date: None,
                        confidence: ResolutionConfidence::High,
                        manager_confidence: person.confidence,
                        club_confidence: club.confidence,
                        competition_confidence: competition
                            .map(|c| c.confidence)
                            .unwrap_or(ResolutionConfidence::Unknown),
                        season_confidence: ResolutionConfidence::Unknown,
                        date_confidence: ResolutionConfidence::Unknown,
                        evidence,
                        relationship_summary: format!(
                            "Manager({manager_id}) → Club({club_id}) → Competition({:?})",
                            competition_id
                        ),
                    }
                    .for_ui();
                }
            }
        }
    }

    evidence.push(Evidence::absent(
        "Registry empty — prefer resolve_managed_club_from_payload on tad. bytes",
    ));
    ManagedClubResult::unknown(evidence)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_resolver_finds_unique_pair() {
        let mut data = vec![0u8; 8];
        data.extend_from_slice(&[0x0E, 0x01]);
        let person = b"Test Manager";
        data.extend_from_slice(&(person.len() as u32).to_le_bytes());
        data.extend_from_slice(person);
        let club = b"Test Club";
        data.extend_from_slice(&(club.len() as u32).to_le_bytes());
        data.extend_from_slice(club);
        let r = resolve_managed_club_from_payload(&data);
        assert_eq!(r.manager_name.as_deref(), Some("Test Manager"));
        assert_eq!(r.club_name.as_deref(), Some("Test Club"));
        assert!(matches!(
            r.club_confidence,
            ResolutionConfidence::High | ResolutionConfidence::Confirmed
        ));
        assert!(r.competition_name.is_none());
    }

    /// `0E 01` pair carrying the managed uid, plus a club record that merely
    /// shares the display name. The wrong record must not supply the club's
    /// details, and the reported id must be the managed one.
    #[test]
    fn name_collision_does_not_adopt_the_wrong_club_record() {
        fn lp(v: &mut Vec<u8>, s: &str) {
            v.extend_from_slice(&(s.len() as u32).to_le_bytes());
            v.extend_from_slice(s.as_bytes());
        }
        let mut data = vec![0u8; 8];
        data.extend_from_slice(&[0x0E, 0x01]);
        lp(&mut data, "Niclas Krieck");
        lp(&mut data, "Macclesfield");
        data.extend_from_slice(&2_000_095_665u32.to_le_bytes());
        data.extend_from_slice(&((2030u32 << 16) | 0x3c7d).to_le_bytes());

        // Base-database club record with the same short name, different uid.
        let mut frame = vec![0u8; 8];
        frame.extend_from_slice(&677u32.to_le_bytes());
        frame.extend_from_slice(&677u32.to_le_bytes());
        frame.push(0x00);
        frame.extend_from_slice(&u32::MAX.to_le_bytes());
        for _ in 0..3 {
            frame.extend_from_slice(&139u32.to_le_bytes());
        }
        frame.extend_from_slice(&0x432u32.to_le_bytes());
        frame.extend_from_slice(&[0x00, 0x00, 0x00, 0x10, 0x00, 0x00]);
        lp(&mut frame, "Macclesfield Town");
        lp(&mut frame, "Macclesfield");

        let r = resolve_managed_club_from_buffers(&data, &[&frame]);
        assert_eq!(r.club_id, Some(2_000_095_665));
        assert_eq!(r.club_name.as_deref(), Some("Macclesfield"));
        assert_ne!(r.club_name.as_deref(), Some("Macclesfield Town"));
    }

    /// Contract with the TypeScript DTO (`src/save/currentSavePersistence.ts`).
    /// The UI reads the uid, the name and the name source as three separate
    /// fields, so their JSON spelling is part of the interface.
    #[test]
    fn serialises_the_fields_the_ui_reads() {
        let mut r = ManagedClubResult::unknown(Vec::new());
        r.competition_id = Some(22);
        r.competition_name = Some("Bundesliga".into());
        r.competition_name_source = CompetitionNameSource::StandardFm26Catalogue;
        let v: serde_json::Value = serde_json::to_value(&r).expect("serialise");
        assert_eq!(v["competitionId"], 22);
        assert_eq!(v["competitionName"], "Bundesliga");
        assert_eq!(v["competitionNameSource"], "standardFm26Catalogue");

        r.competition_name = None;
        r.competition_name_source = CompetitionNameSource::Unavailable;
        let v: serde_json::Value = serde_json::to_value(&r).expect("serialise");
        // The uid survives a missing name — that is the whole point.
        assert_eq!(v["competitionId"], 22);
        assert!(v["competitionName"].is_null());
        assert_eq!(v["competitionNameSource"], "unavailable");

        for (source, expected) in [
            (CompetitionNameSource::SaveShortName, "saveShortName"),
            (CompetitionNameSource::SaveFullName, "saveFullName"),
            (CompetitionNameSource::SaveAbbreviation, "saveAbbreviation"),
        ] {
            r.competition_name_source = source;
            let v: serde_json::Value = serde_json::to_value(&r).expect("serialise");
            assert_eq!(v["competitionNameSource"], expected);
            // The Rust label and the JSON spelling must not drift apart.
            assert_eq!(source.as_str(), expected);
        }
    }

    /// The competition comes from the summary trailer, and only from there.
    #[test]
    fn competition_comes_from_the_summary_trailer() {
        fn lp(v: &mut Vec<u8>, s: &str) {
            v.extend_from_slice(&(s.len() as u32).to_le_bytes());
            v.extend_from_slice(s.as_bytes());
        }
        let mut summary = vec![0u8; 8];
        summary.extend_from_slice(&[0x0E, 0x01]);
        lp(&mut summary, "Niclas Krieck");
        lp(&mut summary, "Salford");
        summary.extend_from_slice(&5_103_842u32.to_le_bytes());
        summary.extend_from_slice(&((2025u32 << 16) | 0x1aca).to_le_bytes());
        // Trailer: date, competition 14, three opaque colours, two badge words.
        summary.extend_from_slice(&0x07E9_6ECCu32.to_le_bytes());
        summary.extend_from_slice(&14u32.to_le_bytes());
        for c in [0xFF28_08B8u32, 0xFFF8_F8F8, 0xFF00_0000] {
            summary.extend_from_slice(&c.to_le_bytes());
        }
        summary.extend_from_slice(&646u32.to_le_bytes());
        summary.extend_from_slice(&4250u32.to_le_bytes());

        let with = resolve_managed_club_with_summary(&summary, &[], Some(&summary), None);
        assert_eq!(with.competition_id, Some(14));
        assert_eq!(with.competition_confidence, ResolutionConfidence::High);

        // Without the section there is nothing to read, and nothing is invented.
        let without = resolve_managed_club_with_summary(&summary, &[], None, None);
        assert_eq!(without.competition_id, None);
        assert_eq!(
            without.competition_confidence,
            ResolutionConfidence::Unknown
        );
    }
}
