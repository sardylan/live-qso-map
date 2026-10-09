//! Last-QSO box columns chosen by the operator (feature 006, research R2).

use serde::Serialize;

use crate::errors::{Error, Result};

/// One column of the last-QSO box. It travels to pages as its keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Column {
    Call,
    Band,
    Mode,
    Date,
    Time,
    CqZone,
    Country,
}

const ALL: [Column; 7] = [
    Column::Call,
    Column::Band,
    Column::Mode,
    Column::Date,
    Column::Time,
    Column::CqZone,
    Column::Country,
];

/// The columns when the operator sets none (FR-002).
pub const DEFAULT_COLUMNS: [Column; 5] = [
    Column::Call,
    Column::Band,
    Column::Time,
    Column::CqZone,
    Column::Country,
];

impl Column {
    pub fn keyword(self) -> &'static str {
        match self {
            Column::Call => "call",
            Column::Band => "band",
            Column::Mode => "mode",
            Column::Date => "date",
            Column::Time => "time",
            Column::CqZone => "cqzone",
            Column::Country => "country",
        }
    }
}

fn invalid(message: String) -> Error {
    Error::Config {
        setting: "map.recent_columns".into(),
        message,
    }
}

/// Parses the operator's column list. Entries may hold comma-separated keywords; keywords ignore
/// case and surrounding spaces. Unknown, repeated or missing keywords are errors (FR-003).
pub fn parse(entries: &[String]) -> Result<Vec<Column>> {
    let mut columns = Vec::new();
    for keyword in entries
        .iter()
        .flat_map(|entry| entry.split(','))
        .map(|k| k.trim().to_ascii_lowercase())
        .filter(|k| !k.is_empty())
    {
        let column = ALL
            .into_iter()
            .find(|c| c.keyword() == keyword)
            .ok_or_else(|| {
                let valid: Vec<&str> = ALL.iter().map(|c| c.keyword()).collect();
                invalid(format!("unknown column '{keyword}' ({})", valid.join(", ")))
            })?;
        if columns.contains(&column) {
            return Err(invalid(format!("column '{keyword}' is listed twice")));
        }
        columns.push(column);
    }
    if columns.is_empty() {
        return Err(invalid("no columns".into()));
    }
    Ok(columns)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::Error;

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn rejected(items: &[&str], needle: &str) {
        match parse(&strings(items)) {
            Err(Error::Config { setting, message }) => {
                assert_eq!(setting, "map.recent_columns");
                assert!(message.contains(needle), "{items:?}: {message}");
            }
            other => panic!("expected a config error for {items:?}, got {other:?}"),
        }
    }

    #[test]
    fn default_list_parses_to_the_defaults() {
        assert_eq!(
            parse(&strings(&["call", "band", "time", "cqzone", "country"])).unwrap(),
            DEFAULT_COLUMNS
        );
    }

    #[test]
    fn keywords_ignore_case_and_spaces_and_keep_their_order() {
        assert_eq!(
            parse(&strings(&[" Call ", "COUNTRY"])).unwrap(),
            [Column::Call, Column::Country]
        );
        let all = ["country", "cqzone", "time", "date", "mode", "band", "call"];
        let parsed = parse(&strings(&all)).unwrap();
        assert_eq!(parsed.iter().map(|c| c.keyword()).collect::<Vec<_>>(), all);
    }

    #[test]
    fn invalid_lists_are_rejected() {
        rejected(&["call", "freq"], "unknown column 'freq'");
        rejected(
            &["call", "freq"],
            "call, band, mode, date, time, cqzone, country",
        );
        rejected(&["call", "call"], "column 'call' is listed twice");
        rejected(&["Call", " call "], "listed twice");
        rejected(&[], "no columns");
        rejected(&[" ", ""], "no columns");
    }

    #[test]
    fn columns_serialise_as_keywords() {
        assert_eq!(
            serde_json::to_value(DEFAULT_COLUMNS).unwrap(),
            serde_json::json!(["call", "band", "time", "cqzone", "country"])
        );
    }
}
