#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod adif;
pub mod band;
pub mod config;
pub mod enrich;
pub mod errors;
pub mod history;
pub mod ingest;
pub mod maidenhead;
pub mod mode;
pub mod pipeline;
pub mod position;
pub mod web;

pub use pipeline::run;
