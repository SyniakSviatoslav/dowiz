# B3 (NOBYPASSRLS) flip — GUC-coverage gap audit + remediation roadmap

**Date:** 2026-06-30. **Verdict: DO NOT FLIP `dowiz_app` NOBYPASSRLS yet.** Two parallel read-only audits
(request routes + services/workers) cross-verified that the app today **fundamentally relies on BYPASSRLS** —
tenant isolation rests on app-level WHERE clauses, not RLS (FORCE-RLS is currently a no-op on the hot path).
Flipping now would break ~40 route surfaces (~26 files) + ~15 background workers. B3 is a **remediation
project**, not a prerequisite flip.

Staging fact: the operational role is **`dowiz_app`** (`rolbypassrls=t`), NOT `deliveryos_api_user` (legacy,
nologin) that the staged migration targets. The migration must be retargeted before it could ever do anything.

## The worst gaps (would break on flip)
- 🔴 **Public checkout rolls back** — `order-persistence.ts:108/124/146` INSERTs `velocity_events`,
  `order_item_modifiers`, `customer_track_grants`; none have an anonymous policy and the anon path cannot set
  `app.user_id` → WITH-CHECK violation / `current_setting` throw → whole POST /orders transaction aborts. Every
  diner. Highest blast radius.
- 🔴 **All auth dies** — `users`, `auth_refresh_tokens`, `ops_worker_heartbeat` are ENABLE+FORCE with **zero
  policies** (mig `1780421100065`, by design relying on bypass). Login/refresh/logout/courier-activate are
  pre-auth → no GUC possible → 0-row/denied.
- 🔴 **Every order transition 409s** — `orderStatusService.updateOrderStatus` `UPDATE orders` matches 0 rows
  from courier (sets `app.current_tenant`), telegram-webhook (autocommit resets the GUC), and owner/dashboard
  (no GUC). `orders` has only a member (`app.user_id`) policy — no courier/owner/anon-UPDATE path. Cash-as-proof
  (`deliveryCompletion.ts`) + dispatch (`dispatch.ts` throws 500 on non-missing-ok courier reads) included.
- 🔴 **Owner endpoints 401 globally** — `get-owner-location.ts:13` reads `memberships` on a raw pool before any
  `withTenant` → 0 rows → null → unauthorized. Gates categories/products/promotions/media/import/onboarding/…
- 🟠 **Storefront shell/theme** lose per-tenant data (`organizations` JOIN, `location_themes` member-only → 0
  rows); **anti-abuse throttle fails OPEN** (`velocity_events` read 0 rows); notifications/push/telegram-connect
  silent (`owner_notification_targets`/`telegram_connect_tokens` are `TO authenticated`, inapplicable to the
  operational role); ~15 background workers (signal-raiser, shift, dispatch, eta) read FORCE-RLS tables with no
  GUC.

## Cross-cutting root causes (fixing these covers most gaps)
1. **Anonymous-checkout policies** on `velocity_events`, `order_item_modifiers`, `customer_track_grants` (+ a
   coherent anon-write path for the POST /orders txn).
2. **Zero-policy auth tables** (`users`/`auth_refresh_tokens`/`ops_worker_heartbeat`) → an ops/role policy
   (pre-auth, no GUC possible).
3. **`memberships`/`organizations` have no anon/ops read** → breaks `getOwnerLocationId`, auth-refresh role
   re-derivation, order-messages owner checks, storefront JOIN. One read policy (or route via a DEFINER fn)
   fixes many at once.
4. **`orders` has no courier/owner-write policy** — courier/owner/telegram transitions set the wrong GUC
   (`app.current_tenant`) or none → 0-row. Needs a courier-aware `orders` policy or run transitions under an
   authorized `app.user_id`.
5. **Courier-table reads use non-missing-ok `current_setting('app.current_tenant')::uuid`** (no `,true`) →
   hard 500 when unset. Add `,true` + null-guards.
6. **`TO authenticated`-only policies** don't apply to the operational role → re-key to `app_current_user()`/ops.
7. **~15 background workers** must set the GUC explicitly (no request context).
8. **Pre-flip verify on staging:** `dowiz_app`'s `authenticated` membership + that it retains all DML grants.

## Recommended path
- **Decouple B3 from payments.** Payments does NOT need the global flip: under today's bypass model the Plisio
  webhook INSERTs fine; we give the new `payments`/`payment_events` tables **correct FORCE-RLS policies as
  defense-in-depth** (ready for when B3 lands) without blocking on app-wide remediation. → payments build can
  proceed now.
- **B3 = its own scoped, council-gated, phased remediation** (this audit is the input): policies first
  (root-causes 1-6, additive + safe under current bypass — they're no-ops until the flip), then workers
  (root-cause 7), then the flip on staging behind full lifecycle E2E, then prod. Each phase verifiable
  red→green via `verify:rls` + the probes in the operator-handoff.
- Until B3 lands, FORCE-RLS remains a no-op (the pre-existing condition flagged in the adversarial audit as
  B3/B6 cross-tenant risk) — isolation continues to rest on app WHERE clauses. Net security unchanged by
  deferring; the remediation is the fix.
