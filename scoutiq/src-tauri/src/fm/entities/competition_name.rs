//! Competition **display name** resolution — strictly downstream of the uid.
//!
//! The current competition *uid* comes from the `save_game_summary` trailer (see
//! [`super::current_league`]) and is never touched here. This module only tries
//! to put a readable label on a uid that is already resolved, and every failure
//! mode ends in "no name", never in "no competition".
//!
//! ```text
//! Step 1  save_game_summary trailer  → competition uid      (current_league)
//! Step 2  competition uid            → optional display name (this module)
//! ```
//!
//! ## Priority
//!
//! ```text
//! 1. Save short name       csnm   the save's own label, sponsor names included
//! 2. Save full name        cnam
//! 3. Standard catalogue           FM's default name for that uid
//! 4. Save abbreviation     c3ln   e.g. "DBL"
//! 5. Unavailable                  caller shows the uid
//! ```
//!
//! Save-embedded names come first so a custom database or a renamed/sponsored
//! competition keeps its own label — the standard catalogue is a fallback for
//! uids the save does not name, never an override.
//!
//! ## Why a standard catalogue is needed at all
//!
//! A save stores a competition name only when it differs from the installed
//! database's default. Checked across the Leverkusen, MSV Duisburg and TSV 1860
//! saves, uid 22 carries **only** `c3ln = "DBL"` — no `cnam`, no `csnm` — so
//! "Bundesliga" is simply not in those files. Conversely uid 30 records
//! `csnm = "Eerste Divisie"` in every modern save, which is what lets an older
//! save that cannot parse its own name table still be labelled.
//!
//! ## What the catalogue may and may not do
//!
//! It maps `competition uid → default name`. It says nothing about which league
//! a club is in — that is decided per save by the trailer — so it cannot
//! reintroduce a club-to-league rule. It is optional: a missing or malformed
//! file yields an empty catalogue and parsing continues unchanged.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::competition_table::CompetitionCatalogue;
use super::types::ResolutionConfidence;

/// Environment override for the standard catalogue file (used by tests and by
/// deployments that ship the catalogue elsewhere).
pub const CATALOGUE_PATH_ENV: &str = "SCOUTIQ_COMPETITION_NAMES";
/// Path of the catalogue relative to the executable or the crate root.
pub const CATALOGUE_RELATIVE_PATH: &str = "resources/competition_names/fm26.json";
/// Locale used when a catalogue entry carries per-language variants.
pub const DEFAULT_LOCALE: &str = "en";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CompetitionNameSource {
    SaveShortName,
    SaveFullName,
    StandardFm26Catalogue,
    SaveAbbreviation,
    Unavailable,
}

impl CompetitionNameSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SaveShortName => "saveShortName",
            Self::SaveFullName => "saveFullName",
            Self::StandardFm26Catalogue => "standardFm26Catalogue",
            Self::SaveAbbreviation => "saveAbbreviation",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompetitionDisplayName {
    pub competition_uid: u32,
    /// Best label available, or `None` when nothing could be resolved.
    pub display_name: Option<String>,
    /// Short name as recorded by the save, if any.
    pub short_name: Option<String>,
    /// Abbreviation as recorded by the save, if any.
    pub abbreviation: Option<String>,
    pub source: CompetitionNameSource,
    /// Confidence in the **name**. The uid's own confidence is separate and is
    /// never affected by this value.
    pub confidence: ResolutionConfidence,
}

impl CompetitionDisplayName {
    /// Label safe to show in a UI: the resolved name, else `Competition <uid>`.
    pub fn label(&self) -> String {
        match &self.display_name {
            Some(n) => n.clone(),
            None => format!("Competition {}", self.competition_uid),
        }
    }
}

/// `competition uid → default display name`, loaded from JSON.
///
/// Two entry shapes are accepted so a client-provided export loads either way:
///
/// ```json
/// { "competitions": { "22": "Bundesliga" } }
/// { "competitions": { "22": { "en": "Bundesliga", "de": "Bundesliga" } } }
/// ```
#[derive(Debug, Clone, Default)]
pub struct StandardCompetitionCatalogue {
    names: HashMap<u32, String>,
    pub database_version: Option<String>,
    pub source_path: Option<PathBuf>,
    /// Set when a file was found but could not be used.
    pub load_error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawEntry {
    Plain(String),
    Localised(HashMap<String, String>),
}

#[derive(Debug, Deserialize)]
struct RawCatalogue {
    #[serde(default, rename = "databaseVersion")]
    database_version: Option<String>,
    #[serde(default)]
    competitions: HashMap<String, RawEntry>,
}

impl StandardCompetitionCatalogue {
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn name(&self, uid: u32) -> Option<&str> {
        self.names.get(&uid).map(|s| s.as_str())
    }

    /// Parse catalogue JSON. Malformed input yields an empty catalogue carrying
    /// `load_error` — it is never an error the caller has to handle, because a
    /// broken catalogue must not stop a save from parsing.
    pub fn from_json(bytes: &[u8], locale: &str) -> StandardCompetitionCatalogue {
        let raw: RawCatalogue = match serde_json::from_slice(bytes) {
            Ok(r) => r,
            Err(e) => {
                return StandardCompetitionCatalogue {
                    load_error: Some(format!("competition catalogue is not valid JSON: {e}")),
                    ..Default::default()
                }
            }
        };
        let mut names = HashMap::new();
        for (k, v) in raw.competitions {
            let Ok(uid) = k.parse::<u32>() else { continue };
            let picked = match v {
                RawEntry::Plain(s) => Some(s),
                RawEntry::Localised(m) => m
                    .get(locale)
                    .or_else(|| m.get(DEFAULT_LOCALE))
                    .or_else(|| m.values().next())
                    .cloned(),
            };
            if let Some(n) = picked.filter(|n| !n.trim().is_empty()) {
                names.insert(uid, n);
            }
        }
        StandardCompetitionCatalogue {
            names,
            database_version: raw.database_version,
            source_path: None,
            load_error: None,
        }
    }

    pub fn load_from(path: &Path, locale: &str) -> StandardCompetitionCatalogue {
        match std::fs::read(path) {
            Ok(bytes) => {
                let mut c = Self::from_json(&bytes, locale);
                c.source_path = Some(path.to_path_buf());
                c
            }
            Err(e) => StandardCompetitionCatalogue {
                load_error: Some(format!("cannot read {}: {e}", path.display())),
                ..Default::default()
            },
        }
    }

    /// Load from the first candidate path that exists, or return empty.
    ///
    /// Absence is normal, not an error: the parser is expected to run without a
    /// catalogue at all. An explicit `SCOUTIQ_COMPETITION_NAMES` override is
    /// authoritative — if it is set, no bundled file is silently substituted,
    /// so "run without a catalogue" is something a caller can actually ask for.
    pub fn load_default() -> StandardCompetitionCatalogue {
        if let Ok(explicit) = std::env::var(CATALOGUE_PATH_ENV) {
            let trimmed = explicit.trim();
            if !trimmed.is_empty() {
                return Self::load_from(Path::new(trimmed), DEFAULT_LOCALE);
            }
        }
        for path in candidate_paths() {
            if path.is_file() {
                return Self::load_from(&path, DEFAULT_LOCALE);
            }
        }
        StandardCompetitionCatalogue::default()
    }
}

fn candidate_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        // Next to the binary, and one and two levels up (target/release/… in dev,
        // Resources/ next to the bundled app in a Tauri build).
        let mut dir = exe.parent().map(|p| p.to_path_buf());
        for _ in 0..3 {
            let Some(d) = dir else { break };
            out.push(d.join(CATALOGUE_RELATIVE_PATH));
            dir = d.parent().map(|p| p.to_path_buf());
        }
    }
    out.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(CATALOGUE_RELATIVE_PATH));
    out
}

/// Resolve a display name for an already-resolved competition uid.
///
/// Never fails and never reports anything about the uid's own validity.
pub fn resolve_competition_name(
    uid: u32,
    save: &CompetitionCatalogue,
    standard: Option<&StandardCompetitionCatalogue>,
) -> CompetitionDisplayName {
    let short_name = save.short_name(uid).map(str::to_string);
    let abbreviation = save.abbreviation(uid).map(str::to_string);

    let (display_name, source, confidence) = if let Some(n) = short_name.clone() {
        (
            Some(n),
            CompetitionNameSource::SaveShortName,
            ResolutionConfidence::Confirmed,
        )
    } else if let Some(n) = save.full_name(uid) {
        (
            Some(n.to_string()),
            CompetitionNameSource::SaveFullName,
            ResolutionConfidence::Confirmed,
        )
    } else if let Some(n) = standard.and_then(|s| s.name(uid)) {
        (
            Some(n.to_string()),
            CompetitionNameSource::StandardFm26Catalogue,
            ResolutionConfidence::High,
        )
    } else if let Some(n) = abbreviation.clone() {
        (
            Some(n),
            CompetitionNameSource::SaveAbbreviation,
            ResolutionConfidence::Medium,
        )
    } else {
        (
            None,
            CompetitionNameSource::Unavailable,
            ResolutionConfidence::Unknown,
        )
    };

    CompetitionDisplayName {
        competition_uid: uid,
        display_name,
        short_name,
        abbreviation,
        source,
        confidence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn save_catalogue(
        short: &[(u32, &str)],
        full: &[(u32, &str)],
        abbrev: &[(u32, &str)],
    ) -> CompetitionCatalogue {
        let mut c = CompetitionCatalogue::default();
        for (id, n) in short {
            c.short_names.insert(*id, n.to_string());
            c.names.insert(*id, n.to_string());
        }
        for (id, n) in full {
            c.full_names.insert(*id, n.to_string());
            c.names.entry(*id).or_insert_with(|| n.to_string());
        }
        for (id, n) in abbrev {
            c.abbreviations.insert(*id, n.to_string());
        }
        c
    }

    fn standard(pairs: &[(u32, &str)]) -> StandardCompetitionCatalogue {
        let mut names = HashMap::new();
        for (id, n) in pairs {
            names.insert(*id, n.to_string());
        }
        StandardCompetitionCatalogue {
            names,
            ..Default::default()
        }
    }

    #[test]
    fn embedded_short_name_wins_over_everything() {
        let save = save_catalogue(
            &[(14, "EFL League Two")],
            &[(14, "Sky Bet League Two")],
            &[(14, "EL2")],
        );
        let std = standard(&[(14, "Standard League Two")]);
        let r = resolve_competition_name(14, &save, Some(&std));
        assert_eq!(r.display_name.as_deref(), Some("EFL League Two"));
        assert_eq!(r.source, CompetitionNameSource::SaveShortName);
    }

    #[test]
    fn embedded_full_name_wins_over_the_standard_catalogue() {
        let save = save_catalogue(&[], &[(23, "2. Bundesliga")], &[(23, "2BL")]);
        let std = standard(&[(23, "Second Division")]);
        let r = resolve_competition_name(23, &save, Some(&std));
        assert_eq!(r.display_name.as_deref(), Some("2. Bundesliga"));
        assert_eq!(r.source, CompetitionNameSource::SaveFullName);
    }

    /// A renamed competition in a custom database must keep its own name.
    #[test]
    fn custom_database_name_overrides_the_standard_catalogue() {
        let save = save_catalogue(&[(12, "Krieck Super League")], &[], &[]);
        let std = standard(&[(12, "EFL Championship")]);
        let r = resolve_competition_name(12, &save, Some(&std));
        assert_eq!(r.display_name.as_deref(), Some("Krieck Super League"));
        assert_eq!(r.source, CompetitionNameSource::SaveShortName);
    }

    #[test]
    fn standard_catalogue_fills_a_uid_the_save_does_not_name() {
        // uid 30 is unnamed in the older VVV-Venlo save's unparseable name table.
        let save = save_catalogue(&[], &[], &[]);
        let std = standard(&[(30, "Eerste Divisie")]);
        let r = resolve_competition_name(30, &save, Some(&std));
        assert_eq!(r.display_name.as_deref(), Some("Eerste Divisie"));
        assert_eq!(r.source, CompetitionNameSource::StandardFm26Catalogue);
        assert_eq!(r.confidence, ResolutionConfidence::High);
    }

    /// Without catalogue data for uid 22 the abbreviation is the safe fallback.
    #[test]
    fn abbreviation_is_used_when_nothing_better_exists() {
        let save = save_catalogue(&[], &[], &[(22, "DBL")]);
        let r = resolve_competition_name(22, &save, None);
        assert_eq!(r.display_name.as_deref(), Some("DBL"));
        assert_eq!(r.source, CompetitionNameSource::SaveAbbreviation);

        // …and the catalogue takes precedence over it once it knows the uid.
        let std = standard(&[(22, "Bundesliga")]);
        let r = resolve_competition_name(22, &save, Some(&std));
        assert_eq!(r.display_name.as_deref(), Some("Bundesliga"));
        assert_eq!(r.source, CompetitionNameSource::StandardFm26Catalogue);
        assert_eq!(r.abbreviation.as_deref(), Some("DBL"));
    }

    /// Without catalogue data the uid is still returned — only the name is lost.
    #[test]
    fn standard_names_appear_only_when_catalogue_data_exists() {
        let save = save_catalogue(&[], &[], &[]);
        let none = resolve_competition_name(30, &save, None);
        assert_eq!(none.competition_uid, 30);
        assert!(none.display_name.is_none());
        assert_eq!(none.source, CompetitionNameSource::Unavailable);
        assert_eq!(none.label(), "Competition 30");

        let empty = StandardCompetitionCatalogue::default();
        let still_none = resolve_competition_name(30, &save, Some(&empty));
        assert_eq!(still_none.source, CompetitionNameSource::Unavailable);
        assert_eq!(still_none.competition_uid, 30);
    }

    #[test]
    fn unknown_uid_stays_valid_with_a_safe_label() {
        let r = resolve_competition_name(987_654, &CompetitionCatalogue::default(), None);
        assert_eq!(r.competition_uid, 987_654);
        assert!(r.display_name.is_none());
        assert_eq!(r.source, CompetitionNameSource::Unavailable);
        assert_eq!(r.label(), "Competition 987654");
    }

    #[test]
    fn malformed_catalogue_loads_as_empty_and_reports_why() {
        let c = StandardCompetitionCatalogue::from_json(b"{not json", DEFAULT_LOCALE);
        assert!(c.is_empty());
        assert!(c.load_error.is_some());
        // Resolution still works, it just falls through to the abbreviation.
        let save = save_catalogue(&[], &[], &[(22, "DBL")]);
        let r = resolve_competition_name(22, &save, Some(&c));
        assert_eq!(r.display_name.as_deref(), Some("DBL"));
    }

    #[test]
    fn missing_catalogue_file_is_not_an_error() {
        let c = StandardCompetitionCatalogue::load_from(
            Path::new("/nonexistent/competition_names.json"),
            DEFAULT_LOCALE,
        );
        assert!(c.is_empty());
        assert!(c.load_error.is_some());
    }

    #[test]
    fn accepts_plain_and_localised_entry_shapes() {
        let json = br#"{
            "databaseVersion": "FM26",
            "competitions": {
                "22": "Bundesliga",
                "30": { "en": "Eerste Divisie", "nl": "Eerste Divisie" },
                "notanumber": "ignored",
                "99": { "de": "Nur Deutsch" }
            }
        }"#;
        let c = StandardCompetitionCatalogue::from_json(json, DEFAULT_LOCALE);
        assert_eq!(c.database_version.as_deref(), Some("FM26"));
        assert_eq!(c.name(22), Some("Bundesliga"));
        assert_eq!(c.name(30), Some("Eerste Divisie"));
        // No `en` variant: fall back to whatever the entry does carry.
        assert_eq!(c.name(99), Some("Nur Deutsch"));
        assert_eq!(c.len(), 3);
    }

    #[test]
    fn locale_selection_prefers_the_requested_language() {
        let json = br#"{"competitions":{"22":{"en":"Bundesliga","de":"Fussball-Bundesliga"}}}"#;
        let de = StandardCompetitionCatalogue::from_json(json, "de");
        assert_eq!(de.name(22), Some("Fussball-Bundesliga"));
        let en = StandardCompetitionCatalogue::from_json(json, "en");
        assert_eq!(en.name(22), Some("Bundesliga"));
    }
}
