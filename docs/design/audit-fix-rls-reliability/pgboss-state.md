# pg-boss state fact sheet — verified 2026-07-03 (companion to proposal.md §4)

## 1. Version skew (confirmed real)

- `packages/platform/package.json:17` → `"pg-boss": "^10.1.5"` → **installed 10.4.2** — this package
  constructs the runtime `PgBoss` instance used by every consumer.
- `apps/api/package.json:44` → `"pg-boss": "^12.18.2"` → installed 12.18.3 — **types only**; the
  runtime object is v10. Acknowledged in-code: `apps/api/src/bootstrap/workers.ts:137-139` "pg-boss
  VERSION SKEW — queue.boss is the v10 instance … workers compile against pg-boss ^12 types", with
  `as unknown as PgBoss` casts at `:140,155,159,169`.
- `apps/worker/package.json` — no pg-boss entry (transitive via `@deliveryos/platform`).

## 2. Queue creation — 100% bare

Every `createQueue()` call in `apps/api/src`, `apps/worker/src`, `packages/platform/src` passes ONLY
the queue name: `server.ts:252` (loop over ALL_QUEUES), `settlement-cron.ts:25`, `dwell-monitor.ts:25`,
`liveness-checker.ts:39`, `courier-offer-sweep.ts:42,46`, `reconciliation.ts:36`,
`backup/index.ts:39-45`, `access-request-notify.ts:35`, `signal-raiser.ts:23`,
`access-request-retention.ts:36-37`, `acquisition-retention.ts:24`, `backup-verify-scheduled.ts:30,39`,
`delivery-trace-retention.ts:24`, `anonymizer-retention.ts:26`, `order-timeout-sweep.ts:32`,
`courier-cron.ts:21,26`.

Grep across all three trees: `policy:` **0** · `deadLetter` **0** · `retryBackoff` **0** ·
`expireInSeconds` **0** · `retryLimit` **1** (`lib/signals/velocity-increment.ts:54`, send-side
`retryLimit:3`) · `singletonSeconds` **1** (`bootstrap/messaging.ts:43`, 3600s — the only
actually-working dedup on `standard` queues). All `singletonKey`-only send/work sites (~20, incl.
`courier-offer-sweep.ts:176`, `order-timeout-sweep.ts:108`, `apps/worker/src/handlers.ts:71`,
`anonymizer-gdpr.ts:17`) are dedup no-ops on default `standard` queues → audit H1 confirmed.

## 3. Boot isolation — none in the bootstrap

`apps/api/src/bootstrap/workers.ts` (186 lines): ~23 sequential bare `await` registrations
(`:55-57,74-79,83,90,93,97,103-104,129,133,141,146,148,150,152,160,170,174,182`). The file's ONLY
try/catch (`:175,178`) wraps the FREE_TIER_WATCH **handler body**, not any registration. A throw in
any registration aborts everything after it — including liveness-checker (`:160`) and reconciliation
(`:170`), i.e. the detectors. Partial internal mitigations exist inside 4 workers only:
`access-request-retention.ts:46-55` (.catch on schedule; its own createQueue at :36-37 is NOT
wrapped), `reconciliation.ts:36-38`, `courier-offer-sweep.ts:46-47`, `server.ts:252-254` (.catch on
createQueue). Audit H3 confirmed; the fix belongs in the bootstrap, per-registration.

## 4. Dead-letter illusion (H2)

`apps/api/src/notifications/workers/index.ts:67` `MAX_RETRIES = 10`; `:513-534` reads
`job.data?.attempt`, archives at cap, else `throw Object.assign(err, { data: {...job.data, attempt:
attempts+1} })`. pg-boss does not propagate `err.data` into the retried job's `job.data` → `attempt`
stays 0 forever; the archive branch is unreachable. Real retry metadata is pg-boss's own
`job.retrycount` — unused.

## 5. Broken cancel (H8)

`apps/api/src/workers/lifecycle-handlers.ts:60` — `await this.boss.cancel(`notify.dispatch.${row.id}`)`
passes a composite string where v10 `cancel` expects a job UUID; throw is swallowed by the outer
catch (`:71-73`) which also aborts the remaining kinds loop.

## 6. Connection topology (M1)

`packages/platform/src/queue-provider.ts:20-32` — fallback `DATABASE_URL_OPERATIONAL` (":6543
transaction pooler"); `migrate:false`; `max:4`. `apps/api/src/server.ts:239-246` hand-rewrites the
port to 5432 before constructing the provider; `apps/worker/src/index.ts:11` passes NOTHING → the
`order.timeout` primary canceller runs pg-boss over the transaction pooler (LISTEN/NOTIFY-hostile).
Neither uses the dedicated `DATABASE_URL_SESSION`.

## 7. One good surprise

`queue-provider.ts:45-52` — the provider's `work()` wrapper DOES normalize the v10 array-of-jobs
shape (`Array.isArray(jobs) ? jobs : [jobs]`). The M5 array hazard therefore applies only to sites
that call `boss.work` **directly** (settlement-cron, dwell-escalation, lifecycle-handlers,
anonymizer-gdpr, etc.), not to `queue.work()` consumers.
