# WS-authz council — open questions for the live council

**Status: DRAFT — NOT APPROVED. Numbered decisions with options + lane recommendation; the council (and operator for 🔴) decides.**
**Date:** 2026-07-04 · **Lane:** R5 · **Companion to:** `linkage-analysis.md`, `packet.md` (this dir).

---

**Q1 🔴 — Customer token revocability.** Today the customer JWT is an irrevocable 7d bearer (no jti,
no session row, no live check on the WS path — linkage A2). Options:
(a) keep as-is (accept 7d irrevocability; order lifetime is hours);
(b) add a jti + grant-backed liveness check (customer parity with couriers — new table traffic on
every REST call and per-subscribe);
(c) **shorten the JWT TTL to 24-48h and lean on the 14d track grant for re-exchange** (no new state;
revocation ≈ TTL; erasure can additionally delete the grant row to cut re-exchange).
**Recommendation: (c)**, plus: GDPR erasure deletes the order's `customer_track_grants` rows
eagerly (today they only age out at 14d).

**Q2 🔴 — Expiry-mid-subscription policy (all roles).** Nothing re-checks `exp` after WS admission;
expired tokens also cause an infinite client reconnect loop on 1008 (linkage A3/§4). Options:
(a) carry verbatim (never re-check; document as contract);
(b) **connection-level enforcement: `TokenExpiring` warning → in-band re-auth grace → close with a
distinct 4401-class code that clients treat as re-authenticate-don't-retry**;
(c) re-verify exp only at each subscribe (cheap but leaves long-lived subscriptions unbounded).
**Recommendation: (b)** — it is the only option that also kills the reconnect-storm class for
expired owner dashboards (24h tokens) without per-frame cost.

**Q3 🔴 — Unify the customer scoping predicate.** Three consumers enforce three different subsets of
the minted `(orderId, locationId, sub)` tuple; `customer/orders.ts` authorizes **any order of the
same customer row**, including cancel (linkage A1). Options:
(a) CARRY-VERBATIM (parity oracle sees identical behavior; document the divergence);
(b) **FIX-IN-PORT: enforce `token.orderId == :orderId` (+ locationId) on ALL customer routes** —
the claims-extractor makes it structural;
(c) widen deliberately: define the customer token as customer-scoped (not order-scoped) and re-mint
docs to match.
**Recommendation: (b)** — matches the mint site's documented authority tuple and the WS gate;
security-narrowing with a small, documented E2E delta. Needs the FIX-IN-PORT council stamp (this
council) per the REBUILD-MAP fix-vs-carry rule.

**Q4 — WS token transport at S6.** The `Sec-WebSocket-Protocol bearer.v1` addendum
(`docs/design/ws-token-in-url/ADR-0013-addendum-DRAFT.md`) is still awaiting operator approval;
meanwhile the SPA sends the token in the URL AND in-band (linkage A4). Options:
(a) approve the addendum and land R1 (dual-accept + client switch) on Node BEFORE S6;
(b) **fold the addendum into the S6 cutover** (Rust ships subprotocol-primary + `?token=` flag'd
dual-accept; SPA stops setting `?token=` in the same release; R2 removal = flag flip after the
deprecation counter hits zero);
(c) reject the addendum, keep in-band message auth only (re-opens the breaker's 5s-budget HIGH).
**Recommendation: (b)** — one migration instead of two; the counter + `WS_URL_TOKEN_ACCEPT` give a
no-deploy rollback. Requires operator co-approval of the addendum.

**Q5 — The 19 published-but-unhandled event types.** Per-type dispositions proposed in packet §3
(9 CONSUME / 5 STOP-PUBLISH / 2 RETIRE / 1 KEEP-PUBLISH-internal / plus enum normalizations).
Decision needed: sign off the table row-by-row, or batch-accept with named exceptions. Each CONSUME
creates FE work in the S6/S7 slices; each STOP-PUBLISH/RETIRE needs a matrix RETIRE row with proof.
**Recommendation: batch-accept; contest rows 8 (`task_offered`) and 16 (`gdpr.erasure_completed`)
explicitly, as they are the two with plausible product arguments both ways.**

**Q6 — Retire the `courier:<id>:shift` room kind.** Zero FE subscribers (grep-proven,
inventory 10 §3); publisher is `shiftService.ts:60`. Options: (a) port it dark; (b) **RETIRE room +
`shift.opened` event, keep the REST/notification paths**.
**Recommendation: (b)** — rooms 5→4; a resurrected shift-history widget can add a typed room later.

**Q7 — Channel heads and realtime (REBUILD-MAP §6 invariant 2).** Do channel principals get any WS
surface in v1? Options: (a) no WS for heads — REST/poll + notifications lane (invariant-1-shaped);
(b) a scoped `ChannelOrder(channel_id, order_id)` room from day one; (c) per-head decision at each
head's own authz council.
**Recommendation: (a) for v1, with (c) as the standing rule** — `Principal::Channel` exists in the
type from S6 so a later grant is additive, but no head is admitted to customer/owner rooms, ever.

**Q8 — Anonymous-order storefront behavior.** No-phone orders get no token and today loop forever on
1008 (linkage A7). Options: (a) client-only fix (stop retrying on 1008); (b) **server sends a
terminal `auth_required` close code the client maps to REST-poll mode**; (c) support anonymous
order-status rooms via a short opaque grant (new surface).
**Recommendation: (b)** — smallest honest contract; (c) only if product wants live status for
anonymous orders.

**Q9 — Upgrade-surface discipline in axum.** Today: any-path upgrade, no Origin check, no upgrade
rate limit (linkage A8). Options: (a) carry verbatim; (b) **single route + Origin allow-list +
per-IP upgrade rate limit**.
**Recommendation: (b)** — cheap, and the Origin check is the standard CSWSH belt even though auth
is required for data.

**Q10 — One generic relay guard vs two ported guards.** The courier and owner guards share their
entire shape but differ in the UNAVAILABLE ceiling (courier: 60s wall + count; owner: none, OR-9).
Options: (a) port both verbatim as separate structs; (b) **one generic `RelayGuard<Policy>` with two
policies** (ceiling on/off), unit tests asserting each policy's ledger-protected behaviors.
**Recommendation: (b)** — the ESLint drift rule's intent becomes a type; divergence requires a new
Policy, which is exactly the review point we want.

**Q11 — `dos_access_token` storage collision (FE).** Customer checkout overwrites the owner/courier
session token in the same browser (linkage A6). Options: (a) separate key (`dos_customer_token`) +
`useWebSocket` picks by surface; (b) keep one key, scope by route. Both are FE-only; sequencing
question is whether this lands pre-S6 on Node (it is live, user-visible today) or with the S6 FE
slice. **Recommendation: (a), pre-S6 on Node** — it is a live defect independent of the rebuild.

**Q12 — GDPR erasure ↔ live sockets.** Erasure does not evict a subscribed customer socket nor
invalidate grants (linkage A2). Options: (a) accept (frames post-erasure carry no PII by claim-check
design); (b) **erasure publishes an eviction for `order:<id>` customer members + deletes the order's
track grants** (belt on Q1c). **Recommendation: (b)** — cheap, and aligns WS behavior with the
GDPR invariant cluster the rebuild is adding (REBUILD-MAP §8).
