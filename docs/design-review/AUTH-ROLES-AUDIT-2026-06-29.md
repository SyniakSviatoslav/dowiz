# Auth + system-role coherence audit — dowiz/DeliveryOS (2026-06-29)

Internal read-only audit of the whole auth/identity/role model for weak spots + contradictions.
Headline: **two structural fault lines, masked today only by `deliveryos_api_user` holding BYPASSRLS.**
The planned NOBYPASSRLS hardening (B3) **unmasks both** — so B3 is NOT shippable as currently staged.

## The two fault lines (drive ~half the findings)
1. **Identity field split.** `AuthToken` (shared-types/legacy.ts:163-174): owner has `userId`+`sub` (equal at mint, jwt.ts:49); **courier/customer have only `sub`, no `userId`.** So `request.user.userId` is `undefined` on every courier/customer handler. Some code defends (`?? sub`), some doesn't → silent wrongness.
2. **RLS GUC split.** Two disjoint contexts: `app.user_id` (a `users.id`, member tables) vs `app.current_tenant` (a `locations.id`, courier/tenant tables). Routes set one while the table reads the other — harmless under BYPASSRLS, a leak-or-break under NOBYPASSRLS.

## CRITICAL — these BLOCK the B3 NOBYPASSRLS flip
- **C1 — anonymous RLS policies fail OPEN.** `anonymous_orders/customer-anonymous-update/customer-rls` predicates are `app_current_user() IS NULL` → TRUE for **every row in every tenant** on any connection that hasn't set `app.user_id` (every `courier/*` route, push worker, funnel, telegram-webhook). Under NOBYPASSRLS, *forgetting* `app.user_id` = all-tenant SELECT on orders/order_items/customers + all-tenant UPDATE on customers. The "safety net" becomes a cross-tenant siphon.
- **C2 — `app_member_location_ids()` is an UNPINNED SECURITY DEFINER** (core-identity.ts:76-78) — the keystone helper behind every member-context policy. No `SET search_path` → a caller that can `SET search_path` to a spoofed `memberships` makes it return attacker-chosen `location_id`s = authz bypass through the exact predicate meant to enforce isolation. (This is B3/MIG-ITEM1, but it's THE keystone — highest-leverage single fix.)

## HIGH
- **H1 — owner WS `order:` room authz drops `status='active'`** (websocket.ts:97-100; the `location:` sibling at :88 has it). A removed/downgraded owner keeps a live order WS feed — the ADR-0004 insider-removal window, still open on this one read. *(Adjacent to ADR-0013 / the courier-realtime-authz PR — same file.)*
- **H2 — spa-proxy trusts JWT `activeLocationId` without the ADR-0004 re-check** (spa-proxy.ts:52/107 return it directly; re-verify only on the absent path — inverse of the canonical `get-owner-location.ts`). Gates the whole spa-proxy owner WRITE surface (brand/settings/courier-invites/onboarding). #1 churn hotspot.
- **H3 — admin routes read cross-tenant PII on the BYPASSRLS pool, no tenant scope** (admin/fallback.ts:13 all-locations name/slug/phone; notification-audit optional unvalidated locationId; backups platform-wide). Concrete data-exposure facet of B4.
- **H4 — courier lifecycle BREAKS (+ partially leaks) under NOBYPASSRLS** — `courier/assignments.ts` sets only `app.current_tenant` but JOINs `locations` + `UPDATE orders` (member tables) → 0 rows (break); orders/customers reads "work" only via C1's fail-open. Same in shifts/settlements.
- **H5 — push worker** JOINs `locations` with no GUC (→ 0 rows, notifications silently never sent) then sets `app.user_id` to a `customers.id` (wrong GUC value).
- **H6 — per-customer fraud/velocity scoping is silently DEAD** (orders.ts:240,292 — `customerId = customer ? user.userId : undefined`; customer tokens have no `userId` → `computeSignals({customerId: undefined})` for every customer → throttle degrades to phone/IP; same fingerprints idempotency at :195). Fails toward "no throttle."

## MEDIUM / LOW (selected)
- M1 `orders.ts` owner/courier access is RLS-only while `order-messages.ts` adds explicit membership checks — two strategies on one resource; courier hits `withTenant(…, userId=undefined)` → always 404. M2 owner-membership resolver reimplemented 5×, spa-proxy copy divergent (→H2). M3 `couriers.ts` takes `locationId` from BODY (bypasses `requireLocationAccess`). M4 multi-location owner resolution non-deterministic (no `ORDER BY` in the per-request `LIMIT 1` helpers). M5 `customers` cross-tenant UPDATE via `anonymous_update`. M6 older menu-read DEFINER fns miss `search_path`.
- L1 `/api/admin` excluded from `AUTH_PREFIXES` (silently public if a future admin route forgets a hook). L2 customer `orderId` claim decorative (=N1). L3/L4/L5 `userId`-vs-`sub` muddle edge cases.

## Coherent (don't "fix")
Mint invariant `sub = sub || userId` (owner sub==userId); dev-token crypto segregation; `get-owner-location.ts` is the canonical ADR-0004 pattern; courier liveness enforced per-request (`courier_sessions`/`courier_locations` EXISTS); `requireLocationAccess` family uniform; `order-messages.ts` belt-and-suspenders; `ops-auth.ts` distinct header-secret identity; the dual-GUC policies (`courier_assignments` …073) + provisioning definers pin `search_path` correctly.

## ⚠️ Bottom line — B3 must be RE-SCOPED before the NOBYPASSRLS flip
Do **not** flip `deliveryos_api_user` to NOBYPASSRLS until, at minimum:
1. **C1** — narrow the `anonymous_select`/`anonymous_update` predicates so a no-user session can't be table-wide TRUE (add tenant scope / `WITH CHECK`).
2. **C2** — pin `app_member_location_ids()` `search_path` (+ the M6 menu-read fns).
3. **H4/H5** — courier routes + push worker either set `app.user_id`, or the member tables they touch (`orders`/`locations`/`customers`) get a dual courier policy mirroring `courier_assignments` (…073).

Otherwise B3 converts today's invisible-but-bounded state into **cross-tenant leaks** (C1 fail-open) or a **broken courier/notification flow** (H4/H5 fail-closed). This is a material correction to the MVP plan's "just apply the staged pg-privilege migrations" step → that step alone would leak or break. The standalone WS courier authz (ADR-0013) is already NOBYPASSRLS-sound by construction (sets `app.current_tenant`), so it's safe either way.
