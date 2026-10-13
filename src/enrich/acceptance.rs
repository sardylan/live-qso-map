//! Whether a remote position can be used for a QSO (feature 010, research R4).
//!
//! A rejection is a verdict, not an error: it is a plain enum, not a `Result`
//! (`docs/rust-guidelines.md`).

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
