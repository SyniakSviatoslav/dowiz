# BLUEPRINT — `native-spa-server` listener-wide DoS hardening (2026-07-20)

- **Date:** 2026-07-20 · **Component:** DELIVERY (P37, order HTTP surface) · **Status:**
  BLUEPRINT v1 (planning artifact, no code). Follow-up to commit `00436dedc` ("real header-read
  timeout, closes slowloris gap"), which fixed the **per-connection** stall but left the
  **listener-wide** capacity gap open — flagged in that commit's own report as a known, deliberately
  not-yet-fixed item, per the space-grade quality bar (memory: never downscope hardening because
  "Cloudflare already covers it in prod" — that is an operational mitigation, not a code-level one).
- **Sources verified this session, live:** `tools/native-spa-server/src/lib.rs` (`serve_with_timeout`,
  lines 135-165 — the accept loop); `tools/native-spa-server/src/api.rs` (`MAX_INFLIGHT_API = 64`
  at line 62, the `inflight` bulkhead in `cap_middleware` at lines 390-401, `MAX_BODY_BYTES` +
  `DefaultBodyLimit` at line 678); `tools/native-spa-server/Cargo.toml` (confirms `tokio` "full"
  features already present — `tokio::sync::Semaphore` needs no new dependency); commit `00436dedc`'s
  full message (the exact prior fix + its own named residual gaps); `kernel::token_bucket` (the
  existing GCRA/token-bucket rate-limit primitive, already used by `llm-adapters`' `Dispatcher`).

---

## 1. Problem statement — the real, still-open gap

`serve_with_timeout` (`tools/native-spa-server/src/lib.rs:135-165`) is a manual accept loop:

```rust
pub async fn serve_with_timeout(listener, router, header_read_timeout) -> std::io::Result<()> {
    loop {
        let (stream, _peer) = listener.accept().await?;
        let router = router.clone();
        tokio::spawn(async move { /* per-connection hyper builder w/ header_read_timeout */ });
    }
}
```

Verified this session: **there is no upper bound on concurrent spawned tasks.** Every accepted TCP
connection unconditionally gets its own `tokio::spawn`, regardless of how many are already active.
`00436dedc`'s `header_read_timeout` (default 10s) bounds how long **one** connection can stall, but
does nothing to bound **how many** connections can be simultaneously stalling at once — a client (or
a handful of coordinated clients) opening many connections and trickling headers slowly, each just
under the 10s timeout, can still exhaust file descriptors / memory / scheduler attention on the
accept loop well before any single connection times out. This is the exact "per-connection, not
per-listener-wide" gap the coordinator's brief names.

Two existing, complementary defenses were checked and confirmed to NOT cover this:
- `api::MAX_INFLIGHT_API = 64` (`api.rs:62`, enforced in `cap_middleware`, `api.rs:396-401`) only
  gates requests that reach the `/api/*` sub-router **after** a full HTTP request has been parsed —
  a connection stalled mid-header-read never reaches this check at all, so it provides zero defense
  against the accept-time exhaustion this blueprint targets.
- `DefaultBodyLimit::max(MAX_BODY_BYTES)` (`api.rs:678`) bounds request-body size on the API
  sub-router only; static `GET` requests (the `ServeDir` fallback path) carry no body and are
  correctly unaffected — not a gap, just a different concern than the one here.

Today's only mitigation is Cloudflare fronting the deployment in prod (per `00436dedc`'s own
message) — an **operational**, not **code-level**, defense. Per this repo's space-grade doctrine
(never downscope hardening on "infra already covers it" reasoning), the server itself needs a real
listener-wide defense as defense-in-depth, independent of what fronts it in any given deployment.

## 2. Design

Two independent, complementary layers, both bounded and both reusing primitives already in this
codebase rather than inventing new ones.

### 2.1 Global concurrent-connection cap — `tokio::sync::Semaphore`

`tokio` is already a full-featured dependency (`Cargo.toml`: `tokio = { version = "1", features =
["full"] }` — `sync` is included), so `Semaphore` costs zero new dependencies.

```rust
/// Hard cap on concurrently-accepted connections, independent of the [`api::MAX_INFLIGHT_API`]
/// per-API-request bulkhead (which only sees connections that finish parsing headers). Sized
/// well above MAX_INFLIGHT_API (64) to allow static-asset traffic headroom, but bounded so an
/// accept-time flood cannot grow the task count without limit.
pub const MAX_CONCURRENT_CONNECTIONS: usize = 512;

pub async fn serve_with_timeout(
    listener: tokio::net::TcpListener,
    router: Router,
    header_read_timeout: std::time::Duration,
) -> std::io::Result<()> {
    let conn_limit = std::sync::Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_CONNECTIONS));
    loop {
        let (stream, peer) = listener.accept().await?;
        // Fail-CLOSED, not blocking: if the cap is already saturated, drop this connection
        // immediately (RST on stream-drop) rather than queueing it — queueing would just move
        // the exhaustion from "tasks" to "an unbounded internal queue", the same failure shape
        // under a different name. The accept loop itself is NEVER blocked, so legitimate new
        // connections are never stuck behind an already-saturated cap once it drains.
        let permit = match conn_limit.clone().try_acquire_owned() {
            Ok(p) => p,
            Err(_) => {
                eprintln!("[native-spa-server] connection cap ({MAX_CONCURRENT_CONNECTIONS}) \
                           reached, dropping connection from {peer}");
                continue; // `stream` drops here, socket closes
            }
        };
        let router = router.clone();
        tokio::spawn(async move {
            let _permit = permit; // held for the connection's lifetime, released on task exit
            /* existing hyper builder body, unchanged */
        });
    }
}
```

### 2.2 Per-IP connection-rate limiting — reuse `kernel::token_bucket`, not a new limiter

Reuse the kernel's existing, already-tested `TokenBucket` (`kernel/src/token_bucket.rs`) rather
than writing a second rate-limiter — the exact same primitive `llm-adapters`' `Dispatcher` already
uses to bound LLM call concurrency (`llm-adapters/src/compose.rs`'s own doc: "Dispatcher {
TokenBucket-bounded }"), applied here to a different call site (per-IP connection admission instead
of per-call LLM budget).

```rust
/// Per-IP connection-rate state. Bounded map: a periodic sweep (§2.2 below) evicts idle
/// entries so a flood of DISTINCT source IPs cannot grow this map without bound — note that,
/// unlike a UDP flood, a TCP connection requires a completed handshake, so IP spoofing at this
/// layer is naturally constrained by the OS TCP stack, not by this code.
struct PerIpLimiter {
    buckets: std::sync::Mutex<std::collections::HashMap<std::net::IpAddr,
                                                          (dowiz_kernel::token_bucket::TokenBucket,
                                                           std::time::Instant)>>, // (bucket, last_seen)
}

impl PerIpLimiter {
    /// capacity=8 connections burst, refill=2/sec per IP — generous for a real browser opening
    /// several asset connections at once, tight against a single-source connection flood.
    /// Named constants, tunable at implementation time against real traffic if these prove wrong.
    const CAPACITY: f64 = 8.0;
    const REFILL_PER_SEC: f64 = 2.0;
    const IDLE_EVICT_AFTER: std::time::Duration = std::time::Duration::from_secs(300);

    fn admit(&self, ip: std::net::IpAddr) -> bool {
        let mut m = self.buckets.lock().unwrap();
        self.sweep_locked(&mut m); // bounded-growth eviction, see below
        let (bucket, seen) = m.entry(ip).or_insert_with(|| {
            (dowiz_kernel::token_bucket::TokenBucket::new(Self::CAPACITY, Self::REFILL_PER_SEC),
             std::time::Instant::now())
        });
        *seen = std::time::Instant::now();
        bucket.try_acquire(1.0)
    }

    /// Evict entries idle past IDLE_EVICT_AFTER — bounds map growth from many distinct IPs
    /// (each of which DID complete a real TCP handshake, per the doc note above).
    fn sweep_locked(&self, m: &mut std::collections::HashMap<std::net::IpAddr, (dowiz_kernel::token_bucket::TokenBucket, std::time::Instant)>) {
        m.retain(|_, (_, seen)| seen.elapsed() < Self::IDLE_EVICT_AFTER);
    }
}
```

Wired into `serve_with_timeout` immediately after `accept()`, before the semaphore permit is even
acquired (cheapest check first — a per-IP-throttled connection never touches the global cap):

```rust
let (stream, peer) = listener.accept().await?;
if !ip_limiter.admit(peer.ip()) {
    continue; // per-IP budget exhausted; drop, socket closes
}
let permit = match conn_limit.clone().try_acquire_owned() { /* as §2.1 */ };
```

## 3. Fits the existing architecture

- **Zero new external dependencies.** `tokio::sync::Semaphore` is already in the `full`-feature
  tokio the crate depends on; `PerIpLimiter` reuses `kernel::token_bucket::TokenBucket` verbatim —
  the same primitive already proven correct and already used elsewhere in this codebase
  (`token_bucket.rs`'s own doc: "the budget primitive the llm-adapters Dispatcher reuses").
- **Defense-in-depth, not a replacement.** `MAX_INFLIGHT_API` and `MAX_BODY_BYTES` stay exactly as
  they are — this blueprint adds a layer strictly BELOW them (at accept-time), closing the gap
  those two never covered, not duplicating what they already do.
- **Fail-closed, never blocking the accept loop.** Both new checks are `try_*`-style
  (`try_acquire_owned`, `try_acquire`) — a saturated cap drops the new connection immediately and
  the loop keeps calling `accept()`, so one exhausted resource never stalls admission of future
  legitimate connections (the same "one bad client cannot block others" property `00436dedc`'s
  header-timeout fix already established for the per-connection case, extended here to the
  per-listener case).

## 4. Acceptance criteria (RED → GREEN, per this repo's standing culture)

Mirrors `00436dedc`'s own test style (`r15_header_read_timeout_closes_stalled_connection`) —
a real client behavior against the real server, not a mocked unit test.

1. **RED, connection-flood test:** open `MAX_CONCURRENT_CONNECTIONS + N` raw TCP connections to a
   test server instance (each holding, not closing, e.g. sending a partial header line and then
   idling — same shape as the existing `r15` test), before this fix: connection count is
   unbounded, task count grows past the intended cap (demonstrable via the semaphore not existing
   yet, or the spawn count exceeding a defined threshold in a pre-fix build).
2. **GREEN, cap enforced:** after the fix, opening `MAX_CONCURRENT_CONNECTIONS + N` connections
   results in exactly `MAX_CONCURRENT_CONNECTIONS` connections accepted and held, and the remaining
   `N` are closed immediately (connection-refused/reset on the client side, verifiable via a
   read-returns-EOF-immediately assertion) — proving the cap is a real, enforced ceiling.
3. **Per-IP throttling, RED → GREEN:** from a single source IP, open `CAPACITY + N` connections in
   rapid succession — assert exactly `CAPACITY` succeed and the remaining `N` are closed
   immediately; from `CAPACITY + 1` DISTINCT source IPs (loopback aliases, e.g. `127.0.0.2`..
   `127.0.0.9` if the test harness supports binding multiple loopback addresses), assert every one
   succeeds independently (the per-IP bucket must not cross-throttle unrelated clients).
4. **No regression to legitimate traffic.** The full existing `tools/native-spa-server` integration
   suite (`tests/integration.rs`, currently 18/19 passing per `00436dedc`'s report, r5's Dockerfile
   test excluded as pre-existing/unrelated) stays green — ordinary single-client request/response
   cycles are unaffected by either new layer under normal load.
5. **Memory-bounded eviction proven.** A test that `admit()`s from `IDLE_EVICT_AFTER`-separated
   distinct IPs across simulated time (or a shortened test-only eviction window) asserts the
   `PerIpLimiter` map does not grow unbounded — i.e. `sweep_locked` actually evicts.
6. **Constants are named and documented**, not magic numbers — `MAX_CONCURRENT_CONNECTIONS`,
   `PerIpLimiter::CAPACITY/REFILL_PER_SEC/IDLE_EVICT_AFTER` all carry a rationale comment (per
   §2's draft) explaining the chosen value, matching `autonomic.rs`'s bounded-constant convention
   already established elsewhere in this codebase.
