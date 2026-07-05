# Reliability / Infra Audit — dowiz / DeliveryOS

**Date:** 2026-07-03  **Mode:** READ-ONLY (findings only, no code changed)
**Scope:** DB pools, boot sequence, pg-boss + workers, backup/restore + DR, migrations, caching, message bus / WS fan-out, health/readiness, graceful shutdown.

**Already-known, NOT re-reported:** `WORKER_BOOT_BUDGET_MS` 25s→3s; prod≠staging drift; deploy-health-timing crashout; CI preflight scripts; ledger dup rows 39–42.

**Severity counts:** CRITICAL 4 · HIGH 10 · MED 17 · LOW 13

**Systemic threads (each spawns multiple findings below):**
1. **pg-boss v10 runtime vs v12 types** (`packages/platform` pins `^10`; `apps/api` compiles against v12). In v10: `work()` ignores `singletonKey`; queues default to policy `standard` (so `singletonKey`-only dedup does **nothing**); handlers receive an **array**; job defaults `retryLimit=2, retryDelay=0, expire_in=15min`. `@ts-nocheck` across backup + resilience files hid the resulting arity/shape bugs.
2. **Transaction-scoped RLS GUC discipline applied inconsistently.** The canonical correct shape is `BEGIN` + `set_config(k, v, true)` (see `lib/notificationPrefsService.ts:35-37`, `lib/courier-room-authz.ts:46-47`, which explicitly warns a bare `set_config(...,true)` under autocommit dies). Two live handlers violate it in opposite directions (H4-rls, H6).
3. **Backup verification was written against an imagined artifact format, never the writer's real one** — every drill/verify layer fails or runs against the wrong DB.

---

## CRITICAL

### C1. Backup system cannot run in the deployed image — no `pg_dump`/`pg_restore`
- `Dockerfile:41-56` runtime stage is `node:22-slim`; the only runtime install is `npm install argon2 sharp @aws-sdk/client-s3 @aws-sdk/lib-storage`. **No `postgresql-client`.** (Verified.)
- `apps/api/src/workers/backup/dump.ts:34` spawns `pg_dump`; `backup-verify.ts:165` spawns `pg_restore`. Both `web` and `worker` processes run from this image (`fly.toml:17-19`).
- `BACKUP_ENABLED` defaults `'false'` (`packages/config/src/index.ts:82`), set nowhere in `fly.toml`.
- **Failure:** every scheduled backup fails `spawn pg_dump ENOENT` (or never registers). `docs/phase5/disaster-recovery.md:10` calls R2 logical backups "the sole recovery net" — the sole net is inoperable in its own environment.
- **Invariant:** a backup system must be executable in the environment it protects.
- **Fix:** install version-matched `postgresql-client` in the runtime stage; boot-probe `pg_dump --version` and FATAL when `BACKUP_ENABLED=true`.

### C2. The daily restore drill has never validated the format the writer produces (broken 3 ways)
- (a) **Reads a non-existent column.** `backup-verify.ts:92-95` selects `metadata->'encryption'->>'iv'` from `backup_metadata`, but that table has no `metadata` column (`packages/db/migrations/1780421100048_backup-metadata.ts:4-19`; no later migration adds one). The writer stores iv/authTag only in the R2 manifest (`workers/backup/index.ts:127-149`).
- (b) **Ciphertext hashed against plaintext checksum.** Writer stores sha256 of the **plaintext** dump (`index.ts:122`); the drill hashes the **encrypted** file (`backup-verify.ts:302`) and compares → guaranteed mismatch. Same in `r2-verify.ts:436-443`, so the `pg_restore --list` schema check at `r2-verify.ts:555` is never reached (`continue` at :552).
- (c) **Smoke-checks hit the live DB, not the sandbox.** `createSessionPool()` takes **zero args** and always connects to `DATABASE_URL_SESSION` (`packages/db/src/index.ts:48-50`); `backup-verify.ts:317` calls `createSessionPool(sandboxUrl)` — the arg is silently discarded (masked by `@ts-nocheck`).
- **Failure:** `disaster-recovery.md:11,28` claims a 24h restore-test verifies integrity; it can never pass — daily false alarms (→ alert fatigue) or never runs (C1).
- **Fix:** store encryption meta in `backup_metadata` (or read the R2 manifest in the drill); hash the decrypted file; remove `@ts-nocheck` from `workers/backup/`; parameterize the pool factory so the sandbox URL is honored.

### C3. Restore tooling depends on the database you just lost (chicken-and-egg)
- `scripts/backup-restore.ts:64-86` (`downloadManifest`) and `:168-185` (`--list`) resolve `r2_key` from `backup_metadata` via `createSessionPool()` → `DATABASE_URL_SESSION`.
- The emergency runbook (`docs/backup/runbooks.md:35-58`) provisions a fresh instance and sets only `BACKUP_ENCRYPTION_KEY`, `R2_*`, `DATABASE_URL_MIGRATIONS` (`DATABASE_URL_SESSION` isn't even listed), then says run `--list`. On the fresh instance `backup_metadata` is empty → `--list` shows nothing and `downloadManifest` throws "Backup not found". No "list/fetch directly from R2" mode exists.
- **Failure:** in the exact disaster the script exists for (primary DB lost), the operator cannot locate any snapshot within the 4h RTO.
- **Fix:** add `--list-r2` / `--r2-key=` modes that enumerate `dowiz-backups/<env>/**/*.manifest.json` directly from the bucket.

### C4. GDPR erasure requests get stuck `in_progress` forever after one failure
- `apps/api/src/workers/anonymizer-gdpr.ts:29,39,90-98` (verified): `run()` scans only `status='pending'` (line 29). Per-row it flips to `in_progress` (line 39); on failure the catch bumps `metadata.retryCount` and enqueues a retry — but the retry re-scans `status='pending'` and can never see the `in_progress` row. `retryCount` never reaches 3, so the "Max retries exceeded" terminal (line 100-105) is unreachable.
- **Failure:** one transient DB error during an erasure → the legally mandated deletion silently never completes and never surfaces as `failed`.
- **Invariant:** a GDPR request must terminally resolve (`completed`|`failed`).
- **Fix:** on retryable failure reset `status='pending'` (or have the scan include stale `in_progress`).

---

## HIGH

### H1. pg-boss `singletonKey`-only dedup is a NO-OP fleet-wide (queues are policy `standard`)
- v10 dedup on `singletonKey` alone requires queue policy `short`; every queue is created default `standard` (`migrations/1790000000011_pgboss-bootstrap-schema.ts:119`, `server.ts:252`, all worker `createQueue` calls). Only jobs that set `singletonSeconds` (e.g. `order.pending_aging`, `messaging.ts:43`) actually dedupe.
- **Blast radius:** duplicate `order.timeout_cancelled` owner notifications (sweep at `order-timeout-sweep.ts:101-108` + per-order handler at `apps/worker/src/handlers.ts:64-71` both rely on it); the courier-offer-sweep's "one in-flight COURIER_DISPATCH per order" (`courier-offer-sweep.ts:173-177`) is false → under backlog each 60s tick re-enqueues, `attempts` inflate (`courier-dispatch.ts:83-89`) → premature `dispatch_exhausted` (false "dispatch failed" owner alert + customer delay push while couriers exist).
- **Fix:** create the affected queues with `policy:'short'` (or add `singletonSeconds`) — one line per `createQueue`.

### H2. No retry/backoff/DLQ config anywhere; NotificationWorker's own dead-letter path is unreachable
- All queues run v10 defaults: 2 retries, **0s delay**, no backoff, no `deadLetter` (only `velocity.flush` sets `retryLimit:3`, `lib/signals/velocity-increment.ts:54`). A transiently failing telegram/push job is hammered twice within ms, then lands permanently in `failed` — the only detector is recon O3 at >10 failed/24h (`reconciliation.ts:232-248`).
- `notifications/workers/index.ts:534` re-throws with `Object.assign(err,{data:{attempt+1}})`; pg-boss ignores `err.data`, so the payload's `attempt` stays 0 and the `MAX_RETRIES=10` archive branch (index.ts:517-531) never fires.
- **Fix:** set `retryLimit/retryDelay/retryBackoff` + `deadLetter` per queue at creation; drop the `err.data` illusion.

### H3. One un-caught `createQueue`/`schedule` throw at boot amputates the rest of the worker fleet (recurrence of the 2026-06-21 incident shape)
- `bootstrap/workers.ts:49-185` is one sequential awaited chain. Most registrations are NOT `.catch`-wrapped (`settlement-cron.ts:25-26`, `courier-cron.ts:21-27`, `dwell-monitor.ts:25-26`, `order-timeout-sweep.ts:32-33`, `backup/index.ts:39-46`, `anonymizer-retention.ts:26-27`, `signal-raiser.ts:23-24`, `liveness-checker.ts:39-42`). A single throw aborts everything after it — including the heartbeats (line 111-125), liveness-checker, and reconciliation, i.e. the detectors themselves — leaving one `[API] worker startup error (continuing to listen)` line (`server.ts:348`). `access-request-retention.ts:23-26` names this exact failure class but the `.catch` fix was applied to only ~4 of ~20 sites.
- **Fix:** per-worker try/catch isolation in `startBackgroundWorkers`.

### H4-rls. Owner/customer status pushes silently die at the NOBYPASSRLS flip — `set_config(..., true)` outside a transaction is a no-op
- `notifications/workers/index.ts:122` (verified): `SELECT set_config('app.user_id',$1,true)` runs in autocommit; the transaction-local GUC dies with its own implicit txn, so the following `customer_devices` SELECT (line 124-129) runs context-free. Works today only because the role still has BYPASSRLS; after the staged Phase-1 flip (`migrations/1790000000077_rls-nobypassrls-phase1-policies.ts:107-109`, FORCE-RLS `customer_owns` on `customer_devices`) the query returns 0 rows → early return → every CONFIRMED/IN_DELIVERY/DELIVERED/CANCELLED push silently stops, no error.
- **Fix:** wrap in `BEGIN`/`COMMIT` (correct pattern is at `courier-events.ts:29-32`).

### H5-rls. Onboarding write path leaks a session GUC onto the pooled connection — `set_config(..., false)` with no transaction on the transaction-mode pool
- `routes/spa-proxy.ts:769-821` (verified): checks out `db.connect()`, runs `SELECT set_config('app.user_id',$1,false)` (**session-scoped**, `is_local=false`) with **no BEGIN/COMMIT**, then a series of writes, then `client.release()`. node-pg does not reset session state on release, so `app.user_id` persists on that physical backend after it returns to the pool. A later `pool.query` on that connection that forgets to set context inherits the prior owner's id.
- Today masked by BYPASSRLS; **at the NOBYPASSRLS flip this is a cross-tenant RLS read/write hazard.** Also unreliable on Supavisor transaction mode (:6543), which does not guarantee session continuity.
- **Fix:** wrap in `BEGIN`/`COMMIT` with `set_config(..., true)` (transaction-scoped), matching the canonical helper.

### H6. Backup advisory lock never released → backups silently skip after the first run
- `workers/backup/index.ts:209` (verified) calls `releaseLock(lockClient)` but the signature is `releaseLock(client, type)` (`:55-58`); `type=undefined` → `getLockKey(undefined)` unlocks `"backup_lock_undefined"`, never the real key (masked by `@ts-nocheck`). The session-level lock stays held on the pooled connection.
- Compounding: the lock is taken on the **operational** pool (transaction mode, `packages/db/src/index.ts:10-16`), whose own docs say advisory locks belong on the session pool. The identical leak was already documented+fixed 30 lines away in `backup-verify.ts:62-87` — but only for the verify path.
- **Failure:** subsequent hourly runs see `locked=false` → "another instance is handling… Skipping"; backups silently stop; with retention (M12) restore points then age out of R2. Data-recovery red-line.
- **Fix:** `this.releaseLock(lockClient, type)`; take the lock on the session pool (port a dedicated-client pattern from `backup-verify.ts:68-87`).

### H7. Health check lies — the `workers` aggregate is hardcoded `'ok'` regardless of heartbeat staleness
- `routes/health.ts:104-116` builds per-worker `status: … 'degraded'` entries from `ops_worker_heartbeat`, but line **291-292** hardcodes the aggregate: `workers: { status: 'ok' as const, entries: workerEntries }`. `hasDown`/`hasDegraded` (line 304-305) iterate the aggregates → a fully-dead worker fleet **never** degrades `/health`.
- Also `backup_restore` treats `never_run` as `status:'ok'` (line 236), so a system that has NEVER successfully restore-tested reports healthy.
- **Failure:** ops dashboards/uptime monitors stay green while every background worker is dead.
- **Fix:** derive the `workers` aggregate status from the entries (any stale → degraded); treat `never_run` restore as degraded.

### H8. Alert auto-resolution truncated — `boss.cancel('notify.dispatch.<alertId>')` always throws
- `workers/lifecycle-handlers.ts:60`: v10 `cancel(name,id)` requires an id and no queue `notify.dispatch.<uuid>` exists (jobs live on `notify.dispatch`). The first resolved alert row throws, the outer catch (line 71-73) aborts the loop, so remaining kinds never resolve and no `DWELL_ALERT_RESOLVED` publishes. The "F1: cancel pending escalation" invariant (relied on at `courier-offer-sweep.ts:258-259`) has never worked.
- **Fix:** drop the bogus payload-cancel (pg-boss can't cancel by payload); gate escalation delivery on alert state.

### H9. Full restore treats `pg_restore` exit code 1 as success — silent partial restore
- `scripts/backup-restore.ts:243-247`: `if (code === 0 || code === 1) resolve()`. Exit 1 covers real "errors ignored on restore" (missing extension/role/constraint), not just benign `--clean` skips. No `--single-transaction`, so an aborted restore leaves a half-dropped schema and still prints "✓ Restore completed" (`:252`).
- **Fix:** capture stderr, parse the "errors ignored" count, whitelist only "does not exist, skipping"; consider `--single-transaction`.

### H10. Restore permanently strips GRANTs, and the runbook ordering guarantees it
- Dump `--no-acl --no-owner` (`dump.ts:27-28`); restore `--clean --if-exists --no-owner --no-acl` (`backup-restore.ts:238`). `runbooks.md:40-57` says migrate (which applies grant migrations) **then** restore; `--clean` drops the just-migrated objects and recreates them grant-less. RLS policies survive; grants don't → operational/session roles can't read → app boots into permission-denied. No runbook step re-applies grants.
- **Fix:** add a post-restore re-grant step (re-run grant migrations or a `grants.sql`) to the runbook and to `runFullRestore`'s output.

---

## MEDIUM

### M1. pg-boss runs on the wrong connection in `apps/worker` (transaction pooler :6543)
- `apps/worker/src/index.ts:11` `new PgBossQueueProvider()` falls back to `DATABASE_URL_OPERATIONAL` unchanged (":6543 transaction pooler", `config/src/index.ts:7`). Only `apps/api` rewrites the port to 5432 (`server.ts:243-244`); the provider comment "Server.ts constructs the URL with port 5432" (`queue-provider.ts:22-23`) is false for this second consumer — which is the `order.timeout` **primary canceller** (`handlers.ts:15`). LISTEN/NOTIFY + advisory locks are unreliable through transaction mode.
- Separately, even in `apps/api` the session URL is hand-built by port-swapping the operational URL instead of using the dedicated `DATABASE_URL_SESSION` — fragile if host/creds differ.
- **Fix:** use `DATABASE_URL_SESSION` for pg-boss in both processes.

### M2. WS fan-out has no backpressure — a slow consumer can OOM the 512 MB web VM
- `websocket.ts` sends via `member.ws.send(payload)` (guard relay at :150, direct at :346/:379, GPS fan-out at :471/:485) with **no `ws.bufferedAmount` check** anywhere. A slow client on a high-frequency `order:` room (courier GPS pings) accumulates unbounded buffered data in the Node ws socket. `[[vm]] web memory=512mb` (`fly.toml`) + `auto_stop_machines=false` → a few stuck sockets balloon heap.
- **Fix:** skip/drop or disconnect when `bufferedAmount` exceeds a threshold.

### M3. `routing.ts` route cache is an unbounded `Map` (no eviction, no max size)
- `lib/routing.ts:39` `private readonly cache = new Map<string, CacheEntry>()`; `getLegRoute` (`:48-57`) sets entries with a TTL but only checks expiry on read — expired entries are never deleted and there is no size bound. Keyed by rounded coord legs; grows with distinct legs across tenants. Called per publish in `courier-events.ts:143`. (Prior SSR cache-OOM was fixed via `lru-cache` in `ssr-renderer.ts:48`; this one wasn't.)
- **Fix:** switch to `lru-cache` with `max` + `ttl`, matching `ssr-renderer`.

### M4. Dwell escalation is always delivered twice per target
- `workers/dwell-monitor.ts:127-142`: the tier-2 send is unconditional; the comment "(if the alert hasn't been resolved)" (line 135) has no matching check, and `handleDispatch` never checks alert state. Combined with H8 (cancel never works), the owner always gets the same `order.dwell_escalation` twice, 30s apart, including for already-resolved alerts.

### M5. v10 array-of-jobs hazard on direct `boss.work` handlers reading `job.data` (money path)
- `settlement-cron.ts:18-20`, `dwell-escalation.ts:20-22`: in v10 the handler gets `[job]`, so `job.data` is `undefined`. A queued `settlement.generate` carrying an explicit `referenceDate` silently degrades to `new Date()` → wrong period generated. Currently latent (cron passes null; the owner route calls `handleGenerate` directly). `access-request-notify.ts:36-42` normalizes correctly — apply that everywhere or route all through `PgBossQueueProvider.work`.

### M6. `DwellEscalationWorker` is dead code wired to a nonexistent queue
- `workers/dwell-escalation.ts:20`: never constructed in `bootstrap/workers.ts`; `QUEUE_NAMES.DWELL_ESCALATE` does not exist in `packages/shared-types/src/queue-names.ts`; nothing sends to it. The tiered push→telegram→SMS escalation ladder silently doesn't exist (M4's double-send is the only "escalation").

### M7. Accept-timeout sweep can resurrect an ended shift
- `courier-offer-sweep.ts:139`: `UPDATE courier_shifts SET status='available' WHERE id=$1` is unguarded, while shifts can be terminally `'offline'` with `ended_at` (`routes/courier/shifts.ts:146,230`). Scenario: assignment released elsewhere, courier ends shift, a stale `'assigned'` row expires → sweep flips the offline shift back to `'available'` → dispatch assigns to an off-duty courier → assign→timeout→assign livelock. Same unguarded class at `lib/bindingRelease.ts:29`, `dashboard.ts:264,282`.

### M8. Backup handler out-sleeps pg-boss's 15-min active-job expiration
- `workers/backup/index.ts:78,185`: in-handler retry sleeps total up to 21 min (1+5+15) while v10 default `expire_in` is 15 min; maintenance fails the job "by timeout in active state" and retries while the original is still running (only the broken lock, H6, prevents a concurrent second dump). Successful long backups get recorded as failed (false O3 drift).
- **Fix:** per-queue `expireInSeconds` sized to worst case; move retry waits to pg-boss `startAfter`.

### M9. LivenessChecker alert state is in-memory on a machine-roaming cron job
- `workers/liveness-checker.ts:25,63,103`: `previouslyStale` is per-process, but the 60s job is fetched by whichever machine wins. With ≥2 web machines a persistently-dead worker is re-alerted as "newly stale" and phantom `WORKER_RECOVERED` events fire. Alert-once needs DB state.

### M10. `assertAccessRequestSchedules` / `assertDeliveryTraceSchedule` swallow read errors, then pass
- `access-request-retention.ts:145-163` and `delivery-trace-retention.ts:66-81`: the boot assertion reads `pgboss.schedule`; on a **query error** it logs and continues with `present=∅` / `present=false`. In prod that path then hits the `process.exit(1)` fast-fail — good — but a transient read error at boot now hard-fails the deploy (false red), while the intended signal (schedule genuinely missing) is indistinguishable from a blip.
- **Fix:** distinguish "read failed" (warn, don't exit) from "schedule absent" (exit).

### M11. Retention deletes without a verified-successor guard; only `production` prefix has lifecycle
- R2 lifecycle expires hourly after 1 day / daily after 30 (`r2-verify.ts:406-411`). If the writer stalls silently (H6/C1), lifecycle keeps deleting; nothing enforces "keep last N verified." Rules only cover `dowiz-backups/production/*`, but the prefix is `${NODE_ENV}` (`index.ts:107`) → non-prod envs accumulate forever.

### M12. No environment guard on destructive restore beyond a flag
- `backup-restore.ts:203-214`: `--confirm` is the only gate — no prod-hostname allowlist, no typed target confirmation. A scripted `--confirm` with `DATABASE_URL_MIGRATIONS` pointed at prod drops prod objects. (Credit: no interactive prompt; creds redacted in logs.)
- **Fix:** require `--target-host` to match the URL, or refuse known-prod hosts without an extra explicit flag.

### M13. Backup key rotation is unrecoverable by design
- `encrypt.ts:91` hardcodes `keyId='primary'`; restore reads only `env.BACKUP_ENCRYPTION_KEY` (`backup-restore.ts:89`). After a documented rotation (`runbooks.md:69-83`) old manifests are indistinguishable from new; if the Fly app+secrets die with the DB, every backup is ciphertext with no recoverable key.
- **Fix:** real per-rotation keyId + a keyring lookup, stored in the SOPS/age vault.

### M14. Runbook / DR-doc claims don't match code
- `runbooks.md:32` dry-run "validates row counts, all steps must pass" — actual dry-run compares manifest counts to the **live** DB and only prints `⚠`, never fails (`backup-restore.ts:148-160`), and never tests pg_restore-ability. `disaster-recovery.md:70-77` routes verification through `pnpm backup:verify` — broken per C2.

### M15. Migration runner accepts out-of-order history
- `package.json:38` `migrate:up … --no-check-order`. Interleaved timestamps from two branches merge silently; evidence of a past collision workaround: `1790000000004_z_add_user_id_to_notification_targets.ts` (`z_` forces sort). Gaps at `…003/…029/…075-076`. The boot schema-guard only checks the expected head **name exists** (`schema-guard.ts:40-47`); holes/out-of-order pass.

### M16. Data-dependent migrations can fail or lose data on a non-conforming prod table
- `1790000000073_deliver-v2-cash-as-proof.ts:22-33` — two partial **unique** indexes on `courier_assignments` with no pre-dedupe → aborts at `release_command` if prod has 2 active rows for one order/courier (fail-safe, but blocks deploy).
- `1790000000019_add_categories_unique.ts:5-20` — DELETEs "duplicate" categories (keep max-id) **before** the unique index: irreversible in-migration data loss by heuristic.
- `1780421100060_anonymization-seam.ts:86` — `down()` does `ALTER COLUMN phone SET NOT NULL`, which fails once any anonymized (NULL-phone) row exists.
- Tenant-data baked into schema history (`…021_rename-slug-pizza-roma`, `…024_update-sushi-durres-settings`, `…045/046/056/057`) couples schema to one tenant's runtime data.

### M17. `pgboss.*` granted to `PUBLIC` — any DB role can forge money-path jobs
- `migrations/1790000000011_pgboss-bootstrap-schema.ts:127-147` `GRANT SELECT, INSERT, UPDATE, DELETE ON pgboss.* TO PUBLIC` (+ default privileges). Any role can read, forge, or delete jobs that drive `settlement.generate` / `courier.dispatch` and carry order/customer ids. (Straddles security; listed here for the job-integrity impact.)

---

## LOW

- **L1. Missing index `order_items(order_id)`** — FK at `1780310074262_orders.ts:52`, no index in any of 158 migrations; hit by `routes/orders.ts:762,796,821`, `customer/orders.ts:60`, `spa-proxy.ts:348`, plus CASCADE. Seq-scan-per-order-poll at growth.
- **L2. Timeout sweep has no covering index** — `app_sweep_timeout_orders` (`1790000000078:16-18`) filters `status='PENDING' AND timeout_at<now()`; only `orders(location_id,status)`/`(location_id,created_at)` exist. Add partial `ON orders(timeout_at) WHERE status='PENDING'`.
- **L3. Zero `CONCURRENTLY` in any migration** — fine now (node-pg-migrate wraps in a txn), a lock hazard for future index adds on hot tables.
- **L4. Backup size never recorded** — manifest `sizeBytes:0` hardcoded (`index.ts:132`); size-drift monitoring blind.
- **L5. "Forward-only" is policy, not tooling** — every migration has a `down()` (many `DROP … CASCADE`); `migrate:down` is wired (`package.json:39`).
- **L6. `anonymizer-gdpr` `FOR UPDATE SKIP LOCKED` is COMMITted before processing** (`:34`) — locks released immediately; two concurrent runs (route + retry across machines) double-process (anonymize itself is idempotent on `anonymized_at`, so bounded).
- **L7. Velocity buffer dropped on send failure** (`velocity-increment.ts:49-70`) — spliced before `boss.send`; a send failure loses anti-fraud events; a mid-batch retry re-INSERTs (no idempotency key) → inflated counters.
- **L8. `server.ts:236` 100 ms `setTimeout` as "LISTEN ready"** is a race, not a guarantee (subscriptions actually register at `bootstrap/workers.ts:155`).
- **L9. `apps/worker` `order.timeout` swallows all errors** (`handlers.ts:78-80`, log-only, no rethrow) → a failed cancel completes with zero retries; survivable only because the api-side sweep is the safety net. Also a raw cross-tenant `UPDATE orders` with no RLS context — silent no-op at the NOBYPASSRLS flip.
- **L10. `unhandledRejection`/`uncaughtException` keep-alive** (`server.ts:73-80`) — a genuinely wedged process keeps serving errors; only `/livez` (event-loop responsiveness, no DB) catches it. Intentional (avoids dropping all sockets on one floating rejection) but means a corrupted-but-responsive process is never restarted. Ensure Sentry alerting compensates.
- **L11. `pgboss_jobs_pending` gauge returns `NaN` on query error** (`server.ts:267-272`) — scrapes silently poison that series instead of erroring.
- **L12. Redundant double `queue.stop()` on shutdown** — `server.ts:870` (onClose) and `shutdown.ts:53-64` both call it; second call after a graceful stop may reject (caught, but noisy). Consolidate to one drain path.
- **L13. WS inbound `client_location` has no rate limit** (`websocket.ts:458-489`) — a customer client can spam location frames, each triggering a guarded courier fan-out (DB read is cache-bounded, but the send amplification is not).

---

## Verified sound (for calibration)
- Route-handler pool discipline: `connect()`/`release()`/`finally` counts balance in **every** route scanned — no client-leak of the 2026-06-20 starvation class in HTTP handlers.
- No named/cached prepared statements on the transaction-mode pool (grep clean) — the Supavisor `:6543` prepared-statement hazard is not triggered.
- Operational pool rejects a `postgres` superuser connection (`packages/db/src/index.ts:32-39`) and sets `statement_timeout` (10s op / 30s session).
- `/livez` (event-loop only, no DB) is the Fly probe; the heavy `/health` correctly 503s only on a **pg** down/timeout (`health.ts:39-79`) so it can't restart the machine on an external blip.
- Per-instance cron duplication is handled by pg-boss's own send-it dedup (`singletonSeconds:60`, works on any policy); heavy sweeps additionally hold distinct session advisory locks with unlock-in-finally.
- `PgMessageBus`: bounded reconnect with a single in-flight flag; NOTIFY payloads guarded against the 8000-byte cap with claim-check truncation; per-handler dispatch isolation prevents one bad subscriber crashing the process; WS room teardown unsubscribes exactly one handler (no fan-out multiplication).
- `order.timeout` enqueue is transactional with the order INSERT (`lib/order-persistence.ts:158-162`); settlement generation is atomic inside one SECURITY DEFINER function.
- `release_command` migrations-before-traffic (`fly.toml:8-15`) + `schema-guard` fail-fast backstop are well-designed.
- The retention/anonymizer workers activated by the recent grants are bounded (per-location `retention_days`, NULL = no-op; delivery-trace floored to the 7-day dispute window; access-request 12-month window behind an advisory lock).

---

### Two highest-leverage fixes
1. **C4** — one-line `status='pending'` reset restores GDPR liveness (legal red-line).
2. **H1** — create the affected queues with `policy:'short'` restores every `singletonKey` dedup layer the sweeps and notification paths were designed around.
