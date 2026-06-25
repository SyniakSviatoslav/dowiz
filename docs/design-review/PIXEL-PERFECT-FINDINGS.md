# dowiz — Pixel-Perfect & Design-System Consistency Findings

> Source: exhaustive state capture of the **demo / "Dubin & Sushi"** tenant on staging
> (`audit/full-capture/`, 42 states — every page × {desktop 1280, mobile 390} × {default,
> loading, error} + product modal + cart drawer), graded against the live token system
> (`packages/ui/src/theme/tokens.css`). Captured 2026-06-25 on the post-fix build (commit b8041f45).

Live token SoT: type **12/14/16/18/22/28/36px** (`--text-xs…3xl`) · spacing **4px grid** ·
radius **4/8/12/16/24/full** · elevation **`--elev-1…4`** (soft) · color = brand/semantic tokens.

---

## ⚠️ Capture-artifact correction (read first)

Verified after the vision pass: **the Tabler icon webfont did not paint during the Playwright capture**
(it loads from a CDN; `networkidle` doesn't guarantee webfont glyphs are rendered). Proof: in
`m-client-menu.png` the cutlery placeholder glyph, the FAB `+`, the search/cart icons, and the card
"taste" glyphs are all blank while text/gradients/colors render normally.

**Therefore a large share of these findings are capture artifacts, not product bugs** and should NOT be
"fixed": "empty header squares", "empty add-circle / FAB", "ghost-circle media placeholder",
"sub-pixel glyph noise" on cards, "qty stepper has no −/+", "faint close button", supplies "stray glyph
prefixes". The components render their icons correctly for real users when the CDN serves the font.
(Several other findings were also false positives on inspection: supplies "unconfirmed" is already amber
`--color-warning` — the green was the *ingredient-type icon*; the courier map-status default was already
correct; the storefront loading/error states are good and branded.)

**The real issue this exposes (A5 below) is genuine and worth fixing.**

## Verification log (what survived code/compute checks)

Each visual finding was checked against code or computed values before any fix. The screenshot pass
had a **high false-positive rate** — most "broken-looking" items are real components degraded by the
capture, or visual estimates that compute fine. **Verified real** issues are few; fix only those.

| Finding | Verdict | Evidence |
|---|---|---|
| Empty icons / ghost-circle media / "stepper has no ±" / "glyph noise" | ❌ **Artifact** | Tabler webfont didn't paint in the Playwright sandbox (CDN font); icons render for real users. |
| Storefront loading skeleton "missing" / error "dark" | ❌ **Artifact** | Both are branded & well-built — wrong-slug capture; re-validated (`state-client-menu-*`). |
| Supplies "unconfirmed" badge is green (inverted) | ❌ **False** | Already `--color-warning` (amber); the green was the ingredient-type icon. `SupplyLibraryPage.tsx:415`. |
| Courier map status defaults phone-less → online | ❌ **False** | Status is normalized upstream (`:179`); `:213` is correct (typecheck rejected the "fix"). |
| Muted text fails AA on dark cards | ❌ **False** | Computed `#959a93`: 4.57–6.21:1 on all default surfaces (≥4.5 AA). Passes (thin on raised). |
| CSP `font-src` omits jsdelivr → blank icons | ⚠️ **Harmless** | The jsdelivr-less CSP (`headers.ts`) doesn't apply to icon pages; storefront uses the correct CSP. |
| Two elevation systems (legacy `--elevation-*`) | ✅ **Real — FIXED** | 32 usages; aliased to `--elev-*`. |
| Icons = unpinned third-party CDN, no fallback | ✅ **Real — partial fix** | `@latest` pinned to `@3.31.0`; self-host pending (needs install). |
| Type scale bypassed by arbitrary `text-[Npx]` | ✅ **DONE (221/221) + guardrailed** | Fully migrated to `text-step-*` + `--text-2xs`; error-level ESLint rail (red→green); validated on staging. |
| Font drift: DESIGN.md (DM) vs tokens.css (Inter) | ✅ **Real — FIXED** | Coherent as-shipped (Inter base + serif via preset); DESIGN.md §2/§8 reconciled. |
| Button hierarchy fragmentation | 🔶 **Likely real — re-confirm** | Estimate confounded by missing icons; re-judge on an icon-rendering capture before consolidating. |

**Takeaway:** the UI is in materially better shape than the raw screenshot pass implied. The genuine
backlog is small: self-host icons, the type-scale migration, the font-drift reconcile, and a
re-confirmation of the button/segmented-control consolidation on a clean (icon-rendering) capture.

## A. Systemic design-system breaks (code-grounded — the root causes)

These are whole-system inconsistencies confirmed by grep across `apps/web/src` + `packages/ui/src`.
Fixing these once removes most per-screen findings below.

### A1 — Type scale is bypassed everywhere: arbitrary `text-[Npx]` · ✅ **DONE (221/221) + guardrailed**
**Fully migrated** (`7039fd94` + `48630022`, validated on staging): on-scale → `text-step-*`; the micro
tier (`text-[10/11px]` ×126) → new `--text-2xs` (11px) + `text-step-2xs`; the rarer off-scale
(7/8/9/13/15/20/24/26px) snapped to the nearest step (7px allergen badges → 11px = legibility win;
checkout h1→step-2xl/28, h2→step-xl/22, hierarchy preserved). **Locked** with an error-level ESLint rail
`local/no-arbitrary-font-size` (red→green proven). Visual validation: Supplies badges + storefront cards
+ checkout heading hierarchy all clean, no line-height/pill regressions. Original note:

The scale exists as tokens (`--text-xs…3xl`) but call-sites hardcode arbitrary pixel sizes instead.
Counts: `text-[10px]` ×77, `text-[11px]` ×49, `text-[13px]` ×21, `text-[9px]` ×19, `text-[8px]` ×3,
`text-[7px]` ×2, plus one-offs 15/20/24/26px. Even the on-scale sizes are arbitrary literals
(`text-[12px]` ×16, `text-[14px]` ×22) rather than `text-xs/sm`. 7–11px is also below the legible/AA
floor. **Concentrated in product UI:** CheckoutPage (22), MenuManagerPage (21), AnalyticsPage (18),
MenuPage (17), SupplyLibraryPage (16), RecipeEditor (14), ProductCard (9).
→ **Fix:** map every `text-[Npx]` to a `--text-*` token; eliminate 7/8/9/10/11/13/15/20/24/26px.
Add an ESLint rail (extend the existing no-arbitrary-tailwind rule) so off-scale sizes can't reappear.

### A2 — Two elevation systems in parallel · **High**
Legacy `--elevation-1…4` / `shadow-elevation` used **32×** across 10+ files — including shared atoms
(Button, Tooltip, CurrencySwitcher) and pages (MenuManager, SupplyLibrary, Dashboard, DeliveryPage,
TaskCard) — alongside the canonical soft `--elev-1…4`. The two have different shadow weights, so the
same conceptual "card" casts visibly different shadows across screens.
→ **Fix:** migrate all `--elevation-*`/`shadow-elevation` → `--elev-*`; delete the legacy tokens.

### A3 — Hardcoded hex / rgba in components · **Med**
Raw hex appears in product UI: StateChip (3), ColorInput (3), MenuPage (3), DashboardPage (3); plus
13 `rgba()` literals in `.tsx`. (The bulk — NomadicScene 25, PaperIllustration 11, PaperScene 7,
MapLibreBase 6 — is decorative/illustration/map and lower priority, but should still be tokenized or
explicitly scoped.) `#ea4f16` (= brand-primary) and `#D97706` (= --color-warning) are token values
written by hand → drift risk.
→ **Fix:** replace product-UI hex with `var(--brand-*)`/`var(--color-*)`; scope illustration palettes
to a documented constants module.

### A5 — Icons depend on a third-party CDN with no fallback (several pinned to `@latest`) · **High**
All Tabler icons load from `cdn.jsdelivr.net/npm/@tabler/icons-webfont` — the SPA (`apps/web/index.html`),
the SSR client/admin shells (`ssr-client-renderer.ts`, `ssr-renderer.ts`), and ~13 static admin pages.
No local/vendored copy → if jsdelivr is slow, blocked, or down, **real users see the exact blank-icon
storefront the capture showed** (especially relevant for the Albanian mobile-first market). It's also a
supply-chain surface, and the SSR client shell + static admin pages used **unpinned `@latest`** (version
drift / unreviewed updates). **Partial fix shipped:** all `@latest` refs pinned to `@3.31.0` to match the
SPA. **Proper fix (needs a network install, do in CI):** `pnpm add @tabler/icons-webfont@3.31.0`, import
the CSS in the app entry, drop the CDN `<link>`s → self-hosted, offline-safe, single pinned version.

### A4 — Font drift: spec vs reality · ✅ **RESOLVED**
The "drift" was only in the contract: the system is coherent — `:root` ships **Inter** (admin/courier/
base, a deliberate dense-UI choice) and **client/branded surfaces apply a serif display heading via the
active theme preset** (DM Serif Display / Cormorant / Playfair / Fraunces), which is why the storefront
headings render serif. DESIGN.md §2 + §8 updated to describe this as-shipped model. No code change
needed (changing the live admin font app-wide would be risky and worse).

---

## B. Client storefront — per-state findings

> ✅ All client states re-captured with the correct slug (`demo`) and a working add-to-cart→cart→
> checkout flow. **Loading skeleton and error state are validated as GOOD** (see below) — the earlier
> "dark/missing" report was a wrong-slug capture artifact, now corrected.

**Validated state quality (strengths — keep):**
- **Loading skeleton** is excellent: branded tenant theme, card-grid skeletons that mirror the real
  layout (image tile + name + price lines) + chip skeletons → no layout jump. ✅
- **Error state** is well-composed AND on-brand: tenant pink/cream, utensil glyph, "Menu e
  padisponueshme" + body + "Provo përsëri" retry CTA. ✅ (The dark-theme fallback only occurs if
  `/info` itself fails — a total-theme-outage edge case; `ClientLayout.tsx:102` → `setTheme(null)`.
  **Downgraded to Low/edge-case**, not the Critical the first pass implied.)
- **Cart drawer** ("Shporta") and **checkout form** ("Përfundimi") render correctly with a success toast.

**Confirmed defects:**

- **Hero venue-name contrast** — "Dubin & Sushi" white over the grey→pink gradient band is likely
  <4.5:1 (both breakpoints). **High/AA.** Fix: scrim or solid surface behind the name.
- **Media placeholder is a bare ghost-circle gradient** on cards AND the modal header — reads as
  broken/loading, not "no photo". **High** (one branded empty-media tile token fixes cards + modal +
  empty-cart glyph at once). Ties to known image-glyph backlog.
- **Qty stepper is a flat pill showing "1" with no visible −/+ affordance** — reads as a static
  field. **High.** Fix: explicit −/+ at 44px, match the primary-button height/elevation.
- **Sub-pixel glyph noise** where allergen/`SHIJE` option rows render as illegible micro-ticks on
  cards and in the modal. **Med.** Fix: hide-when-empty.
- **Add-FAB** overlaps the price block and isn't on the 4px grid. **Med.** Pin to 12px offsets.
- **Modal close affordance** is a faint ghost-circle with no visible ×, possibly <44px. **Med.**
- **Selection idiom split** — active category = 2px underline, filter pills = `full` pill. **Med.** Pick one.
- **Two type families** — serif headings (hero/empty/error) vs Inter cards/body. **Med.** Scope serif or unify (ties to A4).
- **Cart drawer** shows only "Totali" — no itemized subtotal/delivery-fee line (checkout has it below
  the fold). **Med.** Surface the same breakdown in the drawer for transparency.
- **CartFAB sticky bar peeks behind the cart drawer** (a crimson "Shporta · 1500 ALL" bar bleeds under
  the centered drawer on desktop). **Med (z-index/overlap).**
- **Checkout step-indicator is a bare empty grey circle** next to "Përfundimi" — reads unfinished. **Low.**
- **Checkout Messenger country-code `<select>` shows a cryptic "—"** placeholder. **Low.** Use a labeled placeholder.
- **Checkout desktop left-aligns a ~520px form with a large empty right gutter** (same imbalance as
  Settings). **Med.** Center the form column.
- **Delivery fee:** checkout total = subtotal + **hardcoded 200** (`CheckoutPage.tsx:342`). Coincidentally
  matches demo's `deliveryFee:200`, but ignores per-venue fee + `freeDeliveryThreshold:2000`. **Med (flagged earlier).**
- **Empty header square button** (no glyph) appears on the client header too — same cross-cutting empty-icon issue (F#3).
- **Reference component (keep as the bar):** the primary CTA "Shto në Shportë" / "Porosit" — solid
  brand-primary, white text, AA-safe, pill radius. Normalize stepper/pills/FAB up to its polish.

## C. Admin-core — per-state findings (orders, menu, settings, branding, couriers, onboarding)

- **KPI tile number colors are decorative, not semantic** (orders: orange/teal/blue/orange) — blue
  isn't even a defined token. **High.** Map to one neutral or a documented semantic scale.
- **Status-badge color sprawl** — Refuzuar(red)/Në dorëzim(blue)/Konfirmuar(blue)/Pa OTP(amber)/
  Rep(blue text)/Konfirmo:0m(amber) compete; blue is off-token. **High.** Map to the fixed status palette.
- **Product-image placeholders are empty dark squares** (no glyph/skeleton) — reads broken. **High.**
- **Category chips leak raw slug IDs** ("IMG-Cat-1782365739… (0)") and overflow the right edge with no
  scroll cue. **High.** Render display names + scroll affordance.
- **Branding ships a raw native `<input type=file>`** ("Choose File / No file chosen") — the only
  unstyled native control in admin. **High.** Style to the onboarding dropzone.
- **Couriers: green "Online" for phone-less, 0-delivery couriers** + a separate neutral "60 online"
  pill = two visual languages for one concept; semantics inverted. **High.** Grey "pending" state.
- **Desktop dead-space** on Settings / Onboarding / Branding-preview (content pinned top-left). **Med.** Center focused single-column screens.
- **Settings map "loading" is bare centered text**, not a skeleton. **Low→Med.**
- **Two active-segment treatments** (Live/Historiku gold-border vs Të-Gjitha bordered chip vs Menu category gold-border). **High.** One segmented-control token.

## D. Admin-secondary — per-state findings (analytics, promotions, crm, supplies, activation)

- **Analytics bar chart has no zero baseline / gridlines / Y-axis** — 0k and 4k bars read identical. **High (data-viz).**
- **"Export CSV" rendered as a text-link AND a bordered button on the same screen.** **High (consistency).**
- **Promotions primary CTA is an amber OUTLINE** (desktop) / **full-width solid** (mobile) /
  **small solid pill** (in-card) — three treatments of one action; outline appears nowhere else. **High.**
- **CRM LTV wraps to two lines** ("7600 / ALL") — forced-wrap layout bug; **header "Porosia e fundit"
  wraps** too; **mobile search placeholder truncates** ("Kërk"). **High/Med.**
- **Supplies badge system fragmented + semantically inverted** — type(grey ~10px)/allergen(amber)/
  status "pa konfirmim"(**green = success for an UNCONFIRMED state**). **High.** Re-map unconfirmed off green.
- **Supplies filter chips show stray broken-icon glyph prefixes** ("- Përbërësit", "` Salcat") + an
  empty icon-only button + empty info-banner circle. **High.**
- **KPI delta text wraps in one tile but not others** → jagged tile grid (desktop + mobile). **Med.**
- **CRM toolbar controls (search/select/button) render at 3 different heights/radii.** **Med.**
- **Activation (esp. mobile) is the lane's quality benchmark** — clean tabs, proper step states, canonical CTA. Match the others to it.

## E. Courier — per-state findings (tasks, shift, earnings, history)

- **Desktop dead-void (CRITICAL, lane-wide)** — all 4 desktop screens render the ~410px mobile column
  anchored to the top third; 55–70% empty canvas (worst: history ~70%, tasks ~65%). Fix: vertically
  center or device-frame the courier content on desktop.
- **Muted-grey text fails AA on every screen** — empty-state subcopy, KPI labels (SOT/KJO JAVË), "0 dërgesa", shift helper. **High.** One token-tier fix.
- **Empty-state card border is dashed** (tasks/earnings/history) while shift cards / KPI tiles / payouts
  wrapper are solid → empty states read as "broken". **High.** Unify to solid + `--elev-1`.
- **Shift card has ~80px dead padding above "Fillo Turnin"** (worse on desktop). **High.** Center on grid.
- **"Offline" status pill** carries no semantic color and a low-contrast label. **Med.**
- **Button split** — filled-brand (Fillo Turnin) vs outline-brand (Ruaj); ensure shared height/radius. **Med.**
- **Consistent wins:** bottom tab bar, header, page-title type, active-tab amber hold across all 8.

## F. Cross-surface consistency ledger — the system-level fixes (do these first)

The same ~10 root issues generate most per-screen findings. Fixing the **component system** once
propagates everywhere — that is the path to a consistent design system.

| # | System break | Where it shows | Severity | Fix (one source of truth) |
|---|---|---|---|---|
| 1 | **Type scale bypassed** — arbitrary `text-[Npx]`, off-scale 7–11/13/15/20/26px | every product page (A1) | **High** | map all to `--text-*`; ESLint rail to block off-scale |
| 2 | **Button hierarchy fragmented** — same action as solid-pill / amber-outline / full-width / text-link | admin-core, admin-secondary, courier | **Critical** | one `<Button>` with primary/secondary/tertiary variants; replace all ad-hoc buttons |
| 3 | **Empty/broken icon & media placeholders** — mobile-header empty squares, product/courier/supplies thumbnails, branding swatches, info-banner & FAB empty circles, storefront ghost-circle | all four lanes | **Critical** | one media-placeholder token (`--surface-2`+glyph+`--border`); render or remove header icons |
| 4 | **Semantic/status color undisciplined** — blue used as status (off-token), decorative KPI colors, green "Online"/"pa konfirmim" for non-success states, allergen vs type vs status collisions | admin-core, admin-secondary, courier | **Critical** | define full status palette in DESIGN.md; one `<StatusBadge>`; re-map inverted greens |
| 5 | **Muted text fails AA (<4.5:1)** — courier (all), client hero/ETA, admin section headings | all lanes | **High** | retier the muted token to AA `--color-text-secondary` on dark surfaces |
| 6 | **Two elevation systems** — legacy `--elevation-*` ×32 vs `--elev-*` | shared atoms + pages (A2) | **High** | migrate all → `--elev-*`; delete legacy |
| 7 | **Selection-idiom split** — underline vs filled-pill vs bordered-chip for "selected segment" | client, admin-core | **High** | one segmented-control / chip token |
| 8 | **Empty-state card style split** — dashed vs solid borders | courier, promotions | **High** | one `<EmptyState>` (solid `--border` + `--elev-1` + glyph + CTA) |
| 9 | **Theme leakage only on total /info outage** — storefront falls to default dark when the theme fetch itself fails (menu-only failure stays branded ✅) | client storefront (edge case) | **Low** | error/loading boundary keeps last-known tenant theme / neutral branded shell |
| 10 | **Forced text-wrap + control-height drift** — LTV/delta/header wraps; toolbar controls 3 heights | admin-secondary, crm | **Med** | nowrap + reserved slots; one control-height/radius token |
| 11 | **Desktop dead-void** — narrow mobile column stranded top-third | courier (all), admin onboarding/settings | **Med** | vertically center / device-frame focused screens |
| 12 | **Hardcoded hex/rgba + font drift** | product UI + decorative (A3/A4) | **Med** | tokenize product-UI hex; reconcile DESIGN.md ↔ Inter |

### Suggested execution order (component-system-first)
1. **Foundation tokens** (1, 5, 6, 12): finish the type-scale migration + ESLint rail, retier muted-AA, kill legacy elevation, tokenize stray hex. Mechanical, high-leverage, low-risk.
2. **Three shared components** (2, 3, 4, 8): `<Button>`, media-placeholder, `<StatusBadge>` + status palette, `<EmptyState>`. Replacing ad-hoc usages removes ~60% of per-screen findings.
3. **Pattern unifiers** (7, 10): one segmented-control; nowrap + control-height token.
4. **Layout** (11, 9): courier/focused-screen desktop centering; theme-aware error/loading boundary.

This ledger is the backbone; sections B–E hold the per-screen specifics to verify each fix against.
