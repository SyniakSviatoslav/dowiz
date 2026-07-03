# set_config site inventory — verified 2026-07-03 (companion to proposal.md §1)

Canonical correct shape: checkout client → `BEGIN` → `set_config(k, v, true)` → queries → `COMMIT` → release.
Verdicts: **OK** (canonical) · **NO-OP** (`set_config(...,true)` in autocommit — GUC dies with its implicit txn) ·
**SESSION-LEAK** (`set_config(...,false)` persists on the pooled physical connection after release) · **dev-only**.

**Totals: 49 sites — 32 OK · 11 NO-OP · 2 SESSION-LEAK · 4 dev-only.**

## NO-OP (autocommit) — 11 sites; silent 0-row/deny at the NOBYPASSRLS flip

| Site | GUC | Post-flip blast |
|---|---|---|
| `apps/api/src/notifications/workers/index.ts:122` | app.user_id | every customer status push dies (audit H4-rls) — `customer_devices` is `customer_owns` FORCE-RLS via mig 077 RC6 |
| `apps/api/src/routes/owner/couriers.ts:148` | app.current_tenant | owner live-map handler reads 0 rows (siblings :26/:207 are correct — same file, coin-flip pattern) |
| `apps/api/src/routes/courier/assignments.ts:111` | app.current_tenant | GET /assignments/:id — the ONLY one of 9 handlers in the file missing BEGIN |
| `apps/api/src/routes/courier/settlements.ts:25` | app.current_tenant | **worst shape**: `db.query` is POOL.query — the GUC statement and the payout SELECT run on *different connections* even pre-flip |
| `apps/api/src/routes/courier/settlements.ts:59` | app.current_tenant | same pool.query shape |
| `apps/api/src/routes/courier/settlements.ts:75` | app.current_tenant | same; in-code comment ("Set RLS context for items query too") is doubly wrong |
| `apps/api/src/routes/telegram-webhook.ts:281` | app.current_tenant | precedes `updateOrderStatus` — telegram order actions silently no-op post-flip |
| `apps/api/src/routes/telegram-webhook.ts:411` | app.current_tenant | same handler/client |
| `apps/api/src/routes/telegram-webhook.ts:631` | app.current_tenant | second handler (connect at :493) |
| `apps/api/src/routes/customer/push.ts:35` | app.user_id | push subscribe dies post-flip (customer_devices FORCE) |
| `apps/api/src/routes/customer/push.ts:72` | app.user_id | push unsubscribe dies post-flip |

## SESSION-LEAK — 2 sites; latent cross-tenant identity bleed TODAY (worse post-flip)

| Site | Notes |
|---|---|
| `apps/api/src/routes/spa-proxy.ts:771` | `set_config('app.user_id', $1, false)` on `db.connect()` client; finally at :819-820 releases with no RESET — GUC persists on the physical backend. Comment at :766-768 acknowledges the persistence; the persistence is the bug. |
| `apps/api/src/routes/owner/onboarding.ts:75` | same shape; the session GUC is set in autocommit BEFORE the menu-seed `BEGIN` at :102; release-only finally at :138-139 |

Both write the caller's own userId → a later borrower of that physical connection that relies on
`app.user_id` without setting it transacts under a stale identity.

## OK (canonical) — 32 sites

`payments-webhook.ts:41` · `owner/couriers.ts:26,207` · `owner/signals.ts:207` (via withTenant) ·
`courier/assignments.ts:81,139,190,251,317,431,496,544` · `lib/storefrontService.ts:57,85` (caller-supplied
client, BEGIN by caller) · `lib/notificationPrefsService.ts:37` · `workers/courier-offer-sweep.ts:93,128,219` ·
`spa-proxy.ts:458` · `courier/shifts.ts:24,77,120,193,337` · `lib/courier-room-authz.ts:47` ·
`workers/courier-events.ts:30` · `modules/acquisition/provisioning.ts:150` (app.provision_token) ·
`routes/public/funnel.ts:60` · `workers/courier-dispatch.ts:38,49` · `courier/auth.ts:183` ·
`packages/platform/src/auth/tenant.ts:11` (the canonical helper itself).

## dev-only — 4 sites (shape OK)

`routes/dev/mock-auth.ts:136,137,508,509`.

## Authoritative in-code statement of the hazard

`apps/api/src/lib/courier-room-authz.ts:9-13`: "…the predicate MUST run inside an explicit
BEGIN…COMMIT tx that sets the tenant GUC … **a bare `set_config(...,true)` under autocommit dies
before the SELECT**. This makes the gate correct under BOTH BYPASSRLS (today) and the branch's
NOBYPASSRLS hardening…"

## Canonical helper (to be generalized per proposal §1)

`packages/platform/src/auth/tenant.ts:3-21` — `withTenant(pool, userId, fn)`: BEGIN →
`set_config('app.user_id',$1,true)` → fn → COMMIT/ROLLBACK → release. Sets `app.user_id` only;
courier/webhook lane (`app.current_tenant`) has no helper today — hence the 40 hand-rolled copies.
