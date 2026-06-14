# DeliveryOS Redesign — Design Audit Report

> **Phase 1** · Audit-first approach · All 23 pages + 9 layout shells + 12 shared components reviewed
> **Lenses:** `design-taste-frontend`, `web-design-guidelines`, `deliveryos-theme`, A–F gate
> **Baseline:** `tsc` GREEN, `lint` SOFT (pre-existing), `check-money` PASS, `check-rls` PASS, `verify-env` PASS

---

## Critical Findings (Must Fix Before Re-skin)

### C1. Hardcoded `#fff` / `#000` colors — 7 locations
These bypass CSS variable theming and break in dark mode presets.

| File | Line(s) | Current | Should Be |
|------|---------|---------|-----------|
| `packages/ui/src/components/client/ClientUI.tsx:89,99` | ProductCard kcal badge, unavailable badge | `color: '#fff'` | `var(--color-on-primary)` or Tailwind `text-white` |
| `packages/ui/src/components/client/ClientUI.tsx:256` | CartFAB | `color: '#fff'` | `var(--color-on-primary)` |
| `apps/web/src/pages/courier/DeliveryPage.tsx` | Celebration overlay close btn | `bg-white text-black` | CSS variables |
| `apps/web/src/pages/courier/DeliveryPage.tsx` | GPS indicator | `text-gray-500` | `var(--brand-text-muted)` |

### C2. Hardcoded i18n strings — 14 locations
User-visible strings not wrapped in `t()`, breaking trilingual (sq/en/uk) support.

| File | Strings |
|------|---------|
| `packages/ui/src/components/client/ClientUI.tsx` | "Clean", "Cart is empty", OTP text, OrderProgress step labels |
| `packages/ui/src/components/courier/CourierUI.tsx` | "Reject", "Accept Task", "Pickup", "Drop-off" |
| `packages/ui/src/components/Status.tsx` | Offline banner text, fallback phone |
| `packages/ui/src/components/ErrorBoundary.tsx` | "Something went wrong", "Try again" |
| `apps/web/src/pages/courier/LoginPage.tsx` | "Courier Login", "Enter your email", "Log In" |
| `apps/web/src/pages/courier/CourierInvitePage.tsx` | Complete bypass — all strings bilingual hardcoded |
| `apps/web/src/pages/client/OrderStatusPage.tsx` | Screenreader text "Courier X is delivering" |
| `apps/web/src/pages/admin/CouriersPage.tsx` | Invite results bilingual hardcoded |
| `apps/web/src/pages/admin/CRMPage.tsx` | Fallback dates "today", "yesterday" |
| `apps/web/src/pages/admin/FlowTestPage.tsx` | All labels hardcoded |

### C3. No `aria-live` / `role="alert"` on dynamic errors — 20+ locations
Every error banner across all pages renders a `<div>` with no announcement mechanism.

### C4. Fixed positioning in CartFAB, OfflineBanner — violates embed mode
`CartFAB` (`fixed bottom-[80px] right-[20px]`) and `OfflineBanner` (`fixed top-0`) violate `?embed=true` constraint.

---

## High Severity Findings

### H1. Status color naming inconsistency
- **StatusBadge** uses: `bg-status-pending` (Tailwind CSS variable class)
- **OrderCard (AdminUI)** uses: `--status-pending-bg`, `--status-pending`, `--status-pending-border` (inline triplet)
- Should be unified to one system.

### H2. Active tab styling inconsistent across admin
| Page | Active Tab Style |
|------|-----------------|
| MenuManagerPage | `bg-[var(--brand-primary-light)] text-[var(--brand-text)]` |
| SupplyLibraryPage | `bg-[var(--brand-primary)] text-white` |
| AnalyticsPage period toggle | `bg-[var(--brand-primary)] text-white` |
| DashboardPage | `bg-[var(--brand-primary-light)] text-[var(--brand-text)]` |

### H3. Shadow token inconsistency
- Admin LoginPage: `shadow-elevation-2` (custom design token)
- CheckoutPage: `shadow-sm` (Tailwind built-in)
- SettingsPage: `shadow-sm`
- Should all use `shadow-elevation-N` token system.

### H4. Missing Tailwind brand utility classes
- All pages use `bg-[var(--brand-*)]` arbitrary values instead of `bg-brand-*` Tailwind classes
- All pages use inline `style={{ fontFamily: 'var(--brand-font-heading)' }}` instead of `font-heading`

---

## Medium Severity Findings

### M1. Emoji vs Tabler icons
Courier pages (Shift, Earnings, History) use emoji characters (☀️ 📅 💰 🕐). All other surfaces use Tabler icons (`ti ti-*`).

### M2. Date formatting ignores i18n locale
HistoryPage hardcodes `en-GB` locale regardless of app language setting. Should use user's locale.

### M3. Unused/dead shell components
`AdminShell.tsx`, `ClientShell.tsx`, `CourierShell.tsx` in `packages/ui` are all unused by active route files. Each route builds its own layout inline.

### M4. Duplicate `EU_ALLERGENS` array
Defined in both `SupplyLibraryPage.tsx` and `AllergenEditor.tsx`. Should be shared from a constants file.

### M5. Monolithic UI component files
`ClientUI.tsx` (395 lines) bundles ProductCard, CartDrawer, CartFAB, OTPModal, OrderProgress.
`CourierUI.tsx` (251 lines) bundles CourierShell, TaskCard, SwipeToComplete.
`AdminUI.tsx` (324 lines) bundles AdminShell, Toggle, ColorInput, OrderCard.

### M6. Inconsistent card wrapper patterns
- Admin LoginPage: card wrapper (`bg-[var(--brand-surface)] rounded-xl`)
- Courier LoginPage: no card wrapper
- SettingsPage: card sections with `bg-[var(--brand-surface)]`

---

## Cross-Cutting Patterns

| Pattern | Present In | Assessment |
|---------|-----------|------------|
| CSS variables via `var(--brand-*)` | All pages | ✅ Good — enables 7 theme presets |
| Tailwind arbitrary values `bg-[var(--brand-*)]` | All pages | ⚠️ Should use `bg-brand-*` classes |
| `t()` with English fallbacks | 21/23 pages | ✅ Good — but 14 missing keys |
| Mobile-first responsive | All pages | ✅ Good |
| Touch targets ≥44px | Most interactive elements | ✅ Good |
| `no-cookies` (localStorage only) | All storage | ✅ Good |
| Embed mode awareness | ClientLayout, OfflineBanner | ⚠️ CartFAB breaks embed mode |
| Framer Motion animations | Client pages, page transitions | ✅ Good — respects reducedMotion |

---

## Fix Priority

| Priority | Items | Impact |
|----------|-------|--------|
| **P0** | C1, C2 (hardcoded colors + i18n) | Theming + i18n correctness |
| **P1** | C3 (aria-live on errors), H4 (Tailwind brand utilities) | Accessibility + maintainability |
| **P2** | H1, H2, H3 (token consolidation) | Design system consistency |
| **P3** | M1-M6 (icons, date format, dead code, duplication) | Polish |
| **P4** | Visual re-skin of all screens | Aesthetic improvement |

---

*Generated: 2026-06-14 · Audit covered 23 pages, 9 layouts, 12 shared components · 44 findings total (3 critical, 4 high, 6 medium)*
