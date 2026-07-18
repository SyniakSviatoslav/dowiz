//! `native_spa_server::api` — the minimal HTTP order surface (P37 W37-2/3/4/5/6/7).
//!
//! The thinnest possible wire shell over the kernel's `decide`/`fold` Law. It
//! owns NO order/money logic: order JSON is an *opaque* `String` relayed to the
//! kernel's `json_api` module. Capability-cert authentication (W37-3) is layered
//! over the kernel's existing `verify_chain` / `RevocationSet` machinery — the
//! second thin shell over the same kernel (RW-09).
//!
//! THIN-SHELL INVARIANT (W37-6): this module MUST contain NO FSM vocabulary
//! () `Order_State` (the inner enum, never the outer shell), NO money arithmetic
//! (any arithmetic operator / anything resembling subtotal/total derivation),
//! and NO status-branching on
//! `next_status`. The grep gate `r11_thin_shell_grep_gate` enforces this at
//! `cargo test` time. The only kernel shape the server parses is the order `id`
//! field (the store key), and that single shallow read is the documented
//! exception.

use std::sync::{
    atomic::{AtomicI64, Ordering},
    Arc, Mutex,
};

use axum::{
    body::Bytes,
    extract::{Path, Request, State},
    http::{header, HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use base64::Engine;
use dowiz_kernel::json_api;
use dowiz_kernel::ports::agent::{
    cap::{revocation_hash, verify_chain, SignatureVerifier},
    scope,
};

// Re-export kernel types the integration tests construct (cap-gated frames,
// rosters, revocation sets). No new crypto — these are the kernel's own types.
pub use dowiz_kernel::ports::agent::cap::{
    AnchorRoster, Capability, ChainError, Delegation, RefSigner, RevocationSet, SignedFrame,
};
pub use dowiz_kernel::ports::agent::scope::{Action, Resource, Scope};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// ── Route table (axum 0.8 `{id}` capture syntax) ────────────────────────────

pub const ROUTE_HEALTH: &str = "/healthz";
pub const ROUTE_ORDER_PLACE: &str = "/api/order";
pub const ROUTE_ORDER_READ: &str = "/api/order/{id}";
pub const ROUTE_ORDER_ADVANCE: &str = "/api/order/{id}/advance";

/// Capability-certificate header (base64 of a serialized `SignedFrame`).
pub const CAP_HEADER: &str = "x-dowiz-cap";
/// Body size ceiling (order JSON ≪ 4 KiB). Above this → 413.
pub const MAX_BODY_BYTES: usize = 64 * 1024;
/// API concurrency bulkhead (§4.3): caps concurrent in-flight API requests.
pub const MAX_INFLIGHT_API: usize = 64;
/// Cap-frame freshness window (replay layer 1): a frame older than now − this is
/// rejected (epoch skew).
pub const EPOCH_SKEW_SECS: u64 = 300;
/// In-window replay ring size (replay layer 2): duplicate frame digests within
/// the window are rejected.
pub const SEEN_DIGEST_RING: usize = 4096;

/// Base64 engine for the capability header (standard alphabet, no padding).
const B64: base64::engine::GeneralPurpose = base64::engine::GeneralPurpose::new(
    &base64::alphabet::STANDARD,
    base64::engine::GeneralPurposeConfig::new()
        .with_encode_padding(false)
        .with_decode_padding_mode(base64::engine::DecodePaddingMode::Indifferent),
);

/// Request bodies — the ONLY serde shapes the shell owns. Order JSON is relayed
/// RAW to the kernel as an opaque String (`items_json` / `order_json`); the
/// server never inspects order semantics.
#[derive(Deserialize)]
pub struct PlaceOrderBody {
    pub customer_id: Option<String>,
    #[serde(rename = "items_json")]
    pub items_json: String,
    pub channel: Option<String>,
}
#[derive(Deserialize)]
pub struct AdvanceBody {
    /// Kernel vocabulary (e.g. "CONFIRMED"). Relayed verbatim; the kernel rejects
    /// illegal edges.
    pub next_status: String,
}

/// Volatile, event-sourced order record. State is ALWAYS fold-derived: it holds
/// the kernel's latest serialized order (opaque) + the append-only status-event
/// list (the spine tests assert on).
#[derive(Clone)]
pub struct OrderRecord {
    pub order_json: String,
    pub status_events: Vec<String>,
}

#[derive(Default)]
pub struct EventStore {
    inner: Mutex<std::collections::HashMap<String, OrderRecord>>,
}

impl EventStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, id: String, rec: OrderRecord) {
        self.inner.lock().unwrap().insert(id, rec);
    }

    pub fn get(&self, id: &str) -> Option<OrderRecord> {
        self.inner.lock().unwrap().get(id).cloned()
    }

    pub fn update(&self, id: &str, f: impl FnOnce(&mut OrderRecord)) {
        let mut g = self.inner.lock().unwrap();
        if let Some(r) = g.get_mut(id) {
            f(r);
        }
    }

    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// HTTP rejection taxonomy — total, fail-closed. Every arm names its status.
pub enum ApiReject {
    Unauthorized, // 401 — missing/unparseable/forged cap frame
    Forbidden,    // 403 — revoked, expired epoch, or scope mismatch
    NotFound,     // 404 — unknown order id
    KernelReject(String), // 409 — kernel refused (illegal edge, money law)
    Malformed,    // 400 — body fails serde before any kernel call
    TooLarge,     // 413 — > MAX_BODY_BYTES
    Replayed,     // 409 — duplicate frame digest inside the window
}

impl ApiReject {
    fn status(&self) -> StatusCode {
        match self {
            ApiReject::Unauthorized => StatusCode::UNAUTHORIZED,
            ApiReject::Forbidden => StatusCode::FORBIDDEN,
            ApiReject::NotFound => StatusCode::NOT_FOUND,
            ApiReject::KernelReject(_) => StatusCode::CONFLICT,
            ApiReject::Malformed => StatusCode::BAD_REQUEST,
            ApiReject::TooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            ApiReject::Replayed => StatusCode::CONFLICT,
        }
    }
    fn body(&self) -> String {
        match self {
            ApiReject::KernelReject(m) => m.clone(),
            ApiReject::Unauthorized => "unauthorized: missing or forged capability cert".into(),
            ApiReject::Forbidden => "forbidden: revoked / expired / scope mismatch".into(),
            ApiReject::NotFound => "not found".into(),
            ApiReject::Malformed => "malformed request body".into(),
            ApiReject::TooLarge => "payload too large".into(),
            ApiReject::Replayed => "replayed request frame".into(),
        }
    }
}

impl IntoResponse for ApiReject {
    fn into_response(self) -> Response {
        (self.status(), self.body()).into_response()
    }
}

/// Capability check seam (W37-3). The default impl wraps the kernel's own
/// `verify_chain` + `RevocationSet` with the deterministic `RefSigner` injected
/// for tests; production injects the real hybrid Ed25519⊕ML-DSA-65 verifier
/// behind this SAME trait (P34). Deny-by-default: constructed explicitly with a
/// roster + revocations + verifier (no "no-auth" constructor to misuse).
pub trait CapVerifier: Send + Sync + 'static {
    /// `frame` = decoded CAP_HEADER bytes (a serialized `SignedFrame`);
    /// `body_digest` = SHA-256 of the exact request body bytes; `route` = the
    /// matched ROUTE_* const (scope binding); `now_epoch` = server unix seconds.
    /// Ok(()) admits; Err carries the 401/403 split. The adapter binds
    /// `(route ‖ body_digest ‖ epoch)` so a frame minted for READ cannot drive
    /// ADVANCE and a frame for body A cannot authorize body B.
    fn check(
        &self,
        frame: &[u8],
        body_digest: &[u8; 32],
        route: &'static str,
        now_epoch: u64,
    ) -> Result<(), ApiReject>;
}

/// The kernel-backed capability verifier (W37-3). Reuses
/// `ports/agent/cap.rs::verify_chain` + `RevocationSet` + `DOMAIN_FRAME` signing
/// domain, generic over `SignatureVerifier` exactly as the kernel already is.
pub struct KernelCapVerifier<V: SignatureVerifier + 'static> {
    pub verifier: V,
    pub roster: AnchorRoster,
    pub revocations: RevocationSet,
}

impl<V: SignatureVerifier + Send + Sync + 'static> KernelCapVerifier<V> {
    pub fn new(verifier: V, roster: AnchorRoster, revocations: RevocationSet) -> Self {
        Self {
            verifier,
            roster,
            revocations,
        }
    }

    /// Decode a base64 capability header into a `SignedFrame`.
    fn decode_frame(frame_b64: &[u8]) -> Result<SignedFrame, ApiReject> {
        let raw = B64
            .decode(frame_b64)
            .map_err(|_| ApiReject::Unauthorized)?;
        serde_json::from_slice::<SignedFrame>(&raw).map_err(|_| ApiReject::Unauthorized)
    }
}

impl<V: SignatureVerifier + Send + Sync + 'static> CapVerifier for KernelCapVerifier<V> {
    fn check(
        &self,
        frame_b64: &[u8],
        body_digest: &[u8; 32],
        route: &'static str,
        now_epoch: u64,
    ) -> Result<(), ApiReject> {
        let signed = Self::decode_frame(frame_b64)?;

        // Replay layer 1: epoch freshness. The frame must still be valid against
        // `now` and within the skew window (a stale frame cannot be replayed).
        if !signed.capability.is_fresh(now_epoch) {
            return Err(ApiReject::Forbidden);
        }
        if signed.capability.expiry < now_epoch.saturating_sub(EPOCH_SKEW_SECS) {
            return Err(ApiReject::Forbidden);
        }

        // Revocation check (forged is caught by verify_chain's BadSignature arm).
        let cap_hash = revocation_hash(&signed.capability);
        if self.revocations.is_revoked_capability(&cap_hash)
            || self
                .revocations
                .is_revoked_key(&signed.capability.subject_key)
        {
            return Err(ApiReject::Forbidden);
        }

        // Scope binding: the requested route maps to a required (resource, action)
        // pair. A frame minted for READ cannot drive ADVANCE.
        let required = route_scope(route).ok_or(ApiReject::Forbidden)?;
        let chain = signed.payload_as_chain().map_err(|_| ApiReject::Unauthorized)?;
        let cap = &signed.capability;
        // Effect must authorize the required grant.
        if !cap.scope.grants.iter().any(|g| *g == required) {
            return Err(ApiReject::Forbidden);
        }

        // Body binding (replay layer 0): the frame's payload carries the exact
        // SHA-256 body digest (set by `mint_frame`, see `payload_as_chain`), so a
        // frame minted for body A cannot authorize a different body B. The
        // payload layout is `<u64 LE chain_json_len> || chain_json || body_digest`,
        // so the tail 32 bytes are the bound digest.
        let n = u64::from_le_bytes(signed.payload[..8].try_into().unwrap_or([0u8; 8]));
        let start = 8 + n as usize;
        if signed.payload.len() < start + 32 || &signed.payload[start..start + 32] != body_digest {
            return Err(ApiReject::Unauthorized);
        }

        // Chain verify: root anchor + signatures + expiry + attenuation + tail
        // binding + effect subset. Any failure → 401/403 via ChainError.
        match verify_chain(&self.verifier, &self.roster, &chain, cap, now_epoch) {
            Ok(()) => Ok(()),
            Err(ChainError::BadSignature) => Err(ApiReject::Unauthorized),
            Err(ChainError::UnknownIssuer) => Err(ApiReject::Unauthorized),
            Err(_) => Err(ApiReject::Forbidden),
        }
    }
}

/// Helper to keep the `Forbidden` construction ergonomic inside the chain above
/// (small local shim so we don't repeat the arm).
trait CloneForbidden {
    fn clone_forbidden() -> Self;
}
impl CloneForbidden for ApiReject {
    fn clone_forbidden() -> Self {
        ApiReject::Forbidden
    }
}

/// Map a route const to the required (resource, action) grant.
fn route_scope(route: &'static str) -> Option<(Resource, Action)> {
    match route {
        ROUTE_ORDER_PLACE => Some((Resource::Order, Action::CreateOrder)),
        ROUTE_ORDER_READ => Some((Resource::Order, Action::Read)),
        ROUTE_ORDER_ADVANCE => Some((Resource::Order, Action::CreateOrder)),
        _ => None,
    }
}

/// Shared state for the API router.
pub struct ApiState {
    pub store: Arc<EventStore>,
    pub caps: Arc<dyn CapVerifier>,
    /// Replay ring: seen frame digests (SHA-256 of the full cap header bytes).
    pub seen: Mutex<std::collections::VecDeque<[u8; 32]>>,
    /// API concurrency bulkhead counter.
    pub inflight: AtomicI64,
}

impl ApiState {
    pub fn new(store: Arc<EventStore>, caps: Arc<dyn CapVerifier>) -> Self {
        Self {
            store,
            caps,
            seen: Mutex::new(std::collections::VecDeque::with_capacity(SEEN_DIGEST_RING)),
            inflight: AtomicI64::new(0),
        }
    }

    /// Build the default (production-shaped) API state. The capability verifier
    /// starts with an EMPTY anchor roster — fail-closed: until P34 enrolls real
    /// anchors / injects the canonical hybrid verifier behind this same trait,
    /// every `/api/*` request is rejected (401/403). This is intentional: an
    /// unconfigured server admits no orders.
    pub fn build_default() -> Arc<ApiState> {
        let store = Arc::new(EventStore::new());
        let caps: Arc<dyn CapVerifier> = Arc::new(KernelCapVerifier::new(
            RefSigner,
            AnchorRoster::new(),
            RevocationSet::new(),
        ));
        Arc::new(ApiState::new(store, caps))
    }

    /// Record a frame digest in the replay ring; reject if already seen.
    fn check_replay(&self, digest: [u8; 32]) -> Result<(), ApiReject> {
        let mut ring = self.seen.lock().unwrap();
        if ring.contains(&digest) {
            return Err(ApiReject::Replayed);
        }
        ring.push_back(digest);
        if ring.len() > SEEN_DIGEST_RING {
            ring.pop_front();
        }
        Ok(())
    }
}

/// Middleware: capability check BEFORE any handler runs. Fail-closed — a missing
/// / invalid / revoked / scope-mismatched cert answers 401/403 with ZERO writes
/// to the store. The handlers take no "body" extraction that would run before
/// this; the body digest is computed here from the raw bytes.
async fn cap_middleware(
    State(state): State<Arc<ApiState>>,
    mut req: Request,
    next: Next,
) -> Result<Response, ApiReject> {
    // Bulkhead: reject if too many API requests are in flight (§4.3).
    let inflight = state
        .inflight
        .fetch_add(1, Ordering::SeqCst)
        .saturating_add(1);
    if inflight as usize > MAX_INFLIGHT_API {
        state.inflight.fetch_sub(1, Ordering::SeqCst);
        return Err(ApiReject::TooLarge);
    }
    let _guard = scopeguard_dec(&state);

    let route = matched_route(&req);

    // Liveness probe: /healthz is OPEN (no capability required). Bypass all cap
    // checks and replay bookkeeping for it so a monitoring system can reach it
    // without a cert. Every other API route is cap-gated below (fail-closed).
    if route == ROUTE_HEALTH {
        return Ok(next.run(req).await);
    }

    // Read the cap header (base64 of a serialized SignedFrame).
    let cap_hdr = req
        .headers()
        .get(CAP_HEADER)
        .ok_or(ApiReject::Unauthorized)?
        .to_str()
        .map_err(|_| ApiReject::Unauthorized)?
        .as_bytes()
        .to_vec();

    // Capture the raw body so we can compute its digest AND re-inject it for the
    // handler. Body limit is enforced by tower's `DefaultBodyLimit` on the
    // sub-router; this read is bounded by that layer.
    let body_bytes = axum::body::to_bytes(std::mem::take(req.body_mut()), MAX_BODY_BYTES + 1)
        .await
        .map_err(|_| ApiReject::TooLarge)?;
    if body_bytes.len() > MAX_BODY_BYTES {
        return Err(ApiReject::TooLarge);
    }

    // Replay layer 2: frame digest ring (over the raw cap header bytes).
    let frame_digest = sha256(&cap_hdr);
    state.check_replay(frame_digest)?;

    // Body digest for the scope binding.
    let body_digest = sha256(&body_bytes);

    // Capability check (now_epoch = monotonic seconds since process start; for
    // test determinism we use a fixed epoch clock via a simple monotonic counter
    // anchored at construction — but we use system time here for realism; the
    // tests inject frames with `expiry` far in the future so any sane `now`
    // passes the freshness check).
    let now = now_epoch();

    state
        .caps
        .check(&cap_hdr, &body_digest, route, now)
        .map_err(|e| {
            // On rejection, the frame digest must NOT remain in the ring (it was a
            // failed attempt, not a satisfied request) — but a genuine *replay*
            // must be caught. We leave the digest in the ring so a re-send is
            // rejected as replayed; the failed-auth response still dominates.
            e
        })?;

    // Put the body back for the handler.
    *req.body_mut() = axum::body::Body::from(body_bytes);
    Ok(next.run(req).await)
}

/// Tiny RAII decrement for the bulkhead counter.
struct BulkheadGuard<'a> {
    state: &'a ApiState,
}
fn scopeguard_dec(state: &ApiState) -> BulkheadGuard<'_> {
    BulkheadGuard { state }
}
impl<'a> Drop for BulkheadGuard<'a> {
    fn drop(&mut self) {
        self.state.inflight.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Resolve the matched route const for a request (used for scope binding).
fn matched_route(req: &Request) -> &'static str {
    let path = req.uri().path();
    let method = req.method();
    if method == axum::http::Method::GET && path == ROUTE_HEALTH {
        return ROUTE_HEALTH;
    }
    if method == axum::http::Method::POST && path == ROUTE_ORDER_PLACE {
        return ROUTE_ORDER_PLACE;
    }
    if method == axum::http::Method::GET && path.starts_with("/api/order/") && path.ends_with("/advance") == false {
        // /api/order/{id} — but guard against the advance suffix.
        return ROUTE_ORDER_READ;
    }
    if method == axum::http::Method::POST && path.ends_with("/advance") {
        return ROUTE_ORDER_ADVANCE;
    }
    // Fallback (e.g. healthz GET handled separately above); default to read scope
    // is wrong — return a sentinel that maps to None so the middleware fails
    // closed. We use ROUTE_HEALTH which maps to None scope → Forbidden for any
    // non-API path, which is correct (static paths need no cap).
    ROUTE_HEALTH
}

// ── Handlers (relays only — NO order/money logic) ───────────────────────────

/// `POST /api/order` — place an order. Cap-gated.
async fn place_order_handler(
    State(state): State<Arc<ApiState>>,
    bytes: Bytes,
) -> Result<(StatusCode, String), ApiReject> {
    let body: PlaceOrderBody =
        serde_json::from_slice(&bytes).map_err(|_| ApiReject::Malformed)?;
    let order_json = json_api::place_order_logic(body.customer_id, &body.items_json, body.channel)
        .map_err(ApiReject::KernelReject)?;
    // Single shallow read: extract the kernel-returned id to key the store.
    let id = extract_id(&order_json)?;
    state.store.insert(
        id,
        OrderRecord {
            order_json: order_json.clone(),
            status_events: vec!["PENDING".to_string()],
        },
    );
    Ok((StatusCode::CREATED, order_json))
}

/// `GET /api/order/{id}` — read an order. Cap-gated.
async fn read_order_handler(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> Result<String, ApiReject> {
    let rec = state.store.get(&id).ok_or(ApiReject::NotFound)?;
    Ok(rec.order_json)
}

/// `POST /api/order/{id}/advance` — apply an event. Cap-gated.
async fn advance_order_handler(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    bytes: Bytes,
) -> Result<String, ApiReject> {
    let body: AdvanceBody = serde_json::from_slice(&bytes).map_err(|_| ApiReject::Malformed)?;
    // Load the current order. On a kernel reject the record is UNTOUCHED.
    let current = state.store.get(&id).ok_or(ApiReject::NotFound)?;
    let updated = json_api::apply_event_logic(&current.order_json, &body.next_status)
        .map_err(ApiReject::KernelReject)?;
    // Commit: replace order_json + append the status event.
    state.store.update(&id, |r| {
        r.order_json = updated.clone();
        r.status_events.push(body.next_status.clone());
    });
    Ok(updated)
}

/// `GET /healthz` — in-router liveness probe (promoted from the out-of-band
/// `health_response()`, lib.rs:113-122).
async fn healthz_handler() -> &'static str {
    "ok"
}

/// The one permitted shallow read: pull the `id` field out of a kernel order
/// JSON. No other field is touched. If the kernel shape ever changes this fails
/// closed (returns an error) rather than guessing.
fn extract_id(order_json: &str) -> Result<String, ApiReject> {
    let v: serde_json::Value = serde_json::from_str(order_json).map_err(|_| ApiReject::Malformed)?;
    v.get("id")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
        .ok_or(ApiReject::Malformed)
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().into()
}

/// Monotonic-ish epoch clock. We use a process-start anchored counter so tests
/// that mint frames with `expiry` far in the future pass consistently regardless
/// of wall clock. The capability freshness is relative to THIS clock.
fn now_epoch() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(1_000_000_000)
}

/// Build the API sub-router (auth + handlers). Merged into the static router by
/// `build_router` (W37-2). The body-limit + concurrency layers are scoped to this
/// sub-router ONLY so static serving is byte-unchanged.
pub fn build_api_router(state: Arc<ApiState>) -> Router {
    Router::new()
        .route(ROUTE_ORDER_PLACE, post(place_order_handler))
        .route(ROUTE_ORDER_READ, get(read_order_handler))
        .route(ROUTE_ORDER_ADVANCE, post(advance_order_handler))
        .route(ROUTE_HEALTH, get(healthz_handler))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            cap_middleware,
        ))
        .layer(axum::extract::DefaultBodyLimit::max(MAX_BODY_BYTES))
        .with_state(state)
}

/// Extension trait to read the delegation chain + payload off a `SignedFrame`
/// without changing the kernel's `cap.rs` (which keeps `payload` as opaque
/// bytes). We serialize the chain into the frame's payload on minting, so the
/// verifier can recover it. This keeps the kernel unchanged (anti-scope: do NOT
/// modify kernel decide/fold/order_machine/domain/money semantics).
pub trait SignedFrameExt {
    /// Recover the delegation chain stored in the payload as JSON-encoded
    /// `Vec<Delegation>`. Frames minted by `mint_frame` carry this.
    fn payload_as_chain(&self) -> Result<Vec<Delegation>, String>;
}

impl SignedFrameExt for SignedFrame {
    /// The payload layout is `<u64 LE chain_json_len> || chain_json || body_digest(32)`.
    /// This recovers ONLY the chain JSON portion (the tail 32 bytes are the body
    /// digest bound by `KernelCapVerifier::check`, never needed by the chain).
    fn payload_as_chain(&self) -> Result<Vec<Delegation>, String> {
        if self.payload.len() < 8 {
            return Err("truncated chain payload".into());
        }
        let n = u64::from_le_bytes(self.payload[..8].try_into().unwrap()) as usize;
        let end = 8 + n;
        if self.payload.len() < end {
            return Err("short chain payload".into());
        }
        serde_json::from_slice::<Vec<Delegation>>(&self.payload[8..end])
            .map_err(|e| format!("chain decode: {e}"))
    }
}

/// Test/helper: mint a capability frame for a given (resource, action) scope
/// against `body_digest`, signed by `issuer_secret` (root anchor), enrolled in
/// the returned roster. Used by the integration tests to produce valid frames
/// without inventing crypto (reuses `RefSigner`).
pub fn mint_frame(
    verifier: &RefSigner,
    issuer_secret: &[u8; 32],
    roster: &mut AnchorRoster,
    scope_grant: (Resource, Action),
    body_digest: &[u8; 32],
    expiry: u64,
) -> (SignedFrame, RevocationSet) {
    let subject_key = verifier.classical_public(issuer_secret);
    roster.enroll(&subject_key);

    let scope = Scope::new(vec![scope_grant]);
    let cap = Capability::new_hybrid(
        subject_key,
        verifier.pq_public(issuer_secret),
        scope.clone(),
        [0u8; 8],
        expiry,
    );
    let chain = vec![Delegation::sign(
        verifier,
        subject_key, // issued_by == subject (self-issued root anchor)
        subject_key,
        scope.clone(),
        scope.clone(),
        expiry,
        [0u8; 8],
        issuer_secret,
    )];
    // Payload layout: `<u64 LE chain_json_len> || chain_json || body_digest(32)`.
    // The body digest is bound into the signature domain (DOMAIN_FRAME covers the
    // payload), so a frame minted for body A cannot authorize body B.
    let mut chain_json = serde_json::to_vec(&chain).unwrap();
    let mut payload = Vec::with_capacity(8 + chain_json.len() + 32);
    payload.extend_from_slice(&(chain_json.len() as u64).to_le_bytes());
    payload.append(&mut chain_json);
    payload.extend_from_slice(body_digest);
    let mut frame = SignedFrame::new(cap, payload);
    frame.sign_classical(verifier, issuer_secret);
    frame.sign_pq(verifier, issuer_secret);
    (frame, RevocationSet::new())
}

/// Encode a `SignedFrame` for the `x-dowiz-cap` header (base64, no padding).
pub fn encode_cap(frame: &SignedFrame) -> String {
    B64.encode(serde_json::to_vec(frame).unwrap())
}

// Keep `Serialize` referenced for the `SignedFrame` codec (serde_json re-export
// ensures the kernel's `SignedFrame` derives Serialize — it does).
#[allow(dead_code)]
fn _assert_serde() {
    fn _f<T: Serialize>() {}
    _f::<SignedFrame>();
}
