# Breaker findings R2 — RLS/reliability audit fixes, RE-ATTACK on proposal v2

- **Target:** `proposal.md` (v2) + `resolution.md` + `ADR-audit-fix-rls-reliability.md`, re-verified against
  live source/migrations at **HEAD of `fix/audit-remediation`** (commit `dafa0d6b`) — not the proposal's citations.
- **Method:** every v2 mechanism traced to the actual migration policy / worker / helper / pg-boss source and to
  Postgres RLS semantics. Break scenarios grounded in cited lines, not the proposal's self-description.
- **Scope:** signals only, no fixes. Two jobs: (1) did v2 kill v1's CRITICALs; (2) do the v2 mechanisms break anew.

## Six-line summary
1. **v1 CRITICALs are genuinely fixed by redesign.** F1: MIG-1's firebreak (FORCE + `ops_all TO dowiz_app`)
   is a real, verified convention (mig `1780421100065` STEP A2 + `1790000000077` RC2); courier login
   (`courier/auth.ts:249`) and session-validation (`plugins/auth.ts:74-83`) run as bare `dowiz_app` on the main
   pool, so the `ops_all`/`courier_auth_read` policies admit them — P1b tests the key. F2: the conversion gate is
   now falsifiable (P9 drives real flows on an enforced DB, P10 scans all FORCE-table SQL literals).
2. **But the legal centerpiece has a CRITICAL new hole (N1).** The GDPR worker wraps only its *bookkeeping* in
   `withTenantTx({tenantId})`; the actual erasure (`AnonymizerService.anonymize`) opens its **own** context-free
   pool connections (`lib/anonymizer/index.ts:115,192`). Post-MIG-2+flip, `customers` has **no** `app.current_tenant`
   policy arm at all → the anonymizer's `SELECT … FOR UPDATE` / `UPDATE customers` match **0 rows** → the row is
   marked `completed` with an audit row + event while the PII is retained. Silent, permanent (terminal=`completed`
   evades the O-GDPR level-trigger), and P5 is blind to it.
3. **LC4-MIG's `app.current_tenant` arm widens two legal-red-line tables (N2, HIGH).** It is a table-wide `FOR ALL`
   arm on `gdpr_erasure_requests` (subject PII) and the append-only `anonymization_audit_log`, so every
   tenant-context principal (couriers-via-shift, telegram/payments webhooks) — not just the worker — gains
   SELECT/INSERT/UPDATE/DELETE for their location post-flip; it cannot be scoped to the worker alone.
4. **F3 (the erasure-*request* table RLS) is itself CONFIRMED-FIXED** — the arm makes the worker's own
   `gdpr_erasure_requests`/`anonymization_audit_log` writes match under `{tenantId}`; N1 is a *different* surface
   (customers/orders via the anonymizer's private connection), N2 is the arm's blast radius.
5. **pg-boss OPEN-V1 is answerable at HEAD:** installed runtime is `pg-boss@10.4.2` and `manager.updateQueue`
   (`src/manager.js:570`, `src/plans.js:387`) DOES `UPDATE queue SET policy=COALESCE($2,policy)` on a live queue —
   the reconciler is viable. One footgun (N4): `updateQueue` defaults an omitted `policy` to `'standard'`, so a
   partial reconcile silently downgrades a `'short'` queue.
6. **Verdict:** all 12 round-1 findings CONFIRMED-FIXED (F1 with an N3 residual, F12 with an N6 seam);
   v2 introduces **1 CRITICAL (N1) + 1 HIGH (N2) + 2 MEDIUM + 2 LOW + 1 INFO**.

**Severity counts (NEW findings only):** CRITICAL 1 · HIGH 1 · MEDIUM 2 · LOW 2 · INFO 1.

---

## Verdict table — round-1 findings

| # | v1 Sev | Verdict | Verified basis (HEAD) |
|---|--------|---------|-----------------------|
| F1 | CRIT | **CONFIRMED-FIXED** (+ N3 residual) | Firebreak convention is real: `1780421100065:26-31` ENABLE+FORCE, `1790000000077:27-30` `ops_all FOR ALL TO dowiz_app USING(true) WITH CHECK(true)`. Login `courier/auth.ts:246-251` and session-validate `plugins/auth.ts:71-83` run on `request.server.db`/`db.connect()` as bare `dowiz_app`, **no** `SET LOCAL ROLE` → `ops_all TO dowiz_app` admits; the `courier_locations` EXISTS is rescued by MIG-1's `courier_auth_read FOR SELECT TO dowiz_app`. P1b asserts exactly this. Residual: the **failed-login** `courier_audit_log` INSERT (`auth.ts:269-273`) is not firebreak-covered → N3. |
| F2 | CRIT | **CONFIRMED-FIXED** | P9 (`proposal §5`) drives anon-checkout INSERT / track exchange / courier auth / owner auth against a NOBYPASSRLS-enforced DB — fails by construction while any FORCE-table reader is unconverted. P10 scans *SQL literals* for FORCE-table names (superset of `grep set_config`). Not gameable: both are runtime/CI red-until-converted, not inventory bookkeeping. |
| F3 | HIGH | **CONFIRMED-FIXED** | LC4-MIG adds `OR location_id = NULLIF(current_setting('app.current_tenant',true),'')::uuid` to the two member-only policies (`1780421100060:49-51,:57-59`). Under `withTenantTx({tenantId:row.location_id})` the worker's own `gdpr_erasure_requests` UPDATE + `anonymization_audit_log` INSERT (`anonymizer-gdpr.ts:67-78`) now match. (The arm's *blast radius* → N2; the *anonymize* path → N1 — both distinct surfaces.) |
| F4 | HIGH | **CONFIRMED-FIXED** | `gdpr_erasure_requests` has jsonb `metadata` (`1780421100060:23`) and no `updated_at`; v2 stamps `metadata.claimed_at` at claim time (moves per claim, unlike `requested_at`) + claim-token CAS. Slow-row reclaim is data-safe via `anonymizeCustomer`'s `FOR UPDATE` (`index.ts:118`) + `anonymized_at` idempotency (`:127`). |
| F5 | HIGH | **CONFIRMED-FIXED** (design) | Verified the current hazard: `telegram-webhook.ts:281` sets `app.current_tenant` in autocommit (no BEGIN) then `updateOrderStatus`; 409 re-read `:289-292` on the same client. v2 chat-DEFINER-resolve → short `withTenantTx` → post-commit sends (P11) removes the txn-across-HTTP and the context-free re-read. `resolve_telegram_chat` is design-only (grep: absent). Pre-existing spoofability = INFO-1 (not new). |
| F6 | MED | **CONFIRMED-FIXED** | v2 preserves partial-success via phase txns + per-row SAVEPOINT (`spa-proxy.ts:769-821` multi-write flow); no atomicity regression advertised as atomic. |
| F7 | MED | **CONFIRMED-FIXED** (dedups a wrong outcome — see N1) | CAS `WHERE status='in_progress' AND claim_token=$mine` + `FOR UPDATE SKIP LOCKED` in `gdpr_claim_due` genuinely give exactly-one audit row/event. **But** the design writes `completed` "regardless of `result.skipped`" (`proposal §3.4`) → it dedups the N1 false-completion to exactly one. |
| F8 | MED | **CONFIRMED-FIXED** | OPEN-V1 now answerable: `pg-boss@10.4.2` `updateQueue` updates `policy` on a live queue (verified `manager.js:570-597`, `plans.js:387-400`). P6 rewritten against a pre-existing `standard` queue. Footgun → N4. |
| F9 | MED | **CONFIRMED-FIXED** | `notifications/workers/index.ts:100-129`: order read carries `locationId` (`:100,:114`), devices read needs `app.user_id` (`customer_owns`, `1790000000077:109-111`). Two `withTenantTx` contexts satisfy both; P2 pinned to the flip-rehearsal DB removes the ordering false-green. |
| F10 | LOW | **CONFIRMED-FIXED** | `.dlq` is canonical (`queue-provider.ts:52-54`); `createQueueWithDefaults` already wires `deadLetter → ${name}.dlq` (`:78-83`, shipped wave-1). Monitor derives from the `QUEUE_POLICY` map. |
| F11 | LOW | **CONFIRMED-FIXED** | Zero-risk claim withdrawn; direct `boss.work` array-shape sites are real (`bootstrap/workers.ts:174` free-tier, settlement-cron/dwell/lifecycle per `pgboss-state §7`); `queue.work` wrapper normalizes arrays (`queue-provider.ts:117-122`). |
| F12 | LOW | **CONFIRMED-FIXED** (+ N6 seam) | Confirmed no per-registration isolation exists (`bootstrap/workers.ts` — ~30 sequential bare `await …start()`; only try/catch is the FREE_TIER handler body `:175-180`); static `heartbeatConfigs` of 8 (`:111-120`). v2 registry watch-set + boot DRIFT + degraded `/health` is sound; residual coverage seam → N6. |
| BN-1/BN-2 | note | adopted / accurate | Settlements read-only conversion clean; MIG-1..4 inert on the BYPASSRLS main pool (all breaks are flip-time). Verified `dowiz_app` = BYPASSRLS today. |

---

## NEW findings

### N1 — CRITICAL — B-DATA/B-CONSIST · The GDPR erasure marks `completed` while the anonymizer, on its own context-free connection, erases **nothing** post-flip
**Violated invariant:** proposal §3 "every `gdpr_erasure_requests` row reaches `completed`/`failed`; the erasure path
itself survives the flip"; resolution F3 "per-row work runs in `withTenantTx({tenantId})` … correct post-flip."

Trace the actual data path, not the bookkeeping path:

- The worker's per-row block runs in `withTenantTx({tenantId: row.location_id})` (v2), which sets `app.current_tenant`
  **on the worker's own client**. It then calls `this.anonymizerService.anonymize({scope:'gdpr', subject:{customerId,
  locationId}})` (`anonymizer-gdpr.ts:62`).
- `AnonymizerService.anonymize` takes **no client**. `anonymizeCustomer` opens a **fresh** `this.pool.connect()`
  (`lib/anonymizer/index.ts:115`), does its own `BEGIN` (`:117`), `SELECT … FROM customers … FOR UPDATE` (`:118-121`),
  `UPDATE customers SET phone=…, name=NULL, anonymized_at=now()` (`:133-141`), `COMMIT`. `anonymizeOrder` does the
  same on a second fresh connection (`:192`). **None of these connections carry any GUC** — the worker's
  `withTenantTx` context is connection-local (`set_config(...,true)`) and does not propagate to a different pooled
  connection.
- Verified `customers` policy set at HEAD: `tenant_isolation` (member, `app.user_id` → `app_member_location_ids()`;
  `1780310074262:76`), `anonymous_insert` (`1780315000000:16`), `anonymous_update`/`anonymous_select`
  (`1780338981782:6,:10`). **There is NO `app.current_tenant` arm on `customers`** (grep confirmed; RC4 added the
  courier arm only to `orders`/`delivery_trace`/`courier_cash_ledger`, `1790000000077:44-67`). `customers` is FORCE-RLS.

**Break at the flipped end-state (NOBYPASSRLS + MIG-2 applied):** on the anonymizer's context-free connection,
`app.user_id` is NULL → `tenant_isolation` admits ∅; post-MIG-2 `anonymous_select` = `app_current_user() IS NULL AND
location_id = <app.current_tenant>` with no `app.current_tenant` → NULL → admits ∅. So `SELECT … FROM customers
WHERE id=$1 FOR UPDATE` returns **0 rows** → `index.ts:122` `lockRes.rows.length===0` → `ROLLBACK`, returns
`{anon:false, skipped:true}`. **The UPDATE never runs; the customer's `phone`/`name`/`anonymized_at` are untouched.**

Back in the worker: `result.skipped=1`, but the worker writes `status='completed'` (`:67-72`), inserts the
`anonymization_audit_log` "erasure completed" row (`:74-78`, on the worker's context — matches via the N2 arm), and
publishes `gdpr.erasure_completed` (`:80-83`) — **regardless of `result.skipped`** (proposal §3.4 is explicit).

**Net:** a legally-mandated Art.17 erasure is reported `completed`, the compliance audit log confirms it, the
owner-facing surface renders it done — **while the subject's PII persists in full.** It is worse than the original
LC4 bug (which stranded honestly): this reports success. It is **permanently undetected** — O-GDPR is level-triggered
on non-terminal-`>24h` or `failed` rows only, and this row is terminal `completed`. **P5 is blind:** it asserts
`status → completed`, "1 audit row + 1 event", and reclaim behavior — it never asserts `customers.anonymized_at IS NOT
NULL` / `phone` nulled, so it goes green on the false completion. The proposal defers the anonymizer's own scoping to
"security-F2's lane" (§3.9) but that lane is about a sink *predicate*, not the anonymizer connection's RLS visibility;
nothing in the design threads `withTenantTx` context into `AnonymizerService`, and its per-subject `BEGIN…COMMIT`
structure cannot trivially inherit the worker's outer txn. Flip-gated (needs MIG-2 + NOBYPASSRLS), but this proposal
exists precisely to make the flip proceed on the claim that "the erasure path survives" — which it does not.

### N2 — HIGH — B-SEC · LC4-MIG's missing-ok `app.current_tenant` arm widens two legal-red-line tables to every tenant-context principal
**Violated invariant:** permissive-policy hygiene (proposal §2.2.6 — "either a tenant-key predicate OR role-restricted
+ on the firebreak allowlist"); least-privilege on PII/compliance tables.

LC4-MIG adds `OR location_id = NULLIF(current_setting('app.current_tenant',true),'')::uuid` to **both**
`gdpr_tenant_isolation` and `anonymization_audit_tenant_isolation` (`1780421100060:49-51,:57-59`). Both are **`FOR
ALL`** (SELECT + INSERT + UPDATE + DELETE) with USING **and** WITH CHECK, and the arm lands on **both** clauses.

`app.current_tenant` is not a worker-private GUC. It is set by the entire courier/webhook/worker lane: RC4 documents
"Couriers are not members → `app.current_tenant` from their verified active shift" (`1790000000077:44-45`); the
telegram webhook sets it (`telegram-webhook.ts:281`); the payments webhook sets it (`payments-webhook.ts:41`). So
post-flip, **any** principal running as `dowiz_app` with `app.current_tenant = X` gains full CRUD on:
- `gdpr_erasure_requests` at location X — which carries `subject_phone` (PII), `customer_id`, `reason`
  (`1780421100060:14-17`). A courier-lane request could read others' erasure requests, or DELETE them.
- `anonymization_audit_log` at location X — an **append-only compliance artifact** (`COMMENT … 'Append-only audit
  log'`, `:70`). The arm grants INSERT (forge audit rows) and DELETE (erase compliance history) to the courier/webhook
  principal class.

There is no way to scope a GUC-arm to "only the GDPR worker" — the worker uses `app.current_tenant` exactly as a
courier does. Today no courier/webhook *route* touches these tables, so the leak is latent — but the arm converts the
confidentiality/integrity guarantee from "enforced by member-only RLS" to "depends on no courier/webhook code path
ever querying these two tables," on a legal-red-line surface. Defense-in-depth downgrade on the most sensitive
tables in the schema, introduced by the v2 mechanism itself. Flip-gated (inert under BYPASSRLS).

### N3 — MEDIUM — B-FAIL · F1 residual: the failed-login `courier_audit_log` INSERT is not firebreak-covered → wrong-password login 500s post-flip
`courier/auth.ts:269-273` INSERTs `courier_audit_log (…'login.failed'…)` inside the login handler, which runs on
`db.connect()` with **no** `app.current_tenant` set (login is structurally pre-context — same reason F1 firebreaks
`couriers`). `courier_audit_log` carries the RC5 policy `isolate_courier_audit_log USING (location_id =
NULLIF(current_setting('app.current_tenant',true),'')::uuid)` (`1790000000077:71-73`); for a `FOR ALL` policy the
INSERT WITH CHECK inherits USING → `location_id = NULL` → **INSERT rejected (42501)** → the throw propagates out of the
login handler → **500 instead of 401** on every wrong-password courier login post-flip. MIG-1's firebreak covers only
`couriers`/`courier_sessions`/`courier_locations`, not `courier_audit_log`; P1b's "the key still turns" scope is the
*valid* login/session path and does not exercise the failed path, so this is invisible to the stated proof. Not a
lockout of valid logins (F1's core is fixed), but an availability/observability regression on the failure path, and a
member of the "FORCE-table readers the grep missed" that P10 must catch and P9 does not currently drive.

### N4 — MEDIUM — B-OPS · pg-boss 10.4.2 `updateQueue` silently resets `policy` to `standard` when the reconciler omits it
OPEN-V1 is answerable at HEAD (de-risking, good): `updateQueue` exists and changes policy on a live queue. **But** the
implementation is a footgun for the reconciler: `manager.updateQueue(name, options)` does `const { policy =
QUEUE_POLICIES.standard } = options` (`src/manager.js:573`) and passes it into `UPDATE ${schema}.queue SET policy =
COALESCE($2, policy)` (`src/plans.js:390`). Because the JS default fires when `policy` is absent, `$2` is **never
null** — so a reconciler call that intends to update only `retryLimit`/`retryDelay`/`deadLetter` but omits `policy`
will **COALESCE the stored policy back to `'standard'`**, silently turning off `singletonKey` dedup on a queue that
was `'short'`. The proposal's reconciler reads-stored-then-converges, so the intended path passes `policy` — but the
`QUEUE_POLICY`-map-driven reconciler and any future partial `updateQueue` call must pass the full policy every time or
regress dedup. The spec frames OPEN-V1 as "does `updateQueue` support policy change" (yes) and misses this "it also
*overwrites* policy on every call" edge, which P6 (single-queue dedup assert) would not catch if the reconciler is
called with a partial options object elsewhere.

### N5 — LOW — B-CONSIST · `withTenantTx({anonymous:true})` does not reset a session-leaked GUC — the "no-context is a decision" marker gives false assurance
The design sells the `{anonymous:true}` variant as "BEGIN with NO guc — makes 'no context' a decision, not an
accident" (`proposal §1.2`). But `BEGIN` does **not** clear a session-scoped GUC left by a prior borrower's
`set_config(k, v, false)` on the same pooled physical connection — that is the exact Shape-B leak the proposal itself
documents at `onboarding.ts:75` and `spa-proxy.ts:771` (`site-inventory §SESSION-LEAK`). An `{anonymous:true}`
transaction therefore *inherits* any surviving leaked `app.user_id`/`app.current_tenant` rather than transacting
context-free. Bounded — Lane 0 ships the anonymous variant, and the Shape-B fixes + `no-bare-set-config` lint precede
the flip, so the window (leak surviving AND enforcement on) is gated shut — but the marker asserts a property (no
context) it does not enforce (it sets nothing; it does not reset), so a stale-context accident upstream still bleeds
into an "anonymous" transaction. A `RESET`/explicit-NULL-set would be required to make the claim true.

### N6 — LOW — B-OPS · F12 registry watch-set cannot cover a never-started worker; coverage collapses to `/health` + one edge boot DRIFT
The v2 fix correctly kills the *cascade* amputation (per-registration isolation → later workers still start) and
correctly drives the liveness/reconciliation watch-set from the boot registry of **actually-started** workers. But a
worker whose **own** registration throws is, by construction, absent from the "actually-started" set → the
heartbeat-liveness reconciliation (`bootstrap/workers.ts:167-170`, watching the constructed heartbeat set) **cannot**
watch it (you cannot heartbeat-monitor a process that never started). The design signals it two other ways — one
**edge** boot DRIFT and a degraded `/health`. So detection of a never-started worker rests entirely on `/health`
being a **paged level-signal**, which is out of this proposal's scope (deferred to reliability H7). If H7 does not
page on `degraded`, a worker that fails registration is caught only by the single boot-time DRIFT; the reconciliation
liveness loop (the level detector for *running* workers) has a structural coverage seam for *never-run* workers.

### INFO-1 — B-SEC · Telegram chat-based tenant resolution is not *newly* spoofable, but the trust anchor is a shared URL secret
The v2 `resolve_telegram_chat(address)` uses `chatId = from.id` from the Telegram update as the principal — the same
anchor the current code uses (`telegram-webhook.ts:124,167-172`). The webhook is gated by a **shared, single-bot** URL
secret (`/webhook/telegram/${telegramBotSecret}`, `:36`) and an *optional* header check that **processes the request
even when the secret-token header is absent** (`:57-60`). Anyone who knows the bot's webhook URL secret can POST a
forged `from.id` = any owner's chat id and drive `order.*` actions for that owner's orders. This is a **pre-existing**
property, not introduced by v2 (v2 only reorders resolve-then-act), so it is INFO here — but the v2 redesign inherits
it, and the minor resolve→act TOCTOU (target deactivated between the DEFINER resolve and the guarded UPDATE txn) is a
new, low-impact window the guarded `updateOrderStatus` rowcount check bounds.

---

## Balancing notes (what holds under attack)
- **F1 firebreak genuinely turns the key.** Because login and session-validation run as bare `dowiz_app` (no
  `SET LOCAL ROLE`), `ops_all FOR ALL TO dowiz_app` + `courier_auth_read FOR SELECT TO dowiz_app` admit them post-flip.
  The couriers/`courier_sessions`/`courier_locations` triplet is the correct firebreak set. P1b tests it. Sound.
- **OPEN-V1 is de-risked at HEAD**, not just deferred: `pg-boss@10.4.2` ships `updateQueue` that updates the stored
  `policy` column, and `getQueues` to read current state — the reconciler is buildable without the `singletonSeconds`
  fallback (subject to N4).
- **F7 CAS is real exactly-once.** `FOR UPDATE SKIP LOCKED` + claim-token CAS on terminal writes genuinely yields one
  audit row / one event per request under double-trigger — the mechanism is correct (it just faithfully dedups N1's
  wrong outcome).
- **No deploy-time outage.** MIG-1..4 are inert on the BYPASSRLS main pool (confirmed `dowiz_app` = BYPASSRLS); every
  RLS break above is flip-time, behind the out-of-tree `ALTER ROLE … NOBYPASSRLS` + its named gates.

## The single most load-bearing finding
**N1.** The GDPR erasure worker wraps its bookkeeping in `withTenantTx({tenantId})`, but the actual data erasure runs
inside `AnonymizerService.anonymize`, which opens its **own** context-free pool connections
(`lib/anonymizer/index.ts:115,192`). `customers` has **no** `app.current_tenant` policy arm, so at the flipped
end-state (NOBYPASSRLS + MIG-2) the anonymizer's `SELECT … FOR UPDATE` on `customers` matches **0 rows**, the UPDATE
never runs, and the worker nonetheless writes `status='completed'` + an "erasure completed" audit row + a
`gdpr.erasure_completed` event "regardless of `result.skipped`." The result is a **silent, permanently-undetected
false completion of a legally-mandated Art.17 erasure** — worse than the LC4 bug it replaces, invisible to O-GDPR's
level-trigger (row is terminal-`completed`) and to P5 (which never asserts data-level erasure). It directly falsifies
the proposal's core F3/Lane-A claim that "the erasure path itself survives the flip." No fix proposed; the broken
invariant is *actual erasure occurs before `completed` is written, under enforcement*.
