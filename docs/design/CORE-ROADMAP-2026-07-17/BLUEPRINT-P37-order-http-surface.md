# BLUEPRINT P37 — Minimal HTTP/API surface for orders: the thin wire shell over kernel decide/fold (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map in §9 — every point
> addressed, none skipped). This phase is DELIVERY's **P37** as scoped by
> `docs/design/MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.5.3 — it absorbs RW-09
> (thin-shell boundary codify, `docs/design/rust-engine-rewrite/BLUEPRINTS-RUST-ENGINE-REWRITE.md:212`)
> and extends §10.5.3's six-item DoD into named RED→GREEN tests (§5). The roadmap-index note
> "No dedicated blueprint (deliberately)" is superseded by operator direction to blueprint it to
> the standard; the *scope* stays exactly what that note demanded: **the thinnest possible
> adapter**, not a REST design. Structural template: `BLUEPRINT-P-A-kernel-primitives.md`
> (section numbering mirrored).

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Working tree on `main`, 2026-07-18. Every row below was read from source this session, not
inherited from an older doc's claim.

| Claim | Fresh `file:line` (this pass) | Status |
|---|---|---|
| The ONLY HTTP server in-repo is static-file-only: `build_router` = `ServeDir` (+precompressed) with `ServeFile(index.html)` fallback, **zero dynamic routes** | `tools/native-spa-server/src/lib.rs:93-106` (`Router::new().fallback_service(serve_dir)` + 2 middleware layers) | **VERIFIED** — P37's extension point, not a from-scratch server |
| axum 0.8 already the routing substrate; tokio/tower-http/rustls cached & building offline | `tools/native-spa-server/Cargo.toml:23-27` | VERIFIED |
| `/healthz` exists but OUT-OF-BAND (a canned `health_response()`, not a router route) | `tools/native-spa-server/src/lib.rs:113-122`; binary wiring `src/main.rs:45-60` | VERIFIED — the "dynamic route" claim cannot lean on it |
| Existing RED-test convention `r1..r5` incl. a **grep-based deterministic gate** (`r5_zero_oci_gate_*` greps the Dockerfile) | `tools/native-spa-server/tests/integration.rs:161-259` | VERIFIED — §3.6's thin-shell gate reuses this exact pattern |
| Kernel order FSM: `OrderStatus` (Pending…Delivered + Scheduled + P07 compensation state), `fold_transitions` | `kernel/src/order_machine.rs:8-21`, `:156` | VERIFIED |
| Kernel money/order law: `place_order` | `kernel/src/domain.rs:156` | VERIFIED |
| JSON boundary fns exist but are **private + wasm-feature-gated**: `place_order_logic` / `apply_event_logic` (plain `fn`, `Result<String,String>`) inside `wasm.rs`, module gated `#[cfg(feature = "wasm")]` | `kernel/src/wasm.rs:203`, `:247`; gate at `kernel/src/lib.rs:209-210`; `wasm` feature pulls serde `kernel/Cargo.toml:24` | VERIFIED — §3.1's extraction is the load-bearing refactor |
| serde/serde_json are OFFLINE-CACHED (stated + building today) | `wasm/Cargo.toml:20-25` ("OFFLINE-CACHED (verified in registry cache)") | VERIFIED — no network gate on P37 |
| Kernel native default build is serde-free BY DESIGN; the no-serde graph check is already a documented command | `kernel/Cargo.toml:19-23`; `kernel/src/lib.rs:274` (`-e no-dev` graph check precedent) | VERIFIED — §3.1 must preserve this |
| Capability machinery IN-REPO (kernel mirror of bebop2 proto-cap): `verify_chain<V: SignatureVerifier>`, append-only `RevocationSet`, frame domain consts, deterministic `RefSigner` for tests, "production injects the real bebop2 Ed25519 + ML-DSA-65 verifier at the integration boundary" | `kernel/src/ports/agent/cap.rs:480` (`verify_chain`), `:404-433` (`RevocationSet`), `:31-33` (`DOMAIN_CAPABILITY`/`DOMAIN_FRAME`/`DOMAIN_DELEGATION`), `:82` (`SignatureVerifier` trait), `:120` (`RefSigner`), header `:15-18` | VERIFIED — DoD-4 builds on this, zero cross-repo dep needed to land |
| Canonical PROTOCOL gate (P34's swap-in): `HybridGate::check(frame, roster, chain, revocations, epoch)`, deny-by-default `new_redlined` | `/root/bebop-repo/bebop2/proto-cap/src/hybrid_gate.rs:59-129`, `:64` | VERIFIED (bebop-repo; cited as the seam, not a build dep) |
| delivery-domain (mesh side, P34's supplier): pinned wire discriminants 0x10–0x18, `DeliveryReceiver::admit_and_fold`, solo-island proof `ac6_solo_island_full_flow_no_peers` | `/root/bebop-repo/bebop2/delivery-domain/src/lib.rs:42-67`, `intake.rs:234`, `intake.rs:408` | VERIFIED — P37 may land against local kernel first (§10.5.3), P34 wires the mesh half |
| Offline canon F12: "Hub loses all peers, runs solo … island mode … LOCK" | `docs/design/ARCHITECTURE.md:75` | VERIFIED — DoD-5's authority |
| Browser local-decide beachhead: `web/src/app.mjs` console-only, binds 24/24 `_js` kernel exports, its OWN header defers the DOM pass ("G3 … separate work unit") | `web/src/app.mjs:1-12`; export count: 24 `pub fn *_js` in `kernel/src/wasm.rs` | VERIFIED — deliberate first step, NOT throwaway |
| Auth canon: D3 device-bound keypair primary = **capability certs** ("Reuses `HybridGate::check`/`verify_chain`/`RevocationSet`… unchanged"); TOTP/WebAuthn step-up only | `docs/design/BLUEPRINT-AUTH-DEVICE-2FA-2026-07-17.md:113`, `:144`, `:212-215` (§5.2 "device enrollment = capability cert, not a new subsystem") | VERIFIED — P37 must not contradict; P23-P3 wires onto P37's routes later |
| No `fly.toml`, no live deployment anywhere | `find /root/dowiz -name fly.toml` → 0 hits (this pass) | VERIFIED — bootable binary is the ceiling (DoD-6); deploy = P45 |

Ground truth is non-discussible; everything below builds on this table only.

---

## 1. Scope — what P37 owns vs deliberately does NOT (standard item 19 + §10.5.3 anti-scope)

**P37 owns (build items §3):**

| Item | Content |
|---|---|
| W37-1 | Kernel `json-api` feature: extract `place_order_logic`/`apply_event_logic` from `wasm.rs` into a feature-independent public module — ONE JSON authority for both the wasm surface and the wire |
| W37-2 | Dynamic order routes merged into `native-spa-server`'s existing router (place / advance / read + in-router `/healthz`), one binary serving static `web/` AND the API |
| W37-3 | Capability-cert check middleware over the kernel's existing `verify_chain`/`RevocationSet` (RW-09: the wire adapter is the second shell over the same kernel, same thin-shell rule) |
| W37-4 | Wire-lifecycle parity proof (over-the-wire fold ≡ direct fold, byte-identical) |
| W37-5 | Offline parity proof (F12): order placement with the server absent, via the wasm local-decide path |
| W37-6 | Thin-shell invariant as a **deterministic grep gate** (mechanism, not promise — §3.6) |
| W37-7 | Fail-closed adversarial set (forged/revoked/scope-mismatch/replay/malformed/oversize) + the one documented boot command |

**P37 explicitly does NOT own (each with its owner):**

- **NOT a full REST API design** — no pagination, no versioning, no resource modeling, no
  OpenAPI, no content negotiation (§10.5.3 anti-scope verbatim). Three routes and a health
  probe. Anyone adding a fourth route class must show which DoD item requires it.
- **NOT conventional auth** — no sessions, no passwords, no cookies, no login form. Auth is
  capability certificates per canon (D3, `BLUEPRINT-AUTH-DEVICE-2FA-2026-07-17.md`); TOTP/
  WebAuthn are P23/P39 step-up, wired later onto these routes, never primary.
- **NOT an admin CRUD surface** — no menu/product/tenant management routes (§10.5.3 anti-scope).
  P23-P3 unblocks on P37's *existence*, it does not smuggle its routes in here.
- **NOT deployment/monitoring** — no fly.toml, no metrics endpoint, no OpenTofu/Dokploy, no
  log shipping. That is P45 (ECOSYSTEM/OPS), hard-blocked on P37, not part of it. P37's ceiling
  is a bootable binary with one documented command (DoD-6).
- **NOT domain logic in handlers** — no pricing, no discounts, no state-machine edges, no money
  arithmetic in the server crate, ever (RW-09; enforced by §3.6's gate, not by review promise).
- **NOT mesh sync/rejoin** — the offline island rejoin and mesh-backed order data are PROTOCOL
  P34's job (`DeliveryReceiver::admit_and_fold` is P34's fold entry). P37 lands against the
  local kernel and names the seam (§4.4); it does not block on P34 finishing.
- **NOT persistence** — the event store is volatile in-memory (§2); durable storage is the
  pgrust/PgStore operator-gated track. Restart-loses-state is documented behavior, not a bug.
- **NOT WebSocket/live-push** — polling `GET` read is sufficient for every P37 DoD item.

---

## 2. Predefined types & constants (standard item 4 — named BEFORE implementation)

```rust
// ── kernel/Cargo.toml — new feature (W37-1) ─────────────────────────────────
// json-api: the JSON string boundary WITHOUT wasm-bindgen. `wasm` becomes a
// superset so the browser surface is unchanged. Native default stays serde-free
// (kernel/Cargo.toml:19-23 discipline preserved — gate proven in §3.1).
json-api = ["dep:serde", "dep:serde_json"]
wasm     = ["json-api", "dep:wasm-bindgen", "dep:serde_yaml"]

// ── kernel/src/json_api.rs — NEW module (extraction target, not new logic) ──
// Bodies MOVED VERBATIM from kernel/src/wasm.rs:203-.. (place) and :247-..
// (advance); wasm.rs keeps only the #[wasm_bindgen] one-line wrappers. Same
// fns serve browser-wasm AND the wire — parity by construction (§3.1, §8 P2).
pub fn place_order_logic(
    customer_id: Option<String>, items_json: &str, channel: Option<String>,
) -> Result<String, String>;
pub fn apply_event_logic(order_json: &str, next_status: &str) -> Result<String, String>;

// ── tools/native-spa-server/src/api.rs — NEW module in the EXISTING crate ───
// Route table (axum 0.8 `{id}` capture syntax). The verb set is deliberately
// minimal: ONE generic advance route mirrors apply_event_logic exactly —
// per-verb routes (accept/pickup/deliver) would encode FSM vocabulary in the
// shell, violating RW-09. The kernel rejects illegal edges; the shell relays.
pub const ROUTE_HEALTH: &str        = "/healthz";               // GET  (promoted in-router)
pub const ROUTE_ORDER_PLACE: &str   = "/api/order";             // POST — cap-gated
pub const ROUTE_ORDER_READ: &str    = "/api/order/{id}";        // GET  — cap-gated (read scope)
pub const ROUTE_ORDER_ADVANCE: &str = "/api/order/{id}/advance";// POST — cap-gated

pub const CAP_HEADER: &str = "x-dowiz-cap";       // base64 capability frame
pub const MAX_BODY_BYTES: usize = 64 * 1024;       // 413 above this; order JSON ≪ 4 KiB (§4.2)
pub const MAX_INFLIGHT_API: usize = 64;            // bulkhead: API concurrency cap (§4.3)
pub const EPOCH_SKEW_SECS: u64 = 300;              // cap-frame freshness window (replay layer 1)
pub const SEEN_DIGEST_RING: usize = 4096;          // in-window replay ring (replay layer 2)

/// Request bodies — the ONLY serde shapes the shell owns. There is deliberately
/// NO `Order` struct here: kernel-serialized order JSON is an OPAQUE String to
/// the shell (§3.6's gate makes parsing it a red test).
#[derive(serde::Deserialize)]
pub struct PlaceOrderBody {
    pub customer_id: Option<String>,
    pub items_json: Box<serde_json::value::RawValue>, // relayed raw to the kernel, never inspected
    pub channel: Option<String>,
}
#[derive(serde::Deserialize)]
pub struct AdvanceBody { pub next_status: String }    // e.g. "CONFIRMED" — kernel vocabulary, relayed

/// Volatile event store. State is ALWAYS fold-derived: the record holds the
/// kernel's latest serialized Order (opaque) + the append-only status-event
/// list (the event-sourced spine tests assert on, standard item 3).
pub struct OrderRecord { pub order_json: String, pub status_events: Vec<String> }
pub struct EventStore(std::sync::Mutex<std::collections::HashMap<String, OrderRecord>>);

/// HTTP rejection taxonomy — total, fail-closed. Every arm names its status.
pub enum ApiReject {
    Unauthorized,          // 401 — missing/unparseable/forged cap frame
    Forbidden,             // 403 — revoked, expired epoch, or scope mismatch
    NotFound,              // 404 — unknown order id
    KernelReject(String),  // 409 — kernel refused (illegal edge, money law); body = kernel msg
    Malformed,             // 400 — body fails serde before any kernel call
    TooLarge,              // 413 — > MAX_BODY_BYTES
    Replayed,              // 409 — duplicate frame digest inside the window
}

/// Capability check seam (W37-3). Default impl wraps the kernel's own
/// verify_chain + RevocationSet (cap.rs:480/:404) with RefSigner-injected
/// verification in tests; P34 swaps in the real hybrid Ed25519⊕ML-DSA-65
/// verifier / bebop2 HybridGate behind this SAME trait (§4.4). Deny-by-default:
/// every constructor takes the roster+revocations explicitly; there is no
/// "no-auth" constructor to misuse.
pub trait CapVerifier: Send + Sync + 'static {
    /// `frame` = decoded CAP_HEADER bytes; `body_digest` = BLAKE3/SHA-256 of the
    /// exact request body bytes; `route` = the matched ROUTE_* const (scope
    /// binding); `now_epoch` = server unix seconds. Ok(()) admits; Err carries
    /// the 401/403 split. Signing domain: DOMAIN_FRAME (cap.rs:32) — the HTTP
    /// adapter maps header bytes onto the EXISTING check inputs, nothing new.
    fn check(&self, frame: &[u8], body_digest: &[u8; 32], route: &'static str, now_epoch: u64)
        -> Result<(), ApiReject>;
}

/// Router assembly: the existing static router GAINS the API; static behavior
/// is byte-unchanged (r1..r5 stay green as the regression proof).
pub fn build_api_router(store: std::sync::Arc<EventStore>,
                        caps: std::sync::Arc<dyn CapVerifier>) -> axum::Router;
// lib.rs::build_router(root) → .merge(build_api_router(store, caps))
```

Rejected alternatives (DECART one-liners): **per-verb routes** (`/accept`,`/pickup`,`/deliver`)
— rejected: duplicates FSM vocabulary into the shell, RW-09 violation; one advance route
mirrors the single kernel entry. **Server-side Order struct** — rejected: any shell-side parse
of order JSON invites shadow domain logic; opaque String + §3.6 gate makes that structural.
**bebop2 proto-cap as a cargo path-dep now** — rejected for the landing shape: the kernel's
in-repo mirror (`ports/agent/cap.rs`) verifies chains today with zero cross-repo coupling;
the trait seam (§4.4) is where P34 injects the canonical HybridGate without touching handlers.

---

## 3. Build items — spec → RED test → code, each with adversarial cases (items 3, 5)

Spec-driven + event-driven TDD: §2 is the spec, every item's RED test precedes its code, and
lifecycle tests assert on the **status-event sequence** (`OrderRecord::status_events`), not just
end-state — matching the kernel's own decide/fold law.

### 3.1 W37-1 — kernel `json-api` extraction (the load-bearing refactor)

`place_order_logic`/`apply_event_logic` are today private and wasm-gated (`wasm.rs:203/:247`,
gate `lib.rs:209-210`). Move them verbatim (a *move*, not a copy — one authority) into
`kernel/src/json_api.rs` behind the new `json-api` feature; `wasm.rs` keeps its
`#[wasm_bindgen]` wrappers calling `crate::json_api::*`. The existing wasm-side tests
(`wasm.rs:851-878` corpus) move with the logic or re-point at the new module.

- **RED:** `cargo test -p dowiz-kernel --features json-api json_api::` — fails today (module
  absent). **GREEN:** the moved tests pass under `--features json-api` alone (no wasm-bindgen
  in the graph).
- **Adversarial (serde-free default preserved, item 14 gate):** a CI step asserts
  `cargo tree -p dowiz-kernel --no-default-features -e no-dev | grep -c "serde"` → `0`
  (the exact graph-check precedent documented at `kernel/src/lib.rs:274`). This turns "someone
  made serde unconditional" into a CI failure, not a binary-size surprise.
- **Adversarial (behavior pin):** the moved fns' outputs on the existing `SAMPLE_ITEMS` corpus
  are asserted byte-identical to pre-move captured fixtures — extraction must be motion, not
  edit.

### 3.2 W37-2 — dynamic routes on the existing crate

`native-spa-server` gains `src/api.rs` (§2) and a `dowiz-kernel = { path = "../../kernel",
features = ["json-api"] }` dependency. Handlers are relays:

- `POST /api/order`: decode `PlaceOrderBody` → `json_api::place_order_logic` → on Ok, mint an
  id (the kernel-serialized order's own id field is relayed opaque; the store key is the
  kernel-returned id extracted by the ONE permitted shallow read: a `serde_json::Value` id
  lookup, no other field touched) → insert `OrderRecord { order_json, status_events:
  vec!["PENDING"] }` → 201 + kernel JSON verbatim.
- `POST /api/order/{id}/advance`: load record → `json_api::apply_event_logic(&order_json,
  &next_status)` → on Ok replace `order_json`, push `next_status` onto `status_events` → 200 +
  kernel JSON verbatim; on Err → 409 `KernelReject` and the record is UNTOUCHED.
- `GET /api/order/{id}`: 200 + stored kernel JSON verbatim (fold-derived — the stored JSON is
  the kernel's own fold output, never shell-assembled).
- `GET /healthz`: promoted from the out-of-band `health_response()` (`lib.rs:113-122`) into a
  real router route — the same response body/headers, now inside `build_router`'s merged tree.

**RED:** new test `r6_dynamic_route_alive` — boot the binary (reuse `spawn_server`,
`integration.rs:126`), assert `GET /healthz` → 200 **via the router** and `POST /api/order`
without a cap header → **401** (not 404/405 — proves the route exists AND fails closed). Fails
today: the fallback serves `index.html` with 200 for both. **GREEN** after W37-2+W37-3.
**Adversarial:** `GET /api/order/../../etc/passwd` and `GET /api/%2e%2e/` → 400/404, never a
file read (API prefix must not leak into `ServeDir`); static routes `r1..r5` re-run green
byte-identical (the merge changed nothing static).

### 3.3 W37-3 — capability middleware (DoD-4)

`KernelCapVerifier` implements `CapVerifier` (§2) over `ports/agent/cap.rs::verify_chain`
(`:480`) + `RevocationSet` (`:404`) + the `DOMAIN_FRAME` signing domain (`:32`), generic over
`SignatureVerifier` exactly as the kernel already is (`:82`) — tests inject the deterministic
`RefSigner` (`:120`); production injects the real hybrid verifier at the P34 boundary (§4.4).
The check binds `(route const ‖ body digest ‖ epoch)` so a frame minted for READ cannot drive
ADVANCE and a frame for body A cannot authorize body B.

RED tests (each written first, each RED against a handler stack with no middleware):

```text
r8_forged_cap_rejected_401       — valid chain shape, signature bytes flipped → 401; store untouched
r9_revoked_cap_rejected_403      — valid frame, its key id present in RevocationSet → 403
r10_scope_mismatch_rejected_403  — frame minted for ROUTE_ORDER_READ replayed against ADVANCE → 403
r10b_expired_epoch_rejected_403  — frame epoch older than now − EPOCH_SKEW_SECS → 403
r10c_valid_chain_admitted        — the positive control: RefSigner-signed frame → 2xx (a gate that
                                   rejects everything is not a gate; this pins deny≠broken)
```

**Adversarial (fail-closed to what state — the §10.5.3 question answered):** every rejection
arm asserts `EventStore` length AND the target record's `status_events` are unchanged, and that
`json_api::*` was never entered (a test-only call counter on the store wrapper). Reject =
**zero writes, zero kernel calls, connection answered** — the unsafe state "unauthenticated
request mutated an order" is unreachable because the middleware runs before body
deserialization reaches any handler (type-level: handlers take an `Admitted` extension only the
middleware constructs — no constructor exists in handler scope).

### 3.4 W37-4 — wire lifecycle ≡ direct fold (DoD-2)

`r7_wire_lifecycle_matches_direct_fold`: over HTTP, place an order and advance it along the
kernel's golden path (place → CONFIRMED → PREPARING → READY → IN_DELIVERY → DELIVERED — edge
legality is the kernel's to define via `fold_transitions`/`boot_verify_fsm`, NOT this doc's);
in-process, run the identical sequence through `json_api::*` directly. Assert: (a) final order
JSON **byte-identical**; (b) the `status_events` sequence equals the direct sequence
(event-sequence assertion, standard item 3). Because both paths call the SAME extracted fns
(§3.1), identity is structural; the test falsifies accidental divergence (handler-side
mutation, reordering, encoding drift). **RED today** (no server). **Adversarial:** inject one
illegal edge mid-sequence over the wire (e.g. DELIVERED from PENDING) → 409, and the final
fold STILL matches the direct fold of the legal subsequence — a rejected intent leaves no
residue.

### 3.5 W37-5 — offline parity (DoD-5, F12)

Web-side test `web/src/lib/kernel/kernel.offline-order.test.mjs` (beside the existing
`kernel.test.mjs`, `web/package.json:9`): with **no server process at all**, drive
`place_order_js` + `apply_event_js` (the wasm bindings `app.mjs` already binds) through the
same golden sequence and assert the final order JSON equals the §3.4 wire fixture
byte-for-byte. This proves the HTTP server is NOT the required path for order placement
(F12, `ARCHITECTURE.md:75`). Rejoin/sync of the island's events is **P34's job** — this test
deliberately ends at the fold, claiming nothing about sync. **Adversarial:** run the node test
with `--network=none` semantics (no listener on the port; any fetch in the module under test
would throw) — the test must pass with zero network syscalls attempted by the order path.

### 3.6 W37-6 — thin-shell invariant as a deterministic gate (DoD-3: mechanism, not promise)

The named mechanism (the §10.5.3 question "enforced how" answered three ways, all deterministic):

1. **Grep gate** `r11_thin_shell_grep_gate` — same in-crate pattern as the existing
   `r5_zero_oci_gate_*` (`integration.rs:239-259`): reads `src/api.rs` and asserts ZERO
   occurrences of the banned token set `["OrderStatus", "compute_order_total", "checked_add",
   "checked_mul", "unit_price", "quantity *", "match .*next_status"]` — any FSM vocabulary,
   money arithmetic, or status-branching in the shell turns this RED at `cargo test` time.
2. **Type wall** — the shell has no `Order` struct to parse into (§2); the single shallow id
   read is the one documented exception, itself listed in the gate's allowlist with its line.
3. **Dependency wall** — `native-spa-server`'s kernel dep enables ONLY `json-api`; a CI step
   asserts `wasm-bindgen` absent from the server's `cargo tree` (nothing browser-shaped leaks
   into the binary).

RED direction proven at authoring time: temporarily add `let _ = OrderStatus::Pending;` to
`api.rs` → gate must fail; remove → green (the same has-teeth discipline as P-A §3.6's digest).

### 3.7 W37-7 — replay, malformed, oversize, boot command

- `r12_replay_rejected_store_unchanged`: re-send a byte-identical signed place request →
  first 201, second 409 `Replayed` (digest ring, `SEEN_DIGEST_RING`), store has exactly ONE
  record. Outside the ring window, layer 1 (epoch freshness, `EPOCH_SKEW_SECS`) rejects with
  403 — two independent layers, both tested.
- `r13_malformed_body_fail_closed`: syntactically-invalid JSON, valid cap → 400, kernel never
  called, store untouched. Semantically-hostile-but-well-formed JSON (negative quantity,
  absurd unit_price) is relayed and the KERNEL refuses (the existing `wasm.rs:878` neg-qty
  corpus is the authority) → 409 — the shell must NOT pre-validate domain values (that would
  be domain logic; the gate in §3.6 would catch the attempt).
- `r14_oversize_body_rejected`: body of `MAX_BODY_BYTES + 1` → 413 before buffering completes
  (tower body-limit layer), store untouched.
- **Boot command (DoD-6):** `cargo run -p native-spa-server -- --root web` — documented in the
  crate README §Run; the integration suite boots the same binary, so the command is
  test-exercised, not prose. Deploy packaging beyond this is P45.

---

## 4. Cross-cutting design obligations (items 6, 8, 9, 11–16)

### 4.1 Hazard-safety as math (item 6)

Reachability arguments, not prose: **unauthorized mutation** is unreachable because handlers
require the `Admitted` witness type constructible only by the middleware (§3.3) — the unsafe
state is unrepresentable in the handler's type signature, and the forged/revoked/scope tests
falsify the gate itself. **Illegal order states** are unreachable through the wire because the
shell owns no transition function — every mutation is `apply_event_logic`, whose FSM legality
is kernel law already boot-gated by `boot_verify_fsm` (golden-signature check,
`wasm.rs:373` docs); the shell cannot weaken what it cannot express (§3.6 type wall).
**Divergence between wire and local folds** is unreachable by construction (one extracted
function, §3.1) and falsified by §3.4/§3.5. **Money law** untouched: the shell relays integers
opaquely; `place_order`/`compute_order_total` (`domain.rs:129/:156`) remain sole authority —
consistent with the standing money red-line (memory: test-integrity money rules).

### 4.2 Schemas for scaling (item 8)

Stated axis: **orders/process-lifetime and requests/sec.** Assumption: one owner-operated hub
node, real-world order arrival ≪ 1/sec; test budget set at 50 req/s sustained (§6). The
volatile `EventStore` (HashMap + Mutex) is honest to ~10⁴ live orders and ONE process; its
named break points: (a) restart loses state → durable event log = the pgrust/PgStore
operator-gated track (NOT P37 creep); (b) contention at ≫ 10² req/s → shard the map or move to
the kernel event_log spine — both changes land behind `EventStore`'s API without touching
handlers. Order JSON ≪ 4 KiB (5-item order measured shape from the kernel corpus) — wire
budget compatible with P34's ≤ 1 MiB SyncFrame ceiling by 2+ orders of magnitude.

### 4.3 Isolation / bulkhead (item 11) + error-propagation gates (item 14)

- API traffic cannot starve static serving: `MAX_INFLIGHT_API` concurrency layer + body limit
  scoped to the API sub-router ONLY; static `ServeDir` path has no new layers (r1..r5
  byte-identical is the regression proof).
- A poisoned store Mutex answers 500 on API routes; static serving and `/healthz` are
  unaffected (separate state) — one failing subsystem degrades, the binary survives.
- CI gates per bug class: serde-free default (§3.1), thin-shell grep (§3.6), wasm-bindgen
  absence (§3.6), r1..r5 static regression — each named, each a test not a review note.

### 4.4 Mesh awareness (item 12) — the P34 seam, named exactly

P37 is **node-local**. The two seams P34 consumes, by name: (1) `CapVerifier` (§2) — P34
replaces `KernelCapVerifier`'s RefSigner injection with the real hybrid Ed25519⊕ML-DSA-65
verifier / bebop2 `HybridGate::check` (`hybrid_gate.rs:129`), zero handler changes; (2)
`EventStore::status_events` — the append-only per-order event sequence is exactly the shape
P34's `DeliveryReceiver::admit_and_fold` (`intake.rs:234`) folds from signed frames; when P34
lands, the store's write path gains frame emission (P34's blueprint owns that wiring — being
authored in a parallel task this session; cross-reference by phase number, not file, to avoid
a stale cite). Payload/frequency budget: order JSON ≪ 4 KiB at ≪ 1 event/sec — no transport
change needed.

### 4.5 Rollback / self-healing vocabulary (item 13, used precisely)

P37 claims only the **Self-Termination / unrepresentable-state leg**: typed rejection taxonomy
(§2 `ApiReject`, total match), witness-typed admission (§4.1), kernel-refused illegal edges,
fail-closed zero-write rejections (§3.3). **No Self-Healing claim** (no redundancy math here)
and **no Snapshot-Re-entry claim** — the store is volatile by declared scope; restart = clean
island re-entry via F12 local-decide, which is the canon behavior, not a recovery mechanism.
Mechanical rollback: delete `src/api.rs` + the kernel dep → the crate is byte-wise the DK-04
static server again; delete `json_api.rs` + feature → kernel graph unchanged (wrappers
re-inline).

### 4.6 Living memory (item 15) + tensor/spectral (item 16) + Linux discipline (item 9)

Item 15: `status_events` is a temporal append-only access pattern — the same
event-log-as-truth principle as `internal-retrieval-living-memory-arc-2026-07-14`; no flat
mutable state anywhere in the shell. Item 16: no closed-form math lives in this phase (the
shell computes nothing) — eqc-rs/spectral machinery is deliberately NOT applicable; stated
rather than decoratively claimed. Item 9 (verdict framework, reused not re-derived):
**ALREADY-EQUIVALENT** — "mechanism, not policy": the server transports, the kernel decides
(the syscall-boundary discipline); **REINFORCES** — one JSON authority for two surfaces (§3.1)
mirrors "one implementation of one concept"; **GAP** honestly named — no durable state, closed
by the gated pgrust track, not here.

---

## 5. DoD — falsifiable, RED→GREEN, extending §10.5.3's six items (item 2)

| §10.5.3 DoD | Named test(s) (RED today → GREEN at close) | Permanent regression (item 17) |
|---|---|---|
| 1. Dynamic server exists, one binary serves static + dynamic | `r6_dynamic_route_alive` (§3.2); grep finds ≥1 non-static handler in `src/api.rs` | r6 stays in suite |
| 2. Full lifecycle over the wire ≡ direct fold | `r7_wire_lifecycle_matches_direct_fold` (§3.4, byte-identity + event-sequence) | r7 + ledger row |
| 3. Thin-shell invariant, zero domain logic in handlers | `r11_thin_shell_grep_gate` (§3.6) + type wall + dep wall CI steps | r11 + both CI steps |
| 4. Capability-cert auth; forged/revoked rejected | `r8`/`r9`/`r10`/`r10b`/`r10c` (§3.3) | all five |
| 5. Offline parity (F12) | `kernel.offline-order.test.mjs` (§3.5) | stays in `web` suite |
| 6. Bootable with one documented command | command test-exercised by the suite's `spawn_server` boot (§3.7) | README §Run + suite boot |
| (new, fail-closed set) | `r12` replay / `r13` malformed / `r14` oversize (§3.7) | all three |
| (new, extraction integrity) | §3.1 byte-pin + serde-free graph gate | CI step |

Behavior-adding items (r7 wire parity, r11 thin-shell gate, offline parity) each add a row to
`docs/regressions/REGRESSION-LEDGER.md` per its standing ratchet rule. **Not-done clauses:**
weakening any adversarial test, `#[ignore]`-ing a rejection arm, or adding a route not listed
in §2 without a blueprint amendment = NOT done, regardless of green totals.

---

## 6. Benchmark plan (item 10) — measured numbers, not estimates

Budgets (single node, loopback, release build): **p50 ≤ 5 ms, p99 ≤ 25 ms** for
place-and-advance round trips INCLUDING the capability check (chain verify dominates; with the
deterministic RefSigner the bench isolates transport+fold cost; a second series with the real
hybrid verifier lands when P34 injects it — recorded as its own row, not blended).
**Throughput floor:** 50 req/s sustained for 60 s with zero rejections other than intended
409s, static r1..r5 latency unchanged within threshold (bulkhead proof, §4.3).

Mechanism: `r15_latency_budget` integration test drives N=500 sequential + 4×64 concurrent
requests, computes p50/p99, asserts the budget, and prints the numbers; results appended to
`tools/native-spa-server/BENCH_HISTORY.md` (same append-history discipline as
`kernel/benches/BENCH_HISTORY.md`). Telemetry hook: the budget test IS the regression tripwire
in CI (exit non-zero over budget) — P45 owns real runtime telemetry later; the dependency is
named, nothing monitoring-shaped is built here.

---

## 7. Links to docs & memory (item 7)

Depends on / cites: `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.5.3 (scope
authority; P37 DoD extended §5) · `CORE-ROADMAP-STANDARD-2026-07-17.md` (the contract) ·
`docs/design/ARCHITECTURE.md:75` (F12) ·
`docs/design/rust-engine-rewrite/BLUEPRINTS-RUST-ENGINE-REWRITE.md:212` (RW-09, absorbed) ·
`docs/design/BLUEPRINT-AUTH-DEVICE-2FA-2026-07-17.md` (D3 capability-cert canon; P23-P3 wires
onto these routes later) ·
`docs/design/sovereign-roadmap-2026-07-16/BLUEPRINT-P13-delivery-on-protocol.md` (wire-half
consumer) · PROTOCOL P34 blueprint (in parallel authoring this session — the §4.4 seams are
its intake) · `docs/regressions/REGRESSION-LEDGER.md` (item-17 mechanism) ·
`BLUEPRINT-P-A-kernel-primitives.md` (structural template). Memory:
`never-bypass-human-gates-2026-06-29` (pgrust persistence stays operator-gated) ·
`test-integrity-rules-2026-06-27` (money red-lines — shell relays, never computes) ·
`internal-retrieval-living-memory-arc-2026-07-14` (§4.6) · `verified-by-math-2026-07-07`.
Supersedes: the §10.5.3 "no dedicated blueprint" note only; scope unchanged.

---

## 8. Hermetic principles honored (item 20 — load-bearing only)

- **P2 CORRESPONDENCE** (one concept, one authority): §3.1's extraction — ONE JSON boundary
  serving browser-wasm and wire; one advance route mirroring one kernel entry; one health
  response promoted, not duplicated.
- **P6 CAUSE-AND-EFFECT** (determinism as law): state is always fold-derived from an
  append-only event sequence; byte-identity tests (§3.4/§3.5) falsify every determinism claim.
- **P7 GENDER** (paired verification, no self-certification): the wire path is refereed by the
  direct in-process path (§3.4) and by the serverless wasm path (§3.5) — two independent
  referees; the cap gate is refereed by both rejection AND admission tests (`r10c`).

(Other principles not load-bearing here and not claimed decoratively, per the Anu/Ananke
discipline.)

---

## 9. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 (fresh cites, all re-verified this pass) |
| 2 DoD | §5 (extends §10.5.3's 6 items with named tests) |
| 3 spec/event-driven TDD | §2 spec-first; §3 RED-first per item; §3.4 event-sequence assertion |
| 4 predefined types/consts | §2 |
| 5 adversarial/breaking tests | §3.2–§3.7 (forged/revoked/scope/replay/malformed/oversize/traversal/illegal-edge) |
| 6 hazard-safety as math | §4.1 (witness type, unrepresentable states) |
| 7 links docs/memory | §7 |
| 8 scaling axes | §4.2 (named break points) |
| 9 Linux discipline | §4.6 (verdict categories applied) |
| 10 benchmarks+telemetry | §6 (budgets + BENCH_HISTORY + CI tripwire) |
| 11 isolation/bulkhead | §4.3 |
| 12 mesh awareness | §4.4 (P34 seams named; payload budget) |
| 13 rollback/self-heal vocabulary | §4.5 (Self-Termination leg only, precisely) |
| 14 error-propagation gates | §3.1/§3.6 CI gates; §4.3 |
| 15 living memory | §4.6 |
| 16 tensor/spectral | §4.6 (honestly N/A, stated) |
| 17 regression ledger | §5 (rows named) |
| 18 agent-executable instructions | §10 |
| 19 reuse-first | §1 (extends existing crate), §2 (rejected alternatives), §3.6 (r5 gate pattern reused) |
| 20 Hermetic citations | §8 |

---

## 10. Clear instructions for other agentic workers (item 18 — zero session context assumed)

Execute in order; every task names files, acceptance command, and gate. The kernel extraction
(T1) is the critical path; T2+ are server-side.

1. **T1 (W37-1).** In `kernel/Cargo.toml`: add feature `json-api = ["dep:serde",
   "dep:serde_json"]`; change `wasm` to include `"json-api"`. Create `kernel/src/json_api.rs`;
   MOVE (not copy) `place_order_logic` (`kernel/src/wasm.rs:203`) and `apply_event_logic`
   (`:247`) plus their private helpers into it, make them `pub`; declare
   `#[cfg(feature = "json-api")] pub mod json_api;` in `kernel/src/lib.rs`; re-point the
   `#[wasm_bindgen]` wrappers (`wasm.rs:331,:343`) at `crate::json_api::*`. Move/point the
   corpus tests (`wasm.rs:851-878`). Acceptance: `cargo test -p dowiz-kernel --features
   json-api` green; `cargo test -p dowiz-kernel --features wasm` green;
   `cargo tree -p dowiz-kernel --no-default-features -e no-dev | grep -c serde` → 0.
2. **T2 (W37-2 RED).** In `tools/native-spa-server/tests/integration.rs`: add
   `r6_dynamic_route_alive` per §3.2 (boot via the existing `spawn_server`, `:126`; assert
   `/healthz` 200 in-router and `POST /api/order` → 401). Run — MUST FAIL (fallback serves
   index.html 200). Commit RED.
3. **T3 (W37-2/W37-3 GREEN).** Add `tools/native-spa-server/src/api.rs` with everything in §2
   verbatim (consts, bodies, `EventStore`, `ApiReject`, `CapVerifier`,
   `build_api_router`); implement `KernelCapVerifier` over
   `kernel/src/ports/agent/cap.rs::verify_chain` (`:480`) + `RevocationSet` (`:404`) with
   `RefSigner` (`:120`) injection for tests; add the kernel path-dep with
   `features = ["json-api"]`; merge in `lib.rs::build_router` (`:93-106`) via
   `.merge(build_api_router(...))` with body-limit + concurrency layers scoped to the API
   sub-router only. Acceptance: r6 green AND r1..r5 still green byte-identical.
4. **T4 (W37-3 adversarial).** Add `r8`, `r9`, `r10`, `r10b`, `r10c` per §3.3, each asserting
   zero store writes and zero kernel calls on rejection. Acceptance: all five green; removing
   the middleware layer must turn r8-r10b RED (verify once, restore).
5. **T5 (W37-4).** Add `r7_wire_lifecycle_matches_direct_fold` per §3.4 (byte-identity + event
   sequence + illegal-edge residue check). Acceptance: green; add the REGRESSION-LEDGER row.
6. **T6 (W37-5).** Create `web/src/lib/kernel/kernel.offline-order.test.mjs` per §3.5; wire it
   into `web/package.json`'s `test` script (chain after the existing `kernel.test.mjs`).
   Acceptance: `cd web && npm test` green with NO server process running.
7. **T7 (W37-6).** Add `r11_thin_shell_grep_gate` per §3.6 (copy the `r5_zero_oci_gate`
   pattern, `integration.rs:239-259`). Prove teeth: insert `let _ = OrderStatus::Pending;`
   into `api.rs`, confirm RED, remove. Add the two CI steps (serde-free graph, no
   wasm-bindgen in server tree) beside the existing workflow jobs. Acceptance: gate green,
   teeth demonstrated in the commit message.
8. **T8 (W37-7 + close-out).** Add `r12`/`r13`/`r14` per §3.7 and `r15_latency_budget` per §6
   (append results to `tools/native-spa-server/BENCH_HISTORY.md`, create the file). Document
   `cargo run -p native-spa-server -- --root web` in the crate README §Run. Run everything:
   `cargo test -p native-spa-server && cargo test -p dowiz-kernel --features json-api &&
   (cd web && npm test)`. Verify every §5 row. Do NOT mark P37 done if any rejection arm was
   weakened or any route beyond §2's table was added.

**Forbidden in this phase (repeated for the zero-context reader):** no sessions/passwords, no
admin routes, no pagination/versioning, no fly.toml/monitoring, no domain logic in `api.rs`
(the gate will catch you), no persistence layer, no P34 mesh wiring beyond the §4.4 trait seam.
