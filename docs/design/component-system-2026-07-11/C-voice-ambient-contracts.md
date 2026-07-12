# C — Voice + Ambient Contracts: the systematic layer

**Date:** 2026-07-11 · **Status:** DESIGN CONTRACT (no code in this change) · **Owner:** design-system
**Scope:** every dowiz component. This document defines the *contract* by which a component becomes
(1) a citizen of the reactive ambient environment and (2) voice-addressable — as design-system
primitives, not per-screen one-offs.

> **Provenance note.** Several cited inputs (PARTICLE-CLOUD PLAN/REVIEW, INTERFACE-DIRECTION
> "Tide over Bedrock", DESIGN-COMPLETION-BLUEPRINT) are flagged MISSING-on-disk by
> `docs/design/ROADMAP-GROUND-TRUTH-2026-07-11.md` §0.1. This document re-authors that layer to
> disk, grounded ONLY in verifiable repo facts (below) plus the operator brief. Where a value had
> to be (re)invented (e.g. `--ease-tide`), it is marked **PROPOSED**.

## 0. Ground truth this contract binds to (verified in-repo)

| Fact | Source |
|---|---|
| Order statuses: PENDING, CONFIRMED, PREPARING, READY, IN_DELIVERY, DELIVERED, REJECTED, CANCELLED, SCHEDULED, PICKED_UP | `packages/domain/src/order-machine.ts:3-14` (kernel twin: `kernel/src/order_machine.rs`) |
| Canonical hub event names (dot-form): `order.created`, `order.confirmed`, `order.rejected`, `order.delivered`, `order.pending_aging`, `order.dwell_escalation`, `order.timeout_cancelled`, `order.ready_for_pickup`, `courier.assigned`, `cash.reconcile_discrepancy`, `shift.started/closed/close_reminder`, `ops.degradation_changed`, … | `apps/api/src/notifications/event-registry.ts` |
| Particle showcase API: `init / burst(status,count) / setReducedMotion / setPalette / setPointer / dispose`; vocab `order_created`(amber) `courier_assigned`(teal) `delivered`(gold) `dispatch_failed`(blood) `pending_aging`(ember); reduced-motion → 25 % bursts + pointer force off; rAF auto-suspends when idle; 4 165 B gz (≤7 kB gate) | `webgl/particle-cloud/particle-cloud.js`, `webgl/particle-cloud/README.md` |
| Motion tokens: `--motion-instant/fast/base/slow`, `--ease-out/--ease-in-out/--ease-soft`; reduced-motion collapses durations to 0 ms; tap tokens `--tap-min:44px`, `--tap-courier:48px`, `--tap-critical:56px` | `packages/ui/src/theme/tokens.css:19-27,339-341` |
| Locales: `'sq' | 'en' | 'uk'`, default `sq` | `packages/ui/src/lib/i18n.ts:3-9` |
| Live transport: WS hub with rooms + auth + backoff; connection vocabulary `connecting/connected/disconnected/reconnecting/error/disabled` | `apps/web/src/lib/useWebSocket.ts` |
| Vendor storefront: `/s/:slug` SSR themed per-venue via `location_themes` + deterministic `ThemeRenderer` (cssHash, WCAG ≥4.5 warning, latin-ext font whitelist for sq ë/ç) | `apps/api/src/routes/spa-proxy.ts:498-560`, `docs/branding/theme-system.md` |
| Quiet hours engine (`Europe/Tirane` default) | `apps/api/src/notifications/quiet-hours.ts` |
| Red-lines (operator law): NO courier scoring; non-AI runtime; integer money / money-never-tweens (3 live tween violations: `ClientLayout.tsx:154`, `AnalyticsPage.tsx:262`, `courier/EarningsPage.tsx:47`); anonymity/local-first; COD; mutating voice ⇒ visual-confirm + tap | `docs/design/ROADMAP-GROUND-TRUTH-2026-07-11.md` §0.3, `docs/design/DRIFT-ANALYSIS-2026-07-11.md:186` |
| Canonical stack: Astro + Svelte 5 islands + WebGL2 (canvas2D fallback); React/TS app = legacy oracle | `ROADMAP-GROUND-TRUTH` §0.5 |

Contract primitives below are named framework-neutrally, with a **Svelte 5** (canonical) and
**React** (legacy-oracle) mapping each. Hook spellings (`useAmbientState` etc.) are the React
names; Svelte uses runes-store equivalents (`ambient.svelte.ts`) and element actions.

---

## 1. The state→ambient contract

Every component that has live state declares — statically, at definition time — how that state
projects onto the shared ambient layer. No component ever colors, flashes, or animates ad hoc.
The declaration is the `AmbientDecl`.

### 1.1 `AmbientDecl` (per component, declarative)

| Field | Meaning |
|---|---|
| `kind` | Domain noun: `order-card`, `task-row`, `cart`, `shift-bar`, `connection`, `menu-item`, … |
| `emits` | Hub events (registry dot-form) this component's user actions can produce, e.g. an owner order card emits `order.confirmed`, `order.rejected`. |
| `consumes` | Hub events whose arrival changes this component's rendered state, scoped by `{orderId?, locationId?, shiftId?}`. |
| `posture(state)` | A pure map from the component's own status to one of the five **hue postures** (§1.2). Central map, never inline colors. |
| `signatures` | Which of the three signatures it may participate in: `{ particle?: boolean, spectralEdge?: boolean, horizonWash?: boolean }` — constrained by §4 (most components: spectral edge only). |
| `crest` | Which of its transitions are *crest-worthy* (emit exactly one ambient crest via the coalescer) vs silent. Default: only transitions that also exist in the notification `EVENT_REGISTRY` are crest-worthy — the registry is the single gate for "does the environment react". |

### 1.2 The five hue postures (the only ambient colors that exist)

Postures reuse the shipped particle vocabulary palette so CSS layers and the WebGL layer are
chromatically one system (`particle-cloud.js:31-37`):

| Posture | Meaning | Anchor rgb (0–1) | Order-machine / registry mapping |
|---|---|---|---|
| `life` | Something new arrived | amber `1.00,0.64,0.13` | `order.created` / PENDING |
| `alive` | Flow in motion | teal `0.18,0.85,0.78` | CONFIRMED, PREPARING, READY, IN_DELIVERY; `courier.assigned`, `shift.started` |
| `settle` | Terminal success | gold `1.00,0.82,0.27` | DELIVERED, PICKED_UP; `order.delivered`, `shift.closed` |
| `warn` | Aging / needs a human | ember `0.95,0.45,0.20` | `order.pending_aging`, `order.dwell_escalation`, `shift.close_reminder`, WS `reconnecting` |
| `anomaly` | Failure / discrepancy | blood `0.80,0.10,0.16` | REJECTED, CANCELLED, `order.timeout_cancelled`, `cash.reconcile_discrepancy`, `ops.degradation_changed`(worse), WS `error/disconnected` |

Rules: postures are semantic, not decorative — a component may not use `anomaly` red for emphasis.
`anomaly` + `warn` **persist** (state holds until resolved); `life/alive/settle` are **transient**
(crest then decay to neutral). Money digits are never part of a posture animation
(**money-never-tweens** — trust cue; the three live tween sites above are contract violations
scheduled for removal in Tier-0 Batch A).

### 1.3 One-crest-per-transition + coalescing (calm-under-rush)

The rule that makes a busy screen a rising tide instead of a strobe:

1. **One crest per transition.** A single state transition produces at most ONE ambient crest
   across the whole surface — not one per widget that happens to display the order. The crest is
   emitted by the *store* (§3.1), not by components; components only declare crest-worthiness.
   (This is Material's choreography rule — one focal element per transition, "avoid scenes without
   focus" — applied to ambient state: [Material Design, Choreography](https://m2.material.io/design/motion/choreography.html).)
2. **Coalescing window.** The coalescer buffers crest candidates in a **900 ms sliding window**.
   Per window it emits at most one crest: the highest posture present wins
   (`anomaly > warn > settle > alive > life`); same-posture events merge into one crest with
   amplitude `a = min(1, 0.4 + 0.15·log2(N+1))` and the merged count surfaces as a chip badge
   ("+7"), never as extra motion.
3. **Rush mode.** Sustained load (≥5 crest candidates/window for 3 consecutive windows) switches
   the surface from discrete crests to a **raised tide baseline**: individual bursts are fully
   suppressed; the horizon wash's `tideLevel` (EWMA of event rate, 0–1) rises instead, and the
   event count accumulates in the visible chips. When the rate drops, the tide settles on
   `--ease-tide` over ~4×`--motion-slow`. Ambient load is therefore *sub-linear* in event rate.
4. **Flash budget (hard).** Min inter-crest interval 350 ms AND ≤3 luminance-changing ambient
   events per second **summed across all three signatures** — enforced in the coalescer, the one
   choke-point, so WCAG 2.3.1 conformance is systemic rather than per-screen (§3.4).

### 1.4 Component-level API shape (descriptions, not code)

- **`useAmbientState(kind, status)`** — the one hook a stateful component calls. Derives the
  posture from the central map, subscribes the component to its `consumes` scope in the shared
  store, and returns `{ posture, hueTokens, chip, requestCrest(transition) }`. `requestCrest` is
  the only way a component can ask for ambient motion, and it routes through the coalescer —
  a component physically cannot strobe. Svelte: `ambientState(kind, () => status)` returning
  `$derived` values.
- **`<SpectralEdge attending={false | posture}>`** — the cheap, default signature: a wrapper
  primitive painting an animated gradient edge (compositor-only: transform/opacity on a
  pseudo-element) around any card/control when it is the surface's current point of attention.
  At most **one** `attending` SpectralEdge per screen region; contention is arbitrated by the
  store's attention slot, not by whichever component rendered last. Reduced-motion → static 2 px
  posture-colored edge, no animation. Svelte: `use:spectralEdge` action.
- **`<HorizonWash>`** — one per page layout, never nested. Consumes `tideLevel` + dominant
  posture; renders a fixed background gradient whose stops/opacity move on `--ease-tide`.
  Also carries the *connection* posture: WS `reconnecting/error` dims the horizon (the surface
  visibly "goes overcast" — degradation is shown, not hidden; calm tech's "work even when it
  fails": [Case, Principles of Calm Technology](https://www.caseorganic.com/post/principles-of-calm-technology)).
- **`<ParticleField>`** — the expensive signature; at most one per surface, allowed surfaces
  listed in §4. Subscribes to coalesced crests only and calls
  `cloud.burst(project(eventType), countFromAmplitude)` using the registry→vocab projection
  (`order.created→order_created`, `courier.assigned→courier_assigned`,
  `order.delivered→delivered`, dispatch failure→`dispatch_failed`,
  `order.pending_aging→pending_aging`).
- **`useEventStream(selector)`** — read-side of the shared store for anything non-visual
  (lists, counters, sounds). Same store the ambient layer reads: one truth, many projections.
  React: `useSyncExternalStore`-backed; Svelte: `$derived` over the runes store.
- **`emitHubEvent(type, scope)`** — write-side used by mutations (and by the SW `courier_dispatch`
  postMessage path from `public/sw.js`), tagging `source: 'ws' | 'local-optimistic' | 'sw'`.
  Local-optimistic events render state instantly but are crest-silent until the hub confirms —
  the environment celebrates *facts*, not hopes.

**PROPOSED token addition** to `packages/ui/src/theme/tokens.css` (does not exist today):
`--ease-tide: cubic-bezier(0.37, 0, 0.21, 1)` — long swell, late settle; paired with durations
≥ `--motion-slow` and used *only* by ambient layers (UI chrome keeps `--ease-out/--ease-soft`).
Under reduced-motion it is irrelevant because ambient durations collapse (§3.3).

---

## 2. The voice-addressability contract

**Model (fixed, red-line):** pure WebAudio DSP — mic frames → MFCC feature vectors → DTW distance
against *user-enrolled* per-word templates, plus energy-transient (clap) and sustained-tonal-peak
(whistle) detectors. Small enrolled vocabulary, speaker-dependent, fully on-device. No AI runtime,
no cloud ASR, no audio ever leaves the device (anonymity + local-first red-lines). MFCC+DTW is the
classic non-neural small-vocabulary keyword-spotting method
([Aalto, Wake-word and keyword spotting](https://speechprocessingbook.aalto.fi/Recognition/Wake-word_and_keyword_spotting.html);
[IEEE, KWS via MFCC+DTW](https://ieeexplore.ieee.org/document/7322545/)) — cheap enough for a
browser, and speaker-dependence is a *feature* here (only the enrolled owner/courier can drive it).

### 2.1 `voiceIntent` — what a component registers

A component becomes voice-addressable by registering one or more `voiceIntent` declarations
(React `useVoiceIntent(decl)`; Svelte `use:voiceIntent={decl}`; auto-unregister on unmount):

| Field | Meaning |
|---|---|
| `id` | Stable intent id, e.g. `order-card.accept`. |
| `audience` | `'owner' | 'courier' | 'customer'` — gates which word registry it draws from. |
| `kind` | `'read'` (speak/expand info) · `'focus'` (move attention) · `'navigate'` · `'mutate'` (state transition) · `'money'` (anything touching amounts, cash, refunds, COD settlement). |
| `verbKey` | Key into the voice word registry (an i18n-like table, sq/en/uk suggested defaults §2.4); the *user's enrolled recording* of that word is the actual template — suggested words are prompts, not requirements. |
| `actionRef` | **Mandatory** reference to the same handler/element as a *rendered touch control*. An intent cannot be registered without a resolvable visible control — this is the structural guarantee that voice is never the only path (multimodal redundancy: [Aufait UX, VUI best practices](https://www.aufaitux.com/blog/voice-user-interface-design-best-practices/)). Lintable/CI-able invariant. |
| `confirm` | Forced `true` by the registrar for `kind: 'mutate' | 'money'` — not opt-in, not overridable (red-line, §2.3). |
| `fallbackGesture` | Optional `'clap' | 'clap-double' | 'whistle'` — allowed only for `read`/`focus` kinds (§2.5). |
| `scope` | `'focused' | 'screen' | 'global'` — arbitration tier (§2.2). |

### 2.2 Routing: recognized word → action

1. Recognizer runs only while the **push-to-talk** surface is held (default; §2.6) and produces
   `(templateId, dtwScore, margin)`.
2. Accept iff `dtwScore ≤ θ` (per-word threshold set at enrollment from template self-distance +
   the calibrated noise floor) AND margin over the second-best template ≥ δ. Otherwise: a
   listening-ripple on the SpectralEdge + chip "Not recognized — tap instead" (never a guess;
   a mis-fired mutation is worse than a missed word).
3. Arbitration walks `focused` scope (the card currently holding the attention slot) → `screen` →
   `global`. First registered intent whose `verbKey` matches wins; ties within a tier are a
   registration-time error surfaced in dev.
4. `read`/`focus`/`navigate` execute immediately through `actionRef`. `mutate`/`money` open the
   **confirm card** (§2.3). Every acceptance echoes visibly: chip + SpectralEdge pulse on the
   target (visual reinforcement of voice: [Lollypop, VUI best practices](https://lollypop.design/blog/2025/august/voice-user-interface-design-best-practices/)).

### 2.3 The red-line: visual-confirm + tap for anything mutating

For `kind: 'mutate' | 'money'`, a recognized word never commits. It opens a **confirm card**:

- shows the parsed action + target ("Accept order #A12 — 1 450 L, COD") with amounts as static
  integer text (money-never-tweens);
- one primary tap target at `--tap-critical` (56 px) + a Cancel;
- `role="alertdialog"`, focus moves to it;
- auto-dismiss = **cancel** after 8 s; timeout never commits;
- **voice cannot confirm the confirmation** — no enrolled "yes". The commit is a physical tap,
  period. (Explicit confirmation before consequential actions is standard VUI practice
  ([Parallel, VUI design principles](https://www.parallelhq.com/blog/voice-user-interface-vui-design-principles));
  dowiz hardens it to cross-modal: voice proposes, touch disposes.)

This applies to: order accept/reject/ready/delivered transitions, cart checkout, any COD/cash
figure, shift close, refunds, bulk edits. `read`-class intents (repeat order details, read
address) commit freely — they mutate nothing.

### 2.4 Audience word sets (suggested defaults; user enrolls their own)

Words are chosen short and phonetically distinct *within an audience+scope set* — DTW needs
inter-template distance, so **enrollment rejects a new word whose template is within δ of an
existing one** and prompts for an alternative. Suggested (sq default locale / en / uk):

| Intent | sq | en | uk | kind |
|---|---|---|---|---|
| **Owner (kitchen, flour on hands)** | | | | |
| accept order | *prano* | *accept* | *прийняти* | money → confirm+tap |
| reject order | *refuzo* | *reject* | *відхилити* | mutate → confirm+tap |
| mark ready | *gati* | *ready* | *готово* | mutate → confirm+tap |
| next / cycle focus | *tjetra* | *next* | *далі* | focus |
| read order aloud/expand | *lexo* | *read* | *читай* | read |
| **Courier (hands-busy, on the move)** | | | | |
| picked up | *mora* | *picked* | *забрав* | mutate → confirm+tap |
| delivered | *dorëzova* | *delivered* | *доставив* | money (COD) → confirm+tap |
| map / navigate | *harta* | *map* | *карта* | navigate |
| next task | *tjetra* | *next* | *далі* | focus |
| **Customer (minimal, storefront/track)** | | | | |
| order status | *statusi* | *status* | *статус* | read |
| repeat last info | *përsërit* | *repeat* | *повтори* | read |

Customer voice is **read-only** — no mutating or money intents exist for the customer audience
(anonymity + fraud posture: an unenrolled stranger's phone must not be able to mutate an order).

### 2.5 Clap / whistle — the noise fallback

Kitchens defeat word matching. Detectors that survive noise:

- **Clap** (broadband energy transient, ×2 within 700 ms to reject bangs): advances the
  attention slot — same as *tjetra/next*. Focus only.
- **Double-clap**: re-opens the last confirm card if it timed out (still requires the tap).
- **Whistle** (sustained tonal peak ≥300 ms): "summon" — re-read the currently attended card /
  surface the newest `warn|anomaly` item.

Claps and whistles can therefore *never* commit anything; they move attention and re-present.
They are calibrated at enrollment against the room's recorded noise floor.

### 2.6 Push-to-talk, open-mic, and no-mic degradation

- **Default: push-to-talk.** A large hold-to-listen zone (≥ `--tap-courier`) pinned per surface;
  mic streams only while held; a hard visual mic-state chip (idle / listening / matched /
  rejected) is always rendered while the feature is enabled (trust cue).
- **Owner kitchen open-mic (opt-in only):** continuous listening gated by an enrolled attention
  word (same MFCC/DTW machinery as any template), active only inside business hours via the
  existing quiet-hours engine (`quiet-hours.ts`, `Europe/Tirane`), with a persistent on-screen
  "mic live" indicator. Never default-on; per-device toggle.
- **Enrollment UX:** a settings flow per role: pick intent → record the word 3× (VAD-trimmed) →
  templates + thresholds stored in IndexedDB, per device, never synced or uploaded → distinctness
  check against existing set → 5-second live test. Re-enroll anytime; "delete all voice data" is
  one tap (local wipe — nothing exists server-side to delete).
- **No-mic / denied-permission degradation:** capability-check at startup; if absent, voice UI
  renders nothing and *everything still works*, because `actionRef` guarantees each intent is a
  projection of an existing touch control. Voice is an accelerator layer, structurally never a
  gate. Same guarantee covers screen-reader users and browsers without WebAudio.

---

## 3. Reactive-environment orchestration (the whole surface, coherently)

### 3.1 One store per surface

A single **event-stream store** (Svelte 5 runes module `ambient.svelte.ts` / React external
store) is the only path between "something happened" and "the surface reacted":

```
WS hub (useWebSocket rooms) ─┐
local optimistic mutations ──┼─► append(event) ─► ring buffer (256) ─► coalescer ─►
SW postMessage (sw.js) ──────┘                                            │
        ┌─────────────┬───────────────┬────────────────┬─────────────────┤
        ▼             ▼               ▼                ▼                 ▼
  ParticleField  SpectralEdge   HorizonWash      state chips        notification
  (crests only)  (attention)    (tideLevel)      (role="status")    system (registry)
```

Event shape: `{ id, ts, type (EVENT_REGISTRY key), scope {orderId?, locationId?, shiftId?},
posture, source }`. Derived state: `tideLevel` (EWMA event rate 0–1), `attentionSlot` (one per
region), `dominantPosture`, `connectionPosture` (from the WS status vocabulary). Components
never subscribe to raw WebSocket messages for ambient purposes — only to this store — which is
what makes surface-wide coherence possible: the coalescer sees *everything*, so the tide is one
tide.

### 3.2 "Ambient is never the only signal" (hard accessibility rule)

Every ambient effect must be redundant with two non-ambient channels, checked at the contract
level (crest-worthiness is *defined* by registry membership, §1.1):

1. the **notification system** entry for the same event (`event-registry.ts` — Telegram/push/email
   adapters), and
2. a **visible state chip** — text + icon, not color-alone (WCAG 1.4.1) — rendered with
   `role="status"` (implicit `aria-live="polite"`) so screen readers announce it without focus
   stealing per WCAG 4.1.3
   ([W3C, ARIA22](https://www.w3.org/WAI/WCAG21/Techniques/aria/ARIA22);
   [Soueidan, Accessible notifications with ARIA live regions](https://www.sarasoueidan.com/blog/accessible-notifications-with-aria-live-regions-part-1/)).
   `anomaly`-posture chips may use `role="alert"`; everything else stays polite.

The ambient layer is the *periphery* in the calm-technology sense — it informs without demanding
attention, and information moves from periphery to focus (chip → notification) as it becomes
important ([Weiser & Brown via Wikipedia, Calm technology](https://en.wikipedia.org/wiki/Calm_technology);
[Case, Principles of Calm Technology](https://www.caseorganic.com/post/principles-of-calm-technology)).
Turning ambient fully off (per-user "Calm mode" toggle, persisted) must lose zero information.

### 3.3 Reduced-motion collapse (uniform, token-driven)

`prefers-reduced-motion: reduce` OR the in-app Calm toggle collapses the whole ambient tier in
one move (satisfying WCAG 2.2.2's stop/hide mechanism and the reduced-motion technique —
[W3C, Understanding 2.2.2](https://www.w3.org/WAI/WCAG21/Understanding/pause-stop-hide.html);
[BoIA, prefers-reduced-motion](https://www.boia.org/blog/what-to-know-about-the-css-prefers-reduced-motion-feature)):

- durations already collapse to 0 ms via tokens (`tokens.css:339-341`);
- **ParticleField unmounts entirely** (the showcase's 25 %-burst throttle is NOT sufficient under
  this contract — positional motion goes to zero, not to a quarter);
- SpectralEdge → static posture-colored edge; HorizonWash → stepped opacity/gradient crossfade
  (color/opacity only, no positional animation);
- state chips, notifications, voice, and touch are untouched — nothing informational is lost.

### 3.4 WCAG 2.3.1 + 2.2.2 guardrails (enforced at the choke-point)

- **≤3 flashes/sec, globally:** the coalescer's flash budget (§1.3.4) counts luminance-changing
  ambient events across particle + edge + wash combined; 900 ms windows + 350 ms min spacing keep
  the worst case at ~1.1 crests/sec
  ([W3C, Understanding 2.3.1](https://w3c.github.io/wcag21/understanding/three-flashes-or-below-threshold.html)).
- **Red-flash threshold:** `anomaly` (blood) crests specifically are capped — the particle
  projection limits a `dispatch_failed` burst's screen coverage so saturated-red luminance change
  stays under 25 % of any 10° visual field; the persistent anomaly signal is the *held* red edge +
  chip (steady state, not flashing).
- **2.2.2 pause/stop/hide:** ambient motion is auto-starting and can exceed 5 s under rush → the
  Calm toggle is a mandatory, persistent, one-tap mechanism on every dowiz-chrome surface (the
  particle engine's idle auto-suspend alone doesn't satisfy the user-control requirement).
- **4.1.3 / 1.4.1:** covered structurally by §3.2.

### 3.5 Trust cues under orchestration

Money renders instantly and silently — the environment may crest *around* a settled payment
(gold `settle` bloom), but digits themselves never tween, count up, or shimmer. Anomalies never
auto-clear visually before the underlying state clears. The mic state is always visible when
voice is armed. These are the same trust posture as COD + anonymity: the surface never performs
certainty it doesn't have.

---

## 4. Per-signature usage rules (cost-aware) + storefront sovereignty

### 4.1 The three signatures: when each is allowed

| Signature | Cost | Granularity | Allowed use |
|---|---|---|---|
| **Spectral edge** | ~zero (CSS, compositor-only, no JS when idle) | per-component | **Default.** Any component's posture/attention. The only signature a leaf component may self-declare. |
| **Horizon wash** | one fixed gradient layer, opacity/stop transitions on `--ease-tide` | per-page (exactly one, in the layout) | Page-level tone: `tideLevel`, dominant posture, connection posture. Never nested, never per-widget. |
| **Particle cloud** | WebGL2 singleton canvas, ≤7 kB gz chunk gate (4 165 B today), ≤4 096 particles, DPR≤2, rAF auto-suspend | per-surface (exactly one canvas) | **Live-ops surfaces only:** owner live dashboard/kitchen board, courier active-task screen, customer live order-tracking. Never on forms, settings, menus, analytics, marketing. |

**Decision rule:** component state → spectral edge; page mood/load → horizon wash; discrete
domain events on a surface whose *job* is watching events → particle crests. If in doubt, the
answer is spectral edge.

**Fallback ladder:** WebGL2 unavailable → canvas2D particle fallback (per stack decision,
`ROADMAP-GROUND-TRUTH` §0.5) → spectral edge + wash only. Low battery / `saveData` /
low-power mode → drop the particle tier automatically; reduced-motion → §3.3. One rAF owner per
surface (the field); edge/wash never run their own loops.

### 4.2 Storefront sovereignty (the exclusion)

The three signatures, the dowiz palette postures, and the "Tide over Bedrock" feel are the
**dowiz chrome identity** — they apply to `/admin/*`, courier surfaces, the dowiz landing, and
dowiz-owned tracking chrome. The vendor storefront `/s/:slug` is the **vendor's** brand
(`location_themes` → deterministic `ThemeRenderer`, cssHash-cached, WCAG-checked, latin-ext
fonts) and dowiz must not paint its own skin over it.

Mechanism — a `surface` scope provided at each layout root: `dowiz-chrome | vendor`:

- Under `vendor`, the ambient provider **hard-disables all three signature renderers** —
  `AmbientDecl.signatures` flags are ignored, `<SpectralEdge>/<HorizonWash>/<ParticleField>`
  render nothing. Not a style override a page could opt back into; the provider returns null.
- What a storefront component **keeps**: the event-stream store (live status), state chips
  (restyled via the vendor's semantic tokens derived from `location_themes`, with the existing
  ≥4.5 contrast check), the notification pairing, reduced-motion behavior, and **voice** — the
  customer minimal read-only set (§2.4) with push-to-talk and the same enrollment/no-mic rules.
  State and voice are *capabilities*; the three signatures are *dowiz's skin*. Capabilities
  travel, skin doesn't.
- Posture semantics survive as data (`warn/anomaly` still drive chip text/icons) but are painted
  with vendor tokens, never the dowiz amber/teal/gold/ember/blood anchors.

### 4.3 Contract conformance — what a component PR must show

1. `AmbientDecl` present; postures come from the central map (no inline ambient colors —
   lintable).
2. Every crest-worthy transition exists in `event-registry.ts` and has a state chip with
   `role="status"` (§3.2).
3. No self-owned animation loops; all ambient motion via `requestCrest`/signature primitives.
4. Every `voiceIntent` has a resolvable `actionRef` touch control; `mutate/money` intents show
   the confirm-card flow in the Playwright spec (tap commits, timeout cancels, no voice-confirm).
5. Money values render as static integer text (no `AnimatedNumber`/`CountUpPrice` — lintable).
6. Reduced-motion snapshot: color/opacity only; particle tier absent.
7. If rendered under `surface="vendor"`: zero dowiz signature output (zero-diff storefront gate,
   Tier-2).

---

## Sources

- [Material Design — Choreography (single focal element per transition)](https://m2.material.io/design/motion/choreography.html)
- [Amber Case — Principles of Calm Technology](https://www.caseorganic.com/post/principles-of-calm-technology) · [Calm technology (Weiser & Brown) — Wikipedia](https://en.wikipedia.org/wiki/Calm_technology)
- [W3C — Understanding SC 2.3.1 Three Flashes or Below Threshold](https://w3c.github.io/wcag21/understanding/three-flashes-or-below-threshold.html)
- [W3C — Understanding SC 2.2.2 Pause, Stop, Hide](https://www.w3.org/WAI/WCAG21/Understanding/pause-stop-hide.html)
- [BoIA — prefers-reduced-motion and WCAG conformance](https://www.boia.org/blog/what-to-know-about-the-css-prefers-reduced-motion-feature)
- [W3C — ARIA22: role=status for status messages (SC 4.1.3)](https://www.w3.org/WAI/WCAG21/Techniques/aria/ARIA22) · [Sara Soueidan — Accessible notifications with ARIA live regions](https://www.sarasoueidan.com/blog/accessible-notifications-with-aria-live-regions-part-1/)
- [Aalto Univ. — Wake-word and keyword spotting (MFCC/DTW small-vocabulary KWS)](https://speechprocessingbook.aalto.fi/Recognition/Wake-word_and_keyword_spotting.html) · [IEEE — KWS using MFCC and DTW](https://ieeexplore.ieee.org/document/7322545/)
- [Parallel — VUI design principles (confirmation for critical actions)](https://www.parallelhq.com/blog/voice-user-interface-vui-design-principles) · [Aufait UX — VUI best practices (multimodal redundancy)](https://www.aufaitux.com/blog/voice-user-interface-design-best-practices/) · [Lollypop — VUI best practices (visual reinforcement)](https://lollypop.design/blog/2025/august/voice-user-interface-design-best-practices/)

*In-repo evidence: `webgl/particle-cloud/particle-cloud.js`, `apps/api/src/notifications/event-registry.ts`, `packages/domain/src/order-machine.ts`, `packages/ui/src/theme/tokens.css`, `packages/ui/src/lib/i18n.ts`, `apps/web/src/lib/useWebSocket.ts`, `apps/api/src/routes/spa-proxy.ts`, `docs/branding/theme-system.md`, `docs/design/ROADMAP-GROUND-TRUTH-2026-07-11.md`, `docs/design/DRIFT-ANALYSIS-2026-07-11.md`.*
