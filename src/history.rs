//! In-memory QSO history: revision-checked upserts, stable sort keys, eviction
//! (FR-021, FR-028, data-model "History").

use serde::Serialize;
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use crate::adif::AdifRecord;
use crate::position::Position;

#[derive(Debug, Clone, PartialEq)]
pub struct EnrichedQso {
    /// Canonical identity.
    pub id: String,
    pub rev: u32,
    /// QSO start time (unix seconds) when first seen; corrections never change it.
    pub sort_ts: i64,
    pub record: AdifRecord,
    pub remote: Option<Position>,
    pub station: Option<Position>,
    /// DXCC/WAE entity of the remote callsign, from cty.dat (feature 005).
    pub entity: Option<QsoEntity>,
}

/// A DXCC/WAE entity: cty.dat primary prefix (without the WAE `*`) and name (HR-6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QsoEntity {
    pub prefix: String,
    pub name: String,
    /// CQ zone from cty.dat, with per-callsign overrides (feature 006).
    pub cq_zone: u8,
}

#[derive(Debug)]
pub enum Upsert {
    Accepted(Arc<EnrichedQso>),
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Removal {
    Removed,
    NotPresent,
}

#[derive(Debug, Default)]
pub struct History {
    by_id: HashMap<String, Arc<EnrichedQso>>,
    ordered: BTreeSet<(i64, String)>,
    /// Removed ids and the revision of their removal (feature 003, TR-002): older revisions still
    /// in flight must not bring the QSO back.
    tombstones: HashMap<String, u32>,
}

impl History {
    /// An empty history. It is never trimmed: every QSO stays until it is removed (feature 014,
    /// FR-023).
    pub fn new() -> Self {
        Self::default()
    }

    pub fn upsert(&mut self, mut qso: EnrichedQso) -> Upsert {
        if let Some(removed_at) = self.tombstones.get(&qso.id) {
            if qso.rev <= *removed_at {
                return Upsert::Stale;
            }
            self.tombstones.remove(&qso.id);
        }
        if let Some(existing) = self.by_id.get(&qso.id) {
            if qso.rev <= existing.rev {
                return Upsert::Stale;
            }
            qso.sort_ts = existing.sort_ts;
            let stored = Arc::new(qso);
            self.by_id.insert(stored.id.clone(), Arc::clone(&stored));
            return Upsert::Accepted(stored);
        }

        let key = (qso.sort_ts, qso.id.clone());
        let stored = Arc::new(qso);
        self.ordered.insert(key);
        self.by_id.insert(stored.id.clone(), Arc::clone(&stored));
        Upsert::Accepted(stored)
    }

    /// Removes `id` when `rev` is newer than the stored revision, and remembers `rev` so stale
    /// revisions stay out (research R3).
    pub fn remove(&mut self, id: &str, rev: u32) -> Removal {
        let removed = match self.by_id.get(id) {
            Some(existing) if rev > existing.rev => {
                let key = (existing.sort_ts, existing.id.clone());
                self.ordered.remove(&key);
                self.by_id.remove(id);
                Removal::Removed
            }
            _ => Removal::NotPresent,
        };
        let entry = self.tombstones.entry(id.to_string()).or_insert(0);
        *entry = (*entry).max(rev);
        removed
    }

    /// The last `n` QSOs by sort key (`None` = all), ascending.
    pub fn latest(&self, n: Option<usize>) -> Vec<Arc<EnrichedQso>> {
        let skip = n.map_or(0, |n| self.ordered.len().saturating_sub(n));
        self.ordered
            .iter()
            .skip(skip)
            .filter_map(|(_, id)| self.by_id.get(id).cloned())
            .collect()
    }

    /// The newest `n` QSOs (all when `None`) whose `BAND` is one of `bands`, oldest first
    /// (feature 011, FR-013). QSOs without a `BAND` are left out. `bands` of `None` means every QSO,
    /// as [`History::latest`].
    pub fn latest_matching(
        &self,
        n: Option<usize>,
        bands: Option<&[String]>,
    ) -> Vec<Arc<EnrichedQso>> {
        let Some(bands) = bands else {
            return self.latest(n);
        };
        let matches = |q: &EnrichedQso| {
            q.record
                .get("BAND")
                .is_some_and(|band| bands.iter().any(|b| b.eq_ignore_ascii_case(band.trim())))
        };
        let mut newest: Vec<Arc<EnrichedQso>> = self
            .ordered
            .iter()
            .rev()
            .filter_map(|(_, id)| self.by_id.get(id))
            .filter(|q| matches(q))
            .take(n.unwrap_or(usize::MAX))
            .cloned()
            .collect();
        newest.reverse();
        newest
    }

    /// Every QSO, oldest first by sort key (ties by id), for the history file (feature 014).
    pub fn iter_in_order(&self) -> impl Iterator<Item = &Arc<EnrichedQso>> {
        self.ordered.iter().filter_map(|(_, id)| self.by_id.get(id))
    }

    pub fn get(&self, id: &str) -> Option<Arc<EnrichedQso>> {
        self.by_id.get(id).cloned()
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

/// Parses ADIF `QSO_DATE` (`YYYYMMDD`) and `TIME_ON` (`HHMM[SS]`) as UTC unix seconds.
pub fn qso_timestamp(date: &str, time: &str) -> Option<i64> {
    let date = date.trim();
    let time = time.trim();
    if date.len() != 8 || !(time.len() == 4 || time.len() == 6) {
        return None;
    }
    if !date.bytes().chain(time.bytes()).all(|b| b.is_ascii_digit()) {
        return None;
    }
    let num = |s: &str| s.parse::<i64>().ok();
    let (year, month, day) = (num(&date[0..4])?, num(&date[4..6])?, num(&date[6..8])?);
    let (hour, minute) = (num(&time[0..2])?, num(&time[2..4])?);
    let second = if time.len() == 6 {
        num(&time[4..6])?
    } else {
        0
    };
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    Some(days_from_civil(year, month, day) * 86_400 + hour * 3_600 + minute * 60 + second)
}

/// Formats unix seconds as `YYYY-MM-DDTHH:MM:SSZ`.
pub fn format_rfc3339(ts: i64) -> String {
    let days = ts.div_euclid(86_400);
    let secs = ts.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        secs / 3_600,
        secs % 3_600 / 60,
        secs % 60
    )
}

// Howard Hinnant's civil-date algorithms.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qso(id: &str, rev: u32, ts: i64) -> EnrichedQso {
        let mut record = AdifRecord::new();
        record.set("CALL", id);
        EnrichedQso {
            id: id.to_string(),
            rev,
            sort_ts: ts,
            record,
            remote: None,
            station: None,
            entity: None,
        }
    }

    fn ids(items: &[Arc<EnrichedQso>]) -> Vec<&str> {
        items.iter().map(|q| q.id.as_str()).collect()
    }

    #[test]
    fn new_id_is_accepted() {
        let mut h = History::new();
        assert!(matches!(h.upsert(qso("A", 1, 10)), Upsert::Accepted(_)));
        assert_eq!(h.len(), 1);
    }

    #[test]
    fn higher_rev_is_accepted_and_keeps_sort_key() {
        let mut h = History::new();
        h.upsert(qso("A", 1, 10));
        h.upsert(qso("B", 1, 20));
        let Upsert::Accepted(stored) = h.upsert(qso("A", 2, 99)) else {
            panic!("expected accepted");
        };
        assert_eq!(stored.rev, 2);
        assert_eq!(stored.sort_ts, 10);
        assert_eq!(ids(&h.latest(None)), ["A", "B"]);
        assert_eq!(h.len(), 2);
    }

    #[test]
    fn same_or_lower_rev_is_stale() {
        let mut h = History::new();
        h.upsert(qso("A", 2, 10));
        assert!(matches!(h.upsert(qso("A", 2, 10)), Upsert::Stale));
        assert!(matches!(h.upsert(qso("A", 1, 10)), Upsert::Stale));
        assert_eq!(h.get("A").unwrap().rev, 2);
    }

    #[test]
    fn latest_returns_last_n_ascending() {
        let mut h = History::new();
        for (i, id) in ["C", "A", "E", "B", "D"].iter().enumerate() {
            h.upsert(qso(id, 1, [30, 10, 50, 20, 40][i]));
        }
        assert_eq!(ids(&h.latest(Some(3))), ["C", "D", "E"]);
        assert_eq!(ids(&h.latest(None)), ["A", "B", "C", "D", "E"]);
        assert_eq!(ids(&h.latest(Some(10))), ["A", "B", "C", "D", "E"]);
        assert!(h.latest(Some(0)).is_empty());
    }

    #[test]
    fn equal_timestamps_order_by_id() {
        let mut h = History::new();
        h.upsert(qso("B", 1, 10));
        h.upsert(qso("A", 1, 10));
        assert_eq!(ids(&h.latest(None)), ["A", "B"]);
    }

    #[test]
    fn never_evicts() {
        let mut h = History::new();
        for i in 0..50_000 {
            h.upsert(qso(&format!("Q{i:05}"), 1, i));
        }
        assert_eq!(h.len(), 50_000);
        assert!(matches!(h.upsert(qso("OLD", 1, -1)), Upsert::Accepted(_)));
        assert_eq!(h.len(), 50_001);
    }

    #[test]
    fn iter_in_order_is_oldest_first_ties_by_id() {
        let mut h = History::new();
        for (id, ts) in [("C", 30), ("B", 10), ("A", 10), ("D", 20)] {
            h.upsert(qso(id, 1, ts));
        }
        let order: Vec<&str> = h.iter_in_order().map(|q| q.id.as_str()).collect();
        assert_eq!(order, ["A", "B", "D", "C"]);
    }

    #[test]
    fn qso_timestamp_parsing() {
        assert_eq!(qso_timestamp("19700101", "0000"), Some(0));
        assert_eq!(qso_timestamp("20261008", "150404"), Some(1_791_471_844));
        assert_eq!(qso_timestamp("20261008", "1504"), Some(1_791_471_840));
        assert_eq!(qso_timestamp("2026108", "1504"), None);
        assert_eq!(qso_timestamp("20261308", "1504"), None);
        assert_eq!(qso_timestamp("20261008", "2504"), None);
        assert_eq!(qso_timestamp("20261008", "15"), None);
    }

    #[test]
    fn rfc3339_formatting() {
        assert_eq!(format_rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_rfc3339(1_791_471_844), "2026-10-08T15:04:04Z");
    }

    // --- removals and tombstones (feature 003, T016) ------------------------------------------

    #[test]
    fn remove_deletes_and_tombstones() {
        let mut h = History::new();
        h.upsert(qso("A", 1, 10));
        h.upsert(qso("B", 1, 20));
        assert_eq!(h.remove("A", 2), Removal::Removed);
        assert_eq!(h.len(), 1);
        assert!(h.get("A").is_none());
        assert_eq!(ids(&h.latest(None)), ["B"]);
    }

    #[test]
    fn stale_upsert_after_removal_is_rejected() {
        let mut h = History::new();
        h.upsert(qso("A", 1, 10));
        h.remove("A", 2);
        assert!(matches!(h.upsert(qso("A", 1, 10)), Upsert::Stale));
        assert!(matches!(h.upsert(qso("A", 2, 10)), Upsert::Stale));
        assert_eq!(h.len(), 0);
    }

    #[test]
    fn higher_rev_after_removal_comes_back_as_new() {
        let mut h = History::new();
        h.upsert(qso("A", 1, 10));
        h.remove("A", 2);
        let Upsert::Accepted(stored) = h.upsert(qso("A", 3, 99)) else {
            panic!("expected accepted");
        };
        assert_eq!(stored.sort_ts, 99, "a new entry keeps its own sort key");
        assert_eq!(h.len(), 1);
        h.remove("A", 4);
        assert_eq!(h.len(), 0);
    }

    #[test]
    fn remove_of_unknown_id_records_a_tombstone() {
        let mut h = History::new();
        assert_eq!(h.remove("X", 2), Removal::NotPresent);
        assert!(matches!(h.upsert(qso("X", 1, 10)), Upsert::Stale));
        assert!(matches!(h.upsert(qso("X", 3, 10)), Upsert::Accepted(_)));
    }

    #[test]
    fn tombstones_are_never_dropped() {
        let mut h = History::new();
        for i in 0..1000 {
            h.remove(&format!("Q{i}"), 1);
        }
        assert!(matches!(h.upsert(qso("Q0", 1, 0)), Upsert::Stale));
    }

    // --- latest on chosen bands (feature 011, T020) ----------------------------------------

    fn on_band(id: &str, ts: i64, band: Option<&str>) -> EnrichedQso {
        let mut q = qso(id, 1, ts);
        if let Some(band) = band {
            q.record.set("BAND", band);
        }
        q
    }

    #[test]
    fn latest_matching_keeps_only_the_chosen_bands() {
        let mut h = History::new();
        for (id, ts, band) in [
            ("A", 10, Some("40m")),
            ("B", 20, Some("30M")),
            ("C", 30, Some("40M")),
            ("D", 40, None),
            ("E", 50, Some("20m")),
            ("F", 60, Some("30m")),
        ] {
            h.upsert(on_band(id, ts, band));
        }
        let bands = vec!["40m".to_string(), "20m".to_string()];
        assert_eq!(ids(&h.latest_matching(Some(2), Some(&bands))), ["C", "E"]);
        assert_eq!(ids(&h.latest_matching(None, Some(&bands))), ["A", "C", "E"]);
        assert_eq!(
            ids(&h.latest_matching(Some(3), None)),
            ids(&h.latest(Some(3))),
            "no band list means every QSO"
        );
        assert!(h.latest_matching(Some(0), Some(&bands)).is_empty());
    }
}
