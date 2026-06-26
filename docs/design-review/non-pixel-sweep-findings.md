# Non-Pixel Sweep — cross-role findings ledger

Source: `e2e/tests/non-pixel-sweep.spec.ts` (Sense 1 axe + Sense 2 console), mobile
390px, against `https://dowiz-staging.fly.dev`, demo = Dubin & Sushi (`/s/demo`).
Run after the dev-auth cross-origin header strip (phantom CORS removed → true signal).
Live video per role journey under `e2e/artifacts/test-results/**`.

## Shipped + validated on staging (this pass)
1. F1 storefront category nav `role=tab`→`aria-current` — `aria-required-parent` 15→0.
2. `button-name` admin bottom-nav — 4→0 across all owner pages.
3. F2-A `--brand-primary-readable` token + F2-C opacity-on-muted — storefront `color-contrast` 7→**0**, admin/menu 2→0.
4. on-primary language chip → `--brand-primary-strong` — storefront's last contrast node → 0.
5. Toggle/range a11y — settings `aria-required-attr`/`aria-toggle-field-name`/`label` → 0.

**Storefront is now fully a11y-green (aria + contrast). Courier was already clean.**

Remaining = 3 structural/visual passes (need screenshot validation): on-primary `Button`
variant (systemic), F2-B status-badge tokens (theme-dependent), `nested-interactive ×49`
(MenuManager card pattern).

## A11y (Sense 1) — by impact, with the systemic root

| Finding | Impact | Where | Root (one fix → N surfaces) | Status |
|---|---|---|---|---|
| `aria-required-parent` ×15 | critical | storefront | menu category `role="tab"` w/o tablist | ✅ FIXED (976a1029, 15→0 verified) |
| `button-name` ×4 | critical | **every** owner page (dashboard/menu/orders/analytics/settings) | icon-only admin BottomTabBar (label '') → no accessible name | ✅ FIXED (db7d3223, 4→0 verified) |
| `color-contrast` | serious | **all roles** (storefront ×7, checkout ×2, dash ×5, menu ×2, orders ×5, analytics ×4, settings, courier ×2) | **3 distinct roots** (see below) | ▶ F2 (dedicated pass) |
| `nested-interactive` ×49 | serious | owner/menu | MenuManager — interactive nested in interactive | ▶ |
| `aria-required-attr` ×3, `label` ×1, `aria-toggle-field-name` | critical/serious | owner/settings | shared Toggle (role=switch) dropped aria-checked on non-bool + unnamed; MapWithRadius range unlabeled | ✅ FIXED (9230fde8, all→0 verified) |
| `select-name` ×1 | critical | client/checkout | transient (state-dependent select); not reproduced in re-sweep | ◦ watch |
| `scrollable-region-focusable` ×1 | serious | owner dashboard, orders | scroll container not keyboard-focusable | ▶ |

### F2 status (validated on staging)
- **A — DONE.** `--brand-primary-readable` (derivePalette `ensureContrast`, runtime-applied; computes e.g. `#c21b40` for the rose tenant). Storefront `color-contrast` 4→1 nodes; prices fixed. Modal + product-card prices consume it.
- **C — DONE.** opacity-on-muted removed on storefront + menu-manager counts. Admin/menu `color-contrast` 2→0.
- **A′ (on-primary, NEW systemic) — partially done.** Light text (`text-[var(--brand-bg)]`) on RAW `--brand-primary` fill = 4.3 for a mid-tone tenant primary. Root is the `Button` primary variant (`packages/ui/src/components/Base.tsx:20`) + ~18 sites. The one confirmed-visible node (LanguageSwitcher active chip, `I18nProvider.tsx`) fixed via the existing `--brand-primary-strong` (darker fill, documented AA pattern). The broad fix (Button variant → `--brand-primary-strong`, OR a derived `--color-on-primary`) is a design-system change needing cross-app screenshot validation — paired with B below.
- **B — deferred** (theme-dependent status-badge token pairs).

### F2 · color-contrast original roots

- **A — storefront price/accent.** Brand rose `--brand-primary` `#e11d48` used as TEXT
  on light surface `#f5eaf0` = 4.0 (need 4.5); primary button text `#fdf2f8` on
  `#e11d48` = 4.3. Fix: derive `--brand-primary-readable = ensureContrast(primary,
  surface, 4.5)` in `packages/ui/src/theme/palette.ts` + emit it in
  `apps/api/src/routes/public/theme.ts`; point primary-coloured TEXT at it (keep
  rose for fills). On-primary button text → nudge white to 4.5 on the button bg.
- **B — admin status badges.** `--status-info` `#2563eb` on `#18354f` = 2.44;
  `--status-warning` `#d97706` on `#293125` = 4.22. The status token *pairs*
  (fg + tinted bg) aren't contrast-checked. Fix: contrast-ensure each status
  fg against its `*-light` bg in the token derivation.
- **C — opacity-on-muted anti-pattern (systemic).** `opacity-70` / `opacity-40`
  applied to `--brand-text-muted` (which palette already tunes to *exactly* 4.5)
  drops it below AA — e.g. `#727b76` = 2.99 on `(0)` count spans. Fix: remove
  opacity on text-muted; **candidate guardrail** = ESLint `no-opacity-on-text`
  (tools/eslint-plugin-local) so it can't recur.

## Console / runtime (Sense 2)

| Surface | Signal | Verdict |
|---|---|---|
| storefront | 2× 404 (seed logo + product image) | data/seed — broken `<img>`; route to seed |
| storefront | CSP `connect-src` blocks SW/preload prefetch of Google Fonts CSS | real-ish; fonts still render via `<link>`; SW prefetch noisy. Consider self-hosting fonts (privacy + removes 3rd-party) |
| checkout | WebGL "GPU stall" perf warnings (paper/3D scene) | mostly benign; watch on low-end mobile |
| owner dashboard/orders | WS "closed before connection established" (+ JWT in `ws?token=` URL) | navigation-timing; **token-in-URL** worth review |
| owner dashboard/orders/settings | "Expected value to be of type number, but found null" ×many | shared number formatter / AnimatedNumber fed null — real, minor |

## Harness accuracy fix (this run)

`playwright.config` sets a global `x-dev-auth-secret` header; the browser also
attached it to third-party font/tile loads → CORS preflight rejections that NO
real user produces. `stripCrossOriginAuth()` (in `e2e/fixtures/console-guard.ts`)
removes the header for any non-baseURL origin (context route, lower priority than
test page.route mocks). Without it, Sense 2 was ~9 phantom errors/surface against
a deployed origin. Courier shift/earnings now read fully clean.

## Routing (per the net's matrix)

- a11y (label/role/contrast/focus), console warnings → **inline-fix**, re-scan.
- seed images, font self-hosting → seed/infra task.
- WS token-in-URL, price/status/security → **flag/route**, do not patch blind.
