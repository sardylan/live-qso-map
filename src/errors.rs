//! Crate-wide error type and `Result` alias (`docs/rust-guidelines.md` §1).

use std::fmt;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Toml(toml::de::Error),
    Xml(quick_xml::Error),
    Http(reqwest::Error),
    Json(serde_json::Error),
    Task(tokio::task::JoinError),
    Config {
        setting: String,
        message: String,
    },
    Adif {
        record: usize,
        message: String,
    },
    QarTest {
        element: &'static str,
        message: String,
    },
    Provider {
        provider: &'static str,
        message: String,
    },
    InvalidPosition {
        lat: f64,
        lon: f64,
    },
    ConfigWrite {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "i/o error: {e}"),
            Error::Toml(e) => write!(f, "configuration file error: {e}"),
            Error::Xml(e) => write!(f, "xml error: {e}"),
            Error::Http(e) => write!(f, "http error: {e}"),
            Error::Json(e) => write!(f, "json error: {e}"),
            Error::Task(e) => write!(f, "task error: {e}"),
            Error::Config { setting, message } => write!(f, "invalid setting {setting}: {message}"),
            Error::Adif { record, message } => write!(f, "adif record {record}: {message}"),
            Error::QarTest { element, message } => write!(f, "qartest <{element}>: {message}"),
            Error::Provider { provider, message } => write!(f, "{provider}: {message}"),
            Error::InvalidPosition { lat, lon } => {
                write!(f, "invalid position: lat {lat}, lon {lon}")
            }
            Error::ConfigWrite { path, source } => {
                write!(
                    f,
                    "cannot write configuration file {}: {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            Error::Toml(e) => Some(e),
            Error::Xml(e) => Some(e),
            Error::Http(e) => Some(e),
            Error::Json(e) => Some(e),
            Error::Task(e) => Some(e),
            Error::ConfigWrite { source, .. } => Some(source),
            Error::Config { .. }
            | Error::Adif { .. }
            | Error::QarTest { .. }
            | Error::Provider { .. }
            | Error::InvalidPosition { .. } => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<toml::de::Error> for Error {
    fn from(e: toml::de::Error) -> Self {
        Error::Toml(e)
    }
}

impl From<quick_xml::Error> for Error {
    fn from(e: quick_xml::Error) -> Self {
        Error::Xml(e)
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Http(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Json(e)
    }
}

impl From<tokio::task::JoinError> for Error {
    fn from(e: tokio::task::JoinError) -> Self {
        Error::Task(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as _;

    fn xml_error() -> quick_xml::Error {
        let mut reader = quick_xml::Reader::from_str("<a></b>");
        loop {
            match reader.read_event() {
                Err(e) => return e,
                Ok(quick_xml::events::Event::Eof) => panic!("expected an XML error"),
                Ok(_) => {}
            }
        }
    }

    fn http_error() -> reqwest::Error {
        crate::install_crypto_provider();
        reqwest::Client::builder()
            .build()
            .unwrap()
            .get("not a url")
            .build()
            .unwrap_err()
    }

    fn json_error() -> serde_json::Error {
        serde_json::from_str::<serde_json::Value>("{").unwrap_err()
    }

    fn toml_error() -> toml::de::Error {
        toml::from_str::<toml::Table>("x =").unwrap_err()
    }

    async fn join_error() -> tokio::task::JoinError {
        let handle = tokio::spawn(std::future::pending::<()>());
        handle.abort();
        handle.await.unwrap_err()
    }

    async fn all_variants() -> Vec<Error> {
        vec![
            Error::from(std::io::Error::other("boom")),
            Error::from(toml_error()),
            Error::from(xml_error()),
            Error::from(http_error()),
            Error::from(json_error()),
            Error::from(join_error().await),
            Error::Config {
                setting: "udp_port".into(),
                message: "must be between 1 and 65535".into(),
            },
            Error::Adif {
                record: 2,
                message: "truncated value".into(),
            },
            Error::QarTest {
                element: "call",
                message: "missing".into(),
            },
            Error::Provider {
                provider: "qrz",
                message: "password incorrect".into(),
            },
            Error::InvalidPosition {
                lat: 91.0,
                lon: 0.0,
            },
            Error::ConfigWrite {
                path: "/x.toml".into(),
                source: std::io::Error::other("denied"),
            },
        ]
    }

    #[tokio::test]
    async fn display_is_not_empty_for_every_variant() {
        for e in all_variants().await {
            assert!(!e.to_string().is_empty(), "{e:?}");
        }
    }

    #[tokio::test]
    async fn from_conversions_produce_matching_variants() {
        assert!(matches!(
            Error::from(std::io::Error::other("x")),
            Error::Io(_)
        ));
        assert!(matches!(Error::from(toml_error()), Error::Toml(_)));
        assert!(matches!(Error::from(xml_error()), Error::Xml(_)));
        assert!(matches!(Error::from(http_error()), Error::Http(_)));
        assert!(matches!(Error::from(json_error()), Error::Json(_)));
        assert!(matches!(Error::from(join_error().await), Error::Task(_)));
    }

    #[tokio::test]
    async fn source_returns_the_wrapped_error() {
        for e in all_variants().await {
            let wraps = matches!(
                e,
                Error::Io(_)
                    | Error::Toml(_)
                    | Error::Xml(_)
                    | Error::Http(_)
                    | Error::Json(_)
                    | Error::Task(_)
                    | Error::ConfigWrite { .. }
            );
            assert_eq!(e.source().is_some(), wraps, "{e:?}");
        }
    }

    #[test]
    fn config_write_names_the_path_and_wraps_the_io_error() {
        let e = Error::ConfigWrite {
            path: std::path::PathBuf::from("/etc/live-qso-map/config.toml"),
            source: std::io::Error::other("permission denied"),
        };
        assert_eq!(
            e.to_string(),
            "cannot write configuration file /etc/live-qso-map/config.toml: permission denied"
        );
        assert_eq!(
            e.source().map(|s| s.to_string()).as_deref(),
            Some("permission denied")
        );
    }

    #[test]
    fn result_alias_uses_crate_error() {
        fn fails() -> Result<u8> {
            Err(Error::InvalidPosition {
                lat: 0.0,
                lon: 200.0,
            })
        }
        assert!(fails().is_err());
    }
}
