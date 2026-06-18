# UI Improvement Plan — dowiz (post-deploy)

**Date:** 2026-06-18 · **Method:** live dogfood of dowiz.fly.dev (agent-browser) + `web-design-guidelines` review + a11y signals. Deploy `9d18c93` is live; the menu now hydrates and add-to-cart works.

---

## A. Issues spotted in the live dogfood (prioritized)

| # | Sev | Issue | Evidence |
|---|-----|-------|----------|
| L1 | 🔴 **Critical (functional)** | **Cart page stuck on "Loading…"** even with items in the cart. The bundle loads (`window.DowizCart` exists) but never renders and makes **no data fetch**. The order flow now breaks at the *cart* step — the next link after the menu fix. **Pre-existing** (cart bundle unchanged by this session). | `/s/demo/cart` → `appHTML = <p>Loading...</p>`, `DowizCart=object`, no `/public/.../menu` fetch |
| L2 | 🟠 High | **Demo menu buried in e2e test data** — `E2E-Cat-*`, `Test-Cat-*`, `CExt-Cat-*` dominate the first screen. Awful first impression. | menu screenshot; ~31 junk categories |
| L3 | 🟠 High | **Empty image placeholders waste a 160px grey box per card** when a product has no image (most of the demo). Cards are ~70% empty grey. | menu screenshots; `.product-image-placeholder` 160px |
| L4 | 🟡 Med | **Tap targets under 44px** — add button **32×32**, language links **35×30** (checkout buttons are correctly 44px, so the menu is the inconsistent one). | live measure |
| L5 | 🟡 Med | **Sparse desktop layout** — 800px centered container leaves big empty margins; product grid shows ~1 column despite room. | desktop screenshot |
| L6 | 🟢 Low | **Generic/templated look** — flat dark cards, no imagery, no depth/microinteraction polish, plain text language switcher. | visual |

✅ Confirmed working live: U2 (`Dubin & Sushi`, no `&amp;amp;`), the new add buttons + cart FAB (count increments), no horizontal scroll on mobile, add button has `aria-label`.

---

## B. `web-design-guidelines` review (file:line)

### apps/api/src/lib/ssr-renderer.ts
- `ssr-renderer.ts:197` — `<img>` has no explicit `width`/`height` → layout shift (CLS) when images load.
- `ssr-renderer.ts:197,359` — empty state is a 160px grey gradient box; weak empty-image handling — collapse the media area or use a compact branded icon when no image.
- `ssr-renderer.ts:203,365` — add button is 32×32 (below 44px touch target); no `touch-action: manipulation` → 300ms double-tap delay on mobile.
- `ssr-renderer.ts:326` — `theme-color="#ea4f16"` but page background is `#121212` → mismatch; should match the page background (or the brand bar).
- `ssr-renderer.ts:356,365` — transitions are explicit (good, not `transition: all`) ✓ but **no `prefers-reduced-motion`** variant anywhere → `.cart-bounce`/hover lifts ignore the OS reduce-motion setting.
- `ssr-renderer.ts:365` + `.locale-switcher a` — no `:focus-visible` ring → keyboard users get no visible focus.
- `<html>` (whole doc) — no `color-scheme: dark` → native scrollbar/form controls render light on a dark page.
- prices — no `font-variant-numeric: tabular-nums` (minor; matters in the cart list).

### apps/api/src/lib/ssr-client-renderer.ts
- `ssr-client-renderer.ts:62` — `<p>Loading...</p>` uses `...` not `…`, and has no `aria-live="polite"` → screen readers never hear that content arrived (and this is the screen stuck in L1).

### apps/api/src/client/checkout/app.ts
- `checkout/app.ts:227` — phone `<input>` lacks `autocomplete="tel"`, `inputmode="tel"`, and a `name`; label not associated via `for`/wrapping.
- `checkout/app.ts:232` — name input lacks `autocomplete="name"` / `name`.
- `checkout/app.ts:251` — address input lacks `autocomplete="street-address"` / `name`; placeholder `"Enter full address..."` uses `...` not `…`.
- `checkout/app.ts:221,231,248` — labels are siblings, not wrapping or `htmlFor`-linked → clicking the label doesn't focus the field.

---

## C. Improvement plan — phased, mapped to skills

The project already has a strong skill set; I installed two more (`ui-ux-reviewer`, `wcag-accessibility-audit`). Each phase names the skill(s) that drive it.

### Phase 0 — Unblock the order flow (functional, do first)
- **Fix the cart page** (L1) — debug why `DowizCart` renders nothing (likely a missing `dos-location-id` meta or a silently-failing menu fetch in `client/cart/app.ts`). The order loop is incomplete until cart→checkout works end-to-end. → **skill: `systematic-debugging`** + agent-browser to reproduce.
- **Purge the demo test data** (L2) — process fix: run e2e against an ephemeral env, or a cleanup step; reseed `demo` with a real sample menu (images, prices, descriptions). → ops/data task (no skill).

### Phase 1 — Accessibility & interaction hygiene (fast, high-value)
Covers L4 + the §B findings. Mechanical, low-risk, big a11y/UX payoff:
- 44px tap targets (add button, lang links), `touch-action: manipulation`, `:focus-visible` rings, `prefers-reduced-motion` guard, `color-scheme: dark`, `theme-color` match, `…` loading + `aria-live`, form `autocomplete`/`inputmode`/`name`/label association, explicit `<img>` dimensions.
- → **skills: `wcag-accessibility-audit` (new)** for the full POUR audit + conformance levels, **`web-design-guidelines` (existing)** for the interface-rules pass (this doc is the first cut), **`ui-ux-reviewer` (new)** to sanity-check the interaction flow.

### Phase 2 — Visual redesign of the customer menu (L3, L5, L6)
Lift the menu/cart/checkout from "templated dark" to a polished, appetizing storefront:
- Collapse/redesign the empty-image state; introduce a real image treatment + fallbacks; denser, multi-column desktop grid; card depth, hover/press microinteractions; typographic hierarchy; brand the language switcher.
- → **skills: `design-taste-frontend` (existing)** as the anti-slop redesign driver, **`frontend-design` (existing)** for the polished component code, **`deliveryos-ui` (existing, project-specific)** to stay consistent with the design tokens/brand, **`taste-skill`** for the final anti-slop pass.

### Phase 3 — Componentize the SSR client surfaces
The menu/cart/checkout are hand-rolled `html\`\`` strings with inline styles. Consolidate into a small shared component layer (cards, buttons, inputs, sheet) so a11y + visual fixes are made once.
- → **skills: `component-builder` (existing)**, **`screen-builder` (existing)**, **`deliveryos-ui` (existing)**.

### Phase 4 — React admin SPA performance & patterns
Separate track for the `/admin` React app (the hotspots `MenuManagerPage`, `CheckoutPage` from the deep-check): code-split, memoization, data-fetch patterns, the circular-dependency chunk warning seen in the build.
- → **skill: `vercel-react-best-practices` (existing)**.

---

## D. Recommended order
1. **Phase 0** (cart fix + data cleanup) — the order flow is still broken at the cart; nothing else matters until a customer can actually check out.
2. **Phase 1** (a11y hygiene) — cheap, safe, ships fast, each item independently verifiable.
3. **Phase 2** (visual redesign) — the high-visibility upgrade; do after the flow works and the a11y base is solid.
4. **Phase 3/4** in parallel as capacity allows.

Each phase produces independently shippable PRs with Playwright/visual proof (per the project's Mandatory Proof Rule).
