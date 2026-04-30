use futures::StreamExt;
use libp2p::{
    gossipsub, identify, identity, kad, mdns, noise, ping, swarm::NetworkBehaviour, swarm::SwarmEvent,
    tcp, yamux, PeerId,
};
use std::collections::hash_map::DefaultHasher;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use tracing_subscriber::EnvFilter;

mod socks5;
mod telemetry;

use telemetry::TelemetryEvent;
use tokio::sync::broadcast;

// We create a custom network behaviour that combines Gossipsub, Kademlia, Ping, Identify, and mDNS.
#[derive(NetworkBehaviour)]
struct AetherBehaviour {
    gossipsub: gossipsub::Behaviour,
    kademlia: kad::Behaviour<kad::store::MemoryStore>,
    mdns: mdns::tokio::Behaviour,
    ping: ping::Behaviour,
    identify: identify::Behaviour,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize tracing (logging)
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("aether_core=info".parse()?))
        .init();

    tracing::info!("Starting Aether Node...");

    // Create a broadcast channel for telemetry events
    let (tx, _rx) = broadcast::channel(100);
    let tx_clone = tx.clone();

    // Start Telemetry Server
    let tx_telemetry = tx.clone();
    tokio::spawn(async move {
        telemetry::start_telemetry_server(tx_telemetry, 8080).await;
    });

    // Start the local VPN (SOCKS5 Proxy)
    tokio::spawn(async move {
        socks5::start_socks5_server(1080, tx_clone).await;
    });

    // 1. Create a cryptographic identity for the node.
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    tracing::info!("Local Peer ID: {}", local_peer_id);

    // 2. Build the Swarm using the new SwarmBuilder API
    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(local_key.clone())
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_behaviour(|key| {
            // Gossipsub
            let message_id_fn = |message: &gossipsub::Message| {
                let mut s = DefaultHasher::new();
                message.data.hash(&mut s);
                gossipsub::MessageId::from(s.finish().to_string())
            };
            let gossipsub_config = gossipsub::ConfigBuilder::default()
                .heartbeat_interval(Duration::from_secs(10))
                .validation_mode(gossipsub::ValidationMode::Strict)
                .message_id_fn(message_id_fn)
                .build()
                .expect("Valid config");
            let mut gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                gossipsub_config,
            ).unwrap();
            let topic = gossipsub::IdentTopic::new("aether-telemetry");
            gossipsub.subscribe(&topic).unwrap();

            // Kademlia
            let store = kad::store::MemoryStore::new(local_peer_id);
            let kademlia = kad::Behaviour::new(local_peer_id, store);

            // mDNS
            let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id).unwrap();

            // Ping
            let ping = ping::Behaviour::new(ping::Config::default());

            // Identify
            let identify = identify::Behaviour::new(identify::Config::new(
                "/aether/1.0.0".to_string(),
                key.public(),
            ));

            Ok(AetherBehaviour {
                gossipsub,
                kademlia,
                mdns,
                ping,
                identify,
            })
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    // 3. Listen on all interfaces and an OS-assigned port
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    tracing::info!("Aether Swarm built and listening...");

    // 4. Run the event loop
    loop {
        tokio::select! {
            event = swarm.select_next_some() => match event {
                SwarmEvent::NewListenAddr { address, .. } => {
                    tracing::info!("Listening on {:?}", address);
                }
                SwarmEvent::Behaviour(AetherBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                    for (peer_id, _multiaddr) in list {
                        tracing::info!("mDNS discovered a new peer: {}", peer_id);
                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                        let _ = tx.send(TelemetryEvent::NodeDiscovered {
                            peer_id: peer_id.to_string(),
                        });
                    }
                }
                SwarmEvent::Behaviour(AetherBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                    for (peer_id, _multiaddr) in list {
                        tracing::warn!("mDNS discover peer has expired: {}", peer_id);
                        swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                        let _ = tx.send(TelemetryEvent::NodeDisconnected {
                            peer_id: peer_id.to_string(),
                        });
                    }
                }
                SwarmEvent::Behaviour(AetherBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                    propagation_source: peer_id,
                    message_id: id,
                    message,
                })) => {
                    tracing::info!(
                        "Got message: '{}' with id: {id} from peer: {:?}",
                        String::from_utf8_lossy(&message.data),
                        peer_id
                    );
                }
                _ => {}
            }
        }
    }
}
