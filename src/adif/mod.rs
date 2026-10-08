//! ADIF record: the single QSO representation inside the service (FR-006).

pub mod parse;
pub mod write;

use std::collections::BTreeMap;

/// Map from uppercase ADIF field name to value, iterated in name order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdifRecord {
    fields: BTreeMap<String, String>,
}

impl AdifRecord {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.fields
            .get(&name.to_ascii_uppercase())
            .map(String::as_str)
    }

    /// Returns the value only when it is present and not blank.
    pub fn get_non_empty(&self, name: &str) -> Option<&str> {
        self.get(name).map(str::trim).filter(|v| !v.is_empty())
    }

    pub fn set(&mut self, name: &str, value: impl Into<String>) {
        self.fields.insert(name.to_ascii_uppercase(), value.into());
    }

    /// Sets the field only when it is absent or blank, so logger values are never overwritten
    /// (FR-017). Returns whether the value was written.
    pub fn set_if_absent(&mut self, name: &str, value: impl Into<String>) -> bool {
        if self.get_non_empty(name).is_some() {
            return false;
        }
        self.set(name, value);
        true
    }

    pub fn remove(&mut self, name: &str) -> Option<String> {
        self.fields.remove(&name.to_ascii_uppercase())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.fields.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    pub fn len(&self) -> usize {
        self.fields.len()
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    pub fn as_map(&self) -> &BTreeMap<String, String> {
        &self.fields
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_uppercased() {
        let mut r = AdifRecord::new();
        r.set("call", "K1AA");
        assert_eq!(r.get("CALL"), Some("K1AA"));
        assert_eq!(r.iter().next(), Some(("CALL", "K1AA")));
    }

    #[test]
    fn set_if_absent_never_overwrites() {
        let mut r = AdifRecord::new();
        r.set("GRIDSQUARE", "JM49");
        assert!(!r.set_if_absent("GRIDSQUARE", "JO62"));
        assert_eq!(r.get("GRIDSQUARE"), Some("JM49"));
        r.set("NAME", "  ");
        assert!(r.set_if_absent("NAME", "Joe"));
        assert!(r.set_if_absent("QTH", "Rome"));
        assert_eq!(r.get("NAME"), Some("Joe"));
        assert_eq!(r.get("QTH"), Some("Rome"));
    }

    #[test]
    fn get_non_empty_trims() {
        let mut r = AdifRecord::new();
        r.set("OPERATOR", "");
        r.set("CALL", " K1AA ");
        assert_eq!(r.get_non_empty("OPERATOR"), None);
        assert_eq!(r.get_non_empty("CALL"), Some("K1AA"));
    }
}
