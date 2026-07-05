# Astro storefront UX-parity matrix — the re-flip DoD (2026-07-05)

> **Operator directive (2026-07-05): the target FE stack is Astro + Svelte EVERYWHERE — React is
> interim-only** (kept serving staging/prod solely until each surface's parity matrix is green;
> the 2026-07-05 revert to React was stabilization, not direction). Storefront first, then
> admin + courier get their own matrices (harness task #12). Backend prod cutover does NOT wait
> for the FE rewrite — surfaces flip one-by-one via cutover flags.
>
> **Verification method (operator directive): agent-driven REAL-BROWSER sessions**, not static
> parity alone — an agent drives each flow in a live browser on staging, asserts the matrix's
> selectors AND reads screenshots visually (this is what caught both 2026-07-05 incidents that
> API/data-parity nets were blind to). Standing loop: harness task #13.

> The S1 HUMAN page may route to Astro again ONLY when every row here is green on the Astro
> stack (feature present + its detection selector renders + its flow drives E2E), plus the
> visual net diffs clean vs the node baseline. Data-parity (API 0-diff) is NOT sufficient —
> that mistake shipped a Phase-A scaffold to humans (reflection
> 2026-07-05-astro-scaffold-served-humans, ledger #80 class).

Reference implementation: the React SPA (`apps/web`). Astro scaffold state (2026-07-05):
**3/27 islands, 1/27 routes** — rows marked ❌ are absent there unless noted ⚠️ partial.

## Astro status vs the pre-rebuild storefront (condensed; full inventory below)

| Area | Features (count) | Astro today |
|---|---|---|
| Shell: routes (menu / checkout-sheet / order tracking) | A1 | ⚠️ menu only (1/3) |
| Theming: tenant palette + derivePalette + per-tenant Google Fonts + paper-skin + branding-preview | A2-A5 | ⚠️ raw CSS-vars only, no derivePalette/AA, no fonts, no preview |
| i18n al/en in-place + currency/EUR switcher | A6-A7 | ⚠️ in-memory locale only (SSR strings don't re-translate); no currency |
| TMA + PWA (manifest, SW, install prompt) + embed mode + lazy shell | A8-A12 | ❌ |
| Menu: hero, reviews link, state chip, category tabs+scroll-spy, search, price sort, macro-lens, allergen chips, empty/error/loading states | B1-B12 | ⚠️ chips+scroll-spy+search only |
| Menu: chef's pick, product card (R2 photos + fallback chain), sold-out hiding, compare (toggle/bar/panel), detail modal (gallery/modifiers/DishStats/allergen surface/ingredients) | B13-B24 | ❌ (imgs only if API hands absolute imageUrl — no R2/media resolution at all) |
| Menu: add-to-cart UX, cart↔menu reconcile, prefs persistence, footer | B25-B29 | ⚠️ bare add-delegation only |
| Venue gate: closed banner blocks ordering / busy banner / preview shadow non-orderable + claim | B30-B32 | ❌ (status chip shown, ordering never blocked) |
| Cart: provider (persisted/cross-tab/reprice), sticky button, drawer qty/remove/clear, empty state, free-delivery nudge, → checkout | C1-C7 | ⚠️ display-only count/total; no drawer, no checkout handoff |
| Checkout: sheet-over-menu, name, REQUIRED communication selector (6 kinds ADR-0016), receiver, entry photo, map-pin+address+entrance/apt contextual, dropoff chips, payment (cash default/crypto dark), cash-due, tip, total mirror, submit w/ x-channel, OTP modal, soft-signals, error+call-restaurant fallback, privacy, remember-last, confirmation | D1-D24 | ❌ entirely |
| Tracking: WS live status+stepper, honest ETA range, live courier map+route, courier contact, SR announcer, terminal exit, not-found, offline fallback, rating, summary | E1-E16 | ❌ entirely |
| Voice | F1-F3 | N/A — not mounted in React either (PR-3 future) |

**Also required for re-flip:** JSON-LD/OG/hreflang (astro emits none — bots stay on rust
regardless), `is_preview` shadow privacy, feature-flag replication
(`VITE_MENU_*`, crypto, paper-skin), and the island JS budget decision (measured 21.6KB gz vs
8KB budget — 2.7× over, pre-existing flag in ISLAND-BUDGET-OPTIONS.md).

## Verification nets to run green on the Astro stack before re-flip
1. `e2e/visual/client-path.visual.spec.ts` — 17 blocks × al/en (baselines are GENERATED, not
   committed; seed via `/api/dev/seed-visual-state`; harness `e2e/visual/harness.ts`).
   ⚠️ STALE SPEC: `client-path.visual.spec.ts:295,323` still targets `checkout-phone`, which no
   longer exists — current truth is `checkout-communication` + `checkout-comm-handle`
   (ADR-0016, `ContactInfoSection.tsx`). Fix the spec BEFORE using it as a parity oracle.
2. Flow specs: `flow-simpl-s1-sheet-checkout`, `flow-customer-checkout-render`,
   `flow-client-product-images`, `flow-productcard-declutter`, `flow-ingredients`,
   `flow-modifiers-promotions`, `flow-allergen-authoring`, `flow-offline-phone-fallback`,
   `flow-customer-track-link`, `flow-simpl-s6-claim`, `embed-mode`.
3. `e2e/tests/storefront-styles.spec.ts` (ledger #80) + `behavioural-invariants.spec.ts`
   (contrast + resolved-brand-surface).
4. Full feature inventory with file:line + selector per row: session record 2026-07-05
   (lane F distillate) — source files themselves are the contract
   (`MenuPage.tsx`, `ClientLayout.tsx`, `CheckoutPage.tsx` + `checkout/*`, `OrderStatusPage.tsx`,
   `CartProvider.tsx`, `packages/ui/.../ProductCard.tsx`).

## Ops note (rate-limit)
Staging public routes limit 100 req/min/IP — multi-project Playwright runs from one sandbox IP
trip 429 mid-suite (proven: behavioural "failures" were the 429 page). Run verification suites
`--project=mobile --workers=1`, or stagger projects ≥60s apart.
