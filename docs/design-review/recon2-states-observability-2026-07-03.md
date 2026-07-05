# Recon #2 — Error-Matrix, Rare/Edge States & Observability Blind Spots

**Date:** 2026-07-03 · **Mode:** READ-ONLY deep recon (findings only, no edits)
**Scope:** the error-matrix (401/403/404/422/429/5xx/network/timeout), zero/one/many + money + date boundaries, and observability/alerting gaps.
**Method:** three parallel read-only lanes (FE state-handling · API edge-states · observability), every finding confirmed against source lines. Two headline findings independently re-verified by the lead (grep + source read).

**Explicitly excluded** (covered by recon #1, not re-reported here): worker health-checks hardcoded `ok`; fake/demo data used as fallback for real data (CRMPage fabricated analytics, SettingsPage `MOCK_SETTINGS`); retry-button-never-clears-error; paper-gate CI.

## Severity tally

| Severity | Count |
|----------|-------|
| **P0** | 5 |
| **P1** | 13 |
| **P2** | 26 |
| **Total** | **44** |

**Most dangerous silent failure:** `apps/api/src/workers/reconciliation.ts:103` — the nightly reconciliation is the designated backstop for every other blind spot (money drift M1–M4, failed jobs O3, undelivered notifications N1, worker liveness), yet it publishes `ops.reconciliation_drift` to a bus channel with **zero subscribers and no event-registry entry**. The watcher-of-last-resort reports to nobody — which is precisely why the three *other* dead alert wires below (backup, settlement, liveness) have gone unnoticed. The codebase has already documented this exact class before (`messaging.ts:80-84`: a prior "zero subscribers, escalation vanished into the void" incident for `dispatch-failed`).

---

## P0 — must-fix

1. **P0 · Observability** | `apps/api/src/workers/reconciliation.ts:103` | The nightly money-integrity safety net (M1–M4 pricing/cash drift, O3 failed jobs, N1 undelivered notifications) publishes `ops.reconciliation_drift` to a channel with **zero subscribers** — every DRIFT alert terminates in the void (console-only). | Subscribe the channel in `bootstrap/messaging.ts` + add `ops.reconciliation_drift` to event-registry/locales/buildTelegramData.

2. **P0 · Observability** | `apps/api/src/bootstrap/messaging.ts:10` | Backup-failure alert enqueues event name `'backup.failed'`, but registry/switch expect the `ops.`-prefixed `'ops.backup_failed'` → `buildTelegramData` throws "Unsupported event type" per target, so a failed backup can **never** produce a Telegram alert; payload also lacks `locationId` (`backup/index.ts:215`) → falls back to the `'system'` location which has no targets. | Rename to `ops.backup_failed` and route to a real ops target.

3. **P0 · Observability (money)** | `apps/api/src/bootstrap/messaging.ts:22` | Settlement-dispute alert enqueues `'settlement.disputed'`, which exists nowhere in registry/provider/locales/buildTelegramData → same per-target throw → courier cash-settlement disputes **never reach the owner**. | Add the event to registry + switch + locales.

4. **P0 · Observability** | `apps/api/src/workers/liveness-checker.ts:71-107` | Worker-death detection publishes `worker.stale` / `worker.batch_stale` / `alert.worker_liveness` / `liveness.check.failed` — all four channels have **zero subscribers** (verified), and the registered `ops.worker_liveness` event has no producer → a dead dispatcher/settlement-cron alerts no one. | Bridge these channels to `ops.worker_liveness` in `messaging.ts`.

5. **P0 · Error matrix (FE)** | `apps/web/src/pages/admin/SettingsPage.tsx:335-339` | Saving settings that fails with a **404** shows the "Settings saved" success toast + the `justSaved` checkmark animation — the owner's changes were never persisted but the UI claims success. | Treat 404 in the catch as failure (error toast + `setError`), never fake success.

---

## P1 — high

### Error matrix

6. **P1 · FE optimistic** | `apps/web/src/pages/admin/MenuManagerPage.tsx:496-505` | Product-availability toggle is optimistic; on PATCH failure the catch only `console.debug`s — no rollback, no toast — so the owner sees an item as sold-out/hidden while the storefront still sells it (or vice versa). | In catch, revert `categories` state and show an error toast.

7. **P1 · FE silent-load-fail + data-loss** | `apps/web/src/pages/admin/BrandingPage.tsx:50-64` | Initial `/owner/brand` load ends in `.catch(() => {})`; on failure the page silently renders default colors/empty logo with no error/retry, and a subsequent Save **overwrites** stored branding with those defaults (`logoUrl: null` wipes the logo). | Track a `loadFailed` state, show an error banner + retry, disable Save until loaded.

8. **P1 · FE silent-fail + data-loss** | `apps/web/src/pages/admin/BrandingPage.tsx:91-97` | Logo-upload failure is swallowed while the data-URL preview keeps showing the logo; owner then Saves, gets "Branding saved", but `logoUrl` was never set → storefront ends up with no logo. | Show an upload-failed toast and clear the preview in the catch.

9. **P1 · FE global-crash-invisible** | `apps/web/src/main.tsx:77` + `packages/ui/src/components/ErrorBoundary.tsx:45-47` | The SPA mounts `<ErrorBoundary>` with no `onError`, and apps/web has no `window.onerror`/`unhandledrejection` handler and no error tracker — **every frontend crash is invisible** to the team. | Add `onError` + global handlers that beacon to an API endpoint or Sentry-browser.

### Rare/edge — date & money

10. **P1 · Date boundary** | `apps/api/src/routes/public/menu.ts:351-355` | Midnight-crossing hours are always "closed": `isOpen = nowMins >= openMins && nowMins < closeMins` can never be true when `close < open` (e.g. 18:00–02:00) → an overnight venue reports closed 24/7, and the storefront closed-gate blocks ordering. | If `closeMins < openMins`, use `nowMins >= openMins || nowMins < closeMins`. *(lead-verified)*

11. **P1 · Timezone** | `apps/api/src/routes/public/menu.ts:340-352` | Open/closed uses server wall-clock (`new Date().getDay()/getHours()` = **UTC on Fly**), not venue-local time — CET/CEST venues are wrong by 1–2h at every open/close edge and use the wrong weekday after ~22:00–23:00 local. | Store a venue timezone; compute day/minutes via `Intl.DateTimeFormat(..., {timeZone})`.

12. **P1 · Money — uncapped discount** | `apps/api/src/routes/owner/promotions.ts:100,209` | Percentage promo has no ≤100 cap (`z.number().int().positive()`) and validate computes `Math.floor(subtotal * value / 100)` uncapped — a 150% promo yields `discount_amount > subtotal` (fixed-type IS capped via `Math.min`; percentage is not). | Add `.max(100)` for percentage + clamp `discount_amount = Math.min(discount, order_subtotal)`.

### Rare/edge — courier lifecycle

13. **P1 · Split-shift blocked** | `apps/api/src/lib/shiftService.ts:38-45` | A courier who ended a shift earlier the same day gets `400 Cannot open shift in status offline` from `POST /me/shift/start` — the today-shift lookup (`DATE(started_at)=CURRENT_DATE`) finds the offline row and the else-branch hard-throws → split shifts (lunch + dinner) impossible. | Reopen an `offline` today-row (`status='available', ended_at=NULL`) instead of throwing.

### Observability — erasure & webhooks

14. **P1 · Observability (GDPR)** | `apps/api/src/workers/anonymizer-gdpr.ts:100-105,111` | GDPR erasure hitting max retries sets `status='failed'` with no publish/alert (visible only if owner polls), and the worker-level `anonymizer.gdpr.failed` publish has **zero subscribers** — a legally-deadlined erasure failure is silent; `:96` also discards the real error, storing literal `'Processing error'`. | Publish per-request failure + bridge channel to owner/ops; store the actual error.

15. **P1 · Observability (money)** | `apps/api/src/routes/payments-webhook.ts:36-40` | A payment webhook with an unknown provider ref is **ACKed 200 and ROLLBACK'd with zero logging** — misrouted/pruned money events vanish without a trace. | Add `request.log.warn` with `providerPaymentId` before ACK.

16. **P1 · Observability (money)** | `apps/api/src/routes/payments-webhook.ts:84` | `'mismatch'` (customer under/over-paid) is written to `payment_events` with no status flip, but **nothing queries `type='mismatch'`** anywhere (refunds.ts only reads refund_due/refund_sent) — no alert, no UI: a held underpaid order rots silently. | Add a reconciliation check or owner alert on mismatch events.

17. **P1 · Observability (dead-letter)** | `apps/api/src/notifications/workers/index.ts:519-530` | Retry-exhausted notifications are dead-lettered as `status='archived'` with `console.error` only; recon N1 monitors only order.created/confirmed/rejected and its alert rides the dead drift channel → exhausted ops/settlement alerts drop invisibly. | Widen N1's event set + fix the drift channel (finding #1).

### Rare/edge — reconciliation query

18. **P1 · SQL edge (watchdog self-break)** | `apps/api/src/workers/reconciliation.ts:319-321` | T1 cancellation-rate check breaks permanently once a venue has orders on 2+ prior days: `yesterday_rate` and `week_avg` are scalar subqueries over a multi-row CTE → PG 21000 "more than one row returned by a subquery"; `week_avg` isn't an average even when it works. | Use `AVG(cancelled::float/NULLIF(total,0))` aggregates in each subquery.

---

## P2 — medium

### Error matrix — FE silent-fail toggles/actions

19. **P2** | `apps/web/src/pages/admin/PromotionsPage.tsx:260-267` | Activate/deactivate switch: on API failure only `console.error` — switch doesn't move, zero feedback (silent no-op tap). | Add an error toast in the catch.
20. **P2** | `apps/web/src/pages/admin/PromotionsPage.tsx:277-283` | Delete-after-confirm: DELETE failure only `console.error`s — card stays, nothing tells the owner it failed. | Add an error toast.
21. **P2 · 422 not surfaced** | `apps/web/src/pages/admin/PromotionsPage.tsx:241-244` (form `84-87`) | Create/edit discards the server error (`return false`) → a 422/409 (duplicate code, invalid dates) always renders the generic "Could not save this promotion"; envelope code/message/field details never surfaced per-field. | Return the `ApiError`; map field-level details into the form's `errors` map.
22. **P2** | `apps/web/src/pages/admin/SettingsPage.tsx:269-290` | Telegram target enable/disable + category-pref toggles: on failure only `console.warn` — toggle silently doesn't move. | Error toast in both catches.
23. **P2 · load-vs-empty conflated** | `apps/web/src/pages/admin/AnalyticsPage.tsx:117-126,373-374` | Product-orders drilldown fetch failure sets `productOrders=[]`, rendering the same "No orders found" as a genuinely empty result — load-failure and empty indistinguishable, no retry. | Distinct drilldown error state + retry.
24. **P2** | `apps/web/src/pages/client/OrderStatusPage.tsx:129-142` | Customer taps a chat preset; POST failure only `console.warn` — message never appears, no feedback. | Toast / inline "couldn't send".
25. **P2** | `apps/web/src/pages/admin/DashboardPage.tsx:239-243` | Owner sends an order message; failure only `console.warn`s — message silently never appears in thread. | Error toast.
26. **P2 · WS staleness invisible** | `apps/web/src/pages/courier/TasksPage.tsx:69-85,127-137` | TasksPage never consumes the WS `status`; header badge is shift-based only, so with the socket down the courier still sees green "Online" while new task offers silently can't arrive live. | Show a reconnecting/offline indicator (like DashboardPage's `WSStatusDot`).
27. **P2 · 429 no handling** | `apps/web/src/lib/apiClient.ts:202-203` + `admin/LoginPage.tsx:86-88` + `courier/LoginPage.tsx:36` | 429 has no dedicated handling anywhere (empty switch case, no Retry-After/backoff); a rate-limited login renders the misleading "Login failed." with no wait guidance. | Branch on 429 → "too many attempts, wait" (pattern exists in AccessRequestForm).
28. **P2** | `apps/web/src/pages/client/CheckoutPage.tsx:80-90` | Entry-photo upload catch is `/* optional — leave unset on failure */`; spinner ends with no photo attached and no message — explicit user action fails silently. | Inline "upload failed, tap to retry".

### Error matrix — API status codes

29. **P2 · swallow-then-false-success** | `apps/api/src/routes/courier/assignments.ts:163-165,270-272` | Bare `catch { }` around the CONFIRMED/IN_DELIVERY advance swallows ALL errors (404/409/real SQL), not just no-ops — an in-tx SQL failure aborts the tx, the COMMIT silently rolls back everything, and the route still returns `{success:true}`. | Catch only IllegalTransition/SameStatus/CONFLICT; rethrow the rest.
30. **P2 · conflict→500** | `apps/api/src/lib/dispatch.ts:27-52` | Concurrent owner dispatches can select the same 'available' courier (no `FOR UPDATE SKIP LOCKED`); loser's INSERT violates `courier_one_active_assignment` → raw 23505 → **500** to the owner instead of a retryable 409/no_courier. | Add `FOR UPDATE SKIP LOCKED` or map 23505 → `{dispatched:false, reason:'courier_taken'}`.
31. **P2 · first-run 500** | `apps/api/src/routes/owner/promotions.ts:26,31` (`getLocationId`) | Fresh/removed owner (no active membership) gets `throw new Error(...)` → generic **500** on every promotions endpoint instead of 403/404. | Throw a typed `{statusCode:403}` error or `reply.sendError`.
32. **P2 · nondeterministic row** | `apps/api/src/routes/courier/shifts.ts:196-203` | `/shifts/transition` reads all `courier_shifts` rows with no ORDER BY/status filter and takes `rows[0]` — with multiple daily rows the current status and target shiftId are nondeterministic (can resurrect a stale row; no unique index prevents two 'available' rows). | `ORDER BY started_at DESC LIMIT 1` + partial unique index on active shifts.
33. **P2 · missing field details** | `apps/api/src/routes/orders.ts:92-94` (same at `:848`) | POST /orders Zod failure returns 400 with only joined messages — no issues array/field paths, contra the ADR-0010 details envelope. | Pass `parsed.error.issues` (path+message) as the `details` arg of sendError.
34. **P2 · leak + conflict→500** | `apps/api/src/routes/owner/menu-import.ts:587` | Commit catch-all returns 500 with **raw `err.message` leaked** to the client and maps DB conflicts (23505 duplicate external_key race) to 500 instead of 409; commit loop is unbounded per-row INSERTs in one tx with no statement_timeout. | Map known PG codes (23505→409), cap draft rows, set a tx statement_timeout.
35. **P2 · 500 on garbage input** | `apps/api/src/routes/customer/otp.ts:190` | Client `order_intent_hash` is hex-decoded and `JSON.parse`d inside the verify tx — hex-decodable-but-non-JSON input throws → **500 instead of 400**; client fully controls the re-hashed "intent", voiding intent binding. | Validate/parse before the tx (400 on garbage); derive intent hash from server-known data.

### Rare/edge — zero/one/many & unreachable code

36. **P2 · unreachable feature** | `packages/shared-types/src/legacy.ts:58-60` vs `apps/api/src/routes/orders.ts:642` | Crypto checkout is unreachable: `payment: z.object({ method: z.literal('cash') }).strict()` is required, so `input.payment?.method === 'crypto'` can never be true — when PAYMENTS_PREPAID/CRYPTO flip on, every crypto order 400s at Zod before the fork. | Widen to `z.enum(['cash','crypto'])` (flag-gated).
37. **P2 · no upper bound** | `packages/shared-types/src/legacy.ts:43` | `items: z.array(OrderItemInput).min(1)` has no `.max()` — a many-hundred-line cart is accepted into the checkout tx and only dies via the 4.5s statement_timeout as a 503 (repeatable heavy load). | Add `.max(100)` → 422 CART_TOO_LARGE.

### Observability — authz/log hygiene/metrics

38. **P2 · inconclusive not alerted** | `apps/api/src/workers/reconciliation.ts:97` | Alert fires only when `driftCount > 0` — INCONCLUSIVE results (a check that threw) never alert, so if all 12 checks crash (DB perms/schema drift) the watchdog reports nothing; a schedule failure (`:42-44`) is a console.warn and nothing verifies recon last-ran. | Alert when `inconclusiveCount > 0` too.
39. **P2 · no authz audit trail** | `apps/api/src/server.ts:500` + `apps/api/src/lib/reply-send-error.ts:17-23` | Only status ≥500 is logged/Sentry-captured; `reply.sendError` logs nothing — 403/401 denials leave no audit trail (no actor/resource, just a bare status in the access log), so IDOR probing is undetectable. | Log a structured `authz_denied` line (userId, route, code) for 401/403.
40. **P2 · Sentry optional in prod** | `apps/api/src/server.ts:73-80` + `packages/config/src/index.ts:116` | `unhandledRejection`/`uncaughtException` are kept-alive with console.error + Sentry — but `SENTRY_DSN` is optional and absent from fly.toml, so if unset a half-dead wedged process emits one log line and **no alert ever**. | Verify the secret in prod/staging; boot-time warn when Sentry is disabled in production.
41. **P2 · PII in logs + broken correlation** | `apps/api/src/notifications/workers/index.ts:447` (systemic across `workers/`) | All workers/notifications log via raw `console.*`, bypassing the pino deepRedact/correlationId pipeline — `:447` prints `address=${target.address}` (Telegram chat id) raw, and no bus/pg-boss payload carries the originating correlationId → an order's trace breaks at the queue boundary. | Route worker logging through `createPinoLogger`; propagate correlationId in job payloads.
42. **P2 · metrics dark** | `apps/api/src/lib/metrics.ts:10-12` + fly.toml | `/metrics` is 404 unless `METRICS_TOKEN` is set — fly.toml has no `[metrics]` section and no scraper config exists in-repo, so the pool-saturation/error-rate/WS counters (built for the "menu blinks empty" incident) are likely never scraped. | Add the Fly `[metrics]` stanza + token, or document the external scraper.
43. **P2 · backup-absence invisible** | `apps/api/src/workers/backup/index.ts:189-198` | Intermediate backup retry failures log to console + a DB audit row only; combined with the H6 note (`:222-227`, a past bug silently stopped backups after the first run), nothing checks "time since last successful backup" as a positive signal — absence is indistinguishable from health. | Add a recon check on `max(backup completed_at)` age.

### Rare/edge — rate-limit degradation

44. **P2 · rate-limit silently per-IP** | `apps/api/src/routes/orders.ts:76-79` + `customer/otp.ts:36,114` | Phone-keyed rate-limit keyGenerators read `req.body`, but @fastify/rate-limit runs at `onRequest` (body unparsed) → they all silently degrade to per-IP; the "per-phone" throttle never keys on phone. | Register with `hook: 'preHandler'`, or drop the dead body-key and rely on the DB velocity throttles.

---

## Notably clean (checked, no defect — do not re-audit)

- **FE:** apiClient 401 refresh/redirect scoping; OrderStatusPage 401/403/404 matrix + WS indicator + 30s watchdog; DeliveryPage `completeDelivery` reconcile; courier Shift/Earnings/History loading/error/empty triads; CouriersPage detail error+retry; DashboardPage optimistic status update (reconciles + refetches on failure); MenuPage 404/notFound + fetch-error branches; useWebSocket reconnect-forever with backoff.
- **API:** payments-webhook fail-closed HMAC + insert-wins idempotency + 500-for-retry; order-timeout-sweep (all catches logged); checkout idempotency (tenant-scoped key + hash compare); honest-dispatch zero-courier tenant handling; OTP lockout/429 laddering.

## Cross-cutting themes

1. **Dead alert wires are systemic, not incidental** (findings #1–4, #14, #17) — at least 5 distinct publish channels have zero subscribers or a name/registry mismatch. The bus has no "publish to a channel nobody listens on" guard, and this exact class has recurred (`messaging.ts:80-84` dispatch-failed precedent). *Recommend a startup assertion that every published channel/event name resolves to a registered subscriber + registry entry.*
2. **Silent FE failure is the dominant UX defect** — ~10 handlers `console.warn/error/debug` a failed mutation with no user-visible feedback and (worse, #5/#7/#8) two paths that report success or destroy data on failure. *Recommend a lint rule: a catch in a mutation handler must set error UI or rethrow.*
3. **Time is UTC-on-Fly everywhere** (#10, #11) — venue-local semantics (hours, "today" shift lookups, analytics rollovers) all assume server wall-clock; overnight and non-UTC venues are systematically wrong.
