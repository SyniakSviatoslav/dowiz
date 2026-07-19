//! P40 Task D — `/api/agent` proxy integration (red→green).
//!
//! RED `red_agent_route_absent` / `red_agent_turn_bounded`: before P40 there was
//! NO `/api/agent` route → POST 404'd and no turn ran. GREEN:
//!   * a cap-framed POST /api/agent → 200 + typed `AgentResponse` (outcome/text/log),
//!   * an UNSIGNED POST → 401 (reusing the SAME `verify_chain` gate as the order API),
//!   * the turn is BOUNDED: one POST drives exactly one upstream call within a
//!     wall-time budget (the scripted sibling counts calls; no hang).
//!
//! The proxy target is a SCRIPTED localhost double (a tiny `TcpListener` thread
//! that returns a canned `LoopOutcome` JSON) — the real sibling service is heavy
//! and its live path needs an Ollama daemon (OPS precondition, not a code defect).
//! This isolates the WIRING (cap gate + forward + typed relay) under test.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use axum::Router;
use dowiz_kernel::ports::agent::cap::{RefSigner, SignatureVerifier};
use dowiz_kernel::ports::agent::scope::{Action, Resource};
use native_spa_server::api::{
    self, AnchorRoster, ApiState, EventStore, KernelCapVerifier, RevocationSet, ROUTE_AGENT,
};
use native_spa_server::build_router;
use std::path::PathBuf;

// ── raw HTTP client (std-only; mirrors integration.rs) ──────────────────────

struct RawResponse {
    status: u16,
    body: Vec<u8>,
}

fn http_raw(
    addr: SocketAddr,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> RawResponse {
    let mut stream = TcpStream::connect(addr).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
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
    let text = String::from_utf8_lossy(&buf);
    let status: u16 = text
        .split_whitespace()
        .nth(1)
        .expect("status code")
        .parse()
        .expect("numeric status");
    let body_start = buf
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|p| p + 4)
        .unwrap_or(buf.len());
    RawResponse {
        status,
        body: buf[body_start.min(buf.len())..].to_vec(),
    }
}

fn web_root() -> &'static PathBuf {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!("nss-agent-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("assets")).unwrap();
        std::fs::write(dir.join("index.html"), "<!doctype html><html></html>").unwrap();
        std::fs::write(dir.join("assets/app.js"), "console.log('app');").unwrap();
        dir
    })
}

/// A SCRIPTED sibling agent-loop double: binds an ephemeral localhost port and,
/// per connection, returns a canned typed `LoopOutcome` JSON. Counts calls so the
/// test proves EXACTLY ONE bounded turn per POST. Returns (host:port, counter).
fn spawn_scripted_agent() -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind scripted agent");
    let addr = listener.local_addr().unwrap();
    let counter = Arc::new(AtomicUsize::new(0));
    let c = Arc::clone(&counter);
    std::thread::spawn(move || {
        for conn in listener.incoming() {
            let mut stream = match conn {
                Ok(s) => s,
                Err(_) => continue,
            };
            c.fetch_add(1, Ordering::SeqCst);
            // Drain the full request (headers + body) before replying, so the
            // proxy's two-part write (head then body) completes before we close.
            let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
            let mut req = Vec::new();
            let mut chunk = [0u8; 1024];
            loop {
                match stream.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => {
                        req.extend_from_slice(&chunk[..n]);
                        // Stop once we have headers + a plausible body tail.
                        if let Some(p) = req.windows(4).position(|w| w == b"\r\n\r\n") {
                            let cl = String::from_utf8_lossy(&req[..p])
                                .lines()
                                .find_map(|l| {
                                    let (k, v) = l.split_once(':')?;
                                    if k.trim().eq_ignore_ascii_case("content-length") {
                                        v.trim().parse::<usize>().ok()
                                    } else {
                                        None
                                    }
                                })
                                .unwrap_or(0);
                            if req.len() >= p + 4 + cl {
                                break;
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
            let json = r#"{"outcome":"answer","text":"ORD-42 is IN_DELIVERY","log":["iter0: model_reply"]}"#;
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                json.len(),
                json
            );
            let _ = stream.write_all(resp.as_bytes());
            let _ = stream.flush();
        }
    });
    (format!("127.0.0.1:{}", addr.port()), counter)
}

fn test_api_with_anchor(upstream: Option<String>) -> (Arc<ApiState>, [u8; 32]) {
    let secret = [0x42u8; 32];
    let verifier = RefSigner;
    let mut roster = AnchorRoster::new();
    roster.enroll(&verifier.classical_public(&secret));
    let caps = KernelCapVerifier::new(verifier, roster, RevocationSet::new());
    let state = ApiState::new(Arc::new(EventStore::new()), Arc::new(caps))
        .with_agent_upstream(upstream);
    (Arc::new(state), secret)
}

fn spawn_server(api: &Arc<ApiState>) -> SocketAddr {
    let root = web_root().clone();
    let api = Arc::clone(api);
    let (tx, rx) = std::sync::mpsc::channel::<SocketAddr>();
    std::thread::spawn(move || {
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
    let addr = rx.recv_timeout(Duration::from_secs(5)).expect("bind report");
    std::thread::sleep(Duration::from_millis(150));
    addr
}

fn mint_cap(secret: &[u8; 32], grant: (Resource, Action), body: &[u8], expiry: u64) -> String {
    let body_digest = {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(body);
        let d: [u8; 32] = h.finalize().into();
        d
    };
    let (frame, _rev) = api::mint_frame(
        &RefSigner,
        secret,
        &mut AnchorRoster::new(),
        grant,
        &body_digest,
        expiry,
    );
    api::encode_cap(&frame)
}

const AGENT_BODY: &str = r#"{"prompt":"what is the status of ORD-42?"}"#;
const FAR_EPOCH: u64 = 4_000_000_000;

/// GREEN: valid cap-framed POST → 200 + typed AgentResponse, driven through the
/// SAME verify_chain gate the order API uses; exactly one bounded upstream turn.
#[test]
fn red_agent_turn_bounded() {
    let (upstream, calls) = spawn_scripted_agent();
    let (api, secret) = test_api_with_anchor(Some(upstream));
    let addr = spawn_server(&api);

    let cap = mint_cap(
        &secret,
        (Resource::Order, Action::Read),
        AGENT_BODY.as_bytes(),
        FAR_EPOCH,
    );

    let start = Instant::now();
    let res = http_raw(
        addr,
        "POST",
        ROUTE_AGENT,
        &[("x-dowiz-cap", &cap), ("content-type", "application/json")],
        AGENT_BODY.as_bytes(),
    );
    let elapsed = start.elapsed();

    assert_eq!(res.status, 200, "valid cap-framed agent POST must be 200; body={}", String::from_utf8_lossy(&res.body));
    assert!(
        elapsed < Duration::from_secs(10),
        "one turn must be bounded, took {elapsed:?}"
    );

    let body = String::from_utf8_lossy(&res.body);
    assert!(
        body.contains("\"outcome\":\"answer\""),
        "typed AgentResponse.outcome=answer expected, got: {body}"
    );
    assert!(
        body.contains("IN_DELIVERY"),
        "relayed text expected, got: {body}"
    );

    // EXACTLY one bounded upstream turn per POST.
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "one POST drives exactly one upstream turn"
    );
}

/// 401: an UNSIGNED request is rejected by the reused verify_chain gate BEFORE
/// the proxy forwards anything — the sibling is never called.
#[test]
fn red_agent_route_absent_unsigned_401() {
    let (upstream, calls) = spawn_scripted_agent();
    let (api, _secret) = test_api_with_anchor(Some(upstream));
    let addr = spawn_server(&api);

    let res = http_raw(
        addr,
        "POST",
        ROUTE_AGENT,
        &[("content-type", "application/json")], // no x-dowiz-cap
        AGENT_BODY.as_bytes(),
    );
    assert_eq!(res.status, 401, "unsigned agent POST must be 401");
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "cap gate must reject BEFORE any upstream forward"
    );
}

/// 401: a FORGED cap (wrong signing secret) is rejected by verify_chain.
#[test]
fn red_agent_forged_cap_401() {
    let (upstream, _calls) = spawn_scripted_agent();
    let (api, _secret) = test_api_with_anchor(Some(upstream));
    let addr = spawn_server(&api);

    let forged = mint_cap(
        &[0x99u8; 32], // NOT the enrolled anchor secret
        (Resource::Order, Action::Read),
        AGENT_BODY.as_bytes(),
        FAR_EPOCH,
    );
    let res = http_raw(
        addr,
        "POST",
        ROUTE_AGENT,
        &[("x-dowiz-cap", &forged), ("content-type", "application/json")],
        AGENT_BODY.as_bytes(),
    );
    assert!(
        res.status == 401 || res.status == 403,
        "forged cap must be rejected (401/403), got {}",
        res.status
    );
}
