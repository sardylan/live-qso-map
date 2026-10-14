//! Reading the history file (feature 014, R11, contracts/history-file.md).

use std::path::{Path, PathBuf};

use crate::adif::AdifRecord;
use crate::adif::parse::parse_datagram;
use crate::errors::{Error, Result};
use crate::history::format_rfc3339;

/// What reading the history file gave.
#[derive(Debug)]
pub enum Read {
    /// No file: an empty history.
    Missing,
    /// The file cannot be read, or holds neither a header nor a record: the reason.
    Unreadable(String),
    /// Every record that parsed, in file order, and the number that did not.
    Records {
        records: Vec<AdifRecord>,
        malformed: usize,
    },
}

/// Reads `path`. An empty file is an empty history, not an unreadable one (FR-011).
pub fn read(path: &Path) -> Read {
    let data = match std::fs::read(path) {
        Ok(data) => data,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Read::Missing,
        Err(e) => return Read::Unreadable(e.to_string()),
    };
    if !data.is_empty() && !contains_tag(&data, b"<eoh>") && !contains_tag(&data, b"<eor>") {
        return Read::Unreadable("not an ADIF file: no header and no record".to_string());
    }
    let (records, errors) = parse_datagram(&data);
    Read::Records {
        records,
        malformed: errors.len(),
    }
}

fn contains_tag(data: &[u8], tag: &[u8]) -> bool {
    data.windows(tag.len())
        .any(|window| window.eq_ignore_ascii_case(tag))
}

/// Renames `path` to `<name>.unreadable-YYYYMMDDTHHMMSSZ` (UTC, no `:` so the name is valid on
/// Windows) and returns the new path.
pub fn move_aside(path: &Path, now: i64) -> Result<PathBuf> {
    let stamp: String = format_rfc3339(now)
        .chars()
        .filter(|c| *c != '-' && *c != ':')
        .collect();
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".unreadable-{stamp}"));
    let target = path.with_file_name(name);
    std::fs::rename(path, &target).map_err(|e| Error::HistoryFile {
        path: path.to_path_buf(),
        message: format!("cannot move it aside: {e}"),
    })?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const NOW: i64 = 1_792_000_992; // 2026-10-14T18:03:12Z

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "lqm-read-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_missing_file_is_missing() {
        let dir = temp_dir("missing");
        assert!(matches!(read(&dir.join("nope.adi")), Read::Missing));
    }

    #[test]
    fn an_empty_file_is_an_empty_history() {
        let dir = temp_dir("empty");
        let path = dir.join("qso-history.adi");
        std::fs::write(&path, "").unwrap();
        match read(&path) {
            Read::Records { records, malformed } => {
                assert!(records.is_empty());
                assert_eq!(malformed, 0);
            }
            _ => panic!("an empty file is readable"),
        }
    }

    #[test]
    fn an_exported_log_gives_every_parsable_record() {
        match read(&fixture("history-external.adi")) {
            Read::Records { records, malformed } => {
                // The record without TIME_ON parses; the restore step skips it.
                assert_eq!(records.len(), 8);
                assert_eq!(malformed, 0);
                assert_eq!(records[0].get("CALL"), Some("DL1ABC"));
            }
            _ => panic!("the fixture is readable"),
        }
    }

    #[test]
    fn broken_records_are_counted() {
        let dir = temp_dir("broken");
        let path = dir.join("qso-history.adi");
        std::fs::write(&path, "<EOH>\n<CALL:3>K1A <EOR>\n<CALL:99>K2B <EOR>\n").unwrap();
        match read(&path) {
            Read::Records { records, malformed } => {
                assert_eq!(records.len(), 1);
                assert_eq!(malformed, 1);
            }
            _ => panic!("a partly broken file is readable"),
        }
    }

    #[test]
    fn text_that_is_not_adif_is_unreadable() {
        assert!(matches!(
            read(&fixture("history-not-adif.txt")),
            Read::Unreadable(_)
        ));
    }

    #[test]
    fn a_folder_is_unreadable() {
        let dir = temp_dir("folder");
        assert!(matches!(read(&dir), Read::Unreadable(_)));
    }

    #[test]
    fn move_aside_adds_the_utc_time_without_colons() {
        let dir = temp_dir("aside");
        let path = dir.join("qso-history.adi");
        std::fs::write(&path, "junk").unwrap();
        let moved = move_aside(&path, NOW).unwrap();
        assert_eq!(
            moved,
            dir.join("qso-history.adi.unreadable-20261014T180312Z")
        );
        assert!(!path.exists());
        assert_eq!(std::fs::read(&moved).unwrap(), b"junk");
    }

    #[test]
    fn move_aside_of_a_missing_file_is_an_error() {
        let dir = temp_dir("aside-missing");
        assert!(matches!(
            move_aside(&dir.join("nope.adi"), NOW),
            Err(crate::errors::Error::HistoryFile { .. })
        ));
    }
}
