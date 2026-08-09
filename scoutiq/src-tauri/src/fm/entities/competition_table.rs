//! Competition catalogue: `competition_id → display name` (read-only).
//!
//! Competitions live in the same tagged name table as clubs (see
//! [`super::name_table`]), and the two families are told apart by the **case**
//! of the record tag, not by id:
//!
//! ```text
//! competitions   cnam / csnm / c3ln   (stored reversed: manc / mnsc / nl3c)
//! clubs          Cnam / Csnm / Ca3n   (stored reversed: manC / mnsC / n3aC)
//! ```
//!
//! Matching the tag exactly is what keeps clubs out, so no id bound is needed.
//! An earlier version filtered on `id <= 100_000` instead, on the assumption
//! that competition ids occupy a dense low range. That assumption is false:
//! ids like 11–14 (English tiers) and 22/23 (Bundesliga) do sit low, but
//! Regionalliga Bayern is 91107111 and the U19 leagues are in the 92086xxx
//! range, so the bound silently dropped every large-id competition.
//!
//! Some competitions carry only an abbreviation record (`c3ln`, e.g. Bundesliga
//! = "DBL") because the league has no sponsor name to override the default, so
//! the abbreviation is kept as a last-resort label rather than dropped.
//!
//! Nothing here is hardcoded: every id and name is read from the save, so
//! custom databases and renamed competitions resolve on their own terms.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::name_table::NameRecord;

/// Competition full-name tag as stored on disk (reversed `cnam`).
pub const TAG_COMP_FULL_NAME: [u8; 4] = *b"manc";
/// Competition short-name tag as stored on disk (reversed `csnm`).
pub const TAG_COMP_SHORT_NAME: [u8; 4] = *b"mnsc";
/// Three-letter abbreviation tag as stored on disk (reversed `c3ln`).
pub const TAG_ABBREVIATION: [u8; 4] = *b"nl3c";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompetitionCatalogue {
    /// `id → best available display name` (short name preferred).
    pub names: HashMap<u32, String>,
    /// `id → three-letter abbreviation`, when present.
    pub abbreviations: HashMap<u32, String>,
    /// `id → short name` exactly as recorded (`csnm`).
    #[serde(default)]
    pub short_names: HashMap<u32, String>,
    /// `id → full name` exactly as recorded (`cnam`).
    #[serde(default)]
    pub full_names: HashMap<u32, String>,
}

impl CompetitionCatalogue {
    /// Display name for `id`, falling back to the abbreviation.
    pub fn name(&self, id: u32) -> Option<&str> {
        self.names
            .get(&id)
            .or_else(|| self.abbreviations.get(&id))
            .map(|s| s.as_str())
    }

    /// Short name exactly as this save records it, if any.
    pub fn short_name(&self, id: u32) -> Option<&str> {
        self.short_names.get(&id).map(|s| s.as_str())
    }

    /// Full name exactly as this save records it, if any.
    pub fn full_name(&self, id: u32) -> Option<&str> {
        self.full_names.get(&id).map(|s| s.as_str())
    }

    pub fn abbreviation(&self, id: u32) -> Option<&str> {
        self.abbreviations.get(&id).map(|s| s.as_str())
    }

    /// Whether this save's own name table mentions `id` at all — including an
    /// abbreviation-only record. Used to cross-check a competition uid; it is
    /// deliberately *not* the same question as "can we display a name".
    pub fn knows(&self, id: u32) -> bool {
        self.names.contains_key(&id) || self.abbreviations.contains_key(&id)
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

/// Build a catalogue from every competition-family name record in the save.
///
/// Tags are matched case-sensitively, which is what separates competitions from
/// clubs; there is deliberately no id bound (see the module docs).
///
/// Short names win over full names: FM shows "EFL League Two", not the longer
/// legal title. Later duplicates never overwrite an established short name.
pub fn build_catalogue(records: &[NameRecord]) -> CompetitionCatalogue {
    let mut cat = CompetitionCatalogue::default();
    let mut from_short: HashMap<u32, bool> = HashMap::new();

    for r in records {
        if r.tag == TAG_ABBREVIATION {
            cat.abbreviations
                .entry(r.id)
                .or_insert_with(|| r.name.clone());
        } else if r.tag == TAG_COMP_SHORT_NAME {
            cat.short_names
                .entry(r.id)
                .or_insert_with(|| r.name.clone());
            if !from_short.get(&r.id).copied().unwrap_or(false) {
                cat.names.insert(r.id, r.name.clone());
                from_short.insert(r.id, true);
            }
        } else if r.tag == TAG_COMP_FULL_NAME {
            cat.full_names.entry(r.id).or_insert_with(|| r.name.clone());
            cat.names.entry(r.id).or_insert_with(|| r.name.clone());
            from_short.entry(r.id).or_insert(false);
        }
    }
    cat
}

#[cfg(test)]
mod tests {
    use super::super::name_table::{TAG_FULL_NAME, TAG_SHORT_NAME};
    use super::*;

    fn rec(id: u32, tag: [u8; 4], name: &str) -> NameRecord {
        NameRecord {
            id,
            tag,
            name: name.to_string(),
            offset: id as u64,
        }
    }

    #[test]
    fn short_name_wins_over_full_name() {
        let recs = vec![
            rec(20, TAG_COMP_FULL_NAME, "Championnat Divisions Inférieures"),
            rec(20, TAG_COMP_SHORT_NAME, "Championnat Inférieures"),
        ];
        let cat = build_catalogue(&recs);
        assert_eq!(cat.name(20), Some("Championnat Inférieures"));
    }

    #[test]
    fn abbreviation_used_only_when_no_name_exists() {
        // Bundesliga records only its abbreviation in the Leverkusen save.
        let recs = vec![
            rec(22, TAG_ABBREVIATION, "DBL"),
            rec(23, TAG_COMP_SHORT_NAME, "2. Bundesliga"),
            rec(23, TAG_ABBREVIATION, "2BL"),
        ];
        let cat = build_catalogue(&recs);
        assert_eq!(cat.name(22), Some("DBL"));
        assert_eq!(cat.name(23), Some("2. Bundesliga"));
    }

    #[test]
    fn club_records_are_excluded_by_tag_case() {
        // Club tags differ only in case; they must not enter the catalogue.
        let recs = vec![
            rec(14, TAG_COMP_SHORT_NAME, "EFL League Two"),
            rec(901, TAG_SHORT_NAME, "Bayer Leverkusen"),
            rec(901, TAG_FULL_NAME, "Bayer 04 Leverkusen Fußball GmbH"),
        ];
        let cat = build_catalogue(&recs);
        assert_eq!(cat.name(14), Some("EFL League Two"));
        assert_eq!(cat.name(901), None);
    }

    /// The split short/full maps must obey the same case rule as `names`, or a
    /// club would leak into the name resolver through the back door.
    #[test]
    fn split_name_maps_also_exclude_club_records_by_tag_case() {
        let recs = vec![
            rec(14, TAG_COMP_SHORT_NAME, "EFL League Two"),
            rec(14, TAG_COMP_FULL_NAME, "Sky Bet League Two"),
            rec(901, TAG_SHORT_NAME, "Bayer Leverkusen"),
            rec(901, TAG_FULL_NAME, "Bayer 04 Leverkusen Fußball GmbH"),
        ];
        let cat = build_catalogue(&recs);
        assert_eq!(cat.short_name(14), Some("EFL League Two"));
        assert_eq!(cat.full_name(14), Some("Sky Bet League Two"));
        assert_eq!(cat.short_name(901), None);
        assert_eq!(cat.full_name(901), None);
        assert!(cat.knows(14));
        assert!(!cat.knows(901));
    }

    /// `knows` is about the uid existing in the save, not about a display name:
    /// an abbreviation-only competition still counts.
    #[test]
    fn knows_accepts_abbreviation_only_competitions() {
        let cat = build_catalogue(&[rec(22, TAG_ABBREVIATION, "DBL")]);
        assert!(cat.knows(22));
        assert_eq!(cat.short_name(22), None);
        assert_eq!(cat.full_name(22), None);
        assert_eq!(cat.abbreviation(22), Some("DBL"));
    }

    #[test]
    fn large_competition_ids_are_kept() {
        // Regionalliga Bayern is 91107111 — the old `id <= 100_000` bound
        // dropped it, and with it every club managed in such a division.
        let recs = vec![
            rec(91_107_111, TAG_COMP_SHORT_NAME, "Regionalliga Bayern"),
            rec(
                92_086_573,
                TAG_COMP_SHORT_NAME,
                "Bundesliga U19 Süd/Südwest",
            ),
        ];
        let cat = build_catalogue(&recs);
        assert_eq!(cat.name(91_107_111), Some("Regionalliga Bayern"));
        assert_eq!(cat.name(92_086_573), Some("Bundesliga U19 Süd/Südwest"));
    }
}
