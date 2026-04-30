use futures::{StreamExt, SinkExt};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use warp::Filter;
use tracing::{info, error};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TelemetryEvent {
    NodeDiscovered { peer_id: String },
    NodeConnected { peer_id: String },
    NodeDisconnected { peer_id: String },
    VpnTunnelEstablished { target: String },
    ProxyRequest { dest: String },
}

pub async fn start_telemetry_server(tx: broadcast::Sender<TelemetryEvent>, port: u16) {
    let tx_filter = warp::any().map(move || tx.clone());

    let ws_route = warp::path("telemetry")
        .and(warp::ws())
        .and(tx_filter)
        .map(|ws: warp::ws::Ws, tx: broadcast::Sender<TelemetryEvent>| {
            let rx = tx.subscribe();
            ws.on_upgrade(move |socket| handle_ws_client(socket, rx))
        })
        .boxed();

    info!("🌐 Telemetry WebSocket Server listening on ws://127.0.0.1:{}/telemetry", port);
    warp::serve(ws_route).run(([127, 0, 0, 1], port)).await;
}

async fn handle_ws_client(ws: warp::ws::WebSocket, mut rx: broadcast::Receiver<TelemetryEvent>) {
    let (mut client_ws_sender, _client_ws_rcv) = ws.split();

    while let Ok(event) = rx.recv().await {
        if let Ok(msg) = serde_json::to_string(&event) {
            if let Err(e) = client_ws_sender.send(warp::ws::Message::text(msg)).await {
                error!("WebSocket send error: {}", e);
                break;
            }
        }
    }
}
