//! Whether a remote position can be used for a QSO (feature 010, research R4).
//!
//! A rejection is a verdict, not an error: it is a plain enum, not a `Result`
//! (`docs/rust-guidelines.md`).

use crate::entity_extents::ExtentTable;
use crate::position::Position;

/// The decision on one position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Accept,
    Reject(RejectReason),
}

/// Why a position was not used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    /// Outside the area of the callsign's entity (nested entities excluded).
    OutsideEntity,
    /// 0°/0°, or locator square `AA00` outside Antarctica.
    Placeholder,
}

impl RejectReason {
    pub fn as_str(self) -> &'static str {
        match self {
            RejectReason::OutsideEntity => "outside-entity",
            RejectReason::Placeholder => "placeholder",
        }
    }
}

/// The cty.dat primary prefix of Antarctica, where locator square `AA00` (the South Pole) can
/// be a real position.
const ANTARCTICA: &str = "CE9";

/// Judges one position for a QSO whose callsign belongs to `entity` (a cty.dat primary prefix)
/// (research R4), in this order:
/// 1. exactly 0°/0° is a placeholder, from any source;
/// 2. a position inside locator square `AA00` is a placeholder, except for Antarctica;
/// 3. for online providers (`online`), a position outside the entity's area, nested entities
///    excluded, is rejected. The logger's and the country data's positions are trusted
///    (FR-012), and an unknown entity or area accepts the position.
pub fn check(
    position: &Position,
    entity: Option<&str>,
    online: bool,
    areas: &ExtentTable,
) -> Verdict {
    if position.lat == 0.0 && position.lon == 0.0 {
        return Verdict::Reject(RejectReason::Placeholder);
    }
    let in_aa00 =
        (-90.0..-89.0).contains(&position.lat) && (-180.0..-178.0).contains(&position.lon);
    if in_aa00 && entity != Some(ANTARCTICA) {
        return Verdict::Reject(RejectReason::Placeholder);
    }
    if online
        && let Some(prefix) = entity
        && areas.inside(prefix, position.lat, position.lon) == Some(false)
    {
        return Verdict::Reject(RejectReason::OutsideEntity);
    }
    Verdict::Accept
}

/// The value of `APP_LIVEQSOMAP_POS_REJECTED` (feature 010, FR-013, research R7): one
/// `source[@base]:reason` entry per rejection, comma-separated, in the given order. `None` when
/// nothing was rejected. Each entry is (source name, found through the base callsign, reason).
pub fn rejection_note(entries: &[(&str, bool, RejectReason)]) -> Option<String> {
    if entries.is_empty() {
        return None;
    }
    let parts: Vec<String> = entries
        .iter()
        .map(|(source, base, reason)| {
            let round = if *base { "@base" } else { "" };
            format!("{source}{round}:{}", reason.as_str())
        })
        .collect();
    Some(parts.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::{PositionSource, Precision};

    fn at(lat: f64, lon: f64) -> Position {
        Position::new(lat, lon, Precision::Locator6, PositionSource::Qrz).unwrap()
    }

    fn areas() -> ExtentTable {
        ExtentTable::embedded().unwrap()
    }

    const CAGLIARI: (f64, f64) = (39.22, 9.12);
    const ROME: (f64, f64) = (41.9, 12.5);
    const BERLIN: (f64, f64) = (52.52, 13.4);

    #[test]
    fn online_positions_must_lie_inside_the_entity() {
        let areas = areas();
        let check_at =
            |(lat, lon): (f64, f64), entity| check(&at(lat, lon), Some(entity), true, &areas);
        assert_eq!(check_at(CAGLIARI, "IS"), Verdict::Accept);
        assert_eq!(
            check_at(CAGLIARI, "I"),
            Verdict::Reject(RejectReason::OutsideEntity),
            "Cagliari is Sardinia, nested in Italy"
        );
        assert_eq!(
            check_at(BERLIN, "IS"),
            Verdict::Reject(RejectReason::OutsideEntity)
        );
        assert_eq!(check_at(ROME, "I"), Verdict::Accept);
    }

    #[test]
    fn logger_and_cty_positions_are_not_checked_against_the_area() {
        let areas = areas();
        assert_eq!(
            check(&at(BERLIN.0, BERLIN.1), Some("IS"), false, &areas),
            Verdict::Accept
        );
    }

    #[test]
    fn unknown_entity_or_area_is_accepted() {
        let areas = areas();
        assert_eq!(
            check(&at(BERLIN.0, BERLIN.1), None, true, &areas),
            Verdict::Accept
        );
        assert_eq!(
            check(&at(BERLIN.0, BERLIN.1), Some("??"), true, &areas),
            Verdict::Accept
        );
    }

    // --- placeholders (User Story 3, T016) --------------------------------------------------

    const PLACEHOLDER: Verdict = Verdict::Reject(RejectReason::Placeholder);

    #[test]
    fn exactly_zero_zero_is_a_placeholder_from_any_source() {
        let areas = areas();
        for (entity, online) in [
            (Some("DL"), true),
            (None, true),
            (Some("DL"), false),
            (None, false),
        ] {
            assert_eq!(
                check(&at(0.0, 0.0), entity, online, &areas),
                PLACEHOLDER,
                "{entity:?} {online}"
            );
        }
        assert_eq!(
            check(&at(0.0, 0.0), Some("CE9"), true, &areas),
            PLACEHOLDER,
            "not even Antarctica"
        );
    }

    #[test]
    fn near_zero_positions_are_not_placeholders() {
        let areas = areas();
        assert_eq!(check(&at(1e-9, 0.0), None, false, &areas), Verdict::Accept);
        assert_eq!(
            check(&at(0.5, 1.0), None, false, &areas),
            Verdict::Accept,
            "centre of JJ00"
        );
    }

    #[test]
    fn square_aa00_is_a_placeholder_except_for_antarctica() {
        let areas = areas();
        let aa00aa = at(-89.979_166, -179.958_333);
        let corner = at(-90.0, -180.0);
        let aa00 = at(-89.5, -179.0);
        for position in [aa00aa, corner, aa00] {
            assert_eq!(check(&position, Some("DL"), true, &areas), PLACEHOLDER);
            assert_eq!(check(&position, None, true, &areas), PLACEHOLDER);
            assert_eq!(
                check(&position, Some("DL"), false, &areas),
                PLACEHOLDER,
                "logger too"
            );
            assert_eq!(
                check(&position, Some("CE9"), true, &areas),
                Verdict::Accept,
                "Antarctica"
            );
        }
        assert_eq!(
            check(&at(-89.0, -179.0), None, false, &areas),
            Verdict::Accept,
            "AA01 is not AA00"
        );
        assert_eq!(
            check(&at(-89.5, -178.0), None, false, &areas),
            Verdict::Accept,
            "BA00 is not AA00"
        );
    }

    #[test]
    fn rejection_note_lists_source_round_and_reason() {
        assert_eq!(rejection_note(&[]), None);
        assert_eq!(
            rejection_note(&[("qrz", false, RejectReason::OutsideEntity)]).as_deref(),
            Some("qrz:outside-entity")
        );
        assert_eq!(
            rejection_note(&[
                ("logger", false, RejectReason::Placeholder),
                ("qrz", false, RejectReason::OutsideEntity),
                ("hamqth", true, RejectReason::Placeholder),
            ])
            .as_deref(),
            Some("logger:placeholder,qrz:outside-entity,hamqth@base:placeholder")
        );
    }
}
