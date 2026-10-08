//! Flattens shallow XML documents (QARTest messages, provider responses) into
//! `parent element → child element → text`, keyed by lowercase local names.

use std::collections::HashMap;

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::errors::Result;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Flat {
    /// Lowercase local name of the document element.
    pub root: String,
    sections: HashMap<String, HashMap<String, String>>,
}

impl Flat {
    /// The trimmed text of `parent/child`, when present and not empty.
    pub fn get(&self, parent: &str, child: &str) -> Option<&str> {
        self.sections
            .get(parent)?
            .get(child)
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
    }

    pub fn has_section(&self, parent: &str) -> bool {
        self.sections.contains_key(parent)
    }
}

fn local_name(name: &str) -> String {
    name.rsplit(':')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase()
}

fn resolve_entity(name: &str) -> Option<char> {
    match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        _ => None,
    }
}

pub fn flatten(xml: &str) -> Result<Flat> {
    let mut reader = Reader::from_str(xml);
    let mut flat = Flat::default();
    // (element name, text collected so far) for every open element.
    let mut stack: Vec<(String, String)> = Vec::new();

    loop {
        match reader.read_event()? {
            Event::Start(e) => {
                let name = local_name(e.local_name().as_ref());
                if stack.is_empty() && flat.root.is_empty() {
                    flat.root = name.clone();
                }
                flat.sections.entry(name.clone()).or_default();
                stack.push((name, String::new()));
            }
            Event::Empty(e) => {
                let name = local_name(e.local_name().as_ref());
                if stack.is_empty() && flat.root.is_empty() {
                    flat.root = name.clone();
                }
                if let Some((parent, _)) = stack.last() {
                    flat.sections
                        .entry(parent.clone())
                        .or_default()
                        .insert(name, String::new());
                }
            }
            Event::Text(t) => {
                if let Some((_, text)) = stack.last_mut() {
                    text.push_str(&t.html_content());
                }
            }
            Event::CData(c) => {
                if let Some((_, text)) = stack.last_mut() {
                    text.push_str(&c.html_content());
                }
            }
            Event::GeneralRef(r) => {
                if let Some((_, text)) = stack.last_mut() {
                    let name = r.html_content();
                    match r.resolve_char_ref()?.or_else(|| resolve_entity(&name)) {
                        Some(c) => text.push(c),
                        None => {
                            text.push('&');
                            text.push_str(&name);
                            text.push(';');
                        }
                    }
                }
            }
            Event::End(_) => {
                if let Some((name, text)) = stack.pop()
                    && let Some((parent, _)) = stack.last()
                {
                    flat.sections
                        .entry(parent.clone())
                        .or_default()
                        .insert(name, text);
                }
            }
            Event::Eof => {
                if let Some((name, _)) = stack.pop() {
                    return Err(quick_xml::Error::IllFormed(
                        quick_xml::errors::IllFormedError::MissingEndTag(name),
                    )
                    .into());
                }
                break;
            }
            _ => {}
        }
    }
    Ok(flat)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flattens_two_levels_with_namespaces() {
        let flat = flatten(
            r#"<?xml version="1.0"?><H xmlns="https://x"><search><call>K1A</call><qth>A &amp; B &#x41;</qth><empty/><blank></blank></search></H>"#,
        )
        .unwrap();
        assert_eq!(flat.root, "h");
        assert!(flat.has_section("search"));
        assert_eq!(flat.get("search", "call"), Some("K1A"));
        assert_eq!(flat.get("search", "qth"), Some("A & B A"));
        assert_eq!(flat.get("search", "empty"), None);
        assert_eq!(flat.get("search", "blank"), None);
        assert_eq!(flat.get("h", "search"), None);
    }

    #[test]
    fn element_names_are_case_insensitive() {
        let flat = flatten("<QRZDatabase><Session><Key>abc</Key></Session></QRZDatabase>").unwrap();
        assert_eq!(flat.root, "qrzdatabase");
        assert_eq!(flat.get("session", "key"), Some("abc"));
    }

    #[test]
    fn malformed_xml_is_an_error() {
        assert!(flatten("<a><b></a>").is_err());
        assert!(flatten("<a><b>text</b>").is_err(), "unclosed root");
    }
}
