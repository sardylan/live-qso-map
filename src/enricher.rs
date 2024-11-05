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


use crate::qrzcom;
use crate::receiver::ContactInfo;
use async_broadcast::Sender;
use async_channel::Receiver;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QSO {
    call: String,
    band: String,
    latitude: f64,
    longitude: f64,
}

impl Display for QSO {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({}) [{} {}]", self.call, self.band, self.latitude, self.longitude)
    }
}

#[derive(Debug)]
pub enum EnricherError {
    RecvError(async_channel::RecvError<>),
    SendError(async_broadcast::SendError<QSO>),
    QRZComError(qrzcom::QRZComError),
}

impl From<async_channel::RecvError> for EnricherError {
    fn from(value: async_channel::RecvError) -> Self {
        Self::RecvError(value)
    }
}

impl From<async_broadcast::SendError<QSO>> for EnricherError {
    fn from(value: async_broadcast::SendError<QSO>) -> Self {
        Self::SendError(value)
    }
}

impl From<qrzcom::QRZComError> for EnricherError {
    fn from(value: qrzcom::QRZComError) -> Self {
        Self::QRZComError(value)
    }
}

pub async fn run_enricher(qrzcom_user: &str, qrzcom_password: &str, contact_info_receiver: Receiver<ContactInfo>, qso_sender: Sender<QSO>) -> Result<(), EnricherError> {
    loop {
        let contact_info: ContactInfo = contact_info_receiver.recv().await?;
        log::debug!("Contact info to enrich: {}", contact_info);

        let callsign = qrzcom::call_xml_api(qrzcom_user, qrzcom_password, &contact_info.call).await?;

        let qso: QSO = QSO {
            call: contact_info.call,
            band: contact_info.band,
            latitude: callsign.lat.unwrap_or(0.0),
            longitude: callsign.lon.unwrap_or(0.0),
        };
        log::debug!("QSO:: {}", qso);

        if qso.latitude == 0.0 && qso.longitude == 0.0 {
            log::warn!("Latitude or longitude are empty");
        }

        log::trace!("Broadcasting QSO");
        qso_sender.broadcast(qso).await?;
    }
}

