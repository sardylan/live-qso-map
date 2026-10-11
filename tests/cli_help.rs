//! Feature 008: help and usage errors through the real binary (contracts/command-line.md).

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
