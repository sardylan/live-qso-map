//! The QSO history file: one ADIF file read at startup and replaced at a graceful stop
//! (feature 014, contracts/history-file.md).

pub mod read;
pub mod restore;
pub mod write;

/// Names of the fields the service added to a record, comma-separated in name order (R3).
/// Removing them gives the record as received.
pub const ADDED_FIELD: &str = "APP_LIVEQSOMAP_ADDED";

/// Present when every online lookup failed on the logged call: `source:kind[,…]` (R7).
pub const LOOKUP_FAILED_FIELD: &str = "APP_LIVEQSOMAP_LOOKUP_FAILED";

/// Rejected positions (feature 010).
pub const POS_REJECTED_FIELD: &str = "APP_LIVEQSOMAP_POS_REJECTED";

/// The QARTest logger id (feature 001).
pub const LOGGER_ID_FIELD: &str = "APP_LIVEQSOMAP_LOGGER_ID";
