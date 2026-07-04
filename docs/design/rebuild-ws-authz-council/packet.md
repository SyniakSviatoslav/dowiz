# WS-authz council packet — S6 realtime port (axum-WS + PgListener)

**Status: DRAFT — NOT APPROVED. Pre-council packet per REBUILD-MAP Phase-0 item 5. No 🔴 row builds until this council is APPROVED.**
**Date:** 2026-07-04 · **Lane:** R5 · **Covers:** REBUILD-MAP §1 WS row (5 in / 24 out / 5 rooms, 33 publish sites), §3 S6 🔴, §6 invariant 2 (channel principals), §7 open item 1 (linkage — see `linkage-analysis.md` in this dir).
**Ground truth:** `docs/design/rebuild-plan/inventory/10-api-realtime-jobs.md` §§2-6, `inventory/14-crosscutting-proofnet.md` §6, ADR-0013 (+ addendum DRAFT in `docs/design/ws-token-in-url/`), `docs/adr/ADR-security-hardening-2026-07.md` (#4/#5), `docs/security/hardening-findings-2026-07-02.md`.

---

## 1. Scope and non-goals

**In scope (S6):** the WS protocol (5 inbound msg types, 24 outbound event types, 5 room kinds),
Pg LISTEN/NOTIFY fan-out (claim-check, reconnect), the three authz layers (admission, per-frame
relay guards, protocol control frames), connection lifecycle, and the customer-JWT linkage fixes
this council must decide (linkage-analysis A1-A8).
**Out of scope:** the mint sites themselves (S2 auth council owns owner/courier mints; the customer
mint tuple stays as-is unless Q1/Q3 decide otherwise), the internal `BUS_CHANNELS.*` 34-topic
namespace (jobs/notifications lane — never browser-reachable; must NOT be conflated with WS rooms),
money/state-machine semantics of the events (S5 council).

## 2. Target architecture (per REBUILD-MAP decision register)

```
crates/api/src/ws/
  mod.rs        — axum upgrade handler (single mounted path, e.g. GET /ws)
  auth.rs       — upgrade-time authn (Sec-WebSocket-Protocol bearer.v1; ?token= behind flag)
  protocol.rs   — ClientMsg (inbound) + Event (outbound) — the ONE enum pair, serde-exhaustive
  rooms.rs      — typed Room enum + registry (DashMap<Room, RoomState>), per-room broadcast
  authz.rs      — Verdict{Allow|Deny|Unavailable} + per-principal room predicates (tri-state)
  guard.rs      — per-frame relay revalidation (courier binding / owner membership), TTL cache
  pg_fanout.rs  — PgListener (dedicated conn, NOT via Supavisor tx-pooling) → Event decode → rooms
```

- **PgListener** replaces `PgMessageBus`: one dedicated direct connection, `LISTEN` per active
  room-channel, reconnect with capped backoff retried indefinitely + re-LISTEN (parity with
  `message-bus.ts:80-114`); NOTIFY stays on the pool. 8KB claim-check preserved: publisher slims
  frames > ~7800B to `{_truncated, type, id}` — but in Rust the truncated frame becomes an explicit
  `Event::Resync { entity, id }` variant instead of an accidental-refetch contract (inventory 10 §5).
- **Publish sites:** all 33 `messageBus.publish(...)` room-channel sites map to one
  `publish(room: Room, event: Event)` function — serde serialization is the only wire encoder, so an
  ad-hoc frame shape is unrepresentable (same move as the error-enum, REBUILD-MAP §1 row 16).

## 3. ONE event enum — the 24 outbound types + the 19-unhandled disposition table

`protocol::Event` is exhaustive: every variant is either consumed by a named FE surface or does not
exist. The 19 published-but-unhandled types (14 §6 census) each get a council disposition — **never
port blind**:

| # | Type | Today | Proposed disposition (DRAFT) |
|---|---|---|---|
| 1 | `assignment.created` | dashboard room, no FE handler | **CONSUME** — admin dashboard assignment list should go live-fresh (currently REST-poll compensates) |
| 2 | `binding_changed` | order room, no consumer (deliberately courierId-free — ledger row 40) | **KEEP-PUBLISH** as a cache-invalidation signal; in Rust it justifies the relay-guard TTL; document as protocol-internal, not FE-facing |
| 3 | `assignment_aborted` | dashboard, unhandled | **CONSUME** (dashboard freshness) |
| 4 | `assignment_expired` | dashboard, unhandled | **CONSUME** (dashboard freshness) |
| 5 | `offer_sent` | dashboard, unhandled | **CONSUME** (offer lifecycle visibility) |
| 6 | `offer_expired` | dashboard, unhandled | **CONSUME** |
| 7 | `offer_declined` | dashboard, unhandled | **CONSUME** |
| 8 | `task_offered` | courier room, unhandled (TasksPage handles only `task_assigned`) | **CONSUME** in courier task inbox — the offer-handshake exists in REST; WS parity closes a poll gap |
| 9 | `shift.opened` | `courier:<id>:shift` room — **zero FE subscribers, dead-on-the-wire** | **RETIRE** the event AND the 5th room kind (proof: inventory 10 §3 grep) — drops rooms 5→4 |
| 10 | `preflight.signal_raised` | dashboard, unhandled | **CONSUME** (owner sees risk signals live) |
| 11 | `preflight.signal_acknowledged` | dashboard, unhandled | **STOP-PUBLISH** (ack round-trip is same-client; REST response suffices) unless multi-device owner demand exists |
| 12 | `preflight.signal_dismissed` | dashboard, unhandled | **STOP-PUBLISH** (same rationale as 11) |
| 13 | `dwell.alert_created` | dashboard, unhandled | **CONSUME** (alert banner is the stated intent) |
| 14 | `dwell.alert_acknowledged` | dashboard, unhandled | **STOP-PUBLISH** (same-client ack) |
| 15 | `dwell.escalation_tier_changed` | dashboard, unhandled | **CONSUME** (tier change is owner-actionable) |
| 16 | `gdpr.erasure_completed` | dashboard, unhandled | **STOP-PUBLISH** on WS; keep the compliance record in DB/notifications lane (erasure is not a live-dashboard concern) |
| 17 | `customer.contact_revealed` | dashboard, unhandled (raw-string channel construction) | **STOP-PUBLISH** on WS; it is an audit event — belongs in the audit log, not a broadcast |
| 18 | `courier.assignment_status_changed` | couriers room, unhandled (live-map badge intent) | **CONSUME** (admin live map) |
| 19 | `order.picked_up` | unhandled | **RETIRE** — superseded by `order.status` (status enum carries picked_up); one canonical status event |

Also normalized in the enum: `order.status` today has **two shapes for one type string** (flat on
`order:<id>`, `data`-wrapped on dashboard — inventory 10 §3). Rust: ONE
`Event::OrderStatus{..}` envelope; the room determines routing, never the shape. The two synthetic
eviction notices become first-class: `Event::Evicted { reason: BindingRevoked | MembershipRevoked }`
(today both hide inside generic `{type:'error'}` and the FE cannot distinguish them).

Inbound is the same move: `ClientMsg::{Auth{token}, Subscribe{room}, Unsubscribe{room},
ClientLocation{lat,lng}, ClientLocationStop}` — 5 variants, serde-exhaustive, unknown type = protocol
error frame (today: silent `console.warn`).

## 4. Room / channel authz model

**Typed rooms** (kills string-parsing drift — `room.split(':')[1]` suffix-agnosticism noted in
inventory 10 §4 becomes unrepresentable):

```rust
enum Room {
  Order(OrderId),
  LocationDashboard(LocationId),
  LocationCouriers(LocationId),
  Courier(CourierSub),
  // CourierShift(CourierSub) — RETIRE per §3 row 9 (council to confirm)
}
```

**Principals** (axum claims-extractor type-state, per REBUILD-MAP §1 auth row):

```rust
enum Principal {
  Owner    { user_id: UserId },
  Courier  { sub: CourierSub, active_location_id: LocationId, jti: SessionId },
  Customer { order_id: OrderId, location_id: LocationId, sub: CustomerId },
  Channel  { channel_id: ChannelId, scopes: ChannelScopes },   // §6 invariant 2 — see below
}
```

**Admission verdicts** (tri-state everywhere, ADR-0013 parity):

| Principal | Room | Predicate | DB? |
|---|---|---|---|
| Customer | `Order(o)` | `o == claim.order_id` exact | no (see Q1/Q2 for liveness) |
| Customer | anything else | DENY | no |
| Owner | `LocationDashboard(l)` / `LocationCouriers(l)` | live active owner membership on `l` (`ownerRoomVerdict` parity, incl. `status='active'` — the #4 fix) | yes |
| Owner | `Order(o)` | order→location JOIN live active membership (ADR-0004 shape) | yes |
| Courier | `Courier(s)` | `s == claim.sub` exact | no |
| Courier | `Order(o)` | live `courier_assignments` binding, `BINDING_READ_STATUSES = [offered, assigned, accepted, picked_up]`, inside a txn that sets the tenant GUC (NOBYPASSRLS-sound — `courier-room-authz.ts:32-66` parity) | yes |
| Courier | anything else (incl. `Location*`) | DENY, zero DB | no |
| Channel | — | **no WS rooms in v1** (recommendation; Q7) | — |

Tri-state mapping at subscribe (parity, Breaker H1/NEW-A): `Unavailable` → retryable soft error
frame, socket stays open, never close — a pool blip must not fleet-deny; `Deny` → `Forbidden room`
frame, no close. REST wrappers collapse `Unavailable` → fail-closed 404/503 (WS-only retryable
distinction is intentional and must be preserved).

**Fan-out re-authz (ADR-0013 + #4 parity — the "admission isn't enough" half):** ONE generic
`RelayGuard` in `guard.rs`, instantiated twice with role-specific config (today's two hand-written
guards share their entire shape):
- relay-only-on-fresh-ALLOW; absolute TTL (~10s), **no refresh-on-read** (flap can't keep a revoked
  entry warm); LRU-bounded cache; inflight dedup.
- `Deny` → evict from room + `Event::Evicted{reason}` (never socket-close).
- `Unavailable` → withhold; courier config adds the **in-memory 60s wall-clock ceiling / count
  backstop** (holds under total DB starvation); owner config withholds without ceiling (OR-9
  residual, stated honestly: revoked principal stops streaming within ≤TTL, not zero).
- Customers: relayed directly (admission authoritative) — **unless Q2 adds exp enforcement**, which
  is connection-level, not per-frame.
- **Drift guardrail parity:** the `local/no-raw-courier-ws-send` ESLint rule's intent ports as
  module-visibility: `rooms.rs` exposes NO raw send; the guard is the only path that can write to a
  member socket on fan-out. The rule's job becomes a compile-time property.

**Per-head channel-principal rule (REBUILD-MAP §6 invariant 2):** heads (TMA, conversational, feed,
agentic/MCP) authenticate as **channel principals with minimal scope, never as the tenant** and never
as a customer. Consequences for this council:
1. `Principal::Channel` exists in the type from day one so a head can never be admitted "as owner".
2. Every head that ever wants realtime gets **its own authz council row** (per invariant 2 wording);
   default v1 = no WS for heads — catalog reads are cacheable, order intake is REST + cart token
   (invariant 1), and status delivery to a chat head goes through the notifications lane, not a
   browser room.
3. If a head is later granted a room, it is a NEW room kind with channel-scoped predicate
   (e.g. `ChannelOrder(channel_id, order_id)` where the order's `sales_channel` must match) — never
   admission into customer/owner rooms.

## 5. Connection lifecycle

**Authn at upgrade:**
- Primary: `Sec-WebSocket-Protocol: bearer.v1, <jwt>` — server validates, echoes `bearer.v1` only
  (never the token). Adopts `docs/design/ws-token-in-url/ADR-0013-addendum-DRAFT.md` (option b —
  zero extra round-trips, keeps the 5s budget, token never in URL). **That addendum is itself gated
  on operator approval — this council should co-approve or explicitly sequence it.**
- Transition: `?token=` dual-accept behind `WS_URL_TOKEN_ACCEPT` (default ON at S6 cutover for
  cached-client parity, driven to OFF via the existing deprecation counter; the SPA client must stop
  setting `?token=` in the same release — today it sets BOTH, linkage A4).
- In-band `ClientMsg::Auth{token}` retained (5s deadline, close 1008 on miss) — it is the documented
  fallback and the churn-test contract.
- Upgrade discipline (linkage A8): single mounted axum route (unknown paths 404 before upgrade),
  explicit Origin allow-list check at upgrade (browsers only; heads don't get WS in v1), and a
  per-IP upgrade rate limit (tower-governor) — all absent today.

**Re-auth / expiry policy (closes linkage A3/§4 — decision Q2):** recommended default:
- Track `min(exp)` per connection; at `exp` (+ ≤60s jitter to avoid thundering herd) send
  `Event::TokenExpiring`, allow one in-band `Auth` refresh within a grace window, then close with a
  **distinct close code** (e.g. 4401) that clients treat as "re-authenticate, don't blind-retry" —
  kills the infinite 1008 reconnect loop class (A3, A7).
- Customer FE pairs this with silent re-exchange via the 14d track grant where a `?t=` code is at
  hand; otherwise surfaces "reopen your tracking link".

**Backpressure:** per-connection bounded send queue (tokio `mpsc` + `broadcast` lag detection). On
overflow/lag: drop the member's queued frames and send `Event::Resync{...}` (client refetches via
REST — the documented recovery mechanism; there is deliberately NO server-side replay/backlog, parity
with inventory 10 §6). Slow consumer that never drains → close 1013. Heartbeat: server ping 30s,
terminate after one missed round-trip (parity). Room GC: eager on last-leave + periodic sweep
(P1-WSDUP parity — exactly one fan-out subscription per room, re-subscribe races covered by tests).

**Shutdown:** drain order per inventory 10 §5 — stop accepting upgrades → close sockets with 1001 →
unsubscribe listener → release Pg conn.

## 6. Cutover DoD hooks (REBUILD-MAP §3 / 14 §8)

- E2E slice as-is: `courier-room-authz-isolation.spec.ts`, `cross-tenant-realtime-qa.spec.ts`,
  flow-core-lifecycles WS assertions — green against the Rust WS behind the proxy.
- Invariant-cluster unit tests red→green: tri-state verdicts (12/12 parity with
  `courier-room-authz.test.ts`), relay-guard TTL/ceiling/no-refresh-on-read, owner-guard #4
  revocation, eviction frames, claim-check truncation → `Resync`.
- Map-coverage extractor: WS-types set-diff (old grep census vs `protocol.rs` enum) = zero
  UNMAPPED/UNBUILT/ORPHAN; the 19 dispositions land as matrix rows (CONSUME → FE work item,
  STOP-PUBLISH/RETIRE → RETIRE row with proof).
- Regression-ledger rows re-proven on the new stack: #40 (fan-out guard), #42 (URL-secret
  redaction — port `redactUrlSecrets` semantics to the tower-http trace layer), P1-WSDUP.
- Rollback plan: WS traffic is proxy-switchable per-surface; Node WS stays warm one release.

## 7. FIX-IN-PORT vs CARRY-VERBATIM register (this surface)

| Item | Disposition (DRAFT) | Rationale |
|---|---|---|
| Customer REST scoping drift (linkage A1) | **FIX-IN-PORT 🔴** candidate — needs this council + documented E2E delta | authz narrowing; carrying verbatim reproduces a cross-order-cancel authority broader than the minted tuple |
| No exp re-check post-admission (A3) | **FIX-IN-PORT** via lifecycle policy Q2 | correctness/security; today's behavior is an accident, not a contract |
| `?token=` transport (A4) | FIX-IN-PORT per addendum (gated) | already council-designed; S6 is the natural cutover point |
| SSR `place-order.ts` dead session store + `StatusWSClient` (A5) | **RETIRE with proof** | zero importers / wrong field names; do not port |
| `dos_access_token` key collision (A6) | FIX at S1/S6 boundary (FE-only) | one-line key split; not an authz change |
| Anonymous-order reconnect loop (A7) | FIX-IN-PORT via distinct close code (Q2/Q8) | client-visible behavior change, small E2E delta |
| Two-shape `order.status`, generic `error` eviction frames | FIX-IN-PORT (enum makes old shapes unrepresentable) | FE consumers updated in the same surface cutover |
| Tri-state semantics, TTL/ceiling values, no-replay/refetch recovery, heartbeat cadence, claim-check threshold | **CARRY-VERBATIM** | breaker-reviewed, ledger-protected behavior; the parity oracle depends on it |
