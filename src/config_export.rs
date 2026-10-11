//! Writes the effective configuration to a file or standard output (feature 008, research R9).

use std::io::Write;
use std::path::Path;

use crate::config::Config;
use crate::errors::Result;

/// Renders `config` in the configuration file format, with a header comment.
pub fn render(_config: &Config, _version: &str, _now_utc: &str) -> String {
    String::new()
}

/// Writes `text` to `path` atomically, or to `stdout` when `path` is `-`.
pub fn write(_path: &Path, _text: &str, _stdout: &mut dyn Write) -> Result<()> {
    Ok(())
}
