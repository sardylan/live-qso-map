//! Writes the effective configuration to a file or standard output (feature 008, research R9).

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::config::{Config, FileConfig};
use crate::errors::{Error, Result};

/// Renders `config` in the configuration file format: a header comment, then every setting.
/// `now_utc` is an RFC 3339 UTC time ending in `Z` (HR-1).
pub fn render(config: &Config, version: &str, now_utc: &str) -> String {
    let mut text = format!("# live-qso-map {version} configuration, written {now_utc}.\n");
    if has_credentials(config) {
        text.push_str("# This file contains provider passwords: keep it private.\n");
    }
    // Serialising plain strings, numbers and tables cannot fail; an empty body would only
    // drop settings, never write a wrong one.
    let body = toml::to_string_pretty(&FileConfig::from_config(config)).unwrap_or_default();
    text.push_str(&body);
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text
}

/// Whether any lookup provider has credentials, so the written file holds passwords.
pub fn has_credentials(config: &Config) -> bool {
    config.lookup.qrz.is_some() || config.lookup.hamqth.is_some() || config.lookup.qrzcq.is_some()
}

/// Writes `text` to `path` atomically, or to `stdout` when `path` is `-`.
///
/// A new file is created owner-only (`0600`) on Unix. A replaced file keeps its mode, and its
/// owner and group when the writer may set them (FR-022, FR-024).
pub fn write(path: &Path, text: &str, stdout: &mut dyn Write) -> Result<()> {
    if path == Path::new("-") {
        stdout
            .write_all(text.as_bytes())
            .map_err(write_error(path))?;
        return stdout.flush().map_err(write_error(path));
    }
    let tmp = temporary_path(path);
    let result = write_temporary(&tmp, path, text)
        .and_then(|()| fs::rename(&tmp, path).map_err(write_error(path)));
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

/// Maps an I/O error to [`Error::ConfigWrite`] naming the destination, never the temporary file.
fn write_error(path: &Path) -> impl Fn(std::io::Error) -> Error + '_ {
    move |source| Error::ConfigWrite {
        path: path.to_path_buf(),
        source,
    }
}

fn temporary_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "config".to_string());
    path.with_file_name(format!(".{name}.tmp-{}", std::process::id()))
}

fn write_temporary(tmp: &Path, target: &Path, text: &str) -> Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let fail = write_error(target);
    let mut file = options.open(tmp).map_err(&fail)?;
    file.write_all(text.as_bytes()).map_err(&fail)?;
    file.sync_all().map_err(&fail)?;
    drop(file);
    keep_existing_permissions(tmp, target)
}

/// Copies the target's mode, owner and group to the temporary file (Unix only).
#[cfg(unix)]
fn keep_existing_permissions(tmp: &Path, target: &Path) -> Result<()> {
    use std::os::unix::fs::MetadataExt;
    let Ok(meta) = fs::metadata(target) else {
        return Ok(());
    };
    // Only root may give the file away; anyone else keeps their own ownership.
    let _ = std::os::unix::fs::chown(tmp, Some(meta.uid()), Some(meta.gid()));
    fs::set_permissions(tmp, meta.permissions()).map_err(write_error(target))
}

/// Elsewhere the file takes the folder's default permissions.
#[cfg(not(unix))]
fn keep_existing_permissions(_tmp: &Path, _target: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Cli, Credentials, ProviderKind, Secret};
    use clap::Parser;
    use std::collections::HashMap;
    use std::path::PathBuf;

    const NOW: &str = "2026-10-11T14:03:00Z";

    fn load(args: &[&str], env: &[(&str, &str)], file: Option<&str>) -> Config {
        let mut argv = vec!["live-qso-map"];
        argv.extend_from_slice(args);
        let cli = Cli::try_parse_from(argv).unwrap();
        let env: HashMap<String, String> = env
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Config::load(&cli, &|k| env.get(k).cloned(), file)
            .unwrap()
            .0
    }

    fn rendered(config: &Config) -> String {
        render(config, "2.0.0", NOW)
    }

    // --- render (T007) ---

    #[test]
    fn header_names_version_and_utc_time() {
        let text = rendered(&Config::default());
        assert_eq!(
            text.lines().next(),
            Some("# live-qso-map 2.0.0 configuration, written 2026-10-11T14:03:00Z.")
        );
        assert!(!text.contains("contains provider passwords"));
        assert!(text.ends_with('\n'));
    }

    #[test]
    fn password_note_only_with_credentials() {
        let config = load(&["--qrz-username", "u", "--qrz-password", "p"], &[], None);
        let text = rendered(&config);
        assert_eq!(
            text.lines().nth(1),
            Some("# This file contains provider passwords: keep it private.")
        );
    }

    #[test]
    fn defaults_write_every_key() {
        let text = rendered(&Config::default());
        let table: toml::Table = toml::from_str(&text).unwrap();
        for key in [
            "bind_address",
            "udp_port",
            "http_port",
            "workers",
            "ingest_queue",
            "broadcast_queue",
            "log_level",
        ] {
            assert!(table.contains_key(key), "{key}");
        }
        let bands: Vec<&str> = table["bands"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(
            bands,
            ["160m", "80m", "40m", "20m", "15m", "10m"],
            "the default is written out"
        );
        let lookup = table["lookup"].as_table().unwrap();
        assert_eq!(lookup["timeout_secs"].as_integer(), Some(5));
        assert_eq!(
            lookup["provider_order"].as_array().unwrap().len(),
            3,
            "{lookup:?}"
        );
        assert!(!lookup.contains_key("qrz"));
        let cty = table["country_data"].as_table().unwrap();
        assert_eq!(cty["path"].as_str(), Some("./cty.dat"));
        assert!(cty.contains_key("url") && cty.contains_key("max_age_days"));
        let map = table["map"].as_table().unwrap();
        for key in [
            "fallback_color",
            "recent_columns",
            "recent_header",
            "globe_border_deg",
            "bands",
        ] {
            assert!(map.contains_key(key), "map.{key}");
        }
        for key in [
            "style_url",
            "style_attribution",
            "tile_url",
            "tile_attribution",
        ] {
            assert!(!map.contains_key(key), "map.{key} is unset");
        }
        assert_eq!(
            map["bands"].as_table().unwrap().len(),
            Config::default().map.bands.len()
        );
        let station = table.get("station").and_then(|s| s.as_table());
        assert!(station.is_none_or(|s| s.is_empty()));
        assert!(
            !table.contains_key("history_size"),
            "removed in feature 014"
        );
    }

    #[test]
    fn history_file_is_written_only_when_set_explicitly() {
        let default = rendered(&load(&["--cty-path", "/var/lib/lqm/cty.dat"], &[], None));
        let table: toml::Table = toml::from_str(&default).unwrap();
        assert!(!table.contains_key("history_file"), "{default}");
        let explicit = rendered(&load(&["--history-file", "/srv/log.adi"], &[], None));
        let table: toml::Table = toml::from_str(&explicit).unwrap();
        assert_eq!(table["history_file"].as_str(), Some("/srv/log.adi"));
    }

    #[test]
    fn every_band_is_written_as_all_and_reloads_the_same() {
        let config = load(&["--bands", "all"], &[], None);
        let text = rendered(&config);
        let table: toml::Table = toml::from_str(&text).unwrap();
        assert_eq!(
            table["bands"].as_array().unwrap(),
            &vec![toml::Value::from("all")]
        );
        assert_eq!(load(&[], &[], Some(&text)).band_filter, None);

        let default = rendered(&load(&[], &[], None));
        assert_eq!(
            load(&[], &[], Some(&default)).band_filter,
            load(&[], &[], None).band_filter
        );
    }

    #[test]
    fn band_lists_are_in_frequency_order() {
        let config = load(&["--bands", "10m,160m,40m"], &[], None);
        let text = rendered(&config);
        let table: toml::Table = toml::from_str(&text).unwrap();
        let bands: Vec<&str> = table["bands"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(bands, ["160m", "40m", "10m"]);
        let colours = text.find("\n160m = ").unwrap();
        let tens = text.find("\n10m = ").unwrap();
        assert!(
            colours < tens,
            "[map.bands] in frequency order, not alphabetical"
        );
    }

    #[test]
    fn passwords_are_written_in_clear() {
        let config = load(
            &[],
            &[
                ("LQM_HAMQTH_USERNAME", "user"),
                ("LQM_HAMQTH_PASSWORD", "s3cret"),
            ],
            None,
        );
        let text = rendered(&config);
        let table: toml::Table = toml::from_str(&text).unwrap();
        let hamqth = table["lookup"]["hamqth"].as_table().unwrap();
        assert_eq!(hamqth["username"].as_str(), Some("user"));
        assert_eq!(hamqth["password"].as_str(), Some("s3cret"));
        assert!(table["lookup"].get("qrz").is_none());
    }

    #[test]
    fn relative_country_data_path_is_kept() {
        let config = load(&["--cty-path", "data/cty.dat"], &[], None);
        assert!(rendered(&config).contains("path = \"data/cty.dat\""));
    }

    // --- round trip (T008, SC-004) ---

    fn assert_round_trip(config: Config) {
        let text = rendered(&config);
        let back = load(&[], &[], Some(&text));
        assert_eq!(back, config, "{text}");
    }

    #[test]
    fn round_trip_over_many_combinations() {
        let cases: Vec<Config> = vec![
            load(&[], &[], None),
            load(&["--history-file", "/srv/contest.adi"], &[], None),
            load(
                &[
                    "--station-callsign",
                    "is0mzk",
                    "--station-locator",
                    "JM49NH",
                ],
                &[],
                None,
            ),
            load(
                &[
                    "--station-latitude",
                    "-33.9",
                    "--station-longitude",
                    "151.2",
                ],
                &[],
                None,
            ),
            load(
                &[
                    "--qrz-username",
                    "a",
                    "--qrz-password",
                    "b",
                    "--hamqth-username",
                    "c",
                    "--hamqth-password",
                    "d",
                    "--qrzcq-username",
                    "e",
                    "--qrzcq-password",
                    "f",
                    "--provider-order",
                    "qrzcq,qrz,hamqth",
                    "--lookup-timeout",
                    "9",
                ],
                &[],
                None,
            ),
            load(&["--bands", "hf,2m,-60m"], &[], None),
            load(
                &[
                    "--tile-url",
                    "https://tiles.example/{z}/{x}/{y}.png",
                    "--tile-attribution",
                    "Example tiles",
                ],
                &[],
                None,
            ),
            load(
                &[
                    "--style-url",
                    "https://styles.example/s.json",
                    "--style-attribution",
                    "S",
                ],
                &[],
                None,
            ),
            load(
                &["--recent-columns", "mode,call", "--recent-header", "false"],
                &[],
                None,
            ),
            load(
                &["--band-color", "20m=#123456", "--fallback-color", "#abcdef"],
                &[],
                None,
            ),
            load(
                &[
                    "--bind-address",
                    "127.0.0.1",
                    "--udp-port",
                    "2237",
                    "--http-port",
                    "9090",
                    "--workers",
                    "4",
                    "--ingest-queue",
                    "16",
                    "--broadcast-queue",
                    "32",
                    "--log-level",
                    "DEBUG",
                    "--cty-path",
                    "/var/lib/live-qso-map/cty.dat",
                    "--cty-url",
                    "https://example.com/cty.dat",
                    "--cty-max-age-days",
                    "0",
                ],
                &[],
                None,
            ),
            load(
                &[],
                &[("LQM_UDP_PORT", "4000"), ("LQM_BANDS", "40m")],
                Some("udp_port = 3000\nbands = [\"20m\"]\n[station]\ncallsign = \"K1ABC\"\n"),
            ),
        ];
        for config in cases {
            assert_round_trip(config);
        }
    }

    // --- write (T009) ---

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "lqm-export-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn entries(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn dash_writes_to_stdout_only() {
        let dir = temp_dir("dash");
        let mut out = Vec::new();
        write(Path::new("-"), "a = 1\n", &mut out).unwrap();
        assert_eq!(out, b"a = 1\n");
        assert!(entries(&dir).is_empty());
    }

    #[test]
    fn writes_a_new_file() {
        let dir = temp_dir("new");
        let path = dir.join("c.toml");
        write(&path, "a = 1\n", &mut Vec::new()).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "a = 1\n");
        assert_eq!(entries(&dir), ["c.toml"]);
    }

    #[test]
    fn replaces_an_existing_file_without_leftovers() {
        let dir = temp_dir("replace");
        let path = dir.join("c.toml");
        std::fs::write(&path, "old = true\n").unwrap();
        write(&path, "new = true\n", &mut Vec::new()).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new = true\n");
        assert_eq!(entries(&dir), ["c.toml"]);
    }

    #[test]
    fn missing_folder_is_a_config_write_error() {
        let dir = temp_dir("missing");
        let path = dir.join("nope").join("c.toml");
        match write(&path, "a = 1\n", &mut Vec::new()) {
            Err(crate::errors::Error::ConfigWrite { path: p, .. }) => assert_eq!(p, path),
            other => panic!("expected ConfigWrite, got {other:?}"),
        }
        assert!(entries(&dir).is_empty());
    }

    #[test]
    fn failed_rename_names_the_destination_and_leaves_no_temporary() {
        // The temporary file is written, then renaming it onto a folder fails.
        let dir = temp_dir("rename");
        let path = dir.join("c.toml");
        std::fs::create_dir(&path).unwrap();
        match write(&path, "a = 1\n", &mut Vec::new()) {
            Err(crate::errors::Error::ConfigWrite { path: p, .. }) => assert_eq!(p, path),
            other => panic!("expected ConfigWrite, got {other:?}"),
        }
        assert_eq!(entries(&dir), ["c.toml"]);
    }

    #[cfg(unix)]
    #[test]
    fn unix_modes_new_file_private_replaced_file_kept() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let dir = temp_dir("modes");
        let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;

        let fresh = dir.join("fresh.toml");
        write(&fresh, "a = 1\n", &mut Vec::new()).unwrap();
        assert_eq!(mode(&fresh), 0o600);

        for kept in [0o640, 0o644] {
            let path = dir.join(format!("kept-{kept:o}.toml"));
            std::fs::write(&path, "old\n").unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(kept)).unwrap();
            let before = std::fs::metadata(&path).unwrap();
            write(&path, "a = 1\n", &mut Vec::new()).unwrap();
            let after = std::fs::metadata(&path).unwrap();
            assert_eq!(mode(&path), kept);
            assert_eq!((after.uid(), after.gid()), (before.uid(), before.gid()));
        }
    }

    // --- shipped configuration, deploy/config.example.toml (feature 008 T019; feature 015) ---

    const EXAMPLE: &str = include_str!("../deploy/config.example.toml");

    /// The example with every `# key = value` line uncommented.
    fn uncommented_example() -> String {
        EXAMPLE
            .lines()
            .map(|line| {
                let rest = line.trim_start().strip_prefix("# ").unwrap_or("");
                let is_setting = rest.split_once(" = ").is_some_and(|(key, _)| {
                    let key = key.trim_matches('"');
                    !key.is_empty()
                        && key
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
                });
                if is_setting {
                    rest.to_string()
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// `section.key` names from a TOML document, sub-tables included.
    fn keys(table: &toml::Table, prefix: &str, out: &mut Vec<String>) {
        for (key, value) in table {
            let name = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{prefix}.{key}")
            };
            match value.as_table() {
                Some(sub) if !prefix.ends_with("bands") && key != "bands" => keys(sub, &name, out),
                _ => out.push(name),
            }
        }
    }

    #[test]
    fn example_lists_every_exported_setting() {
        let mut config = Config::default();
        config.station.callsign = Some("IS0MZK".into());
        config.station.locator = Some("JM49NH".into());
        config.station.latitude = Some(39.0);
        config.station.longitude = Some(9.0);
        // Credentials for every provider, so their usernames and passwords are required too. The
        // match is exhaustive: a new provider does not compile here until it is handled.
        for kind in Config::default().lookup.provider_order {
            let slot = match kind {
                ProviderKind::Qrz => &mut config.lookup.qrz,
                ProviderKind::HamQth => &mut config.lookup.hamqth,
                ProviderKind::QrzCq => &mut config.lookup.qrzcq,
            };
            *slot = Some(Credentials {
                username: "u".into(),
                password: Secret::new("p"),
            });
        }
        let exported: toml::Table = toml::from_str(&rendered(&config)).unwrap();
        let example: toml::Table = toml::from_str(&uncommented_example()).unwrap();
        let (mut want, mut have) = (Vec::new(), Vec::new());
        keys(&exported, "", &mut want);
        keys(&example, "", &mut have);
        for key in want {
            assert!(
                have.contains(&key),
                "deploy/config.example.toml lacks {key}"
            );
        }
    }

    #[test]
    fn uncommented_example_loads() {
        // Placeholders replaced by valid samples; the style kept, the raster tiles dropped.
        let cli = Cli::try_parse_from(["live-qso-map"]).unwrap();
        Config::load(&cli, &|_| None, Some(&sampled_example())).unwrap();
    }

    // --- the example's content (feature 015, FR-003, FR-004) ---

    /// Settings with no default, shown in the example as a `"<…>"` placeholder.
    const PLACEHOLDER_KEYS: &[&str] = &[
        "station.callsign",
        "station.locator",
        "station.latitude",
        "station.longitude",
        "lookup.qrz.username",
        "lookup.qrz.password",
        "lookup.hamqth.username",
        "lookup.hamqth.password",
        "lookup.qrzcq.username",
        "lookup.qrzcq.password",
    ];

    /// A valid value for each placeholder, by the key's last segment.
    const PLACEHOLDER_SAMPLES: &[(&str, &str)] = &[
        ("callsign", "\"IS0MZK\""),
        ("locator", "\"JM49NH\""),
        ("latitude", "39.3125"),
        ("longitude", "9.125"),
        ("username", "\"user\""),
        ("password", "\"secret\""),
    ];

    /// The uncommented example with every placeholder replaced by its sample, and without the
    /// raster-tile alternative (a style and raster tiles can't be set together).
    fn sampled_example() -> String {
        uncommented_example()
            .lines()
            .filter(|l| !l.starts_with("tile_url") && !l.starts_with("tile_attribution"))
            .map(|line| {
                let placeholder = line.split_once(" = ").and_then(|(key, value)| {
                    let (_, sample) = PLACEHOLDER_SAMPLES.iter().find(|(k, _)| *k == key)?;
                    value
                        .trim_start()
                        .starts_with("\"<")
                        .then(|| format!("{key} = {sample}"))
                });
                placeholder.unwrap_or_else(|| line.to_string())
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The value at a dotted path such as `lookup.qrz.password`.
    fn at<'a>(table: &'a toml::Table, path: &str) -> Option<&'a toml::Value> {
        let mut parts = path.split('.');
        let mut value = table.get(parts.next()?)?;
        for part in parts {
            value = value.as_table()?.get(part)?;
        }
        Some(value)
    }

    /// Every value in a TOML table that is not itself a table.
    fn values(table: &toml::Table) -> Vec<&toml::Value> {
        table
            .values()
            .flat_map(|value| match value.as_table() {
                Some(sub) => values(sub),
                None => vec![value],
            })
            .collect()
    }

    #[test]
    fn example_unchanged_sets_nothing() {
        // Section headers stay uncommented, so an uncommented line lands in its section.
        let parsed: toml::Table = toml::from_str(EXAMPLE).unwrap();
        assert!(values(&parsed).is_empty(), "active settings: {parsed:?}");
        assert_eq!(
            rendered(&load(&[], &[], Some(EXAMPLE))),
            rendered(&load(&[], &[], None))
        );
    }

    #[test]
    fn example_uses_placeholders_where_there_is_no_default() {
        let example: toml::Table = toml::from_str(&uncommented_example()).unwrap();
        for key in PLACEHOLDER_KEYS {
            let value = at(&example, key).and_then(toml::Value::as_str);
            assert!(
                value.is_some_and(|v| v.starts_with('<') && v.ends_with('>')),
                "{key} is {value:?}, wanted a \"<…>\" placeholder"
            );
        }
    }

    #[test]
    fn example_shows_the_defaults() {
        // Band colours are labelled illustrations. The style lines are checked against the
        // defaults below: set explicitly they give the same map, but render differently.
        let mut in_band_colors = false;
        let text = sampled_example()
            .lines()
            .filter(|line| {
                if line.starts_with('[') {
                    in_band_colors = *line == "[map.bands]";
                }
                !in_band_colors
                    && !line.starts_with("style_url")
                    && !line.starts_with("style_attribution")
            })
            .collect::<Vec<_>>()
            .join("\n");
        let mut flags = vec![
            "--station-callsign",
            "IS0MZK",
            "--station-locator",
            "JM49NH",
            "--station-latitude",
            "39.3125",
            "--station-longitude",
            "9.125",
        ];
        for provider in ["qrz", "hamqth", "qrzcq"] {
            let (user, pass) = match provider {
                "qrz" => ("--qrz-username", "--qrz-password"),
                "hamqth" => ("--hamqth-username", "--hamqth-password"),
                _ => ("--qrzcq-username", "--qrzcq-password"),
            };
            flags.extend([user, "user", pass, "secret"]);
        }
        assert_eq!(
            rendered(&load(&[], &[], Some(&text))),
            rendered(&load(&flags, &[], None))
        );

        let example: toml::Table = toml::from_str(&uncommented_example()).unwrap();
        assert_eq!(
            at(&example, "map.style_url").and_then(toml::Value::as_str),
            Some(crate::config::DEFAULT_STYLE_URL)
        );
        assert_eq!(
            at(&example, "map.style_attribution").and_then(toml::Value::as_str),
            Some(crate::config::DEFAULT_STYLE_ATTRIBUTION)
        );
    }

    // --- Debian live configuration, deploy/debian/config.toml (feature 015, FR-010a) ---

    const DEBIAN_LIVE: &str = include_str!("../deploy/debian/config.toml");

    #[test]
    fn debian_live_file_loads() {
        let cli = Cli::try_parse_from(["live-qso-map"]).unwrap();
        assert!(Config::load(&cli, &|_| None, Some(DEBIAN_LIVE)).is_ok());
    }

    #[test]
    fn debian_live_file_sets_the_packaged_paths() {
        let config = load(&[], &[], Some(DEBIAN_LIVE));
        assert_eq!(
            config.country_data.path,
            PathBuf::from("/var/lib/live-qso-map/cty.dat")
        );
        assert_eq!(
            config.history_file,
            Some(PathBuf::from("/var/lib/live-qso-map/qso-history.adi"))
        );
    }

    #[test]
    fn debian_live_file_points_to_the_example() {
        assert!(DEBIAN_LIVE.contains("/usr/share/doc/live-qso-map/config.example.toml"));
    }
}
