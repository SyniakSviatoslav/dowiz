# Telegram Mini App (TMA) — manual validation script

**Status:** dark, flag-gated, unlaunched. Both flags default OFF/unset.
**Design record:** `docs/design/tma-menu-button-wiring/` (proposal + breaker-findings +
counsel-opinion + resolution, council-APPROVED 2026-07-04) and
`docs/adr/ADR-tma-menu-button-wiring.md`.
**Research grounding:** `docs/research/2026-07-04-customer-distribution-channels.md` §3.
**Framing (Counsel, non-blocking):** this is a **distribution probe**, not a launch step —
classify it as adjacent/cheap-to-try, not something that competes for attention with the
actual growth trigger (the first real paying order). Keep it dark until Product answers the
open audience question below.

This feature has no automated E2E coverage of the real Telegram client (there is no headless
Telegram runtime to drive in CI) — validation is inherently manual. This document is the
exact script an operator runs before ever flipping either flag to `true`.

---

## What ships in this phase (and what does not)

| Built | Not built (explicit non-goal / gap) |
|---|---|
| `TMA_ENABLED` (server) — best-effort `setChatMenuButton` on `/start` connect | Telegram `initData` auth (Phase-2, only needed for personalized/write features — storefront is public read-only today) |
| `VITE_TMA_ENABLED` (client) — WebApp detection + theme-attribute mapping + back-button wiring | Loading `telegram-web-app.js` itself (CSP gap — see below) |
| Pure, unit-tested config builders + FE detection module | Telegram Payments / any in-chat checkout (out of scope, council-gated, not proposed) |
| `?ch=telegram-tma` passive URL attribution tag | Any analytics pipeline reading that tag (inert today) |

## Honest gap: the CSP blocks the Telegram bridge script today

The Mini App bridge object (`window.Telegram.WebApp`) only exists if the page loads
`https://telegram.org/js/telegram-web-app.js`. This build does **not** add that `<script>`
tag, for two concrete reasons verified against the live tree:

1. **CSP.** The storefront's `script-src` (`apps/api/src/lib/spa-shell.ts:159`, mirrored in
   `apps/api/src/routes/public/branding-preview.ts:12`) does not include `telegram.org`. A
   script tag pointing there would be silently blocked by the browser today.
2. **Shared shell.** `apps/web/index.html` is one SPA entry point for every route — admin,
   courier, and storefront alike (`apps/web/src/main.tsx`). Unconditionally adding a
   third-party script there would load Telegram's JS on every admin/courier page load too,
   not just Telegram-opened storefront sessions — a real, unreviewed cost for zero benefit
   outside Telegram.

**Consequence:** `apps/web/src/lib/tma.ts`'s `detectTelegramWebApp()` will honestly return
`undefined` in production until a follow-up phase either (a) scopes the CSP allowance and
conditionally injects the script only for `/s/:slug` requests carrying `?ch=telegram-tma`
(mirroring the existing per-route CSP override pattern already used by
`branding-preview.ts`/`spa-shell.ts`), or (b) some other page in the WebView loads it first.
This module is real, tested groundwork — not a working feature yet. Do not represent it as
functional to a vendor until that follow-up ships.

**To manually exercise the FE module today** (proves the code path, not the real deployment):
open the deployed storefront in a normal browser, open devtools console, and paste:
```js
window.Telegram = { WebApp: { colorScheme: 'dark', themeParams: { bg_color: '#1c1c1e' }, ready(){}, expand(){} } };
```
then reload-free re-trigger (React already ran the effect on mount — use the browser's
"disable cache + hard reload" or navigate away and back). Inspect `document.documentElement`
in Elements — `data-tma="true"`, `data-tma-scheme="dark"`, `data-tma-bg-color="#1c1c1e"`
should be present. This validates the mapping logic only, not real Telegram behavior.

## Second gap: the build-time flag wiring is incomplete (protected path)

`Dockerfile` needs an `ARG`/`ENV` pair for `VITE_TMA_ENABLED` (matching the existing
`VITE_MENU_ALLERGEN_FILTER` etc. block) before `--build-arg VITE_TMA_ENABLED=true` can ever
take effect in a real Fly deploy. `Dockerfile` is a protected path in this worktree
(`.claude/hooks/protect-paths.sh`) — this lane could not add it without manual approval. Add,
right after the `VITE_MENU_ALLERGEN_FILTER` block:
```dockerfile
ARG VITE_TMA_ENABLED=false
ENV VITE_TMA_ENABLED=$VITE_TMA_ENABLED
```
Until this is added, `VITE_TMA_ENABLED` is simply absent from every build (identical runtime
effect to `false`) — the darkness invariant holds, it just cannot be flipped ON yet either.

---

## Pre-flip checklist (BLOCKING — do not set `TMA_ENABLED=true` without this)

Council finding (HIGH, resolved as a mandatory gate, not accept-risk): `APP_BASE_URL` has at
least two other live/documented values in this repo (`api.dowiz.org` per
`docs/deployment-plan.md:51`; a prior prod incident set it to `staging.dowiz.app` —
`docs/audit/issues-matrix-2026-06-13.md` S5/P2) that are **not** the customer storefront
host. If misconfigured, the Mini App button opens the wrong site entirely.

1. On the target environment, resolve the actual value:
   `flyctl ssh console -a <app> -C 'printenv APP_BASE_URL'` (or check the Fly secrets/env UI).
2. `curl -sI "$APP_BASE_URL/s/<a-real-slug>"` — must return `200` and the body (or a
   `curl -s | grep -o '<title>[^<]*</title>'`) must show `<title>…Order Online | Dowiz</title>`,
   NOT an API 404/JSON error page.
3. If wrong: fix `APP_BASE_URL` at the Fly secret/env level (this also fixes the storefront's
   OG-tag meta host, a pre-existing, unrelated correctness issue this lane discovered but did
   not introduce or fix — `spa-shell.ts:66` / `spa-proxy.ts:155` share the same variable).
4. Only once step 2 passes: proceed to the BotFather + connect-flow steps below.

---

## Manual test script (operator, real Telegram client required)

### A. Bot-side menu button (`TMA_ENABLED`)

**Prerequisites:** `TELEGRAM_BOT_TOKEN` already configured (existing owner-notifications
bot); `APP_BASE_URL` verified per the checklist above; a test location/owner account.

1. **BotFather sanity check (no code change needed).** In Telegram, message `@BotFather` →
   `/mybots` → select the bot → confirm it's the same bot used for owner notifications today
   (no new bot registration required — this reuses the existing `TELEGRAM_BOT_TOKEN`).
2. **Flip the flag on staging only:** set `TMA_ENABLED=true` on the staging Fly app
   (`flyctl secrets set TMA_ENABLED=true -a dowiz-staging` or via env, per this repo's normal
   flag-flip process), redeploy.
3. **Connect a test owner:** in `/admin`, use "Connect Telegram" to get a fresh
   `t.me/<bot>?start=<token>` deep link (a **new** token — reusing an already-consumed link
   will not re-trigger this code path; see the recovery note below).
4. Open the link in Telegram, tap **Start**.
5. **Expected:** the existing `start.connected` message appears (unchanged), AND within a
   few seconds the chat's menu button (bottom-left, next to the text input) changes to show
   **"My Storefront"** with a small web-app icon.
6. Tap the menu button. **Expected:** Telegram opens an in-app WebView loading
   `<APP_BASE_URL>/s/<the-owner's-slug>?ch=telegram-tma` — the real storefront menu renders.
7. **Negative test (per B-SEC council finding):** confirm this button is **per-chat**, not
   global — connect a second, different test owner and confirm their menu button opens
   *their* slug, not the first owner's. (Guards against ever regressing `chat_id` out of the
   request — the code throws if `chatId` is empty, see `telegram-mini-app.ts`, but this is
   the live-system confirmation.)
8. **Degraded-path test:** temporarily point `TELEGRAM_BOT_TOKEN` at an invalid value (or
   throttle-test with rapid reconnects to trigger a 429), redo step 3-4. **Expected:**
   `start.connected` still arrives normally (the connect flow itself is unaffected); the menu
   button simply does not update; a `console.warn('[TelegramWebhook] Failed to set Mini App
   menu button (non-fatal):', …)` line appears in the API logs. No user-visible error.
9. **Recovery note:** the connect token is single-use — re-sending the *same* `/start <token>`
   link after a transient failure will NOT retry the button; it will instead show
   "start.token_invalid_or_expired". Recovery requires a **fresh** connect link (new token)
   from `/admin`. This is accepted (best-effort, matches existing `TG_STOREFRONT_ACTION`/
   `TG_CATEGORY_GATING` conventions) — do not represent it as auto-retrying.
10. **Off-parity:** with `TMA_ENABLED` unset/false, repeat steps 3-4 — confirm the menu
    button is untouched (whatever it was before, likely Telegram's default "Menu" showing
    bot commands) and no extra log lines/DB reads occur.

### B. Client-side detection + theme + back-button (`VITE_TMA_ENABLED`)

Real end-to-end validation requires (a) the CSP gap above resolved and the bridge script
actually loading, which is NOT part of this phase — until then, use the devtools-injection
method in the "honest gap" section to validate the mapping/back-button logic path, or wait
for the follow-up phase and re-run this same script for real in-Telegram confirmation:

1. With `VITE_TMA_ENABLED=true` baked into the build and the bridge script present (future
   phase), open the Mini App via step A6 above.
2. **Expected:** the page expands to fill the WebView (no default collapsed/half-height
   state) — `ready()`+`expand()` fired.
3. Inspect `<html>` in devtools (remote-debug via desktop Telegram + Chrome
   `chrome://inspect`, or the injection method above): `data-tma="true"`,
   `data-tma-scheme="dark"|"light"` matching the user's Telegram theme, and
   `data-tma-*` for each theme-param color Telegram sent. No visual change should occur yet
   (no CSS currently reads these attributes — by design, "do not restyle" this phase).
4. Open the cart, then tap Telegram's back gesture/button. **Expected:** the cart sheet
   closes; the Mini App does NOT exit. Repeat with the checkout sheet open.
5. **Off-parity:** with `VITE_TMA_ENABLED` unset/false, open the storefront normally in any
   browser — confirm zero `data-tma-*` attributes appear and no `Telegram` global lookups
   throw (the module must be a pure no-op when off).

---

## Phase-2 (explicitly deferred, not built here)

- **`initData` HMAC auth.** Not required now — the storefront is public/read-only. Needed
  only if a future phase adds TMA-personalized features (saved carts tied to Telegram
  identity, order history inside the Mini App, etc.). Any such feature needs its own
  council pass — Telegram identity + PII/consent surface is a materially different ethical
  weight than the read-only wrapper shipped here (Counsel note, `counsel-opinion.md` §3.4).
- **Consent line in `start.connected`.** Counsel recommended (non-blocking) adding one
  sentence disclosing that a menu button was added and how to remove it in BotFather.
  Deferred from this PR to keep the diff minimal; worth adding as a fast-follow.
- **Owner audience question (R1, OPEN — Product).** The button is wired to the *owner's own*
  chat (self-preview/self-order value). If the real goal is a *customer-facing* channel, that
  is a different, larger build (customer-facing bot flow, deep-links from the storefront,
  fresh council pass on the owner-vs-customer boundary — see `counsel-opinion.md` §5: once an
  owner shares the link with a customer, the audience boundary is porous regardless of what
  this flag does). Confirm intended audience before flipping `TMA_ENABLED=true` anywhere but
  a single test/staging owner.
