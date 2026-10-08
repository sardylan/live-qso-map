#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use live_qso_map::config::Config;
use live_qso_map::pipeline::HistoryChange;
use tokio::net::UdpSocket;
use tokio::sync::broadcast;

/// A configuration bound to localhost with ephemeral ports and no lookups.
pub fn test_config() -> Config {
    let mut config = Config {
        bind_address: IpAddr::V4(Ipv4Addr::LOCALHOST),
        udp_port: 0,
        http_port: 0,
        ..Config::default()
    };
    config.station.callsign = Some("IS0MZK".into());
    config.station.locator = Some("JM49NH".into());
    config.country_data.path = "/nonexistent/cty.dat".into();
    config.country_data.max_age_days = 0;
    config
}

pub async fn send_udp(to: SocketAddr, payload: &[u8]) {
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    socket.send_to(payload, to).await.unwrap();
}

/// Receives the next upsert within `within`, or `None`.
pub async fn next_upsert(
    rx: &mut broadcast::Receiver<HistoryChange>,
    within: Duration,
) -> Option<std::sync::Arc<live_qso_map::history::EnrichedQso>> {
    match tokio::time::timeout(within, rx.recv()).await {
        Ok(Ok(HistoryChange::Upsert(q))) => Some(q),
        _ => None,
    }
}
