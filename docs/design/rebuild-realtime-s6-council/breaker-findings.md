# S6-REALTIME/WS Council Packet — BREAKER FINDINGS

> Adversarial pass by System Breaker DeliveryOS against `proposal.md` + `open-questions.md` +
> `threat-model.md` + the folded-in `docs/design/rebuild-ws-authz-council/`. Ground truth:
> `apps/api/src/websocket.ts`, `apps/api/src/lib/courier-relay-guard.ts` + `courier-room-authz.ts`,
> `packages/platform/src/message-bus.ts`, `packages/db/src/index.ts`,
> `rebuild/crates/api/src/auth/extractors.rs`, `docs/design/rebuild-cutover-harness/matcher/cutover-matcher.ts`.
> **No fixes proposed — that is the architect's job.** Each row: `[SEV] vector · finding · break/number · violated invariant`.
> Read-only verification only; nothing in product changed.

**Severity counts: 1 CRITICAL · 3 HIGH · 6 MEDIUM · 1 LOW.**

---

## CRITICAL

### C1 — [B-OPS / B-FAIL] The pooler-block silent-blackout is UNDETECTABLE by the proposed health signal (false-green on the one failure mode the surface is named for)
- **Finding.** The packet's detection for the silent-no-fanout class (§11, WS-T5, Q4) is: (a) a config
  guardrail that "asserts the listener DSN is `DATABASE_URL_SESSION`", and (b) a degraded-health signal
  where **"degraded = listener disconnected/reconnecting"** (`proposal.md:428-433`). Ground truth of the
  ported health model: `message-bus.ts:238` `checkHealth()` returns `isDegraded ? 'degraded' : 'ok'`, and
  `isDegraded` is set **true only** in the connect-catch (`:91`), the listener `'error'` handler (`:69`),
  and the `'end'` handler (`:75`). It is reset **false** on a successful `connect()` (`:44`).
- **Break scenario (demonstrable).** Connect the `PgListener` on a transaction-pooler endpoint (the
  HIGH-1 misconfig: `DATABASE_URL_SESSION` accidentally points at `:6543`, or a Supavisor/pgbouncer in tx
  mode). `pool.connect()` **succeeds**, `LISTEN "order:<id>"` **returns success** (it is a valid
  statement — the tx-pooler simply returns the backend to the pool afterward, orphaning the registration).
  There is **no `'error'`, no `'end'`, no connect-catch** — the socket object stays "open" from
  node-pg's view. Therefore `isDegraded` stays `false` → `checkHealth()` returns **`'ok'` forever** while
  **zero** NOTIFYs ever arrive. The Rust port that mirrors this health model (`proposal.md:428` "parity
  with `message-bus.ts:237-239`") inherits the identical blind spot: the proposed detector observes
  *disconnect*, but the actual failure is *connected-but-mute* — orthogonal signals.
- **Why the DoD tests false-green too.** The packet's proofs (`proposal.md:496-497`, §13) are: a config
  guardrail (a DSN *identity/name* check — passes for a mislabeled `DATABASE_URL_SESSION`, it never does a
  NOTIFY round-trip) and "a reconnect test (drop the listener conn → re-LISTEN → resume fan-out)" (uses a
  *working* connection). Nothing in the DoD issues a NOTIFY and asserts a frame arrives over the listener's
  own connection. The invariant-cluster unit tests poll DB state directly. Net: **green build, green
  guardrail, green health, green tests — and total, all-tenant realtime blackout in production.**
- **Number / blast radius.** Every one of the 24 outbound event types over every room silently delivers
  0 frames: couriers receive no offers (deliveries stall), owner dashboards freeze, customers get no
  status. Undetected **past the stated `<1 min` SLO indefinitely** — recovery is a human noticing "realtime
  feels dead," not a signal.
- **Violated invariant.** `proposal.md:431-433` / threat-model WS-T5: *"a degraded listener must surface
  `<1 min`… never a binary up/down — a degraded listener serves HTTP fine while realtime is dead, the exact
  trap."* The proposed mechanism cannot surface the connected-but-mute degradation **at all**; the packet
  builds the mitigation on a detector aimed at the wrong event.

---

## HIGH

### H1 — [B-SEC / B-CONSIST] Courier session-liveness fix is ADMISSION-ONLY; the per-frame guard re-reads *binding*, never *session* → a logged-out / kicked courier keeps a live tail (customer GPS + order feed) for up to 14 days
- **Finding.** The packet's own NEW finding (Q1b / WS-T8) is closed by "reuse the S2 `CourierSession`
  bind at upgrade" (`proposal.md:216-226`), claimed as **"parity with REST"**. But `CourierSession`
  (`extractors.rs:160-208`) performs the `courier_sessions` liveness bind **once, at extraction/upgrade**.
  The per-frame fan-out re-authz — the whole point of ADR-0013 — runs `courierReadVerdict`
  (`websocket.ts:248`, `courier-room-authz.ts:69-72`) which queries **`courier_assignments` only**. Grep
  confirms `courier_sessions` is **never** touched by any relay/websocket per-frame path (empty result over
  `apps/api/src/lib/` + `websocket.ts`).
- **Break scenario.** A courier authenticates and subscribes `order:<O>` (holds a live binding). The
  courier then **logs out** (or an admin revokes the stolen/fired courier's `courier_sessions` row) but
  **keeps the WS socket open**. REST dies instantly (`plugins/auth.ts:60-93` → 401). WS: the relay guard
  re-checks the binding, which is **still ALLOW** (logout does not drop the assignment), so
  `client_location` (live GPS of a real person, asset W4) and every `order.status` frame **keep streaming**.
  Nothing re-checks `exp` after admission either (linkage A3) — so the tail persists until the socket drops
  or the 14d token `exp` finally trips Q8's policy.
- **Why the claim is false.** REST checks `courier_sessions` on **every request**; the proposed WS fix
  checks it on **upgrade only**. That is the exact asymmetry ADR-0013 was written to kill for *binding*
  (admission gates only NEW subscribes; C1 rides the fan-out) — reproduced here for *session*. Bind-at-
  upgrade closes the *reconnect-denial* case (a fresh socket is denied), not the *mid-stream-revocation*
  case, so it is **not** parity with REST.
- **Violated invariant.** threat-model W3 / OR-9: *"a revoked principal must stop streaming within ≤TTL."*
  For session-revocation the stream never stops within ≤TTL because the per-frame source is the binding,
  not the session. `proposal.md:222` "a WS courier admission IS a live-session check — parity with REST" is
  overstated: it is a live-session check *at admission*, not per-frame.

### H2 — [B-CONSIST / B-ANTIPATTERN] "Exhaustive typed serde Event" (§3) and "carry every wire shape byte-verbatim" (§7/Q6) are mutually exclusive — the named cutover parity gate is unachievable as specified, and a cross-stack reconnect silently drops fields
- **Finding.** The Node fan-out is an **opaque passthrough**: `websocket.ts:218`
  `JSON.stringify({ room, data: msg })` where `msg = JSON.parse(notify.payload)` — whatever the producer
  emitted flows through untouched. The Rust design (`proposal.md:174,326`) says `protocol::Event` is
  *"exhaustive (24 outbound types)… serde is the only wire encoder, so an ad-hoc frame is unrepresentable."*
  To fan out, Rust must **decode** the NOTIFY payload into a typed `Event`, then **re-encode** into
  `{room, data:<event>}`. §7/Q6 simultaneously demands the re-encode be **byte/shape-identical** to Node's
  passthrough (`proposal.md:335-339`, the golden-frame gate, "co-equal with the authz tests").
- **Break scenario (three concrete drift classes).**
  1. **Unmodeled-field drop.** Any field a producer puts in a NOTIFY that the `Event` variant does not
     declare is **dropped on decode** (serde default) → absent on re-encode. Node passes it through; Rust
     eats it. A client on Node sees `data.foo`, reconnects to Rust, `data.foo` is gone. The 19
     published-but-unhandled types (`proposal.md:474`, deferred/"carried") are the highest-risk carriers —
     "carry" them opaquely on Node vs type them on Rust cannot both hold.
  2. **Key-order drift.** `JSON.parse→JSON.stringify` in V8 preserves the producer's source key order;
     serde emits **struct-declaration order**. Even a fully-modeled event byte-differs.
  3. **Number reformat.** A customer `client_location` of `{lat:52,lng:13}` (integers) re-emits as `52`
     from Node but `52.0` from a Rust `f64` (serde_json) — a byte mismatch the golden gate flags.
- **Number.** The golden-frame test (`proposal.md:498-499`, "exact Node JSON, both directions") will
  **fail on benign reformat/reorder** → the team either burns cycles chasing byte-equality that a typed
  encoder cannot deliver, OR relaxes the gate to shape-equality and then **misses the real field-drop
  drift** (class 1) — the exact S5 request-hash failure mode the packet invokes as precedent.
- **Violated invariant.** threat-model W7 / TB-5 / WS-T9: *"a client reconnecting mid-session onto the
  other stack must parse identical frames."* The two stated design properties (typed-exhaustive vs
  byte-verbatim) contradict; the packet does not resolve which wins, so the co-equal cutover gate rests on
  an impossibility.

### H3 — [B-SCALE / B-FAIL] The session-mode connection ceiling — named as THE scaling gate — is asserted with NO number, while production already draws ~9+ session backends before the net-new Rust listener
- **Finding.** `proposal.md:132-139` names the SESSION-mode (5432) budget as *"the real ceiling"* and
  *"the S6 scaling gate,"* then instructs *"budget it against the Supavisor session-mode ceiling"* — but
  gives **neither the ceiling nor the current draw**. Breaker rule: a scaling claim without a number is
  rejected.
- **Number (the packet omits).** `createSessionPool()` sets **`max: 3`** (`db/index.ts:49-50`).
  Production session-mode consumers (non-test, non-node_modules): the message bus
  (`server.ts:222-223`), the worker (`apps/worker/src/index.ts:10`), and the free-tier-watch worker
  (`workers/free-tier-watch.ts:40-41`) — each a `createSessionPool` × max 3 ≈ **9 session backends**,
  plus migrations' `release_command` (transient). The Rust `PgListener` adds **1 NET-NEW** for the life of
  the overlap. Supavisor session mode is **non-multiplexed** (every backend is a real Postgres process) and
  its default pool is small (single-digit to low-double-digit on typical Supabase tiers). The packet never
  states which — so "+1 listener, budget it" is **un-verifiable and may already be at/over the ceiling.**
- **Break / caskade into C1.** If the session ceiling is exceeded, the **listener's own** `connect()`
  fails → `isDegraded=true` at first (reconnect loop), but once it *does* land a connection during a lull
  and the pool later starves it silently, you are back in the **C1 connected-but-mute blackout** — the two
  findings compound.
- **Violated invariant.** B-SCALE: back-of-envelope must close on the target resource. `proposal.md:139`
  defers the only computation that matters (does listener +1 fit the ceiling?) to prose.

---

## MEDIUM

### M1 — [B-OPS / B-FAIL] The cutover matcher's `isWebSocketUpgrade` keys on `Upgrade: websocket` ALONE — it diverges from Node's actual upgrade dispatch (`Connection: Upgrade`), so a crafted/mangled request mis-routes across the stack boundary during overlap
- **Finding.** `cutover-matcher.ts:163-167` decides S6 iff `headers['upgrade'].toLowerCase()==='websocket'`,
  evaluated **before path/method** (`:180-190`). Its comment claims this *"matches the real `ws` library's
  behavior"* (`:170-172`) and `proposal.md:246-257` leans on it as a *"security-narrowing FIX with a tiny,
  safe delta… carry the header-first steering unchanged."* But Node's HTTP server emits the `'upgrade'`
  event (what `ws` hooks) on the **`Connection: Upgrade`** token, **not** on the `Upgrade` header value.
- **Break scenario.** Send `POST /api/orders` with header `Upgrade: websocket` and **no**
  `Connection: Upgrade`. On the **real server** Node does not fire `'upgrade'` → the request reaches
  Fastify as an ordinary POST → **an order is created.** Through the matcher, `isWebSocketUpgrade` returns
  **true** → the request is steered to **S6** (the WS surface) → the order is **not** created (handshake
  fails / wrong stack). The matcher is therefore **looser and divergent** from the server it claims to
  mirror; an intermediary or client library that opportunistically adds `Upgrade` mis-routes state-changing
  REST calls during the overlap, or can be used to force a request onto whichever stack lacks a given
  path-scoped control.
- **Violated invariant.** cutover parity (surprise #7): the matcher must mirror the real `ws` acceptance
  condition. `cutover-matcher.ts:185-188` asserts a fidelity it does not have; the packet's "safe delta"
  claim rests on it.

### M2 — [B-SEC] The JWT-in-URL deprecation is unachievable-by-design: the counter that gates `?token=` removal is pinned above zero by exactly the cached SW/PWA clients dual-accept exists to serve
- **Finding.** Transport plan (`proposal.md:195-201`, Q1a-b): `?token=` dual-accept behind
  `WS_URL_TOKEN_ACCEPT` **default ON**; removal = flag flip *after the deprecation counter hits zero*
  (`websocket.ts:179-181`). But the SPA today sets `?token=` **and** sends `{type:'auth'}`
  (linkage A4, `useWebSocket.ts:50,61`), and the packet itself notes *"the counter can never hit zero while
  the client sets the param"* (`proposal.md:199-201`). The new SPA release stops setting it — but the
  **cached service-worker/PWA clients (the entire reason dual-accept exists)** keep running the **old** SPA
  from the SW cache until the SW updates, which can be indefinite.
- **Break / number.** For the full SW-cache tail, the URL token stays in `req.url` → exposed via browser
  history/console/Referer/any intermediary that logs URLs **before** our redactor (`redactUrlSecrets`,
  ledger #42, which by design **cannot reach** those vectors — linkage A4 "Residual"). So the leak the
  subprotocol switch was meant to close (WS-T4) is **permanent, not transitional**, and the removal
  mechanism can provably never fire. Labeling it "deprecation window" understates an open-ended residual.
- **Violated invariant.** threat-model W5/WS-T4: the token must leave the URL. The plan keeps it in the URL
  for an unbounded population and provides no exit condition that population can satisfy.

### M3 — [B-SCALE] §2's back-of-envelope for the operational-pool authz draw is a WARM-cache steady-state number; the drain (cold cache) is a thundering herd whose only jitter lives in client code the cached clients don't run
- **Finding.** `proposal.md:130-131` argues the 6543 operational draw is *"O(1) re-reads/10s… negligible"* —
  true only when the relay-guard TTL caches are **warm**. At the cutover drain every socket reconnects with
  an **empty** guard cache: each re-subscribe is a cold authz read (`courierRoomVerdict` / `ownerCanAccessRoom`,
  a multi-statement `BEGIN…set_config…SELECT…COMMIT` for couriers, `courier-room-authz.ts:44-53`) **plus**
  the first fan-out frame is withheld → a second cold read (`courier-relay-guard.ts:127-129`,
  `websocket.ts:145-155`). ≈ **2 cold operational reads per reconnecting principal**, on a pool of
  `max: 20` (`db/index.ts:23`).
- **Break scenario.** The drain's storm mitigation is *"jittered client backoff… already `useWebSocket.ts`"*
  (`proposal.md:391-393`, WS-T11). But the clients most likely to storm are the **cached SW/PWA clients**
  (§M2) that run the **old** `useWebSocket.ts` — which may lack the jitter. A synchronized mass-close
  (explicit 1001) against un-jittered cached clients = hundreds of simultaneous cold re-subscribes
  contending with normal API traffic on the same 20-slot operational pool → `connectionTimeoutMillis: 5000`
  trips → `courierRoomVerdict` returns UNAVAILABLE → retryable soft-error → client re-subscribes →
  amplification.
- **Violated invariant.** B-SCALE: the drain's capacity number must model the cold-cache flip, not the
  warm steady state; the mitigation must exist in the client population that actually storms.

### M4 — [B-CONSIST] The owner relay guard drops the FIRST frame after every TTL expiry (and the first post-subscribe frame) with NO replay/Resync → sparse owner dashboards silently miss discrete `order.status` events
- **Finding.** Owner relay is withhold-then-revalidate (`websocket.ts:142-156`): a cache miss/stale entry
  → **withhold this frame** + async re-read; the withheld frame is **never re-delivered** (no replay). The
  subscribe gate (`websocket.ts:404`) does **not** warm the guard cache, so the **first fan-out frame after
  every subscribe** and **the first frame after each ~10s TTL expiry** (`createOwnerRelayGuard`, ttl 10s,
  `:104`) is dropped.
- **Break scenario.** Courier GPS is 1 Hz, so a dropped frame is self-healed by the next point ~1s later —
  fine. Owner dashboard events are **sparse and bursty**: if the single dropped frame is the
  `order.status: delivered` transition and no further event fires for minutes, the dashboard shows **stale
  state** with no correction. Unlike the claim-check path (which emits an explicit `Event::Resync`,
  `proposal.md:309-310`), a **withheld owner frame has no Resync/refetch trigger** — it is a silent loss.
- **Violated invariant.** consistency of the owner feed; `proposal.md:130` counts the re-read cost but does
  not surface that the triggering frame is discarded. Carried from #4 verbatim, unflagged.

### M5 — [B-CONSIST] "Each client is on exactly one stack → no double-delivery" is false during the non-atomic reconnect window; both stacks LISTEN the same channel and can deliver one NOTIFY twice
- **Finding.** `proposal.md:386-387` / Q5(b): during overlap both stacks LISTEN + fan out, and the claim is
  *"a client is on exactly one stack, so no double-delivery to a single client."* Reconnect is **client-
  driven and not atomic**: the FE opens the new socket (Rust) while the old socket (Node) is still open
  (draining, or TCP half-open, or the 1001 not yet processed).
- **Break scenario.** For the reconnect-transition window a client holds **two** sockets across **two**
  stacks. A NOTIFY on `order:<id>` fires; **both** stacks fan out; the client's message handler runs
  **twice** for the same event. Idempotent `order.status` set → benign; any non-idempotent FE handler
  (appended list item, incremented badge, GPS trail point) **double-applies**. The "exactly one stack"
  premise holds only *after* the handoff settles, not *during* it — which is precisely the flip the packet
  markets as gentle.
- **Violated invariant.** TB-5 exactly-once-ish delivery assumption at the stack boundary; the packet
  asserts single-stack membership as invariant when it is only eventually true.

### M6 — [B-SEC / B-ANTIPATTERN] `Principal::Channel` is added to the enum but has NO explicit subscribe verdict in the §4 admission table, and the reference subscribe handler has no default-DENY — the admission model is allow-list-by-role with no fail-closed floor
- **Finding.** The Rust `Principal` enum gains a 4th arm `Channel{…}` (`proposal.md:212`), and the
  admission table row for Channel is *"— | no WS rooms in v1 (Q11) | —"* (`proposal.md:239`) — a **dash**,
  not a concrete verdict. The Node reference it ports from (`websocket.ts:388-437`) is `if customer … else
  if owner … else if courier …` with **no `else` branch**: an unmatched role falls straight through to
  `subscribeToRoom` (`:435`) with **zero authz**. (Today unreachable — the verifier's `AuthToken`
  discriminated union yields only owner/courier/customer, `legacy.ts:163-174` — but the port explicitly
  widens the principal set.)
- **Break scenario.** A future channel-head grant, or any handler that constructs a `Channel` principal and
  reaches subscribe, hits an admission model with no fail-closed floor; if the port carries the reference's
  no-`else` structure (or writes `Channel => unreachable!()`), the outcome is a **fallthrough admit** or a
  panic, on the one surface whose whole reason to exist is cross-tenant isolation.
- **Violated invariant.** B-SEC default-deny: a 🔴 admission gate must fail closed for every principal,
  including ones with "no rooms in v1." The packet leaves Channel's subscribe verdict unspecified.

---

## LOW

### L1 — [B-FAIL] URL-path token verify failure is swallowed with no close, relying on the 5s auth timeout — the anonymous/expired-token reconnect-loop cost survives the overlap even under Q8
- **Finding.** `websocket.ts:352-354`: a failed `?token=` verify only `console.warn`s — no close, no signal
  — so the socket idles until the 5s `authTimeout` closes 1008 (`:357-362`), which the SPA treats as
  retryable → infinite reconnect (linkage A3/A7). Q8's `TokenExpiring`→4401 policy (`proposal.md:361-368`)
  addresses the **authenticated-then-expired** path, but a malformed/again-expired token arriving on the
  **URL path** during the dual-accept overlap still rides the swallow→timeout→retry loop.
- **Break / number.** Bounded server-side churn (a ~15s reconnect cadence per looping client, linkage A7),
  no data exposure. Minor, but it is a carried noise/DoS-lite cost the packet's lifecycle section does not
  fully retire while `?token=` dual-accept is ON.
- **Violated invariant.** lifecycle: an auth failure should yield a distinct, client-actionable close, not
  a silent timeout-retry — Q8 fixes one entry path, not the URL one during overlap.

---

## Regression check vs the folded-in `rebuild-ws-authz-council/`
- The folded packet's **A2 (customer irrevocable bearer)**, **A6 (`dos_access_token` collision)**, and
  **Q12 (GDPR socket eviction)** are carried into the S6 packet as acknowledged/owned residuals (Q10,
  Q-cross-ref) — no new attack beyond what is already flagged; **not re-raised.**
- **A1/Q3 (customer scoping drift across REST consumers)** is a **customer/REST-route** concern
  (`customer/orders.ts` authorizes any order of the same `sub`); it rides the S5/customer-auth surfaces,
  not S6 fan-out. `CustomerClaimsExt::require_order` (`extractors.rs:120-137`) is the structural fix on the
  REST side. Out of scope for this S6 breaker pass; **noted, not scored.**
- The Node raw-string `LISTEN "${channel}"` / `NOTIFY "${channel}"` interpolation
  (`message-bus.ts:125,194`) is a latent injection seam that the typed-`Room` port **closes** (parse/validate
  → unrepresentable). This is an improvement of the port, not a defect of the packet — **no finding.**
