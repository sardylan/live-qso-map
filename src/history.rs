//! In-memory QSO history: revision-checked upserts, stable sort keys, eviction
//! (FR-021, FR-028, data-model "History").

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
}

#[derive(Debug)]
pub enum Upsert {
    Accepted(Arc<EnrichedQso>),
    Stale,
}

#[derive(Debug, Default)]
pub struct History {
    by_id: HashMap<String, Arc<EnrichedQso>>,
    ordered: BTreeSet<(i64, String)>,
    capacity: usize,
}

impl History {
    /// `capacity` 0 means unbounded.
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            ..Self::default()
        }
    }

    pub fn upsert(&mut self, mut qso: EnrichedQso) -> Upsert {
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
        if self.capacity > 0 && self.by_id.len() >= self.capacity {
            match self.ordered.first() {
                Some(oldest) if *oldest > key => return Upsert::Stale,
                _ => {}
            }
        }
        let stored = Arc::new(qso);
        self.ordered.insert(key);
        self.by_id.insert(stored.id.clone(), Arc::clone(&stored));
        while self.capacity > 0 && self.by_id.len() > self.capacity {
            if let Some((_, id)) = self.ordered.pop_first() {
                self.by_id.remove(&id);
            }
        }
        Upsert::Accepted(stored)
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
        }
    }

    fn ids(items: &[Arc<EnrichedQso>]) -> Vec<&str> {
        items.iter().map(|q| q.id.as_str()).collect()
    }

    #[test]
    fn new_id_is_accepted() {
        let mut h = History::new(0);
        assert!(matches!(h.upsert(qso("A", 1, 10)), Upsert::Accepted(_)));
        assert_eq!(h.len(), 1);
    }

    #[test]
    fn higher_rev_is_accepted_and_keeps_sort_key() {
        let mut h = History::new(0);
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
        let mut h = History::new(0);
        h.upsert(qso("A", 2, 10));
        assert!(matches!(h.upsert(qso("A", 2, 10)), Upsert::Stale));
        assert!(matches!(h.upsert(qso("A", 1, 10)), Upsert::Stale));
        assert_eq!(h.get("A").unwrap().rev, 2);
    }

    #[test]
    fn latest_returns_last_n_ascending() {
        let mut h = History::new(0);
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
        let mut h = History::new(0);
        h.upsert(qso("B", 1, 10));
        h.upsert(qso("A", 1, 10));
        assert_eq!(ids(&h.latest(None)), ["A", "B"]);
    }

    #[test]
    fn capacity_evicts_oldest() {
        let mut h = History::new(3);
        for (id, ts) in [("A", 10), ("B", 20), ("C", 30), ("D", 40)] {
            h.upsert(qso(id, 1, ts));
        }
        assert_eq!(h.len(), 3);
        assert_eq!(ids(&h.latest(None)), ["B", "C", "D"]);
        assert!(h.get("A").is_none());
    }

    #[test]
    fn insert_older_than_everything_in_full_history_is_stale() {
        let mut h = History::new(2);
        h.upsert(qso("B", 1, 20));
        h.upsert(qso("C", 1, 30));
        assert!(matches!(h.upsert(qso("A", 1, 10)), Upsert::Stale));
        assert_eq!(ids(&h.latest(None)), ["B", "C"]);
    }

    #[test]
    fn capacity_zero_never_evicts() {
        let mut h = History::new(0);
        for i in 0..100 {
            h.upsert(qso(&format!("Q{i:03}"), 1, i));
        }
        assert_eq!(h.len(), 100);
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
}
