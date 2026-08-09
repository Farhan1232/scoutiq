//! Current primary league, from the `save_game_summary` trailer.
//!
//! `save_game_summary` is the section FM renders on its own load screen, and it
//! ends with a fixed 28-byte record describing the badge that screen draws next
//! to the club: the club's **current competition** and that competition's three
//! display colours.
//!
//! ```text
//! …LP(player name)… <u32 count> <count bytes>   // squad shortlist, variable
//! ┌─ trailer, always the last 28 bytes ─────────────────────────────────────┐
//! │ u32 packed_date      next-fixture date, or the 1/1900 null date         │
//! │ u32 competition_uid  the club's current primary competition             │
//! │ u32 colour_primary   RGBA, alpha 0xFF                                   │
//! │ u32 colour_secondary RGBA, alpha 0xFF                                   │
//! │ u32 colour_tertiary  RGBA, alpha 0xFF                                   │
//! │ u32 badge_a, u32 badge_b                                                │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! **Packed date.** The low 9 bits are the day of the year and the high 7 bits a
//! quarter-hour slot, with the year in the high u16. The 1/1900 null date means
//! no fixture is scheduled — which is exactly what a save taken between seasons
//! shows. Decoding it is what lets the resolver report *when* a competition
//! reading applies, and it is validated against the same year range as the
//! summary's own in-game date.
//!
//! **Why this and not `game_db`.** The competition is not stored on the club:
//! walking the compact club record for Leverkusen, Aachen and Nürnberg finds no
//! occurrence of their league ids (22, 35010926, 23) at any alignment. The
//! counted `01 <kind> <uid> <uid> 00 00` array inside a club record is a list of
//! *regional* competitions — 1. FC Nürnberg and the amateur side FC Tegernheim
//! carry byte-identical arrays — so it identifies a club's region, never its
//! division. See the research notes in `fm26-research/FINDINGS.md`.
//!
//! Nothing here is keyed on a club, nation, tier or filename. Promotions and
//! relegations need no special handling: the value is whatever FM recorded at
//! the moment the save was written.

use serde::{Deserialize, Serialize};

use super::types::{Evidence, EvidenceKind};

/// Size of the fixed trailer at the end of `save_game_summary`.
const TRAILER_LEN: usize = 28;
/// Plausible in-game years; FM26 starts in 2025 and careers can run decades.
const YEAR_MIN: u16 = 2020;
const YEAR_MAX: u16 = 2100;
/// FM's "no date" sentinel year (the 1/1/1900 null date).
const NULL_YEAR: u16 = 1900;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackedDate {
    pub day_of_year: u16,
    pub year: u16,
    /// Quarter-hour slot within the day (0–127).
    pub slot: u16,
}

impl PackedDate {
    fn decode(v: u32) -> PackedDate {
        let low = (v & 0xFFFF) as u16;
        PackedDate {
            day_of_year: low & 0x01FF,
            slot: low >> 9,
            year: (v >> 16) as u16,
        }
    }

    /// The 1/1/1900 sentinel FM writes when nothing is scheduled.
    pub fn is_null(self) -> bool {
        self.year == NULL_YEAR
    }

    fn plausible(self) -> bool {
        self.is_null() || ((YEAR_MIN..=YEAR_MAX).contains(&self.year) && self.day_of_year >= 1)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentLeague {
    pub competition_uid: u32,
    /// Next-fixture date, or the null date between seasons.
    pub fixture_date: PackedDate,
    /// The competition's three RGBA display colours.
    pub colours: [u32; 3],
    /// Offset of the competition uid inside the decompressed summary section.
    pub uid_offset: u64,
    /// Offset of the trailer itself.
    pub trailer_offset: u64,
}

impl CurrentLeague {
    pub fn evidence(&self, section_len: usize) -> Vec<Evidence> {
        vec![
            Evidence {
                kind: EvidenceKind::StructuralRecord,
                detail: format!(
                    "save_game_summary trailer ({TRAILER_LEN} bytes at {}..{section_len}): \
                     date + competition uid + 3 RGBA colours + 2 badge words",
                    self.trailer_offset
                ),
                block_index: None,
                offset: Some(self.trailer_offset),
            },
            Evidence {
                kind: EvidenceKind::RelationshipLink,
                detail: format!(
                    "Managed club → current competition uid {} (colours {:#010x}/{:#010x}/{:#010x})",
                    self.competition_uid, self.colours[0], self.colours[1], self.colours[2]
                ),
                block_index: None,
                offset: Some(self.uid_offset),
            },
            Evidence {
                kind: EvidenceKind::HeaderField,
                detail: if self.fixture_date.is_null() {
                    "Next-fixture date is the 1/1900 null date — save sits between fixtures".into()
                } else {
                    format!(
                        "Next-fixture date: day {} of {} (slot {})",
                        self.fixture_date.day_of_year,
                        self.fixture_date.year,
                        self.fixture_date.slot
                    )
                },
                block_index: None,
                offset: Some(self.trailer_offset),
            },
        ]
    }
}

fn rd_u32(d: &[u8], at: usize) -> Option<u32> {
    d.get(at..at + 4)
        .map(|s| u32::from_le_bytes(s.try_into().unwrap()))
}

/// Parse the trailer of a decompressed `save_game_summary` payload.
///
/// Returns `None` unless the whole 28-byte shape checks out: three colour words
/// whose alpha byte is `0xFF`, a plausible packed date, and a non-sentinel
/// competition uid. A partial match is rejected rather than reported, so a
/// layout change in a future FM build surfaces as "unresolved" and never as a
/// wrong league.
pub fn parse_current_league(data: &[u8]) -> Option<CurrentLeague> {
    let len = data.len();
    let at = len.checked_sub(TRAILER_LEN)?;

    let fixture_date = PackedDate::decode(rd_u32(data, at)?);
    if !fixture_date.plausible() {
        return None;
    }

    let competition_uid = rd_u32(data, at + 4)?;
    if competition_uid == 0 || competition_uid == u32::MAX {
        return None;
    }

    let colours = [
        rd_u32(data, at + 8)?,
        rd_u32(data, at + 12)?,
        rd_u32(data, at + 16)?,
    ];
    // Alpha is the high byte of each RGBA word and is opaque in every save seen.
    if colours.iter().any(|c| (c >> 24) != 0xFF) {
        return None;
    }

    Some(CurrentLeague {
        competition_uid,
        fixture_date,
        colours,
        uid_offset: (at + 4) as u64,
        trailer_offset: at as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a summary payload whose trailer carries `uid`.
    fn with_trailer(prefix: &[u8], date: u32, uid: u32, colours: [u32; 3]) -> Vec<u8> {
        let mut v = prefix.to_vec();
        v.extend_from_slice(&date.to_le_bytes());
        v.extend_from_slice(&uid.to_le_bytes());
        for c in colours {
            v.extend_from_slice(&c.to_le_bytes());
        }
        v.extend_from_slice(&686u32.to_le_bytes());
        v.extend_from_slice(&4122u32.to_le_bytes());
        v
    }

    const OPAQUE: [u32; 3] = [0xFF88_8888, 0xFFF8_F8F8, 0xFF00_0000];

    /// Byte-for-byte tails taken from the calibration saves.
    #[test]
    fn reads_the_competition_uid_from_real_trailers() {
        // Macclesfield before promotion: day 125 of 2030, EFL League One (13).
        let d = with_trailer(b"squad", 0x07EE_3C7D, 13, OPAQUE);
        let c = parse_current_league(&d).expect("trailer");
        assert_eq!(c.competition_uid, 13);
        assert_eq!(c.fixture_date.year, 2030);
        assert_eq!(c.fixture_date.day_of_year, 125);

        // Same club after the season rollover: EFL Championship (12).
        let d = with_trailer(
            b"squad",
            0x07EE_4AC9,
            12,
            [0xFF40_98B0, 0xFFF8_F8F8, 0xFF00_0000],
        );
        assert_eq!(parse_current_league(&d).unwrap().competition_uid, 12);

        // TSV 1860 München: a competition uid far above the old small-id range.
        let d = with_trailer(
            b"squad",
            0x07EA_42F8,
            91_107_111,
            [0xFF50_B8E8, 0xFFF8_F8F8, 0xFF50_B8E8],
        );
        assert_eq!(
            parse_current_league(&d).unwrap().competition_uid,
            91_107_111
        );
    }

    /// Between seasons FM writes the 1/1900 null date; the league still resolves.
    #[test]
    fn null_fixture_date_still_yields_a_competition() {
        let d = with_trailer(b"squad", 0x076C_0001, 13, OPAQUE);
        let c = parse_current_league(&d).expect("trailer");
        assert_eq!(c.competition_uid, 13);
        assert!(c.fixture_date.is_null());
    }

    #[test]
    fn rejects_a_trailer_whose_colours_are_not_opaque() {
        let d = with_trailer(
            b"squad",
            0x07EE_3C7D,
            13,
            [0x0088_8888, 0xFFF8_F8F8, 0xFF00_0000],
        );
        assert!(parse_current_league(&d).is_none());
    }

    #[test]
    fn rejects_an_implausible_date() {
        let d = with_trailer(b"squad", 0x0001_3C7D, 13, OPAQUE);
        assert!(parse_current_league(&d).is_none());
    }

    #[test]
    fn rejects_sentinel_and_zero_competition_ids() {
        assert!(parse_current_league(&with_trailer(b"squad", 0x07EE_3C7D, 0, OPAQUE)).is_none());
        assert!(
            parse_current_league(&with_trailer(b"squad", 0x07EE_3C7D, u32::MAX, OPAQUE)).is_none()
        );
    }

    #[test]
    fn short_payload_is_not_a_trailer() {
        assert!(parse_current_league(&[0u8; 12]).is_none());
    }

    #[test]
    fn decodes_the_packed_date_layout() {
        // Null date: day 1, year 1900, slot 0.
        let n = PackedDate::decode(0x076C_0001);
        assert_eq!((n.day_of_year, n.year, n.slot), (1, 1900, 0));
        // Leverkusen: day 223 of 2025 (11 August), slot 13.
        let l = PackedDate::decode(0x07E9_1ADF);
        assert_eq!((l.day_of_year, l.year, l.slot), (223, 2025, 13));
    }
}
