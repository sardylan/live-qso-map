//! Datagram format detection from content alone (FR-002, research R5).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFormat {
    Adif,
    QarTest,
    Unrecognised,
}

pub fn detect(data: &[u8]) -> SourceFormat {
    let data = trim_start(data);
    if looks_like_adif(data) {
        return SourceFormat::Adif;
    }
    SourceFormat::Unrecognised
}

fn trim_start(data: &[u8]) -> &[u8] {
    let data = data.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(data);
    let start = data
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(data.len());
    &data[start..]
}

/// An ADIF data specifier `<NAME:digits` (XML tags never carry `:length`), or `<EOH>`/`<EOR>`.
fn looks_like_adif(data: &[u8]) -> bool {
    data.windows(5)
        .any(|w| w.eq_ignore_ascii_case(b"<EOR>") || w.eq_ignore_ascii_case(b"<EOH>"))
        || data
            .iter()
            .enumerate()
            .filter(|(_, b)| **b == b'<')
            .any(|(i, _)| is_data_specifier(&data[i + 1..]))
}

fn is_data_specifier(rest: &[u8]) -> bool {
    let name_len = rest
        .iter()
        .take_while(|b| b.is_ascii_alphanumeric() || **b == b'_')
        .count();
    name_len > 0
        && rest.get(name_len) == Some(&b':')
        && rest.get(name_len + 1).is_some_and(u8::is_ascii_digit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adif_with_data_specifier() {
        assert_eq!(detect(b"<CALL:4>K1AA <EOR>"), SourceFormat::Adif);
        assert_eq!(detect(b"<call:4>K1AA"), SourceFormat::Adif);
    }

    #[test]
    fn adif_markers_alone() {
        assert_eq!(detect(b"<EOR>"), SourceFormat::Adif);
        assert_eq!(detect(b"header text <eoh>"), SourceFormat::Adif);
    }

    #[test]
    fn unrecognised_inputs() {
        assert_eq!(detect(b"hello world"), SourceFormat::Unrecognised);
        assert_eq!(detect(b""), SourceFormat::Unrecognised);
        assert_eq!(
            detect(br#"<?xml version="1.0"?><ADX><HEADER></HEADER></ADX>"#),
            SourceFormat::Unrecognised
        );
        assert_eq!(detect(b"<b>bold</b>"), SourceFormat::Unrecognised);
    }

    #[test]
    fn skips_whitespace_and_bom() {
        assert_eq!(detect(b"  \r\n\t<CALL:4>K1AA <EOR>"), SourceFormat::Adif);
        let mut data = vec![0xEF, 0xBB, 0xBF];
        data.extend_from_slice(b"<CALL:4>K1AA <EOR>");
        assert_eq!(detect(&data), SourceFormat::Adif);
    }
}
