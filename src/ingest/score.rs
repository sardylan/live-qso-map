//! QARTest score messages (dynamicresults): score, QSO total and multiplier total (feature 007,
//! research R1).

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

use crate::errors::{Error, Result};

/// The logger's latest contest score (data-model "ContestScore").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContestScore {
    pub score: u64,
    pub qsos: u64,
    /// The sum of every multiplier total, whatever its type (HR-10).
    pub mults: u64,
}

/// Which value an open element's text feeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Score,
    Qso,
    Mult,
}

impl Target {
    fn element(self) -> &'static str {
        match self {
            Target::Score => "score",
            Target::Qso => "qso",
            Target::Mult => "mult",
        }
    }
}

fn invalid(element: &'static str, message: &str) -> Error {
    Error::QarTest {
        element,
        message: message.into(),
    }
}

fn local_name(name: &str) -> String {
    name.rsplit(':')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase()
}

/// Whether a breakdown entry is an all-modes contest total: `band="total" mode="ALL"`, in any case.
fn is_total(start: &BytesStart<'_>) -> bool {
    let (mut band, mut mode) = (false, false);
    for attribute in start.attributes().flatten() {
        let value = &attribute.value;
        match local_name(attribute.key.local_name().as_ref()).as_str() {
            "band" => band = value.trim().eq_ignore_ascii_case("total"),
            "mode" => mode = value.trim().eq_ignore_ascii_case("all"),
            _ => {}
        }
    }
    band && mode
}

/// What an opening element feeds, given its name and its parent's.
fn target_of(start: &BytesStart<'_>, name: &str, parent: Option<&str>) -> Option<Target> {
    match (parent, name) {
        (Some("dynamicresults"), "score") => Some(Target::Score),
        (Some("breakdown"), "qso") if is_total(start) => Some(Target::Qso),
        (Some("breakdown"), "mult") if is_total(start) => Some(Target::Mult),
        _ => None,
    }
}

fn number(target: Target, text: &str) -> Result<u64> {
    text.trim()
        .parse::<u64>()
        .map_err(|_| invalid(target.element(), "missing or invalid"))
}

#[derive(Default)]
struct Totals {
    score: Option<u64>,
    qsos: Option<u64>,
    mults: u64,
}

impl Totals {
    fn add(&mut self, target: Target, text: &str) -> Result<()> {
        let value = number(target, text)?;
        match target {
            Target::Score => self.score = Some(value),
            Target::Qso => self.qsos = Some(value),
            Target::Mult => {
                self.mults = self
                    .mults
                    .checked_add(value)
                    .ok_or_else(|| invalid("mult", "the multiplier total overflows"))?;
            }
        }
        Ok(())
    }
}

/// Reads a `dynamicresults` document (research R1). The shared `xml::flatten` can't be used: it
/// drops attributes and keeps one text per element name.
pub fn parse(xml: &str) -> Result<ContestScore> {
    let mut reader = Reader::from_str(xml);
    let mut stack: Vec<String> = Vec::new();
    let mut root = String::new();
    // The element being read and its text so far.
    let mut open: Option<(usize, Target, String)> = None;
    let mut totals = Totals::default();

    loop {
        match reader.read_event()? {
            Event::Start(e) => {
                let name = local_name(e.local_name().as_ref());
                if root.is_empty() {
                    root = name.clone();
                }
                if let Some(target) = target_of(&e, &name, stack.last().map(String::as_str)) {
                    open = Some((stack.len(), target, String::new()));
                }
                stack.push(name);
            }
            Event::Empty(e) => {
                let name = local_name(e.local_name().as_ref());
                if let Some(target) = target_of(&e, &name, stack.last().map(String::as_str)) {
                    totals.add(target, "")?;
                }
            }
            Event::Text(t) => {
                if let Some((_, _, text)) = open.as_mut() {
                    text.push_str(&t.html_content());
                }
            }
            Event::GeneralRef(r) => {
                if let Some((_, _, text)) = open.as_mut() {
                    // Only character references can be part of a number; anything else spoils it.
                    match r.resolve_char_ref()? {
                        Some(c) => text.push(c),
                        None => text.push('?'),
                    }
                }
            }
            Event::End(_) => {
                stack.pop();
                if let Some((depth, target, text)) = open.take() {
                    if depth == stack.len() {
                        totals.add(target, &text)?;
                    } else {
                        open = Some((depth, target, text));
                    }
                }
            }
            Event::Eof => {
                if let Some(name) = stack.pop() {
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
    if root != "dynamicresults" {
        return Err(invalid("root", "not a dynamicresults message"));
    }
    Ok(ContestScore {
        score: totals
            .score
            .ok_or_else(|| invalid("score", "missing or invalid"))?,
        qsos: totals
            .qsos
            .ok_or_else(|| invalid("qso", "missing or invalid"))?,
        mults: totals.mults,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::qartest::reference_packets;

    fn packet(i: usize) -> String {
        let packets = reference_packets(include_str!(
            "../../specs/001-live-qso-map/reference/qartest-16.9.1-cqww-ssb.txt"
        ));
        String::from_utf8(packets[i].clone()).unwrap()
    }

    fn doc(body: &str) -> String {
        format!(
            "<?xml version=\"1.0\"?><dynamicresults><soft>QARTest</soft><call>IS0MZK</call>{body}</dynamicresults>"
        )
    }

    fn breakdown(entries: &str) -> String {
        doc(&format!(
            "<score>1</score><breakdown><qso band='total' mode='ALL'>3</qso>{entries}</breakdown>"
        ))
    }

    fn element_of(result: Result<ContestScore>) -> &'static str {
        match result {
            Err(Error::QarTest { element, .. }) => element,
            other => panic!("expected a QarTest error, got {other:?}"),
        }
    }

    #[test]
    fn reference_packets_give_their_scores() {
        assert_eq!(
            parse(&packet(3)).unwrap(),
            ContestScore {
                score: 90,
                qsos: 6,
                mults: 10
            }
        );
        assert_eq!(
            parse(&packet(9)).unwrap(),
            ContestScore {
                score: 132,
                qsos: 7,
                mults: 11
            }
        );
    }

    #[test]
    fn every_multiplier_type_is_summed() {
        let one = breakdown("<mult band='total' mode='ALL' type='prefix'>250</mult>");
        assert_eq!(parse(&one).unwrap().mults, 250);
        let three = breakdown(
            "<mult band='total' mode='ALL' type='country'>10</mult>\
             <mult band='total' mode='ALL' type='zone'>5</mult>\
             <mult band='total' mode='ALL' type='state'>7</mult>",
        );
        assert_eq!(parse(&three).unwrap().mults, 22);
        let none = breakdown("<mult band='40' mode='ALL' type='country'>4</mult>");
        assert_eq!(parse(&none).unwrap().mults, 0);
    }

    #[test]
    fn only_all_mode_totals_count() {
        let xml = breakdown(
            "<mult band='40' mode='ALL' type='country'>6</mult>\
             <mult band='total' mode='PH' type='country'>6</mult>\
             <mult band='total' mode='CW' type='country'>5</mult>\
             <mult band='total' mode='ALL' type='country'>8</mult>",
        );
        let score = parse(&xml).unwrap();
        assert_eq!(score.mults, 8);
        assert_eq!(score.qsos, 3);
        let per_band_qso = doc(
            "<score>1</score><breakdown><qso band='40' mode='ALL'>2</qso>\
             <qso band='total' mode='PH'>9</qso><qso band='total' mode='ALL'>4</qso></breakdown>",
        );
        assert_eq!(parse(&per_band_qso).unwrap().qsos, 4);
    }

    #[test]
    fn attributes_match_ignoring_case() {
        let xml = doc(
            "<score>5</score><breakdown><qso band=\"TOTAL\" mode=\"all\">2</qso>\
             <mult band=\"Total\" mode=\"All\" type=\"zone\">3</mult></breakdown>",
        );
        assert_eq!(
            parse(&xml).unwrap(),
            ContestScore {
                score: 5,
                qsos: 2,
                mults: 3
            }
        );
    }

    #[test]
    fn bad_scores_are_errors() {
        let qso = "<breakdown><qso band='total' mode='ALL'>3</qso></breakdown>";
        for score in [
            "",
            "<score></score>",
            "<score>abc</score>",
            "<score>-5</score>",
            "<score>1.5</score>",
        ] {
            assert_eq!(
                element_of(parse(&doc(&format!("{score}{qso}")))),
                "score",
                "{score}"
            );
        }
    }

    #[test]
    fn bad_qso_totals_are_errors() {
        assert_eq!(element_of(parse(&doc("<score>1</score>"))), "qso");
        assert_eq!(
            element_of(parse(&doc(
                "<score>1</score><breakdown><qso band='total' mode='ALL'>x</qso></breakdown>"
            ))),
            "qso"
        );
    }

    #[test]
    fn bad_or_overflowing_multipliers_are_errors() {
        let bad = breakdown("<mult band='total' mode='ALL' type='zone'>x</mult>");
        assert_eq!(element_of(parse(&bad)), "mult");
        let max = u64::MAX;
        let overflow = breakdown(&format!(
            "<mult band='total' mode='ALL' type='country'>{max}</mult>\
             <mult band='total' mode='ALL' type='zone'>{max}</mult>"
        ));
        assert_eq!(element_of(parse(&overflow)), "mult");
    }

    #[test]
    fn ill_formed_xml_is_an_error() {
        assert!(parse("<dynamicresults><score>1</score>").is_err());
        assert!(parse("not xml at all").is_err());
        assert!(parse("").is_err());
    }
}
