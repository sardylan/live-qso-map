//! Reading a callsign as logged: its base callsign and whether a lookup may be retried with it
//! (feature 010, research R2; `docs/ham-radio.md` §2.2).

/// Portable designators that don't change the entity (`/P`, `/M`, `/QRP`, …).
pub(crate) const IGNORED_SUFFIXES: &[&str] = &["P", "M", "QRP", "QRPP", "A", "B", "LH"];
/// Maritime and aeronautical mobile: no entity, and the station is not at home.
pub(crate) const NO_ENTITY_SUFFIXES: &[&str] = &["MM", "AM"];

/// The parts of a callsign as logged: trimmed, uppercase, split on `/`, empty parts dropped.
fn parts(call: &str) -> Vec<String> {
    call.trim()
        .to_ascii_uppercase()
        .split('/')
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect()
}

fn is_call_area_digit(part: &str) -> bool {
    part.len() == 1 && part.chars().all(|c| c.is_ascii_digit())
}

/// The base callsign to retry online with: the callsign without its location prefix and its
/// portable suffixes (`IS0/DL0AAA` → `DL0AAA`, `IS0AAA/P` → `IS0AAA`, `K0AAA/6` → `K0AAA`).
/// `None` when there is nothing to remove.
pub fn base_call(call: &str) -> Option<String> {
    let parts = parts(call);
    let kept: Vec<&String> = parts
        .iter()
        .filter(|p| !IGNORED_SUFFIXES.contains(&p.as_str()) && !is_call_area_digit(p))
        .collect();
    // The longest remaining part is the callsign; a shorter one is the location prefix.
    let base = kept.iter().max_by_key(|p| p.len())?;
    (parts.len() > 1).then(|| base.to_string())
}

/// Whether a lookup may be retried with the base callsign: never for maritime or aeronautical
/// mobile stations (FR-007).
pub fn retry_allowed(call: &str) -> bool {
    !parts(call)
        .iter()
        .skip(1)
        .any(|p| NO_ENTITY_SUFFIXES.contains(&p.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_call_strips_location_prefixes_and_portable_suffixes() {
        for (call, base) in [
            ("IS0/DL0AAA", "DL0AAA"),
            ("DL0AAA/IS0", "DL0AAA"),
            ("IS0AAA/P", "IS0AAA"),
            ("K0AAA/6", "K0AAA"),
            ("IS0/DL0AAA/P", "DL0AAA"),
            ("I/HB9AAA", "HB9AAA"),
            ("is0/dl0aaa", "DL0AAA"),
            ("  IS0AAA/QRPP ", "IS0AAA"),
            ("IS0//DL0AAA", "DL0AAA"),
            ("G0AAA/LH", "G0AAA"),
            ("DL0AAA/B", "DL0AAA"),
            ("DL0AAA/M", "DL0AAA"),
        ] {
            assert_eq!(base_call(call).as_deref(), Some(base), "{call}");
        }
    }

    #[test]
    fn plain_calls_have_no_base() {
        for call in ["DL0AAA", " dl0aaa ", "", "/", "P"] {
            assert_eq!(base_call(call), None, "{call:?}");
        }
    }

    #[test]
    fn maritime_and_aeronautical_mobile_are_never_retried() {
        assert!(!retry_allowed("DL0AAA/MM"));
        assert!(!retry_allowed("DL0AAA/AM"));
        assert!(!retry_allowed("dl0aaa/mm"));
        assert!(retry_allowed("DL0AAA/P"));
        assert!(retry_allowed("IS0/DL0AAA"));
    }
}
