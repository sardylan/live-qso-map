#![forbid(unsafe_code)]

use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use live_qso_map::config::{
    Cli, Config, ExportAction, config_path, program_folder_base, resolve_data_path,
};
use live_qso_map::{config_export, console};
use tracing_subscriber::EnvFilter;

fn main() -> ExitCode {
    let code = run();
    // Feature 008 (FR-019): keep a double-clicked Windows console readable after an error.
    if console::should_wait(
        code != ExitCode::SUCCESS,
        std::io::stdin().is_terminal(),
        cfg!(windows),
    ) {
        console::wait_for_enter();
    }
    code
}

/// The folder holding the running program, if it can be determined.
fn program_folder() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
}

fn run() -> ExitCode {
    // `try_parse`, not `parse`: a usage error must return here so `main` can still pause.
    // Help and version print and exit with code 0, so they never pause.
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            // Printing fails only when stdout/stderr are gone; the exit code still reports it.
            let _ = e.print();
            return ExitCode::from(u8::try_from(e.exit_code()).unwrap_or(2));
        }
    };
    let env = |key: &str| std::env::var(key).ok();
    let exe_dir = program_folder();

    let file_text = match config_path(&cli, &env, exe_dir.as_deref(), &|p| p.is_file()) {
        Some(path) => match std::fs::read_to_string(&path) {
            Ok(text) => Some(text),
            Err(e) => {
                eprintln!("cannot read configuration file {}: {e}", path.display());
                return ExitCode::FAILURE;
            }
        },
        None => None,
    };
    let (mut config, mut warnings) = match Config::load(&cli, &env, file_text.as_deref()) {
        Ok(loaded) => loaded,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    // Feature 008: write the validated configuration before anything is bound.
    if let Some(action) = cli.export_action() {
        let (ExportAction::Save(path) | ExportAction::Dump(path)) = &action;
        if let Err(code) = export(&config, path) {
            return code;
        }
        if matches!(action, ExportAction::Save(_)) {
            return ExitCode::SUCCESS;
        }
    }

    // After the export, which keeps relative paths as configured (feature 008, R10).
    if program_folder_base() {
        if exe_dir.is_none() {
            warnings.push(
                "cannot determine the program's folder; data paths are relative to the current folder"
                    .to_string(),
            );
        }
        config.country_data.path =
            resolve_data_path(&config.country_data.path, exe_dir.as_deref(), true);
        // Feature 014 (R1): the history file follows the same rule.
        config.history_file = config
            .history_file
            .map(|path| resolve_data_path(&path, exe_dir.as_deref(), true));
    }

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(format!(
            "warn,live_qso_map={}",
            config.log_level
        )))
        .init();
    for warning in warnings {
        tracing::warn!("{warning}");
    }

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(e) => {
            tracing::error!("cannot start the async runtime: {e}");
            return ExitCode::FAILURE;
        }
    };
    // Feature 014: complete a file and exit, with no listener.
    if let Some(file) = cli.enrich_only {
        let Some(path) = file.or_else(|| config.history_file.clone()) else {
            tracing::error!("no history file to enrich");
            return ExitCode::FAILURE;
        };
        let run = live_qso_map::pipeline::enrich_only(
            config,
            &path,
            None,
            None,
            live_qso_map::pipeline::wait_for_signal(),
        );
        return match runtime.block_on(run) {
            Ok(_) => ExitCode::SUCCESS,
            Err(e) => {
                tracing::error!("{e}");
                ExitCode::FAILURE
            }
        };
    }
    match runtime.block_on(live_qso_map::run(config)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            tracing::error!("{e}");
            ExitCode::FAILURE
        }
    }
}

/// Writes the effective configuration to `path` (`-` = standard output) and reports it.
fn export(config: &Config, path: &std::path::Path) -> Result<(), ExitCode> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default();
    let text = config_export::render(
        config,
        env!("CARGO_PKG_VERSION"),
        &live_qso_map::history::format_rfc3339(now),
    );
    if let Err(e) = config_export::write(path, &text, &mut std::io::stdout()) {
        eprintln!("{e}");
        return Err(ExitCode::FAILURE);
    }
    if path != std::path::Path::new("-") {
        eprintln!("configuration written to {}", path.display());
        if config_export::has_credentials(config) {
            eprintln!("note: {} contains provider passwords", path.display());
        }
    }
    Ok(())
}
