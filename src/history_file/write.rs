//! Writing the history file (feature 014, R10, contracts/history-file.md).

use std::io::Write;
use std::path::{Path, PathBuf};

use crate::adif::AdifRecord;
use crate::adif::write::to_adif_string;
use crate::errors::{Error, Result};
use crate::history::format_rfc3339;

/// The whole file: a header, then one record per line in the order given.
pub fn render(records: &[AdifRecord], now: i64) -> String {
    let written = format_rfc3339(now);
    // `YYYY-MM-DDTHH:MM:SSZ` → `YYYYMMDD HHMMSS` (ADIF CREATED_TIMESTAMP, UTC).
    let digits: String = written.chars().filter(char::is_ascii_digit).collect();
    let created = format!("{} {}", &digits[..8], &digits[8..14]);
    let version = env!("CARGO_PKG_VERSION");
    let mut out = format!(
        "live-qso-map QSO history, written {written}\n\
         <ADIF_VER:5>3.1.4 <PROGRAMID:12>live-qso-map <PROGRAMVERSION:{}>{version} \
         <CREATED_TIMESTAMP:{}>{created} <EOH>\n",
        version.len(),
        created.len()
    );
    for record in records {
        out.push_str(&to_adif_string(record));
        out.push('\n');
    }
    out
}

/// Replaces `path` with the rendered file: written to `<name>.tmp` in the same folder, synced,
/// then renamed over `path`. On any error the temporary file is removed and `path` is untouched.
pub fn save(path: &Path, records: &[AdifRecord], now: i64) -> Result<()> {
    let tmp = temporary_path(path);
    let result =
        write_and_sync(&tmp, &render(records, now)).and_then(|()| std::fs::rename(&tmp, path));
    result.map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        Error::HistoryFile {
            path: path.to_path_buf(),
            message: e.to_string(),
        }
    })
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    path.with_file_name(name)
}

fn write_and_sync(path: &Path, text: &str) -> std::io::Result<()> {
    let mut file = std::fs::File::create(path)?;
    file.write_all(text.as_bytes())?;
    file.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adif::AdifRecord;
    use crate::adif::parse::parse_datagram;
    use std::path::{Path, PathBuf};

    const NOW: i64 = 1_792_000_992; // 2026-10-14T18:03:12Z

    fn record(call: &str, time: &str) -> AdifRecord {
        let mut r = AdifRecord::new();
        r.set("CALL", call);
        r.set("QSO_DATE", "20261014");
        r.set("TIME_ON", time);
        r.set("QTH", "Città");
        r
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "lqm-write-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn header_names_program_version_and_utc_time() {
        let text = render(&[], NOW);
        let mut lines = text.lines();
        assert_eq!(
            lines.next(),
            Some("live-qso-map QSO history, written 2026-10-14T18:03:12Z")
        );
        let version = env!("CARGO_PKG_VERSION");
        assert_eq!(
            lines.next().unwrap(),
            format!(
                "<ADIF_VER:5>3.1.4 <PROGRAMID:12>live-qso-map <PROGRAMVERSION:{}>{version} \
                 <CREATED_TIMESTAMP:15>20261014 180312 <EOH>",
                version.len()
            )
        );
        assert_eq!(lines.next(), None);
    }

    #[test]
    fn one_record_per_line_in_the_given_order_and_parses_back() {
        let records = [record("K1A", "1201"), record("K2B", "1200")];
        let text = render(&records, NOW);
        let lines: Vec<&str> = text.lines().skip(2).collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("<CALL:3>K1A") && lines[0].ends_with("<EOR>"));
        assert!(lines[1].contains("<CALL:3>K2B"));
        let (back, errors) = parse_datagram(text.as_bytes());
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(back, records);
    }

    #[test]
    fn save_replaces_the_file_and_leaves_no_temporary() {
        let dir = temp_dir("replace");
        let path = dir.join("qso-history.adi");
        std::fs::write(&path, "old").unwrap();
        save(&path, &[record("K1A", "1200")], NOW).unwrap();
        assert!(
            std::fs::read_to_string(&path)
                .unwrap()
                .contains("<CALL:3>K1A")
        );
        assert_eq!(names(&dir), ["qso-history.adi"]);
    }

    #[test]
    fn missing_folder_is_an_error_and_writes_nothing() {
        let dir = temp_dir("missing");
        let path = dir.join("gone").join("qso-history.adi");
        match save(&path, &[record("K1A", "1200")], NOW) {
            Err(crate::errors::Error::HistoryFile { path: p, .. }) => assert_eq!(p, path),
            other => panic!("expected a history file error, got {other:?}"),
        }
        assert!(names(&dir).is_empty());
    }

    #[test]
    fn failed_rename_keeps_the_target_and_removes_the_temporary() {
        // A directory where the file should be: the rename fails on every platform.
        let dir = temp_dir("rename");
        let path = dir.join("qso-history.adi");
        std::fs::create_dir(&path).unwrap();
        assert!(save(&path, &[record("K1A", "1200")], NOW).is_err());
        assert!(path.is_dir());
        assert_eq!(names(&dir), ["qso-history.adi"]);
    }

    #[cfg(unix)]
    #[test]
    fn read_only_folder_keeps_the_previous_file_byte_for_byte() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp_dir("readonly");
        let path = dir.join("qso-history.adi");
        std::fs::write(&path, "previous content").unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
        let result = save(&path, &[record("K1A", "1200")], NOW);
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        // Root ignores folder permissions; the check only means something otherwise.
        if result.is_err() {
            assert_eq!(std::fs::read(&path).unwrap(), b"previous content");
            assert_eq!(names(&dir), ["qso-history.adi"]);
        }
    }
}
