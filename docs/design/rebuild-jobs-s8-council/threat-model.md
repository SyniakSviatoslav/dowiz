# S8-JOBS/NOTIFICATIONS Port — Council Packet · THREAT MODEL

> **STATUS: 🟡 DRAFT — NOT APPROVED.** Adversarial input to the S8 council. Assets, trust boundaries, and
> the failure modes the Rust port must not silently introduce — on the surface that owns the background
> runtime (the queue that guarantees a side effect fires, and the crons that fire money/GDPR/backup on a
> clock). Read alongside `proposal.md`. Docs only; no code.

- **Method:** STRIDE-lite over the S8 jobs/notifications surface + fold-in of the queue-runtime invariants
  (at-least-once × idempotency), the notification consent/PII model, the money-adjacent cron idempotency
  guards (ADR-audit-fix-money L-D / mig 086), and the **novel cross-stack cutover class** unique to a
  strangler where **two background fleets can run against one Postgres**.
- **Scope note:** the B3 (NOBYPASSRLS) flip is a **B3-council fix** — recorded where it changes what a
  background job must hold (§4), but its fix lives there. The **money-math** (`app_generate_settlements`),
  the **order state machine** (`updateOrderStatus`), the **GDPR erasure logic** (`gdpr_erase_customer`), the
  **backup pipeline**, and the **Plisio payment webhook** are OUT of S8 (proposal §2) — S8 owns their
  **scheduling + single-flight + at-least-once idempotency plumbing**, not their semantics. Plisio shares
  only the "unauthenticated webhook → fail-closed" threat class with Telegram (T5).

---

## 1. Assets

| ID | Asset | Where it lives | Why it matters |
|---|---|---|---|
| J1 | The **job ledger** (scheduled/pending side effects) | new `jobs` table + `pgboss.job` during overlap | The durability guarantee that a notification/timeout/settlement eventually fires; a lost or double-drained row is a missed or duplicated side effect |
| J2 | The **VAPID private key** | config (`VAPID_PRIVATE_KEY`) → `Arc`, never persisted/logged | Signs every web-push JWT; a leak lets **anyone push to our subscribers as us** (phishing at fleet scale) |
| J3 | **Push subscriptions** (`customer_devices` p256dh/auth/endpoint; `owner_notification_targets` JSON sub) | tenant tables (RLS) | Device-identifying + the target of every push; a leak is a tracking/spam vector |
| J4 | **Notification consent/prefs** (`customer_devices.opted_in`; `owner_notification_targets.prefs`; `notification_prefs_audit`) | tenant tables (RLS) + audit | GDPR consent; pushing an **opted-out** user is a consent violation; the audit is the proof-of-consent trail |
| J5 | **PII in notification content** (customer phone/name/address) | `customers`/`orders` (tenant, RLS) — **never** in a job payload | Must be **claim-checked**: fetched at render, masked, never durable in a job/DLQ row/log |
| J6 | The **Telegram bot secret + token** (`TELEGRAM_BOT_SECRET`, `TELEGRAM_BOT_TOKEN`) | config | The webhook auth + the send credential; a forged webhook mutates store-open/close + order confirm/reject |
| J7 | The **Resend API key** (`RESEND_API_KEY`) | config | A leak sends email as us (ops-alert channel) |
| J8 | **Money-adjacent job effects** (settlement generation, auto-cancel, refund_due) | `settlements`, `orders.status`, `payment_events` (money tables) | A double-fired job **double-pays a courier**, double-cancels an order, or double-obligates a refund |
| J9 | The **cron single-flight lock** (`pg_try_advisory_lock`) | database-global advisory lock | The guard against a cron double-firing across machines **and** across stacks; the money-idempotency backstop of last resort |

## 2. Trust boundaries

- **TB-1 Telegram → webhook (`POST /webhook/telegram/:secret`)** — **unauthenticated except the shared
  secret.** The URL-path secret is a router token (referer/log-leakable); the header `secret_token` is the
  real gate. **Currently fail-OPEN on a missing header** (T5). The Rust port must fail-closed on the
  constant-time **header** compare.
- **TB-2 browser push service (FCM / Mozilla) → us** — the push endpoint is **vendor-controlled**; the only
  trusted signals back are `410/404` (gone → prune) and `429` (back off). A push send is fire-and-forget over
  a per-subscription URL; no response is order-authoritative.
- **TB-3 producer → queue (transactional enqueue)** — the ONE site (`order-persistence.ts:158`) inserts a job
  **inside the caller's tx**; a ROLLBACK must roll back the enqueue. The trust: the job row is as durable and
  as atomic as the order it pairs with.
- **TB-4 background job → business tables (no request principal)** — a job has **no JWT**. It writes under a
  **SECURITY DEFINER function** (cross-tenant sweeps) or a **seated GUC** (`app.user_id`/`app.current_tenant`
  on a per-entity re-fetch). Post-B3 a context-free job write matches 0 rows / raises (the anonymizer-N1 class,
  replayed on the worker plane).
- **TB-5 stack → stack (cutover)** — the **novel** boundary: during the overlap two background fleets
  (Node + Rust) can run against one Postgres. The trust each places in the other is mediated **only** by the
  shared DB guards (advisory locks, watermark, status-CAS, partial-unique) — a dual-run fleet or a mismatched
  lock-id namespace breaks it. **The safe posture is exactly-one-fleet.**
- **TB-6 job payload / DLQ / logs → durable sinks** — a job row, a DLQ row, and a log line are **durable**.
  The claim-check keeps PII out of all three; a payload that inlines contact turns every sink into a PII
  store.

## 3. Port-specific failure scenarios (Rust)

| # | Scenario | Trigger in the port | Mitigation to prove red→green |
|---|---|---|---|
| **S8-T1** | **Double-send / double-pay on retry** — a money/notify job re-runs (crash-before-complete, or a queue retry) and double-settles / double-messages | Treating the at-least-once runner as at-most-once; no DB-level idempotency guard on the handler | At-least-once runner (§3) + **idempotent-by-Postgres** effect: settlement watermark, auto-cancel `WHERE status='PENDING'`, refund_due N5 partial unique, notify dedup-key. Probe: a **double-fired settlement → one settlement**; double-fired auto-cancel → one CANCELLED |
| **S8-T2** | **VAPID private-key leak** — the key is logged, in an error payload, in a DLQ row, or committed | `tracing`/`Debug` over the config; error-serializing the adapter; DLQ capturing the signer | Key in config → `Arc`, never logged; **guardrail asserts absence** from every `tracing` field, serialized error, DLQ row; secret-scan gate on git (the prod-Supabase-creds incident precedent) |
| **S8-T3** | **Cron double-fire across machines/stacks** — two `web` instances OR Node+Rust both run `settlement.generate` → double-pay | Dropping `pg_try_advisory_lock`; a mismatched lock-id namespace across stacks; a dual-run fleet | `pg_try_advisory_lock` single-flight (database-global) + **exactly-one-stack-owns-the-fleet** (Q7); the settlement effect is watermark-idempotent so even a lock miss is bounded, not doubled. Probe: two runners never double-run; fleet-atomic-flip probe proves single ownership |
| **S8-T4** | **PII in a durable sink** — customer phone/name/address inline in a job payload, a DLQ row, or a log | Inlining contact to skip the re-fetch; DLQ capturing a re-fetched order; Debug-printing an order row | Claim-check: payload = `{entity_id, location_id, event}` (guardrail asserts no PII field representable); re-fetch under the GUC seat; DLQ holds the claim + **redacted** `last_error`; render masks phone; customer push = no-PII body. Probe: no payload/DLQ/log contains a phone/name/address |
| **S8-T5** | **Unauthenticated / forged Telegram webhook** — an attacker POSTs updates and flips store open/close or confirms/rejects orders | Carrying the fail-open (missing header → processed) | **FIX-IN-PORT**: fail-closed unconditionally — missing OR wrong header → 401, constant-time compare on the **header**. Probe: missing-header → 401 (E2E delta vs the current fail-open, matching the stale test) |
| **S8-T6** | **Pushing an opted-out user** — a consent race or a missed prefs check pushes a user who opted out | Checking prefs at enqueue time (stale) instead of at dispatch; dropping the `opted_in`/category gate | Re-check consent at **dispatch** under `FOR UPDATE`; `transactional` never suppressed, `operational`/`quality` category-gated; carry `setCategoryPref` atomic `jsonb_set` + same-txn audit. Probe: an opted-out customer / category-disabled owner target is **not pushed** |
| **S8-T7** | **Queue starvation / poison job** — a job that always throws hammers the runner; one hot queue starves others | No `max_attempts` cap; no per-queue fairness; no DLQ quarantine | `max_attempts → DLQ` quarantines the poison job; `ORDER BY priority, run_after` + per-queue claim quota; the visibility-timeout self-reclaim prevents a stuck job holding a slot forever. Probe: a poison job lands in the DLQ after N attempts and stops re-hammering |
| **S8-T8** | **Visibility-timeout mis-set → double-run or stuck job** — `vt` too short re-claims a still-running job (double effect); too long strands a dead worker's job | Guessing `vt` below the handler's max runtime; unbounded external calls extending the handler | `vt` > max handler runtime; **every external call timeout-bounded** (T11) so max runtime is knowable; idempotency (T1) absorbs a double-run; the `ops:*_lag` detection + liveness surface a stuck job. Probe: an `active` job past `locked_until` is re-claimed; a slow handler within `vt` completes once |
| **S8-T9** | **Lost `order.created` notification during the S5→S8 window** — a Rust-created order's owner Telegram never fires | Rust-S5 not enqueuing `notify.telegram.send` into the live (Node) queue; assuming a sweep floors it (it does not — only `order.timeout` is sweep-floored) | Rust-S5 enqueues into the **shared queue contract** (`pgboss.job`, Node-drained) for the window (or routes `order.created` via the S6 bus); S8's atomic flip switches producers+consumers to `jobs` together. Probe: a Rust-created order's `order.created` Telegram fires via the shared queue |
| **S8-T10** | **Worker-liveness false-green** — the heartbeat is healthy but the queue is not draining (a lost/stuck consumer) | Trusting the heartbeat (proves the VM) as a drain signal; dropping the DETECTION query | Carry the sweep's DETECTION (overdue-undrained `order.timeout` count → `ops:order_timeout_lag`) + the `liveness.check` watcher-of-the-watcher; **unify the two hardcoded rosters** into one with a `critical` flag. Probe: an undrained queue raises `ops:order_timeout_lag` while the heartbeat is still healthy |
| **S8-T11** | **External call pins a worker connection** — a hung Telegram/Resend/web-push call holds a DB connection past `vt` | Dropping the per-chat circuit breaker / rate-limit; an unbounded `reqwest` inside a held tx | Every external call timeout-bounded (the TMA 5s `Promise.race`, the email 5s abort precedents); per-chat rate-limit (1/1.2s) + circuit breaker (5→60s); **no external call inside a held DB tx**. Probe: a stalled external call fails within the timeout, does not extend the handler past `vt` |
| **S8-T12** | **DLQ silent accumulation** — jobs die into the DLQ and nobody looks (current: no `.dlq` consumer) | Reproducing the "6/30 DLQs, zero consumers" state; a `failed`-with-no-salvage default | DLQ default-on for every queue + a **periodic ops-alert on dead-job count** (the ops bus). Probe: an exhausted job lands in the DLQ and raises an alert, not a silent `failed` |
| **S8-T13** | **Context-free job write matches 0 rows post-B3** — a background write with no GUC/DEFINER silently no-ops under FORCE RLS | Porting a cross-tenant sweep as a raw pool query (BYPASSRLS-masked today) instead of a DEFINER fn / seated GUC | KEEP the DEFINER sweeps (`app_sweep_timeout_orders`, `app_reconcile_refund_due`, `app_generate_settlements`); seat the per-entity GUC on re-fetch (`app.user_id` for customer devices); NOBYPASSRLS probe on the worker plane |

## 4. What the B3 RLS flip changes for S8

- **Today (BYPASSRLS):** the cross-tenant sweeps "work" as raw pool queries because RLS is bypassed — the
  danger is invisible (the anonymizer-N1 / raw-pool masking, now on the worker plane). The DEFINER functions
  already in place (`app_sweep_timeout_orders`, `app_reconcile_refund_due`) are correct **independent** of the
  pool role.
- **Post-flip (NOBYPASSRLS):** every background write must satisfy RLS via a DEFINER boundary (cross-tenant
  sweeps) or a seated GUC (per-entity re-fetch). The notification worker already seats `app.user_id` before
  reading `customer_devices` (source-verified) — carry the seat. **The context-free write matches 0 rows
  (S8-T13).**
- **S8's rule:** every background job is correct **independent of which pool role is live** — the DEFINER
  sweeps and the seated-GUC re-fetches hold pre- and post-B3, so the B3 flip and the Node→Rust fleet flip are
  two orthogonal, independently-reversible events. **Land mig 086 before the fleet flip** (S5 dependency) so
  the refund floor is stack-agnostic.

## 5. Residual risks (summary for the human)

- **The hardened-baseline behavior change (Q8 / S8-T12)** — shipping backoff+DLQ everywhere and paging on
  dead jobs is strictly-more-reliable, but IS a behavior change from the 24/30 bare-defaults state. Must be an
  **explicit operator-signed improvement**, not a silent parity break. **A likely counsel flag.** Owner: S8
  lead + operator.
- **Cross-stack double-fire of a money-adjacent cron (S8-T3 / Q7)** — the money-irreversible failure the
  "one stack owns the fleet" posture exists to prevent; bounded by the watermark/status-CAS guards **iff** the
  fleet is truly single-owned and the settlement effect is atomic. **The most likely breaker escalation** —
  the council should have the breaker attack the single-ownership assumption and the S5→S8 window shim. Owner:
  architect + operator.
- **The 2026-07-10 settlement watermark (Q7c)** — the settlement cron is S8-scheduled; a settlement apply that
  slips past the literal DOUBLE-PAYS old courier rows unless the operator bumps all three occurrences.
  Surfaced (as in S5 Q7) so the rebuild schedule cannot silently trip it. Owner: operator.
- **PII claim-check regression (S8-T4 / Q5)** — the current claim-check is correct; the residual is that a
  future producer inlines contact "for convenience." The guardrail (no PII field representable in a payload)
  makes the regression a compile/test failure, not a silent leak. Owner: S8 lead + counsel.
- **The anonymizer.gdpr global-singleton serialization (Q11)** — erasure throughput is serialized to the
  batch-of-10 loop; a queueing-theory ceiling if erasure volume grows. Accepted as a *current, correct*
  property (erasure logic is S9); the CAS model (Q11a) removes the ceiling without changing semantics. Owner:
  S8 lead + S9.
- **VAPID key at-rest posture (Q3b)** — the key lives in the config struct + secret-scan gate today, not a
  secrets manager. Accepted as the current posture (matches RESEND/PLISIO); a manager is an ops upgrade, not
  an S8 blocker. Owner: operator.

**None of J1-J9's failure modes is *introduced* by the rewrite** — the claim-check, the consent model, the
DEFINER sweeps, the money-idempotency guards, and the advisory-lock single-flight are all **current**
properties the port must carry **visibly** (matrix row + test). The **fail-open Telegram webhook (S8-T5)** is
a current *defect* the port FIXES. The rewrite's genuinely *new* risk is entirely the **cross-stack cutover**
(S8-T3 / T9, TB-5) — two background fleets on one Postgres — which no prior single-stack packet faced.
**Breaker-escalation candidate: the cross-stack money-cron double-fire + the S5→S8 window shim (S8-T3/T9).**
**Counsel-flag candidate: the Q8 hardened-baseline behavior change** and the **VAPID/consent** pair — both
must be explicit, owned decisions, never carried by silence.
