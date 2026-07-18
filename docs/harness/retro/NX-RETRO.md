# NX Audit Retrospective — Telegram Notification Fixes

> **Session:** 2026-06-09 to 2026-06-12 · **Commits:** `385138d` → `bccfd32`
> **Scope:** Fix Telegram notification delivery broken by pg-boss v10 migration, connection topology errors, incomplete event wiring, and callback authorization.

## 1. Issue Matrix (20 issues found & fixed)

| # | Issue | Commit | Root Cause | Category | TS Catch? | Test Catch? |
|---|---|---|---|---|---|---|
| 1 | Missing `locale` in PUT notification target schema | `63e32c5` | Zod schema didn't include `locale` despite DB having the column | **API/Schema gap** | No | Yes |
| 2 | pg-boss connected to wrong pool (session vs operational) | `d5eef9c` | Hardcoded `DATABASE_URL_SESSION` instead of `DATABASE_URL_OPERATIONAL` | **Topology** | No | Yes |
| 3 | pg-boss defaulted to `public` schema | `19fd31b` | No explicit `schema` option passed to PgBoss constructor | **Security** | No | Yes |
| 4 | Runtime role had DDL privileges on `pgboss` schema | `19fd31b` | pg-boss `migrate: true` auto-creates tables; role had CREATE on public | **Security** | No | Yes |
| 5 | Worker callback signature wrong for pg-boss v10 | `19fd31b` | v10 changed `work()` to pass `Job[]` array instead of single `Job` | **Library API drift** | Partial | Yes |
| 6 | MessageBus used transaction pooler (LISTEN/NOTIFY broken) | `19fd31b` | Operational pool (6543) doesn't support LISTEN/NOTIFY | **Topology** | No | Yes |
| 7 | LISTEN commands issued before listener client connected | `19fd31b` | `subscribe()` had `if (listenerClient)` guard but no replay queue | **Ordering** | No | Yes |
| 8 | No dedup on notification events → duplicate messages | `19fd31b` | Missing `singletonKey` in pg-boss `send()` | **Resilience** | No | Yes |
| 9 | 6 of 10 queues not explicitly created before use | `19fd31b` | Assumed auto-migration would create queues | **Pre-flight** | No | Yes |
| 10 | Default privileges didn't cover existing pg-boss tables | `19fd31b` | `ALTER DEFAULT PRIVILEGES` only applies to future tables | **Permission** | No | Yes |
| 11 | `boss` field on `PgBossQueueProvider` was `private` | `19fd31b` | Visibility modifier prevented external access | **TS/API** | **Yes** | N/A |
| 12 | `velocity.flush` worker used wrong `work()` call | `19fd31b` | Inconsistency — missed when switching to new wrapper | **Library API drift** | Partial | Yes |
| 13 | Secret-token validation rejected missing header | `bccfd32` | Strict validation didn't account for pre-existing webhook configs | **Backward compat** | No | Yes |
| 14 | `answerCallbackQuery` called after processing → loading spinner | `bccfd32` | Telegram best-practice ordering reversed | **UX timing** | No | Manual |
| 15 | No follow-up message for CONFIRMED/REJECTED | `bccfd32` | Only edited original message, didn't send new confirmation | **Incomplete wiring** | No | Yes |
| 16 | Duplicated inline Telegram API calls (3+ copies) | `bccfd32` | No shared helper — raw fetch scattered everywhere | **Code quality** | No | No |
| 17 | Poor/Ukrainian-only error messages on status conflicts | `bccfd32` | Hardcoded strings without locale support; no granular categories | **i18n** | No | Yes |
| 18 | Schema revocation too strict — broke pg-boss startup | `19fd31b` | `REVOKE CREATE ON SCHEMA pgboss` was too aggressive | **Permission** | No | Yes |
| 19 | `RedisMessageBus` used with PostgreSQL pool | `19fd31b` | Class name mismatch (likely renamed but import stale) | **Topology** | Partial | Yes |
| 20 | Permissive test assertions not flagged `expect([200,400,500]).toContain(x)` | `6ac20e3` | No lint rule forbidding the pattern | **Testing** | No | No |

### Stats
- **20 issues total** across 5 commits
- **1 catchable by TypeScript** (#11 — private field)
- **2 partially catchable** by TypeScript with stricter types (#5, #12)
- **17 catchable by a targeted test**
- **2 not test-catchable** (#14 UX timing, #16 code duplication)
- **1 caught by lint rule added** (#20)

## 2. Error Pattern Catalog

### Pattern A — Schema-Query Mismatch (#1)
Writing SQL queries that reference columns/types that don't exist in the actual database schema.
- **Example:** `short_id` referenced in query but column doesn't exist
- **Fix:** Query `information_schema.columns` or read the migration file before writing JOINs
- **Rule:** Before writing any SQL JOIN or column reference, verify columns exist

### Pattern B — Library API Drift (#5, #12, #20)
Assuming the installed library's API matches documentation or prior experience without checking the actual version.
- **Example:** pg-boss v10 changed `work((job: Job) => ...)` to `work((jobs: Job[]) => ...)`
- **Example:** ESLint API changed from `createEslintRule` to direct `RuleCreator` calls
- **Fix:** Read the actual type declarations of the installed version before using new APIs

### Pattern C — Incomplete Event Wiring (#15, plus locale/render gaps)
Adding a new event flow but missing links in the chain.
- **Chain:** publisher → subscriber → handler → locale strings → render case → type union
- **Example:** `order.confirmed` was published but subscribers, locale messages, render cases all missing
- **Fix:** Verify every link in the event chain when wiring a new event

### Pattern D — Connection Lifecycle Leak (#7)
Creating new connections without properly releasing old ones.
- **Example:** `PgMessageBus.connect()` didn't release old client before creating new one
- **Fix:** Every `.connect()` must have a matching `.close()`/`.release()` at the same scope level

### Pattern E — Resilience Gap (#8)
Inter-component communication without rate-limiting, circuit breaking, idempotency, or dead-letter.
- **Example:** Notification jobs had no dedup key → duplicate Telegram messages
- **Fix:** Every pg-boss `send()` should include `singletonKey`; every notification channel needs rate-limiter + circuit breaker

### Pattern F — Backward Compat Blindspot (#13)
Adding strict validation that breaks existing producers who haven't updated.
- **Example:** `x-telegram-bot-api-secret-token` header required but some webhooks registered before `secret_token`
- **Fix:** Start lenient, log warnings, add strict validation only after verifying all producers are updated

### Pattern G — Topology Ignorance (#2, #6, #19)
Assuming all database connections are equivalent.
- **Example:** Transaction pooler (6543) vs session pooler (5432) — LISTEN/NOTIFY works only on session
- **Fix:** Know which pool type each connection uses; document the port map

### Pattern H — Permission Assumption (#4, #10, #18)
Assuming the runtime role has privileges it doesn't (or shouldn't).
- **Example:** `ALTER DEFAULT PRIVILEGES` only covers tables created by that user — existing tables owned by `postgres` were missed
- **Example:** Revoking all DDL broke pg-boss which needs CREATE on `pgboss` schema
- **Fix:** Verify privileges with `has_schema_privilege()` / `has_table_privilege()` before depending on them

### Pattern I — Missing Pre-flight Check (#9, #13)
Not verifying infrastructure exists before depending on it.
- **Example:** Queue names used without calling `createQueue()` first
- **Example:** Webhook assumed to have `secret_token` without checking with Telegram
- **Fix:** Verify external dependencies at startup; fail fast if they don't exist

### Pattern J — Code Duplication (#16)
Repeating the same API call pattern instead of creating a shared helper.
- **Example:** 3+ inline `fetch` calls to Telegram API with inconsistent error handling
- **Fix:** When a third repeat of a pattern appears, extract a helper immediately

## 3. Test Gap Analysis

### What tests exist now

| Test | What it covers | Issues detected by |
|---|---|---|
| `test-stage36.ts` T-1 — NX durability | pg-boss jobs survive app restart | — |
| `test-stage36.ts` T-2 — NX off-critical-path | Notification failure doesn't block order | — |
| `test-stage36.ts` T-3 — NX topology/privileges | Runtime role has correct privileges | #4, #10, #18 |
| `test-stage36.ts` T-4 — NX idempotency | Duplicate events produce 1 job | #8 |
| `verify-nx-flow.ts` | Full chain: order → pg-boss → audit | — |
| `e2e/tests/telegram-test.spec.ts` | Webhook handler responds correctly | #13 |

### What tests are missing (should be added)

| Missing test | What it would detect | Priority |
|---|---|---|
| **Schema-query integrity test** — verifies SQL column references against `information_schema.columns` | #1 pattern (schema-query mismatch) | Medium |
| **Library API compatibility test** — verifies installed pg-boss version's actual API | #5, #12 pattern (library API drift) | High |
| **Event wiring completeness test** — for each event type, verify all chain links exist | #15 pattern (incomplete wiring) | High |
| **Connection lifecycle test** — verify `.connect()` has matching `.close()` for all paths | #7 pattern (lifecycle leak) | Low |
| **Queue pre-creation test** — verify all queues exist before workers start | #9 pattern (pre-flight) | Medium |
| **Telegram webhook backward-compat test** — call without secret token, expect 200 | #13 pattern (backward compat) | Medium |
| **Follow-up message delivery test** — verify CONFIRMED/REJECTED produces a new message | #15 | Medium |
| **Duplicate API call detection** — lint rule for inline `fetch` patterns | #16 pattern (code duplication) | Low |
| **Status conflict message test** — confirm twice, verify correct message | #17 | Low |
| **Cross-role message delivery test** — owner A confirms, verify owner B gets notification | #8 (cross-user delivery) | Medium |

### Why existing tests didn't catch these

| Reason | Count | Examples |
|---|---|---|
| No test existed for the code path | 14 | #1-#10, #12, #15, #17, #19 |
| Test existed but passed due to permissive assertion | 1 | #20 (`expect(status).toContain([200,400,500])`) |
| Not functionally testable with current tooling | 2 | #14 (Telegram loading indicator), #16 (code duplication) |
| Stale test fixture didn't match schema | 1 | #17 (test used old order data without new status) |
| Test didn't cover the topology variation | 2 | #2, #6 (tests used dev env, not Supabase topology) |

## 4. Rules Extracted

### If we had these rules, these issues would have been prevented

| Rule | Prevents | How |
|---|---|---|
| **Schema-first verification** — before writing SQL JOINs, `SELECT column_name FROM information_schema.columns WHERE table_name = ?` | #1 | Catches column name mismatches before deployment |
| **Library API pinning** — for each third-party dependency, verify the installed version's API before using it (read `node_modules/pkg/package.json` + type decls) | #5, #12 | Catches v10 API drift before writing `work()` calls |
| **Event wiring checklist** — when adding a new event type, verify: publisher → subscriber → handler → locale → render → type union (grep for each link) | #15 | Catches missing links before deployment |
| **Connection lifecycle audit** — every `.connect()` must have a matching `.close()`/`.release()`. Grep for orphaned connections. | #7 | Catches connection leaks |
| **Resilience-by-default for IPC** — every pg-boss send must include `singletonKey`. Every notification channel needs rate-limiter + circuit breaker. | #8 | Prevents duplicate/silent failures |
| **Backward compat first for webhooks** — start lenient (log warning), add strict validation only after 24h of telemetry | #13 | Prevents breaking existing producers |
| **Topology documentation** — maintain a port map: which port, which pool type, what operations each supports | #2, #6, #19 | Prevents wrong-pool errors |
| **Runtime privilege verification** — test `has_schema_privilege()` and `has_table_privilege()` at startup | #4, #10, #18 | Catches privilege gaps |
| **Infrastructure pre-flight** — verify external dependencies (queues, webhooks, secrets) exist at startup | #9, #13 | Fail fast vs silently broken |
| **Don't repeat API calls** — if a third `fetch(url, {...})` to the same host appears, extract a helper | #16 | Prevents inconsistent error handling |

### Why TypeScript alone is insufficient

Only 1 of 20 issues (#11) was clearly catchable by TypeScript. The rest were:
- **Runtime topology** (6 bugs): wrong pool, wrong schema, wrong port — invisible to types
- **Configuration** (4 bugs): missing env vars, wrong schema names — stringly-typed
- **Logic ordering** (3 bugs): calling things in wrong order — valid TS but wrong behavior
- **External API semantics** (3 bugs): Telegram best practices, pg-boss API changes — types don't capture behavioral contracts
- **Privileges/permissions** (3 bugs): DB grants — invisible to application code

TypeScript catches type errors. These were **system errors** — they require runtime verification, integration tests, and environmental probes.
