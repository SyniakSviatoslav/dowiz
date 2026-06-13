# Critical lifecycle E2E — multi-actor, against a deployed service

One serial Playwright test, three browser contexts (customer / owner / courier), proving the
full order lifecycle live over real api/ws. This is your **launch-gating smoke**: it exercises the
state machine + WebSocket fan-out (MessageBus N-safety) + cross-role propagation + idempotent
order creation in a single pass.

## Files

```
e2e/
  playwright.config.ts          # deployed-service config (no webServer; baseURL from env)
  auth.setup.ts                 # logs owner+courier in once → .auth/*.json
  critical-lifecycle.spec.ts    # THE test
  support/
    env.ts                      # all deployment-specific values (URLs, accounts, slug, geo)
    selectors.ts                # the ONE place to adapt selectors + order-state strings
    helpers.ts                  # WS frame capture, geo-track emulation, id/token extraction
```

## What you must fill in (≈15 min, isolated)

Everything deployment-specific is in `support/env.ts` and `support/selectors.ts`. The spec itself
shouldn't need edits.

1. **Env vars** (`.env`, shell, or CI secrets):
   ```bash
   E2E_BASE_URL=https://staging.your-app        # set the 3 *_URL vars too if surfaces differ
   E2E_RESTAURANT_SLUG=test-sushi               # a seeded test tenant (see caveats)
   E2E_OWNER_EMAIL=...   E2E_OWNER_PASSWORD=...
   E2E_COURIER_EMAIL=... E2E_COURIER_PASSWORD=...
   # optional: E2E_DEV_LOGIN_PATH, E2E_AUTH_STORAGE_KEY, USE_UI_LOGIN=1, E2E_TEST_PHONE
   ```
2. **Auth seam** (`env.ts`): confirm `devLoginPath` + `authStorageKey` against `useAuth`. No
   login endpoint? Set `USE_UI_LOGIN=1` — `auth.setup.ts` drives the form instead.
3. **Response shapes** (`helpers.ts`): `extractOrderId` and `extractToken` guess common field
   names — confirm against your real responses.
4. **Order states** (`selectors.ts` → `STATES`): replace with the server's real status strings.
5. **Test seams** the test asserts on (add to components — i18n-safe, zero contract change, and a
   legitimate convergence-loop fix). If you'd rather not, rewrite the selectors as `getByRole`:

   | Attribute        | On                                   | Values            |
   |------------------|--------------------------------------|-------------------|
   | `data-testid`    | every element in `selectors.ts`      | (the ids listed)  |
   | `data-status`    | StatusBadge, order card, task card   | your status strings |
   | `data-connected` | WSStatusDot                          | `"true"`/`"false"` |
   | `data-online`    | courier online toggle                | `"true"`/`"false"` |

   Why attributes over text: your UI is al/en, so `getByText` would flake on language switch.

## Run

```bash
npm i -D @playwright/test && npx playwright install chromium
npx playwright test                 # headless
npx playwright test --headed        # watch all three contexts live
npx playwright show-report          # trace/video/screenshot retained on failure
```

## Important caveats — running against a deployed service

- **Staging, not prod.** This creates a **real order** and walks it to `delivered`. Point it at
  staging, or a deployment whose data you're happy to generate. Never untargeted prod.
- **Real side effects.** A real order can fire real push/Telegram/owner notifications. Use a
  **dedicated test tenant** whose notification targets are test sinks so the suite doesn't ping a
  real owner. (One reason `restaurantSlug` is a seeded test restaurant, not a live one.)
- **Leaves a terminal order behind.** `delivered` is terminal, so no cleanup is required; the test
  intentionally does not hard-delete anything.
- **One courier at a time.** The suite is serial (`workers: 1`); the test courier going online
  shouldn't collide with other runs against the same staging tenant.

## What this test does NOT cover (by design — separate tests)

- **Edge/error matrix** on this path: double-`confirm`=1 order, drifted `total` ignored,
  kill-backend → fallback phone + cart intact, geocode-timeout → manual. (High value; layer next.)
- **Durable/time-based** flows (auto-timeout, dwell escalation, 30-min feedback, worker-liveness) —
  those need the test-only job-trigger seam, not this event-driven happy path.
- **Per-screen states / breakpoints / micro-interactions** — those belong in component/unit tests,
  not in this full-browser multi-actor flow.
- **The MapLibre marker pixel** — WebGL canvas, no DOM. The test asserts propagated state + a real
  WS frame instead.

## How it fits your convergence loop

This proves X3 (client critical path) + X4 (owner) + X5 (courier) for the **happy path, live**, in
one shot. Treat *this* green as the launch gate; grind the full X1–X11 matrix as convergence after.
