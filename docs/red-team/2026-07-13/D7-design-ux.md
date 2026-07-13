# D7 — Red-Team Design / UX / Accessibility Audit

**Date:** 2026-07-13 (Sunday) · **Auditor:** red-team design critic (adversarial pass)
**Targets (live):** https://dowiz-staging.fly.dev · https://dowiz.fly.dev — landing, `/s/:slug` storefront, `/login`, `/admin/*`
**Method:** Live Playwright drives (Chromium 1366×850 desktop + 390×844 mobile touch), scripted a11y probes (contrast sampling, tab traversal, tap-target measurement, reduced-motion emulation, money-tween sampling), curl HTTP/meta probes, plus static source review of `/root/dowiz/apps/web/src`, `/root/dowiz/packages/ui/src`, `/root/dowiz/web/src`. Screenshots in `assets/`.
**Session hygiene:** the browser-use MCP screenshot backend was broken in this environment ("Root CDP client not initialized"); all browser work fell back to direct Playwright (headless Chromium, own isolated contexts, closed after each script). No orders were placed on prod; the prod funnel was walked to the empty-submit validation step only. No fixture data was mutated (the Settings hours form was deliberately NOT used — see F-08).

---

## 1. Bottom line

**Would a stranger trust this enough to order and pay? On the storefront funnel itself — mostly yes. Everywhere around the funnel — no.**

The core customer path on prod (`dowiz.fly.dev/s/demo`: menu → add → cart → checkout sheet) is genuinely competent: coherent dark venue theme, serif display headings, instant add-to-cart toast, free-delivery microcopy, clean cart drawer (`assets/pr-3-cart-mobile.png`, `assets/pr-4-checkout-mobile.png`). A hungry customer handed a direct link will probably complete a cash order.

Everything that *builds or destroys trust around* that funnel is currently working against conversion:

- **Prod has no front door.** `dowiz.fly.dev/` 302s straight to a context-free `/start` upload wizard with no logo, no pitch, no nav (F-01). The excellent Warm Cosmo-Noir landing exists only on staging.
- **The product's own numbers lie.** Analytics shows revenue **0** with a green **"+15% vs previous period"**, next to a top-products list summing to **199,500 ALL** — on one screen (F-03). For a product whose entire pitch is "Paratë e tua" (your money), self-contradicting money figures are fatal to owner trust.
- **Test garbage is customer-visible.** `UI-FCat-1783260801575` is a live category tab on the staging storefront a prospect would be demoed on (F-04).
- **The checkout form is the least accessible screen in the app** — most required inputs have no programmatic label and validation falls back to the browser's English "Please fill out this field." bubble on an Albanian page (F-05, F-06).
- **Owner surfaces contradict each other**: activation says "customers can order now" while the storefront says ordering is suspended; the Settings hours form renders every day as closed while the storefront footer shows real hours — saving that form would wipe the schedule (F-07, F-08).

Verdict: **customer funnel B− · owner trust D · first impression (prod) F.** The team can fix the top 10 below in days, and most of them are copy/meta/label-level, not architecture.

---

## 2. Findings

Severity: **Critical** = blocks conversion or destroys trust · **High** = major friction/credibility damage · **Med** = measurable friction · **Low** = polish.

### A. First impression & trust

**F-01 · Critical · Prod has no landing page — root redirects into a bare wizard**
`https://dowiz.fly.dev/` → `/start` ("Filloni me menunë tuaj").
Evidence: `assets/prod-landing-desktop.png`, `assets/prod-landing-mobile.png` — no logo, no product name, no value proposition, no nav, no footer; just an upload dropzone and two 20 px-tall text links.
Why it costs: a prospect typing the bare domain gets an unexplained file-upload form asking for their menu PDF. Nothing on screen says what dowiz is, who runs it, or why to trust it with a menu. The polished cosmo-noir pitch page ("Kuzhina jote. Klientët e tu. Paratë e tua.", `assets/staging-landing-desktop.png`) exists **only on staging**.
Fix: ship the staging landing to prod as `/`; keep `/start` as the CTA target. Add logo + one-line context to `/start` regardless.

**F-02 · High · Login page is a dead end and brands itself as a different product**
`/login` (staging + prod). Evidence: `assets/admin-1-login-desktop.png`.
- Title says **"DeliveryOS"** while the landing, sidebar and footer all say **"dowiz"** — a payment-adjacent login screen under a name the user has never seen reads phishy.
- **No sign-up link, no "forgot password"** — a new owner or an owner with a lost password has zero on-screen path forward.
- "Vazhdo me Telegram" white-on-`#229ed9` at 14 px measures **3.02:1** (probe output, walk-3 log) — fails WCAG AA 4.5:1.
Fix: unify brand name; add "Krijo llogari" → `/start` and a password-reset path; darken the Telegram button or bump label size/weight.

**F-03 · Critical · Analytics money figures contradict each other on one screen**
`/admin/analytics` (staging fixture). Evidence: `assets/admin-sec-analitika.png`.
- "Të Ardhurat: **0**" with a green "**+15% kundrejt periudhës së mëparshme**" — a fabricated-looking delta on a zero value (a 0-revenue period cannot be +15%).
- Same screen, "Produktet Kryesore": Set Premium **199,500 ALL**, Pepperoni **156,000 ALL** … while the revenue trend above shows **"Totali: 0 ALL"** and every bar is "0k".
- Top-products list includes **"Pita Test Sushi Updated"** — test data.
Why it costs: this is the screen that must prove "your money, visible and honest." An owner who spots one impossible number stops believing every other number, including payouts. (Also violates the project's own VERIFIED-BY-MATH standing rule.)
Fix: hide the delta chip when the base is 0/insufficient; make top-products and trend read from the same period + source; purge test rows from the fixture.

**F-04 · Critical (staging) · e2e fixture garbage is customer-visible**
`/s/demo` (staging). Evidence: `assets/dm-1-menu-mobile.png`, `assets/dm-2-after-add-mobile.png` — a live category tab **"UI-FCat-1783260801575 (1)"** and product **"UI-FProd-1783260801575"** between "Pizzas" and "Salads"; `/admin/menu` shows **38 categories**, most of them `Dsh-Cat-…`, `ModPromo-Cat-…`, `UI-Checkout-Cat-…`, `WS2-Cat-…`, `E2E-Debug…` empties (`assets/admin-sec-menu.png`).
Why it costs: staging is the demo environment shown to prospects; machine-named categories on a "real" storefront scream unfinished. It also proves e2e runs leak permanent state into the shared fixture.
Fix: namespace e2e data behind a hidden flag or hard-delete in teardown; add a guardrail asserting no `UI-*`/`E2E-*` names in any public menu payload.

**F-05 · High · Trust-metadata basics missing: constant `<title>`, no favicon, landing unshareable**
Evidence (curl + probe log):
- `<title>` is **"Dowiz"** on every route — landing, storefront, admin, checkout (recon-report.json: all 12 captures). Venue pages have correct `og:title` ("Dubin & Sushi — Order Online | Dowiz") but the tab/Google title never changes — WCAG 2.4.2 fail and weak SEO for the exact pages meant to be shared.
- `favicon.ico` → **404** (generic tab globe on the surface customers bookmark).
- The staging landing has **zero** `og:*`/`meta description` tags (grep count: 0) — pasting the marketing page into any chat yields a naked link.
- Storefront OG has **no `og:image`** and its description is **English** while `og:locale` is `sq_AL`.
Fix: dynamic `document.title` per route/venue; serve favicon; add landing OG set; per-venue OG image (owner logo/hero) + localized description.

**F-06 · High · Venue media 404s on every venue tested (both envs)**
Evidence: console/network logs in every walk — `/media/<venueId>/hero/cover.webp` → 404, `/media/<venueId>/hero/video.mp4` → 404 (staging demo, dubin-sushi, prod demo), plus a product image `/images/28239442-…d35f3cf75070.webp` → 404. Menu manager shows placeholder icons for most products (`assets/admin-sec-menu.png`).
Why it costs: food sells with photos; the hero silently falls back to a map graphic, cards render text-only, and the console noise masks real errors. On prod the demo storefront a prospect sees has no hero media at all.
Fix: fix the media pipeline or stop requesting missing renditions; give owners an explicit "add photos" nudge (menu-manager already has the placeholder — add upload CTA on it).

### B. Conversion path (customer)

**F-07 · Critical (staging demo surface) · The public demo storefronts cannot convert anyone**
`/s/artepasta`, `/s/dubin-sushi` (staging shadow demos). Evidence: `assets/sf-1-menu-mobile.png`, `assets/sf-3-item-detail-mobile.png` (dialog: "Maki Salmon | 600 ALL | … | **Vetëm pamje — pa porosi**").
- Tapping an item card opens a detail sheet whose only "action" is a disabled notice; card-level add buttons don't exist. The funnel dead-ends for a hungry customer.
- The dead-end is equally bad for the *owner* funnel: the huge pink "Kjo është një demo" banner (full first viewport under the hero on mobile, `assets/sf-1-menu-mobile.png`) pitches the OWNER (0% komision, CRM…) inside the customer's menu, yet contains **no button** — no "Claim this shop", no link to `/start`. A pitch with no CTA converts nobody.
Why it costs: these demo pages are the outbound growth artifact (QR/OG/claim flow per roadmap). Right now they burn both audiences.
Fix: add a single CTA to the demo banner ("Ky është dyqani yt? Merre falas →" → claim/start); collapse the banner to one line after first scroll; consider letting demo carts run to a "this is a preview" checkout wall instead of killing interaction at the first tap.

**F-08 · High · Owner-facing status contradicts customer-facing reality — and the hours form is a data-loss trap**
Evidence:
- `/admin/activation` says "**Dyqani juaj është online — Klientët mund të porosisin tani**" (`assets/admin-4-activation.png`) while `/s/demo` shows "Aktualisht i mbyllur … porositja rihapet gjatë orarit të punës" and every add tap is refused (`assets/dm-2-after-add-mobile.png`).
- `/admin/settings` "Orari i Punës" renders **all seven days toggled off/"Mbyllur"** (`assets/admin-7-hours.png`) while the same venue's storefront footer shows real hours (Mon–Thu 09:00–22:00, Fri 09:00–23:00, Sat 10:00–23:00; walk-8 log). The form fails to hydrate saved hours; an owner pressing "Ruaj" would silently overwrite their real schedule with "closed every day". (For this reason the audit did not touch the form.)
- Same page: "**Delivery radius: 0 km**" — untranslated English label AND a zero radius displayed as if configured.
- Settings notifications list renders a **raw JSON blob** for a push subscription (`push | {"endpoint":…`) among 14 `test-chat-…` Telegram rows.
Why it costs: an owner told "you're online" while customers are told "closed" will misdiagnose lost sales; the hours form can destroy a live schedule with one click.
Fix: hydrate the hours form from the API before enabling Save (disable Save on hydration failure); reconcile the activation "online" copy with actual orderability (closed-by-hours ≠ online-now); localize the radius label; render push subscriptions with a friendly name.

**F-09 · High · Closed-state affordance mismatch: 50 bright enabled "+" buttons that refuse to work**
`/s/demo` staging (closed on Sundays). Evidence: `assets/dm-2-after-add-mobile.png` — every card keeps its saturated amber `+` (the strongest visual affordance on the page); tapping it fires a blue toast "Restoranti është i mbyllur — porositë janë pezulluar…" at the TOP of the screen, far from the bottom-of-screen thumb that tapped.
- The closed banner says ordering "reopens during working hours" but never says WHEN (hours are only in the footer, 4,000 px down).
- The "Mbyllur" pill on the hero is grey-on-photo with a decorative border — low salience (`assets/dm-1-menu-mobile.png`).
Why it costs: enabled-looking controls that scold on tap teach distrust of every control; customers won't scroll to the footer to learn when to come back.
Fix: while closed, demote `+` to outline/quiet style, and put "Hapet nesër 09:00" (next-open time) in the closed banner and the toast.

**F-10 · High · Checkout is a sheet on the same URL — refresh/share loses the checkout**
Prod funnel walk: cart "Porosit" opens the "Përfundimi" sheet while the address bar stays `https://dowiz.fly.dev/s/demo` (walk-12 log; `assets/pr-4-checkout-mobile.png`).
Why it costs: an accidental refresh, tab restore, or OS-level app switch on low-memory phones dumps the customer back to the menu with the sheet closed (cart survives via localStorage, checkout inputs don't); support can't be sent a checkout link. A `/s/demo/checkout` route exists in the codebase (`apps/web/src/routes/ClientRoutes.tsx:13`) — the deployed prod build doesn't use it for this flow.
Fix: push `/s/:slug/checkout` onto history when the sheet opens (restore on load), or ship the route-based checkout consistently.

**F-11 · Med · Empty-cart / bad-slug handling is decent but leaks a soft-404**
`/s/definitely-not-real-xyz` returns **HTTP 200** with the full app shell, then a client-side "Restoranti nuk u gjet" card with a home CTA (`assets/staging-badslug-desktop.png`).
Good: the not-found state itself is well-designed (distinct from retryable errors — `MenuPage.tsx:127,245`).
Bad: 200-for-anything means search engines index infinite junk slugs, and the pre-state header shows a generic "Menu" title + placeholder map pin, briefly implying a real venue.
Fix: have the SPA-proxy return 404 (or `<meta name="robots" content="noindex">`) for unknown slugs; brand the interim header neutrally.

**F-12 · Med · Feature bloat crowds the one action that matters**
Storefront toolbar: sort-by-price, "Më shumë proteina", "Kalori: ulët→lartë", plus a per-card "⇄ Shto për krahasim" compare button on all 50 items (walk-1 log; `assets/dm-2-after-add-mobile.png`).
Why it costs: a sushi menu is not a spec-sheet market. Protein/calorie sorts and a compare tray add 50+ extra interactive elements between the customer and "+", and the ⇄ glyph is unguessable (icon-only, top-left slot where a favorite/heart normally lives).
Fix: collapse nutrition sorts behind one "Filtro" control; make compare opt-in from the item sheet; A/B if in doubt — the null hypothesis is that compare costs conversions.

**F-13 · Med · Item card layout: title truncation + no images**
`assets/dm-2-after-add-mobile.png`: two-up mobile grid truncates "Pepperoni" → "Peppero…", "Margherita" → "Margher…" because the floating `+` overlaps the title row; most cards have no photo, so the grid is mostly whitespace + truncated serif names.
Fix: single-column list rows on ≤400 px (photo left, full name, price, + right) — the current 2-up grid only makes sense with images.

### C. Visual design

**F-14 · Med · Two demo storefronts, two different themes, same restaurant name**
`/s/dubin-sushi` (light "shadow demo" of *Dubin & Sushi*, `assets/ds-1-menu-mobile.png`) and `/s/demo` (dark, ALSO named *Dubin & Sushi*, different menu) coexist on staging.
Why it costs: anyone exploring the demo environment sees the same brand twice with different menus/themes — reads as broken multi-tenancy.
Fix: rename the shadow demo or the fixture; one canonical demo per brand.

**F-15 · Med · "0k" money formatting on the owner dashboard**
`assets/admin-3-dashboard-desktop.png`: "Totali **0k**". Source: `apps/web/src/pages/admin/DashboardPage.tsx:421` — `Math.round(stats.revenue/1000)` + literal `"k"`, bypassing `formatMoney`/`PriceDisplay` (the app's declared single money authority) and losing everything under 1000 ALL; the "k" is also not localized.
Fix: route through `PriceDisplay` with compact notation; never show "0k" for zero.

**F-16 · Med · Stacked-opacity muted text collapses contrast**
`apps/web/src/pages/client/MenuPage.tsx:634` — category count badge `opacity-40` **on top of** `--brand-text-muted`; on light tenant palettes effective contrast ≈ **1.5:1** (fails AA at any size). Same double-muting pattern at `MenuPage.tsx:584` and `ClientLayout.tsx:153` (separator dots).
Also live-measured: white-on-`--color-success` (`#059669`) pairings ≈ **3.77:1** (SwipeToComplete.tsx:105, MapWithPin.tsx:111) and the offline banner white-on-warning ≈ **3.18:1** at 12 px semibold (`OrderStatusPage.tsx:491`) — both under AA for normal text.
Fix: ban opacity-on-muted via lint rule; adjust success/warning "on-" tokens (darken bg or use dark text).

**F-17 · Low · Brand documentation drift**
`docs/design/dowiz-brand/BRAND-BIBLE.md` **does not exist** (the audit brief and project memory both point at it; "Warm Cosmo-Noir" appears only in `agent-governance/index.ts:331-349`). The real spec, `docs/design/DESIGN.md`, documents a default palette (`#ea4f16`/`#121212`) that differs from the shipped `:root` (`#d69a3d`/`#061b1a`, `packages/ui/src/theme/tokens.css:37-58`).
The good news: the *shipped* landing genuinely delivers the intended brand (serif display + gold on deep teal, "SHIHEMI, KAUBOJ I HAPËSIRËS" footer, dry voice) — the code is more on-brand than the docs.
Fix: write the brand bible where it's claimed to be; sync DESIGN.md tokens to tokens.css.

**F-18 · Low · Paper skin can leak into the customer storefront**
`packages/ui/src/theme/paperSkin.ts:6` promises "never the client storefront", but `apps/web/src/routes/ClientLayout.tsx:114` spreads `paperSkinAttr()` on the client shell — any device with `localStorage['dos_paper_skin']='on'` (e.g., a QA phone used for a live demo) reskins the customer UI.
Fix: drop the attr from ClientLayout or hard-gate it to non-client roots.

### D. Accessibility (see scorecard §3)

**F-19 · Critical · Checkout required fields have no accessible labels**
Live (prod, walk-12 field dump): `Emri juaj`, `Adresa e rrugës`, `Numri ose emri i hyrjes`, `Numri i apartamentit`, and the required find-the-place `TEXTAREA` all render `label=""` — placeholder-only. Source: `apps/web/src/pages/client/CheckoutPage.tsx:670-783` — 10 of 12 visual labels lack `htmlFor`/`id`; the required notes `Textarea` (L767-777) has **no accessible name at all** (the design-system `Textarea` label prop is simply not passed). WCAG 1.3.1/4.1.2/3.3.2 failures on the money screen; placeholders vanish on input, so sighted users also lose context mid-form.
Fix: use the existing `Input`/`Textarea` primitives with `label` (they wire `htmlFor` correctly — `packages/ui/src/components/atoms/Input.tsx:9`).

**F-20 · High · Validation speaks the browser's English, one field at a time**
Evidence: `assets/pr-5-checkout-validation-mobile.png` — empty submit on the Albanian checkout produces the native Chrome bubble "**Please fill out this field.**" on `Emri`.
Why it costs: language whiplash at the moment of maximum commitment; native bubbles disappear on scroll, only show one error, and are inconsistent with the app's own `role="alert"` errors (phone/entrance already have proper ones — `CheckoutPage.tsx`).
Also: required marking is inconsistent — "Komunikimi" carries a red `*`, the equally-required "Emri" doesn't (`assets/pr-4-checkout-mobile.png`).
Fix: `noValidate` on the form + run the existing custom validators for all fields; mark all required fields uniformly.

**F-21 · High · Cash-amount insufficiency is signaled by border color only**
`CheckoutPage.tsx:878` — when the entered cash is less than the total, only `borderColor` flips to danger: no text, no icon, no `aria-invalid`, no `role="alert"` (unlike neighboring fields). WCAG 1.4.1 on the payment step.
Fix: add the same `<p role="alert">` used for phone errors.

**F-22 · Med · Item cards and dialogs have incomplete semantics**
- Item card is `ARTICLE tabindex=0` with `onclick` but **no `role="button"`** (live probe, walk-11): focusable but announced as plain content; Enter/Space activation not guaranteed.
- Product-detail dialog `role="dialog" aria-modal="true"` without `aria-labelledby` (`MenuPage.tsx:878`).
- Cart `ResponsiveDialog` never moves focus into itself on open (no focus trap/initial focus — `ResponsiveDialog.tsx:68-129`).
- Language switcher trigger lacks `aria-expanded` (`I18nProvider.tsx:88-96`).
- Admin MenuManager modals ship with a11y eslint-disables instead of fixes, and no Escape handling (`MenuManagerPage.tsx:853-921`) — while sibling pages do it right (`DashboardPage.tsx:564`).
Fix: role/keyup on cards; `aria-labelledby` to the `<h2>`; initial focus + trap in dialogs; `aria-expanded`; port the DashboardPage backdrop pattern to MenuManager.

**F-23 · Med · Keyboard reach is exhausting on the storefront**
Live tab traversal (walk-11): focus cues are **present and visible everywhere probed** (2–3 px outlines — genuinely good), but 18 Tab presses were still inside the category-pill row (16 categories + sorts before the first product). The pill row is not a `tablist` with arrow-key navigation, so keyboard users pay ~25 tabs to reach food.
Fix: `role="tablist"` + roving tabindex on categories; add a skip-to-menu link.

**F-24 · Med · Money values animate (count-up) across surfaces**
Source-verified inventory: storefront sticky cart total (`ClientLayout.tsx:154`, `AnimatedNumber`, 240 ms cubic-out), owner dashboard revenue/counters (`DashboardPage.tsx:421,423`), analytics stat cards (`AnalyticsPage.tsx:262`), and courier earnings `CountUpPrice` which re-animates **from 0 on every data refresh** (`EarningsPage.tsx:47-65`, 400 ms).
Mitigations exist (all respect `useReducedMotion`; durations are short), but: (a) the project's own canonical-stack decision is money-never-tweens — the deployed SPA still tweens it in 4 places; (b) two duplicate tween implementations have already drifted (240 vs 400 ms); (c) the from-zero restart makes courier earnings look like they're being recalculated on every glance.
Fix: freeze money display (tween nothing, or tween only non-money counts); at minimum make `CountUpPrice` tween from the previous value.

**F-25 · Low · Tap targets under minimum on landing/start**
Probe (recon-report.json): staging landing footer links 17 px tall ("Privatësia", "Kodi burim", "makemepulse"), radio widget 89×32; prod `/start` links "Filloni pa menu" 97×20, "Tashmë kam një llogari" 149×20. Under the 24 px WCAG 2.2 AA floor and far under the 44 px mobile guideline.
Fix: min-height 44 px tap areas via padding.

### E. Responsive / mobile

**F-26 · Low (pass with notes) · No horizontal overflow anywhere probed**
`hOverflow: 0` at 390 px on all 12 captured pages — layout discipline is genuinely good. Notes: the search input collapses so far its placeholder truncates to "Kër" (`assets/dm-2-after-add-mobile.png`); the checkout summary truncates "2 arti…" where space exists (`assets/pr-4-checkout-mobile.png`); the demo-pitch banner consumes the entire first mobile viewport on demo venues (F-07).

### F. Micro-interactions & perceived performance

**F-27 · Med · Owner dashboard greets new owners with a dead orders panel (staging)**
`/api/owner/orders` returned **HTTP 500** on every dashboard load of the primary test owner (walk-3/5 logs); the panel shows a well-designed error state ("Gabim / Ngarkimi i porosive dështoi / Provo përsëri" — `assets/admin-3-dashboard-desktop.png`), but retry keeps failing.
Good: the error state itself, and the courier-map empty state, are properly designed.
Bad: the flagship panel of the flagship screen 500s on the canonical fixture — whatever demo you run on staging starts with an error.
Fix: fix the 500 (staging data/regression), and add an alert hook so a persistent 5xx on this endpoint pages someone.

**F-28 · Low · Positive feedback is good where it exists**
Add-to-cart: instant green toast "U shtua në shportë" + cart bar "2 Shporta · 2100 ALL" (`assets/pr-3-cart-mobile.png`); loading skeletons with a 300 ms anti-flash floor (`MenuPage.tsx:119,259`); cart free-delivery microcopy "Dorëzimi falas u aktivizua!". Two nits: the toast spawns top-of-screen while the thumb is at the bottom; "Pastro" (clear cart, destructive) sits directly beneath the primary "Porosit" with no confirmation.

### G. i18n (sq/en/ua)

**F-29 · Med · Switching works; leaks are specific and enumerable**
Live: SQ→EN switch fully relabels the chrome and updates `<html lang>` (walk-11) — the mechanism (`packages/ui/src/lib/i18n.ts`, ~1291 keys) is solid. Confirmed leaks:
- **Live, customer-facing:** native validation bubbles in English (F-20); storefront OG description English under `og:locale sq_AL` (F-05).
- **Live, owner-facing (SQ mode):** "Delivery radius: 0 km" (Settings); "Not available yet — Ingredient consumption isn't connected…" panel (Analytics); chart weekday labels Mon…Sun (Analytics) — `assets/admin-sec-analitika.png`, `assets/admin-7-hours.png`.
- **Source-confirmed:** entire `AuthCallback.tsx` English-only (L17-35); `OrderStatusPage.tsx:206` 'Failed to fetch order status'; checkout phone tooltip `title="+355 followed by 7-14 digits"` (`CheckoutPage.tsx:680`); locale-blind `.toLocaleString()` calls (CouriersPage, AnalyticsPage, ShiftPage, CRMPage).
- **Data-level:** menu item names/descriptions and half the category names are English on Albanian storefronts ("Water", "Beverages", "sushi rice, nori…") — owner content, but the product gives owners no per-locale content nudge. Also "DurrëS" (typo) in the prod venue address.
Fix: kill native validation; sweep the five hardcoded strings; pass the app locale to date formatting; per-locale menu-content hint in the menu manager.

---

## 3. Accessibility scorecard (WCAG 2.1 AA lens)

| Criterion | Status | Evidence |
|---|---|---|
| 1.1.1 Non-text content (alt) | **Pass** | all `<img>` carry alt (static sweep); QR alt present |
| 1.3.1 Info & relationships (labels) | **Fail** | checkout: 10/12 fields placeholder-only; notes textarea nameless (F-19) |
| 1.4.1 Use of color | **Fail** | cash-amount border-only error (F-21) |
| 1.4.3 Contrast (minimum) | **Fail** | Telegram btn 3.02:1; white-on-success 3.77:1; offline banner 3.18:1 @12px; stacked-opacity ≈1.5:1 (F-16, F-02) |
| 1.4.10 Reflow (320–390 px) | **Pass** | 0 horizontal overflow on all probed pages |
| 2.1.1 Keyboard | **Partial** | full chrome tabbable; item cards lack role/activation guarantee; MenuManager modals no Esc (F-22) |
| 2.4.2 Page titled | **Fail** | `<title>` constant "Dowiz" on every route (F-05) |
| 2.4.7 Focus visible | **Pass** | 2–3 px outlines on every probed focus stop (walk-11) |
| 2.5.5/2.2-2.5.8 Target size | **Fail (AA 2.2)** | 17–20 px links on landing//start (F-25) |
| 3.1.1/3.1.2 Language of page | **Pass** | `lang` set and updated on switch (sq/en) |
| 3.3.1/3.3.2 Error id / labels | **Fail** | English native bubbles; inconsistent required markers (F-20) |
| 4.1.2 Name, role, value | **Fail** | dialog w/o name, card w/o role, switcher w/o aria-expanded (F-22) |
| Reduced motion (best practice) | **Pass** | global 0.01 ms rail + `useReducedMotion` guards; 0 animations under emulation (walk-11); one gap: courier map marker tween (`use-courier-marker.ts:39-132`) |

**Net: ~6/13 pass.** The failures cluster on exactly one screen (checkout) plus metadata — high-leverage fixes.

---

## 4. Top 10 prioritized fixes

1. **Ship a real front door on prod** — landing at `/`, context header on `/start` (F-01).
2. **Label the checkout form + custom validation in-locale** — use the existing `Input`/`Textarea` primitives; `noValidate` + `role="alert"` everywhere; fixes 4 WCAG failures and the worst conversion screen in one PR (F-19/20/21).
3. **Make analytics numbers non-contradictory** — no delta on zero base, one data source for trend vs top products, purge test rows (F-03).
4. **Purge e2e artifacts from shared fixtures + add a no-`UI-*`-in-public-payload guardrail** (F-04).
5. **Fix the Settings hours form hydration and block Save on failed load** — active data-loss trap (F-08).
6. **Give the demo storefront banner a claim/start CTA and shrink it** — turns the growth artifact from dead end into funnel (F-07).
7. **Per-route `<title>`, favicon, landing OG, storefront `og:image` + localized description** (F-05).
8. **Fix `/api/owner/orders` 500 on staging + alerting** (F-27).
9. **Closed-state honesty**: quiet the `+` buttons, show next-opening time in banner/toast (F-09).
10. **Retire money tweens (or unify + tween-from-previous)** per the canonical money-never-tweens decision (F-24).

Honorable mentions (cheap, do alongside): login sign-up/reset links + "dowiz" naming (F-02); Telegram button contrast (F-02); "0k" formatting (F-15); i18n five-string sweep (F-29); checkout URL state (F-10).

---

## Appendix — evidence index

- Probe data: `walk-1..12` logs + `recon-report.json` (scratchpad, session-local); key numbers reproduced inline above.
- Screenshots (all in `assets/`): landing `staging-landing-*`, `landing-scroll-2..5`; prod entry `prod-landing-*`; storefront demos `sf-*` (artepasta), `ds-*` (dubin-sushi), `dm-*` (staging demo), `pr-*` (prod funnel incl. cart/checkout/validation); not-found `staging-badslug-*`; admin `admin-1..7-*`, `admin-sec-*`; i18n/keyboard `i18n-*`, `kbd-*`.
- Environment notes: staging fixture venue closed (Sunday) → staging checkout unreachable without mutating hours (declined; see F-08); `sushi-demo` slug referenced in project docs is dead on both envs (404 from `/public/locations/sushi-demo/*` — update the fixture docs); prod funnel audited to validation only, no order placed.
