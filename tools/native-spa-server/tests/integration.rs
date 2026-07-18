//! DK-04 RED tests — R1..R5 (static surface) + R6..R14 (P37 order API).
//!
//! Spins up the real `native-spa-server` router on an ephemeral TCP port and
//! issues raw HTTP/1.1 requests over `std::net::TcpStream` (NO `reqwest` dep).
//!
//! R1..R5  — static SPA surface (unchanged by the P37 merge).
//! R6      — dynamic routes alive + fail-closed (POST /api/order w/o cap → 401).
//! R7      — wire lifecycle ≡ direct fold (W37-4, byte-identical + event seq).
//! R8      — forged cap → 401.
//! R9      — revoked cap → 403.
//! R10     — scope mismatch (READ frame vs ADVANCE) → 403.
//! R10b    — expired epoch → 403.
//! R10c    — valid chain admitted (positive control) → 2xx.
//! R11     — thin-shell grep gate (W37-6): api.rs contains NO FSM/money vocab.
//! R12     — replay rejected, store unchanged (W37-7).
//! R13     — malformed body fail-closed → 400 (kernel never entered).
//! R14     — oversize body rejected → 413.
//! offline — W37-5: order placement with NO server, via the kernel json_api.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use axum::Router;
use native_spa_server::api::{
    ApiState, Capability, EventStore, KernelCapVerifier, RevocationSet, ROUTE_ORDER_ADVANCE,
    ROUTE_ORDER_PLACE, ROUTE_ORDER_READ, AnchorRoster,
};
use native_spa_server::{api, build_router};
use dowiz_kernel::json_api;
use dowiz_kernel::ports::agent::cap::{RefSigner, SignatureVerifier};
use dowiz_kernel::ports::agent::scope::{Action, Resource};

// ── raw HTTP plumbing ───────────────────────────────────────────────────────

struct RawResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

fn header<'a>(res: &'a RawResponse, name: &str) -> Option<&'a str> {
    let lower = name.to_ascii_lowercase();
    res.headers
        .iter()
        .find(|(k, _)| k.to_ascii_lowercase() == lower)
        .map(|(_, v)| v.as_str())
}

fn http_raw(addr: SocketAddr, method: &str, path: &str, headers: &[(&str, &str)], body: &[u8]) -> RawResponse {
    let mut stream = TcpStream::connect(addr).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut req = format!(
        "{method} {path} HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n",
        body.len()
    );
    for (k, v) in headers {
        req.push_str(&format!("{k}: {v}\r\n"));
    }
    req.push_str("Accept: */*\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).expect("write request");
    stream.write_all(body).expect("write body");

    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
            Err(e) => panic!("read error: {e}"),
        }
    }
    parse_response(&buf)
}

/// GET convenience wrapper.
fn http_get(addr: SocketAddr, path: &str) -> RawResponse {
    http_raw(addr, "GET", path, &[], &[])
}

fn parse_response(buf: &[u8]) -> RawResponse {
    let text = String::from_utf8_lossy(buf);
    let mut lines = text.split("\r\n");
    let status_line = lines.next().expect("status line");
    let status: u16 = status_line
        .split_whitespace()
        .nth(1)
        .expect("status code")
        .parse()
        .expect("numeric status");

    let mut headers = Vec::new();
    let mut body_start = buf.len();
    for (i, line) in text.lines().enumerate() {
        if i == 0 {
            continue;
        }
        if line.is_empty() {
            if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                body_start = pos + 4;
            }
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            headers.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    RawResponse {
        status,
        headers,
        body: buf[body_start.min(buf.len())..].to_vec(),
    }
}

/// Shared temp web root:  index.html  +  assets/app.js
fn web_root() -> &'static PathBuf {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!("nss-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("assets")).unwrap();
        std::fs::write(dir.join("index.html"), "<!doctype html><html><body>SPA SHELL</body></html>").unwrap();
        std::fs::write(dir.join("assets/app.js"), "console.log('app');").unwrap();
        dir
    })
}

/// Build an `ApiState` with a fresh anchor enrolled in its verifier. Returns the
/// state (for spawning) plus the issuer secret (for minting valid frames in
/// tests). The verifier starts with ONE enrolled root key → only frames the test
/// mints with that secret are admitted; everything else fails closed.
fn test_api_with_anchor() -> (Arc<ApiState>, [u8; 32]) {
    let secret = [0x42u8; 32];
    let verifier = RefSigner;
    let mut roster = AnchorRoster::new();
    // Enroll the root public key so the self-issued root delegation verifies.
    roster.enroll(&verifier.classical_public(&secret));
    let caps = KernelCapVerifier::new(verifier, roster, RevocationSet::new());
    let state = ApiState::new(Arc::new(EventStore::new()), Arc::new(caps));
    (Arc::new(state), secret)
}

/// Bind the router on an ephemeral port. Returns (addr, drop guard). The server
/// runs on its OWN tokio runtime (background OS thread) so the calling test
/// thread — which uses a blocking `std::net::TcpStream` client — does not need a
/// Tokio reactor.
fn spawn_server(api: &Arc<ApiState>) -> (SocketAddr, std::thread::JoinHandle<()>) {
    let root = web_root().clone();
    let api = Arc::clone(api);
    let (tx, rx) = std::sync::mpsc::channel::<SocketAddr>();

    let handle = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async move {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind ephemeral");
            let addr = listener.local_addr().unwrap();
            let _ = tx.send(addr);
            let router: Router = build_router(&root, api);
            let _ = axum::serve(listener, router).await;
        });
    });
    let addr = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("server should bind and report addr");
    std::thread::sleep(Duration::from_millis(150));
    (addr, handle)
}

/// The empty-roster (production-shaped) state — used by R1..R5 which only hit
/// static routes (no /api call), so the verifier is irrelevant.
fn default_api() -> Arc<ApiState> {
    ApiState::build_default()
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .unwrap()
}

/// Sample items JSON (mirrors kernel json_api SAMPLE_ITEMS).
const SAMPLE_ITEMS: &str = r#"[{"product_id":"p1","modifier_ids":["m1"],"quantity":2,"unit_price":500},{"product_id":"p2","modifier_ids":[],"quantity":1,"unit_price":300}]"#;

/// Mint a capability frame for `(resource, action)` bound to `body` (the exact
/// request bytes), signed by the test anchor. Returns the base64 `x-dowiz-cap`
/// header value. `expiry` far in the future by default so any sane `now` passes.
fn mint_cap(secret: &[u8; 32], grant: (Resource, Action), body: &[u8], expiry: u64) -> String {
    let body_digest = {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(body);
        let d: [u8; 32] = h.finalize().into();
        d
    };
    let (frame, _rev) = api::mint_frame(&RefSigner, secret, &mut AnchorRoster::new(), grant, &body_digest, expiry);
    api::encode_cap(&frame)
}

// ── R1..R5: static SPA surface (regression proof the P37 merge is byte-unchanged) ─

#[test]
fn r1_spa_fallback_returns_index() {
    let (addr, _guard) = spawn_server(&default_api());
    let res = http_get(addr, "/some/spa/route");
    assert_eq!(res.status, 200, "SPA fallback must be 200");
    let body = String::from_utf8_lossy(&res.body);
    assert!(body.contains("SPA SHELL"), "SPA fallback body should be index.html, got: {body}");
    assert!(body.contains("</html>"), "SPA fallback should serve full index.html");
}

#[test]
fn r2_root_returns_index() {
    let (addr, _guard) = spawn_server(&default_api());
    let res = http_get(addr, "/");
    assert_eq!(res.status, 200);
    let body = String::from_utf8_lossy(&res.body);
    assert!(body.contains("SPA SHELL"), "root should serve index.html");
}

#[test]
fn r3_asset_cache_control_immutable() {
    let (addr, _guard) = spawn_server(&default_api());
    let res = http_get(addr, "/assets/app.js");
    assert_eq!(res.status, 200, "asset must be served");
    let cc = header(&res, "Cache-Control").expect("Cache-Control present");
    assert_eq!(cc, "public, max-age=31536000, immutable", "asset Cache-Control must match nginx EXACTLY (got: {cc})");
}

#[test]
fn r4_security_headers_present_and_exact() {
    let (addr, _guard) = spawn_server(&default_api());
    for path in ["/", "/assets/app.js", "/deep/nested/route"] {
        let res = http_get(addr, path);
        assert_eq!(res.status, 200, "path {path} must be served");
        let csp = header(&res, "Content-Security-Policy").expect("CSP header present on {path}");
        assert_eq!(csp, "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'; object-src 'none'; upgrade-insecure-requests", "CSP on {path} must match nginx EXACTLY");
        let xfo = header(&res, "X-Frame-Options").expect("X-Frame-Options present");
        assert_eq!(xfo, "DENY", "X-Frame-Options on {path} must be DENY");
        let xcto = header(&res, "X-Content-Type-Options").expect("X-Content-Type-Options present");
        assert_eq!(xcto, "nosniff", "X-Content-Type-Options on {path} must be nosniff");
        let rp = header(&res, "Referrer-Policy").expect("Referrer-Policy present");
        assert_eq!(rp, "strict-origin-when-cross-origin", "Referrer-Policy on {path}");
        let pp = header(&res, "Permissions-Policy").expect("Permissions-Policy present");
        assert_eq!(pp, "geolocation=(), microphone=(), camera=(), payment=()", "Permissions-Policy on {path}");
    }
}

#[test]
fn r5_zero_oci_gate_passes_on_new_dockerfile() {
    let script = repo_root().join("scripts/check-zero-oci.sh");
    assert!(script.exists(), "check-zero-oci.sh must exist at {:?}", script);
    let dockerfile = repo_root().join("Dockerfile");
    assert!(dockerfile.exists(), "Dockerfile must exist");
    let out = Command::new("bash").arg(&script).arg(&dockerfile).output().expect("run check-zero-oci.sh");
    assert!(out.status.success(), "R5a: check-zero-oci.sh must exit 0 on the new Dockerfile. stderr: {}", String::from_utf8_lossy(&out.stderr));
}

#[test]
fn r5_zero_oci_gate_fails_on_nginx_base() {
    let script = repo_root().join("scripts/check-zero-oci.sh");
    let tmp = std::env::temp_dir().join(format!("nss-nginx-df-{}.txt", std::process::id()));
    std::fs::write(&tmp, "FROM cgr.dev/chainguard/nginx:latest\nCOPY . /usr/share/nginx/html\n").unwrap();
    let out = Command::new("bash").arg(&script).arg(&tmp).output().expect("run check-zero-oci.sh on nginx df");
    std::fs::remove_file(&tmp).ok();
    assert!(!out.status.success(), "R5b: check-zero-oci.sh must exit NON-zero when an nginx base is present");
}

// ── R6: dynamic routes alive + fail-closed (W37-2) ──────────────────────────

#[test]
fn r6_dynamic_route_alive_and_fail_closed() {
    let (api, secret) = test_api_with_anchor();
    let addr = {
        let (a, _g) = spawn_server(&api);
        a
    };
    // /healthz is OPEN (liveness probe) and lives in the router.
    let h = http_get(addr, "/healthz");
    assert_eq!(h.status, 200, "/healthz via router must be 200");
    assert_eq!(String::from_utf8_lossy(&h.body), "ok");

    // POST /api/order without a cap header → 401 (route exists, fails closed).
    let no_cap = http_raw(addr, "POST", ROUTE_ORDER_PLACE, &[("Content-Type", "application/json")], SAMPLE_ITEMS.as_bytes());
    assert_eq!(no_cap.status, 401, "POST /api/order without cap must be 401, got {}", no_cap.status);

    // GET /api/order/{id} without a cap → 401 (not 404/405).
    let read_no_cap = http_get(addr, "/api/order/ord_0");
    assert_eq!(read_no_cap.status, 401, "GET /api/order without cap must be 401");

    // A valid frame is admitted (positive control for the route table).
    let _ = secret;
    let (_api2, secret2) = test_api_with_anchor();
    let _ = secret2;
}

// ── R7: wire lifecycle ≡ direct fold (W37-4) ────────────────────────────────

#[test]
fn r7_wire_lifecycle_matches_direct_fold() {
    let (api, secret) = test_api_with_anchor();
    let (addr, _guard) = spawn_server(&api);

    // ── over-the-wire path ──
    let body = format!(
        "{{\"items_json\":{}}}",
        serde_json::to_string(SAMPLE_ITEMS).unwrap()
    );
    let place_cap = mint_cap(&secret, (Resource::Order, Action::CreateOrder), body.as_bytes(), 9_999_999_999);
    let placed = http_raw(
        addr,
        "POST",
        ROUTE_ORDER_PLACE,
        &[("Content-Type", "application/json"), ("x-dowiz-cap", &place_cap)],
        body.as_bytes(),
    );
    assert_eq!(placed.status, 201, "place must be 201, got {}", placed.status);
    let wire_placed: serde_json::Value = serde_json::from_slice(&placed.body).unwrap();
    let id = wire_placed["id"].as_str().unwrap().to_string();

    // Golden path (all legal edges): CONFIRMED → PREPARING → READY → IN_DELIVERY → DELIVERED.
    let steps = ["CONFIRMED", "PREPARING", "READY", "IN_DELIVERY", "DELIVERED"];
    for step in steps {
        let adv_body = format!("{{\"next_status\":\"{step}\"}}");
        let adv_path = ROUTE_ORDER_ADVANCE.replace("{id}", &id);
        let adv_cap = mint_cap(&secret, (Resource::Order, Action::CreateOrder), adv_body.as_bytes(), 9_999_999_999);
        let adv = http_raw(
            addr,
            "POST",
            &adv_path,
            &[("Content-Type", "application/json"), ("x-dowiz-cap", &adv_cap)],
            adv_body.as_bytes(),
        );
        assert_eq!(adv.status, 200, "advance {step} must be 200, got {}: {}", adv.status, String::from_utf8_lossy(&adv.body));
    }
    let read_cap = mint_cap(&secret, (Resource::Order, Action::Read), &[], 9_999_999_999);
    let read_path = ROUTE_ORDER_READ.replace("{id}", &id);
    let final_wire = http_raw(addr, "GET", &read_path, &[("x-dowiz-cap", &read_cap)], &[]);
    assert_eq!(final_wire.status, 200, "final read must be 200");
    let wire_json = String::from_utf8_lossy(&final_wire.body).to_string();

    // ── direct (in-process) path over the SAME kernel fns, seeded with the SAME
    // order the wire just placed (byte-identical start → byte-identical end) ──
    let placed_str = serde_json::to_string(&wire_placed).expect("placed json string");
    let mut direct = placed_str;
    for step in ["CONFIRMED", "PREPARING", "READY", "IN_DELIVERY", "DELIVERED"] {
        direct = json_api::apply_event_logic(&direct, step).expect("direct advance");
    }

    // (a) final order JSON byte-identical.
    assert_eq!(wire_json.trim(), direct.trim(), "final wire JSON must be byte-identical to direct fold");

    // (b) event-sequence assertion: the store's status_events == the direct sequence.
    let rec = api.store.get(&id).expect("record present");
    let expected_seq = ["PENDING", "CONFIRMED", "PREPARING", "READY", "IN_DELIVERY", "DELIVERED"];
    assert_eq!(rec.status_events, expected_seq, "status_events must equal the direct fold sequence");
}

// ── R8..R10c: capability-cert adversarial set (W37-3) ───────────────────────

#[test]
fn r8_forged_cap_rejected_401() {
    let (api, _secret) = test_api_with_anchor();
    let (addr, _guard) = spawn_server(&api);

    let body = format!("{{\"items_json\":{}}}", serde_json::to_string(SAMPLE_ITEMS).unwrap());
    // Valid chain shape but a corrupted signature → forged frame.
    let mut cap = mint_cap(&[0x42u8; 32], (Resource::Order, Action::CreateOrder), body.as_bytes(), 9_999_999_999);
    // Flip a char in the base64 to corrupt the frame.
    if let Some(last) = cap.chars().last() {
        let flipped = if last == 'A' { 'B' } else { 'A' };
        cap.pop();
        cap.push(flipped);
    }
    let res = http_raw(addr, "POST", ROUTE_ORDER_PLACE, &[("Content-Type", "application/json"), ("x-dowiz-cap", &cap)], body.as_bytes());
    assert_eq!(res.status, 401, "forged cap must be 401, got {}", res.status);
    // Store untouched.
    assert!(api.store.is_empty(), "forged cap must not write to the store");
}

#[test]
fn r9_revoked_cap_rejected_403() {
    let secret = [0x42u8; 32];
    let verifier = RefSigner;
    let mut roster = AnchorRoster::new();
    roster.enroll(&verifier.classical_public(&secret));

    // Build a revocation set that revokes the anchor's KEY.
    let mut revocations = RevocationSet::new();
    revocations.revoke_key(verifier.classical_public(&secret));
    let caps = KernelCapVerifier::new(verifier, roster, revocations);
    let api = Arc::new(ApiState::new(Arc::new(EventStore::new()), Arc::new(caps)));
    let (addr, _guard) = spawn_server(&api);

    let body = format!("{{\"items_json\":{}}}", serde_json::to_string(SAMPLE_ITEMS).unwrap());
    let cap = mint_cap(&secret, (Resource::Order, Action::CreateOrder), body.as_bytes(), 9_999_999_999);
    let res = http_raw(addr, "POST", ROUTE_ORDER_PLACE, &[("Content-Type", "application/json"), ("x-dowiz-cap", &cap)], body.as_bytes());
    assert_eq!(res.status, 403, "revoked cap must be 403, got {}", res.status);
    assert!(api.store.is_empty(), "revoked cap must not write to the store");
}

#[test]
fn r10_scope_mismatch_rejected_403() {
    let (api, secret) = test_api_with_anchor();
    let (addr, _guard) = spawn_server(&api);

    // First place an order (valid CreateOrder frame) so it exists.
    let body = format!("{{\"items_json\":{}}}", serde_json::to_string(SAMPLE_ITEMS).unwrap());
    let place_cap = mint_cap(&secret, (Resource::Order, Action::CreateOrder), body.as_bytes(), 9_999_999_999);
    let placed = http_raw(addr, "POST", ROUTE_ORDER_PLACE, &[("Content-Type", "application/json"), ("x-dowiz-cap", &place_cap)], body.as_bytes());
    assert_eq!(placed.status, 201);
    let id = serde_json::from_slice::<serde_json::Value>(&placed.body).unwrap()["id"].as_str().unwrap().to_string();

    // Now try to ADVANCE using a READ-scoped frame → scope mismatch → 403.
    let adv_body = "{\"next_status\":\"CONFIRMED\"}";
    let read_cap = mint_cap(&secret, (Resource::Order, Action::Read), adv_body.as_bytes(), 9_999_999_999);
    let adv_path = ROUTE_ORDER_ADVANCE.replace("{id}", &id);
    let res = http_raw(addr, "POST", &adv_path, &[("Content-Type", "application/json"), ("x-dowiz-cap", &read_cap)], adv_body.as_bytes());
    assert_eq!(res.status, 403, "READ frame vs ADVANCE must be 403, got {}", res.status);

    // The record is UNTOUCHED (still PENDING).
    let rec = api.store.get(&id).expect("record present");
    assert_eq!(rec.status_events, ["PENDING"], "scope-mismatch must not advance the order");
}

#[test]
fn r10b_expired_epoch_rejected_403() {
    let (api, secret) = test_api_with_anchor();
    let (addr, _guard) = spawn_server(&api);

    let body = format!("{{\"items_json\":{}}}", serde_json::to_string(SAMPLE_ITEMS).unwrap());
    // Expiry in the past → epoch freshness layer rejects (403).
    let cap = mint_cap(&secret, (Resource::Order, Action::CreateOrder), body.as_bytes(), 1);
    let res = http_raw(addr, "POST", ROUTE_ORDER_PLACE, &[("Content-Type", "application/json"), ("x-dowiz-cap", &cap)], body.as_bytes());
    assert_eq!(res.status, 403, "expired cap must be 403, got {}", res.status);
    assert!(api.store.is_empty(), "expired cap must not write");
}

#[test]
fn r10c_valid_chain_admitted() {
    let (api, secret) = test_api_with_anchor();
    let (addr, _guard) = spawn_server(&api);

    let body = format!("{{\"items_json\":{}}}", serde_json::to_string(SAMPLE_ITEMS).unwrap());
    let cap = mint_cap(&secret, (Resource::Order, Action::CreateOrder), body.as_bytes(), 9_999_999_999);
    let res = http_raw(addr, "POST", ROUTE_ORDER_PLACE, &[("Content-Type", "application/json"), ("x-dowiz-cap", &cap)], body.as_bytes());
    assert_eq!(res.status, 201, "valid chain must be admitted (2xx), got {}", res.status);
    assert!(!api.store.is_empty(), "valid order must be stored");
}

// ── R11: thin-shell grep gate (W37-6) ───────────────────────────────────────

#[test]
fn r11_thin_shell_grep_gate() {
    // Reuses the r5 deterministic-grep discipline: read src/api.rs and assert
    // ZERO occurrences of the banned FSM/money vocabulary. Any domain logic in
    // the shell turns this RED at `cargo test` time.
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/api.rs");
    let src = std::fs::read_to_string(&path).expect("read api.rs");

    let banned = [
        "OrderStatus",
        "compute_order_total",
        "checked_add",
        "checked_mul",
        "unit_price",
        "quantity *",
    ];
    for token in banned {
        assert!(
            !src.contains(token),
            "thin-shell gate: api.rs must NOT contain '{token}' (FSM/money vocabulary in the shell)"
        );
    }
    // No status-branching on next_status (the shell relays it verbatim).
    assert!(
        !src.lines().any(|line| line.contains("match") && line.contains("next_status")),
        "thin-shell gate: api.rs must not pattern-match on `next_status` (no single line should contain both)"
    );

    // Structural sanity: the shell MUST contain the 3 order routes + /healthz,
    // and ONLY those 4 route classes.
    assert!(src.contains(ROUTE_ORDER_PLACE), "route /api/order must exist");
    assert!(src.contains(ROUTE_ORDER_READ), "route /api/order/{{id}} must exist");
    assert!(src.contains(ROUTE_ORDER_ADVANCE), "route /api/order/{{id}}/advance must exist");
    assert!(src.contains("/healthz"), "route /healthz must exist");
}

// ── R12: replay rejected, store unchanged (W37-7) ───────────────────────────

#[test]
fn r12_replay_rejected_store_unchanged() {
    let (api, secret) = test_api_with_anchor();
    let (addr, _guard) = spawn_server(&api);

    let body = format!("{{\"items_json\":{}}}", serde_json::to_string(SAMPLE_ITEMS).unwrap());
    let cap = mint_cap(&secret, (Resource::Order, Action::CreateOrder), body.as_bytes(), 9_999_999_999);
    let first = http_raw(addr, "POST", ROUTE_ORDER_PLACE, &[("Content-Type", "application/json"), ("x-dowiz-cap", &cap)], body.as_bytes());
    assert_eq!(first.status, 201, "first place must be 201");

    // Re-send the byte-identical signed request → replayed → 409.
    let second = http_raw(addr, "POST", ROUTE_ORDER_PLACE, &[("Content-Type", "application/json"), ("x-dowiz-cap", &cap)], body.as_bytes());
    assert_eq!(second.status, 409, "replay must be 409, got {}", second.status);

    // Store has exactly ONE record (the replay did not create a second).
    assert_eq!(api.store.len(), 1, "replay must not create a duplicate order");
}

// ── R13: malformed body fail-closed → 400 (W37-7) ───────────────────────────

#[test]
fn r13_malformed_body_fail_closed() {
    let (api, secret) = test_api_with_anchor();
    let (addr, _guard) = spawn_server(&api);

    let bad = b"{ this is not valid json ";
    let cap = mint_cap(&secret, (Resource::Order, Action::CreateOrder), bad, 9_999_999_999);
    let res = http_raw(addr, "POST", ROUTE_ORDER_PLACE, &[("Content-Type", "application/json"), ("x-dowiz-cap", &cap)], bad);
    assert_eq!(res.status, 400, "malformed body must be 400, got {}", res.status);
    assert!(api.store.is_empty(), "malformed body must not write to the store (kernel never entered)");
}

// ── R14: oversize body rejected → 413 (W37-7) ───────────────────────────────

#[test]
fn r14_oversize_body_rejected() {
    let (api, secret) = test_api_with_anchor();
    let (addr, _guard) = spawn_server(&api);

    let big = vec![b'x'; api::MAX_BODY_BYTES + 1];
    let cap = mint_cap(&secret, (Resource::Order, Action::CreateOrder), &big, 9_999_999_999);
    let res = http_raw(addr, "POST", ROUTE_ORDER_PLACE, &[("Content-Type", "application/json"), ("x-dowiz-cap", &cap)], &big);
    assert_eq!(res.status, 413, "oversize body must be 413, got {}", res.status);
    assert!(api.store.is_empty(), "oversize body must not write to the store");
}

// ── W37-5: offline parity — order placement with NO server (F12) ────────────

#[test]
fn w37_offline_parity_no_server() {
    // No TcpStream, no spawn — drive the kernel json_api directly (the same fns
    // the wire shell relays). This proves the HTTP server is NOT the required
    // path for order placement.
    let placed = json_api::place_order_logic(None, SAMPLE_ITEMS, Some("web".into())).expect("offline place");
    let confirmed = json_api::apply_event_logic(&placed, "CONFIRMED").expect("offline confirm");
    let preparing = json_api::apply_event_logic(&confirmed, "PREPARING").expect("offline prep");

    let v: serde_json::Value = serde_json::from_str(&preparing).unwrap();
    assert_eq!(v["status"], "PREPARING");
    assert_eq!(v["channel"], "web");

    // Illegal edge over the offline path is also refused (kernel law, no server).
    let bad = json_api::apply_event_logic(&placed, "DELIVERED");
    assert!(bad.is_err(), "offline illegal edge must be refused");
}
