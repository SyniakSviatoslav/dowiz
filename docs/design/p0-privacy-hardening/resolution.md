# Resolution — P0 Privacy Hardening (RESOLVE phase)

- Role: System Architect (Triad Council), RESOLVE phase
- Date: 2026-06-21
- Slug: `p0-privacy-hardening`
- Inputs: `proposal.md`, `counsel-opinion.md`, breaker findings (delivered via RESOLVE brief — note: `breaker-findings.md` was not present on disk; findings reconstructed from the RESOLVE brief and **re-verified against live code** before disposition; see "Source note" below).
- Outputs updated: `proposal.md` (P0-3 pivot, P0-1 reconcile, P0-4 honesty, P0-2 MEDs, F-1, risk table), `docs/adr/ADR-p0-privacy-hardening.md` (D3 rewrite, D1 divergence, new DEV/risks).

## Source note (epistemic honesty)

The breaker's `breaker-findings.md` file is absent from `docs/design/p0-privacy-hardening/`. I did not invent findings. I took the finding substance from the RESOLVE brief and **independently verified every cited code site** before ruling. The two CRITICALs and the MED are confirmed true against `HEAD` of `feat/golive-remediation`:

- `websocket.ts:36` room handler signature is `(msg: unknown) => void` — **synchronous**, fans verbatim (`{ room, data: msg }`) with no DB read. Confirmed.
- `websocket.ts:87,:96` use `fastify.db.query(...)` (raw operational pool) with **no `set_config('app.current_tenant', …)`** anywhere in the file. Per ADR-006 the operational role bypasses RLS. Confirmed.
- `courier-events.ts:91-99` (`fetchLatestPosition`) + `:155-164` (`handlePositionUpdated`) exist *specifically* to surface idle on-shift couriers on the owner map (explicit comment :156-157). Confirmed.
- `orders.ts:722-737` dashboard producer ships `itemsSummary` + `customerNameMasked` + `customerPhoneMasked`; there is **no consumer-side re-fetch today** — the handler forwards verbatim. Confirmed.
- `provider.ts:3` channel union includes `'whatsapp'`; `workers/index.ts:320` query is `channel IN ('telegram','whatsapp') AND status='active'`. Confirmed.
- `i18n.ts:388,:1308,:2228` `courier.gps_active` exists in sq/en/uk; `client.sharing_location_note` (:560,:1479,:2402) tells the *customer* the courier sees them, but the *courier* is told nothing about the boundary. Confirmed.

---

## A. Disposition summary (decision-dense)

| ID | Finding (short) | Disposition |
|---|---|---|
| P0-3 CRITICAL-1 | Re-fetch in sync WS handler breaks NOTIFY ordering | **FIX — design PIVOT** (publish minimized non-PII projection; delete the re-fetch idea entirely) |
| P0-3 CRITICAL-2 | WS re-fetch has no RLS/tenant session (raw pool bypasses RLS) | **FIX — dissolved by the same pivot** (no consumer re-fetch → no un-scoped read) |
| P0-1 HIGH | Client hard-stops poller on 403 → assigned-but-not-accepted courier goes invisible while driving to pickup | **FIX** (define active set = consent boundary `accepted`+`picked_up`; client BACKS OFF on 403, never hard-stops) + recorded deliberate divergence from broader `'assigned'` set |
| P0-1 MED | Guard deletes the idle-courier owner-map dot (real product feature) | **HUMAN-DECISION (STOP-ETHICS gate)** — owner ruling: privacy-max (lose idle dots) vs coarse last-known idle position |
| P0-4 HIGH | No structured address field; `area` regex fails-closed to no-address for most free-text Albanian addresses → `area` silently ≈ `minimal` | **FIX (honest rename)** + **HUMAN-DECISION** on the default if it changes existing owners' dispatch workflow |
| P0-2 MED (F-2) | Owner-silent-on-whatsapp warning is "should" not a proof-gated MUST | **FIX** — promoted to a proof-gated, result-pasted GO-gate checklist item |
| P0-2 MED (A-1/DEV-1) | Broad CHECK leaves DB able to accept new whatsapp rows forever | **FIX** — add `CHECK (channel IN ('telegram','push')) NOT VALID` (rejects new, tolerates existing) |
| P0-2 MED | Dangling `'whatsapp'` type-union refs (`provider.ts`, `render.ts`, `workers/index.ts` as-any) | **FIX** — narrow union to `'telegram'|'push'`; worker reads `channel as string` (no union dependency); render fn deleted |
| Counsel F-1 | Courier never told the tracking boundary | **FIX (revise)** — one i18n string per locale near `courier.gps_active` |
| Counsel F-2 | (= P0-2 MED above) | **FIX** (see above) |
| Counsel A-1 | (= P0-2 MED above) | **FIX** (see above) |
| Counsel A-2 | Disabled-target prompt as future WA-Business re-onboarding surface | **DEFER-FLAG** (cheap forward-compat; not this batch) |
| Counsel open question | Courier as data subject of own movement | **DEFER-FLAG** (future ADR; recorded as future, not this batch) |

Three buckets at the bottom: **Resolved**, **Accept-risk (+owner)**, **HUMAN-DECISION (STOP-ETHICS gate)**.

---

## B. P0-3 — the pivot (CRITICAL-1 + CRITICAL-2), in full

### Why the proposal's Decision A is wrong as written
The proposal's P0-3 Decision A says: publish `{event, location_id, order_id, seq}` and *"the dashboard WS room handler gains a re-fetch step: on a claim-check message, run a tenant-scoped SELECT … then fan the result to room members."* Verified against `websocket.ts`:

1. **Ordering break (CRITICAL-1).** The room handler is `(msg: unknown) => void` and the MessageBus invokes handlers synchronously in NOTIFY (FIFO-per-channel) order. To re-fetch you must `await` a DB read inside that handler. Two outcomes, both broken:
   - Make the handler `async` → the dispatcher either fires it and moves on (overlapping reads resolve out of order → events fan to browsers out of order) or the dispatcher would have to serialize awaits (it does not, and adding head-of-line blocking on a shared NOTIFY listener is a self-inflicted latency cliff).
   - Fire-and-forget the read → same out-of-order fan, plus unbounded concurrent reads under burst.
   The current design's correctness *depends* on the handler being synchronous and verbatim. A re-fetch is fundamentally incompatible with it.

2. **No tenant scope (CRITICAL-2).** `websocket.ts` calls `fastify.db.query(...)` on the raw operational pool with no `set_config('app.current_tenant', …)`. Per ADR-006 that role bypasses RLS. A re-fetch here is an **un-scoped read** — the proposal's §8 claim that the re-fetch "runs with `set_config('app.current_tenant', location_id)`" describes code that does not exist and would require wrapping each fan-out in a `BEGIN; set_config; SELECT; …` on a pool shared across all rooms — exactly the leak-prone pattern (a missed reset leaks the next room's tenant). The room-authz gate (`ownerCanAccessRoom`) limits *who receives* the fan but does nothing to scope *the read itself*.

### The fix: minimized non-PII projection at the PRODUCER (no consumer re-fetch)
**Insight (from the RESOLVE brief, verified):** the PII to remove from the bus is **customer NAME / PHONE / ADDRESS / items-summary** (the free-text item names can echo dietary/medical preferences and are PII-adjacent). The dashboard live card needs **status / total / itemCount / shortId** — **none of which are PII**. So:

- The producer (`orders.ts:722-737`) publishes a **minimized non-PII projection**:
  `{ type:'order.created', data: { orderId, locationId, status, total, currency, itemCount, shortId, createdAt, seq } }`.
- **DROP** `customerNameMasked`, `customerPhoneMasked`, `itemsSummary`, `courierName` from the dashboard NOTIFY body.
- The WS room handler stays **exactly as-is** — synchronous, verbatim fan-out. **No re-fetch. No async. No un-scoped read.** CRITICAL-1 and CRITICAL-2 both **dissolve** because the offending operation is deleted, not relocated.
- The masking helpers in `orders.ts:734-735` are deleted (Counsel's "elegant tell": the right design makes the defensive code disappear).

This is still the **claim-check spirit** — the bus never carries PII, and the only path to customer name/phone is the existing authenticated owner-reveal route (`ownerRevealContactRoutes`, RLS+JWT). The difference from the rejected design is *where* minimization happens: **at the producer (no transport of PII at all), not at a consumer re-fetch.** Strictly simpler and strictly safer.

### Explicit comparison: minimized-non-PII-payload (CHOSEN) vs re-fetch (REJECTED)

| | Minimized non-PII payload (CHOSEN) | Re-fetch at WS boundary (REJECTED) |
|---|---|---|
| Ordering | Preserved — handler stays sync/verbatim | **Broken** — async read in a FIFO handler reorders fan-out |
| Tenant scope | N/A — no read happens at the seam | **Leak-prone** — raw pool, no `set_config`, shared across rooms |
| DB load | Zero added reads | +1 read/event (and a `BEGIN…set_config…` wrapper) |
| Blast radius | One producer line | Producer + a new stateful read path in the WS hot loop |
| Code deleted | masking helpers vanish | masking helpers move to a new place |
| Cost | dashboard loses live name/phone/itemsSummary on a brand-new order until next full fetch | none functional, but two CRITICALs |

### The cost of the pivot, and how the client absorbs it
The dashboard's brand-new-order card loses **live customer name, phone, and items-summary** until the next full fetch. Handling (specified in proposal §4 P0-3):
- The card renders **status / shortId / total / itemCount / createdAt** live (all present in the projection) — enough to *see a new order arrived and its size*.
- Customer name / phone / items render via the **owner's normal order-list fetch** (already RLS+JWT scoped) which the dashboard already runs on load and on interval; a fresh order's PII appears on the next poll/refresh (seconds), or immediately when the owner opens the order.
- **Live search by customer name for in-flight orders** (if the dashboard offers it) falls back to an **on-demand authenticated fetch** (the owner order-search endpoint), NOT to bus data. Until then the card shows a **masked placeholder** ("New order #ABCD") rather than a name.
- This is a deliberate, recorded UX degradation (R7) — *not* a regression to hide. It is the correct trade: a few seconds of "name pending" on the owner's own screen vs PII on an un-RLS'd, logged transport.

### New `seq`
`seq` stays a per-process monotonic counter for client gap-detection only (no DB, no sequence table) — unchanged from the proposal and unaffected by the pivot.

---

## C. P0-1 HIGH — reconcile breaker (omit `assigned`) vs Counsel (start at `accepted` is the consent boundary)

**They are not in conflict; they were talking past each other.** Counsel is right that `accepted` is the honest consent boundary (the courier voluntarily took the job) and that excluding `'assigned'` (dispatcher-assigned-but-not-yet-consented) is a *stronger* dignity posture. The breaker's real break is **not** "you should include `assigned`" — it is **client behavior**: the proposal said the client *"must stop the poller and not retry"* on `403 GPS_NOT_ON_ACTIVE_DELIVERY`. That hard-stop means a courier who is `assigned` and **driving to pickup**, or whose `accepted` row briefly lags, goes **invisible** with no automatic recovery — tracking only resumes if something else restarts the poller.

**Resolution (deliberate, recorded):**
- **Active set = consent boundary**: `ACTIVE_DELIVERY_ASSIGNMENT_STATUSES = ['accepted','picked_up']`. `'assigned'` is **excluded** — tracking begins at the courier's own act of acceptance (autonomy by construction).
- **Client does NOT hard-stop on 403.** It **backs off and retries** (exponential backoff, cap ~30-60s) so tracking resumes **the instant** the courier accepts. The 403 is a "not yet" signal, not a "stop forever" signal. This is the actual fix for the HIGH.
- **Recorded as a deliberate divergence** from the broader `'assigned'`-inclusive active set used elsewhere in the codebase (e.g. `shifts.ts` enumerates `('assigned','accepted','picked_up')`). The privacy rationale (pre-consent = no tracking) is written into the ADR so a future "tidy this enum" refactor cannot silently re-add `'assigned'` to the GPS guard. (DEV-3.)

This is a **revision**, not an accept-risk: the client backoff change is required for GO.

---

## D. P0-4 HIGH — the `area` mode is honestly mostly `minimal`

**The break (verified by reasoning over the design):** there is **no structured address field** — delivery addresses are free-text (Albanian addressing is notoriously unstructured: "te ura e Tabakëve, kati 2, mbi farmaci"). The proposal's `area` mode strips house-number tokens via regex and **falls back to `minimal` when it can't confidently split**. For the *majority* of real free-text Albanian addresses the regex *cannot* confidently split → `area` silently degrades to "order# + total + deep-link" = **`minimal`**. So the default `area` is, in practice, mostly `minimal` while *claiming* to give the owner a dispatch hint. That is a dishonest mode name.

**Resolution — two parts:**
1. **FIX (honesty):** rename and re-document. `area` becomes **best-effort** and the proposal/owner-copy states plainly: *"area shows a coarse district/street **only when it can be safely extracted**; otherwise it shows order# + total only."* The owner is told the truth: on ambiguous addresses they will not get an area hint and must open the app. No mode pretends to do more than it does.
2. **HUMAN-DECISION (default selection):** because `area` is mostly-`minimal` in practice, the *effective default* materially changes the solo-operator dispatch workflow (owners who today read the street from the Telegram body will, on most orders, now have to deep-link into the app per order — the steel-man in Counsel §4). The **default level** (`minimal` vs `area` vs `full`) is therefore an **owner ruling**, flagged to the STOP-ETHICS gate, because it affects existing owners' live workflow. Architect recommendation: default `area` (best-effort) + measure the `full`-opt-in rate as the canary (Counsel's R1 signal); if owners flee to `full`, `area` failed as a *usable* default, not privacy being too aggressive.

Fail-closed behavior (ambiguous → never emit raw address) is **unchanged and correct**.

---

## E. P0-2 MED items

### E-1 (F-2): WhatsApp-owner warning → proof-gated MUST
Promoted from "ops should notify affected owners" to a **recorded pre-deploy GO-gate checklist item with the query result pasted**:
```
SELECT location_id, address FROM owner_notification_targets
 WHERE channel='whatsapp' AND status='active';
```
The deploy does not proceed until (a) this query is run against the prod DB, (b) the result is pasted into the deploy record, and (c) every affected owner has been warned **and** has at least one *other* active channel OR has been individually contacted. This binds the one cell where the batch can silently lose a real owner's order. Procedural, not code. (Updated in proposal §9 and §10/R4.)

### E-2 (A-1 / DEV-1): `NOT VALID` CHECK
Revise DEV-1. Instead of leaving the CHECK fully broad (relying solely on app-layer discipline forever), the migration adds:
```
ALTER TABLE owner_notification_targets
  ADD CONSTRAINT owner_notification_targets_channel_not_whatsapp
  CHECK (channel IN ('telegram','push')) NOT VALID;
```
`NOT VALID` **rejects all new/updated rows** that violate the constraint while **tolerating the existing disabled `'whatsapp'` rows** (Postgres does not validate pre-existing rows for a `NOT VALID` constraint). This restores the schema-level invariant ("no new whatsapp, ever") without deleting owner config and without violating forward-only/non-destructive. We deliberately **do NOT** run `VALIDATE CONSTRAINT` (that would fail on the disabled rows). DEV-1 is thereby downgraded from "app-layer is the only writer forever" (an assumption that quietly rots) to a DB-enforced invariant. (Updated in proposal §5 migration step 2 and ADR DEV-1.)

### E-3: dangling `'whatsapp'` type-union references
- `provider.ts:3` — narrow `channel: 'telegram'|'push'|'whatsapp'` → **`'telegram'|'push'`**.
- `render.ts:47-71` `renderWhatsAppMessage` — **deleted** (and its import sites).
- `workers/index.ts:320` query — narrow `channel IN ('telegram','whatsapp')` → **`channel IN ('telegram')`**. The loop reads `target.channel as string` (line 333), so it has **no compile dependency** on the union — no as-any cast is needed; the disabled rows are already excluded by `status='active'` AND now by the narrowed `channel IN` filter AND by the `NOT VALID` CHECK (three independent gates). Belt, suspenders, and a second belt.
- Dispatcher `register('whatsapp', …)` call removed (`server.ts:338-340`), so the runtime map never has a whatsapp provider.

---

## F. Counsel non-blocking items

### F-1 (revise): tell the courier the boundary
Add one i18n string per locale adjacent to `courier.gps_active`, e.g. key `courier.gps_boundary_note`:
- sq: "Pozicioni juaj ndahet vetëm gjatë një dorëzimi aktiv — jo në kohën tuaj të lirë."
- en: "Your location is shared only during an active delivery — never in your free time."
- uk: "Ваше місцезнаходження передається лише під час активного доставлення — ніколи у вільний час."
Surfaced near the GPS indicator on `apps/web/src/pages/courier/DeliveryPage.tsx`. Cost: one string × 3 locales. Folded in. (UI surface → needs a Playwright assertion that the string is visible; added to proof list.)

### A-2 (defer-flag): disabled-target prompt as future WA-Business re-onboarding surface
Recorded as a **future** consideration. Design the disabled-target reconfigure prompt so its copy can later become "WhatsApp is back, officially — re-enable?" Not built this batch (YAGNI for now); flagged so the future ADR for official WA Business API knows the surface exists.

### Counsel open question (defer-flag): courier as data subject of own movement
**Defer-flag → future ADR.** `courier_positions` is the courier's own location data; the platform is its controller. The 24h purge + the active-delivery guard are good but the courier's "what is kept about me, for how long, can I see it" transparency is structurally absent. This is **explicitly out of scope for this batch** and recorded as a future work item (parallel to the customer `checkout.privacy.*` copy). Not a launch blocker.

---

## G. NEW risks created by these revisions (hand to the breaker for re-attack)

| # | New risk introduced by the resolution | Mitigation / disposition |
|---|---|---|
| NR-1 | **Dashboard PII-staleness (from P0-3 pivot).** A brand-new order's customer name/phone/items are absent from the live card until the next owner-list fetch. If the dashboard's "new order" toast or name-search reads from bus data, it now shows a masked placeholder. | The owner order-list fetch (RLS+JWT) is the source of name/phone; card shows shortId/total/itemCount live. Name-search for in-flight orders must use the on-demand authenticated fetch, never bus data. **Breaker should re-attack: does any current dashboard code read `customerNameMasked`/`itemsSummary` off the bus payload and will it break (undefined) when those keys vanish?** (R7) |
| NR-2 | **Client backoff loop (from P0-1 reconcile).** The courier client now retries on 403 with backoff instead of stopping. A misconfigured backoff (too tight) could hammer the guard endpoint for a courier who never gets assigned. | Exponential backoff with a 30-60s cap and jitter; the guard is a single indexed EXISTS (<1/s budget). **Breaker should re-attack the backoff bound and the thundering-herd case (many on-shift idle couriers all polling).** |
| NR-3 | **`NOT VALID` CHECK + future migration (from E-2).** A future migration that runs `VALIDATE CONSTRAINT` (e.g. a well-meaning "clean up NOT VALID constraints" pass) would **fail** on the disabled whatsapp rows, or worse, a tooling default that auto-validates would block deploy. | The constraint name encodes intent (`_not_whatsapp`); ADR DEV-1 records "never VALIDATE — disabled rows are intentional." **Breaker should check the project's migration tooling does not auto-validate NOT VALID constraints on boot.** |
| NR-4 | **Honest-`area` adoption (from P0-4 rename).** Telling owners the truth ("area often shows nothing extra") may push them to `full` *faster* than a silently-mostly-minimal `area` would have — accelerating the very PII-to-chat-history we want to avoid. | Default stays `area` (best-effort); `full`-opt-in rate is the measured canary (R1). This is also why the default is a HUMAN-DECISION. |
| NR-5 | **i18n drift (from F-1).** A new key in 3 locales is one more thing that can fall out of sync; `i18n.ts` is the #1 churn hotspot (99.9th %ile). | Single key, added to all three locale blocks in the same edit; Playwright asserts visibility in at least the default locale. |

---

## H. Final buckets

### RESOLVED (revised in code/design this batch — required for GO)
- P0-3 CRITICAL-1 + CRITICAL-2 — pivot to producer-side minimized non-PII payload; no consumer re-fetch. (proposal §4 P0-3 rewritten, §8 corrected, ADR D3 rewritten.)
- P0-1 HIGH — active set = `accepted`+`picked_up`; **client backs off on 403, does not hard-stop**; deliberate `'assigned'`-exclusion divergence recorded. (proposal §4 P0-1, ADR D1 + DEV-3.)
- P0-4 HIGH (honesty half) — `area` re-documented as best-effort; owner copy tells the truth. (proposal §4 P0-4, §10/R1.)
- P0-2 MED — F-2 warning promoted to proof-gated MUST; A-1 `NOT VALID` CHECK added; dangling `'whatsapp'` union/render/query references removed. (proposal §4 P0-2, §5, §9, ADR DEV-1.)
- F-1 — courier boundary i18n string in sq/en/uk. (proposal §4 P0-1, proof list.)

### ACCEPT-RISK (+owner)
- R1 — `area` regex mis-parse on Albanian free-text → fail-closed to `minimal`; refine post-launch. **Owner: Architect → Backend.**
- R2 — owners on `full` leak address/phone to Telegram history (explicit opt-in). **Owner: Product.**
- R5 — brief GPS gap when `accepted` row lags the courier's accept (seconds; ETA → last-known). **Owner: Architect.**
- R6 — claim-check load at 10× future scale (now moot for dashboard since no re-fetch; retained for other re-fetch paths). **Owner: Architect.**
- NR-1…NR-5 — handed to breaker for re-attack (see §G); provisionally accepted with stated mitigations pending re-attack.

### HUMAN-DECISION → STOP-ETHICS gate (the human/operator decides, on the record)
- **HD-1 (P0-1 MED): idle-courier owner-map dot.** The guard deletes the feature that shows idle on-shift couriers on the owner map (`courier-events.ts:155-164` + `fetchLatestPosition`). This is a genuine product-vs-privacy tradeoff. **Owner ruling required:** (a) privacy-max — accept losing idle-courier dots (idle couriers vanish from the map until they accept a job), or (b) keep a coarse / last-known idle position for them. **Do not silently delete the feature.** Architect lean: (a) aligns with the dignity posture (Counsel: "surveillance of a worker who is not working"), but the operator may value dispatch visibility — their call.
- **HD-2 (P0-4 default level).** Because best-effort `area` is, in practice, mostly `minimal` on free-text Albanian addresses, the default level materially changes the solo-operator dispatch workflow. **Owner ruling required:** confirm default = `area` (best-effort) vs `minimal` vs allowing `full` as default. Affects existing owners' live workflow → human decision.

### DEFER-FLAG (future, recorded as MISSING-for-now, not this batch)
- A-2 — disabled-target prompt as future WA-Business re-onboarding surface.
- Counsel open question — courier as data subject of their own movement data (transparency parallel to customer privacy copy). Future ADR.

---

## I. Re-verification checklist for the breaker's next round
1. Grep the web app for any read of `customerNameMasked` / `customerPhoneMasked` / `itemsSummary` off the dashboard WS payload (NR-1).
2. Confirm `websocket.ts` room handler is **unchanged** (no async, no DB call) after the P0-3 pivot.
3. Confirm the courier client 403 path backs off (no hard-stop) and the backoff is bounded (NR-2).
4. Confirm the migration adds the `NOT VALID` CHECK and **never** validates it (NR-3).
5. Confirm `provider.ts` union, `render.ts` (no `renderWhatsAppMessage`), and `workers/index.ts:320` query no longer reference `'whatsapp'`.
6. Confirm `courier.gps_boundary_note` exists in all three locale blocks and is rendered on the courier delivery page.

---

## R3 resolution

Round R3 closes the **two open HIGH** findings from the breaker's RE-ATTACK (R2) and the Counsel RE-EXAMINE (R2). Every cited line re-verified against live `HEAD` of `feat/golive-remediation` before disposition.

### HIGH-1 — P0-3 producer census was incomplete (second PII producer) — CLOSED

**Re-verified live:**
- `orderStatusService.ts:108-114` publishes `{type:'order.status', data:{...delta, statusUpdatedAt}}` to `dashboardChannel(dbLocationId)` on **every** status transition; `fetchOrderDelta` (`:6-29`) `string_agg`s `quantity×name_snapshot` into `itemsSummary` (`:10-11`, returned `:26`). **Confirmed — this is a real second producer of item-name PII the R2 table omitted.**
- `message-bus.ts:48` logs `msg.payload` verbatim via `console.log` (bypassing the structured logger's redaction at `logger.ts:18` / `sentry.ts:13`). **Confirmed — `itemsSummary` lands in stdout on every order.created AND every transition today.**

**Action taken — COMPLETE producer census.** Grepped every `messageBus.publish(...)` to a dashboard/order/courier channel. The full enumeration is now in proposal §4 P0-3 (10-row table). **Exactly two** producers carried customer PII: `orders.ts:722-737` (R2-known) and `orderStatusService.ts:108-114` (R3-new). All others verified to carry only status / opaque UUIDs (`orderId`, `customerId`, `alertId`, `signalId`) / position / the *courier's* own masked contact on the per-order channel — none carry customer name/phone/address/item-names.

**Decision on `itemsSummary` (is an item-name list special-category-adjacent PII?):** **YES — drop it from the bus.** A list of item names ("2×Insulin-friendly bowl") can echo dietary/medical/religious preferences (special-category-adjacent under GDPR). It does not belong on an un-RLS'd LISTEN/NOTIFY transport that is also logged verbatim. **Resolution:** remove the `items_summary` subquery from `fetchOrderDelta`'s SELECT and the `itemsSummary` field from its return shape; remove `itemsSummary`/masked name+phone from `orders.ts:722-737`. The dashboard keeps `itemCount` (a number, non-PII) for the card; the **item-name list is fetched client-side under the owner's RLS+JWT order-detail route**, never the bus. (`itemCount` is retained, not the name list — answering the brief's "keep only itemCount" option directly.)

**`message-bus.ts:48` disposition:** with both producers minimized, **the payload is already non-PII** — confirmed: every dashboard publish now carries only `{orderId, status, total, currency, itemCount, shortId, createdAt, statusUpdatedAt}`. The verbatim log is therefore non-leaking. **Defence-in-depth added:** demote the line to log `channel` + payload length at info, full payload at `debug` only, so a future careless producer cannot silently re-leak via this `console.log` that bypasses the redaction layer. Recorded as a hardening, not a blocker.

**Invariant "zero PII on the bus" is now provably met by a complete census, not one file.**

### HIGH-2 — P0-1 client backoff was structurally impossible — CLOSED

**Re-verified live:** `DeliveryPage.tsx:138-153` is `useEffect(..., [position])` (fires only on a NEW OS `position`) with a bare `.catch((err) => console.debug(...))` (`:150-152`) — no retry, no backoff, no timer. `useGeolocation` (`use-geolocation.ts:32-34`) drives `position` via `watchPosition` and **keeps the latest fix in React state**. **Confirmed:** when the courier is stationary at pickup, `position` never changes → the effect never re-fires → an assigned-but-not-yet-accepted courier stays invisible and a 403 is never retried. The R2 "exponential backoff" had no timer to run on — structurally impossible as written.

**Action taken — TIME-BASED heartbeat (replaces event-driven post).** Replace `useEffect([position])` with a `setInterval` heartbeat at the named constant `COURIER_GPS_POST_INTERVAL_MS = 12_000` that **re-posts the last-known `position`** (read from the hook's React state — no fresh OS event needed). The timer runs only while the page holds an assignment/open shift; cleared on unmount. A 403 is naturally retried by the *next interval* (retry is steady-state, not a special backoff path) → tracking resumes **within ≤12s of the courier tapping accept, independent of physical movement**. The server guard remains the hard gate.

**Back-of-envelope (post rate vs battery):** ≤1 post/12s/courier = ~5 POSTs/min, ~300/hr — negligible vs the GPS radio (already on continuously via the mounted watcher; the timer adds no second GPS subscription, it decouples *posting* from *sampling*). No battery regression. 8 couriers → ≤0.67 INSERT/s peak (the 12s interval is a strict ceiling on the previous event-driven 0.8/s estimate).

**Does NOT reintroduce idle-courier tracking (reconciled with HD-1):** the timer only fires while the page holds an assignment/open shift, and the **server still rejects** any post unless `status IN ('accepted','picked_up')` → an idle courier's heartbeat posts are **403'd and discarded, never stored**. The client posting is harmless because the server is authoritative. HD-1 (idle dot survival) is governed by the server guard + `fetchLatestPosition`, unaffected by the client timer.

**Thundering-herd (NR-2 rebounded):** fixed interval, no exponential ramp, no synchronized storm; per-client mount-relative phase; optional ±1-2s jitter only if clustering observed. R8 updated.

### Counsel R2 recommendations folded in as PROPOSED DEFAULTS (human rules at STOP-ETHICS)
- **HD-1 → propose (a) privacy-max** (lose idle-courier map dots). Operational loss documented in proposal §10: dispatcher loses live "which idle courier is roughly where"; with the guard, dots go stale (last pre-idle row) then disappear at 24h purge, not instantly. Alternative (b) coarse last-known acceptable only if genuinely degraded + F-1 copy discloses it. **Proposed default, pending owner ruling.**
- **HD-2 → propose `area` best-effort** default (honest degradation to `minimal`), `full` opt-in, `full`-opt-in rate as the canary. **Proposed default, pending owner ruling.**

### R2 MEDs dispositioned
- **NR-1 (on-demand authenticated search fallback doesn't exist):** verified `DashboardPage.tsx:257` is a pure client-side `.filter()` over local state; no server fallback. **Disposition: accept-risk for launch (owner=Product)** — live name/item search misses in-flight orders until the next poll/reload backfills the card (seconds); option to build a debounced server-search fallback deferred to a follow-up. New risk R11 in proposal §10. **Product owns the ruling.**
- **Redundant P0-1 index:** verified `…100041.ts:24` already has `courier_assignments_courier_idx(courier_id, status)` — identical tuple. **The new index is DROPPED from scope** (proposal §5 step 1); §3a now cites the pre-existing index.
- **Two-CHECK footgun:** documented as R12 (proposal §10) and in the ADR — `_not_whatsapp NOT VALID` is load-bearing and must never be dropped; `_channel_check` must not be read as "whatsapp allowed"; we deliberately do not ALTER `_channel_check` (would VALIDATE-fail on disabled rows).
- **§9 grep false positives:** the P0-2 proof predicate is rescoped to `apps/api/src/notifications/**` + `provider.ts`, explicitly EXCLUDING `courier/me.ts:79` (wa.me click-to-chat) and `spa-shell.ts:14` (BOT_UA regex) — both verified legitimate, both must stay. (proposal §9.)

### NEW risks from R3 revisions
| # | New risk | Mitigation / disposition |
|---|---|---|
| R3-NR-A | Removing `items_summary` from `fetchOrderDelta` may break a dashboard consumer that reads `itemsSummary` off the `order.status` delta (mirror of NR-1 for the status path). | Same handling as NR-1: card uses `itemCount`; item names via RLS+JWT order-detail fetch. Breaker to grep web app for any read of `itemsSummary` off the `order.status` (not just `order.created`) bus payload. |
| R3-NR-B | The 12s heartbeat keeps posting a *stale* last-known position while the courier is stationary (the timer re-sends the same fix). | Acceptable — server stores append-only positions bounded by 24h purge; `fetchLatestPosition` returns the most recent, which is correct (the courier hasn't moved). Cosmetically the breadcrumb has duplicate points; harmless. |
| R3-NR-C | `message-bus.ts:48` log demotion touches the #-churn platform bus; a botched edit could drop the diagnostic entirely. | Keep `channel` + length at info (diagnostic preserved); only the payload body moves to debug. Single-line change, covered by existing bus tests. |
| R3-NR-D | Accept-risk on NR-1 (no server search fallback) means an owner searching for a just-arrived order's customer name finds nothing for a few seconds until backfill — could read as "search is broken." | Owner=Product ruling (R11). If Product rejects the risk, build the debounced server-search fallback in scope before GO. |

### R3 three-bucket split (for the STOP-ETHICS gate)

**RESOLVED (revised in design this batch — required for GO):**
- HIGH-1 — complete producer census; `itemsSummary` dropped at BOTH producers (`orders.ts` + `orderStatusService.ts`/`fetchOrderDelta`); `message-bus.ts:48` confirmed non-PII + demoted defence-in-depth. Zero-PII-on-the-bus provable by census.
- HIGH-2 — time-based GPS heartbeat (`COURIER_GPS_POST_INTERVAL_MS=12_000`); 403 retried within ≤12s independent of movement; no idle tracking (server still gates); NR-2 rebounded.
- Redundant P0-1 index dropped from scope.
- §9 P0-2 grep proof rescoped to exclude the 2 legitimate `'whatsapp'` survivors.
- Two-CHECK footgun + `message-bus.ts:48` demotion documented.

**ACCEPT-RISK (+owner):**
- R11 / NR-1 — live customer-name/item search misses in-flight orders until backfill. **Owner: Product** (lean accept; build server fallback only if Product rejects).
- R12 — overlapping `_channel_check` / `_not_whatsapp` constraint pair. **Owner: Architect** (documented; do not drop `_not_whatsapp`).
- R3-NR-A…D — handed to breaker / provisionally accepted with stated mitigations.

**HUMAN-DECISION → STOP-ETHICS gate (proposed defaults stated; human rules):**
- HD-1 — idle-courier map dot. **Proposed default: (a) privacy-max** (operational loss documented). Pending owner ruling.
- HD-2 — default Telegram detail level. **Proposed default: `area` best-effort** (`full` opt-in, canary on full-adoption). Pending owner ruling.
- R11 escalates to Product as an accept-risk-or-build call (named above).
