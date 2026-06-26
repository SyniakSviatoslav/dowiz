# Non-Pixel Sweep — cross-role findings ledger

Source: `e2e/tests/non-pixel-sweep.spec.ts` (Sense 1 axe + Sense 2 console), mobile
390px, against `https://dowiz-staging.fly.dev`, demo = Dubin & Sushi (`/s/demo`).
Run after the dev-auth cross-origin header strip (phantom CORS removed → true signal).
Live video per role journey under `e2e/artifacts/test-results/**`.

## A11y (Sense 1) — by impact, with the systemic root

| Finding | Impact | Where | Root (one fix → N surfaces) | Status |
|---|---|---|---|---|
| `aria-required-parent` ×15 | critical | storefront | menu category `role="tab"` w/o tablist | ✅ FIXED (976a1029, 15→0 verified) |
| `button-name` ×4 | critical | **every** owner page (dashboard/menu/orders/analytics/settings) | shared admin icon-button(s) missing aria-label | ▶ next |
| `color-contrast` | serious | **all roles** (storefront ×7, checkout ×2, dash ×5, menu ×2, orders ×5, analytics ×4, settings, courier ×2) | brand rose `#e11d48` as text + muted text vs light surface (palette derivation) | ▶ F2 (readable-primary token) |
| `nested-interactive` ×49 | serious | owner/menu | MenuManager — interactive nested in interactive | ▶ |
| `aria-required-attr` ×3, `label` ×1, `aria-toggle-field-name` | critical/serious | owner/settings | settings form controls/toggles unlabeled | ▶ |
| `select-name` ×1 | critical | client/checkout | a `<select>` without accessible name | ▶ |
| `scrollable-region-focusable` ×1 | serious | owner dashboard, orders | scroll container not keyboard-focusable | ▶ |

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
