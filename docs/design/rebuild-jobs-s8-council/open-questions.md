# S8-JOBS/NOTIFICATIONS Port — Council Packet · OPEN QUESTIONS

> **STATUS: 🟡 DRAFT — NOT APPROVED.** Everything the live Triadic Council (architect / breaker /
> counsel / human) must decide before S8 jobs/notifications is ported. Each question has options + a
> lane-R3 recommendation — a *starting position for friction*, not a decision. S8 is **less red-line
> than S5-S7** (no order-total composition, no dispatch state machine on the hot path) but carries real
> ones: the queue runtime's at-least-once/idempotency contract, the VAPID private key, PII in
> notifications, money-adjacent double-fire, and the cross-stack fleet cutover. Docs only.

Legend: **[QUEUE]** runtime/at-least-once · **[MONEY]** money-adjacent idempotency · **[SEC]**
secret/consent/tenancy · **[PII]** claim-check · **[RELIABILITY]** retry/DLQ/observability ·
**[INFRA]** cutover/topology · **[SCOPE]** surface placement. 🔴 = red-line, operator sign-off required.

---

### Q1 🔴 [QUEUE] The hand-rolled SKIP LOCKED runner — at-least-once + idempotency baseline
The verdict (REBUILD-MAP §2) is `SELECT … FOR UPDATE SKIP LOCKED` + PgListener. The runner is
**at-least-once by construction** (completion is a write *after* the side effect; a crash re-runs the job).
Proposal §3 specifies: the claim query (with a `locked_until` visibility-timeout self-reclaim), a
session-mode PgListener wake + a poll floor, retry/backoff, a DLQ, a `enqueue(&mut tx, …)` for the one
transactional producer, and — critically — that **every money/notify handler is idempotent by a Postgres
guard, never by a non-existent at-most-once**.
- **(a) Build the runner exactly as §3, at-least-once + DB-level idempotency; new `jobs` table; 21 cron-only
  workers shed the queue (tokio loop + advisory lock), only ~9 keep the queue table.** *(recommend)*
- **(b) Adopt a crate (apalis-postgres / sqlxmq / graphile_worker_rs)** — rejected as the *primary* (apalis
  RC-only, sqlxmq/underway stale per the census research); kept as a **documented fallback** if the
  hand-rolled runtime proves heavier than a maintained crate at Phase-A.
- **(c) Port pg-boss 1:1 semantics** — rejected: re-imports the `policy:'short'` singletonKey footgun and the
  24/30-bare-defaults gap; 21 queues don't need a queue table at all.

**R3 recommendation:** (a). **🔴** because the at-least-once × idempotency contract is the whole surface's
correctness spine — a runner that silently assumes at-most-once double-sends money/notifications on the first
crash. The visibility-timeout value (`vt` > max handler runtime) and the transactional-enqueue API are named
build gates. Owner: architect + operator.

### Q2 [QUEUE] Cron scheduling in Rust — tokio + advisory-lock single-flight
23 crons, all UTC. Today each takes `pg_try_advisory_lock(id)` so two `web` instances never double-run.
- **(a) tokio cron loops (`tokio_cron_scheduler` for cron-expr parity) + `pg_try_advisory_lock`, with a
  lock-id registry (Q10) and boot-assert of the full roster (Q-BOOT-ASSERT).** *(recommend)*
- **(b) A DB-backed schedule table + leader election** — rejected: over-engineering; advisory locks already
  give database-global single-flight for free, and the cron roster is static (23 known jobs), not dynamic.

**R3 recommendation:** (a). Not 🔴 on the *mechanism* (boring/proven), but the **cross-stack ownership** of the
crons during overlap **is** 🔴 and escalates into **Q7** (a settlement cron double-firing across Node+Rust
double-pays). Owner: S8 lead (mechanism) + operator (Q7 cross-stack ownership).

### Q3 🔴 [SEC] VAPID private key + notification consent
The VAPID private key signs every web-push JWT (leak = anyone pushes to our subscribers as us). Consent is
authoritative: an opted-out customer (`customer_devices.opted_in=false`) or a category-disabled owner target
must **not** be pushed; prefs are re-checked at **dispatch** time under `FOR UPDATE`, with a same-txn audit
into `notification_prefs_audit`.
- **(a) Key in the ONE Rust config struct, preflight-validated, adapter registers only if both keys set;
  NEVER logged / in an error payload / in a DLQ row / in git (guardrail asserts absence). Carry the atomic
  `setCategoryPref` + audit; re-check consent at dispatch; `transactional` never suppressed, `operational`/
  `quality` category-gated; quiet-hours via `chrono-tz` against `locations.timezone`.** *(recommend)*
- **(b) Key from a secrets manager at runtime** — deferred: acceptable later, but the config-struct + secret-
  scan gate is the current posture (matches RESEND/PLISIO); a manager is an ops upgrade, not an S8 blocker.

**R3 recommendation:** (a). **🔴** — a leaked VAPID private key is a fleet-wide push-spoof, and pushing an
opted-out user is a consent/GDPR violation. Both are current, correct properties the port must carry
**visibly** (guardrail + dispatch-time re-check), not silently. Owner: S8 lead + operator + (consent)
counsel.

### Q4 🔴 [SEC] Telegram webhook — FIX-IN-PORT the fail-open signature
`telegram-webhook.ts:96-99` (source-verified): with `TELEGRAM_BOT_SECRET` set but the
`x-telegram-bot-api-secret-token` header **absent**, the handler logs a warning and **processes the request
anyway** ("backward compat"); it 401s only on a *wrong* header. `telegram-webhook.spec.ts:53-59` asserts the
opposite (missing→401) — a live fail-open gap. The URL-path secret (`/webhook/telegram/:secret`) is
referer/log-leakable and is a router token, not the auth.
- **(a) FIX-IN-PORT: fail-closed unconditionally — missing OR mismatched header → 401, constant-time compare
  (`subtle::ConstantTimeEq`); gate on the HEADER, never the URL alone. E2E delta: missing-header → 401
  (matches the existing test).** *(recommend)*
- **(b) CARRY the fail-open (parity-pure)** — rejected: re-ships an unauthenticated-webhook processing gap
  through a deliberate security rewrite; the fix-vs-carry rule mandates FIX-IN-PORT for 🔴 security with a
  documented E2E delta.

**R3 recommendation:** (a). **🔴** — an unauthenticated webhook that mutates store open/close + order
confirm/reject state is a real attack surface; the fix is one constant-time compare and it aligns code with
the test the suite already asserts. Owner: S8 lead + operator.

### Q5 🔴 [PII] The claim-check — no customer PII in job payloads, logs, or the DLQ
The current design is already a correct claim-check: job payloads carry `{entity_id, location_id, event}`,
the worker re-fetches under tenant isolation, renders with `maskPhone`, and the customer-status push is a
minimal no-PII body (source-verified). The port must carry this *visibly* and extend it to the DLQ.
- **(a) Job payload type = `{entity_id, location_id, event}` only (a guardrail asserts no PII field is
  representable); re-fetch under the tenant/user GUC seat; render masks the phone; customer push = money +
  short-id only; the DLQ persists the claim + a **redacted** `last_error`; `tracing` never Debug-prints a
  re-fetched order.** *(recommend)*
- **(b) Inline the contact in the payload to skip the re-fetch** — rejected: a job row / DLQ row / log line
  is a durable PII sink; the re-fetch is cheap and the claim-check is the whole point (owner-data-export-ai
  ETHICAL-STOP precedent + the money-audit anonymizer invariants).

**R3 recommendation:** (a). **🔴** — PII in a durable queue/DLQ/log is a data-protection defect and a GDPR
exposure; the claim-check is a current, correct property the port must not regress. Owner: S8 lead +
counsel.

### Q6 🔴 [MONEY] Money-adjacent jobs — at-least-once queue + idempotent-by-Postgres effect
`settlement.generate`, `order.timeout(_sweep)` auto-cancel, and the refund_due reconciler are money-adjacent.
The queue is at-least-once (Q1); the effect must be exactly-once **by a DB guard**, never by trusting the
queue: settlement = watermark (one atomic DEFINER call, all-or-nothing); auto-cancel = `WHERE status='PENDING'`
status-CAS; refund_due = the N5 partial unique `(payment_id) WHERE type='refund_due'` (mig 086).
- **(a) Preserve every DB-level idempotency guard; the S8 job owns only the scheduling + single-flight; a
  double-fired settlement/auto-cancel/reconciler → ONE effect (proven red→green). The money-math DEFINER
  functions are KEEP (S5/S7 councils).** *(recommend)*
- **(b) Rely on the queue's retry-dedup for money idempotency** — rejected: there is no at-most-once; a
  crash-before-complete re-runs the handler; only the DB guard is authoritative (idempotency in Postgres,
  not the queue).

**R3 recommendation:** (a). **🔴** — a retried or cross-stack-double-fired settlement that double-pays, or an
auto-cancel that double-terminalizes, is a live money defect; the DB guards are the exactly-once boundary and
must be proven, not assumed. Ties to Q7 (cross-stack single-flight) and the **085 watermark 2026-07-10**
timing landmine (Q7c). Owner: S8 lead + operator (watermark) + S5/S7 money council (the DEFINER math).

### Q7 🔴 [INFRA] Cutover — the background fleet flips as ONE unit; the S5→S8 window shim
Unlike S5's request surface, the fleet is **not** strangled route-by-route. Three coupled decisions:
- **Q7a [INFRA]:** **(a) New `jobs` table owned by Rust (forward-only additive migration); Node drains
  `pgboss.job`, Rust drains `jobs`, exactly one runs at a time.** *(recommend)* — (b) co-drain `pgboss.job`
  from both stacks: rejected (v10's internal state machine is fragile to co-drain).
- **Q7b 🔴 [INFRA]:** the S5→S8 window (S5-Rust create / S8-Node fleet). `order.timeout` is **sweep-floored**
  (the Node sweep recovers any overdue order stack-agnostically) so the per-order job can be skipped; but
  `notify.telegram.send order.created` is **NOT** floored — skip it and the owner never sees the new order.
  **(a) Rust-S5 enqueues into the shared queue contract (`pgboss.job`, Node-drained) for the window — a
  bounded compat shim, symmetric with S5 keeping `idempotency_keys` shared; S8's atomic flip switches
  producers + consumers to `jobs` together.** *(recommend)* — (b) route `order.created` via the S6 bus for
  the window (viable alternative); (c) skip it (rejected — lost order-created notifications).
- **Q7c 🔴 [MONEY/INFRA]:** exactly **ONE stack owns the entire fleet** at any instant (crons + consumers flip
  atomically, one ops flag / process-group swap) — a settlement cron double-firing across stacks double-pays;
  the **085 watermark 2026-07-10** is an operator timing gate (early literal double-pays).

**R3 recommendation:** Q7a (a); Q7b (a); Q7c fleet-atomic + operator-owned watermark. **🔴** — this is the
scariest S8 cutover point: two worker fleets on one Postgres, with money-adjacent crons acting on shared
tables. The controls are single-stack ownership + the shared-queue window shim + the DB idempotency guards
(Q6). Owner: architect + operator + S5 lead (the producer coupling) + breaker (attack the cross-stack
double-fire + the window shim).

---

### Settled-by-carry / reliability questions (not port-blocking, dispositioned for the record)

### Q8 [RELIABILITY] Hardened retry/backoff/DLQ baseline — FIX-IN-PORT
Only 6/30 queues (backup family) run with backoff+DLQ; 24 run bare v10 (2 retries, 0 backoff, no DLQ), and
**no consumer reads any `.dlq`** (census §5).
- **(a) Ship backoff + DLQ as the baseline for EVERY queue; a periodic ops-alert consumes the DLQ (dead-job
  count → ops bus). Documented as a deliberate strictly-more-reliable improvement, not a silent parity
  break.** *(recommend)* — (b) reproduce the 24/30 bare defaults for parity: rejected (the census names this
  the exact gap the rebuild should NOT reproduce).

**R3 recommendation:** (a). Not 🔴 (strictly-more-reliable), but flagged to the operator as a **documented
behavior improvement** — a job that previously vanished into `failed` now retries with backoff and pages if
it dies. Owner: S8 lead.

### Q9 [SCOPE] `velocity.flush` — fold into an in-process channel vs keep the queue
The 5s velocity debounce buffer is flushed via a queue job; the buffer is already in-memory (lost on a crash
before flush).
- **(a) Fold into an in-process tokio `mpsc`/debounce — shed the Postgres round-trip; the crash-loss risk is
  unchanged (already accepted).** *(recommend)* — (b) keep the queue round-trip: rejected as unnecessary
  Postgres traffic for a buffer that is not durable anyway.

**R3 recommendation:** (a). A product confirmation that losing an in-flight 5s velocity buffer on a crash is
acceptable (it already is today). Owner: S8 lead + product.

### Q10 [RELIABILITY] Advisory-lock id registry — FIX-IN-PORT the id=5 collision
`order-timeout-sweep` and `access-request-retention` both use `pg_try_advisory_lock(5)` (census §4); they
avoid a collision only by each holding a separate connection — a latent bug.
- **(a) A lock-id registry (one source of truth, unique named constants).** *(recommend)* — (b) reproduce the
  raw reused ints: rejected.

**R3 recommendation:** (a). Not 🔴; a hygiene fix the port makes for free. Owner: S8 lead.

### Q11 [SCOPE] `anonymizer.gdpr` global singletonKey — carry vs the CAS model
The erasure queue's singletonKey is the bare queue-name → at most ONE erasure in-flight system-wide; the
batch-of-10 `FOR UPDATE SKIP LOCKED` loop inside is how multiple requests get processed (census A11).
- **(a) CARRY the intentional serialization for now, but implement it as a claim-before-send CAS on the
  request row (the `access-request.notify` gold-standard model) rather than a global singletonKey, so
  throughput isn't baked into a dedup-key choice.** *(recommend)* — (b) reproduce the global singletonKey
  verbatim: acceptable but bakes a queueing-theory ceiling into the key.

**R3 recommendation:** (a). Not 🔴 (erasure logic is S9); an S8 plumbing choice. Owner: S8 lead + (S9 for the
erasure semantics).

### Q12 [SCOPE] RETIRE the dead queues
`dwell.escalate` (broken/dead), `order.pending_aging` (dead const), `settlement.cron`-as-queue (dead const),
`health-job` (never enqueued) — census §7 A1/A3/A4/A8.
- **(a) RETIRE each with a proof-of-deadness matrix row (not a silent omission).** *(recommend)*

**R3 recommendation:** (a). Not 🔴; RETIRE-with-proof per the map-coverage gate. Owner: S8 lead.

---

## Decision-ordering note for the council
**Q1 (runner at-least-once/idempotency)**, **Q5 (claim-check)**, and **Q6 (money idempotency)** are the
**correctness spine** — decide them first; every handler's shape depends on the at-least-once × idempotent-by-
Postgres contract. **Q3 (VAPID/consent)** and **Q4 (telegram fail-closed)** are the **security** pair — both
are current, correct/near-correct properties the port must carry or fix visibly. **Q7 (cutover)** is
**cutover-blocking, not build-blocking** — the Rust fleet can be built + dark-verified before Q7 settles, but
the **flip** cannot happen until the fleet-atomic-ownership + the S5→S8 window shim + the money-idempotency
guards are green. **Q2 (cron mechanism)** is settled boring; its cross-stack-ownership half lives in Q7.
Q8-Q12 are reliability/scope carries that do not block the build.

**The single most likely breaker escalation:** the **cross-stack double-fire of a money-adjacent cron**
(Q6→Q7) — a settlement cron running on both stacks double-pays; the whole "one stack owns the fleet" posture
exists to prevent it, and the breaker should attack the assumption that the fleet is truly single-owned during
the flip (and that the S5→S8 window shim doesn't silently drop `order.created` notifications). **The single
most likely counsel flag:** the **Q8 hardened-baseline behavior change** and the **VAPID/consent** pair —
shipping backoff+DLQ everywhere is strictly-more-reliable but IS a behavior change the operator should sign;
and re-shipping the notification consent model must be provably opt-out-respecting, not "it worked before."
