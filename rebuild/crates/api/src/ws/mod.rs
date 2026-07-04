//! S6 realtime-WS surface (`docs/design/rebuild-realtime-s6-council/`) — the ONE `GET /ws` route,
//! its admission authn, the per-frame cross-tenant re-authz, and the `PgListener` fan-out
//! transport. Ports `apps/api/src/websocket.ts` (the whole WS server), `lib/courier-relay-guard.ts`
//! and `lib/courier-room-authz.ts` (ADR-0013), and `packages/platform/src/message-bus.ts`
//! (`PgMessageBus`), reusing the S2 verifier/extractor family verbatim (`crate::auth`) — no second
//! JWT verifier, no re-derived courier-liveness policy.
//!
//! Module map (mirrors the council packet's target architecture, proposal §3):
//!   - [`admission`]   — upgrade-time authn: token extraction (subprotocol/query/in-band) + the
//!     per-connection [`admission::Principal`], incl. the REV-S6-2 admission-time courier
//!     session-liveness bind.
//!   - [`protocol`]    — the wire contract: 5 inbound `ClientMsg` kinds, the typed control frames
//!     this binary itself originates, and the REV-S6-3 opaque `{room, data}` passthrough.
//!   - [`rooms`]       — the typed `Room` enum + the in-process `RoomRegistry` (the fan-out
//!     chokepoint — no raw member-socket access outside this module tree).
//!   - [`repo`]        — S6's own tri-state authz reads (owner membership / courier binding) that
//!     `crate::auth::repo::AuthRepo` does not carry (that trait owns identity/session, not
//!     per-room authorization).
//!   - [`guard`]       — the ADR-0013/`#4` fan-out re-authz: `CourierRelayGuard` (TTL + 60s
//!     ceiling, REV-S6-2 session-liveness folded in) and `OwnerRelayGuard` (TTL, no ceiling).
//!   - [`pg_fanout`]   — `PgListener` on the SESSION-mode DSN, the REV-S6-1 active heartbeat, and
//!     the REV-S6-6 claim-check→`Resync` translation.
//!
//! ## Mount (see `main.rs`)
//! Rides the SAME S2 auth-env gate as S3/S4/S5: `WsState` needs `AuthState`, so "auth env present"
//! is this surface's precondition too. `run_fanout` is spawned as a background task alongside the
//! router mount — the socket surface and the bus-consumer surface are two halves of one dark
//! launch (mounting ≠ launching; nothing here is reachable from a real client until the proxy
//! steers traffic to this stack, REV-S6-5).
//!
//! ## utoipa / OpenAPI (deliberately exempt)
//! `#[utoipa::path]` models an HTTP request/response pair; a WS upgrade is a `101 Switching
//! Protocols` handshake followed by a bidirectional frame stream with no fixed request/response
//! shape utoipa's schema can express. `openapi.rs` does not list `/ws` — this is a documented
//! omission, not an oversight (proposal §3 mount note; REBUILD-MAP §3 Phase-B: the cutover matcher
//! special-cases `Upgrade: websocket` BEFORE any path/method rule, so `/ws` was never going to be a
//! `(method, path)` OpenAPI operation to begin with).

pub mod admission;
pub mod guard;
pub mod pg_fanout;
pub mod protocol;
pub mod repo;
pub mod rooms;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use axum::Router;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Extension, Query};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio::time::{Duration, Instant};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::auth::AuthState;
use crate::config::Config;
use admission::{AdmissionError, Principal, build_principal, subprotocol_token};
use guard::{
    CourierRelayCtx, CourierRelayGuard, OwnerRelayCtx, OwnerRelayGuard, OwnerRoomKind, RelayOutcome,
};
use pg_fanout::{
    HEARTBEAT_TTL, Health, HeartbeatMonitor, NotifyFrame, PgFanout, interpret_notify_payload,
};
use protocol::{ClientLocationRelay, ClientMsg, ControlFrame, RoomEnvelope, WireMessage};
use repo::WsAuthzRepo;
use rooms::{MemberHandle, Room, RoomRegistry};

/// The in-band `ClientMsg::Auth` fallback deadline (`websocket.ts:357-362` parity).
const AUTH_DEADLINE: Duration = Duration::from_secs(5);
/// Server ping cadence; one missed round-trip terminates (`websocket.ts:287-297` parity).
const PING_INTERVAL: Duration = Duration::from_secs(30);
/// Sends the self-heartbeat probe well inside `HEARTBEAT_TTL` so a healthy listener always has a
/// fresh echo on hand before the TTL could lapse.
const HEARTBEAT_PROBE_INTERVAL: Duration = Duration::from_secs(4);

static NEXT_CONN_ID: AtomicU64 = AtomicU64::new(1);

/// A room's membership transitioned to/from empty — the fan-out task (`run_fanout`) is the only
/// thing allowed to `LISTEN`/`UNLISTEN` the shared `PgListener` connection, so connection tasks
/// signal it over this channel rather than reaching for the listener directly (single-owner).
pub enum RoomLifecycle {
    Listen(Room),
    Unlisten(Room),
}

/// The S6 shared state — cloned into every connection task and the router extension. Building it
/// also yields the `RoomLifecycle` receiver half for `run_fanout` (see [`WsState::build`]).
#[derive(Clone)]
pub struct WsState {
    pub auth: AuthState,
    pub ws_authz_repo: Arc<dyn WsAuthzRepo>,
    pub registry: Arc<RoomRegistry>,
    pub courier_guard: Arc<CourierRelayGuard>,
    pub owner_guard: Arc<OwnerRelayGuard>,
    /// `WS_URL_TOKEN_ACCEPT` (Q1) — the flagged `?token=` dual-accept transition transport.
    pub url_token_accept: bool,
    room_lifecycle_tx: mpsc::UnboundedSender<RoomLifecycle>,
    /// The listener health signal (REV-S6-1/REV-S6-6): the status-dot truth-signal binds to THIS,
    /// never raw socket liveness — see `pg_fanout`'s module doc.
    listener_healthy: Arc<AtomicBool>,
}

impl WsState {
    pub fn build(
        auth: AuthState,
        ws_authz_repo: Arc<dyn WsAuthzRepo>,
        url_token_accept: bool,
    ) -> (WsState, mpsc::UnboundedReceiver<RoomLifecycle>) {
        let (room_lifecycle_tx, room_lifecycle_rx) = mpsc::unbounded_channel();
        let courier_guard = Arc::new(CourierRelayGuard::new(
            ws_authz_repo.clone(),
            auth.repo.clone(),
        ));
        let owner_guard = Arc::new(OwnerRelayGuard::new(ws_authz_repo.clone()));
        let state = WsState {
            auth,
            ws_authz_repo,
            registry: Arc::new(RoomRegistry::new()),
            courier_guard,
            owner_guard,
            url_token_accept,
            room_lifecycle_tx,
            listener_healthy: Arc::new(AtomicBool::new(true)),
        };
        (state, room_lifecycle_rx)
    }

    /// Current listener health (proposal §11 — never folded into a binary up/down; this is the
    /// signal a future `/readyz` or metrics endpoint should read, not raw socket liveness).
    #[allow(
        dead_code,
        reason = "forward-looking observability seam — no /readyz or metrics endpoint reads this \
                  yet in this dark build; `run_fanout` already keeps it live via the heartbeat"
    )]
    pub fn listener_healthy(&self) -> bool {
        self.listener_healthy.load(Ordering::Relaxed)
    }
}

pub fn ws_router(state: WsState) -> Router {
    // JWT-in-URL redaction (Q1 dual-accept / ledger #42 / security-sweep P1): a `?token=<jwt>`
    // handshake carries a ~14d courier token and must NEVER reach a trace span or access log. This
    // scoped `TraceLayer` is the `/ws` route's ONLY tracing — axum applies a `.layer()` only to the
    // routes present at layer-time, so `build_router`'s global `TraceLayer::new_for_http()`
    // (main.rs), applied BEFORE this router is `.merge`d in, does not wrap `/ws`. The custom
    // `make_span_with` records a REDACTED uri (the `token` value stripped) and, by only ever
    // recording method + uri, never records a request HEADER — so `sec-websocket-protocol` (the
    // primary bearer transport) is structurally never logged either. This is the redaction the
    // `admission` module doc references.
    let trace = TraceLayer::new_for_http().make_span_with(|req: &axum::extract::Request| {
        tracing::info_span!(
            "ws_request",
            method = %req.method(),
            uri = %redact_ws_uri(req.uri()),
        )
    });
    Router::new()
        .route("/ws", get(ws_upgrade))
        .layer(trace)
        .layer(Extension(state))
}

/// Rebuild the request URI with the `token` query-param VALUE replaced by `REDACTED`, so the
/// `?token=<jwt>` dual-accept transport (Q1) never lands in a trace span / access log (JWT-in-URL,
/// ledger #42). The path and every OTHER query param survive so a span stays debuggable. Operates
/// on the RAW request uri (before the `Query` extractor runs), which is the only place the token
/// bytes are present.
fn redact_ws_uri(uri: &axum::http::Uri) -> String {
    let path = uri.path();
    match uri.query() {
        None => path.to_string(),
        Some(query) => {
            let redacted = query
                .split('&')
                .map(|pair| match pair.split_once('=') {
                    Some(("token", _)) => "token=REDACTED".to_string(),
                    _ => pair.to_string(),
                })
                .collect::<Vec<_>>()
                .join("&");
            format!("{path}?{redacted}")
        }
    }
}

#[derive(Debug, Deserialize)]
struct WsQuery {
    token: Option<String>,
}

/// The upgrade handler — token extraction happens BEFORE the handshake completes, so an invalid
/// PROVIDED token rejects the upgrade outright (a plain 401, never a socket that immediately
/// closes) — REV-S6-2's admission half. A client that offers NO pre-token at all still upgrades
/// and falls back to the in-band `ClientMsg::Auth` deadline (`handle_socket`), matching the Node
/// server's permissiveness there.
async fn ws_upgrade(
    ws: WebSocketUpgrade,
    Extension(state): Extension<WsState>,
    headers: HeaderMap,
    Query(query): Query<WsQuery>,
) -> Response {
    let offered_subprotocol = subprotocol_token(&headers);
    let pre_token = offered_subprotocol.clone().or_else(|| {
        if state.url_token_accept {
            query.token.filter(|t| !t.is_empty())
        } else {
            None
        }
    });

    let Some(token) = pre_token else {
        return ws.on_upgrade(move |socket| handle_socket(socket, state, None));
    };

    match build_principal(&state.auth, &token).await {
        Ok(principal) => {
            let mut upgrade = ws;
            if offered_subprotocol.is_some() {
                // Echo `bearer.v1` ONLY — never the token (RFC 6455 §4.2.2).
                upgrade = upgrade.protocols(["bearer.v1"]);
            }
            upgrade.on_upgrade(move |socket| handle_socket(socket, state, Some(principal)))
        }
        Err(AdmissionError::InvalidToken) => {
            (StatusCode::UNAUTHORIZED, "invalid token").into_response()
        }
        Err(AdmissionError::SessionRevoked) => {
            (StatusCode::UNAUTHORIZED, "session revoked").into_response()
        }
        Err(AdmissionError::RepoUnavailable) => {
            (StatusCode::SERVICE_UNAVAILABLE, "temporarily unavailable").into_response()
        }
    }
}

fn text_message(msg: impl Into<WireMessage>) -> Message {
    Message::Text(msg.into().into_text().into())
}

fn principal_role_str(principal: &Principal) -> &'static str {
    match principal {
        Principal::Owner { .. } => "owner",
        Principal::Courier { .. } => "courier",
        Principal::Customer { .. } => "customer",
    }
}

/// The per-connection lifecycle: admit (pre-token or in-band fallback) → `auth_success` → message
/// loop (subscribe/unsubscribe/client_location/client_location_stop) with a 30s server heartbeat →
/// on close, leave every room and signal `UNLISTEN` for any that emptied.
///
// DEFER (council Q8): mid-stream token `exp` re-check → distinct 4401 close is council-DEFERRED to
// a post-cutover FE-lockstep release (proposal §8 / resolution Q8). Admission verifies `exp` once
// (S2 verifier); nothing here re-checks it after the socket is open, so a token that expires
// mid-connection keeps the socket alive until the client reconnects. Flagged in-code (parity with
// guard.rs / protocol.rs's DEFER notes) so this is a known residual, not a silent omission.
async fn handle_socket(mut socket: WebSocket, state: WsState, principal: Option<Principal>) {
    let conn_id = NEXT_CONN_ID.fetch_add(1, Ordering::Relaxed);

    let principal = match principal {
        Some(p) => p,
        None => match wait_for_inband_auth(&state, &mut socket).await {
            Some(p) => p,
            None => {
                // `websocket.ts:357-362` parity — auth timeout closes 1008 with an explicit
                // reason (an inherent `.send()`, not `SinkExt::close()` — no new dependency).
                socket
                    .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                        code: 1008,
                        reason: "Authentication timeout".into(),
                    })))
                    .await
                    .ok();
                return;
            }
        },
    };

    let (frame_tx, mut frame_rx) = mpsc::unbounded_channel::<WireMessage>();
    if socket
        .send(text_message(ControlFrame::AuthSuccess {
            role: principal_role_str(&principal),
        }))
        .await
        .is_err()
    {
        return;
    }

    let mut ping_timer = tokio::time::interval(PING_INTERVAL);
    ping_timer.tick().await; // the first tick fires immediately — consume it so PING_INTERVAL is the real cadence.
    let mut awaiting_pong = false;

    loop {
        tokio::select! {
            outgoing = frame_rx.recv() => {
                match outgoing {
                    Some(msg) => { if socket.send(text_message(msg)).await.is_err() { break; } }
                    None => break,
                }
            }
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        if !dispatch_message(&state, &principal, conn_id, text.as_str(), &frame_tx).await {
                            break; // truly invalid JSON — close (websocket.ts's outer catch → close(1008)).
                        }
                    }
                    Some(Ok(Message::Pong(_))) => { awaiting_pong = false; }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(_)) => break,
                }
            }
            _ = ping_timer.tick() => {
                if awaiting_pong {
                    break; // zombie connection — one missed round-trip terminates.
                }
                awaiting_pong = true;
                if socket.send(Message::Ping(axum::body::Bytes::new())).await.is_err() {
                    break;
                }
            }
        }
    }

    for room in state.registry.leave_all(conn_id) {
        state
            .room_lifecycle_tx
            .send(RoomLifecycle::Unlisten(room))
            .ok();
    }
}

/// Waits up to [`AUTH_DEADLINE`] for `{type:'auth', token}`, verifying it the SAME way a pre-token
/// would be (`admission::build_principal`) — `None` on timeout/invalid shape/failed verify, which
/// the caller closes 1008 for (`websocket.ts:357-362,373-386` parity).
async fn wait_for_inband_auth(state: &WsState, socket: &mut WebSocket) -> Option<Principal> {
    let deadline = Instant::now() + AUTH_DEADLINE;
    loop {
        let remaining = deadline.checked_duration_since(Instant::now())?;
        let next = tokio::time::timeout(remaining, socket.recv())
            .await
            .ok()??;
        let Ok(Message::Text(text)) = next else {
            continue;
        };
        let Ok(ClientMsg::Auth { token }) = serde_json::from_str::<ClientMsg>(text.as_str()) else {
            continue; // an unrecognized first message keeps waiting out the deadline, then times out.
        };
        return build_principal(&state.auth, &token).await.ok();
    }
}

/// Dispatches ONE inbound text frame. Returns `false` only when the frame is not valid JSON at all
/// (`websocket.ts`'s outer `catch` → `ws.close(1008, 'Invalid message')`); an unrecognized `type`
/// or a malformed-but-known type is a no-op (console.warn parity — the socket stays open, protocol.rs's
/// module doc explains why this is intentionally lenient during the cutover overlap).
async fn dispatch_message(
    state: &WsState,
    principal: &Principal,
    conn_id: u64,
    text: &str,
    frame_tx: &mpsc::UnboundedSender<WireMessage>,
) -> bool {
    // A JSON-parse failure at ALL is the one case that closes the socket; an unrecognized `type`
    // or a shape mismatch on a known type is caught as a `ClientMsg` deserialize error below and
    // treated as a silent no-op instead (see fn doc).
    if serde_json::from_str::<serde_json::Value>(text).is_err() {
        return false;
    }
    let Ok(msg) = serde_json::from_str::<ClientMsg>(text) else {
        return true; // unrecognized type / malformed known type — warn-parity no-op.
    };

    match msg {
        ClientMsg::Auth { .. } => {} // already authenticated on this path — a stray re-auth is a no-op.
        ClientMsg::Subscribe { room } => {
            handle_subscribe(state, principal, conn_id, &room, frame_tx).await
        }
        ClientMsg::Unsubscribe { room } => handle_unsubscribe(state, conn_id, &room),
        ClientMsg::ClientLocation { payload } => {
            handle_client_location(state, principal, payload).await
        }
        ClientMsg::ClientLocationStop {} => handle_client_location_stop(state, principal).await,
    }
    true
}

/// The tri-state admission table (proposal §4, ADR-0013 parity — carry verbatim).
async fn handle_subscribe(
    state: &WsState,
    principal: &Principal,
    conn_id: u64,
    room_str: &str,
    frame_tx: &mpsc::UnboundedSender<WireMessage>,
) {
    let Some(room) = Room::parse(room_str) else {
        frame_tx
            .send(
                ControlFrame::Error {
                    error: "Forbidden room".to_string(),
                    retryable: None,
                }
                .into(),
            )
            .ok();
        return;
    };

    let verdict = match principal {
        Principal::Customer { order_id, .. } => {
            if room == Room::Order(*order_id) {
                repo::Verdict::Allow
            } else {
                repo::Verdict::Deny
            }
        }
        Principal::Owner { user_id } => match room {
            Room::LocationDashboard(loc) | Room::LocationCouriers(loc) => {
                state
                    .ws_authz_repo
                    .owner_location_verdict(*user_id, loc)
                    .await
            }
            Room::Order(order) => {
                state
                    .ws_authz_repo
                    .owner_order_verdict(*user_id, order)
                    .await
            }
            Room::CourierSelf(_) => repo::Verdict::Deny,
        },
        Principal::Courier {
            sub,
            active_location_id,
            ..
        } => match room {
            Room::CourierSelf(self_id) => {
                if self_id == *sub {
                    repo::Verdict::Allow
                } else {
                    repo::Verdict::Deny
                }
            }
            Room::Order(order) => {
                state
                    .ws_authz_repo
                    .courier_binding_verdict(*sub, *active_location_id, order)
                    .await
            }
            Room::LocationDashboard(_) | Room::LocationCouriers(_) => repo::Verdict::Deny,
        },
    };

    match verdict {
        repo::Verdict::Unavailable => {
            // WS-only retryable soft error — the socket stays open (courier-room-authz.ts's
            // tri-state distinction: a pool blip must not fleet-deny).
            frame_tx
                .send(
                    ControlFrame::Error {
                        error: "Service temporarily unavailable".to_string(),
                        retryable: Some(true),
                    }
                    .into(),
                )
                .ok();
        }
        repo::Verdict::Deny => {
            frame_tx
                .send(
                    ControlFrame::Error {
                        error: "Forbidden room".to_string(),
                        retryable: None,
                    }
                    .into(),
                )
                .ok();
        }
        repo::Verdict::Allow => {
            let is_first = state.registry.join(
                room,
                MemberHandle {
                    conn_id,
                    sender: frame_tx.clone(),
                    principal: principal.clone(),
                },
            );
            if is_first {
                state
                    .room_lifecycle_tx
                    .send(RoomLifecycle::Listen(room))
                    .ok();
            }
            frame_tx
                .send(
                    ControlFrame::Subscribed {
                        room: room_str.to_string(),
                    }
                    .into(),
                )
                .ok();
        }
    }
}

fn handle_unsubscribe(state: &WsState, conn_id: u64, room_str: &str) {
    let Some(room) = Room::parse(room_str) else {
        return;
    };
    if state.registry.leave(room, conn_id) {
        state
            .room_lifecycle_tx
            .send(RoomLifecycle::Unlisten(room))
            .ok();
    }
}

/// Customer GPS relay — targets ONLY courier members of the customer's own order room, each
/// binding-revalidated by the guard (`websocket.ts:458-476` parity).
async fn handle_client_location(
    state: &WsState,
    principal: &Principal,
    payload: protocol::ClientLocationInput,
) {
    let Principal::Customer { order_id, .. } = principal else {
        return;
    };
    if !(-90.0..=90.0).contains(&payload.lat) || !(-180.0..=180.0).contains(&payload.lng) {
        return; // out-of-range payload is silently dropped, matching websocket.ts:460-461.
    }
    let relay = ControlFrame::ClientLocation {
        payload: ClientLocationRelay {
            lat: payload.lat,
            lng: payload.lng,
            timestamp: chrono::Utc::now().timestamp_millis(),
        },
    };
    relay_to_couriers_in_order_room(state, *order_id, relay.into()).await;
}

async fn handle_client_location_stop(state: &WsState, principal: &Principal) {
    let Principal::Customer { order_id, .. } = principal else {
        return;
    };
    relay_to_couriers_in_order_room(state, *order_id, ControlFrame::ClientLocationStop {}.into())
        .await;
}

async fn relay_to_couriers_in_order_room(state: &WsState, order_id: Uuid, payload: WireMessage) {
    let room = Room::Order(order_id);
    let now = Instant::now();
    for member in state.registry.members_of(room) {
        let Principal::Courier {
            sub,
            active_location_id,
            jti,
        } = &member.principal
        else {
            continue;
        };
        let ctx = CourierRelayCtx {
            order_id,
            courier_sub: *sub,
            active_location_id: *active_location_id,
            jti: *jti,
        };
        match state.courier_guard.relay(ctx, now).await {
            RelayOutcome::Relayed => {
                member.sender.send(payload.clone()).ok();
            }
            RelayOutcome::Withheld => {}
            RelayOutcome::Evict => evict_member(state, room, &member, "binding_revoked"),
        }
    }
}

fn evict_member(state: &WsState, room: Room, member: &MemberHandle, reason: &str) {
    member
        .sender
        .send(
            ControlFrame::Error {
                error: reason.to_string(),
                retryable: None,
            }
            .into(),
        )
        .ok();
    if state.registry.leave(room, member.conn_id) {
        state
            .room_lifecycle_tx
            .send(RoomLifecycle::Unlisten(room))
            .ok();
    }
}

fn owner_room_kind(room: Room) -> Option<OwnerRoomKind> {
    match room {
        Room::LocationDashboard(l) | Room::LocationCouriers(l) => Some(OwnerRoomKind::Location(l)),
        Room::Order(o) => Some(OwnerRoomKind::Order(o)),
        Room::CourierSelf(_) => None,
    }
}

/// Bus-fanout dispatch — one parsed NOTIFY event, routed to every member of `room` through the
/// guard that matches their principal kind (customers relay directly: admission is authoritative
/// for them, proposal §5). This is the ONE place a bus event ever reaches a socket.
async fn fanout_to_room(state: &WsState, room: Room, payload: WireMessage, now: Instant) {
    for member in state.registry.members_of(room) {
        match &member.principal {
            Principal::Customer { .. } => {
                member.sender.send(payload.clone()).ok();
            }
            Principal::Owner { user_id } => {
                let Some(kind) = owner_room_kind(room) else {
                    continue;
                };
                match state
                    .owner_guard
                    .relay(
                        OwnerRelayCtx {
                            room: kind,
                            owner_user_id: *user_id,
                        },
                        now,
                    )
                    .await
                {
                    RelayOutcome::Relayed => {
                        member.sender.send(payload.clone()).ok();
                    }
                    RelayOutcome::Withheld => {}
                    RelayOutcome::Evict => evict_member(state, room, &member, "membership_revoked"),
                }
            }
            Principal::Courier {
                sub,
                active_location_id,
                jti,
            } => match room {
                // Self-scoped room (`courier:<self>`) — admission authorized it by an EXACT
                // `self_id == sub` match, and it carries no per-ORDER binding to revoke, so it
                // relays directly, exactly as Node's guard short-circuits a non-order room
                // (`courier-relay-guard.ts:116` `orderId === null → relay`). This is the branch that
                // delivers `task_offered`/`task_assigned` OFFERS; the previous order-only gate
                // mapped a CourierSelf room to "no order → continue" and silently starved couriers
                // of realtime offers (fail-closed, no leak — but no offers either). The `self_id ==
                // *sub` guard is belt-and-suspenders (admission already guarantees it) so "reaches
                // that courier and NOT another" holds structurally, not just by admission.
                Room::CourierSelf(self_id) if self_id == *sub => {
                    member.sender.send(payload.clone()).ok();
                }
                // Order room — the per-frame binding + session re-authz applies (the C1 leak path).
                Room::Order(order_id) => {
                    let ctx = CourierRelayCtx {
                        order_id,
                        courier_sub: *sub,
                        active_location_id: *active_location_id,
                        jti: *jti,
                    };
                    match state.courier_guard.relay(ctx, now).await {
                        RelayOutcome::Relayed => {
                            member.sender.send(payload.clone()).ok();
                        }
                        RelayOutcome::Withheld => {}
                        RelayOutcome::Evict => {
                            evict_member(state, room, &member, "binding_revoked")
                        }
                    }
                }
                // A courier is never admitted to a location/dashboard room, nor to another
                // courier's self-room; skip fail-closed rather than relay if one somehow appears.
                _ => {}
            },
        }
    }
}

/// The background bus-consumer: owns the single `PgListener` connection, drives the REV-S6-1
/// heartbeat, services `RoomLifecycle` LISTEN/UNLISTEN requests from connection tasks, and fans out
/// every NOTIFY it receives. Reconnects with the `pg_fanout::reconnect_backoff` cap, forever.
pub async fn run_fanout(
    state: WsState,
    config: Config,
    mut lifecycle_rx: mpsc::UnboundedReceiver<RoomLifecycle>,
) {
    let mut attempt: u32 = 0;
    loop {
        let mut fanout = match PgFanout::connect(&config).await {
            Ok(f) => f,
            Err(err) => {
                tracing::error!(%err, attempt, "PgFanout connect failed");
                state.listener_healthy.store(false, Ordering::Relaxed);
                attempt += 1;
                tokio::time::sleep(pg_fanout::reconnect_backoff(attempt)).await;
                continue;
            }
        };
        attempt = 0;
        state.listener_healthy.store(true, Ordering::Relaxed);
        let mut heartbeat = HeartbeatMonitor::new(HEARTBEAT_TTL);
        let mut probe_timer = tokio::time::interval(HEARTBEAT_PROBE_INTERVAL);

        loop {
            tokio::select! {
                _ = probe_timer.tick() => {
                    if fanout.send_heartbeat_probe(&mut heartbeat, Instant::now()).await.is_err() {
                        break;
                    }
                    let health = heartbeat.check(Instant::now());
                    state.listener_healthy.store(health == Health::Healthy, Ordering::Relaxed);
                }
                lifecycle = lifecycle_rx.recv() => {
                    match lifecycle {
                        Some(RoomLifecycle::Listen(room)) => { fanout.listen_room(&room.wire()).await.ok(); }
                        Some(RoomLifecycle::Unlisten(room)) => { fanout.unlisten_room(&room.wire()).await.ok(); }
                        None => return, // WsState (and every sender clone) dropped — shutting down.
                    }
                }
                received = fanout.recv() => {
                    match received {
                        Ok((channel, payload)) => {
                            if channel == pg_fanout::HEARTBEAT_CHANNEL {
                                if heartbeat.echo_received() {
                                    // REV-S6-6: degraded→healthy recovery — a NOTIFY may have been
                                    // lost mid-outage; nudge every live socket to refetch.
                                    state.listener_healthy.store(true, Ordering::Relaxed);
                                    let resync: WireMessage =
                                        ControlFrame::Resync { entity: "listener".to_string(), id: "recovered".to_string() }.into();
                                    for sender in state.registry.all_member_senders() {
                                        sender.send(resync.clone()).ok();
                                    }
                                }
                                continue;
                            }
                            let Some(room) = Room::parse(&channel) else { continue };
                            let now = Instant::now();
                            match interpret_notify_payload(&payload) {
                                NotifyFrame::Room(value) => {
                                    fanout_to_room(&state, room, RoomEnvelope { room: channel, data: value }.into(), now).await;
                                }
                                NotifyFrame::Resync { entity, id } => {
                                    fanout_to_room(&state, room, ControlFrame::Resync { entity, id }.into(), now).await;
                                }
                            }
                        }
                        Err(err) => {
                            tracing::warn!(%err, "PgFanout recv error — reconnecting");
                            break;
                        }
                    }
                }
            }
        }
        state.listener_healthy.store(false, Ordering::Relaxed);
        attempt += 1;
        tokio::time::sleep(pg_fanout::reconnect_backoff(attempt)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::repo::fake::FakeAuthRepo;
    use crate::ws::repo::fake::FakeWsAuthzRepo;
    use std::sync::{Arc, Mutex};

    fn test_ws_state() -> WsState {
        let (state, _rx) = WsState::build(
            crate::auth::AuthState::test_state(Arc::new(FakeAuthRepo::default())),
            Arc::new(FakeWsAuthzRepo::default()),
            true,
        );
        state
    }

    fn courier_member(
        conn_id: u64,
        sub: Uuid,
        loc: Uuid,
    ) -> (MemberHandle, mpsc::UnboundedReceiver<WireMessage>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (
            MemberHandle {
                conn_id,
                sender: tx,
                principal: Principal::Courier {
                    sub,
                    active_location_id: loc,
                    jti: None,
                },
            },
            rx,
        )
    }

    // ── Fix 2 (functional regression): a CourierSelf-room NOTIFY reaches that courier, not another ──

    #[tokio::test]
    async fn notify_to_a_courier_self_room_reaches_that_courier_not_another() {
        // Before the fix, `fanout_to_room`'s courier branch mapped a CourierSelf room to "no order
        // → continue", so `task_offered`/`task_assigned` offers were silently dropped. This is the
        // red→green guard: courier A (member of `courier:A`) must receive a NOTIFY published to
        // `courier:A`, and courier B (member of `courier:B`) must NOT.
        let state = test_ws_state();
        let loc = Uuid::new_v4();
        let courier_a = Uuid::new_v4();
        let courier_b = Uuid::new_v4();
        let (member_a, mut rx_a) = courier_member(1, courier_a, loc);
        let (member_b, mut rx_b) = courier_member(2, courier_b, loc);
        state.registry.join(Room::CourierSelf(courier_a), member_a);
        state.registry.join(Room::CourierSelf(courier_b), member_b);

        let payload: WireMessage = RoomEnvelope {
            room: Room::CourierSelf(courier_a).wire(),
            data: serde_json::value::RawValue::from_string(
                r#"{"type":"task_offered"}"#.to_string(),
            )
            .unwrap(),
        }
        .into();
        fanout_to_room(
            &state,
            Room::CourierSelf(courier_a),
            payload,
            Instant::now(),
        )
        .await;

        assert!(
            rx_a.try_recv().is_ok(),
            "courier A must receive the offer NOTIFY to their own self-room (the regression)"
        );
        assert!(
            rx_b.try_recv().is_err(),
            "courier B must NOT receive courier A's self-room NOTIFY (isolation)"
        );
    }

    #[tokio::test]
    async fn courier_order_room_still_goes_through_the_per_frame_guard() {
        // The order-room path keeps the ADR-0013 binding re-authz — a DENY verdict evicts, no frame
        // relayed. Proves the CourierSelf fix did not accidentally bypass the guard for order rooms.
        let ws_repo = Arc::new(FakeWsAuthzRepo::default());
        *ws_repo.courier_binding.lock().unwrap() = repo::Verdict::Deny;
        let (state, _rx) = WsState::build(
            crate::auth::AuthState::test_state(Arc::new(FakeAuthRepo::default())),
            ws_repo,
            true,
        );
        let loc = Uuid::new_v4();
        let courier = Uuid::new_v4();
        let order = Uuid::new_v4();
        let (member, mut rx) = courier_member(1, courier, loc);
        state.registry.join(Room::Order(order), member);

        let payload: WireMessage = RoomEnvelope {
            room: Room::Order(order).wire(),
            data: serde_json::value::RawValue::from_string(
                r#"{"type":"order.status"}"#.to_string(),
            )
            .unwrap(),
        }
        .into();
        fanout_to_room(&state, Room::Order(order), payload, Instant::now()).await;

        // DENY → evicted with a `binding_revoked` notice, the room frame itself never relayed.
        let first = rx.try_recv().expect("an eviction notice is sent");
        assert!(
            matches!(first, WireMessage::Control(ControlFrame::Error { ref error, .. }) if error == "binding_revoked"),
            "an order-room DENY must evict, not relay"
        );
        assert!(rx.try_recv().is_err(), "no room frame is relayed on a DENY");
    }

    // ── Fix 1 (HIGH): JWT-in-URL redaction ──

    #[test]
    fn redact_ws_uri_strips_the_token_value_but_keeps_other_params() {
        let uri: axum::http::Uri = "/ws?token=SUPERSECRETJWT&foo=bar".parse().unwrap();
        let redacted = redact_ws_uri(&uri);
        assert_eq!(redacted, "/ws?token=REDACTED&foo=bar");
        assert!(
            !redacted.contains("SUPERSECRETJWT"),
            "the raw token value must never survive redaction"
        );
    }

    #[test]
    fn redact_ws_uri_handles_token_only_middle_position_and_no_query() {
        assert_eq!(
            redact_ws_uri(&"/ws?token=abc.def.ghi".parse().unwrap()),
            "/ws?token=REDACTED"
        );
        assert_eq!(redact_ws_uri(&"/ws".parse().unwrap()), "/ws");
        assert_eq!(
            redact_ws_uri(&"/ws?foo=1&token=x.y.z&bar=2".parse().unwrap()),
            "/ws?foo=1&token=REDACTED&bar=2",
            "a token anywhere in the query is redacted, others preserved"
        );
    }

    /// A shared in-memory buffer the fmt subscriber writes into, so the test can assert on exactly
    /// what was recorded to a span/log.
    #[derive(Clone, Default)]
    struct SharedBuf(Arc<Mutex<Vec<u8>>>);
    impl std::io::Write for SharedBuf {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl tracing_subscriber::fmt::MakeWriter<'_> for SharedBuf {
        type Writer = SharedBuf;
        fn make_writer(&self) -> Self::Writer {
            self.clone()
        }
    }

    /// The wiring proof the coordinator asked for: drive a real `?token=<jwt>` request through the
    /// actual `ws_router`, capture EVERYTHING the tracing layer records, and assert the raw token
    /// never appears while the `ws_request` span is recorded with the token param redacted. This
    /// fails if `make_span_with` were removed / recorded the raw uri, or if any layer logged the
    /// full URI.
    #[tokio::test]
    async fn ws_route_trace_span_records_a_redacted_uri_not_the_raw_token() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;
        use tracing_subscriber::fmt::format::FmtSpan;

        let buf = SharedBuf::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(buf.clone())
            .with_ansi(false)
            .with_max_level(tracing::Level::TRACE)
            .with_span_events(FmtSpan::NEW)
            .finish();
        // `#[tokio::test]` runs on a current-thread runtime, so this thread-local default subscriber
        // stays active while the oneshot future is polled on the same thread.
        let _guard = tracing::subscriber::set_default(subscriber);

        let app = ws_router(test_ws_state());
        let _resp = app
            .oneshot(
                Request::builder()
                    .uri("/ws?token=SUPERSECRETJWT.payload.sig&foo=bar")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        drop(_guard);

        let logged = String::from_utf8(buf.0.lock().unwrap().clone()).unwrap();
        assert!(
            logged.contains("ws_request"),
            "the scoped ws_request span must have been recorded: {logged:?}"
        );
        assert!(
            !logged.contains("SUPERSECRETJWT"),
            "the raw ?token= value must NEVER appear in any recorded span/log line: {logged:?}"
        );
        assert!(
            logged.contains("token=REDACTED"),
            "the token param must be recorded redacted, proving make_span_with used redact_ws_uri: {logged:?}"
        );
    }
}
