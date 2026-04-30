use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info};
use tokio::sync::broadcast;
use crate::telemetry::TelemetryEvent;

pub async fn start_socks5_server(port: u16, tx: broadcast::Sender<TelemetryEvent>) {
    let addr = format!("127.0.0.1:{}", port);
    let listener = match TcpListener::bind(&addr).await {
        // Wait, tokio::net::TcpListener::bind(&addr).await
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind SOCKS5 server on {}: {}", addr, e);
            return;
        }
    };

    info!("🚀 Aether VPN (SOCKS5 Proxy) running on socks5://{}", addr);
    info!("Configure your phone/laptop to use this SOCKS5 proxy for the Aether Network.");

    loop {
        match listener.accept().await {
            Ok((stream, peer_addr)) => {
                info!("New proxy connection from: {}", peer_addr);
                let tx_clone = tx.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_client(stream, tx_clone).await {
                        error!("SOCKS5 error: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
            }
        }
    }
}

async fn handle_client(mut client: TcpStream, tx: broadcast::Sender<TelemetryEvent>) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Handshake Phase
    let mut header = [0u8; 2];
    client.read_exact(&mut header).await?;
    
    if header[0] != 0x05 {
        return Err("Unsupported SOCKS version".into());
    }
    
    let n_methods = header[1] as usize;
    let mut methods = vec![0u8; n_methods];
    client.read_exact(&mut methods).await?;
    
    // We only support "No Authentication" (0x00) for local prototype
    client.write_all(&[0x05, 0x00]).await?;

    // 2. Request Phase
    let mut req_header = [0u8; 4];
    client.read_exact(&mut req_header).await?;
    
    if req_header[0] != 0x05 || req_header[1] != 0x01 { // Version 5, CONNECT command
        return Err("Unsupported SOCKS request".into());
    }

    let dest_addr: String = match req_header[3] {
        0x01 => { // IPv4
            let mut ip = [0u8; 4];
            client.read_exact(&mut ip).await?;
            let mut port = [0u8; 2];
            client.read_exact(&mut port).await?;
            let p = u16::from_be_bytes(port);
            format!("{}.{}.{}.{}:{}", ip[0], ip[1], ip[2], ip[3], p)
        }
        0x03 => { // Domain Name
            let mut len = [0u8; 1];
            client.read_exact(&mut len).await?;
            let mut domain = vec![0u8; len[0] as usize];
            client.read_exact(&mut domain).await?;
            let mut port = [0u8; 2];
            client.read_exact(&mut port).await?;
            let p = u16::from_be_bytes(port);
            format!("{}:{}", String::from_utf8_lossy(&domain), p)
        }
        0x04 => { // IPv6
            return Err("IPv6 not currently supported in Aether PoC".into());
        }
        _ => return Err("Invalid address type".into()),
    };

    info!("🛡️ Aether VPN: Routing traffic to -> {}", dest_addr);
    let _ = tx.send(TelemetryEvent::ProxyRequest { dest: dest_addr.clone() });

    // 3. Forward Phase (In a full ecosystem, this goes over the P2P swarm. Here we simulate acting as the exit node)
    let server = match TcpStream::connect(&dest_addr).await {
        Ok(s) => s,
        Err(e) => {
            // Reply with General SOCKS server failure
            client.write_all(&[0x05, 0x01, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await?;
            return Err(format!("Could not connect to destination: {}", e).into());
        }
    };

    // Reply success
    client.write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await?;

    info!("🔗 Tunnel established to {}", dest_addr);

    let (mut cr, mut cw) = client.into_split();
    let (mut sr, mut sw) = server.into_split();

    let client_to_server = tokio::io::copy(&mut cr, &mut sw);
    let server_to_client = tokio::io::copy(&mut sr, &mut cw);

    tokio::select! {
        _ = client_to_server => {}
        _ = server_to_client => {}
    }

    Ok(())
}
