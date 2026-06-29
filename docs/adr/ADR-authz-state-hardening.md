# ADR: Authz / State Hardening Batch — B7 + N1 / N2 / N4 / N5

**Status:** DRAFT — **round-1-resolved** (RESOLVE folded in the Breaker's 2 HIGH + 2 MED + the
`referenceDate` contract; see `docs/design/authz-state-hardening/resolution.md`). Banks pending Breaker
**re-attack** + the one human N5-6b decision. **No production code in this change.** 6a/B7/N1/N2/N4 are
decided pending re-attack; **N5-6b is held NEEDS-HUMAN (STOP-ETHICS).**
**Date:** 2026-06-29
**Companion design:** `docs/design/authz-state-hardening/proposal.md`
**Security class:** 🔴 tenant-isolation (B7) · customer-authz/BOLA (N1) · RLS-GUC correctness (N2) ·
money-visibility (N4) · order-state-machine + reputation/dignity (N5).
**Extends:** ADR-0013 (binding-scoped authz, the WS/REST mirror pattern), ADR-0010 (error envelope),
ADR-0004 (order-scoped customer token revocation). **Supersedes:** nothing.
**Contradicts no existing ADR.** Each item lands/reverts independently.

## Context

Five independent findings from the adversarial register, all on red-line surfaces, verified at source:

- **B7** — `owner/settlements.ts:301-317` `POST .../:locationId/settlements/regenerate` ignores its
  `:locationId` route param and calls `SettlementCronWorker.handleGenerate(new Date(referenceDate))`,
  which scopes to **all** tenants (`settlement-cron.ts:29-119`, one transaction, platform-wide
  `FOR UPDATE OF ca SKIP LOCKED`). `referenceDate` is an unvalidated `z.string()`; the audit row is
  `actor_kind='system'`, no `actor_id`; rate-limit 5/5min. `requireLocationAccess` proves the owner is
  in `:locationId` (`auth.ts:117-145`) — but the runtime effect ignores that boundary. → cross-tenant
  write + self-inflicted DoS lever.
- **N1** — the customer JWT is **order-scoped** (`orderId` claim, `jwt.ts:117-132`; every minter passes
  a concrete order — `customer/track.ts:75`, `orders.ts:609`; OTP issues a non-JWT opaque token,
  `otp.ts:202`). WS enforces it (`websocket.ts:203-208`). REST does **not** — `customer/orders.ts:49,
  235,283` and `order-messages.ts:70,146,181` check only `customer_id = sub`. → a 7-day token can read/
  rate/cancel/message any order under the same `customer_id` (BOLA).
- **N2** — `customer/push.ts:35,38,**53**,72,75` (**FIVE** sites, incl. the INSERT `customer_id` VALUES
  param at `:53`) uses `user.userId` to set the `app.user_id` RLS GUC + as query/INSERT/UPDATE params,
  but the customer token carries only `sub` (`jwt.ts:126-131`). RLS on `customer_devices` is FORCE with
  `WITH CHECK (customer_id IN (SELECT app_current_user()))` (`1780421100059:25-27`), keyed on
  `app.user_id`, `customer_id NOT NULL`. → GUC `undefined`; the **INSERT (100% of first-ever subscribes)**
  writes `customer_id = NULL` → WITH CHECK / NOT NULL violation → throw; the SELECT/UPDATE match nothing.
- **N4** — `owner/settlements.ts:69-70` `catch { return { payouts: [] }; }` → a decrypt/query failure
  shows the owner "nothing owed" instead of an error. Money-blindness.
- **N5** — `owner/signals.ts:198,211-230` marks `no_show`: it fetches `status` but **never checks** it
  (a never-dispatched PENDING order can be struck), then bumps `customers.no_show_count` directly with
  no owner-attributable/acknowledgeable record and no disclosure to the subject.

**Ground-truth correction (load-bearing).** The steer's premise — "`customers` has no `location_id` →
the strike is GLOBAL/cross-tenant" — is **false**. `customers` is location-scoped: `location_id uuid NOT
NULL`, `UNIQUE (location_id, phone)`, RLS `ENABLE` + `FORCE`, `tenant_isolation USING (location_id IN
(SELECT app_member_location_ids()))` (`1780310074262_orders.ts:8-16,74-77`). Reputation is per
(location, phone); there is no cross-tenant counter. N5's residual is therefore a **state-machine bug**
+ a **per-location dignity/disclosure** gap, not a tenant-isolation leak.

## Decision

| # | Decision | Mechanism | Migration |
|---|----------|-----------|-----------|
| **B7** | Scope regenerate to the route tenant; validate input; attribute the actor. | `handleGenerate(referenceDate, opts?: { locationId? })` appends `AND ca.location_id = $n` to the pairs scan **only when `locationId` is provided** (never `= NULL` — that would zero-match the all-tenant cron scan); threads the acting owner into the audit row; route validates `referenceDate` as **`z.string().date()` (YYYY-MM-DD only) + UTC-day + sane range** → 400; cron path stays whole-fleet (unmodified query). | none |
| **N1** | Order-scoped customer JWT may act **only** on its own order. | Central preHandler on the customer order plugin + inline check in `order-messages.ts` customer branches: `role==='customer' && token.orderId !== params.orderId` → **404**. Keep the `customer_id = sub` checks (defence in depth). | none |
| **N2** | Use `user.sub` (the canonical customer id the RLS expects) at all **5** sites (incl. the INSERT `customer_id` value at `:53`); never set the RLS GUC to a falsy value. | `userId → sub` at `35,38,53,72,75`; guard rejects a falsy resolved id with 500 before `set_config`. | none |
| **N4** | Fail loud on money: query failure → 500 envelope; a single bad PII blob → masked fallback, row + integer amount still shown. | Replace the swallow with `sendError(500, 'INTERNAL', ...)`; wrap **only** per-row `decryptPII` (LOAD-BEARING — it's inside the `.map` inside the `try`, `settlements.ts:47-51`, so one bad blob 500s the whole list without it). **FE error-state on 500 is a required, proven DoD item, not a TODO.** | none |
| **N5-6a** | No-show requires a real delivery attempt; cannot double-strike. | Precondition gates on the **assignment reaching `picked_up`** (`courier_assignments.status IN ('picked_up','out_for_delivery','delivered')`, per `orders.ts:282`) — **NOT** order `status='IN_DELIVERY'` (which `dispatch.ts:46` sets at assignment creation, pre-pickup). Block PENDING + pre-pickup IN_DELIVERY + terminal → 409 `NO_SHOW_NOT_ALLOWED_STATUS`; checked under the existing `FOR UPDATE` lock. (CONFIRMED/PREPARING/READY already blocked by the state machine's missing `→CANCELLED` edge.) | none |
| **N5-6b** | **HELD — NEEDS-HUMAN.** Make the strike attributable + acknowledgeable (reuse `customer_signals`) and decide subject disclosure. | Flag-gated; recommendation 6b-1 now, 6b-2 (disclosure) as a flagged follow-up. | none / additive only |

## Rationale (why these dominate)
- **B7-B over a staff-only guard** — the regenerate action is legitimately owner-facing; the bug is the
  *boundary*, not the *actor*. Parameterizing the aggregate boundary makes the runtime honor the authz
  gate that already passed (`requireLocationAccess`), and the per-location job arg already exists in the
  cron (`settlement-cron.ts:18-21`) as a future scaling lever — schema-rich, runtime-minimal.
- **N1 central gate over per-handler** — mirrors the proven WS invariant (`websocket.ts:204`), fails
  closed for any future customer order route, and is safe because no legitimate account-scoped customer
  JWT exists (verified: only order-scoped minters). 404-not-403 matches the existing no-leak posture
  (`auth.ts:129,137`).
- **N2 `sub`** — `issueCustomerToken({ customerId }) → sub` (`jwt.ts:130`); `app_current_user()` reads
  `app.user_id`; `sub` is the `customers.id` the RLS predicate compares. One-field correctness.
- **N4 fail-loud** — money-blindness (silent "nothing owed") is strictly worse than an error; an owner
  could under-pay a courier on a transient blip. Integer `total_earned` must be shown or errored.
- **N5-6a guard** — a no-show without a delivery attempt is semantically void; the guard is
  unambiguously correct and charter-safe, so it ships independent of the 6b policy decision.

## Consequences
- **Positive:** cross-tenant settlement write eliminated; customer-token BOLA closed to exactly one
  order; push RLS correct; settlement money truthfully shown or errored; no-show requires a real attempt
  and cannot double-strike.
- **Accepted trade-offs:** no fleet-wide owner regenerate route (cron + ops covers it); a transient DB
  blip surfaces as a 500 on the settlements list (correct vs money-blindness); possible orphan
  `customer_devices` rows from the undefined-GUC window (disposable).
- **Forward constraint:** any future cross-order customer view MUST be a separate, explicitly
  account-scoped token/path — never a relaxation of the N1 gate.

## NEEDS-HUMAN / STOP-ETHICS
**N5-6b — reputation recording + subject disclosure (ETHICAL-STOP-N5b, friction not veto).** Whether/how
a person's no-show strike is recorded (attribution, reversibility) and whether the subject is told are
charter "record, don't judge" + dignity policy calls beyond an architect's authority. Escalated to
Counsel + product. Ship **6a only**; gate 6b behind a flag (default off) until decided.

**The single human question (must be answered on the record before N5 is "closed"):** *Must every
owner-marked reputation strike be an attributable, dismissible record before it may touch a person's
counter — or is a raw, unattributed increment acceptable for MVP, given the strike's only effect today is
an acknowledgeable `soft_confirm` that already shows the customer the count?*

- **6b-1 (floor) is verified ZERO-MIGRATION** (architect-confirmed): reuse the FORCE-RLS, location-scoped
  `customer_signals` table (already carries `kind`, `evidence` jsonb, `acknowledged_by_owner_id`,
  timestamp; acknowledge/dismiss handlers already write owner ids — `signals.ts:139,179`). A distinct
  `kind` (e.g. `no_show_manual`) is a code-enum addition (`signals.ts:10`), not a schema change.
- **6b-2 deferral is NAMED, not "later":** a subject contest channel becomes mandatory, shipped
  simultaneously, the **first time `no_show` is consumed by anything stronger than an acknowledgeable
  `soft_confirm`** (escalation toward `hard_block`, any auto-gating, or feeding a feature the subject
  cannot pass through). Today the only effect is the acknowledgeable `soft_confirm` at
  `evaluatePreflight.ts:127-134`. This trigger is recorded so the deferral cannot become silent.
- **Open question for the human (courier-as-witness):** the owner presses the button but the first-hand
  witness is the courier; `assignment picked_up` is "a delivery was attempted," not "the courier attested
  no-answer-at-door." Should a strike attach/require the courier's attestation as evidentiary ground
  (owner assertion is structurally hearsay)? If 6b-1 ships, its `evidence` jsonb carries it. Deferred with
  6b-2. The deferral of all of 6b (Counsel §4 steel-man) is a legitimate human choice **if recorded**.

## DoD (red → green) — per item
- **B7:** RED (scoped) — owner-of-A regenerate writes a payout for tenant B in-period. GREEN (scoped) —
  no B row touched; A generated; `referenceDate` `"not-a-date"`/`"2026"`/datetime/far-future → 400,
  `"2026-06-29"` accepted; manual audit row has `actor_kind='owner'` + caller `actor_id`. **RED+GREEN
  (unscoped cron — load-bearing):** seed delivered+cash assignments under **≥2 tenants** in-period; call
  `handleGenerate(date)` with **no locationId** → a payout is generated for **each** tenant (goes red on
  the unconditional-`AND location_id=NULL` refactor → zero rows → all tenants silently unsettled).
- **N1:** RED — track-token for O1 reads/rates/messages O2 (same `customer_id`) → 200. GREEN — cross-
  order → 404; own order O1 → 200.
- **N2:** RED (cold INSERT — load-bearing) — a customer with no existing device row `POST /push/subscribe`
  throws on `customer_id = NULL` (WITH CHECK / NOT NULL); RED (warm) — leaves no RLS-readable row for
  `sub`. GREEN — cold subscribe INSERTs `customer_id = sub` (no violation), warm re-subscribe UPDATEs;
  falsy id → 500, no `NULL`-GUC write.
- **N4:** RED — forced query/decrypt failure returns `200 { payouts: [] }`. GREEN — query failure → 500;
  **one** bad PII blob among ≥2 rows → 200 with **all** rows present (bad row masked `A***` + integer
  amount); FE renders a visible error state on 500 (Playwright `toBeVisible`), not an empty list.
- **N5-6a:** RED — mark-no-show on a PENDING order **and** on a pre-pickup IN_DELIVERY order (assignment
  still `assigned`/`accepted`) → 200 + `no_show_count++`. GREEN — both → 409 `NO_SHOW_NOT_ALLOWED_STATUS`,
  counter unchanged; an order whose assignment reached `picked_up` succeeds once (replay → 409).
- **N5-6b:** conditional on the human decision (see proposal §6b).

## Verification
Each DoD asserts a fail-when-wrong condition (Mandatory Proof Rule). No item is "done" without a
red→green guardrail (integration/E2E) + a `docs/regressions/REGRESSION-LEDGER.md` row when the fix
lands. No production code is written in this ADR.
