//! Feature 009: `CHANGELOG.md` keeps an "Unreleased" section and has a non-empty section for the
//! crate's version, so a release can never be tagged without notes (FR-031, FR-032).

use std::collections::HashSet;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const UNRELEASED: &str = "## [Unreleased]";

fn changelog() -> Result<String, std::io::Error> {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("CHANGELOG.md"))
}

/// The version of a `## [X.Y.Z] - YYYY-MM-DD` heading, or `None` for any other line.
fn version_heading(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("## [")?;
    let (version, date) = rest.split_once("] - ")?;
    is_date(date).then_some(version)
}

fn is_date(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 10
        && bytes.iter().enumerate().all(|(i, b)| match i {
            4 | 7 => *b == b'-',
            _ => b.is_ascii_digit(),
        })
}

/// The lines of the section whose heading is `heading`, up to the next `## ` heading.
fn section(text: &str, heading: impl Fn(&str) -> bool) -> Option<Vec<&str>> {
    let mut lines = text.lines().skip_while(|line| !heading(line));
    lines.next()?;
    Some(lines.take_while(|line| !line.starts_with("## ")).collect())
}

#[test]
fn has_exactly_one_unreleased_section_before_any_version() -> TestResult {
    let text = changelog()?;
    let headings: Vec<&str> = text.lines().filter(|l| l.starts_with("## ")).collect();
    assert_eq!(
        headings
            .iter()
            .filter(|h| h.trim_end() == UNRELEASED)
            .count(),
        1
    );
    assert_eq!(headings.first().map(|h| h.trim_end()), Some(UNRELEASED));
    Ok(())
}

#[test]
fn has_a_non_empty_section_for_the_crate_version() -> TestResult {
    let text = changelog()?;
    let version = env!("CARGO_PKG_VERSION");
    let body = section(&text, |line| version_heading(line) == Some(version));
    let Some(body) = body else {
        return Err(format!("CHANGELOG.md has no '## [{version}] - YYYY-MM-DD' section").into());
    };
    assert!(
        body.iter().any(|l| l.starts_with("### ")),
        "section {version} has no ### subsection"
    );
    assert!(
        body.iter().any(|l| l.starts_with("- ")),
        "section {version} has no '- ' entry"
    );
    Ok(())
}

#[test]
fn version_headings_are_unique() -> TestResult {
    let text = changelog()?;
    let mut seen = HashSet::new();
    for version in text.lines().filter_map(version_heading) {
        assert!(seen.insert(version), "version {version} appears twice");
    }
    Ok(())
}

#[test]
fn heading_parser_accepts_only_dated_versions() {
    assert_eq!(version_heading("## [2.0.1] - 2026-10-12"), Some("2.0.1"));
    assert_eq!(version_heading("## [2.0.1]"), None);
    assert_eq!(version_heading("## [2.0.1] - 2026-1-12"), None);
    assert_eq!(version_heading("## [Unreleased]"), None);
}
