# DOWIZ INTERFACE DIRECTION — "Tide over Bedrock"
> The visible-interface direction for every dowiz-owned surface. Authored 2026-07-11.
> **Extends** `BRAND-BIBLE.md` ("Warm Cosmo-Noir") — it does not replace one token, law, or
> anti-pattern of it. Where the bible is the tuning fork, this is the wave that runs through it.
> Scope exception up front, because it is principled, not incidental: **the owner's branded
> storefront (`/s/:slug`) is excluded.** See §4.0 and §6.4.

---

## 1. THE DUALITY PRINCIPLE

**The creed, one line:** *The surface breathes because the core never does.*

dowiz is two materials fused: a surface of light — effortless, smooth, ambient, wave-like,
spectral to a degree — over a core of reinforced concrete: deterministic, event-sourced,
mathematically exact, a Swiss watch built to survive a nuclear war. The temptation is to treat
these as a tension to be managed. They are not. **Each one is what makes the other possible:**

- **Determinism is what makes ambience trustworthy.** A state transition can be rendered as a
  slow, sensual wave morph *only because* the state machine beneath it cannot skip, lie,
  double-fire, or invent money. If the core were sloppy, every soft animation would be a soft
  lie. Ours are renderings of real transitions — the order-status wave crests exactly once,
  exactly when `CONFIRMED` became true in the event log, because there is exactly one truth.
- **Ambience is how determinism becomes felt.** Nobody experiences an event-sourced ledger.
  They experience a room that stays calm during a dinner rush, a number that never wobbles, a
  glow that travels toward their food because a real courier bearing says so. The surface's job
  is to *transduce* the core's precision into atmosphere — never to decorate over it.

Three operating consequences, applied everywhere below:

1. **Every motion is caused.** Animation maps 1:1 to a state transition, a user act, or a real
   ambient rhythm (idle breath, drift). Motion invented "to feel alive" is banned — the system
   *is* alive; we only need to let that show.
2. **The calm is load-bearing.** Under burst load the surface coalesces (one wave carrying
   "×12"), it never strobes. Composure under pressure is the visible face of the Swiss watch.
3. **The spectral is honest.** "Spectral" is not a mood word we borrowed — the core genuinely
   runs spectral mathematics (graph heat kernel and FFT, oracle-verified to 1e-10…1e-16 in
   `docs/design/bebop-field-sim-2026-07-11/SYNTHESIS.md` §2). The surface may speak in spectra
   because the machine actually does. The same synthesis found the *iterative* diffusion paths
   broken (sign bug, §3) — so the rule is absolute: **visuals are driven by verified state and
   verified primitives only; we never animate broken math into a costume of life.** If a
   visualization claims to show the field, the field must be real.

This whole document is the bible's HYBRID (§0) with the fader pushed: the soul gets a new
instrument (the tide), the machine gets a new way to be seen (the trust cues of §5). The 90/10
law, the three pillars, the seven voice laws, the anti-pattern red lines — all still binding.

---

## 2. THE AESTHETIC LANGUAGE

### 2.1 Color — the noir palette becomes a spectral system

The bible gives us single notes: amber `#E8A544`, teal `#46B0A4`, rust `#B26850`, gold
`#C79675`, blood `#E0543E`, magenta `#C8438F` on the `--void/--hull/--slate` field. The tide
direction adds **ramps** — each semantic hue gains two shoulders so it can *travel*, the way
light does across water. A ramp is not a second accent: it is **one accent with a gradient
body**, so color law 2 ("one saturated accent per view") survives intact.

**The five spectral ramps** (proposed tokens; live in the `[data-skin="bebop"]` block of
`packages/ui/src/theme/tokens.css` per governance §13, each with a role comment + contrast note):

| Token | Stops (dark field) | Semantic | Where it plays |
|---|---|---|---|
| `--spectral-life` | `#B26850 → #E8A544 → #F0B95E` (rust → amber → lit amber) | new order, action, warmth | order.created burst, primary CTAs' glow, the horizon wash |
| `--spectral-alive` | `#2A6E66 → #46B0A4 → #3FB7C4` (deep teal → teal → cyan) | success, signal, movement | courier streams, ready-for-pickup, live-connection pulse |
| `--spectral-settle` | `#C79675 → #E8A544 → transparent` (gold → amber → fade) | completion, rest | delivered bloom, save-success, end-of-shift |
| `--spectral-warn` | `#B26850 → #E0543E` (rust → blood) | aging, degradation, danger | dwell escalation, dispatch failure — always paired with icon/label |
| `--spectral-anomaly` | `#C8438F` held alone — deliberately **not** a ramp | money anomalies, the rare | cash discrepancy: a single held note, no gradient, no company |

Plus one *direction*, not a color: `--spectral-void` — the desaturation vector toward ash
`#8F8474`. Degradation is rendered by draining chroma from whatever is on screen (the particle
sim's `uSat` uniform, a CSS `filter: saturate()` step on ambient layers), never by adding a new
alarm color. A sick room goes grey; it does not go red.

**Ramp laws (the taste is in these):**

- **≤ ~60° of hue per ramp.** rust→amber→gold is iridescence; teal→cyan is sheen. Amber→teal in
  one gradient is a rainbow, and rainbows are the #1 tell of spectral-as-costume. "Spectral to a
  degree" means adjacent-hue shimmer — mother-of-pearl, not RGB party.
- **Interpolate in OKLCH.** Wide sRGB gradient midpoints collapse into grey mud — exactly the
  dead, desaturated middle we reserve for *meaning* degradation. `linear-gradient(in oklch, …)`
  keeps chroma alive through the body of every ramp (supported in all mainline browsers since
  2023; see [Josh Comeau on gradient dead zones](https://www.joshwcomeau.com/css/make-beautiful-gradients/)
  and [Keith J. Grant's workarounds](https://keithjgrant.com/posts/2023/11/problematic-color-gradients-and-workarounds/)).
  Concrete rule: every `--spectral-*` consumer declares `in oklch`; grey midpoints are a bug.
- **Ramps are for light, never for text.** Washes, edges, particles, glows. Text stays bone
  `#F2E9DB` / taupe `#BCB09C` on dark, warm-ink `#1A1712` / `#5A5044` on daylight — the bible's
  contrast pairs, verified with grain applied (bible §13). A gradient never carries a message a
  label doesn't also carry (WCAG 1.4.1; bible color law 4).
- **Opacity ceilings:** ambient washes ≤ 16% opacity, glow cores ≤ 35% alpha, edge-lights at
  full chroma but 1–2 px thin. The spectral system whispers; the void does the talking.

**Daylight (light) counterparts.** On the courier daylight remap (`[data-daylight="true"]`) and
any future light context, glow is a lie under sunlight — the bible already rules **density
replaces glow**. Ramps remap to their ink-anchored twins: life anchors on darkened amber
`#A85F14`, alive on `#1F6E5A`, warn on `#B0201B` (the existing daylight tokens), rendered as
*density/weight* gradients (particle count, stroke weight, fill steps) rather than luminous
haze. Same music, matte finish.

### 2.2 Motion — three tempos, one new curve

The bible's two jazz curves stay canon: `--ease-jazz-in` `cubic-bezier(0.23, 1, 0.32, 1)` for
entrances, `--ease-jazz-snap` `cubic-bezier(0.32, 0.72, 0, 1)` at `--dur-snap` 180 ms for
interactions. The tide adds the third and final curve:

- **`--ease-tide: cubic-bezier(0.37, 0, 0.63, 1)`** — a pure sinusoidal in-out. No attack, no
  overshoot, no snap: a swell. This is the curve of *state*, of things that were always going
  to happen and now simply do. Order-status morphs, spectral-edge travel, gradient pans,
  particle palette crossfades.
- **`--dur-tide: 720 ms`** — the state-change swell (inside the bible's ≤ ~1 s choreography law).
- **`--dur-breath: 3600 ms`** — formalizes the bible's 3–4 s pilot-light idle into a token.
- **`--dur-drift: 48 s`** — alias of the existing `--dur-ambient` (the Horizon Drift loop),
  named for its role: the slowest layer of atmosphere.

**The tempo doctrine:** every animation on a dowiz surface declares one of exactly four tempos —
**snap (180 ms)** for the machine answering your hand, **reveal (480/900 ms, jazz-in)** for
content arriving, **tide (720 ms)** for the world changing state, **breath/drift (3.6 s / 48 s)**
for the atmosphere existing. Nothing lives between tempos; a 350 ms mush animation is a bug the
same way an off-palette hex is. The contrast *between* tempos — languid idle, sinusoidal state
swell, instant snap — is the whole feeling of "effortless surface, exact core" (it is the
bible's "low-power idle → sharp engage," §6, given a middle register).

**Reduced motion** (hard law, inherited and extended): durations to 0.01 ms everywhere, ambient
loops frozen to composed stills, particle sim `dt = 0`, and **every event that would have been
motion becomes a color/opacity crossfade only** — state changes remain fully legible as palette
+ glyph + label. The `prefers-reduced-motion` media query must be *listened to*, not sampled
once at mount (review §6.7).

### 2.3 Depth & atmosphere — the layer recipe

One canonical stack, back to front, for any dowiz surface that carries atmosphere:

1. **Field** — solid `--void` or `--hull`. Never `#000`.
2. **Spectral wash** — one large gradient (radial or linear, `in oklch`), ≤ 16% opacity,
   drifting at breath or drift tempo. The landing's `lp-sky` god-lit horizon is the archetype.
3. **Particle canvas** — the singleton WebGL2 layer (§3), z-indexed under content,
   `pointer-events` per surface policy.
4. **Content** — type, data, controls. Fully opaque, fully legible, owns the z-order.
5. **Grain** — the existing `::after` 35 mm layer at `--grain-opacity` 0.05, `soft-light`,
   opted out over `.data-surface`/`.stage-surface` (already in the skin).
6. **Vignette** — stage surfaces only, one radial darkening.

Restraint budget per view: **one wash, one canvas, one focal glow, at most one
backdrop-blur element.** Glow sits behind amber accents (warm bloom, bible §8) and along the
spectral edge (§3.2) — never behind or on text. Anamorphic (horizontal) flares only; radial
star-bursts stay banned.

### 2.4 Typography — unchanged, and that is the point

The three-register system (Fraunces display / Oswald condensed caps / DM Sans body / Space Mono
data) is already effortless: the serif seduces, the sans disappears, the mono testifies. The
tide direction adds **no new faces** and one emphasis: as surfaces get more atmospheric, the
type must get *more* disciplined, not less — atmosphere is carried by light and motion, never
by distorted, gradient-filled, or animated type. Numbers, money, IDs, timestamps stay mono
`tabular-nums` always (§5.1). Letter-stagger reveals stay on jazz-in at reveal tempo; no
per-character rainbow tricks, no text masks over video.

### 2.5 "Sexy without tacky" — the taste guardrails

The line between wave-like sexy and lava-lamp kitsch is discipline. The rules that hold it:

- **Negative space is the instrument.** ~90% of any view is still, dark, quiet field (the
  bible's governing ratio). The shimmer is desirable because it is scarce.
- **One focal shimmer per view.** One wash, one glow, one traveling wave. Two simultaneous
  shimmer sources compete; ten is a casino.
- **Adjacent hues only** (the ≤ 60° law). Iridescence, not rainbow.
- **Slow is sensual, fast is alarm.** Anything luminous moves at tide tempo or slower. Fast +
  glowing = slot machine.
- **The atmosphere never touches the data.** Washes, particles, and grain live behind and
  around; tables, forms, prices, maps sit on clean `.data-surface` islands.
- **No glass stacks, no synthwave.** One backdrop-blur max; no neon-on-`#000`, no HUD chrome,
  no grid horizons — the bible's cyberpunk red line, restated because spectral work drifts
  there fastest.
- **The dead-simple test:** unplug the atmosphere (reduced-motion, no-WebGL fallback) — the
  surface must still be a beautiful, complete, warm-noir interface. If the design collapses
  without its shimmer, the shimmer was doing structure's job, and it must be redesigned.

---

## 3. THE WAVE/SPECTRAL MOTIF, OPERATIONALIZED

Three signatures make the motif recognizable across every surface. All three are already
specced, measured, or built — this section fixes their roles.

### 3.1 Signature 1 — the particle cloud (the connective tissue)

The spectral particle cloud (`docs/research/2026-07-11-particle-cloud-interaction-analysis.md`
+ `docs/design/particle-cloud-2026-07-11/{REVIEW,PLAN}.md`) is the flagship ambient element:
a hand-rolled WebGL2 transform-feedback sim (measured ~3.6 kB gz core; realistic production
core 5–7 kB), one **singleton canvas per page**, palette fed verbatim from the Warm Cosmo-Noir
tokens, idle at the "2 a.m. bar" state (energy ≈ 0.15, slow hash-noise drift, amber-in-noir
90/10 grading, breathing at `--dur-breath`).

Its grammar: every event maps to a tuple **(shape-target, palette-shift, motion-energy,
transient|sustained)**. The canonical vocabulary (research §4.2, all events verified against
the 21-key registry at `apps/api/src/notifications/event-registry.ts`):

| Event | The wave it makes |
|---|---|
| `order.created` | **warm burst** — `--spectral-life` pulse → particles condense into the order-number glyph ~4 s → release to ambient |
| `order.pending_aging` / dwell | **sustained agitation** — rust drift, slowly tightening ring; derived from state, persists until actioned |
| offer sent → `courier.assigned` | **directional stream** — `--spectral-alive` current toward the screen edge; assignment lands as a brief teal condense |
| `order.ready_for_pickup` | teal condense pulse |
| `order.delivered` | **`--spectral-settle` bloom** — gold rises, slow settle; the completion breath |
| `order.dispatch_failed` / flag | blood-tinted turbulence + field desaturation + "!" glyph (always paired with the real notification) |
| `cash.reconcile_discrepancy` | `--spectral-anomaly` magenta flicker + hold — the rare-accent budget spent only on money |
| `ops.degradation_changed` / WS degraded | **global desaturation** along `--spectral-void` + turbulence up — the room itself feels off until healthy |

**Calm under the dinner rush (invariant, not tunable):** transient bursts pass a token bucket —
**max 1 burst per 1.5 s**; N same-kind events in a window collapse into **one** wave carrying a
"×N" glyph; sustained states derive from store state, never replayed events (40 missed frames
reconcile to one final posture, not 40 animations). This is simultaneously the composure cue
(§1.2) and the WCAG 2.3.1 three-flashes guarantee (PLAN §3). The cloud is a **redundant,
peripheral channel** — this is the calm-technology doctrine verbatim: technology that "informs
but doesn't demand our focus," moving between periphery and center only when meaning requires
it ([Weiser & Brown, "The Coming Age of Calm Technology"](https://calmtech.com/papers/coming-age-calm-technology);
[Amber Case's principles](https://www.caseorganic.com/post/principles-of-calm-technology)).
Every event it renders is also delivered by notifications/badges/ARIA live regions; no
information exists only in the cloud.

### 3.2 Signature 2 — the spectral edge (the cheap ubiquitous cousin)

Where the canvas doesn't ship (dense admin sub-pages, emails' header rule, low-power fallback),
the motif survives as **one 1.5 px edge-light**: a gradient line along a single container edge
(top of the console header, the tracking page's progress rail) whose hue position encodes the
current sustained state, and along which an event travels as a soft wave of light at
`--ease-tide`/`--dur-tide`. Technically a `background-position` pan on an `in oklch` gradient —
CSS-only, zero JS, reduced-motion falls back to a plain crossfade of the line's color. The
reference for the feel — a spectral light that lives at the *edge* of the interface and waves
when the system is attending — is Apple's Siri/Apple Intelligence edge glow, built from
hue-shifting mesh gradients with sine-wave edge motion
([Rudrank Riyam's reconstruction](https://rudrank.com/exploring-swiftui-creating-new-siri-animation),
[pocket-lint overview](https://www.pocket-lint.com/how-to-get-new-siri-look-glowing-border/)) —
ours is warmer, slower, and never full-perimeter (one edge, not a frame; a frame is a costume).

### 3.3 Signature 3 — the horizon wash (the resting state)

The landing's `lp-sky` — amber lamp-glow bleeding up from below into warm near-black — is the
brand's resting image (the "god-lit line," bible §7). Reused as the canonical backdrop for
empty states, loading gates, login, and 404 across all dowiz surfaces: one radial
`--spectral-life` wash at the bottom of the field, breathing at drift tempo. Where the Horizon
Drift ship flies (landing, admin login), it is the full signature; everywhere else the wash
alone is enough — the horizon without the ship still says dowiz.

---

## 4. PER-SURFACE APPLICATION

### 4.0 The boundary, first: the vendor's storefront is sovereign

**dowiz frames and hosts; the vendor's storefront content is sovereign.** The per-tenant menu,
cart, and checkout at `/s/:slug` belong to the *vendor's* brand — their fonts, their colors,
their theme, chosen in the Branding console. The dowiz ambient aesthetic — washes, particles,
spectral edges, grain, bebop type — **must never override, tint, or leak into that surface.**
This is not a styling preference; it is the product's ideology (the bible's "dining room vs
engine room," §1, and the tenant lock in `tokens.css`: `[data-skin="bebop"]` is
internal-brand, never storefront-overridable — and the inverse holds identically). The vendor
paid for sovereignty; the frame proves it by staying out.

Where dowiz *may* appear on that surface: the functional chrome dowiz itself owns (a discreet
"powered by dowiz" line, system-level error/offline states rendered in plain, stakes-appropriate
style) — quiet, mono, unbranded-ambient. No shimmer on someone else's stage.

**The handoff moment:** the instant an order is placed, the customer walks from the vendor's
dining room into dowiz's engine room — the tracking view (§4.4). The vendor's name stays on the
receipt content (venue name, items, their identity in the header); the *room* — status light,
ambient layer, motion, trust cues — is dowiz's, because from here on the thing being displayed
is dowiz's reliability.

### 4.1 Landing / marketing — the seduction (built; the seed)

Already live and canon: Nomadic skeleton, Horizon Drift, gated sessions, dry-wit trilingual
copy. The tide direction adds only: convert the hero's decorative mesh-gradient nebula into
the disciplined §2.3 stack (one wash + grain, `in oklch` stops from `--spectral-life`), and —
when the particle program is funded (PLAN D-PC1) — let the end-card CTA moment carry the one
focal condense (particles forming the wordmark, once, on scroll-arrival, never looping).
Landing is the only surface where the atmosphere may take up to ~20% of the frame; it is the
concert hall. Money/CTA copy stays plain per voice law 7.

### 4.2 Owner/admin console — the terminal that breathes

The working tool. Bebop machine register turned up (mono data, terminal calm, dry labels;
bible §11), and the ambience turned *down into the periphery*: the particle canvas behind the
orders board at low energy, the spectral edge on the console header carrying sustained state,
the horizon wash only on empty states. Data lives on `.data-surface` islands — no grain, no
wash, no blur over tables, forms, or charts, ever. The dashboard's emotional promise: **you can
feel the room without reading it** — a quiet amber shimmer somewhere at the edge of vision
means orders are flowing; a slow greying means something is wrong *before* you read the toast.
During a rush the cloud coalesces (×N waves) and the console itself stays metronome-steady:
lists don't reflow-jump (new orders enter with a single jazz-in reveal, 480 ms, then hold),
totals update in place, nothing bounces. Panic is a customer-facing aggregator behavior; the
pass at dinner rush is calm hands.

### 4.3 Courier app — daylight, density, one-handed

Outdoors, sunlight, moving, high stakes: the ambient system defers to legibility. Daylight
remap on (`data-daylight="true"`): grain off, glow off, ramps as ink-density (§2.1). Motion
budget minimal — snaps and one tide morph per state change; no drift loops while riding. The
two sanctioned ambient moments: **task offered** — particles (or, cheaper, the spectral edge)
condense into the order glyph with a shrinking countdown ring, the accept window made honest
and visible; and **shift close** — a perimeter ring quietly dispersing. Everything else is
buttons a thumb can hit on a moving bike.

### 4.4 Customer order-tracking — the flagship ambient moment

Post-order, the customer holds their phone for 20–40 minutes. This is dowiz's one long scene
with the end customer, and where the duality is *the product*: the honest ETA range (already
server-derived), the exact status ladder, and above it an atmosphere that makes waiting feel
attended. Per status (the existing 10-state machine, `--status-*` tokens):

- Each status owns one hue posture; **transitions are single tide swells** (`--ease-tide`,
  `--dur-tide`) of wash + edge + particle palette — one crest per transition, because there is
  exactly one transition.
- `IN_DELIVERY`: the traveling wave — flow field biased toward the live courier bearing; the
  cloud literally leans toward your food (research §4.2).
- `DELIVERED`: the settle bloom, once, then rest.
- `REJECTED`/`CANCELLED`: desaturate and slow-fall behind the plain, stakes-rule message. The
  atmosphere grieves quietly; it never dramatizes bad news.
- Touchless (P2): tilt-primary (zero permission), camera "wave mode" strictly opt-in per
  PLAN D-PC3.
- Battery honesty: idle throttle (~24–30 fps under energy threshold), settle-to-static after
  N idle minutes, `visibilitychange` pause — a 40-minute companion must sip, not gulp.

Tenant identity remains on the content layer (venue name, items, their logo); the ambient/status
layer speaks dowiz's spectral language — the semantic ramps, not tenant colors, so status reads
identically under every tenant theme.

### 4.5 Notifications — the voice, at the doorstep

Telegram/push/toast copy is already canon (voice §9–10: dry wit on brand moments, plain on
money/security). Visual direction: toasts enter at jazz-in reveal tempo, hold still, exit at
ease-in; `role="alert"`/ARIA live regions stay the authoritative channel the ambient layer
merely echoes. Notification bursts coalesce under the same 1/1.5 s doctrine ("6 new orders",
never six stacked toasts). In-app badges may carry a one-time spectral-edge shimmer on arrival;
they never pulse continuously — a permanently animated badge is nagging, and nagging is
aggregator behavior.

### 4.6 Empty / loading / error states — the atmosphere's true home

These are the states with no data to protect — the one place atmosphere may take the room.

- **Empty:** the horizon wash + the canonical dry-wit line ("Quiet night. Nothing on the pass
  yet."), particles at their lowest idle. Empty is not an apology; it is the 2 a.m. bar.
- **Loading:** honest skeletons that mirror the exact layout they will become, shimmering at
  breath tempo (never a fast sweep); indeterminate spinners only where progress is genuinely
  unknowable — where progress is real (menu import stages, model downloads) show real stages.
  See §5.3.
- **Error:** calm and truthful (§5.4). The room desaturates one step; one `--blood` accent +
  icon + plain or dry copy per the stakes rule; no shakes, no red floods, no sad-face
  illustrations. Errors are where the bulletproof core shows its manners.

---

## 5. WHERE THE SURFACE SHOWS THE CORE'S STRENGTH

Trust cues — the Swiss watch made visible without bragging. Each one is a standing rule.

### 5.1 Money is displayed, never performed

Prices, totals, refunds: mono, `tabular-nums`, minor-unit exact via the one `formatMoney` path
(`PriceDisplay` already does this — keep it the only way money renders). **A money value never
tweens.** No count-up odometers, no rolling digits: an animated number is an approximated
number, and the core does not approximate. When a total changes, the *container* may take a
180 ms snap of acknowledgment; the digits themselves cut to the new exact value. Same for
order IDs and timestamps: the machine's CRT voice (bible §4) states; it does not perform.

### 5.2 State transitions are exact, so render them exactly once

The wave morph is the visual contract of the state machine: **one transition, one crest.** If
the tab was hidden through three transitions, the surface reconciles silently to the final
state (snapshot reconcile / `read_since` catch-up) — it does not replay theater of the missed
states. The status ladder always shows the full path with the current step lit: the customer
can see the machine's plan, not just its present. And a transition never animates before the
truth is committed — optimistic shimmer on unconfirmed state is lying with light.

### 5.3 Loading is honest

Skeletons reflect the real shape and count of what's coming (never generic grey soup);
progress bars appear only when progress is measurable, else the breath pulse + the canonical
line ("Working. The machine doesn't rush — neither should you."). The ETA is a server-derived
*range*, displayed as a range — fake single-minute precision is forbidden. If the system is
degraded, loading says so (desaturation + plain line) rather than spinning forever: an honest
"this is slow right now" outranks an infinite optimistic spinner.

### 5.4 Errors are calm, specific, and carry a handle

The error-contract discipline (docs/design/error-contract-parse-token-economy/resolution.md) is
a *visible* asset: every error has a machine `code`, a `correlationId`, and the UI surfaces a
short speakable handle (first 8 chars) inside a "report this problem" affordance — the
interface saying *we log everything; this failure has a name; you are not shouting into a
void.* Voice per stakes: money/auth errors plain and identical-in-clarity in sq/en/uk; brand
moments may keep the dry wit. Visually: one step of desaturation, one blood accent with icon,
still composition. Degradation states must be *true*: the desaturation vocabulary is reserved
for genuinely degraded health (WS heartbeat, ops events) — which is why GPU context-loss must
restore rather than masquerade as system trouble (REVIEW §6.3).

### 5.5 The quiet flex

The core's strength is never claimed in adjectives on working surfaces ("blazing fast,"
"rock solid" — banned, voice law 4). It is demonstrated: the number that is always exact, the
wave that crests exactly once, the room that stays calm at ×12, the error that introduces
itself by name. On the landing, where selling is the job, the machine may be *described* — in
the dry register already shipped ("It cannot invent a price or lose a sale. It simply refuses
to lie."). Everywhere else: show, don't say.

---

## 6. GUARDRAILS

### 6.1 Performance budgets (measured, ratified — not aspirations)

- **Critical-path JS on the customer surface stays ~21.6 kB gz** — the ambient stack never
  rides the critical path. Route classes 25/35/60 kB gz per G05 remain binding.
- **Full ambient stack ≈ 8–11 kB gz, lazy-loaded** (`client:idle`-class or post-hydration):
  particle chunk (core+vocab+store+mount) CI-gated at **≤ 7,000 B gz** with a RED-provable
  fixture; pointer+tilt inputs **≤ +1,500 B gz**; camera motion-flow a separate opt-in lazy
  chunk **≤ 1,500 B gz**; voice DSP core and enrollment UI **≤ 4,000 B gz each**, both lazy
  (PLAN §2). Whether deferred decoration chunks count against route budgets is the FE-0.1
  signature decision — until signed, assume they count.
- **Frame discipline:** 60 fps on mid-tier Android or reduce layers — never framerate (bible
  §7). Idle throttle ~24–30 fps, settle-to-static after N idle minutes, pause offscreen/hidden,
  adaptive particle count (halve N on >20 ms frames). **One WebGL canvas per page, period.**
  Context-loss must re-init on `webglcontextrestored`.
- **Degradation terminus everywhere:** no WebGL2 / no JS / reduced-everything → the CSS horizon
  wash + static grain. The brand's floor is already beautiful; nothing may exist that has an
  ugly fallback.

### 6.2 Accessibility

- **Reduced motion = color/opacity only.** Sim frozen (`dt=0`), loops stilled to composed
  frames, events as palette/glyph crossfades; honor `change` events live. Non-zero 0.01 ms
  durations so `animationend` still fires (bible §6).
- **The ambient is never the only signal.** Every visualized event also exists as
  notification/badge/ARIA live text; glyph pairs with color so no state is color-only
  (WCAG 1.4.1). The cloud accompanies; it never replaces.
- **Flash ceiling as invariant:** ≤ 1 burst / 1.5 s stated as a WCAG 2.3.1 design invariant —
  a future "make it punchier" request bounces off this line, not off someone's memory.
- **Contrast:** every text/accent pair verified (APCA Lc ≥ 60 / WCAG AA) **with the grain
  overlay applied** and on the actual surface step, not just canvas; glows always have a
  solid-outline fallback; `--ash` never on small body text. Daylight variant re-verified
  separately.

### 6.3 i18n — sq / en / uk

Full trilingual parity is already first-class (`SUPPORTED_LOCALES = ['sq','en','uk']`); the
direction adds layout discipline: **condensed-caps labels and eyebrows get width headroom**
(uk runs long; sq diacritics need clean line-height), buttons and chips size on `ch`/content,
never hard px widths. **RTL-safe habits now, cheaply:** logical properties
(`margin-inline-start`, `inset-inline-end`) for all chrome offsets (the corner-pinned HUD, the
spectral edge's travel direction), direction-agnostic status encoding (hue + glyph carry state;
"leftward" never does), and directional streams (courier flow) derived from geometry/bearing,
not reading direction. Dry wit is *adapted*, not translated (voice §9); money/error strings
plain and identical-in-clarity across all three.

### 6.4 The sovereignty boundary (restated as a check)

Before shipping anything ambient, ask: **is this pixel on the vendor's stage?** If it renders
inside the `/s/:slug` menu/cart/checkout content, the dowiz aesthetic does not apply — tenant
theme only, however plain. If it is dowiz chrome (landing, console, courier, tracking room,
notifications, system states), the full direction applies. The tenant lock is mechanical
(skin tokens are not storefront-overridable) but the discipline is cultural: no "just a subtle
dowiz shimmer" on the vendor's menu, ever. Their room, their light.

### 6.5 Process guardrails

New `--spectral-*` and `--ease-tide`/`--dur-*` tokens land only in the `[data-skin="bebop"]`
block with role + contrast comments (governance §13); no hexes in component files. Every
ambient feature ships with its reduced-motion twin, its no-WebGL fallback, its size-gate, and
a falsifiable proof (VbM) in the same change. And the final taste check, extending the bible's
directive: *would the chain-smoking bounty hunter find this beautiful at 2 a.m. — and would
the machine underneath still take their money honestly at 8 p.m. on a Friday?* Both yes, or
it doesn't ship.

---

## Appendix — proposed token additions (names final, values to verify per §6.5)

| Token | Value / stops | Role |
|---|---|---|
| `--ease-tide` | `cubic-bezier(0.37, 0, 0.63, 1)` | sinusoidal state swell — status morphs, spectral travel |
| `--dur-tide` | `720ms` | one state-change crest |
| `--dur-breath` | `3600ms` | idle breathing (formalizes bible §6's 3–4 s) |
| `--dur-drift` | `48s` | alias of `--dur-ambient`; slowest atmosphere layer |
| `--spectral-life` | `#B26850 → #E8A544 → #F0B95E`, `in oklch` | action/new-order ramp; horizon wash |
| `--spectral-alive` | `#2A6E66 → #46B0A4 → #3FB7C4`, `in oklch` | success/signal/movement ramp |
| `--spectral-settle` | `#C79675 → #E8A544 → transparent`, `in oklch` | completion bloom |
| `--spectral-warn` | `#B26850 → #E0543E`, `in oklch` | aging/degradation/danger (icon-paired) |
| `--spectral-anomaly` | `#C8438F` (single stop by design) | money anomalies — the held note |
| `--spectral-void` | desaturation vector → `#8F8474` | degradation direction (drain, don't add) |
| `--edge-glow-w` | `1.5px` | spectral-edge thickness ceiling |
| `--wash-opacity` | `0.16` | ambient wash opacity ceiling |

**External references that shaped concrete choices:** OKLCH gradient interpolation for the
ramps ([Comeau](https://www.joshwcomeau.com/css/make-beautiful-gradients/),
[Grant](https://keithjgrant.com/posts/2023/11/problematic-color-gradients-and-workarounds/));
the Siri/Apple Intelligence edge glow for the spectral edge's feel and its restraint-inversion
([Riyam](https://rudrank.com/exploring-swiftui-creating-new-siri-animation),
[pocket-lint](https://www.pocket-lint.com/how-to-get-new-siri-look-glowing-border/)); Weiser &
Brown's calm technology + Amber Case's principles for the periphery↔center doctrine of the
ambient layer ([calmtech.com](https://calmtech.com/papers/coming-age-calm-technology),
[caseorganic.com](https://www.caseorganic.com/post/principles-of-calm-technology)). In-repo
foundations: `BRAND-BIBLE.md`, the built landing (`apps/web/src/pages/landing/`), the
particle-cloud analysis/review/plan (2026-07-11), and the field-sim synthesis
(`bebop-field-sim-2026-07-11/SYNTHESIS.md`) for the honest-spectral rule.
