# S8-JOBS/NOTIFICATIONS — Council RESOLVE

> **Verdict: PROCEED-WITH-REVISIONS. No ETHICAL-STOP (counsel).** Packet-status 🟡 — NOT
> COUNCIL-APPROVED until operator signs §3. Seats: architect (packet) · breaker (1 CRIT / 3 HIGH /
> 3 MED / 2 LOW) · counsel (PROCEED-WITH-REVISIONS) · lead (this RESOLVE). The breaker + a lead
> ground-truth verification corrected a fact propagated across THREE councils (085) — see REV-S8-3.

## 1. Frozen revision set

- **REV-S8-1 (breaker CRIT + counsel #3 — notification idempotency is Potemkin).** The dedup the
  packet calls "Postgres-guarded" does NOT durably exist: `notifications/workers/index.ts:70/350/484`
  uses an in-memory `Set` (resets on restart/crash/flip), `notification_outbox_audit` has NO unique
  constraint (`mig 1790000000007:12-31`) so its `ON CONFLICT DO NOTHING` never dedups, and the enqueue
  `singletonKey` is a no-op on the bare queue. On an at-least-once crash-after-send the owner's
  `order.created` Telegram DOUBLE-sends. REV: the Rust port **BUILDS** a real Postgres dedup —
  claim-before-send / a CAS on a unique `(dedup_key)` — the packet's own gold-standard, NOT the `Set`.
  (Customer push is safe — device coalesces by `tag:order-<id>`.) This is the S8-twin of the S5
  Potemkin promo: a property documented as built that the runtime doesn't hold.
- **REV-S8-2 (breaker HIGH + counsel #4 — Telegram fail-open, FIX NOW on Node).** `telegram-webhook.ts:96-99`
  fail-OPENS: a missing `secret_token` → processed → a forged webhook flips owner order state
  (`order.confirm/reject`), LIVE on production today. The e2e spec ALREADY asserts missing-header→401
  (`e2e/tests/telegram-webhook.spec.ts:53-59`) — so the guardrail is red→green-ready; the handler is
  the bug. REV: fix **NOW, standalone on Node** (fail-closed, constant-time header compare on ALL
  paths — missing/empty/wrong-length), don't wait for the S8 port (that leaves the hole open until the
  last surface, 6 days out). The port carries the already-fixed behavior. Also flag `/start login_` as
  a second forged-webhook surface. **[Dispatched as a live hotfix lane in parallel with this RESOLVE.]**
- **REV-S8-3 (breaker HIGH — 085/settlement RECONCILIATION, cross-council fact correction).**
  Lead-verified against the tree: (a) 085/086/087 exist ONLY as un-applied DRAFTS in
  `docs/design/audit-fix-money/migration-drafts/` (the `2026-07-10` literal is there, ×5); (b) they are
  NOT in `packages/db/migrations/`; (c) the LIVE settlement dedup is `settlement_items.assignment_id`
  NOT-EXISTS + `FOR UPDATE … SKIP LOCKED` (`mig 1790000000078:178-184`), **idempotent by
  construction** — a cron double-fire TODAY collapses to one effect (the second finds the row). So the
  "085 watermark 2026-07-10 landmine" propagated across S5/S7/S8 is a DRAFT's future concern, NOT a
  live one; the real live guard was under-named. CORRECTION: the Rust settlement cron is a thin caller
  of the DEFINER fn (which carries the assignment_id guard) — inherit the REAL guard; the watermark
  timing gate applies ONLY if/when 085 is applied. Advisory-lock single-flight = optional
  defense-in-depth, NOT the load-bearing guard (the breaker correctly noted there is no advisory lock
  today and none is needed for correctness). Update the S5/S7 DoD language to name the assignment_id
  guard, not a fictional live watermark.
- **REV-S8-4 (breaker HIGH — S5→S8 window shim is partition-blind).** pg-boss v10 is
  partition-per-queue (`CREATE TABLE pgboss.<hash>` via runtime `create_queue()`,
  `mig 1790000000011:49-50`); a hand-rolled Rust `INSERT INTO pgboss.job` lands in the wrong/absent
  partition → the Rust-created order's `order.created` Telegram is SILENTLY DROPPED. The
  "symmetric with `idempotency_keys`" analogy is false (that table isn't partitioned). REV: the
  overlap bridge must be partition-aware (call `create_queue`/insert into the right partition) OR use a
  different bridge (e.g. a NOTIFY the Node worker consumes, or Node keeps owning order.created
  notifications until the S5+S8 surfaces cut over together). Design the bridge; do not assume a bare INSERT.
- **REV-S8-5 (breaker MED + counsel #1/#2).** (a) Consent: the `FOR UPDATE` is on the prefs WRITER,
  the dispatch READ is a plain filtered SELECT (correct — do NOT over-lock the hot read); the sub-ms
  read→send gap is irreducible, document it; no spam window (affirmed). (b) `handleCustomerStatus`
  reads `orders` BEFORE seating the tenant GUC (`index.ts:108-115`) → post-B3 0-rows silent
  customer-push drop → FIX-IN-PORT (seat first, the S7 complete-census pattern). (c) runner pool
  sized for 4 minute-crons + listener + claim loop (≥6), not 4. (d) PII: `error_message` writes raw
  `err.message` (incidental-PII-free, not structural, `mig 007:8`) → the port BUILDS the guarantee
  (redacted `last_error` + a no-PII-pattern assert), converting incidental→structural.
- **REV-S8-6 (counsel #6 — the opted-in person's silence).** All the apparatus protects people from
  UNWANTED notifications; nothing tells the person who OPTED IN and relies on a notification that it
  silently FAILED (webhook-200-on-fail, circuit-open skip, target auto-disabled on exhaustion,
  subscription auto-pruned on 410 = four silent failures). REV: extend the Q8 ops-alert from
  dead-jobs to notification-channel degradation, routed to the WAITING person (owner/courier), not
  just an audit table. Named future-hardening with owner + trigger (not a silent defer).
- **VAPID/queue baseline:** VAPID private key never logged/DLQ'd/committed (guardrail asserts absence);
  visibility-timeout (`locked_until`) > max bounded handler runtime (every external call timeout-bounded);
  DLQ default-on + the existing nightly failed-job detector (`reconciliation.ts:60`) as consumer.

## 2. Question resolutions
- Q1 → hand-rolled SKIP-LOCKED runner, at-least-once + **Postgres-BUILT idempotency** (REV-S8-1). 🔴
- Q2 → cron mechanism settled (advisory-lock single-flight for cron leader; DB-backed schedule).
- Q3 → VAPID key guardrail + consent re-check (no spam window). 🔴
- Q4 → Telegram fail-CLOSED, **fixed now on Node** (REV-S8-2). 🔴
- Q5 → PII claim-check + error_message structural redaction (REV-S8-5d). 🔴
- Q6 → money crons idempotent by the REAL `assignment_id` guard (REV-S8-3), thin DEFINER caller. 🔴
- Q7 → cutover fleet-atomic flip + partition-aware S5→S8 shim (REV-S8-4) + CORRECTED 085 status. 🔴

## 3. 🔴 OPERATOR SIGN-OFF (blocks build)
Q1 (Postgres-built notification idempotency) · Q3 (VAPID + consent) · Q4 (Telegram fail-closed —
approve the LIVE Node hotfix + its staging deploy) · Q5 (PII incl. error_message) · Q6 (money real-guard
naming, corrected) · Q7 (fleet-atomic flip + partition-aware shim + 085-is-a-draft correction).

## 4. Build/cutover DoD deltas
Postgres-dedup crash-recovery test (REV-S8-1) · Telegram fail-closed = the existing e2e spec red→green
(REV-S8-2) · settlement double-fire→one-effect probe against the REAL assignment_id guard (REV-S8-3) ·
partition-aware order.created bridge test (REV-S8-4) · tenant-seat-first customer-push probe (REV-S8-5b)
· error_message no-PII assert (REV-S8-5d) · notification-degradation ops-alert (REV-S8-6). Cross-council:
correct the S5/S7 085-watermark language to the assignment_id guard.
