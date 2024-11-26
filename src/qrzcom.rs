/*
 * Copyright (C) 2024 Luca Cireddu <sardylan@gmail.com>
 *
 * This program is free software: you can redistribute it and/or modify it under
 * the terms of the GNU General Public License as published by the Free Software
 * Foundation, version 3.
 *
 * This program is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * this program. If not, see <https://www.gnu.org/licenses/>.
 *
 */
use reqwest::{Client, Method};
use serde::Deserialize;
use std::fmt::Display;
use std::time::Duration;

#[derive(Debug)]
pub enum QRZComError {
    Request(reqwest::Error),
    Parsing(serde_xml_rs::Error),
    ApiError(String),
}

impl Display for QRZComError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QRZComError::Request(e) => {
                write!(f, "Request error: {}", e)
            }
            QRZComError::Parsing(e) => {
                write!(f, "Parsing error: {}", e)
            }
            QRZComError::ApiError(e) => {
                write!(f, "API error: {}", e)
            }
        }
    }
}

impl From<reqwest::Error> for QRZComError {
    fn from(value: reqwest::Error) -> Self {
        Self::Request(value)
    }
}

impl From<serde_xml_rs::Error> for QRZComError {
    fn from(value: serde_xml_rs::Error) -> Self {
        Self::Parsing(value)
    }
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct Callsign {
    pub call: Option<String>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Session {
    error: Option<String>,
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ResponseBody {
    callsign: Option<Callsign>,
    session: Session,
}

pub async fn call_xml_api(
    username: &str,
    password: &str,
    callsign: &str,
) -> Result<Callsign, QRZComError> {
    let response_body: String = Client::new()
        .request(Method::POST, "https://xmldata.qrz.com/xml/1.34/")
        .body(format!(
            "username={}&password={}&callsign={}",
            username, password, &callsign
        ))
        .timeout(Duration::from_secs(5))
        .send()
        .await?
        .text()
        .await?;

    let response: ResponseBody = parse_response(&response_body)?;
    response.callsign.ok_or(QRZComError::ApiError(
        response.session.error.unwrap_or("".to_string()),
    ))
}

fn parse_response(payload: &str) -> Result<ResponseBody, serde_xml_rs::Error> {
    serde_xml_rs::from_str(&payload)
}

#[cfg(test)]
mod tests {
    use crate::qrzcom::{parse_response, Callsign, ResponseBody, Session};

    #[test]
    fn test_parse_response_ok() {
        let input = "<?xml version=\"1.0\" encoding=\"utf-8\" ?>
<QRZDatabase version=\"1.34\" xmlns=\"http://xmldata.qrz.com\">
<Callsign>
<call>IS0GVH</call>
<lat>39.123456</lat>
<lon>9.654321</lon>
<grid>JM49</grid>
</Callsign>
<Session>
</Session>
</QRZDatabase>";

        let expected = ResponseBody {
            session: Session { error: None },
            callsign: Some(Callsign {
                call: Some("IS0GVH".to_string()),
                lat: Some(39.123456),
                lon: Some(9.654321),
            }),
        };

        let actual = parse_response(input).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parse_response_error() {
        let input = "<QRZDatabase version=\"1.34\" xmlns=\"http://xmldata.qrz.com\">
<Session>
<Error>Not found: ISGVH</Error>
</Session>
</QRZDatabase>";

        let expected = ResponseBody {
            session: Session {
                error: Some("Not found: ISGVH".to_string()),
            },
            callsign: None,
        };

        let actual = parse_response(input).unwrap();

        assert_eq!(actual, expected);
    }
}
