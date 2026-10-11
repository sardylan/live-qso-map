//! Feature 008: `--save-config` / `--dump-config` through the real binary (contracts/command-line.md).

use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{Duration, Instant};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn temp_dir(name: &str) -> Result<PathBuf, std::io::Error> {
    let dir = std::env::temp_dir().join(format!("lqm-it-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Runs the binary with a clean `LQM_*` environment and no file next to the program.
fn run(args: &[&str], env: &[(&str, &str)]) -> Result<Output, std::io::Error> {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_live-qso-map"));
    for (key, _) in std::env::vars() {
        if key.starts_with("LQM_") {
            cmd.env_remove(key);
        }
    }
    cmd.args(args).envs(env.iter().copied()).output()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn save_writes_the_file_and_exits_at_once() -> TestResult {
    let dir = temp_dir("save")?;
    let path = dir.join("a.toml");
    let path_text = path.to_string_lossy().into_owned();
    let start = Instant::now();
    let out = run(
        &[
            "--station-callsign",
            "IS0MZK",
            "--bands",
            "hf-contest",
            "--save-config",
            &path_text,
        ],
        &[],
    )?;
    assert!(start.elapsed() < Duration::from_secs(5));
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert!(text(&out.stderr).contains("configuration written to"));
    assert!(std::fs::read_to_string(&path)?.contains("callsign = \"IS0MZK\""));
    Ok(())
}

#[test]
fn dash_prints_to_stdout() -> TestResult {
    let out = run(&["--station-callsign", "IS0MZK", "--save-config", "-"], &[])?;
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let stdout = text(&out.stdout);
    assert!(stdout.starts_with("# live-qso-map "));
    assert!(stdout.contains("callsign = \"IS0MZK\""));
    Ok(())
}

#[test]
fn invalid_configuration_writes_nothing() -> TestResult {
    let dir = temp_dir("invalid")?;
    let path = dir.join("b.toml");
    let out = run(
        &["--bands", "nope", "--save-config", &path.to_string_lossy()],
        &[],
    )?;
    assert_eq!(out.status.code(), Some(1));
    assert!(!path.exists());
    Ok(())
}

#[test]
fn unwritable_destination_fails_with_the_path() -> TestResult {
    let dir = temp_dir("unwritable")?;
    let path = dir.join("missing").join("c.toml");
    let out = run(&["--save-config", &path.to_string_lossy()], &[])?;
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("cannot write configuration file"));
    Ok(())
}

#[test]
fn save_and_dump_together_is_a_usage_error() -> TestResult {
    let out = run(&["--save-config", "x", "--dump-config", "y"], &[])?;
    assert_eq!(out.status.code(), Some(2));
    Ok(())
}

#[test]
fn credentials_trigger_the_password_note() -> TestResult {
    let dir = temp_dir("passwords")?;
    let path = dir.join("d.toml");
    let out = run(
        &["--save-config", &path.to_string_lossy()],
        &[("LQM_QRZ_USERNAME", "u"), ("LQM_QRZ_PASSWORD", "p")],
    )?;
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert!(text(&out.stderr).contains("contains provider passwords"));
    Ok(())
}
