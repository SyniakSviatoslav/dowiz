# S6-REALTIME/WS Port — Council Packet · PROPOSAL

> **STATUS: 🟡 DRAFT — NOT APPROVED.** Description input to the live Triadic Council
> (system-architect + system-breaker + counsel + human). **No S6 code is ported to Rust until this
> packet is council-APPROVED, every quirk-register row (§12) is dispositioned one by one, and the
> operator signs the 🔴 open questions (`open-questions.md`).** This is a **🔴 red-line surface**
> (realtime authz + cross-tenant fan-out): a port defect here is a *cross-tenant order-feed leak*
> or a *silent realtime blackout*, and the cutover is the only one where a **live courier's
> in-flight-delivery connection crosses the flip**. Docs only; no product code.

- **Lane:** R5 (complete-rebuild) · **Surface:** S6 realtime WS (REBUILD-MAP §3 Phase-B; NOT a
  `(method, path)` rule — the cutover matcher special-cases `Upgrade: websocket` BEFORE any path
  match, mirroring the real server's lack of a path filter — route-surface-map surprise #7).
- **Date:** 2026-07-04 · **Source commit:** `fix/audit-remediation` (working tree).
- **Folds in (does not duplicate):** the partial S6 packet in `docs/design/rebuild-ws-authz-council/`
  (`packet.md` event-disposition table + room-authz model + lifecycle; `linkage-analysis.md`
  customer-JWT↔WS mint census A1–A8; `open-questions.md` prior Q1–Q12). Those remain the fine-grain
  backing; this packet is the council-facing synthesis + the sharpened cutover/parity analysis.
- **Census SSOT:** `inventory/10-api-realtime-jobs.md` §§2-6 (5 inbound msg types, 24 outbound event
  types, 5 room kinds, 33 publish sites) + the live source: `apps/api/src/websocket.ts` (the whole
  WS server), `apps/api/src/lib/courier-relay-guard.ts` + `courier-room-authz.ts` (ADR-0013),
  `packages/platform/src/message-bus.ts` (`PgMessageBus` LISTEN/NOTIFY), `packages/db/src/index.ts`
  (the operational-6543 / session-5432 pool split).
- **Governing ADRs / prior councils (this surface inherits hard-won invariants — do not re-litigate):**
  - **ADR-0013** (courier-realtime-authz) + its **DoD** (memory `adr-0013-courier-realtime-authz-dod-2026-06-29`)
    — the fan-out re-authz guard is already hardened on the Node stack (fan-out guard, ledger #40;
    25/25+2 mutants; 6/6 staging E2E). **Port it, do not re-derive it.**
  - **ADR-0004** (owner-token P-d live `status='active'` membership re-read) — the `#4` WS owner
    fan-out revocation gap fix (`websocket.ts:35-63,102-171`, `createOwnerRelayGuard`).
  - **ADR-0013-addendum-DRAFT** (`docs/design/ws-token-in-url/`) — the `Sec-WebSocket-Protocol
    bearer.v1` transport migration off the URL token; **still awaiting operator approval** — this
    council co-approves or explicitly sequences it (Q1).
  - **S2-auth RESOLVE** (`rebuild-auth-s2-council/`) — the RS256 body-`kid` verifier (`crypto.rs`,
    verifier `verify()`), the type-state extractor family (`extractors.rs`:
    `VerifiedClaims`/`OwnerClaimsExt`/`CustomerClaimsExt`/`CourierSession`), and the **REV-1 courier
    live-session bind** + **REV-3/T-12 `CustomerClaimsExt::require_order`** — all already built. **WS
    auth reuses the S2 verifier and claims types; it does not mint a second verifier.**
  - **S5-orders RESOLVE** (`rebuild-orders-s5-council/`) — S5 *publishes* to the bus; S6 owns the
    *transport + fan-out authz*. The `order:`/`dashboard:` room events are produced by S5's
    `updateOrderStatus`; their money/state semantics are S5's, their delivery is S6's.
- **Parity oracle:** the Playwright net's realtime slices — `courier-room-authz-isolation.spec.ts`,
  `cross-tenant-realtime-qa.spec.ts` (memory `cross-tenant-realtime-qa-2026-06-27`: 3 roles + WS +
  isolation, 6/6 staging-green), and the flow-core-lifecycle WS assertions — **plus** the
  invariant-cluster unit tests (tri-state verdicts 12/12 vs `courier-room-authz.test.ts`;
  relay-guard TTL/ceiling/no-refresh-on-read; owner `#4` revocation; claim-check truncation). No
  behavior change is real without a red→green test (Mandatory Proof Rule). Cutover DoD in §13.

---

## 1. Port objective and the load-bearing seams

S1 was read-only; S2 ported auth verification; S3/S4 ported owner catalog/media writes; S5 ports the
money+state machine and *publishes* order events. **S6 is the surface that DELIVERS those events to
browsers — the one place where an authz defect leaks one tenant's live order feed to another, and the
one cutover where a stateful-looking, long-lived connection crosses the flip mid-delivery.**

Four load-bearing seams, each an independent failure mode the port must hold simultaneously:

1. **The admission-authz seam** — a WS upgrade authenticates with the **same S2 verifier**
   (RS256 double-pin + kid-select + strict-parse + body-`kid`), pins a **per-connection principal**
   once, and subscribe gates each room with a **tri-state** verdict (ALLOW/DENY/UNAVAILABLE). A
   customer token for order A must never admit `order:B`; an owner must never admit another tenant's
   `location:`/`order:`; a courier must hold a **live binding** for the `order:` room (ADR-0013). The
   token transport must leave the URL (the `#5`/JWT-in-URL finding). (§4, Q1 🔴 / Q3 quirk.)
2. **The fan-out cross-tenant seam (the 🔴 heart)** — admission is *not enough*. A principal admitted
   and then **revoked** (courier reassigned, owner membership downgraded) keeps streaming until
   disconnect unless every frame is re-authorized at fan-out time. ADR-0013 (couriers) + `#4` (owners)
   already solve this on Node with two relay guards sharing one shape; the port collapses them to **one
   generic `RelayGuard<Policy>`** whose only per-frame write path to a member socket is the guard
   itself (the `local/no-raw-courier-ws-send` ESLint rule's intent becomes a compile-time module
   property). **A message for tenant A must never reach tenant B — at admission AND per-frame.**
   (§5, Q2 🔴.)
3. **The LISTEN/NOTIFY fan-out-transport seam** — `PgMessageBus` (Node) becomes a Rust **`PgListener`
   on a dedicated SESSION-mode connection**. LISTEN/NOTIFY does **not** work over the Supavisor
   transaction pooler (6543, multiplexed); it needs the session pooler (5432). If the Rust listener
   silently lands on the wrong pool, NOTIFYs are published by other processes but **this process never
   fans out — a silent realtime blackout with no error** (cutover HIGH-1). (§6, Q4 🔴.)
4. **The stateless-tail cutover seam** — a WS socket carries **no durable state**; it is a live tail
   over DB-authoritative order/assignment state. That single fact makes S6 the *gentlest* cutover of
   the whole rebuild: the flip is a **mass reconnect**, and reconnect-recovery (refetch-on-open, no
   server replay) is *already* the designed mechanism for any disconnect. But it introduces a **wire
   contract-parity** obligation — a client reconnecting mid-session onto the other stack must parse
   the same frame shapes — the S6 analog of S5's request-hash byte-fidelity gate. (§8/§9, Q5 🔴 / Q6 🔴.)

**The sharpest cutover fact (§9, Q5 🔴):** because fan-out is decoupled through DB NOTIFY channels
(stack-agnostic within one database), **both stacks can LISTEN and fan out concurrently during the
overlap** — a Node order-status change NOTIFYs the channel and the Rust listener hears it, and vice
versa. There is therefore **no hard flip moment** for S6 and **no connection migration is needed**: a
courier's in-flight-delivery socket on Node keeps working during overlap, and when it reconnects
(naturally or on drain) it lands on Rust, re-subscribes, re-authorizes the binding (ADR-0013), and
resumes tailing — the delivery state was in the DB the whole time. The residual is a **bounded
reconnect-gap event loss** (recovered by the REST refetch-on-open) and a **reconnect-storm** class
(mitigated by jittered backoff + the upgrade rate limit) — *provided* the channel names + payload
shapes are byte/shape-identical across stacks (Q6).

---

## 2. Scope, non-goals, and back-of-envelope

**In scope (S6):** the WS server (upgrade authn, per-connection principal), the 5 inbound message
types + 24 outbound event types + room kinds, the tri-state subscribe gate, the ADR-0013+`#4` fan-out
re-authz, the `PgListener` LISTEN/NOTIFY transport (claim-check, reconnect), connection lifecycle
(heartbeat, backpressure, expiry policy, drain), and the WS-specific cutover.

**Explicitly NOT S6:**
- **The token mint sites.** Owner/customer mints are S2; the **courier mint is `courier/auth.ts`**,
  path-owned by **S7** yet carrying S2's body-`kid` JWT-parity obligation (route-surface-map surprise
  #2, row 134). S6 *verifies* all three; it depends on those mints being body-`kid`-parity-correct —
  a **cross-surface dependency**: S6 WS courier auth ⟸ S7 `courier/auth.ts` mint ⟸ S2 body-`kid`
  contract (§4, threat WS-T7).
- **The event *producers*.** `updateOrderStatus` (S5), dispatch/offer/deliver (S7), and the
  timeout/notify workers (S8) *emit* to the bus. S6 owns delivery, not the money/state semantics of
  what is delivered.
- **The internal `BUS_CHANNELS.*` 34-topic namespace** (jobs/notifications lane, never
  browser-reachable) — must NOT be conflated with the 5 browser-facing WS rooms.
- **No schema change** — the DB is frozen. No new table, no new column; `customer_track_grants`,
  `courier_assignments`, `memberships` are read as-is.

**Back-of-envelope (why boring wins, and where the real ceiling is).**
- **Scale:** N ≈ 10–50 active locations → **peak concurrent WS sockets ≈ low-hundreds to ~1k**
  (per active order: ~1 customer + 0–1 courier tail; per location: 1–2 owner dashboards). Each socket
  is a tokio task + a bounded `mpsc` — memory-cheap; not the constraint.
- **Fan-out rate:** 24 event types over low-hundreds orders/day → **a handful of frames/sec
  steady-state**, bursty at lunch/dinner (~3–5×). The one sustained stream is customer GPS
  (`client_location`, ~1 Hz per active delivery) relayed to **one** courier member through the guard —
  at ~20 concurrent deliveries that is ~20 frames/sec. Negligible.
- **The operational (6543) pool draw is bounded by the TTL cache, not by fan-out volume.** Authz
  re-reads (courier binding / owner membership) hit the operational pool, but the relay guard's
  absolute ~10s TTL + inflight-dedup means a room with K members re-reads **at most once per ~10s per
  (room, principal)** — a location dashboard is O(1) re-reads/10s regardless of frame rate. Negligible
  on 6543.
- **The real ceiling is the SESSION-mode (5432) connection budget during overlap.** The `PgListener`
  needs **one dedicated session-mode connection** (LISTEN cannot ride the multiplexed 6543 tx pooler).
  Session mode does **not** multiplex — every session connection holds a real Postgres backend. During
  the S6 overlap the session-mode pool is drawn by: **Node `PgMessageBus` (1)** + **Rust `PgListener`
  (1, NET-NEW)** + **workers** (timeout-sweep / settlement-cron / notify-outbox on the session pool,
  `createSessionPool` max 3) + **migrations** (`release_command`, transient). **This is the S6 scaling
  gate:** the Rust listener adds a session-mode backend for the life of the overlap; budget it against
  the Supavisor session-mode ceiling and time-box the overlap (§9, Q4/Q5).
- **Conclusion:** S6 is **not** throughput-bound; boring monolith-in-`api` (one `/ws` route, one
  listener task, no new runtime) is correct. The engineering risk is entirely **correctness**
  (cross-tenant authz, silent-no-fanout) and **cutover** (wire parity, reconnect storm, session-conn
  budget), not scale. "Schema-rich, runtime-minimal": the `Principal::Channel` seam exists from day one
  (heads), but no head-WS runtime is switched on (Q11).

---

## 3. Target architecture (per REBUILD-MAP decision register)

```
crates/api/src/ws/
  mod.rs        — axum upgrade handler, single mounted GET /ws (WebSocketUpgrade extractor)
  auth.rs       — upgrade-time authn: reuse AuthState.verifier.verify(); Sec-WebSocket-Protocol
                  bearer.v1 (primary) / ?token= (flagged) / in-band Auth{token} (fallback);
                  build Principal; couriers ALSO run the REV-1 live-session bind (Q1)
  protocol.rs   — ClientMsg (5 inbound) + Event (24 outbound) — ONE enum pair, serde-exhaustive;
                  wire shapes carried VERBATIM during overlap, normalizations deferred (Q6)
  rooms.rs      — typed Room enum + registry (DashMap<Room, RoomState>); NO raw send export
  authz.rs      — Verdict{Allow|Deny|Unavailable} + per-principal room predicates (tri-state)
  guard.rs      — ONE generic RelayGuard<Policy> (courier ceiling-on / owner ceiling-off) — the
                  ONLY code path that can write to a member socket on fan-out
  pg_fanout.rs  — PgListener on a DEDICATED session-mode connection (NOT 6543); reconnect + re-LISTEN
                  with capped backoff, retried indefinitely; Event decode → guarded fan-out (Q4)
```

- **`PgListener` replaces `PgMessageBus`:** one dedicated session-mode connection, `LISTEN` per active
  room-channel, reconnect with capped backoff (parity with `message-bus.ts:101-114`) + re-LISTEN of
  all live channels on reconnect (`message-bus.ts:80-84`). **NOTIFY stays on a session-mode
  connection too** (the Node bus publishes NOTIFY on its own session pool, `message-bus.ts:116-131`, NOT
  the caller's 6543 tx — carry this: a NOTIFY issued on a returned-to-pool 6543 connection is
  unreliable). Claim-check preserved: a payload > ~7800 B is slimmed (Postgres NOTIFY 8000 B cap,
  `message-bus.ts:22-23,140-154`).
- **Publish sites:** all 33 room-channel `messageBus.publish(...)` sites map to one
  `publish(room, event)` — serde is the only wire encoder, so an ad-hoc frame is unrepresentable.
  During the strangler overlap, S5/S7/S8 producers on Node publish; the Rust listener consumes — so the
  Rust `publish` is only needed once those producers themselves flip (§9).

---

## 4. Concern 1 — WS admission authn + the token transport (Q1 🔴, Q3 quirk)

**Authn at upgrade — reuse the S2 verifier, never a second one.** The Rust WS handler extracts the raw
token and calls **`AuthState.verifier.verify(&token)`** (the exact function `extractors.rs:88-90`
uses) → the role-tagged `Claims`. It does **not** use the header extractors (`OwnerClaimsExt` et al.)
directly, because a browser cannot set `Authorization` on a WS handshake — so the transport differs but
the **crypto verify is identical** (RS256 double-pin, kid-select, `alg:none` reject, strict Zod-parity
parse, body-`kid` — S2 REV-C1). Then the handler builds the per-connection `Principal`.

**Token transport (adopt the ADR-0013 addendum, gated — Q1 🔴):**
- **Primary:** `Sec-WebSocket-Protocol: bearer.v1, <jwt>` — server validates, echoes **`bearer.v1`
  only** (never the token; RFC 6455 §4.2.2). Zero extra round-trips; keeps the 5s auth budget; token
  leaves `req.url` entirely (closes the residual browser-history/Referer/intermediary-log vector that
  P1/ledger #42's Pino redaction cannot reach). **The addendum is itself awaiting operator approval —
  this council co-approves it or explicitly sequences it.**
- **Transition:** `?token=` dual-accept behind **`WS_URL_TOKEN_ACCEPT`** (default ON at S6 cutover for
  cached PWA/SW clients; driven to OFF via the existing deprecation counter,
  `websocket.ts:179-181`). The SPA must stop setting `?token=` in the same release — today
  `useWebSocket.ts:50` sets it **AND** sends `{type:'auth'}` (linkage A4), so the counter can never
  hit zero while the client sets the param.
- **In-band fallback:** `ClientMsg::Auth{token}` (5s deadline → close 1008, `websocket.ts:357-386`) —
  the documented churn-test contract; retained.
- **Redaction guardrail:** port `redactUrlSecrets` semantics (ledger #42) to the tower-http trace
  layer AND add `sec-websocket-protocol` to the header redact list (addendum guardrail 1).

**Per-connection principal — pinned once at upgrade (type-state):**

```rust
enum Principal {
  Owner    { user_id: UserId },
  Courier  { sub: CourierSub, active_location_id: LocationId, jti: SessionId },
  Customer { order_id: OrderId, location_id: LocationId, sub: CustomerId },
  Channel  { channel_id: ChannelId, scopes: ChannelScopes },   // §11 invariant 2 — no WS in v1
}
```

**The NEW finding this port must close — courier WS admission verifies crypto only, not session
liveness (Q1 🔴).** On the Node stack a courier WS upgrade runs `verifyAuthToken` (crypto) then
per-subscribe `courierRoomVerdict` (order binding) — but **never the REV-1 `courier_sessions`
liveness check** that every REST courier request runs (`plugins/auth.ts:60-93`). So a **logged-out
courier** (session revoked) holding an unexpired 14d token can reconnect WS and keep receiving frames
for orders they still hold a binding on (logout does not drop the binding). REST would 401 them; WS
does not. **The port closes this by reusing the S2 `CourierSession` bind at upgrade** (the
`courier_session_bind` + `courier_session_valid` path, `extractors.rs:166-208`) so a WS courier
admission IS a live-session check — parity with REST, and the natural place to close the gap since S2
already built the bind. (Owners already get live membership at subscribe + per-frame; customers are
the deliberate irrevocable-bearer, prior-Q1/S2 — bounded by short TTL + order-scope.)

**Tri-state admission table (ADR-0013 parity — carry verbatim):**

| Principal | Room | Predicate | DB? |
|---|---|---|---|
| Customer | `Order(o)` | `o == claim.order_id` **exact** (the mint's own order id — `websocket.ts:392-397`) | no |
| Customer | anything else | DENY | no |
| Owner | `LocationDashboard(l)`/`LocationCouriers(l)` | live active owner membership (`ownerRoomVerdict`, incl. `status='active'` — the `#4` fix) | yes |
| Owner | `Order(o)` | order→location JOIN live active membership (ADR-0004) | yes |
| Courier | `Courier(s)` | `s == claim.sub` exact | no |
| Courier | `Order(o)` | live `courier_assignments` binding, `BINDING_READ_STATUSES=[offered,assigned,accepted,picked_up]`, inside a tenant-GUC tx (NOBYPASSRLS-sound — `courier-room-authz.ts:32-66`) | yes |
| Courier | anything else (incl. `Location*`) | DENY, zero DB | no |
| Channel | — | no WS rooms in v1 (Q11) | — |

Tri-state mapping (Breaker H1/NEW-A parity): `Unavailable` → **retryable soft-error frame, socket
stays open** (a pool blip must not fleet-deny); `Deny` → `Forbidden room` frame, no close;
`Allow` → admit. The WS-only retryable distinction (vs REST's fail-closed 404/503) is intentional and
must be preserved (`courier-room-authz.ts:16-20`).

**The no-path-filter quirk → FIX-IN-PORT (Q3).** Today `new WebSocketServer({ server: fastify.server })`
(`websocket.ts:192`) has **no `path`** → the `ws` package intercepts **every** HTTP Upgrade on any URL
(the dead widget's `/ws/orders/:id` and the SPA's `/ws` both land here; linkage A8). In axum a WS route
is inherently path-bound: mount **one** `GET /ws` with the `WebSocketUpgrade` extractor → an upgrade to
any other path 404s before the handshake. The quirk becomes **unrepresentable**. **Interaction with the
cutover matcher:** the matcher routes by `isWebSocketUpgrade()` (the `Upgrade` header) **before** any
path template (surprise #7) — so during overlap the proxy steers *every* WS upgrade to whichever stack
owns S6, regardless of URL. If Rust owns S6, a non-`/ws` upgrade reaches Rust and gets a clean 404/close
(correct — the only live client uses `/ws`, `useWebSocket.ts:6`; the `/ws/orders/:id` widget is dead,
linkage A5). This is a **security-narrowing FIX with a tiny, safe delta**; carry the matcher's
header-first steering unchanged (do not add a `/ws` path template to the proxy — that would be a phantom
precision the Node server never had).

## 5. Concern 2 — Fan-out cross-tenant re-authz (Q2 🔴 — ADR-0013 + `#4`)

**Admission gates NEW subscribes; the fan-out gates EVERY frame.** The role-agnostic bus handler
(`websocket.ts:215-233`) sends to *every* room member — the exact site the C1 involuntary-reassign leak
rides. The port keeps **one chokepoint**: `rooms.rs` exports **no raw send**; the only path that writes
to a member socket on fan-out is **one generic `RelayGuard<Policy>`** (`guard.rs`), instantiated twice:

- **Courier policy** (`createCourierRelayGuard` parity): relay-only-on-fresh-ALLOW; absolute ~10s TTL,
  **no refresh-on-read** (a reconnect-flap cannot keep a revoked entry warm); withhold-then-async-revalidate
  (never relay-then-check); `Deny` → evict from room + `Event::Evicted{BindingRevoked}` (never
  socket-close); `Unavailable` → withhold + the **in-memory 60s wall / count ceiling** that fires from
  in-memory state only (holds under total DB starvation — Breaker NEW-A). Inflight-dedup + LRU bound.
- **Owner policy** (`createOwnerRelayGuard`/`ownerRoomVerdict` parity, the `#4` fix): identical shape,
  ceiling **off** — withhold without eviction on `Unavailable` (owners have no GPS stream to bound; a
  pool blip must not bounce a live owner). `Deny` (revoked/downgraded membership) → evict +
  `Event::Evicted{MembershipRevoked}`. OR-9 residual, stated honestly: a revoked owner stops streaming
  within **≤TTL, not literally zero** (`websocket.ts:98-101`).
- **Customers:** relayed directly — admission is authoritative (no per-order binding to revoke). The
  ONLY additive is the connection-level expiry policy (Q8), which is not per-frame.

**The drift guardrail becomes a type.** The `local/no-raw-courier-ws-send` ESLint rule (keyed on the
send *site*, banning any raw `member.ws.send` over a courier-joinable room outside the guard) ports as
**module visibility**: a member socket handle is only reachable through the guard. A divergence requires
a new `Policy`, which is exactly the review point we want (Q2 recommends one generic guard over two
ported structs). **The invariant the whole surface exists for: a message for tenant A never reaches
tenant B — at admission AND at every frame — even after a mid-stream revocation.**

## 6. Concern 3 — LISTEN/NOTIFY → `PgListener` (Q4 🔴 — the silent-blackout class)

**The transport, spelled out because it is the one place a green build silently delivers zero events.**

- **Dedicated SESSION-mode connection.** `LISTEN` requires a session that persists across statements;
  the Supavisor **transaction pooler (6543) multiplexes and returns connections to the pool between
  statements**, so a `LISTEN` issued there is orphaned and **no notifications arrive** — with no error.
  The Node bus already sidesteps this: `PgMessageBus` defaults to `createSessionPool()` (port 5432,
  `message-bus.ts:25-28` + `db/index.ts:44-63`). **The Rust `PgListener` MUST connect on the
  session-mode DSN (`DATABASE_URL_SESSION`), never the operational one.** A guardrail asserts the
  listener's DSN is the session DSN (a config-time check, red→green).
- **NOTIFY also stays on a session-mode connection.** The Node bus issues `NOTIFY` on its own session
  pool (`message-bus.ts:116-131`), **not** the caller's 6543 tx — because a NOTIFY on a tx-pooled
  connection that is immediately released may not deliver. When the Rust producers (S5/S7/S8) eventually
  flip, their `publish` must NOTIFY on a session-mode connection too. During the current overlap the
  producers are still on Node, so this is a *forward* obligation, flagged now so it is not "discovered"
  at the S5/S7 flip.
- **Reconnect = indefinite, with re-LISTEN.** Carry `message-bus.ts:95-114`: capped exponential backoff
  (cap 30s), retried **forever** (the old 5-attempt cap left a machine "alive but realtime-dead"), and
  on every successful reconnect **re-LISTEN all live channels**. A single in-flight `reconnecting`
  guard prevents stacked timers.
- **Claim-check → explicit `Resync`.** A payload > ~7800 B is slimmed to `{_truncated, type, data.id}`
  (`serializeForNotify`, `message-bus.ts:140-154`). In Rust the truncated frame becomes a first-class
  **`Event::Resync{entity, id}}`** the FE handles as "refetch via REST" — an explicit contract, not an
  accidental-refetch heuristic (inventory 10 §5).
- **Per-handler isolation — carry verbatim.** `dispatch` (`message-bus.ts:168-181`) swallows a throwing
  subscriber so one bad handler degrades a single broadcast, not the process. In Rust each room's
  fan-out is an isolated task; a panic in one fan-out never aborts the listener. (Historical: an async
  handler rejection once crashed the whole API — `message-bus.ts:156-167`.)

**The residual this port must state honestly (threat WS-T6):** NOTIFY is **fire-and-forget after
COMMIT**, not a transactional outbox. A producer that COMMITs the order-status change and then dies
before the NOTIFY lands **loses that live event** (no replay, no backlog — by design, inventory 10 §6).
Recovery is the client's **refetch-on-reconnect**. This is a *carried* property (the canon's preferred
"transactional outbox" is a **future** hardening, not an S6 deliverable — "schema-rich, runtime-minimal":
S6 preserves the seam by making the truncated/lost case an explicit `Resync`, but does not build the
outbox). Accepted-risk row + owner (§11).

## 7. The event contract — ONE enum, but wire-shapes carried verbatim during overlap (Q6 🔴)

`protocol::Event` is exhaustive (24 outbound types); `ClientMsg` is exhaustive (5 inbound:
`Auth{token}`, `Subscribe{room}`, `Unsubscribe{room}`, `ClientLocation{lat,lng}`,
`ClientLocationStop`). Unknown inbound type = a protocol-error frame (today: silent `console.warn`,
`websocket.ts:491`). **The critical cutover constraint the prior draft under-stated:** the envelope and
every frame shape must be **byte/shape-identical across stacks** because a client can reconnect
mid-session onto the other stack (§9). Today's envelope is `{room, data: msg}`
(`websocket.ts:218`); control frames are `{type:'auth_success', role}`, `{type:'subscribed', room}`,
`{type:'error', error}`, `{type:'error', error, retryable}`, `{type:'client_location', payload}`.

- **CARRY-VERBATIM during the overlap.** The Rust `Event`/`ClientMsg` serde encoders must reproduce the
  exact Node wire shapes (including the `{room, data}` envelope and the current `{type:'error', error}`
  frames). A **golden-frame parity test** (fixed event → exact Node JSON string, both directions) is the
  S6 analog of S5's request-hash golden-vector gate — a shape drift breaks a mid-session cross-stack
  reconnect.
- **DEFER the normalizations to a post-cutover FE-lockstep release.** The prior draft (`packet.md` §3)
  proposes valuable normalizations — collapsing `order.status`'s two shapes (flat on `order:` vs
  `data`-wrapped on dashboard) to one; promoting the two generic `{type:'error'}` eviction notices to a
  typed `Event::Evicted{reason}`; the 19 published-but-unhandled dispositions (9 CONSUME / 5
  STOP-PUBLISH / 2 RETIRE / 1 KEEP-internal). **Every one of these changes the wire and therefore
  requires the FE to move in lockstep** — so they land **after** Node is decommissioned (single-stack)
  and the Astro FE consumes the new shapes, **not** during the overlap. During overlap: emit the legacy
  shapes; after cutover: flip to normalized shapes + FE in one release. (Q6 + Q7 — the disposition table
  is a *post-cutover work list*, dispositioned now, applied later.)

## 8. Connection lifecycle + expiry policy (Q8 🔴)

- **Heartbeat:** server ping 30s, terminate after one missed round-trip (`websocket.ts:287-297` parity).
- **Backpressure:** per-connection bounded send queue (tokio `mpsc` + broadcast-lag detection). On
  overflow/lag: drop the member's queued frames + send `Event::Resync{...}` (client refetches via REST —
  the documented recovery; deliberately NO server-side replay). Slow consumer that never drains → close
  1013.
- **Room GC:** eager teardown on last-leave (incl. `UNLISTEN`) + periodic sweep — exactly one
  bus/LISTEN subscription per room (P1-WSDUP parity, `websocket.ts:196-210,498-517`; re-subscribe races
  covered by tests).
- **Expiry-mid-subscription (Q8 🔴 — closes linkage A3/A7).** Today **nothing re-checks `exp` after
  admission** (for any role), and an expired token on reconnect → 1008 → the SPA hook reconnects
  **forever** (`useWebSocket.ts:86-109`) — a silent permanent reconnect loop (owner 24h dashboards hit
  this daily). Port a **connection-level** policy: track `min(exp)`; near `exp` send
  `Event::TokenExpiring`, allow one in-band `Auth` refresh within a grace window, then close with a
  **distinct 4401-class code** the client treats as "re-authenticate, don't blind-retry." This is the
  only option that kills the reconnect-storm class without per-frame cost. **Anonymous orders** (no
  token, linkage A7): the server sends a **terminal close code** the client maps to REST-poll mode
  (stop the unbounded 1008 loop).
- **Shutdown/drain:** stop accepting upgrades → close sockets 1001 (going-away) → `UNLISTEN` → release
  the session connection (inventory 10 §5).

## 9. Cutover concurrency — the stateless-tail flip (Q5 🔴)

**Counsel's S6 concern: a live courier's in-flight-delivery WS connection crossing the flip.** The
resolution is structural, not a special-case:

1. **A WS socket carries no durable state.** Assignment, order status, GPS position, cash-as-proof —
   all DB. The socket is a live tail. So a dropped socket loses **nothing durable**; the client
   reconnects, re-subscribes, re-authorizes (ADR-0013 binding re-check), and refetches current state
   via REST (accept/pickup/delivered are REST POSTs, S7). **In-flight delivery survives the reconnect
   by construction.**
2. **No hard flip moment; no connection migration.** Fan-out is decoupled through DB NOTIFY channels
   (stack-agnostic within one database). During overlap **both stacks LISTEN + fan out concurrently**:
   a Node order-status change NOTIFYs the channel; both Node's and Rust's listeners hear it; each fans
   out to **its own** room members. A client is on exactly one stack, so **no double-delivery to a
   single client**. A courier's socket on Node keeps working *during* overlap; the flip just steers
   **new** upgrades to Rust and lets Node's sockets **drain** (natural reconnect, or an explicit 1001
   to trigger immediate jittered reconnect).
3. **The reconnect-storm is the real hazard, and it is bounded.** Draining all sockets at once →
   thundering herd on the Rust upgrade path. Controls: **jittered client backoff** (already
   `useWebSocket.ts` reconnect + the ≤60s expiry jitter), the **per-IP upgrade rate limit** (Q9,
   tower-governor), and **gradual drain** (stop-new + let heartbeat/idle churn migrate, rather than a
   synchronized mass-close). The reconnect-gap event loss (a few seconds) is recovered by
   refetch-on-open — the designed mechanism.
4. **Wire-parity is the flip's correctness gate (Q6).** A socket that reconnects from Node (legacy
   shapes) onto Rust must parse Rust's frames identically → the golden-frame parity test is a **named
   cutover gate**, co-equal with the authz tests. This is why the shape normalizations (§7) defer to
   post-cutover.
5. **The session-connection budget is the flip's capacity gate (§2, Q4).** The Rust `PgListener` adds a
   NET-NEW session-mode backend for the overlap; time-box the overlap and monitor the session-mode
   ceiling (the flip is the *contract* to shed the Node listener's session connection).
6. **Rollback = steer new upgrades back to Node.** Because both stacks LISTEN the same channels and
   fan out DB-authoritative state, a rollback mid-overlap is a proxy-flag flip (`WS_SURFACE=node`) +
   mass-reconnect — no data migration, no lost order.

**S6 can be flipped INDEPENDENTLY of S5/S7** (the coupling is DB NOTIFY, stack-agnostic) — but only if
the channel names + payload shapes are identical across stacks (Q6). This decoupling is a feature: WS
does not have to wait for orders/dispatch to flip.

## 10. Security + tenant isolation (woven through §§4–5)

- **Cross-tenant leak (WS-T1) is the surface's whole reason to exist** — closed at admission (tri-state
  per-principal predicates) AND per-frame (the relay guard). Neither alone suffices: admission misses
  mid-stream revocation (ADR-0013 C1); the guard alone would relay-then-check. Both, or neither is
  sound.
- **JWT-in-URL (WS-T2)** — the subprotocol transport (Q1) removes the token from the URL entirely;
  redaction (ledger #42 + `sec-websocket-protocol`) is defense-in-depth for the dual-accept window.
- **Unauthenticated upgrade (WS-T3)** — single `/ws` route + Origin allow-list (CSWSH belt) + per-IP
  upgrade rate limit + the 5s auth deadline (Q9). Data still requires auth even before these; they bound
  churn/DoS surface.
- **Tenant GUC on the courier binding read** — `courierRoomVerdict` runs inside a `BEGIN` +
  `set_config('app.current_tenant', activeLocationId, true)` tx (`courier-room-authz.ts:44-53`), sound
  under both BYPASSRLS (today) and NOBYPASSRLS (post-B3) — order-independent of the B3 flip. Carry
  verbatim; the owner predicate's `memberships` JOIN is the tenant boundary.

## 11. Operability + open/accepted risks

- **Health: degraded-vs-down.** The `PgListener` exposes `checkHealth() → ok|degraded`
  (`message-bus.ts:237-239` parity): **degraded = listener disconnected/reconnecting** (NOTIFYs still
  published by other processes but this process is not LISTENing → the silent-no-fanout window). This
  must surface **<1 min**: a metric on listener-disconnect + a distinct `degraded` health signal
  (never fold it into a binary up/down — a degraded listener serves HTTP fine while realtime is dead,
  the exact trap). Down = the axum process gone (livez).
- **Observability.** Per-room member gauge, fan-out counter, `withheld`/`evicted` counters (the relay
  guard's `_stats`), listener-reconnect counter, upgrade rate-limit rejections — each <1 min to a
  dashboard.
- **Rollback / flag / scaling-gate.** `WS_SURFACE=rust|node` (proxy steer) + `WS_URL_TOKEN_ACCEPT`
  (transport) + the Node listener stays warm one release. The scaling-gate is the session-mode
  connection ceiling (§2): do not run an indefinite dual-listen.
- **Accepted / residual risks (each with an owner):**
  1. **Fire-and-forget post-COMMIT NOTIFY (WS-T6)** — a producer crash between COMMIT and NOTIFY loses
     a live event; recovery is refetch-on-reconnect; no transactional outbox in v1. Owner: architect
     (defer the outbox to a post-rebuild hardening council). **Most likely counsel flag.**
  2. **OR-9 owner/courier revocation is ≤TTL, not zero** — carried from ADR-0013/`#4`; a true
     zero-window needs a push-based drop at `memberships.status`/binding flip (out of scope). Owner:
     S6 lead.
  3. **Customer is the irrevocable per-order bearer** — bounded by order-scope (exact `order_id`) +
     the S2 short-TTL/track-grant policy (prior-Q1); GDPR erasure eviction is the WS belt (Q10). Owner:
     S2 lead.
  4. **Reconnect-storm at flip** — bounded by jitter + upgrade rate limit + gradual drain, not
     eliminated. Owner: architect + operator. **Most likely breaker escalation** (attack a synchronized
     mass-close + the session-conn budget).

## 12. Quirk register — carry-vs-fix (default = CARRY-VERBATIM)

**FIX-IN-PORT only for a 🔴 security/correctness issue or a build-correctness bug, each with an explicit
test/E2E delta.** Everything else CARRIES; wire-shape normalizations defer to post-cutover FE-lockstep.

| ID | Quirk (source) | Disposition |
|---|---|---|
| Q-WS-VERIFIER | WS upgrade uses the same crypto verify as REST (`verifyAuthToken` → Rust `verifier.verify`) | **CARRY the verifier** (S2 body-`kid`); reuse it, do not mint a second one |
| Q-WS-COURIER-SESSION | courier WS admission verifies crypto only — **no REV-1 `courier_sessions` liveness** (`websocket.ts` courier branch), so a logged-out courier keeps a live tail | **FIX-IN-PORT 🔴** — reuse the S2 `CourierSession` bind at upgrade; E2E: revoke session → next WS reconnect denied (Q1) |
| Q-WS-TOKEN-URL | token in URL **and** in-band (`websocket.ts:338-355` + `useWebSocket.ts:50,61`), dual-transmit blocks the deprecation counter | **FIX-IN-PORT** per addendum (gated): subprotocol-primary + `?token=` flag'd; SPA stops setting `?token=` same release (Q1) |
| Q-WS-NOPATH | `WebSocketServer({server})` accepts upgrades on **any** URL (`websocket.ts:192`) | **FIX-IN-PORT** — single axum `GET /ws`; non-`/ws` upgrade 404s; matcher keeps header-first steering (Q3) |
| Q-WS-TRISTATE | subscribe tri-state (ALLOW admit / DENY forbid-no-close / UNAVAILABLE retryable-soft-error) (`courier-room-authz.ts:16-20`) | **CARRY verbatim** — the WS-only retryable distinction; a pool blip must not fleet-deny |
| Q-WS-RELAY-GUARD | ADR-0013 courier guard + `#4` owner guard, relay-only-on-fresh-ALLOW, no-refresh-on-read, ceiling (courier) / no-ceiling (owner), evict-not-close | **CARRY behavior + collapse to ONE `RelayGuard<Policy>`** (Q2); the ESLint drift rule → module visibility |
| Q-WS-EVICT-FRAME | eviction hides in generic `{type:'error', error}` (`websocket.ts:260,280`); FE cannot distinguish binding-vs-membership | **CARRY the wire shape during overlap; normalize to `Event::Evicted{reason}` post-cutover FE-lockstep** (Q6/Q7) |
| Q-WS-LISTEN-SESSION | `PgMessageBus` LISTEN + NOTIFY on the **session** pool (5432), never the tx pooler (6543) (`message-bus.ts:25-28,116-131`) | **CARRY verbatim + GUARDRAIL** — assert the `PgListener` DSN is `DATABASE_URL_SESSION` (Q4); silent-no-fanout otherwise |
| Q-WS-RECONNECT | indefinite capped-backoff reconnect + re-LISTEN all channels; single `reconnecting` guard (`message-bus.ts:95-114`) | **CARRY verbatim** — the old 5-attempt cap left a machine "alive but realtime-dead" |
| Q-WS-CLAIMCHECK | payload > ~7800 B slimmed to `{_truncated,type,data.id}` (NOTIFY 8000 B cap) (`message-bus.ts:22-23,140-154`) | **CARRY behavior + FIX-shape** — first-class `Event::Resync{entity,id}` instead of an accidental-refetch heuristic |
| Q-WS-NOTIFY-FAF | NOTIFY is fire-and-forget **after** COMMIT, no outbox, no replay | **CARRY + accepted-risk FLAG** (WS-T6) — outbox is a future hardening; the seam is the explicit `Resync` (§6/§11) |
| Q-WS-HANDLER-ISO | a throwing subscriber is swallowed per-broadcast (`message-bus.ts:168-181`) | **CARRY verbatim** — a bad fan-out degrades one broadcast, never the process (historical crash) |
| Q-WS-ENVELOPE | `{room, data: msg}` envelope + two-shape `order.status` (flat vs `data`-wrapped) (`websocket.ts:218`, inventory 10 §3) | **CARRY the wire verbatim during overlap; normalize to one shape post-cutover FE-lockstep** (Q6) — golden-frame parity gate |
| Q-WS-19-UNHANDLED | 19 published-but-unhandled event types (packet.md §3 dispositions) | **CARRY-PUBLISH during overlap; apply CONSUME/STOP-PUBLISH/RETIRE post-cutover** (Q7); contest rows 8 (`task_offered`) + 16 (`gdpr.erasure_completed`) explicitly |
| Q-WS-SHIFT-ROOM | `courier:<id>:shift` room + `shift.opened` event — **zero FE subscribers** (grep, inventory 10 §3) | **RETIRE post-cutover** (rooms 5→4) — port dark during overlap; a resurrected widget adds a typed room later (Q7) |
| Q-WS-EXPIRY | nothing re-checks `exp` after admission; expired token → infinite 1008 reconnect (linkage A3/A7) | **FIX-IN-PORT 🔴** — connection-level `TokenExpiring`→grace→distinct 4401 close; anonymous → terminal close→REST-poll (Q8) |
| Q-WS-HEARTBEAT | ping 30s, terminate on one missed round-trip; room GC eager on last-leave + sweep (P1-WSDUP) | **CARRY verbatim** — liveness + exactly-one-subscription-per-room |
| Q-WS-ORIGIN | no Origin check, no upgrade rate limit (`websocket.ts:192`, linkage A8) | **FIX-IN-PORT** — Origin allow-list (CSWSH belt) + per-IP upgrade rate limit (Q9) |
| Q-WS-GDPR-SOCKET | erasure evicts no subscribed customer socket, invalidates no grant (linkage A2) | **FIX-IN-PORT** — erasure publishes an eviction for `order:<id>` customer members + deletes the order's track grants (Q10; aligns S9 cluster) |
| Q-WS-DOS-KEY | `dos_access_token` FE key collision — customer checkout overwrites owner/courier session (linkage A6) | **FIX on Node PRE-S6** — FE-only key split; a live defect independent of the rebuild (Q-cross-ref, not an S6 build item) |
| Q-WS-CHANNEL-HEAD | heads authenticate as channel principals, never as tenant/customer (REBUILD-MAP §6 inv 2) | **CARRY the type seam (`Principal::Channel`), NO WS runtime for heads in v1** (Q11); a later grant is a new channel-scoped room, never customer/owner rooms |

## 13. Cutover DoD (REBUILD-MAP §3, this surface)

- **Realtime E2E slice green** against the Rust WS behind the proxy: `courier-room-authz-isolation.spec.ts`,
  `cross-tenant-realtime-qa.spec.ts` (3 roles + WS + isolation), flow-core-lifecycle WS assertions.
- **Invariant-cluster red→green:** tri-state verdicts (12/12 vs `courier-room-authz.test.ts`);
  relay-guard TTL/ceiling/no-refresh-on-read/inflight-dedup (both policies); owner `#4` revocation
  eviction; courier binding eviction; claim-check truncation → `Resync`; the **NEW** courier
  WS-session-liveness (revoke session → next WS reconnect denied).
- **Cross-tenant proof:** a message published to tenant A's `order:`/`location:` room is asserted
  **never delivered** to a tenant-B socket — at admission AND after a mid-stream binding/membership
  revocation.
- **Transport:** subprotocol auth succeeds; `?token=` flag'd dual-accept succeeds; `sec-websocket-protocol`
  redacted in logs; the deprecation counter observable.
- **PgListener:** a config guardrail asserts the session DSN; a reconnect test (drop the listener conn →
  re-LISTEN → resume fan-out); a degraded-health signal fires on listener-down **<1 min**.
- **Wire parity (the named cutover gate):** golden-frame parity — a fixed event → exact Node JSON, both
  directions; a client reconnecting mid-session from Node→Rust (and Rust→Node) parses every frame.
- **Cutover-concurrency probe:** both stacks LISTEN concurrently on `order:<id>`; a Node-published
  NOTIFY reaches a Rust socket and a Node socket, each exactly once; a drain (stop-new + reconnect)
  lands sockets on Rust with in-flight delivery state intact (via REST refetch); session-mode
  connection count monitored under the ceiling.
- **Lifecycle:** expiry → `TokenExpiring` → grace → 4401 (no infinite reconnect); anonymous → terminal
  close → REST-poll; heartbeat zombie terminate; room GC exactly-one-subscription.
- map-coverage zero-diff for the WS event/room census (old grep vs `protocol.rs` enum) ·
  **council sign-off + rollback plan** (proxy `WS_SURFACE` flip back to Node + mass-reconnect;
  time-boxed overlap). **No 🔴 S6 row builds before this packet is APPROVED and the 🔴 questions
  (Q1/Q2/Q4/Q5/Q6/Q8) are operator-signed.**

---

**council seats to run: breaker, counsel** (architect authored; human operator signs 🔴 Q1 WS
admission+token / Q2 fan-out cross-tenant / Q4 PgListener session-conn+NOTIFY / Q5 cutover concurrency /
Q6 wire-parity / Q8 lifecycle expiry).
**packet-status: 🟡 DRAFT.**
