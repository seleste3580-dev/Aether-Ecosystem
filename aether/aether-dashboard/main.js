import Globe from 'globe.gl';

// Configure Globe
const globeContainer = document.getElementById('globe-container');
const world = Globe()
  (globeContainer)
  .globeImageUrl('//unpkg.com/three-globe/example/img/earth-dark.jpg')
  .bumpImageUrl('//unpkg.com/three-globe/example/img/earth-topology.png')
  .backgroundColor('#050505')
  .pointColor(() => '#00ffc8')
  .pointAltitude(0.02)
  .pointRadius(0.5)
  .arcColor(() => 'rgba(0, 255, 200, 0.5)')
  .arcDashLength(0.4)
  .arcDashGap(0.2)
  .arcDashAnimateTime(1500)
  .arcAltitudeAutoScale(0.2)
  .arcsTransitionDuration(1000);

// Add some ambient rotation
world.controls().autoRotate = true;
world.controls().autoRotateSpeed = 1.0;

// Set initial camera position
world.pointOfView({ lat: 20, lng: 0, altitude: 2.5 });

// Data management
let nodes = [];
let connections = [];

function updateGlobeData() {
  world.pointsData(nodes);
  world.arcsData(connections);
  document.getElementById('node-count').innerText = nodes.length;
}

function addLog(msg, isNewNode = false) {
  const logContainer = document.getElementById('log-container');
  const div = document.createElement('div');
  div.className = 'log-entry' + (isNewNode ? ' new-node' : '');
  div.innerText = `[${new Date().toLocaleTimeString()}] ${msg}`;
  logContainer.prepend(div);
}

// Connect to Rust Telemetry WebSocket
const ws = new WebSocket('ws://127.0.0.1:8080/telemetry');

ws.onopen = () => {
  document.getElementById('connection-status').innerText = 'SECURE P2P LINK';
  document.querySelector('.indicator').classList.add('online');
  addLog('Connected to Aether Core Telemetry.');
};

ws.onmessage = (event) => {
  try {
    const data = JSON.parse(event.data);
    
    if (data.NodeDiscovered) {
      const peerId = data.NodeDiscovered.peer_id;
      // Add a random node to the globe for visualization
      const lat = (Math.random() - 0.5) * 160;
      const lng = (Math.random() - 0.5) * 360;
      
      const newNode = { lat, lng, id: peerId };
      nodes.push(newNode);
      addLog(`Discovered peer: ${peerId.substring(0,8)}...`, true);
      
      // Connect to a previous node
      if (nodes.length > 1) {
        const prev = nodes[nodes.length - 2];
        connections.push({
          startLat: prev.lat,
          startLng: prev.lng,
          endLat: lat,
          endLng: lng
        });
      }
      updateGlobeData();
    }
    else if (data.NodeDisconnected) {
      const peerId = data.NodeDisconnected.peer_id;
      addLog(`Peer disconnected: ${peerId.substring(0,8)}...`);
      // Keep them on the globe for visual effect, or remove them
    }
    else if (data.ProxyRequest) {
      const dest = data.ProxyRequest.dest;
      addLog(`Tunneling proxy request to ${dest}`, true);
    }
  } catch (e) {
    console.error('Failed to parse telemetry', e);
  }
};

ws.onclose = () => {
  document.getElementById('connection-status').innerText = 'CORE OFFLINE';
  document.querySelector('.indicator').classList.remove('online');
  addLog('Lost connection to Aether Core.');
};

// Handle window resize
window.addEventListener('resize', () => {
  world.width(window.innerWidth);
  world.height(window.innerHeight);
});
