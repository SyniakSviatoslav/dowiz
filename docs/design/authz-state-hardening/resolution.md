# Resolution — Authz / State Hardening Batch (B7 + N1 / N2 / N4 / N5)

**Round:** 1 — RESOLVE. **Status after this round:** DRAFT, **round-1-resolved**. Banks pending
(a) Breaker re-attack and (b) the one human decision on N5-6b (STOP-ETHICS). **No production code.**
**Inputs:** `breaker-findings.md` (verbatim), `counsel-opinion.md` §3. **Outputs of this round:**
this file + updated `proposal.md` + updated `docs/adr/ADR-authz-state-hardening.md`.

All seven Breaker findings + the Counsel ETHICAL-STOP were re-verified at source before disposition
(see "Verification" per row). The steer's directive is honored exactly: **fix the 2 HIGH + 2 MED
structurally; one definite `referenceDate` contract; accept N1 inline-drift with an owner + a one-line
shared-helper note; pre-stage 6b-1 as the floor (zero-migration verified) + route to STOP-ETHICS.**

---

## Disposition table

| # | Severity | Disposition | Owner |
|---|----------|-------------|-------|
| N2 INSERT site (`push.ts:53`) | HIGH | **FIX (structural)** — 5th site + INSERT-path regression | data/auth |
| B7 unscoped-cron regression | HIGH | **FIX (structural)** — conditional clause + ≥2-tenant cron regression | platform-ops |
| N5-6a pre-pickup IN_DELIVERY strike | MED | **FIX (structural)** — gate on assignment `picked_up`, not order status | dispatch/state |
| N4 per-row decrypt + FE-error-state | MED | **FIX (structural)** — N4-C promoted load-bearing + FE-state DoD | owner-finance UX |
| B7 `referenceDate` validator either/or | LOW-MED | **FIX** — ONE contract (`z.string().date()` + UTC-day + range) | platform-ops |
| N1 inline-gate drift on `order-messages.ts` | LOW | **ACCEPT-RISK (owner) + shared-helper note** | customer-auth |
| N2 `sub` correct / push-only scope | LOW | **ACCEPT (confirmation, no action)** | data/auth |
| Counsel ETHICAL-STOP-N5b (6b) | STOP-ETHICS | **DEFER-FLAG → NEEDS-HUMAN** (6b-1 pre-staged as floor) | Counsel + product |

---

## 1. [HIGH] N2 — INSERT site `push.ts:53` (FIX, structural)

**Verified.** `push.ts` has **five** `user.userId` reads, not four. Confirmed at source: `:35`
(`set_config('app.user_id', …)`), `:38` (SELECT existing), **`:53`** (the INSERT — `user.userId` is the
**first VALUES param**, i.e. the `customer_id` column value, `[user.userId, subscription.endpoint, …]`),
`:72` (set_config, unsubscribe), `:75` (UPDATE WHERE, unsubscribe). The proposal's §4.1 / §0.1 table /
ADR row all enumerate only `35,38,72,75`.

**Why it bites worse than the SELECT miss.** `customer_devices` is FORCE RLS with
`WITH CHECK (customer_id IN (SELECT app_current_user()))` (`1780421100059:25-27`) and `customer_id NOT
NULL`. The INSERT branch is **100% of first-ever subscribes** (the SELECT finds nothing → fall to
INSERT). With `user.userId` undefined the INSERT writes `customer_id = NULL` → NOT NULL / WITH CHECK
violation → throw. So the *primary* (cold-start) path is the one the omission leaves broken.

**Fix.** N2-A substitutes `user.sub` at **all five** sites (35, 38, **53**, 72, 75) — not four. The
"never `set_config` an RLS GUC to a falsy value" guard (`:35`, `:72`) stays. Both `set_config` and the
INSERT/SELECT/UPDATE params now carry the same `customers.id` the RLS predicate compares.

**DoD delta (the missing regression).** Add an explicit **INSERT-path** (cold subscribe) case to N2's
DoD: a customer with **no** existing `customer_devices` row calls `POST /push/subscribe` → a row is
INSERTed with `customer_id = token.sub`, readable back under that customer's RLS context, with no NOT
NULL / WITH CHECK violation. (The prior DoD's "re-subscribe UPDATEs" case only exercises the warm path
and would stay green even with the INSERT site unfixed.)

**Reversibility.** Pure runtime field swap, no migration; revert the file diff.

---

## 2. [HIGH] B7 — unscoped cron regression (FIX, structural)

**Verified.** `handleGenerate(referenceDate)` today takes one arg; the pairs scan
(`settlement-cron.ts:38-45`) has **no** location filter; the nightly cron calls it with **no** locationId
(`settlement-cron.ts:20`). The refactor adds `opts?: { locationId? }`. The Breaker's failure mode is real:
the trivial-wrong refactor appends `AND location_id = $n` **unconditionally** with `n` bound to
`undefined`/`NULL`; `location_id = NULL` is never true → the nightly all-tenant scan matches **zero
rows** → every courier across every tenant **silently unsettled** while every listed DoD stays green
(the per-tenant B7 test only exercises the *scoped* path).

**Fix (structural — two parts).**
1. **Conditional clause, not unconditional.** The WHERE appends `AND ca.location_id = $n` **only when
   `opts?.locationId` is provided**; the no-arg cron path emits the *unmodified* whole-fleet query. Build
   the SQL by conditional concatenation (param pushed only when set), the same pattern already used for
   the settlements list filters (`owner/settlements.ts:39-42`) — never a `location_id = $n` with a NULL
   bind.
2. **DoD regression for the cron path** (the missing guardrail): an integration test seeds delivered+cash
   assignments under **≥2 distinct tenants** in-period, calls `handleGenerate(referenceDate)` with **no
   locationId**, and asserts a `courier_payouts` row is generated for **each** tenant. This fails red on
   the unconditional-clause refactor (zero rows) and stays green only when the clause is truly
   conditional. It runs alongside the existing scoped B7 test (owner-of-A does not touch tenant B).

**DoD delta.** B7 now has **two** regressions, not one:
- *(existing)* scoped: owner-of-A regenerate writes **no** tenant-B row; A is generated.
- *(new, load-bearing)* unscoped: `handleGenerate(date)` with no locationId settles **≥2 tenants**.

**Reversibility.** No migration; the `opts` arg is additive and optional, cron call-site unchanged in
shape; revert the worker + route diff.

---

## 3. [MED] N5-6a — pre-pickup IN_DELIVERY strike (FIX, structural)

**Verified.** `dispatch.ts:46` calls `updateOrderStatus(client, orderId, locationId, 'IN_DELIVERY', …)`
at assignment **creation**, while the just-inserted assignment is `status='assigned'` (`:49-51`); pickup
is a later courier action. So an order can be `IN_DELIVERY` with an **un-picked-up** assignment — the
courier never went to the door — yet 6a's `status='IN_DELIVERY' OR assignment picked_up` OR-clause would
**admit the strike**, contradicting 6a's own "a real delivery attempt" rationale.

**Also verified (scope tightening).** The order state machine has no `IN_DELIVERY→CANCELLED` problem for
the *other* states: from CONFIRMED/PREPARING/READY the mark would call `updateOrderStatus('CANCELLED')`
on a non-existent edge → throws → rolls back the strike inside the `withTenant` tx. So those states are
**already** blocked by the machine. The **only reachable illegitimate** states are: **PENDING**, and
**pre-pickup IN_DELIVERY**. The proposal's "guard CONFIRMED/PREPARING/READY" overstates the surface.

**Fix (structural — the load-bearing predicate).** Drop the `OR order.status='IN_DELIVERY'` admission.
Gate the no-show on the **assignment lifecycle**: require an assignment for the order that reached
**`picked_up`** (i.e. `courier_assignments.status IN ('picked_up','out_for_delivery','delivered')` per
the lifecycle), mirroring the cancel handler's `picked_up` gate (`orders.ts:282`). `picked_up` is the
real "courier attempted the delivery" fact; order `status` is not. Block PENDING (no assignment) and
pre-pickup IN_DELIVERY (assignment exists but `< picked_up`) → 409 `NO_SHOW_NOT_ALLOWED_STATUS`. The
terminal-state idempotency guard (no second strike on an already-terminal order) stays, read under the
existing `FOR UPDATE` lock (`signals.ts:211`).

**DoD delta.** Replace the order-status RED case with the **pre-pickup IN_DELIVERY REJECTED** case:
- RED: mark-no-show on an `IN_DELIVERY` order whose assignment is still `assigned`/`accepted`
  (pre-pickup) returns 200 + `no_show_count++`.
- GREEN: same call → **409 `NO_SHOW_NOT_ALLOWED_STATUS`**, counter unchanged; a no-show on an order whose
  assignment reached `picked_up` still succeeds **exactly once** (replay → 409). PENDING (no assignment)
  → 409 as before.

**Reversibility.** Pure precondition predicate; no migration; revert the handler diff.

---

## 4. [MED] N4 — per-row decrypt + FE error state (FIX, structural — promote N4-C)

**Verified.** `decryptPII` is invoked **inside the `.map` inside the `try`** (`settlements.ts:47-51`):
`res.rows.map(r => { const name = … decryptPII(r.full_name_encrypted) …})` all under the single
`try { … } catch { return { payouts: [] } }`. One corrupt cipher blob throws inside the map → the whole
map throws → the catch swallows → `200 { payouts: [] }` for the **entire** list. So N4-C (per-row decrypt
try/catch) is **not** a droppable "refinement" — it is the **only** thing preventing one bad PII blob
from blinding **all** payouts, money-blindness through a second door.

**Fix (structural — promote two things from "refinement" to load-bearing requirements).**
1. **N4-C is load-bearing, not optional.** Wrap **only** the per-row `decryptPII` in its own try/catch
   yielding the existing empty-name fallback `courierNameMasked: 'A***'` (`:55`); the row + integer
   `total_earned` are always emitted. A bad blob degrades the *name mask*, never the *amount*, never the
   *list*. The outer catch is reserved for **query** failure → `sendError(500, 'INTERNAL', …)` (N4-A).
2. **FE error-state is a DoD item, not a TODO.** §5.5's "confirm the owner settlements page renders an
   error state on 500" is promoted from a parenthetical flag to a **required, proven** DoD line — else
   the 500 becomes white-screen = money-blindness via a different silence (Breaker + Counsel both pin
   this). This is an **inline-fix** (Task-Exit class), owner = owner-finance UX; it must ship in the same
   change as the API 500, with a Playwright assertion on the rendered error state.

**DoD delta.**
- *(new, load-bearing)* one-corrupt-row case: inject **one** bad cipher blob among ≥2 rows → response is
  **200** with **all** rows present, the bad row's `courierNameMasked='A***'` and correct integer
  `totalEarned`; the good rows decrypt normally. (Distinct from the query-failure → 500 case.)
- *(new, FE)* on a forced 500 the owner settlements page renders a visible error state (Playwright
  `expect(errorState).toBeVisible()`), **not** an empty "all settled" list.

**Reversibility.** No migration; revert the handler diff + the FE component diff independently.

---

## 5. [LOW-MED] B7 `referenceDate` — ONE definite contract (FIX)

**Verified the either/or.** `z.string().datetime()` rejects date-only `"2026-06-29"` (the natural input
for a *daily* settlement period); `z.coerce.date()` accepts junk — `"2026"`→Jan 1, bare numbers as epoch
ms — with no range bound. The proposal offered both ("`z.coerce.date()` or `z.string().datetime()`"),
which is an undecided contract with opposite failure modes.

**Decision — ONE contract.** `referenceDate` is a **calendar day** (it selects a settlement period via
`getSettlementPeriodBoundaries`), so:
1. **Shape:** `z.string().date()` — accepts **`YYYY-MM-DD`** only (rejects datetimes, epochs, `"2026"`,
   and all junk). Date-only is the honest type for a day-keyed period.
2. **Normalization:** interpret as a **UTC calendar day** (parse to `…T00:00:00.000Z`) before handing to
   `getSettlementPeriodBoundaries`, so period boundaries are deterministic and TZ-independent.
3. **Range bound:** reject far-future / far-past — accept only within a sane window
   (e.g. `[today-90d, today+1d]` UTC; exact bound is an implementation detail, but it MUST exist) →
   400 `VALIDATION_FAILED` envelope (ADR-0010) before any DB work. No unbounded epoch, no Jan-1-from-junk.

**DoD delta.** Add to B7: `"2026-06-29"` → accepted (period computed); `"not-a-date"`, `"2026"`,
`"2026-06-29T10:00:00Z"`, a far-future day → **400** before any DB write.

**Reversibility.** Validation-only; revert the schema line.

---

## 6. [LOW] N1 — inline-gate drift on `order-messages.ts` (ACCEPT-RISK + shared-helper note)

**Accepted, per steer.** `order-messages.ts` is a **multi-role** plugin (owner/courier/customer branches
share the file), so it cannot mount the customer-only central preHandler (N1-A); its three customer
branches (`:70,146,181`) must carry the **inline** `token.orderId === params.orderId` check (N1-B). That
inline check is a **drift surface**: a future 4th customer branch added to this file could forget it and
silently reopen the BOLA on that route.

**Accept-risk with an owner + a one-line mitigation (no over-build now).**
- **Owner:** customer-auth.
- **Shared-helper note (the one line):** extract the predicate into a single exported guard
  `assertCustomerOwnsOrder(request)` (returns/throws the 404) and call it at the top of every customer
  branch in `order-messages.ts` (and reuse it inside the N1-A preHandler), so "the next customer branch"
  copies a **named call**, not a re-derived inline condition. This is a refactor convenience, not a new
  abstraction layer — it does not change the runtime gate, only its drift resistance. Lighter than a
  lint rule; if drift recurs, **escalate** to a guardrail (an eslint-plugin-local rule asserting every
  `role==='customer'` branch in this file calls the helper). Defer the lint rule until there is a second
  occurrence (YAGNI).

**Positive (recorded, no action):** the Breaker confirmed N1's endpoint enumeration is **complete** —
the only customer `:orderId` routes are `customer/orders.ts:20,218,258` + the 3 `order-messages` customer
branches; no account-scoped route exists; both token minters carry `orderId`; the mismatch-404
short-circuits **before** any DB query → no timing/existence side-channel. So the gate, once placed, is
total.

**Reversibility.** N/A (accept-risk; the helper, if added, is a pure refactor revertible by diff).

---

## 7. [LOW] N2 `sub` correctness / push-only scope (ACCEPT — confirmation)

**No action — Breaker confirmation, banked.** `app_current_user() =
NULLIF(current_setting('app.user_id'),'')::uuid`; `issueCustomerToken` sets `sub = customers.id`; so
`sub` **is** the value the RLS predicate compares — N2-A's substitution is correct. And `push.ts` is the
**sole** customer-path `.userId` site: the other `.userId` reads (signals / auth) are **owner-path**,
where the owner token genuinely carries `userId`. So the bug is **push-only, not systemic** — the fix
does not need to sweep other `.userId` sites, and must **not** blindly rewrite owner-path `.userId`
(that would break owner RLS). Scope stays exactly `push.ts`.

---

## 8. [STOP-ETHICS] Counsel ETHICAL-STOP-N5b — DEFER-FLAG → NEEDS-HUMAN

**Not the architect's to settle.** Per the operating model, an ETHICAL-STOP is **revise-or-route**, and
this one is a charter "record, don't judge" + dignity **policy** call. The architect's job here is to
(a) make the route honest, (b) verify the floor is cheap, (c) hand the human a clean decision — **not**
to decide it unilaterally.

**Architect actions taken this round (all non-deciding):**
1. **Pre-stage 6b-1 as the floor + verify zero-migration (DONE — confirmed).** Re-checked
   `customer_signals`: it is **FORCE-RLS**, **location-scoped**, and already carries `kind`, an
   `evidence` jsonb, `acknowledged_by_owner_id`, and a timestamp — and the contrast acknowledge/dismiss
   handlers (`signals.ts:139,179`) already write owner ids to it. So writing the manual no-show as an
   attributable, dismissible `customer_signals` row is **zero-migration**: it reuses the existing table;
   a distinct `kind` (e.g. `no_show_manual`) is a code-enum addition (`KIND_VALUES`, `signals.ts:10`),
   **not** a schema change. Confirmed: **6b-1 is near-free and additive-only**. This discharges the
   Counsel floor's "architect verifies zero-migration" ask.
2. **Route to STOP-ETHICS (don't decide).** The single human question stands **verbatim** (see
   `breaker-findings.md` / `counsel-opinion.md` §3). Recorded for the human, with the recommendation
   (advisory, non-binding): **6b-1 now** (attribution + reversibility, the minimal self-consistency fix),
   **6b-2** (subject contest channel) deferred to the **named trigger**: *"the first time `no_show` is
   consumed by anything stronger than an acknowledgeable `soft_confirm`"* — written into the ADR so the
   deferral cannot silently become permanent. The steel-man for deferring **all** of 6b (Counsel §4) is
   also on the record; the human may choose it if recorded.
3. **Courier-as-witness (Counsel §5) — capture, don't decide.** Recorded as an **open question** for the
   human, not resolved: should a strike attach the courier's delivery-attempt attestation as its
   evidentiary ground (owner-button = hearsay; courier = witness)? If 6b-1 ships, its `evidence` jsonb is
   the natural carrier for that attestation — noted so it is not lost, deferred with 6b-2.

**Disposition:** **DEFER-FLAG behind a flag (default off), NEEDS-HUMAN.** **6a ships now** (unambiguously
correct, charter-safe, no gate). N5 is **NOT considered closed on 6a alone** — it banks only after the
human answers the §3 question on the record.

---

## Residuals — honest (what this round does NOT close)

- **R1 — N5-6b is unresolved by design.** The reputation-recording / disclosure policy is a human
  decision (STOP-ETHICS). Until answered, the manual no-show still writes a raw counter without an
  attributable `customer_signals` row. 6a removes the false-positive; it does **not** discharge the
  self-contradiction Counsel grounded. **Owner: Counsel + product.** Banks on the recorded human answer.
- **R2 — N1 inline-drift is accepted, not eliminated.** The `order-messages.ts` customer branches rely on
  an inline check (+ the proposed shared helper). No lint guardrail yet (deferred YAGNI). A future
  customer branch is the recurrence trigger that promotes it to a guardrail. **Owner: customer-auth.**
- **R3 — N4 FE error-state is asserted as a DoD requirement but not yet built/proven.** The API 500 is
  inert money-blindness if the FE white-screens. The FE error-state Playwright proof is part of N4's
  GREEN and must land in the **same** change. **Owner: owner-finance UX.**
- **R4 — courier-as-witness** (evidentiary ground for a strike) is captured, deferred with 6b-2. **Owner:
  Counsel + product.**
- **R5 — round-1-resolved, not converged.** This banks pending Breaker **re-attack** on the now-corrected
  proposal (esp. the 5th N2 site, the unscoped-cron regression, the `picked_up` predicate, the
  per-row-decrypt isolation, the single `referenceDate` contract) **and** the human N5-6b decision. No
  item is marked "done"; no production code written.

---

## What changed in `proposal.md` / ADR this round
- §0.1 / §4.1 / §4.9 (N2): four sites → **five** (added `:53` INSERT `customer_id` value) + INSERT-path
  (cold-subscribe) regression in the DoD.
- §2.1 / §2.2 / §2.9 (B7): clause is **conditional** (only when `locationId` provided), never `= NULL`;
  **single** `referenceDate` contract (`z.string().date()` + UTC-day + range); added the **unscoped-cron
  settles-≥2-tenants** regression to the DoD.
- §5.1 / §5.2 / §5.5 / §5.9 (N4): N4-C **promoted to load-bearing**; FE error-state promoted from TODO to
  a required, proven DoD line; one-corrupt-row regression added.
- §6a (N5): predicate corrected to gate on **assignment `picked_up`**, not order `status='IN_DELIVERY'`;
  scope narrowed (machine already blocks CONFIRMED/PREPARING/READY); DoD RED case → **pre-pickup
  IN_DELIVERY REJECTED**.
- §3.8 (N1): inline-drift owner + shared-helper note recorded (accept-risk).
- §6b (N5): 6b-1 confirmed zero-migration; named deferral trigger for 6b-2; courier-as-witness captured;
  STOP-ETHICS routing affirmed.
- ADR Decision table + DoD rows updated to match all of the above; `referenceDate` contract pinned;
  the named 6b-2 trigger written in so the deferral cannot become silent.
