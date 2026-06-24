# Regression Ledger — dowiz / DeliveryOS

> **Tier-1 Regression Ratchet** (HARNESS infrastructure, not product).
> One row per bug class that recurred ≥1× or is clearly recurrence-prone. Each row
> names a **deterministic guardrail** that makes the bug physically hard to reintroduce.

## Ratchet process rule (standing)

> **Every future fix adds a guardrail (with a red→green proof) and a ledger row BEFORE it is "done".**

- **Red→green proof** = the guardrail must FAIL on the bad behavior (the bug), and PASS on
  the corrected / current code. "It should work" is not proof.
- The guardrail must also be **green on the whole current repo** — a rule that flags existing
  legitimate code is mis-scoped; narrow it until it only catches the regression.
- **Monotonic / ratchet-only**: never weaken or delete an existing gate, test, or rule to go
  green. No `skip` / `.only` / `fixme` / inflated timeout / `expect(true)` / commented-out
  assertions.
- **Reversible**: each guardrail is removable, but only with a written justification appended here.
- Guardrail types: `eslint` (code-pattern), `boot-guard` (FATAL on bad env/state at startup),
  `migration` / `release_command` (schema-drift), `E2E` (UI/contract proof), `CI-gate` (pipeline),
  `unit/integration` test.

## Ledger

| # | Symptom | Root cause | Guardrail type | Where (file / test / rule) | Date / commit |
|---|---------|-----------|----------------|----------------------------|---------------|
| 1 | Live prod auth backdoor: `POST /api/auth/local/login` minted a real owner JWT; `/dev/mock-auth` minters rode the same path | `devLoginAllowed` returned `!!DEV_AUTH_SECRET` (no flag/`NODE_ENV` gate); staging secret leaked to prod; dev tokens accepted on prod kid | `boot-guard` + `CI-gate` + `E2E` | `ALLOW_DEV_LOGIN` flag folded into `devLoginAllowed`+`isDevRequestAuthorized`; dev-kid segregation via `signDevToken`/`verifyAuthToken`; boot-guard D in config `loadEnv`; `scripts/migrate-runner.ts` release guard; auth E2E | 2026-06-22 · `5da9d136`, `ef0954c9` (ADR-0003) |
| 2 | Predictable / forgeable security tokens, OTPs, nonces, session ids (same trust-boundary class as #1) | `Math.random()` is non-CSPRNG; if used to mint a token/otp/secret it is guessable | **`eslint`** | **`local/no-insecure-random`** — flags `Math.random()` assigned to a security-named identifier (token/otp/secret/nonce/session/passwd/salt/csrf/api-key/verif/recovery/reset/magic); requires `crypto.*`. Fixtures: `__fixtures__/{bad,good}-insecure-random.ts` | 2026-06-22 · this change |
| 3 | Prod outage — API crash-loops at boot, both machines `stopped`, 000 timeouts | Schema drift: image boot-guard FATAL-exits when DB head < expected migration; CI deployed code needing migrations 032–040 but ran NONE (no `release_command`) | `release_command` + `boot-guard` | `release_command` wired in `fly.toml` to auto-migrate before rollout; pre-existing boot-guard that asserts DB head ≥ expected | 2026-06-20 / 2026-06-21 · `a91e78e9`, `5e2bc924` |
| 4 | Prod boot hangs before `listen` — `/livez` critical, never binds `:8080` | Worker `start()` calls `boss.createQueue()` → needs `CREATE on schema pgboss`; runtime role had `USAGE` only; schema owned by `postgres` | `migration` / `CI-gate` | `GRANT CREATE ON SCHEMA pgboss`; migration 042 made resilient to `insufficient_privilege`; fresh-provision pre-creates pgboss schema | 2026-06-21 / `929f0282`, `c55074fa`, `9589c2e7` |
| 5 | Out-of-order WebSocket frames flip order status backwards; reconnect-storm bugs | Components subscribing to a raw `new WebSocket()` bypass the shared client that owns reconnect-jitter + frame ordering | **`eslint`** + guard | **`local/no-direct-websocket`** — flags `new WebSocket()` in `apps/web/**` and `packages/ui/src/**` outside the **single** shared client (`useWebSocket.ts`). Fixtures: `__fixtures__/{bad,good}-websocket.tsx`. Complements the runtime status-monotonicity guard (`3b186fcb`). **F14 (2026-06-24):** the dead reconnect-capped `packages/ui/src/lib/websocket.ts` (froze after 10 tries, 0 consumers) was deleted; its rule exception removed so a re-add is now flagged | 2026-06-22 · this change |
| 7 | Cart silently carries stale prices → checkout hard-block ambush (F9); owner card flashes nameless/"0 items" on a fresh WS order (F7); a 2nd reconnect-capped WS client freezes permanently (F14) | Cart never tracked `menu_version`; OrderCard rendered `items.length` before the authed backfill arrived; a dead capped WS client diverged from the reconnect-forever one | **`test`** + `eslint` | `e2e/tests/polish-debt-logic.spec.ts` (11 cases): `reconcileCart` reprice/remove/modifier-skip/version-gate/legacy + `isOrderDetailsPending` + a guardrail asserting the capped `websocket.ts` cannot be reintroduced. Pure cores extracted (`apps/web/src/lib/cartReconcile.ts`, `isOrderDetailsPending`). Tightened `local/no-direct-websocket` (row 5) | 2026-06-24 · this change |
| 6 | Storefront cards unreadable — dark text on dark surface (~1.08:1 contrast) on light tenant themes | Partial tenant theme (primary/bg/text only) merged with default-DARK tokens; `var(...)` placeholders fell through to Food-Dark surfaces | `eslint` (existing) | `local/no-hardcoded-color`, `local/no-hardcoded-tailwind-color` + `derivePalette` coherent-palette util | 2026-06-21 · `4dab5af4` |
| 7 | Money rendered/stored with float drift | Currency handled as float instead of integer minor units | `eslint` (existing) + test | integer-tax fix + money assertions in E2E | 2026-06-20 · `otp-disabled-money-fix` |
| 8 | Hardcoded user-visible strings ship untranslated; Albanian diacritics lost | UI strings not routed through `t('key','fallback')` | `eslint` (existing) | `local/no-hardcoded-string` | 2026-06-21 · `f1f044da`, `be1529c3` |
| 11 | "Fixed A, broke B" — a shared-component change silently regresses another screen; nobody re-checks every critical screen by hand | no automated pixel-level guard on the critical path; phase gates are too coarse to catch per-change visual drift | **`test`/visual net** | Critical-Path Visual Regression Net: `playwright.visual.config.ts` (deterministic — reducedMotion/frozen-tz/perceptual-threshold/`[data-dynamic]` masks) + `e2e/visual/` harness (`loginAs`/`seedVisualState`/`MASK`) + dev-gated `/dev/seed-visual-state` + 180 snapshot tests (client/owner/courier × states × 3bp × 2lang). Baselines locked in CI (pinned Playwright image + PG service) — `docs/operating-model/proposed-visual-ci/APPLY.md`. DoD-prove: intentional shared-component regression → red snapshot (runs at lock). | 2026-06-24 · this change |
| 10 | Visually-wrong-but-valid UI ships (off-scale spacing/off-token colour via arbitrary Tailwind values; silent-English chips); code review + linter don't see pixels | agent reasons in tokens not pixels; arbitrary `p-[13px]`/`text-[#fff]` bypass the scale/tokens; keys used via `t('x','fallback')` but never catalogued render English in all locales | **`eslint`** + gate + loop | `local/no-arbitrary-tailwind` (red→green fixtures `__fixtures__/{bad,good}-arbitrary-tailwind.tsx`); i18n-parity gate extended to flag code-used-but-uncatalogued keys (`--strict`); `scripts/ui-verify-floor.sh` FLOOR runner; UI Build-Verification Loop (agent-as-eye Layer 4) — first run `e2e/journeys/UI_LOOP_RUN_2026-06-24.md` fixed `state.*` English-in-SQ/UK | 2026-06-24 · this change |
| 9 | A key routed through `t()` is added to one locale but **silently missing** in `sq`/`uk` → non-English users get an invisible English fallback, no error | resolver is `messages[locale]?.[key] \|\| fallback \|\| key`; locale-major maps let a locale lag with zero signal | **`test`/gate** + SSOT refactor | single source of truth = key-major `packages/ui/src/lib/i18n-catalog.ts` (`{key:{en,sq,uk}}`); `messages` derived from it (lossless, proven 3237/3237). `scripts/i18n-parity.ts` fails on any missing/empty/TODO locale (red→green proven), wired into `.husky/pre-commit` when i18n files are staged; `scripts/i18n-add.ts` adds a key in one step (untranslated locales become TODO drafts the gate rejects); dev-only missing-key `console.warn` in `translate()` | 2026-06-24 · this change |
| 9 | Raw SQL string interpolation (SQLi surface) | Query built with template-literal/`+` interpolation instead of `$1` params | `eslint` (existing) | `local/no-raw-sql` | existing |
| 10 | Cross-courier / cross-tenant IDOR (assignment-accept) | Handler not scoped by `courier_id` / tenant | `eslint` (existing) + test | `local/require-auth-hook` + scoped query fix | 2026-06-22 · `68c2cc6d` |
| 11 | Permissive test assertions hide contract regressions | `expect([200,400,...]).toContain(status)` passes on the wrong status | `eslint` (existing) | `local/no-permissive-status-assertion` | existing |
| 12 | Unhandled `localStorage` access crashes the SPA (private mode / storage off) | Direct `localStorage` reads not guarded | runtime guard + `E2E` | `safeStorage` wrapper + chaos-monkey harness | 2026-06-22 · `213bdfb5`, `2ed555a9` |
| 14 | Render-time defects invisible to text gates (dark-on-dark contrast #6, silently-dropped CSS rule #13, tenant-theme fall-through) — grep/typecheck/build/lint green, output wrong | Static rules see source, not the COMPUTED outcome; only a live-DOM check catches "renders unreadable / blank" | `E2E` (live-DOM, systemic) | `e2e/tests/behavioural-invariants.spec.ts` — the home for outcome-invariants: WCAG-AA contrast on SOLID-surface text (skips image/gradient-backed text where ratio is undefined) + body paints a resolved opaque brand surface. Red arm proves it flags solid dark-on-dark. Extensible (add `expect`s; never weaken). Closes Council root R1 | 2026-06-23 · this change (systemic ratchet) |
| 13 | A whole CSS rule silently dropped by the browser — present in file/served/dist CSS, invisible to grep/typecheck/build/lint (the `[data-skin="paper"]` token block didn't apply) | A literal `*/` in CSS comment PROSE (`--ink-*/--paper-*`) closed the block comment early; the browser's error-recovery consumed the next rule until resync | `E2E` (live-DOM) + `lesson` | `e2e/tests/paper-skin-tokens.spec.ts` (live-DOM, semantic) + `packages/ui/src/theme/css-comment-integrity.test.ts` (cheap static arm: strip canonical comments → any leftover `/*`/`*/` marker = early-closed comment; red/green arms) + lesson `docs/lessons/2026-06-23-css-comment-star-slash.md` (TRIGGER `packages/ui/src/theme/**.css`) | 2026-06-23 · this change (static arm added by Council ratchet) |
| 16 | Removed/downgraded owner keeps tenant read/WRITE access for the full token life — owner authorization queries filter `role='owner'` but not `status='active'`, and helpers trust the baked JWT `activeLocationId` with no DB re-check (insider-removal window) | A revoked owner's still-valid access token carries `role:owner`+`activeLocationId`; per-request scoping never re-checked live membership status, so revocation didn't bite until token expiry | `CI-gate` (static) + `E2E` (no-regression) | `scripts/guardrail-owner-active-membership.mjs` (fails on any `memberships` query with `role='owner'` lacking `status='active'`; red-proven via a probe file, green on repo; wired into `verify:all` + `pnpm guardrail:owner-active`) + `e2e/tests/owner-revocation.spec.ts` (P-b logout route wired+auth; active owner not locked out). ADR-0004, council `docs/design/owner-token-revocation/` | 2026-06-23 · this change |
| 15 | Storefront "blinks empty" under load — `/public/locations/:slug/menu` returns HTTP 500 for multi-second windows during a concurrent burst; FE renders an empty menu | Operational-pool starvation: no server-side cache on the hottest read + **2 conns/request** (Promise.all of `read_public_menu` + a redundant `locations` lookup) + heavy per-row `product_available_now()`; burst > pool (`max:8`) → checkout waits `connectionTimeoutMillis` (5s) → 500 | `E2E` (load) + `unit`(SQL) + `boot` (env-tunable pool) | `e2e/tests/menu-load.spec.ts` (30-wide burst → zero 5xx, products present — RED reproduced ~15-20×500@5.1s via curl); `packages/db/tests/read-public-menu-availability-equivalence.sql` (F4 set-based predicate ≡ `product_available_now` across 18 cases — proven green); F1 in-proc cache + F2 drop redundant query (migration 1790000000064, real `down()`) + F3 `OPERATIONAL_POOL_SIZE` (default 20) | 2026-06-23 · this change |

## Guardrails added by this change (red→green proof)

Both new ESLint rules live in `tools/eslint-plugin-local/src/index.js`, are registered in
`eslint.config.js`, and are gated by `pnpm lint:gates` (lints `__fixtures__/*`).

### `local/no-insecure-random` (row 2)
- **RED** — `__fixtures__/bad-insecure-random.ts`: flags `sessionToken`, `otpCode`, `resetToken`,
  `csrfNonce` built from `Math.random()`.
- **GREEN** — `__fixtures__/good-insecure-random.ts`: `crypto.randomBytes/randomInt/randomUUID`
  pass; and legitimate non-security `Math.random()` (jitter, particle hue, toast id) stays clean.
- **Repo-green**: zero hits across `apps/` + `packages/` (all real id/token/otp generation already
  uses `crypto.*`; `Math.random()` only appears in jitter/animation/toast-id contexts).

### `local/no-direct-websocket` (row 5)
- **RED** — `__fixtures__/bad-websocket.tsx`: flags `new WebSocket(url)` in a component.
- **GREEN** — `__fixtures__/good-websocket.tsx`: subscribing via the shared `useWebSocket` client.
- **Repo-green**: the only frontend `new WebSocket(` is the designated shared client
  (`apps/web/src/lib/useWebSocket.ts`), excluded; scope is frontend-only so back-end/test WS
  constructions are untouched. **F14 (2026-06-24):** the second client
  `packages/ui/src/lib/websocket.ts` was deleted and its exception removed — re-adding it (or any
  raw `new WebSocket`) is now flagged.

### `polish-debt-logic` regression test (row 7)
- **RED** — pre-fix, `reconcileCart` didn't exist (cart never tracked `menu_version`) and OrderCard
  rendered `items.length` (→ "0 items" on a fresh WS order); the capped `websocket.ts` existed.
- **GREEN** — `e2e/tests/polish-debt-logic.spec.ts` 11/11: reprice/remove/modifier-skip/version-gate/
  legacy reconcile, `isOrderDetailsPending` truth table, and an `existsSync` guard that fails if the
  capped `websocket.ts` is reintroduced. Runs headless (no browser): `playwright test polish-debt-logic`.

## Reversal log

_(none — both guardrails active. To remove a guardrail, delete its rule + fixtures and append a
justification row here.)_
