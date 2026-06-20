# RESOLVE — Go/No-Go Pre-Launch Remediation

> One entry per finding: **fix** / **accept-risk(+owner)** / **defer-flag**.
> Every revision grounded against the live working tree (paths + line numbers cited).
> Invariants held: forward-only, RLS `ENABLE`+`FORCE`, integer-money, JWT RS256, no new schema.
> Companion: `proposal.md` (updated in place), `docs/adr/ADR-golive-remediation.md`.

`breaker-findings.md` is absent in the tree; the five enumerated findings in the resolve brief ARE the breaker matrix and are treated as such. Counsel ETHICAL-STOP-1/-2 are resolved under #5 / #4.

---

## Ground-truth verification (done before any design change)

| Claim under test | Verdict | Evidence |
|---|---|---|
| `deliveryos_api_user` has `BYPASSRLS` | **TRUE** | `packages/db/migrations/1780691681296_ops-location-alerts-policy.ts:8` — `ALTER ROLE deliveryos_api_user BYPASSRLS` |
| `orders` RLS keys on `app.user_id` (via `app_member_location_ids()`), NOT `app.current_tenant` | **TRUE** | policy `packages/db/migrations/1780310074262_orders.ts:82-84` `USING ( location_id IN (SELECT app_member_location_ids()) )`; function `1780310071220_core-identity.ts:76-79` selects memberships `WHERE user_id = app_current_user()`; `app_current_user()` reads `app.user_id` (`:70-72`). **No policy in the tree reads `app.current_tenant`.** |
| The "set `app.current_tenant`" fix for the worker is a no-op | **TRUE** | nothing keys on that GUC; setting it changes no row visibility |
| A future `NOBYPASSRLS` flip would break the sweep | **TRUE** | the background timeout job has no member `user_id` → `app_member_location_ids()` returns empty SETOF → policy matches zero rows → `UPDATE … WHERE status='PENDING'` silently affects 0 rows |
| `health.ts` timeout → `degraded` unconditionally | **TRUE** | `apps/api/src/routes/health.ts:31` `if (err.message === 'timeout' || treatErrorAsDegraded)` returns `degraded` regardless of the flag |
| `orders.ts:376` "FOR UPDATE … authoritative lock" comment | **FALSE / lying** | query `:378-382` is a plain `SELECT … WHERE id = ANY($1) AND location_id = $2` — **no `FOR UPDATE`** |
| `menu_version` is folded into the idempotency key | **TRUE** | `orders.ts:194` `menu_version: menuVersion` inside `canonicalBody`; `:197` `requestHash = sha256(canonicalBody)` |
| products/modifiers reads already set-based; only modifier-**groups** loop is per-item | **TRUE** | products `:378-382` `= ANY($1)`, modifiers `:410-416` `= ANY($1)`; the per-item round-trip is the `modifier_groups` query inside the `for (item of items)` loop at `:430-436` |
| `isDevMode()` has exactly one consumer | **TRUE** | defined `CheckoutPage.tsx:42`, used only at `:439`. The `dos_dev`/`?dev=true` checks in `CourierRoutes.tsx:24` / `AdminRoutes.tsx:57` are *separate* dev gates, not `isDevMode()` consumers, and their feature mounts (`FlowTestPage`) are already compile-gated behind `import.meta.env.DEV` (`AdminRoutes.tsx:22,79,258`). |
| Retention sweep is actually scheduled & observable | **TRUE** | `apps/api/src/workers/anonymizer-retention.ts:22-27` registers `boss.work` + `boss.schedule(ANONYMIZER_RETENTION_CRON, default '0 3 * * *')`; worker id `anonymizer-retention` is in `CRITICAL_WORKERS` (`liveness-checker.ts:11`) → heartbeat-watched; every anonymize writes `anonymization_audit_log` (`anonymizer/index.ts:285-289`) |
| `boss.createQueue` works at runtime (the "no new queues" non-goal is too strong) | **TRUE** | `anonymizer-retention.ts:26` and `reconciliation.ts:35` both call `this.boss.createQueue(...)` successfully at runtime. Migration 009 revoked **schema-level DDL** on `pgboss`, but pg-boss's own managed `createQueue` path is permitted. The cancel-reconciliation sweep therefore does **not** require a forbidden raw `CREATE`. |
| anonymizer = anonymize-not-delete | **TRUE** | `anonymizer/index.ts:133-141` phone→`'anon_'||uuid`, name→NULL, marketing_opt_in→false, row kept; `:210-217` order delivery_address→NULL, client_ip_hash→NULL, row kept |
| `owner/gdpr.ts` has no customer self-service erasure | **TRUE (per counsel grounding)** | every route `requireRole(['owner'])`; no anonymous/customer route |

---

## #1 — Order auto-cancel reliability  ·  **FIX** (worker handler + machine-routing + reconciliation sweep, NOT deferred)

**Severity: CRITICAL. Disposition: FIX.** The earlier "set `app.current_tenant`" proposal is **DROPPED entirely** — it is a verified no-op (it keys on a GUC no policy reads) and worse, it would create a false sense that the worker is RLS-correct, masking the real exposure: the sweep only works because `deliveryos_api_user` carries `BYPASSRLS`. A future `NOBYPASSRLS` flip (the stated security direction) would silently zero-match the UPDATE, leaving timed-out orders stuck PENDING with no error.

Resolution, all forward-only, no schema change:

- **(a) Keep the existing consumer** at `apps/worker/src/handlers.ts:15` (`queue.work(QUEUE_NAMES.ORDER_TIMEOUT, …)`). It is a real deployed process (`fly.toml` worker VM). No new worker is written.
- **(b) Add the missing `order.timeout_cancelled` notification emit.** Enqueue `QUEUE_NAMES.NOTIFY_TELEGRAM_SEND` with `event:'order.timeout_cancelled'`, mirroring the `order.created` outbox at `orders.ts:660-668`. The event is fully wired downstream (`event-registry.ts:59`, render/locales/notification-worker) — only the emit is missing. **REVISED by RESOLVE round 2 / C4:** do NOT gate the emit on the emitter's own `rowCount > 0` (that made the recovery sweep silent on the handler-crash case → zero notification). Key the emit on *observing* CANCELLED-by-timeout, and make it durable via `INSERT … ON CONFLICT (event,target_id) DO NOTHING` backed by a new partial UNIQUE (C4a / proposal §5). `singletonKey: order.timeout_cancelled:{orderId}` kept as secondary in-flight dedup. Post-COMMIT → see rewritten R2.
- **(c) ~~Route the cancel through the state machine via `assertTransition`.~~ SUPERSEDED by RESOLVE round 2 / H2.** This round-1 sub-point is **WRONG against the tree**: `apps/worker` cannot import `@deliveryos/domain` (`apps/worker/package.json:11-16` lacks the dep), so the worker cannot call `assertTransition`. The corrected position (see "H2" in RESOLVE round 2 below): the **guarded `WHERE status='PENDING'` UPDATE IS the transition authority** for this one fixed `PENDING→CANCELLED` edge; no domain import is added. The audit row (`order_status_history`, actor `system:timeout`, `handlers.ts:40-43`) is still kept.
- **(d) Keep the guarded `WHERE status='PENDING'`** (`handlers.ts:26-30`) for idempotency / race-safety: at-least-once redelivery, or a manual confirm landing first, makes the UPDATE a `rowCount 0` no-op. **REVISED by H2:** the WHERE is not merely a "fence" complementing an `assertTransition` authority — it **IS** the transition authority (no domain import); see (c) and the H2 section.
- **(e) ADD the reconciliation sweep as the actual safety net (NOT deferred).** A short-interval scheduled job:
  `UPDATE orders SET status='CANCELLED', timeout_at=NULL WHERE status='PENDING' AND timeout_at < now() RETURNING id, location_id`
  — re-detecting orders the per-order `order.timeout` job missed (lost job, dead consumer, enqueue gap). It reuses the SAME guarded UPDATE SQL contract + conflict-keyed notification emit as the handler (single *specified* contract, test-enforced — not a shared import, per H2). **REVISED by RESOLVE round 2 / C3 + H1:** mount it as its **own pg-boss schedule in `apps/api`** (beside the live `NotificationWorker`, `server.ts:347,352-354`) at 1-minute cadence — the "OR as a sub-check on the reconciliation worker" alternative is **STRUCK**: `ReconciliationWorker` is removed in prod (`server.ts:382`). NOT the nightly `0 3 * * *` slot. Backed by the new partial index `orders_pending_timeout_idx ON orders(timeout_at) WHERE status='PENDING'` (H1 — the `(status,timeout_at)` index claimed here did NOT exist). Runs under `deliveryos_api_user`/`BYPASSRLS` today; future `NOBYPASSRLS` flip re-validated at R4.
- **Lost-job detectability < 1 min — `pgboss.job` THROUGHPUT query, NOT the heartbeat (REVISED by RESOLVE round 2 / C2).** The round-1 phrasing "add to the heartbeat-watched set" is **unimplementable**: the worker heartbeat is keyed by `FLY_MACHINE_ID` with `job_name` NULL (`heartbeat.ts:9,28-31`) and `CRITICAL_WORKERS` matches by string name (`liveness-checker.ts:11`) — it can never bind `order.timeout`. Detection is the pg-boss query (SELECT-able under the runtime role, `reconciliation.ts:237-246`): `SELECT count(*) FROM pgboss.job WHERE name='order.timeout' AND state IN ('created','active') AND start_after < now()` (overdue count) + a completed-delta over the window; alert when `overdue > 0` for > 60s. The `ops_worker_heartbeat` row stays as coarse VM-liveness only. The sweep is *recovery*; this query is *detection*.

**Hard Go gate (live-verify):** place an order → let it time out (or set a short `confirm_timeout_min`) → assert (i) status flips PENDING→CANCELLED, (ii) `order_status_history` row with actor `system:timeout`, (iii) the `order.timeout_cancelled` notification fires (owner Telegram), (iv) with the per-order consumer artificially stopped, the **reconciliation sweep** still cancels it within its cadence. SQL spot-check + E2E on the target env. **No Go without this green.**

---

## #2 — DB pool / transaction hold  ·  **FIX** (batch reads, keep them in-tx; bound with statement_timeout)

**Severity: HIGH (idempotency + money). Disposition: FIX — revised.** The original "move reads outside `BEGIN…COMMIT`" is **DROPPED**: it breaks two correctness invariants.

1. **Idempotency.** `menu_version` is folded into `request_hash` (`orders.ts:194,197`), the idempotency key. Reading `menu_version` outside the tx opens a window where the menu version observed for the hash differs from the version the order is written against → two logically-different carts could collide (or fail to collide) on the key. The key's integrity requires the read to share the order's snapshot.
2. **Money snapshot.** The price snapshot's integrity rests on the **in-transaction MVCC snapshot**, not a lock — the `:376` "FOR UPDATE … authoritative lock" comment is **false** (the query has no lock). Moving the price read out drops it to a *different* snapshot than the INSERT, re-introducing exactly the TOCTOU the in-tx read avoids. Integer-money is byte-identical either way, but the *snapshot coherence* is the invariant.

**Revised fix — keep all reads inside the transaction; shorten the connection-hold by cutting round-trips, not by relocating reads:**

- **(a) Batch the per-item reads.** Products (`:378-382`) and modifiers (`:410-416`) are **already** set-based (`= ANY($1)`). The remaining per-item round-trip is the `modifier_groups` query inside the item loop at `:430-436` (one query per item). Replace it with a single set-based query over all `product_id`s (one `modifier_groups JOIN product_modifier_groups WHERE pmg.product_id = ANY($1)`), then partition the result by product in memory. This collapses ~N round-trips to ~1. Net: the held-tx round-trip count drops from ~(7 writes + N group-reads) to ~(7 writes + 1 group-read).
- **(b) Bound worst-case hold.** The operational pool already sets `SET statement_timeout='10s'` per connection (`packages/db/src/index.ts:30`). ADD a tighter per-transaction `SET LOCAL statement_timeout` (≈4s) at `BEGIN` of the write path so a wedged write self-aborts well inside `connectionTimeoutMillis=5000` (`index.ts:22`) — a wedge becomes one fast 5xx, not a pool-wide stall. Failure-first lever.
- **(c) Optional real lock + fix the lie.** If a hard guarantee against in-window availability flips is wanted, add a genuine `SELECT id FROM products WHERE id = ANY($1) AND location_id=$2 FOR SHARE` at the top of the write tx (one round-trip) and **correct the `:376` comment** to describe what the code actually does. Default: rely on the MVCC snapshot (no lock) + the existing `is_available` re-check (`:386-388`); fix the comment regardless — a lying lock comment is a latent trap.

**Pool note unchanged:** keep `max: 8` (`index.ts:20`). Win via fewer round-trips, not more connections. A bump would spend the scarce Supavisor/free-tier budget and make a wedge *bigger*.

---

## #3 — Health truthfulness  ·  **FIX** (pg timeout → DOWN; stop serializing pg internals)

**Severity: LOW. Disposition: FIX — precise.** A hung/unreachable Postgres (the live failure: connection-timeout, not a fast error) currently yields `degraded` + HTTP 200, so external monitors stay green during a real outage.

- **Exact change — make the pg check specifically classify a timeout as `down`.** The generic `withTimeout` (`health.ts:15-40`) must NOT change for the soft checks (telegram/r2/messageBus keep `degraded` on failure so a blip never 503s the app and never triggers a Fly restart — the `/livez` vs `/health` split, `health.ts:54-56`, is preserved). The narrow change: the **pg** call must treat a timeout as `down`. Either pass an explicit `treatTimeoutAsDown` flag to the pg invocation (`:60-63`) and branch on it before the `err.message==='timeout'` short-circuit at `:31`, or give the pg check its own classifier that maps both timeout and fast-error to `down`. Once pg yields `down`, the existing overall-status logic (the `pgResult.status === 'down' → 503` path at `:64-70`) fires correctly — no change there.
- **Stop serializing raw pg result internals.** `withTimeout` stuffs `data: result` (`:28`) — the node-postgres result object (rows incl. `oid`/`dataTypeID`/type-catalog) — into the response, and `/health` is unauthenticated (`config:{rateLimit:false}`, no auth). Drop `data` from the public payload; the public `/health` returns only `{ status, timestamp, checks: { <name>: { status, latencyMs } } }`. Rich internals (worker entries, free-tier %) move behind auth or to a separate authenticated `/health/detail`.

**Verifiable:** E2E asserts pg-unreachable → 503 + `status:"unhealthy"`, and the body contains no `oid`/`dataTypeID`/raw-row keys.

---

## #4 — Checkout fallback + dev-mock containment  ·  **FIX** (compile-time gate + full consumer grep)

**Severity: HIGH. Disposition: FIX.** Today `isDevMode()` (`CheckoutPage.tsx:42`) is `sessionStorage.getItem('dos_dev')==='1'` — a flag a real session can carry — and at `:439` a *failed real order* runs `clearCart(); navigate('…/o_mock_123')`, showing fake success on a real failure. (Counsel ETHICAL-STOP-2.)

- **Gate with build-time `import.meta.env.DEV`** (statically `false` in a prod build → Vite dead-strips the branch and the `o_mock_123` nav cannot exist in the prod bundle). Combine with the existing sessionStorage flag: `if (import.meta.env.DEV && isDevMode()) { … }`. This is a compile-time boundary, not a runtime one — matching the existing pattern at `main.tsx:92`, `AdminRoutes.tsx:22,79,258`.
- **Grep-all consumers — DONE, recorded for the implementer.** `isDevMode()` has exactly ONE consumer: `CheckoutPage.tsx:439` (defn `:42`). No other file calls it. The separate `dos_dev`/`?dev=true` gates in `CourierRoutes.tsx:24` and `AdminRoutes.tsx:57` are *different* code paths; their feature mounts are already compile-gated behind `import.meta.env.DEV`. **Implementer assertion:** re-grep `isDevMode` and `o_mock_123` across `apps/web/src` before merge and confirm no path reaches a prod success-spoof; the audit shows none today.
- **Restaurant-phone fallback** on the `catch` (`CheckoutPage.tsx:437-450`): in addition to the toast, surface a "call the restaurant" CTA using `location.phone` from the existing public `GET /api/public/locations/:slug/fallback-config` (`routes/public/fallback-config.ts`), shown when `showPhoneOnError !== false`. The fallback fetch itself fail-soft to the generic toast. **Cart preserved** — `clearCart()` only runs on success (`:431`); the fallback must NOT clear it (and the dev-mock `clearCart()` at `:439` is dead-stripped in prod by the gate).

**Verifiable:** E2E against a **prod build** — force order failure → phone CTA visible, NO `o_mock_123` nav, cart intact.

---

## #5 — Privacy notice  ·  **FIX** (anonymize-not-delete copy + contact-the-restaurant + retention-sweep Go gate)  ·  resolves ETHICAL-STOP-1

**Severity: ETHICAL-STOP (human-approved direction). Disposition: FIX.** The notice is only honest if its words match what the code does.

- **(a) "Remove the data that identifies you" / anonymize-not-delete — NOT "delete everything".** Grounded: `anonymizer/index.ts` keeps the row; customer phone→`'anon_'||uuid`, name→NULL, marketing_opt_in→false (`:133-141`); order delivery_address→NULL, client_ip_hash→NULL (`:210-217`); avatar purged from storage (`:144-160`). The sq/en copy MUST say *"we remove the details that identify you"* (sq: e.g. *"heqim të dhënat që ju identifikojnë"*) — never *"fshijmë gjithçka" / "we delete everything"*, which the system does not do. A human reads the final sq string against `anonymizeCustomer`'s actual field list before Go.
- **(b) "Contact the restaurant to remove" — NO self-service button.** `owner/gdpr.ts` is owner-only; there is no customer erasure endpoint. The copy MUST read "to remove your data, contact the restaurant" (reuse `location.phone`, already surfaced for #4) and MUST NOT imply a "click to delete" the customer can press. Dignity at pilot scale = truth about the mechanism, not a fabricated power. (Counsel N1: post-pilot, add a customer-initiated `gdpr_erasure_request`; out of launch scope.)
- **(c) Retention number is honest only if the sweep runs — ADD a hard Go gate.** The notice renders `retention_days` ("deri në {{days}} ditë") as a removal promise. Verified: the retention sweep **is** scheduled (`anonymizer-retention.ts:22-27`, cron default `0 3 * * *`), is a `CRITICAL_WORKERS` member (`liveness-checker.ts:11`) → heartbeat-watched, and every run writes `anonymization_audit_log` (`anonymizer/index.ts:285-289`). **Go gate:** verify in the *target env* that the `anonymizer-retention` schedule exists AND has fired (an `anonymization_audit_log` row with `scope='retention'` in-window, or an observed heartbeat for `anonymizer-retention`). Add this signal to the §9 operability table, mirroring #1's timeout-consumer gate. **If the gate cannot be made green for launch**, soften the copy per counsel ETHICAL-STOP-1(b) — "we keep your data only as long as needed for your orders and remove identifying details on request" — so the displayed number stops being a hard promise. Architect default: gate green (a) — the pulse already exists; do not ship an honest-looking number the runtime doesn't enforce.

**Data source:** `locations.retention_days` (`1780421100060_anonymization-seam.ts`, default 365), surfaced via the public location/menu payload. No new column. **Verifiable:** E2E asserts the notice block is visible near submit on `/s/:slug` checkout, the retention number renders, and the copy contains the anonymize-not-delete + contact-the-restaurant wording (string assertion).

### #5 data-task — scrub proof
ADD a verification assertion to §10-DATA: after load, `SELECT phone FROM customers/locations WHERE slug='demo'` on **staging** MUST NOT equal the prod `sushi-durres` phone (the scrub at §10-DATA.2 must be *proven*, not assumed), **in addition to** the existing `customers=0` / `orders=0` checks. (Slug corrected to `sushi-durres` per breaker C1 — see RESOLVE round 2.) Three assertions gate the data task: `customers=0`, `orders=0`, `staging demo.phone ≠ prod sushi-durres phone`.

---

## #6 — ESM `__dirname` fix  ·  **FIX** (unchanged)

`e2e/tests/flow-ui-admin-branding.spec.ts` uses `path.resolve(__dirname, …)`; `__dirname` is undefined under ESM → Playwright fails at *collection*, killing the whole suite. Derive dirname from `import.meta.url` via `fileURLToPath`+`dirname`. One file, no behavior change. **Verifiable:** `playwright test --list` collects clean.

---

## Residual accepted-risks (with owners)

| # | Risk | Disposition | Owner |
|---|---|---|---|
| R1 | #2 in-window availability flip: a product goes unavailable between the in-tx read and COMMIT. Mitigated — same MVCC snapshot + `is_available` re-check (`orders.ts:386`); optional `FOR SHARE`. (Smaller than before: reads are NOT moved out, so the only window is intra-tx.) | **Accepted** at pilot scale; re-evaluate at 10× volume | System Architect |
| R2 | #1 `order.timeout_cancelled` emit is post-COMMIT (not same-tx as the UPDATE). A worker crash between COMMIT and enqueue loses the owner notification. | **Accepted** — non-critical; dashboard already updated via bus (`handlers.ts:51-56`); the reconciliation sweep (#1e) re-detects and re-emits | System Architect |
| R4 | #1 sweep + handler currently run under `deliveryos_api_user`/`BYPASSRLS`. A future `NOBYPASSRLS` flip would zero-match the UPDATE (no member `user_id`). | **Accepted now + flagged** — documented as the explicit re-validation point; when `NOBYPASSRLS` is pursued, the timeout sweep must move to a SECURITY DEFINER function or an explicit ops policy. NOT silently fixed via `app.current_tenant` (no-op). | Owner/Architect |
| R5 | #2 `max:8` unchanged — a pathological burst beyond pool capacity could still pressure connections | **Accepted** for launch; observable via `free_tier_snapshots.connections_pct`; scaling-gate documented | System Architect |
| R6 | Data task: staging `demo` drifts from prod `sushi-durres` over time | **Accepted** — point-in-time snapshot, refreshable by re-running the loader | Owner |
| R7 | #4 compile-time `import.meta.env.DEV` gate removes the dev-mock convenience from prod-like sessions | **Accepted** — dev-mock is dev-only; compile-time is the correct boundary | System Architect |
| R8 | #5(c): the retention promise now *creates a standing obligation* — if the sweep silently dies post-launch the notice becomes a false statement to every customer | **Accepted + observable** — covered by the §9 sweep-pulse signal (audit-log count / heartbeat); same shape as #1 detectability | Owner/Architect |
| N5 | Counsel open question: the **courier** holds a stranger's name/phone/address/door-photo on-device. No copy/policy governs the courier's holding. | **Deferred — post-pilot, flagged** | Owner |

(R3 from the old proposal — "queue-registration relies on migrate-past-0011" — is **closed**: the per-order job uses the transactional outbox at `orders.ts:650-654`; #1's reconciliation sweep (#1e) makes the system robust to a lost job regardless, so the old "must verify queue registered or promote Option B" risk is subsumed by the now-mandatory sweep.)

---

## Threat-model items to carry into implementation as test assertions

1. **#1 auto-cancel end-to-end (Go gate):** order times out → PENDING→CANCELLED + `order_status_history` actor `system:timeout` + `order.timeout_cancelled` notification fires. Assert all four sub-conditions.
2. **#1 lost-job recovery:** with the per-order consumer stopped, the reconciliation sweep cancels an overdue PENDING order within its cadence. Assert the overdue-job throughput metric goes >0 then back to 0.
3. **#1 RLS regression guard:** a test that asserts, under `deliveryos_api_user`, the sweep UPDATE matches the overdue row — AND a documented note that flipping to `NOBYPASSRLS` requires the SECURITY-DEFINER path (R4), so the flip can't ship without re-greening this test.
4. **#2 idempotency:** identical POST /orders (same cart, same `menu_version`) collides on `request_hash` and returns the prior order (23505 path inside the tx). Assert one order row, two 200s.
5. **#2 money snapshot coherence:** price snapshot on `order_items` equals the in-tx product price; batching does not change the snapshotted values.
6. **#2 hold bound:** a wedged write self-aborts via `SET LOCAL statement_timeout` before `connectionTimeoutMillis`, returning a 5xx for that one order without cross-tenant 500s on menu/`/health`.
7. **#3 health truth:** pg-unreachable → 503 + `status:"unhealthy"`; soft-check failure → still 200/`degraded`; body has no `oid`/`dataTypeID`/raw-row keys.
8. **#4 no false success:** prod build, forced failure → no `o_mock_123` nav, cart intact, phone CTA shown when `showPhoneOnError !== false`.
9. **#4 dead-strip proof:** grep the prod bundle for `o_mock_123` → absent.
10. **#5 copy truth:** rendered notice asserts anonymize-not-delete wording + "contact the restaurant" + retention number; never "delete everything".
11. **#5 retention pulse (Go gate):** target-env `anonymization_audit_log` has a `scope='retention'` row in-window OR `anonymizer-retention` heartbeat is fresh.
12. **#5 scrub proof:** staging `demo.phone ≠` prod `sushi-durres.phone`; `customers=0`; `orders=0`.
13. **#6 collection:** `playwright test --list` collects without throwing.

---

## RESOLVE раунд 2 — breaker C1–L2

> System Breaker delivered `breaker-findings.md` (C1–C4, H1–H4, M1–M3, L1–L2) — proven breaks, each cited file:line, re-verified here against the live tree before disposition. These are not opinions; the round-1 resolution above is amended where they land. Disposition per finding: **fix** (proposal/ADR updated in place) / **accept-risk** (+owner, into the risk table) / **defer-flag** (MISSING, out of launch-scope). Invariants held throughout: forward-only, RLS `ENABLE`+`FORCE`, integer-money, JWT RS256.
>
> Headline correction the breaker forced: two of round-1's load-bearing claims were FALSE against the tree — "no schema change" (H1: the sweep needs an index) and "single transition authority / no logic fork" (H2: `apps/worker` cannot import `@deliveryos/domain`). Both are corrected below honestly rather than defended.

### Re-verification (live tree, round 2)

| Breaker claim under test | Verdict | Evidence (re-checked) |
|---|---|---|
| Prod slug is `sushi-durres`, not `durres-sushi` | **TRUE** | `packages/db/migrations/1790000000021_rename-slug-pizza-roma-to-sushi-durres.ts:5` `UPDATE locations SET slug = 'sushi-durres' WHERE slug='pizza-roma'`. Grep of source for `durres-sushi` = empty. The entire round-1 doc named it backwards. |
| Worker heartbeat has no per-queue identity | **TRUE** | `apps/worker/src/heartbeat.ts:9` `workerId = process.env.FLY_MACHINE_ID \|\| 'local-worker'`; `:28-31` INSERTs `(worker_id, last_seen_at)` only — `job_name` never written. One row per VM, keyed by machine id. |
| `CRITICAL_WORKERS` matches by name | **TRUE** | `apps/api/src/workers/liveness-checker.ts:11` `'dispatcher,settlement-cron,dwell-monitor,anonymizer-retention'` split by `,` — string-name match. `'order.timeout'` would never appear as a `worker_id`. |
| ReconciliationWorker is NOT running in prod | **TRUE** | `apps/api/src/server.ts:382` `// Nightly Reconciliation Worker — temporarily removed (esbuild bundle issue). Re-add in separate deploy.` No `new ReconciliationWorker` anywhere except the file's own doc comment. O1 (stale PENDING) + O3 (failed jobs, `reconciliation.ts:237-253`) are therefore dead in prod too. |
| NotificationWorker runs in `apps/api` | **TRUE** | `server.ts:347,352-354` registers `NOTIFY_*` workers in the API process. |
| Notification dedup is in-memory, audit index non-unique | **TRUE** | `apps/api/src/notifications/workers/index.ts:57` `private dedupCache = new Set<string>()` (process-local, max 1000); `1790000000007_notification-outbox-audit.ts:31` `pgm.createIndex(..., ['event','target_id'])` — **plain index, no UNIQUE**. `ON CONFLICT DO NOTHING` has nothing to conflict on. |
| `(status, timeout_at)` index does not exist | **TRUE** | `1780310074262_orders.ts:44-45` — only `orders(location_id, status)` and `orders(location_id, created_at DESC)`. `timeout_at` (`:38`) is a bare column. Cross-tenant `WHERE status='PENDING' AND timeout_at<now()` cannot use the location-leading index. |
| `apps/worker` cannot import the domain guard | **TRUE** | `apps/worker/package.json:11-16` deps = `@deliveryos/{config,db,platform,shared-types}` — **no `@deliveryos/domain`**. `assertTransition` lives in `packages/domain/src/order-machine.ts`. |
| `pgboss.job` is SELECT-able by the runtime role for an overdue query | **TRUE** | `reconciliation.ts:237-246` already runs `SELECT name,count(*) FROM pgboss.job WHERE state='failed' …` under the runtime role — the throughput/overdue SELECT is permitted. |
| H4: only MIN_ORDER is special-cased; 5xx falls to generic toast | **TRUE** | `CheckoutPage.tsx:440` branches only on `err?.status===422 && code==='MIN_ORDER_NOT_MET'`; everything else → generic toast `:448`. No phone CTA on 5xx/network today. |
| fallback_config is owner-authored free-form JSON | **TRUE** | `routes/public/fallback-config.ts:17,25,28` returns `config.phone` from `locations.fallback_config` JSON — can carry an owner-typed personal phone/name. |
| Fly health check reads `/livez`, not `/health` | **TRUE (per round-1 grounding)** | `fly.toml:21` path `/livez`. A `/health` 503 is invisible to Fly. |
| `/health` down/degraded `detail` leaks driver message | **TRUE** | `health.ts:35` `detail: \`${label} check timed out or failed: ${err.message}\``, `:38` `detail: \`${label}: ${err.message}\`` — raw pg driver text on the unauthenticated endpoint. |

---

### C1 — prod slug is `sushi-durres`, not `durres-sushi` · **FIX** (critical)

Round-1 named the source tenant `durres-sushi` everywhere (proposal §2/§10-DATA, ADR Decision 7, resolution #5 data-task). The live slug is **`sushi-durres`** (`1790000000021:5`); `durres-sushi` exists nowhere in source. With the wrong name every read in the data task — `GET /s/durres-sushi`, `/api/public/theme/durres-sushi`, `/api/public/locations/durres-sushi/fallback-config` — returns 404 (`fallback-config.ts:22` 404s on `rowCount===0`), the loader produces an empty `demo`, and the §10-DATA verification fails not on a bug but on a typo.

**Fix:** corrected to `sushi-durres` everywhere — `proposal.md` §2, §10, §10-DATA.1-3, R6; `ADR` Decision 7 + Consequences; this resolution (#5 data-task, threat-model #12, R6 above). No design changes — only the target name. Grounded against `1790000000021:5`.

### C2 — lost-job detection cannot ride the name-keyed heartbeat · **FIX** (critical, redesigns the detection mechanism)

Round-1 #1 and proposal §7 said "add `order.timeout` to the heartbeat-watched set." This is **unimplementable**: the worker writes one heartbeat row per VM keyed by `FLY_MACHINE_ID` with `job_name` NULL (`heartbeat.ts:9,28-31`), and `CRITICAL_WORKERS` matches by string name (`liveness-checker.ts:11`). No row will ever carry `worker_id='order.timeout'`, so adding it yields a permanent false-positive (always "dead") or is silently ignored. The heartbeat proves "a machine breathed," never "the timeout queue drained" — exactly the gap the design claimed to close.

**Fix — detection #1 is rebuilt on a `pgboss.job` throughput query, NOT on heartbeat-name.** The breaker confirmed this SELECT is permitted under the runtime role (`reconciliation.ts:237-246` already queries `pgboss.job`). The lost-job signal becomes:
```
SELECT count(*) FROM pgboss.job
WHERE name = 'order.timeout' AND state IN ('created','active') AND start_after < now()
```
— overdue-but-undrained count; alert when `> 0` for `> 60s`, plus a completed-delta over the window. The `ops_worker_heartbeat` row stays as coarse VM-liveness (it is genuinely useful: a dead VM = no heartbeat), but it is **explicitly NOT the lost-job detector** — proposal §7 #1 and §9 now say so. Updated: proposal §1 #1, §3 #1 "Lost-job detectability" block, §7 #1, §9 operability row #1, resolution #1 detectability (above). Grounded against `heartbeat.ts:9,28-31`, `liveness-checker.ts:11`, `reconciliation.ts:237-246`.

### C3 — the reconciliation worker the sweep leans on is REMOVED in prod · **FIX** (critical, choose standalone schedule)

Round-1 #1(e) offered the sweep "as its own pg-boss schedule OR as a sub-check on the reconciliation worker." The second option is **dead infrastructure**: `server.ts:382` shows `ReconciliationWorker` was temporarily removed for an esbuild bundle issue and is constructed nowhere. Worse, its removal means **O1 (stale PENDING > 1h) and O3 (failed pg-boss jobs > 10, `reconciliation.ts:237-253`) have no detector at all in prod today** — the design's broader detectability story silently assumed a process that isn't deployed.

**Decision — mount the cancel sweep as a STANDALONE pg-boss schedule in `apps/api`, NOT on the recon worker.** Rationale, grounded:
- The `NotificationWorker` already runs in `apps/api` (`server.ts:347,352-354`), and `boss.createQueue` + `boss.schedule` are proven runnable there (`anonymizer-retention.ts:26`). A standalone 1-minute schedule beside the notification workers reuses live, deployed machinery — no dependency on the removed recon worker, no esbuild-bundle gamble on the Go path.
- Re-enabling `ReconciliationWorker` + fixing its esbuild bundle is a larger, riskier change with its own failure mode on the critical launch path — rejected as the sweep's host. (It SHOULD be re-enabled to restore O1/O3 detection, but that is tracked separately as R9 below, not gated to the cancel sweep.)

**Fix:** proposal §3 #1 Part B + resolution #1(e) updated to specify "own pg-boss schedule in `apps/api` (beside `NotificationWorker`)"; the "sub-check on the reconciliation worker" alternative is **struck** as referencing dead infrastructure. **New R9 (accept-risk + flag):** O1/O3 detectors are dead in prod since the recon-worker removal — re-enable the ReconciliationWorker (fix esbuild bundle) in a follow-up deploy; until then stale-PENDING and failed-job detection rely solely on the new C2 throughput metric for the `order.timeout` queue. Owner/Architect. Grounded against `server.ts:347,352-354,382`, `reconciliation.ts:182-195,237-253`, `anonymizer-retention.ts:26`.

### C4 — notification dedup is not durable: double-Telegram on race/restart, or ZERO on the R2 crash-case · **FIX** (critical — add UNIQUE, revise R2)

The round-1 anti-duplication claim ("`rowCount>0` guard + `singletonKey` + dedup at the notification layer") leaks on the live tree:
1. `rowCount>0` makes the **sweep silent in the exact recovery case** R2 names: handler commits the CANCEL then dies before its post-COMMIT enqueue → sweep's later UPDATE finds status already CANCELLED → `rowCount 0` → no emit. Net **zero** notification, not one. R2's "sweep recovers" is FALSE as written.
2. `singletonKey` only dedups jobs in `created/active/retry`; once the handler's notify completes (ms), the sweep's enqueue ~60s later is a fresh accepted job.
3. The cross-process dedup is a **per-process in-memory `Set`** (`notifications/workers/index.ts:57`), and `notification_outbox_audit` has **no UNIQUE** on `(event,target_id)` (`1790000000007:31` is a plain index) — `ON CONFLICT DO NOTHING` suppresses nothing. Two api instances or one restart between the two emits → **two identical owner Telegrams**.

This is a genuine fork: the leak is in BOTH directions (double-send on race/restart, zero-send on the R2 crash). Resolving it honestly requires a durable dedup substrate, which means a migration — so the round-1 "no schema change" claim is now explicitly false (see H1 too).

> **SUPERSEDED by RESOLVE round 3 / RC2 below:** the partial UNIQUE in (a) is REMOVED (it crashes the legitimate multi-row attempt-log). Sub-point (b) — emit on *observing* CANCELLED-by-timeout — STANDS (it closes the zero-notification hole independent of substrate). Dedup is now the in-process `Set` + `singletonKey`; cross-restart duplicate is accepted (R12).

**Decision — make dedup durable via a forward-only UNIQUE, and key the emit on cancel-detection, not handler-identity:**
- **(a) FIX — add a forward-only migration: `UNIQUE` partial index on `notification_outbox_audit (event, target_id) WHERE event = 'order.timeout_cancelled'`** (partial keeps the constraint scoped to the cancel event, so no other notification flow's cardinality is constrained). The emit path then does a real `INSERT … ON CONFLICT (event,target_id) DO NOTHING` that durably suppresses the second emit across processes/restarts. This makes "dedup at the notification layer" TRUE instead of aspirational. Forward-only, atomic, additive index — RLS/integer-money/RS256 untouched. **This converts §5's "Schema changes: NONE" to "ONE additive index" — corrected in proposal §5.**
   - *Honest caveat:* a partial UNIQUE on an existing table requires the existing rows to already satisfy it for `order.timeout_cancelled` — at pilot there are none (the emit doesn't exist yet), so the index builds clean. The migration must run before the emit ships.
- **(b) FIX R2 — decouple emit from canceller-identity.** Both handler AND sweep attempt the emit **whenever they observe the order in CANCELLED-by-timeout state**, not only when their own UPDATE returned `rowCount>0`. The durable `ON CONFLICT DO NOTHING` (a) makes the second attempt a no-op; the first attempt — from whichever of handler/sweep acts — always lands. This closes the zero-notification hole: if the handler dies post-COMMIT pre-emit, the sweep's emit (keyed on observing CANCELLED+timeout_at-was-set, not on having done the UPDATE itself) still fires exactly once. R2 in the risk table is rewritten accordingly.
- **Alternative considered — accept single-instance for pilot (`fly.toml:15` `auto_stop_machines=false`, one api VM):** rejected as the sole mitigation. It masks the double-send across a *restart* (deploy during a timeout window) and does nothing for the zero-send R2 crash-case, which is intra-process. The UNIQUE is cheap, forward-only, and closes both — boring and proven beats relying on never restarting.

**Fix:** proposal §5 (schema-change claim), §6 (#1 idempotent cancel), §7 #1 updated; resolution R2 rewritten below; threat-model #1 gains a "exactly-once owner notification across handler/sweep/restart" assertion. Grounded against `notifications/workers/index.ts:57`, `1790000000007:31`, `server.ts:347`.

### H1 — "Schema changes: NONE" is false; the sweep needs `(status, timeout_at)` · **FIX** (add forward-only index)

Round-1 §5 said "Schema changes required: NONE" and §1/§3 claimed the sweep is "Indexed on `(status, timeout_at)`." That index does not exist (`1780310074262:44-45` only has location-leading indexes; `timeout_at` is bare, `:38`). The cross-tenant sweep `WHERE status='PENDING' AND timeout_at<now()` (no `location_id` predicate) cannot use a `(location_id, status)` index → seq scan every minute. The claim "indexed" and the claim "no schema change" cannot both be true.

> **SUPERSEDED by RESOLVE round 3 / RC1 below:** this index is REMOVED — `CONCURRENTLY` is incompatible with the single-transaction migration runner, and the pilot seq-scans cheaply. §5/ADR return to "Schema changes: NONE." H1 is a non-issue at pilot, not shipped as an index.

**Fix — add a forward-only partial index, and correct the false claim.** `CREATE INDEX CONCURRENTLY orders_pending_timeout_idx ON orders (timeout_at) WHERE status='PENDING'` — a partial index on PENDING-only rows (a handful at any moment) is tiny and exactly matches the sweep predicate. Forward-only, additive, `CONCURRENTLY` so no write-lock on `orders`. With C4's UNIQUE this is the **second** migration — so proposal §5 is corrected from "Schema changes: NONE" to "TWO forward-only additive indexes (no column/constraint changes to existing data, no money/RLS/auth change)." The honest framing: the round-1 "no migrations" boast was wrong; two narrow forward-only indexes is still well within the boring-and-proven envelope and does not touch the integer-money / RLS-FORCE / RS256 invariants.

*Pilot-scale note (kept honest):* at pilot scale a seq scan over a few thousand PENDING-or-not rows every 60s would also be cheap, so the index is not strictly required to *function* at launch — but shipping it (a) makes the design's own performance claim true rather than false, and (b) costs nothing forward-only. Accepted to ship the index rather than defend a seq scan. Grounded against `1780310074262:38,44-45`.

### H2 — "single transition authority / no logic fork" is structurally impossible; `apps/worker` can't import `@deliveryos/domain` · **FIX** (correct the overclaim)

Round-1 #1(c) insisted the worker cancel routes through `assertTransition('PENDING','CANCELLED')` "reusing the single transition authority." But `apps/worker/package.json:11-16` has no `@deliveryos/domain` dep, and `assertTransition` lives in `packages/domain/src/order-machine.ts`. Three call sites (worker handler, api sweep, domain guard) sit in non-overlapping dep graphs; "no logic fork" is contradicted by the module boundaries — the handler's UPDATE (`handlers.ts:26-30`) is already a hand-written duplicate of the sweep SQL, not a shared function.

**Decision — the guarded `WHERE status='PENDING'` UPDATE IS the transition authority for the timeout-cancel path; drop the `assertTransition`-in-worker overclaim.** Rationale, grounded:
- `PENDING→CANCELLED` on timeout is a single, fixed, unconditional transition — there is no branching legality to centralize. The guard `WHERE status='PENDING'` *is* the legality check: it cancels iff still pending, atomically. Importing a state-machine guard to assert one hardcoded edge adds a cross-package dependency edge for zero added safety.
- Adding `@deliveryos/domain` to `apps/worker` just to call `assertTransition('PENDING','CANCELLED')` is over-engineering against "schema rich, runtime minimal" — a new dep-edge and bundle weight to restate what the WHERE already enforces. Rejected.
- **What IS shared and MUST stay single-authority:** the *SQL shape* of the guarded cancel (`SET status='CANCELLED', timeout_at=NULL WHERE status='PENDING'`) and the notification emit. The sweep (in `apps/api`) and the handler (in `apps/worker`) are in different packages, so a literally-shared function is not free; instead the resolution mandates the **identical SQL string + emit contract** in both, asserted by a DoD test that runs both paths against the same fixture and asserts identical row outcomes + exactly-one notification (threat-model #2/#3). "Single authority" is downgraded honestly to "single *specified* contract, test-enforced," not "single imported function."

**Fix:** proposal §1 #1, §3 #1 Part A(b)/(c), §6 (#1 idempotent cancel) rewritten to drop the `assertTransition`/`orderStatusService` reuse claim for the worker path and state the WHERE-guard-is-authority position; resolution #1(c) above is superseded by this. The API-side order routes keep using `assertTransition` (they already depend on `@deliveryos/domain`); only the worker/sweep timeout path is corrected. Grounded against `apps/worker/package.json:11-16`, `handlers.ts:26-30`, `packages/domain/src/order-machine.ts`.

### H3 — batched modifier-group partition consumed once skips validation on a 2nd same-product line · **FIX** (specify per-line re-read + DoD test)

The #2 batching replaces the per-line `modifier_groups` query (`orders.ts:430-436`) with one set-based query partitioned by `product_id`. The current loop validates min/max-select **per line-item** against that line's `groupCounts` (`:438,456,466-476`). The same `product_id` can appear in multiple cart lines ("1 Margherita no cheese" + "1 Margherita extra cheese"). If the in-memory partition is built as `Map<product_id, groups>` and **consumed once**, the 2nd+ line referencing that product silently skips group validation → an invalid modifier selection passes.

**Fix — specify the broadcast semantics explicitly + add a DoD test.** proposal §2/§3 #2A now states: the batched result is a `Map<product_id, groups[]>` built once, but **every line-item independently re-reads its product's group set from that Map** and runs the existing per-line min/max validation (`:466-476`) against its own `groupCounts` — the partition is broadcast (read N times), never consumed/drained. This is a pure round-trip optimization only if each line re-reads; the resolution makes that a hard requirement, not an inference. **New DoD test (threat-model #5b):** a cart with two line-items of the SAME product, one with a valid and one with an invalid modifier-group selection, must reject (422 `MODIFIER_MIN_NOT_MET`/`MAX_EXCEEDED`) on the invalid line after batching — proving the partition is re-read per line, not consumed. Grounded against `orders.ts:426,430-436,438,456,466-476`.

### H4 — degradation is thin under #2's own 5xx; phone CTA only on MIN_ORDER · **FIX** (phone CTA on all non-422; cache fallback-config on mount)

Under #2's `SET LOCAL statement_timeout` abort, the most likely failure is a fast **5xx** — and `CheckoutPage.tsx:440` special-cases only `422 MIN_ORDER_NOT_MET`; every 5xx/network falls to the generic toast `:448` with **no phone CTA**. The round-1 #4 fallback then depends on an *additional* `/fallback-config` fetch made inside the catch — itself fragile under the same DB/load pressure that caused the 5xx.

**Fix — two changes, grounded:**
- **(a) Phone CTA on ALL non-422 errors** (5xx, network, timeout), not only MIN_ORDER. proposal §4 #4 + §7 #4 now specify: the `catch` shows the "call the restaurant" CTA for any error that is not a 422 business-validation error (422s carry an actionable message — min-order, modifier — and the customer can fix the cart; 5xx/network cannot be fixed by the customer → give the out-of-band path). MIN_ORDER keeps its specific message AND, since it is fixable, does not need the phone CTA.
- **(b) Cache `fallback-config` on mount, not in the catch.** Fetch `GET /api/public/locations/:slug/fallback-config` once when the checkout page mounts and hold `{phone, showPhoneOnError}` in component state. The catch then reads cached state — the phone CTA does NOT depend on a network call succeeding under the exact load that caused the failure. If the mount fetch failed (rare), the catch degrades to the generic toast (fail-soft preserved). proposal §4 #4 + §7 #4 updated. Grounded against `CheckoutPage.tsx:437-450`, `fallback-config.ts:27-31`.

### M1 — down→503 is invisible to Fly; assign an external monitor or call it external-only · **FIX** (assign owner + explicit act)

Fly reads `/livez` (`fly.toml:21`), so a `/health` 503 changes nothing Fly sees — correct (no restart loop) but also means #3's down→503 is a **no-op for production observability unless an external monitor is pointed at `/health`**. Round-1 did not establish any consumer.

**Fix — name the consumer and add an explicit act.** proposal §7 #3 + §9 now state: #3's down→503 is *for an external uptime monitor*, not Fly. **New launch act (assigned, Owner):** point an external uptime monitor (the existing UptimeRobot/Better Uptime, or one to be configured) at `https://dowiz.fly.dev/health` and alert on non-200. Without this, #3 is a correctness fix to a code path nothing reads — honestly flagged. If no external monitor exists for launch, #3 still ships (it is correct and the leak-fix in L2 stands on its own) but its observability value is recorded as **pending the monitor**, owner-tracked (R10). Grounded against `fly.toml:21`, `health.ts:64-70`.

### M2 — 4s `SET LOCAL statement_timeout` can abort a legitimate large order; retry with regenerated key → double-order · **FIX** (raise the bound + key-stability note)

A 15-line × 3-modifier cart is ~60 inserts; at ~50ms/round-trip under the latency spike the timeout targets, that is ~3s baseline with no headroom under 4s. The timeout meant to protect the pool fires on the legitimate large order, the customer sees the generic failure (now the phone CTA per H4), and **if the client regenerates the idempotency key on retry** (common on a fresh attempt) → double-order.

**Fix — raise the per-tx bound and tie it to `connectionTimeoutMillis`, not an arbitrary 4s.** proposal §3 #2(b) + §6 corrected: set `SET LOCAL statement_timeout` to **~4.5s** chosen as *just inside* `connectionTimeoutMillis=5000` (`index.ts:22`) with a stated margin, and document the back-of-envelope: a legitimate worst-case cart (~60 inserts × ~50ms = ~3s) must complete inside it with headroom; the bound exists to abort a *wedged* write (one stuck >>5s), not a slow-but-progressing one. The breaker's ~60-insert math is the explicit sizing input. **Plus a key-stability note (proposal §6, threat-model #4):** the idempotency key is derived from cart content + `menu_version` (`orders.ts:194,197`) — so a retry of the *same cart* re-derives the *same* `request_hash` and collides on `idempotency_keys` (23505 path) → no double-order, *provided the client does not inject a fresh nonce*. The resolution makes "retry re-uses the content-derived hash (no client-side random nonce in the key)" an explicit invariant + test (threat-model #4 extended). This closes the double-order vector at the idempotency layer regardless of the timeout. Grounded against `orders.ts:194,197,609-624`, `packages/db/src/index.ts:22`.

### M3 — loader idempotency conflict-target undefined; scrub checks only phone · **FIX** (define key + widen scrub)

§10-DATA.3 claimed "idempotent loader: upsert by stable external key" without naming the key, and the scrub proof checks only `phone` equality — but `fallback_config` is owner-authored free-form JSON (`fallback-config.ts:25-28`) and branding can carry an owner name/email/address.

**Fix — two corrections to §10-DATA:**
- **(a) Define the conflict target:** the loader upserts on `slug='demo'` (the only stable natural key across the prod→staging boundary, since `location_id`s differ). On re-run it `UPDATE`s the existing `demo` location and `ON CONFLICT (location_id, <natural-key>)`-upserts categories/products/modifiers keyed by their stable source identifiers (product name/sort within the demo tenant), never blind-inserting → no duplication. proposal §10-DATA.3 + R6 updated.
- **(b) Widen the scrub proof beyond phone.** The transform must scrub/placeholder ALL free-form identifying fields, and the proof asserts: `staging demo.phone ≠ prod sushi-durres.phone` AND the `demo` location's `fallback_config` JSON contains no prod phone/email/owner-name string AND branding `name`/contact fields are demo placeholders. proposal §10-DATA.2 (transform) + §10-DATA.verification + resolution #5 data-task + threat-model #12 widened. Grounded against `fallback-config.ts:17,25-28`.

### L1 — retention audit-row won't appear ~a year on a fresh pilot; day-1 gate is heartbeat-presence, not enforcement-proof · **FIX** (honest gate framing)

The §5(c) gate "an `anonymization_audit_log` row `scope='retention'` in-window" is unmeetable at launch: the sweep only writes an audit row when it anonymizes something, and a fresh tenant has nothing older than `retention_days=365` for ~a year. The OR-branch (fresh `anonymizer-retention` heartbeat) only proves the *api-side* anonymizer worker breathes — and per C2's lesson, a heartbeat proves "process alive," not "enforcement happened."

**Fix — split the gate honestly into two phases.** proposal §5(c) + §9 + resolution #5 corrected:
- **Day-1 gate (launch):** `anonymizer-retention` schedule is **registered** (`boss.schedule` present) AND its heartbeat is fresh. This proves the sweep is *wired and breathing* — the honest, available day-1 signal. It is explicitly labelled **heartbeat-presence, NOT enforcement-proof**, so a green gate is never misread as "retention has run." (Matches counsel re-exam §1 caveat #1.)
- **Maturing gate (after the first retention window produces expired data):** the `anonymization_audit_log scope='retention'` in-window row becomes the *positive enforcement* signal; only then does "row in-window" carry weight. Until then a missing audit row is **expected, not a failure** — the gate must not flip red on emptiness, and the copy must not be softened on the false assumption the sweep is dead.
- The honest day-1 statement to the human: "the retention promise is *wired and observable*; positive enforcement proof matures with the first expiry window." Grounded against `anonymizer-retention.ts:22-27,60-65`, `anonymizer/index.ts:247-271,285-289`.

### L2 — `/health` `detail` still leaks driver error text on unauthed endpoint · **FIX** (drop detail from public payload too)

Round-1 #3 dropped `data: result` but not `detail`, which on the down/degraded paths surfaces `${label}: ${err.message}` — raw pg driver text (host, role, SQLSTATE fragments) — on the unauthenticated `/health` (`health.ts:35,38`).

**Fix:** proposal §4 #3 + §8 updated: the **public** `/health` payload drops `detail` as well as `data`. The public shape is exactly `{ status, timestamp, checks: { <name>: { status, latencyMs } } }` — no `detail`, no `data`, no driver text. The `detail` string (useful for diagnosis) moves to the **authenticated** `/health/detail` only. threat-model #7 extended: assert the public `/health` body contains no `detail` key and no `err.message`-shaped substring. Grounded against `health.ts:28,35,38`.

---

### Updated / new risk dispositions from round 2

> **Round-3 note (RC1/RC2):** R2 and R11 below are SUPERSEDED — the durable UNIQUE they reference is removed; see the round-3 R2/R11/R12 dispositions and the authoritative risk table in `proposal.md §10`. The "emit on observing CANCELLED" half of R2 stands.

| # | Risk | Disposition | Owner |
|---|---|---|---|
| R2 (rewritten) | #1 `order.timeout_cancelled` emit is post-COMMIT. **Round-1 framing was wrong** (C4): `rowCount>0`-gated emit makes the sweep *silent* in the crash-recovery case → zero notification. **Fixed**: emit is keyed on observing CANCELLED-by-timeout (not on having done the UPDATE), backed by a durable UNIQUE `ON CONFLICT DO NOTHING` (C4a). Residual: a process that dies *after* COMMIT *and after* the conflict-insert but *before* pg-boss accepts the notify job loses that one notify — recovered on the next sweep tick. | **Accepted (minimized)** — exactly-once across handler/sweep/restart now holds; the residual is a single ~60s-recoverable gap | System Architect |
| R9 (new, C3) | O1 (stale PENDING >1h) + O3 (failed pg-boss jobs >10) detectors are **dead in prod** since `ReconciliationWorker` was removed (`server.ts:382`). | **Accept-risk + flag** — cancel sweep does NOT ride the recon worker (standalone schedule in `apps/api`), so launch is unblocked; but re-enable `ReconciliationWorker` (fix esbuild bundle) in a follow-up to restore O1/O3. Until then the C2 throughput metric covers `order.timeout` only. | Owner/Architect |
| R10 (new, M1) | #3 down→503 is invisible to Fly (`/livez`); observability value depends on an external monitor hitting `/health`. | **Accept-risk + explicit act** — point an external uptime monitor at `/health`; if none for launch, #3 still ships (correct + L2 leak-fix), observability value recorded as pending the monitor. | Owner |
| R11 (new, H1+C4) | Round-1 "no schema change" claim was **false**. Launch now carries **two forward-only additive migrations**: `orders_pending_timeout_idx` (partial, on PENDING) and a partial UNIQUE on `notification_outbox_audit(event,target_id) WHERE event='order.timeout_cancelled'`. | **Accepted** — both additive, `CONCURRENTLY`/online-safe, forward-only, touch no existing data, no money/RLS/auth change; well within the boring envelope. The "NONE" claim is corrected in proposal §5 + ADR. | System Architect |

All round-2 fixes hold the invariants: the two new migrations are forward-only and additive (no column drops, no data rewrite); RLS `ENABLE`+`FORCE` untouched; integer-money arithmetic untouched; JWT RS256 untouched. No finding required an `accept-risk` that crosses a red line, and none is deferred out of launch-scope except R9's O1/O3 *restoration* (the cancel-path detection it would have hosted is re-homed onto the C2 throughput metric, which ships at launch).

---

## RESOLVE раунд 3 — RC1 / RC2 / RH1

> The round-2 fixes that introduced the two additive migrations (M-1 index, M-2 partial UNIQUE) and the pg-boss overdue-query detector each broke against the live tree. The breaker is correct on all three counts; I re-verified each against source before disposition. The honest outcome: **both round-2 migrations are removed.** That returns §5/ADR to "Schema changes: NONE" — which is now true again, and is *better* than carrying two migrations to make a perf/dedup claim the pilot does not need. Detection is re-homed into the live standalone sweep job. Invariants held: forward-only, RLS `ENABLE`+`FORCE`, integer-money, RS256.

### Re-verification (live tree, round 3)

| Breaker claim under test | Verdict | Evidence (re-checked) |
|---|---|---|
| node-pg-migrate runs ALL migrations in ONE transaction; `CREATE INDEX CONCURRENTLY` would throw 25001 | **TRUE** | `packages/db/migrations/1790000000011_pgboss-bootstrap-schema.ts:18-30` documents verbatim *"node-pg-migrate runs ALL pending migrations inside ONE transaction (`--single-transaction` defaults to true)"*; the ONLY migration that escaped is 0011 itself, via `pgm.noTransaction()` + manual COMMIT (`:25-30`). No `pgm.createIndex(...{unique:true})` in the tree uses `CONCURRENTLY`. A `CONCURRENTLY` in a normal up-migration → `25001: cannot run inside a transaction block` → boot fails (same class as `:9` "the API cannot boot"). |
| `notification_outbox_audit` is an attempt-log (N rows per (event,target_id)), not an event-log | **TRUE** | `writeAudit` (`apps/api/src/notifications/audit.ts:31-44`) is a **bare `INSERT` with NO `ON CONFLICT`**, called for statuses `sending`/`delivered`/`failed`/`prefs_disabled`/`circuit_open`/`dedup`/… — 2+ rows per target per attempt (`workers/index.ts:387-444`). A UNIQUE on `(event,target_id)` makes the 2nd legit row of the same target throw `23505` → the handler crashes (no `ON CONFLICT` in `writeAudit`) → the very notification we are protecting fails. |
| `target_id` is nullable and written NULL; NULL does not dedup in a UNIQUE | **TRUE** | schema `1790000000007:15` nullable; `audit.ts:37` writes `entry.targetId || null`; dead-letter path `workers/index.ts:470-473` writes NULL. Postgres treats NULL as distinct in a UNIQUE → `ON CONFLICT DO NOTHING` suppresses nothing for NULL rows. And `target_id` = `owner_notification_targets` id (`workers/index.ts:391`), NOT the order id — the dedup key was semantically wrong. |
| The standalone 1-min sweep already lives in `apps/api` via `boss.schedule` | **TRUE** | `anonymizer-retention.ts:22,26,27` — `boss.work` + `boss.createQueue` + `boss.schedule(cron, …, { singletonKey })` all run in `apps/api`. A new scheduled sweep job is the same proven pattern. |
| The pg-boss overdue-query has NO executor in prod | **TRUE** | `LivenessChecker.run()` reads only `ops_worker_heartbeat` (`liveness-checker.ts:46-54`); `ReconciliationWorker` (where the `pgboss.job` SELECT lives, `reconciliation.ts:237-246`) is REMOVED (`server.ts:382`). Nothing deployed runs the overdue count. |
| Single web VM, `auto_stop_machines=false`, in-memory dedup works in-process | **TRUE** | `fly.toml:15` `auto_stop_machines=false`, single web VM (no `min_machines_running>1`); the per-process `Set` (`notifications/workers/index.ts:57`) is valid within the one live process. |

### RC1 — `CREATE INDEX CONCURRENTLY` is structurally incompatible with the single-transaction runner · **FIX = REMOVE the index (M-1)**

The breaker is right: every migration in this repo runs inside one transaction (0011:18-30); `CONCURRENTLY` throws 25001 there, and a *non*-`CONCURRENTLY` `CREATE INDEX` on `orders` takes an `AccessExclusiveLock` on the table for the whole migration batch — that is exactly the write-lock the round-2 §5/R11 "online-safe, no write-lock" claim said it avoided. Two candidates:
- **(a)** `pgm.noTransaction()` + non-concurrent `CREATE INDEX` (short lock on `orders`, a few thousand rows at pilot → ms).
- **(b)** Drop the `orders_pending_timeout` index entirely — my own round-1 H1 already conceded *"a seq scan over a few thousand PENDING-or-not rows every 60s would also be cheap, so the index is not strictly required to function at launch."*

**Decision: (b) — remove M-1.** Honest reasoning: the index was shipped only to make the round-1 "indexed on `(status,timeout_at)`" claim *true*, not because launch needs it. At pilot scale (back-of-envelope below) the 1-minute sweep is a seq scan over a single small table — cheap. Adding `pgm.noTransaction()` machinery + an `AccessExclusiveLock` (option a) to support a claim the workload does not need is over-engineering against "schema rich, runtime minimal." Removing it is forward-only by construction (there is no migration to write) and returns §5 to "Schema changes: NONE," which is the truthful, simpler state.

*Back-of-envelope that keeps this OUT of CRITICAL (H1 does not regress):* pilot = 1 active tenant, ~3-10 locations, peak ~0.5 orders/sec system-wide (proposal §2). PENDING orders at any instant are bounded by `arrival_rate × confirm_timeout` ≈ 0.5/s × ~5 min = ~150 rows worst case; the whole `orders` table is low thousands of rows for months. A seq-scan `UPDATE … WHERE status='PENDING' AND timeout_at < now()` once per 60s over a few-thousand-row table is sub-millisecond and runs off the hot path (standalone schedule, not a request). The index would save microseconds on a query that runs 1,440×/day on a tiny table — below the noise floor of the free-tier DB. H1 stays a **non-issue at pilot**, not a CRITICAL: there is no correctness or availability break from a seq scan here, only a perf claim that is now simply not made. Re-evaluate at 10× volume (R1's existing trigger) — at which point an index can be added properly via its own `pgm.noTransaction()` migration. **Owner: System Architect (accept-risk: seq scan at pilot).**

### RC2 — partial UNIQUE on `(event,target_id)` breaks the legitimate attempt-log · **FIX = REMOVE the UNIQUE (M-2); accept at-most-once notification on single-instance pilot**

The breaker is right and this index was a mistake. `notification_outbox_audit` is a **delivery-attempt journal**, not an event ledger: `writeAudit` (audit.ts:31-44) is a bare `INSERT` with no `ON CONFLICT`, and one logical send writes `sending`→`delivered`/`failed` (2+ rows per target per attempt, workers/index.ts:387-444). A UNIQUE on `(event,target_id)` would make the 2nd legit row throw `23505` and crash the handler — failing the very notification it was meant to dedup. Worse, `target_id` is nullable and written NULL (audit.ts:37, workers/index.ts:470-473) so NULL rows never dedup, and `target_id` semantically = `owner_notification_targets` id, not the order id — the dedup key never made sense. **Remove M-2 entirely.**

That re-opens C4's question: how is `order.timeout_cancelled` deduplicated? Two honest paths:
- **(a) Accept at-most-once on single-instance pilot.** `fly.toml:15` `auto_stop_machines=false`, one web VM. The NotificationWorker's in-memory LRU `Set` (`notifications/workers/index.ts:57`, key `${event}:${entity_id}:${location_id}`) works correctly **within the one live process** — handler-emit and sweep-emit in the same process dedup correctly. The only leak is across a process boundary: a rare api **restart** (deploy) landing inside the ~60s window between the handler's emit and the sweep's emit → owner gets a duplicate "auto-cancelled" Telegram. My own breaker C4 conceded this is *"mostly masked at pilot scale"* with one VM. A duplicate Telegram on a rare restart-during-timeout-window is a benign, low-frequency annoyance — NOT a correctness or money break.
- **(b) Durable dedup via a SEPARATE dedup table** (`notification_dedup(event_id TEXT PRIMARY KEY, …)`, `event_id = order.timeout_cancelled:{orderId}`, `INSERT … ON CONFLICT DO NOTHING`). This is the correct shape *if* durable dedup is required — a dedicated dedup ledger, never a UNIQUE on the attempt-log.

**Decision: (a) — accept-risk at-most-once on the pilot, with owner.** Reasoning, with numbers: the duplicate fires only on `P(api restart) × P(restart lands in the ~60s emit-gap of an order that timed out)`. Deploys are infrequent (manual, days apart at pilot); a timeout-cancellation is itself rare (most orders are confirmed); their intersection is near-zero per month. The cost of the leak is one extra Telegram line to the owner — recoverable by the owner simply reading the order is already cancelled. Against that: option (b) is a new table + migration (re-introducing the very schema-change cost RC1 just removed) to prevent a near-zero-frequency duplicate notification. **Boring-and-proven says: do not add a durable substrate for a pilot-scale rare-restart duplicate.** Accept it, flag it, and if a 2nd web instance is ever added — the moment the in-process Set stops covering — promote to option (b) (a dedup *table*, never the attempt-log UNIQUE). The emit keeps `singletonKey: order.timeout_cancelled:{orderId}` (dedups in-flight jobs) + the in-memory Set (dedups within the process); these two cover the common case, and the residual cross-restart duplicate is the accepted risk.

*Why this does NOT return C4 to CRITICAL:* C4's true defects were (i) the zero-notification crash-case and (ii) the double-send. (i) is unrelated to the UNIQUE — it is fixed by **R2's design change (kept): both handler and sweep emit on *observing* CANCELLED-by-timeout, not on their own `rowCount>0`** — so whichever path acts emits, never zero. The in-process Set + singletonKey then make a same-process double a no-op. (ii)'s only surviving leak is the cross-**restart** duplicate, now explicitly accept-risk on single-instance pilot. So C4 drops from "non-durable dedup" to "exactly-once in-process + at-most-once-plus-rare-duplicate across restart" — an accepted pilot risk, not a CRITICAL. **Owner: Owner/Architect.**

### RH1 — the overdue-query has no executor · **FIX = co-locate detection in the same standalone sweep job**

The breaker is right: `LivenessChecker` reads only `ops_worker_heartbeat`; the `pgboss.job` overdue SELECT lives only in the removed `ReconciliationWorker`. So as round-2 left it, nothing in prod runs the detection.

**Decision: fold the overdue count into the SAME 1-minute standalone sweep job** that already recovers (RH1 itself confirms this job is HELD-runnable in `apps/api` via `boss.schedule`, like `anonymizer-retention.ts:22,26`). On each tick, before/after the recovery UPDATE, the sweep also runs:
```
SELECT count(*) FROM pgboss.job
WHERE name = 'order.timeout' AND state IN ('created','active') AND start_after < now()
```
and logs / emits it as an ops counter (warn when `> 0`). Detection is thereby **co-located with recovery in live infra** — one scheduled job, one executor, no dependence on the dead `ReconciliationWorker` or the name-keyed heartbeat. The job both *recovers* (the guarded UPDATE) and *detects* (the overdue count) every 60s. This is strictly better than round-2's split: the same process that fixes the problem also measures it, so the detector cannot silently lose its host. (O1 stale-PENDING / O3 failed-jobs detectors remain deferred under R9 — they need the `ReconciliationWorker` re-enabled and are out of launch scope.)

**Updated dispositions:**

| # | Risk | Disposition | Owner |
|---|---|---|---|
| R9 (unchanged) | O1/O3 detectors dead in prod (`ReconciliationWorker` removed). | **Accept-risk + flag** — re-enable in a follow-up. The cancel sweep's `order.timeout` overdue count now ships co-located in the standalone sweep job (RH1), so the launch-critical detector is live. | Owner/Architect |
| R11 (rewritten) | Round-2 carried two additive migrations (M-1 index, M-2 UNIQUE). Both proved broken/needless against the runner and the audit-log shape (RC1/RC2). | **Both REMOVED.** §5/ADR return to **"Schema changes: NONE"** — true again. No `noTransaction`, no `AccessExclusiveLock`, no UNIQUE on the attempt-log. Forward-only by construction (no migration to write). | System Architect |
| R12 (new, RC2) | `order.timeout_cancelled` dedup is now in-process (LRU Set) + `singletonKey`; the only residual is a duplicate owner Telegram if an api **restart** lands in the ~60s emit-gap of a timed-out order. | **Accept-risk** on single-instance pilot (`auto_stop_machines=false`, one web VM). If a 2nd web instance is ever added, promote to a durable dedup **table** (`event_id PK`), NOT a UNIQUE on the attempt-log. | Owner/Architect |

**Invariants:** removing two migrations is forward-only (nothing to apply); RLS `ENABLE`+`FORCE`, integer-money, RS256 all untouched (they always were). Neither RC1's seq-scan nor RC2's accept-risk re-opens H1 or C4 as CRITICAL — both are bounded, owner-accepted pilot risks justified by the numbers above.
