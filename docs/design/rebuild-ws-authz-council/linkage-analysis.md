# Customer-JWT-mint ↔ WS-authz linkage — focused pass (REBUILD-MAP §7 open item 1)

**Status: DRAFT — NOT APPROVED. Council input only. No code changed.**
**Date:** 2026-07-04 · **Lane:** R5 · **Closes:** inventory 10 §6.4 item 1 ("token-mint↔WS-authz linkage needs one focused pass in council C4")
**Sources:** working tree at branch `fix/audit-remediation` (note: `apps/api/src/routes/orders.ts` is in a merge state in this tree — line numbers verified against the current working copy, no conflict markers present).

---

## 1. Mint-site census (customer-scope JWTs)

Exactly **one mint function** and **two production call sites**. Grep-verified: no other site signs
`role: 'customer'` anywhere in `apps/api/src` or `packages/`.

| # | Site | Route / trigger | Claims | TTL | Guard rails |
|---|---|---|---|---|---|
| M0 | `packages/platform/src/auth/jwt.ts:117-132` — `issueCustomerToken()` (the only definition) | called by M1/M2 | `{role:'customer', orderId, locationId, sub: customerId, iat, exp, kid}` — RS256, header+body `kid`. **No `aud`, no `iss`, no `jti`.** Phone deliberately excluded (P0-PII, comment at :122-125) | **7d** (`:131`) | `signAuthToken` → env kid + private key; dev mints go through `signDevToken` (separate dev kid, prod-rejected — ADR-0003), never through this function |
| M1 | `apps/api/src/routes/orders.ts:650-656` | `POST /api/orders` (order create), post-COMMIT | via M0, tuple `(order.id, locationId, resolvedCustomerId)` | 7d | Minted **only when** `cust?.phone && resolvedCustomerId` (`:648`) — anonymous/no-phone orders get NO token. Mint failure is swallowed (`:657-659` log only); 201 still returned with `authToken: undefined` |
| M2 | `apps/api/src/routes/customer/track.ts:75-79` | `POST /api/customer/track/exchange` (pre-auth, NO_AUTH_PATHS) | via M0, tuple recovered by JOIN `customer_track_grants → orders` (`:43-53`) | 7d | Opaque-code bearer: sha256(code) lookup, uniform 410 for unknown/expired/gone, 10/min/IP rate limit. Grant is **reusable until expiry** (`:66-71`, use_count = observability, not single-use) |
| M2a (feeder) | `apps/api/src/lib/order-persistence.ts:151-160` | inside the order-create txn | opaque grant (not a JWT): 32-byte token, sha256 at rest | **14d** grant expiry | Same mint condition as M1 (`input.custPhone && input.resolvedCustomerId`); raw code returned once as the `?t=` tracking-link code |

Dev-only: `routes/dev/mock-auth.ts` mints dev-kid tokens (`signDevToken`) — cryptographically rejected
in prod (ADR-0003); not part of this census. `POST /couriers/invites` and owner/courier auth mints are
out of scope (different roles; see inventory 10 §1).

## 2. Where each WS path validates the token (verify side)

| Path | file:line | What is checked | What is NOT checked |
|---|---|---|---|
| WS upgrade, URL token | `apps/api/src/websocket.ts:338-355` | `verifyAuthToken()` — same verifier as REST (`jwt.ts:82-115`): kid-selected key, RS256 pinned twice, alg=none rejected, Zod strict-parse of the claim shape | expiry only at this instant; no DB touch for customers; `#5` dual-accept deprecation logging (`:350-351`) |
| WS first-message auth | `websocket.ts:373-386` | same `verifyAuthToken()`; 5s `authTimeout` closes 1008 if neither path lands (`:357-362`) | same |
| WS subscribe (customer branch) | `websocket.ts:392-397` | **exact string match** `room === \`order:${user.orderId}\`` — the JWT's own bound order id, zero DB query | order existence, order status, GDPR-erasure state, token expiry, location — nothing live is consulted, ever |
| Fan-out relay (customer member) | `apps/api/src/lib/courier-relay-guard.ts:112-119` | **none — relayed directly.** "Non-courier members (owner/customer) are relayed directly — their room admission is authoritative" | no revalidation of any kind for the life of the socket |
| `client_location` inbound | `websocket.ts:458-476` | `role === 'customer'` + lat/lng range; outbound to courier members only, each through the ADR-0013 guard | — |
| REST `GET /api/orders/:id` | `apps/api/src/routes/orders.ts:735-760` | `softVerifyAuth` + explicit principal check: customer must satisfy `user.orderId === id` (404 otherwise), P2-ANONORDER 401 for anonymous | — (this is the tight, mint-symmetric predicate) |
| REST `customer/orders.ts` routes (status/rating/cancel) | `apps/api/src/routes/customer/orders.ts:50,236,284` | `verifyAuth` + `requireRole(['customer'])`, then `WHERE o.id = $1 AND o.customer_id = $2` (`$2 = token.sub`) | **the token's `orderId` and `locationId` claims are ignored** — see asymmetry A1 |

Verifier symmetry: mint and verify share one keypair/kid path and one Zod schema
(`packages/shared-types/src/legacy.ts:162-174`) — crypto-side mint↔verify is symmetric and sound
(confirmed SOLID in `docs/security/hardening-findings-2026-07-02.md`). The asymmetries are all in
**scoping, revocation, transport and lifetime**, below.

## 3. Mint↔verify asymmetries found

**A1 — Authority-tuple asymmetry (scoping drift across consumers).** The minted authority is the
tuple `(orderId, locationId, sub)`. Three consumers enforce three different subsets:
- WS subscribe: `orderId` only (`websocket.ts:393`).
- `GET /api/orders/:id`: `orderId` (+ `locationId` used for scoping) (`orders.ts:751-754`).
- `customer/orders.ts` status/rating/cancel: **`sub` only** (`customer_id = $2`), so a token minted
  for order A authorizes status-read, rating and **cancel** of any other order B belonging to the same
  customer row — broader than the minted per-order authority. Comment at `customer/orders.ts:218`
  states this is deliberate ("enforced by customer_id = token.sub"), but it contradicts the
  issueCustomerToken doc-comment ("the token's authority is the (orderId, locationId, customerId)
  tuple"). In the Rust port a single claims-extractor must pick ONE predicate; carrying both verbatim
  reproduces the drift. → open-questions Q3.

**A2 — Revocation asymmetry (customer = the only irrevocable principal).** Courier: JWT + `jti` +
`courier_sessions` liveness on EVERY REST request (`plugins/auth.ts:60-93`) + live binding re-check at
WS subscribe AND per-frame fan-out (ADR-0013). Owner: live membership at WS subscribe + per-frame
owner relay guard (#4, `websocket.ts:35-63,102-171`). Customer: **no session row, no jti, no live
check anywhere** — a 7d bearer that cannot be revoked short of key rotation. GDPR erasure
(`anonymizer.gdpr`) does not invalidate outstanding customer tokens or grants for that order (grants
are only swept at their own 14d expiry — `workers/anonymizer-retention.ts:46`).

**A3 — Lifetime asymmetry (JWT 7d vs grant 14d vs socket ∞).** Three clocks disagree:
- The tracking **grant** outlives the **JWT** (14d vs 7d) — recoverable by re-exchange, by design.
- The **socket** outlives them both: token `exp` is checked only at auth; nothing re-checks it after
  admission — for ANY role. An authenticated subscription streams past `exp` until disconnect (the
  owner/courier guards re-check DB state, not token expiry).
- On reconnect with an expired token: `verifyAuthToken` throws → URL-path failure is swallowed
  (`websocket.ts:352-354`) → 5s `authTimeout` → close 1008 → `useWebSocket.ts:86-109` treats 1008 as
  retryable and **reconnects forever** (~15s steady-state) — an expired-token client becomes a
  permanent, silent reconnect loop with no re-exchange trigger and no user-visible signal.

**A4 — Token-in-URL exposure (the #5 finding, current state).** Both live WS clients still put the
JWT in the URL:
- SPA: `apps/web/src/lib/useWebSocket.ts:50` sets `?token=` **and** also sends `{type:'auth'}`
  (`:61`) — dual transmission, so the deprecation counter (`websocket.ts:179-181`) can never reach
  zero while the SPA itself sets the param.
- Server dual-accepts (`websocket.ts:338-355` URL, `:373-386` message).
- **P1 (shipped, ledger #42):** the Pino serializer now strips secret-bearing query params
  (`apps/api/src/lib/logger.ts:18-40`, `redactUrlSecrets`) — the log vector is closed.
- **Residual:** browser history/console, Referer edge cases, any intermediary that logs URLs before
  our logger. P2/P3 (`Sec-WebSocket-Protocol bearer.v1`) is designed and gated in
  `docs/design/ws-token-in-url/ADR-0013-addendum-DRAFT.md` — **still awaiting operator approval**.

**A5 — SSR head: mint→consume is broken (dead linkage).**
- `apps/api/src/client/checkout/place-order.ts:47-49` stores `data.jwt` under
  `dowiz:session:${data.orderId}` — but the 201 response fields are **`authToken`** and **`id`**
  (`orders.ts:695-703`). Both reads are `undefined`; the SSR session store never fires.
- `apps/api/src/client/status/ws.ts` (`StatusWSClient`, URL-token-only) has **zero importers**
  (grep: only its own definition + a logger comment). The escalation treated it as a live P3
  constraint; it appears to be dead code. → RETIRE-candidate row with proof-of-deadness; if
  confirmed, the P3 migration constraint ("widget is URL-only") is void, which simplifies R2.

**A6 — SPA storage-key collision.** `CheckoutPage.tsx:404-406` stores the customer token under
**`dos_access_token`** — the same safeStorage key the owner/courier session uses, and the same key
`useWebSocket.ts:48` reads. Placing an order in a browser that holds an owner session **replaces the
owner token with a 7d customer token** (admin surfaces then 401/behave as customer until re-login).
Likewise `OrderStatusPage.tsx:178-183` overwrites it on track-exchange.

**A7 — Anonymous orders: guaranteed WS reconnect loop.** No phone → no JWT, no grant (M1/M2a
condition). `useWebSocket.ts:62-64` then sends a bare `subscribe` with no auth; the server closes
1008 `Invalid auth format` (`websocket.ts:382-384`); the SPA hook retries forever (~15s). Anonymous
order status is REST-poll only by design, but the client's WS loop is unbounded noise (and a small
server-side connection-churn cost).

**A8 — No upgrade-path/origin discipline.** `new WebSocketServer({ server: fastify.server })`
(`websocket.ts:192`) upgrades **any** path (the dead widget's `/ws/orders/:id` and the SPA's `/ws`
both land in the same handler) with no Origin check and no upgrade rate limit; unauthenticated
sockets are only bounded by the 5s auth timeout. Not exploitable for data (auth still required) but
it is unpinned surface the axum port should make explicit.

## 4. What happens at token expiry mid-subscription (precise trace)

1. `t0`: customer authenticates (URL or message), subscribes `order:<id>`. `exp = t0 + 7d` (M1) or
   exchange-time + 7d (M2).
2. `t0 → disconnect`: frames flow. **No component re-reads `exp`** — not the heartbeat (liveness
   only, `websocket.ts:287-297`), not the fan-out (customers relayed directly,
   `courier-relay-guard.ts:112-119`), not room GC. Same holds for owner/courier (their guards
   re-check DB authz, never `exp`).
3. Past `exp`, same socket: streaming continues indefinitely. REST calls with the same token now 401
   (`verifyAuth`), so the FE's refetch-on-event begins failing while WS frames still arrive —
   a state-skew window unique to expiry.
4. Any disconnect after `exp`: reconnect fails auth → 1008 → SPA infinite retry (A3). The customer's
   escape hatch is re-opening the `?t=` tracking link (M2 re-exchange, valid to day 14); nothing in
   the FE does this automatically on WS 1008/REST 401.

**Practical severity:** low for customers (order lifetime ≪ 7d) but the same no-exp-recheck property
applies to courier (7d/14d) and owner (24h) sockets, where long-lived dashboards routinely outlive
`exp`. The port must pick an explicit policy (open-questions Q2).

## 5. One-line verdict

Crypto and per-role WS admission are sound; the linkage's real gaps are **policy holes, not verifier
bugs**: an irrevocable 7d customer bearer with three inconsistent scoping predicates, no expiry
enforcement after WS admission, a still-open (gated) URL-transport migration, and a dead SSR
mint→consume pair that should be retired rather than ported.
