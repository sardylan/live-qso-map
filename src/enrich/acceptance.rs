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

/// Judges one position for a QSO whose callsign belongs to `entity` (a cty.dat primary prefix)
/// (research R4). `online` is true for online providers: only their positions must lie inside
/// the entity's area, nested entities excluded. The logger's and the country data's positions
/// are trusted (FR-012). An unknown entity or area accepts the position.
pub fn check(
    position: &Position,
    entity: Option<&str>,
    online: bool,
    areas: &ExtentTable,
) -> Verdict {
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
