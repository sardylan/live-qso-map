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

use async_channel::Sender;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use tokio::net::UdpSocket;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ContactInfo {
    pub call: String,
    pub band: String,
}

impl Display for ContactInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.call, self.band)
    }
}

#[derive(Debug)]
pub enum ReceiverError {
    UDPSocket(std::io::Error),
    XMLParsing(serde_xml_rs::Error),
    QueueSenderError(async_channel::SendError<ContactInfo>),
}

impl From<std::io::Error> for ReceiverError {
    fn from(value: std::io::Error) -> Self {
        Self::UDPSocket(value)
    }
}

impl Display for ReceiverError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ReceiverError::UDPSocket(e) => {
                write!(f, "UDP Socket error: {}", e)
            }
            ReceiverError::XMLParsing(e) => {
                write!(f, "XML Parsing error: {}", e)
            }
            ReceiverError::QueueSenderError(e) => {
                write!(f, "Queue sender error: {}", e)
            }
        }
    }
}

impl From<serde_xml_rs::Error> for ReceiverError {
    fn from(value: serde_xml_rs::Error) -> Self {
        Self::XMLParsing(value)
    }
}

impl From<async_channel::SendError<ContactInfo>> for ReceiverError {
    fn from(value: async_channel::SendError<ContactInfo>) -> Self {
        Self::QueueSenderError(value)
    }
}

pub async fn run_receiver(
    bind_host: &str,
    bind_port: u16,
    contact_info_sender: Sender<ContactInfo>,
) -> Result<(), ReceiverError> {
    let binding = format!("{}:{}", bind_host, bind_port);
    let sock = UdpSocket::bind(binding).await?;

    let mut buf = [0; 8192];

    loop {
        let (len, addr) = sock.recv_from(&mut buf).await?;
        let payload = String::from_utf8(buf[..len].to_vec()).unwrap();
        log::debug!("Received {} bytes from {:?}: {}", len, addr, &payload);

        let contact_info = parse_contact_info(&payload).await;
        if contact_info.is_err() {
            log::warn!(
                "Failed to parse contact info: {}",
                contact_info.unwrap_err()
            );
            continue;
        }

        let contact_info = contact_info?;
        log::info!("Received contact info: {}", &contact_info);
        let result = contact_info_sender.send(contact_info).await;
        if result.is_err() {
            log::warn!("Failed to send contact info: {}", result.unwrap_err());
        };
    }
}

async fn parse_contact_info(payload: &str) -> Result<ContactInfo, ReceiverError> {
    let contact_info: ContactInfo = serde_xml_rs::from_str(payload)?;
    Ok(contact_info)
}
