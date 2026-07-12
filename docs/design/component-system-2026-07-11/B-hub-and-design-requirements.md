# Component System — Requirements & Architecture Constraints (B) — Hub + New Design — 2026-07-12

> **What this is:** the binding requirements and architecture constraints a new dowiz component
> design system must satisfy, derived first from the HUB ARCHITECTURE and second from the
> Tide-over-Bedrock design direction. Read-only session; the only file created is this report.
>
> **Provenance (important):** most of the 2026-07-11 source reports cited below are **not on
> disk** — `docs/design/ROADMAP-GROUND-TRUTH-2026-07-11.md` §0.1 confirms ~20 reports were never
> written to file. For this report their full texts were **recovered verbatim from the authoring
> session's transcripts** (session `c6a4c73f`, `/root/.claude/projects/-root/…/subagents/*.jsonl`
> — Write/Edit payload replay; hub review reconstructed to 74 KB with zero placeholders). Cited
> as their intended repo paths, marked **(recovered)**. Corroborated on disk by:
> `docs/design/ROADMAP-GROUND-TRUTH-2026-07-11.md` (canonical stack §0.5, DONE ledger §1),
> living memory `/root/.claude/projects/-root/memory/local-first-and-no-courier-scoring-2026-07-11.md`
> (operator rulings 1–7), and the trees `kernel/src/{order_machine,money,domain,analytics}.rs`,
> `web/src/components/Storefront.svelte`, `webgl/particle-cloud`, `packages/ui/src/theme/tokens.css`.
>
> **Sources (all read in full this session):**
> `docs/research/2026-07-11-hub-architecture-review.md` (recovered) — "hub review";
> `docs/design/local-first-hub-2026-07-11/SYNTHESIS.md` (recovered) — "synthesis";
> `docs/design/local-first-hub-2026-07-11/layer-channel-adapters.md` (recovered) — "adapters";
> `docs/design/local-first-hub-2026-07-11/layer-notifications-push.md` (recovered) — "notif";
> `docs/design/dowiz-brand/INTERFACE-DIRECTION-2026-07-11.md` (recovered) — "direction";
> `docs/design/dowiz-brand/DESIGN-COMPLETION-BLUEPRINT-2026-07-11.md` (recovered) — "blueprint";
> `docs/research/2026-07-11-particle-cloud-interaction-analysis.md` (recovered) — "particle";
> living memory above — "memory"; `docs/design/ROADMAP-GROUND-TRUTH-2026-07-11.md` — "roadmap".

---

## 0. Standing decisions the component system inherits as law (memory 1–7; roadmap §0.5)

These are operator rulings, not design preferences. Every requirement below is written under them:

1. **Local-first is the ratified destination** (memory §1) — components bind to the wire contract,
   never a backend's internals, so they survive Node-today → Rust-kernel → device-resident hub.
2. **NO courier scoring — hard red line** (memory §2; roadmap §1 "NO-COURIER-SCORING final").
3. **COD is mandatory, first-class** (memory §3) — cash is the only settlement rail components assume.
4. **Anonymity is a design value, layered honestly** (memory §4) — data-layer guarantee always;
   per-channel network anonymity is labeled, not enforced.
5. **Multichannel, NO dedicated app** (memory §5) — every funnel is an adapter into one
   `kernel::decide` door; nothing the component system ships may require an install.
6. **Quality-first "stable enough to send" bar** (memory §6; blueprint §5.7) — the component
   system's definition of done is a checkable list, not a feeling.
7. **Canonical stack** (roadmap §0.5): Rust→WASM kernel; **Astro + Svelte 5 islands**; WebGL2
   particle layer; Node/TS/React is the **legacy oracle** — component specs must be authored
   stack-neutral (CSS custom properties + vanilla-TS cores + portable acceptance criteria) per
   blueprint §4, so the same system serves the React interim and the Astro/Svelte target.

---

## 1. HUB-DERIVED REQUIREMENTS (first priority)

### H-1 · One door: ordering components produce one canonical Command, never a channel payload

- **H-1.1** The entire ordering funnel terminates in **exactly one order-creating call**:
  `POST /api/orders` today, `Command::PlaceOrder → kernel::decide` as the target (hub review §3.1:
  the one non-test `INSERT INTO orders` per stack; §3.2 finding 1: the Rust checkout currently
  bypasses `decide` — a defect being fixed, not a pattern to serve). **No component may write an
  order by any other path.** The standing falsifiable proof (adapters §1.2 C-1): grep
  `INSERT INTO orders` = one non-test site per stack — the component system must never break it.
- **H-1.2** Checkout components emit the **canonical order contract**, not a channel-specific
  payload: items (ids, qty 1–99, modifier ids), customer contact (6-kind `MessengerKind` + handle
  ≤500, `receiver{}` — the G03 shape), delivery pin/type, `payment.method: cash`,
  `idempotency_key` (hub review §3.1 step 1). **Price fields are unrepresentable in what
  components send** — the door re-prices from the DB every time (hub review §1.1 idea 3; kernel's
  typed `PlaceOrder` carries no price slot, §3.2). Components display server-priced integer money;
  they never compute, estimate, or transmit it.
- **H-1.3** The doctrine test applies to every component that touches an order (hub review §1.1,
  EXPANSION-PLAN rule): *"does this door decide anything, or only carry?"* Components **carry**;
  they never price, never transition state, never invent money.
- **H-1.4** Cart-token redemption is a component requirement: the checkout must accept
  `/s/:slug/checkout?ct=<token>` prefill (Class-I intent adapters hand off here — adapters §1.2/
  §1.3 row 5). The component treats the token as untrusted input: render only after the server
  verifies sig/exp/slug, re-validates every line, re-prices, burns the nonce. No component ever
  mints, parses, or displays token internals.

### H-2 · Channel-abstract vs web-only: the split, stated precisely

The adapter taxonomy (adapters §1.2) has three classes; only one has UI, and that UI is reused by
two of the three:

| Adapter class | Examples | Component consequence |
|---|---|---|
| **Class L (link)** | web, QR, NFC, GBP/Apple, Instagram link, widget, subdomain, TMA wrap | The adapter *is a URL* into `/s/:slug?ch=…` — the **storefront funnel components ARE the adapter body** for every Class-L channel |
| **Class I (intent)** | Telegram bot, WhatsApp Cloud API, IG DM, SimpleX bot, MCP/agent | **No dowiz UI components at all** — dialogue in the platform's idiom → cart-token → the same web checkout (H-1.4) |
| **Class M (mirror)** | `.onion` mirror, future LAN/kiosk | The **same** storefront components served over another transport — zero new body |

Therefore:

- **H-2.1 Channel-abstract components** (must run under any transport, incl. Telegram WebView and
  Tor Browser): the storefront funnel — menu browse, product detail, cart, checkout, order
  confirmation. Constraints this imposes: **no third-party asset origins** (Tor/CSP/TMA all break
  them), **SSR-first with a no-JS-legible baseline** (Class M; the `.onion` tier is
  foreground-only — notif §2 T3), no reliance on push/notification permission, no assumption of
  clearnet latency (memory §4c: order placement is the latency-tolerant leg).
- **H-2.2 Web-only dowiz-chrome components**: owner console, courier app, customer tracking room,
  notifications UI, landing. These may assume a modern browser, WS, lazy chunks — inside budgets
  (C-6).
- **H-2.3 Presentation-free primitives** (a third, load-bearing class): the **status/string layer
  must be renderer-agnostic**, because order status is rendered in web chips, Telegram messages,
  and push strings alike (`renderStatus` in the channel idiom — adapters §1.3 row 6; notif §4.1
  "out-of-app push and in-app ambient are two renderings of one vocabulary"). Requirement: one
  pure module mapping `OrderStatus → {label(locale), hue-role, glyph}` with **zero DOM/framework
  imports**, consumed by web components, bot renderers, and push-string builders. Same for money
  formatting (one `formatMoney` path — direction §5.1) and ETA ranges.
- **H-2.4 Vocabulary SSOT rule (V-1)**: no component declares a private copy of any vocabulary —
  `OrderChannel` (13-value), `MessengerKind` (6-kind), the 10 statuses, event names. One
  source-of-truth module each + the mechanical cross-stack parity gate (adapters §1.1; the
  `backup.failed` name-drift breakage is the cautionary tale — hub review §5.3). A component
  system that re-declares an enum re-creates the G03 bug class.
- **H-2.5 Attribution is write-only in components too**: the storefront captures `?ch=` once at
  landing (sessionStorage per-slug) and stamps the header at checkout — and **no component ever
  reads the channel to alter pricing, flow, layout, or copy** (hub review §1.1 idea 2: never read
  by pricing/state machine/dispatch/notifications/authz). The only sanctioned readers are the
  owner analytics card and the channels page (adapters §3.4).

### H-3 · The 10-status machine is the status components' contract

- **H-3.1** Status components render the **byte-frozen 10 statuses** — `PENDING, CONFIRMED,
  PREPARING, READY, IN_DELIVERY, DELIVERED, REJECTED, CANCELLED, SCHEDULED, PICKED_UP`
  (particle §4.1; hub review §6.1.3: identical in `packages/domain/src/order-machine.ts` ≡
  `order_status.rs` ≡ `kernel/src/order_machine.rs` on disk). The set is closed; a status
  component that accepts arbitrary strings is wrong by construction.
- **H-3.2** The **status ladder shows the full path with the current step lit** — "the machine's
  plan, not just its present" (direction §5.2).
- **H-3.3** **One transition, one crest**: a committed state transition renders exactly one visual
  swell; nothing animates before the truth is committed (no optimistic shimmer); a tab hidden
  through N transitions **reconciles silently to the final state** — never replay theater
  (direction §5.2; blueprint §3.2 acceptance: crest-counter with the replay RED twin).
- **H-3.4** Transition legality is the kernel's job, not the component's — but components must
  render **refusal honestly**: honest dispatch (no courier on shift ⇒ the order does not advance
  ⇒ the UI says so — hub review §4.2), `DISPATCH_DELAYED` truth to the customer, REJECTED/
  CANCELLED in the calm plain-stakes register (direction §4.4).

### H-4 · Events: one store, one vocabulary, coalesced

- **H-4.1** Every component showing live state subscribes through **one shared event store** —
  never raw WS handling per component (particle §5: `event-stream` store, WS rooms `order:<id>`,
  `location:<id>:dashboard`, `courier:<id>` today; the rebuild's PgListener + `Resync` +
  `read_since(seq)` catch-up later; one core, thin per-framework adapters).
- **H-4.2** Components consume the **unified event registry** — entries carry
  `audience: owner|courier|customer`, `pushClass: alert|status|ambient-only`, and
  `visual: keyof PARTICLE_VOCAB | null` (notif §4.1). A component may not invent an event name;
  drift is a **boot failure** via `assertVocabulary()` (notif §4.2).
- **H-4.3** **Coalescing lives in the store, not in components** (particle §4.3; notif §4.4):
  token bucket ≤1 transient burst / 1.5 s; N same-kind events collapse to one "×N"; **sustained
  states derive from store state, never replayed events**. Out-of-app the same doctrine: ≤1 push
  per order-status transition per audience, `tag`-replace / message-edit, never stacking.
- **H-4.4** Reconnect/visibility discipline is a component-system service: on `visibilitychange`
  → visible or WS reconnect, refetch the snapshot and reconcile sustained state; missed transients
  are deliberately dropped (particle §5).
- **H-4.5** Degradation truthfulness: the `--spectral-void` desaturation vocabulary maps 1:1 to
  genuinely degraded health (`ops.degradation_changed`, WS `Health::Degraded`, stale heartbeat) —
  never to local rendering trouble (GPU context loss must restore, not masquerade — direction §5.4).

### H-5 · Notification components are the doorstep of the same system

- **H-5.1** Toast/badge components obey the coalescing doctrine ("6 new orders", never six
  toasts); `role="alert"`/ARIA live regions remain the authoritative channel the ambient layer
  merely echoes; badges get one-shot shimmer, never continuous pulse (direction §4.5).
- **H-5.2** Push opt-in components carry the **honest transport label** per anonymity tier
  (notif §2): T2 wording ("status alerts use your phone's push service — Apple/Google will see
  that a ping happened"); and are **origin-gated: never rendered on the onion origin** (T3,
  fail-closed server-side too). The onion tracking room says "keep this tab open for live status."
- **H-5.3** Anonymity labels are a component: wherever a channel is offered, its honest label
  renders (adapters §1.3 row 8 — "Convenient, NOT anonymous — Telegram knows your number" /
  "Network-anonymous by default").

### H-6 · Future-proofing against the local-first ladder (synthesis §5)

Components must not embed assumptions the P2–P5 rungs will break: no hard dependency on a central
server URL (base-URL injected), status via poll/WS with foreground-only as a first-class mode,
push treated as **wake signal never state** (notif §1.1, R3), and tolerance for
relay-assisted/intermittent sync. Nothing in the component system may block on bebop2 crypto
(hybrid-only gate, synthesis §3 Q4) — components consume verified state, never verify themselves.

---

## 2. STOREFRONT SOVEREIGNTY — THE TWO-PLANE TOKEN ARCHITECTURE (resolved precisely)

**The nuance:** the customer ordering UI (menu/product/cart/checkout at `/s/:slug`) IS the
vendor-branded storefront, which the design direction **excludes** from dowiz's ambient aesthetic
(direction §4.0/§6.4; blueprint §6.4). Yet those screens must be built from the same disciplined
component system. Resolution: **one structural component library, two token planes.**

### S-1 · The two planes

| | **Plane A — dowiz chrome** | **Plane B — storefront content** |
|---|---|---|
| Surfaces | landing, owner console, courier app, customer **tracking room**, notifications, system states (empty/loading/error/404/login) | menu, product, cart, checkout **content** at `/s/:slug` (and its subdomain/TMA/onion mirrors) |
| Skin | `[data-skin="bebop"]` — Tide over Bedrock: spectral ramps, `--ease-tide`/`--dur-*`, `.horizon-wash`/`.spectral-edge`/`.focal-glow`, grain, noir status remap (blueprint §1) | **vendor theme tokens** (per-tenant fonts/colors/logo from the Branding console) — however plain |
| Who owns the look | dowiz | the vendor — "their room, their light" (direction §6.4) |
| Ambient signatures | particle cloud, spectral edge, horizon wash allowed per budgets | **NEVER** — no wash, no particles, no edge, no grain, no bebop type (blueprint §6.4) |

### S-2 · Three token tiers (what is shared vs vendor-owned)

1. **Structural tokens — shared, plane-neutral (tier 1):** spacing scale, sizing/breakpoints,
   z-order, radii scale, hit-target minima (≥48 px — blueprint §3.4), layout grid, and the
   **motion-discipline tokens as names**: the four tempos (snap 180 ms / reveal 480·900 ms /
   tide 720 ms / breath·drift — direction §2.2; "nothing lives between tempos"). Both planes use
   the same durations/easings; a 350 ms mush animation is a bug on either plane.
2. **Semantic role tokens — shared names, per-plane values (tier 2):** `--surface`, `--text`,
   `--accent`, `--status-*`, focus ring. Components reference **only** role tokens. Resolution:
   Plane A resolves them inside the `[data-skin="bebop"]` block (incl. the §1.5 noir status
   remap); Plane B resolves them from `:root` + the tenant theme — **the `:root --status-*`
   values stay untouched by the bebop remap** (blueprint §1.5), which is exactly how one
   `<StatusChip>` component renders noir in the tracking room and tenant-plain on the vendor menu.
3. **Brand signature tokens — Plane A only (tier 3):** `--spectral-*`, `--wash-opacity`,
   `--edge-glow-w`, the recipe classes. **No component in the shared library may reference a
   tier-3 token**; only chrome-surface compositions may. Mechanically: tier-3 tokens are defined
   only inside the skin block and are non-overridable/non-leaking **in both directions** — "the
   tenant lock cuts both ways" (direction §4.0).

### S-3 · What is shared beyond tokens (structure + trust, both planes)

- **Component anatomy and states**: every component defines loading/empty/error/success; honest
  skeletons that mirror the exact layout they become (direction §5.3); `ErrorHandle` (machine
  `code` + 8-char `correlationId` in a "report this problem" affordance — blueprint §3.3) —
  system-level errors inside the storefront render in "plain, stakes-appropriate style … quiet,
  mono, unbranded-ambient" (direction §4.0).
- **Accessibility contract**: AA contrast gates (with grain composited on Plane A; tenant-theme
  contrast checked on Plane B), WCAG 1.4.1 glyph+label pairing (no state is color-only),
  reduced-motion law, non-text ≥3:1 (blueprint §1.8, §6.2).
- **Trust rules are product-wide, not brand** (blueprint §6.4 clarification — the precise line):
  **money never tweens** (the cart-total `AnimatedNumber` retirement crosses the boundary
  legitimately because it removes a dishonest animation, not adds styling), one money render path
  (`PriceDisplay`/`formatMoney`, mono `tabular-nums`), ETA always a range, one-crest transitions,
  calm errors. Sovereignty excludes **aesthetic**, never **correctness**.
- **i18n discipline** (C-5) and the canonical order contract (H-1.2) — both planes.

### S-4 · What is vendor-owned (Plane B only)

Color palette, typography faces, imagery/logo/hero media, menu copy voice, decorative flavor —
chosen in the Branding console (per-tenant themes + per-tenant fonts are live product features).
dowiz appears on the vendor's stage only as the discreet "powered by dowiz" line and system-level
error/offline states (direction §4.0).

### S-5 · The handoff moment (the seam, specified)

The instant an order is placed, the customer walks from Plane B into Plane A: the tracking room's
status light, ambient layer, motion, and trust cues are dowiz's — **the semantic status ramps,
not tenant colors, so status reads identically under every tenant theme** — while the vendor's
identity (venue name, items, logo) stays on the content layer (direction §4.0/§4.4;
blueprint §3.2). The component system must express this as a **scope boundary in markup** (the
tracking room mounts under the bebop skin scope; the receipt content block does not restyle).

### S-6 · Falsifiable enforcement (the gates the architecture must pass)

- **Storefront zero-diff gate**: Playwright screenshots of `/s/<demo-slug>` menu/cart/checkout
  **byte-identical** before/after any dowiz-brand change — any diff = the aesthetic leaked
  (blueprint §5.7).
- **Token-scope gate**: no tier-3 token referenced outside Plane-A scopes; no raw hex in any
  component file (bible §13 governance via blueprint §1.7); contrast-audit + specimen snapshot
  gates with committed RED cases (blueprint §1.8).
- **Tempo-literal grep gate**: any duration literal outside the token set fails (blueprint §1.3).

---

## 3. NEW-CAPABILITY REQUIREMENTS

### C-1 · Voice: every actionable component is voice-addressable — through one intent bus

- Voice emits the **same intent/event vocabulary as touch, CV, and server events** — one
  `vocab.ts`, one visual system (particle §7.5). Requirement on components: actionable components
  **declare their intents** (a registry the matcher can target), never wire mic handling
  themselves.
- The recognizer is **zero-AI deterministic DSP** (mel/MFCC + DTW on per-user enrolled words,
  measured 2.3 kB gz; language-independent ⇒ **sq/uk/en solved by design** — particle §7.1) plus
  clap/whistle acoustic commands (§7.2). Reject-quietly is the failure mode.
- **Safety invariant (red-line)**: every mutating intent (confirm/reject/accept/pause) passes the
  fail-closed `ConfirmationGate` — a **confirm posture (glyph + amber hold) + an explicit tap**;
  a false match can never mutate state; voice **structurally cannot place an order**
  (particle §7.5; ADR-0015 via hub review §2 row 9). Read-only intents (status, scroll, ambient)
  are unrestricted.
- Context honesty: courier-on-bike voice is impractical in wind — whistle or nothing while
  moving (particle §7.2); components must not assume voice availability (C-4).
- Privacy copy is part of the component: "processing happens on this device; the microphone
  stream is analyzed in the page and never recorded or sent anywhere" (particle §7.6).

### C-2 · Dynamic environment: components express live state through the ambient/spectral layer

- Every event worth surfacing maps to the fixed grammar **(shape-target, palette-shift,
  motion-energy, transient|sustained)** (particle §4.2; direction §3.1 table: order.created =
  life burst; dwell = sustained rust agitation; courier = alive stream; delivered = settle bloom;
  cash discrepancy = anomaly held note; degradation = void desaturation).
- Component hooks, not custom canvases: hosts expose `data-cloud-state` / `data-edge-state` and
  toggle `.is-waving` — the ambient system owns rendering (blueprint §2.1–2.2). **One WebGL
  canvas per page, period** (direction §6.1).
- The ambient layer is **redundant and peripheral** — every visualized event also exists as
  notification/badge/ARIA text; no information exists only in the cloud (direction §3.1;
  particle §4.3). Calm-under-rush is an invariant, not a tuning knob (H-4.3).
- The CSS-only signatures (`.spectral-edge`, `.horizon-wash`) are the **baseline**; the particle
  cloud slots into places components already reserved, without redesign (blueprint §2 preamble).

### C-3 · Touch reactivity

Pointer Events only, `{passive: true}`, **`touch-action: pan-y`** so the ambient canvas never
eats scroll (particle §2); press-hold/flick/pinch as uniform updates; `visualViewport`
keyboard-open resize ignored; battery discipline (pause on `visibilitychange` +
IntersectionObserver). Touchless: **frame-diff optical flow is the required phone tier**
(0.8 kB gz, camera soft-ask) with **tilt/shake as the zero-permission fallback** (iOS one-tap
permission) — particle §3.2. Full hand-tracking (11.2 MB) is kiosk/desktop opt-in only, never a
component dependency.

### C-4 · Graceful degradation — the ladder every component must survive

| Missing | Required behavior |
|---|---|
| Reduced motion | durations 0.01 ms (non-zero so `animationend` fires), sim `dt=0`, loops stilled, events as palette/glyph/opacity crossfades; `change` listener honored **live** (direction §2.2/§6.2) |
| No mic / permission denied | voice tier absent; **every action reachable by touch**; no dead affordances (particle §7.6) |
| No camera | touchless falls to tilt/shake; then touch-only (particle §3.2) |
| No WebGL2 | canvas2D few-hundred particles → **CSS horizon wash + static grain terminus**; "nothing may exist that has an ugly fallback" (direction §6.1) |
| No JS / SSR / onion | storefront funnel legible and orderable (H-2.1); tracking foreground-poll |
| Dead-simple test | unplug the atmosphere — every surface must still be a complete, beautiful interface; if it collapses, the shimmer was doing structure's job (direction §2.5) |

### C-5 · i18n — sq / en / uk as a layout constraint

Trilingual parity is first-class (`SUPPORTED_LOCALES = ['sq','en','uk']`, ~1,515 keys —
blueprint §0.8). Components: width headroom on condensed-caps labels (uk runs long, sq diacritics
need line-height), size on `ch`/content never hard px, **logical properties** for all chrome
offsets, state encoded as hue+glyph never "leftward", directional streams from geometry/bearing
not reading direction; money/error strings plain and identical-in-clarity in all three; dry wit
adapted not translated (direction §6.3). Voice ships per-language suggested vocabularies chosen
for acoustic distinctness (particle §7.4).

### C-6 · Budgets (measured, ratified — component-system acceptance numbers)

- Customer-surface **critical-path JS ~21.6 kB gz**; Astro route classes **25/35/60 kB gz**
  binding (blueprint §6.1; particle §1.3).
- **Ambient stack ≈ 8–11 kB gz, lazy** (`client:idle`-class): particle chunk **≤ 7,000 B gz
  CI-gated** (wave-2 build measured 4,165 B — roadmap §1), inputs ≤ +1,500 B, camera ≤ 1,500 B
  opt-in, voice ≤ 4,000 B ×2 — all lazy, never critical-path (direction §6.1).
- The token/CSS layer itself: **≤ ~2.5 kB CSS, 0 B critical-path JS** (blueprint §6.1).
- Frame discipline: 60 fps mid-tier Android **or reduce layers, never framerate**; idle throttle
  24–30 fps; settle-to-static; context-restore mandatory (direction §6.1).
- Flash ceiling **≤ 1 burst / 1.5 s** as a stated WCAG 2.3.1 invariant (direction §6.2).

### C-7 · Red-lines reflected in what components may SHOW

- **NO-COURIER-SCORING (memory §2; notif §2 T1; adapters §3.4):** no component may render courier
  rankings, ratings, scores, leaderboards, per-courier ack-latency, or any channel/outcome-derived
  courier metric. Telemetry components aggregate by rail/device-class/location — never joined to
  a courier identity. The component library must simply **have no courier-score primitives** —
  absence is the design.
- **Cash truthfulness (notif §3.3; hub review §4.6-3):** settlement components phrase
  *cash held / owed / settled* — never "earnings" for cash-collected, never anything implying
  auto-deduction (NO-AUTO-DEDUCT). `payment_outcome` renders all five outcomes honestly
  (`paid_partial` stays unrepresentable).
- **COD-only surfaces (memory §3; adapters §3.4):** no component renders, links, or requests a
  payment instrument in any channel context; checkout shows cash; in-chat payment affordances are
  forbidden indefinitely (🔴 council-gated).
- **Money display law (direction §5.1):** mono `tabular-nums`, minor-unit exact, one render path;
  a money value **never tweens** — the container may snap 180 ms; digits cut.
- **No marketing/nag surfaces (notif §4.6; direction §4.5):** no re-engagement push components,
  no permanently animated badges — "nagging is aggregator behavior."
- **Anonymity honesty (memory §4; notif §2):** channel-offer components carry pass-through labels;
  push opt-in absent on the onion origin; no component creates a customer profile surface.

---

## 4. AUDIENCE MATRIX — component families × token plane

| Audience | Surface(s) | Component families needed | Token plane | Notes |
|---|---|---|---|---|
| **Owner** | Admin console (orders board, menu manager, settings, couriers, analytics, channels/QR kit, CRM, GDPR) + Telegram ops bot (non-UI) | Console shell + header (spectral edge = room state); orders board + `OrderCard` (status chips, dwell tint); KPI tiles (no money tween); menu CRUD forms; `Toast`/`ConfirmDialog`; `EmptyState`/`SkeletonBase`/`ErrorHandle`; `WSStatusDot` + degradation vocabulary; **attribution reader card** ("Orders by channel" — the hub's missing mirror, hub review §5.4); channels/QR-kit page; courier live map (`.data-surface`); voice confirm posture | **Plane A** (bebop, dark; "terminal that breathes" — direction §4.2) | Data on `.data-surface` islands — no grain/wash/blur over tables ever; burst-calm CLS < 0.02 gate (blueprint §3.1) |
| **Courier** | Courier PWA (tasks, delivery, shift, earnings-as-cash-owed) + out-of-app push/Telegram beep (non-UI rails) | Offer card with **honest countdown ring** (accept window rendered truthfully); task list incl. `offered`; decline/abort/`payment_outcome` controls (the FE catch-up — hub review §4.6-2); shift open/close (ring dispersal); cash-owed summary (C-7 wording); iOS Add-to-Home-Screen interstitial at invite-redeem (notif N2); thumb-first `StickyActionBar`/`BottomTabBar` ≥48 px | **Plane A, daylight remap** (`data-daylight="true"`: grain/glow off, ramps as ink-density — direction §4.3) | Motion budget minimal: snaps + one tide morph; **no drift loops while riding**; voice = whistle-or-nothing moving |
| **Customer — ordering** | Vendor storefront `/s/:slug` menu/product/cart/checkout (+ QR/TMA/widget/onion mirrors) | Menu browse/category/product card; cart; checkout form (6-kind contact selector, `receiver{}`, pin/address, COD); `?ct=` redemption; anonymity/channel labels; SSR/no-JS baseline; system error/offline (plain, mono, unbranded) | **Plane B** (vendor tokens; structure/a11y/trust shared per §2) | Channel-abstract (H-2.1); zero-diff gate protects it; trust rules (money, skeletons, calm errors) still apply |
| **Customer — tracking** | Tracking room `/s/:slug/order/:id` + status push (non-UI) | Status ladder/stepper (10-status, full path); one-tide-swell transition; progress rail spectral edge; ETA **range** display; courier position + polyline map (`.data-surface`); settle bloom / calm grief states; push opt-in (T2 label, onion-gated); touch/tilt ambient reactivity | **Plane A** (the handoff moment — dowiz's room; vendor identity on content layer) | The flagship ambient scene, 20–40 min (direction §4.4); battery honesty |
| **Cross-audience / non-UI renderers** | Telegram messages, push strings, email header rule | The **presentation-free primitives** (H-2.3): status→label/hue/glyph map, money formatter, ETA range, event registry with `audience`/`pushClass`/`visual`, i18n catalogs; static-gradient email fallback of the spectral edge | Plane-neutral (tier 1/2 tokens only) | One vocabulary, three audiences, two contexts — drift = boot failure (notif §4.1–4.2) |

---

## 5. The condensed constraint checklist (what "the component system satisfies the brief" means)

1. One door: components carry Commands, never decide; canonical contract; no second order-writing
   surface; cart-token redemption supported (H-1).
2. Funnel components channel-abstract (SSR, no third-party origins, Tor/TMA-safe); chrome
   components web-only; status/money/event primitives renderer-agnostic; all vocabularies SSOT
   with parity gates; attribution write-only (H-2).
3. 10-status ladder, one-transition-one-crest with replay-RED, refusal honesty (H-3).
4. One event store, unified registry (audience/pushClass/visual), coalescing in the store,
   snapshot reconcile, truthful degradation vocabulary (H-4/H-5).
5. Two-plane tokens: tier-1 structural shared, tier-2 role tokens per-plane, tier-3 signatures
   Plane-A-only; trust rules product-wide; zero-diff + token-scope + tempo gates (S-1…S-6).
6. Voice-addressable via one intent bus, ConfirmationGate on mutations; ambient/touch/touchless
   per the ladder; full degradation ladder; sq/en/uk layout discipline (C-1…C-5).
7. Budgets: 21.6 kB critical / 25-35-60 routes / ambient 8–11 kB lazy / particle ≤7,000 B gated /
   ≤2.5 kB CSS 0 JS for the token layer; 1-burst-per-1.5 s flash ceiling (C-6).
8. Red-lines visible in the library's shape: no courier-score primitives, cash-owed wording,
   COD-only, money-never-tweens, no nag/marketing surfaces, anonymity labels (C-7).
9. Authored once, served twice: CSS custom properties + vanilla-TS cores target-grade for
   Astro/Svelte; React interim marked `// react-interim:`; acceptance criteria portable
   (blueprint §4; roadmap §0.5).
10. Every requirement above ships with a falsifiable gate (VbM) — a gate that cannot go RED does
    not count (blueprint §5.7).

*Prepared 2026-07-12, read-only. The only file created is this report. Source texts recovered
from session transcripts where noted; on-disk corroboration cited in §0 provenance.*
