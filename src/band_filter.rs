//! Band selection: groups, exclusions and the effective band set (feature 003, research R1).

use std::fmt;

use crate::band::{BANDS_IN_ORDER, band_rank, canonical_band};
use crate::errors::{Error, Result};

/// Named band groups, each in frequency order (`docs/ham-radio.md` HR-2, FR-003).
pub const GROUPS: &[(&str, &[&str])] = &[
    ("lf-mf", &["2190m", "630m", "560m"]),
    (
        "hf",
        &[
            "160m", "80m", "60m", "40m", "30m", "20m", "17m", "15m", "12m", "10m",
        ],
    ),
    ("hf-contest", &["160m", "80m", "40m", "20m", "15m", "10m"]),
    ("warc", &["30m", "17m", "12m"]),
    ("vhf", &["8m", "6m", "5m", "4m", "2m", "1.25m"]),
    ("uhf", &["70cm", "33cm", "23cm"]),
    ("shf", &["13cm", "9cm", "6cm", "3cm", "1.25cm"]),
    ("ehf", &["6mm", "4mm", "2.5mm", "2mm", "1mm", "submm"]),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Band(&'static str),
    Group(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SelectionEntry {
    exclude: bool,
    target: Target,
}

impl SelectionEntry {
    fn parse(raw: &str) -> Result<Self> {
        let text = raw.trim();
        let (exclude, name) = match text.strip_prefix('-') {
            Some(rest) => (true, rest.trim()),
            None => (false, text),
        };
        let target = canonical_band(name)
            .map(Target::Band)
            .or_else(|| {
                GROUPS
                    .iter()
                    .find(|(group, _)| group.eq_ignore_ascii_case(name))
                    .map(|(group, _)| Target::Group(group))
            })
            .ok_or_else(|| bands_error(format!("unknown band or group '{text}'")))?;
        Ok(Self { exclude, target })
    }

    fn bands(&self) -> Vec<&'static str> {
        match self.target {
            Target::Band(band) => vec![band],
            Target::Group(name) => GROUPS
                .iter()
                .find(|(group, _)| *group == name)
                .map(|(_, members)| members.iter().filter_map(|b| canonical_band(b)).collect())
                .unwrap_or_default(),
        }
    }
}

/// The effective set of considered bands: frequency order, no duplicates, never empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BandSet {
    bands: Vec<&'static str>,
}

impl BandSet {
    pub fn contains(&self, band: &str) -> bool {
        canonical_band(band).is_some_and(|b| self.bands.contains(&b))
    }

    pub fn bands(&self) -> &[&'static str] {
        &self.bands
    }
}

impl fmt::Display for BandSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.bands.join(" "))
    }
}

fn bands_error(message: String) -> Error {
    Error::Config {
        setting: "bands".to_string(),
        message,
    }
}

/// (Union of the positive entries, or every band when there are none) minus the excluded
/// entries. Exclusions win (FR-004); an empty result is an error (FR-005).
pub fn resolve(entries: &[&str]) -> Result<BandSet> {
    let parsed = entries
        .iter()
        .map(|e| SelectionEntry::parse(e))
        .collect::<Result<Vec<_>>>()?;
    let mut included: Vec<&'static str> = parsed
        .iter()
        .filter(|e| !e.exclude)
        .flat_map(|e| e.bands())
        .collect();
    if included.is_empty() {
        included = BANDS_IN_ORDER.to_vec();
    }
    let excluded: Vec<&'static str> = parsed
        .iter()
        .filter(|e| e.exclude)
        .flat_map(|e| e.bands())
        .collect();
    included.retain(|b| !excluded.contains(b));
    included.sort_by_key(|b| band_rank(b));
    included.dedup();
    if included.is_empty() {
        return Err(bands_error(format!(
            "selection '{}' leaves no bands",
            entries.join(",")
        )));
    }
    Ok(BandSet { bands: included })
}

/// Parses the `bands` setting: entries may hold comma-separated lists; blanks are skipped.
/// Nothing left means no filter (`None`).
pub fn parse_setting(entries: &[String]) -> Result<Option<BandSet>> {
    let parts: Vec<&str> = entries
        .iter()
        .flat_map(|e| e.split(','))
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    if parts.is_empty() {
        return Ok(None);
    }
    resolve(&parts).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::band::{band_rank, is_adif_band};
    use crate::errors::Error;

    fn names(set: &BandSet) -> Vec<&str> {
        set.bands().to_vec()
    }

    fn resolve_strs(entries: &[&str]) -> crate::errors::Result<BandSet> {
        resolve(entries)
    }

    #[test]
    fn group_table_is_valid_and_in_frequency_order() {
        let expected: &[(&str, &[&str])] = &[
            ("lf-mf", &["2190m", "630m", "560m"]),
            (
                "hf",
                &[
                    "160m", "80m", "60m", "40m", "30m", "20m", "17m", "15m", "12m", "10m",
                ],
            ),
            ("hf-contest", &["160m", "80m", "40m", "20m", "15m", "10m"]),
            ("warc", &["30m", "17m", "12m"]),
            ("vhf", &["8m", "6m", "5m", "4m", "2m", "1.25m"]),
            ("uhf", &["70cm", "33cm", "23cm"]),
            ("shf", &["13cm", "9cm", "6cm", "3cm", "1.25cm"]),
            ("ehf", &["6mm", "4mm", "2.5mm", "2mm", "1mm", "submm"]),
        ];
        assert_eq!(GROUPS, expected);
        for (group, members) in GROUPS {
            for band in *members {
                assert!(is_adif_band(band), "{group}: {band}");
            }
            for pair in members.windows(2) {
                assert!(band_rank(pair[0]) < band_rank(pair[1]), "{group}: {pair:?}");
            }
        }
    }

    #[test]
    fn user_story_2_examples() {
        assert_eq!(
            names(&resolve_strs(&["hf-contest"]).unwrap()),
            ["160m", "80m", "40m", "20m", "15m", "10m"]
        );
        assert_eq!(
            names(&resolve_strs(&["hf", "2m", "-60m"]).unwrap()),
            [
                "160m", "80m", "40m", "30m", "20m", "17m", "15m", "12m", "10m", "2m"
            ]
        );
        assert_eq!(
            names(&resolve_strs(&["warc", "vhf"]).unwrap()),
            ["30m", "17m", "12m", "8m", "6m", "5m", "4m", "2m", "1.25m"]
        );
        let all_but_60 = resolve_strs(&["-60m"]).unwrap();
        assert_eq!(all_but_60.bands().len(), 32);
        assert!(!all_but_60.contains("60m"));
        assert!(all_but_60.contains("2190m") && all_but_60.contains("submm"));
        assert_eq!(
            resolve_strs(&[" HF-Contest ", "40M"]).unwrap(),
            resolve_strs(&["hf-contest"]).unwrap()
        );
        assert_eq!(
            names(&resolve_strs(&["20m", "-20m", "40m"]).unwrap()),
            ["40m"]
        );
    }

    fn config_message(r: crate::errors::Result<BandSet>) -> String {
        match r {
            Err(Error::Config { setting, message }) => {
                assert_eq!(setting, "bands");
                message
            }
            other => panic!("expected a bands config error, got {other:?}"),
        }
    }

    #[test]
    fn errors_name_the_entry() {
        assert!(config_message(resolve_strs(&["hf-dx"])).contains("hf-dx"));
        assert!(config_message(resolve_strs(&["21m"])).contains("21m"));
        assert!(config_message(resolve_strs(&["warc", "-warc"])).contains("no bands"));
        config_message(resolve_strs(&["-"]));
    }

    #[test]
    fn empty_setting_means_no_filter() {
        assert_eq!(parse_setting(&[]).unwrap(), None);
        assert_eq!(parse_setting(&["".into(), " ".into()]).unwrap(), None);
        assert_eq!(parse_setting(&[" , ".into()]).unwrap(), None);
    }

    #[test]
    fn setting_entries_split_on_commas() {
        let set = parse_setting(&["warc,2m".into(), "-12m".into()])
            .unwrap()
            .unwrap();
        assert_eq!(names(&set), ["30m", "17m", "2m"]);
    }

    #[test]
    fn contains_display_and_order() {
        let set = resolve_strs(&["2m", "160m", "20m"]).unwrap();
        assert!(set.contains("20M"));
        assert!(!set.contains("40m"));
        assert_eq!(set.to_string(), "160m 20m 2m");
        assert_eq!(names(&set), ["160m", "20m", "2m"]);
    }
}
