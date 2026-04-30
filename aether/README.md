# Aether Ecosystem 🌐🛡️

Aether is a next-generation prototype for a decentralized peer-to-peer anonymity network, built from the ground up using Rust and modern Web Technologies. It features military-grade encryption, a built-in VPN/Proxy tunnel, and a futuristic real-time 3D telemetry dashboard.

## 🚀 Features

- **The Rust Core**: Powered by `libp2p`, ensuring decentralized peer discovery (Kademlia DHT, mDNS) without any central servers.
- **True Cryptographic Identity**: Every node generates an `Ed25519` keypair, providing a mathematically verifiable identity on the network.
- **End-to-End Encryption**: All P2P traffic utilizes the Noise Protocol (TLS 1.3 equivalent) and Yamux multiplexing.
- **Built-in SOCKS5 VPN**: A locally hosted proxy that allows any application (browsers, curl, mobile OS) to route its traffic securely through the Aether swarm.
- **Live Telemetry & Dashboard**: A `warp` WebSocket server streams real-time swarm events to a stunning Vite-powered `Three.js` interactive 3D globe.

---

## 📂 Project Structure

```
aether/
├── aether-core/        # The Rust Backend (Node, VPN Proxy, WebSocket Telemetry)
│   ├── src/
│   │   ├── main.rs      # Core libp2p Swarm & event loop
│   │   ├── socks5.rs    # SOCKS5 VPN tunnel interface
│   │   └── telemetry.rs # Warp WebSocket server for UI
│   └── Cargo.toml
│
└── aether-dashboard/   # The Web Frontend (Vite, Globe.gl, Three.js)
    ├── index.html       # UI Layout
    ├── style.css        # Cyberpunk aesthetics & styling
    ├── main.js          # WebSocket client & 3D Globe rendering
    └── package.json
```

---

## 🛠️ Installation & Setup

### Prerequisites
- [Rust & Cargo](https://rustup.rs/) (v1.70+)
- [Node.js](https://nodejs.org/) (v20+) & npm

### 1. Start the Rust Core (The Node & VPN)
The core handles P2P networking, the local proxy, and the telemetry server.
```bash
cd aether-core
cargo run
```
*Note: This binds the SOCKS5 proxy to `127.0.0.1:1080` and the WebSocket server to `127.0.0.1:8080`.*

### 2. Start the Global Dashboard (The UI)
In a new terminal window, run the Vite frontend to visualize your node.
```bash
cd aether-dashboard
npm install
npm run dev
```
*Open `http://localhost:5173` in your browser.*

---

## 🕵️‍♂️ Usage & Testing

Once both the Rust Core and Dashboard are running, you can test the built-in VPN routing:

1. Look at your running 3D dashboard.
2. In a new terminal, use `curl` to make a web request via the Aether proxy:
   ```bash
   curl -v --socks5 127.0.0.1:1080 http://example.com
   ```
3. Look back at the dashboard—you will instantly see the telemetry log: `Tunneling proxy request to example.com:80`.

---

## 🔮 Future Roadmap (Scaling to Global)
Currently, this repository acts as a highly advanced local proof-of-concept. To deploy this as a true global alternative to existing darknets:
1. Deploy `aether-core` to public VPS instances to serve as Bootstrap Nodes.
2. Implement true sub-stream multiplexing for proxy requests across the `libp2p` swarm.
3. Integrate WireGuard for native iOS/Android OS-level VPN support.
