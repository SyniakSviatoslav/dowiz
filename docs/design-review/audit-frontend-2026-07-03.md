# Frontend / UX / Design-System Audit — dowiz / DeliveryOS

**Date:** 2026-07-03 · **Scope:** `apps/web/src` (storefront `/s/:slug`, `/admin/*`, `/courier/*`), `packages/ui` (theme, tokens, i18n, components), per-tenant theming, i18n parity (sq/en/uk), the design system, PWA InstallPrompt + Wave-1 (voice / reorder / cinematic / ProductDetailSheet — much uncommitted in the working tree).
**Method:** READ-ONLY. 4 parallel lanes (storefront+theming, admin, courier+Wave-1, design-system+i18n). Every `file:line` verified against the working tree; contrast ratios computed with the repo's own WCAG relative-luminance math (`palette.ts:contrastRatio`); dead-code and undefined-token claims grep-verified for call sites.
**Excluded (already known, not re-reported):** design-system prune (~2,600 lines already cut); lucide icon migration (planned, not done); PWA prompt *existence*; voice needs `@deliveryos/voice` workspace-dep; 44-tap demo & scroll-anchor chip *concepts*; sold-out chip removal.

## Severity counts

| Severity | Count |
|----------|-------|
| CRITICAL | 3 |
| HIGH | 27 |
| MED | 34 |
| LOW | 26 |
| **Total** | **90** |

---

## Systemic classes (fix the class, not the instance)

These recur across ≥2 surfaces; each has its own numbered findings below but the root fix is shared.

- **S1 — "Retry" buttons that never clear `error`, so a successful refetch still shows the error screen.** The `error ? <ErrorState/> : …` render branch is never re-evaluated because the success path never does `setError('')`. Dead recovery on: OrderStatusPage, AnalyticsPage (retry re-sets same value), HistoryPage, EarningsPage, TasksPage. (ShiftPage & MenuPage do it right — copy them.)
- **S2 — Tenant CTAs pair the wrong on-primary token → sub-AA text on the tenant brand.** `palette.contrast.test.ts` guarantees *derivePalette's* outputs (`onPrimary`/`primaryStrong`), but consumers bypass them: `text-[var(--brand-bg)]` on raw `--brand-primary`, or hand-rolled `color-mix(--brand-bg …)`, or the **non-existent** token `--brand-on-primary`. Every one should be `var(--color-on-primary)` on a `--brand-primary-strong` fill.
- **S3 — Mock/fake data used as a fetch-failure fallback and presented as real.** On error/404/empty, several surfaces render fabricated data (fake customer history, fake ingredient BOM, mock store settings, Tirana/Durrës coordinates) indistinguishable from real — and some is savable/exportable. Replace every fabricated fallback with an explicit empty/error state.
- **S4 — Silent mutation failures (`console.*` only, no user feedback).** Toggles/saves/sends that fail leave the UI asserting success. Pervasive across admin + courier. One shared toast-on-error pattern.
- **S5 — No focus trap on any modal repo-wide** (`grep focus-trap|inert` = 0 hits) despite `aria-modal="true"`; divergent Escape/scroll-lock coverage across 3 ui scaffolds + ~10 hand-rolled page modals. Extract one modal-shell.
- **S6 — Undefined CSS custom properties silently fall back.** `--brand-on-primary`, `--brand-primary-rgb`, `--brand-on-primary` are referenced but defined nowhere (grep-verified); values fall through to inherited/hardcoded defaults, defeating tenant theming and (under sunlight) contrast.

---

## CRITICAL

1. **[CRITICAL]** `apps/web/src/pages/admin/CRMPage.tsx:79-87` — On customer-analytics fetch failure a hardcoded FAKE dataset (order "Pizza x2" 150000 ALL at "Rruga e Durrësit", 750000 spent, fake heatmap) is written into cache and rendered as that customer's real history. — Show error+retry; delete the fabricated fallback. *(S3)*
2. **[CRITICAL]** `apps/web/src/pages/client/CheckoutPage.tsx:714` — The Place-Order CTA is `bg-[var(--brand-primary-strong)] text-[var(--brand-bg)]`; `primaryStrong` is tuned against `onPrimary`, not `bg`. Computed contrast **1.29:1** (#fff on #ffe08a), **1.42:1** (pale rose), **3.74:1** (teal) — the button that places every order is illegible on pale brands. — `color: var(--color-on-primary)`. *(S2)*
3. **[CRITICAL]** `apps/web/src/routes/ClientLayout.tsx:195,285` — Sticky cart bar + cart-sheet "Checkout" button: `text-[var(--brand-bg)]` on raw `var(--brand-primary)`. Same math, 1.29–3.74:1 on 4 of 6 tested brand palettes — the two buttons that start every order. — `var(--color-on-primary)` + `--brand-primary-strong` fill. *(S2)*

---

## HIGH

### Contrast (tenant-themed surfaces)
4. **[HIGH]** `packages/ui/src/components/client/StateChip.tsx:30-39` — `open`/`available` chip text is raw `var(--brand-primary)` over a 12% primary tint: **3.84:1 on the DEFAULT dark theme** (#ea4f16 on #38261f), 1.17 pale amber; small semibold needs 4.5. The venue "Open" chip fails out of the box. — Use `var(--brand-primary-readable)`.
5. **[HIGH]** `packages/ui/src/components/client/ProductCard.tsx:122` — Chef's-Pick badge on photos: text `color-mix(--brand-bg 88%, #000)` on raw primary = **1.02:1** pale amber, 2.85 teal. The photoless variant (:138) already uses `primary-readable` correctly. — Use `onPrimary` on `primaryStrong`. *(S2)*
6. **[HIGH]** `packages/ui/src/components/client/ProductCard.tsx:147` — Add-to-cart "+" button `text-[var(--brand-bg)]` on raw primary: icon fails pale brands at 1.29:1 (3:1 graphic threshold). Same pattern at `MenuPage.tsx:1085,1089,1108,1657`. — Token swap. *(S2)*
7. **[HIGH]** `apps/web/src/pages/client/MenuPage.tsx:899,925,948,1007,1176,1254` — Active sort/lens/compare chips hand-roll `color-mix(--brand-bg 86%, #000)` on raw primary — 1.07 pale, 2.71 teal — recipe copy-pasted 6×. — One token pair `--color-on-primary`/`--brand-primary-strong`. *(S2)*
8. **[HIGH]** `packages/ui/src/theme/tokens.css:229-231,262-296` + `apps/web/src/routes/CourierRoutes.tsx:47,58` — Sunlight mode forces white surfaces but does NOT re-override the `data-surface="dark"` lightened semantic text (`--color-danger:#F87171`, `--color-success:#34D399`…). The courier shell hardcodes `data-surface="dark"`, so in the outdoor mode *built for couriers*, the offer countdown (TaskCard:65), deliver/accept error text, and Online badge render ~1.9–2.5:1 on white — illegible in sunlight. — Add `--color-*` overrides to `html[data-sunlight="on"]`. *(S6)*

### Fake data / money / auth
9. **[HIGH]** `apps/web/src/pages/admin/AnalyticsPage.tsx:80-89,394-451` — "Ingredient Consumption" is a static fake sushi dataset (Salmon/Nori/Wasabi) shown to EVERY tenant with working CSV/JSON export and "Based on today's orders" caption. — Remove or wire to real BOM; never export fake. *(S3)*
10. **[HIGH]** `apps/web/src/pages/admin/SettingsPage.tsx:69-79,152,156` — On 404/empty the form silently loads MOCK_SETTINGS ("Downtown Tirana", +35542345678…) which the owner can then SAVE as live store settings. — Render empty setup form. *(S3)*
11. **[HIGH]** `apps/web/src/pages/admin/SupplyLibraryPage.tsx:25-74` — Entire Supply Library is localStorage-only, pre-seeded with 15 fake sushi supplies for every owner regardless of cuisine; diverges per device, vanishes on storage clear, while server-persisted recipes (RecipeEditor:38) reference these device-local ids. — Server persistence or labeled empty start.
12. **[HIGH]** `apps/web/src/pages/admin/PromotionsPage.tsx:360,380 vs 139,147` — Unit mismatch: form reads/writes `discount_value`/`min_order_amount` as whole ALL but the list renders `/100`; a 500 ALL discount displays as "5 ALL". — Drop the `/100` (system convention is whole ALL per `formatMoney`).
13. **[HIGH]** `apps/web/src/pages/admin/ActivationPage.tsx:359` — Price-edit label claims "minor units, e.g. 850 = 8.50" but prices are whole ALL everywhere; an owner following the hint publishes 100× prices. — Fix the label.
14. **[HIGH / ESCALATE — auth red-line]** `apps/web/src/routes/AdminRoutes.tsx:144-148` + `apps/web/src/lib/apiClient.ts:152-158` — Logout removes only `dos_access_token`; `dos_refresh_token` survives and apiClient silently mints a fresh access token from it on any 401 (sessionStorage copy also stays). Logout does not end the session on a shared device. — Clear both tokens + server-side revoke.

### Logic bugs
15. **[HIGH]** `apps/web/src/pages/client/OrderStatusPage.tsx:229,465` — `error` is set on any non-401/404 failure but never cleared on a later success; one transient network blip permanently shows "This link is no longer active" even after the 15s watchdog refetches successfully. (:229 also hardcoded English.) — Clear on success; separate transient-retry from dead-link. *(S1)*
16. **[HIGH]** `apps/web/src/pages/client/CheckoutPage.tsx:381-390,495` — On a successful order `clearCart()` + `setShowConfirmation(true)` batch into one render; the `items.length === 0` early-return wins, so the customer sees "Your cart is empty — Browse menu" for 1.5s instead of the "Order placed!" animation (the confirmation overlay at :740 is unreachable). — Check `showConfirmation` before the empty-cart return.
17. **[HIGH]** `apps/web/src/pages/admin/AnalyticsPage.tsx:163` — Error-state Retry does `setPeriod(p => p)` (same value) so the `[period]` effect never re-fires; the only recovery action is dead. — Explicit refetch trigger. *(S1)*
18. **[HIGH]** `apps/web/src/pages/admin/SettingsPage.tsx:337-340` — A save that 404s shows the "Settings saved" success toast + saved animation (fake green). — Treat 404 as error. *(S4)*
19. **[HIGH]** `apps/web/src/pages/admin/DashboardPage.tsx:74-77,512-525` — Storefront copy-link is built by re-slugifying the location NAME, never using the real `slug` from /owner/settings; custom slugs (chosen in onboarding) diverge, so the owner copies/shares a dead URL. — Use `res.slug` (as MenuManagerPage:322 / BrandingPage:66 do).
20. **[HIGH]** `apps/web/src/pages/courier/DeliveryPage.tsx:264-272` — When `customer.lat/lng` or `restaurant.lat/lng` are missing IN PROD, the destination pin + route line silently fall back to hardcoded Tirana MOCK coords; courier gets a confident route to the wrong place (`||` also drops legitimate 0 coords). — Render no pin/route instead. *(S3)*
21. **[HIGH]** `apps/web/src/pages/courier/DeliveryPage.tsx:120-138,299-320` — `fetchTask` only handles the DEV-404 branch; any 500/network error (couriers are offline constantly) falls to `task=null` → "Delivery task not found" with only "Back to tasks", mid-delivery. — Distinguish error from not-found; offer retry.
22. **[HIGH / money-adjacent]** `apps/web/src/pages/courier/DeliveryPage.tsx:528-536 vs 234-237` — The post-pickup "How much did the customer pay?" input (`cashCollected`) is never read; `handleComplete` always posts `cash_amount: task.total`. A money input the courier edits and the server ignores. — Wire it or remove it.
23. **[HIGH]** `apps/web/src/pages/courier/HistoryPage.tsx:98,43-53` + `EarningsPage.tsx:151,93-116` — Retry calls `fetchHistory`/`fetchEarnings` without clearing `error` and the success path never resets it, so after a successful retry the `error ? …` branch still shows the error state forever. — Clear on success. *(S1)*
24. **[HIGH]** `packages/ui/src/components/courier/TaskCard.tsx:27-38` — The 30s offer countdown keeps ticking while the accept request is in flight (`isLoading` only sets `pointer-events-none`); at 0 it fires `onReject`, which optimistically removes the card and POSTs reject — racing the in-flight accept. — Pause the timer when `isLoading`.

### Privacy / a11y / Wave-1
25. **[HIGH / privacy]** `apps/web/src/pages/client/MenuPage.tsx:473-490` — Every storefront load with venue coords fires `navigator.geolocation.getCurrentPosition` (permission prompt, zero user gesture) and on grant POSTs the customer's exact GPS into a URL on the public third-party `router.project-osrm.org` — PII egress to an unaffiliated party for a "~N min" hint. Matches this repo's prior P0 privacy class. — Gate behind explicit action; proxy server-side.
26. **[HIGH]** `packages/ui/src/components/client/ProductDetailSheet.tsx:61-160` — `role="dialog" aria-modal` sheet has no focus trap, no Escape, no body scroll-lock (not one `useEffect` in the file); focus stays behind and the page scrolls under the sheet. — Add trap + Escape + scroll-lock before wiring. *(S5)*
27. **[HIGH]** `packages/ui/src/components/client/ProductDetailSheet.tsx:110,140,148` + `ProductCard.tsx:114,143,179` + `ThemeProvider.tsx:116` — The card→detail cinematic reveal uses `layoutId`, but the app's only Motion provider is `LazyMotion features={domAnimation}`; shared-element projection needs `domMax`. Flipping `VITE_CINEMATIC_REVEALS` on produces NO morph — the flag governs a dead code path. — Load `domMax` when the flag is on.
28. **[HIGH]** `packages/ui/src/components/client/ProductCard.tsx:86-96` — The card is a clickable `m.article` (opens the detail modal) with no `role`/`tabIndex`/key handler: keyboard & AT users cannot open product details at all for modifier-free items. Also the clickable-container-with-nested-button pattern. — Make the title an overlay button/link; keep the add button a sibling.
29. **[HIGH]** `apps/web/src/components/voice/VoiceSearchButton.tsx` (all `t('voice.search.*')`) — Zero `voice.search.*` keys exist in `i18n-catalog.ts` (grep=0); `pnpm verify:i18n-coverage` (wired into CI via `verify:all --ci`) FAILS right now, and every mic-state renders English for sq/uk. The untracked voice feature breaks CI on commit. — Backfill the 8 keys before commit.

### Design-system / dead code
30. **[HIGH]** `packages/ui/src/components/atoms/{Button,Input,StatusBadge}.tsx` vs `packages/ui/src/components/Base.tsx:14,61,102` — TWO full implementations of Button/Input/StatusBadge; the atoms trio is orphaned behind `atoms/index.ts` which has ZERO importers (verified). `atoms/StatusBadge.tsx:4-16` bakes `t()` labels into a module-scope map (locale-frozen at import). — Delete the atoms cluster (~235 lines incl. Skeleton/Icon).
31. **[HIGH]** `packages/ui/src/theme/index.ts` (79 lines) — The entire preset system (PRESETS ×7, `applyBrandTheme`, `getPresetConfig`, `injectThemeSSR`) has zero importers anywhere and isn't re-exported from the barrel; a dead third source of brand-color truth carrying the stale `#ea4f16` identity. (Repowise index stale-lists it as an "entry point".) — Delete.
32. **[HIGH / latent]** `apps/web/src/routes/AdminRoutes.tsx:123` + `CourierRoutes.tsx:58` — The shell sets `{...paperSkinAttr()}` AND unconditional `data-surface="dark"`; paper is a LIGHT cream skin, but `data-surface="dark"` swaps status/semantic text to Tailwind-400 lights → sub-AA light-on-light whenever paper is enabled (the dark-on-dark class, inverted). Flag OFF in prod today. — Make `data-surface` conditional on the active skin.

---

## MED

### States / logic (admin)
33. **[MED]** `apps/web/src/pages/admin/DashboardPage.tsx:437,447,382-388` — "Revenue" tile uses key `cart.total` ("Total"/"Totali", not Revenue), computes from `filteredOrders` so Live view excludes delivered orders despite the "Today's total revenue" tooltip, and `Math.round(x/1000)k` shows "0k" for <500 ALL. — Dedicated key + all-of-today aggregate + threshold formatting.
34. **[MED]** `apps/web/src/pages/admin/CouriersPage.tsx:88,211-224,522-527` — "Live Map": `setCourierPositions` has zero call sites (no WS subscription), so every courier marker renders at the store center / Tirana fallback — the map claims all couriers stand at the restaurant. — Subscribe to the couriers room (as DashboardPage:163-183) or drop the map.
35. **[MED]** `apps/web/src/pages/admin/MenuManagerPage.tsx:496-505` — Availability toggle is optimistic with no rollback and no toast on failure (console.debug only); card says "Available" while the server has it stop-listed. — Revert + error toast. *(S4)*
36. **[MED]** `apps/web/src/pages/admin/MenuManagerPage.tsx:412-418,1090` — Save silently no-ops on invalid price (0/negative/NaN): guard `return`s with no message; button only checks truthiness. — Inline price error like prep-time (1011-1013).
37. **[MED]** `apps/web/src/pages/admin/DashboardPage.tsx:136-138` — First WS subscription runs with `enabled: true` before tenantId resolves → subscribes to malformed room `location::dashboard` on every mount (second subscription gates correctly). — `enabled: !!tenantId`.
38. **[MED]** `apps/web/src/pages/admin/CouriersPage.tsx:121-138` + `AnalyticsPage.tsx:105-122` — Details race: expanding courier A then quickly B shares one `courierDetails` state; the slower response wins so B's panel shows A's earnings. Same shape for product orders. — Key results by id / discard stale responses.
39. **[MED]** `apps/web/src/pages/admin/CouriersPage.tsx:140-165,266` — Send-Invite has no pending/disabled state during the POST → double-click creates two invites. — Saving flag on the button.
40. **[MED]** `apps/web/src/pages/admin/BrandingPage.tsx:84-97` — Logo upload silently skipped if locationId hasn't resolved, and upload failure is a bare catch — in both cases the preview shows and the owner believes it saved. — Error toast + disable until locationId ready. *(S4)*
41. **[MED]** `SettingsPage.tsx:270-291` (Telegram toggles), `DashboardPage.tsx:232-245` (order messages), `PromotionsPage.tsx:260-283` (toggle/delete), `MenuManagerPage.tsx:62-102,145-151` (kitchen-busy, schedule delete) — Silent mutation failures (console-only). — One shared toast-on-error pattern. *(S4)*
42. **[MED]** `apps/web/src/pages/admin/FlowTestPage.tsx:227-230` — Error handler searches the stale `steps` closure captured at run start, so the failing step never gets the red marker (dev-only page). — Track active step id in a ref.

### States / logic (storefront + courier)
43. **[MED]** `apps/web/src/pages/client/CheckoutPage.tsx:291-292` — With no customer pin and no venue lat/lng, the order silently submits `delivery.pin` = hardcoded Durrës coords (41.324, 19.456) as a real pin; courier map shows a confident wrong location. — Omit `pin` when synthetic. *(S3)*
44. **[MED]** `apps/web/src/hooks/useReorder.ts:103` — `if (opts?.replace) clearCart()` runs before checking that any reordered line survived reconciliation; when every item is sold out, the current in-progress cart is wiped and nothing replaces it. — Only clear when `result.items.length > 0`.
45. **[MED]** `apps/web/src/pages/client/CheckoutPage.tsx:157` — Draft restore sets `deliveryType` from localStorage, but the order-type switch was removed (§4): a stale `'pickup'`/`'scheduled'` draft locks the user into that mode with no UI to exit; `'scheduled'` renders no address/pin fields yet submits as `type:'delivery'` with the fallback pin. — Clamp restored `deliveryType` to `'delivery'`.
46. **[MED]** `apps/web/src/pages/client/CheckoutPage.tsx:225-246` + `ContactInfoSection.tsx:104,111-113` — Validation for the two non-native fields (no communication channel; invalid phone) renders inline at the top of the form while the user is at the sticky bottom button — submit appears to do nothing (only `orderError` gets `scrollIntoView`). — Scroll/focus the failing field on `commError`/`phoneError`.
47. **[MED]** `apps/web/src/pages/courier/DeliveryPage.tsx:209-219` — `handlePickup` failure is `console.warn` only: offline courier taps "Mark as Picked Up", spinner ends, nothing happens. — Surface like `deliverError`. *(S4)*
48. **[MED]** `apps/web/src/pages/courier/DeliveryPage.tsx:459-463` — Call button always renders `href={'tel:' + task.customer.phone}` but `phone` is optional → `tel:undefined` dead button on phoneless orders. — Conditionally render.
49. **[MED]** `apps/web/src/pages/courier/TasksPage.tsx:38-49,83,158` — `fetchTasks` success doesn't clear `error`; WS `onReconnect: fetchTasks` after a failure loads tasks but the `error ?` branch still shows the error screen. — Clear on success. *(S1)*

### Wave-1 / new code
50. **[MED]** `packages/ui/src/components/courier/SwipeToComplete.tsx:73-76` — Drag registers `touchmove/touchend/mousemove/mouseup` but never `touchcancel`/`pointercancel` (and no `touch-action:none` on the knob); a system-cancelled touch never runs `handleEnd` → document listeners leak and the knob freezes mid-slide until unmount. — Add cancel handlers + `touch-action`.
51. **[MED]** `packages/ui/src/components/courier/SwipeToComplete.tsx:61-67` — `triggerComplete()` (async POST /delivered) is called inside the `setSlideRatio` updater; under `StrictMode` (main.tsx:71) updaters double-invoke in dev → double delivery POST. — Decide outside the updater.
52. **[MED]** `packages/ui/src/components/courier/SwipeToComplete.tsx:121-127,160-167` — `role="button"` container wrapping a real `<button>` = nested interactive (axe `nested-interactive`) + two tab stops for one action. — Drop role/tabIndex/keydown from the container; keep the sr-only button. *(S5-adjacent)*
53. **[MED]** `apps/web/src/pages/courier/DeliveryPage.tsx:509` + `EarningsPage.tsx:152` — `text-[var(--brand-on-primary)]` — that token is defined NOWHERE (real token is `--color-on-primary`); text falls back to inherited `--brand-text` = #0a0a0a on darkened-gold primary (~2:1) under sunlight, on the main "Mark as Picked Up" CTA. Also ActivationPage. — Use `--color-on-primary`. *(S6)*
54. **[MED]** `apps/web/src/components/pwa/InstallPrompt.tsx:123-125` (+ main.tsx:81) — The app-wide banner yanks focus into itself on appear (iOS 1.5s after load; Android on `beforeinstallprompt`), stealing focus from users mid-typing in checkout/login/courier. — Non-modal nudge: `role="region"` + polite announcement, no autofocus.
55. **[MED]** `apps/web/src/components/pwa/InstallPrompt.tsx:176-177,187` — Fixed bottom-sheet at `z: var(--z-toast)` (500) sits above `--z-modal` (400) and every sticky bottom bar: overlaps open-modal action areas, the storefront sticky cart CTA, the courier tab bar, and the slide-to-deliver control; its "faster ordering" copy also shows on /admin and /courier. — Drop below modal; gate per surface.
56. **[MED]** `apps/web/src/components/voice/VoiceSearchButton.tsx:239-250` — Keyboard push-to-talk has no end condition on blur (no `onPointerCancel`): press Space, Tab away → the mic records indefinitely (privacy). — `onBlur={stopCapture}`.
57. **[MED]** `apps/web/src/components/voice/VoiceSearchButton.tsx:174-179` — `TransformersTranscriber` is cached with the locale at first press and never rebuilt; switching sq→en keeps transcribing the old language. — Reset `transcriberRef` on `effectiveLocale` change.
58. **[MED]** `packages/ui/src/hooks/use-online.ts` — Exists but is not exported from the barrel (`index.ts:26-34`) and no courier page consumes it; all offline failures collapse into generic "Failed to fetch". — Export it; add an offline banner on Tasks/Delivery.

### a11y / dialogs / nested-interactive (admin)
59. **[MED]** `apps/web/src/pages/admin/MenuManagerPage.tsx:892,957,1100` — Three hand-rolled modals: no `role="dialog"`/`aria-modal`, no Escape, no focus trap; the import modal (1100) also can't be closed via backdrop (`pointer-events-none`). PromotionsPage:411, SupplyLibraryPage:378, CouriersPage:531 lack Escape/focus-trap too. — Migrate to `ResponsiveDialog`. *(S5)*
60. **[MED]** `apps/web/src/pages/admin/MenuManagerPage.tsx:1079-1086` — "Available for order" is a click-handler on a `<label>` with no input/role/tabIndex — not keyboard operable; duplicates the shared `Toggle` (as does the mini switch at 818-824). — Use `Toggle`.
61. **[MED]** `apps/web/src/pages/admin/CRMPage.tsx:222-229,280-283` — Customer analytics are reachable only by clicking a `<tr>`/`<div>` with no tabIndex/role (desktop row even has a focus-visible ring but is never focusable); the expanded data is not visible otherwise, so keyboard users can't access it at all. — Real button semantics on the row toggle.
62. **[MED]** `apps/web/src/pages/ClaimPage.tsx:151-154` — "This isn't my restaurant — remove it" irreversibly deletes the preview on a single tap, no confirmation. — Confirm dialog.
63. **[MED]** `apps/web/src/pages/admin/DashboardPage.tsx:805-813` + `CRMPage.tsx:365` — Order-detail dialog shows raw enums (`IN_DELIVERY`, paymentMethod) unlocalized; `OrderCard.tsx:71-76` already solved this. — Reuse the statusLabel catalog.
64. **[MED]** `apps/web/src/pages/client/MenuComparePanel.tsx:84-102` — `role="dialog" aria-modal` sheet with no Escape and a non-focusable div backdrop; keyboard close is only the "Done" button (the MenuPage detail modal got the Escape fix; this didn't). — Add Escape/backdrop. *(S5)*

### CLS / layout
65. **[MED]** `apps/web/src/pages/client/MenuPage.tsx:743-756` — Hero renders `h-[150px]` then jumps to `h-[240px]` (`md:320`) when the hero photo's `onLoad` flips state — a 90-170px layout shift pushing the whole menu down after first paint on every venue with a hero. Same class: detail-modal hero (1341-1346) and ProductDetailSheet (109-115) reserve no height. — Reserve final height up front (ride the /info payload).

### Design-system drift / architecture
66. **[MED]** `packages/ui/src/components/molecules/{ResponsiveDialog,ConfirmDialog}.tsx` + `client/ProductDetailSheet.tsx` — Three independent modal scaffolds hand-rolling portal+backdrop+Escape+scroll-lock with divergent behavior; none traps focus despite `aria-modal`. — Extract one modal-shell primitive with focus trap (OTPModal composes ResponsiveDialog correctly — the pattern to follow). *(S5)*
67. **[MED]** `apps/web/src/pages/admin/CRMPage.tsx:390` — `rgba(var(--brand-primary-rgb, …), …)`: `--brand-primary-rgb` is defined nowhere, so heatmap chips are always the default blue regardless of tenant brand. — Use `color-mix` on `--brand-primary` (as AnalyticsPage:493-495). *(S6)*
68. **[MED]** `packages/ui/src/components/molecules/MapLibreBase.tsx:124,179,190,241,284,291` (+MapWithPin:72, MapWithRadius:40) — `getCSSVar('--brand-primary', '#ea4f16')` ×11: the fallback is the OLD orange identity; the current default brand is `#d69a3d` (tokens.css:40). — Point all fallbacks at one shared `DEFAULT_BRAND` constant.
69. **[MED]** z-index: two uncoordinated scales — token layers 100–500 have 17 semantic usages vs 55 raw Tailwind `z-10..z-50` + magic `z-[999]` (CheckoutPage:744, DeliveryPage:330) + raw 350/450 (TourHint:172,182); `var(--z-*)` has exactly 1 direct consumer. — Migrate overlays to semantic classes; ESLint-ban `z-[N]`.
70. **[MED]** `packages/ui/src/theme/tokens.css:289-292` vs `ResponsiveDialog.tsx:77,113` — Sunlight mode flattens only the deprecated `--elevation-*` aliases; canonical `--elev-*` untouched and the `[class*="shadow"]` catch-all misses inline `style={{boxShadow:'var(--elev-4)'}}` — dialogs keep soft shadows in AAA sunlight. — Override `--elev-*` too.
71. **[MED]** `packages/ui/src/components/molecules/TourHint.tsx` — `TourProvider` is mounted (main.tsx:77) but `useTour`/`startTour` have zero callers; the spotlight/tooltip machinery (z-350/450, hardcoded-English "Step {n}/{m}") is unreachable — ~200 of 271 lines dead. Only `HintCard` is alive. — Unmount the provider, keep HintCard.
72. **[MED]** `packages/ui/src/hooks/use-voice-order.ts` (97 lines) — Zero callers; the shipped voice path is VoiceSearchButton + WhisperProvider. Promise never settles if `stopListening` precedes a final result; no unmount cleanup. — Delete.
73. **[MED]** `packages/ui/src/theme/tokens.css:398` — `html[data-theme="dark"] [data-skin="paper"]` night-paper variant: `data-theme` is set nowhere (grep=0); only the `prefers-color-scheme` copy (411) can fire — a dead selector masquerading as an API. — Delete or wire the attribute.
74. **[MED]** Theme architecture — four+ uncoordinated axes with no arbiter: tenant inline vars (ThemeProvider), manual `data-surface="dark"`, `prefers-color-scheme`, `data-sunlight` (`!important` war, tokens.css:263), `data-skin="paper"` (proximity-wins). Each pair is hand-reasoned in comments; one (paper×data-surface, #32) is already wrong. — Single mode-resolution layer + a mode-matrix AA test.

### i18n gate blind spots
75. **[MED]** `packages/ui/src/lib/i18n.ts:38` — Missing key falls back to `hit || fallback || key`: silent English in prod (console.warn dev-only); 63 `t('key')` calls have NO fallback arg → a miss leaks the raw key to the UI. — Require fallback via lint or make the coverage gate `--strict`.
76. **[MED]** `scripts/i18n-parity.ts:45` + `apps/api/scripts/verify-i18n-coverage.ts:66` — Both gates regex only quoted literals `t('…')`; the 21 dynamic-template sites (e.g. CourierRoutes.tsx:23 `t(\`courier.${d.key}\`)`, fallback-less) are invisible to every gate. — Per-family exhaustiveness check or enum-typed keys.
77. **[MED]** `.husky/pre-commit:21` — The parity gate runs only when `i18n-catalog.ts`/`i18n.ts` are themselves staged; adding new `t()` calls in app code never trips it locally (only later in CI). — Trigger on any staged `.tsx`, or always run (<2s).

### i18n hardcoded strings
78. **[MED]** `apps/web/src/pages/admin/AuthCallback.tsx:17,25,28,34` — Whole page untranslated ('Missing login code.', 'Login failed.', 'Signing you in…'). — Route through `t()`.
79. **[MED]** `apps/web/src/components/media/MediaGallery.tsx:89,128,137,145` — aria-labels "Product media"/"Previous media"/"Next media"/"Choose media" English-only; sq/uk screen-reader users get nothing. — `t()` them.
80. **[MED]** `apps/web/src/pages/admin/CouriersPage.tsx:169,195,286,301` — "Ftesë për Korrier / Courier Invite:" clipboard text, `setError('Failed to load couriers')` shown in the EmptyState, "Link", "Kodi i Sigurisë / Security Code (16 chars)" all bypass `t()`.
81. **[MED]** `apps/web/src/pages/admin/MenuManagerPage.tsx:538,975,979-980,1000,1066` — "File too large (max 10MB).", "JPG/PNG", "4:3 ratio, max 5 MB", placeholder "e.g. Margherita Pizza", English TASTE_LABELS in title attributes.

---

## LOW (condensed)

- **[LOW]** `OrderStatusPage.tsx:514` + `MenuPage.tsx:1043` — Offline banner `--color-warning` with `--color-on-primary` = 3.19:1 (dark-mode warning token 1.67:1); amber-on-tint busy banner ~3.1:1. — Fixed dark ink on warning.
- **[LOW]** `CheckoutPage.tsx:626` — Active crypto payment chip `--brand-primary-readable` on `--brand-primary` (same hue) = 1.00:1; flag-dark but ships broken when flipped.
- **[LOW]** `OrderStatusPage.tsx:799-820` — Nutrition snapshot card keys off `order.kcal_total`/`protein_mg_total` — no producer exists repo-wide; permanently-dead 22-line branch (mislabels `_mg_` as "g"). — Delete or wire.
- **[LOW]** `apps/web/src/components/client/SatelliteMap.tsx` — Zero importers (footer map removed 2026-06-30); only remaining arcgisonline egress. — Delete.
- **[LOW]** `ClientLayout.tsx:110,128` — Branding-draft branch early-`return`s after `addEventListener('message', …)` without returning cleanup — listener leak per effect run in preview mode.
- **[LOW]** `MenuPage.tsx:296,1119` — A `filterAllergen` persisted before the allergen freeze still suppresses Chef's Picks while the chips that could clear it are flag-hidden — stale localStorage hides Chef's Picks forever.
- **[LOW]** `PaymentSection.tsx:39` — Cash-method subtitle renders `t('checkout.place_order')` = "Place order" (wrong key pasted).
- **[LOW]** `MenuPage.tsx:1075` — Fetch-error headline always says catalog "Menu unavailable" (implies unpublished); only the small hint differs. — Separate error key.
- **[LOW]** `MenuPage.tsx:817` ("min" hardcoded), `:1243` ("  vs  " joiner), `OrderStatusPage.tsx:22-31,335` (no `PICKED_UP`/`SCHEDULED` keys → raw English toast in sq/uk).
- **[LOW]** Clipboard: false "Copied!" + unhandled rejection on denial — `DashboardPage.tsx:518-520`, `CouriersPage.tsx:170`, `BrandingPage.tsx:252`. — `.catch` + confirm only on success.
- **[LOW]** `BrandingPage.tsx:78` — Native `alert()` for file-too-large while everything else uses `showToast`.
- **[LOW]** Muted text further dimmed with opacity (sub-AA on dark/paper) — `BrandingPage.tsx:495` (opacity-60), `AnalyticsPage.tsx:397` (opacity-50), `MenuManagerPage.tsx:1042` (opacity-50). — One muted token, no opacity stacking.
- **[LOW]** Validation theater / dead code — `MenuManagerPage.tsx:9` `AnySchema` unused, `DashboardPage.tsx:12,18` `z.any()`, `AnalyticsPage.tsx:8` `z.custom`; `AllergenEditor.tsx:119-158` `ReadinessIndicator` zero callers; `CRMPage.tsx:35,45` `navigate`/`onboardingDone` unused; `DashboardPage.tsx:42,79` `readiness.placeOrder` never set.
- **[LOW]** Unmount leaks — Telegram poll setTimeout chains never cancelled (`LoginPage.tsx:47-63`, `MenuFirstOnboarding.tsx:148-165`); debounce timer never cleared (`DashboardPage.tsx:130-134`).
- **[LOW]** `MenuManagerPage.tsx:68,115,321` — /owner/settings fetched 3× per page mount (page + 2 embedded widgets). — Fetch once, pass down.
- **[LOW]** `RecipeEditor.tsx:65` — parent setState invoked inside `useMemo` (side effect during render; latent React warning); currently dead since `onBomAllergensChange` isn't passed.
- **[LOW]** `PromotionsPage.tsx:194,296-298` — dates forced 'en-GB'; "{n} active of {m}" assembled from 3 concatenated `t()` fragments (word order breaks in sq/uk). `AnalyticsPage.tsx:200-207` — heatmap day labels 'Mon'…'Sun' hardcoded (CRMPage:391 localizes correctly).
- **[LOW]** `TaskCard.tsx:46-66` — 30s offer window has no SR exposure (no `role="timer"`/`aria-live`); expiry auto-decline silent.
- **[LOW]** `use-sound.ts` (unexported, unconsumed) duplicates `apps/web/src/lib/hooks.ts:44` `useSound` and rebuilds `sounds` per render, defeating its own `useCallback`. — Delete.
- **[LOW]** `CourierInvitePage.tsx:175,193` — hardcoded `label="Email"`, English `title="+355 followed by 7-14 digits"`.
- **[LOW]** `VoiceSearchButton.tsx:258-269` — third inconsistent primary fallback `#4f46e5` (indigo) + hardcoded `rgba(79,70,229,.18)` keyframe injected per instance inside the `<button>`.
- **[LOW]** Hover-on-touch: `hover:hover:` (compiles to `:hover:hover`, still fires on touch) instead of `[@media(hover:hover)]:hover:` — `DeliveryPage.tsx:461,468,509`, `HistoryPage.tsx:99,127`, `EarningsPage.tsx:152,171,211`. Sticky lift on tap.
- **[LOW]** `ShiftPage.tsx:58` — messenger save failure fully silent. `HistoryPage.tsx:122` — `tabIndex={0}` on non-interactive card (keyboard noise).
- **[LOW]** `InstallPrompt.tsx:90,128` — "Not now" persists forever (no TTL/re-ask); mounted OUTSIDE the ErrorBoundary (main.tsx:78-81) so a throw in it blanks the whole app.
- **[LOW]** `CourierRoutes.tsx:41-55` — active-delivery branch renders no header, so `SunlightToggle` is unreachable on the screen most used in direct sun.
- **[LOW]** i18n identical-to-en (gate can't see): 20 sq / 9 uk values equal English — genuine misses e.g. `allergen.mustard` sq="Mustard", `admin.embed_iframe` sq="Embed iframe", uk `admin.live`="Live". `constants/allergenColors.ts:7-17` `ALLERGEN_COLORS` (19 raw hex) zero users. `atoms/Icon.tsx`+`atoms/Skeleton.tsx` reachable only via the dead atoms barrel.

---

## Design-system improvement notes (concrete)

1. **Delete the dead third theming system + shadow-atoms cluster (~650 lines, grep-verified importer-free).** `theme/index.ts` (79) + `atoms/{Button,Input,StatusBadge,Skeleton,Icon,index}` (~235) + `use-voice-order` (97) + TourHint machinery (~200) + `SatelliteMap` (31) + `ALLERGEN_COLORS` (11). Removes 2 of the 3 competing answers to "what is a Button / a StatusBadge / the brand color".
2. **One modal-shell primitive with a focus trap.** 3 ui scaffolds + ~10 page modals hand-roll `fixed inset-0` overlays (15 occurrences); **zero focus traps exist repo-wide**; Escape/scroll-lock coverage is divergent. Extract ResponsiveDialog's chrome (portal, backdrop, Escape, scroll-lock, + trap) and have ConfirmDialog / ProductDetailSheet / page modals compose it, as OTPModal already does. Fixes the a11y gap once instead of 13×.
3. **Single on-primary / brand-default truth.** `--color-on-primary` has 18 consumers but MenuPage hand-rolls it 6× (wrong on light themes) and 11 map fallbacks pin the retired `#ea4f16` while the real default is `#d69a3d`. Export one `DEFAULT_BRAND` and reference it in every `getCSSVar` fallback; swap MenuPage's `color-mix` for the token. Kill the three undefined tokens (`--brand-on-primary`, `--brand-primary-rgb`) that silently defeat theming (S6).
4. **Finish z-index semantics.** 17 semantic vs 55 raw + 4 magic (999/450/350). The Tailwind mapping already exists (`ui/tailwind.config.ts:107`) — mechanical migration + an ESLint ban on `z-[\d+]`. Today's conflicts (InstallPrompt over modals, #55) are accidental ordering, not design.
5. **Close the two i18n-gate blind spots the parity gate provably misses:** (a) dynamic-template keys — 21 sites invisible to both gates; (b) identical-to-en translations — 29 pass silently. Both are ~20-line additions to `scripts/i18n-parity.ts`. And backfill the 8 `voice.search.*` keys before the voice feature is committed, or CI goes red (#29).
6. **Add a mode-matrix AA test for theme-axis interactions.** The paper×data-surface break (#32) and the sunlight×`--elev-*`/`--color-*` misses (#8, #70) are all "two axes composed, nobody checked". A unit test that renders the token cascade for each of the ~8 real mode combinations and asserts AA on the status-text pairs (`contrastRatio()` already exists in `palette.ts:53`) would catch all three.

---

*Verified-clean (checked, no finding worth reporting):* OnboardingPage, dashboard-utils.ts, Toggle/ColorInput/AdminUI, MediaManager, AccessRequestForm, PrivacyPage, OTPModal, CartProvider merge-key, the `maps.tsx`/`maps.ts` consolidation shim (all 5 consumers migrated correctly), `cinematic.ts` (does not duplicate motion.ts), useReorder pricing (reuses reconcileCart), LiveDot IntersectionObserver cleanup, DeliveryPage GPS-heartbeat cleanup, and the new Wave-1 unit tests (no false-green patterns). derivePalette remains the single genuine live path for storefront theming with AA enforcement built in.
