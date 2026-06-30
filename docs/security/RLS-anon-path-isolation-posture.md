# Anon-path isolation posture under NOBYPASSRLS (HIGH-1 — accepted 2026-06-30)

**Decision (operator):** ACCEPT + DOCUMENT. The anonymous (pre-customer-identity) request path's cross-tenant
isolation rests on **application-layer `WHERE` clauses + UUID-unguessability**, not on RLS — and that is the
intended, accepted model. Source: the 2026-06-30 policy/role security re-audit, finding HIGH-1.

## What HIGH-1 is
The baseline policies on `orders`, `customers`, `order_items` include tenant-GLOBAL anon policies:
- `orders.anonymous_select` / `customers.anonymous_select` / `order_items.anonymous_select` → `USING (app_current_user() IS NULL)`
- `customers.anonymous_update` → `USING (app_current_user() IS NULL)`

The anon checkout (`order-persistence.ts`) MUST run with `app.user_id` unset (it has no customer identity yet —
it relies on the `anonymous_insert` policies' `app_current_user() IS NULL` check). So once `dowiz_app` is
NOBYPASSRLS, any GUC-less connection that touches these tables sees/мутує **every tenant's** rows at the RLS
layer — isolation is provided only by the app's `WHERE id = $1` / `WHERE phone = $1` clauses and the
unguessability of the UUID order/customer ids.

## Why accepted (not "scope anon policies to id-only")
- The anon path has **no member identity** to scope RLS by — there is nothing to key a per-tenant predicate on
  at the moment of an anonymous checkout. A per-row GUC (e.g. `app.order_id`) would be a behavior-sensitive
  baseline redesign with real breakage risk to checkout + order-tracking, for marginal gain over the existing
  app-layer guard.
- The **authenticated** owner/courier paths ARE fully RLS-isolated post-flip (member / `app.current_tenant`
  policies) — that is where the flip delivers its value. HIGH-1 is strictly about the anon surface.
- UUID order/customer ids are not enumerable; every anon read/write is already `WHERE`-scoped to a specific id
  the caller already holds. The practical exposure equals the pre-flip posture (RLS was a global no-op under
  BYPASSRLS), so the flip does not regress the anon path — it simply does not (cannot) harden it.

## Standing guardrail (to keep the posture honest)
Any NEW anon-context (GUC-less) read/write of `orders`/`customers`/`order_items` MUST be `WHERE`-scoped to a
specific caller-held id — never an unbounded scan/update. This is the invariant that the app-layer isolation
depends on; a future unbounded anon query would silently become a cross-tenant leak. Revisit if/when an anon
identity primitive (e.g. a signed per-order token GUC) is introduced — at which point scope-to-id becomes
feasible and this acceptance should be re-evaluated.
