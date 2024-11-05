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


use crate::enricher::QSO;
use crate::models::Point;
use actix_web::middleware::Logger;
use actix_web::{get, route, rt, web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder};
use actix_web_rust_embed_responder::{EmbedResponse, IntoResponse, WebEmbedableFile};
use actix_ws::AggregatedMessage;
use async_broadcast::Receiver;
use rust_embed_for_web::{DynamicFile, RustEmbed};
use serde::{Deserialize, Serialize};

#[derive(RustEmbed)]
#[folder = "assets/"]
struct Assets;

#[route("/assets/{path:.*}", method = "GET", method = "HEAD")]
async fn serve_assets(path: web::Path<String>) -> EmbedResponse<WebEmbedableFile<DynamicFile>> {
    let path = if path.is_empty() {
        "index.html"
    } else {
        path.as_str()
    };

    Assets::get(path).into_response()
}

#[get("/api/public/v1/points/home")]
async fn home_point_service(home_point: web::Data<Point>) -> impl Responder {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct ResponseBody {
        latitude: f64,
        longitude: f64,
    }

    HttpResponse::Ok()
        .content_type("application/json")
        .json(&home_point)
}

#[get("/health")]
async fn health() -> impl Responder {
    HttpResponse::NoContent()
}

async fn ws(req: HttpRequest, stream: web::Payload, qso_receiver: web::Data<Receiver<QSO>>) -> Result<HttpResponse, Error> {
    let (res, mut session, stream) = actix_ws::handle(&req, stream)?;

    let mut rx_stream = stream
        .aggregate_continuations()
        .max_continuation_size(2_usize.pow(20));

    let mut rx_session = session.clone();

    rt::spawn(async move {
        // receive messages from websocket
        while let Some(msg) = rx_stream.recv().await {
            match msg {
                Ok(AggregatedMessage::Ping(msg)) => {
                    rx_session.pong(&msg).await.unwrap();
                }

                _ => {}
            }
        }
    });

    rt::spawn(async move {
        let mut qso_receiver = qso_receiver.get_ref().clone();
        while let Some(qso) = qso_receiver.recv().await.ok() {
            let data = serde_json::to_string(&qso).unwrap();
            session.text(data).await.unwrap();
        }
    });

    Ok(res)
}

pub async fn run_http_server(http_host: &str, http_port: u16, home_point: Point, qso_receiver: Receiver<QSO>) -> std::io::Result<()> {
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(qso_receiver.clone()))
            .app_data(web::Data::new(home_point))
            .wrap(Logger::default())
            .service(web::redirect("/", "/assets/"))
            .service(serve_assets)
            .service(home_point_service)
            .service(health)
            .service(web::resource("/api/public/v1/map/ws").route(web::get().to(ws)))
    })
        .bind((http_host, http_port))?
        .run()
        .await
}
