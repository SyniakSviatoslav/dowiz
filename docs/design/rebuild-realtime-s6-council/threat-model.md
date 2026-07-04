# S6-REALTIME/WS Port — Council Packet · THREAT MODEL

> **STATUS: 🟡 DRAFT — NOT APPROVED.** Adversarial input to the S6 council. Assets, trust boundaries, and
> the failure modes the Rust port must not silently introduce — on the surface whose *whole purpose* is
> to keep one tenant's live order feed out of another tenant's browser, and whose cutover is the only one
> a **live courier crosses mid-delivery**. Read alongside `proposal.md` / `open-questions.md`. Docs only.

- **Method:** STRIDE-lite over the WS surface + fold-in of ADR-0013 (courier fan-out C1 leak), the `#4`
  owner-revocation gap (ADR-0004), the `#5`/JWT-in-URL finding (`security-sweep-findings-2026-07-02`),
  the customer-JWT↔WS linkage A1–A8 (`rebuild-ws-authz-council/linkage-analysis.md`), and the
  cutover-concurrency class unique to a strangler where a **long-lived stateful-looking connection**
  crosses the flip.
- **Scope note:** the B3 (NOBYPASSRLS) flip and the `app_member_location_ids()` search_path pin are
  **B3-council fixes**; the courier binding read is already GUC-seated + NOBYPASSRLS-sound
  (`courier-room-authz.ts:44-53`). The **token mint sites** (S2 owner/customer; **S7** `courier/auth.ts`)
  are OUT of S6 — S6 *verifies* with the S2 verifier and *depends* on those mints being body-`kid`-parity-
  correct (WS-T7). The **event producers** (S5 `updateOrderStatus`, S7 dispatch, S8 workers) are their
  own surfaces; S6 owns *delivery + fan-out authz*, not what is produced.

---

## 1. Assets

| ID | Asset | Where it lives | Why it matters |
|---|---|---|---|
| W1 | The **live order/dashboard feed** (`order:<id>`, `location:<id>` frames) | in-memory rooms, fed by DB NOTIFY | One tenant's live orders, courier positions, status changes — leaking it to another tenant is the surface's canonical failure |
| W2 | The **per-connection principal** (owner/courier/customer/channel claims) | pinned once at upgrade, in the socket task | The authority every room predicate + relay guard reads; a wrong or stale principal is a wrong fan-out |
| W3 | The **live binding / membership** (`courier_assignments`, `memberships.status='active'`) | tenant tables (FORCE RLS) | The per-frame re-authz source (ADR-0013 / `#4`); a revoked principal must stop streaming within ≤TTL |
| W4 | The **customer GPS stream** (`client_location` lat/lng) | inbound customer frame → courier members only | Live location of a real person; must reach ONLY the bound courier, each frame re-authorized |
| W5 | The **WS auth token** (RS256 JWT, per-role) | `Sec-WebSocket-Protocol` / (legacy) URL / in-band | A bearer; in the URL it leaks via history/Referer/intermediary logs (`#5`) |
| W6 | The **LISTEN/NOTIFY fan-out path** (session-mode connection + channels) | dedicated 5432 session conn | The transport; on the wrong pool it silently delivers ZERO events (HIGH-1) |
| W7 | The **wire contract** (frame envelope + event/control shapes) | serde encoders, both stacks | A client reconnecting mid-session onto the other stack must parse identical frames (the cutover parity gate) |
| W8 | The **connection budget** (session-mode 5432 backends) | Supavisor session pooler (non-multiplexed) | The scarce resource; the listener adds a NET-NEW backend for the overlap |

## 2. Trust boundaries

- **TB-1 anonymous/authenticated client → WS upgrade** — the handshake is the input. Gates: the S2
  verifier (crypto), the 5s auth deadline, the Origin allow-list (Q9), the per-IP upgrade rate limit
  (Q9). No data flows before authn; an unauthenticated socket is bounded by the 5s timeout.
- **TB-2 authenticated principal → room subscribe** — the tri-state predicate per principal (customer
  order-scope exact / owner live membership / courier live binding). ALLOW admits; DENY forbids (no
  close); UNAVAILABLE is a retryable soft error (a pool blip must NOT be read as "no binding" and
  fleet-deny — Breaker H1).
- **TB-3 admitted member → per-frame fan-out** — the role-agnostic bus handler sends to every member;
  the relay guard re-derives each courier/owner member's LIVE authority before every frame (ADR-0013 +
  `#4`). This is the boundary the C1 involuntary-reassign leak crosses; admission alone does NOT hold it.
- **TB-4 producer → NOTIFY → listener** — an S5/S7/S8 producer NOTIFYs a channel; the listener fans out.
  The payload is claim-check/non-PII by design; the boundary trust is that the channel name + payload
  shape are stack-agnostic and the listener is on a session-mode connection that actually receives.
- **TB-5 stack → stack (cutover)** — the **novel** boundary: during overlap both stacks LISTEN the same
  channels and fan out to their own members; a client reconnecting mid-session moves from one stack's
  fan-out to the other's. The trust each stack places in the other is mediated ONLY by the shared DB
  NOTIFY channels + the identical wire contract (W7). A shape drift breaks a reconnecting client.
- **TB-6 head/channel → WS** — NOT admitted in v1 (`Principal::Channel` exists in the type but no head
  gets a room; REBUILD-MAP §6 invariant 2). A head is never admitted as owner/customer.

## 3. Port-specific failure scenarios (Rust)

| # | Scenario | Trigger in the port | Mitigation to prove red→green |
|---|---|---|---|
| **WS-T1** | **Cross-tenant message leak** — tenant A's `order:`/`location:` frames reach a tenant-B socket | Porting admission WITHOUT the per-frame relay guard, or relaying-then-checking, or admitting a courier without a live binding | Tri-state admission (per-principal predicates) + the generic `RelayGuard<Policy>` (relay-only-on-fresh-ALLOW, withhold-then-revalidate); E2E: a message to tenant A's room is NEVER delivered to a tenant-B socket, at admission AND after a mid-stream revocation |
| **WS-T2** | **Mid-stream revocation leak (ADR-0013 C1 / `#4`)** — a reassigned courier / downgraded owner keeps streaming until disconnect | Porting the subscribe gate but NOT the fan-out guard (subscribe gates only NEW subscribes) | Port both relay policies; DENY → evict + `Event::Evicted{reason}` (never close); courier UNAVAILABLE ceiling fires from IN-MEMORY state (holds under total DB starvation); E2E: revoke binding/membership → member stops receiving within ≤TTL / ≤ceiling |
| **WS-T3** | **Unauthenticated / any-path upgrade** — an upgrade on any URL, no Origin check, no rate limit (`websocket.ts:192`, linkage A8) | Carrying `WebSocketServer({server})` semantics (any-path) verbatim | Single axum `GET /ws` (non-`/ws` → 404) + Origin allow-list (CSWSH) + per-IP upgrade rate limit + 5s auth deadline; data still requires auth regardless |
| **WS-T4** | **JWT-in-URL logging leak (`#5`)** — the token in `req.url` captured by browser history / Referer / an intermediary that logs URLs before our redactor | Carrying `?token=` as the primary transport | Adopt `Sec-WebSocket-Protocol bearer.v1` primary (token leaves the URL); `?token=` flag'd dual-accept + deprecation counter; redact `sec-websocket-protocol` in logs (ledger #42 extension); SPA stops setting `?token=` same release |
| **WS-T5** | **Silent no-fan-out (pooler block, HIGH-1)** — a green build delivers ZERO events | Connecting the `PgListener` on the operational (6543 tx-pooler) DSN, where LISTEN is orphaned with no error | Guardrail asserts the listener DSN is `DATABASE_URL_SESSION` (5432 session mode); a reconnect test (drop conn → re-LISTEN → resume); a **degraded-health** signal on listener-down <1 min (never a binary up/down — a degraded listener serves HTTP fine while realtime is dead) |
| **WS-T6** | **Lost live event (fire-and-forget NOTIFY)** — a producer COMMITs then dies before NOTIFY; the event never fans out | Carrying the post-COMMIT non-transactional NOTIFY (no outbox, no replay) | **CARRY + accepted-risk:** recovery is refetch-on-reconnect; the truncated/lost case is an explicit `Event::Resync`. A transactional outbox is a FUTURE hardening (schema-rich, runtime-minimal). Owner + accepted-risk row |
| **WS-T7** | **Courier token verify drift** — a WS courier token verified with drifted `kid` handling → silent auth-fail, or worse a mismatch admits the wrong principal | Minting a second verifier instead of reusing the S2 verifier; OR the S7 `courier/auth.ts` mint drifting from the S2 body-`kid` contract | Reuse `AuthState.verifier.verify` (the ONE S2 verifier); S6's DoD inherits S2's JWT body-`kid` round-trip gate for courier tokens; flag the S7-mint dependency (route-map row 134) |
| **WS-T8** | **Courier session-revocation bypass (NEW)** — a logged-out courier with an unexpired token keeps a live tail | Carrying the crypto-only WS admission (no REV-1 `courier_sessions` liveness check) | Reuse the S2 `CourierSession` bind at WS upgrade (`extractors.rs:166-208`); E2E: revoke the session → the next WS reconnect is denied (parity with REST 401) |
| **WS-T9** | **Cross-stack reconnect break (wire drift)** — a client reconnecting mid-session from Node→Rust (or Rust→Node) cannot parse a frame | Shipping the shape normalizations (one `order.status`, typed eviction) AT the cutover instead of carrying the legacy wire | Golden-frame parity test (fixed event → exact Node JSON, both directions) as a NAMED cutover gate; defer all wire normalizations to a post-cutover FE-lockstep release |
| **WS-T10** | **Flip-mid-delivery connection loss** — a courier's in-flight-delivery socket drops at the flip and loses delivery state | Treating WS like an HTTP surface (hard flip) or assuming socket-held state | Structural: the socket holds NO durable state (DB is authoritative); both stacks LISTEN concurrently (no hard flip moment); mass-reconnect drain + refetch-on-open; accept/pickup/delivered are REST (S7) — delivery completes regardless of the tail |
| **WS-T11** | **Fan-out amplification / reconnect-storm DoS** — a synchronized mass-close (or a malicious reconnect flood) storms the upgrade path; or an oversized payload | A synchronized drain; no upgrade rate limit; unbounded per-connection send queue | Gradual drain (stop-new + churn-migrate) + jittered client backoff + the ≤60s expiry jitter + per-IP upgrade rate limit; bounded per-connection send queue → drop + `Resync` on lag, close 1013 on a never-draining consumer; claim-check bounds payloads (8000 B NOTIFY cap) |
| **WS-T12** | **Expired-token infinite reconnect loop** — an expired token (or an anonymous order) reconnects forever on 1008 (linkage A3/A7), a silent client + server-churn cost | Carrying "never re-check `exp` after admission" + the 1008-treated-as-retryable client | Connection-level `Event::TokenExpiring` → grace → distinct 4401 close (client re-authenticates, does not blind-retry); anonymous → terminal close → REST-poll mode |
| **WS-T13** | **Session-connection exhaustion at overlap** — the Rust listener's NET-NEW session-mode backend + Node's + workers + migrations exceed the 5432 (non-multiplexed) ceiling | Running an indefinite dual-listen; ignoring that session mode does not multiplex | Time-box the overlap (the flip is the contract to shed the Node listener's session conn); monitor session-mode connection count under the ceiling; the listener multiplexes ALL channels onto ONE session connection (not one-per-channel) |
| **WS-T14** | **Duplicate fan-out / room-subscription leak** — an event delivered N× to one client (the "Calling N handlers" leak) or a stacked LISTEN after room re-create | Dropping the P1-WSDUP invariant (exactly one bus/LISTEN subscription per room; eager teardown on last-leave) | Carry P1-WSDUP: room GC eager on last-leave (incl. `UNLISTEN`) + periodic sweep; re-subscribe races covered by tests; each room registers exactly one listener handler |

## 4. What the B3 RLS flip changes for S6

- **Today (BYPASSRLS):** the courier binding read + owner membership JOIN carry explicit predicates; the
  courier read ALSO seats `app.current_tenant` inside a tx (`courier-room-authz.ts:44-53`) so it is
  **already NOBYPASSRLS-sound** — order-independent of B3. The owner predicate is a `memberships` JOIN
  (the JOIN is the tenant boundary).
- **Post-flip (NOBYPASSRLS):** the courier binding SELECT needs the seated GUC to pass FORCE RLS — which
  it already has. The B3-council fixes named (not fixed) here: the `app_member_location_ids()`
  search_path pin (any owner-membership read inherits it transitively). **S6 does not regress under B3
  because its one DB-touching authz read is already GUC-seated** — this is a property to *carry*, not to
  build.
- **S6's rule:** every fan-out authz read is correct **independent of which pool role is live** (belt =
  explicit predicate; suspenders = the courier read's seated GUC). The B3 flip and the Node→Rust flip are
  two orthogonal, independently-reversible events.

## 5. Residual risks (summary for the human)

- **Fire-and-forget post-COMMIT NOTIFY (WS-T6 / Q4 residual)** — re-shipping a "live event can be
  silently lost, recovered only by client refetch" property through a deliberate rewrite. Defensible as
  "no transactional outbox is a future hardening, not a defect," but must be an **explicit accepted-risk
  with a named owner**, not a silent omission. **The most likely counsel flag.** Owner: architect
  (defer the outbox to a post-rebuild hardening council) + S6 lead (the `Resync` seam).
- **Cross-tenant fan-out under mid-stream revocation (WS-T1/T2 / Q2)** — the leak the whole surface
  exists to prevent; bounded to ≤TTL (OR-9: not literally zero, carried from ADR-0013/`#4`). **The most
  likely breaker escalation** — the council should have the breaker attack the withhold-vs-relay
  ordering, the UNAVAILABLE ceiling under total DB starvation, and the single-chokepoint module
  visibility. Owner: architect + operator + breaker.
- **Courier session-revocation on WS (WS-T8)** — a real WS-vs-REST asymmetry (logout does not drop a WS
  tail today). Closed by reusing the S2 `CourierSession` bind at upgrade; the residual is the customer's
  deliberate irrevocable-bearer (bounded by order-scope + short TTL, S2). Owner: S6 lead + S2 lead.
- **Reconnect-storm + session-mode budget at the flip (WS-T11/T13 / Q5)** — the flip's operational
  hazards; bounded by gradual drain + jitter + rate limit + time-boxed overlap, not eliminated. Owner:
  architect + operator.
- **Cross-stack wire drift (WS-T9 / Q6)** — a shape divergence breaks a mid-session reconnecting client;
  bounded by the golden-frame parity gate + deferring normalizations. Owner: architect + S6/FE lead.

**None of W1–W8's core failure modes is *introduced* by the rewrite** — cross-tenant authz, per-frame
revocation, claim-check, and the session-mode listener are all **current** properties the port must carry
**visibly** (matrix row + test). The rewrite's *new* risks are the **cutover concurrency** (WS-T9/T10/T13,
TB-5) — a stateless-tail flip across a live delivery, which no prior single-stack packet faced — and the
**courier session-liveness gap (WS-T8)** the port is the natural moment to close. **Breaker-escalation
candidate: the cross-tenant fan-out under mid-stream revocation (WS-T1/T2).** **Counsel-flag candidate:
the fire-and-forget post-COMMIT NOTIFY (WS-T6)** — carrying a silent-event-loss property through a
deliberate rewrite is acceptable only as an explicit, owned accepted-risk, never by silence.
