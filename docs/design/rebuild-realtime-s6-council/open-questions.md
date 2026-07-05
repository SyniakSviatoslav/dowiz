# S6-REALTIME/WS Port — Council Packet · OPEN QUESTIONS

> **STATUS: 🟡 DRAFT — NOT APPROVED.** Everything the live Triadic Council (architect / breaker /
> counsel / human) must decide before S6 realtime/WS is ported. Each question has options + a lane-R5
> recommendation — a *starting position for friction*, not a decision. This surface is 🔴 for realtime
> authz + cross-tenant fan-out; the sharpest questions are cross-tenant leak (Q2) and the cutover of a
> live in-flight-delivery connection (Q5). **Folds and renumbers the prior draft's Q1–Q12**
> (`docs/design/rebuild-ws-authz-council/open-questions.md`) — cross-refs noted per question. Docs only.

Legend: **[AUTHZ]** admission/fan-out authz · **[SEC]** security/tenancy · **[INFRA]** transport/cutover ·
**[CONTRACT]** wire shape/parity · **[SCOPE]** surface placement · **[LIFECYCLE]** connection policy.
🔴 = red-line, operator sign-off required.

---

### Q1 🔴 [AUTHZ] WS upgrade authn + token transport + the courier session-liveness gap
*(folds prior Q4 transport + the NEW courier-session finding)*
WS admission reuses the **S2 verifier** (`AuthState.verifier.verify` — RS256 body-`kid`, `extractors.rs:88-90`);
a browser cannot set `Authorization` on a handshake, so the *transport* differs but the *crypto verify*
is identical. Two coupled decisions:
- **(1a) Token transport.** **(a)** approve the ADR-0013-addendum and land the subprotocol switch on
  Node BEFORE S6; **(b)** **fold the addendum into the S6 cutover** — Rust ships `Sec-WebSocket-Protocol
  bearer.v1` primary + `?token=` behind `WS_URL_TOKEN_ACCEPT` (default ON), SPA stops setting `?token=`
  same release, removal = flag flip after the deprecation counter hits zero; **(c)** reject the addendum,
  keep in-band message-auth only (re-opens the breaker's 5s-budget HIGH). **Recommend (b)** — one
  migration, no-deploy rollback via the flag + counter. **Requires operator co-approval of the addendum**
  (`docs/design/ws-token-in-url/ADR-0013-addendum-DRAFT.md`).
- **(1b) Courier WS session liveness (NEW).** Today courier WS admission verifies **crypto only** — no
  REV-1 `courier_sessions` liveness check — so a **logged-out courier** with an unexpired 14d token keeps
  a live tail on orders they still hold a binding for (logout does not drop the binding); REST would 401
  them. **(a)** carry the gap (WS ≠ REST on courier revocation); **(b)** **reuse the S2 `CourierSession`
  bind at WS upgrade** (`extractors.rs:166-208`) so WS admission IS a live-session check — parity with
  REST. **Recommend (b)** — S2 already built the bind; this closes a real WS-vs-REST authz asymmetry.

**R5 recommendation:** 1a(b) + 1b(b). **🔴** — the token transport touches every live client and the
courier-session gap is a revocation hole. Owner: S6 lead + operator (addendum co-approval) + S2 lead
(the shared verifier/`CourierSession` bind). **Cross-surface note:** courier tokens are minted by
`courier/auth.ts` (path-owned **S7**, route-map row 134) carrying S2's body-`kid` obligation — S6's WS
courier verify inherits that parity dependency; if the S7 mint's `kid` handling drifts, WS courier auth
silently fails. Flag S7's cutover DoD to inherit S2's JWT-parity gate (already flagged in the route-map).

### Q2 🔴 [AUTHZ] Fan-out cross-tenant re-authz — port ADR-0013 + `#4` as one generic guard
*(folds prior Q10)*
Admission gates NEW subscribes; the role-agnostic bus handler (`websocket.ts:215-233`) sends to EVERY
member, so a principal admitted then revoked keeps streaming until disconnect unless every frame is
re-authorized. Node already solves this with two guards (`courier-relay-guard.ts` +
`createOwnerRelayGuard`) sharing one shape.
- **(a)** port both verbatim as two separate structs;
- **(b)** **one generic `RelayGuard<Policy>`** — courier policy (ceiling on: 60s wall / count fires from
  in-memory state, holds under DB starvation) + owner policy (ceiling off) — with unit tests asserting
  each policy's ledger-protected behaviors (relay-only-on-fresh-ALLOW, no-refresh-on-read, evict-not-close).
**Recommend (b)** — the `local/no-raw-courier-ws-send` drift rule's intent becomes **module visibility**
(`rooms.rs` exports no raw send; the guard is the only fan-out write path); a divergence requires a new
`Policy`, exactly the review point we want.

**R5 recommendation:** (b). **🔴** — this is the surface's whole reason to exist: **a message for tenant
A must never reach tenant B, at admission AND per-frame, even after a mid-stream revocation** (ADR-0013
C1 leak; the `#4` owner-revocation gap). Owner: architect + operator + breaker (attack the withhold-vs-relay
ordering + the UNAVAILABLE ceiling under total DB starvation).

### Q3 [AUTHZ/QUIRK] The no-path-filter upgrade quirk — CARRY or FIX-IN-PORT a real path
`new WebSocketServer({ server })` (`websocket.ts:192`) has NO `path` → upgrades on ANY URL land in one
handler (the dead `/ws/orders/:id` widget + the SPA's `/ws`); the cutover matcher special-cases
`isWebSocketUpgrade()` (the `Upgrade` header) BEFORE any path template (surprise #7).
- **(a)** CARRY the any-path behavior (accept upgrades on any URL in Rust too);
- **(b)** **FIX-IN-PORT: single axum `GET /ws`** — a non-`/ws` upgrade 404s before the handshake; the
  quirk becomes unrepresentable. The matcher keeps header-first steering (do NOT add a `/ws` path
  template to the proxy — phantom precision the Node server never had).
**Recommend (b)** — security-narrowing with a tiny, safe delta: the only live client uses `/ws`
(`useWebSocket.ts:6`); `/ws/orders/:id` is dead (linkage A5). Not 🔴 (behavior-narrowing, no live client
relies on the any-path behavior). Owner: S6 lead.

### Q4 🔴 [INFRA] `PgListener` dedicated session connection + NOTIFY off the transaction pooler
LISTEN/NOTIFY does NOT work over the Supavisor **transaction pooler (6543, multiplexed)** — a LISTEN
there is orphaned and **no notifications arrive, with no error** (cutover HIGH-1). The Node bus already
uses `createSessionPool()` (5432) for both LISTEN and NOTIFY (`message-bus.ts:25-28,116-131`).
- **(a)** carry the transport implicitly (risk the Rust listener lands on the operational DSN);
- **(b)** **the Rust `PgListener` MUST connect on `DATABASE_URL_SESSION` (5432); a config guardrail
  asserts it** — plus NOTIFY on a session-mode connection when the Rust producers eventually flip;
  reconnect indefinite + re-LISTEN all channels (`message-bus.ts:95-114`); claim-check → `Event::Resync`.
**Recommend (b)**.

**R5 recommendation:** (b). **🔴** — the wrong pool is a **silent realtime blackout** (green build, zero
events), and the listener's session-mode connection is a NET-NEW backend against the non-multiplexed
5432 ceiling during overlap (the S6 scaling gate). Owner: architect + operator (session-mode connection
budget). **Also decide the fire-and-forget-NOTIFY residual** (a producer crash between COMMIT and NOTIFY
loses a live event; no outbox, recovery = refetch-on-reconnect) — recommend **accept + defer the
transactional outbox to a post-rebuild hardening council**; the explicit `Resync` is the seam. **Most
likely counsel flag.**

### Q5 🔴 [INFRA] Cutover concurrency — the live in-flight-delivery connection across the flip
*(counsel's S6 concern)* A WS socket carries **no durable state** (assignment/status/GPS/cash-as-proof
are all DB); it is a live tail. Fan-out is decoupled through DB NOTIFY channels (stack-agnostic).
- **(a)** treat WS like an HTTP surface — hard flip + connection migration (rejected: there is no socket
  state to migrate; migration is engineering for a problem that does not exist);
- **(b)** **mass-reconnect drain: both stacks LISTEN + fan out concurrently during overlap (no hard flip
  moment); steer NEW upgrades to Rust; Node sockets drain (natural reconnect or explicit 1001); each
  client is on one stack so no double-delivery; in-flight delivery survives because state is in the DB
  and the client refetches on reconnect; controls: jittered backoff + per-IP upgrade rate limit +
  gradual drain to bound the reconnect storm; rollback = `WS_SURFACE=node` + mass-reconnect**;
- **(c)** block S6 until S5+S7 flip (rejected: the DB-NOTIFY decoupling means S6 can flip independently;
  blocking over-couples it).
**Recommend (b)**.

**R5 recommendation:** (b). **WS is the gentlest cutover in the rebuild** precisely because the socket is
a stateless tail and fan-out is DB-decoupled — no connection migration is needed. **🔴** because it is
the one flip a live courier crosses mid-delivery and the reconnect-storm + session-conn budget are real.
Owner: architect + operator + breaker (attack a synchronized mass-close + the session-mode ceiling).

### Q6 🔴 [CONTRACT] Message wire-parity — carry shapes verbatim during overlap, defer normalizations
A client can reconnect mid-session onto the OTHER stack (Q5), so every frame shape must be
byte/shape-identical across stacks — the S6 analog of S5's request-hash gate. Today: `{room, data: msg}`
envelope, two-shape `order.status`, generic `{type:'error', error}` eviction frames.
- **(a)** ship the prior draft's normalizations (one `order.status` shape, typed `Event::Evicted{reason}`,
  the 19-unhandled dispositions) AT the cutover — rejected: each changes the wire, and a Node→Rust
  reconnect mid-session would break;
- **(b)** **CARRY every wire shape verbatim during the overlap (golden-frame parity test, both
  directions, as a NAMED cutover gate); apply the normalizations in a POST-cutover FE-lockstep release**
  (single-stack + Astro FE consuming the new shapes).
**Recommend (b)** — mirrors S5's "shape-migration defers to post-Astro FE-lockstep." **🔴** because a
one-shape drift is a silent broken-frame on a mid-session cross-stack reconnect. Owner: architect +
S6/FE lead.

### Q7 [SCOPE/CONTRACT] The 19 published-but-unhandled events + retire the shift room
*(folds prior Q5 + Q6)* Packet.md §3 dispositions 19 published-but-unhandled types (9 CONSUME /
5 STOP-PUBLISH / 2 RETIRE / 1 KEEP-internal) and the `courier:<id>:shift` room (zero FE subscribers,
grep-proven).
- **(a)** apply the dispositions at the cutover — rejected: all are wire changes (Q6);
- **(b)** **carry-publish everything during overlap; apply CONSUME (FE work) / STOP-PUBLISH / RETIRE
  post-cutover; RETIRE the `courier:<id>:shift` room (rooms 5→4) then.** Batch-accept the table;
  **contest rows 8 (`task_offered`) and 16 (`gdpr.erasure_completed`) explicitly** (the two with
  plausible product arguments both ways).
**Recommend (b)** — the disposition table is a *post-cutover work list*, dispositioned now, applied
later. Not 🔴 individually (no live behavior change at the cutover). Owner: S6/FE lead + operator (rows 8/16).

### Q8 🔴 [LIFECYCLE] Expiry-mid-subscription + the anonymous reconnect loop
*(folds prior Q2 + Q8)* Nothing re-checks `exp` after admission (any role); an expired token on reconnect
→ 1008 → the SPA reconnects FOREVER (`useWebSocket.ts:86-109`) — a silent permanent loop (owner 24h
dashboards hit this daily). Anonymous orders (no token) loop identically (linkage A7).
- **(a)** carry verbatim (document the infinite loop as contract) — rejected;
- **(b)** **connection-level policy: track `min(exp)`; near `exp` send `Event::TokenExpiring` → allow one
  in-band `Auth` refresh in a grace window → close with a distinct 4401-class code the client treats as
  "re-authenticate, don't blind-retry"; anonymous → a terminal close code the client maps to REST-poll
  mode.**
- **(c)** re-verify `exp` only at each subscribe (leaves long-lived subscriptions unbounded) — rejected.
**Recommend (b)** — the only option that also kills the expired-dashboard reconnect-storm without
per-frame cost. **🔴** because it is a client-visible behavior change on a live surface (a small,
documented E2E delta: an expired token → 4401 not infinite-retry; an anonymous order → REST-poll not
loop). Owner: S6 lead + operator + FE lead.

### Q9 [SEC] Upgrade-surface discipline in axum
*(folds prior Q9)* Today: any-path upgrade, no Origin check, no upgrade rate limit (linkage A8).
- **(a)** carry verbatim;
- **(b)** **single `/ws` route (Q3) + Origin allow-list (CSWSH belt — browsers only; heads get no WS in
  v1) + per-IP upgrade rate limit (tower-governor)** — all absent today; data still requires auth even
  before these, but they bound churn/DoS surface.
**Recommend (b)** — cheap; the Origin check is the standard cross-site-WS-hijack belt. Not 🔴 (additive
hardening). Owner: S6 lead.

### Q10 [SEC] GDPR erasure ↔ live sockets
*(folds prior Q12)* Erasure (`anonymizer.gdpr`) evicts no subscribed customer socket nor invalidates any
grant; grants only age out at 14d (linkage A2).
- **(a)** accept (frames post-erasure carry no PII by claim-check design);
- **(b)** **erasure publishes an eviction for `order:<id>` customer members + deletes the order's
  `customer_track_grants` rows.**
**Recommend (b)** — cheap, and aligns WS behavior with the S9 GDPR invariant cluster the rebuild is
adding. Not 🔴 on the WS surface (belt over an already-non-PII stream), but cross-refs the **S9** council.
Owner: S6 lead + S9 lead.

### Q11 [SCOPE] Channel heads and realtime (REBUILD-MAP §6 invariant 2)
*(folds prior Q7)* Do channel principals (TMA / conversational / feed / agentic-MCP heads) get any WS
surface in v1?
- **(a)** no WS for heads — REST/poll + the notifications lane (invariant-1-shaped);
- **(b)** a scoped `ChannelOrder(channel_id, order_id)` room from day one;
- **(c)** per-head decision at each head's own authz council.
**Recommend (a) for v1, with (c) as the standing rule** — **`Principal::Channel` exists in the type from
S6** so a later grant is additive, but no head is ever admitted to customer/owner rooms. "Schema-rich,
runtime-minimal": the seam exists, the runtime stays off. Not 🔴. Owner: architect.

### Q-cross-ref [SEC, NOT an S6 build item] `dos_access_token` FE storage collision
Customer checkout stores the 7d customer token under **`dos_access_token`** — the same key the
owner/courier session uses and `useWebSocket.ts:48` reads — so placing an order in an owner's browser
replaces the owner token with a customer token (linkage A6). **Recommend: FE-only key split
(`dos_customer_token`), landed on Node PRE-S6** — a live defect independent of the rebuild; not an S6
build target, surfaced so it is not lost. Owner: FE lead.

---

## Decision-ordering note for the council
**Q1 (admission+token+courier-session)**, **Q2 (fan-out cross-tenant)**, and **Q4 (PgListener
session-conn)** are **build-blocking** — they define the three load-bearing seams (authz-in, authz-per-frame,
transport); decide them first. **Q5 (cutover concurrency)** and **Q6 (wire-parity)** are
**cutover-blocking, not build-blocking** — the Rust WS can be built + dark-verified before they settle,
but the **flip** cannot happen until Q6's golden-frame parity gate is green and Q5's drain/session-budget
plan is signed. **Q8 (lifecycle expiry)** is a live behavior change with a small E2E delta — build-time,
but 🔴 for the client-visible contract. **Q3/Q7/Q9/Q10/Q11** are settled by recommendation
(quirk-narrowing / post-cutover work list / additive hardening / scope seam).

**The single most likely breaker escalation:** the **cross-tenant fan-out under mid-stream revocation**
(Q2) — the leak the whole surface exists to prevent, plus the reconnect-storm + session-mode budget at
the flip (Q5). **The single most likely counsel flag:** the **fire-and-forget post-COMMIT NOTIFY**
(Q4 residual) — carrying a "live event can be silently lost, recovered only by refetch" property through
a deliberate rewrite is defensible as "no outbox is a future hardening, not a defect" but must be an
explicit accepted-risk with an owner, not silence.
