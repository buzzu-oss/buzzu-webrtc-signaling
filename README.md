# BuzzU WebRTC Signaling Server

A Rust-based WebSocket signaling server for WebRTC peer connection, running on Cloudflare Workers with Durable Objects.

## Features

- **Pure WebSocket** - Real-time signaling for WebRTC
- **Durable Objects** - Room isolation and state management
- **Hibernatable** - Efficient memory usage during idle periods
- **Rust/WASM** - High performance and type safety

## Message Types

```json
{ "type": "Join", "room_id": "abc", "peer_id": "peer1" }
{ "type": "Offer", "from": "peer1", "to": "peer2", "sdp": "..." }
{ "type": "Answer", "from": "peer2", "to": "peer1", "sdp": "..." }
{ "type": "IceCandidate", "from": "peer1", "to": "peer2", "candidate": "..." }
{ "type": "PeerList", "peers": ["peer1", "peer2"] }
{ "type": "Leave", "peer_id": "peer1" }
```

## Endpoints

- `GET /` - Server info
- `GET /health` - Health check
- `GET /room/{room_id}` - WebSocket connection
- `GET /room/{room_id}/websocket?peer_id=xxx` - WebSocket with peer ID

## Deployment

### Prerequisites

1. Cloudflare account with Workers paid plan (for Durable Objects)
2. Add secrets to GitHub repo:
   - `CF_API_TOKEN` - Cloudflare API token with Workers permissions
   - `CF_ACCOUNT_ID` - Your Cloudflare account ID

### Deploy via GitHub Actions

Push to `main` branch to trigger automatic deployment.

### Manual Deployment

```bash
npm install -g wrangler
cargo install worker-build
worker-build --release
wrangler deploy
```

## Client Usage

```javascript
const ws = new WebSocket('wss://your-worker.workers.dev/room/my-room?peer_id=my-peer');

ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  // Handle signaling messages
};

// Send offer
ws.send(JSON.stringify({
  type: 'Offer',
  to: 'other-peer-id',
  sdp: offer.sdp
}));
```

## License

MIT
