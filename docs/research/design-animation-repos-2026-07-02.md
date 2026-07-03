# Design / animation OSS candidates — decision doc (2026-07-02)

**STATUS: RESEARCH / SHORTLIST. None adopted.** 20 design + animation OSS projects assessed against
dowiz's actual surfaces (storefront `/s/:slug`, admin, courier, the three.js landing). Ranked by fit and
by *net-new value given what's already installed* — not by popularity. Adoption of any one is a separate
decision, subject to the `no-arbitrary-tailwind` lint, `.size-limit.json` bundle budget, and
reduced-motion gates.

---

## 0. What's already in the tree (grounds everything below)
- **`framer-motion@^12.40.0`** in BOTH `apps/web` and `packages/ui` — **33 imports in `packages/ui/src`**.
  framer-motion v12 *is* Motion (same library, `motiondivision/motion`). **The motion engine is already
  chosen.** → Tier A is mostly "don't add a second engine"; the only Tier-A *action* is a bundle
  optimization (LazyMotion, below).
- **`three@^0.184.0`** in `apps/web` — the landing `NomadicScene`/`PaperScene` is hand-rolled three.js.
  r3f/drei/theatre (Tier E) are *ergonomics migrations*, not new capability.
- **Not installed** (genuine net-new): radix, vaul, sonner, embla, lenis, auto-animate, number-flow,
  tailwindcss-motion, lottie, magicui, motion-primitives.

**Branch context:** `chore/design-system-prune` just deleted bespoke `Drawer`, `BottomSheet`, `Modal`,
`Tooltip`, `ToastManager`, `SwipeableRow`, `PullToRefresh`. The "what replaces the pruned molecule"
question is answered by **Tier C** — that's where the real value is.

Legend: **✅ ADD** (net-new, high fit) · **🔁 SKIP-redundant** (you already have the capability) ·
**🕓 DEFER** (real but trigger-gated) · **➖ SKIP** (low fit for this app).

---

## Tier A — motion engines (engine ALREADY chosen: Motion)
1. **[motiondivision/motion](https://github.com/motiondivision/motion)** — MIT. Already installed as
   `framer-motion@12`. Full bundle ~30–46KB gzip, **but `LazyMotion` + the `m` component drop the initial
   payload to ~4.6KB** and lazy-load features. **Action (not an add): 🔁** migrate the 33 `motion.*`
   imports in `packages/ui` to `m` + `LazyMotion` to cut storefront bundle — this is a `perf` task, not a
   new dependency. Gotcha: `m` only animates features you register in the `features` prop; a stray
   `motion.*` import defeats the tree-shake.
2. **[greensock/GSAP](https://github.com/greensock/GSAP)** — ➖ **SKIP.** Core ~23KB + ScrollTrigger ~7KB.
   Genuinely better for scroll-scrubbed/pinned landing sequences, **but**: (a) it's a *second* engine on
   top of Motion (bundle + mental overhead), and (b) it's closed-source, Webflow-owned, and its license
   bars use "in any tool that competes with Webflow" — a needless legal question for a commercial product
   when Motion's scroll API already covers our needs. Only revisit if the landing needs timeline scrubbing
   Motion can't do.
3. **[pmndrs/react-spring](https://github.com/pmndrs/react-spring)** — 🔁 SKIP-redundant. Spring physics
   (~29k★). Overlaps Motion (which has springs). No reason to run both.
4. **[juliangarnier/anime](https://github.com/juliangarnier/anime)** — ➖ SKIP. Lightweight JS/SVG (~65k★).
   Framework-agnostic; only useful for a standalone SVG/logo animation outside React, which we don't have.
5. **[formkit/auto-animate](https://github.com/formkit/auto-animate)** — ✅ **ADD (low-risk).** ~3KB, MIT,
   one hook (`useAutoAnimate`). Zero-config enter/leave/reorder transitions. **Surface:** menu item lists,
   cart line add/remove, admin order queue reordering, courier job list. Complements Motion (it's for the
   "I just want lists to not jump" cases where hand-writing `AnimatePresence` is overkill). Gotcha:
   respects `prefers-reduced-motion` automatically — good, but confirm against our reduced-motion gate.

## Tier B — copy-paste animated component libraries (landing / marketing polish)
6. **[magicuidesign/magicui](https://github.com/magicuidesign/magicui)** — 🕓 DEFER. MIT, 150+ animated
   components on React/TS/Tailwind/Motion. Copy-paste (not an npm lock-in), so zero adoption cost — pull
   *individual* components into the landing/claim-preview when a specific effect is wanted. Don't
   "install" it wholesale.
7. **[ibelick/motion-primitives](https://github.com/ibelick/motion-primitives)** — 🕓 DEFER. MIT,
   copy-paste animated primitives built on Motion (which we have). Same model as Magic UI — cherry-pick.
8. **[birobirobiro/awesome-shadcn-ui](https://github.com/birobirobiro/awesome-shadcn-ui)** — ➖ (not a dep)
   — discovery index only. Useful bookmark if we move toward a shadcn-style system.

## Tier C — headless primitives (REPLACE the just-deleted bespoke molecules) ← the real value
9. **[emilkowalski/vaul](https://github.com/emilkowalski/vaul)** — ✅ **ADD (highest fit).** MIT, actively
   maintained (v1.1.x). Accessible drawer/bottom-sheet with drag-to-dismiss + snap points. **Directly
   replaces the deleted `Drawer` + `BottomSheet`.** Already conceptually endorsed (the `emil-design-eng`
   skill is installed). **Surface:** mobile storefront cart/checkout bottom-sheet, filter drawers. Gotcha:
   built on Radix Dialog under the hood → pairs naturally with #11.
10. **[emilkowalski/sonner](https://github.com/emilkowalski/sonner)** — ✅ **ADD.** MIT, tiny. Opinionated
    toaster with stacking/swipe/promise states. **Directly replaces the deleted `ToastManager`.**
    **Surface:** order-placed / error / "added to cart" feedback across all three apps. Gotcha: single
    `<Toaster />` at app root; our i18n (al/en) strings pass through as content — verify RTL/long-string.
11. **[radix-ui/primitives](https://github.com/radix-ui/primitives)** — ✅ **ADD.** MIT, per-primitive
    packages (tree-shakes; `@radix-ui/react-slot` alone is ~131M weekly downloads). 30+ WAI-ARIA-correct
    unstyled primitives. **Replaces the deleted `Modal` (Dialog) + `Tooltip`** with real focus-trap /
    keyboard / ARIA instead of hand-rolled a11y. Style with our Tailwind tokens; animate with Motion.
    **Caveat:** acquired by WorkOS; update velocity has slowed on complex components. If that worries you,
    the alternatives are **Base UI** (same team's successor) or **Ark UI** (state-machine based,
    cross-framework) — but Radix is the safe default given shadcn ubiquity. Pick ONE headless base.
12. **[davidjerleke/embla-carousel](https://github.com/davidjerleke/embla-carousel)** — ✅ **ADD (if we
    want galleries).** MIT, ~5KB, no deps, accessible, touch/drag. **Surface:** menu-item photo galleries,
    storefront hero. Only add when a carousel is actually designed in — not speculatively.

## Tier D — scroll & Tailwind motion utilities
13. **[darkroomengineering/lenis](https://github.com/darkroomengineering/lenis)** — 🕓 DEFER (landing
    only). MIT, small. Smooth scroll that *wraps* native scroll so `position: sticky`, anchors, and a11y
    keep working. **Surface:** landing / long marketing pages. **Do NOT put on the app shell** — the
    storefront uses an `IntersectionObserver` scroll-spy on `.app-shell-main`; hijacking scroll there risks
    breaking the category anchors and mobile momentum. Landing-scoped or skip.
14. **[romboHQ/tailwindcss-motion](https://github.com/romboHQ/tailwindcss-motion)** — 🕓 DEFER. MIT,
    Tailwind plugin, preset animations via class syntax, reduced-motion aware. Tempting for low-friction
    motion, **but** it partially overlaps Motion and adds a Tailwind-config surface the `no-arbitrary-
    tailwind` gate must tolerate. Only if designers want to author motion in markup without touching Motion.
15. **[barvian/number-flow](https://github.com/barvian/number-flow)** — ✅ **ADD (targeted).** MIT, small,
    React component for animated number transitions with locale-aware formatting. **Surface:** prices,
    order totals, ETA/prep-time counters, admin KPI tiles. Genuinely nice for money/time that changes
    in place. Gotcha: must respect our integer-money convention (format from integer cents, animate the
    display only).

## Tier E — WebGL / 3D (three.js ALREADY installed for the landing)
16. **[mrdoob/three.js](https://github.com/mrdoob/three.js)** — 🔁 already installed (`three@0.184`).
17. **[pmndrs/react-three-fiber](https://github.com/pmndrs/react-three-fiber)** — 🕓 DEFER. MIT. React
    renderer for three.js. Would make the landing scene declarative/maintainable, **but** it's a rewrite
    of a working hand-rolled scene — do it only if the landing scene becomes a maintenance burden or grows.
18. **[pmndrs/drei](https://github.com/pmndrs/drei)** — 🕓 DEFER. MIT. r3f helpers (loaders/controls/
    effects). Only meaningful *after* an r3f migration (#17).
19. **[theatre-js/theatre](https://github.com/theatre-js/theatre)** — 🕓 DEFER. Apache-2.0. Timeline
    sequencing with an r3f extension. Nice for a choreographed landing hero; premature until #17.

## Tier F — vector/asset animation
20. **[airbnb/lottie-web](https://github.com/airbnb/lottie-web)** — 🕓 DEFER (watch weight). MIT.
    After-Effects JSON animations. **Surface:** empty/success/error states (order placed, empty cart,
    courier assigned). **Gotcha: lottie-web is heavy (~250KB+).** If adopted, use `@lottiefiles/dotlottie-
    react` (lighter, compressed `.lottie`) and lazy-load it, or a Motion/SVG animation instead for simple
    states. Only worth it for genuinely complex illustrated moments.

_Honorable mentions (outside the 20): [tsparticles](https://github.com/tsparticles/tsparticles) (particle
FX — too heavy for this app), [shadcn-ui/ui](https://github.com/shadcn-ui/ui) (component base if we adopt
Radix + move off bespoke molecules), Base UI / Ark UI / React Aria (alternatives to Radix at #11)._

---

## Recommendation — sequenced

**Adopt now (closes the prune, low risk, high fit):**
1. **vaul** → Drawer/BottomSheet · **sonner** → ToastManager · **radix-ui/primitives** → Modal/Tooltip.
   These replace deleted bespoke components with accessible, maintained ones. This is the group that pays
   for itself immediately and is on-theme with the current prune branch.
2. **@formkit/auto-animate** (~3KB) for list transitions — trivial, high perceived polish.
3. **number-flow** for price/ETA/total counters — targeted, respects integer-money.

**Do as a `perf` task (not an add):**
- Migrate the 33 `framer-motion` imports in `packages/ui` to `LazyMotion` + `m` to cut initial bundle
  (~46KB → ~4.6KB initial). Biggest bundle win here, zero new dependency.

**Defer (trigger-gated): everything else.** magicui/motion-primitives = cherry-pick copy-paste when a
specific effect is designed; lenis/r3f/drei/theatre/lottie = landing-scoped and only when the landing
work justifies it; embla = when a gallery is actually designed.

**Skip (redundant / low-fit):** GSAP (2nd engine + Webflow license clause), react-spring, anime.js.

**Constraints on all of the above:** one motion engine (Motion), one headless base (Radix or its successor
— don't mix Radix + Ark + Headless UI), and every add must pass `.size-limit.json` + the reduced-motion
and `no-arbitrary-tailwind` gates before merge.
