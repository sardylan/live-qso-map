//! QSO identity (FR-007, data-model "Identity").

use crate::adif::AdifRecord;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Identity {
    Logger {
        station_call: String,
        logger_id: String,
    },
    Composite {
        station_call: String,
        call: String,
        band: String,
        mode: String,
        qso_date: String,
        time_on_hhmm: String,
    },
}

impl Identity {
    /// Builds the identity of `record`. The station callsign is `STATION_CALLSIGN`, then
    /// `default_station`, then empty. `OPERATOR` is never used: it changes per operator.
    pub fn from_record(
        record: &AdifRecord,
        logger_id: Option<&str>,
        default_station: Option<&str>,
    ) -> Self {
        let station_call = normalise(
            record
                .get_non_empty("STATION_CALLSIGN")
                .or(default_station)
                .unwrap_or_default(),
        );
        if let Some(id) = logger_id.map(normalise).filter(|id| !id.is_empty()) {
            return Identity::Logger {
                station_call,
                logger_id: id,
            };
        }
        let field = |name: &str| normalise(record.get(name).unwrap_or_default());
        let time_on = field("TIME_ON");
        Identity::Composite {
            station_call,
            call: field("CALL"),
            band: field("BAND"),
            mode: field("MODE"),
            qso_date: field("QSO_DATE"),
            time_on_hhmm: time_on.chars().take(4).collect(),
        }
    }

    pub fn canonical(&self) -> String {
        match self {
            Identity::Logger {
                station_call,
                logger_id,
            } => format!("L|{station_call}|{logger_id}"),
            Identity::Composite {
                station_call,
                call,
                band,
                mode,
                qso_date,
                time_on_hhmm,
            } => format!("C|{station_call}|{call}|{band}|{mode}|{qso_date}|{time_on_hhmm}"),
        }
    }
}

fn normalise(value: &str) -> String {
    value.trim().to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(fields: &[(&str, &str)]) -> AdifRecord {
        let mut r = AdifRecord::new();
        for (k, v) in fields {
            r.set(k, *v);
        }
        r
    }

    fn dl5qfn() -> AdifRecord {
        record(&[
            ("STATION_CALLSIGN", "IS0MZK"),
            ("CALL", "DL5QFN"),
            ("BAND", "40m"),
            ("MODE", "SSB"),
            ("QSO_DATE", "20261008"),
            ("TIME_ON", "150404"),
        ])
    }

    #[test]
    fn logger_id_identity() {
        let id = Identity::from_record(&dl5qfn(), Some("3809734F4"), None);
        assert_eq!(id.canonical(), "L|IS0MZK|3809734F4");
    }

    #[test]
    fn composite_identity_truncates_time_to_minutes() {
        let id = Identity::from_record(&dl5qfn(), None, None);
        assert_eq!(id.canonical(), "C|IS0MZK|DL5QFN|40M|SSB|20261008|1504");
    }

    #[test]
    fn components_are_trimmed_and_uppercased() {
        let mut r = dl5qfn();
        r.set("CALL", " dl5qfn ");
        r.set("STATION_CALLSIGN", "is0mzk");
        assert_eq!(
            Identity::from_record(&r, None, None),
            Identity::from_record(&dl5qfn(), None, None)
        );
        assert_eq!(
            Identity::from_record(&r, Some(" 3809734f4 "), None).canonical(),
            "L|IS0MZK|3809734F4"
        );
    }

    #[test]
    fn station_falls_back_to_configured_default_not_operator() {
        let mut r = dl5qfn();
        r.remove("STATION_CALLSIGN");
        r.set("OPERATOR", "IK0ABC");
        let id = Identity::from_record(&r, None, Some("is0mzk"));
        assert_eq!(id.canonical(), "C|IS0MZK|DL5QFN|40M|SSB|20261008|1504");

        let mut other_op = r.clone();
        other_op.set("OPERATOR", "IS0XYZ");
        assert_eq!(id, Identity::from_record(&other_op, None, Some("IS0MZK")));

        let no_default = Identity::from_record(&r, None, None);
        assert_eq!(no_default.canonical(), "C||DL5QFN|40M|SSB|20261008|1504");
    }

    #[test]
    fn missing_band_or_mode_is_empty() {
        let mut r = dl5qfn();
        r.remove("BAND");
        r.remove("MODE");
        assert_eq!(
            Identity::from_record(&r, None, None).canonical(),
            "C|IS0MZK|DL5QFN|||20261008|1504"
        );
    }

    #[test]
    fn different_mode_is_different_identity() {
        let mut cw = dl5qfn();
        cw.set("MODE", "CW");
        assert_ne!(
            Identity::from_record(&cw, None, None),
            Identity::from_record(&dl5qfn(), None, None)
        );
    }
}
