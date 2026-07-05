# S8-JOBS/NOTIFICATIONS Port — Council Packet · PROPOSAL

> **STATUS: 🟡 DRAFT — NOT APPROVED.** Description input to the live Triadic Council
> (system-architect + system-breaker + counsel + human). **No S8 code is ported to Rust until this
> packet is council-APPROVED, every quirk-register row (§11) is dispositioned one by one, and the
> operator signs the 🔴 open questions (`open-questions.md`).** This surface is **less red-line than
> S5-S7** (no order-total composition, no dispatch state machine on the hot path) — but it carries
> **real** ones: the **VAPID private key**, **PII in notification payloads**, and **at-least-once
> money-adjacent jobs** (settlement generation / auto-cancel / refund reconciliation) where a retried
> job that double-pays or double-cancels is a live money defect. Docs only; no product code.

- **Lane:** R3 (complete-rebuild) · **Surface:** S8 jobs/notifications (REBUILD-MAP §3 Phase B, 8th
  strangler — `S5 orders → S6 WS → S7 dispatch → S8 jobs`), 18 route-surface rows + the whole
  background-worker fleet (30 live queues / 23 UTC crons).
- **Date:** 2026-07-04 · **Source commit:** `fix/audit-remediation` (working tree).
- **Census SSOT:** `inventory/10-api-realtime-jobs.md` §3 (jobs/queues/cron — 33 queue-name constants /
  30 live / 23 crons / **1** transactional-enqueue site) + §4.1-2 (Telegram + web-push/VAPID pipeline) +
  §4.3 (email/Resend). Route rows: `rebuild-cutover-harness/route-surface-map.generated.md` (S8 = 18:
  notifications 65-69, owner push 87-89, alerts 99-101, signals-monitoring 50-53, customer push 144-145,
  telegram-webhook 234).
- **Decision inherited (REBUILD-MAP §2 Decision register):** **Job queue = hand-rolled `SELECT … FOR
  UPDATE SKIP LOCKED` + PgListener worker** (apalis-postgres RC-only, sqlxmq/underway stale;
  fallback graphile_worker_rs). The 7 requirements the pick must meet: **txn enqueue · retry/backoff ·
  DLQ · cron · singleton · Supavisor-safe · embedded**. This packet turns that one-line verdict into a
  buildable design (§3) and dispositions every queue/cron against it.
- **Governing ADRs / prior councils (inherited invariants — do not re-litigate):**
  - **telegram-notifications-council-2026-06-22** (per-category prefs + storefront-toggle; the
    consent/quiet-hours model) · **checkout-communication-2026-06-30 / ADR-0016** (the 6-kind messenger
    selector that notification render targets) · **owner-data-export-ai** (ETHICAL-STOP: zero PII to AI —
    reinforces the claim-check posture here).
  - **ADR-audit-fix-money** (`docs/design/audit-fix-money/`) — the **L-D `app_reconcile_refund_due()`**
    reconciler runs on the `order.timeout_sweep` tick (this packet OWNS its scheduling, not its money-math);
    migration **085** settlement-catchup **watermark 2026-07-10 HARD gate** (the settlement CRON is S8-scheduled
    — the watermark timing is surfaced here as well as in S5 Q7).
  - **S5-orders RESOLVE** — the ONE transactional-enqueue site (`order-persistence.ts:158`) lives inside the
    `POST /orders` txn; S5 **produces** to the queue, S8 **owns the runtime**. The producer/consumer
    cutover coupling across the S5→S8 window is §9 Q7.
  - **S6-WS RESOLVE** — jobs are **producers** to the message bus (`order.status`, `ops:*` topics); the
    fan-out transport is S6. S8 ports the `publish` calls through the S6 bus interface, never re-implements
    transport.
- **Parity oracle:** the 174-spec Playwright net (load-bearing S8 specs: `telegram-webhook.spec.ts`,
  `notification-events.spec.ts`, `flow-core-lifecycles.spec.ts` push arm, `dispatch-recovery.test.ts`
  sweep arm, `backup.verify` arm) **plus** the jobs/notifications invariant cluster: the queue-runtime
  unit tests (claim/retry/DLQ/visibility-timeout/transactional-enqueue), the money-idempotency vectors
  (a double-fired settlement/auto-cancel → one effect), and the PII-claim-check assertion (no customer PII
  in a job payload or DLQ row). No behavior change is real without a red→green test (Mandatory Proof Rule).
  Cutover DoD in §12.

---

## 1. Port objective and the load-bearing seam

S5 wrote money + drove the order machine; S6 carried the realtime fan-out; S7 owns dispatch. **S8 is the
first Rust surface that owns the BACKGROUND-WORK RUNTIME itself** — the thing that keeps running when no
request is in flight: the queue that guarantees a scheduled side effect eventually fires, and the crons
that fire settlement generation, auto-cancel, GDPR retention, and backup on a clock. The three
load-bearing seams, each an independent failure mode the port must hold simultaneously:

1. **The at-least-once × idempotency seam.** The hand-rolled `SKIP LOCKED` runner is **at-least-once by
   construction** — completion is a write that happens *after* the side effect, so a worker that does the
   work then dies before marking `completed` re-runs the job. Every handler that sends money or a
   notification **must therefore be idempotent by a Postgres-level guard** (status-CAS / watermark /
   partial-unique / dedup-key), **never** by a queue-level "at-most-once" that does not exist (§3, §6,
   Q1/Q6 🔴). This is the base-knowledge rule "idempotency in Postgres, not Redis," made structural.
2. **The consent/PII seam.** A notification carries the two most abusable things in the system — the
   **VAPID private key** that signs every push (leak = anyone pushes as us) and **customer PII** (phone,
   name, address). The current design is already correct: the job payload carries only
   `{entity_id, location_id}`, the worker **re-fetches under tenant isolation**, renders with `maskPhone`,
   and the customer-status push builds a **minimal no-PII payload** — a textbook **claim-check** (§4, §5,
   Q3/Q5 🔴). The port must carry this *visibly*, and additionally keep the DLQ payload PII-free (it holds
   the claim, never the re-fetched contact).
3. **The single-flight seam (cross-machine AND cross-stack).** 23 crons run every minute-to-nightly; a
   settlement cron that double-fires **double-pays**, an auto-cancel that double-fires is idempotent-by-guard
   but a naive port could lose that. Today each cron takes a `pg_try_advisory_lock(id)` so two `web`
   instances never double-run. During the S8 overlap the **novel** hazard is two *stacks* (Node + Rust)
   each running the fleet. Advisory locks are database-global so they *would* hold cross-stack **iff the
   ids match** — but the safe posture is **exactly one stack runs the background fleet at any instant**
   (the fleet flips as ONE unit, not route-by-route) (§9, Q7 🔴).

**The sharpest cutover fact (see §9, Q7 🔴):** the background fleet is **not** strangled route-by-route.
Because crons act on **shared business tables** (double-firing = double settlement / double retention-erase)
and the ONE transactional-enqueue producer (`POST /orders` → `order.timeout` + `notify.telegram.send`)
lives in S5 which cuts over **before** S8, the packet must resolve *which stack owns the queue table and
the crons during the S5-Rust / S8-Node window*, and enqueue into a **shared queue contract** so a
Rust-created order's `order.created` Telegram notification is not silently dropped.

## 2. Scope — what is S8, what is explicitly NOT

**In this packet (S8):**
1. **The hand-rolled queue RUNTIME** — the `SKIP LOCKED` claim loop, PgListener wake-on-enqueue + poll
   floor, tokio cron loops, `pg_try_advisory_lock` single-flight, retry/backoff, DLQ, visibility timeout,
   and the **transactional-enqueue API** (`enqueue(&mut tx, …)`) the one producer needs (§3).
2. **The 30 live queues + 23 crons** as job modules (port each; RETIRE the 3 dead queue constants + the
   dead scaffold — §11 Q-DEAD-QUEUES). Money/state-machine handlers **call** the existing DB functions
   (`app_sweep_timeout_orders`, `app_reconcile_refund_due`, `app_generate_settlements`) — KEEP, per
   REBUILD-MAP §8; S8 owns the **scheduling + single-flight + at-least-once idempotency plumbing**, not the
   money-math.
3. **The notification pipeline** (`event-registry` 21 events → quiet-hours → prefs/consent → dispatch →
   render → audit): **Telegram** (webhook + send), **web-push** (VAPID + customer/owner subscriptions),
   **customer-status push** (no-PII), and **email** (Resend, ops-alert-only) (§4).
4. **The VAPID config + push send pipeline + subscription store** (`customer_devices`,
   `owner_notification_targets`), the public-key route, and the 410/404 prune (§4, Q3 🔴).
5. **The Telegram webhook** (route 234, `POST /webhook/telegram/:secret`) with **fail-closed** signature
   verification (a FIX-IN-PORT of a live fail-open gap — §4, Q4 🔴).
6. **The S8 owner/customer routes** (route-surface rows): notifications/targets (65-69), owner push
   (87-89), alerts (99-101), signals-monitoring reads (50-53), customer push (144-145).
7. **The producer side of the S6 bus** — jobs `publish` order.status / `ops:order_timeout_lag` /
   `ops.reconciliation_drift` through the ported S6 bus interface (transport is S6).

**NOT S8 (explicit boundary — each a separate slice):**
- **The money-math itself** — `app_generate_settlements()` composition, the settlement ledger, migration
  085's watermark rewrite (S5/S7 money council). S8 **schedules** the settlement cron and guarantees it is
  single-flight + at-least-once-idempotent; it does not author or review the DEFINER money SQL.
- **The courier dispatch ENGINE** (`courier.dispatch` handler logic, offer-handshake, `releaseBindingAndReoffer`)
  — S7. S8 owns the cron plumbing (`courier.offer_sweep`) that *pumps* it and the single-flight lock; the
  dispatch state transitions are S7.
- **The order state machine + `updateOrderStatus` folds** — S5. The `order.timeout`/`order.timeout_sweep`
  auto-cancel **calls** `app_sweep_timeout_orders()` (whose L-A/086 refund floor S5 owns); S8 owns the
  timing + single-flight + the fact that the sweep is the **cross-tenant safety-net floor**.
- **The GDPR erasure LOGIC** (`gdpr_erase_customer` DEFINER draft, `anonymizeOrder`) — S9. S8 owns the
  `anonymizer.gdpr` / `*.retention-sweep` cron plumbing + the batch-of-10 `FOR UPDATE SKIP LOCKED` loop,
  never the erasure semantics.
- **The backup PIPELINE** (pg_dump/encrypt/upload/restore-drill) — backup/DR council + the **ops-binary
  sidecar** (REBUILD-MAP §8). S8 owns the backup **cron triggers only**; the admin trigger endpoints
  (`/api/admin/backups*`) are **S10**; the actual dump/upload lives in a `tokio::process` sidecar, never
  the request/worker hot path.
- **The Plisio payment webhook** (`payments-webhook.ts`, HMAC-SHA1 over PHP-serialize) — a **money-in front
  door** with its own HMAC/DEFINER threat model, ported dark (flags off); its own webhook slice. It shares
  only the "unauthenticated webhook → fail-closed signature" **threat class** with Telegram (§4); its
  money-settle logic is S5-money-council.
- **No schema change** — the DB is frozen. The hand-rolled queue introduces a **new `jobs` table** owned by
  Rust (not a schema change to any business table); whether that table is a fresh migration or reuses
  `pgboss.job`'s shape is a Q7 cutover decision, resolved as a **forward-only additive migration** the
  operator places verbatim, staging-first — never an edit to a business table.

**Back-of-envelope (why boring wins, and where the real ceiling is).**
- **Scale:** target **N ≈ 10-50 locations**, low-hundreds orders/day. **Job volume:** ~2-4 jobs/order
  (order.timeout, notify.telegram.send order.created, notify.dispatch, notify.customer_status) →
  **~1-2k jobs/day**, bursty at lunch/dinner. **Cron ticks:** 4 minute-crons (order.timeout_sweep,
  courier.offer_sweep, dwell.monitor, liveness.check@60s) ≈ **~5.7k ticks/day**, each a bounded
  single-DB-transaction sweep. Notification sends: a few hundred/day; web-push fan-out ≤3 devices/customer,
  a handful of owner targets/location.
- **This surface is NOT throughput-bound.** A `SKIP LOCKED` claim against a table of a few thousand rows/day
  is trivial; the whole fleet is idle most of the time. The engineering risk is entirely **correctness**
  (at-least-once idempotency, consent, single-flight) and **connection topology**, not QPS.
- **The real ceiling is the connection topology, not job QPS.** pg-boss today uses a **session-mode
  connection on port 5432** (the transaction pooler on 6543 **blocks** both `LISTEN/NOTIFY` and DDL —
  `queue-provider.ts:93-96`). The Rust runner inherits this hard constraint: **≥1 dedicated session-mode
  connection for PgListener** + the advisory-lock connections (each cron's `pg_try_advisory_lock` is
  session-scoped) + the runner's own claim/complete pool. During the S8 overlap the fleet is a **single
  stack** (Q7) — so unlike S5's request surface, the worker fleet does **not** double its connection draw;
  the budget is one Rust runner pool (bounded, mirror pg-boss `max=4`) sitting under the Supavisor ceiling
  alongside the API pool (REBUILD-MAP §Decision register, Supavisor Phase-A answer).
- **Conclusion:** boring wins — a hand-rolled `SKIP LOCKED` table + tokio cron loops + `pg_try_advisory_lock`
  (all Postgres-native, no new runtime, no Redis) is the simplest thing that holds. **21 of 30 queues are
  pure cron sweeps that can shed the queue abstraction entirely** (tokio loop + advisory lock, no job row);
  only ~9 genuinely need the durable queue table (§3.4). This is "schema-rich, runtime-minimal": build the
  queue table + listener for the ~9 that need durable cross-process handoff; do NOT round-trip the 21
  cron-only workers through a job table they never needed.

---

## 3. Concern 1 — The hand-rolled SKIP LOCKED job runner (Q1 🔴, load-bearing)

**The pick (REBUILD-MAP §2): `SELECT … FOR UPDATE SKIP LOCKED` + PgListener.** This section makes it
buildable. The DB is frozen for business tables; the runner introduces a **new `jobs` table** (additive
forward-only migration, operator-placed).

### 3.1 The claim loop (the core)
```
-- one atomic claim: FOR UPDATE SKIP LOCKED gives lock-free multi-consumer fan-out
UPDATE jobs
  SET state = 'active',
      locked_until = now() + make_interval(secs => vt_seconds),
      attempts = attempts + 1,
      started_at = now()
WHERE id IN (
  SELECT id FROM jobs
   WHERE (state = 'queued'  AND run_after   <= now())      -- ready work
      OR (state = 'active'  AND locked_until <  now())      -- reclaim a dead worker's job (visibility timeout)
   ORDER BY priority DESC, run_after
   FOR UPDATE SKIP LOCKED
   LIMIT batch_size
)
RETURNING *;
```
- **At-least-once by construction** — completion (`UPDATE … SET state='completed'`) is a **separate write
  after** the handler's side effect. A crash in between re-claims the job (its `locked_until` lapses).
  There is **no at-most-once**; §6/§5 make every money/PII handler idempotent to absorb this.
- **Visibility timeout** — `locked_until = now() + vt`. The claim predicate itself re-claims any `active`
  job past `locked_until` (a worker died mid-job), so no separate reaper thread is required. `vt` must be
  **> the handler's max runtime** (external calls are timeout-bounded — §4/threat S8-T11 — so `vt` is
  bounded and knowable). Too short → double-run (idempotency covers it); too long → a dead worker's job
  waits `vt` (the liveness-checker + `ops:*_lag` detection surface it — §7).

### 3.2 Wake + poll (PgListener, session-mode)
- **Wake-on-enqueue:** `NOTIFY jobs_new` on insert; the runner holds a dedicated **session-mode
  (port 5432)** `PgListener` and wakes immediately — carries pg-boss's exact topology constraint
  (Q-SESSION-MODE). This is the "embedded" + "Supavisor-safe" requirement: the listener connection must be
  direct-session, never the transaction pooler.
- **Poll floor:** a `tokio::time::interval` (e.g. 1-5s) claims even if a `NOTIFY` was missed (a pooler
  hiccup, a delayed job whose `run_after` just matured). Correctness never depends on the notification
  arriving — the poll is the floor, the LISTEN is the latency optimization.

### 3.3 Retry / backoff / DLQ (ship the hardened baseline, close the 24/30 gap — Q8)
On handler error:
```
UPDATE jobs SET
  state     = CASE WHEN attempts >= max_attempts THEN 'failed' ELSE 'queued' END,
  run_after = now() + backoff(attempts),            -- exponential + jitter, capped
  last_error = $err_redacted                        -- NO PII (§5); redact before persist
WHERE id = $1;
-- state='failed' → move (or logically route) to the DLQ; NOT silently accumulated (Q8, threat S8-T12)
```
- **The census's explicit finding:** only **6/30** queues (the backup family) run with backoff+DLQ; the
  other **24** run bare pg-boss v10 defaults (retryLimit=2, **0s** backoff, no DLQ) — a transient failure
  is hammered twice in milliseconds then lands in `failed` with **no salvage path**. **FIX-IN-PORT:** the
  Rust runner ships **backoff + DLQ as the baseline for every queue**, not an opt-in 6/30 get (Q8). This is
  strictly-more-reliable; document it as a deliberate behavior improvement, not a silent parity break.
- **DLQ must be consumed, not a silent sink.** Today no consumer reads any `.dlq` (census §5). **FIX-IN-PORT:**
  a periodic ops-alert on dead-job count to the ops bus (the same channel liveness/reconciliation use) —
  a dead job is a paged signal, not landfill (threat S8-T12).
- **Poison-job quarantine** — the `max_attempts → DLQ` path is the poison-job fuse (a job that always
  throws stops re-hammering the runner after N attempts). Per-queue fairness via `ORDER BY priority,
  run_after` + a per-queue claim quota prevents one hot queue starving others (threat S8-T7).

### 3.4 Idempotency (the load-bearing rule)
Every enqueue **may** carry an `idempotency_key` (unique partial index) so a duplicate producer-enqueue is
a no-op insert. But the primary guard is **at the effect**, in Postgres, per the base knowledge:
- **`order.timeout` / auto-cancel:** idempotent by the `WHERE status='PENDING'` guard inside
  `app_sweep_timeout_orders()` — a re-run cancels nothing already cancelled (census §5, source verified
  `order-timeout-sweep.ts:62-72`). Carry the guard; the queue does **not** need at-most-once.
- **`notify.telegram.send`:** dedup via a **dedup key** (`order.created:<id>:<loc>`) + archive to
  `notification_outbox_audit` on exhaustion (source verified). Carry the dedup-key; the notification is
  at-least-once with a dedup floor.
- **`settlement.generate` / refund_due:** idempotent by DB-level watermark / the **N5 partial unique
  `(payment_id) WHERE type='refund_due'`** (mig 086). §6.
- **The `access-request.notify` model is the gold standard** (census §1 row 27): a **claim-before-send CAS**
  on the `access_requests` row — idempotency lives in the DB row, not the queue. **Recommend this pattern
  for any new idempotent producer** over reusing a global singletonKey.

### 3.5 Transactional enqueue — the ONE hard requirement (Q-TXN-ENQUEUE)
Exactly **one** site needs same-transaction enqueue: `order-persistence.ts:158-173` inserts `order.timeout`
+ `notify.telegram.send` into the queue **inside the `POST /orders` txn** (a ROLLBACK of the order also
rolls back the enqueue — genuine atomicity, no outbox poller). The Rust runner must expose
`enqueue(&mut PgTransaction, name, payload, opts)` — sqlx's `Executor` over `&mut Transaction` makes this
native (the job INSERT runs on the caller's tx). **The other ~29 queues are fire-and-forget-after-commit,
a DB-journal-pumped-by-cron (`courier_dispatch_queue`), or in-handler self-requeue** — none needs
transactional enqueue. This one site is S5-owned (producer); the cross-stack window is §9/Q7.

### 3.6 Cron-only workers shed the queue (the runtime-minimal cut)
**21 of 30 queues** (census §6: #5,7,8,9,10,11,13,17,18,19-24,25,26,28,29,30,31) are pure
"run-on-a-schedule, single-flight-across-instances" sweeps. In Rust they are a **`tokio` cron loop +
`pg_try_advisory_lock(id)` + one `sqlx` transaction** — **no job row, no queue table**. Only the ~9
genuinely event-driven / durable-cross-process queues (§3.7) live in the `jobs` table. This is the
materially smaller surface the from-scratch rebuild earns over "port pg-boss 1:1."

### 3.7 The ~9 that keep the queue table
`order.timeout` (delayed per-order + the one txn-enqueue), `notify.dispatch`, `notify.customer_status`,
`notify.telegram.send` (multi-producer event fan-out, real per-job retry), `courier.dispatch` (pumped by
the sweep, singleton per order), `anonymizer.gdpr` (on-demand, owner-triggered, durable). `velocity.flush`
is a candidate to fold into an **in-process tokio `mpsc`/debounce** and shed Postgres entirely — a product
decision (lose the 5s buffer on a crash?) — Q9.

## 4. Concern 2 — Notification dispatch: VAPID, Telegram, email (Q3 🔴 / Q4 🔴)

### 4.1 Web-push + VAPID private key (Q3 🔴)
**The pipeline (census §4.2, source-verified):** event → `event-registry` (21 types → quiet-hours policy,
render group, target scope) → `handleDispatch` (re-fetch target, verify `status='active'`, check prefs,
evaluate quiet-hours, re-fetch order **under tenant isolation**) → render → adapter → retry → audit on
every branch.
- **VAPID key handling (RED LINE, Q3).** `VAPID_PUBLIC_KEY` / `VAPID_PRIVATE_KEY` (required) +
  `VAPID_SUBJECT` (default `push@deliveryos.app`) live in the ONE Rust config struct
  (`packages/config` → `AppConfig`), preflight-validated; the adapter registers **only if both keys are
  set** (soft-disable otherwise — carry). **The private key is NEVER logged, NEVER in an error payload,
  NEVER in a DLQ row, NEVER in git** (threat S8-T2). The public-key route (`GET /api/push/vapid-public-key`,
  404 if unconfigured) serves the **public** key only. Rust: `web-push` crate `VapidSignatureBuilder`;
  the key is loaded once into an `Arc`, and a guardrail asserts it is absent from every `tracing` field
  and every serialized error.
- **Consent is authoritative (Q3, threat S8-T6).** A customer who opted out (`customer_devices.opted_in =
  false`) or an owner target whose category pref is off **must not be pushed**. Prefs are re-checked **at
  dispatch time** (not enqueue time): `transactional` = default-on, never suppressed; `operational`
  (`shift.*`) and `quality` (`rating.low_received`) are category-gated. `setCategoryPref` does an atomic
  `jsonb_set` under `FOR UPDATE` + a same-transaction audit into `notification_prefs_audit` — **carry this
  atomic-update-plus-audit verbatim** (consent changes are GDPR-auditable). Quiet-hours per target evaluated
  against `locations.timezone` (the ONLY tz-aware evaluation in the whole jobs surface — everything else is
  UTC) via `chrono-tz`.
- **Subscription store.** Customer subs in `customer_devices` (`opted_in`, `push_subscription`,
  `vapid_endpoint`, `keys_p256dh`, `keys_auth`; deduped by **SHA-256 endpoint fingerprint**); owner subs in
  `owner_notification_targets` (`channel='push'`, address = JSON subscription). The external endpoint is
  **not a fixed URL** — `web-push` POSTs to each subscription's own browser-vendor push service (FCM /
  Mozilla autopush) with VAPID JWT auth. **Prune on 410/404** (stale subscription → clear the row) — carry.

### 4.2 Telegram (Q4 🔴 — fail-closed webhook)
- **Webhook signature — FIX-IN-PORT a live fail-open (threat S8-T5, census §4.1 finding + §route 234).**
  `telegram-webhook.ts:87-100` (source-verified): when `TELEGRAM_BOT_SECRET` is set but the
  `x-telegram-bot-api-secret-token` header is **absent**, the handler logs a warning and **processes the
  request anyway** ("backward compat"); it 401s **only** when the header is present but wrong. The E2E
  `telegram-webhook.spec.ts:53-59` ("missing secret returns 401") asserts the opposite — a live fail-open.
  **The Rust port makes verification unconditionally fail-closed:** if `TELEGRAM_BOT_SECRET` is configured,
  a missing OR mismatched header → 401, **constant-time compare** (`subtle::ConstantTimeEq`). This is a
  🔴 security FIX-IN-PORT with a documented E2E delta (missing-header → 401, matching the existing test).
- **The URL-path secret is a router token, not the auth (Q-TG-URL-SECRET).** The route is
  `POST /webhook/telegram/:secret` — the path secret is how Telegram addresses us and is **referer/log-leakable**;
  the **header `secret_token` is the real gate**. Carry both, but the fail-closed decision keys on the
  **header** constant-time compare, never the URL alone.
- **Webhook always returns 200 to Telegram** even on internal failure (best-effort, off critical path) —
  carry (a 500 makes Telegram retry-storm). Business failures are absorbed, not surfaced as retries.
- **Send path (carry verbatim):** raw `reqwest` (no SDK), per-chat **rate-limit (1/1.2s) + circuit breaker
  (5 fail → 60s cooldown) + dedup cache**; 401/403 → permanent target `status='disabled'`; 429 honors
  `retry-after`; network error caught. Every external call is **timeout-bounded** (threat S8-T11; the TMA
  `setChatMenuButton` 5s `Promise.race` is the precedent — no external call may pin a held DB connection).

### 4.3 Email (Resend) — carry the separation
`EmailAdapter` is a **direct ops-alert path**, deliberately **outside** the tenant dispatcher (no
locationId, no prefs/quiet-hours/audit) — only `access-request-notify` uses it, raw `reqwest` to
`POST https://api.resend.com/emails`, `Bearer RESEND_API_KEY`, **5s abort**, soft-disable when the key is
absent (Q-EMAIL-DIRECT). Carry the separation verbatim; `RESEND_API_KEY` is a 🔴 secret (config-only, never
logged), same posture as VAPID.

## 5. Concern 3 — PII in jobs & notifications (Q5 🔴, the claim-check)

**The current design is already a correct claim-check — the port must carry it visibly and extend it to the
DLQ.**
- **Job payloads carry a claim, not the contents.** `notify.dispatch` / `notify.telegram.send` /
  `notify.customer_status` payloads carry `{entity_id (order id), location_id, event}` — **no customer
  phone/name/address** (source-verified: `handleCustomerStatus` "Build minimal payload (no PII)" +
  `handleDispatch` re-fetch "under tenant isolation + 0 PII payload check"). The worker **re-fetches** the
  order/customer under the tenant's isolation at dispatch time (§4.1). Carry: the Rust job `payload` type is
  `{entity_id, location_id, event}` only; a guardrail asserts no PII field is representable in a job payload.
- **Render masks before egress.** The Telegram render path masks the customer phone (`maskPhone`,
  `pii-mask.ts`) before building the message; the customer-status **push body is money-only + a short order
  id** (`Order #ABCD Delivered`, formatted total — no name/phone/address). Carry both.
- **The DLQ is PII-free by the same claim-check (threat S8-T4/T12).** Because the payload is only the claim,
  a job that dies into the DLQ carries `{entity_id, location_id}` — **not** the re-fetched contact. The Rust
  DLQ must persist the **redacted** `last_error` (§3.3) and the claim payload only; a guardrail asserts no
  DLQ row contains a phone/name/address pattern.
- **Logs are PII-free.** `last_error`, `tracing` fields, and the `ops:*` bus payloads carry ids and counts,
  not contact data (the reconciliation-drift alert carries `order_id`/`payment_id`, `substring(…,1000)`).
  Carry; the Rust `tracing` layer must not `Debug`-print a re-fetched order row.

## 6. Concern 4 — Money-adjacent jobs: at-least-once × idempotency (Q6 🔴)

**Every money-adjacent job's effect is idempotent by a Postgres-level guard — never by the queue. The rule:
at-least-once queue + idempotent-by-DB-constraint handler = exactly-once effect.**

| Job (cron) | Effect | Idempotency guard (Postgres) | S8 owns / KEEP |
|---|---|---|---|
| `settlement.generate` (`0 2 * * *`) | courier payout generation | `app_generate_settlements()` DEFINER — watermark-based; the whole generation is **one atomic DB call**, a thrown error aborts the entire sweep (all-or-nothing) | S8 owns the **cron + single-flight**; money-math KEEP (S5/S7); **mig 085 watermark 2026-07-10** timing landmine surfaced (Q7c) |
| `order.timeout` + `order.timeout_sweep` (`* * * * *`) | PENDING→CANCELLED past `timeout_at` | `WHERE status='PENDING'` guard inside `app_sweep_timeout_orders()` — a re-run cancels nothing already cancelled (idempotent by guard) | S8 owns the **timing + the cross-tenant safety-net floor**; the sweep recovers ANY overdue order regardless of the per-order job (stack-agnostic) |
| refund_due reconciler (`app_reconcile_refund_due()`, on the sweep tick) | records a missed `refund_due` obligation | **N5 partial unique `(payment_id) WHERE type='refund_due'`** (mig 086) — a re-run cannot double-insert | S8 owns the **tick that runs it**; the money-fold + trigger are S5/086 |
| `reconciliation.nightly` (`0 3 * * *`) | 12 read-only drift checks | read-only → idempotent trivially | S8 owns the cron |

- **The double-fire hazard is entirely about SCHEDULING (threat S8-T1/T3), which is S8's job.** A settlement
  cron that fires twice — because of a queue retry OR a cron double-fire across two web instances OR across
  Node+Rust during overlap — must not double-pay. The **idempotency lives in the DB fn** (watermark), so a
  retry either fully re-applies (no-op past the watermark) or fully rolls back. **S8's obligation:** the
  settlement cron is **single-flight** (`pg_try_advisory_lock`, and exactly-one-stack during overlap — Q7),
  and the runner's at-least-once retry never partially-applies (the effect is one atomic DEFINER call). The
  packet does **not** trust the queue to be at-most-once — it trusts the DB guard.
- **The 085 watermark (`2026-07-10 00:00:00+00`) is a shared timing landmine (Q7c).** The settlement cron is
  S8-scheduled; erring EARLY (literal before the real apply) **double-pays** old rows, erring LATE is safe.
  If the rebuild schedule slips the settlement apply past 2026-07-10 the operator must bump all three literal
  occurrences before apply. Surfaced here (as in S5 Q7) so the rebuild schedule cannot silently trip it.

## 7. Concern 5 — Cron scheduling + worker liveness (Q2)

- **Cron mechanism (Q2): tokio loop + `pg_try_advisory_lock(id)` — boring, proven, Postgres-native.** No
  leader-election framework, no external scheduler. Each cron is a `tokio::time::interval` (or
  `tokio_cron_scheduler` for cron-expr parity — 23 crons, all **UTC**, 6-field where needed for the
  seconds-granularity `liveness.check`); it takes a **database-global** advisory lock so two `web` instances
  never double-run (carry the existing pattern verbatim). This is the same single-flight pg-boss provides via
  singletonKey, but native and without the `policy:'short'` footgun (Q-SINGLETONKEY-POLICY).
- **Advisory-lock id collision — FIX-IN-PORT (Q10, census §4).** `order-timeout-sweep` and
  `access-request-retention` **both use id=5**. They currently avoid a collision only because each takes its
  own `pool.connect()`, but this is a latent bug. The Rust port introduces a **lock-id registry** (one
  source of truth, unique ids, named constants) — never raw small ints reused across unrelated workers.
- **Worker liveness — carry the two-tier watcher, unify the roster (Q-WORKER-ROSTER-DUP, threat S8-T10).**
  The heartbeat proves the **VM breathes**, not that the **queue drains** — the `order.timeout_sweep`'s
  DETECTION query (overdue-but-undrained `order.timeout` count → `ops:order_timeout_lag`) is the real
  drain-signal; carry it. The `liveness.check` "watcher of the watcher" reads `ops_worker_heartbeat` and
  pages on a critical worker going stale; carry it. **But the two hardcoded rosters** (`CRITICAL_WORKERS`
  5-id real-time subset vs `EXPECTED_WORKERS` 8-id nightly-completeness set — confirmed intentional two-tier,
  not drift) must become **one roster with a `critical: bool` flag**, not two parallel arrays.
- **Boot-assert the schedules (Q-BOOT-ASSERT).** `assertAccessRequestSchedules` / `assertDeliveryTraceSchedule`
  fail-fast (`process.exit(1)` in production) if the expected schedule rows are missing — a **visible red
  deploy instead of a silent zombie cron**. **Carry AND extend this to every cron** (a Rust preflight that
  asserts the full cron roster is scheduled before `listen`), closing the "only 2 of 23 crons are boot-asserted"
  gap.

## 8. Tenancy — how a background job writes under FORCE RLS (Q6/Q5)

Background jobs have **no request principal** — so how do they satisfy FORCE RLS post-B3? Three carried
patterns, spelled out because the port must not "fix" them into a broken flow:
- **SECURITY DEFINER functions for cross-tenant sweeps (KEEP).** `app_sweep_timeout_orders()` cancels
  overdue orders **across all tenants in one pass** with **no GUC** — it runs as a DEFINER fn (mirrors the
  `WHERE status='PENDING'` guard + folds the `order_status_history` audit atomically), which is exactly why
  it is a structural safety boundary (REBUILD-MAP §8 stays-in-Postgres register class). The Rust worker
  **CALLS** it; disposition = KEEP. Same for `app_reconcile_refund_due()` and `app_generate_settlements()`.
- **Per-tenant re-fetch seats the tenant/user GUC.** `handleCustomerStatus` seats
  `set_config('app.user_id', customer_id, true)` before reading `customer_devices` (source-verified) —
  the notification worker re-fetches under the tenant/customer isolation the same way the request path does.
  Carry the seat; a context-free re-fetch matches 0 rows post-B3.
- **The mig-086 L-C trigger's per-row GUC save/restore dance is the template** for any DEFINER surface that
  writes `payment_events` under FORCE RLS — the refund reconciler inherits it. Land 086 before the flip
  (S5 Q7) so the floor is stack-agnostic.

## 9. Cutover concurrency — the background fleet flips as ONE unit (Q7 🔴)

**Unlike S5's request surface, the background fleet is NOT strangled route-by-route.** The failure classes
and controls:

1. **Cron double-fire across stacks — the primary money hazard (threat S8-T3).** If both Node and Rust run
   the settlement cron, `app_generate_settlements()` fires twice. Advisory locks are **database-global** so a
   shared `pg_try_advisory_lock(id)` *would* hold cross-stack **iff the id namespaces match exactly** — but
   depending on id-matching across two codebases is fragile. **Control: exactly ONE stack owns the entire
   background fleet at any instant.** The fleet (all crons + all queue consumers) flips **atomically** — a
   single ops flag / process-group swap — never a per-cron or per-queue strangle. This is the S8 analogue of
   S5's "atomic per-surface flip."
2. **The transactional-enqueue producer/consumer coupling (threat S8-T9, the sharpest S8 cutover fact).**
   S5 (order create) cuts over **before** S8. During the **S5-Rust / S8-Node window**, a Rust-created order
   must still enqueue `order.timeout` + `notify.telegram.send` **into the live (Node-drained) queue**, or:
   - `order.timeout` is **sweep-floored** (the Node `order.timeout_sweep` recovers any overdue order from the
     `orders` table directly, stack-agnostic) — so the per-order job can be **skipped** during the window with
     no correctness loss, only ≤1-min timing precision.
   - `notify.telegram.send` `order.created` is **NOT sweep-floored** — if the Rust create doesn't enqueue it,
     the owner **never sees the new order**. **Control:** Rust-S5 enqueues into the **shared queue contract**
     during the window (the existing `pgboss.job` table, drained by the still-Node fleet — symmetric with how
     S5 kept `idempotency_keys` a shared table), a **bounded compatibility shim** for the S5→S8 window; OR the
     `order.created` notification rides the S6 bus. **Recommend: Rust-S5 writes the shared-queue job row for
     the window; S8's atomic flip switches producers + consumers to the new `jobs` table together.** This is
     a coupling the operator + S5 lead must sign (Q7b).
3. **The new `jobs` table vs `pgboss.job` (Q7a).** The hand-rolled queue owns a **new `jobs` table** (clean
   schema, forward-only additive migration). Reusing `pgboss.job`'s schema to co-drain from both stacks is
   **rejected** — pg-boss v10's internal state machine (created/active/retry/completed, archive, singleton
   columns) is fragile to co-drain safely. The clean posture: Node drains `pgboss.job`, Rust drains `jobs`,
   and **exactly one runs at a time** (control 1). The one exception is the §9.2 window shim (Rust *produces*
   into `pgboss.job` while Node still *consumes* it).
4. **Rollback = flip the fleet back to Node.** Because both stacks' crons act on the **same** business tables
   through the **same** DB guards (status-CAS, watermark, partial-unique), a fleet flipped back to Node leaves
   every committed effect valid — provided the money-idempotency guards (§6) are green and the fleet never
   dual-ran. The rollback is an ops flag, not a data migration.
5. **Connection budget (§2 back-of-envelope).** Because the fleet is single-stack (control 1), the overlap
   does **not** double the worker connection draw. The Rust runner pool (bounded, session-mode listener +
   advisory-lock connections + claim pool) sits under the Supavisor ceiling; monitor combined operational +
   runner utilization; the flip sheds the Node fleet's connections.

**Cutover DoD gates specific to S8 (in addition to §12):** the fleet-atomic-flip proof (no cron/consumer runs
on both stacks — a probe asserts single ownership) · the settlement/auto-cancel **double-fire → one effect**
idempotency probe · the S5→S8 window shim proven (a Rust-created order's `order.created` Telegram notification
fires via the shared queue) · migration 086 landed (shared refund floor, from S5) · the new-`jobs`-table
migration landed staging-first.

## 10. Migration / integration-draft interaction (Q7c)

- **086 (refund_due trigger)** — land BEFORE the flip (S5 dependency, shared floor). S8's refund reconciler
  relies on its **N5 partial unique**. Inert until crypto flips. **Do NOT "fix" its non-throwing per-row
  swallow to a throwing template** — a throwing trigger would wedge the fleet-wide sweep.
- **085 (settlements-catchup, watermark `2026-07-10`)** — the settlement cron is S8-scheduled; the watermark
  is an **operator timing gate** (§6, Q7c). S8 does not author/apply it.
- **087 (reconciler)** — the L-D reconciler runs on the S8 sweep tick; land with S8. Worker-path, not
  request-path.
- **The new `jobs` table migration** — a `packages/db/migrations/` red-line, **forward-only, additive**,
  operator-placed-verbatim, staging-first. It touches **no business table** (a new table + its indexes only).
- All are staging-first, forward-only; **S8 does not author or apply any migration** — it consumes 086's floor
  and provides the schema for its own `jobs` table for the operator to place.

## 11. Quirk register — carry-vs-fix (default = CARRY-VERBATIM)

**FIX-IN-PORT only for a 🔴 security/correctness issue or a build-correctness bug, each with an explicit
test/E2E delta.** Everything else CARRIES.

| ID | Quirk (source) | Disposition |
|---|---|---|
| Q-SKIP-LOCKED | pg-boss → hand-rolled `SELECT … FOR UPDATE SKIP LOCKED` runner (REBUILD-MAP §2) | **PORT** — new `jobs` table; at-least-once + visibility timeout + LISTEN/poll (§3); 🔴 Q1 |
| Q-TXN-ENQUEUE | ONE transactional enqueue (`order-persistence.ts:158`, order.timeout + notify.telegram.send in the POST /orders txn) | **PORT** `enqueue(&mut tx, …)` (sqlx native); S5-produced; window shim §9.2 (🔴 Q7) |
| Q-SESSION-MODE | pg-boss uses port 5432 session-mode (pooler 6543 blocks LISTEN/NOTIFY + DDL) (`queue-provider.ts:93`) | **CARRY the topology constraint** — the Rust listener + advisory-lock conns must be session-mode; Supavisor-safe requirement |
| Q-BARE-DEFAULTS | 24/30 queues run bare v10 (retryLimit=2, 0 backoff, no DLQ); only backup family hardened (census §5) | **FIX-IN-PORT (Q8)** — backoff+DLQ as the **baseline for every queue**; strictly-more-reliable; documented behavior improvement |
| Q-DLQ-NOCONSUMER | 6 DLQs exist, **no consumer reads any `.dlq`** (census §5) | **FIX-IN-PORT (Q8)** — DLQ default-on + periodic ops-alert on dead-job count; not a silent sink (threat S8-T12) |
| Q-SINGLETONKEY-POLICY | v10 honors singletonKey dedup only when queue policy='short'; bare='standard' silently no-ops it (`queue-provider.ts:24-27`) | **REMOVED by construction** — the hand-rolled queue makes dedup an explicit unique `idempotency_key`; no silent-no-op footgun |
| Q-ADVLOCK-COLLISION | id=5 shared by order-timeout-sweep + access-request-retention (census §4) | **FIX-IN-PORT (Q10)** — a lock-id registry (unique named constants); never raw reused small ints |
| Q-CLAIM-CHECK | notify payloads = `{entity_id, location_id}`, re-fetch under tenant isolation, maskPhone render, no-PII customer push (source-verified) | **CARRY verbatim + EXTEND to DLQ** — the load-bearing PII pattern (§5); DLQ holds the claim only (🔴 Q5) |
| Q-VAPID-KEY | VAPID private key config-only, adapter registers only if both keys set (`bootstrap/notifications.ts:52`) | **CARRY + HARDEN** — never logged/error-payload/DLQ/git; guardrail asserts absence (🔴 Q3) |
| Q-CONSENT-ATOMIC | `setCategoryPref` atomic `jsonb_set` under FOR UPDATE + same-txn audit into `notification_prefs_audit` | **CARRY verbatim** — consent is GDPR-auditable; opted-out never pushed (🔴 Q3, threat S8-T6) |
| Q-PUSH-PRUNE | prune subscription on 410/404 (`workers/index.ts:183`) | **CARRY verbatim** |
| Q-TG-FAILOPEN | telegram webhook processes on **missing** secret header ("backward compat"), 401 only on wrong header (`telegram-webhook.ts:96-99`) | **FIX-IN-PORT (🔴 Q4)** — fail-closed unconditionally, constant-time compare; E2E missing-header→401 (matches the existing stale test) |
| Q-TG-URL-SECRET | secret is in the URL path (`/webhook/telegram/:secret`) — referer/log-leakable | **CARRY (Telegram's router model) but gate on the HEADER** constant-time compare, not the URL alone |
| Q-TG-200-ALWAYS | webhook returns 200 to Telegram even on internal failure (best-effort) | **CARRY verbatim** — a 500 triggers a Telegram retry-storm; business failures absorbed |
| Q-TG-CIRCUIT | per-chat rate-limit (1/1.2s) + circuit breaker (5→60s) + dedup cache; 401/403→disable target, 429→retry-after | **CARRY verbatim** — every external call timeout-bounded (threat S8-T11) |
| Q-EMAIL-DIRECT | Resend email bypasses the tenant dispatcher (ops-alert only, `access-request-notify`), 5s abort, soft-disable | **CARRY the separation verbatim** — RESEND_API_KEY is a 🔴 secret, config-only |
| Q-GDPR-GLOBAL-SINGLETON | anonymizer.gdpr singletonKey = queue-name → at most ONE erasure in-flight system-wide; batch-of-10 SKIP LOCKED inside (census A11) | **CARRY (intentional serialization) — but recommend the claim-before-send CAS** model over a global singleton; erasure throughput is a queueing-theory ceiling if volume grows (Q11) |
| Q-VELOCITY-INPROC | velocity.flush 5s in-process debounce buffer, flushed via a queue job (census §6) | **PRODUCT DECISION (Q9)** — fold into an in-process tokio mpsc (lose the 5s buffer on crash, already the current risk) vs keep the queue round-trip |
| Q-BOOT-ASSERT | only access-request + delivery-trace crons are boot-asserted (fail-fast on missing schedule) (census §3) | **CARRY + EXTEND to all 23 crons** — a Rust preflight asserts the full roster; visible red deploy vs silent zombie |
| Q-WORKER-ROSTER-DUP | two hardcoded worker-id arrays (CRITICAL_WORKERS 5 / EXPECTED_WORKERS 8; intentional two-tier, not drift) (census §3/§7) | **FIX-IN-PORT** — one roster with a `critical: bool` flag; no parallel arrays (threat S8-T10) |
| Q-LIVENESS-DETECT | the sweep's DETECTION query (overdue-undrained order.timeout count → `ops:order_timeout_lag`) is the real drain-signal, not the heartbeat | **CARRY verbatim** — the heartbeat proves the VM, the detection proves the queue drains |
| Q-UTC-CRON | every cron is UTC (no tz option passed); quiet-hours is the ONLY tz-aware eval (against `locations.timezone`) | **CARRY** — UTC crons + `chrono-tz` only for quiet-hours |
| Q-BACKUP-2TIER | backup family's in-handler 3-attempt loop BEFORE queue-level retry (deliberate two-tier) | **CARRY the shape**; the dump/upload itself → **DEFER to backup/DR council** (ops-binary sidecar, REBUILD-MAP §8); S8 owns only the cron trigger |
| Q-DEAD-QUEUES | `dwell.escalate` (dead), `order.pending_aging` (dead const), `settlement.cron`-as-queue (dead const), `health-job` (never enqueued) | **RETIRE (proof-of-deadness)** — matrix rows, not silent omissions (census §7 A1/A3/A4/A8) |
| Q-085-WATERMARK | settlement cron is S8-scheduled; mig 085 watermark `2026-07-10` — early literal double-pays (census/S5 Q7) | **OPERATOR TIMING GATE (Q7c)** — S8 does not author/apply; surfaced so the schedule can't trip it |

## 12. Cutover DoD (REBUILD-MAP §3, this surface)

Jobs/notifications E2E slice green (as-is specs — `telegram-webhook`, `notification-events`,
`flow-core-lifecycles` push, `dispatch-recovery` sweep, `backup.verify`) · `openapi-diff` empty for the S8
namespace · invariant-cluster red→green:
- **Queue runtime** — claim under `FOR UPDATE SKIP LOCKED` (two concurrent runners never claim the same row);
  at-least-once (a crash-before-complete re-runs the job); visibility-timeout reclaim (an `active` job past
  `locked_until` is re-claimed); retry→backoff→DLQ (exhausted job lands in the DLQ, not `failed`-with-no-salvage);
  **transactional enqueue** (a rolled-back producer tx rolls back the enqueue).
- **Money idempotency (Q6)** — a **double-fired settlement** → one settlement (watermark holds); a
  double-fired auto-cancel → one CANCELLED (status-CAS holds); a double-run refund reconciler → one
  `refund_due` (N5 partial unique holds). The queue is proven at-least-once; the **effect** is proven
  exactly-once.
- **VAPID/consent (Q3)** — the private key is absent from every log/error/DLQ/serialized-payload (guardrail);
  an **opted-out** customer / a category-disabled owner target is **not pushed** (dispatch-time prefs re-check);
  prune-on-410/404.
- **PII claim-check (Q5)** — a job payload contains no phone/name/address (guardrail); a DLQ row contains the
  claim only; the customer push body is no-PII; the Telegram render masks the phone.
- **Telegram fail-closed (Q4)** — a webhook with `TELEGRAM_BOT_SECRET` set and the header **missing** → 401
  (E2E delta vs the current fail-open); wrong header → 401 (constant-time); correct → 200.
- **Cron single-flight (Q2/Q7)** — two runners never double-run a cron (advisory lock); the fleet-atomic-flip
  probe proves no cron/consumer runs on both stacks; every cron is boot-asserted (preflight red on a missing
  schedule).
- **Cutover (Q7)** — the S5→S8 window shim proven (a Rust-created order's `order.created` Telegram fires via
  the shared queue); migration 086 landed; the new `jobs` table migration landed staging-first; rollback =
  fleet flag back to Node.

map-coverage zero-diff for the S8 namespaces · **council sign-off + rollback plan** (atomic fleet flip back to
Node; single-stack ownership throughout). **No 🔴 S8 row builds before this packet is APPROVED and the 🔴
questions (Q1/Q3/Q4/Q5/Q6/Q7) are operator-signed.**

---

**council seats to run: breaker, counsel** (architect authored; human operator signs 🔴 Q1 runner
at-least-once/idempotency baseline / Q3 VAPID key + consent / Q4 telegram fail-closed / Q5 PII claim-check /
Q6 money-adjacent idempotency / Q7 cutover fleet-flip + cron ownership + the S5→S8 window shim).
**packet-status: 🟡 DRAFT.**
