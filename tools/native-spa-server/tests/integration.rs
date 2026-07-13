//! DK-04 RED tests — R1..R5.
//!
//! Spins up the real `native-spa-server` router on an ephemeral TCP port and
//! issues raw HTTP/1.1 requests over `std::net::TcpStream` (NO `reqwest` dep).
//! We assert:
//!   R1 — SPA fallback:  GET /some/spa/route        -> index.html
//!   R2 — Root:         GET /                       -> index.html
//!   R3 — Asset cache:  GET /assets/app.js          -> Cache-Control immutable
//!   R4 — Security:     every response carries CSP + X-Frame-Options + X-CTO
//!   R5 — Zero-OCI gate: scripts/check-zero-oci.sh exits 0 on new Dockerfile
//!                       and non-zero when an nginx base is present.

use axum::Router;
use native_spa_server::build_router;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;

/// Minimal HTTP/1.1 request + response parser. We deliberately avoid any HTTP
/// client crate — these are RED tests for a static server, not a benchmark.
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

fn http_get(addr: SocketAddr, path: &str) -> RawResponse {
    let mut stream = TcpStream::connect(addr).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let req = format!(
        "GET {path} HTTP/1.1\r\nHost: localhost\r\nAccept: */*\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(req.as_bytes()).expect("write request");

    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
            Err(e) => panic!("read error: {e}"),
        }
        if buf.windows(4).any(|w| w == b"\r\n\r\n") && buf.len() > 4 {
            // For Connection: close, wait for EOF; but break once we have
            // headers + (for HEAD-less GET) body if Content-Length known.
        }
    }
    parse_response(&buf)
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
    let mut found_end = false;
    for (i, line) in text.lines().enumerate() {
        if i == 0 {
            continue;
        }
        if line.is_empty() {
            found_end = true;
            // compute body offset
            if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                body_start = pos + 4;
            }
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            headers.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    let _ = found_end;
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
        let dir = std::env::temp_dir().join(format!(
            "nss-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("assets")).unwrap();
        std::fs::write(
            dir.join("index.html"),
            "<!doctype html><html><body>SPA SHELL</body></html>",
        )
        .unwrap();
        std::fs::write(dir.join("assets/app.js"), "console.log('app');").unwrap();
        dir
    })
}

/// Bind the router on an ephemeral port. Returns (addr, drop guard).
/// The server runs on its OWN tokio runtime (background OS thread) so the
/// calling test thread — which uses a blocking `std::net::TcpStream` client —
/// does not need a Tokio reactor.
fn spawn_server() -> (SocketAddr, std::thread::JoinHandle<()>) {
    let root = web_root().clone();
    // Channel to hand the bound addr back to the test thread.
    let (tx, rx) = std::sync::mpsc::channel::<SocketAddr>();

    let handle = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async move {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind ephemeral");
            let addr = listener.local_addr().unwrap();
            let _ = tx.send(addr);
            let router: Router = build_router(&root);
            let _ = axum::serve(listener, router).await;
        });
    });
    let addr = rx.recv_timeout(std::time::Duration::from_secs(5))
        .expect("server should bind and report addr");
    // Give the server a moment to start accepting.
    std::thread::sleep(Duration::from_millis(150));
    (addr, handle)
}

fn repo_root() -> PathBuf {
    // tests/ lives in the crate; repo root is two levels up.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .unwrap()
}

// --- R1: SPA fallback returns index.html for unknown client routes ----------
#[test]
fn r1_spa_fallback_returns_index() {
    let (addr, _guard) = spawn_server();
    let res = http_get(addr, "/some/spa/route");
    assert_eq!(res.status, 200, "SPA fallback must be 200");
    let body = String::from_utf8_lossy(&res.body);
    assert!(
        body.contains("SPA SHELL"),
        "SPA fallback body should be index.html, got: {body}"
    );
    assert!(
        body.contains("</html>"),
        "SPA fallback should serve full index.html"
    );
}

// --- R2: root path returns index.html ---------------------------------------
#[test]
fn r2_root_returns_index() {
    let (addr, _guard) = spawn_server();
    let res = http_get(addr, "/");
    assert_eq!(res.status, 200);
    let body = String::from_utf8_lossy(&res.body);
    assert!(body.contains("SPA SHELL"), "root should serve index.html");
}

// --- R3: /assets/* carries the immutable long-cache header ------------------
#[test]
fn r3_asset_cache_control_immutable() {
    let (addr, _guard) = spawn_server();
    let res = http_get(addr, "/assets/app.js");
    assert_eq!(res.status, 200, "asset must be served");
    let cc = header(&res, "Cache-Control").expect("Cache-Control present");
    assert_eq!(
        cc, "public, max-age=31536000, immutable",
        "asset Cache-Control must match nginx EXACTLY (got: {cc})"
    );
}

// --- R4: every response carries the EXACT security headers ------------------
#[test]
fn r4_security_headers_present_and_exact() {
    let (addr, _guard) = spawn_server();

    // Check on a static asset, the SPA fallback, and the root — all must match.
    for path in ["/", "/assets/app.js", "/deep/nested/route"] {
        let res = http_get(addr, path);
        assert_eq!(res.status, 200, "path {path} must be served");

        let csp = header(&res, "Content-Security-Policy")
            .expect("CSP header present on {path}");
        assert_eq!(
            csp,
            "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'; object-src 'none'; upgrade-insecure-requests",
            "CSP on {path} must match nginx EXACTLY"
        );

        let xfo = header(&res, "X-Frame-Options").expect("X-Frame-Options present");
        assert_eq!(xfo, "DENY", "X-Frame-Options on {path} must be DENY");

        let xcto = header(&res, "X-Content-Type-Options")
            .expect("X-Content-Type-Options present");
        assert_eq!(xcto, "nosniff", "X-Content-Type-Options on {path} must be nosniff");

        let rp = header(&res, "Referrer-Policy").expect("Referrer-Policy present");
        assert_eq!(rp, "strict-origin-when-cross-origin", "Referrer-Policy on {path}");

        let pp = header(&res, "Permissions-Policy").expect("Permissions-Policy present");
        assert_eq!(
            pp, "geolocation=(), microphone=(), camera=(), payment=()",
            "Permissions-Policy on {path}"
        );
    }
}

// --- R5: zero-OCI gate -------------------------------------------------------
// R5a: scripts/check-zero-oci.sh exits 0 against the current (rewritten)
//      Dockerfile. R5b: it exits non-zero when an nginx base is present.
#[test]
fn r5_zero_oci_gate_passes_on_new_dockerfile() {
    let script = repo_root().join("scripts/check-zero-oci.sh");
    assert!(script.exists(), "check-zero-oci.sh must exist at {:?}", script);

    let dockerfile = repo_root().join("Dockerfile");
    assert!(dockerfile.exists(), "Dockerfile must exist");

    let out = Command::new("bash")
        .arg(&script)
        .arg(&dockerfile)
        .output()
        .expect("run check-zero-oci.sh");
    assert!(
        out.status.success(),
        "R5a: check-zero-oci.sh must exit 0 on the new Dockerfile. stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn r5_zero_oci_gate_fails_on_nginx_base() {
    let script = repo_root().join("scripts/check-zero-oci.sh");
    let tmp = std::env::temp_dir().join(format!("nss-nginx-df-{}.txt", std::process::id()));
    std::fs::write(
        &tmp,
        "FROM cgr.dev/chainguard/nginx:latest\nCOPY . /usr/share/nginx/html\n",
    )
    .unwrap();
    let out = Command::new("bash")
        .arg(&script)
        .arg(&tmp)
        .output()
        .expect("run check-zero-oci.sh on nginx df");
    std::fs::remove_file(&tmp).ok();
    assert!(
        !out.status.success(),
        "R5b: check-zero-oci.sh must exit NON-zero when an nginx base is present"
    );
}
