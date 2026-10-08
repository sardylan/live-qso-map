#![forbid(unsafe_code)]

use std::process::ExitCode;

use clap::Parser;
use live_qso_map::config::{Cli, Config, config_path};
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
