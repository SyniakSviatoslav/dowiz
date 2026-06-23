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
| 5 | Out-of-order WebSocket frames flip order status backwards; reconnect-storm bugs | Components subscribing to a raw `new WebSocket()` bypass the shared client that owns reconnect-jitter + frame ordering | **`eslint`** + guard | **`local/no-direct-websocket`** — flags `new WebSocket()` in `apps/web/**` and `packages/ui/src/**` outside the two shared clients (`useWebSocket.ts`, `websocket.ts`). Fixtures: `__fixtures__/{bad,good}-websocket.tsx`. Complements the runtime status-monotonicity guard (`3b186fcb`) | 2026-06-22 · this change |
| 6 | Storefront cards unreadable — dark text on dark surface (~1.08:1 contrast) on light tenant themes | Partial tenant theme (primary/bg/text only) merged with default-DARK tokens; `var(...)` placeholders fell through to Food-Dark surfaces | `eslint` (existing) | `local/no-hardcoded-color`, `local/no-hardcoded-tailwind-color` + `derivePalette` coherent-palette util | 2026-06-21 · `4dab5af4` |
| 7 | Money rendered/stored with float drift | Currency handled as float instead of integer minor units | `eslint` (existing) + test | integer-tax fix + money assertions in E2E | 2026-06-20 · `otp-disabled-money-fix` |
| 8 | Hardcoded user-visible strings ship untranslated; Albanian diacritics lost | UI strings not routed through `t('key','fallback')` | `eslint` (existing) | `local/no-hardcoded-string` | 2026-06-21 · `f1f044da`, `be1529c3` |
| 9 | Raw SQL string interpolation (SQLi surface) | Query built with template-literal/`+` interpolation instead of `$1` params | `eslint` (existing) | `local/no-raw-sql` | existing |
| 10 | Cross-courier / cross-tenant IDOR (assignment-accept) | Handler not scoped by `courier_id` / tenant | `eslint` (existing) + test | `local/require-auth-hook` + scoped query fix | 2026-06-22 · `68c2cc6d` |
| 11 | Permissive test assertions hide contract regressions | `expect([200,400,...]).toContain(status)` passes on the wrong status | `eslint` (existing) | `local/no-permissive-status-assertion` | existing |
| 12 | Unhandled `localStorage` access crashes the SPA (private mode / storage off) | Direct `localStorage` reads not guarded | runtime guard + `E2E` | `safeStorage` wrapper + chaos-monkey harness | 2026-06-22 · `213bdfb5`, `2ed555a9` |
| 14 | Render-time defects invisible to text gates (dark-on-dark contrast #6, silently-dropped CSS rule #13, tenant-theme fall-through) — grep/typecheck/build/lint green, output wrong | Static rules see source, not the COMPUTED outcome; only a live-DOM check catches "renders unreadable / blank" | `E2E` (live-DOM, systemic) | `e2e/tests/behavioural-invariants.spec.ts` — the home for outcome-invariants: WCAG-AA contrast on SOLID-surface text (skips image/gradient-backed text where ratio is undefined) + body paints a resolved opaque brand surface. Red arm proves it flags solid dark-on-dark. Extensible (add `expect`s; never weaken). Closes Council root R1 | 2026-06-23 · this change (systemic ratchet) |
| 13 | A whole CSS rule silently dropped by the browser — present in file/served/dist CSS, invisible to grep/typecheck/build/lint (the `[data-skin="paper"]` token block didn't apply) | A literal `*/` in CSS comment PROSE (`--ink-*/--paper-*`) closed the block comment early; the browser's error-recovery consumed the next rule until resync | `E2E` (live-DOM) + `lesson` | `e2e/tests/paper-skin-tokens.spec.ts` (live-DOM, semantic) + `packages/ui/src/theme/css-comment-integrity.test.ts` (cheap static arm: strip canonical comments → any leftover `/*`/`*/` marker = early-closed comment; red/green arms) + lesson `docs/lessons/2026-06-23-css-comment-star-slash.md` (TRIGGER `packages/ui/src/theme/**.css`) | 2026-06-23 · this change (static arm added by Council ratchet) |

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
- **Repo-green**: the only two frontend `new WebSocket(` are the designated shared clients
  (`apps/web/src/lib/useWebSocket.ts`, `packages/ui/src/lib/websocket.ts`), both excluded;
  scope is frontend-only so back-end/test WS constructions are untouched.

## Reversal log

_(none — both guardrails active. To remove a guardrail, delete its rule + fixtures and append a
justification row here.)_
