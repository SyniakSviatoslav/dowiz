# Layer Blueprint — Notifications · Push · Courier Signal — 2026-07-11

> **Layer blueprint** for the NOTIFICATION / PUSH / COURIER-SIGNAL layer of the local-first hub
> program. Closes the hub review's #1 courier gap (*"couriers get NO out-of-app notifications — a
> locked phone silently loses dispatch after the 5-min sweep"*, hub review §4.6-1) and the
> customer/owner status-delivery story, reconciled with the anonymity design's sharpest surviving
> floor (*the push gateway is a de-anonymizing Big-Tech edge*, 04-revision §4.3).
>
> **Method:** read-only design session — repos untouched (code is being changed by a parallel
> session); the ONLY file created is this blueprint. Grounds: hub review
> (`docs/research/2026-07-11-hub-architecture-review.md` §4, §5.3),
> `C-runtime-transport-identity.md` (§1.3 background wall, §4.1 push-woken peers),
> `03-anonymity-architecture.md` (§3.5), `04-anonymity-mesh-messenger-revision.md` (§4.3, §5),
> the live notification code (`apps/api/src/notifications/*`), the particle-cloud analysis
> (`docs/research/2026-07-11-particle-cloud-interaction-analysis.md` §4), and fresh 2026-07-11 web
> research on APNs/FCM/Web Push/UnifiedPush. Labels: **VERIFIED** (code read or primary web source
> fetched this session) · **VERIFIED-in-repo** (from a cited repo doc that verified it) ·
> **VERIFIED-secondary** (reputable secondary source) · **UNVERIFIED** (assessment, flagged).
>
> **Standing decisions respected throughout (binding):** local-first ratified · COD mandatory
> (settlement notifications are obligation-state, never payment receipts) · **NO courier scoring**
> · anonymity a stated value · multichannel / **no dedicated app** · storefront sovereignty.
> Parent session consolidates layer docs — **no Telegram/etc. transport build is licensed by this
> doc**; intake transports stay G7-survey-gated (hub review §7.2). This layer is *outbound only*.

---

## 0. Verdict + the current truth table

**One sentence:** the hub already owns a disciplined outbound spine (event registry, category
gating with reversibility-of-consequence, timezone quiet-hours with zero-silent-drop, circuit
breakers, audit ledger — all VERIFIED) but points it at exactly one audience (the owner) plus a
half-broken customer web-push path — while the audience whose *job* depends on a beep (the courier)
gets nothing; the fix is not a new notification system, it is **aiming the existing one at two more
audiences over rails that are honest about the 2026 OS reality**, with a falsifiable
delivery-receipt loop so a silent drop is a RED alarm instead of a lost dinner.

What exists today (all VERIFIED in code this session unless noted):

| Audience | In-app | Out-of-app | Status |
|---|---|---|---|
| **Owner** | WS dashboard rooms | Telegram (confirm/reject buttons) + Web Push (`owner_notification_targets`, channel CHECK `telegram|push`, mig `1780348982032`) | LIVE; category gating dark behind `TG_CATEGORY_GATING`; 2 ops events broken by name drift (§4.2) |
| **Customer** | tracking page 15s poll + WS `order:<id>` | Web Push opt-in (`customer_devices`, `routes/customer/push.ts`, worker `handleCustomerStatus` — CONFIRMED/IN_DELIVERY/DELIVERED + DISPATCH_DELAYED/CANCELLED) | Server side LIVE; **device side broken: the served service worker has no `push` handler** (§1.5-N0) |
| **Courier** | WS `courier:<id>` + in-app sound (`TasksPage.tsx:26,89` — VERIFIED-in-repo) | **NOTHING.** No courier device table, no courier Telegram target, zero push registration | THE GAP. Locked phone ⇒ offer expires at the sweep unseen |

Two load-bearing numbers discovered in code that shape everything below (VERIFIED,
`workers/courier-offer-sweep.ts:120-141`): the auto-assign acceptance timeout defaults to **5 min**,
and the FE offer accept window (`COURIER_ACCEPT_WINDOW_MS`) is **30 s**. Any out-of-app signal whose
P95 latency is not comfortably inside those windows is theater — §1.4 makes the coupling a config
invariant.

---

## 1. The courier out-of-app signal (the load-bearing gap)

### 1.1 The gap, restated precisely

Every courier-lifecycle event today notifies the *owner* (Telegram) or the *customer* (webpush);
courier push tables do not exist (hub review §4.6-1, VERIFIED — push migrations `1780421100059`/
`1780348982033` are customer-only). A courier learns of `task_offered`/assignment only through an
open, foregrounded WS tab. The OS reality (C-lens §1.3, VERIFIED against Apple/Android docs): **a
backgrounded iOS app or PWA holds no sockets, ever; Android suspends network in Doze for non-exempt
apps.** So the WS tab is structurally incapable of being the alerting rail. The courier must be a
**push-woken participant** (C-lens §4.1): push carries a *wake signal*; state is re-fetched and
(later, local-first) signature-verified on reconnect — push is never trusted state.

### 1.2 The 2026 delivery-path matrix (researched fresh, honest about reliability)

dowiz has **no native app and no Firebase/APNs SDK anywhere** (VERIFIED — zero `firebase`/`apns`
deps in `apps/*/package.json`; `notifications/adapters/push.ts` is a fake-ack scaffold that returns
`delivered:true` without sending — a false-positive-metric hazard, retired in N0). Under the no-app
doctrine the realistic rails are:

| Rail | Reaches a locked phone? | 2026 reality | Verdict for the courier |
|---|---|---|---|
| **Web Push (VAPID, RFC 8030/8291/8292)** — the stack already in the repo (`web-push` lib, `adapters/webpush.ts`) | **Yes** | Android Chrome: delivered via FCM's transport under the hood; **iOS ≥16.4: only for Home-Screen-installed web apps** (VERIFIED-in-repo C-lens §1.2 — webkit.org; re-confirmed this session: [MagicBell 2026 guide](https://www.magicbell.com/blog/pwa-ios-limitations-safari-support-complete-guide) — *"push works for PWAs added to the Home Screen… no silent push or background wake"*). **Declarative Web Push** shipped Safari/iOS 18.4 (VERIFIED — [webkit.org/blog/16574](https://webkit.org/blog/16574/webkit-features-in-safari-18-4/), [WWDC25 §235](https://developer.apple.com/videos/play/wwdc2025/235/)) and is on the W3C track with positive Mozilla position by 2026 ([aimtell state-of-DWP 2026](https://aimtell.com/blog/state-of-declarative-web-push-2026), VERIFIED-secondary) — notification described in the push JSON itself, no SW code path to break (exactly the N0 bug class) | **PRIMARY rail.** Reuses the built vertical; no store account; fits no-app. Cost: courier must install the PWA to Home Screen on iOS (one guided step at invite-redeem time) |
| **FCM native (Android)** | Yes — `"android":{"priority":"high"}` punches Doze (VERIFIED — [firebase docs](https://firebase.google.com/docs/cloud-messaging/android-message-priority), [firebase blog Apr-2025](https://firebase.blog/posts/2025/04/fcm-on-android/)) | Requires a native app + Google account/SDK | **Deferred** — only if/when the C-lens courier native shell (UniFFI) ships (local-first Phase 4+). Not before |
| **APNs native (iOS)** | Alert push yes (priority 10); **silent/background push throttled to ~2-3/hr and OS-discretionary** (VERIFIED-secondary — [techconcepts](https://techconcepts.org/blog/ios-push-notifications), [bugfender](https://bugfender.com/blog/advanced-ios-push-notifications/)) | Requires a native app + Apple dev account | **Deferred**, same trigger. Note for the future: even native iOS gives no reliable *silent* wake — the alert-push (user-visible) is the only dependable signal, which the Web-Push rail already equals |
| **UnifiedPush (Android, de-Googled)** | Yes via a distributor app's own connection | 2026 state (VERIFIED — [F-Droid "5 years of UnifiedPush", 2026-01-08](https://f-droid.org/2026/01/08/unifiedpush-5-years.html)): live distributors **ntfy** (default-recommended, self-hostable), **Sunup** (Mozilla autopush-rs, no-account — [unifiedpush.org/users/distributors/sunup](https://unifiedpush.org/users/distributors/sunup/)), NextPush, Conversations; >20 apps incl. SimpleX; **the spec converged on Web Push — RFC 8291 encryption + RFC 8292 VAPID, endpoint = an RFC 8030 push resource** (VERIFIED — [unifiedpush.org spec definitions](https://unifiedpush.org/developers/spec/definitions/)); **Firefox forks (Fennec/IronFox) gained UnifiedPush-backed web push** | **FREE COMPATIBILITY, not a build.** Because a UnifiedPush endpoint is a standard RFC 8030 resource, the existing `web-push` sender delivers to it unchanged — the server must merely never assume Google/Mozilla endpoint hostnames (it doesn't — VERIFIED `adapters/webpush.ts` is endpoint-agnostic). De-Googled courier phone = IronFox/Fennec PWA + ntfy/Sunup distributor. Document + test it; write no server code |
| **Telegram bot message** | Yes — Telegram's own app push (riding FCM/APNs, Telegram's account) | Bot infra, deep-link connect flow, and courier `/open` already LIVE (hub review §5.2) | **SECOND rail** — independent failure domain, zero new transport code, works on any phone with Telegram in <1 session of work. The hub review's own "give couriers a beep" recommendation (§6.4-3) |

**Architecture: two independent rails behind the one dispatcher.** `NotificationDispatcher`
(`provider.ts:80-94`) already routes by `target.channel ∈ {telegram, push}`; courier targets reuse
both adapters verbatim. A dispatch event fans out to **every active courier target** (both rails if
connected) — redundancy is the reliability strategy, dedup is the UX strategy (§4.4: same
`tag`/`dedupKey` per order-event, so two rails collapse to one visible alert where the OS allows).

### 1.3 The delivery-receipt loop (how a silent drop becomes RED)

Web Push has no sender-side delivery receipt, and the current scaffold's `delivered:true` habit is
exactly the false-positive metric VbM forbids. The loop:

1. Every courier push payload carries `{event_id, nonce, sent_at}` (no PII — §2.3).
2. The service worker's `push` handler, after `showNotification`, fires
   `fetch('/api/courier/push/ack', {event_id, nonce, displayed_at})` (SW fetch inside
   `event.waitUntil` — standard, works while app closed). Telegram rail: the Bot API send response
   *is* the gateway-accept receipt (message_id), recorded as `gateway_ack` (weaker — accepted by
   Telegram, not displayed; labeled as such in the audit, never conflated).
3. Both land in the existing `notification_outbox_audit` (extended statuses `displayed_ack` /
   `gateway_ack` / `ack_timeout`) — one ledger for all audiences, same zero-silent-drop doctrine.
4. **The RED path:** a `task.offered`/`task.assigned` push with **no ack on any rail within
   ACK_RED_MS while the courier is on-shift** writes `push_silent_drop` to the audit **and raises
   the existing owner alert rail** (`order.dispatch_failed` pattern) so a human dispatches by voice
   before the 5-min sweep, not after. The customer keeps getting the honest `DISPATCH_DELAYED`
   push — never a false "on its way" (existing ETHICAL-STOP-1 doctrine, preserved).

**VbM for the loop itself (falsifiable):** staging Playwright — register a courier push
subscription, background/close the page, enqueue `task.offered`, assert a `displayed_ack` audit row
with `displayed_at − enqueued_at ≤ 30 s` (P95 target; N below). **RED case that must fail:** serve
the current `apps/api/public/sw.js` (no push handler — today's prod state, §1.5-N0) → ack rate 0 →
`push_silent_drop` fires. The bug that exists today is the proof the metric can go red.

### 1.4 The latency ⇄ TTL coupling rule (config invariant)

An offer a courier cannot see in time is a trap. Standing rule, enforced by a boot assert next to
`assertVocabulary()`: **`COURIER_ACCEPT_WINDOW_MS ≥ 4 × ACK_P95_TARGET_MS` and
`COURIER_ASSIGN_ACCEPT_TIMEOUT_MS ≥ 2 × COURIER_ACCEPT_WINDOW_MS`.** Concretely: with the push
target P95 ≤ 30 s, the 30 s FE accept window is **too small for out-of-app discovery** — flipping
`COURIER_OFFER_HANDSHAKE_ENABLED` requires raising the offer window to ≥ 120 s in the same change.
Web Push sends use `TTL: 120, urgency: 'high'` for offers (an expired offer must NOT arrive late
and dangle — today's `TTL: 86400` is wrong for offers, right for status), `Topic`/`tag` per order
so a re-offer replaces a stale one. Doze/OEM honesty (VERIFIED-in-repo C-lens §1.3 —
dontkillmyapp): on aggressive OEMs even high-urgency web push can be minutes late — which is why
the RED path escalates to the owner instead of pretending.

### 1.5 Phases (courier signal)

**N0 — Truth repair (prerequisite, effort S, ~0.5 session).**
*Entry:* none — pure defect closure; coordinates with the parallel code session (this doc only
specifies; that session implements).
*Modules:* `apps/api/src/client/pwa/sw.ts` (the source of the SERVED `apps/api/public/sw.js` —
VERIFIED `server.ts:158-159` serves `../public`) **gains the `push` + `notificationclick`
handlers** currently stranded in the dead sibling `apps/api/src/public/sw.js` (VERIFIED this
session: the served, minified worker has install/activate/fetch/message listeners only — customer
web push is server-sent but never displayed); delete the dead sibling; delete/neuter the fake-ack
`adapters/push.ts`; fix the two event-name-drift ops alerts (`bootstrap/messaging.ts:9-27`
enqueues `backup.failed`/`settlement.disputed`, registry names are `ops.backup_failed`/none —
broken by construction, hub review §5.3) and make `assertVocabulary()` validate event names against
`EVENT_REGISTRY`; fix the `c.name → full_name_encrypted` courier-enrichment bug
(`notifications/workers/index.ts:623,648`, hub review §4.6).
*VbM RED:* Playwright vs staging — subscribe → send `test` push → OS notification asserted
displayed; run the same spec against the pre-fix worker → it fails (the live bug is the RED
fixture). Vocabulary assert: enqueue `backup.failed` → boot/enqueue-time throw, not a Telegram 400.
*Dependencies:* none. *This phase unblocks every other phase and repairs the CUSTOMER path too.*

**N1 — Courier Telegram rail (the beep, effort S–M, ~1 session).**
*Entry:* N0 vocabulary assert green; owner bot live (it is).
*Modules:* migration `courier_notification_targets` (mirror of `owner_notification_targets`:
channel CHECK `telegram|push`, prefs, quiet_hours, status; RLS in the courier auth universe);
`telegram-webhook.ts` gains `/start c_<connect-token>` (the owner deep-link connect pattern reused,
`telegram-webhook.ts:605-644`); courier settings page gets the connect QR/deep-link (mirror of
owner `SettingsPage.tsx:236-253`); new registry events `task.offered` / `task.assigned` /
`task.revoked` (audience `courier`, §4.1) enqueued from the three existing choke points —
`owner/dashboard.ts` assign, `workers/courier-dispatch.ts` execute, `lib/bindingRelease.ts` — via
the existing transactional pg-boss pattern; render group `open_in_app` with a deep link to
`/courier/tasks`.
*VbM RED:* integration spec — assign order to courier with an active Telegram target → Bot API
send recorded with `gateway_ack` + message_id; revoke the target (401-simulate) → adapter
auto-disable path fires (existing `adapters/telegram.ts:14-53` behavior) and audit says `failed`,
not `delivered`. RED fixture: point the bot token at an invalid value → the spec must go red.
*Dependencies:* N0. *Non-goal:* no courier commands beyond the existing `/open`; acting on tasks
stays in the web app (offer-handshake FE catch-up is the separate hub-review item §7.1).

**N2 — Courier Web Push + ack loop (the falsifiable rail, effort M, ~1.5–2 sessions).**
*Entry:* N0 shipped (SW displays pushes); N1 optional but recommended first (independent rails).
*Modules:* migration `courier_devices` (mirror of `customer_devices`, courier universe);
`routes/courier/push.ts` subscribe/unsubscribe (mirror of `routes/customer/push.ts` incl.
410/404 pruning); `routes/courier/push-ack.ts` + audit statuses `displayed_ack`/`ack_timeout`;
SW `push` handler posts the ack (N0's handler + ~10 lines); `notifications/workers/index.ts`
gains `handleCourierDispatch` (clone of `handleDispatch` reading courier targets/devices, both
rails); offer sends switch to `TTL:120` + per-order `tag`; the §1.4 boot assert; iOS install
guidance at `/courier-invite` redeem ("Add to Home Screen" interstitial — push is impossible in a
Safari tab, VERIFIED-in-repo C-lens §1.2); ack-timeout sweep piggybacked on the existing 1-min
`CourierOfferSweepWorker` (no new worker).
*VbM (the layer's headline metric):* **a backgrounded courier device shows a dispatch alert with
P95 ≤ 30 s, P99 ≤ 120 s from enqueue** (measured from `notification_outbox_audit` timestamps —
deterministic, thresholded); **RED = `push_silent_drop`** (no ack on any rail ≤ 240 s on-shift) →
owner alert + audit row; proven by the N0 RED fixture plus a kill-the-SW-handler staging drill.
*Dependencies:* N0; VAPID keys already provisioned (owner webpush live).
*Explicitly not built:* native FCM/APNs SDKs (no app exists to host them — deferred to the
local-first courier shell, D-transition Phase 4); SMS (cost + SIM metadata, no demand evidence);
`declarative` Web Push payloads may be added additively later (same endpoint, JSON per WWDC25
format) — a robustness upgrade on iOS 18.4+, zero new infra (UNVERIFIED benefit size — measure).

**N2.5 — De-Googled courier option (documentation + test, effort S, ~0.5 session).**
*Entry:* N2 green. *Modules:* zero server code (endpoint-agnostic sender VERIFIED); a runbook page:
IronFox/Fennec + ntfy or Sunup distributor; one staging test against a real ntfy endpoint.
*VbM RED:* the staging test asserts a `displayed_ack` from a UnifiedPush-distributor endpoint;
RED = sender code that special-cases gateway hostnames (grep-gate: no `fcm.googleapis`/`push.apple`
literals in `apps/api/src`). *Dependencies:* N2.

---

## 2. The anonymity tension, resolved honestly (the tiered answer)

The floor, stated without cosmetics (04-revision §4.3, the sharpest surviving residual): **waking a
locked phone requires a push gateway — APNs or FCM (Web Push on Android Chrome rides FCM; on iOS
rides APNs; even SimpleX's iOS push routes via its notification server → APNs — VERIFIED-in-repo
04 §2.3). The gateway sees device-token ↔ app ↔ timing, retained, warrant-gated (EFF Apr-2026,
Wyden Dec-2023 — VERIFIED-in-repo).** No engineering inside dowiz removes that edge; the design
answer is to decide, per actor, whether the edge is acceptable — and never to pretend it is absent.

| Tier | Actor | Push identity to a Big-Tech gateway? | Rationale + rules |
|---|---|---|---|
| **T1 — Courier** | Vendor-employed, known-in-person worker | **ACCEPTED** (Web Push→FCM/APNs; Telegram→its own FCM/APNs) | Courier anonymity-from-vendor was never a goal (04 §4.2 — the SIM floor already lands on the worker; the courier runs registered cellular by necessity). The gateway edge adds token↔timing to an actor the state can already trace via carrier. **What still binds — NO COURIER SCORING:** the ack-latency telemetry (§1.3) is channel-health data, keyed by rail/device-class, aggregated per location; it is **never surfaced per-courier, never joined to assignments/ratings, never an input to dispatch order** (dispatch stays freshest-heartbeat — `lib/dispatch.ts`, VERIFIED-in-repo). Guardrail: the Stage-21 NO-AUTO-DEDUCT/NO-COURIER-SCORING invariant test extends to forbid any query joining `push acks × courier_id` outside aggregate ops views. Retention: ack rows purge with the existing audit retention. Payload rule R1 below applies (the gateway edge carries timing, never content) |
| **T2 — Default customer** (clearnet web, phone-bound messengers) | **OPT-IN, labeled** | Status rides the channel they chose (§3): a Telegram/WhatsApp customer already has that platform's push identity — dowiz adds nothing; a web customer may opt into Web Push (existing `customer_devices` flow) and the UI labels it truthfully per the 04 §2.2 doctrine: *"status alerts use your phone's push service (Apple/Google will see that a ping happened)."* The always-on hub guarantee (no profile, PII envelope, crypto-shred) is unaffected — it lives behind the adapter, not in the transport |
| **T3 — Anonymous customer** (`.onion` mirror / Tor Browser / no-phone messenger) | **NEVER** | A push subscription is a durable pseudonymous identity at a gateway — it would undo exactly what the customer chose the channel to avoid. Rules: the storefront **must not render the push opt-in on the onion origin** (origin-gated UI + server refuses `push/subscribe` for onion-session orders — fail-closed); status is **foreground-only**: the tracking page's existing 15 s poll / WS over the onion channel (`OrderStatusPage.tsx` behavior, VERIFIED-in-repo) — the honest UX copy: *"keep this tab open for live status."* Optional future (Android-only, self-hosted ntfy over Tor): a **content-free wake ping** via UnifiedPush is the one metadata-clean push path (03 §3.5, VERIFIED-in-repo: on iOS even self-hosted ntfy relays through APNs — there is no clean iOS path); offer it only as an explicitly-labeled expert option, never default |

Cross-tier payload rules (all rails, all audiences — these are what "content-free-ish" means here):

- **R1 — Minimal payloads.** Courier push: order short-id + "new task" + deep link — **never the
  delivery address, customer name/phone, or items** (the address is fetched inside the
  authenticated app on wake; C-lens §4.1: push is a wake signal, state is re-fetched). Customer
  push: status word + short-id + total (already the shape — VERIFIED `handleCustomerStatus`
  builds no-PII payloads). Web Push payloads are RFC 8291-encrypted in transit through the gateway,
  but R1 holds anyway — the endpoint device stores what it shows.
- **R2 — The gateway edge is timing-only by construction.** With R1, APNs/FCM learn *that* a
  courier/customer got pinged, never *what* — the exact minimization 03 §3.5 ratified.
- **R3 — Push is never authoritative.** Acting on a task requires the app to fetch signed/authz'd
  state; a forged push can at worst make a phone buzz (matters for the local-first future where
  the relay is untrusted).
- **R4 — No cross-audience identity reuse.** `customer_devices` / `courier_devices` /
  `owner_notification_targets` stay separate universes (matching the separate auth universes,
  VERIFIED); no shared device fingerprint.

---

## 3. Multichannel status delivery (customer/owner) — same channel back

### 3.1 Disambiguating the three things called "channel" (hub review §6.2.3)

The binding attribution ADR is explicit: `metadata->>'channel'` is write-only and **"never read
by … notifications"** (order-channel-attribution proposal §8, VERIFIED-in-repo). So multichannel
status delivery must NOT key on the attribution label. The correct keys already exist or are
adapter-owned:

1. **Attribution label** (13-value taxonomy) — analytics only. Untouched by this layer.
2. **Contact preference** — the customer's declared coordination handle
   (`customer.messenger_kind` × 6 kinds + handle, ADR-0016; G03 fix makes all 6 accepted). This is
   a *human* reply path (owner/courier taps a deep link to chat) — not a machine rail.
3. **Reply route (NEW, adapter-owned)** — when a future intake adapter (Telegram bot order,
   WhatsApp Cloud API, SimpleX bot) carries an order, the adapter records
   `order_reply_routes(order_id, transport, route_ref, expires_at)` at intake (e.g. a Telegram
   `chat_id`). Machine-deliverable status goes back over **that** route. The adapter that opened
   the conversation owns closing the loop — transports stay thin heads (REBUILD-MAP §6), and the
   kernel/notification core never learns platform specifics.

**Rule: status returns on the channel the order arrived on; push is only for app-context users.**
Today's reality — web is the only intake — makes the v1 matrix small and honest:

| Order came via | Machine status rail today | + Optional |
|---|---|---|
| Web storefront (incl. QR/subdomain/TMA wrapper) | Tracking page (poll/WS — always works, no identity) | Web Push opt-in (T2) |
| `.onion` mirror (fast-follow) | Tracking page over onion, foreground-only (T3) | UnifiedPush expert option |
| Telegram bot intake (FUTURE, G7-gated) | Bot message to the intake `chat_id` via `order_reply_routes` | — |
| WhatsApp Cloud API (FUTURE, G7-gated) | Template message on the same conversation (24h-window rules live in the adapter) | — |

### 3.2 What ships now vs what waits

Ships in this layer (**N3, effort M, ~1 session**):
*Entry:* N0 (customer SW displays pushes at all); G03 contract fix landed (6 messenger kinds
accepted — otherwise the contact-preference row is unreliable).
*Modules:* `order_reply_routes` migration + a `ReplyRouteRegistrar` seam in the adapter contract
(so the FIRST future transport lands on a rail instead of inventing one — the "scattered channel
definitions" lesson, hub review §6.2.2); customer status worker consults reply routes before/beside
`customer_devices` (today: zero rows — dormant by design, like the aggregator trait);
channel-truth labels on the tracking page + push opt-in copy (T2/T3 wording, §2); the onion
origin-gating of push opt-in (T3).
*VbM RED:* unit — an order with a reply route gets status via the route adapter mock, an order
without gets webpush-only; onion-flagged session POSTing `push/subscribe` → 403 + audit row (RED
fixture: remove the origin gate → spec fails). Dormant-rail guard: `assertVocabulary`-style boot
check that every `transport` value in reply routes has a registered adapter — an orphan route is a
boot error, not a silent no-status order.
*Dependencies:* N0, G03. *Explicitly deferred:* any actual messenger transport (G7 survey +
cart-token council remain the gate — this layer builds the socket, not the plug).

### 3.3 COD + sovereignty constraints (standing, restated as notification rules)

- **Settlement notifications are obligation-state.** `cash.reconcile_discrepancy`, shift-close
  cash summaries, and any future Stage-21 events phrase *cash held / owed / settled* — never
  "earnings", never "payment received" (the `courier_payouts.total_earned` mislabel is the
  cautionary tale — hub review §4.6-3). No notification implies auto-deduction (NO-AUTO-DEDUCT
  invariant).
- **Storefront sovereignty.** Every customer-facing message is venue-voiced (venue name in title —
  already the shape in `handleCustomerStatus`; VERIFIED), links go to the venue's storefront/track
  URL (`/s/:slug/...` or the venue subdomain), never a dowiz-branded portal; the owner controls
  targets and the activation gate (≥1 active alert channel before publish — VERIFIED
  `routes/owner/activation.ts`) stays the sovereignty invariant: **no silent vendor**. Per-venue
  bot identities (owner's own Telegram bot token) are a plausible future sovereignty upgrade —
  parked, not licensed (adds per-venue webhook ops; revisit at multi-venue scale).

---

## 4. The unified event vocabulary (in-app ambient + out-of-app push = ONE model)

### 4.1 One registry, three audiences, two contexts

The particle-cloud analysis already defines the in-app half: a fixed event→visual grammar
`(shape-target, palette, energy, transient|sustained)` fed by the same WS rooms
(particle analysis §4.2, VERIFIED-in-repo). This layer completes the pairing so that **out-of-app
push and in-app ambient are two renderings of one vocabulary**, never two lists that drift:

`EventEntry` (`notifications/event-registry.ts:8-14`) gains three fields:

```
audience:   'owner' | 'courier' | 'customer'          // who this event is FOR (one entry per audience-view of an event)
pushClass:  'alert' | 'status' | 'ambient-only'       // alert = out-of-app rails; status = coalescable push; ambient-only = never pushed (particle/WS only)
visual:     keyof PARTICLE_VOCAB | null               // the §4.2 tuple name — null for no in-app visual
```

New/changed entries (additive — the 21 owner events keep their exact semantics): courier
`task.offered` (alert, transactional, visual `courier.task_offered` — the countdown-ring morph),
`task.assigned`, `task.revoked` (alert; visual `scatter+desaturate`), courier-facing
`order.ready_for_pickup` view (alert), `shift.close_reminder` courier view (status, operational);
customer `status.confirmed/in_delivery/delivered/dispatch_delayed/cancelled` (status —
formalizing today's `CUSTOMER_STATUS_EVENTS` hardcode into the registry, closing the
`CUSTOMER_PUSH_EVENTS`-vs-worker duplication in `lib/registry.ts:59` — VERIFIED both exist
separately today). Category law extends unchanged: **reversibility of consequence** — `task.*` and
customer live-order status are `transactional` (never suppressible: a courier on shift accepted
the duty of being interruptible; a customer with food in flight wants the terminal states);
`shift.*` stay operational; nothing marketing-shaped exists or is licensed.

### 4.2 Kill the drift class, mechanically

The `backup.failed` incident (enqueued name ≠ registry name → adapter destructures undefined →
Telegram 400, hub review §5.3, VERIFIED) generalizes: today event names live in the registry, the
bus channel list, `CUSTOMER_PUSH_EVENTS`, the worker switch, and the (future) particle `vocab.ts`.
N4 makes `assertVocabulary()` the single boot-time parity gate: every enqueue-site event name ∈
`EVENT_REGISTRY`; every registry entry with `visual ≠ null` ∈ `PARTICLE_VOCAB`; every `pushClass
≠ ambient-only` entry has push-strings in all locales (`push-strings.ts` + `bot-strings.ts`
coverage check — the i18n al/en rule). One vocabulary, one gate, drift = boot failure.
**VbM RED:** add a registry entry with a visual name not in the vocab → boot throws (fixture test);
re-introduce the literal `'backup.failed'` enqueue → boot throws.

### 4.3 Quiet hours, per audience (policy, reusing the built engine)

The timezone-aware engine with held-once-then-deliver semantics is built and correct
(`quiet-hours.ts`, `workers/index.ts:241-263` — VERIFIED). Policy per audience:

- **Owner:** unchanged (transactional punches through; operational/quality held — zero silent
  drops).
- **Courier:** quiet hours are **shift-gated, not clock-gated** — an on-shift courier receives
  every `task.*` regardless of hour (being on shift *is* consent to be interrupted); an off-shift
  courier receives nothing except `shift.close_reminder` under the operational policy. No
  courier-configurable quiet window in v1 (the shift is the window).
- **Customer:** live-order status events are transactional relative to *their own in-flight
  order* — a 23:40 "delivered" push on a 23:10 order is wanted; there is no other customer
  notification class, so no customer quiet-hours engine is needed. (If any non-order class ever
  appears, it enters through the registry + category law first.)

### 4.4 No-strobe coalescing (dinner rush ≠ strobe, on every surface)

One coalescing doctrine, two enforcement points, borrowed verbatim from the particle analysis §4.3:

- **In-app:** token bucket — max 1 transient burst / ~1.5 s; N same-kind events collapse to one
  burst with a ×N glyph; sustained states derive from store state, not replayed events (40 missed
  frames reconcile to one final state).
- **Out-of-app:** Web Push `tag` = `order-<id>` (already the shape — a newer status *replaces* the
  prior notification, VERIFIED `adapters/webpush.ts:75`) + `Topic` header for collapse at the push
  service; Telegram: status escalations on the same order **edit** the prior message
  (`editMessageText`) instead of stacking, with a new message only on renderGroup change (buttons
  appear/disappear); per-chat rate limit + circuit breaker already exist (`workers/index.ts:59-67`,
  VERIFIED) and now cover courier chats too; hard rule: **≤1 push per order-status transition per
  audience** — retries re-send the same `tag`/message-edit, never a duplicate alert.
- **VbM RED (N4):** integration — fire 10 `order.created` in 3 s → in-app store emits ≤2 bursts
  with ×N; 5 status transitions on one order → exactly 5 tag-replacing pushes and ≤2 Telegram
  messages (1 + edits); fixture: disable the coalescer → counts explode → spec red.

### 4.5 Phases (vocabulary + local-first alignment)

**N4 — Unified vocabulary + coalescing + quiet policy (effort M, ~1–1.5 sessions).**
*Entry:* N1+N2 emitting courier events; particle P1 store exists (or the vocab lands in
`packages/particle-cloud/vocab.ts` first and the registry references it — either order works, the
gate makes them converge). *Modules:* `event-registry.ts` field extension + entries;
`assertVocabulary()` hardening; `CUSTOMER_STATUS_EVENTS`→registry migration; Telegram edit-in-place
in the adapter; coalescing bucket in the shared event store. *VbM:* §4.2 + §4.4 RED cases.
*Dependencies:* N0 (assert exists), N1/N2 (courier events real), particle P1 (visual names).

**N5 — Local-first alignment (design-forward, effort S now, real work rides D-transition Phase 4).**
*Entry:* D-transition Phase 4 preconditions (venue-device single-writer). *What changes:* push
becomes a pure **content-free wake ping** (03 §3.5 doctrine): payload = `{wake: true}` only; the
woken app pulls the signed offer from the vendor node/relay and verifies (C-lens §4.1); the ack
loop survives unchanged (ack = "device woke and fetched"); the vendor's always-on node — not Fly —
becomes the sender (VAPID keys move into the vendor vault; UnifiedPush/self-hosted ntfy becomes
the sovereign-stack option for Android couriers). The push gateway remains the irreducible floor
the SYNTHESIS names — this layer's design keeps the payloads already gateway-blind so the
migration is a sender relocation, not a redesign. *VbM:* the Phase-4 kill-the-relay drill extends
with a kill-the-gateway drill — gateway unreachable → RED within one sweep cycle + owner-node
local alarm (the venue's own device beeps — the last-resort rail that needs no gateway at all).
*Dependencies:* D-transition Phase 4 gate; G09 crypto tiers for signed offers.

### 4.6 Phase ladder summary

| Phase | What | Entry | Effort | Depends on | Headline VbM RED |
|---|---|---|---|---|---|
| **N0** | SW push handler in the SERVED worker; retire fake-ack adapter; event-name assert; `c.name` fix | none | S (~0.5 s) | — | today's served `sw.js` fails the display spec |
| **N1** | Courier Telegram rail (`courier_notification_targets`, `/start c_<token>`, `task.*` events) | N0 | S–M (~1 s) | N0, live bot | bad bot token → audit `failed`, spec red |
| **N2** | Courier Web Push + ack loop + latency⇄TTL assert + iOS Home-Screen onboarding | N0 | M (~1.5–2 s) | N0, VAPID | no ack ≤240 s on-shift → `push_silent_drop` + owner alert; P95 ≤30 s display |
| **N2.5** | UnifiedPush/de-Googled runbook + endpoint-agnostic grep-gate | N2 | S (~0.5 s) | N2 | gateway-hostname literal in src → grep-gate red |
| **N3** | Reply-route seam + channel-truth labels + onion push-gating | N0, G03 | M (~1 s) | N0, G03 | onion-session subscribe → 403; orphan route transport → boot error |
| **N4** | Unified registry (audience/pushClass/visual) + coalescing + quiet policy | N1, N2, particle P1 | M (~1–1.5 s) | N0–N2 | vocab drift → boot throw; coalescer off → burst-count spec red |
| **N5** | Wake-ping payloads + vendor-node sender (local-first) | D-transition P4 | S design now | P4 gate, G09 | kill-the-gateway drill → RED + local alarm |

**Not built, with re-entry triggers:** native FCM/APNs SDKs (re-entry: courier native shell,
D-transition P4) · SMS rail (re-entry: operator + cost council) · any messenger *intake* transport
(re-entry: G7 survey — this layer ships only the reply-route socket) · per-venue bot identities
(re-entry: multi-venue vendor demand) · customer marketing/re-engagement pushes (no category
exists; would require its own ethics pass — default answer is no).

---

## Sources

**Repo (read-only, this session — key anchors):**
`docs/research/2026-07-11-hub-architecture-review.md` §4.6, §5.3, §6.4, §7.1;
`docs/design/local-first-hub-2026-07-11/C-runtime-transport-identity.md` §1.2–1.4, §4.1;
`03-anonymity-architecture.md` §3.5; `04-anonymity-mesh-messenger-revision.md` §2, §4.2–4.3, §5;
`SYNTHESIS.md` §1; `D-transition-blueprint.md` phase ladder;
`docs/research/2026-07-11-particle-cloud-interaction-analysis.md` §4;
`apps/api/src/notifications/{event-registry.ts, provider.ts, quiet-hours.ts, adapters/webpush.ts,
adapters/push.ts, workers/index.ts}`; `apps/api/src/routes/customer/push.ts`;
`apps/api/src/lib/registry.ts:59`; `apps/api/src/workers/courier-offer-sweep.ts:120-141`;
`apps/api/public/sw.js` (served — no push handler) vs `apps/api/src/public/sw.js` (dead — has
handlers) vs `apps/api/src/client/pwa/sw.ts` (build source); `apps/api/src/server.ts:158-159`.

**Web (fetched/confirmed 2026-07-11 this session):**
[F-Droid — 5 years of UnifiedPush (2026-01-08)](https://f-droid.org/2026/01/08/unifiedpush-5-years.html) —
distributors (ntfy default, Sunup/Mozilla-autopush, NextPush, Conversations), spec converged on
Web Push (RFC 8291/8292), Firefox forks gained UP web push;
[unifiedpush.org spec definitions](https://unifiedpush.org/developers/spec/definitions/) — endpoint
= RFC 8030 push resource; [unifiedpush.org Sunup](https://unifiedpush.org/users/distributors/sunup/);
[WebKit — Safari 18.4 features (Declarative Web Push)](https://webkit.org/blog/16574/webkit-features-in-safari-18-4/);
[WWDC25 §235 — Declarative Web Push](https://developer.apple.com/videos/play/wwdc2025/235/);
[aimtell — State of Declarative Web Push 2026](https://aimtell.com/blog/state-of-declarative-web-push-2026) (VERIFIED-secondary);
[MagicBell — PWA iOS limitations 2026](https://www.magicbell.com/blog/pwa-ios-limitations-safari-support-complete-guide) (VERIFIED-secondary — Home-Screen-only push, no silent push/background wake on iOS PWA);
[Firebase — Android message priority](https://firebase.google.com/docs/cloud-messaging/android-message-priority) +
[Firebase blog Apr-2025 — reach users on Android](https://firebase.blog/posts/2025/04/fcm-on-android/) —
high priority punches Doze, normal is batched;
[techconcepts — iOS push/APNs](https://techconcepts.org/blog/ios-push-notifications) +
[Bugfender — advanced APNs](https://bugfender.com/blog/advanced-ios-push-notifications/)
(VERIFIED-secondary — priority 5/10, silent-push ~2-3/hr throttle, token-based JWT auth).
Inherited VERIFIED-in-repo (not re-fetched): webkit.org 16535 (iOS 16.4/18.4 web push),
developer.apple.com backgroundtasks, developer.android.com Doze/FGS, dontkillmyapp, EFF Apr-2026 +
Wyden Dec-2023 push-metadata record (via C-lens/doc 03/doc 04 §4.3).
**UNVERIFIED left standing:** real-world P95 web-push latency on Durrës-class Android/OEMs (the
N2 exit measurement exists precisely because no trustworthy published number does); Declarative
Web Push reliability delta vs SW-push (mixed developer reports — measure, don't assume);
`specialUse` FGS Play-review criteria (inherited UNVERIFIED from C-lens, irrelevant until a native
shell exists).

*Prepared 2026-07-11. Read-only session; the only file created is this blueprint. No code was
touched; implementation belongs to the parallel code session(s) under the repo's VbM + Ship
Discipline rules.*
