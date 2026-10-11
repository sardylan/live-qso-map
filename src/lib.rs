#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod adif;
pub mod band;
pub mod band_filter;
pub mod config;
pub mod config_export;
pub mod console;
pub mod enrich;
pub mod entity_extents;
pub mod errors;
pub mod history;
pub mod ingest;
pub mod maidenhead;
pub mod mode;
pub mod pipeline;
pub mod position;
pub mod recent_columns;
pub mod web;
pub mod xml;

pub use pipeline::{run, start};

/// Installs the `ring` rustls crypto provider used by every HTTPS client (research R17).
/// Safe to call more than once.
pub fn install_crypto_provider() {
    // An error only means a provider is already installed.
    let _ = rustls::crypto::ring::default_provider().install_default();
}
