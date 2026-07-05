# Recon #2 — Concurrency / Races / Idempotency / Transaction Boundaries (2026-07-03)

READ-ONLY deep recon, run #2. Excludes run #1 territory (money state-machine leaks, courier
double-accept, sweep-vs-user cancel, GDPR-erasure stranding, GUC discipline, pg-boss retry/DLQ).
Every finding below was verified against the live working tree (file:line cited). Interleavings are
written as thread A vs thread B. Counts: **1 CRITICAL · 5 HIGH · 5 MED · 6 LOW**.

---

## CRITICAL

### C1 — Publish-before-commit is systemic: `NOTIFY` escapes the caller's transaction on every status transition

**Root mechanism:** `packages/platform/src/message-bus.ts:116-131` — `PgMessageBus.publish` issues
`NOTIFY` on `this.pool` (a separate autocommit connection), NOT on the caller's transactional
client. Postgres delivers it to listeners immediately, regardless of whether the caller's
`BEGIN…COMMIT` ever commits.

**Affected call sites (publish while the caller's txn is still open):**
- `apps/api/src/lib/orderStatusService.ts:187-216` — every `updateOrderStatus` publishes
  `order.status` to the customer channel, a dashboard delta, and `ORDER_CONFIRMED`/`ORDER_REJECTED`
  lifecycle events pre-commit. Worse: `fetchOrderDelta` (line 202) reads on the in-txn client, so
  the dashboard is broadcast **uncommitted** state.
- `apps/api/src/lib/dispatch.ts:46-55` — `attemptHonestDispatch` publishes `assignment.created` +
  `task_assigned` to the courier before the caller's `withTenant` COMMIT.
- `apps/api/src/lib/courierAssignmentService.ts:55` — `ORDER_COURIER_ACCEPTED` pre-commit (the
  sibling `ORDER_PICKED_UP`/`ORDER_DELIVERED` at `assignments.ts:276/386` are correctly post-commit).

**Interleaving (deliver path example):**
1. A (courier deliver, `routes/courier/assignments.ts:316-378`): BEGIN → `completeDelivery` →
   `updateOrderStatus(DELIVERED)` → **NOTIFY `order.status=DELIVERED` fires now** (separate conn).
2. B (customer page WS): receives DELIVERED, repaints; or refetches `/orders/:id` on another
   connection → reads the pre-commit `picked_up` row (stale flash).
3. A: anything after the publish throws (`SET LOCAL statement_timeout`, later insert, 40001) →
   ROLLBACK. DB says picked_up forever; customer was told DELIVERED. Ghost event, no correction.

Dispatch variant: courier app receives `task_assigned` → fetches `GET /me/assignments` on a fresh
connection → assignment row not committed yet → empty list / "task not found"; if A rolls back, the
assignment never existed at all.

**Invariant violated:** no consumer may observe a transition before it is durable.
**Fix direction:** collect events during the txn, flush after COMMIT — the pattern already used by
`routes/orders.ts:579→584` and `workers/courier-dispatch.ts:150-153`. One shared helper
(after-commit event buffer) fixes all three sites.

---

## HIGH

### H1 — `courier_shifts` double-open: phantom TOCTOU + midnight clock hole; no DB backstop

- `apps/api/src/lib/shiftService.ts:15-57` (openShift), `apps/api/src/routes/courier/shifts.ts:196-274`
  (transition), `:122-147` (end); DDL `packages/db/migrations/1780421036157_courier-shifts.ts` —
  **no unique constraint on active shifts** (assignments got `courier_one_active_assignment`
  in mig 066/073; shifts never did).

Three ways to mint two concurrently-active shift rows:
1. **Phantom TOCTOU:** courier double-taps start (or start + transition race). A: openShift
   `SELECT … FOR UPDATE LIMIT 1` → 0 rows (FOR UPDATE locks nothing on an empty result). B: same →
   0 rows. A INSERTs shift-1, COMMIT. B INSERTs shift-2, COMMIT. Two `available` shifts.
2. **Midnight hole (clock race, no concurrency needed):** openShift matches only
   `DATE(started_at) = CURRENT_DATE` (shiftService.ts:18, DB-server UTC date). A shift opened 23:50
   and still active is invisible after midnight UTC → the next start INSERTs a second active shift.
3. **Unordered pick:** `shifts/transition` (shifts.ts:196-200) SELECTs with no status filter, no
   ORDER BY → `rows[0]` is arbitrary; the `to=available` branch (line 270-274) can resurrect an old
   offline row (`ended_at = NULL`) while a different row is already active.

**Downstream corruption:** `/me/shift/end` (shifts.ts:133) terminates only `rows[0]` → the ghost
shift stays `available` forever → honest-dispatch (`lib/dispatch.ts:27-40`) selects that courier as
free and assigns real deliveries to someone off shift; liveness/stats skew.
**Invariant:** ≤1 active shift per (courier, location).
**Fix:** partial unique index `ON courier_shifts(courier_id, location_id) WHERE status IN
('available','on_delivery')` + adopt-on-conflict; drop the `DATE()=` predicate (match any active
row); add ORDER BY/status filter to the transition SELECT.

### H2 — Crypto checkout commits the order as a CASH order, then re-marks it crypto post-commit in autocommit; pooled connection held across the Plisio HTTP call

- `apps/api/src/routes/orders.ts:579` (COMMIT — the INSERT hardcodes `payment_method='cash'`,
  `lib/order-persistence.ts:87`) → `:584-618` (`order.created` published) → `:641-666` (post-commit,
  autocommit statements: INSERT payments → `UPDATE orders SET payment_method='crypto',
  payment_status='pending'` → `provider.createCharge(...)` **external HTTP on the still-held pooled
  client** → UPDATE payments).

**Interleaving:**
1. A commits the order (visible to all as a normal cash PENDING order) and publishes `order.created`.
2. B (owner dashboard, reacting live) confirms it as a cash order before A's line-651 UPDATE lands —
   or A crashes/errors anywhere in 579→651 → the crypto-intent order is **permanently** a cash
   order: customer is never charged, courier is told to collect cash the customer never agreed to.
3. Independent: the comment at :638 claims the order is "held: not offered to fulfillment until the
   webhook flips payment_status" — **no code enforces this.** `payment_status` is read nowhere in
   the confirm/dispatch path (grep: only payments-webhook.ts, refunds.ts, deliver path). The hold
   invariant is documentation-only.
4. Pool: `createCharge` runs on the checked-out client (released only in `finally`, :706). N slow
   provider calls hold N of `OPERATIONAL_POOL_SIZE` connections — exactly the pool-wedge the 4.5s
   in-txn timeout at :112-118 was built to prevent, reintroduced after COMMIT.

**Invariant:** a crypto-intent order must never be committed/observable as a cash order; no external
call on a held pool connection.
**Fix:** thread payment method into `insertOrderWithItems` (set method/status inside the txn);
release the client before `createCharge`; gate PENDING→CONFIRMED on
`payment_status NOT IN ('pending')` for prepaid methods. (DARK: PAYMENTS_CRYPTO_ENABLED off —
severity applies the day it's lit.)

### H3 — Deliver-vs-webhook: cash + crypto double-collection, undetected

- `apps/api/src/routes/courier/assignments.ts:319-340` + `lib/deliveryCompletion.ts:57-124`
  vs `apps/api/src/routes/payments-webhook.ts:64-81`.

**Interleaving** (enabled by H2's missing fulfillment gate — an unpaid crypto order can be
confirmed/dispatched):
1. Crypto order, `payment_status='pending'`; courier at the door; customer's crypto tx confirms.
2. A (deliver): reads `o.payment_status='pending'` (assignments.ts:320, row locked by the plain
   FOR UPDATE) → `payment_method='crypto' && status!=='paid'` so the prepaid override (:338) does
   NOT fire → outcome resolves from `cash_collected=true` → `paid_full` → cash ledger 'hold',
   `payment_outcome='paid_full'`, COMMIT.
3. B (webhook, was blocked on the row lock): `UPDATE orders SET payment_status='paid' WHERE …
   payment_status IN ('pending','authorized')` (payments-webhook.ts:66-68) — **no guard on order
   status or payment_outcome** → succeeds.
4. End state: `payment_status='paid'` AND `payment_outcome='paid_full'` — customer paid twice
   (crypto + cash at the door). `workers/reconciliation.ts` has zero payment coverage (grep:
   no payment_status/payment_outcome) → never detected, never refunded.

**Invariant:** exactly one settlement medium per order.
**Fix:** webhook adds `AND payment_outcome IS DISTINCT FROM 'paid_full'`, recording an
over-collection `refund_due` payment_event in the ELSE branch. (Adjacent to run-#1 money findings
but a distinct interleaving; flagged as such.)

### H4 — Advisory lock ID 3 shared by signal-raiser and backup-verify → daily 04:00 mutual exclusion

- `apps/api/src/workers/signal-raiser.ts:30` (`pg_try_advisory_lock(3)`, cron `*/5 * * * *`) vs
  `apps/api/src/workers/backup/backup-verify.ts:19,71` (`BACKUP_VERIFY_LOCK = 3`, scheduled
  `0 4 * * *` in backup-verify-scheduled.ts:37, held up to `TIMEOUT_MS` = 30 min).

Both fire at 04:00:00 daily. If signal-raiser wins → the daily DR restore-verification drill is
**silently skipped for the day**. If backup-verify wins → fraud/velocity signal raising is
suppressed for up to 30 minutes (~6 skipped runs).
**Invariant:** unrelated workers must not share an advisory lock key.
**Fix:** move backup-verify to a dedicated id outside the 2–9 worker range.

Full lock-ID inventory (verified): 2 dwell-monitor · **3 signal-raiser ⟷ backup-verify (COLLISION)** ·
4 anonymizer-retention · **5 order-timeout-sweep ⟷ access-request-retention (COLLISION, see M3)** ·
6 access-request-reconcile · 7 acquisition-retention · 8 delivery-trace-retention ·
9 courier-offer-sweep · 8192 rates-refresh · uint32(sha256) backup/index.ts. All acquire/release on
the same checked-out client (the historical backup-verify unlock-scope bug is fixed).

### H5 — Owner Telegram notification dedup is process-memory only → duplicate sends on retry across restart/instance

- `apps/api/src/notifications/workers/index.ts:70` (`dedupCache = new Set<string>()`), checked
  `:350`, populated only after a successful send `:484`. The `notification_outbox_audit`
  `ON CONFLICT DO NOTHING` inserts (`:439/:491`) are audit-only — they never gate the send.

**Failure sequence:** job sends to Telegram (owner receives it) → a later step in the same job
throws (audit insert blip, next target's build) → pg-boss retries the whole job → retry lands after
a deploy/restart or on the other machine (fly.toml runs separate `web` + `worker` processes; deploys
recycle them) → empty Set → message re-sent to every target.
**Invariant:** at-most-once owner notification per (event, entity, location).
**Fix:** durable delivered-marker (unique row keyed on the dedupKey) checked before dispatch,
written in the same breath as the send result.

---

## MED

### M1 — OTP single-use tokens are double-consumable; attempts cap is check-then-act

- Order-create path: `apps/api/src/routes/orders.ts:161-175` (verified-session consume: SELECT
  `consumed_at IS NULL` without lock → `UPDATE … SET consumed_at=now() WHERE id=$1` with **no
  `AND consumed_at IS NULL`**, no rowCount check) and `:309-328` (phone_otp: same pattern, plus the
  `attempts < 5` gate is read-then-verify).
- Verify endpoint: `apps/api/src/routes/customer/otp.ts:137-186` — same unguarded consume at
  `:185-186`; attempts gate at `:152` read pre-verify.

**Interleaving:** two concurrent POSTs with the same single-use token/code both SELECT the unconsumed
row → both verify → both consume → two orders ride one OTP verification (or two `verified_token`s
minted from one code). Parallel wrong-code requests all read `attempts=4` → all get a guess →
brute-force cap exceeded (the fastify rate-limit is per-instance memory, so it multiplies by machine
count).
**Invariant:** single-use means exactly-once; attempt caps must be atomic.
**Fix:** `UPDATE … SET consumed_at=now() WHERE id=$1 AND consumed_at IS NULL RETURNING id` + treat
rowCount=0 as not-verified; gate attempts via `UPDATE … SET attempts=attempts+1 WHERE attempts<5
RETURNING`.

### M2 — Courier refresh rotation: benign concurrent refresh revokes the whole session family

- `apps/api/src/routes/courier/auth.ts:400-427`. A and B fire the same refresh token concurrently
  (network retry, two tabs). Truly parallel: B's `FOR UPDATE NOWAIT` (:401) throws 55P03 →
  unhandled → generic 500. Slightly staggered (the common retry case): A rotates and sets
  `revoked_at`; B re-reads the row post-commit → `revoked_at` set → **reuse-detection** (:418-427)
  revokes the entire family **including the new session A just minted** → courier force-logged-out
  mid-shift by their own retry.
**Invariant:** a benign retry must be distinguishable from token theft.
**Fix:** grace window — if `revoked_at` is < N seconds old and `replaced_by` is set, return the
successor session's token instead of nuking the family; map 55P03 to 409/retry.

### M3 — Advisory lock ID 5 shared by order-timeout-sweep and access-request-retention

- `apps/api/src/workers/order-timeout-sweep.ts:19` (`SWEEP_LOCK_ID=5`, every minute) vs
  `apps/api/src/workers/access-request-retention.ts:10` (`RETENTION_LOCK=5`, daily 03:00). At
  03:00:00 they contend; whichever loses silently skips (retention → skips a day; sweep → skips a
  minute). Self-healing on the next tick, hence MED. Fix: renumber.

### M4 — Claim decline: invite burned in txn 1, shadow erased in txn 2 — crash strands un-erased scraped data behind a dead token

- `apps/api/src/modules/acquisition/claim.ts:133-157`: `declineAndErase` COMMITs
  `used_at/revoked_at` on the invite, **then** calls `hardDeleteShadow` and `flagTerminal` in
  separate transactions. Crash between them → the restaurant's one-click "delete it" consumed the
  token but erased nothing; retry gets INVALID_OR_EXPIRED_TOKEN. Data survives until the
  acquisition-retention TTL sweep — the Art-14 notice's "one click … erase" promise silently held
  only eventually.
**Invariant:** decline == erase, atomically or with a durable driver.
**Fix:** perform the erase (`erase_shadow_tenant` is a single fn call) inside the same txn as the
invite burn, or insert an outbox "erase-due" row in that txn for the reaper to drain.

### M5 — Product PATCH: attributes jsonb read-merge-write → lost updates on a safety-adjacent column

- `apps/api/src/routes/owner/products.ts:419-444`: SELECT `attributes` (no lock) → JS spread-merge
  (`stockCount`/`taste`/`bom`/free-form) → `UPDATE … attributes = $9` (whole-column overwrite).
- **Interleaving:** A PATCHes `stockCount`, B concurrently PATCHes `taste`. Both snapshot the same
  base; B commits last → A's `stock_count` silently reverts. `attributes.bom` carries recipe /
  characteristics data (ADR-0014 territory) — a clobber here can silently drop it.
- Same class: menu-import commit upsert (`routes/owner/menu-import.ts:342-352`) overwrites
  `attributes` wholesale — an import racing a live owner edit clobbers it.
- The repo already knows the rule: `lib/notificationPrefsService.ts:8` — "atomic per-cell jsonb_set
  under a FOR UPDATE row lock — NO read-merge-write". Products PATCH predates/violates it.
**Fix:** `attributes = attributes || $9::jsonb` for the touched keys (or jsonb_set per key) under
FOR UPDATE.

---

## LOW

### L1 — Liveness-checker cross-tick state is per-instance memory
`apps/api/src/workers/liveness-checker.ts:25,63,95` — `previouslyStale` Set; the singleton job can
hop instances → repeated "newly stale" alerts + dropped `WORKER_RECOVERED`. Persist last-alerted
state.

### L2 — telegram.poll acks before processing
`apps/api/src/notifications/workers/telegram.poll.ts:30-33` — `this.offset = update.update_id + 1`
executes **before** `processUpdate`; a handler throw means the update is confirmed to Telegram and
never redelivered → a lost `/start` owner-bind. Advance the offset only after success.

### L3 — Idempotent double-submit returns 409 instead of the replayed order
`apps/api/src/routes/orders.ts:376-393` (check) + `:693-694` (23505 → `IDEMPOTENCY_CONFLICT`).
Composite PK (`migrations/1790000000029`) correctly prevents a double order, but two concurrent
same-key requests → loser gets a 409 error, not the winner's order (replay only works when the
first fully committed before the second's SELECT). Client double-click = error UX. Fix: on 23505,
re-SELECT the key and return the winner's order as 200.

### L4 — Deliver handler's plain `FOR UPDATE` locks the tenant's `locations` row
`apps/api/src/routes/courier/assignments.ts:319-326` — the ca⋈orders⋈locations join uses bare
`FOR UPDATE` (contrast `FOR UPDATE OF ca` at `:436/:501`) → every delivery completion serializes on
the location row against the storefront pause-toggle (`lib/storefrontService.ts:103`) and any other
locations-locker, for the whole completion txn (which includes pre-commit NOTIFY publishes per C1).
Fix: `FOR UPDATE OF ca, o`.

### L5 — Velocity throttle is count-then-insert
`apps/api/src/routes/orders.ts:254-298` — N parallel checkouts each COUNT committed
`velocity_events` < limit → all pass → the 5/15min phone cap (and 20/15min IP cap) is exceeded by
exactly the attacker's parallelism; the route-level fastify limiter is per-instance memory. Bounded
overshoot → LOW. Fix if desired: take a per-phone advisory xact lock around count+insert.

### L6 — Theme version = MAX(version)+1
`apps/api/src/routes/owner/themes.ts:82-101` — two concurrent theme PUTs both read MAX=v, both
insert v+1 (ON CONFLICT target is `(location_id, css_hash)`, not version) → duplicate version
numbers; MAX-based CSS resolution then picks arbitrarily. Cosmetic surface. Fix: sequence or
`INSERT … SELECT COALESCE(MAX(version),0)+1` with a unique (location_id, version) retry.

---

## Verified clean (checked, not just skipped)

- `modules/acquisition/provisioning.ts` — exemplary: grant FOR UPDATE → state-pinned advance →
  consume-LAST, all one txn; partial-unique mint guard.
- `lib/storefrontService.ts` — FOR UPDATE CTE toggle with was/now distinction; nonce consumed
  atomically with the toggle.
- `routes/owner/settlements.ts` — all payout transitions are status-guarded UPDATEs (RETURNING +
  rowCount check).
- `lib/order-persistence.ts` — true transactional outbox (pg-boss enqueue via in-txn `db: txDb`,
  singletonKey dedupe); idempotency INSERT backed by composite PK.
- `routes/owner/menu-import.ts` commit — session FOR UPDATE + committed/token idempotency replay;
  website brand fetch explicitly outside the txn (`:533`).
- `lib/settlement-period.ts` — pure UTC boundaries, `[start, end)` consistent; no DST hazard.
- `lib/notificationPrefsService.ts` — atomic jsonb_set under FOR UPDATE (the pattern M5 should copy).
- Concurrent dispatch double-assign — DB-backstopped by `courier_one_active_assignment` +
  `courier_assignments_order_active_uniq` partial uniques (mig 066/073); loser gets 23505, no
  silent corruption (shifts lack exactly this — H1).
- `lib/r2-storage.ts` — deterministic keys; retry = idempotent overwrite.
- `payments-webhook.ts` writer guards — monotonic (`WHERE payment_status IN (…)`); the gap is the
  missing payment_outcome cross-guard (H3), not the state machine itself (run-#1 territory).
