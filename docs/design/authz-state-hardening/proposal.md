# Design Proposal — Authz / State Hardening Batch (B7 + N1 / N2 / N4 / N5)

**Status:** DRAFT — **round-1-resolved** (Breaker's 2 HIGH + 2 MED + `referenceDate` contract folded in;
see `breaker-findings.md` + `resolution.md`). No production code in this change. Banks pending Breaker
re-attack + the one human N5-6b decision.
**Companion ADR:** `docs/adr/ADR-authz-state-hardening.md` (DRAFT, round-1-resolved).
**Security class:** 🔴 red-line — tenant-isolation (B7), customer-authz / BOLA (N1), RLS-GUC
correctness (N2), money-visibility (N4), order-state-machine + reputation/dignity (N5).
**Scope discipline:** five *independent* fixes that share one council. Each has its own §, its own
decision, its own red→green DoD. They do **not** depend on one another and can land/revert
independently. No new ADR is silently contradicted; this extends ADR-0013 (binding-scoped authz),
ADR-0010 (error envelope), ADR-0004 (token revocation) and the customer-track-grant model.

---

## 0. Problem + Non-goals (shared frame)

### 0.1 The five findings (verified against source, file:line)

| # | Class | One-line | Verified at |
|---|-------|----------|-------------|
| **B7** | tenant-isolation + DoS | Owner `POST .../:locationId/settlements/regenerate` ignores `:locationId` and runs a **platform-wide** settlement re-generation under one long transaction with platform-wide row locks. | `owner/settlements.ts:301-317` → `settlement-cron.ts:29-119` |
| **N1** | customer BOLA | The order-scoped customer JWT (`orderId` claim) is enforced on WS but **not** on REST — REST checks only `customer_id = sub`, so a 7-day token can read/rate/cancel/message **any** order sharing its `customer_id`. | `customer/orders.ts:49,235,283`; `order-messages.ts:70,146,181`; WS gate `websocket.ts:203-208`; token `jwt.ts:117-132` |
| **N2** | RLS-GUC correctness | `customer/push.ts` sets the `app.user_id` GUC + query/INSERT/UPDATE params from `user.userId`, but the customer token carries only `sub` → GUC = `undefined` → RLS keyed on `app_current_user()` matches nothing; **the INSERT (`:53`) writes `customer_id = NULL`** → FORCE-RLS `WITH CHECK` / NOT NULL violation on the 100%-of-cold-subscribe path. | **FIVE** sites: `customer/push.ts:35,38,53,72,75`; token `jwt.ts:126-131`; RLS `WITH CHECK` `1780421100059:25-27` |
| **N4** | money-blindness | Settlement LIST swallows any decrypt/query error and returns `{ payouts: [] }` → owner sees "nothing owed" instead of an error. | `owner/settlements.ts:46-71` |
| **N5** | order-state-machine + dignity | Owner can mark a **never-dispatched PENDING** order `no_show` (status is fetched but never checked) and the strike bumps a raw `customers.no_show_count` with **no owner-attributable, acknowledgeable record** and **no disclosure to the subject**. | `owner/signals.ts:198,211-230,250` |

### 0.2 Ground-truth correction carried into this proposal (do not propagate the steer verbatim)

The design steer asserts for N5 that "`customers` has NO location_id → the strike is GLOBAL/cross-tenant."
**That is false.** `customers` is location-scoped: `location_id uuid NOT NULL`, `UNIQUE (location_id,
phone)`, RLS `ENABLE` + `FORCE`, policy `tenant_isolation USING (location_id IN (SELECT
app_member_location_ids()))` (`1780310074262_orders.ts:8-16,74-77`). Each (location, phone) is a
distinct customer row; owner A's `no_show_count++` touches only owner A's row and is invisible to other
tenants. The same human at a different shop is a different row. **There is no cross-tenant counter.**

The residual N5 problem is therefore **not** cross-tenant — it is (a) a real **state-machine authz
bug** (no-show without a dispatch attempt) and (b) a **per-location dignity / disclosure** gap (an
owner-judged strike written as a raw counter, not as an attributable + acknowledgeable + subject-visible
record). This re-scoping changes the fix and the ethics framing materially; §5 carries it forward and it
is flagged NEEDS-HUMAN for Counsel.

### 0.3 Non-goals
- No redesign of the settlement period/cron model (B7 is a scoping + validation fix, not a rewrite).
- No new customer **account** identity / cross-order login. Every customer JWT is order-scoped by
  design (the only minters are `customer/track.ts:75` and `orders.ts:609`, both pass a concrete
  `orderId`; OTP issues an opaque non-JWT token — `customer/otp.ts:202`). N1 does not introduce, and
  must not require, an account-scoped token.
- No change to the *advisory, never-auto-ban* nature of `no_show_count` (the column comment is the
  contract: `1780421100053:14`). N5 hardens *when* and *how visibly* it is written, not its semantics.
- No auto-ban, auto-block, or score-based gating anywhere (charter: record, don't judge).

---

## 1. Back-of-envelope (where it bites)

**Tenant fleet (planning figure):** MVP target ≈ 50–200 active locations; design headroom 1,000.
Couriers/location ≈ 1–10; orders/location/day ≈ 20–300. Settlement period = daily (`SETTLEMENT_PERIOD`).

**B7 — lock blast radius (the load-bearing number).** `handleGenerate` opens ONE transaction
(`settlement-cron.ts:35`), scans `courier_assignments` across **all** tenants for the period
(`:38-45`), and inside the same tx takes `FOR UPDATE OF ca SKIP LOCKED` per delivered assignment
(`:73`) plus writes `courier_payouts` / `settlement_items` rows, holding all locks until a single
`COMMIT` (`:119`). At 1,000 locations × ~100 delivered cash assignments/period that is up to **~100k
row locks held in one transaction**, triggered by **any one owner**, rate-limited only at **5 calls /
5 min** (`:306`). That is both a cross-tenant write (an owner re-generates *other tenants'* payouts) and
a self-inflicted-DoS lever (long tx → lock contention + autovacuum stall on `courier_assignments`,
which is on the order hot-path). Scoped to a single `:locationId` the same operation is ~100 locks for
~1 location — three to four orders of magnitude smaller and bounded by the caller's own tenant.

**N1 — exposure.** Per leaked token: every order under the same `customer_id` at the same location.
A repeat customer accumulates orders unboundedly over the 7-day token life; the token is a bearer
credential held client-side (`jwt.ts:124`). Exposure = O(orders-per-customer), not O(1). Read (status,
PII-masked courier, address, items, totals), write (rating → fake reputation, cancel → griefing,
message → impersonation in another order's thread). Closing it caps each token to exactly its one order.

**N2 — volume.** Every customer push subscribe/unsubscribe (`push.ts`) currently writes/locates rows
with a `NULL` tenant GUC → 100% of customer device rows are orphaned or silently fail. Low blast
radius (push is non-critical) but 100% incorrect, and it sets an RLS GUC → red-line review.

**N4 / N5 — not rate-bound;** correctness/dignity, not throughput.

---

## 2. §B7 — Settlement regenerate: scope to the route tenant + validate + attribute

### 2.1 Options
- **B7-A — Guard only (reject if multi-tenant).** Keep `handleGenerate` global; in the route, refuse
  unless the caller is platform-staff. *Concept: fail-closed authz gate.* Rejected: it removes the
  owner's legitimate "regenerate my period" action entirely; the feature is owner-facing.
- **B7-B — Location-scoped generate path (CHOSEN).** Add an overload
  `handleGenerate(referenceDate, { locationId })` that adds `AND ca.location_id = $locationId` to the
  pairs scan (`settlement-cron.ts:38-45`) and threads `locationId` through; the cron path calls it with
  no `locationId` (all tenants, unchanged). The route passes `request.params.locationId`. *Concept:
  parameterize the aggregate boundary — make the tenant a first-class arg, default to the cron's
  whole-fleet only on the scheduled path.* Validate `referenceDate` as a real ISO date; record the
  owner as `actor_id`. **Chosen** — smallest change that makes the runtime boundary match the route
  contract, and `requireLocationAccess` already proves membership in `:locationId` (`auth.ts:117-145`).
- **B7-C — enqueue a per-location job.** Publish a `SETTLEMENT_GENERATE` job with `{ locationId }` and
  let the worker pick it up. *Concept: transactional outbox / async.* Deferred: adds latency + a new
  job shape for no correctness gain over B7-B; the synchronous scoped call is bounded once scoped. Keep
  as a scaling lever (schema-rich, runtime-minimal): the per-location job arg is *already* supported
  (`settlement-cron.ts:18-21` reads `data.referenceDate`); extend its `data` with `locationId` only if
  per-location latency ever needs to move off the request.

### 2.2 Decision
B7-B. Concretely: (1) `handleGenerate(referenceDate, opts?: { locationId?: string })`; **the pairs query
appends `AND ca.location_id = $n` ONLY when `opts.locationId` is provided** — the no-arg cron path emits
the *unmodified* whole-fleet query (NEVER a `location_id = $n` bound to `NULL`, which would match zero
rows and silently unsettle every tenant). Build the SQL by conditional concatenation + push the param
only when set — the same pattern as the settlements-list filters (`owner/settlements.ts:39-42`). When
scoped, the audit row records the acting owner. (2) **`referenceDate` — ONE definite contract** (not the
prior either/or): `z.string().date()` accepts **`YYYY-MM-DD` only** (rejects datetimes, epochs, `"2026"`,
junk — `z.coerce.date()` would accept them, `z.string().datetime()` would wrongly reject the natural
date-only input), normalize to a **UTC calendar day** (`…T00:00:00.000Z`) before
`getSettlementPeriodBoundaries`, and **range-bound** it (accept only a sane window, e.g. `[today-90d,
today+1d]` UTC) → 400 `VALIDATION_FAILED` (ADR-0010) before any DB work. (3) Route passes `{ locationId }`
from the already-membership-checked param. (4) Audit log: pass `actorKind='owner'`,
`actorId=request.user.sub` into the generate path so the manual regenerate is attributable (cron stays
`'system'`).

### 2.3 Data / migrations
None required. `settlement_audit_log` already has `actor_id` (`1780421100046`; used by approve/pay at
`settlements.ts:135,191`). The scoped query reuses existing indexes. Forward-only, no schema change.

### 2.4 Consistency / idempotency
Generation is already idempotent: `courier_payouts` upsert `ON CONFLICT (courier_id, location_id,
period_start, period_end)` (`settlement-cron.ts:52`) and `settlement_items` `ON CONFLICT
(assignment_id) DO NOTHING` (`:84`). Scoping the WHERE to one location does not change idempotency —
re-running for the same (location, period) is a no-op on already-settled items. The single-location tx
is short, so the `FOR UPDATE OF ca SKIP LOCKED` window is bounded to the caller's own assignments.

### 2.5 Failures / degradation
- `referenceDate` invalid → 400 `VALIDATION_FAILED` envelope (ADR-0010), zero DB work.
- Generate throws → existing ROLLBACK (`settlement-cron.ts:120-123`); route returns 500 envelope.
  Because the tx is now single-tenant, a failure cannot leave *other* tenants' payouts half-written.
- Keep rate-limit 5/5min: defensible *now that the work is tenant-bounded* (an owner spamming it only
  re-locks their own ~100 rows). Document the justification rather than tightening blindly.

### 2.6 Security / tenant-isolation
This is the fix. `requireLocationAccess` (`auth.ts:11`) already proves the owner is an active member of
`:locationId`; B7-B makes the *runtime effect* honor that same boundary. No cross-tenant write remains.
The SECURITY-DEFINER-free worker runs under the API pool but is now WHERE-scoped to one tenant.

### 2.7 Integer-money
Untouched. `cash_amount` / `total_earned` stay integer minor units; B7 changes *which rows* are summed,
not the arithmetic (`settlement-cron.ts:95,100-105`).

### 2.8 Open/accepted risks
- *Accepted:* a platform operator that legitimately needs a fleet-wide regenerate now has no route.
  Owner: platform-ops. Mitigation: the cron path (`SETTLEMENT_CRON`) already covers fleet-wide
  generation nightly; an out-of-band ops job can call `handleGenerate(date)` with no `locationId`.

### 2.9 DoD (red → green)
- **RED (scoped, cross-tenant write):** owner of location A calls
  `POST /owner/locations/A/settlements/regenerate` while a delivered+cash assignment exists under location
  B in the same period; assert a `courier_payouts` row is created for B. (Today: it is — cross-tenant
  write proven.)
- **GREEN (scoped):** after fix, same test asserts **no** B-tenant payout/audit row is created or mutated,
  and A's own payout is generated. Plus `referenceDate` validation: `"not-a-date"`, `"2026"`,
  `"2026-06-29T10:00:00Z"`, a far-future day → **400** before any DB write; `"2026-06-29"` → accepted;
  audit row for the manual run has `actor_kind='owner'` and `actor_id = caller.sub`.
- **RED+GREEN (unscoped cron — NEW, load-bearing regression for the refactor blast radius):** seed
  delivered+cash assignments under **≥2 distinct tenants** in-period; call `handleGenerate(referenceDate)`
  with **no `locationId`** (the nightly cron path, `settlement-cron.ts:20`) and assert a `courier_payouts`
  row is generated for **each** tenant. This goes **red** on the trivial-wrong refactor (unconditional
  `AND location_id = $n` with a NULL bind → zero rows → every courier silently unsettled while the scoped
  DoD above stays green) and green only when the clause is truly conditional. *(This catches the failure
  mode no other DoD line would.)*
- Run via `pnpm vitest` on the settlement suite + Playwright owner settlement spec against staging.

---

## 3. §N1 — Enforce `token.orderId === :orderId` on customer REST (mirror the WS gate)

### 3.1 Options
- **N1-A — central preHandler on the customer order plugin (CHOSEN).** One hook that, for
  `role==='customer'`, compares `request.user.orderId` to `request.params.orderId` and 404s on
  mismatch — applied to `customer/orders.ts` and the customer branch of `order-messages.ts`. *Concept:
  capability-token scope check at the edge; mirror `websocket.ts:204` (`order:${user.orderId}`).*
  Chosen: one gate, no per-handler drift, fail-closed by default for any new customer order route.
- **N1-B — inline check in each handler.** Add `if (token.orderId !== orderId) 404` to GET status, POST
  rating, POST cancel, and the three `order-messages.ts` customer branches. *Concept: same predicate,
  per-site.* Rejected as the *primary* mechanism (drift risk — a 7th handler added later forgets it) but
  used as the **belt-and-suspenders** in `order-messages.ts` where the existing customer branch already
  does `order.customer_id !== userId` (extend that line to also require `token.orderId === orderId`).
- **N1-C — drop `customer_id` checks, rely on `orderId` only.** Rejected: keep *both* — `orderId` scopes
  the capability, `customer_id` defends against a stale token after a customer-id reassignment. Defence
  in depth.

### 3.2 Decision
N1-A as the load-bearing fix + N1-B inline in `order-messages.ts`. The predicate everywhere:
`request.user.role === 'customer' && request.user.orderId !== request.params.orderId` → **404**
`NOT_FOUND` (never 403 — do not leak that the order exists; matches the existing 404-on-mismatch posture
at `auth.ts:129,137` and the order-messages 404s). Sites: `customer/orders.ts` GET `/orders/:orderId/status`
(`:20`), POST `/orders/:orderId/rating` (`:218`), POST `/orders/:orderId/cancel` (`:258`); and
`order-messages.ts` customer branches (`:70,146,181`).

### 3.3 Data / migrations
None. The authority already lives in the JWT claim (`jwt.ts:128`, `AuthToken.orderId`). Pure runtime
predicate.

### 3.4 Consistency / idempotency
No state change; read-time/precondition gate. The existing `customer_id = sub` WHERE clauses remain, so
the gate composes with them (AND-of-both).

### 3.5 Failures / degradation
A legitimate customer always has `token.orderId === :orderId` for their own order (the token was minted
*for* that order), so zero false negatives. A token missing `orderId` (shouldn't happen for
`role==='customer'`; `AuthToken` requires it) → 404 fail-closed.

### 3.6 Security / tenant-isolation
Closes the BOLA. Caps each order-scoped token to exactly its order — matching the WS invariant that has
held since `websocket.ts:203-208`. Confirms no legitimate account-scoped customer JWT exists (§0.3), so
nothing legitimate is denied.

### 3.7 Integer-money
N/A — read/precondition only. (Rating/cancel touch no money math.)

### 3.8 Open/accepted risks
- *Accepted:* if a future feature genuinely needs cross-order customer access (e.g. an order-history
  list), it MUST be a *separate, explicitly account-scoped* token/path — not a relaxation of this gate.
  Owner: customer-auth. Flag in ADR as a forward constraint.
- *Accept-risk (inline-drift, owner: customer-auth):* `order-messages.ts` is a **multi-role** plugin and
  cannot mount the customer-only central preHandler, so its three customer branches (`:70,146,181`) carry
  an **inline** `token.orderId === params.orderId` check (N1-B) — a drift surface if a 4th customer branch
  is later added and forgets it. **Mitigation (one line, not a new abstraction):** extract the predicate
  into a single exported guard `assertCustomerOwnsOrder(request)` (throws the 404), called at the top of
  each customer branch here and reused inside the N1-A preHandler, so the next branch copies a *named
  call* not a re-derived condition. Defer a lint guardrail (eslint-plugin-local) until a **second**
  occurrence (YAGNI); that recurrence is the promote-to-guardrail trigger. *(Breaker-confirmed positive:
  N1's endpoint set is complete — only `customer/orders.ts:20,218,258` + the 3 `order-messages` branches;
  no account-scoped route; both minters carry `orderId`; the mismatch-404 short-circuits before any DB
  query → no timing/existence leak.)*

### 3.9 DoD (red → green)
- **RED:** E2E — mint a customer track-token for order O1 (`/api/customer/track/exchange`), then call
  `GET /api/customer/orders/O2/status` where O2 shares O1's `customer_id`; assert 200 + O2 data
  (leak proven). Repeat for POST rating and POST `/orders/O2/messages`.
- **GREEN:** after fix, each cross-order call returns **404**; the same call on the token's own order O1
  still returns 200. Playwright API assertions against staging (`VITE_BASE_URL=...dowiz-staging`).

---

## 4. §N2 — `user.userId` → `user.sub` in customer push (RLS-GUC correctness)

### 4.1 Options
- **N2-A — substitute `user.sub` at all FIVE sites (CHOSEN).** `push.ts:35,38,**53**,72,75` — the prior
  draft enumerated only four and **omitted `:53`, the INSERT `customer_id` VALUES param**
  (`[user.userId, subscription.endpoint, …]`). The INSERT branch is **100% of first-ever subscribes**;
  with `user.userId` undefined it writes `customer_id = NULL` → FORCE-RLS `WITH CHECK (customer_id IN
  (SELECT app_current_user()))` (`1780421100059:25-27`) + NOT NULL both fail → throw. So the omission
  left the *cold-start* path broken. *Concept: use the canonical subject claim the RLS predicate
  expects.* `app_current_user()` reads the `app.user_id` GUC; the customer token's identity is `sub`
  (= `customers.id`, set by `issueCustomerToken`), no `userId` claim (`jwt.ts:126-131`). Chosen —
  one-field correctness fix, applied to **all five** sites.
- **N2-B — set the GUC via `withTenant` helper.** Rejected for the customer path: `withTenant` is keyed
  on the owner/courier `userId` membership model; the customer device RLS is `customer_id IN (SELECT
  app_current_user())` (`1780421100059:27`), already satisfied by `set_config('app.user_id', sub)`.
  Don't import the owner helper into a customer route.

### 4.2 Decision
N2-A + a **guard** so this never silently regresses: a not-undefined assertion before `set_config`
(reject/500 if the resolved id is falsy — never set an RLS GUC to `undefined`). Confirm `sub` is the
`customers.id` the RLS expects (it is — `issueCustomerToken({ customerId }) → sub`, `jwt.ts:130`).

### 4.3 Data / migrations
None. RLS policy unchanged (`1780421100059` keeps `customer_id IN (SELECT app_current_user())`).

### 4.4 Consistency / idempotency
The subscribe path is already idempotent (fingerprint = sha256(endpoint); UPDATE-if-exists else INSERT,
`push.ts:36-55`). Fixing the GUC makes the existing `SELECT ... WHERE customer_id = $1` actually match
under RLS, so the UPDATE branch starts working (today it always misses → duplicate INSERT attempts that
themselves fail the WITH CHECK).

### 4.5 Failures / degradation
Resolved id falsy → 500 envelope + log, never a `NULL`-GUC write. Push is non-critical (best-effort
notifications), so a failure here degrades silently to "no push" without blocking the order flow.

### 4.6 Security / tenant-isolation
Red-line because it sets an RLS GUC. Post-fix the GUC carries the real `customers.id`, so a customer can
only write/read *their own* device rows — restoring the intended isolation that the `undefined` GUC
accidentally broke (broke *closed*, i.e. nothing matched, so no leak today — but a latent correctness +
audit hole).

### 4.7 Integer-money
N/A.

### 4.8 Open/accepted risks
- *Accepted:* existing orphan `customer_devices` rows written during the buggy window (GUC undefined)
  may exist. Owner: data-hygiene. Mitigation: a forward-only cleanup is optional (push rows are
  disposable); not a launch blocker.

### 4.9 DoD (red → green)
- **RED (cold INSERT path — load-bearing):** a customer with **no** existing `customer_devices` row calls
  `POST /push/subscribe`; today the INSERT (`:53`) writes `customer_id = undefined → NULL` →
  WITH CHECK / NOT NULL violation → throw (the 100%-of-first-subscribe path is broken, not just orphaned).
- **RED (warm path):** call `POST /push/subscribe` as a customer; assert a `customer_devices` row exists
  for `customer_id = token.sub` **and** is readable back under that customer's RLS context. Today: missing
  / unreadable (GUC undefined).
- **GREEN:** after fix, a **cold** subscribe INSERTs a row with `customer_id = sub`, readable under that
  customer's RLS context, no NOT NULL / WITH CHECK violation; a **warm** re-subscribe UPDATEs (not
  duplicates). Guard test: a token with no resolvable id → 500, no row, no `set_config('app.user_id', NULL)`.
  *(Note: the prior "re-subscribe UPDATEs" case alone exercises only the warm path and stays green even
  with `:53` unfixed — the cold-INSERT case is the one that catches the omission.)*

---

## 5. §N4 — Settlement LIST must surface failure, never silent `{ payouts: [] }`

### 5.1 Options
- **N4-A — 500 error envelope on failure (CHOSEN).** Replace `catch { return { payouts: [] }; }`
  (`settlements.ts:69-70`) with `catch (err) { request.log.error(err); return reply.sendError(500,
  'INTERNAL', 'Failed to load settlements'); }`. *Concept: fail-loud on money; money-blindness is worse
  than an error.* Chosen — an owner must never be shown "nothing owed" when the truth is "we couldn't
  read it".
- **N4-B — partial result with an explicit `error` flag (`{ payouts, error: true }`).** *Concept:
  graceful partial.* Rejected for the list query: it is all-or-nothing (single query + decrypt loop), so
  a partial `payouts` array would itself be a lie. (B-shape would only make sense if individual rows
  could fail independently; they don't here.)
- **N4-C — per-row decrypt isolation (LOAD-BEARING, not a droppable refinement).** Verified at source:
  `decryptPII` runs **inside the `.map` inside the single `try`** (`settlements.ts:47-51`), so **one**
  corrupt cipher blob throws → the whole map throws → the outer catch swallows → `200 { payouts: [] }`
  for the **entire** list. So N4-C is the **only** thing preventing one bad PII blob from blinding **all**
  payouts (money-blindness through a second door) — it is a **required** part of the fix, not optional.
  Wrap **only** the per-row `decryptPII` in its own try/catch yielding `courierNameMasked: 'A***'`
  (the existing empty-name fallback, `:55`); the row + integer `total_earned` are always emitted. The
  outer catch is reserved for **query** failure → 500. This separates "one bad PII blob" (degrade the
  mask) from "can't read the money" (fail loud).

### 5.2 Decision
N4-A with the N4-C refinement: query failure → 500 envelope; a single-row decrypt failure → masked
fallback name, row still shown with its (integer) `total_earned`. The amount is never hidden by a PII
problem.

### 5.3 Data / migrations
None.

### 5.4 Consistency / idempotency
Read-only endpoint; no idempotency concern.

### 5.5 Failures / degradation
The whole point: degradation is now *visible*. 500 → the FE shows an error state rather than an empty
"all settled" list. **The FE error-state is a REQUIRED, proven DoD item, not a TODO** (promoted from a
parenthetical flag): a 500 that white-screens is money-blindness via a different silence. The owner
settlements page MUST render a visible error state on 500, proven with a Playwright
`expect(errorState).toBeVisible()` assertion, shipped in the **same** change as the API 500. Class:
**inline-fix** (Task-Exit), owner = owner-finance UX.

### 5.6 Security / tenant-isolation
Unchanged — query already scoped `WHERE p.location_id = $1` under `requireLocationAccess`.

### 5.7 Integer-money
Directly money-visibility. `total_earned` is integer minor units (`settlement-cron.ts:100-105`); N4
ensures it is *shown or errored*, never silently zeroed. No arithmetic change.

### 5.8 Open/accepted risks
- *Accepted:* a transient DB blip now surfaces as a 500 instead of an empty list. Correct trade — owner
  retries; the alternative (silent zero) risks an owner under-paying a courier.

### 5.9 DoD (red → green)
- **RED:** integration test — force the list query (or `decryptPII`) to throw (e.g. inject a bad cipher
  blob / stub a query rejection); assert the current handler returns `200 { payouts: [] }`.
- **GREEN (query failure):** a query failure returns **500** `INTERNAL` (not an empty list).
- **GREEN (one-corrupt-row — NEW, load-bearing):** inject **one** bad cipher blob among **≥2** rows →
  response is **200** with **all** rows present; the bad row's `courierNameMasked: 'A***'` + correct
  integer `totalEarned`; the good rows decrypt normally. (Proves N4-C isolates the bad blob — without it
  the whole list 500s.)
- **GREEN (FE error-state — NEW, required):** on a forced 500 the owner settlements page renders a visible
  error state (Playwright `expect(errorState).toBeVisible()`), **not** an empty "all settled" list.

---

## 6. §N5 — No-show requires a dispatch attempt; strike must be attributable + disclosable (NEEDS-HUMAN)

> **Ground-truth correction (carried from §0.2):** `customers` is location-scoped (RLS FORCE,
> `UNIQUE(location_id, phone)`). N5 is **not** a cross-tenant counter bug. It is (a) a state-machine
> authz bug and (b) a per-location dignity/disclosure gap. The cross-tenant framing in the steer does
> not survive the schema. This section is flagged **NEEDS-HUMAN / STOP-ETHICS** for the reputation/
> disclosure decision (6b), which is a charter "record, don't judge" + dignity matter.

### 6a. State-machine bug (decided — no human gate needed)

**Problem.** `mark-no-show` fetches `status` (`signals.ts:211,217`) but never checks it, so a **PENDING**
order — one that was never dispatched and therefore had no delivery attempt — can be marked `no_show`,
incrementing `no_show_count` and cancelling the order. A no-show is, by definition, *a customer who did
not receive an attempted delivery*; without a dispatch there was no attempt to miss.

**Ground-truth correction (carried into the predicate).** The draft's `status='IN_DELIVERY' OR
assignment picked_up` OR-clause is **wrong**: `dispatch.ts:46` advances the ORDER to `IN_DELIVERY` at
assignment **creation**, while the assignment is still `assigned` (`:49-51`); pickup is a later courier
action. So an `IN_DELIVERY` order can have an **un-picked-up** assignment — the courier never went to the
door — yet the OR-clause would admit the strike, contradicting 6a's own "real delivery attempt" rationale.
The **assignment `picked_up`** clause is the real attempt test; the order-status clause is not. Also: the
state machine **already** blocks CONFIRMED/PREPARING/READY (no `→CANCELLED` edge → `updateOrderStatus
('CANCELLED')` throws and rolls back the strike inside the `withTenant` tx), so the **only reachable
illegitimate** states are **PENDING** (no assignment) + **pre-pickup IN_DELIVERY** (assignment `<
picked_up`). The draft's "guard CONFIRMED/PREPARING/READY" overstates the surface.

**Options.**
- **6a-A — gate on the ASSIGNMENT reaching `picked_up` (CHOSEN, corrected).** Require an assignment for
  the order whose `status` reached `picked_up` (i.e. `IN ('picked_up','out_for_delivery','delivered')`),
  mirroring the cancel handler's `picked_up` gate (`orders.ts:282`). **Drop** the `OR order.status=
  'IN_DELIVERY'` admission — order status is not the attempt fact. *Concept: precondition guard keyed on
  the courier-lifecycle witness, not the order projection.* Reject with 409 `NO_SHOW_NOT_ALLOWED_STATUS`
  otherwise. Chosen.
- **6a-B — allow when order `status='IN_DELIVERY'`.** Rejected — admits the pre-pickup strike (an
  IN_DELIVERY order whose courier never reached the door); see the correction above.
- **6a-C — allow from any non-terminal state.** Rejected — preserves the original bug; a PENDING order has
  no attempt to no-show.

**Decision (6a).** Add the guard: allow no-show **only** if the order has an assignment that reached
`picked_up` (canonical via `courier_assignments.status`, `orders.ts:282`) — **not** on order
`status='IN_DELIVERY'`. PENDING (no assignment) and pre-pickup IN_DELIVERY (assignment `< picked_up`) →
409, no increment. The `FOR UPDATE` row lock (`signals.ts:211`) already serializes concurrent marks; the
guard reads the same locked row, so the check is race-free within the tx.

**Idempotency (6a).** A no-show on an already-CANCELLED/terminal order must be a 409, not a second
increment. Add `AND status NOT IN (terminal)` to the same precondition so a double-submit cannot
double-strike. (Today it can: nothing stops two increments.)

**DoD (6a).**
- RED (PENDING): mark-no-show on a freshly created PENDING order returns 200 and bumps `no_show_count`.
- RED (pre-pickup IN_DELIVERY — NEW, the case the OR-clause would have missed): an order advanced to
  `IN_DELIVERY` whose assignment is still `assigned`/`accepted` (pre-`picked_up`) is marked no-show →
  today (and under the wrong OR-clause) returns 200 + `no_show_count++`.
- GREEN: **both** the PENDING and the **pre-pickup IN_DELIVERY** calls return **409
  `NO_SHOW_NOT_ALLOWED_STATUS`**, `no_show_count` unchanged; a no-show on an order whose assignment
  reached `picked_up` still succeeds **exactly once** (second submit → 409).

### 6b. Reputation strike scope + disclosure (NEEDS-HUMAN — STOP-ETHICS)

**Problem (re-scoped).** The strike is per-location (not cross-tenant). The dignity concern is:
`mark-no-show` writes the raw counter directly (`customers.no_show_count++`, `last_no_show_at = now()`,
`signals.ts:224-230`) **without**: (i) an owner-attributable, **acknowledgeable** `customer_signals`
record (contrast acknowledge/dismiss which write owner ids — `signals.ts:139,179`); the manual no-show
only publishes a `CUSTOMER_NO_SHOW` *event* (`:250`), and the `signal-raiser` worker that would persist a
`customer_signals` row keys off computed velocity/no-show signals, not the manual mark, so the manual
judgment may never land as an attributable, dismissible ledger row; and (ii) **any disclosure to the
subject** — the customer order-status endpoint surfaces `payment_outcome` so a refuser can contest it
(`customer/orders.ts:153-159`), but it does **not** surface a no-show strike. So the strike is
owner-judged, weakly attributed, and invisible to the person it is about.

**Options (for the human to decide — do not auto-resolve).**
- **6b-1 — write an attributable `customer_signals` row on manual no-show** (`kind='manual_flag'` or a
  new `no_show_manual`, with `acknowledged_by_owner_id`/actor on the mark), so the judgment is logged,
  dismissible, and decays via the existing `last_no_show_at` forgiveness path (`signals.ts:148-156`).
  Keep the counter increment. *Concept: judgment becomes a first-class, reversible record, not a silent
  counter bump.*
- **6b-2 — 6b-1 + disclose to the subject:** add the no-show to the customer-visible outcome surface
  (parity with `payment_outcome` disclosure) so the accused can see and contest it. *Concept: due
  process / right-to-know.* Likely required by the charter dignity line and the audit MED-4 finding.
- **6b-3 — keep the raw counter only (status quo minus the 6a guard).** Rejected on charter grounds:
  an owner-judged, weakly-attributed, invisible strike is exactly the "record-don't-judge" violation the
  ethics lane flagged.

**Recommendation to the human (not a decision):** 6b-1 now (attribution + reversibility, the minimal
record-don't-judge fix), 6b-2 deferred to a **named trigger** (below). **This is escalated to Counsel /
human** because it sets policy on how a person's reputation is recorded and whether they are told —
outside an architect's authority to settle alone. The architect verified the floor is cheap (zero-
migration, below); the human answers the single question; the architect does not decide it.

**Named deferral trigger for 6b-2 (so the deferral cannot become silent).** A subject contest channel
becomes **mandatory, shipped simultaneously**, the **first time `no_show` is consumed by anything stronger
than an acknowledgeable `soft_confirm`** — i.e. if `evaluatePreflight` ever escalates it toward
`hard_block`, any auto-gating, or it feeds a feature the subject cannot pass through (today the only
effect is the acknowledgeable `soft_confirm` at `evaluatePreflight.ts:127-134`, which already discloses
the count). Disclosure obligation **scales with consequence severity**. This trigger is written into the
ADR.

**Open question for the human (Counsel §5 — captured, not decided): courier-as-witness.** The owner
presses the button, but the first-hand witness of a no-show is the **courier** who attempted the
delivery; `assignment picked_up` is "a delivery was attempted," not "the courier attested the customer
did not answer." Should a strike attach/require the courier's delivery-attempt attestation as its
evidentiary ground rather than rest on owner assertion (structurally hearsay)? If 6b-1 ships, its
`evidence` jsonb is the natural carrier. Deferred with 6b-2; recorded so it is not lost.

**Data / migrations (only if 6b-1/6b-2 chosen, forward-only):**
- 6b-1: likely **no schema change** — reuse `customer_signals` (`kind`, `evidence` jsonb,
  `acknowledged_by_owner_id`, location-scoped, RLS already `FORCE`). If a distinct kind is wanted, it is
  a code enum addition (`KIND_VALUES`, `signals.ts:10`), not a migration. Any new column would be
  additive, nullable, RLS-inherited.
- 6b-2: a customer-visible disclosure needs only a read addition on the existing status endpoint (the
  no-show is derivable from order `status_notes='no_show'` + `customer_signals`), so **no migration** —
  a query + response-field change, flag-gated.

**Consistency / idempotency (6b).** The `customer_signals` de-dup (same kind within 1h,
`signal-raiser.ts:110-116`) prevents a double manual flag; reuse it. The counter increment stays guarded
by 6a's terminal-state check so it cannot double-strike.

**Security / tenant-isolation (6b).** Already location-scoped + RLS FORCE on both `customers` and
`customer_signals`. No cross-tenant surface. Disclosure (6b-2) is to the order's own customer only
(N1 gate applies).

**Integer-money (6b).** N/A.

**Open risk / owner.** The reputation-recording policy is **NEEDS-HUMAN**; owner = Counsel + product.
Until decided, ship **6a only** (the state-machine guard is unambiguously correct and charter-safe);
hold 6b behind the council decision and a flag.

**DoD (6b, conditional on the human decision).** RED: a manual no-show leaves no attributable,
dismissible `customer_signals` row and is invisible to the customer. GREEN (if 6b-1): a manual no-show
creates a `customer_signals` row carrying the acting owner id, dismissible via the existing endpoint,
decaying via `last_no_show_at`. GREEN (if 6b-2): the affected customer's order-status response exposes
the no-show outcome so it can be seen/contested.

---

## 7. Operability (shared)
- **Health degraded-vs-down:** none of these five changes a liveness signal. N4 converts a silent zero
  into an observable 500 → it *improves* observability (error rate on the settlements list becomes a
  real signal within <1 min via existing error logging).
- **Observability:** B7 audit rows now carry the acting owner (was `system`) → manual regenerations are
  attributable in `settlement_audit_log` within the query window. N5/6b (if chosen) makes manual
  no-shows queryable in `customer_signals`.
- **Rollback:** every item is a pure runtime predicate or error-path change with **no migration** (6b is
  the only one that *might* add an enum value, additive). Each reverts independently by reverting its
  file diff; no data backfill, no irreversible step.
- **Flag / scaling-gate:** 6b (reputation recording/disclosure) ships behind a flag, default off, until
  Counsel decides. 6a, B7, N1, N2, N4 are correctness/security fixes — no flag, land directly (still via
  the staging → proof → prod ship discipline).

---

## 8. Open / accepted risks register
| Risk | Item | Disposition | Owner |
|------|------|-------------|-------|
| No fleet-wide owner regenerate route post-fix | B7 | Accept — cron + out-of-band ops covers it | platform-ops |
| Future cross-order customer view needs a new account-scoped token, not a gate relaxation | N1 | Accept as forward constraint (ADR) | customer-auth |
| Orphan `customer_devices` from the undefined-GUC window | N2 | Accept — disposable rows; optional cleanup | data-hygiene |
| Transient DB blip now surfaces as 500 on settlements list | N4 | Accept — correct trade vs money-blindness | owner-finance UX |
| **Reputation strike recording + subject disclosure policy** | **N5-6b** | **NEEDS-HUMAN / STOP-ETHICS — hold behind flag** | **Counsel + product** |

---

## 9. NEEDS-HUMAN summary
- **N5-6b (reputation scope + disclosure)** — STOP-ETHICS. The state-machine guard (6a) is decided and
  charter-safe; the *recording/disclosure of a person's reputation strike* is a "record-don't-judge" +
  dignity policy call escalated to Counsel/human. Ship 6a now; gate 6b.
- **Ground-truth correction (N5 not cross-tenant)** — surfaced for the council so the breaker matrix and
  any downstream audit register (`docs/design-review/ADVERSARIAL-AUDIT-*`) are not anchored on the false
  "global counter" premise.
