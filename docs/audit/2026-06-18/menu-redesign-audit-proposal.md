# Public menu — impeccable audit + redesign proposal

**Surface:** `/s/:slug` storefront menu (`apps/api/src/lib/ssr-renderer.ts`)
**Brief (confirmed):** premium / appetite-forward · boutique-restaurant feel · themeable per-restaurant template.
**Method:** `impeccable audit` (5-dimension scored scan) + `taste-skill`/brand-register slop test, against the live deployed page.

---

## Anti-Patterns Verdict — **FAIL the brand slop test**
Would someone say "AI made that"? **Yes, without hesitation.** Tells present: flat dark **card grid**, **empty grey image placeholder boxes** (160px of dead space per dish), **Inter** (reflex-default font), washed-out muted body text, **zero motion**, "cards everywhere." It reads as a generic dark delivery-app template — the opposite of "a boutique restaurant's own page."

## Audit Health Score

| # | Dimension | Score | Key finding |
|---|-----------|-------|-------------|
| 1 | Accessibility | **2/4** | no `:focus-visible`, sub-44px targets, no reduced-motion |
| 2 | Performance | **3/4** | no `<img>` dims (CLS), no `content-visibility` on a 31-category page |
| 3 | Theming | **1/4** | **menu hardcodes `#121212`/`#1e1e1e`/`#ea4f16` — ignores the `--brand-*` tokens, so it does NOT re-skin per tenant** |
| 4 | Responsive | **3/4** | works, but sub-44px targets + sparse 800px desktop |
| 5 | Anti-Patterns | **1/4** | heavy AI aesthetic (4+ tells) |
| **Total** | | **10/20** | **Acceptable → Poor (significant overhaul)** |

## Detailed findings (by severity)

- **[P0] Menu ignores the theme tokens** — `ssr-renderer.ts:356-392` hardcodes the dark palette in its inline `<style>`. The cart/checkout shells use `var(--brand-*)` + load the per-location `theme.css`; the **menu does not** → a "themeable per-restaurant template" that doesn't actually theme. *Category: Theming. → `$impeccable colorize`.*
- **[P1] Empty-image dead space** — `ssr-renderer.ts:197,359`: 160px grey gradient box when a dish has no photo (≈the whole demo). Cards are ~70% empty. *Anti-pattern + layout. → `$impeccable delight` (crafted fallback) + `$impeccable layout`.*
- **[P1] No motion / no reduced-motion** — no entrances, no add-to-cart feedback beyond a CSS bounce; no `prefers-reduced-motion`. *A11y + craft. → `$impeccable animate`.*
- **[P1] Sub-44px targets + no focus** — add button 32×32, lang links 35×30; no `:focus-visible`. *A11y/WCAG 2.5.5 + 2.4.7. → `$impeccable harden`.*
- **[P1] Reflex font (Inter) + flat hierarchy** — no display face, no pairing intent; everything one geometric sans. *Anti-pattern. → `$impeccable typeset`.*
- **[P2] 31-category wall, no navigation** — long scroll, no sticky category nav/jump. *Layout. → `$impeccable layout`.*
- **[P2] Sparse desktop** — 800px centered, ~1 column, big dead margins. *Responsive. → `$impeccable layout`.*
- **[P2] `theme-color` mismatch** (`#ea4f16` vs `#121212` bg); **[P3]** generic "Menu not available" empty state.

## Positive findings (keep)
Clean semantic headings (h1/h2/h3); `loading="lazy"`; responsive `auto-fill` grid; no horizontal scroll; the JSON-LD + meta are correct (this session's fixes); a real per-location token system already exists (`--brand-*`, presets Crimson/Ocean/Midnight) — the menu just isn't using it.

---

## Redesign proposal — "the restaurant's own page"

**Aesthetic lane:** a boutique restaurant's bespoke ordering page — warm, editorial, food-led. *Not* a delivery-app card grid. A competitor's one-liner for the current page would be "a dark delivery-app menu" — which fits the modal category, so we restart from there.

### 1. Make it actually themeable (P0)
Drive every menu color/spacing/radius/font through `--brand-*` tokens (extend the set: `--brand-surface-raised`, `--brand-border`, `--brand-text-muted`, `--brand-radius-*`, `--brand-font-heading/-body`). One restaurant in Crimson, another in Ocean — same layout, different soul. **This is the single highest-leverage change** and unlocks the "per-restaurant" promise.

### 2. Food is the hero — kill the grey box
- **With a photo:** large, warm, appetizing imagery; subtle gradient scrim for legible overlaid text.
- **Without a photo (the common case):** a *crafted* fallback, never a dead box — a branded tile (dish initial / monogram on a `--brand-surface` wash with a faint food-texture or the restaurant's accent), at a smaller, intentional height. Better still for image-less menus: switch to **editorial list rows** (name · price · short description · `+`) — denser, calmer, no wasted space. *"Cards are the lazy answer."*

### 3. Typography with a point of view
Pair on a contrast axis: a **characterful display** for the restaurant name + category headers (warmth/appetite — chosen per brand via `--brand-font-heading`, default a humanist/old-style serif or a warm grotesque — **not** Inter), with a clean **humanist sans** for body/prices. Tabular-nums on prices. `text-wrap: balance` on headings. Letter-spacing floor −0.04em on display.

### 4. Motion as part of the build
Staggered category/dish reveals on scroll (each reveal fits what it reveals, not one uniform reflex); a real add-to-cart micro-interaction (item → flies to the cart FAB; FAB counts up with an ease-out-expo pop); FAB entrance. **Every animation gets a `prefers-reduced-motion` crossfade/instant variant.** Library: `motion` (Framer Motion) for the SPA surfaces; CSS for the SSR menu.

### 5. Layout & navigation
- **Sticky category chip-nav** for long menus (the demo has 31 categories) — jump + active-state.
- **Mobile:** bottom-anchored "View cart · {total}" bar (44px+), one-handed.
- **Desktop:** wider, denser grid (or a 2-pane: menu + sticky cart summary) — no 800px dead margins.

### 6. A11y baseline (folds in)
≥44px targets, `:focus-visible` rings, `prefers-reduced-motion`, `color-scheme: dark`, `theme-color` = bg, explicit `<img>` dims. Target WCAG 2.2 AA.

### 7. Honest, on-brand states
Empty menu, sold-out dish, load failure — each crafted and warm, in the restaurant's voice (and localized). No bare "Error loading menu."

---

## Recommended command sequence (impeccable)
1. **[P0] `$impeccable colorize`** — token-ify the menu; make it re-skin per tenant.
2. **[P1] `$impeccable layout`** — food-hero cards/rows, category nav, desktop density, bottom cart bar.
3. **[P1] `$impeccable typeset`** — display+body pairing via `--brand-font-*`.
4. **[P1] `$impeccable delight`** — the crafted image-less fallback.
5. **[P1] `$impeccable animate`** — reveals + add-to-cart micro-interaction (+ reduced-motion).
6. **[P1] `$impeccable harden`** — targets/focus/a11y baseline.
7. **[P2/P3] `$impeccable polish`** — final craft pass + states.

**Prereq for the data side:** the demo menu must be reseeded with real dishes + photos (the e2e test-data pollution) so the redesign shows its best.

**Next step:** on your go I'll start with **`$impeccable colorize`** (token-ify + theme the menu — the P0), build it against the real page with screenshot proof, and bring it back before moving down the list.
