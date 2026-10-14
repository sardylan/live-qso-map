//! Resend gate: drops identical resends before enrichment and numbers corrections
//! (FR-008, FR-009, research R3, data-model "ResendGate").

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::adif::AdifRecord;
use crate::ingest::identity::Identity;
use crate::ingest::{EventKind, QsoEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateDecision {
    Forward { rev: u32 },
    Drop,
}

#[derive(Debug, Clone)]
struct GateEntry {
    fingerprint: u64,
    rev: u32,
    /// The history id, fixed when the QSO was first seen.
    id: String,
}

#[derive(Debug, Default)]
pub struct ResendGate {
    entries: HashMap<Identity, GateEntry>,
}

/// Hash of the normalised record before enrichment; fields are iterated in name order.
pub fn fingerprint(record: &AdifRecord) -> u64 {
    let mut hasher = DefaultHasher::new();
    for (name, value) in record.iter() {
        name.hash(&mut hasher);
        value.hash(&mut hasher);
    }
    hasher.finish()
}

/// Splits a QARTest-style `YYYY-MM-DD HH:MM:SS` (or ADIF `YYYYMMDD HHMM[SS]`) timestamp into
/// ADIF `QSO_DATE` and `TIME_ON`.
pub fn split_timestamp(text: &str) -> Option<(String, String)> {
    let (date, time) = text.trim().split_once(' ')?;
    let date: String = date.chars().filter(char::is_ascii_digit).collect();
    let time: String = time.chars().filter(char::is_ascii_digit).collect();
    (date.len() == 8 && (time.len() == 4 || time.len() == 6)).then_some((date, time))
}

impl ResendGate {
    /// An empty gate. Like the history, it is never trimmed (feature 014, FR-023).
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Decides whether `event` goes on to enrichment, and sets its `rev` and history `id`.
    pub fn check(&mut self, event: &mut QsoEvent) -> GateDecision {
        let fp = fingerprint(&event.record);

        if let Some(entry) = self.entries.get_mut(&event.identity) {
            if entry.fingerprint == fp {
                return GateDecision::Drop;
            }
            entry.fingerprint = fp;
            entry.rev += 1;
            event.rev = entry.rev;
            event.id = entry.id.clone();
            return GateDecision::Forward { rev: entry.rev };
        }

        if let Some(old) = self.previous_identity(event)
            && let Some(mut entry) = self.entries.remove(&old)
        {
            entry.fingerprint = fp;
            entry.rev += 1;
            event.rev = entry.rev;
            event.id = entry.id.clone();
            self.insert(event.identity.clone(), entry);
            return GateDecision::Forward { rev: event.rev };
        }

        event.rev = 1;
        self.insert(
            event.identity.clone(),
            GateEntry {
                fingerprint: fp,
                rev: 1,
                id: event.id.clone(),
            },
        );
        GateDecision::Forward { rev: 1 }
    }

    /// Whether the gate knows this QSO, by its identity or, for a correction without a logger
    /// id, by its previous identity (feature 003: candidates for removal).
    pub fn contains(&self, event: &QsoEvent) -> bool {
        self.entries.contains_key(&event.identity)
            || self
                .previous_identity(event)
                .is_some_and(|old| self.entries.contains_key(&old))
    }

    /// The identity a correction had before the edit, from `oldcall`/`oldtimestamp`.
    /// Only used when the source carries no logger id.
    fn previous_identity(&self, event: &QsoEvent) -> Option<Identity> {
        if event.kind != EventKind::Correction || event.logger_id.is_some() {
            return None;
        }
        let (old_call, old_timestamp) = event.previous.as_ref()?;
        let (date, time) = split_timestamp(old_timestamp)?;
        let mut old = event.record.clone();
        old.set("CALL", old_call.clone());
        old.set("QSO_DATE", date);
        old.set("TIME_ON", time);
        let station = match &event.identity {
            Identity::Composite { station_call, .. } | Identity::Logger { station_call, .. } => {
                station_call.clone()
            }
        };
        Some(Identity::from_record(&old, None, Some(&station)))
    }

    fn insert(&mut self, identity: Identity, entry: GateEntry) {
        self.entries.insert(identity, entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::StationConfig;
    use crate::ingest::{EventKind, QsoEvent};

    fn station() -> StationConfig {
        StationConfig {
            callsign: Some("IS0MZK".into()),
            ..StationConfig::default()
        }
    }

    fn record(call: &str, mode: &str, extra: &[(&str, &str)]) -> AdifRecord {
        let mut r = AdifRecord::new();
        for (k, v) in [
            ("STATION_CALLSIGN", "IS0MZK"),
            ("CALL", call),
            ("BAND", "40m"),
            ("MODE", mode),
            ("QSO_DATE", "20261008"),
            ("TIME_ON", "150422"),
        ]
        .iter()
        .chain(extra)
        {
            r.set(k, *v);
        }
        r
    }

    fn event(record: AdifRecord) -> QsoEvent {
        QsoEvent::new(EventKind::New, record, None, &station())
    }

    fn correction(record: AdifRecord, old_call: &str, old_timestamp: &str) -> QsoEvent {
        let mut e = QsoEvent::new(EventKind::Correction, record, None, &station());
        e.previous = Some((old_call.to_string(), old_timestamp.to_string()));
        e
    }

    #[test]
    fn unknown_identity_is_forwarded_as_rev_1() {
        let mut gate = ResendGate::new();
        let mut e = event(record("K6QA", "SSB", &[]));
        assert_eq!(gate.check(&mut e), GateDecision::Forward { rev: 1 });
        assert_eq!(e.rev, 1);
    }

    #[test]
    fn identical_resend_is_dropped() {
        let mut gate = ResendGate::new();
        gate.check(&mut event(record("K6QA", "SSB", &[])));
        assert_eq!(
            gate.check(&mut event(record("K6QA", "SSB", &[]))),
            GateDecision::Drop
        );
    }

    #[test]
    fn changed_content_is_a_new_revision() {
        let mut gate = ResendGate::new();
        gate.check(&mut event(record("K6QA", "SSB", &[("SRX_STRING", "3")])));
        let mut e = event(record("K6QA", "SSB", &[("SRX_STRING", "15")]));
        assert_eq!(gate.check(&mut e), GateDecision::Forward { rev: 2 });
        assert_eq!(e.rev, 2);
        let mut back = event(record("K6QA", "SSB", &[("SRX_STRING", "3")]));
        assert_eq!(gate.check(&mut back), GateDecision::Forward { rev: 3 });
    }

    #[test]
    fn fingerprint_ignores_field_order() {
        let mut a = AdifRecord::new();
        a.set("CALL", "K1A");
        a.set("BAND", "20m");
        let mut b = AdifRecord::new();
        b.set("BAND", "20m");
        b.set("CALL", "K1A");
        assert_eq!(fingerprint(&a), fingerprint(&b));
        b.set("MODE", "CW");
        assert_ne!(fingerprint(&a), fingerprint(&b));
    }

    #[test]
    fn correction_without_logger_id_is_rekeyed_by_old_identity() {
        let mut gate = ResendGate::new();
        let mut original = event(record("K6QA", "SSB", &[]));
        gate.check(&mut original);
        let original_id = original.id.clone();

        let mut fixed = correction(record("K6QB", "SSB", &[]), "K6QA", "2026-10-08 15:04:22");
        assert_eq!(gate.check(&mut fixed), GateDecision::Forward { rev: 2 });
        assert_eq!(fixed.id, original_id, "the history entry keeps its id");

        let mut again = correction(record("K6QC", "SSB", &[]), "K6QB", "2026-10-08 15:04:22");
        assert_eq!(gate.check(&mut again), GateDecision::Forward { rev: 3 });
        assert_eq!(again.id, original_id);
    }

    #[test]
    fn correction_of_unknown_qso_is_new() {
        let mut gate = ResendGate::new();
        let mut e = correction(
            record("HB9FQZ", "SSB", &[]),
            "HB9FQZ",
            "2026-10-08 14:57:48",
        );
        assert_eq!(gate.check(&mut e), GateDecision::Forward { rev: 1 });
    }

    #[test]
    fn logger_id_identity_tracks_callsign_changes() {
        let mut gate = ResendGate::new();
        let mut a = QsoEvent::new(
            EventKind::New,
            record("K6QA", "SSB", &[]),
            Some("3809734F4".into()),
            &station(),
        );
        gate.check(&mut a);
        let mut b = QsoEvent::new(
            EventKind::Correction,
            record("K6QB", "SSB", &[]),
            Some("3809734F4".into()),
            &station(),
        );
        assert_eq!(gate.check(&mut b), GateDecision::Forward { rev: 2 });
        assert_eq!(a.id, b.id);
    }

    #[test]
    fn never_evicts() {
        let mut gate = ResendGate::new();
        for i in 0..500 {
            gate.check(&mut event(record(&format!("K{i}A"), "CW", &[])));
        }
        assert_eq!(gate.len(), 500);
    }

    #[test]
    fn different_mode_is_a_different_qso() {
        let mut gate = ResendGate::new();
        assert_eq!(
            gate.check(&mut event(record("K6QA", "SSB", &[]))),
            GateDecision::Forward { rev: 1 }
        );
        assert_eq!(
            gate.check(&mut event(record("K6QA", "CW", &[]))),
            GateDecision::Forward { rev: 1 }
        );
    }

    #[test]
    fn old_timestamp_formats() {
        assert_eq!(
            split_timestamp("2026-10-08 15:04:22"),
            Some(("20261008".into(), "150422".into()))
        );
        assert_eq!(
            split_timestamp("20261008 1504"),
            Some(("20261008".into(), "1504".into()))
        );
        assert_eq!(split_timestamp("garbage"), None);
    }

    #[test]
    fn contains_known_identities_and_previous_ones() {
        let mut gate = ResendGate::new();
        let mut original = event(record("K6QA", "SSB", &[]));
        gate.check(&mut original);
        assert!(gate.contains(&event(record("K6QA", "SSB", &[("SRX_STRING", "x")]))));
        assert!(!gate.contains(&event(record("K1ZZ", "SSB", &[]))));
        let moved = correction(record("K6QB", "SSB", &[]), "K6QA", "2026-10-08 15:04:22");
        assert!(gate.contains(&moved));
        let unknown = correction(record("K6QB", "SSB", &[]), "K9XX", "2026-10-08 15:04:22");
        assert!(!gate.contains(&unknown));
    }
}
