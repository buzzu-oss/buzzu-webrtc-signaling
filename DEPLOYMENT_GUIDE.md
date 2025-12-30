# Deploying BuzzU Signaling Server

## Manual Deployment Steps

Since OAuth authentication is required for Cloudflare Workers deployment, here are the manual steps to deploy your signaling server:

### 1. Login to Cloudflare
```bash
npx wrangler login
```

### 2. Deploy the Worker
```bash
npx wrangler deploy
```

### 3. Update Your React App
Once deployed, update your WebSocket connection URL in your React app:

```typescript
// Replace with your actual worker URL
const WORKER_URL = 'wss://buzzu-signaling.YOUR_SUBDOMAIN.workers.dev';

const ws = new WebSocket(`${WORKER_URL}/room/${roomId}/websocket`);
```

### 4. Test the Connection
Test the signaling server with a simple WebSocket connection:

```javascript
const ws = new WebSocket('wss://buzzu-signaling.YOUR_SUBDOMAIN.workers.dev/room/test123/websocket');

ws.onopen = () => {
  console.log('Connected to signaling server');
  ws.send(JSON.stringify({
    type: 'Join',
    peer_id: 'test-peer-1'
  }));
};

ws.onmessage = (event) => {
  console.log('Received:', event.data);
};
```

### Worker Configuration
Your worker is configured with:
- **Name**: buzzu-signaling
- **Durable Objects**: ROOMS binding for RoomDurableObject
- **Compatibility Date**: 2024-01-01

### Features Implemented
✅ WebSocket signaling for WebRTC
✅ Room-based peer management
✅ Offer/Answer/ICE candidate routing
✅ Peer join/leave notifications
✅ CORS support
✅ Error handling and logging

The signaling server is ready for deployment! 🚀