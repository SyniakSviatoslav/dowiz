# ADR — P0 Privacy Hardening (4-change batch)

- Status: PROPOSED — REVISED post-breaker/Counsel (RESOLVE phase) (Triad Council GO gate)
- Date: 2026-06-21
- Slug: `p0-privacy-hardening`
- Design: `docs/design/p0-privacy-hardening/proposal.md` · Resolution: `docs/design/p0-privacy-hardening/resolution.md`
- Supersedes/affects: extends the claim-check posture of the pg-boss path (`bootstrap/messaging.ts` + `notifications/workers/index.ts`) to the realtime LISTEN/NOTIFY bus; does not change ADR-006 (operational role RLS bypass) or the LISTEN/NOTIFY transport choice.

## Context

Four P0 data-minimization defects in production: (1) couriers tracked while on-shift but not delivering; (2) customer PII egressed to Meta via the unofficial Baileys WhatsApp library; (3) PII (masked name/phone, item summary) carried inside MessageBus NOTIFY payloads — from **two** producers: `orders.ts` (order.created) and `orderStatusService.ts`/`fetchOrderDelta` (order.status delta on every transition); (4) full delivery address + phone rendered into permanent Telegram chat history. All four are launch blockers.

## Decisions

### D1 — Courier GPS active-delivery guard (P0-1)
- "Active delivery" = courier has ≥1 `courier_assignments` row with `status IN ('accepted','picked_up')` — the assignment-level projection of order status `IN_DELIVERY` (enum `1780310044710:14`; machine `order-machine.ts`). Named constant `ACTIVE_DELIVERY_ASSIGNMENT_STATUSES`.
- Before every `INSERT INTO courier_positions`, run an indexed `EXISTS` guard in the same txn; on miss → `403 GPS_NOT_ON_ACTIVE_DELIVERY` + ROLLBACK (fail-closed).
- Drop the position INSERT from the shift-open/transition handlers (`shifts.ts:85, :285`) — no assignment exists at shift open, and per the privacy goal idle snapshots are not justified.
- **No new index (R3).** The guard's EXISTS is already served by the pre-existing `courier_assignments_courier_idx ON courier_assignments(courier_id, status)` (`…1780421100041:24`, identical tuple). The R2 "new index" is dropped from scope — adding it would be a misleading no-op. Caching the predicate (Option B) is rejected (read is <1/s; caching introduces an invalidation correctness bug at assignment time).
- `COURIER_GPS_RETENTION_HOURS = 24` named constant replaces the hardcoded interval at `courier-cron.ts:32`.
- **Client GPS post is a TIME-BASED heartbeat (RESOLVE R3 FIX — HIGH-2), not event-driven.** The R2 "back off on 403" was structurally impossible: `DeliveryPage.tsx:138-153` posts from `useEffect([position])` (fires only on a new OS fix) with a bare catch and no timer — a courier **stationary at pickup** (when `accepted` lags) never re-fires the effect → stays invisible, 403 never retried. **Fix:** a `setInterval` at `COURIER_GPS_POST_INTERVAL_MS = 12_000` re-posts the **last-known** position (held in `useGeolocation` state) while the page holds an assignment/open shift; cleared on unmount. A 403 is retried by the next interval (retry is steady-state) → tracking resumes within ≤12s of accept, **independent of physical movement**. Battery: ~5 POSTs/min, no extra GPS subscription (decouples posting from sampling) — negligible. **No idle tracking introduced:** the server guard still 403s+discards any post while not `accepted`/`picked_up`; the timer only runs on-shift. NR-2 rebounded to a fixed interval (no exponential storm). Server guard remains the hard gate. Counsel F-1: courier-facing i18n line (`courier.gps_boundary_note`, sq/en/uk) states the tracking boundary.

### D2 — Remove WhatsApp / Baileys (P0-2)
- Remove the adapter, registration (`server.ts:338-340`), render function (`render.ts:47-71`), env vars (`WHATSAPP_ENABLED`, `***REDACTED***`), the `@whiskeysockets/baileys` dependency, and the `'whatsapp'` channel literal from the TS union (`provider.ts:3` → `'telegram'|'push'`) and the worker query (`workers/index.ts:320` → `channel IN ('telegram')`). The worker reads `target.channel as string` so no as-any cast is needed.
- Existing `channel='whatsapp'` targets are migrated to `status='disabled'`, `last_error='WHATSAPP_REMOVED_RECONFIGURE'` (never silently dropped); owner UI prompts reconfiguration.
- Telegram, push, email channels retained. Official WhatsApp Business Cloud API explicitly deferred to a future ADR.
- **GO-gate MUST (RESOLVE, Counsel F-2):** before deploy, run `SELECT location_id, address FROM owner_notification_targets WHERE channel='whatsapp' AND status='active';` against prod, paste the result into the deploy record, and confirm each affected owner has another active channel or has been individually warned.

### D3 — MessageBus PII minimization at the producer (P0-3) — REWRITTEN (RESOLVE pivot)
- **Original (re-fetch in the WS room handler) is WITHDRAWN.** Verified against `websocket.ts`: the room handler is **synchronous** (`:36`) and fans in NOTIFY FIFO order — an awaited DB read inside it **reorders the fan-out** (breaker CRITICAL-1); and `websocket.ts` reads on the **raw operational pool** with no `set_config('app.current_tenant', …)`, which **bypasses RLS** per ADR-006 — an un-scoped read (breaker CRITICAL-2).
- **Decision:** minimize PII **at the producer** — applied at the COMPLETE producer census (R3: the R2 design enumerated only `orders.ts`; the breaker found a second producer). A full grep of `messageBus.publish(...)` to every dashboard/order/courier channel found **exactly two** producers carrying customer PII:
  - **`orders.ts:722-737`** (`order.created`): publish only `{ orderId, locationId, status, total, currency, itemCount, shortId, createdAt, seq }` — **drop** `customerNameMasked`, `customerPhoneMasked`, `itemsSummary`, `courierName`.
  - **`orderStatusService.ts:108-114` via `fetchOrderDelta` (`:6-29`)** (`order.status` delta, R3-new — leaked `itemsSummary` on EVERY status transition): **remove the `items_summary string_agg` from the SELECT and the `itemsSummary` field from the returned delta**; keep `itemCount` only. Item-name lists are special-category-adjacent (dietary/medical signal) and do not belong on the bus or in logs.
  - All other publishers (`order-timeout-sweep`, `dwell-monitor`, `dwell-escalation`, `signal-raiser`, `anonymizer-gdpr`, `server.ts` assignment, lifecycle) verified to carry only status / opaque UUIDs / position / the *courier's* own masked contact on the per-order channel — no customer name/phone/address/item-names.
- **`message-bus.ts:48` verbatim payload log:** with both producers minimized the logged payload is non-PII (confirmed). Demoted defence-in-depth: log `channel`+length at info, full payload at `debug` only (this `console.log` bypasses the structured logger's redaction at `logger.ts:18`/`sentry.ts:13`).
- The WS room handler is **unchanged** (sync, verbatim, no DB read) → ordering preserved, no un-scoped read. Both criticals dissolve by deleting the offending operation rather than relocating it. **Zero-PII-on-the-bus is now provable by a complete census, not one file.**
- Customer name/phone reach the owner only via the existing RLS+JWT order-list fetch and owner-reveal route — never the bus. Cost: the dashboard card lacks live name/phone/items on a brand-new order until the next owner-list fetch (deliberate UX degradation, R7); the card shows shortId/total/itemCount live.
- `seq` is a per-process monotonic counter for client gap-detection only; no sequence table. The bus returns to a pure pass-through.
- Chosen over: re-fetch at the WS boundary (CRITICAL-1+2), per-subscriber re-fetch (N+1), payload encryption (still crosses un-RLS'd transport + logs).

### D4 — Minimize Telegram alert body (P0-4)
- New per-location setting `telegram_alert_detail ∈ {'minimal','area','full'}`, default `'area'`.
- `area` (default, **best-effort — RESOLVE honesty fix**): `#order · item count · total · district/street WITHOUT house number` + authenticated owner-app deep-link. Because addresses are **free-text** (no structured field) and Albanian addressing is unstructured, the house-number-strip regex usually **cannot confidently split**, so `area` falls back to `minimal` for most real addresses. Documented honestly as best-effort; never emits a raw address (fail-closed). Phone removed from body.
- `full`: legacy detail, explicit owner opt-in (accepted retention risk).
- Full address/phone reachable only via the existing JWT-gated owner SPA deep-link (`render.ts:78-79`) → owner RS256 JWT + membership RLS. A customer track-grant is deliberately NOT used (principal mismatch).
- **HUMAN-DECISION (HD-2):** because best-effort `area` is mostly `minimal` in practice, the default level materially changes the solo-operator dispatch workflow → the default is an owner ruling (STOP-ETHICS gate). Recommendation: default `area`; measure `full`-opt-in rate as the usability canary.

## Deviations (recorded)

- **DEV-1 (RESOLVE-REVISED — `NOT VALID` CHECK, Counsel A-1):** instead of leaving the CHECK fully broad (the original deviation), the migration adds `CHECK (channel IN ('telegram','push')) NOT VALID` (constraint `owner_notification_targets_channel_not_whatsapp`). This **rejects all new/updated rows** while **tolerating the existing disabled `'whatsapp'` rows** (Postgres skips pre-existing rows for `NOT VALID`). The schema-level invariant ("no new whatsapp") is thus DB-enforced, not app-layer-only — closing the "app layer is the only writer forever" assumption. **The constraint must NEVER be `VALIDATE`d** (would fail on the disabled rows) — see NR-3. Forward-only and non-destructive preserved; no owner config deleted. Owner: Architect. (Risk R3 resolved; NR-3 introduced.)
- **DEV-2 (vs spec's "authenticated deep-link" reusing track-grant):** P0-4 reuses the **owner-app** JWT deep-link, not the customer track-grant, because the track grant authenticates as the customer (wrong principal for an owner dispatch). The auth requirement is still met (RS256 + RLS), via a different, already-existing mechanism. Owner: Architect.
- **DEV-3 (RESOLVE — deliberate `'assigned'` exclusion from the GPS active set):** `ACTIVE_DELIVERY_ASSIGNMENT_STATUSES = ['accepted','picked_up']` deliberately **excludes** `'assigned'` (dispatcher-assigned-but-not-yet-consented), diverging from the broader `('assigned','accepted','picked_up')` active set used elsewhere (`shifts.ts`). Rationale: tracking begins at the courier's own act of acceptance = consent boundary (Counsel dignity lens). Recorded so a future enum-tidy refactor cannot silently re-add `'assigned'` to the GPS guard. Owner: Architect.
- **DEV-4 (R3 — overlapping channel CHECK pair on `owner_notification_targets`):** the schema now carries TWO constraints — the pre-existing validated `owner_notification_targets_channel_check` (permits `whatsapp`) AND the new `owner_notification_targets_channel_not_whatsapp NOT VALID` (forbids it). They AND together correctly (new whatsapp rows rejected; disabled rows tolerated). **Load-bearing:** `_not_whatsapp` must **never be dropped** (a "tidy overlapping constraints" pass would silently re-open whatsapp writes), and `_channel_check` must **not** be read as "whatsapp is allowed." We deliberately do NOT ALTER `_channel_check` (would VALIDATE-fail on the disabled rows). Owner: Architect. (Risk R12.)
- **DEV-5 (R3 — `'whatsapp'` literal survivors out of P0-2 scope):** two `'whatsapp'` references are legitimate and MUST stay — `courier/me.ts:79` (`messenger_kind` customer-facing wa.me click-to-chat, official Meta link, not Baileys) and `spa-shell.ts:14` (`BOT_UA` link-preview crawler regex). The §9 P0-2 grep proof is scoped to `notifications/**` + `provider.ts` and excludes these. Owner: Architect.

## Consequences

- Positive: ~50% fewer position writes; zero PII on the realtime bus or in bus logs; no TOS-violating third-party PII egress; PII-free Telegram history by default; **zero added DB load** for P0-3 (post-pivot: no re-fetch); DB-enforced "no new whatsapp" invariant.
- Negative: owners on `full` still leak to chat history (opt-in, accepted); brief GPS gap on assignment lag; owners with only-whatsapp targets must reconfigure (proof-gated pre-deploy warning required); dashboard live card lacks customer name/phone/items on a brand-new order until the next owner-list fetch (R7).
- Migration is forward-only, non-destructive (index + nullable-defaulted column + idempotent UPDATE + `NOT VALID` CHECK never validated); code-only rollback is safe against it (P0-3 revert is a one-line producer change; the WS handler was never touched).

## Open / Accepted Risks
See proposal §10 (R1–R12) + resolution.md §G (NR-1…NR-5) and §R3 (R3-NR-A…D). R11 (no live-search server fallback) is accept-risk owned by **Product**; R12 (overlapping CHECK pair) accept-risk owned by Architect.

## GO-gate pre-reqs
- **Proof-gated MUST (Counsel F-2):** run + paste the active-whatsapp-targets query and warn affected owners before deploy.

## HUMAN-DECISION items (STOP-ETHICS gate) — Counsel R2 recs folded in as proposed defaults
- **HD-1 (P0-1 MED):** idle-courier owner-map dot. **Proposed default (Counsel + Architect): (a) privacy-max** — lose idle-courier dots; operational loss documented (dispatcher loses "which idle courier is roughly where"; dots go stale then disappear at 24h purge). Alternative (b) coarse last-known acceptable only if genuinely degraded + F-1 copy discloses it. Do not delete `courier-events.ts:155-164` + `fetchLatestPosition` without an owner ruling. **Pending owner ruling.**
- **HD-2 (P0-4):** default `telegram_alert_detail` level. **Proposed default (Counsel + Architect): `area` best-effort** (honest degrade to `minimal`; `full` opt-in; `full`-opt-in rate as canary). **Pending owner ruling.**
- **R11 (NR-1, P0-3 pivot side-effect):** no server-side fallback for live customer-name/item search of in-flight orders — escalated to **Product** as accept-risk (lean accept; build debounced server search only if Product rejects).

## Deferred (future ADR)
- Counsel A-2: disabled-target prompt as future official-WhatsApp re-onboarding surface.
- Counsel open question: courier as data subject of their own movement data (transparency parallel to customer privacy copy).
