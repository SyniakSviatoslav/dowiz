# Cinematic Brand UX — reveals, micro-interactions & a per-vendor brand-personality model

> Design-time plan. No code, no commit, **no `package.json` edit**. Author: research+plan lane,
> 2026-07-02. Storefront-first, flag-dark, visual-regression-gated.
>
> Scope split from siblings: **`cinematic-product-media/`** owns the rich *asset* pipeline
> (video / 360-spin / 3D behind `MEDIA_RICH_ENABLED`, lazy on modal open → `ProductMedia[]`).
> **`icon-system/`** owns the lucide swap. **This plan owns MOTION + BRAND FEEL**: how a dish
> *reveals* itself (card→detail, scroll choreography, image parallax), how the storefront feels
> *alive* (add-to-cart delight, state transitions), and the **per-vendor brand-personality model**
> that makes each `/s/:slug` feel bespoke. It composes with both siblings and reuses their seams.

## 0. TL;DR

Three deliverables, all riding the **already-provisioned `LazyMotion` + `m`** motion runtime (no new
motion dep) and the **`derivePalette` → CSS-var → `ThemeProvider`** brand seam that already exists:

1. **Cinematic menu reveals** — a `layoutId` shared-element **card→detail** morph (the ProductCard
   image + title + price fly into the detail sheet instead of a disconnected bottom-sheet pop),
   scroll-choreographed category reveals (upgrade the existing `whileInView` stagger into an
   intentional, budgeted cascade), and a tasteful **image zoom/parallax** on the detail hero. Each
   maps to `motion.ts`/`tokens.css` tokens, honours `prefers-reduced-motion`, and carries an explicit
   perf budget.
2. **Alive-not-gratuitous micro-interactions** — an add-to-cart "fly-to-cart" + cart-badge count
   spring, a success path-draw check on confirm, and calmer state transitions (availability, price
   reconcile, empty/loading→content). Motion only ever *signals* interaction or state — never idles.
3. **Per-vendor brand-personality model** — extend `derivePalette`/theme so a vendor's brand drives
   not just palette + fonts (already done) but a **`brandPersonality`** with four dialable axes:
   **motion intensity**, **corner-radius/shape**, **density**, and **elevation/depth**. One derived
   `BrandPersonality` object → CSS vars (`--motion-scale`, `--radius-scale`, `--density-scale`,
   `--elev-scale`) that every reveal/micro-interaction reads, so ArtePasta (elegant Italian) and
   Eljo's (bold pizzeria) feel like *different products*, not one template reskinned.

All three ship storefront-first, behind flags, gated by the visual-regression net + Playwright on a
real staging `/s/:slug`. Smallest first increment = the **card→detail shared-element on ProductCard**
(§7), which needs zero schema, zero new dep, and no brand-model work.

---

## 1. Ground truth — what exists today (read, verified)

### Motion runtime (the reuse target)
- `packages/ui/src/theme/ThemeProvider.tsx` mounts **one** `LazyMotion features={domAnimation}`
  (framer-motion) at the app root — `m.*` components are cheap (~4.6 KB `m` + lazy features vs ~46 KB
  full `motion`). `packages/ui` was just migrated to `m`; **apps/web still uses full `motion.*`**
  (MenuPage imports `{ motion, AnimatePresence, useReducedMotion }` directly). No `strict` flag, by
  design, so both coexist. **Constraint: any new storefront motion reuses this — no `gsap`/`lottie`/
  `@react-spring`/`motion` v2.**
- `packages/ui/src/lib/motion.ts` = the motion SoT (Emil-calibrated): `ease` (out/inOut/soft/bounce),
  `spring` (press/enter/bounce/gentle), `duration` (instant .08 / fast .15 / base .24 / slow .4), and
  a full variant library — including `cardEntry`, `modalIn`, `overlayIn`, `pulseDot`, `staggerChildren`,
  `scalePress/scaleTap`. **Rule already codified in the file header:** entrances use `ease.out`; UI
  durations < 0.3s; bounce is reserved for rare delight; every consumer honours reduced-motion.
- `packages/ui/src/theme/tokens.css` mirrors motion as CSS vars (`--motion-instant/fast/base/slow`,
  `--ease-out/in-out/soft`) and — critically — a `@media (prefers-reduced-motion: reduce)` block that
  **zeroes every `--motion-*` duration** (state preserved). CSS-var-driven animation is reduced-motion-
  safe automatically; `m` variants gate via `useReducedMotion()` at the call site.

### Brand seam (what makes each vendor unique today)
- `packages/ui/src/theme/palette.ts` → `derivePalette(input)` computes a full, contrast-safe token
  set from minimal tenant input (often just a primary + bg), luminance-aware, WCAG-AA-guaranteed. Pure,
  runs server + client.
- `packages/ui/src/theme/fonts.ts` → `FONT_ALLOWLIST` (13 ids), `fontPairingForCuisine(cuisine)`,
  `fontStack`, `googleFontsHref` (egress-safe). Per-tenant heading/body fonts are **already shipped**
  (mig 084, `location_themes.heading_font/body_font`; memory `storefront-per-tenant-fonts`).
- `ThemeProvider` writes the derived tokens onto `document.documentElement` as `--brand-*` vars. The
  demo-builder (`scripts/demo-builder.mjs`, memory `demo-builder-loop`) already seeds palette + fonts
  per prospect at provision time via `fontPairingForCuisine` + a cuisine palette map.
- **So the personality seam already exists** — palette + fonts. This plan adds *motion + shape +
  density + depth* to the same derived object and the same CSS-var pipeline. **No new bridge.**

### The two surfaces this plan touches
- **`packages/ui/src/components/client/ProductCard.tsx`** (`m.article`) — already animates on
  `m`: `cardVariants` (rest/hover/tap lift, hover gated behind `(hover: hover)`), `imgVariants`
  (hover `scale 1.04`), `addBtnVariants`. Title uses `--brand-font-heading`, description
  `--brand-font-body`, price `--brand-primary-readable`. **This is where `layoutId` gets attached.**
- **`apps/web/src/pages/client/MenuPage.tsx`** (1,769 lines, top churn hotspot) — owns the grid
  render (`whileInView` section + card stagger, ~L1124–1226) and the detail modal (`AnimatePresence` +
  bottom-sheet `motion.div`, ~L1272–1676). Modal already: locks scroll, Escape-closes, sticky dismiss,
  full-image hero (`object-contain`), lazy `MediaGallery`/`MediaRenderer` for rich media, a crafted
  no-photo brand-gradient fallback, staged content rise (`delay: 0.1`), price pop (`delay: 0.2`),
  `tactileAdd()` haptic on add. `handleProductClick` opens; `closeDetail` closes.

### Discipline this plan must satisfy (CLAUDE.md + memories)
- Mandatory Proof: any UI surface → Playwright on `https://dowiz-staging.fly.dev/s/:slug` with real
  `toBeVisible`/`toContainText` assertions. Visual-regression net (memory `visual-regression-net`) is
  the primary gate for pure-motion/appearance change.
- Ship discipline: commit (feature branch) → staging deploy → validate. Flag-dark; launching is a
  separate explicit act.
- `don't-conflict-utilize` / `subtractive-first` / `license-first` (tooling grammar) — reuse `m`, add
  no dep, and where possible *replace* ad-hoc motion with tokenised motion (net-neutral or negative).

---

## 2. Deliverable 1 — Cinematic menu reveals

Three reveals, ranked by impact-per-risk. Each: what, how (tokens), reduced-motion, perf budget.

### 1a. Card → detail **shared-element** morph  ⭐ (the flagship; also the smallest increment, §7)

**What.** Today tapping a card triggers a disconnected effect: the card stays put while a bottom-sheet
slides up from `translateY(28px) scale(0.97)`. Replace with a **FLIP shared-element transition** — the
tapped card's **image**, **title**, and **price** animate *from their on-grid position/size into their
place in the detail sheet*, so the dish visibly *becomes* the detail view. This is the single highest-
impact "cinematic" upgrade and the pattern premium apps use (card expands into full detail).

**How (framer-motion `layoutId`, reuses the mounted `LazyMotion`).**
- Give the shared nodes a stable `layoutId` keyed by product id: `media-${id}`, `title-${id}`,
  `price-${id}`. Put them on `m.*` elements in **both** ProductCard and the modal hero/heading.
- **Critical correctness constraint (from research):** the outgoing (card) and incoming (modal)
  elements sharing a `layoutId` MUST live inside the **same `AnimatePresence` boundary**. The modal is
  already wrapped in `AnimatePresence` at MenuPage L1272; the grid is not. Either (a) hoist a single
  `AnimatePresence` around both grid + modal, or (b) — lower-risk, preferred — use the `layout`+
  `layoutId` pair which does NOT require shared `AnimatePresence` for the *morph* when both are mounted,
  and let the card node stay mounted (it does — the grid doesn't unmount on open). Prototype both in
  §7; pick by measured smoothness.
- Motion performs layout animation via **`transform` only** (GPU-cheap, per research). Pass a
  `layoutDependency` (the open product id) so measurement runs only on open/close, not every render —
  important on this 50-item hotspot page.
- Transition: `spring.gentle` for the morph (the modal already uses a spring); `ease.out` +
  `duration.base` for the crossfade of non-shared content. Scale the spring stiffness by the vendor's
  `--motion-scale` (§4).

**Reduced-motion.** `useReducedMotion()` → skip the layout morph entirely; fall back to the current
opacity-only modal open (already implemented as the `prefersReduced` branch). No FLIP, no transform.

**Perf budget.** Morph ≤ 350 ms; only `transform`/`opacity` animated (no layout thrash, no
`box-shadow` in the keyframe path — animate a pseudo/overlay for shadow if needed). Zero added JS to
the initial bundle (framer already loaded). Target 60fps on a mid Android (the storefront's real
device class). Guard: no `layout` on the *list container* (would measure all 50 cards) — only on the
3 shared leaf nodes.

**Interplay with rich media.** The modal hero may be a lazy `MediaGallery`/`MediaRenderer`
(`cinematic-product-media`). The `layoutId` morph targets the **poster/primary image** node; when rich
media resolves after open, it crossfades in *inside* the settled hero (media load is already async and
best-effort). The morph never blocks on media.

### 1b. Scroll-choreographed category reveals

**What.** The grid already does `whileInView` (section fade + `staggerChildren: 0.03` card cascade,
`viewport once`). Upgrade from "fade up 6px" to an intentional, *choreographed* reveal: category
heading leads, its cards cascade under it with a slightly longer stagger and a subtle scale, so each
section "arrives" as a unit as you scroll — the scrollytelling feel, scaled way down for a utility menu.

**How.** Reuse `staggerChildren` from `motion.ts` (don't re-author). Heading: `slideUp` +
`ease.out`. Cards: `cardEntry` variant (already exists: y16 + scale .97 → 0, `duration.base`,
`ease.out` — Emil-calibrated NOT bounce). Stagger step and travel distance scale by `--motion-scale`
(§4): an elegant vendor gets a slower, longer cascade; a bold one gets a snappier, tighter one.
Keep `viewport={{ once: true }}` — a menu must never re-animate on scroll-back (that reads as broken).

**Reduced-motion.** Already handled — the existing `prefersReduced ? false` initial + zeroed CSS
durations. Keep: reveal becomes an instant appear.

**Perf budget.** `once: true` means each node animates exactly once → bounded cost. Cap concurrent
in-view stagger to the cards actually intersecting (IntersectionObserver `margin` already tuned). No
scroll-linked `useScroll` on the list (avoid a per-frame handler over 50 items); reveals are
*trigger-on-enter*, not *scrubbed*. (Scroll-scrubbed motion is reserved for 1c's single hero only.)

### 1c. Detail-hero image zoom / parallax

**What.** In the detail sheet, give the hero photo a slow, subtle **Ken-Burns-style zoom** on open
(scale 1 → 1.06 over the sheet's settle) and an optional **parallax**: as the sheet body scrolls, the
hero drifts at a slower rate than the content, adding depth. Elegant vendors get more drift; bold get
near-zero (crisp, punchy).

**How.** Zoom: an `m.img`/`m.div` variant `scale: [1, 1.06]` over `duration.slow` × `--motion-scale`,
`ease.out`, running once on open. Parallax: `useScroll({ container: sheetRef })` + `useTransform` to
map the sheet's scrollY → a small hero `y` (e.g. 0 → −24px × `--motion-scale`). Scoped to the **one**
hero node inside the **one** open sheet — never the page. Amount driven by `--parallax-depth` (from
`--elev-scale`/personality, §4).

**Reduced-motion.** Both OFF under reduced motion (no zoom, no scroll-transform) — static hero.
Also gate parallax OFF on `(hover: none)` low-end if a cheap perf signal warrants (start on, measure).

**Perf budget.** One `useScroll` handler bound to the sheet container (not window), `transform`-only
output, throttled by rAF (framer does this). Zoom is a one-shot GPU transform. Disable both when a rich
`<video>`/spin renderer is active (don't stack a transform on a decoding video). Budget: no measurable
main-thread cost while idle; parallax handler only live while the sheet is open.

---

## 3. Deliverable 2 — Alive-not-gratuitous micro-interactions

Research-backed constraints held throughout: **200–400 ms**, distinct+consistent (same interaction for
same action class), quick+light, no decorative idling, `trigger→rules→feedback→loop` structure.

| Interaction | Trigger | Motion | Token | Notes / why |
|-------------|---------|--------|-------|-------------|
| **Add-to-cart fly** | tap "+" on card or modal | a ghost of the product image/price arcs from the button to the cart icon, then the **cart badge count springs** (scale 1→1.25→1) | `duration.base` arc (`ease.out`), `spring.bounce` on the badge | The single most-recommended commerce micro-interaction ("product glides into cart"). Reuses the existing `bounceCart()` + `tactileAdd()` haptic already wired in MenuPage. Ghost element is one absolutely-positioned `m.div`, removed on animation end. |
| **Add confirm (no-modifier fast-add)** | successful add | button "+" briefly morphs to a **path-draw check** then back | `duration.fast` draw, `ease.out` | Reuses the vendored success path-draw from `icon-system` §4 (`AnimatedCheck` already exists as an inline svg). Signals success without a toast-only confirm. |
| **Cart bar entrance** | first item added | cart/checkout bar rises with `slideUp` + settle | `spring.enter` | Already partially present; tokenise + scale by `--motion-scale`. |
| **Availability / price-reconcile** | menu reconcile toast (already fires) | the affected card does a one-shot soft `pulseDot`-style tint pulse | reuse `pulseDot` (ONE cycle, not `Infinity`) | Makes the existing silent reconcile *visible* on the item, not just a toast. |
| **Empty/loading → content** | skeleton → data | skeleton crossfades to the real grid (not a hard swap); `MIN_SKELETON_DWELL` already prevents flash | `overlayIn`/`fadeIn`, `duration.fast` | Calm content reveal; pairs with 1b's stagger. |
| **State chips (open/closed/busy)** | venue status change | chip color/label crossfade, no jump | `duration.fast`, `ease.soft` | `StateChip` already rendered; add a crossfade on state change only. |

**Non-goals (explicit anti-slop):** no infinite idle wiggle, no parallax on every element, no
attention-seeking bounce on frequent entries (Emil rule, already in `motion.ts`), no motion that
delays a tap's effect (the add happens immediately; the animation is *feedback*, never a gate — the
research "roadblock" failure mode).

All of the above degrade to instant/no-op under `useReducedMotion()` + the zeroed CSS durations.

---

## 4. Deliverable 3 — The per-vendor brand-personality model

**Goal:** each `/s/:slug` should feel *bespoke*, not one template reskinned. Palette + fonts already
vary per vendor; add **motion, shape, density, depth** to the same derived object and same CSS-var
pipeline. One brand → one `BrandPersonality` → four scalar CSS vars → every reveal/micro-interaction
and every component reads them. Zero per-component branching.

### The model (extend `palette.ts` / theme, alongside `derivePalette`)

```ts
// packages/ui/src/theme/personality.ts  (new, pure — like palette.ts)
export type BrandVibe = 'elegant' | 'bold' | 'playful' | 'minimal' | 'warm';

export interface BrandPersonality {
  vibe: BrandVibe;
  motionScale: number;   // 0.6 (snappy/minimal) … 1.0 (default) … 1.4 (expressive/elegant slow)
  radiusScale: number;   // 0.4 (sharp/editorial) … 1.0 … 1.6 (soft/rounded/playful)
  densityScale: number;  // 0.85 (airy/fine-dining) … 1.0 … 1.15 (dense/fast-food value)
  elevScale: number;     // 0.3 (flat/matte) … 1.0 … 1.5 (deep shadow/parallax)
}
```

**Derivation — cheapest-sufficient, layered (mirrors the font 3-tier model):**
- **T0 cuisine default** (always available): a `VIBE_FOR_CUISINE` map, exactly like
  `CUISINE_FONT_PAIRINGS` — `finedining/sushi → elegant`, `burger/fastfood/pizzeria → bold`,
  `cafe/bakery → warm`, `street → playful`, unknown → a neutral default (`motion 1.0, radius 1.0,
  density 1.0, elev 1.0`). This ships value on day one for every existing tenant with zero data.
- **T1 signal-derived** (opportunistic): the demo-builder brand-extractor already parses a vendor's
  site for fonts/colours; extend it to nudge the vibe — a serif-heavy, high-whitespace, muted-palette
  site → `elegant`; a saturated, chunky, sans site → `bold`. Advisory only; T0 stands if no signal.
- **T2 owner override** (admin BrandingPage): a single **vibe picker** (5 presets) with a live
  preview, exactly like the existing font `<Select>`. Presets set all four scalars; an "advanced"
  disclosure can expose the raw sliders later (YAGNI for v1 — ship the 5 presets).

**Storage.** Additive columns on `location_themes` (like `heading_font`/`body_font` in mig 084):
`brand_vibe text` (nullable → cuisine default). Store the *vibe id* only (bounded charset, injection-
safe), derive the 4 scalars from it client+server via `personalityForVibe(vibe)` — never store raw
numbers over the wire (same discipline as fonts: store an id, resolve to values). One forward-only
migration; **package.json/migrations are protect-paths → staged for the operator** (§6).

**Application (the seam — pure CSS vars, no JS bridge, like colours/fonts).** `derivePalette`'s
output object gains the 4 scalars (or a sibling `derivePersonality`); `ThemeProvider` writes them:

```
--motion-scale:  <n>;   /* multiplies every reveal/micro-interaction duration & travel */
--radius-scale:  <n>;   /* multiplies --brand-radius / --brand-radius-sm / --brand-radius-btn */
--density-scale: <n>;   /* multiplies card gap / padding tokens */
--elev-scale:    <n>;   /* multiplies --elev-* shadow spread + --parallax-depth */
```

- **Motion:** `m` variants read the scalar via a tiny helper (`d * motionScale`), and CSS keyframes/
  transitions multiply `--motion-* * --motion-scale`. Reduced-motion still zeroes at the CSS layer,
  so personality never overrides accessibility.
- **Shape:** re-derive `--brand-radius* = base * --radius-scale`. ProductCard/`rounded-xl` become
  `calc()` off the scaled token. An `elegant` vendor gets sharper editorial corners; `playful` gets
  pill-round.
- **Density:** grid `gap`, card padding read `calc(base * --density-scale)`.
- **Depth:** `--elev-*` and hero `--parallax-depth` scale — a `bold` vendor gets punchy shadows +
  parallax; a `minimal` one goes flat/matte.

**Guardrails (non-negotiable, red-line-adjacent since it touches every tenant's public surface):**
- **Contrast/AA is invariant** — personality NEVER touches colour derivation; `derivePalette`'s
  AA-guarantee stands untouched. Radius/density/motion cannot break contrast.
- **Tap-target floor invariant** — `densityScale` may shrink *padding/gaps* but a guardrail clamps
  every interactive element to `--tap-min` (44px). Add a red→green test: a max-density vendor still
  has ≥44px targets on `/s/:slug`.
- **Reduced-motion wins** — `motionScale` is multiplied *before* the reduced-motion zero, so reduced
  motion always flattens to instant regardless of vibe.
- **Bounded scalars** — clamp each scalar to its documented range in `personalityForVibe` so a bad
  value can't produce a broken layout.

**Demo-builder integration.** `scripts/demo-builder.mjs` already seeds palette+fonts at provision;
add `brand_vibe` from `vibeForCuisine(cuisine)` to the theme upsert (the L2 palette layer). Then a new
prospect demo is bespoke in *feel*, not just colour — strengthening the sales pitch the preview banner
already makes.

---

## 5. Phased build — storefront-first, flag-dark, visual-regression-gated

Each phase is a shippable increment with its own flag + proof. Flags (Vite env, like the existing
`VITE_MENU_CHARACTERISTICS_*`): `VITE_CINEMATIC_REVEALS`, `VITE_BRAND_PERSONALITY` (default OFF).

| Phase | Scope | Flag | Proof gate |
|-------|-------|------|-----------|
| **P0 (lead, this doc)** | design + tokens plan; no code | — | this file |
| **P1 · card→detail shared-element** ⭐ | `layoutId` on ProductCard (media/title/price) + MenuPage modal; reduced-motion fallback kept | `VITE_CINEMATIC_REVEALS` | §7 — visual-regression baseline on `/s/:slug` open/close + Playwright morph assertion + 60fps trace on mid-Android profile |
| **P2 · scroll reveals + micro-interactions** | upgrade grid `whileInView` choreography; add-to-cart fly + badge spring + confirm check; reconcile pulse; content crossfade | `VITE_CINEMATIC_REVEALS` | visual-regression net (states × 3bp × 2lang) + Playwright: card visible after scroll, cart badge increments, reduced-motion path |
| **P3 · detail hero zoom/parallax** | Ken-Burns + sheet parallax, scoped to open sheet; disabled over active video/spin | `VITE_CINEMATIC_REVEALS` | perf trace (one scroll handler, transform-only), reduced-motion off, no-jank on rich-media modal |
| **P4 · brand-personality model** | `personality.ts` + `personalityForVibe` + T0 cuisine map; `ThemeProvider` writes 4 scalars; components read scaled tokens; tap-target clamp guardrail | `VITE_BRAND_PERSONALITY` | red→green tap-target test; 2-vendor visual diff (ArtePasta elegant vs Eljo's bold look distinct); AA unchanged |
| **P5 · owner vibe picker + persistence** | `location_themes.brand_vibe` migration (operator-staged), admin BrandingPage picker, `/public/theme/:slug` returns vibe, demo-builder seeds it | `VITE_BRAND_PERSONALITY` | migration on staging DB first (ship discipline); owner-set vibe renders on `/s/:slug`; demo-builder dry-run |

**Sequencing rule.** `packages/ui` shared atoms (`ProductCard`, `motion.ts`, `personality.ts`,
`tokens.css`, `ThemeProvider`) land before the `apps/web` MenuPage wiring that depends on them — same
lead-before-fanout discipline as the icon plan. P1–P3 are motion-only (no schema, no dep). P4–P5 add
the one migration (protect-path).

**Collision note.** MenuPage is the top-churn hotspot and is concurrently touched by the icon
migration (L3 lane) and rich-media. **One edit per turn**, lead-serialised on this file; do reveals
after (or coordinated with) the icon swap to avoid two lanes rewriting the same JSX.

---

## 6. Dependency / protect-path posture

- **Zero new runtime dependency.** All motion reuses the mounted `LazyMotion`+`m`. `layoutId`,
  `useScroll`, `useTransform`, `useReducedMotion` are all framer-motion APIs already installed. This
  satisfies `don't-conflict-utilize` + `license-first` (nothing new to license) by construction.
- **`package.json` untouched** → no `new-dep-scan.mjs` run needed.
- **Migrations are a protect-path** — the P5 `location_themes.brand_vibe` migration is **staged for
  the operator**, not written here. Additive, forward-only, `IF NOT EXISTS`/`ADD COLUMN … DEFAULT
  NULL` (metadata-only, no rewrite), and it must **not** silently no-op on the owner picker (use the
  SET-when-provided CASE pattern that fixed the font columns, not COALESCE — memory
  `storefront-per-tenant-fonts`).
- **Bundle proof.** P1–P3 add ~0 KB (framer already in bundle); confirm no bundle-size regression in
  the visual-regression/CI check. Personality is pure CSS vars + a tiny map — negligible.

---

## 7. Smallest first increment (do this first): card→detail shared-element on ProductCard

The minimum viable cinematic upgrade — no schema, no dep, no brand model, one visible "wow":

1. **ProductCard** (`packages/ui/src/components/client/ProductCard.tsx`): add `layoutId={media-${id}}`
   to the `m.img` (and the no-photo medallion), `layoutId={title-${id}}` to the `<h3>` (make it
   `m.h3`), `layoutId={price-${id}}` to the `PriceDisplay` wrapper. Accept an optional
   `sharedLayout?: boolean` prop so the layoutIds are only attached on the storefront path (keeps the
   card reusable elsewhere without accidental morphs). Gate all of it behind `useReducedMotion()`.
2. **MenuPage modal** (`apps/web/src/pages/client/MenuPage.tsx` ~L1321 hero, L1407 title, L1420
   price): put the matching `layoutId`s on the modal's hero image node, `<h2>` title, and price. Keep
   the existing `prefersReduced` opacity-only branch as the fallback.
3. Ensure the shared nodes resolve inside a common animation context — prototype (a) hoisting one
   `AnimatePresence` around grid+modal vs (b) relying on both-mounted `layout`; pick by measured
   smoothness (research: they must share an `AnimatePresence` boundary when one side unmounts — here
   the card stays mounted, so (b) is likely enough and lower-risk).
4. Pass `layoutDependency={detailProduct?.id}` so the 50-card grid doesn't re-measure every render.
5. **Proof:** `e2e/tests/` spec on staging `/s/:slug` — tap a card, assert the modal hero + title +
   price become visible with the card's content (`toContainText` the same name/price), assert
   `closeDetail` returns; a reduced-motion run asserts the modal still opens (opacity path). Add a
   visual-regression baseline for the open state. Paste the Playwright result (Mandatory Proof).

**Why first:** highest cinematic impact per line, reuses everything, fully reversible behind
`useReducedMotion` + a prop, and it establishes the `layoutId` naming convention that 1c and the brand
model build on. It also composes cleanly with rich media (morph targets the poster; media crossfades
in after).

---

## 8. Sources

- Framer Motion / Motion — layout & shared-element (`layoutId`, FLIP, transform-only, `layoutDependency`,
  shared `AnimatePresence` boundary): <https://motion.dev/docs/react-layout-animations>,
  <https://blog.maximeheckel.com/posts/framer-motion-layout-animations/>,
  <https://www.framer.com/motion/component>
- Scroll-driven / cinematic reveal patterns (scrollytelling 2.0, parallax, Apple-style product reveals,
  Tokopedia CPU 50%→2% via native scroll-driven anim, `once`-trigger vs scrubbed):
  <https://developer.chrome.google.cn/blog/css-ui-ecommerce-sda>,
  <https://scroll-driven-animations.style/>,
  <https://medium.com/@axtriqdesign/top-ui-animations-for-2025-where-motion-meets-meaning-0d06630bc5d9>
- Add-to-cart / commerce micro-interactions (200–400ms, glide-to-cart, scale-down+drop, consistency,
  avoid roadblock): <https://www.toptal.com/designers/animators/ux-microinteractions-e-commerce-design>,
  <https://www.kensium.com/blog/micro-animating-the-purchase-path-to-boost-conversions>,
  <https://www.mobiloud.com/blog/micro-interactions-mobile-apps>
- Brand-personality motion (easing as "vibe", dial intensity up/down per brand, motion as identity):
  <https://www.designsystems.com/5-steps-for-including-motion-design-in-your-system/>,
  <https://uianimation.medium.com/motion-design-systems-the-future-of-scalable-brand-motion-c28c00942e69>,
  <https://www.everything.design/blog/motion-brand-guidelines>
- Repo grounding: `packages/ui/src/components/client/ProductCard.tsx`, `apps/web/src/pages/client/MenuPage.tsx`,
  `packages/ui/src/lib/motion.ts`, `packages/ui/src/theme/{palette.ts,fonts.ts,tokens.css,ThemeProvider.tsx}`,
  `docs/design/cinematic-product-media/` (rich-media sibling), `docs/design/icon-system/plan.md`,
  memories `storefront-per-tenant-fonts`, `storefront-branding-overhaul`, `demo-builder-loop`,
  `visual-regression-net`.
```
