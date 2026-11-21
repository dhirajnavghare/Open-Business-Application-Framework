// Copyright 2026 Dhiraj Navghare
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use axum::{
    async_trait,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        FromRequestParts, State,
    },
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
    response::Response,
};
use obaf_core::verify_token;
use tracing::info;
use uuid::Uuid;

use crate::state::SharedState;

pub struct WsAuthUser {
    pub tenant_id: Uuid,
    pub is_superadmin: bool,
}

#[async_trait]
impl FromRequestParts<SharedState> for WsAuthUser {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &SharedState,
    ) -> Result<Self, Self::Rejection> {
        // 1. Extract authentication token from Authorization header or ?token= query parameter
        let raw_token = if let Some(auth) = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
        {
            auth.strip_prefix("Bearer ").map(|t| t.to_string())
        } else {
            parts.uri.query().and_then(|q| {
                q.split('&').find_map(|pair| {
                    let mut p = pair.split('=');
                    if p.next() == Some("token") {
                        p.next().map(|v| v.to_string())
                    } else {
                        None
                    }
                })
            })
        };

        let token = match raw_token {
            Some(t) if !t.trim().is_empty() => t,
            _ => return Err((StatusCode::UNAUTHORIZED, "Missing authentication token")),
        };

        if state.is_token_revoked(&token) {
            return Err((
                StatusCode::UNAUTHORIZED,
                "Authentication token has been revoked",
            ));
        }

        // 2. Verify token
        let claims = verify_token(&token, &state.jwt_secret)
            .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid authentication token"))?;

        if claims.role == "superadmin_mfa_pending" {
            return Err((StatusCode::UNAUTHORIZED, "MFA verification pending"));
        }

        Ok(WsAuthUser {
            tenant_id: claims.tenant_id.0,
            is_superadmin: claims.role == "superadmin",
        })
    }
}

pub async fn ws_handler(
    auth: WsAuthUser,
    ws: WebSocketUpgrade,
    State(state): State<SharedState>,
) -> Response {
    let tenant_id = auth.tenant_id;
    let is_superadmin = auth.is_superadmin;

    ws.max_message_size(64 * 1024)
        .max_frame_size(16 * 1024)
        .on_upgrade(move |socket| handle_socket(socket, state, tenant_id, is_superadmin))
}

async fn handle_socket(
    mut socket: WebSocket,
    state: SharedState,
    client_tenant_id: Uuid,
    is_superadmin: bool,
) {
    info!(
        "Authenticated WebSocket client connected (tenant: {}, superadmin: {})",
        client_tenant_id, is_superadmin
    );
    let mut receiver = state.event_sender.subscribe();

    let mut ping_interval = tokio::time::interval(std::time::Duration::from_secs(30));
    ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            // Heartbeat keep-alive to prevent intermediate proxy / NAT timeouts and detect dead clients
            _ = ping_interval.tick() => {
                if socket.send(Message::Ping(vec![])).await.is_err() {
                    break;
                }
            }
            // Monitor client socket for disconnects, ping/pong, or close frames
            client_msg = socket.recv() => {
                match client_msg {
                    Some(Ok(Message::Close(_))) | None => {
                        break;
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        // RFC 6455: Must reply with Pong containing identical payload
                        if socket.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {
                        // Client acknowledged heartbeat ping
                    }
                    Some(Err(_)) => {
                        break;
                    }
                    _ => {
                        // Keep connection alive, ignore incoming payload on read-only stream
                    }
                }
            }
            // Stream broadcast events to client
            broadcast = receiver.recv() => {
                match broadcast {
                    Ok(msg_text) => {
                        if is_superadmin {
                            if socket.send(Message::Text(msg_text)).await.is_err() {
                                break;
                            }
                            continue;
                        }

                        // Tenant users strictly receive only their own tenant's events
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&msg_text) {
                            if let Some(msg_tenant_str) = val.get("tenant_id").and_then(|v| v.as_str()) {
                                if let Ok(msg_tenant_id) = Uuid::parse_str(msg_tenant_str) {
                                    if msg_tenant_id != client_tenant_id {
                                        // Drop cross-tenant message for data isolation security
                                        continue;
                                    }
                                } else {
                                    continue;
                                }
                            } else {
                                continue;
                            }
                        }

                        if socket.send(Message::Text(msg_text)).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        // Slow consumer lagged, skip dropped messages
                        continue;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
        }
    }
}
