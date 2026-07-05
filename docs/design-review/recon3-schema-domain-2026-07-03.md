# Recon #3 — Data-Model / Schema Integrity + Delivery-Domain Correctness (READ-ONLY)

Date: 2026-07-03 · Scope: `packages/db/migrations/*` (157 files) + how `apps/api/src/routes|lib|workers/**` use the tables.
Angles deliberately NOT re-covered here (owned by runs 1–2): money-rounding/tax math, security-IDOR, reliability-workers, perf/index, concurrency-races, integrations. Where a finding here touches those docs it is marked **[confirms …]** and not re-counted.

Method: three parallel read-only lanes — (A) schema/FK/constraint/migration-history, (B) courier-dispatch + shift/settlement/cash domain, (C) order-lifecycle state-machine + fee/ETA/geo. Every claim was checked against live file content; line numbers are current-tree. No files were modified except this report.

Finding IDs: `S-*` schema lane, `D-*` dispatch/cash lane, `L-*` lifecycle/fee/eta lane. Cross-lane duplicates are merged and counted once.

## Severity tally

| Severity | Count | IDs |
|----------|-------|-----|
| CRITICAL | 4 | D-F1, D-F6, S-H6, L-H3 |
| HIGH | 16 | S-H1(=D-F3), S-H2(=D-F10), S-H3, S-H4+H5, S-H7, S-M3, D-F2, D-F4, D-F7, D-F11, D-F13, D-F14, L-H1, L-H2, L-H4, L-H5 |
| MED | 25 | D-F5, D-F8, D-F12, D-F16, D-F17(=S-M9), D-F18, D-F21, S-M1, S-M2, S-M4, S-M5, S-M6, S-M7, S-M8, S-M10, S-M11, S-M12, L-M1..L-M9 |
| LOW | 13 | S-L1..L5, D-F9, D-F19, D-F20, L-L1..L5 |

Worst schema-integrity gap: **S-H6** — the migrations directory is not a reproducible schema history (075/076 applied out-of-band per an operator handoff doc, numbers 011/030 reused after deletion, `migrate:up --no-check-order` suppresses all drift detection). A DR restore, a fresh environment, or the pending OSS clone rebuilds a DB **without the NOBYPASSRLS/RLS hardening** and nothing reports it. Every other schema fix here compounds on top of this because "the schema in git" and "the schema in prod" are not provably the same thing.

---

## CRITICAL

### D-F1 — Dispatch-exhaustion counter resets on every successful assignment → an order can loop between one courier forever with zero escalation
`apps/api/src/workers/courier-dispatch.ts:148` deletes the journal row on assign:
```ts
await client.query(`DELETE FROM courier_dispatch_queue WHERE order_id = $1`, [orderId]);
```
`courier_dispatch_queue.attempts` (PK `order_id`) is the only exhaustion counter, and every re-enqueue (reject `routes/courier/assignments.ts:213-217`, decline `:558-561`, accept-timeout `workers/courier-offer-sweep.ts:141-144`, offer expiry `:94-97`) is `INSERT … ON CONFLICT DO UPDATE SET attempts = attempts+1` — but the row was deleted, so it restarts at 0. The `maxAttempts` check lives only in the no-courier branch (`courier-dispatch.ts:110`). One on-shift courier with a dead/ignored app → assign → 5-min accept-timeout → re-enqueue(0) → assign → … forever. `dispatch_exhausted_at` is never set, `ORDER_DISPATCH_FAILED` never fires, the owner Telegram alert and the honest customer push (ADR dispatch-recovery ETHICAL-STOP-1 "never silent") never happen.
**Invariant:** dispatch must terminate in bounded time with honest escalation at both ends. **Fix:** age/attempt-count on the order (or don't reset `attempts`) — exhaust on `enqueued_at` age, not the ephemeral journal row.

### D-F6 — Accept-timeout sweep strands the order at `IN_DELIVERY` with no courier (self-declared red-line)
`apps/api/src/workers/courier-offer-sweep.ts:131-146` (pass 2) cancels the un-accepted `assigned` binding, frees the shift, re-enqueues the journal — but never touches the order row. `attemptHonestDispatch` advances the order to `IN_DELIVERY` at *assign* time (`lib/dispatch.ts:46`), before any acceptance. When the courier never accepts and dispatch then exhausts (grace-cancel pass 4 is flag-OFF/dark), the order sits `IN_DELIVERY` **forever with zero courier** — the customer is told delivery is underway. `bindingRelease.ts:45-50` does the correct thing for the analogous pre-pickup release (revert `IN_DELIVERY→READY`); pass 2 does not.
**Invariant:** `lib/dispatch.ts:7-8` — "an order must NEVER reach IN_DELIVERY with no courier." **Fix:** pass 2 must mirror `releaseBindingAndReoffer` — if order is `IN_DELIVERY` and binding never picked up, revert to `READY` in the same tx.

### S-H6 — Migration numbering gap 075/076: security migrations applied out-of-band, absent from the repo; `--no-check-order` hides the drift
`packages/db/migrations/` jumps 074 → 077; git confirms 075/076 **never existed** in any branch. `docs/security/pg-privilege-hardening-OPERATOR-HANDOFF.md:20-27` instructs the operator to hand-copy `OPERATIONAL-ROLE-nobypassrls.migration.ts` + `SECURITY-DEFINER-search-path.migration.ts` into slots 075/076 on live DBs. `package.json:38` runs `migrate:up … --no-check-order`. Consequence: staging/prod `pgmigrations` contain rows whose source files are not in the repo; a fresh env, a DR restore-then-migrate, or the OSS clone rebuilds a database **without the NOBYPASSRLS strip and DEFINER search_path re-pin** — the RLS hardening silently evaporates and nobody is told. Mig 077's own header ("Verified against live staging policies") confirms it depends on that out-of-band state. Compounds with **S-H7** (numbers 011 and 030 reused after their original files were deleted — different environments ran different SQL under identical-looking histories).
**Invariant:** the migrations directory is the complete, reproducible schema history. **Fix:** land the two `docs/security/*.migration.ts` files in-repo as idempotent 075/076 matching the names recorded in live `pgmigrations`; add a CI guard retiring deleted numbers.

### L-H3 — `free_delivery_threshold` bypasses the deliverability check → any pin on Earth accepted with fee 0
`apps/api/src/routes/orders.ts:490-505`:
```ts
if (location.free_delivery_threshold !== null && subtotal >= location.free_delivery_threshold) {
  deliveryFee = 0;
} else {
  … resolveDeliveryFee({ location, pin, tiers }) …   // NOT_DELIVERABLE lives ONLY here
}
```
The distance/tier range test (`lib/order-pricing.ts:171-178`, throws `NOT_DELIVERABLE` when `distKm` exceeds every tier) is only reached in the *else* branch. Any cart ≥ the free-delivery threshold is accepted for **any pin, any distance**, fee 0 — the kitchen then cannot deliver.
**Invariant:** an address outside every delivery tier is not deliverable regardless of basket size. **Fix:** run the tier-range check first; apply the free threshold to the *fee* only, never to deliverability.

---

## HIGH

### S-H1 (= D-F3) — Auto-dispatch ignores `couriers.status`; deactivation never closes the shift → deactivated/suspended couriers stay dispatchable
`workers/courier-dispatch.ts:95-104` selects on `courier_shifts.status='available'` with **no** join to `couriers.status` (contrast `lib/dispatch.ts:32` and `routes/owner/dashboard.ts:241` which both filter `c.status='active'`). `routes/owner/couriers.ts:110-116` on deactivate only revokes sessions — the `available` shift stays open, active bindings untouched. New orders keep getting offered/assigned to a courier who can no longer log in; each cycle burns the 5-min accept-timeout and (via D-F1) loops forever.
**Invariant:** eligibility ("active courier, open shift, this location") enforced identically on every path; a tombstoned parent is excluded from selecting its children. **Fix:** `JOIN couriers c ON c.id=cs.courier_id AND c.status='active'` in the worker query; on deactivate `UPDATE courier_shifts SET status='offline', ended_at=now()` and release bindings in the same tx.

### S-H2 (= D-F10) — No "one active shift per courier" uniqueness; shift reads are nondeterministic
`migrations/1780421036157_courier-shifts.ts` has only a non-unique partial index. `lib/shiftService.ts:15-23` scopes its open-check to `DATE(started_at)=CURRENT_DATE`, so a shift opened yesterday and still `available` after midnight is invisible → a second concurrently-active row is INSERTed. `routes/courier/shifts.ts:196-203` reads `SELECT id,status … WHERE courier_id=$1 AND location_id=$2 FOR UPDATE` (no status filter, no ORDER BY, no LIMIT) and uses `rows[0]` — arbitrary among 2+ rows; `/me/shift/end` (`:122-133`) closes only that arbitrary row, so "end shift" can leave a second shift open and the courier still dispatchable.
**Invariant:** ≤1 open shift per (courier, location); "end shift" removes the courier from dispatch. **Fix:** `CREATE UNIQUE INDEX … ON courier_shifts(courier_id,location_id) WHERE status IN ('available','on_delivery')`; drop the `DATE()` scoping; `ORDER BY started_at DESC LIMIT 1` (or close-all).

### S-H3 — "Immutable audit" money/trace rows are `ON DELETE CASCADE`-deleted with the order
`migrations/1790000000028_courier-cash-ledger.ts:15` and `1790000000027_delivery-trace.ts:12` both declare `order_id … REFERENCES orders(id) ON DELETE CASCADE` while their headers call themselves "immutable audit log"/"immutable audit record" (also `order_status_history`, `order_sensor_events`, `customer_contact_reveals`, `order_ratings`). Any future hard-delete of an orders row (a GDPR path change, ops cleanup, `migrate:down`) silently erases cash-hold evidence and delivery traces — the exact records meant to survive disputes. Latent today (no route hard-deletes orders) but the schema contradicts its stated invariant.
**Invariant:** audit/money ledgers outlive (or tombstone with) their subject, never cascade. **Fix:** `ON DELETE RESTRICT` on `courier_cash_ledger.order_id` and `delivery_trace.order_id` at minimum.

### S-H4 + S-H5 — Replace-mode menu import is a guaranteed FK dead-end for any venue with real order history
`routes/owner/menu-import.ts:452-472`: `DELETE FROM categories` (452) runs **before** `DELETE FROM products` (461) → a removed category still holding a removed product raises FK 23503 → whole import 500s. And `DELETE FROM modifier_groups` (469) cascades into `modifiers`, but `order_item_modifiers.modifier_id` is NO ACTION (`migrations/1780338982010_menu_modifiers.ts:35`) — once any real order used a modifier, replace-mode is a guaranteed FK error; the pre-check (`:443-450`) only inspects `order_items`, not `order_item_modifiers`. Root cause **S-H5**: mig `1780338982023` deliberately gave `order_items.product_id` `ON DELETE SET NULL` (snapshots preserve history) but `order_item_modifiers` has the identical `name_snapshot`/`price_delta_snapshot` pair and kept NO ACTION — so `routes/owner/modifier-groups.ts:128,221` also 500s deleting any historically-used modifier.
**Invariant:** deletion order respects FK direction; snapshot-carrying children SET NULL. **Fix:** products-before-categories order; one migration mirroring 023: `order_item_modifiers.modifier_id … ON DELETE SET NULL`.

### S-H7 — Migration numbers reused after deletion (011, 030)
`1790000000011_create-reconciliation-queue.ts` deleted (`f284fdb8`), 011 reused by `pgboss-bootstrap-schema`; `1790000000030_feedback-reminder-queue.ts` deleted (`49d24f70`), 030 reused by `onboarding-publish-state`. Environments that ran the old files hold `pgmigrations` rows under the old names; the replacements run later, out of order, silently under `--no-check-order`. Different environments have executed different SQL under identical-looking histories — undiagnosable from the repo alone. **Fix:** retire deleted numbers permanently (CI guard); new content takes a fresh head number.

### S-M3 — Payments idempotency UNIQUE is void when `provider_payment_id` is NULL (dark, pre-launch)
`migrations/1790000000083_payments-ledger.ts:29,41,54,61`: `provider_payment_id text` (nullable) inside `UNIQUE (provider, provider_payment_id)` and `UNIQUE (provider, provider_payment_id, type)`. Postgres default NULLS DISTINCT means every NULL-ref row passes both "insert-wins idempotency" constraints — duplicate charge / replayed-event rows are admissible in exactly the failure mode (provider omits the ref) where dedup matters. Dark today (payment flags off); flip before launch. **[confirms recon2-integrations H1/M1 provider-trust class]** **Fix:** `provider_payment_id NOT NULL` on `payments`, or `UNIQUE NULLS NOT DISTINCT`, or partial-unique + CHECK requiring the ref for non-pending rows.

### D-F2 — No decline/reject/timeout memory + recency-ordered pick → the same courier is re-picked every cycle; others starve
`workers/courier-dispatch.ts:95-104` / `lib/dispatch.ts:27-38` exclude only *currently-active* bindings and `ORDER BY cs.last_heartbeat_at DESC`. A courier whose prior binding for this same order ended `rejected`/`offered_expired`/`assign_accept_timeout` is immediately eligible again, and the freshest-heartbeat (same foreground app) keeps winning every 60s pump tick while a second courier receives **zero** offers. No rotation, no per-order tried-courier memory, no load balancing.
**Invariant:** dispatch fairness — every available courier eventually offered; re-offer prefers untried couriers. **Fix:** exclude couriers with a terminal assignment row for the same `order_id` (fall back to all when everyone tried); order by least-recently-assigned.

### D-F4 — `attemptHonestDispatch` shift join not location-filtered → cross-location assignment for multi-venue couriers
`lib/dispatch.ts:28-39` filters membership (`courier_locations`) by location but joins `courier_shifts` with **no** `cs.location_id=$1`. A courier belonging to venues A+B and on shift at B is selected for A's order; the insert binds A's assignment to B's `shift_id` and flips B's shift to `on_delivery`. Live because the caller runs on the BYPASSRLS operational pool (RLS doesn't mask the foreign shift). **Fix:** add `AND cs.location_id = $1` to the join.

### D-F7 — `offered` assignments unreachable via REST → a pending offer silently dies on reload/reconnect
`routes/courier/assignments.ts:86`: `AND ca.status IN ('assigned','accepted','picked_up')` — `'offered'` excluded. The courier task list is fetched from this endpoint (`TasksPage.tsx:41`) and renders `status==='offered'` (`:193`), but an offer only ever arrives via the `task_offered` WS push (`dashboard.ts:336`). A reload/reconnect/app-restart makes the offer invisible; it dies at TTL, re-enqueues, loops (feeds D-F1). **Fix:** include `'offered'` in the `/me/assignments` filter.

### D-F11 — Same-day shift re-open lockout drains the courier pool for the rest of the UTC day
`lib/shiftService.ts:29-45`: today's latest shift with `status='offline'` (a normally ended shift) hits `throw { statusCode:400, error: 'Cannot open shift in status offline' }`. The FE calls exactly this (`ShiftPage.tsx:117 → /courier/me/shift/start`). A courier who ends a shift at lunch cannot go back on until the next UTC day; the venue silently loses its courier pool (and orders then exhaust via D-F1). **Fix:** treat a today-`offline` shift as reopenable (`status='available', ended_at=NULL`) or insert a fresh row.

### D-F13 — Owner reassign of a busy courier reverts a **picked-up** order to READY (food already out the door)
`routes/owner/dashboard.ts:274-291`: the busyCheck displaces the courier's `accepted` **or `picked_up`** binding and, when the order is `IN_DELIVERY`, reverts it to `READY`. `bindingRelease.ts:40-44` sets the domain rule for exactly this state ("food is out with the failed courier → honest terminal CANCELLED"); here the picked-up order becomes re-assignable while the physical food is with the displaced courier — a second courier is dispatched to a venue with no food. Also mislabels displaced real assignments as `'offered_expired'` (`:261`), corrupting offer-vs-assignment stats. **Fix:** mirror `releaseBindingAndReoffer`'s picked_up branch (CANCELLED terminal); terminalize displaced bindings as `'cancelled'`.

### D-F14 — Owner `/deliver` proxy fabricates a cash `hold` + settlement earnings for crypto-prepaid orders (dark)
`routes/owner/dashboard.ts:452-497`: the body enum omits `delivered_prepaid`, `cash_collected` defaults `true`, `finalCashAmount = cashAmount ?? total`, and — unlike the courier path (`assignments.ts:338-340`, which auto-resolves `crypto+paid → delivered_prepaid`) — there is no prepaid auto-resolve. An owner tapping deliver on a crypto-paid order records `paid_full` with cash===total → `completeDelivery` writes a `courier_cash_ledger 'hold'` (`deliveryCompletion.ts:118-124`) for money never handed over, and `app_generate_settlements` counts it into payout. Violates `deliveryCompletion.ts:59` ("a paid order must never create a till-debt"). Dark (crypto/payments flags off). **[confirms audit-money-orders H4]** **Fix:** replicate the courier crypto auto-resolve in the owner proxy; add `delivered_prepaid` to its enum.

### L-H1 — Owner PATCH `DELIVERED`/`PICKED_UP` bypasses `completeDelivery`: cash hold never written, courier stranded permanently
`routes/orders.ts:885-892` special-cases only `IN_DELIVERY`; `assertOwnerTargetAllowed` (`lib/orderAuthz.ts:19-27`) guards only CANCELLED. `StatusUpdateInput` accepts any enum, so an owner can PATCH `IN_DELIVERY→DELIVERED`. `updateOrderStatus`'s central fold (terminalize binding + free shift) runs only for CANCELLED / `IN_DELIVERY→READY` (`orderStatusService.ts:134-145`) — DELIVERED cleanup lives exclusively in `completeDelivery`. Result: `courier_assignments` stays `picked_up`, shift stays `on_delivery`, no cash `hold`, no `delivery_trace`; the courier is permanently excluded from dispatch (`courier-dispatch.ts:99-102`) and their own later `/delivered` throws `SameStatusError` → 500. Same fold-skip for `READY→PICKED_UP` on a delivery order with an active binding.
**Invariant:** every terminal transition terminalizes the active binding in the same tx (`orderStatusService.ts:125`); cash-as-proof HOLD guaranteed on every delivered order. **[extends audit-money-orders H1 with the permanent-dispatch-exclusion + PICKED_UP variant]** **Fix:** reject `DELIVERED`/pickup in the PATCH route (route owners to `/deliver`) or extend the fold + hold to those branches.

### L-H2 — POST `/orders` ignores `delivery_paused`, opening hours, and `kitchen_busy_until`
`routes/orders.ts:121-141` fetches `busy_mode, published_at…` but neither `delivery_paused` nor `hours_json` nor `kitchen_busy_until`, and no check exists (grep confirms `delivery_paused` is read only by storefront/menu/telegram). The pause toggle the owner flips from Telegram (`lib/storefrontService.ts:105`) only affects the storefront `isOpen` badge (`routes/public/menu.ts:335`). A direct POST or a stale open tab places a live order into a paused/closed kitchen; timeout later auto-cancels it — a promise that was never going to be kept. **Fix:** add `delivery_paused` + hours to the section-1 location gate → 409 `STORE_PAUSED`/`STORE_CLOSED`.

### L-H4 — The global `order.status` bus channel has 2 subscribers and 0 publishers → dwell alerts never auto-resolve on progress + customer status pushes never fire
Exhaustive grep finds no `publish(BUS_CHANNELS.ORDER_STATUS)` anywhere (`owner/dashboard.ts:363` publishes to the *per-order* `order:{id}` channel, a different NOTIFY channel). Dead consumers: `workers/lifecycle-handlers.ts:31-36` — the only path that resolves `dwell_confirmed/preparing/en_route` on PREPARING/IN_DELIVERY/DELIVERED; and `bootstrap/messaging.ts:134-146` — the sole enqueue for customer push on `order.confirmed/in_delivery/delivered` (`CUSTOMER_PUSH_EVENTS`). Combined with `ORDER_CANCELLED` being published only by no-show and dark grace-cancel (never by the timeout worker or owner PATCH cancels), dwell alerts effectively resolve only on CONFIRMED/REJECTED; everything else stays `active` forever, inflating the batch threshold and leaving stale "stuck" alerts on delivered orders; and customers never get progress pushes.
**Invariant:** progress/terminal transitions retire their outstanding side-effects (alerts) and emit their notifications. **Fix:** publish `BUS_CHANNELS.ORDER_STATUS` (global) from `updateOrderStatus` step 4, and `ORDER_CANCELLED` from every CANCELLED path.

### L-H5 — Stuck-state gap: only PENDING has a hard exit; READY isn't even monitored
Timeout sweep + per-order job cover **PENDING only** (`app_sweep_timeout_orders() WHERE status='PENDING'`, `migrations/1790000000078:15-22`; `apps/worker/src/handlers.ts:26-27`). Dwell monitor covers `['PENDING','CONFIRMED','PREPARING','IN_DELIVERY']` (`workers/dwell-monitor.ts:11`) — **READY absent**; `TRANSITION_RESOLVE_MAP` has no READY/PICKED_UP keys. The only exit for an ignored CONFIRMED/PREPARING/READY order is grace-cancel pass 4, which is dark. An order sitting in READY (courier never appears, owner closes the tab) lives forever — no sweep, no dwell alert, no notification; the customer polls an eternally "Ready" order. **Fix:** add READY to `MONITORED_STATUSES` (+ a `ready_s` threshold) and a long-tail sweep for non-terminal orders older than N hours.

---

## MED

### D-F5 — Capacity pre-checks inconsistent with the DB partial-unique → manual assign 500s instead of a domain answer
DB cap is 1 active per courier incl. `'offered'` (`courier_one_active_assignment`, `migrations/1790000000073:31-33`). But `routes/owner/dashboard.ts:267-273` busyCheck matches only `('accepted','picked_up')` (misses `offered`/`assigned`), and `lib/dispatch.ts:48-52` has no 23505 handler (unlike the worker). A target holding an unaccepted binding → INSERT hits 23505 → unhandled → owner 500. **Fix:** align both pre-checks to the partial-unique's status list; map 23505 → 409/next-candidate.

### D-F8 — Courier `/reject` publishes a false `ORDER_CONFIRMED` lifecycle event
`routes/courier/assignments.ts:226-227` publishes `BUS_CHANNELS.ORDER_CONFIRMED` to "kick dispatch" — but nothing consuming `ORDER_CONFIRMED` kicks dispatch. What subscribes: `bootstrap/messaging.ts:102-108` (re-sends the owner an "order.confirmed" Telegram) and `lifecycle-handlers.ts:26` (resolves `dwell_pending`). Every rejection thus re-notifies the owner of a long-confirmed order and resolves dwell alerts on a false signal. **Fix:** delete the publish (journal + pump already re-dispatch) or publish a dedicated `assignment.rejected`.

### D-F12 — Closed-shift resurrection via a dangling offer
`routes/courier/shifts.ts:135-143` end-shift guard checks `status IN ('assigned','accepted','picked_up')`, not `'offered'` — a courier can close their shift with a pending offer. Accepting it later (`assignments.ts:151`) runs `UPDATE courier_shifts SET status='on_delivery' WHERE id=$1` unconditionally → the closed shift (with `ended_at` set) is resurrected; `completeDelivery` then flips it `'available'` → off-duty courier back in the pool. **[extends audit-money-orders L1]** **Fix:** include `'offered'` in the end/transition guards; guard shift updates with `AND status <> 'offline'`.

### D-F16 — `app_generate_settlements` appends items/money to an already-**paid** payout
`migrations/1790000000078:168-171,188-190`: `INSERT … ON CONFLICT (courier_id,location_id,period_start,period_end) DO UPDATE SET status=courier_payouts.status` (no status guard) then `UPDATE … total_earned = total_earned + …`. A regenerate for an old date after late-arriving delivered rows mutates the totals of an `approved`/`paid` payout — the paid amount on record no longer matches what was paid; delta never surfaced. **[relates to audit-money-orders H5]** **Fix:** when the existing payout isn't `pending`, open an adjustment payout (or refuse + report).

### D-F17 (= S-M9) — The `'voided'` settlement-reversal state is orphaned; a settled amount can never be corrected
`migrations/1780421100047:11-13` added `'voided'` + `voided_at/voided_reason/settlement_item_id` + the `prevent_cash_mutation` trigger with an `app.settlement_reversal` escape hatch. `migrations/1790000000073:9-13` **rewrote the CHECK without `'voided'`**. Repo-wide grep: zero code writes `'voided'`; owner `dispute`/`reopen` (`owner/settlements.ts:206-298`) only flips payout status — items and `total_earned` are immutable. A refund/dispute after settlement cannot be corrected in-system, and attempting the designed void now violates the CHECK (23514). **Fix:** restore `'voided'` + build the reversal path, or drop the dead void columns/trigger and document dispute-resolution as manual — pick one.

### D-F18 — `courier_cash_ledger` holds are never released and never reconciled against settlements
`migrations/1790000000028:3-8` (comment: "'release'/'settle' … not written here") — confirmed: `'hold'` is the only type ever inserted (`deliveryCompletion.ts:118-124`); nothing writes release/settle, incl. payout `pay`. The "till bond until shift reconciliation" (`deliveryCompletion.ts:117`) never closes, and nightly recon (`reconciliation.ts`) never checks `Σ ledger holds = Σ settlement_items` per courier/period — drift between the audit ledger and the settlement source-of-truth is undetectable. **Fix:** write a `'settle'` ledger row per settled order in `app_generate_settlements` (or at payout-paid); add a recon check ledger↔settlement_items.

### D-F21 — Stale `dispatch_exhausted_at` never cleared after a successful re-binding (dark, flag-gated)
Grep: set only in `courier-dispatch.ts:118-121`, reset nowhere. With `DISPATCH_OWNER_GRACE_ENABLED=true`, `graceCancelExhausted` (`courier-offer-sweep.ts:199-251`) checks only "no active assignment right now" — a recovered order mid-cycle between accept-timeout and next assign gets CANCELLED off a stale marker. **Fix:** `SET dispatch_exhausted_at = NULL` in the worker's assign path.

### S-M1 — `orders.courier_id` has no FK (denormalized mirror, hand-maintained by 3 sites)
`migrations/1780310074262_orders.ts:25` (`courier_id uuid`, never given a REFERENCES). Writers `assignments.ts:153`, `owner/dashboard.ts:290,357`, `bindingRelease.ts:32` maintain the mirror by hand; nothing stops a stale/foreign UUID persisting (reassign that updates the assignment then crashes before the mirror). Owner dashboards/analytics then show a phantom courier. **Fix:** `ADD FOREIGN KEY (courier_id) REFERENCES couriers(id) ON DELETE SET NULL`.

### S-M2 — `locations.status` is free TEXT with live value drift ('closed'/'open'/'active')
`migrations/1780310071220:37` (`status text NOT NULL DEFAULT 'closed'`, no CHECK). Seed writes `'active'` (`…024:11`), publish writes `'open'` (`owner/activation.ts:122-125`), old public read required `'active'` (`…016:15`), current read is `status IN ('active','open') OR published_at IS NOT NULL` (`…064:56`), dev mock still requires `'active'` (`mock-auth.ts:100`). Storefront visibility depends on which string a given code era wrote; the widened OR-gate is a patch over the drift. **Fix:** normalize then `CHECK (status IN ('closed','open','active'))`; collapse `active`→`open`.

### S-M4 — `cash_pay_with` has no DB constraint; invariant only caught by an after-the-fact sweep
`migrations/1780310074262:36` (`cash_pay_with integer`, no CHECK; the int-fix mig added none). `reconciliation.ts:147-154` detects `cash_pay_with < total` *after commit*. Negative/below-total values insert cleanly; the courier UI then instructs change-making from a nonsense denomination. Also `1790000000000_cash-pay-with-integer.ts` `down()` maps every value except 0/1 to NULL (lossy, money-destroying). **Fix:** `CHECK (cash_pay_with IS NULL OR cash_pay_with >= 0)`.

### S-M5 — Money knobs on `locations`/`delivery_tiers`/`promotions` lack CHECKs; percentage promos unbounded
`migrations/1780338982014:6-13` (`tax_rate numeric` unbounded; `min_order_value`/`free_delivery_threshold`/`delivery_fee_flat` no `>=0`; `delivery_tiers.min_order` unchecked, `max_distance_km` no `>0`); `1790000000017:25` (`discount_value CHECK (>0)` but no `<=100` for percentage type). The only guard is per-route Zod; any other writer (import backfill, spa-proxy claim `spa-proxy.ts:706,791`, SQL fixups) can persist a negative fee or a 5000% discount checkout then applies. **Fix:** one additive migration — non-negative money ints, `tax_rate BETWEEN 0 AND 100`, `CHECK (type <> 'percentage' OR discount_value <= 100)`.

### S-M6 — `orders` money-breakdown has no cross-column consistency CHECK
`migrations/1780338982013:5-8` — each of `delivery_fee/discount_total/tax_total` is `>=0` but nothing ties `total` to `subtotal + delivery_fee + tax_total - discount_total`, nor `discount_total <= subtotal + delivery_fee`. A partial UPDATE or a fee-mirror drift yields an internally inconsistent order that settlement and analytics interpret differently; only the recon sweep notices, after money moved. **Fix:** add the total-identity CHECK (or at minimum the discount bound).

### S-M7 — Allergen data (safety-critical, ADR-0014) lives in unvalidated `products.attributes` jsonb and is shape-trusted at read
`migrations/1780338982012:6` (`attributes jsonb NOT NULL DEFAULT '{}'`, no `jsonb_typeof` CHECK — unlike `onboarding_state`/`fallback_config`); `lib/product-mapper.ts:6-10` does `const bom = r.attributes?.bom ?? []; for (const line of bom) if (Array.isArray(line.allergens)) …`; served raw to the public in `read_public_menu`. A malformed import (`bom` as object) throws → storefront 500; `allergens` as a string is silently dropped → chips vanish with no error — the exact failure ADR-0014 exists to prevent. **[relates to arch F12 allergen fragmentation]** **Fix:** `CHECK (jsonb_typeof(attributes)='object' AND (attributes->'bom' IS NULL OR jsonb_typeof(attributes->'bom')='array'))` + Zod-parse at the mapper boundary.

### S-M8 — `prevent_cash_mutation` immutability is escapable by any operational session via a plain GUC
`migrations/1780421100047:16-35` bypasses the trigger when `current_setting('app.settlement_reversal', true) = 'true'`; `routes/customer/orders.ts:315-326` — a **customer-facing** cancel route — does `SET LOCAL app.settlement_reversal='true'` then nulls `cash_collected/cash_amount`. The escape hatch is an unauthenticated session variable reachable by every operational-role path (and any SQL-injection foothold). **Fix:** move the reversal path into a SECURITY DEFINER function owned by a privileged role (as `claim_transfer` does); remove the GUC branch from app-reachable code.

### S-M10 — Satellite/audit tables carry unconstrained uuid columns (no FK), several keyed on `location_id` under RLS
`courier_cash_ledger.courier_id/location_id` (`…028:13-14` — a money ledger on free uuids), `delivery_trace.*` (`…027:13-14`), `order_ratings.*` (`…025:8-10`), `order_status_history.location_id` (`…015:8`), plus polymorphic `anonymization_audit_log.subject_id` and `recipe_components.parent_id` (orphan-trigger for the product side only). A garbage `location_id` doesn't just dangle — it becomes invisible to every tenant RLS policy (write-only ghost rows in money/audit tables). **Fix:** FKs where the referent is never hard-deleted (locations, couriers); add the missing ingredient-side companion trigger.

### S-M11 — Order-lifecycle status modeling drift (enum vs TEXT+CHECK vs bare TEXT)
`orders.status`=enum; `orders.payment_status`=TEXT+CHECK coexisting with enum `payment_outcome` on the same table; `courier_shifts/assignments/couriers/payouts/payments.status`=TEXT+CHECK (each redefined by DROP/ADD churn); `location_alerts.status`=**bare TEXT, no CHECK** (`1780348982034:10`) yet filtered by the dwell/escalation workers. **[complements arch F4 — that doc covered code-side order-status modeling; this is the schema-side enum drift]** **Fix:** add the missing CHECK to `location_alerts.status`; standardize on one convention going forward.

### S-M12 — Real-brand PII baked into migrations (permanent git history before the OSS flip)
`migrations/1790000000024_update-sushi-durres-settings.ts:5-13` contains a real business name, **personal phone `+355683085694`**, street address, exact lat/lng; also `…021`, `…045`, `…046`, `…056/57`. With the ADR-020 open-source flip pending, these ship a real person's phone/geo in every clone forever, and make fresh-env builds diverge (seed rows exist only where a matching slug row existed). **[extends the secrets-exposure-incident scrub scope]** **Fix:** move brand/venue seeding to the encrypted dev-seed path; include these files in the history-scrub.

### L-M1 — Dwell clock measured from the wrong timestamp for every status
`migrations/1790000000078:29-33`: `app_dwell_due_orders` uses `COALESCE(o.confirmed_at, o.created_at)` for **all** kinds though `preparing_at`/`in_delivery_at` exist (mig 059). An order confirmed 20 min ago that just entered IN_DELIVERY instantly trips `dwell_en_route` (900s); `dwell_preparing` fires by wall-clock-since-confirm even if prep just started → false "stuck" alerts. **Fix:** key each kind on its own stage timestamp.

### L-M2 — Dwell escalation ladder is dead code; the delayed duplicate fires even after resolution
`DwellEscalationWorker` consumes `QUEUE_NAMES.DWELL_ESCALATE` (`workers/dwell-escalation.ts:20`) but nothing ever sends to that queue. `dwell-monitor.ts:127-142` instead sends two unconditional `NOTIFY_DISPATCH` jobs (immediate + `tier2Delay`) with no still-active re-check → the "Tier 2 only if unresolved" promise is false. `lifecycle-handlers.ts:60` `boss.cancel(\`notify.dispatch.${row.id}\`)` passes a fabricated string where pg-boss expects a job UUID — cancels nothing. **Fix:** enqueue `DWELL_ESCALATE` (with tier) from `scheduleEscalation`, or delete the worker + fake `boss.cancel`.

### L-M3 — `PICKED_UP` missing from every ETA terminal list → a finished pickup order reports a live, eventually-"overdue" ETA
`lib/etaService.ts:42` and `etaGather.ts:22,174` use `['DELIVERED','REJECTED','CANCELLED']` while the domain `isTerminal()` correctly includes PICKED_UP. `gatherOrderEtaRange` on a PICKED_UP order still sums `kitchenQueueAhead` and, with no pin, falls back to `fallbackDeliveryMin=20` → a completed pickup reports ~"10–40 min" ETA and flips `overdue:true` once elapsed passes it; every pickup ETA is also inflated by a phantom 20-min delivery leg. FE mirrors the gap (`OrderStatusPage.tsx:274,407,473`) keeping the 30s poll watchdog alive forever. **Fix:** add `PICKED_UP` to the three server terminal lists (leg=0 for pickup) and the FE terminal sets.

### L-M4 — Pickup orders are funneled into the delivery lane; `PICKED_UP` is unreachable in practice
The owner card's only READY action is `IN_DELIVERY` ("Send for delivery", `OrderCard.tsx:238-251`, no `order.type` branch); the route's honest-dispatch gate applies only when `type==='delivery'` (`orders.ts:885`), so a pickup order is PATCHed straight `READY→IN_DELIVERY` → the customer's pickup page shows "Preparing your order" while the DB says IN_DELIVERY and the pickup stepper computes `statusIndex=-1`. Nothing anywhere sends `PATCH {status:'PICKED_UP'}` — the terminal exists only in theory. **Fix:** block `IN_DELIVERY` for `type='pickup'` (409); add a `READY→PICKED_UP` owner action for pickup orders.

### L-M5 — Owner `/pickup` proxy broadcasts order status `PICKED_UP` while the order is IN_DELIVERY
`routes/owner/dashboard.ts:426-429` publishes `{type:'order.status', data:{status:'PICKED_UP'}}` though only the *assignment* moved to `picked_up`; `orders.status` stays IN_DELIVERY. The dashboard merge (no PICKED_UP in `STATUS_RANK`) displays a terminal, raw, unlocalized "PICKED UP" for an active delivery until the next refetch. **Fix:** publish `status:'IN_DELIVERY'` or a dedicated `assignment.picked_up` event type.

### L-M6 — Owner "kitchen busy" toggle affects neither the confirm timeout nor the ETA
POST `/orders` doubles the timeout on legacy `busy_mode` (`orders.ts:538`, written only by the dev seeder), while the toggle the owner actually has writes `kitchen_busy_until` (`owner/menu-availability.ts:39`), read only by the public menu badge (`public/menu.ts:365`). `etaGather.ts` reads neither. The one operational lever the owner has is cosmetic. **Fix:** treat `kitchen_busy_until > now()` as busy in POST `/orders` and `gatherOrderEtaRange`; retire `busy_mode`.

### L-M7 — `delivery_tiers.min_order` is silently ignored
Column exists (`…014:20`) and owners can seed it, but `resolveDeliveryFee`'s `DeliveryTier` interface (`lib/order-pricing.ts:144-147`) carries only `max_distance_km, fee` and the SELECT (`orders.ts:494`) doesn't fetch it. A per-zone minimum is never enforced — small orders accepted for the farthest zone. **[schema companion of S-M5]** **Fix:** fetch `min_order`, 422 `MIN_ORDER_NOT_MET` when `subtotal < tier.min_order` (or drop the column).

### L-M8 — Tiers configured but venue coords NULL → silent flat-fee with unlimited range; `delivery_radius_km` never enforced
`lib/order-pricing.ts:171-183`: the tier branch requires `location.lat != null && location.lng != null`; otherwise it falls through to `delivery_fee_flat` with **no distance bound**. A flat-fee-only location accepts a pin at any distance, and `locations.delivery_radius_km` (settable via spa-proxy `:710`) is checked nowhere in the order path. **Fix:** tiers-with-missing-coords → 422 `DELIVERY_NOT_CONFIGURED`; enforce `delivery_radius_km` in the flat-fee branch.

### L-M9 — Owner dashboard: REJECTED and PICKED_UP orders live in the "live" tab forever, never reach history
`apps/web/src/pages/admin/DashboardPage.tsx:344-347,367` filters live as `status !== 'DELIVERED' && status !== 'CANCELLED'` and history as `=== 'DELIVERED' || === 'CANCELLED'` — two of the four terminal statuses are misclassified as active and unselectable in history. **Fix:** filter on the shared terminal set {DELIVERED, CANCELLED, REJECTED, PICKED_UP}.

---

## LOW

- **S-L1** — `migrations/1790000000019_add_categories_unique.ts:5-20`: the dedup DELETE is a provable no-op (`DISTINCT ON` collapses before the `HAVING count>1`), so `dup` is always empty; only the `CREATE UNIQUE INDEX` ever acted. Latent footgun if copied as a template.
- **S-L2** — Destructive `up()` history: `1780421031109:5` `DROP TABLE courier_invites CASCADE` (dropped FORCE-RLS until re-forced by mig 051), `1790000000017:5` `DROP TABLE promotions CASCADE`.
- **S-L3** — Data-destroying `down()` paths: most courier/settlement migrations `DROP TABLE … CASCADE`; `cash-pay-with-integer.ts:12-18` down-casts amounts >1 to NULL. `migrate:down` is wired in `package.json:39`. Consider `throw` on red-line-table down().
- **S-L4** — `location_alerts.kind` added `NOT NULL` with no DEFAULT (`1780348982034:9`) — safe only because the table held placeholder rows; template hazard.
- **S-L5** — Circular FK pair `settlement_items.assignment_id` (RESTRICT) ↔ `courier_assignments.settlement_item_id` (NO ACTION) — hard-wires "no cleanup" nobody documented.
- **D-F9** — `routes/courier/assignments.ts:144-147` offered-accept branch checks `status='offered'` but not `offered_expires_at` → an expired offer is acceptable until the next 1-min sweep (benign; partial-unique prevents double-binding). Add `AND (offered_expires_at IS NULL OR offered_expires_at > now())`.
- **D-F19** — Owner `/settlements/regenerate` (`owner/settlements.ts:301-317`) runs the money sweep for **all** tenants at an arbitrary date (route comment admits it). Add `p_location_id` to `app_generate_settlements`. **[confirms audit-money-orders L4]**
- **D-F20** — Dead `EN_ROUTE` branch: `lifecycle-handlers.ts:15` maps `EN_ROUTE` which is not in `ORDER_STATUSES`; unreachable (also `dwell-thresholds.ts:26`).
- **L-L1** — `customer/orders.ts:90-99` still serves a naive single-number `etaMinutes` next to the honest range, contradicting etaService's "never a single number" contract.
- **L-L2** — `ORDER_CANCEL_AFTER_DISPATCH` (`registry.ts:12`, `customer/orders.ts:341`) has zero subscribers — even once the cancel 500 is fixed, no owner notify / dwell resolve / courier push happens.
- **L-L3** — FE status maps omit PICKED_UP/SCHEDULED (`OrderCard.tsx:44-76`, `OrderStatusPage.tsx:21-63`) → grey badge, `ti-help` icon, raw "PICKED UP" text in an otherwise-localized UI.
- **L-L4** — Duplicate FE state machine (`packages/ui/src/utils/index.ts:26-43`) drifted from the domain machine (missing edges, contains server-forbidden ones). Uncalled today; delete or re-export from `@deliveryos/domain`.
- **L-L5** — Note: the customer cancel-after-dispatch 500 (writes non-existent `cancelled_at`/`cancellation_reason`, evades the raw-status-update guard because it's split across lines) is **[already logged as audit-money-orders C2]** — re-confirmed here from the schema side: no migration ever adds those columns to `orders` (only `orders.rejection_reason` exists). Fix belongs with C2.

---

## Verified clean (checked, no defect — do not re-audit)

- **Geo math** — `distanceKm` haversine correct (`lib/geo.ts:1-15`); `deviationMeters` equirectangular fine at city scale (`lib/routing.ts:72-97`); no degree/meter mixups. **GPS precision** — `courier_positions numeric(8,5)` (~1.1 m) cannot break any geofence at these radii.
- **Fee authority** — POST `/orders` computes subtotal/fee/tax entirely server-side from the in-tx snapshot; tier boundary `distKm <= max_distance_km` inclusive with 3-dp rounding, no boundary hole. Nothing fee-related is trusted from the client.
- **ETA null-safety** — `computeEtaRange` is total (floors, min band, NaN-guards, monotonic cap); no NaN/0/low>high reaches the customer.
- **Idempotency schema** — `idempotency_keys` composite PK (mig 029), `import_sessions` partial uniques, `courier_cash_ledger UNIQUE(order_id,type)`, `order_sensor_events UNIQUE(order_id,event_type)`, `claim_invites` one-active-per-source — all present and matching code. The active-redispatch trap was already fixed in-tree (partial unique on active assignment states, mig 073).
- **jsonb with guards** — `dwell_thresholds`, `onboarding_state`, `fallback_config` all carry `jsonb_typeof` CHECKs and are Zod-validated on write. (Contrast S-M7: `products.attributes` has neither.)
- **Anonymization design** — `anonymized_at` tombstones, `phone DROP NOT NULL`, dispute-window floor in `anonymize_stale_delivery_trace`, GDPR `receiver_*` coverage — internally consistent.
- **Domain cleanup paths** — `releaseBindingAndReoffer` / `completeDelivery` / no-show / grace-cancel all route through `updateOrderStatus`; the CANCELLED / READY-revert fold is idempotent and cash-safe. Assignment/settlement period boundaries are half-open (no double-count); every assignment status written is in the mig-073 CHECK.

---

## Cross-lane compounding (read this before triaging point fixes)

**A single deactivated-or-unresponsive courier can absorb a venue's entire dispatch capacity indefinitely, silently.** D-F3/S-H1 (deactivated courier still selected) + D-F1 (exhaustion counter never accumulates) + D-F2 (recency pick + no tried-courier memory) chain: the same broken courier is re-offered every 60s tick, no other courier is tried, no exhaustion alert ever fires, and (D-F6) the order can be stranded at `IN_DELIVERY` with no one driving. The customer sees "on the way" forever. Fix these as one cluster, not four tickets.

**The schema is not reproducible (S-H6 + S-H7 + `--no-check-order`), which caps the trust you can place in every RLS/money guard below it** — you cannot prove prod, staging, a DR restore, and an OSS clone share the same constraints. Land 075/076 in-repo and retire reused numbers *before* the NOBYPASSRLS flip council and *before* the open-source flip, or those gates are asserting over an unknown schema.
