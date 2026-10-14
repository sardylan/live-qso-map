//! Feature 008: help and usage errors through the real binary (contracts/command-line.md).

use std::path::Path;
use std::process::{Command, Output};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn run(args: &[&str], env: &[(&str, &str)]) -> Result<Output, std::io::Error> {
    Command::new(env!("CARGO_BIN_EXE_live-qso-map"))
        .args(args)
        .envs(env.iter().copied())
        .output()
}

#[test]
fn piped_help_with_no_color_has_no_escape_codes() -> TestResult {
    let out = run(&["--help"], &[("NO_COLOR", "1")])?;
    assert_eq!(out.status.code(), Some(0));
    let text = String::from_utf8(out.stdout)?;
    assert!(text.contains("--udp-port"));
    assert!(!text.contains("\x1b["), "colour codes in piped help");
    Ok(())
}

#[test]
fn short_help_exits_zero() -> TestResult {
    let out = run(&["-h"], &[])?;
    assert_eq!(out.status.code(), Some(0));
    Ok(())
}

#[test]
fn typo_is_a_usage_error_with_a_suggestion() -> TestResult {
    let out = run(&["--udp-prot", "1"], &[])?;
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8(out.stderr)?.contains("--udp-port"));
    Ok(())
}

#[test]
fn bad_option_value_is_a_usage_error() -> TestResult {
    let out = run(&["--udp-port", "not-a-port", "--save-config", "-"], &[])?;
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8(out.stderr)?;
    assert!(stderr.contains("not-a-port"), "no clap message: {stderr}");
    assert!(out.stdout.is_empty(), "something was written");
    Ok(())
}

#[test]
fn version_exits_zero() -> TestResult {
    let out = run(&["--version"], &[])?;
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8(out.stdout)?.starts_with("live-qso-map "));
    Ok(())
}

#[test]
fn help_lists_the_history_options_of_feature_014() -> TestResult {
    let out = run(&["--help"], &[("NO_COLOR", "1")])?;
    let text = String::from_utf8(out.stdout)?;
    assert!(text.contains("--history-file <PATH>"), "{text}");
    assert!(text.contains("--enrich-only [<FILE>]"), "{text}");
    assert!(
        !text.contains("--history-size"),
        "removed option still in help"
    );
    Ok(())
}

#[test]
fn enrich_only_on_a_missing_file_fails_with_exit_code_1() -> TestResult {
    let dir = std::env::temp_dir().join(format!("lqm-cli-enrich-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let missing = dir.join("missing.adi");
    let missing = missing.to_string_lossy();
    let out = run(&["--enrich-only", &missing], &[("NO_COLOR", "1")])?;
    assert_eq!(out.status.code(), Some(1));
    let text = String::from_utf8(out.stdout)? + &String::from_utf8(out.stderr)?;
    assert!(text.contains("history file"), "{text}");
    assert!(!Path::new(missing.as_ref()).exists());
    Ok(())
}

#[test]
fn enrich_only_and_save_config_conflict() -> TestResult {
    let out = run(&["--enrich-only", "--save-config", "-"], &[])?;
    assert_eq!(out.status.code(), Some(2));
    Ok(())
}
