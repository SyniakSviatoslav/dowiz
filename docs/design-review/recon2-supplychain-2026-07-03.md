# Recon #2 — Supply-Chain, Dependencies, Config & Secrets Hygiene

READ-ONLY audit of dowiz/DeliveryOS. Scope: dependabot alerts + reachability, phantom/outdated deps,
pinning, postinstall exec-risk, raw `process.env` reads bypassing the Zod config schema, secret leakage
(logs / error bodies / client bundle / URLs), `.env.example` accuracy, telegram token handling,
security headers (CSP/HSTS/XFO), CORS.

Run #1 topics (leaked DB creds, feature-flag sprawl, JWT/auth soundness) are **not** re-reported here.

**Severity counts (effective, after reachability):** HIGH 1 · MEDIUM 4 · LOW 9 · INFO 3.
Dependabot: 8 open alerts (1 dependabot-HIGH, 3 MEDIUM, 4 LOW) — see §1. The lone dependabot-HIGH
(`tmp`) is **dev-only, transitive, not reachable** in first-party code (details below).

---

## §1 — Dependabot alerts (8 open) with real reachability

| # | Package | DB sev | Scope | Rel. | Installed | Reachable in *our* code? | Effective | Fix |
|---|---------|--------|-------|------|-----------|--------------------------|-----------|-----|
| 12 | `tmp` (path-traversal, CVE-2026-44705) | **HIGH** | dev | transitive | 0.0.33 + 0.1.0 (both < 0.2.6) | **NO** — pulled only by `external-editor@3.1.0`→`inquirer@6.5.2` and `proxy-agent@6.5.0` (dev CLI tooling). Zero first-party `import 'tmp'`; nothing passes untrusted `prefix/postfix/dir`. | LOW | Bump the dev tool pulling `inquirer@6`; or `pnpm.overrides` `tmp@>=0.2.6` (may break the ancient parents — test). |
| 5 | `@opentelemetry/core` (baggage memory DoS) | MEDIUM | **runtime** | transitive | 1.30.1 (vuln) **and** 2.8.0 (patched) both present | Partial — no first-party `@opentelemetry/*` import; dragged in transitively (Sentry/pino path). W3C-baggage propagation only bites if baggage headers are parsed. | MEDIUM | `pnpm.overrides` `@opentelemetry/core@>=2.8.0`; verify the 1.30.1 copy is dedup'd out. **Only runtime-scope alert worth patching.** |
| 2 | `uuid` (v3/v5/v6 buf bounds) | MEDIUM | runtime | transitive | 8.3.2 + 9.0.1 (vuln) + 13.0.2 | **NO** — no first-party `import ... from 'uuid'`; no `v3()/v5()/v6()` call with a `buf` arg (all `uuid` hits are Zod `z.string().uuid()` validators / `::uuid` SQL casts). | LOW | Optional override to `uuid@>=11.1.1`; not exploitable as-is. |
| 13 | `js-yaml` (quadratic merge-key DoS) | MEDIUM | dev | transitive | 3.14.2 (vuln, `< 3.15.0`) + 4.2.0 (patched) | **NO** — dev toolchain only; no prod path parses untrusted YAML with the 3.x copy. | LOW | Override `js-yaml@>=4.2.0` or dedup the 3.x parent. |
| 11 | `tmp` (symlink `dir`) | LOW | dev | transitive | as #12 | NO | LOW | Same as #12. |
| 10 | `cookie` (OOB chars in name/path/domain) | LOW | dev | transitive | < 0.7.0 | NO (dev) | LOW | Override `cookie@>=0.7.0`. |
| 3 | `esbuild` (dev-server arbitrary file read, **Windows-only**) | LOW | dev | **direct** | 0.28.0 (in vuln range `>=0.27.3,<0.28.1`) | NO — prod is Linux, no esbuild dev-server exposed. | LOW | Bump `apps/api` devDep `esbuild ^0.28.0` → `>=0.28.1`. |
| 1 | `esbuild` (same) | LOW | runtime | transitive | `audit-sentinel/package-lock.json` | NO — `audit-sentinel` is an out-of-tree tool, not shipped. | LOW | Bump in `audit-sentinel` or ignore. |

**Bottom line:** every dependabot alert is either dev-scope or a transitive-only path not exercised by
first-party code. The single genuinely prod-runtime, plausibly-reachable one is **#5
`@opentelemetry/core` 1.30.1** (memory-DoS via baggage) — patch via override. The "HIGH" (`tmp`) is a
paper tiger here: dev-only, transitive, no code path feeds it untrusted input.

---

## §2 — Findings (ranked)

### 🔴 HIGH

**H1 — Telegram webhook secret sits in the URL *path* and is logged in plaintext.**
- `apps/api/src/routes/telegram-webhook.ts:36` registers `POST /webhook/telegram/${telegramBotSecret}`
  — the shared secret is a **path segment**, not a header/query param.
- `apps/api/src/lib/logger.ts:114` runs `redactUrlSecrets(req.url)` on every request… but
  `redactUrlSecrets` (`logger.ts:24`) **only scrubs query-string params** (splits on `?`, redacts
  `SENSITIVE_QUERY_PARAMS`). A secret in the *path* is returned verbatim → the bot secret is written
  into request logs (and any downstream log sink / Fly log drain) in cleartext on every webhook hit.
- Exploitability: **reachable in prod**. Anyone with log access recovers the secret; with it they can
  POST forged Telegram updates to the webhook (order/notification spoofing). Also leaks via any proxy
  access log or crash trace that captures the URL.
- Fix (one line): extend `redactUrlSecrets` to also mask the known secret path segment
  (`/webhook/telegram/<...>` → `/webhook/telegram/[REDACTED]`), or move the secret to the
  `x-telegram-bot-api-secret-token` header only and register a static path.

### 🟠 MEDIUM

**M1 — `@opentelemetry/core@1.30.1` (runtime) — baggage-propagation memory DoS (dependabot #5).**
Only prod-runtime dependabot alert with a real reachability story. Transitive (no first-party otel
import; enters via Sentry/pino). Fix: `pnpm.overrides` `@opentelemetry/core@>=2.8.0` and confirm the
1.30.1 copy dedups out of the runtime graph.

**M2 — Primary app-shell CSP allows `'unsafe-inline'` AND `'unsafe-eval'` in `script-src`.**
- `apps/api/src/lib/spa-shell.ts:161` — the CSP served with the **storefront + admin HTML shell** is:
  `script-src 'self' 'unsafe-inline' 'unsafe-eval' https://cdn.tailwindcss.com https://plausible.io`.
- `'unsafe-inline'` + `'unsafe-eval'` together neutralise most of CSP's XSS value on the exact surface
  that renders tenant-controlled content. A stricter **nonce-based** CSP already exists
  (`apps/api/src/lib/security/headers.ts:24-33`, no unsafe-inline/eval) but is applied to API/JSON
  routes, not the shell.
- Reachable in prod (every `/s/:slug` and `/admin/*` page). Root cause: Tailwind CDN + inline bootstrap
  scripts. Fix: migrate the shell to the nonce path (drop `'unsafe-inline'`/`'unsafe-eval'`), or at
  minimum remove `'unsafe-eval'`.

**M3 — 23 env vars are read via raw `process.env` and bypass the Zod config schema — including 3 secrets.**
- Schema is `packages/config/src/index.ts` (`EnvSchema` → `loadEnv()` fail-fast). These names are read
  directly and are **not** in the schema, so a typo / missing value fails *silently* instead of
  aborting boot:
  `METRICS_TOKEN`, `PLISIO_SECRET_KEY`, `PROVISION_OPS_SECRET` (**secrets**),
  `PAYMENTS_PROVIDER`, `PAYMENTS_PREPAID_ENABLED`, `PAYMENTS_CRYPTO_ENABLED`,
  `VOICE_KILL`, `VOICE_CONTROL_ENABLED`, `TG_CATEGORY_GATING`, `TG_STOREFRONT_ACTION`,
  `COURIER_OFFER_TTL_MIN`, `COURIER_OFFER_HANDSHAKE_ENABLED`,
  `DELIVERY_TRACE_RETENTION_CRON`, `DELIVERY_TRACE_GPS_RETENTION`,
  `ACQUISITION_RETENTION_CRON`, `ACQUISITION_SHADOW_TTL_DAYS`,
  `MENU_GROUNDING_ENABLED`, `MENU_OCR_ENGINE`, `PADDLE_OCR_PYTHON`, `PADDLE_OCR_SCRIPT`,
  `PG_DATABASE`, `PUBLIC_API_BASE_URL`, `STORAGE_DIR` (`VITE_BASE_URL` is build/test-only — N/A).
- Security angle: the *fail-open on misconfig* class. Both secret-gated surfaces do fail **closed**
  today (`METRICS_TOKEN` unset → `/metrics` 404 `metrics.ts:136`; `PROVISION_OPS_SECRET` unset → 404
  `ops-auth.ts:27`), and both compares are timing-safe (`crypto.timingSafeEqual`), so no *current*
  exploit — but a security control whose enable/disable pivots on an unvalidated string is one typo
  from silently-off with no boot error. Fix: add these to `EnvSchema` (secrets as `z.string().optional()`,
  flags as `z.enum(['true','false'])`) and read the parsed `env` object, not `process.env`.

**M4 — `js-yaml@3.14.2` quadratic-DoS (dependabot #13).** Dev-scope, transitive, not reachable in prod
(no untrusted-YAML prod path uses the 3.x copy). Listed MEDIUM by dependabot; effective LOW. Override to
`js-yaml@>=4.2.0` when convenient.

### 🟡 LOW

- **L1 — Telegram webhook backward-compat auth-skip.** `telegram-webhook.ts:57-60`: when the
  `x-telegram-bot-api-secret-token` header is **absent**, the request is processed anyway (warn-only).
  The URL-path secret still gates the route, so this is only a weakened defense-in-depth layer — but it
  means a leaked path secret (see H1) fully authenticates with no second factor. Fix: once all webhooks
  are re-registered with `secret_token`, make the header mandatory.
- **L2 — Mismatch log leaks real-secret length.** `telegram-webhook.ts:51-54` logs
  `expectedLength: telegramBotSecret.length` on mismatch — minor oracle. Drop it.
- **L3 — All first-party deps float (`^`).** Every `apps/*` / `packages/*` runtime dep uses caret
  ranges (`fastify ^5.8.5`, `pg ^8.21.0`, `argon2 ^0.44.0`, `web-push ^3.6.7`, …); only
  `@tabler/icons-webfont`, `pdfjs-dist` are pinned. `pnpm-lock.yaml` pins exact resolved versions, so
  risk only materialises on a non-frozen install. Ensure CI/Docker use `pnpm install --frozen-lockfile`.
- **L4 — `pdfjs-dist@4.8.69` is well behind latest (6.1.200).** Pinned exact; a major-version lag on a
  library that parses untrusted uploaded PDFs (menu-import) is worth a scheduled bump + regression test.
- **L5 — `.env.example` omits required boot vars.** It lists ~16 keys but the schema *requires*
  (`z.string().min(1)`, no default) `VAPID_PUBLIC_KEY` and `VAPID_PRIVATE_KEY`, which are **absent** —
  a fresh clone following `.env.example` FATAL-fails at `loadEnv()`. It also omits every operational
  secret the app can use (`R2_*`, `METRICS_TOKEN`, `PROVISION_OPS_SECRET`, `PLISIO_SECRET_KEY`,
  `OPENROUTER_API_KEY`, `BACKUP_ENCRYPTION_KEY`, `COURIER_PII_ENCRYPTION_KEY`, …). Add the required ones
  at minimum. (Values must stay placeholder — never real secrets.)
- **L6 — `@deliveryos/voice` is an orphan workspace package.** `packages/voice` builds a package named
  `@deliveryos/voice` that **no app declares or imports** (zero import sites in `apps/web`/`apps/api`;
  not in any `package.json` deps). Not a live phantom (nobody imports it), but if voice is later wired
  in without adding the dep it becomes an undeclared/phantom import. Either declare-and-consume or fence
  it as intentionally-dark.
- **L7 — `cookie` (dependabot #10) / `tmp` symlink (#11) / `esbuild` (#3,#1).** Dev/transitive or
  Windows-dev-server-only; none reachable in the Linux prod runtime. Batch-bump via overrides.
- **L8 — `uuid` v3/v5/v6 buf-bounds (dependabot #2).** Not reachable (no first-party `uuid` import, no
  buffered v3/v5/v6 call). Optional override to `>=11.1.1`.
- **L9 — `mem0ai` declared but only used in one dark path.** Declared in `apps/api`; used only at
  `apps/api/src/lib/memory.ts`. Confirm the memory feature is actually wired / flagged, else it's
  ~unused runtime weight. Housekeeping.

### ⓘ INFO / no-action (verified sound)

- **No hardcoded `Ihatenuclearwar` / bot-secret / token literal** anywhere in source — grep clean. The
  telegram secret is sourced from `env.TELEGRAM_BOT_SECRET` (`server.ts:531`), Zod-typed
  (`config/index.ts:42`).
- **No secret VALUES in the client bundle.** `apps/web` exposes only `VITE_`-prefixed vars
  (`VITE_GOOGLE_OAUTH_ENABLED`, `VITE_MEDIA_RICH_ENABLED`, `VITE_PAYMENTS_CRYPTO_ENABLED`,
  `VITE_TG_CATEGORY_GATING`). The `-----BEGIN…` / `*_KEY` hits in `dist/api/server.cjs` are the
  **server** bundle (Zod field names, key-handling code, a *public* Redis Cloud CA cert) — not shipped
  to browsers, no secret values.
- **CORS is safe.** `server.ts:140` default denies cross-origin (`origin` cb → `false`); public routes
  set `Access-Control-Allow-Origin: '*'` **with `credentials:false`** (`server.ts:145,152`) — wildcard
  without credentials is not a vector. HSTS/`X-Content-Type-Options`/`X-Frame-Options: SAMEORIGIN`/
  `Referrer-Policy` set globally (`server.ts:126-136`); `Permissions-Policy` on API routes
  (`security/headers.ts:41`). Both secret compares are `crypto.timingSafeEqual`. Prior CVE overrides
  (`form-data@>=4.0.6`, `undici@>=6.27.0`) already in `package.json`. `.env` is gitignored + untracked.

---

## §3 — Suggested batch remediation (cheapest → highest value)

1. **H1** — fix `redactUrlSecrets` to mask the telegram path secret (or move it to header-only). *(highest value)*
2. **M1** — `pnpm.overrides`: `@opentelemetry/core@>=2.8.0` (only reachable runtime CVE).
3. **M2** — drop `'unsafe-eval'` (min) / migrate shell to nonce CSP.
4. **M3** — fold the 3 secret + 20 config vars into `EnvSchema`; read parsed `env`, not `process.env`.
5. **Batch** — `pnpm.overrides` for the dev-scope alerts (`tmp>=0.2.6`, `js-yaml>=4.2.0`, `cookie>=0.7.0`,
   `esbuild>=0.28.1`, `uuid>=11.1.1`) + add `VAPID_*` to `.env.example`. Test the old `tmp` parents.
