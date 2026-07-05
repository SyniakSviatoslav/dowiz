# S8-JOBS/NOTIFICATIONS Port — SYSTEM BREAKER FINDINGS

> Adversarial pass over `proposal.md` / `open-questions.md` / `threat-model.md` against ground truth
> (`apps/api/src/workers/*`, `apps/api/src/notifications/workers/index.ts`, `telegram-webhook.ts`,
> `order-persistence.ts`, `packages/db/migrations/*`, `packages/platform/src/queue-provider.ts`).
> Read-only verification. **No fixes proposed — the architect fixes.** Each finding = specific,
> demonstrable break + violated invariant. Severity is calibrated, not inflated.

**Counts: CRITICAL 1 · HIGH 3 · MEDIUM 3 · LOW 2**

---

## [CRITICAL] B-CONSIST · The busiest notify handler is idempotent ONLY by an in-memory Set, not a Postgres guard — a worker restart / second instance / cutover flip double-sends the owner's Telegram

**Violated invariant:** proposal §1 seam-1 ("every handler that sends money or a notification MUST
therefore be idempotent by a **Postgres-level guard** … never by a queue-level at-most-once"), §3.4
(`notify.telegram.send`: "dedup via a dedup key … archive to `notification_outbox_audit`"), Q1/Q6, and the
base-knowledge rule the packet repeats verbatim: *"idempotency in Postgres, not Redis."*

**The break — three layers, all verified, that together leave zero durable dedup:**
1. The dedup for `notify.telegram.send` is an **in-process `Set`**:
   `apps/api/src/notifications/workers/index.ts:70` (`private dedupCache = new Set<string>()`),
   checked at `:350` (`if (this.dedupCache.has(dedupKey))`), populated **only on delivered** at `:484`.
   A process restart, a second worker instance, or the Node→Rust fleet flip starts with an **empty**
   cache.
2. The `notification_outbox_audit` "archive" the packet cites as the dedup floor has **no unique
   constraint** — `packages/db/migrations/1790000000007_notification-outbox-audit.ts:12-31` creates only
   a `gen_random_uuid()` PK + **non-unique** indexes on `location_id`, `status`, `[event, target_id]`.
   Therefore the `INSERT … ON CONFLICT DO NOTHING` at `index.ts:439-444` / `:492-496` / `:521-526`
   **never conflicts** and the code never inspects its rowcount — it is an append-only log, **not** an
   idempotency gate. Dispatch happens unconditionally after it.
3. The enqueue-side `singletonKey` (`order-persistence.ts:179`, `order-timeout-sweep.ts:116`) is a
   **silent no-op** on this queue — the packet's own `Q-SINGLETONKEY-POLICY` states v10 honors
   singletonKey dedup only on `policy:'short'`, and the queue is created bare (`standard`). It also only
   guards duplicate **enqueue**, never duplicate **execution** of the same job on retry.

**Break scenario (the exact at-least-once case proposal §3.1 describes):** owner's order comes in →
`order.created` job claimed → `handleTelegramSend` sends the Telegram message successfully → process
crashes (or the cutover flip stops this worker) **before** completion is committed → at-least-once
re-delivers the job to a fresh process → `dedupCache` is empty → **the owner receives a second
"new order" Telegram.** Same for `order.timeout_cancelled` re-sends. The packet instructs the port to
"carry the dedup-key … the notification is at-least-once with a **dedup floor**" (§3.4) — the floor it
names does not exist in Postgres; a faithful port carries an in-memory dedup and reproduces the
double-send. This is a **direct falsification of the surface's stated correctness spine**, on its
highest-frequency job (~1-2k/day per §2 back-of-envelope). Money blast radius is bounded (settlement /
auto-cancel / refund ARE DB-guarded — see HIGH-3), but the invariant the whole packet is built on is
broken here.

---

## [HIGH] B-SEC · The Q4 "parity oracle" contradicts itself — the spec cited as proof of the fail-closed fix asserts behavior the live handler does not implement

**Violated invariant:** proposal §Parity-oracle (line 42, `telegram-webhook.spec.ts` listed as a
load-bearing **green** as-is spec), DoD §12 ("jobs/notifications E2E slice **green** — as-is specs —
`telegram-webhook`"), vs §4.2 line 299 + threat S8-T5 (the same spec "asserts the opposite — a live
fail-open"). A test cannot simultaneously be a green as-is oracle **and** the assertion that catches a
still-present defect.

**The break (verified both sides):**
- Live handler fail-**opens**: `telegram-webhook.ts:96-99` — with `TELEGRAM_BOT_SECRET` set and the
  header **absent**, it logs a warning and **processes the request** (returns `200 {ok:true}` at `:123`).
  401 fires **only** on a present-but-wrong header (`:89-95`).
- The oracle asserts the opposite: `e2e/tests/telegram-webhook.spec.ts:53-59` `WEBHOOK-2` POSTs with no
  header and `expect(resp.status()).toBe(401)`.

`WEBHOOK-2` therefore **cannot pass against the real handler** when the secret is configured. Either
(a) it is **red / not green** today → the DoD's "as-is specs green" claim is false and the parity oracle
is broken; or (b) it is not actually exercising the missing-header branch (skipped on prod via
`test.skip(isProd,…)` at `:29`; on staging the URL is built from `BOT_SECRET` at `:6`, so a secret
mismatch yields 404, not the handler's 200) → the fail-open has **no working guardrail** and the packet's
"matches the existing stale test" is aspirational. Signing Q4 on "the suite already asserts missing→401,
so we're aligned" ships the fix with an oracle in an indeterminate/contradictory state.

**Calibration (not inflation):** the fail-open's *current* real-world exploitability is **lower** than
§4.2 / TB-1 imply — today the secret is embedded in the **URL path literal**
(`telegram-webhook.ts:75`, `` /webhook/telegram/${telegramBotSecret} ``), so an attacker must already
know the secret to reach the handler at all, and that same value is the header. The fail-open only
becomes a live forge-vector once the port demotes the URL secret to a router-token `:secret` param (the
packet's own plan) — so the FIX-IN-PORT priority is right, but the threat narrative overstates *present*
exploitability while the oracle backing it is not demonstrably green.

---

## [HIGH] B-CONSIST / B-DATA · The settlement double-fire money-safety argument rests on three pillars that are all false; the mechanism that actually prevents double-pay is never named

**Violated invariant:** proposal §6 (idempotency table: settlement = "watermark-based … one atomic DB
call"), §7 line 368 ("each cron takes a `pg_try_advisory_lock(id)`"), Q7c + `Q-085-WATERMARK` (the
"mig 085 watermark `2026-07-10` HARD gate"), DoD §12 line 511 ("double-fired settlement → one settlement,
**watermark holds**"), threat S8-T3/J9 ("advisory lock … the money-idempotency **backstop of last
resort**").

**Each pillar, checked against ground truth:**
1. **No advisory lock on the settlement cron.** `apps/api/src/workers/settlement-cron.ts:16-27` registers
   the queue + a pg-boss `schedule(...,{singletonKey})` and **nothing else** — no `pg_try_advisory_lock`.
   (Every *other* cron does take one: `order-timeout-sweep.ts:40`=5, `dwell-monitor.ts:33`=2,
   `signal-raiser.ts:30`=3, `anonymizer-retention.ts:33`=4, `acquisition-retention.ts:38`=7, etc.) The
   packet's "carry the existing advisory-lock pattern verbatim" has **nothing to carry** for the one
   money-critical cron, and threat-S8-T3's "advisory-lock backstop of last resort" **does not exist**
   for settlement. Its `singletonKey` is additionally a no-op (bare `standard` queue, `:25`, per the
   packet's own `Q-SINGLETONKEY-POLICY`).
2. **There is no watermark.** `app_generate_settlements` — `1790000000078_phase2-sweep-fns.ts:160-195` —
   contains **no watermark column or predicate**. Idempotency is actually: `settlement_items.assignment_id`
   **UNIQUE** (`1780421100045_settlement-items.ts:15`) + `AND NOT EXISTS (SELECT 1 FROM settlement_items …)`
   (`:178`) + `ON CONFLICT (assignment_id) DO NOTHING` (`:183`) + `FOR UPDATE OF ca SKIP LOCKED` (`:179`)
   for concurrency. (This guard *is* robust — concurrent double-fire is safe — which is why this is HIGH,
   not CRITICAL: the money is protected, but by an entirely different mechanism than the packet claims.)
3. **Migration 085 / the `2026-07-10` watermark does not exist.** `grep` across all of `packages/db/`
   and `apps/api/src/` returns **zero** matches for `2026-07-10`, `settlements-catchup`, `catchup`,
   `watermark`, `last_settled`, `settled_watermark`. The "085 watermark HARD gate" surfaced as a 🔴
   operator timing landmine (§Census line 32-34, §6 line 361-364, §10, Q7c, threat §5 line 103) has **no
   referent in the frozen DB.**

**Why it breaks the port:** the DoD probe "double-fired settlement → one settlement (**watermark holds**)"
validates a mechanism that isn't there. A port author told "the guard is a watermark; the whole thing is
one atomic all-or-nothing DEFINER call" who reimplements accordingly — dropping the `FOR UPDATE OF ca
SKIP LOCKED` + `assignment_id` unique index as "internal detail" — **regresses to double-pay**, and the
DoD would pass on a fictional watermark probe. All three of the packet's stated defenses are wrong; the
real defense is unmentioned.

---

## [HIGH] B-FAIL / B-CONSIST · The S5→S8 window shim ("byte-compatible `pgboss.job` row") underestimates pg-boss v10's partition-per-queue schema — the order.created notification the shim exists to save can be silently dropped

**Violated invariant:** proposal §9.2 / Q7b ("Rust-S5 enqueues into the **shared queue contract**
(`pgboss.job`, Node-drained) for the window — a bounded compat shim, **symmetric with S5 keeping
`idempotency_keys` shared**"), threat S8-T9 (the failure it guards: "a Rust-created order's owner
Telegram never fires").

**The break:** pg-boss v10 does **not** store jobs in one flat `pgboss.job` table — each queue is backed
by its **own partition table**, verified in
`packages/db/migrations/1790000000011_pgboss-bootstrap-schema.ts:49-50`: *"pg-boss 10 backs each queue
with its OWN partition table created via `CREATE TABLE pgboss.<hash> (LIKE pgboss.job)`"*, created by a
**runtime `create_queue()`** whose CREATE grant is separately provisioned/revoked
(`1790000000047_pgboss-runtime-create-grant.ts`, `1790000000009_pgboss-revoke-runtime-ddl.ts`). pg-boss's
`send()` is a stored-function insert carrying id / state-enum / `singletonkey` / `keep_until` / `policy`
state-machine columns. So Rust-S5 hand-`INSERT`ing an `order.created` job into the shared queue must (a)
target the correct **existing hashed partition** (not the parent), and (b) replicate the exact v10
state-machine column semantics the Node consumer's claim query expects. The packet's stated analogy —
"symmetric with keeping `idempotency_keys` shared" — is **false**: `idempotency_keys` is a plain,
unpartitioned table; `pgboss.job` is partitioned-per-queue with a stored-procedure write path and a
fragile internal state machine the packet itself calls off-limits to co-drain (§9.3, Q7a).

**Scenario:** during the S5-Rust / S8-Node window, Rust creates an order and writes a job row that lands
in the wrong/absent partition or with a state value the Node claim loop
(`ORDER_TIMEOUT`-style predicate, cf. `order-timeout-sweep.ts:49-51` reading `pgboss.job … state IN
('created','active')`) does not select → the job is **never drained** → **the owner never sees the new
order** (exactly S8-T9). `order.timeout` is sweep-floored so it survives; `order.created` is **not**
floored (packet §9.2) — so this is a silent lost-notification, and it is the single hardest thing in the
whole shim, dispatched in one hand-waving sentence.

---

## [MEDIUM] B-SEC · Consent re-check at dispatch is a plain `SELECT`, not "under `FOR UPDATE`" — a TOCTOU across network I/O pushes an opted-out user

**Violated invariant:** Q3 ("prefs are re-checked at **dispatch** time under `FOR UPDATE`"), threat S8-T6
("Re-check consent at dispatch **under `FOR UPDATE`**").

**The break:** neither dispatch path locks the consent row.
- `handleCustomerStatus` (`index.ts:124-130`) reads `customer_devices … WHERE opted_in = true` with a
  **plain `SELECT`**, then loops sending web-push over the network (`:165-196`). An opt-out committed
  between the SELECT and the `webpush.sendNotification` still delivers — the window **includes network
  round-trips per device**.
- `handleDispatch` (`index.ts:209-215`) reads the owner target + prefs with a plain `SELECT`; the
  `FOR UPDATE` the packet credits lives only on the **write** path (`setCategoryPref`), a different
  transaction that does not block this reader.

The "under `FOR UPDATE`" protection the packet attributes to the dispatch-time re-check does not exist; a
faithful port reproduces the TOCTOU while believing it is closed. Consent/GDPR-adjacent (J4), bounded
window → MEDIUM.

---

## [MEDIUM] B-SEC / B-FAIL · The customer-status order re-fetch seats no tenant GUC — post-B3 (NOBYPASSRLS) it 0-rows and silently drops every customer push

**Violated invariant:** proposal §8 ("Per-tenant re-fetch seats the tenant/user GUC … a context-free
re-fetch matches 0 rows post-B3"), threat S8-T13. The §8 remedy only covers `customer_devices`
(`app.user_id`), not the **orders** read that precedes it.

**The break:** `handleCustomerStatus` runs its FIRST query — `SELECT … FROM orders o JOIN locations l …
WHERE o.id=$1 AND o.location_id=$2` (`index.ts:108-115`) — **before any `set_config`**. `app.user_id` is
seated only afterward at `:122`, and `orders` RLS is keyed on `app.current_tenant`, not `app.user_id`.
Post-B3 the orders read returns **0 rows** → `return` at `:116` → the customer never receives their
CONFIRMED / IN_DELIVERY / DELIVERED push. The packet flags the class (S8-T13) but its §8 "seat the GUC"
prescription is scoped to `customer_devices` and misses this earlier read, so the port would carry a
push pipeline that goes silently dark the moment B3 flips.

---

## [MEDIUM] B-SCALE · Runner pool "mirror pg-boss `max=4`" is undersized for advisory-lock-per-cron at the minute boundary — 6 connections wanted, 4 available

**Violated invariant:** proposal §2 ("the budget is one Rust runner pool (bounded, mirror pg-boss
`max=4`)") + §9.5 ("the overlap does not double the worker connection draw"). The budget accounts for
cross-stack draw but not **intra-fleet cron alignment**.

**Back-of-envelope:** every session-scoped `pg_try_advisory_lock` holds a **dedicated connection for the
whole run**. At `:00` the four minute-crons fire together — `order.timeout_sweep`, `courier.offer_sweep`,
`dwell.monitor`, `liveness.check@60s` — each wanting 1 connection, **+** the dedicated session-mode
PgListener (1, proposal §3.2) **+** the claim/complete loop (1) = **6 concurrent connections vs a `max=4`
pool.** `order-timeout-sweep` holds its lock-5 connection across the detection query +
`app_sweep_timeout_orders()` + `reconcileRefundDue()` + a per-order loop of bus publishes and notify
enqueues (`order-timeout-sweep.ts:38-120`) — hundreds of ms to seconds — so the contention is real, not
instantaneous. Today these crons draw from the **larger operational pool** (`this.pool.connect()`,
`order-timeout-sweep.ts:38`); consolidating 23 crons' advisory-lock connections into a `max=4` runner
pool is a **regression** the "mirror `max=4`" budget does not size for. Self-heals (bounded N), hence
MEDIUM, but the stated budget is wrong.

---

## [LOW] B-OPS · The advisory-lock-collision rationale (Q10) is factually wrong — advisory locks are global by key, separate connections do NOT isolate them

**Violated invariant:** Q10 / `Q-ADVLOCK-COLLISION` ("they currently avoid a collision only because each
takes its own `pool.connect()`").

**The break:** `pg_try_advisory_lock(key)` is **database-global across all sessions** — the key is the
namespace, the connection is not. `order-timeout-sweep.ts:19` (`SWEEP_LOCK_ID = 5`, every 60s) and
`access-request-retention.ts:10` (`RETENTION_LOCK = 5`) genuinely collide: whichever runs second gets
`locked = false` and **silently skips its run** (self-heals next schedule). Separate `pool.connect()`
calls do **not** avoid this. Practical impact is tiny (the sweep holds lock-5 for <<60s, so collision
probability per retention run is sub-percent), hence LOW — but a port whose lock-id registry is designed
on the "separate pools isolate locks" mental model could mis-scope it. The fix direction (a registry) is
right; the stated reason is wrong.

---

## [LOW] B-OPS · "No consumer reads any `.dlq`" (Q8 / Q-DLQ-NOCONSUMER) is imprecise — a nightly failed-job detector already exists

**Context:** `reconciliation.ts:60` runs check `O3 = checkFailedJobs()` nightly, reading pg-boss
`failed`-state jobs and raising DRIFT. It is not a per-`.dlq`-queue consumer, so the literal claim ("no
consumer reads any `.dlq`") is technically true, but the packet presents the observability gap as total
when a weak failed-job alarm is already wired. Minor accuracy note; does not block the (correct) Q8
hardened-baseline direction.

---

### Regression check vs prior S-surface breaker precedent
Consistent with the S5/S6/S7 pattern, the surviving CRIT is an **idempotency/consistency** hole on the
hot path (here: notification dedup that is in-memory, not Postgres). The two money-adjacent crons
(settlement, auto-cancel) were attacked directly and found **safe by their real DB guards** — the finding
is that the packet documents the *wrong* guards (HIGH-3), not that money double-pays.

### Verified SOUND (attacked, held — recorded so the council doesn't re-litigate)
- `auto-cancel` idempotency: `app_sweep_timeout_orders()` `WHERE status='PENDING'` guard holds under
  double-fire (`order-timeout-sweep.ts:62-72`). ✔
- `settlement` concurrency: `FOR UPDATE OF ca SKIP LOCKED` + `settlement_items.assignment_id` UNIQUE make
  a true concurrent double-fire safe **regardless of period boundaries** — no double-pay. ✔ (but see
  HIGH-3: documented as "watermark," which it is not).
- Job **payloads** carry `{event, entity_id, location_id}` only — no PII inlined at the enqueue sites
  (`order-persistence.ts:166-181`, `order-timeout-sweep.ts:111-116`); the claim-check at the payload
  layer is intact. The re-fetched name/address reach only the owner's Telegram (by design), and the
  re-throw path attaches the **claim**, not the fetched order (`index.ts:534`) — DLQ stays PII-free. ✔
