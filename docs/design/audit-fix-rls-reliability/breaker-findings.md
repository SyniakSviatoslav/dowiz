# Breaker findings — audit-fix-rls-reliability (Council STEP 2 · ATTACK)

- **Date:** 2026-07-03
- **Author:** system-breaker (Council STEP 2)
- **Target:** `proposal.md` + `ADR-audit-fix-rls-reliability.md` (+ fact sheets), verified against source at
  HEAD of `feat/phase0-safety-hardening`.
- **Method:** proved the FIX design breaks / regresses / locks out / hides a new bug. No fixes proposed.

## Ground-truth resolved first (kills two hypotheses, sharpens the rest)

- **The main request pool is `dowiz_app` = `LOGIN BYPASSRLS`** (`.github/workflows/ci.yml:101`;
  `docs/ops/PROD-UNBLOCK-RUNBOOK-2026-07-03.md:50`; wired via `createOperationalPool()` →
  `DATABASE_URL_OPERATIONAL`, `packages/db/src/index.ts:17-19`, decorated as `fastify.db`,
  `apps/api/src/server.ts:209-210`). Therefore **MIG-1..4 are genuinely inert on deploy** — there is
  **no deploy-time / "MIG-2 lands before the helper conversion" prod outage** on the request path
  (the proposal's inertness claim is correct). Every RLS break below is a **FLIP-time** break, gated
  behind the deferred out-of-tree `ALTER ROLE dowiz_app NOBYPASSRLS`. The dangerous flaws are in what
  the flip walks into, not in the deploy.
- **Answer to "B3 never flips → does anything degrade the current BYPASSRLS system?"** The migrations
  do not (inert). But the **helper conversions and the pg-boss changes take effect immediately,
  flip-independent**, and carry their own regressions today (F5, F6, F7, F8, F10, F11, F12).

Severity counts: **CRITICAL 2 · HIGH 3 · MEDIUM 4 · LOW 3** (+ 2 balancing notes).

---

## CRITICAL

### F1 — MIG-1 on `couriers` is unimplementable as written AND architecturally wrong → total courier-auth lockout at the flip
**Violated invariant:** "MIG-1 is inert/safe; verified by per-policy probe txns" — the probe proves the
lock, never the key (that the legitimate courier can still authenticate).

1. **The policy references a column that does not exist.** Proposal §2.2.3 / ADR decision #2 specify
   `couriers … tenant_isolation FOR ALL USING (location_id = <app.current_tenant GUC> OR location_id IN
   (SELECT app_member_location_ids()))`. **`couriers` has no `location_id` column** — verified
   `packages/db/migrations/1780421029538_couriers.ts:5-18` (columns: id, email_encrypted, email_hash,
   phone_*, full_name_encrypted, status, password_hash, timestamps, deactivated_by_owner_id). The
   courier↔location mapping lives in the **separate `courier_locations` join table** (`:21-28`; a
   courier is deliberately multi-location). The migration as written **fails to apply**
   (`column "location_id" does not exist`), or must be rewritten as a `couriers.id IN (SELECT
   courier_id FROM courier_locations WHERE location_id = <tenant>)` subquery — a different security model.

2. **Even rewritten, it locks out login.** Courier login reads
   `SELECT id, password_hash, status FROM couriers WHERE email_hash = $1 OR phone_hash = $1`
   (`apps/api/src/routes/courier/auth.ts:249`) on the main pool, in autocommit, **with no tenant
   context — and there can be none**: at login there is no session, no JWT, and the courier may belong
   to many locations. Post-flip any tenant-scoped RLS on `couriers` returns **0 rows → "Invalid
   credentials" for every courier → login impossible.**

3. **Session validation dies too.** Every courier-authenticated request runs
   `SELECT s.courier_id, s.revoked_at, s.expires_at, EXISTS(SELECT 1 FROM courier_locations cl …) FROM
   courier_sessions s WHERE s.id=$1 AND s.courier_id=$3` (`apps/api/src/plugins/auth.ts:74-83`) on the
   main pool, **no `app.current_tenant` set**. Post-MIG-1 (courier_sessions RLS) that returns 0 rows →
   `401 "Session revoked"`; and the `EXISTS` subquery hits `courier_locations`, whose existing policy is
   the **throw-on-unset dialect** `current_setting('app.current_tenant')::uuid`
   (`1780421029538:32`) → the whole query **throws → 500**. **Courier app fully bricked at the flip.**
   Note: `plugins/auth.ts:74` and `courier/auth.ts:249` are **not in the 49-site inventory** (they have
   no `set_config` to grep).

4. **Wrong bucket.** The proposal itself keeps `users`/`auth_refresh_tokens` as ENABLE-no-FORCE — the
   "B3 RC2 lockout firebreak" (`rls-state.md §4`, `1780421100065:69-73`) — because they are
   credential-lookup tables read *before* context exists. **`couriers`/`courier_sessions` are the exact
   same class** (password_hash, token_hash, read pre-context) yet are placed in the tenant-RLS set. This
   is not an ordering slip fixable by "convert first"; the login lookup is structurally pre-tenant.

**Why most dangerous:** billed as the safest, most-inert migration ("closes the credential-table
hole"), it silently arms a total courier-auth outage at the flip via reads that cannot carry tenant
context, and the migration may not even apply. P1 (isolation) would go green while the real courier
login/session paths are untested.

### F2 — The conversion inventory measures the wrong surface → flip-time fail-closed checkout/tracking, with the ordering guarantee unfalsifiable
**Violated invariant:** §6 hard edge "(5) public-lane helper conversion before MIG-2/3 — scoped anon
policies without the anon GUC = broken checkout." The guarantee is defined against a set that cannot
contain the paths that matter.

- The 49-site inventory is built from `grep set_config`. But the flip-safety surface is **every path
  that reads/writes a FORCE+anon-scoped table**, and the highest-value ones have **zero `set_config`
  today**, so they are structurally invisible to the inventory:
  - **Checkout INSERT** (`apps/api/src/routes/orders.ts` — `client.connect()`+`BEGIN` at :104/:111,
    **no `set_config` anywhere**; `INSERT INTO customers` :523; `insertOrderWithItems` :543 →
    `INSERT INTO orders/order_items/velocity_events/idempotency_keys/customer_track_grants`,
    `lib/order-persistence.ts:73-150`). Post-MIG-2 the anon INSERT WITH CHECK becomes
    `app_current_user() IS NULL AND location_id = <app.current_tenant>`; with the GUC unset →
    `location_id = NULL` → **WITH CHECK false → INSERT rejected → checkout 500 for ALL tenants.**
  - **Order tracking** (`apps/api/src/routes/customer/track.ts:43-53`, `SELECT … FROM
    customer_track_grants JOIN orders …`, no GUC — comment :15-16 "no tenant context … RLS bypassed").
    Post-MIG-2 the `orders` anon SELECT is scoped → JOIN yields 0 → **every track-link exchange 410s.**
- Because "public-lane conversion complete" is judged on the 49 grep-sites, an operator can believe the
  §6 precondition is satisfied and flip — into a fail-closed checkout/track. The proposal's hand-wave
  ("the storefront/public pool paths already resolve slug→location before querying; the helper
  conversion makes the GUC available") is not grounded: these paths resolve `locationId` but **never set
  a tenant GUC**, and are not tracked as conversion work.

**Systemic root of F2/F5/F9:** the inventory is `grep(set_config)`; flip-safety needs `every reader/
writer of a FORCE table`. Different, larger set.

---

## HIGH

### F3 — LC4's `withTenantTx({ tenantId })` does not satisfy the erasure table's RLS → GDPR erasure re-strands post-flip (fix's own compat claim is false)
**Violated invariant:** §3.5 "the worker's UPDATE/SELECTs run through `withTenantTx(pool, { tenantId:
row.location_id }, …)` so the erasure path itself survives the flip."

`gdpr_erasure_requests` RLS is **keyed on `app_member_location_ids()`**, i.e. `app.user_id`→memberships,
not `app.current_tenant`: `CREATE POLICY gdpr_tenant_isolation … USING (location_id IN (SELECT
app_member_location_ids())) WITH CHECK (…)` (`1780421100060_anonymization-seam.ts:49-51`). Setting
`app.current_tenant` (what `{ tenantId }` does) leaves `app.user_id` NULL → `app_member_location_ids()`
returns ∅ → **the worker's own `SELECT … WHERE status='pending'`, the `UPDATE … in_progress/completed`,
and the `INSERT anonymization_audit_log` (`:57-59` same key) all match 0 rows / fail WITH CHECK** under
NOBYPASSRLS. The GDPR worker is a system actor with no membership; the chosen GUC family is wrong. Net:
the legal-red-line liveness hole LC4 exists to close **reopens at the flip**, now silently (0 rows, green
queue) — the exact failure shape of the original bug.

### F4 — LC4 stale-reclaim depends on a non-existent `updated_at`; "no schema change" is false; the `requested_at` fallback causes false reclaims (concurrent double-trigger)
**Violated invariant:** §7 "LC4: worker-local logic + one recon check; **no schema change** (uses
existing status/metadata)."

`gdpr_erasure_requests` has **no `updated_at` column** (`1780421100060:12-24`), and the `in_progress`
flip (`anonymizer-gdpr.ts:39`) never stamps a mtime. §3.3's reclaim `WHERE in_progress AND updated_at <
now() - interval '15 minutes'` therefore **cannot be written without a schema change** (add column +
trigger) — contradicting §7 and the ADR "no schema change" rollback claim. If instead `requested_at` is
used, it is set once at creation and never moves: a row **legitimately being processed right now** but
requested >15 min ago (e.g. queued during a backlog, or a slow anonymize of a high-order customer) is
**indistinguishable from a stuck row** → the sweep reclaims it while another worker holds it → the very
"reclaims a row another worker is actively processing" hazard the attack targets.

### F5 — Converting the 3 telegram order handlers is not a "wrap"; it either holds a DB txn across external Telegram HTTP calls or leaves the catch-path context-free
**Violated invariant:** §1.5.2 "Shape-A sites: wrap the existing client block." The telegram "client
block" is not wrappable that way.

`handleCallbackQuery` (`telegram-webhook.ts:113-488`) opens one long-lived `client` and runs a whole
authz+action flow on it: multiple reads (`:156,167,194,213,240`), an **external network call**
`answerCallbackQuery` at `:267`, then the `switch` where `set_config('app.current_tenant',…,true)` +
`updateOrderStatus(client,…)` sit (`:281-282`, `:411-412`; `handleMessage` `openShift` at `:631-632`),
then **more external calls** `callTelegramApi`/`sendMessage`/`editMessageText` (`:329,357,449,464`).
- Wrapping "the client block" in `withTenantTx` puts a **BEGIN…COMMIT (and the pooled connection, and
  the order row locks taken by `updateOrderStatus`'s guarded `UPDATE orders … / UPDATE
  courier_assignments …`, `orderStatusService.ts:91-145`) across 2–4 `api.telegram.org` HTTP round
  trips.** On a `max:4` pool (`queue-provider.ts:99`; and the known public-menu pool-starvation
  incident) this is idle-in-transaction + lock-hold across the network — pool starvation and
  order-row contention **today, flip-independent.**
- Wrapping **only** `updateOrderStatus` in a separate `withTenantTx` opens a **second** connection, and
  the 409 catch-path re-read `SELECT status FROM orders WHERE id=$1 AND location_id=$2`
  (`:289-291,:419-422`) stays on the **old autocommit client with no GUC** → post-MIG-2 returns 0 →
  "order not found" instead of "already confirmed"/"cannot confirm state".
- **Tenant is discovered, not known:** the first read `SELECT location_id FROM orders WHERE id=$1`
  (`:156-159`) resolves the tenant *from* an unscoped order read. Post-MIG-2 that read itself 0-rows
  (no `app.current_tenant` possible before you know it) → the whole handler dead-ends at "order not
  found." The enumerated site `:281` is necessary but not sufficient; the real blocker (`:156`) is
  unaddressed.

---

## MEDIUM

### F6 — Converting the no-BEGIN multi-write provisioning flows to a single `withTenantTx` changes partial-failure atomicity (regression today, flip-independent)
`spa-proxy.ts:769-821` (`POST /api/owner/onboarding`) is a **fully autocommit multi-write** flow:
`set_config(…,false)` `:771`, `bootstrap_owner` `:776`, `UPDATE locations` `:785/790`, a **products
INSERT loop** `:798-808`, `INSERT location_themes` `:813`, no BEGIN/COMMIT. Today a failure on product
row 5 leaves rows 1–4 + the location committed (best-effort; response still `success:true` `:823`).
Wrapping it in one `withTenantTx` makes it **all-or-nothing**: a single bad product row (a constraint
the `ON CONFLICT DO NOTHING` doesn't cover, an oversize field) **rolls back the entire location
provisioning → 500, no storefront** where today the owner got a storefront minus one item. This is
precisely the "handler that relied on autocommit semantics" the attack asked to enumerate; it applies to
the multi-write members of the "13 sites with no BEGIN" (spa-proxy onboarding; and the two-phase
`onboarding.ts:55-129`, whose menu-seed already has its own BEGIN and whose split into "sequential
`withTenantTx` calls" orphans a committed `bootstrap_owner` on any seed failure — same as today, but now
advertised as atomic).

### F7 — LC4 double-trigger (reset-to-pending + targeted retry + periodic scan) duplicates the audit/notification side effects even though customer data is safe
Customer-row anonymization is serialized (`anonymizeCustomer` takes `SELECT … FOR UPDATE` then skips if
`anonymized_at` set — `lib/anonymizer/index.ts`), so data is not double-anonymized. **But the GDPR
worker around it does not check `result.skipped`:** it unconditionally writes `status='completed'`,
`INSERT anonymization_audit_log`, and publishes `gdpr.erasure_completed` (`anonymizer-gdpr.ts:62-83`).
The fix creates **two concurrent triggers for the same row** — the periodic `status='pending'` scan
(whose `FOR UPDATE SKIP LOCKED` is released by the immediate `COMMIT` at `:34`, so it guards nothing
during processing) and the targeted `job.data` retry — so one anonymizes and the other skips-but-still
writes: **duplicate rows in the append-only `anonymization_audit_log` (a compliance artifact),
duplicate dashboard erasure_completed events, and a redundant `completed` write.** "Idempotent on
anonymized_at" defends the data, not the audit/notification layer.

### F8 — `policy:'short'` on already-created `standard` queues may be a no-op in prod → P6 goes green on a fresh test DB while prod dedup stays broken
pg-boss stores a queue's policy in `pgboss.queue` at create time; `createQueue` is effectively
create-if-absent. Every prod queue already exists as `standard` (`pgboss-state.md §2`; `server.ts:252`
loops `createQueue(qName)` bare). Re-invoking `createQueueWithDefaults(name,{policy:'short'})`
(`queue-provider.ts:70-86`) against an **existing** queue is not guaranteed to ALTER its stored policy.
P6 ("send twice with same singletonKey, assert 1 job") is written against a freshly-created `short`
queue and passes; the **deployed** queue keeps deduping as a no-op. Classic false-green: the proof
exercises a different object than production. (Also unverified: whether flipping a live queue's policy is
even a supported online operation, or requires drop/recreate — a data-loss hazard for queued jobs.)

### F9 — The notify-worker conversion covers only the devices read; the order read is left context-free and breaks post-MIG-2 (and P2 can false-green)
§1.5.2 wraps `notifications/workers/index.ts:105-131` in one `withTenantTx({ userId:
order.customer_id })` and asserts "the order read at :108-115 stays explicit-predicate; it needs no
GUC." The order read must run **before** the wrap (it supplies `customer_id`). Under BYPASSRLS→pre-MIG-2
it works (anon fail-open). **Post-MIG-2** the `orders` anon SELECT requires `app.current_tenant`; the
order read has neither GUC → 0 rows → early `return` at `:116` → **no push** — the same silent-death
outcome the fix targets, relocated from the devices read to the order read. This handler genuinely needs
**two** GUC families (`app.current_tenant` for the order, `app.user_id` for the devices). P2 as
specified ("assert push adapter invoked on a NOBYPASSRLS probe") **passes if seeded before MIG-2 and
fails after** — the proof's validity depends on ordering the proposal doesn't pin.

---

## LOW

### F10 — Dead-letter naming mismatch: helper emits `${name}.dlq`, proposal/monitor say `.dead` → the generic monitor watches nothing
`deadLetterQueueName()` returns `${name}.dlq` (`queue-provider.ts:52-54`), but §3.4/§4.2/§4.3 name the
DLQ `anonymizer.gdpr.dead` / route a generic `*.dead` monitor. Two conventions → the `*.dead` monitor
never sees the `.dlq` sinks the helper actually creates → exhausted GDPR/other jobs land in a DLQ nobody
consumes → the "failed GDPR row is an ops page" guarantee (§3.4b) silently fails.

### F11 — "Pin types to v10 + remove `@ts-nocheck`" is not zero-risk: it surfaces the real M5 handler-signature bug (build-red, scope creep)
The direct `boss.work` sites are written v12-style single-job (`anonymizer-gdpr.ts:17` `async (job) =>
run(job)`; also settlement-cron, dwell, lifecycle-handlers per `pgboss-state.md §7`), and are currently
masked by `@ts-nocheck`. v10's `work` handler receives an **array** of jobs. Removing `@ts-nocheck` under
v10 types (proposal §4.1, "zero runtime risk") turns M5 from latent into a **red typecheck across every
direct-work site**, which must then be fixed as real code in the same lane — not a config-only pin. The
"zero runtime risk" framing understates that these sites are already runtime-wrong for the v10 array
shape.

### F12 — "Register detectors FIRST" conflicts with reconciliation's need for the full worker set → false DRIFT / startup race
§4.4 says detectors (heartbeats, liveness, reconciliation) register first so they survive later
failures. But `ReconciliationWorker`'s A6 liveness check is explicitly built to watch "the true set of 8
heartbeating ids" known only after all heartbeat workers are constructed (`bootstrap/workers.ts:160-171`
+ comment). Registering it before those workers exist risks it computing an incomplete/empty watch set,
or flagging not-yet-registered workers as dead → **false DRIFT alerts at every boot.** The isolation
wrapper is sound; the ordering prescription fights an existing dependency.

---

## Balancing notes (not findings — steel-manning what holds)

- **Settlements conversion is clean.** `courier/settlements.ts:12-91` is read-only (SELECT
  `courier_payouts`/`settlement_items`, no `FOR UPDATE`); wrapping the pool.query pair in a single
  `withTenantTx({ tenantId: activeLocationId })` under READ COMMITTED changes neither snapshot
  visibility (each statement still re-snapshots) nor locking, and is **required** for correctness
  post-flip (else the payout SELECT runs on a different connection than the GUC → 0-row payouts). No
  regression from the fix here; the "money read" is safe to convert. Contrast with F5.
- **No deploy-time outage.** As established up top, MIG-1..4 are inert on the BYPASSRLS main pool; the
  "MIG-2 deployed before the helper conversion → outage" scenario the attack hypothesized **does not
  fire on deploy** — it fires only at the deferred flip. The proposal's inertness claim is accurate.

## The single most dangerous flaw

**F1.** MIG-1 places `couriers`/`courier_sessions` in the tenant-RLS set. The `couriers` policy
references a non-existent `location_id` column (may not apply at all), and — however rewritten — the
login-by-email read (`courier/auth.ts:249`) and the per-request session-validation read
(`plugins/auth.ts:74`, plus the throw-on-unset `courier_locations` EXISTS) **cannot carry a tenant GUC**
because they run before any tenant/session context exists. At the flip this is a **total courier-auth
lockout**, and it is invisible to the proposal's proofs: P1 asserts cross-tenant isolation (the lock),
never that a legitimate courier can still log in and validate (the key). These are exactly the tables the
proposal already firebreaks for `users`/`auth_refresh_tokens` — the same reasoning was not applied to
their courier twins. No migration-ordering discipline fixes it; the RLS model itself is wrong for
pre-context credential lookups.
