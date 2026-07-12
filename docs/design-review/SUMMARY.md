# dowiz — design review (screenshot pass)

Vision review of the canonical screenshot set (`audit/validate-final/*` + root gap-fillers),
4 surfaces, 40 screens, 2026-06-25. Full per-screen critiques:

- [admin-core.md](admin-core.md) — Orders, Menu, Settings, Branding, Couriers (×D/M)
- [admin-secondary.md](admin-secondary.md) — Analytics, CRM, Promotions, Supplies, Activation (×D/M)
- [courier.md](courier.md) — Login, Home/Tasks, Shift, Earnings, History, Delivery (×D/M)
- [storefront.md](storefront.md) — Menu, Product modal, Cart, Checkout, Checkout-validation, Tracking-404

## ⚠️ Screenshot data-quality caveats (re-capture needed)
The existing screencasts don't actually cover some key screens — several are stale/error/login states:
- **Courier active-delivery/map** (the most safety- & revenue-critical surface) — both shots are a *not-found* error, not a live map.
- **Admin Analytics** — desktop shot is the login/session-expired screen; mobile is an error state. The real dashboard is unreviewed.
- **Promotions / some others** — desktop vs mobile disagree (empty vs error).
- Much of the data is **test fixtures** ("Test-Cat-…", "E2E Stepper"), which undercuts "your store is live" surfaces.
→ Recommend a fresh, seeded capture run against current staging before acting on those screens.

## Cross-cutting themes (all surfaces)
1. **Color is decorative, not semantic** — only the primary CTA carries brand color, so real controls (qty steppers, toggles, status pills, KPI tiles) look disabled/pale-on-pale; status colors are misused (green zeros, grey "Online").
2. **No shared empty / loading / error component** — states float mid-screen with dead voids; seed/placeholder data (53 "?" courier rows, 50 blank menu thumbs) reads as broken instead of an onboarding state.
3. **Chrome buries content** — welcome banners, full-width status bars, oversized KPI grids push the actual orders/products/menu below the fold (worst on mobile).
4. **Low-contrast secondary text/icons fall below AA** — pervasive; a measured contrast/AA guardrail is warranted.
5. **Weak affordances & mobile ergonomics** — detached/edge-aligned icon buttons, non-tappable rows, crowded control rows truncating labels; primary actions not in the thumb zone.

## Highest-impact fixes (ranked)
1. **Storefront has no food photography** — every item falls back to the same glyph on a pink wash; biggest conversion drag. Ship a text-first sellable card (name+desc+price hero, labeled "Add") + image pipeline.
2. **Branding tool ships illegible storefronts** — generates pale-price-on-pale-card (contrast bug); add an AA guardrail to the palette generator. (Looks like a real bug, not polish.)
3. **Transparent order math** — cart + checkout lack an itemized summary (subtotal/fee/min-order/total); total even differs between shots. Surface it before pay.
4. **Courier live-delivery screen** — design + capture: full-bleed map, single bottom-anchored state-driven primary (Navigate→Picked up→Delivered), color-coded status, one-tap call.
5. **Lead admin Orders/Menu with content, not chrome** (mobile) + real empty/onboarding states for Couriers & Menu; stop marking phone-less couriers "Online."
6. **Fix primary-action language** — make logins' primary look enabled (amber/filled, not grey); one loud, color-coded online/offline toggle for couriers.
7. **Localize + brand validation** — replace English native browser tooltips in the Albanian UI; persistent in-language inline errors + scroll-to-first-error.
