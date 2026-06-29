# Breaker Findings — Authz / State Hardening Batch (B7 + N1 / N2 / N4 / N5)

**Round:** 1 (post-PROPOSE re-attack). Reproduced **verbatim** from the Breaker; dispositions live in
`resolution.md`. Source for the Counsel ETHICAL-STOP is `counsel-opinion.md` §3 — folded in below for a
single attack-surface ledger.

---

## Breaker findings (verbatim)

- **[HIGH] N2 fix omits the INSERT site (`push.ts:53`).** The bug is FIVE `.userId` reads: 35
  (set_config), 38 (SELECT), **53 (the INSERT `customer_id` value, `[user.userId, subscription.endpoint, …]`)**,
  72, 75 — the proposal lists only four (35,38,72,75). `customer_devices` is FORCE RLS
  `WITH CHECK (customer_id IN (SELECT app_current_user()))` (`1780421100059:25-27`); a new subscribe (the
  INSERT branch, 100% of first-ever subscribes) would still write `customer_id = undefined → NULL` →
  WITH CHECK fails / NOT NULL violation → throw. Fix must cover line 53.

- **[HIGH] B7 DoD has no regression for the cron's all-tenant path.** `handleGenerate(referenceDate, opts?)`
  makes the pairs-scan conditionally append `AND location_id=$n`; the nightly cron calls with NO locationId
  (`settlement-cron.ts:20`). A common refactor error (always appending `AND location_id=$n` with n=NULL)
  makes the nightly query match ZERO rows → every courier across every tenant silently unsettled, all
  listed DoD still green. Need a regression asserting the locationId-LESS call still settles ≥2 tenants;
  and the WHERE must append the clause ONLY when locationId is provided.

- **[MED] N5-6a OR-clause admits a pre-pickup strike.** 6a allows no_show when order `status='IN_DELIVERY'`
  OR assignment `picked_up`. But `dispatch.ts:46` sets the ORDER to IN_DELIVERY at assignment CREATION
  while the assignment is still `assigned`/`accepted` (pickup is later). So an IN_DELIVERY order with an
  un-picked-up assignment (courier never went to the door) can be struck — contradicting 6a's "a real
  delivery attempt." The `picked_up` (assignment) clause is the real attempt test. Also: the state machine
  ALREADY blocks CONFIRMED/PREPARING/READY (no CANCELLED edge → `updateOrderStatus('CANCELLED')` throws +
  rolls back the strike in the withTenant tx), so the ONLY reachable illegitimate states are PENDING +
  pre-pickup IN_DELIVERY. The proposal's "guard CONFIRMED/PREPARING/READY" overstates; load-bearing =
  block PENDING + require the ASSIGNMENT reached picked_up.

- **[MED] N4's 500 depends on two unproven things.** (1) The owner settlements FE must render a real error
  state on 500 — else white-screen = money-blindness via a different silence (the proposal's own §5.5
  TODO). (2) `decryptPII` is inside the `.map` inside the `try` (`settlements.ts:51`); one corrupt blob
  throws → whole map → 500 on the ENTIRE list. The per-row decrypt try/catch (N4-C) is the ONLY thing
  preventing one bad row blinding all payouts — it must be load-bearing, NOT a droppable "refinement."

- **[LOW-MED] B7 `referenceDate` validator is an either/or with opposite failure modes.**
  `z.string().datetime()` REJECTS date-only `"2026-06-29"`; `z.coerce.date()` ACCEPTS junk (`"2026"`→Jan1,
  bare numbers as epoch) with no range bound. Pick ONE definite contract + a UTC-day normalization + a sane
  range (no far-future/past).

- **[LOW] N1 inline gate on `order-messages.ts`** (multi-role file, can't use the central preHandler) is a
  drift surface for the next customer branch. (Positive: N1 endpoint enumeration is COMPLETE — only
  customer `:orderId` routes are `customer/orders.ts:20,218,258` + 3 order-messages customer branches; no
  account-scoped route; both minters carry orderId; the mismatch-404 short-circuits before any DB query →
  no timing/existence leak.)

- **[LOW] N2 `sub` is the correct RLS value** (`app_current_user()`=`NULLIF(current_setting('app.user_id'),'')::uuid`;
  `issueCustomerToken` sets `sub=customers.id`); push is the SOLE customer-path `.userId` site (signals/auth
  `.userId` are owner-path where the owner token DOES carry userId). Bug is push-only, not systemic.

---

## Counsel ETHICAL-STOP-N5b (verbatim — folded in)

> **ETHICAL-STOP-N5b** (friction → NEEDS-HUMAN/STOP-ETHICS). The grounded line: the manual no_show mark
> BYPASSES `customer_signals` (whose own comment says "Owner acknowledge/dismiss only", `1780421100057:104`)
> to write a raw, unattributed, non-dismissible counter (`signals.ts:224-250`) → the system contradicts its
> own dignity contract. Counsel verified the strike's ONLY effect today is an acknowledgeable `soft_confirm`
> that already discloses the count to the customer (`evaluatePreflight.ts:127-134`) — no auto-deny.
> **Floor = 6b-1:** write an attributable + dismissible `customer_signals` record (owner_id + reason +
> timestamp) — architect-verify it's zero-migration (reuse the FORCE-RLS `customer_signals` table).
> **6b-2** (subject contest channel) deferred to a NAMED trigger: "the first time no_show is consumed by
> anything stronger than acknowledgeable soft_confirm." Counsel's open Q: should a strike attach the
> COURIER's delivery-attempt attestation (the witness is the courier; the button is the owner — hearsay)?
> The single human question: "Must every owner-marked reputation strike be an attributable, dismissible
> record before it may touch a person's counter — or is a raw, unattributed increment acceptable for MVP,
> given the strike's only effect today is an acknowledgeable soft_confirm that already shows the customer
> the count?"
