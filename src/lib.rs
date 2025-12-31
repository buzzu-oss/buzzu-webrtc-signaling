use serde::{Deserialize, Serialize};
use worker::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SignalingMessage {
    Join { room_id: String, peer_id: String },
    Offer { from: String, to: String, #[serde(default)] sdp: Option<String>, #[serde(default)] sdp_compressed: Option<String> },
    Answer { from: String, to: String, #[serde(default)] sdp: Option<String>, #[serde(default)] sdp_compressed: Option<String> },
    IceCandidate { from: String, to: String, candidate: String },
    PeerList { peers: Vec<String> },
    Leave { peer_id: String },
    Error { message: String },
    Relay { from: String, to: String, via: String, payload: String, hop_count: u32, timestamp: u64 },
    RelayRequest { from: String, to: String, target_peer: String },
    RelayResponse { from: String, to: String, candidates: Vec<RelayCandidate> },
    Reachability { from: String, reachable_peers: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayCandidate {
    peer_id: String,
    rtt_ms: u32,
    reliability: f32,
}

#[durable_object]
pub struct RoomDurableObject {
    state: State,
}

impl DurableObject for RoomDurableObject {
    fn new(state: State, _env: Env) -> Self {
        Self { state }
    }

    async fn fetch(&self, req: Request) -> Result<Response> {
        // Check for WebSocket upgrade
        let upgrade = req.headers().get("Upgrade")?;
        if upgrade.map(|u| u == "websocket").unwrap_or(false) {
            // Create WebSocket pair
            let WebSocketPair { client, server } = WebSocketPair::new()?;
            
            // Accept the server side
            self.state.accept_web_socket(&server);
            
            // Get peer ID from query
            let url = req.url()?;
            let peer_id = url.query_pairs()
                .find(|(k, _)| k == "peer_id")
                .map(|(_, v)| v.to_string())
                .unwrap_or_else(|| format!("peer_{}", uuid::Uuid::new_v4()));
            
            // Attach peer ID as tag
            server.serialize_attachment(&peer_id)?;
            
            // Notify existing peers about new peer
            let websockets = self.state.get_websockets();
            let peer_list: Vec<String> = websockets.iter()
                .filter_map(|ws| {
                    ws.deserialize_attachment::<String>()
                        .ok()
                        .flatten()
                })
                .collect();
            
            // Send peer list to new peer
            let peer_list_msg = SignalingMessage::PeerList { peers: peer_list.clone() };
            if let Ok(json) = serde_json::to_string(&peer_list_msg) {
                let _ = server.send_with_str(&json);
            }
            
            // Notify others about new peer
            let join_msg = SignalingMessage::Join { 
                room_id: "".to_string(), 
                peer_id: peer_id.clone() 
            };
            if let Ok(json) = serde_json::to_string(&join_msg) {
                for ws in &websockets {
                    let other_peer: String = ws.deserialize_attachment::<String>()
                        .ok()
                        .flatten()
                        .unwrap_or_default();
                    if other_peer != peer_id {
                        let _ = ws.send_with_str(&json);
                    }
                }
            }
            
            Response::from_websocket(client)
        } else {
            Response::ok("Room Durable Object - WebSocket endpoint")
        }
    }
    
    async fn websocket_message(&self, ws: WebSocket, message: WebSocketIncomingMessage) -> Result<()> {
        let text = match message {
            WebSocketIncomingMessage::String(s) => s,
            WebSocketIncomingMessage::Binary(b) => String::from_utf8_lossy(&b).to_string(),
        };
        
        let from_peer: String = ws.deserialize_attachment::<String>()
            .ok()
            .flatten()
            .unwrap_or_default();
        
        if let Ok(msg) = serde_json::from_str::<SignalingMessage>(&text) {
            match msg {
                SignalingMessage::Offer { to, sdp, sdp_compressed, .. } => {
                    self.forward_to_peer(&to, SignalingMessage::Offer { 
                        from: from_peer, 
                        to: to.clone(), 
                        sdp,
                        sdp_compressed 
                    });
                }
                SignalingMessage::Answer { to, sdp, sdp_compressed, .. } => {
                    self.forward_to_peer(&to, SignalingMessage::Answer { 
                        from: from_peer, 
                        to: to.clone(), 
                        sdp,
                        sdp_compressed 
                    });
                }
                SignalingMessage::IceCandidate { to, candidate, .. } => {
                    self.forward_to_peer(&to, SignalingMessage::IceCandidate { 
                        from: from_peer, 
                        to: to.clone(), 
                        candidate 
                    });
                }
                SignalingMessage::Relay { to, via, payload, hop_count, timestamp, .. } => {
                    // If sender IS the relay (from == via), forward to target (to)
                    // Otherwise, forward to relay (via)
                    let target = if from_peer == via { &to } else { &via };
                    
                    self.forward_to_peer(target, SignalingMessage::Relay { 
                        from: from_peer, 
                        to: to.clone(), 
                        via: via.clone(), 
                        payload, 
                        hop_count, 
                        timestamp 
                    });
                }
                SignalingMessage::RelayRequest { to, target_peer, .. } => {
                    // Forward relay request to the target peer
                    self.forward_to_peer(&to, SignalingMessage::RelayRequest { 
                        from: from_peer, 
                        to: to.clone(), 
                        target_peer 
                    });
                }
                SignalingMessage::RelayResponse { to, candidates, .. } => {
                    // Forward relay response to the requesting peer
                    self.forward_to_peer(&to, SignalingMessage::RelayResponse { 
                        from: from_peer, 
                        to: to.clone(), 
                        candidates 
                    });
                }
                SignalingMessage::Reachability { reachable_peers, .. } => {
                    // Broadcast reachability to all peers (except sender)
                    let reach_msg = SignalingMessage::Reachability { 
                        from: from_peer.clone(), 
                        reachable_peers 
                    };
                    if let Ok(json) = serde_json::to_string(&reach_msg) {
                        for ws in self.state.get_websockets() {
                            let other_peer: String = ws.deserialize_attachment::<String>()
                                .ok()
                                .flatten()
                                .unwrap_or_default();
                            if other_peer != from_peer {
                                let _ = ws.send_with_str(&json);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        
        Ok(())
    }
    
    async fn websocket_close(&self, ws: WebSocket, _code: usize, _reason: String, _was_clean: bool) -> Result<()> {
        let peer_id: String = ws.deserialize_attachment::<String>()
            .ok()
            .flatten()
            .unwrap_or_default();
        
        // Notify other peers
        let leave_msg = SignalingMessage::Leave { peer_id: peer_id.clone() };
        if let Ok(json) = serde_json::to_string(&leave_msg) {
            for other_ws in self.state.get_websockets() {
                let other_peer: String = other_ws.deserialize_attachment::<String>()
                    .ok()
                    .flatten()
                    .unwrap_or_default();
                if other_peer != peer_id {
                    let _ = other_ws.send_with_str(&json);
                }
            }
        }
        
        Ok(())
    }
}

impl RoomDurableObject {
    fn forward_to_peer(&self, target_peer_id: &str, message: SignalingMessage) {
        if let Ok(json) = serde_json::to_string(&message) {
            for ws in self.state.get_websockets() {
                let peer_id: String = ws.deserialize_attachment::<String>()
                    .ok()
                    .flatten()
                    .unwrap_or_default();
                if peer_id == target_peer_id {
                    let _ = ws.send_with_str(&json);
                    break;
                }
            }
        }
    }
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let path = req.path();
    
    // CORS preflight
    if req.method() == Method::Options {
        return Response::ok("")
            .map(|r| add_cors_headers(r));
    }
    
    let response = match path.as_str() {
        "/" => Response::ok("BuzzU Signaling Server v1.0"),
        "/health" => Response::ok("OK"),
        _ if path.starts_with("/room/") => {
            // Extract room ID: /room/{room_id} or /room/{room_id}/websocket
            let room_id = path
                .strip_prefix("/room/")
                .and_then(|p| p.strip_suffix("/websocket").or(Some(p)))
                .unwrap_or("default");
            
            let namespace = env.durable_object("ROOMS")?;
            let id = namespace.id_from_name(room_id)?;
            let stub = id.get_stub()?;
            
            stub.fetch_with_request(req).await
        }
        _ => Response::error("Not Found", 404),
    };
    
    response.map(|r| add_cors_headers(r))
}

fn add_cors_headers(response: Response) -> Response {
    let headers = Headers::new();
    let _ = headers.set("Access-Control-Allow-Origin", "*");
    let _ = headers.set("Access-Control-Allow-Methods", "GET, POST, OPTIONS");
    let _ = headers.set("Access-Control-Allow-Headers", "Content-Type, Upgrade, Connection");
    
    response.with_headers(headers)
}