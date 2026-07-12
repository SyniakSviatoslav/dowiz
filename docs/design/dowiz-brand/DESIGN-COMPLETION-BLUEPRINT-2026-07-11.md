# DOWIZ DESIGN-COMPLETION BLUEPRINT — "Tide over Bedrock", made executable
> Authored 2026-07-11. **Operationalizes** `INTERFACE-DIRECTION-2026-07-11.md` (same directory)
> into a phased, quality-gated completion plan for every dowiz-owned surface. It adds no new
> taste — every aesthetic decision here traces to the direction, the `BRAND-BIBLE.md`, or a
> measured fact in the tree. **The owner-branded storefront (`/s/:slug` menu/cart/checkout
> content) is excluded throughout** (direction §4.0/§6.4) — restated as a hard, *falsifiable*
> rule in §6.4.
>
> **Status: BLUEPRINT ONLY.** No code, no token, no file outside this one was created or
> modified. Repo left as found on `feat/paleo-dinosaur-digs`. Every file:line below was
> verified read-only this session.
>
> **Why this document exists:** the operator values quality and holds remote vendor outreach
> until design + tech are STABLE. §5.7 gives that word a checkable definition — a list that can
> go RED — so "stable enough to send" is a gate, not a feeling.

---

## 0. GROUND TRUTH — the measured delta between the direction and the tree

The direction describes a destination. This is the audited distance to it (all verified this
session):

| # | Finding | Evidence | Consequence for the plan |
|---|---|---|---|
| 0.1 | **The bebop skin is DORMANT everywhere except the landing.** Only `LandingPage.tsx:50` sets `data-skin="bebop"`. Admin, courier, and client shells still spread `paperSkinAttr()` (off by default → prod admin/courier run the pre-brand identity); the 404 in `main.tsx:93` hardcodes `data-skin="paper"`. | `packages/ui/src/theme/tokens.css:573` ("STATUS: DORMANT"), `apps/web/src/routes/AdminRoutes.tsx:132`, `CourierRoutes.tsx:45,58`, `ClientLayout.tsx:203`, `main.tsx:93`, `packages/ui/src/theme/paperSkin.ts:24` | The Pass-2 skin flip (bible §14, blast radius already mapped) is the **first move of P1**. Nothing in the direction can ship to a surface that isn't wearing the skin. |
| 0.2 | **The 10 `--status-*` tokens are pre-brand Tailwind hexes** (`#2563EB` cobalt confirmed, `#7C3AED` violet scheduled, …) living in `:root`, never remapped in the bebop block. The flagship tracking surface currently speaks a foreign color language. | `tokens.css:87–108`; bebop block `tokens.css:578–726` contains no `--status-*` | §1.5 defines the noir status remap; it is a P1 token deliverable, consumed by P2 tracking. |
| 0.3 | **Money tweens exist in production today** — `AnimatedNumber` (rAF count-up, `molecules/AnimatedNumber.tsx`) animates the **cart total through `formatMoney`** and the **dashboard revenue figure**. Direct violation of direction §5.1 ("a money value never tweens"). | `ClientLayout.tsx:245` (cart total), `DashboardPage.tsx:451` (revenue `/1000`k), `AnalyticsPage.tsx:265` (KPI cards incl. money formatters) | §3.1/§3.2 acceptance criteria retire it from every money path; §5.7 carries a grep-level CI gate so it cannot return. |
| 0.4 | **Landing gradients interpolate in sRGB** — `lp-sky` and friends have no `in oklch`, exactly the grey-midpoint dead zone the direction bans. | `apps/web/src/pages/landing/landing.css:44–50,111,119–120`; nebula at `LandingPage.tsx:69` | P4 converts them; the token specimen snapshot (§1.8) makes regressions visible. |
| 0.5 | **The error-contract handle is never surfaced** — zero renders of `correlationId` anywhere in `apps/web`. §5.4's "report this problem" affordance is **net-new design work**, not a port. | grep over `apps/web/src` (0 hits); contract at `docs/design/error-contract-parse-token-economy/resolution.md` | §3.3 specs the `ErrorHandle` molecule; P2 deliverable. |
| 0.6 | **Strong foundations already stand** (do not redesign): honest server ETA *range* + real-machine stepper + SR announcements on tracking (`OrderStatusPage.tsx:80–84,253,529,594`); `PriceDisplay`/`formatMoney` as the single money render path (`atoms/PriceDisplay.tsx`); a TS motion SSOT that mirrors CSS tokens (`packages/ui/src/lib/motion.ts:1–37`); an AA contrast-gate *pattern* to extend (`theme/palette.contrast.test.ts`); `SkeletonBase`/`EmptyState`/`WSStatusDot` (`components/Status.tsx:7,14,41`); grain, daylight remap, reduced-motion freeze already in the skin (`tokens.css:667–726`). | cited inline | The plan **extends** these; it never forks them. |
| 0.7 | Component census: atoms (`PriceDisplay, CurrencySwitcher, SearchInput, SegmentedControl, Select, SunlightToggle, Textarea`), molecules (`Toast, ConfirmDialog, ResponsiveDialog, StickyActionBar, LiveDot, Pressable, BottomTabBar, MessageThread, AnimatedCheck, AnimatedNumber, PullToRefreshIndicator, MobilePicker, TourHint, UndoRedoButtons, CommandPalette…`), maps (`MapLibreBase, MapWithPin, MapWithRadius, CourierLiveMap`). | `packages/ui/src/components/*` | §3's per-surface inventories reference these by name. |
| 0.8 | i18n is trilingual first-class (`SUPPORTED_LOCALES = ['sq','en','uk']`), catalog ~1,515 keys. | `packages/ui/src/lib/i18n.ts:72`; G05 §2.6 | §6.3 width/RTL discipline applies to every new component. |

---

## 1. THE TOKEN LAYER, MADE REAL

### 1.1 Where it lands

All additions go **inside the existing `[data-skin="bebop"]` block** of
`packages/ui/src/theme/tokens.css` (bible §13 governance: single source of truth, role +
contrast comment per token, no hex ever in a component file). The block is extended, not
duplicated. One PR carries: ramp stops, motion tokens, recipe utilities, the status remap, the
daylight counterparts, and the two gates of §1.8.

### 1.2 Spectral ramps — stop-token architecture

**Design decision:** ramps ship as **stop triplets**, not pre-baked gradient strings. A custom
property holding a whole `linear-gradient(...)` cannot be re-composed (radial for the wash,
linear for the edge, `stop-color` if ever SVG), and it hides the interpolation mode. Stops keep
one source of truth; each consumer composes with `in oklch` **at the point of use** — which is
the direction's own rule ("every `--spectral-*` consumer declares `in oklch`"; grey midpoints
are a bug, per [Comeau](https://www.joshwcomeau.com/css/make-beautiful-gradients/) /
[Grant](https://keithjgrant.com/posts/2023/11/problematic-color-gradients-and-workarounds/) —
`in oklch` gradient interpolation ships in all mainline browsers since 2023).

Proposed tokens — values derived from the **existing** bible hexes (nothing invented), OKLCH
computed this session (sRGB→OKLab, Björn Ottosson's reference matrices):

| Token | Hex (existing) | OKLCH (computed) | Role comment to carry |
|---|---|---|---|
| `--spectral-life-0` | `#B26850` (--rust) | `oklch(59.5% 0.102 38.7)` | life ramp, dark shoulder |
| `--spectral-life-1` | `#E8A544` (--amber) | `oklch(76.9% 0.136 73.0)` | life ramp, body — the brand note |
| `--spectral-life-2` | `#F0B95E` (--amber-hi) | `oklch(81.8% 0.125 78.0)` | life ramp, lit crest |
| `--spectral-alive-0` | `#2A6E66` | `oklch(49.3% 0.069 184.6)` | alive ramp, deep shoulder |
| `--spectral-alive-1` | `#46B0A4` (--teal) | `oklch(69.3% 0.099 184.9)` | alive ramp, body |
| `--spectral-alive-2` | `#3FB7C4` (--cyan) | `oklch(71.9% 0.105 205.4)` | alive ramp, sheen crest |
| `--spectral-settle-0` | `#C79675` (--gold) | `oklch(71.2% 0.075 55.1)` | settle ramp, rest |
| `--spectral-settle-1` | `#E8A544` (--amber) | `oklch(76.9% 0.136 73.0)` | settle ramp, glow (fades → transparent at use) |
| `--spectral-warn-0` | `#B26850` (--rust) | `oklch(59.5% 0.102 38.7)` | warn ramp, aging |
| `--spectral-warn-1` | `#E0543E` (--blood) | `oklch(62.9% 0.179 31.4)` | warn ramp, danger — **always icon/label-paired** |
| `--spectral-anomaly` | `#C8438F` (--magenta) | `oklch(59.2% 0.185 349.0)` | single held note by design — money anomalies only, no gradient |
| `--spectral-void` | `#8F8474` (--ash) | `oklch(61.9% 0.027 77.3)` | desaturation *terminus* — degradation drains chroma toward this; never an added alarm color |

**≤60° hue law — verified by math, this session:** life spans 38.7°→78.0° = **39.3°** ✓ ·
alive 184.6°→205.4° = **20.8°** ✓ · settle 55.1°→73.0° = **17.9°** ✓ · warn 38.7°→31.4° =
**7.3°** ✓. All four ramps are iridescence, not rainbow. The §1.8 gate re-computes these spans
from the CSS file so a future stop edit that widens a ramp past 60° goes RED.

### 1.3 Motion tokens — CSS block + the TS mirror

CSS additions to the bebop block (names final per the direction's appendix):

```
--ease-tide:  cubic-bezier(0.37, 0, 0.63, 1);  /* sinusoidal state swell — status morphs, edge travel */
--dur-tide:   720ms;   /* one state-change crest */
--dur-breath: 3600ms;  /* idle breathing (formalizes bible §6's 3–4 s pilot light) */
--dur-drift:  48s;     /* alias of --dur-ambient — slowest atmosphere layer */
```

**TS mirror** in `packages/ui/src/lib/motion.ts` (the declared motion SSOT, header lines 3–13:
"never inline a raw cubic-bezier in a component"): add `ease.tide = [0.37, 0, 0.63, 1]` and
`duration.tide = 0.72`, `duration.breath = 3.6`, `duration.drift = 48`, so React/framer-motion
consumers and CSS consumers cannot drift apart. The Astro/Svelte world consumes the CSS custom
properties directly (§4) — the TS mirror is the React-interim shim, marked as such in a comment.

**Tempo doctrine as a check:** exactly four tempos exist — snap 180 ms / reveal 480·900 ms /
tide 720 ms / breath·drift 3.6 s·48 s. The §1.8 gate greps new/changed CSS in the skin +
signature sheets for duration literals and fails on any value not in the token set (a 350 ms
mush animation is a bug the same way an off-palette hex is — direction §2.2).

### 1.4 Layer, glow, grain — named recipes, not folklore

Three utility classes land next to the skin so no component ever re-invents atmosphere:

- **`.horizon-wash`** — the §2.3-stack wash: one bottom-anchored
  `radial-gradient(in oklch, --spectral-life-1 → -0 → transparent)` at `--wash-opacity`,
  breathing (background-position/opacity oscillation) at `--dur-breath`, drifting at
  `--dur-drift`. Composable under the existing grain `::after`.
- **`.spectral-edge`** — the signature-2 component (full spec §2.2).
- **`.focal-glow`** — the one sanctioned glow: warm radial bloom, core alpha ≤ 0.35,
  horizontal-biased (anamorphic; radial star-bursts stay banned, bible §8), never on/behind text.

Ceiling tokens, enforced by the gate: `--wash-opacity: 0.16` · `--edge-glow-w: 1.5px` ·
glow-core alpha ≤ 0.35. Restraint budget per view (one wash, one canvas, one focal glow, ≤1
backdrop-blur) is an acceptance-criteria line on every surface in §3 — checked in review
against the rendered page, since it is compositional, not lintable.

### 1.5 The status remap — `[data-skin="bebop"]`-scoped

The 10-state machine keeps its token *names* (`--status-*`, consumed by tracking + admin chips
today) and gains noir *values* inside the bebop block only — the `:root` values stay untouched
so the vendor storefront (which may show status chips in *its* room) is unaffected. Proposed
mapping, ramp-consistent (every chip keeps its icon + label — WCAG 1.4.1, bible color law 4):

| Status | Bebop value | Rationale (measured contrast on `--void`) |
|---|---|---|
| pending | `--amber` `#E8A544` | the lamp is lit, waiting — 8.95:1 ✓ |
| scheduled | `--taupe` `#BCB09C` | muted patience, no urgency — 8.89:1 ✓ |
| confirmed | `--teal` `#46B0A4` | the machine acknowledged — 7.24:1 ✓ |
| preparing | `--amber-deep` `#C68530` | warm work in progress — verify in gate (≈6.5:1 expected) |
| ready | `--teal` `#46B0A4` | ready-condense per the cloud vocabulary — 7.24:1 ✓ |
| picked-up | `--cyan` `#3FB7C4` | movement begins — ≈7:1, verify in gate |
| in-delivery | `--cyan` `#3FB7C4` | the traveling wave — same |
| delivered | `--gold` `#C79675` | the settle bloom's rest — 7.28:1 ✓ |
| rejected / cancelled | `--blood` `#E0543E` | 4.98:1 on `--void` — **large-text/icon-paired only**; on `--slate` it measures **3.82:1**, below AA-small — so rejected/cancelled body text stays `--bone` with a blood icon+chip, never blood paragraphs |

`--status-*-bg` companions re-derive as `rgba(<hue>, 0.14)` washes (under the 16% ceiling).
This mapping is *proposed-pending-gate*: §1.8's script verifies every pair at implementation
time and the two flagged rows either pass or get their darker/lighter twin adjusted **in the
token block, with the contrast note updated** — never ad hoc in a component.

### 1.6 Daylight counterparts — density replaces glow

Per direction §2.1 the ramps remap on `[data-daylight="true"]` to ink-anchored twins already in
the skin: life → `#A85F14`, alive → `#1F6E5A`, warn → `#B0201B`; rendered as density/weight
steps, not luminous haze; grain and glow off (already the case, `tokens.css:705–722`).
**Measured caution:** `#A85F14` on daylight paper `#EDE6D8` = **3.92:1** — passes non-text/
large-text AA (≥3:1) but **fails small-text AA**. The gate therefore classifies daylight amber
as *UI-component/large-only*; small text in daylight stays `#1A1712`/`#5A5044`
(14.39:1 / AA ✓). The daylight variant is re-verified as a separate gate matrix (direction §6.2).

### 1.7 Governance (existing rules, made checkable — no new process)

Canonical set = the bebop block + these additions; every token carries role + contrast comment
(bible §13, already the standing rule). What this blueprint adds is only **enforcement**: the
§1.8 script parses `tokens.css` itself — no duplicated hex lists anywhere — so the gate can
never drift from the source of truth.

### 1.8 VbM — the token layer's two gates (P1 deliverables, RED cases committed)

1. **Contrast-audit gate** — `packages/ui/src/theme/bebop.contrast.test.ts`, same harness
   pattern as the existing `palette.contrast.test.ts` (node:test + `contrastRatio`/`parseColor`
   from `theme/palette.ts`). It: (a) parses the `[data-skin="bebop"]` block + daylight block
   out of `tokens.css`; (b) composites **worst-case grain** (soft-light blend, `--grain-opacity`
   0.05, extreme noise pixel both directions, take the min ratio) before checking — the bible's
   "verify WITH grain applied" made deterministic; (c) asserts a declared classification matrix:
   small-text pairs ≥ 4.5:1, large-text ≥ 3:1, non-text (edges, chips, focus rings) ≥ 3:1
   (WCAG 1.4.11); (d) re-computes ramp hue spans ≤ 60°; (e) asserts `--wash-opacity` ≤ 0.16 and
   `--edge-glow-w` ≤ 1.5px.
   **RED cases (committed alongside, per VbM):** a fixture classifying `--spectral-life-1` as
   small-body-text fails (ramps are for light, never text — direction §2.1); `--blood` on
   `--slate` as small text fails (real: measured 3.82); bumping `--wash-opacity` to 0.20 fails.
   *This is exactly the operator's RED definition: a ramp used on text, or a wash > 16%.*
2. **Visual token snapshot** — a token **specimen page** (a Playwright-mounted fixture route
   rendering every ramp as wash + edge + glow, all 10 status chips, both motion curves as
   side-by-side animation strips, dark + daylight) asserted with `toHaveScreenshot` against a
   committed baseline. RED = any unreviewed pixel drift in the token layer; baseline updates
   are deliberate, reviewed diffs. (Playwright's built-in visual comparisons are the repo's
   existing proof tool — no new infrastructure.)

**Size cost of the whole token layer: ~1.0–1.5 kB of CSS** on the already-shipping skin sheet;
0 B JS. Critical-path budgets untouched by construction.

---

## 2. THE THREE SIGNATURES AS COMPONENTS

Design completion **must not depend on the particle cloud** — the direction's dead-simple test
(§2.5) requires every surface to be complete and beautiful with the atmosphere unplugged. So:
signatures 2 and 3 (CSS-only) are the **baseline** shipped by this plan; signature 1 slots in
when its own program is funded (D-PC1) without any surface redesign.

### 2.1 Signature 1 — `<ParticleCloud>` (spec by reference; do not re-plan)

The build is already planned and reviewed — `docs/design/particle-cloud-2026-07-11/PLAN.md`
(phases P1 owner-dashboard → P2 customer tilt-primary → P3 voice → P4 parked) with the
reviewer's corrections in force (tilt-primary touchless, push-to-talk voice, battery throttle,
context-restore, singleton canvas; REVIEW §6). This blueprint only fixes its **component
contract** so surfaces can reserve its place today:

- **Props:** `surface` (`'admin-board' | 'tracking' | 'landing-endcard'` — selects vocabulary
  subset + energy cap), `paletteAudience` (dark | daylight — feeds `core/palette.ts` verbatim
  Warm Cosmo-Noir stops), `interactive` policy (pointer/tilt flags per PLAN P2),
  `pointerEvents` per surface (none on admin, opt-in on tracking).
- **States** (exposed as `data-cloud-state` on the wrapper — the VbM observability hook from
  PLAN P1): `idle` (2 a.m. bar, energy ≈ 0.15, breath tempo) · `burst:<kind>` (transient,
  token-bucketed 1/1.5 s, ×N coalesced) · `mode:<sustained>` (derived from store state, never
  replayed events) · `degraded` (chroma drain along `--spectral-void`) · `frozen`
  (reduced-motion: `dt = 0`, events as palette/opacity crossfades, `change` listener live).
- **Fallback ladder:** no WebGL2 / no JS → the `.horizon-wash` renders in its place (signature
  3 is the terminus; nothing may exist with an ugly fallback — direction §6.1).
- **Hard rules:** one canvas per page, `webglcontextrestored` re-init, idle throttle
  ~24–30 fps, settle-to-static after N idle minutes, `visibilitychange` pause.
- **Size (measured/ratified):** core measured 3,594 B gz (REVIEW §1), realistic production core
  5–7 kB; **CI chunk gate ≤ 7,000 B gz** with the committed RED fixture (heavy-import turns it
  red) per PLAN P1. Inputs ≤ +1,500 B; camera "wave mode" separate opt-in lazy chunk ≤ 1,500 B.
  **RED = budget exceeded — the gate, not a reviewer's eye, says so.**

### 2.2 Signature 2 — `.spectral-edge` (CSS-only, the ubiquitous cousin)

The 1.5 px edge-light: one gradient line on a **single** container edge whose hue position
encodes sustained state and along which events travel as one soft wave. Feel reference: the
Siri/Apple Intelligence edge glow ([Riyam's reconstruction](https://rudrank.com/exploring-swiftui-creating-new-siri-animation)) —
ours warmer, slower, **never full-perimeter** (one edge; a frame is a costume).

- **Anatomy:** an `::after` (or dedicated element) of `block-size: var(--edge-glow-w)` pinned
  with **logical properties** (`inset-block-start`, `inset-inline: 0`) so RTL costs nothing
  later; background = `linear-gradient(to right in oklch, <ramp stops>)` sized `300% 100%`.
- **API (CSS component; host JS adds only class/data-attribute toggles):**
  `data-edge-state="life | alive | warn | void"` maps to `background-position` steps — the
  sustained-state hue posture; `.is-waving` runs a one-shot `background-position` pan
  (`--ease-tide` / `--dur-tide`), applied by the host on a state transition and removed on
  `animationend` — **one transition, one crest** by construction.
- **Reduced motion:** the wave declaration collapses to a color crossfade
  (`transition: background-position 0.01ms, filter …`) — state stays fully legible as palette;
  the media query is honored live (it's CSS).
- **A11y:** decorative — `aria-hidden`; the state it echoes always also exists as text/badge
  (ambient never the sole signal). At 1.5 px it is exempt from text contrast; as a non-text
  state indicator it still passes ≥3:1 by the §1.8 matrix.
- **Where:** console header (sustained room-state), tracking progress rail, in-app badge
  one-shot shimmer, email header rule (static gradient image fallback).
- **Cost:** ~0.4–0.6 kB CSS, **0 B JS** beyond one `classList` line per host.
  **VbM:** Playwright asserts computed `block-size` = 1.5px (RED if fattened); under
  `page.emulateMedia({ reducedMotion: 'reduce' })` asserts no `background-position` animation
  runs while `data-edge-state` changes still recolor; the specimen snapshot (§1.8) covers hue
  postures.

### 2.3 Signature 3 — `.horizon-wash` (the resting state)

The landing's `lp-sky` god-lit line (built: `landing.css:44–50`) generalized into the canonical
backdrop for empty states, loading gates, login, and 404:

- **Anatomy:** one bottom-anchored radial `--spectral-life` gradient (`in oklch`) at
  ≤ `--wash-opacity`, on `--void`/`--hull`, breathing at `--dur-breath`, drifting at
  `--dur-drift`; grain rides above per the existing layer; the Horizon Drift *ship* appears
  only where it already lives (landing, admin login) — everywhere else the wash alone says
  dowiz (direction §3.3).
- **Reduced motion:** frozen to the composed still (already the skin's law).
- **Mounts:** `EmptyState fullPage` (`Status.tsx:14`) gains an `ambient` prop rendering the
  wash behind its copy; `LoginPage`, 404 (`main.tsx:93`), loading gates.
- **Cost:** ~0.3 kB CSS, 0 B JS.
  **VbM:** gate asserts composed alpha ≤ 0.16 (parse of the utility's declaration — the
  operator's RED case); specimen snapshot covers it visually.

### 2.4 Budget roll-up

| Item | Cost | Rides critical path? |
|---|---|---|
| Token layer + status remap + recipes (§1) | ~1.0–1.5 kB CSS | on the skin sheet (already shipped) — no JS |
| `.spectral-edge` + `.horizon-wash` | ~0.7–0.9 kB CSS | same — no JS |
| `<ParticleCloud>` chunk | ≤ 7,000 B gz JS, lazy | **never** — post-hydration/idle only |
| Inputs / camera / voice chunks | ≤ 1,500 / 1,500 / 4,000×2 B gz, lazy, gated | never |

Astro route classes 25/35/60 kB gz (G05 ratified) untouched: the CSS-only signatures add zero
JS; the particle chunk's budget accounting awaits the FE-0.1 signature (assume it counts until
signed — PLAN §1, D-PC2).

---

## 3. PER-SURFACE DESIGN COMPLETION (sequenced by value)

**Sequence:** owner/admin console + customer tracking first — the console is where the product
is *worked* and tracking is where dowiz's reliability is *watched*; together they are the
entire first-client demo path (place order → run the pass → watch it arrive). The
notification/system-state kit is cross-cutting and rides P2. Courier next (a working tool with
a smaller inventory). Landing last — it is already the strongest surface and is outreach, not
product. Storefront: excluded, §6.4.

### 3.1 Owner/admin console — "the terminal that breathes" (P2)

**Component inventory to design (all against existing components — extend, don't create):**

| Component | Design work |
|---|---|
| Console shell + header (`AdminRoutes.tsx`) | bebop skin on; `.spectral-edge` on the header's top edge carrying sustained room-state (life = orders flowing · warn = dwell aging · void = degraded); nav in condensed-caps register |
| `DashboardPage` orders board | `.data-surface` audit (no grain/wash/blur over the board — verify the opt-out actually covers every table/form/chart); new orders enter with **one** jazz-in reveal (480 ms) then hold — no reflow-jump; burst load coalesces (×N), the board stays metronome-steady |
| Stats row / KPI tiles | **retire the money tween**: `DashboardPage.tsx:451` revenue and `AnalyticsPage.tsx:265` money cards cut to exact values; the tile *container* may take one 180 ms snap of acknowledgment; `AnimatedNumber` survives (if at all) only for non-money, non-ID decorative counts — and nothing on a `.data-surface` qualifies |
| `OrderCard` states | status chips on the §1.5 remap; dwell-aging = rust edge tint + label (never color-only); one saturated accent per view respected |
| `EmptyState` | `.horizon-wash` + canonical line ("Quiet night. Nothing on the pass yet."), particles-lowest-idle when the cloud exists |
| `Toast` / `ConfirmDialog` / `ResponsiveDialog` | enter jazz-in reveal, hold still, exit ease-in; destructive confirms in the plain stakes register |
| `LoginPage` (admin) | horizon wash + (existing) Horizon Drift subtle variant per bible §7 |
| `CommandPalette`, `SearchInput`, forms | bebop focus ring (`--amber` 2px, already in skin), mono for IDs, condensed eyebrows |
| `WSStatusDot` + degradation | wire the `--spectral-void` desaturation vocabulary: WS degraded ⇒ ambient layers drain chroma one step + plain line; **truthfulness rule** — desaturation only on genuinely degraded health (REVIEW §6.3: GPU context-loss must restore, not masquerade) |

**Ambient treatment:** particle canvas behind the orders board at low energy **when funded**
(D-PC1); until then the console's ambience is the spectral edge + wash-on-empty only — and must
already feel complete (dead-simple test).

**Trust cues (standing, checkable):** money never tweens (0.3 fixed); totals update in place;
one-transition-one-crest on every status morph; lists never reflow-jump; errors per §3.3.

**Acceptance criteria (each falsifiable):**
- [ ] Playwright: admin shell computed `background-color` = `#12100E` and `data-skin="bebop"`
      present (RED today — the skin is dormant).
- [ ] Playwright: inject 10 synthetic `order.created` in 1 s ⇒ board gains 10 rows with
      exactly one reveal animation batch and **CLS < 0.02** during the burst (layout-shift
      observer assertion) — calm-under-rush, measured.
- [ ] grep/CI: zero `AnimatedNumber` usage co-located with `formatMoney`/`PriceDisplay` money
      values (RED today at `DashboardPage.tsx:451`, `AnalyticsPage.tsx:265`).
- [ ] Visual: dashboard, empty dashboard, login re-baselined; reduced-motion emulation run
      shows identical layout with zero animation.
- [ ] `.data-surface` audit checklist per admin page (grain/wash/blur absent over data) — one
      Playwright computed-style assertion per page class.

### 3.2 Customer order-tracking — the flagship ambient moment (P2)

The one long scene with the end customer (20–40 min). Foundations verified (§0.6): honest ETA
range, real-machine stepper, SR announcements. Design work:

| Component | Design work |
|---|---|
| Status ladder / stepper (`OrderStatusPage.tsx:594`) | full path always visible, current step lit (the machine's plan, not just its present); step colors from the §1.5 remap; per-status hue posture on the ambient layer |
| Transition moment | **one tide swell** per committed transition: wash + edge recolor over `--ease-tide`/`--dur-tide`, fired from the same state-change the stepper consumes; never before truth is committed (no optimistic shimmer); missed transitions reconcile silently to final state (the page already refetches — `OrderStatusPage.tsx:284`) |
| Progress rail | `.spectral-edge` along the rail; `IN_DELIVERY` = the traveling wave (edge pan biased by courier bearing when the cloud lands; CSS pan direction from geometry, not reading direction — §6.3) |
| `DELIVERED` | settle bloom once (wash flares to `--spectral-settle`, decays at breath tempo), then rest |
| `REJECTED`/`CANCELLED` | one step of desaturation + slow-fall; message in the plain stakes register; the atmosphere grieves quietly, never dramatizes |
| ETA display | stays a mono range (server-derived) — **never** a single fake-precise minute; digits cut, never tween (`ClientLayout.tsx:245` cart-total tween also retired — see §6.4 note on why this trust fix crosses the storefront boundary legitimately) |
| Loading | honest skeleton mirroring the exact ladder+map+receipt layout, shimmer at breath tempo |
| Map | `.data-surface` (no grain over maplibre — pattern exists for paper at `tokens.css:488`, port it to bebop) |
| Tenant identity | venue name/items/logo on the content layer untouched; the *room* (status light, ambient, motion, trust cues) speaks dowiz's semantic ramps so status reads identically under every tenant theme (direction §4.0 handoff moment) |

**Battery honesty** (when the cloud lands): idle throttle, settle-to-static,
`visibilitychange` pause — PLAN P2 carries these; the CSS-only baseline has no battery cost.

**Acceptance criteria:**
- [ ] Playwright vs staging: drive a fixture order through `CONFIRMED → PREPARING → READY`;
      assert exactly one `.is-waving` crest per transition (animation-event counter), zero
      crests on WS reconnect replay (RED case: replaying 3 missed events must NOT fire 3 waves).
- [ ] ETA renders as `NN–MM min` mono range for non-terminal statuses (regex assertion);
      no digit-tween (rAF hook assertion or absence of `AnimatedNumber`).
- [ ] Reduced-motion emulation: status change ⇒ color/label update only, stepper still fully
      legible (screenshot diff of before/after states differs — proving state is visible
      without motion).
- [ ] Skeleton snapshot matches loaded-layout geometry (bounding-box comparison, tolerance
      ±8 px) — "honest skeletons," measured.
- [ ] sq/en/uk: ladder labels unclipped at 360 px viewport (no horizontal scroll assertion).

### 3.3 Notifications + empty/loading/error kit (cross-cutting, rides P2)

- **`Toast`:** jazz-in enter, still hold, ease-in exit; bursts coalesce ("6 new orders", never
  six stacked toasts — same 1/1.5 s doctrine); `role="alert"`/live regions remain the
  authoritative channel (already the pattern at `OrderStatusPage.tsx:529`).
- **Badges:** one-time `.spectral-edge` shimmer on arrival; never continuous pulse (a
  permanently animated badge is nagging — aggregator behavior).
- **`SkeletonBase`:** shimmer re-timed to breath tempo (never a fast sweep); consumers must
  mirror real layout shape+count (acceptance per surface).
- **`EmptyState`:** `ambient` wash variant (§2.3) + canonical dry-wit lines from the bible §10
  library, adapted sq/uk in-tone.
- **`ErrorHandle` molecule (net-new — §0.5):** the error-contract made visible: machine `code`
  + first-8-chars `correlationId` in mono inside a "report this problem" affordance
  (copy-to-clipboard). Visual: one desaturation step on the surface, one `--blood` accent
  with icon, still composition. Voice per stakes rule: money/auth plain and
  identical-in-clarity sq/en/uk; brand moments may keep the wit. Spec source:
  `docs/design/error-contract-parse-token-economy/resolution.md`.
- **Acceptance:** Playwright forces a 500 on a staging fixture route ⇒ handle renders, is
  8 chars, matches the response's `correlationId` (RED if the wiring is cosmetic); reduced
  motion ⇒ no shake/flood anywhere (there is none by design — assert zero animations on the
  error container).

### 3.4 Courier app — daylight, density, one-handed (P3)

| Component | Design work |
|---|---|
| Shell (`CourierRoutes.tsx`) | skin flip + `data-daylight="true"` default via existing `SunlightToggle`; grain/glow off (already in skin); ramps as ink-density (§1.6) |
| `TasksPage` offer card | the sanctioned moment: order glyph + **honest countdown ring** (the accept window rendered truthfully — ring maps 1:1 to remaining time, mono seconds; spectral-edge condense as the cheap version, particle condense when funded) |
| `ShiftPage` close | perimeter ring quietly dispersing, once |
| `DeliveryPage`, `EarningsPage` | money mono + exact-cut (no tweens); buttons sized for thumbs on a moving bike — hit targets ≥ 48 px, `StickyActionBar`/`BottomTabBar` audit |
| Motion budget | snaps + one tide morph per state change; **no drift loops while riding** |

**Acceptance:** contrast gate daylight matrix green (incl. the §1.6 amber classification);
Playwright daylight screenshot set at 360 px; countdown-ring truth test (mock a 20 s window ⇒
ring reaches zero at 20 s ± 1 frame; RED: a decorative ring that loops); zero ambient washes
on any courier working screen.

### 3.5 Landing / marketing — polish only (P4)

Already live and canon. Work: convert `lp-sky`/`lp-nebula` gradients to `in oklch` with
`--spectral-life` stops (`landing.css:44–50,119`; visually near-identical, midpoints richer);
fold the hero nebula into the disciplined §2.3 stack (one wash + grain); hold atmosphere
≤ ~20% of frame (the concert-hall exception); the end-card particle condense (wordmark, once,
on scroll-arrival) is **deferred to D-PC1** — the end card must be complete without it.
Money/CTA copy stays plain (voice law 7).
**Acceptance:** landing visual re-baseline; gradient declarations contain `in oklch` (grep
gate); reduced-motion still shows the composed hero still; Lighthouse perf unchanged ±2 pts.

---

## 4. REACT-NOW vs ASTRO-TARGET

**The design ships on the live React app first.** That is what a first client sees: React
serves humans on staging + prod (`CUTOVER_ASTRO_UPSTREAM` unset — G05 §1); the Astro rebuild is
3/27 islands and arbiter-contingent beyond FE-0. Designing for Astro-first would target vapor
(the particle plan reached the same verdict — PLAN §1).

**Authored once, served twice — the layering that makes the port cheap:**

| Artifact | Lives in | React consumes | Astro/Svelte consumes | Class |
|---|---|---|---|---|
| Tokens, ramps, status remap, recipes (§1) | `packages/ui/src/theme/tokens.css` | via skin attr on shells | same stylesheet, same custom properties — zero port | **neutral (author once)** |
| `.spectral-edge`, `.horizon-wash` (§2.2–2.3) | skin-adjacent CSS | class + one `classList` toggle | class + one Svelte `class:` binding | **neutral** |
| Particle cloud core/vocab/store | `packages/particle-cloud` (vanilla TS, zero deps — PLAN module table) | `store/react-adapter.ts` (`useSyncExternalStore`) | thin Svelte-runes adapter + `client:idle` island (PLAN P2b) | **neutral core, per-framework mounts** |
| Motion TS mirror (`motion.ts` additions) | `packages/ui/src/lib/motion.ts` | framer-motion variants | **not ported** — Svelte uses CSS transitions on the same custom properties | **React-interim** |
| `AnimatedNumber` retirement, shell skin flips, per-page treatments (§3) | `apps/web` | direct | re-expressed per island during G05 FE-1/FE-4/FE-6/FE-7 ports — the *specs* (§3 acceptance criteria) are the portable artifact | **React-interim implementation, portable acceptance criteria** |
| Playwright acceptance specs | e2e suite | vs React staging | same specs re-pointed at Astro staging (G05's dual-oracle wiring, FE-0.3) | **neutral by design** |

**Interim vs target markers:** anything expressed as CSS custom properties, skin-scoped CSS, or
vanilla-TS package is **target-grade now** (survives the rebuild untouched). Anything importing
`framer-motion` or React APIs is **interim** and must carry a `// react-interim:` note pointing
at its CSS-token equivalent, so the FE-6/FE-7 porter never re-derives design intent.
**Budget note per world:** on React admin (~234 kB gz context, G05 §2.2) the ambient CSS+chunk
is noise; on Astro the 25/35/60 kB classes bind and the FE-0.1 signature (D-PC2) decides how
lazy decoration counts — until signed, the particle island is NO-GO on Astro (P2b posture,
PLAN §1). The CSS-only signatures are exempt by construction: 0 B JS.

---

## 5. PHASED PLAN — quality gates, effort, definitions of done

Effort unit = sessions (the project's unit). Every phase ships its RED case with its GREEN
(VbM). No phase starts before its predecessor's "stable" checklist is fully checked — that is
the quality-first finish line, per phase.

### P1 — Token + signature foundation, and the skin goes live (2–3 sessions)

| Deliverable | Detail |
|---|---|
| Skin flip (bible Pass-2, blast radius §14 already mapped) | `paperSkinAttr()` → bebop equivalent on Admin/Courier shells + 404 + Privacy; Paper block + helpers deleted; fonts already allowlisted (HANDOFF) |
| Token layer (§1 complete) | ramps, motion tokens + TS mirror, recipes, status remap, daylight matrix |
| Signatures 2+3 CSS (§2.2–2.3) | `.spectral-edge`, `.horizon-wash`, mounted nowhere yet except specimen |
| Gates | contrast-audit test (with grain compositing + hue-span + ceilings + RED cases) · token specimen `toHaveScreenshot` baseline · tempo-literal grep gate |

**VbM/visual proof:** contrast gate green with 3 committed RED fixtures (§1.8); specimen
baseline committed; admin/courier Playwright visual nets re-baselined (NEEDS-REBASE is by
design, G05 §5); staging screenshots of flipped admin + courier attached to the PR.
**RED case class:** ramp-as-text fixture fails · wash 0.20 fails · a duration literal `350ms`
in the skin sheet fails.
**Definition of stable/done:** [ ] all four shells wear bebop on staging · [ ] zero `paper`
references left (`grep -r 'data-skin="paper"\|paperSkin'` = 0, test files updated) · [ ] gates
green + REDs proven · [ ] no visual regression on the landing (it already wore the skin) ·
[ ] typecheck/build/e2e green vs staging.

### P2 — Owner/admin console + customer tracking + the system-state kit (3–4 sessions)

Deliverables = §3.1 + §3.2 + §3.3 inventories, including the money-tween retirement (3 cited
call sites) and the net-new `ErrorHandle` molecule.
**VbM/visual proof:** the acceptance checklists of §3.1/§3.2/§3.3 — each line is a Playwright
assertion or CI grep, run vs staging (`VITE_BASE_URL=…`, `--workers=1` per the standing
rate-limit note). Burst-calm CLS < 0.02 measured. One-crest-per-transition counter with its
replay RED twin.
**RED case class:** money tween reintroduced (grep gate) · two crests on one transition ·
skeleton geometry diverges from loaded layout.
**Definition of stable/done:** [ ] all §3.1/§3.2/§3.3 boxes checked with pasted proof ·
[ ] reduced-motion full-suite pass (Playwright `reducedMotion: 'reduce'` project run) ·
[ ] sq/en/uk screenshot set clean at 360/768/1280 px · [ ] error handle verified against a
real staging `correlationId`.

### P3 — Courier (1–2 sessions)

Deliverables = §3.4. **VbM:** daylight contrast matrix green; countdown-ring truth test with
RED; 360 px daylight visual set. **Done:** §3.4 boxes checked; zero washes/grain on working
courier screens (computed-style assertions).

### P4 — Landing polish (1 session)

Deliverables = §3.5. **VbM:** `in oklch` grep gate green (RED: an sRGB gradient in `lp-*`
classes); visual re-baseline; Lighthouse delta ≤ 2. **Done:** §3.5 boxes checked.

### P5 — Astro port (2–3 sessions, **contingent**)

Re-expression of §3 treatments on the Astro islands as they land in G05 FE-1/FE-4 (storefront
shell is excluded chrome-wise; tracking + admin islands carry the treatments). Gated on: the
arbiter funding G05 FE-1+ · the FE-0.1 budget signature (D-PC2) for any JS-bearing ambient ·
G05's own per-phase parity oracles. The token/CSS layer ports for free (§4); the work is
mounts + re-pointed acceptance specs. **VbM:** the same §3 acceptance specs green against
Astro staging (dual-oracle, G05 FE-0.3); route budget gates 25/35/60 with the committed RED
fixture (G05 §3.1). **Done:** identical checklist as P2/P3, Astro-pointed.

### Particle-cloud program (parallel track, separately funded)

Slots in per its own PLAN (P1 admin → P2 tracking) after **D-PC1**, earliest Wave-4-parallel;
its size gates (7,000 B chunk + RED fixture) and Playwright `data-cloud-state` proofs are
already specified there. **This blueprint's P1–P4 do not wait for it and do not include its
sessions.** When it lands, it mounts into places §3 already reserved (behind the orders board;
tracking ambient layer; landing end-card).

### Effort summary

| Phase | Sessions | Gate to start |
|---|---|---|
| P1 foundation + flip | 2–3 | none (design program go) |
| P2 console + tracking + kit | 3–4 | P1 stable |
| P3 courier | 1–2 | P2 stable |
| P4 landing polish | 1 | P2 stable (parallelizable with P3) |
| **Stable-to-send bar (§5.7)** | **7–10 total** | P1–P4 checklists |
| P5 Astro port | 2–3 | arbiter + FE-0.1 + G05 phase oracles |
| Particle cloud | 4–6 (own plan) | D-PC1 |

### 5.7 "DESIGN IS STABLE ENOUGH TO SEND" — the outreach bar

The operator sends remote vendor outreach when **every** box below is checked. Each is
mechanically checkable; any unchecked box = not stable; any gate that cannot go RED does not
count (VbM).

- [ ] **Skin live everywhere:** all dowiz surfaces wear `data-skin="bebop"` on prod; zero
      `paper` remnants (grep = 0).
- [ ] **Token gates green, REDs committed:** contrast-audit (with grain compositing, daylight
      matrix, hue-span ≤ 60°, wash ≤ 16%, edge ≤ 1.5 px) + token specimen snapshot — two
      consecutive green CI runs.
- [ ] **Money never tweens — proven:** CI grep gate green (no `AnimatedNumber` on money paths);
      the three §0.3 call sites fixed; `PriceDisplay`/`formatMoney` remains the only money
      render path.
- [ ] **One transition, one crest — proven:** the P2 crest-counter spec green including its
      replay RED twin.
- [ ] **All §3 acceptance checklists** (console, tracking, kit, courier, landing) checked with
      pasted Playwright proof vs staging.
- [ ] **Reduced-motion suite pass:** every surface, `reducedMotion: 'reduce'` project — layout
      identical, state changes legible as color+glyph+label.
- [ ] **Trilingual visual pass:** sq/en/uk screenshot set at 360/768/1280 px — no clipped
      labels, no horizontal scroll (assertion, not eyeball).
- [ ] **Error contract visible:** a forced staging error renders code + 8-char handle matching
      the response `correlationId`.
- [ ] **Calm under rush — measured:** 10-events-in-1 s dashboard burst ⇒ CLS < 0.02, one
      coalesced reveal.
- [ ] **Storefront zero-diff:** Playwright screenshots of `/s/<demo-slug>` (menu, cart,
      checkout) byte-identical before/after the entire brand program — the sovereignty
      boundary as a falsifiable gate (§6.4). Any diff = RED = the aesthetic leaked.
- [ ] **Budgets green:** customer critical path unchanged (~21.6 kB gz Astro-side; React
      bundle delta ≤ +2 kB CSS/0 JS from this program); particle chunk gate green **or**
      particle program not yet funded (both are stable states).
- [ ] **The demo path dry-run:** landing → console login → place a test order → track it to
      DELIVERED, screen-recorded on staging, on a mid-tier Android profile, 60 fps ambient or
      layers reduced — operator watches it once and signs.
- [ ] **The taste check** (bible §13, direction §6.5): the operator answers yes twice —
      beautiful at 2 a.m., honest at 8 p.m. Friday. (The only non-mechanical line, and it is
      deliberately the operator's, not an agent's.)

---

## 6. GUARDRAILS (binding numbers, restated once)

### 6.1 Budgets
Customer-surface critical-path JS stays ~21.6 kB gz; Astro route classes **25/35/60 kB gz**
(G05 ratified) with the committed RED fixture; ambient stack ≈ 8–11 kB gz lazy total —
particle chunk **≤ 7,000 B gz** (CI gate), inputs **≤ +1,500 B**, camera **≤ 1,500 B** opt-in,
voice **≤ 4,000 B ×2**, all lazy; deferred-chunk accounting awaits FE-0.1 (assume it counts).
This program's own additions: **≤ ~2.5 kB CSS, 0 B critical-path JS.** Frame discipline: 60 fps
mid-tier Android or reduce layers, never framerate; one WebGL canvas per page; context-restore
mandatory; degradation terminus = CSS wash + static grain (nothing ships with an ugly fallback).

### 6.2 Accessibility
Reduced motion = color/opacity only, durations 0.01 ms (non-zero so `animationend` fires),
sim `dt=0`, `change` events honored live. **The ambient is never the sole signal** — every
visualized event also exists as notification/badge/ARIA text; glyph pairs with color
(WCAG 1.4.1). Flash ceiling **≤ 1 burst / 1.5 s** as a stated WCAG 2.3.1 invariant — "make it
punchier" bounces off this line. Contrast: every pair AA (small 4.5 / large 3.0 / non-text 3.0
per WCAG 1.4.11) **with grain composited**, on the actual surface step; daylight matrix
verified separately; `--ash` never on small body; glows carry solid-outline fallbacks.

### 6.3 i18n — sq / en / uk
Condensed-caps labels get width headroom (uk runs long; sq diacritics need line-height);
buttons/chips size on `ch`/content, never hard px. Logical properties for all chrome offsets
(HUD corners, edge travel); hue + glyph carry state, never "leftward"; directional streams
derive from geometry/bearing, not reading direction. Dry wit adapted, not translated;
money/error strings plain and identical-in-clarity across all three.

### 6.4 Storefront sovereignty — the hard rule
**If the pixel renders inside `/s/:slug` menu/cart/checkout content, the dowiz aesthetic does
not apply. Ever.** No wash, no particles, no spectral edge, no grain, no bebop type — tenant
theme only, however plain. Mechanically: skin tokens stay non-overridable-and-non-leaking (the
tenant lock cuts both ways); the `:root` `--status-*` values stay untouched by the §1.5 remap.
Culturally: no "just a subtle dowiz shimmer" on the vendor's stage. **Enforced falsifiably**
by the §5.7 storefront zero-diff gate.
One precise clarification so the boundary is applied correctly, not over-applied: the
sovereignty exclusion covers **brand aesthetic**, not product correctness — retiring the cart
total's money tween (`ClientLayout.tsx:245`) imposes no dowiz styling; it removes a dishonest
number animation, and the trust rules (§5.1 money discipline, honest loading, calm errors) are
product-wide. The handoff moment stands: order placed ⇒ the tracking *room* is dowiz's; the
vendor's identity stays on the content layer.

---

## References that drove concrete choices here
OKLCH gradient interpolation and sRGB dead zones — [Comeau](https://www.joshwcomeau.com/css/make-beautiful-gradients/),
[Grant](https://keithjgrant.com/posts/2023/11/problematic-color-gradients-and-workarounds/)
(drives the stop-token + compose-with-`in oklch` architecture, §1.2). Siri/Apple Intelligence
edge glow — [Riyam](https://rudrank.com/exploring-swiftui-creating-new-siri-animation) (drives
the one-edge, hue-position-as-state spectral edge, §2.2). Calm technology (Weiser & Brown;
Amber Case) — periphery↔center doctrine for the ambient layer (§2.1, §3.1). WCAG 1.4.1 /
1.4.11 / 2.3.1 — the non-text 3:1 classification in the contrast gate and the flash-ceiling
invariant (§1.8, §6.2). In-repo: `INTERFACE-DIRECTION-2026-07-11.md`, `BRAND-BIBLE.md` §6–§14,
`particle-cloud-2026-07-11/{PLAN,REVIEW}.md`, `G05-astro-fe-parity.md`, and the tree state
audited in §0.

*Written 2026-07-11 by a read-only design-planning session. The only file created is this
blueprint; working tree and branches left exactly as found.*
