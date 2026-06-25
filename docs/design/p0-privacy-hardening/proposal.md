# Design Proposal — P0 Privacy Hardening (slug: `p0-privacy-hardening`)

Status: REVISED post-breaker/Counsel (RESOLVE phase) — for Triad Council GO gate
Author: System Architect
Branch: feat/golive-remediation
Companion ADR: `docs/adr/ADR-p0-privacy-hardening.md` · Resolution: `docs/design/p0-privacy-hardening/resolution.md`

> **RESOLVE delta (2026-06-21):** P0-3 design **pivoted** away from "re-fetch in the WS handler" (broke NOTIFY ordering + ran un-scoped on the raw pool) to **minimizing PII at the producer** — no consumer re-fetch. P0-1 client now backs off on 403 instead of hard-stopping; `'assigned'` deliberately excluded. P0-4 `area` re-documented as honest best-effort. P0-2 gets a `NOT VALID` CHECK + a proof-gated GO-gate warning. Two **HUMAN-DECISIONs** to the STOP-ETHICS gate: idle-courier map dot (HD-1), default Telegram detail level (HD-2). See resolution.md.

---

## 1. Problem + Non-goals

Four P0 data-minimization defects in the live system leak or over-expose PII/location data:

- **P0-1** Couriers post GPS to `courier_positions` whenever they hold *any active shift* (`status IN ('available','on_delivery')` — `apps/api/src/routes/courier/shifts.ts:359-362`). There is **no check that the courier is actually on an active delivery**. An on-shift-but-idle courier is continuously tracked, and the position is fanned to the owner live map (`apps/api/src/workers/courier-events.ts:155-164`). Tracking a worker who is not delivering is unjustified surveillance.
- **P0-2** WhatsApp via Baileys (`@whiskeysockets/baileys` 7.0.0-rc13 — `apps/api/package.json:31`) is an unofficial WhatsApp-Web reverse-engineering library. Running it ships customer order data (name, phone, address, totals — `renderWhatsAppMessage`, `apps/api/src/notifications/render.ts:47-71`) through a TOS-violating, ban-prone channel. It is a legal + reliability liability.
- **P0-3** The MessageBus (Postgres `LISTEN/NOTIFY`, `packages/platform/src/message-bus.ts`) carries PII *inside the event payload*. The dashboard fan-out at `apps/api/src/routes/orders.ts:722-737` ships `customerNameMasked`, `customerPhoneMasked`, `itemsSummary` directly in the NOTIFY body. Masking ≠ minimization: the payload still crosses an un-RLS'd transport, is logged verbatim (`message-bus.ts:48` logs full payload), and any subscriber on the channel receives it.
- **P0-4** The Telegram owner alert renders the **full** order body — `deliveryAddress`, `customerPhone` (masked to last-4), full item list, `cashPayWith` (`fetchOrderDetails`, `apps/api/src/notifications/workers/index.ts:488-537`; rendered via `renderTelegramMessage`). The full street address + house number sits unauthenticated in a Telegram chat history forever.

**Goal:** minimize PII at every seam — GPS only while delivering, kill the unofficial channel, strip PII from the bus, strip address/phone from the Telegram body behind an authenticated link.

**Non-goals:**
- Official WhatsApp Business Cloud API integration (out of scope; future ADR).
- Re-architecting LISTEN/NOTIFY → Redis/Kafka (the bus stays Pg; ADR-unchanged).
- Changing the customer-facing live-track UX or the courier mobile GPS cadence beyond a client-side optimization.
- Encrypting `courier_positions` at rest (separate concern; 24h purge already bounds exposure).
- Touching the `order_status` enum or the order state machine values (we *consume* them, never redefine).

---

## 2. Hard Invariants (must hold after the batch)

| Invariant | Source of truth | Enforcement in this batch |
|---|---|---|
| Cash-only, no card PII | `payment_method ENUM ('cash')` `…044710:15` | unchanged |
| No cookies | track-grant → JWT exchange `track.ts` | deep-link reuses JWT, no cookie |
| IPs/phones hashed/masked, never raw | `maskPhone`, track-grant `token_hash` | P0-4 removes phone from default body entirely |
| RLS ENABLE + FORCE per tenant table | `…100051_force-rls.ts`, `orders` `…074262:80-85` | **post-pivot: no new read at the WS seam** — PII minimized at the producer (inside the already tenant-scoped order-create txn); the un-RLS'd raw-pool read the original design implied is removed; migration keeps FORCE |
| Integer money | `subtotal/total integer` `…074262:32-33` | rendering only; no math change |
| RS256 JWT | `issueCustomerToken` / `verifyAuthToken` | deep-link reuses existing RS256 customer JWT |
| Idempotency in Postgres | `idempotency_keys` `…074262:62`, pg-boss `singletonKey` | claim-check re-fetch is idempotent (pure read) |
| Claim-check already used by pg-boss | `NOTIFY_TELEGRAM_SEND` carries `{event, entity_id, location_id}` only `messaging.ts:42-50` | P0-3 keeps PII off the realtime bus too (claim-check *spirit*) but achieves it at the **producer** (minimized payload), not a consumer re-fetch — see RESOLVE pivot |
| Forward-only migrations | discipline; `down()` mostly no-op | new migration is additive + a forward data UPDATE |

**Key existing fact (do not regress):** the **pg-boss notification path is already claim-checked** — `tgSend()` enqueues `{event, entity_id, location_id}` and `NotificationWorker.fetchOrderDetails` re-reads under `location_id` (`workers/index.ts:488-499`). P0-3 is about bringing the **realtime LISTEN/NOTIFY** path (dashboard + customer order channel) to the same standard.

---

## 3. Back-of-envelope

**Scale assumption (human GO gate — adjust if wrong):** launch target ≈ **20 active locations**, peak ≈ **2 orders/min/location aggregate at busy locations**, ≈ **8 couriers concurrently on delivery** system-wide at peak. Connection budget already tracked in `docs/connection-budget.md`.

### 3a. Position-post rate (P0-1) — now driven by a TIME-BASED client loop (R3)
- **Client cadence is now a fixed-interval timer, not the OS watcher** (see §4 P0-1 client fix). `COURIER_GPS_POST_INTERVAL_MS = 12_000` (12s) → **~0.083 post/s/courier** while the courier has an open shift/assignment.
- **Post rate ceiling:** the timer posts at most 1/12s *regardless of how often the OS fires position updates*. This is a **strict upper bound** — the event-driven path could burst faster on a fast-moving courier; the timer caps it. 8 couriers on delivery → **0.67 INSERT/s** peak (~40/min). The previous "0.8/s" event-driven estimate is now the ceiling, not the average.
- **Battery back-of-envelope:** the GPS *radio* is already on continuously (the `watchPosition` watcher in `useGeolocation` runs whenever the delivery page is mounted — the timer does not add a second GPS subscription, it reuses the last-known `position` state). The only added cost is one `fetch()` POST per 12s = **~5 POSTs/min, ~300/hour** — negligible vs the GPS radio itself (the dominant battery draw, unchanged). The timer does **not** increase GPS sampling; it decouples *posting* from *sampling*. No battery regression.
- **While the courier is STATIONARY at pickup** (the exact failure HIGH-2 names): the OS watcher does not fire (`position` unchanged) but the **timer still posts the last-known position every 12s** → a 403 from a lagging `accepted` row is retried within one interval, and tracking resumes within ≤12s of the courier tapping accept — independent of physical movement. This is the structural fix.
- **Today's waste:** idle on-shift couriers also post. P0-1's server guard 403s those posts (they are *not stored*); HD-1 governs whether the idle dot survives. The timer only runs while the page holds an assignment/open shift, so off-shift couriers post nothing.
- **Guard cost:** P0-1 adds **one indexed SELECT per post**. At ≤0.67/s this is trivial. The guard query is `EXISTS (SELECT 1 FROM courier_assignments WHERE courier_id=$1 AND status IN ('accepted','picked_up'))` — **already served** by `courier_assignments_courier_idx ON courier_assignments(courier_id, status)` (`…100041.ts:24`, identical tuple — see §5; **no new index needed**). Sub-millisecond, same txn as the INSERT, no new connection. No cache needed at this scale (caching an assignment-active bit would add invalidation complexity for zero benefit — YAGNI).

### 3b. Bus event rate × subscriber fan-out (P0-3)
- Realtime bus events at peak: order lifecycle (~2 orders/min × ~5 transitions = ~10/min) + position updates (**0.8/s = 48/min**, the dominant term) ≈ **~1 event/s** system-wide.
- Subscribers per channel: dashboard channel (`location:{id}:dashboard`) → WS room handler fan-out to owner browsers; order channel (`order:{id}`) → customer WS + `CourierEventsWorker`; courier channel → owner map. The **dashboard channel is the high-fan-out one** (every owner browser tab on that location).

### 3c. DB-read amplification for P0-3 (post-pivot: ZERO added reads)
- **Post-pivot the dashboard fix adds NO DB reads.** PII is dropped at the producer; the WS room handler fans verbatim (no re-fetch). The original re-fetch design (one read/event) is withdrawn — so the amplification analysis is moot for the dashboard channel: **0 added reads/s.**
- **The position path is ALREADY claim-check-shaped** (`courier-events.ts:155-168` publishes `{courierId, locationId, shiftId}` then re-fetches position from DB — this is a *worker*-side read, not a WS-handler read, and it already exists). The remaining PII leak this batch fixes is the **order.created dashboard payload** (`orders.ts:722-737`), which is fixed by *removing* fields, not by adding a read.
- Net P0-3 effect on the connection budget: **none added** (we delete a payload, we do not introduce a query). The fan-out from NOTIFY → N WebSocket clients remains fully in-process (`websocket.ts:36-44`).

### 3d. Telegram volume (P0-4)
- ~2 orders/min/busy-location → at 20 locations, low tens of Telegram messages/min aggregate, already rate-limited to ~1 msg/s/chat with a circuit breaker (`workers/index.ts:48-53`). P0-4 changes *body content only*, not volume. Adding a track deep-link mint per order.created alert: **1 extra INSERT into `customer_track_grants` per alert** — but order.created **already mints a track grant** for the customer link (`orders.ts:764`), so we **reuse the existing grant**, zero new writes.

---

## 4. The Four Changes — Options + Decisions

### P0-1 — Courier GPS active-delivery guard

**Active-delivery definition (from REAL enum).** Order statuses: `PENDING, CONFIRMED, PREPARING, READY, IN_DELIVERY, DELIVERED, REJECTED, CANCELLED, SCHEDULED, PICKED_UP` (`…044710:14`, mirrored in `packages/domain/src/order-machine.ts:3-14`). The "courier en route" state is **`IN_DELIVERY`** (machine: `CONFIRMED|READY → IN_DELIVERY → DELIVERED`, `order-machine.ts:18-29`). At the assignment level, the courier-side truth is `courier_assignments.status IN ('accepted','picked_up')` (used as active everywhere: `courier-cron.ts:51`, `courier-events.ts:40`). **Active-delivery = the courier has ≥1 `courier_assignments` row in `('accepted','picked_up')`** — this is the assignment-level projection of order `IN_DELIVERY`/`READY`-pickup and is the constant the guard checks.

Define named constants `ACTIVE_DELIVERY_ASSIGNMENT_STATUSES = ['accepted','picked_up']` (single source, shared by the two REST insert sites + the WS path), `COURIER_GPS_RETENTION_HOURS = 24` (replacing the hardcoded `interval '24 hours'` at `courier-cron.ts:32`), and `COURIER_GPS_POST_INTERVAL_MS = 12_000` (client heartbeat interval, R3).

| Option | Concept | Tradeoffs |
|---|---|---|
| **A. Guard query per post (CHOSEN)** | Per-request authorization predicate | +1 indexed EXISTS/post (~0.8/s, trivial §3a); always correct; no staleness. Slight latency add per ping (sub-ms). |
| B. Cache active-assignment bit in Redis/memory, TTL 30s | Read-through cache of the predicate | Avoids a read we can already afford; adds invalidation on assign/unassign → a courier who just got assigned is rejected until TTL expires (correctness bug at the worst moment). Premature optimization vs §3a. |
| C. Drop the shift-status check entirely, rely on client to stop | Trust the client | Violates failure-first: client is not the authority. Rejected — server guard is the hard gate per spec. |

**Decision: A.** Before INSERT (REST sites `shifts.ts:85, :285, :375`; the main ping handler is `:374`), add in the same txn:
```
SELECT 1 FROM courier_assignments
 WHERE courier_id = $1 AND status = ANY($2)  -- ACTIVE_DELIVERY_ASSIGNMENT_STATUSES
 LIMIT 1
```
If absent → `ROLLBACK` + `reply.status(403).send({ error: 'GPS_NOT_ON_ACTIVE_DELIVERY' })` (explicit machine-readable reason so the client backs off). Add index `courier_assignments_courier_status_idx ON courier_assignments(courier_id, status)` (forward migration). The **shift-open insert sites** (`shifts.ts:85, :285` — the position recorded *at shift open/transition*) are a nuance: at shift-open the courier has no assignment yet. Per the privacy goal these initial position snapshots should **also** be gated — drop the courier-positions INSERT from the shift-open/transition handlers entirely (the shift can open without recording a position; the live ping path `:374` resumes tracking once a delivery is active). The ETA/ORS flow is unaffected because it reads the *latest* position which only exists while on delivery (`courier-events.ts:91-99, :173-185`), exactly when we now permit posts.

**Active-set divergence (RESOLVE, recorded — DEV-3).** `'assigned'` (dispatcher-assigned-but-not-yet-consented) is **deliberately excluded** from `ACTIVE_DELIVERY_ASSIGNMENT_STATUSES`. Tracking begins at the courier's **own act of acceptance** (`accepted`), not at dispatcher assignment — autonomy/consent by construction (Counsel dignity lens). This is a deliberate divergence from the broader `('assigned','accepted','picked_up')` active set used elsewhere (e.g. `shifts.ts`); recorded so a future "tidy the enum" refactor cannot silently re-add `'assigned'` to the GPS guard.

**Client GPS-post loop — TIME-BASED heartbeat (RESOLVE R3 FIX — P0-1 HIGH-2).**

*The break (verified live):* `DeliveryPage.tsx:138-153` posts GPS from `useEffect(..., [position])` — i.e. only when the OS geolocation watcher (`useGeolocation`/`watchPosition`, `use-geolocation.ts:32-34`) fires a NEW `position`. The error handler is a bare `.catch((err) => console.debug(...))` (`:150-152`) — **no retry, no backoff, no status inspection**. The R2 "exponential backoff" was *structurally impossible*: backoff needs an independent timer, and there is none. Worse, when the courier is **STATIONARY at pickup** (exactly when `accepted` lags), `position` never changes → the effect never re-fires → an assigned-but-not-yet-accepted courier stays **invisible AND a 403 is never retried**. The recovery mechanism the R2 design promised does not exist in the code.

*The fix — a fixed-interval post loop, independent of physical movement:*
- Replace `useEffect([position])` with a `setInterval` heartbeat at `COURIER_GPS_POST_INTERVAL_MS = 12_000` (named constant) that **re-posts the last-known `position`** (the `useGeolocation` hook keeps the latest fix in React state; the timer reads it — it does not require a fresh OS event).
- The timer runs **only while the courier holds an assignment / open shift** (the delivery page is mounted with an active task) and is cleared on unmount/shift-close → **no idle-courier tracking introduced**: the timer simply does not exist off-shift.
- On `403 GPS_NOT_ON_ACTIVE_DELIVERY` the client does **nothing special** — it just keeps the heartbeat running and the *next* interval naturally retries. Because retry is the steady-state behavior (not a special backoff path), a 403 is retried within ≤1 interval, and tracking resumes **within ≤12s of the courier tapping accept**, even if the courier never moves. No exponential-backoff machinery is needed — the fixed interval IS the retry, and it is already bounded (1 post/12s).
- The bare `console.debug` catch stays (a transient post failure is non-fatal — the next heartbeat covers it), but `403` specifically is expected-and-benign, not an error to surface.

*Why this does not reintroduce idle tracking (reconciles with HD-1):* the timer only fires while the page has an assignment/open shift, and the **SERVER guard still rejects** any post unless `courier_assignments.status IN ('accepted','picked_up')`. So an on-shift-but-idle courier's heartbeat posts are **403'd and discarded — never stored**. The client posting is harmless because the server is the hard gate; off-delivery posts produce a 403 and a dropped row, not a tracked position. HD-1 (does the idle dot survive on the owner map) is governed entirely by the *server* guard + `fetchLatestPosition`, unaffected by the client timer.

*Thundering-herd (NR-2, re-bounded):* with a fixed 12s interval there is no exponential ramp and no synchronized retry storm — each courier posts on its own mount-relative phase, ≤0.083/s/courier, ≤0.67/s for 8 couriers. The guard is a single indexed EXISTS. Add small per-client jitter (±1–2s on the interval start) only if mount-time clustering is observed — not required at launch scale. (`apps/web/src/pages/courier/DeliveryPage.tsx`; constant in `packages/ui` or the courier client config alongside the existing 12s throttle.)

**Courier-facing boundary copy (RESOLVE, Counsel F-1).** Add i18n key `courier.gps_boundary_note` (sq/en/uk) adjacent to `courier.gps_active` (`i18n.ts:388,:1308,:2228`), rendered near the GPS indicator on the courier delivery page: "Your location is shared only during an active delivery — never in your free time." Makes the privacy gate *felt* by the person it protects.

RLS/tenant: guard query runs after `set_config('app.current_tenant', …)` (`shifts.ts:346`), same txn, courier-scoped by `courier_id = request.user.sub`. No isolation change.

**Product-vs-privacy collision (RESOLVE — HUMAN-DECISION HD-1).** The guard deletes the data that feeds an existing, intentional feature: `courier-events.ts:155-164` + `fetchLatestPosition` (`:91-99`) surface **idle on-shift couriers on the owner live map** (explicit comment: "even when idle … the dashboard tracks every on-shift courier"). Once idle couriers stop posting positions, those map dots disappear. This is a genuine product-vs-privacy tradeoff, **not** a silent deletion. **Owner ruling required (STOP-ETHICS gate):** (a) **privacy-max** — accept losing idle-courier dots (idle couriers vanish from the map until they accept a job), or (b) **keep a coarse / last-known idle position** for on-shift couriers (a softer privacy posture). Architect lean: (a), aligning with "do not surveil a worker who is not working" (Counsel justice lens) — but this is the operator's call. Do not delete the feature without a recorded ruling.

### P0-2 — Remove WhatsApp / Baileys

| Option | Concept | Tradeoffs |
|---|---|---|
| **A. Full removal + forward data migration (CHOSEN)** | Decommission a channel | Deletes dep, env, adapter, registration; migrates existing whatsapp targets to a flagged-disabled state so owners are prompted to reconfigure. No silent notification loss. |
| B. Feature-flag off but keep code | Dark-ship retention | Leaves the banned dep in `package.json` (supply-chain + image-size + the legal liability persists). Rejected. |

**Decision: A.** Remove:
- dep `@whiskeysockets/baileys` (`apps/api/package.json:31`) + lockfile entries.
- adapter file `apps/api/src/notifications/channels/whatsapp.ts` and its import + conditional registration (`server.ts:77, :338-340`).
- `renderWhatsAppMessage` (`render.ts:47-71`).
- env `WHATSAPP_ENABLED`, `***REDACTED***` from `packages/config/src/index.ts:23-24` + `.env.example`.
- the `'whatsapp'` literal from the `NotificationTarget.channel` union (`provider.ts:3` — narrow to `'telegram'|'push'`) and the `channel IN ('telegram','whatsapp')` query in the Telegram worker (`workers/index.ts:320`) → narrow to `channel IN ('telegram')`.
- the dispatcher `register('whatsapp', …)` call (`server.ts:338-340`) so the runtime adapter map never holds a whatsapp provider.

**Dangling-union safety (RESOLVE — E-3).** Verified: the worker loop reads `target.channel as string` (`workers/index.ts:333`), so it has **no compile dependency** on the narrowed union → **no as-any cast needed**. The disabled rows are excluded by three independent gates: `status='active'` (already), the narrowed `channel IN ('telegram')` filter, and the new `NOT VALID` CHECK. `renderWhatsAppMessage` (`render.ts:47-71`) is deleted along with its import sites; no remaining reference to `'whatsapp'` survives in TS or runtime.

**Data:** owners with `channel='whatsapp'` targets must NOT silently lose alerts. Forward migration: set those rows `status='disabled'`, `last_error='WHATSAPP_REMOVED_RECONFIGURE'`, `disabled_at=now()`. The owner UI surfaces disabled targets so they re-add Telegram/push. The CHECK constraint added in `…020_notification_channel_whatsapp.ts` is narrowed back to `('telegram','push')` **after** the data is migrated (else the constraint add fails on existing rows). Telegram + push + email channels remain fully intact. Official WA Business API explicitly out of scope.

### P0-3 — MessageBus PII minimization at the producer (highest blast radius)

> **RESOLVE PIVOT (post-breaker, 2026-06-21).** The original Decision A — *re-fetch the row inside the WS room handler* — was withdrawn after the breaker's CRITICAL-1 and CRITICAL-2 were verified against `websocket.ts`. (1) The room handler is **synchronous** (`websocket.ts:36`, `(msg: unknown) => void`) and the bus fans events in NOTIFY FIFO order; an `await`ed DB read inside it reorders the fan-out (ordering break). (2) `websocket.ts` reads on the **raw operational pool** (`fastify.db.query`, no `set_config('app.current_tenant', …)`) which **bypasses RLS** (ADR-006); a re-fetch there is an un-scoped read. **Both criticals dissolve by removing PII at the producer instead of re-fetching at the consumer.** The bus card needs status/total/itemCount/shortId — none of which are PII. See `resolution.md` §B for the full comparison.

**Concept (CHOSEN):** the producer publishes a **minimized non-PII projection**; the bus never carries customer name/phone/address/items-summary at all. The WS room handler stays **exactly as-is** — synchronous, verbatim fan-out, no DB read. This is the **claim-check spirit** (the bus carries no PII; the only path to name/phone is the authenticated owner-reveal route), achieved at the *producer* rather than a consumer re-fetch — strictly simpler and strictly safer than the original.

**COMPLETE producer census — every publisher to a dashboard / order / courier channel** (RESOLVE R3 — the R2 table enumerated only `orders.ts`; the breaker found a SECOND PII producer `orderStatusService.ts`. Below is the full grep-verified census of `messageBus.publish(...)` to any owner/customer-facing channel. The invariant "zero PII on the bus" is now provably met by enumerating ALL producers, not one file.):

| # | Producer (verified line) | Channel | PII today? | Action (R3) |
|---|---|---|---|---|
| 1 | `orders.ts:722-737` (`order.created`) | `dashboard` | **YES** — `customerNameMasked`, `customerPhoneMasked`, `itemsSummary` | **DROP all three** + `courierName`. New body: `{orderId, locationId, status, total, currency, itemCount, shortId, createdAt, seq}`. |
| 2 | **`orderStatusService.ts:108-114` (`order.status` delta) — via `fetchOrderDelta()` :6-29** | `dashboard` | **YES — the missed producer.** `fetchOrderDelta` (`:10-11`) `string_agg`s `quantity×name_snapshot` → `itemsSummary` (full item names, dietary/medical-adjacent) on **EVERY** status transition (CONFIRMED/PREPARING/READY/IN_DELIVERY/...) | **DROP `itemsSummary` from `fetchOrderDelta`'s SELECT and return shape.** Keep `{orderId, status, total, currency, createdAt, shortId, itemCount, statusUpdatedAt}` — all non-PII. `courierName` is already `null` here. The dashboard merges by id, so `itemCount` is sufficient; the item-name list is fetched client-side under the owner's RLS+JWT order-detail route, never the bus. |
| 3 | `orders.ts:713-719` (`order.status`) | `order:{id}` | no — status only | keep as-is; add `seq`. |
| 4 | `orderStatusService.ts:95-101` (`order.status`) | `order:{id}` | no — `{orderId, status, locationId, timestamp}` | keep as-is (no PII). |
| 5 | `order-timeout-sweep.ts:94-99` (`order.status` CANCELLED) | `order:{id}` + `dashboard` | no — `{orderId, status, statusUpdatedAt}` | keep as-is (no PII; verified). |
| 6 | `server.ts:743-745` (`assignment.created`) | `dashboard` | no — `{orderId, courierId, ...}` (verify no name) | keep; **confirm** the published body carries no courier/customer name (it publishes `orderId`+meta only — verified at :744-745). |
| 7 | `courier-events.ts:188-198, 223-233` (`order.courier_updated`) | `order:{id}` | masked `courierName`+`phoneMasked` (DB-sourced) | **KEEP** — operationally required for the customer live-track; this is the *courier's* masked name+phone, DB-sourced by the worker (claim-check-shaped: trigger event carries only `{courierId,locationId,shiftId}`), and it rides the per-order channel the customer is authorized for, not the dashboard. Out of P0-3's "customer-PII off the bus" scope. (HD-1 governs whether the *idle* courier dot survives at all.) |
| 8 | `courier-events.ts:160-163, 217-220` (`courier.position_updated` / `assignment_status_changed`) | `couriers` | no customer PII — `{courierId, position}` | keep (position is the feature; gated by P0-1 + HD-1). |
| 9 | `dwell-monitor.ts:106-108`, `dwell-escalation.ts:184`, `signal-raiser.ts:126-128`, `anonymizer-gdpr.ts:80-82`, `lifecycle-handlers.ts` | `dashboard` | no — `{alertId, orderId, kind, severity}` / `{signalId, customerId, kind}` / `{requestId, customerId}` | **KEEP** — carry **opaque IDs** (`orderId`/`customerId`/`alertId`/`signalId`), not names/phones/addresses/items. `customerId` is a UUID, not PII-in-the-clear. Verified no name/phone/address/itemsSummary in any of these bodies. |
| 10 | `BUS_CHANNELS.ORDER_*` (`orders.ts:704-711`, `orderStatusService.ts:118-122`, `courier-dispatch.ts`) | internal lifecycle | no — `{orderId, locationId, status, total, currency}` | downstream re-reads by id (`messaging.ts:42-50`); optionally drop `total`/`currency` (the pg-boss worker re-reads them) — cosmetic, no PII. |

**Verdict of the census:** exactly **TWO** producers carried customer PII onto the bus — `orders.ts:722-737` (R2) and **`orderStatusService.ts:108-114`/`fetchOrderDelta` (R3-new)**. Both are now minimized at the producer. Every other publisher carries status, opaque IDs, position, or the *courier's* own masked contact on the per-order channel — none carries customer name/phone/address/item-names. Zero-PII-on-the-bus is met by complete census.

**`message-bus.ts:48` verbatim payload logging (HIGH-1 sub-point):** `this.listenerClient.on('notification', (msg) => { console.log('[PgMessageBus] ✓ Received notification on:', msg.channel, 'payload:', msg.payload); ...})` logs the **full NOTIFY payload verbatim**. Today this leaks `itemsSummary`+masked name/phone into stdout logs on every order.created AND every status transition. **After the two producer fixes above the payload is already non-PII**, so the log line no longer leaks. **Confirmed disposition:** with the producers minimized, the verbatim log is non-PII and may stay. As **defence-in-depth** (the log line is one careless future producer away from leaking again), demote it to truncated/structured: log `msg.channel` + `payload.length` at info, full payload only at `debug`. The proof (§9) asserts the dashboard NOTIFY body contains none of the PII keys at the producer; the log inherits that guarantee. (Note: `logger.ts:18` + `sentry.ts:13` already redact `customer_name`/`customer_phone`/`customer_address` — but this `console.log` bypasses the structured logger entirely, hence the demotion.)

**Options:**

| Option | Concept | Tradeoffs |
|---|---|---|
| **A. Minimized non-PII projection at the producer (CHOSEN)** | Drop PII before it ever hits the bus | Zero added reads; WS handler unchanged (sync/verbatim) → **ordering preserved**, **no un-scoped read** (both criticals dissolve); one producer line changes. Cost: dashboard loses live name/phone/items on a brand-new order until the next owner-list fetch (NR-1, R7). |
| B. Re-fetch at the WS room-handler boundary (ORIGINAL, REJECTED post-breaker) | Claim-check at the fan-out seam | **Ordering break** — the handler is synchronous (`websocket.ts:36`); an awaited read reorders fan-out. **No tenant scope** — `fastify.db` raw pool bypasses RLS (ADR-006), no `set_config`. Rejected on CRITICAL-1 + CRITICAL-2. |
| C. Re-fetch inside each subscriber/per-member | Claim-check per consumer | **N+1 fan-out** (read per browser tab). Rejected. |
| D. Encrypt the PII fields in the NOTIFY payload | Keep payload, protect it | Still ships data over an un-RLS'd transport + into logs; key management; doesn't satisfy minimization. Rejected. |

**Decision: A — applied at BOTH PII producers (R3).**
- **Producer 1 — `orders.ts:722-737`** (`order.created`): publish `{ type:'order.created', data: { orderId, locationId, status, total, currency, itemCount, shortId, createdAt, seq } }` — **dropping `customerNameMasked`, `customerPhoneMasked`, `itemsSummary`, `courierName`**. Delete the now-unused `itemsSummary` computation at `orders.ts:721` and the masking expressions at `:734-735`.
- **Producer 2 — `orderStatusService.ts:108-114` via `fetchOrderDelta` (`:6-29`)** (`order.status` delta, R3-new): **remove the `items_summary` `string_agg` subquery from the SELECT (`:10-11`)** and the `itemsSummary` field from the returned object (`:26`). The published delta becomes `{ orderId, status, total, currency, createdAt, shortId, itemCount, statusUpdatedAt }` — non-PII. `courierName` is already `null`. The owner dashboard merges deltas by id (`itemCount` suffices for the card); the item-name list is fetched client-side under the owner RLS+JWT order-detail route, never the bus.

The dashboard WS room handler (`websocket.ts:36-44`) is **unchanged** — it continues to fan the message verbatim; there is **no re-fetch, no async, no DB read at the seam**, so NOTIFY ordering is preserved and no un-scoped read occurs. The masking helpers in `orders.ts:734-735` are **deleted** (PII never leaves the DB tier). `seq` is a per-process monotonic counter for client gap-detection only (no DB, no sequence table). Customer name/phone reach the owner only via the existing RLS+JWT order-list fetch and the authenticated owner-reveal route (`ownerRevealContactRoutes`, `server.ts:606`) — never on the bus.

**Client handling of the cost (NR-1 / R7):** the live card renders status/shortId/total/itemCount/createdAt from the projection; customer name/phone/items appear via the owner's normal order-list fetch (already RLS+JWT scoped) on the next poll/refresh or when the order is opened. Any dashboard name-search for in-flight orders falls back to an **on-demand authenticated fetch**, never bus data; until then the card shows a masked placeholder ("New order #ABCD"). This is a deliberate, recorded UX degradation (R7), not a regression.

**Ordering/correctness:** with no re-fetch, the bus is once again a pure pass-through and Pg `NOTIFY` FIFO-per-channel ordering is the only ordering concern — unchanged from today. The `_truncated` fallback in `serializeForNotify` (`message-bus.ts:137-151`) becomes mostly dead for this channel since the minimized payload is tiny — a bonus.

### P0-4 — Minimize Telegram alert body

**Concept:** default body carries **non-identifying** order facts + an **authenticated deep-link**; full address/phone live only behind the link (RS256 JWT via the existing track-grant exchange).

Today `fetchOrderDetails` (`workers/index.ts:488-537`) returns full `deliveryAddress` + masked phone + full item list; `renderTelegramMessage` (`render.ts:73`) renders them and attaches an `Open in app` button to `app.dowiz.org/admin/...` (an **owner-app** link, which is fine — but the *body itself* leaks the address).

| Option | Concept | Tradeoffs |
|---|---|---|
| **A. District/street-without-house + item count/total + authenticated owner deep-link (CHOSEN)** | Minimized body + auth link | Owner can still triage (#order, total, item count, rough area) from chat; precise address/phone only after clicking into the RLS+JWT-protected app. Requires parsing address to strip house number (best-effort) OR omit address entirely when parsing is uncertain. |
| B. Order # + total only, no address at all | Maximal minimization | Safest, but owner loses at-a-glance dispatch context (which area). Offer as the config default if address parsing is unreliable. |
| C. Keep full body but only in "private" chats | Channel-type gating | Telegram chat history is forever regardless of "private"; doesn't solve retention. Rejected. |

**Decision: A, with B as a configurable level.** Add a per-location config `telegram_alert_detail` ∈ `{'minimal','area','full'}` (owner setting), default **`'area'`**:
- `minimal`: `#ABCD · 3 items · 1.200 ALL` + deep-link.
- `area` (**best-effort**, default): adds district/street name **with house number stripped** (regex: drop trailing house-number tokens; if the address can't be confidently split, fall back to `minimal` — never emit the raw address by accident).
- `full`: legacy behavior (opt-in, for owners who dispatch entirely from chat and accept the retention risk — an **accepted risk**, see §10).

**Honesty correction (RESOLVE FIX, post-breaker — P0-4 HIGH).** There is **no structured address field** — addresses are free-text, and Albanian addressing is notoriously unstructured ("te ura e Tabakëve, kati 2, mbi farmaci"). For the *majority* of real addresses the house-number-strip regex **cannot confidently split**, so `area` falls back to `minimal`. Therefore **`area` is, in practice, mostly `minimal`** and must be named/documented honestly: it is **best-effort** and shows a coarse area *only when one can be safely extracted*, otherwise just order# + total. Owner copy states this plainly; no mode pretends to do more than it does. (R1.)

**Default level is a HUMAN-DECISION (HD-2, STOP-ETHICS gate).** Because best-effort `area` is mostly-`minimal` on real addresses, the *effective* default materially changes the solo-operator dispatch workflow (owners who today read the street from the chat body will, on most orders, now have to deep-link into the app per order — see Counsel §4 steel-man). The **default level** (`minimal` vs `area` vs whether `full` may be a default) is therefore an **owner ruling**, because it affects existing owners' live workflow. Architect recommendation: default `area` (best-effort) and **measure the `full`-opt-in rate as the canary** — mass flight to `full` means `area` failed as a *usable* default, not that privacy was too aggressive.

Phone is **removed from the default body** in all levels except `full`. The **deep-link** is the *existing* owner-app URL pattern already produced by `renderTelegramMessage` (`render.ts:78-79`, `…/admin/locations/{loc}/orders/{order}`), which lands in the **JWT-gated owner SPA** — already authenticated, already RLS-scoped. We do **not** put the customer track-grant code into the owner alert (that grant authenticates *as the customer* — wrong principal). The owner clicks → owner SPA → owner JWT → owner sees full address/phone via the existing reveal flow. So "full detail behind an authenticated link" is satisfied by the **owner-app deep-link**, requiring no new auth mechanism.

`fetchOrderDetails` is modified to compute `districtStreet` (house-number-stripped) and to **omit** `deliveryAddress`/`customerPhone` from the rendered vars unless detail level is `full`.

---

## 5. Data / Migrations (forward-only, RLS FORCE)

One new forward-only migration `17xxxxxxxxxx_p0-privacy-hardening.ts` (timestamp after `…040`):

1. **Index for P0-1 guard — DROPPED FROM SCOPE (R3, MED NR/redundant).** The R2 design added `courier_assignments_courier_status_idx ON courier_assignments(courier_id, status)`. Verified `…1780421100041_courier-assignments.ts:24` **already** creates `courier_assignments_courier_idx ON courier_assignments(courier_id, status)` — **identical column tuple**. The EXISTS guard is already index-served today. **No new index is added** (a `CREATE INDEX IF NOT EXISTS` would silently no-op and falsely credit this batch with work it did not do). The §3a "guard is cheap" claim now correctly cites the *pre-existing* index `…100041:24`.
2. **P0-2 data migration (order matters) — RESOLVE REVISION (Counsel A-1 / DEV-1):**
   - `UPDATE owner_notification_targets SET status='disabled', last_error='WHATSAPP_REMOVED_RECONFIGURE', disabled_at=now() WHERE channel='whatsapp';`
   - then add a **`NOT VALID`** CHECK so the DB rejects *new* whatsapp rows while tolerating the existing disabled ones:
     ```
     ALTER TABLE owner_notification_targets
       ADD CONSTRAINT owner_notification_targets_channel_not_whatsapp
       CHECK (channel IN ('telegram','push')) NOT VALID;
     ```
   - **Do NOT** run `VALIDATE CONSTRAINT` — Postgres skips pre-existing rows for a `NOT VALID` constraint, so the disabled `'whatsapp'` rows survive while every new/updated row is enforced. This **restores the schema-level invariant** ("no new whatsapp, ever") without deleting owner config and without violating forward-only/non-destructive — upgrading DEV-1 from "app-layer is the only writer forever" (an assumption that rots) to a DB-enforced guarantee. The original broad-CHECK-only approach is withdrawn. New risk NR-3 (a future `VALIDATE` pass would fail) handed to the breaker; the constraint name encodes the intent and the ADR records "never VALIDATE." *(DEV-1 updated in ADR.)*
3. **P0-4 config column:** `ALTER TABLE locations ADD COLUMN IF NOT EXISTS telegram_alert_detail text NOT NULL DEFAULT 'area' CHECK (telegram_alert_detail IN ('minimal','area','full'));` (forward-only, default safe).

No RLS policy is added/removed; all touched tables (`courier_assignments`, `owner_notification_targets`, `locations`) already have RLS ENABLE+FORCE. The new index and column inherit existing policies. `down()` stays a no-op per discipline (forward-only).

**Named constants (code, not migration):** `ACTIVE_DELIVERY_ASSIGNMENT_STATUSES = ['accepted','picked_up']` and `COURIER_GPS_RETENTION_HOURS = 24` centralized in `apps/api/src/lib/registry.ts` (alongside `BUS_CHANNELS`); `courier-cron.ts:32` interpolates the constant. `COURIER_GPS_POST_INTERVAL_MS = 12_000` lives client-side (courier web app) where the heartbeat runs.

---

## 6. Consistency + Idempotency

- **P0-1** guard + INSERT in one txn (existing `BEGIN/COMMIT`, `shifts.ts:345-384`). No partial state. Idempotent: a duplicate ping just inserts another position row (append-only series, bounded by 24h purge) — no idempotency key needed (positions are a stream).
- **P0-3** (post-pivot) the bus carries a smaller, non-PII payload and the WS seam stays a pure pass-through → no new state, no idempotency concern introduced. NOTIFY at-most-once delivery is unchanged; the existing reconnect logic (`message-bus.ts:98-111`) plus the pg-boss durable path (`orders.ts:688` transactional enqueue) cover realtime gaps — the bus was always best-effort, and the authoritative copy is the DB row served by the RLS+JWT order-list/reveal routes (not the bus).
- **P0-4** track grant is reused (idempotent; `use_count` is observability only, `track.ts:66-71`). The Telegram job already has `singletonKey`/`dedupKey` (`messaging.ts:43-49`) — unchanged.
- **P0-2** data migration is a single idempotent UPDATE (re-runnable; sets terminal `disabled` state).

---

## 7. Failures + Degradation (failure-first)

| Seam | Failure | Detection | Degradation (no cascade) |
|---|---|---|---|
| P0-1 guard SELECT | guard query errors | txn ROLLBACK | reject the ping `503`/`409`; client retries with backoff; **no** position written (fail-closed — safer to drop a ping than to bypass the privacy gate). |
| P0-1 guard | courier legitimately on delivery but assignment row lags | EXISTS returns 0 | brief tracking gap until assignment commits; acceptable (seconds). ETA degrades to last-known position. |
| P0-3 WS fan-out (post-pivot: no read) | bus dispatch error | `dispatch` swallows handler errors (`message-bus.ts:165-178`) | client gets the next event or polls; pg-boss path notified the owner independently; no DB read at the seam to fail. |
| P0-3 dashboard card | new order's PII not yet on screen (no bus PII) | n/a (by design) | card shows shortId/total/itemCount live; name/phone via the next RLS+JWT order-list fetch / on-demand search. (R7) |
| P0-4 deep-link | owner SPA down | owner sees minimized body in chat | owner still has #order + total + area to act; full detail when app returns. Body is self-sufficient for triage. |
| P0-4 address parse | house-number strip ambiguous | parser returns null | **fall back to `minimal`** (omit address) — never emit raw address by accident (fail-closed to more-private). |
| P0-2 removal | an owner had only a whatsapp target | target now `disabled` | owner gets the in-app reconfigure prompt; **no silent loss** — but they DO miss alerts until they re-add a channel. Mitigation: ops should notify affected owners pre-deploy (operability §9). |

Failure-first principle honored: every guard and every claim-check read **fails closed toward more privacy** (drop the ping / drop the broadcast / drop the address), never toward leaking.

---

## 8. Security + Tenant Isolation

- **P0-3 (post-pivot): no read at the WS seam.** The dashboard payload is minimized **at the producer** inside the authenticated, tenant-scoped order-create txn (`orders.ts`); the bus carries no PII and the WS room handler performs **no DB read** (it fans verbatim). This removes the un-scoped-read risk the original design carried (`websocket.ts` reads on the raw operational pool with no `set_config('app.current_tenant', …)` — RLS-bypassed per ADR-006; we therefore do **not** read there). Tenant isolation for the dashboard now rests on (a) the producer only emitting non-PII facts for the location it just wrote, and (b) the existing room authz (`ownerCanAccessRoom`, `websocket.ts:82-109`) gating *who* receives the fan-out. PII (name/phone) is served only by the RLS+JWT order-list / owner-reveal routes, each of which *does* set tenant scope. No PII transits an un-RLS'd path.
- **Deep-link auth (P0-4):** the owner alert link targets the owner SPA, which requires the owner RS256 JWT and runs all reads under the owner's membership-scoped RLS. No unauthenticated PII exposure: the Telegram body itself is now PII-free (no house number, no phone) and the only path to full detail is JWT-gated. We deliberately do **not** embed a customer track-grant in an owner alert (principal mismatch — a track grant authenticates as the *customer*).
- **P0-1:** courier can only post their own position (`courier_id = request.user.sub`), guard is courier-scoped, tenant-scoped via `app.current_tenant`. No cross-courier or cross-tenant read.
- **P0-2:** removes a TOS-violating egress of customer PII to a third-party (Meta via reverse-engineered protocol) entirely.

---

## 9. Operability

- **Health:** no new external dependency added (P0-2 removes one). Bus health unchanged (`checkHealth`, `message-bus.ts:234`). P0-1 guard failures should increment a counter so a spike (e.g. assignment-table lag) is visible <1 min — log `GPS_NOT_ON_ACTIVE_DELIVERY` rejects with courier_id at debug, count at info.
- **Observability:** post-pivot P0-3 adds **zero** DB reads (§3c) — no pool-budget impact. Position-post reject rate (`GPS_NOT_ON_ACTIVE_DELIVERY`) is the canary for the P0-1 client rollout; `full`-opt-in rate is the canary for whether `area` is a usable default (HD-2 / R10).
- **Rollback:** P0-1 and P0-4 are behavior changes guarded by code; revert is a code revert (no destructive migration). P0-3 revert is a one-line producer revert (re-add the PII fields) — the WS handler was never changed, so there is nothing else to undo. The migration is forward-only and non-destructive (index + nullable-defaulted column + idempotent UPDATE + `NOT VALID` CHECK); rolling back code without rolling back the migration is safe. Note: the `NOT VALID` CHECK must **never** be `VALIDATE`d (NR-3).
- **Flag / scaling gate:** P0-4 ships behind the `telegram_alert_detail` column (default `'area'`); an owner who needs `full` flips it themselves.
- **GO-gate MUST (RESOLVE, Counsel F-2 — promoted from "should"):** P0-2 owner-impact is now a **proof-gated, result-pasted pre-deploy checklist item**, not a "should." The deploy does **not** proceed until: (a) the query below is run against the **prod** DB, (b) its result is **pasted into the deploy record**, and (c) every affected owner has been warned **and** has at least one other active channel OR has been individually contacted.
  ```
  SELECT location_id, address FROM owner_notification_targets
   WHERE channel='whatsapp' AND status='active';
  ```
  This binds the single cell where the batch can silently lose a real owner's order. Procedural, not code.
- **Proof (Mandatory Proof Rule):** P0-1 → Playwright/integration asserting a `403 GPS_NOT_ON_ACTIVE_DELIVERY` for an idle courier and `200` once an assignment is `accepted`, **plus** an assertion that the client retries (backs off) rather than hard-stopping after a 403. P0-3 → integration asserting **BOTH** dashboard producers emit no PII: (1) the `order.created` payload from `orders.ts` AND (2) the `order.status` delta payload from `orderStatusService.ts`/`fetchOrderDelta` (assert on a real status transition) contain **no** `customerName*`/`customerPhone*`/`itemsSummary`/`courierName` keys, and the WS client still receives `shortId`/`total`/`itemCount`; **and** that `websocket.ts` room handler performs no DB query (the seam stays read-free). P0-1 (client) → Playwright/integration asserting the courier delivery page re-posts GPS on a **fixed interval while stationary** (`position` unchanged) and resumes a `200` within one interval after the assignment flips to `accepted` (i.e. a 403 is retried by the heartbeat, not by movement). P0-4 → assert the Telegram render output contains no house number / phone at `area` level. P0-2 → build + a **scoped** grep proving no `baileys` / `WHATSAPP_` / `renderWhatsAppMessage` references remain, plus a grep for the **notification-channel** `'whatsapp'` literal scoped to `apps/api/src/notifications/**` + `provider.ts` (the Baileys/channel surface). The grep must **EXCLUDE two legitimate non-Baileys survivors** (R3, MED — verified live): (a) `apps/api/src/routes/courier/me.ts:79` — `messenger_kind: z.enum(['telegram','whatsapp','viber'])`, the courier's customer-facing **click-to-chat** handle (official `wa.me` deep-link, migration `…038_messenger-deeplink.ts`), NOT Baileys, must stay; (b) `apps/api/src/lib/spa-shell.ts:14` — `whatsapp` in the `BOT_UA` link-preview-crawler regex, must stay. A bare repo-wide `grep 'whatsapp'` returns these 2 hits and would falsely fail the proof or trigger a real regression if an engineer deletes them. Predicate: `rg "whatsapp" apps/api/src/notifications apps/api/src/notifications/provider.ts` → zero hits. **and** a migration test that an INSERT with `channel='whatsapp'` is rejected by the `NOT VALID` CHECK while existing disabled rows survive. F-1 → Playwright asserting `courier.gps_boundary_note` is visible on the courier delivery page.

---

## 10. Open / Accepted Risks

| # | Risk | Disposition | Owner |
|---|---|---|---|
| R1 | Address house-number stripping is locale-specific (Albanian addressing) and may mis-parse | **Accepted + fail-closed**: ambiguous parse → `minimal` (omit address). Refine regex post-launch with real data. | Architect → Backend |
| R2 | Owners on `full` detail still leak address/phone into Telegram history | **Accepted risk** (explicit owner opt-in; default is `area`). Documented in owner UI copy. | Product |
| R3 | P0-2 leaves disabled `channel='whatsapp'` rows | **RESOLVED via `NOT VALID` CHECK** (Counsel A-1): DB now rejects new whatsapp rows while tolerating disabled ones; invariant is DB-enforced, not app-layer-only. Caveat NR-3 (never `VALIDATE`). | Architect |
| R4 | Owners with *only* a whatsapp target miss alerts until reconfigure | **GO-gate MUST** (Counsel F-2): proof-gated pre-deploy query + warning, result pasted into deploy record. Residual accepted. | Ops |
| R5 | Brief GPS gap when assignment row lags the courier's accept | **Accepted** (seconds; ETA degrades to last-known). | Architect |
| R6 | Claim-check re-fetch on a hot dashboard channel under a future 10× load | **Moot for the dashboard** post-pivot (no re-fetch). Retained note for other DB-sourced realtime paths (`courier-events` worker reads). | Architect |
| R7 | **(NR-1)** Dashboard live card lacks customer name/phone/items on a brand-new order until the next owner-list fetch (P0-3 pivot cost) | **Accepted + handled**: card shows shortId/total/itemCount live; PII via RLS+JWT order-list fetch / on-demand search; masked placeholder until then. Breaker to re-attack any code reading `customerNameMasked`/`itemsSummary` off the bus. | Architect → Frontend |
| R8 | **(NR-2, R3-rebounded)** Courier client heartbeat could hammer the guard for a never-assigned idle courier | **Accepted + structurally bounded**: a *fixed* 12s interval (`COURIER_GPS_POST_INTERVAL_MS`), no exponential ramp, ≤0.083/s/courier, ≤0.67/s for 8 couriers; guard is a single index-served EXISTS. Timer only runs while the page holds an assignment/open shift. Optional ±1-2s jitter if mount-clustering observed. | Architect → Frontend |
| R11 | **(NR-1 search, R3)** Live customer-name / item search on the dashboard misses in-flight orders whose card came from the bus (name/items now empty until a full order-list fetch backfills) — the "on-demand authenticated search fallback" the R2 design referenced **does not exist in code** (`DashboardPage.tsx:257` is a pure client-side `.filter()` over local state). | **HUMAN-DECISION-adjacent → owner=Product.** Two options stated: (a) **build it in scope** — wire the dashboard search box to the existing owner order-search endpoint (RLS+JWT) as a debounced server fallback when local state lacks a match; or (b) **accept-risk** — live name/item search misses in-flight orders until the next interval poll/reload backfills the card (seconds on a short poll). Architect lean: **(b) accept-risk** for launch (the card backfills within the existing dashboard poll; search-for-an-order-that-just-arrived is a narrow window), defer (a) to a follow-up if owners report it. **Product owns the ruling.** | Product |
| R12 | **(R3)** Two overlapping channel CHECK constraints on `owner_notification_targets` — the pre-existing validated `_channel_check` (permits whatsapp) AND the new `_not_whatsapp NOT VALID` (forbids it). A future "tidy constraints" pass that drops the redundant-looking `_not_whatsapp` silently re-opens whatsapp writes. | **Accepted + documented** (footgun, MED). The constraint name encodes intent; ADR + this row record that `_not_whatsapp` is load-bearing and must not be dropped, and `_channel_check` must not be the basis for "whatsapp is allowed." We deliberately do NOT ALTER `_channel_check` (forbidden — it would VALIDATE-fail on disabled rows). | Architect |
| R9 | **(NR-3)** Future migration running `VALIDATE CONSTRAINT` would fail on disabled whatsapp rows | **Accepted**: constraint name encodes intent; ADR records "never VALIDATE"; verify tooling does not auto-validate on boot. | Architect |
| R10 | **(NR-4)** Honest `area` copy may push owners to `full` faster (more PII to chat) | **Accepted + measured**: `full`-opt-in rate is the canary (R1). Feeds HD-2. | Product |

**HUMAN-DECISION items (STOP-ETHICS gate — see resolution.md §H). Counsel R2 recommendations are folded in as the PROPOSED DEFAULTS; the human still rules at the gate:**
- **HD-1 (P0-1 MED):** idle-courier owner-map dot. **PROPOSED DEFAULT (Counsel R2 + Architect lean): (a) privacy-max** — accept losing idle-courier map dots; an on-shift-but-idle courier vanishes from the owner map until they accept a job. **Operational loss documented:** a dispatcher loses live visibility of *which idle couriers are roughly where* (note: with the server guard, idle posts are dropped, so `fetchLatestPosition` returns the last pre-idle row until the 24h purge — dots go **stale, then disappear at purge**, not instantly). Alternative (b) keep coarse/last-known idle position is acceptable **only if** genuinely degraded (last-known point, neighborhood-level, no fresh idle polling) **and** the F-1 courier copy is updated to disclose it. **Pending owner ruling.** Do not silently delete `courier-events.ts:155-164` + `fetchLatestPosition`.
- **HD-2 (P0-4):** default `telegram_alert_detail` level. **PROPOSED DEFAULT (Counsel R2 + Architect): `area` (best-effort), honestly degrading to `minimal`** on free-text Albanian addresses that can't be safely split. `full` remains an explicit opt-in (R2). **Measure the `full`-opt-in rate as the canary** — mass flight to `full` means `area` failed as a *usable* default, a usability bug to fix, not a privacy retreat. **Pending owner ruling** (it changes existing owners' dispatch workflow).

**DEFER-FLAG (future, not this batch):** Counsel A-2 (disabled-target prompt as future WA-Business re-onboarding surface); Counsel open question (courier as data subject of own movement — transparency parallel to customer privacy copy, future ADR).

---

## Appendix — Code anchors (verified)

- Order status enum: `packages/db/migrations/1780310044710_extensions-and-enums.ts:14`; machine `packages/domain/src/order-machine.ts:3-29` (active = `IN_DELIVERY`).
- Courier position INSERTs: `apps/api/src/routes/courier/shifts.ts:85, :285, :374`; ping handler context `:328-388`; shift gate `:359-362`.
- 24h purge: `apps/api/src/workers/courier-cron.ts:32` (hardcoded `interval '24 hours'`), scheduled `:21`.
- Active-assignment statuses in use: `courier-cron.ts:51`, `courier-events.ts:40`.
- MessageBus: `packages/platform/src/message-bus.ts` (publish `:113`, truncate `:137`, dispatch `:165`).
- Realtime producers w/ customer PII (the COMPLETE set — exactly two): `apps/api/src/routes/orders.ts:722-737` (`order.created` dashboard — masked name/phone + itemsSummary) and `apps/api/src/lib/orderStatusService.ts:108-114` via `fetchOrderDelta:6-29` (`order.status` delta dashboard — `itemsSummary` on every transition). Verbatim payload log: `packages/platform/src/message-bus.ts:48`.
- §9 grep false-positive survivors (must NOT be removed): `apps/api/src/routes/courier/me.ts:79` (`messenger_kind` wa.me click-to-chat); `apps/api/src/lib/spa-shell.ts:14` (`BOT_UA` crawler regex).
- Pre-existing P0-1 guard index (no new index needed): `packages/db/migrations/1780421100041_courier-assignments.ts:24` `courier_assignments_courier_idx(courier_id, status)`.
- Client GPS poster (HIGH-2): `apps/web/src/pages/courier/DeliveryPage.tsx:138-153` (event-driven, bare catch); geolocation source `packages/ui/src/hooks/use-geolocation.ts:32-34` (`watchPosition`, last-known in state).
- Bus subscribers: `apps/api/src/bootstrap/messaging.ts`, `apps/api/src/workers/courier-events.ts:16-19`, `apps/api/src/workers/lifecycle-handlers.ts:26-36`, WS room handler `apps/api/src/websocket.ts:36-47`.
- pg-boss claim-check (existing): `messaging.ts:42-50` → `apps/api/src/notifications/workers/index.ts:488-537`.
- WhatsApp seam: `apps/api/src/notifications/channels/whatsapp.ts`, registration `apps/api/src/server.ts:77, :338-340`, render `apps/api/src/notifications/render.ts:47-71`, env `packages/config/src/index.ts:23-24`, dep `apps/api/package.json:31`, worker query `apps/api/src/notifications/workers/index.ts:320`, channel union `apps/api/src/notifications/provider.ts:3`.
- Telegram body: `apps/api/src/notifications/render.ts:73-150`, data `apps/api/src/notifications/workers/index.ts:488-537`.
- Track-grant / auth deep-link: `packages/db/migrations/1790000000026_customer-track-grants.ts`, `apps/api/src/routes/customer/track.ts`, owner deep-link `render.ts:78-79`.
- owner_notification_targets schema: `…1780348982032`, whatsapp CHECK `…1790000000020`.
