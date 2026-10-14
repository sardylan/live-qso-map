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

/// Counts of one load, logged at the end of it (data-model "LoadReport").
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LoadReport {
    /// Records kept: reused, to enrich and parked.
    pub loaded: usize,
    /// Malformed records, and records without a valid `CALL`, `QSO_DATE` and `TIME_ON`.
    pub skipped: usize,
    pub reused: usize,
    pub to_enrich: usize,
    pub parked: usize,
}
