#![forbid(unsafe_code)]

use std::process::ExitCode;

use clap::Parser;
use live_qso_map::config::{Cli, Config, ExportAction, config_path};
use live_qso_map::config_export;
use tracing_subscriber::EnvFilter;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let env = |key: &str| std::env::var(key).ok();

    let file_text = match config_path(&cli, &env) {
        Some(path) => match std::fs::read_to_string(&path) {
            Ok(text) => Some(text),
            Err(e) => {
                eprintln!("cannot read configuration file {}: {e}", path.display());
                return ExitCode::FAILURE;
            }
        },
        None => None,
    };
    let (config, warnings) = match Config::load(&cli, &env, file_text.as_deref()) {
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
