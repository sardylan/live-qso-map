//! Logger mode names to ADIF MODE/SUBMODE (research R8).

/// Returns `(MODE, SUBMODE)`. Unknown modes are kept uppercased without a submode.
pub fn map_mode(raw: &str) -> (String, Option<String>) {
    let mode = raw.trim().to_ascii_uppercase();
    match mode.as_str() {
        "LSB" | "USB" => ("SSB".into(), Some(mode)),
        "FT4" => ("MFSK".into(), Some(mode)),
        "PSK31" => ("PSK".into(), Some(mode)),
        "CW" | "RTTY" | "FM" | "AM" | "FT8" | "SSB" => (mode, None),
        _ => {
            tracing::debug!(mode, "unmapped mode kept as is");
            (mode, None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(raw: &str) -> (String, Option<String>) {
        map_mode(raw)
    }

    #[test]
    fn sidebands_become_ssb_with_submode() {
        assert_eq!(m("LSB"), ("SSB".into(), Some("LSB".into())));
        assert_eq!(m("USB"), ("SSB".into(), Some("USB".into())));
    }

    #[test]
    fn plain_modes_pass_through() {
        for mode in ["CW", "RTTY", "FM", "AM", "FT8", "SSB"] {
            assert_eq!(m(mode), (mode.to_string(), None));
        }
    }

    #[test]
    fn submodes_get_their_adif_mode() {
        assert_eq!(m("FT4"), ("MFSK".into(), Some("FT4".into())));
        assert_eq!(m("PSK31"), ("PSK".into(), Some("PSK31".into())));
    }

    #[test]
    fn lowercase_and_unknown() {
        assert_eq!(m(" lsb "), ("SSB".into(), Some("LSB".into())));
        assert_eq!(m("cw"), ("CW".into(), None));
        assert_eq!(m("xyz"), ("XYZ".into(), None));
    }
}
