# Breaker Findings — P0 Privacy Hardening (slug: `p0-privacy-hardening`)

> Note: the R1 findings file was never persisted to disk (the resolution's "Source note"
> reconstructed them from the RESOLVE brief). This document opens directly at the R2
> re-attack round against the **updated** `proposal.md` + `resolution.md` (§G NR-1..NR-5),
> re-verified against live code at `HEAD` of `feat/golive-remediation`.

## RE-ATTACK (R2)

**Framing — this is a PRE-IMPLEMENTATION re-attack.** The resolution is a *design* revision;
the fixes are **not yet in the code**. I verified the design against the live tree and the
verdict is split: the P0-3 *pivot* is sound in principle but its scope is **incomplete** (it
misses a second PII producer), and two NR mitigations (NR-1, NR-2) describe code that does not
yet exist and will break / no-op against the current code. Severity below reflects "what
happens if the batch ships as the proposal currently specifies it."

---

### [HIGH] B-SEC / regression · P0-3 enumeration misses a SECOND dashboard PII producer — `orderStatusService.ts` ships `itemsSummary` on the bus

The proposal's P0-3 table (§4) and resolution §B enumerate exactly **one** dashboard producer:
`orders.ts:722-737`. There is a **second** producer the design never mentions:
`apps/api/src/lib/orderStatusService.ts:108-114` calls `fetchOrderDelta()` and publishes its
result to `dashboardChannel(dbLocationId)` on **every order status transition** (CONFIRMED,
PREPARING, READY, IN_DELIVERY, ...). `fetchOrderDelta` (orderStatusService.ts:6-29) selects and
returns:
```
items_summary: string_agg(quantity || '×' || name_snapshot)   -- PII-adjacent (dietary/medical signal)
```
and publishes `{ type:'order.status', data:{ ...delta, itemsSummary, ... } }`.

- **Scenario:** order moves PENDING→CONFIRMED. `orders.ts` is fixed to drop PII, but every
  subsequent transition re-publishes the full `itemsSummary` ("2×Insulin-friendly bowl, 1×...")
  onto the un-RLS'd LISTEN/NOTIFY transport — the exact leak P0-3 claims to close. The pivot
  fixes `order.created` and leaves `order.status` deltas leaking item names on every status
  change.
- **Invariant violated:** §2 "P0-3 keeps PII off the realtime bus" — false; the bus still
  carries item-summary PII via a producer outside the proposal's enumeration. The deliverable
  §4 explicitly demands "every realtime-bus producer" — this one is absent from the table.
- Also note `message-bus.ts:48` logs full payload verbatim → `items_summary` lands in logs too.

This is the single biggest hole in the R2 design: the pivot's correctness rests on "minimize at
EVERY producer," and the producer census is incomplete.

---

### [HIGH] B-FAIL · NR-2 is unimplemented AND the current GPS-post path cannot back off — it is event-driven, not timer-driven

NR-2 / proposal §4 P0-1 claim the client "backs off and retries (exponential backoff + jitter,
cap ~30-60s)" on `403 GPS_NOT_ON_ACTIVE_DELIVERY`. Verified against the live courier client:

- The GPS post is `apps/web/src/pages/courier/DeliveryPage.tsx:138-153`, a
  `useEffect(..., [position])` that fires **only when the geolocation `position` object changes**.
  Its error handler is `.catch((err) => console.debug(...))` (line 150-152) — no retry, no
  backoff, no status inspection. There is **no backoff machinery anywhere** in the file
  (grep: zero `backoff`/`retry`/`403` handling).
- **Structural problem the design ignores:** retry-with-backoff requires an independent timer.
  The current poster has none — it is driven by the OS geolocation watcher. When the courier is
  stationary (waiting at pickup, the exact moment `accepted` may lag), `position` does **not**
  change, so the effect does **not** re-fire → "retry the instant the courier accepts" cannot
  happen as designed; tracking only resumes on the next GPS jitter, which may be tens of seconds
  to minutes when stationary. The proposal's "tracking resumes the instant the courier accepts"
  is unachievable without adding a real timer the design does not specify.
- **Invariant/claim violated:** §10/R8 + resolution §C "client BACKS OFF on 403, does not
  hard-stop" — the code neither hard-stops nor backs off; it drops the ping and waits for an
  unrelated event. The HIGH that the pivot claimed to *fix* (assigned-but-driving courier goes
  invisible) is **still open** because the recovery mechanism does not exist in code.

Thundering-herd sub-point (NR-2 proper): once a real timer IS added, the §3a math holds (8
couriers, <1/s indexed EXISTS) — the herd is not the risk. The risk is the opposite: the timer
doesn't exist, so there is no herd and no recovery.

---

### [MEDIUM] B-CONSIST · NR-1 — `dashboard-utils.ts` does NOT crash, but `order.created` rows render with broken identity, and name-search silently breaks

Re-attack of NR-1 (does `dashboard-utils.ts` crash on undefined name/itemsSummary?). Verified
`apps/web/src/pages/admin/dashboard-utils.ts`:

- **No crash** — `mergeDelta` uses `payload.customerNameMasked || undefined`,
  `payload.itemsSummary || ''`, `payload.courierName || null` (lines 26-31, 47-48), all
  null-coalesced. Confirmed: dropping those keys yields `customerName: undefined`,
  `itemsSummary: ''`. NR-1's "graceful degradation" claim holds for the no-crash part.
- **But two real degradations the proposal under-states:**
  1. `DashboardPage.tsx:257` name-search:
     `o.customerName?.toLowerCase().includes(q) || ... || o.itemsSummary?.toLowerCase().includes(q)`.
     For any in-flight order whose card came from the bus, `customerName` and `itemsSummary` are
     now empty → **the order is invisible to name/item search** until a full order-list fetch
     backfills it. The proposal (§4 P0-3, "name-search falls back to on-demand authenticated
     fetch") describes a fallback that **does not exist in the code** — the search is a pure
     client-side `.filter()` over local state with no server fallback. NR-1's mitigation is
     aspirational, not implemented.
  2. `orderDeltaChanged` (dashboard-utils.ts:3-11) compares `itemsSummary` and `customerName`.
     When a later full fetch DOES backfill the real name, `mergeDelta`'s `payload.customerNameMasked
     != null` guard (line 47) means a bus delta will **never overwrite** a backfilled name with a
     blank — good — but it also means the masked-placeholder→real-name transition relies entirely
     on the non-bus fetch path. If that fetch path is the existing interval poll, "seconds" is
     optimistic only if the poll interval is short; verify the actual interval before claiming R7
     is "seconds."
- **Invariant:** none hard-violated; this is a UX-correctness degradation. MEDIUM because
  name-search breaking for live orders is a real owner-workflow regression that R7 dismisses as
  "a few seconds of name pending" — understated, since the *fallback fetch* it leans on is not
  written.

---

### [MEDIUM] B-DATA · P0-1 guard index ALREADY EXISTS — migration step 1 is a redundant no-op (and the proposal asserts it as new)

§5 migration step 1 and §3a both state the guard needs a **new** index
`courier_assignments_courier_status_idx ON courier_assignments(courier_id, status)`. Verified
`packages/db/migrations/1780421100041_courier-assignments.ts:24` already creates
`courier_assignments_courier_idx ON courier_assignments(courier_id, status)` — **identical
column tuple**. The EXISTS guard is already index-served today.

- **Scenario:** harmless at runtime (`CREATE INDEX IF NOT EXISTS` no-ops), but it signals the
  back-of-envelope (§3a "the guard adds one indexed SELECT… served by an index on
  (courier_id,status)") was written without checking the schema — the index it credits as the
  reason the guard is cheap was never the new index; it predates this batch. No correctness
  break; flagged so the migration doesn't ship a misleading "added index for the guard" line.
- **Invariant:** none. LOW-to-MED only as an anti-pattern (claimed work that isn't work).

---

### [MEDIUM] B-CONSIST / P0-2 · the existing whatsapp-permitting CHECK constraint is never removed; the NOT VALID design layers on top of it (works, but is undocumented and fragile)

NR-3 re-attack. Verified `packages/db/migrations/1790000000020_notification_channel_whatsapp.ts`
created a **validated** constraint `owner_notification_targets_channel_check CHECK (channel IN
('telegram','push','whatsapp'))`. The proposal §5 step 2 adds a **separate** constraint
`owner_notification_targets_channel_not_whatsapp CHECK (channel IN ('telegram','push')) NOT VALID`
and never mentions the pre-existing `_channel_check`.

- **It does work:** both constraints are ANDed; a new `channel='whatsapp'` row fails the NOT VALID
  one. Existing disabled whatsapp rows survive (NOT VALID skips them; the old `_channel_check`
  already permits them). So R3/NR-3's core claim holds.
- **The fragility the proposal misses:** the schema now carries **two contradictory channel
  constraints** — one (validated) saying whatsapp is legal, one (NOT VALID) saying it isn't. A
  future "tidy constraints" pass that drops the redundant-looking `_not_whatsapp` (it overlaps
  `_channel_check`!) silently re-opens whatsapp writes. NR-3 only warns against `VALIDATE`; the
  real footgun is the **duplicate/overlapping** constraint pair, which the design does not
  acknowledge. Cleaner would have been to ALTER the existing `_channel_check`, but the proposal
  forbids touching it (so it can't VALIDATE-fail) — leaving the contradiction.
- **Migration tooling check (NR-3 specific ask):** verified `node-pg-migrate` (package.json:24,
  `migrate:up`) — it does **not** auto-`VALIDATE CONSTRAINT` on boot; migrations are explicit
  forward files. So no auto-validate footgun. NR-3's "verify tooling does not auto-validate" =
  PASS.
- **Invariant:** §2 "no new whatsapp, ever" — achieved, but via a contradictory constraint pair.
  MEDIUM.

---

### [MEDIUM] B-ANTIPATTERN · P0-2 "no 'whatsapp' references remain" proof claim is false against the live tree — two legitimate non-Baileys references survive

§9 proof spec: "build + grep proving no `baileys`/`WHATSAPP_`/`renderWhatsAppMessage`/`'whatsapp'`
references remain." Verified two `'whatsapp'` literals that are **correctly out of scope** but
will make that grep proof FAIL:

- `apps/api/src/routes/courier/me.ts:79` — `messenger_kind: z.enum(['telegram','whatsapp','viber'])`.
  This is the courier's customer-facing **click-to-chat** contact handle (`wa.me/...` deep-link,
  migration `...038_messenger-deeplink.ts`), an **official** Meta link, NOT Baileys, NOT a TOS
  violation. Must NOT be removed.
- `apps/api/src/lib/spa-shell.ts:14` — `whatsapp` appears in the `BOT_UA` crawler-detection regex
  (WhatsApp link-preview bot). Unrelated; must stay.

- **Scenario:** an engineer follows §9 literally, greps `'whatsapp'`, gets 2 hits, and either (a)
  wrongly deletes the courier messenger channel + the bot-UA token (a real regression), or (b)
  declares the proof failed and blocks GO on a false positive.
- **Invariant:** Mandatory Proof Rule — the proof predicate must be specific enough to pass when
  the code is right. As written it conflates Baileys-removal with all `'whatsapp'` strings.
  MEDIUM (proof spec must scope to the Baileys adapter / notification channel, not the substring).

---

### [LOW] B-DATA / P0-4 · regex fail-closed is sound in design; nothing to verify in code yet, but the "no house number leaks" guarantee is untestable as specified

P0-4 re-attack (NR-4). The honesty correction (area ≈ minimal on free-text Albanian addresses)
is intellectually honest and the fail-closed direction (ambiguous → omit address) is correct.
No code exists yet to attack. One residual:

- The proposal commits to "drop trailing house-number tokens (regex)" but never specifies the
  regex, so the proof "Telegram render contains no house number at `area` level" (§9) cannot be
  asserted against a concrete pattern. A regex that strips *trailing* digits will still leak a
  **leading** or **embedded** house number ("Rr. Myslym Shyri 23, kati 2" → strips "2" not "23"
  if "kati 2" is the trailing token). Fail-closed only triggers on *parse failure*, not on a
  *confident-but-wrong* parse. NR-4 frames the risk as "owners flee to full"; the sharper risk
  is a **confident mis-strip emitting a partial address**, which fail-closed does NOT catch.
- **Invariant:** "no house number can leak" (§4 P0-4) — provably true only for the fall-back path,
  not the success path. LOW because area is rarely the success path on real data (per D), so
  blast radius is small — but the guarantee is overstated.

---

### [LOW] B-ANTIPATTERN · NR-5 i18n drift — `courier.gps_boundary_note` does not yet exist; only `gps_active` is present in all three locales

Verified `packages/ui/src/lib/i18n.ts:388,:1308,:2228` contain `courier.gps_active` (sq/en/uk);
`gps_boundary_note` is **absent** from all three (grep: zero hits). Confirms F-1 is unimplemented
(expected, pre-implementation). NR-5's mitigation (single key, same edit, Playwright visibility
assert) is adequate **if** the Playwright assert covers the rendered string on
`DeliveryPage.tsx`. LOW, no break.

---

## R2 verdict — regressions & verification status

**Did any R2 fix open a NEW hole?**
- The P0-3 *pivot* itself is sound (no re-fetch → both R1 CRITICALs genuinely dissolve;
  `websocket.ts:36` confirmed still synchronous/verbatim, raw-pool reads confirmed at :87/:96
  with no `set_config`). **But** its **scope is wrong**: the producer census omitted
  `orderStatusService.ts` (HIGH above), so "PII off the bus" is not actually achieved by the
  design as enumerated.
- NR-1: no crash (verified), but the on-demand-fetch fallback it relies on is not in the code and
  name/item search silently breaks for live orders (MEDIUM).
- NR-2: the backoff/retry it promises is not in the code and **cannot work** with the current
  event-driven poster without new timer machinery (HIGH).

**Unresolved CRITICAL/HIGH after R2:** TWO HIGH remain open as the proposal is currently written —
(1) the second PII producer `orderStatusService.ts` leaking `itemsSummary`, and (2) the NR-2
backoff being both unimplemented and structurally incompatible with the current poster. Both are
**design-completeness** breaks, not "implement it and you're done" — the design must (1) extend
the producer census to `orderStatusService.ts` / `fetchOrderDelta`, and (2) specify the timer
that makes "retry on accept" possible. Until then I cannot certify "zero HIGH remain."

**PASS items (R2 confirmed resolved / no break):**
- P0-3 ordering + tenant-scope criticals dissolve under the pivot (no re-fetch at the seam).
- NR-3 migration tooling does not auto-VALIDATE (node-pg-migrate, explicit files).
- NR-1 does not crash `dashboard-utils.ts`.
- P0-1 guard is already index-served (the "new" index is a redundant no-op — verify, don't add).
- `messenger_kind`/`wa.me` and the bot-UA `whatsapp` token are correctly out of P0-2 scope.

**HUMAN-DECISION audit (HD-1/HD-2): both are genuine human calls, not punts.**
- HD-1 (idle-courier map dot): the guard literally deletes the data feeding
  `courier-events.ts:155-164` `handlePositionUpdated` → `fetchLatestPosition` (:91-99), which exist
  *specifically* to show idle on-shift couriers on the owner map (comment :156-157, confirmed).
  This is a real product feature vs a real privacy posture — not architect-resolvable. Correctly
  escalated. (One technical note: the guard is in the *REST ping path* `shifts.ts:374`; if idle
  couriers stop posting positions, `fetchLatestPosition` returns the last pre-idle row until 24h
  purge, so dots don't vanish instantly — they go *stale*, then disappear at purge. The operator
  should rule knowing it's "stale then gone," not "gone immediately.")
- HD-2 (default telegram_alert_detail): because `area` is empirically ≈ `minimal` on free-text
  addresses (verified-by-reasoning, no structured address field exists), the *effective* default
  changes existing owners' dispatch workflow. Genuinely a workflow/owner call, not resolvable in
  code. Correctly escalated.

---

## RE-ATTACK (R3) — convergence check

**Framing:** still pre-implementation; this is a *design*-completeness re-attack against the R3
resolution + the updated proposal §3a/§4/§5/§9/§10. Goal: certify the two R2 HIGH are genuinely
closed and that no R3 revision opened a new CRITICAL/HIGH. Every line below re-verified against
live `HEAD` of `feat/golive-remediation`.

### R2 HIGH-1 (second PII producer) — VERIFIED CLOSED

I independently re-ran the producer census. **`grep -rn '\.publish(' apps/api/src packages/platform/src`**
returned ~60 publish sites; I read every one that targets `dashboardChannel`, `orderChannel`,
`courierChannel`, or `location:*:dashboard`. The customer-PII carriers are **exactly two**, both
named and both stripped by R3:

- `orders.ts:722-737` (`order.created`) — live code still ships `customerNameMasked`,
  `customerPhoneMasked`, `itemsSummary`, `courierName` (confirmed verbatim in the running file).
  R3 drops all four → `{orderId, locationId, status, total, currency, itemCount, shortId, createdAt, seq}`.
- `orderStatusService.ts:108-114` via `fetchOrderDelta:6-29` — confirmed: the `string_agg(quantity||'×'||name_snapshot)`
  subquery (`:10-11`) is selected and returned as `itemsSummary` (`:26`), published in the
  `{type:'order.status', data:{...delta, statusUpdatedAt}}` body on **every** transition. R3
  removes the subquery + the return field.

**Census negatives I personally confirmed carry NO customer name/phone/address/item-names:**
`orders.ts:704` (ORDER_CREATED — total/currency/status only), `orders.ts:713` (order.status, status only),
`orders.ts:934` (`assignment.created` — orderId+courierId), `owner/dashboard.ts:318/381` (`order.status` delta
with **only** `{orderId,status,statusUpdatedAt}` — no items, no name — verified), `server.ts:743`
(courier-update, IDs only), `owner/signals.ts:152/183` (signalId/customerId(UUID)/kind),
`owner/alerts.ts:136/176` (alertId/orderId/kind), `dwell-monitor.ts:106`, `anonymizer-gdpr.ts:80`,
`courier-events.ts` (courier's own masked contact on the per-order channel — out of scope, correct).
**Notable correct-by-design site the census table omits but I verified safe:** `owner/reveal-contact.ts:64`
publishes `{type:'customer.contact_revealed', data:{orderId, revealedAt}}` — the revealed name/phone
go in the **HTTP reply body** (`:73-75`), NOT on the bus. PII-free on the bus. Good.

**`message-bus.ts:48` log:** confirmed it `console.log`s `msg.channel` + `msg.payload` verbatim,
bypassing the redacting structured logger. After both producers are minimized the payload is
non-PII, so the verbatim log no longer leaks customer name/phone/items. The R3 defence-in-depth
demotion (channel+length at info, body at debug) is a sound hardening, not a blocker.

**Verdict: HIGH-1 CLOSED.** The census is now complete and grep-reproducible; "zero customer PII
on the bus" is provable by enumeration, not by trusting one file.

### R2 HIGH-2 (client cannot back off / stationary-403) — VERIFIED CLOSED by the time-based design

Re-verified the live break: `DeliveryPage.tsx:138-153` is `useEffect(..., [position])` with a
`lastPingRef` 12s **throttle** (`:137-141`) and a bare `.catch(console.debug)` (`:149-151`). It
fires **only** when `useGeolocation` pushes a new `position` (`use-geolocation.ts:32` `watchPosition`
→ `setPosition`). Confirmed: stationary courier → `position` unchanged → effect never re-fires →
403 never retried → assigned-but-not-accepted courier stays invisible. The R2 "exponential backoff"
was structurally impossible (no timer). **The R3 design (replace the effect with
`setInterval(COURIER_GPS_POST_INTERVAL_MS=12_000)` re-posting the last-known `position` from hook
state) genuinely solves it:** the next interval fires regardless of movement, so a 403 is retried
within ≤12s of accept. Confirmed `COURIER_GPS_POST_INTERVAL_MS` does not yet exist in code
(grep: zero hits) — expected, pre-implementation.

- **Idle tracking NOT reintroduced — verified.** The server ping handler `shifts.ts:355-378`
  currently INSERTs a position whenever `courier_shifts.status IN ('available','on_delivery')` —
  i.e. it does NOT yet gate on assignment. P0-1 adds the `courier_assignments.status IN
  ('accepted','picked_up')` guard. With that guard the server 403s+discards an idle heartbeat
  regardless of the client timer firing → client posting is harmless, server is authoritative.
  Confirmed reconciled with HD-1.
- **Double-poster — none, IF implemented as written.** The design says *replace*
  `useEffect([position])`, not add alongside it. Verified there is exactly one poster today; no
  second timer exists. (See R3-LOW-1 for the throttle-vs-interval edge.)
- **Battery/rate — sane.** ≤1 POST/12s/courier (~300/hr); the GPS radio is already on via the
  mounted `watchPosition`; the timer reuses last-known state, adds no second GPS subscription.
  8 couriers → ≤0.67 INSERT/s peak. Guard is index-served by the pre-existing
  `courier_assignments_courier_idx(courier_id, status)` (`…100041:24`, identical tuple — confirmed;
  the redundant new index is correctly dropped from scope). Thundering-herd: fixed interval, no
  ramp. Sound.

**Verdict: HIGH-2 CLOSED** as a design (the recovery mechanism now exists in the spec and is
structurally capable of retrying a stationary 403).

### R3 new risks R3-NR-A..D — re-attacked

#### [MEDIUM] R3-NR-A · `order.status` delta IS read by the dashboard, but dropping `itemsSummary` degrades search, never crashes — confirmed (downgrade from HIGH)

I traced the full consumer path the resolution asked me to check. `DashboardPage.tsx:122-130`:
`payload = envelope.data || envelope`; on `envelope.type === 'order.status'` it calls
`mergeDelta(prev, payload, false)`. `mergeDelta` (`dashboard-utils.ts:36-50`) **does** read
`itemsSummary` off the delta: `...(payload.itemsSummary != null && { itemsSummary: payload.itemsSummary })`.

- **No crash, no blank-overwrite (good).** When R3 drops `itemsSummary` from the delta, the
  `!= null` guard is false → the merge keeps the **existing** `itemsSummary` on the card. And the
  `isNew=false` path with `i === -1` returns `prev` unchanged — a status delta **never creates** a
  card, so there is no "status-path-born order with empty items" failure. R3-NR-A's worst case
  (status path breaks a card) does **not** materialize.
- **The real residual = search, same as R7/R11.** `DashboardPage.tsx:257`
  `o.itemsSummary?.toLowerCase().includes(q)` + `o.customerName?...`. A card born from
  `order.created` (now PII-stripped) has `itemsSummary:''` and `customerName:undefined`, so
  **item-name and customer-name search miss that in-flight order** until a full order-list fetch
  backfills it. The "on-demand authenticated search fallback" the proposal leans on **still does
  not exist** — `DashboardPage.tsx:257` is a pure client-side `.filter()` over local state
  (re-confirmed). This is exactly R11; R3-NR-A is its mirror on the status path and collapses into
  the same accept-risk.
- **Invariant:** none hard-violated. MEDIUM, fully covered by R11 (owner=Product). Not a new HIGH.

#### [LOW] R3-NR-B · stale-position heartbeat — agreed, harmless

The 12s timer re-posting an unchanged fix while stationary produces duplicate breadcrumb points,
bounded by the 24h purge; `fetchLatestPosition` returns the correct (latest=same) row. No data
integrity or scale issue. As dispositioned.

#### [LOW] R3-NR-C · `message-bus.ts:48` log demotion on the #1-churn platform file — agreed, low

Single-line change on a hot file; keeping `channel`+length at info preserves the diagnostic. The
only real exposure is a botched edit dropping the notification-received signal — caught by the
existing bus tests the proof list cites. LOW.

#### [LOW] R3-NR-D · "search looks broken" for a just-arrived order — agreed, = R11

Owner=Product ruling (build debounced server search vs accept the few-second backfill window).
Not a new severity; folded into R11. LOW.

### R3-NR-A..D verdict: none rises to HIGH. A and D are facets of the already-tracked R11 accept-risk.

### New residuals found this round (not previously listed)

#### [LOW] R3-LOW-1 · 12s `setInterval` colliding with the existing 12s `lastPingRef` throttle could intermittently skip a heartbeat

If the implementer keeps the existing `lastPingRef` 12s throttle (`DeliveryPage.tsx:137-141`) AND
adds a 12s `setInterval`, a timer tick landing a few ms inside the throttle window
(`now - lastPingRef < 12000`) is silently dropped → that beat skipped, next real post ~24s later.
For the stationary-403-retry case this means worst-case recovery is ~24s, not ≤12s as claimed.
- **Scenario:** courier taps accept at T; timer ticks at T+0.05s but `lastPingRef` was T-11.96s →
  throttled+skipped; next tick T+12.05s posts → ~12s recovery anyway in most phases, but a
  pathological phase alignment doubles it. LOW because (a) it self-corrects within two intervals
  and (b) it only bites if both the throttle and the timer survive the rewrite. Flag: the design
  says "replace the effect" — if the throttle is removed with it, this evaporates. The proposal
  does not state the throttle's fate. **No invariant; LOW.**

#### [LOW] R3-LOW-2 · §3a/§3b internal number inconsistency (position-post rate)

§3a (R3-updated) correctly states the timer caps posts at **≤0.67 INSERT/s** (8 couriers × 1/12s).
§3b still reads "position updates (**0.8/s = 48/min**, the dominant term)" — the pre-R3 event-driven
figure. Harmless (the §3b figure is now a conservative over-estimate, not an under-estimate, so the
fan-out budget is not understated) but the two sections disagree on the same quantity. **No break;
LOW — cosmetic doc drift, flagged so the back-of-envelope reads consistently.**

### Accept-risk owner audit (brief item 4)

Spot-checked §10: every row R1–R12 carries an explicit Owner cell (R1 Architect→Backend, R2 Product,
R3 Architect, R4 Ops, R5 Architect, R6 Architect, R7 Architect→Frontend, R8 Architect→Frontend,
R9 Architect, R10 Product, R11 Product, R12 Architect). HD-1/HD-2 carry proposed defaults + "pending
owner ruling" at the STOP-ETHICS gate. **No orphaned risks.** R11 (the only live accept-vs-build
call) is correctly escalated to Product with both options stated.

### Regression check (did R3 reopen anything from R2?)

- P0-3 pivot: `websocket.ts` room handler not touched by R3 → still synchronous/verbatim, no
  re-fetch. CRITICAL-1/CRITICAL-2 stay dissolved. No regression.
- Redundant index: confirmed dropped from §5 scope; §3a now cites `…100041:24`. R2 MED resolved.
- §9 grep: rescoped to `apps/api/src/notifications/**` + `provider.ts`, explicitly excluding
  `courier/me.ts:79` (wa.me) and `spa-shell.ts:14` (BOT_UA) — both re-verified as legitimate
  survivors that must stay. R2 MED resolved.
- Two-CHECK footgun: confirmed live (`…020:12-13` validated `_channel_check` permits whatsapp; new
  `_not_whatsapp NOT VALID` forbids it) → documented as R12 with the "never drop `_not_whatsapp`,
  never ALTER `_channel_check`" caveat. Migration tooling re-confirmed `node-pg-migrate` v8 explicit
  forward files, no auto-VALIDATE. R2 MED resolved.

### R3 convergence verdict

Both R2 HIGH are genuinely closed at the design level: HIGH-1 by a complete, grep-reproducible
producer census stripping `itemsSummary`+name+phone at BOTH producers; HIGH-2 by a time-based
heartbeat that can retry a stationary 403 and does not reintroduce idle tracking (server stays the
hard gate). The four R3-NR risks are all MEDIUM-or-lower; R3-NR-A and R3-NR-D are facets of the
already-tracked R11 (owner=Product) accept-risk, not new findings. Two fresh LOWs (throttle/interval
collision; §3b stale number) are cosmetic/implementation-detail. No accept-risk is orphaned.

**This is a clean exit signal: no CRITICAL or HIGH remains unresolved.** The remaining open items
are (a) MEDIUM/LOW accept-risks with named owners and (b) two HUMAN-DECISIONs (HD-1 idle dot, HD-2
default detail level) correctly routed to the STOP-ETHICS gate with proposed defaults. Standard
caveat: this certifies the *design*; the Mandatory Proof Rule still binds at implementation
(Playwright/integration proof per §9, especially the BOTH-producers no-PII assertion and the
stationary-403-retry assertion).

**CRITICAL remaining: 0; HIGH remaining: 0**
