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

mod config;
mod enricher;
mod http;
mod logging;
mod models;
mod qrzcom;
mod receiver;

use crate::config::Config;
use crate::enricher::QSO;
use crate::models::Point;
use crate::receiver::ContactInfo;
use clap::Parser;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let configuration = Config::parse();

    logging::configure(&configuration.log_level);

    let (contact_info_sender, contact_info_receiver): (
        async_channel::Sender<ContactInfo>,
        async_channel::Receiver<ContactInfo>,
    ) = async_channel::unbounded();
    let (qso_sender, qso_receiver): (async_broadcast::Sender<QSO>, async_broadcast::Receiver<QSO>) =
        async_broadcast::broadcast(10);

    let bind_host = configuration.bind_host;
    let bind_port = configuration.bind_port;
    let _task_receiver = tokio::spawn(async move {
        receiver::run_receiver(&bind_host, bind_port, contact_info_sender).await
    });

    let qrzcom_user = configuration.qrzcom_user;
    let qrzcom_password = configuration.qrzcom_password;
    let _task_enricher = tokio::spawn(async move {
        enricher::run_enricher(
            &qrzcom_user,
            &qrzcom_password,
            contact_info_receiver,
            qso_sender,
        )
        .await
    });

    let home_point = Point {
        latitude: configuration.home_latitude,
        longitude: configuration.home_longitude,
    };

    http::run_http_server(
        &configuration.http_host,
        configuration.http_port,
        home_point,
        qso_receiver,
    )
    .await
}
