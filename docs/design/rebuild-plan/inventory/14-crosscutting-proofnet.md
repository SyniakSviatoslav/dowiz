# 14 — Cross-Cutting Behaviors, Integrations & Proof-Net Map (Lane E)

- **Date:** 2026-07-04 · **Lane:** E (rebuild mapping program) · **Type:** docs-only census + methodology
- **Mission:** (1) census every cross-cutting behavior/integration that spans surfaces — the class of
  things rewrites lose *silently*; (2) design the verification methodology that makes "nothing missed"
  **provable, never narrated**. Every census below carries its extraction command + count so it can be
  re-run mechanically against both codebases.
- **Grounding:** `06-complete-rebuild-stack.md` (Rust axum/sqlx + Astro 5/Svelte 5 islands; DB and
  Playwright E2E net UNCHANGED), `REBUILD-PLAN.md` §4 never-regress invariants.
- **Convention:** every row that a strangler cutover could silently drop gets a stable ID
  (`FLAG-`, `ENV-`, `ERR-`, `AUTH-`, `INT-`, `WS-`, `TEST-`) — these IDs feed the master traceability
  matrix (§8). 🔴 = red-line (council-gated port).

Contents: §1 feature flags · §2 env vars · §3 error contract · §4 auth/session lifecycle 🔴 ·
§5 integrations · §6 realtime client contract · §7 proof-net census + classification ·
§8 completeness methodology · §9 not-fully-enumerable classes.

---

## §1 Feature-flag census

### §1a Client `VITE_*` flags — **19**

**Extraction:** `grep -rhoE 'VITE_[A-Z0-9_]+' apps/web/src packages/ui/src | sort -u` → **19**
(+ `VITE_PROXY_TARGET` in `vite.config.ts`, dev-server-only, not client-bundled). No `envPrefix`
override. Build-arg reality: `Dockerfile:17-36` declares only **6** as ARGs (all default `false`:
ACCESS_GATE_PUBLIC, TG_CATEGORY_GATING, MENU_CHARACTERISTICS_{ENABLED,COMPARISON,FILTER},
MENU_ALLERGEN_FILTER); **zero** CI workflows pass any `--build-arg VITE_*` — the other 13 can only
ever take their in-source fallback in a pipeline-built image (this is the staging-deploy-flags
lesson generalized). **In Astro the equivalent is `import.meta.env.PUBLIC_*` baked at build — the
map-coverage gate (§8) must diff the flag list, the defaults, AND the Docker/CI build-arg wiring.**

| ID | Flag | Default (file:line) | Gates | Status |
|---|---|---|---|---|
| FLAG-C01 | VITE_ACCESS_GATE_PUBLIC_ENABLED | false — `AccessRequestForm.tsx:13`, `Dockerfile:20` | public "register interest" CTA (soft access gate) | dark |
| FLAG-C02 | VITE_API_BASE_URL | `''`/`'/api'` — `apiClient.ts:4`, `MenuPage.tsx:729` | API base URL | config |
| FLAG-C03 | VITE_GOOGLE_OAUTH_ENABLED | false — `admin/LoginPage.tsx:11` | Google sign-in button (mirrors server GOOGLE_OAUTH_ENABLED) | dark |
| FLAG-C04 | VITE_KEYBOARD_SHORTCUTS_ENABLED | **ON** (`!== 'false'`) — `AdminCommandCenter.tsx:12` | ⌘K palette / g-sequences | live |
| FLAG-C05 | VITE_MEDIA_RICH_ENABLED | false — `MenuManagerPage.tsx:30` | cinematic media manager (ADR-0002; mirrors MEDIA_RICH_ENABLED) | dark |
| FLAG-C06 | VITE_MENU_ALLERGEN_FILTER | false — `client/MenuPage.tsx:27`, `Dockerfile:35` | allergen filter on public menu | dark |
| FLAG-C07 | VITE_MENU_CHARACTERISTICS_COMPARISON | false — `MenuPage.tsx:36`, `Dockerfile:31` | compare-arrows (ADR-0014) | dark |
| FLAG-C08 | VITE_MENU_CHARACTERISTICS_ENABLED | false — `MenuPage.tsx:35`, `Dockerfile:29` | characteristics layer master (ADR-0014) | dark |
| FLAG-C09 | VITE_MENU_CHARACTERISTICS_FILTER | false — `MenuPage.tsx:37`, `Dockerfile:33` | macro sort/filter lenses | dark |
| FLAG-C10 | VITE_OWNER_TWO_TAP | false — `packages/ui/src/components/admin/OrderCard.tsx:14` | 2-tap owner order lane | dark |
| FLAG-C11 | VITE_PAPER_SKIN | off (`=== 'on'` else `localStorage dos_paper_skin`) — `packages/ui/src/theme/paperSkin.ts:10` | PAPER/MOEBIUS internal skin | dark (session opt-in) |
| FLAG-C12 | VITE_PAYMENTS_CRYPTO_ENABLED | false — `CheckoutPage.tsx:58` | crypto checkout option (ADR-0017; mirrors PAYMENTS_CRYPTO_ENABLED) | dark |
| FLAG-C13 | VITE_PULL_TO_REFRESH_ENABLED | **ON** (`!== 'false'`) — `courier/TasksPage.tsx:14` | courier pull-to-refresh | live |
| FLAG-C14 | VITE_TG_CATEGORY_GATING | false — `SettingsPage.tsx:11`, `Dockerfile:24` | notification-category prefs UI (mirrors TG_CATEGORY_GATING) | dark |
| FLAG-C15 | VITE_TILE_PROVIDER | `'free'` (zod) — `lib/tileConfig.ts:28` | tile provider select (geo-seams) | config |
| FLAG-C16 | VITE_TILE_STYLE_URL | openfreemap liberty (zod) — `tileConfig.ts:27` | MapLibre style URL | config |
| FLAG-C17 | VITE_UNDO_REDO_ENABLED | **ON** (`!== 'false'`) — `MenuManagerPage.tsx:36` | undo/redo on product draft | live |
| FLAG-C18 | VITE_VOICE_ENABLED | declared `vite-env.d.ts:9`, **never read** | intended MicFab mount (ADR-0015) — scaffolding only | dark/not-wired |
| FLAG-C19 | VITE_WS_BASE_URL | computed `${proto}//${host}/ws` — `lib/useWebSocket.ts:6` | WS base URL | config |

**Reconciliation:** declared-and-used 2 (TILE_*), declared-but-dead 1 (VOICE_ENABLED),
used-but-undeclared 16 (typecheck via Vite's `any` index-signature fallback — the Astro rebuild
should declare all of them; undeclared flags are exactly how a rewrite drops one).

### §1b Server flags/toggles — **22** (12 schema-declared · 8 pure drift · 2 presence-pairs)

**No central flags module exists** — flags are ad-hoc `loadEnv().X === 'true'` /
`process.env.X === 'true'` reads. Extraction: `grep -rnoE "process\.env\.[A-Z0-9_]+" apps/api/src`
cross-referenced against `EnvSchema` keys (`packages/config/src/index.ts`).

| ID | Flag | Schema? | Default (file:line) | Gates | Status |
|---|---|---|---|---|---|
| FLAG-S01 | GOOGLE_OAUTH_ENABLED | ✓ `config/index.ts:19` | 'false' | `/api/auth/google*` (404 off) — `auth.ts:38,65` | dark |
| FLAG-S02 🔴 | ALLOW_DEV_LOGIN | ✓ `:29` | 'false' | ALL dev/test auth bypasses (with DEV_AUTH_SECRET) — `dev-guard.ts:31`; prod boot-guard FATAL `config/index.ts:230` | dark |
| FLAG-S03 | OTP_ENABLED | ✓ `:48` | 'false' | phone OTP (send = console.log scaffold) — `orders.ts:22`, `customer/otp.ts:9` | dark |
| FLAG-S04 | MEDIA_RICH_ENABLED | ✓ `:53` | 'false' | media endpoints; AND-gated with `plan='business'` — `public/menu.ts:74` | dark |
| FLAG-S05 | FUNNEL_INGEST_ENABLED | ✓ `:57` | **'true'** | funnel-ingest kill-switch (ADR-0009) — `public/funnel.ts:32` | live |
| FLAG-S06 🔴 | ENFORCE_VENUE_HOURS | ✓ `:64` | 'false' | closed-venue 409 VENUE_CLOSED — `orders.ts:151` | dark |
| FLAG-S07 | BACKUP_ENABLED | ✓ `:89` | 'false' | backup crons — `workers/backup/index.ts:28` | dark |
| FLAG-S08 | DWELL_TIER3_ENABLED | ✓ `:103` | 'false' | **never read — dead** | dead |
| FLAG-S09 | RESTORE_VERIFY_FULL_HASH | ✓ `:135` | 'false' | full-hash restore verify — `backup-verify-scheduled.ts:34` | dark |
| FLAG-S10 🔴 | DISPATCH_OWNER_GRACE_ENABLED | ✓ `:161` | 'false' | auto-cancel after dispatch grace ("flag-off until operator ratifies at STOP-ETHICS") — `courier-offer-sweep.ts:201` | dark |
| FLAG-S11 | ACCESS_GATE_PUBLIC_ENABLED | ✓ `:188` | 'false' | registers `POST /api/access-requests` — `bootstrap/routes.ts:118` | dark |
| FLAG-S12 | ACCESS_GATE_INVITE_GATING_SHIPPED | ✓ `:191` | 'false' | CI banned-strings companion (test-only consumer) | dark/test-only |
| FLAG-S13 🔴 | PAYMENTS_PREPAID_ENABLED | ✗ drift | implicit false — `lib/payments/registry.ts:6` | prepaid methods | dark |
| FLAG-S14 🔴 | PAYMENTS_CRYPTO_ENABLED | ✗ drift | implicit false — `registry.ts:7` | Plisio crypto (ADR-0017) | dark |
| FLAG-S15 | VOICE_CONTROL_ENABLED | ✗ drift | implicit false — `lib/voice-flag.ts:12` | `/api/public/voice-config` + CSP widening | dark |
| FLAG-S16 | VOICE_KILL | ✗ drift | unset/inert — `voice-flag.ts:12` | runtime hot-kill (inverse polarity `!== 'true'`) | live-by-default |
| FLAG-S17 | TG_CATEGORY_GATING | ✗ drift | implicit false — `notifications/workers/index.ts:49`, `telegram-webhook.ts:19` | TG category dispatcher | dark |
| FLAG-S18 | TG_STOREFRONT_ACTION | ✗ drift | implicit false — `telegram-webhook.ts:25` | TG storefront quick-action | dark |
| FLAG-S19 | COURIER_OFFER_HANDSHAKE_ENABLED | ✗ drift | implicit false — `owner/dashboard.ts:323` | courier offer handshake | dark |
| FLAG-S20 | MENU_GROUNDING_ENABLED | ✗ drift | implicit false — `ai-ocr-parser.ts:637` | menu-import LLM grounding pass | dark |
| FLAG-S21 | TELEGRAM_BOT_TOKEN (presence) | ✓ `:41` optional | absent | Telegram live-or-not — `health.ts:131` | live-if-configured |
| FLAG-S22 | VAPID_PUBLIC/PRIVATE_KEY (presence pair) | ✓ `:115-116` required | must be set | Web Push — `bootstrap/notifications.ts:51` | live |

**Rebuild consequence:** the 8 drift flags get zero validation from `loadEnv()`. The Rust config
layer (figment/envy struct) must declare **all 22** — the map-coverage gate diffs Rust struct
fields against this list, so a drift flag can't silently vanish in the port.

---

## §2 Env-var census

**Verification layer:** `packages/config/src/index.ts` — single zod `EnvSchema` + `loadEnv()`
(throws with all issues) + `assertDevAuthDisabledInProd()`; `verify-env.ts` is the CLI wrapper.
**Extraction:** `grep -cE '^\s*[A-Z][A-Z0-9_]*:\s*z\.' packages/config/src/index.ts` → **119 fields**.

**The "32 classified service envs"** = the subset matching `/_(URL|KEY|TOKEN|SECRET|DSN|ENDPOINT)$/`,
force-classified in `compliance/env-classification.md` by the fail-closed gate
`scripts/guardrail-license.mjs`. Extraction:
`grep -oE '^[A-Z][A-Z0-9_]*' packages/config/src/index.ts | grep -E '_(URL|KEY|TOKEN|SECRET|DSN|ENDPOINT)$' | sort -u | wc -l` → **32**;
`grep -c '^| [A-Z]' compliance/env-classification.md` → **32**; `comm` diff → **identical sets,
zero compliance drift**. The Rust config preflight must validate the same 119-minus-dead set AND
keep the suffix-triggered compliance gate.

| ENV | Class | Consumer | ENV | Class | Consumer |
|---|---|---|---|---|---|
| APP_BASE_URL | internal | api | OPENAI_API_KEY / OPENAI_ENDPOINT | ext (OpenAI) | api (ocr) |
| BACKUP_ENCRYPTION_KEY | internal | api-cron | OPENCODE_ZEN_API_KEY / _ENDPOINT | ext (OC Zen) | api (ocr) |
| COURIER_PII_ENCRYPTION_KEY | internal | api | OPENROUTER_API_KEY / _ENDPOINT | ext (OpenRouter) | api (ocr) |
| DEV_AUTH_SECRET | internal | api | R2_ENDPOINT / R2_PUBLIC_URL / R2_SECRET_ACCESS_KEY | ext (Cloudflare R2) | api |
| JWT_PRIVATE/PUBLIC_KEY | internal | platform lib | REDIS_URL | ext (Upstash) | api |
| JWT_DEV_PRIVATE/PUBLIC_KEY | internal | platform lib (prod-forbidden) | RESEND_API_KEY | ext (Resend) | api-cron |
| VAPID_PUBLIC/PRIVATE_KEY | internal | api-cron (push) | ROUTING_API_KEY / ROUTING_BASE_URL | ext (OpenRouteService) | platform lib |
| LLM_ENDPOINT | internal | api (ocr) | SENTRY_DSN | ext (Sentry) | api |
| MEM0_OLLAMA_URL | internal | api (memory) | TELEGRAM_BOT_SECRET / _TOKEN | ext (Telegram) | api |
| TRANSLATION_ENDPOINT | internal | api | GROQ_API_KEY / GROQ_ENDPOINT | ext (Groq) | api (ocr) |
| GOOGLE_CLIENT_SECRET | ext (Google) | api | | | |

Remaining 87 schema fields (re-derivable via the extraction command; classes =
required/default/optional per zod): DB URL triple `DATABASE_URL_OPERATIONAL/SESSION/MIGRATIONS`
(staging-DB-access lesson: there is NO plain `DATABASE_URL`), pool sizes, JWT_KID,
GOOGLE_CLIENT_ID, dev-login pair, worker crons (BACKUP_*/DWELL_*/SIGNAL_RAISE/RATES/
ANONYMIZER_RETENTION/RESTORE_VERIFY/ACCESS_REQUEST_*), courier-dispatch knobs
(COURIER_ACCEPT_WINDOW_MS, CANCEL_AFTER_DISPATCH_WINDOW_MS, COURIER_DISPATCH_MAX_ATTEMPTS,
COURIER_ASSIGN_ACCEPT_TIMEOUT_MS, DISPATCH_OWNER_GRACE_MS, COURIER_GPS_MAX_DIST_KM),
IP_HASH_SALT (required), LOG_LEVEL, GIT_SHA, FLY_MACHINE_ID, WORKER_HEARTBEAT/LIVENESS set,
LLM adapter set, MEM0 set, TRANSLATION_PROVIDER, ROUTING_PROVIDER, PRIVACY_NOTICE_VERSION,
WAITLIST_NOTIFY_EMAIL.

**Dead/shadowed schema fields — 13 of 119 (~11%) have ZERO runtime effect** (do NOT port):
unconsumed `BACKUP_HOURLY_RETENTION_HOURS`, `BACKUP_DAILY_RETENTION_DAYS`,
`BACKUP_MONTHLY_RETENTION_YEARS`, `BACKUP_PII_FIELDS`, `DWELL_TIER3_DELAY_MS`,
`DWELL_TIER3_ENABLED`, `R2_RETENTION_OVERRIDE_DAYS`, `RESTORE_POOL_SIZE`; shadowed by hardcoded
constants `OTP_SEND/VERIFY_RATE_LIMIT`, `OTP_TTL_MS` (`lib/otp.ts:5-7`),
`VELOCITY_WINDOW/THRESHOLD_{1H,24H}` (`lib/signals/compute.ts:40-41`),
`ANONYMIZER_RETENTION_BATCH_SIZE` (`anonymizer-retention.ts:11`). Rust port decision per field:
delete or actually wire — never copy a dead knob.

**Drift — 24 vars read via raw `process.env` in `apps/api/src` but ABSENT from EnvSchema**
(extraction: `grep -rohE 'process\.env\.[A-Z][A-Z0-9_]*' apps/api/src | sed 's/process\.env\.//' | sort -u`
then `comm -23` against schema keys → **24**): ACQUISITION_RETENTION_CRON,
ACQUISITION_SHADOW_TTL_DAYS, BACKUP_KEYRING (`workers/backup/encrypt.ts:47`),
COURIER_OFFER_HANDSHAKE_ENABLED, COURIER_OFFER_TTL_MIN, DELIVERY_TRACE_GPS_RETENTION,
DELIVERY_TRACE_RETENTION_CRON, MENU_GROUNDING_ENABLED, MENU_OCR_ENGINE, METRICS_TOKEN
(`lib/metrics.ts:135`), PADDLE_OCR_PYTHON/SCRIPT, PAYMENTS_CRYPTO/PREPAID_ENABLED,
PAYMENTS_PROVIDER, **PLISIO_SECRET_KEY** (`lib/payments/registry.ts:16` — a SECRET outside both
the schema AND the compliance gate, because the gate only scans schema keys),
PROVISION_OPS_SECRET (`server.ts:545` — same exposure), PUBLIC_API_BASE_URL, STORAGE_DIR,
TG_CATEGORY_GATING, TG_STOREFRONT_ACTION, VITE_BASE_URL (server reading a client-prefixed var,
`registry.ts:18`), VOICE_CONTROL_ENABLED, VOICE_KILL. **Rust target: one exhaustive config
struct = schema ∪ drift − dead; raw `std::env::var` denied by clippy outside the config module.**

---

## §3 Error contract (ADR-0010) — one Rust enum, zero drift

**SSOT:** `docs/adr/0010-error-contract-envelope.md` (status "Proposed" but SHIPPED in code).
As-built envelope — `buildErrorEnvelope`, `apps/api/src/lib/api-error.ts:63-71`:

```jsonc
{ "code": "SCREAMING_SNAKE",        // machine code, FE-branchable — never rename/normalize/drop
  "message": "human text",           // generic on 5xx
  "fields": [{"path","code"}],       // 422 only — paths never values (PII-safe)
  "correlationId": "uuid",           // server-authoritative (genReqId, inbound header NOT trusted — server.ts:92-100), echoed in x-correlation-id
  "retryAfterMs": 1234,              // 429 only
  "status": 422,                     // numeric HTTP status (legacy field)
  "error": "= message" }             // legacy alias for un-migrated FE call sites
```

**Emission paths (Node):** `ApiError` throwable (`api-error.ts:17-38`) · `isContractCode` gate
`^[A-Z][A-Z0-9_]*$` (`api-error.ts:41-43`) · `reply.sendError()` decorator
(`lib/reply-send-error.ts:14-24`, registered `server.ts:432`) · central `setErrorHandler`
(`server.ts:443-517` — AJV/Zod → `VALIDATION_FAILED`@400, PG codes like `23505` never leak,
5xx generic + Sentry) · `rateLimitError()` for `@fastify/rate-limit` (`api-error.ts:83-91` →
`server.ts:375`).

**Census (extraction commands + counts):**
- `grep -rn "\.sendError(" apps/api/src | wc -l` → **311 call sites**;
  `grep -rhoE "\.sendError\([0-9]+,\s*'[A-Za-z0-9_./-]+'" apps/api/src | sed -E "s/.*'(.*)'/\1/" | sort -u | wc -l`
  → **68 unique SCREAMING_SNAKE codes** (+~5 dynamic sites resolving to `MODIFIER_*`,
  `NOT_DELIVERABLE`, `DELIVERY_NOT_CONFIGURED` from `lib/order-pricing.ts:91-183`).
- Separate namespace (do NOT merge): preflight `reasons[].code` — lowercase business-outcome
  tokens `item_unavailable/velocity/no_show_history/otp_required` (**8 unique**, all
  `lib/preflight.ts`; ADR §4b) — intentionally lowercase, FE branches on them
  (`CheckoutPage.tsx:420-449`).
- Ad-hoc bypasses still alive: `grep -rn "reply\.status([0-9]\+)\.send({" apps/api/src --include='*.ts' | grep -icE "error|code"`
  → **51 divergent sites** (e.g. `plugins/auth.ts:47-157`, `plugins/turnstile.ts:16-39`,
  `server.ts:414,424,656`) incl. one live divergent shape: `MIN_ORDER_NOT_MET` at
  `routes/orders.ts:494-499` (no correlationId, no message/error symmetry).

**Status→code matrix (top codes; full list mechanically re-derivable via the commands above):**

| Status | Codes |
|---|---|
| 400 | VALIDATION_FAILED (41× + central handler), GPS_REQUIRED/OUT_OF_RANGE, INVALID_GROUP/KEY/ROLE, NOT_NULL, NO_UPDATES, ONBOARDING_INCOMPLETE, OTP_NOT_REQUIRED, STEP_ALREADY_COMPLETED, UNKNOWN_PRESET, UNSUPPORTED_LOCALE/SOURCE/TYPE |
| 401 | UNAUTHORIZED (36×), INVALID_CREDENTIALS/TOKEN/REFRESH_TOKEN/CODE, SESSION_NOT_FOUND, REFRESH_EXPIRED/REUSED, OWNER_REVOKED, COURIER_DEACTIVATED, WRONG_AUTH_METHOD |
| 403 | FORBIDDEN, COURIER_DEACTIVATED, NO_LOCATION_ASSIGNED, NOT_AUTHORIZED_FOR_LOCATION |
| 404 | NOT_FOUND (107×), ASSIGNMENT_NOT_FOUND_OR_* (5 variants) |
| 409 | CONFLICT, SLUG_TAKEN, CATEGORY_NOT_EMPTY, ACTIVE_DELIVERY_EXISTS, INVALID_TRANSITION, IDEMPOTENCY_CONFLICT, DUPLICATE_KEY, FK_VIOLATION, VENUE_CLOSED, NOT_PUBLISHED, NOT_CASH_PAYMENT, NOT_DELIVERED, NOT_LEAVE_AT_DOOR, NO_ACTIVE_SHIFT, NO_COURIER_ASSIGNED, CANNOT_GO_OFFLINE_WITH_ACTIVE_ORDER, CANCEL_NOT_ALLOWED_STATUS, RATING_WINDOW_CLOSED |
| 410 | CANCEL_WINDOW_EXPIRED, INVITE_INVALID, OTP_EXPIRED |
| 413 | FILE_TOO_LARGE |
| 422 | CASH_AMOUNT_TOO_LOW, IDEMPOTENCY_KEY_REUSED, PRODUCT_NOT_FOUND/UNAVAILABLE, MODIFIER_* (dyn), MIN_ORDER_NOT_MET (ad-hoc) |
| 429 | RATE_LIMIT, OTP_LOCKOUT, OTP_RATE_LIMIT |
| 5xx | INTERNAL (13×), SERVICE_UNAVAILABLE 503 (9×), MISSING_COLUMN 500 |

**Rust mapping (design):** one `enum ApiError` with a variant per code family, each variant
carrying `(code: &'static str, status: StatusCode, fields: Option<Vec<FieldIssue>>, retry_after_ms: Option<u64>)`;
a single `impl IntoResponse` that emits the envelope above **verbatim including the legacy
`status`+`error` aliases** (FE call sites read `errorData?.message || errorData?.error`,
`apiClient.ts:211` — drop the alias only after the FE is fully on Astro/Svelte and re-audited).
The 68-code list becomes a `#[non_exhaustive]` const table; a CI conformance test asserts
`rust_codes ⊇ node_codes` (extracted by the grep above) — this is a §8 map-coverage gate lane.
The 51 ad-hoc bypass sites are the rewrite's opportunity: in Rust there is exactly one exit
(IntoResponse); the port must map each bypass site's shape to the canonical envelope and the
E2E slice must prove the FE still branches correctly (esp. `MIN_ORDER_NOT_MET` on checkout).

**FE expectations (what the new stack must keep true):** parse at `apiClient.ts:163-211`
(`.code`/`.correlationId` getters `apiClient.ts:73-97`; correlationId shown as on-screen support
code); **no central mapApiError exists** — per-page `if (err.data?.code === 'X')` chains
(`CheckoutPage.tsx:420-449`, `MenuFirstOnboarding.tsx:107,127`); unknown code → generic i18n
fallback (`checkout.order_failed`); network error is NOT an ApiError (no .status → generic +
phone-fallback CTA); non-JSON → statusText; 10s AbortController timeout → synthetic
`ApiError(408)` (`apiClient.ts:121-122,227-229`).

---

## §4 Auth/session lifecycle census — 🔴 every row (council-gated port)

Architecture fact that shapes the whole port: **Bearer-JWT-only, zero cookies.** The API sets no
cookie anywhere (`grep -rn cookie apps/api/src` → only logger/sentry redaction lists; no
`@fastify/cookie` registered). All session state is client-side (`localStorage`/`sessionStorage`
`dos_access_token`/`dos_refresh_token`, `apps/web/src/lib/safeStorage.ts:5-27`) sent as
`Authorization: Bearer`. The Rust port either preserves this exactly or redesigns to httpOnly
cookies as an explicit council decision (AUTH-GAP-5 below) — not by accident.

**Extraction command + count:** 8 auth-touching route files
(`find apps/api/src/routes -iname "*auth*"` → 4, plus `customer/otp.ts`, `customer/track.ts`,
`public/claim.ts`, `owner/courier-invites.ts`), **29 route handlers** across them
(`grep -c "fastify\.\(get\|post\|put\|delete\)("` per file) + 1 inline mint at `server.ts:549`.
**12 distinct flows** (table below).

| ID | Flow | Entry (file:line) | Mechanism | Token/TTL | Revocation | Rebuild note |
|---|---|---|---|---|---|---|
| AUTH-01 🔴 | Owner email+password | `POST /api/auth/local/login` — `apps/api/src/routes/auth/local.ts:36` | argon2id verify (`local.ts:100-108`); membership resolve `local.ts:113-139` | RS256 access 24h (`local.ts:146`) + opaque refresh sha256-hashed, `auth_refresh_tokens` 7d (`local.ts:147-159`) | logout deletes all user rows; refresh-reuse kills family | argon2 crate; identical claim set |
| AUTH-02 🔴 | Google OAuth (owner) | `GET /api/auth/google` + `/callback` — `apps/api/src/routes/auth.ts:34,62` | PKCE S256, state/nonce in Redis TTL 600s (`auth.ts:46`); linking by `google_sub` then email (`auth.ts:122-144`) | 24h access + 7d refresh (`auth.ts:149-157`); one-time handoff code in Redis 60s → `POST /api/auth/exchange` (`auth.ts:173-185`) | same as AUTH-01 | server-gated by `GOOGLE_OAUTH_ENABLED` (404 when off, `auth.ts:38,65`) — port the *gate*, not just the flow; Redis state cache → Pg/in-proc per A19 |
| AUTH-03 🔴 | Telegram owner login (undocumented in mission list) | `POST /api/auth/telegram/start` + `GET /poll` — `auth.ts:191-233` | `telegram_login_tokens` 5min TTL, deep-link `t.me/<bot>?start=login_<token>`, single-use atomic flip (`auth.ts:216-221`) | 24h access + 7d refresh | same family model | FE 2s poll loop 5min deadline (`LoginPage.tsx:38-68`) |
| AUTH-04 | Customer OTP (dark) | `POST /api/customer/locations/:slug/otp/{send,verify}` — `apps/api/src/routes/customer/otp.ts:34,112` | 6-digit `crypto.randomInt`, argon2id-hashed, `phone_otp` 5min + `customer_otp_sessions`; verified_token 15min consumed at order-create (`orders.ts:177-186`) | n/a (feeds order intent, not a session) | attempt lockout 5/1h (`otp.ts:152-175`) | `OTP_ENABLED='false'` default — send step is a `console.log` scaffold (`otp.ts:100`). Port dark, same kill switch |
| AUTH-05 🔴 | Courier invite + redeem | mint `POST /api/owner/locations/:locationId/courier-invites` — `owner/courier-invites.ts:27`; redeem `POST /api/courier/auth/invites/:inviteId/redeem` — `courier/auth.ts:23` | 16-hex code argon2id-hashed, TTL default 48h; role hard-allowlisted `'courier'` ("an invite must never mint an owner", `courier-invites.ts:34-36`); PII encrypted at courier create | redeem mints JWT 14d + `courier_sessions` 30d (`courier/auth.ts:118-136`) — **inconsistent vs login 24h** (AUTH-GAP-3) | `revoked_at` + per-request session bind | port with TTLs *unified* (council decides target values) |
| AUTH-06 🔴 | Courier login/refresh | `courier/auth.ts` login `:335`, refresh `:354-476` | password argon2; refresh format `sessionId.tokenPlain`; reuse of revoked session revokes family (`:418-428`) | 24h JWT / 30d `courier_sessions` row | logout `courier/auth.ts:479` sets `revoked_at` | per-request live bind check is in `verifyAuth` itself (below) |
| AUTH-07 🔴 | Demo/shadow claim (`/s/:slug` → `/claim`) | request `POST /api/claim/request` (`public/claim.ts:49-67`, always generic 202 — no enumeration); ops verify/mint `/internal/acquisition/claim/{verify,mint}` (`modules/acquisition/route.ts:143,160`); accept `POST /api/claim/accept` (`claim.ts:17-43`); decline `:69-83` | single-use 72h token, sha256-hashed, delivered as **URL fragment** `#token=` (never in logs); contact-bound invites only on web accept (`CONTACT_REQUIRED`, `claim.ts:97-115`); atomic `claim_transfer()` SECURITY DEFINER | claimer's existing session; response `reauth:true` — membership re-derives next request | decline → `declineAndErase()` hard-delete | ops plane gated by `PROVISION_OPS_SECRET` (fail-closed 404, `ops-auth.ts:1-31`), independent of dev-auth |
| AUTH-08 🔴 | Dev-auth (staging) — ADR-0003 | dev branch in `local.ts:51-71` + 5 more mint sites (`server.ts:549`, `routes/dev/mock-auth.ts:14,122,204,583-584`) | `devLoginAllowed` = `ALLOW_DEV_LOGIN==='true' && DEV_AUTH_SECRET` (`plugins/dev-guard.ts:29-31`); header `x-dev-auth-secret` timing-safe; global 404 gate on dev paths (`server.ts:405-427`) | dev tokens signed under **separate dev kid/key** — cryptographically unverifiable on prod (`packages/platform/src/auth/jwt.ts:73-115`); boot fail-fast if any dev var set in prod (`packages/config/src/index.ts:230-244`) | n/a | port ALL FOUR layers: flag+secret, path-404, dev-kid segregation, boot guard. E2E suite depends on this flow on staging |
| AUTH-09 🔴 | Refresh + rotation (owner) | `POST /api/auth/refresh` — `auth.ts:235-318` | single-use atomic `UPDATE…SET used=true WHERE used=false`; concurrent-refresh <5s sibling → soft 409; genuine reuse → family DELETE (`auth.ts:270-286`) | re-mint 24h; **role re-derived live** — `401 OWNER_REVOKED` if no active owner membership (`auth.ts:293-306`, ADR-0004 P-c) | family delete | the rotation semantics (409-vs-family-kill) are subtle — needs a dedicated Rust test vector set |
| AUTH-10 | Customer track exchange | `POST /api/customer/track/exchange` — `customer/track.ts:28-86` (pre-auth allowlisted `server.ts:403`) | opaque grant from `customer_track_grants` minted at order-create, 14d TTL (`lib/order-persistence.ts:145-148`); reusable, `use_count` observability | `issueCustomerToken` 7d JWT `{orderId, locationId}` — **no phone claim** (P0-PII, `jwt.ts:122-125`) | expiry only | anonymous order flow (softVerifyAuth, `orders.ts:730`) is the live default while OTP dark |
| AUTH-11 🔴 | Platform admin | not a JWT claim — allowlist table `platform_admins`, re-read EVERY request via root `onRequest` gate (`lib/platform-admin.ts:19-83`, wired `server.ts:829-830`) | DB error → 503 **fail-closed**; structural gate covers `/api/admin*` so no route can forget authz (eslint `local/no-admin-register-outside-plane`) | n/a | revoke = set `revoked_at` | port as axum layer on the `/api/admin` router — structural, not per-handler |
| AUTH-12 🔴 | WS auth | `apps/api/src/websocket.ts:339-380` | (a) deprecated `?token=` query (logged as DEPRECATED); (b) in-band `{type:'auth', token}` first message; both → same RS256 `verifyAuthToken` | — | — | ADR-0013 addendum DRAFT proposes `Sec-WebSocket-Protocol` carriage (`docs/design/ws-token-in-url/ADR-0013-addendum-DRAFT.md`) — rebuild should land the addendum, not the deprecated query param |

**JWT contract (must port byte-compatible during strangler overlap):** RS256 only — sign
`jwt.ts:55-56`, verify rejects any other alg twice (`jwt.ts:105-111`). Claims per role
(Zod union `packages/shared-types/src/legacy.ts:161-175`): base `sub/iat/exp/kid`; owner
`{role:'owner', userId, activeLocationId?}`; courier `{role:'courier', activeLocationId!, jti?}`
(jti = session id for the live bind check `plugins/auth.ts:63-83`); customer
`{role:'customer', orderId, locationId}`. Keys: `JWT_PRIVATE_KEY/PUBLIC_KEY/KID` (+ dev triple,
prod-forbidden). Key selected by header `kid` before verify (`jwt.ts:87-102`); kid+key passed as a
pair so dev-kid-with-prod-key is unrepresentable (`jwt.ts:48-60`, ADR-0003 C.1). **During Phase B
both stacks verify the same tokens against the same keys — this is the strangler's load-bearing seam.**

**Middleware order (load-bearing, port as an ordered tower stack):** authPlugin decorators
(`server.ts:395`) → dev-path 404 gate (`server.ts:405-416`) → `AUTH_PREFIXES` Bearer-presence
pre-check minus `NO_AUTH_PATHS`/OTP regex (`server.ts:417-426`) → per-route `verifyAuth`
(+ courier live session bind) → `requireRole` → `requireLocationAccess` (owner branch does a live
`status='active'` membership re-read, returns **404 not 403** to avoid existence leak,
`plugins/auth.ts:117-159`) → admin-plane gate (order comment: "MUST run AFTER verifyAuth").
Also: 4 direct `verifyAuthToken` call sites in `spa-proxy.ts:62,100,113,127` (parallel authn path —
dissolves when Astro takes over the shell, but the *authz decisions* it makes must be re-homed).

**Client-side session slots (FE contract):** `localStorage dos_access_token`/`dos_refresh_token`,
`sessionStorage dos_access_token` (write-only — likely dead), `dos_auth_expired` (one-shot banner
flag, `apiClient.ts:192` → `LoginPage.tsx:31`), `dos_claim_token` (pre-signin stash,
`ClaimPage.tsx:144`). Silent-refresh trigger in `apiClient.ts:26,155`.

**Gaps found (feed into rebuild backlog, do not silently "fix" during port):**
- AUTH-GAP-1: FE "Exit" button (`apps/web/src/routes/AdminRoutes.tsx:145-150`) never calls
  `POST /api/auth/logout` and never clears `dos_refresh_token` — refresh family survives logout.
  ADR-0004 P-b wiring was never completed (target file no longer exists).
- AUTH-GAP-2: `POST /api/auth/courier/activate` (`auth.ts:339-393`) has **zero FE callers** — dead
  parallel activation path, 7d TTL into the *owner* refresh table. Decide delete-vs-port explicitly.
- AUTH-GAP-3: courier TTL matrix inconsistent (redeem 14d JWT / login+refresh 24h JWT / session row 30d).
- AUTH-GAP-4: no password-reset flow exists at all (grep-confirmed zero hits).
- AUTH-GAP-5: zero-cookie architecture = tokens XSS-exfiltrable from localStorage; council decides
  keep-vs-httpOnly at the auth-surface port (Phase B1), not mid-implementation.
- ADR-0003 residual: leaked prod-kid token killable only by operator key rotation (R-6 open).

ADR set governing this section: `docs/adr/0003-dev-login-fail-closed.md`,
`docs/adr/0004-owner-token-revocation.md`, `ADR-admin-platform-authz.md`, `ADR-audit-fix-authz.md`,
`ADR-authz-state-hardening.md`, `ADR-b3-deep-auth-hardening.md`, `ADR-security-hardening-2026-07.md`,
`ADR-p0-privacy-hardening.md`, `docs/design/ws-token-in-url/ADR-0013-addendum-DRAFT.md`.

---

## §5 Integration census — **17 integrations** (10 live · 4 dark · 3 inert/dead)

**Extraction:** outbound-host census
`grep -rnoE "https?://[a-zA-Z0-9.-]+" apps/api/src apps/web/src packages/{platform,voice,ui}/src --include='*.ts' --include='*.tsx' | grep -viE '\.test\.|__tests__|/dist/' | grep -oE "https?://[a-zA-Z0-9.-]+$" | sort -u`
→ **35 distinct hosts, every one reconciled** to a row below (or classified: self-hosts,
XML/JSON-LD namespaces, placeholder strings, SSRF-blocklist literal). Re-run must include
`packages/config/src` (ORS default URL lives there — extraction-command gap found and fixed here).

| ID | Integration | Entry (file:line) | Direction | Env/flags | Status | Rebuild note |
|---|---|---|---|---|---|---|
| INT-01 | Telegram (bot notify + webhook + login + connect) | webhook `routes/telegram-webhook.ts:36` (secret-pathed, reg `server.ts:528`); adapter `notifications/adapters/telegram.ts:4`; poller `workers/telegram.poll.ts:4`; prefs `routes/owner/notifications.ts` | in+out (`api.telegram.org`, hand-rolled fetch, no SDK) | TELEGRAM_BOT_TOKEN/SECRET/USERNAME; TG_CATEGORY_GATING + TG_STOREFRONT_ACTION dark | LIVE core + dark sub-features | Tables: owner_notification_targets, telegram_connect/login_tokens, notification_outbox_audit. `telegram_alert_detail∈{minimal,area,full}` PII minimization (ADR-p0 D4). WhatsApp/Baileys was REMOVED — never re-add |
| INT-02 | Web-push | adapter `notifications/adapters/webpush.ts:7`; `GET /api/push/vapid-public-key` (`public/vapid.ts:5`); customer+owner subscribe routes (`customer/push.ts:21`, `owner/push.ts:23`) | out (web-push lib) | VAPID_PUBLIC/PRIVATE_KEY/SUBJECT (adapter registered only if keys present, `bootstrap/notifications.ts:51`) | LIVE-if-configured | `customer_devices` table. Owner subscribe route has **no FE caller** (server-only). SW has NO push/notificationclick handler — browser-default display. Dead scaffold: `adapters/push.ts` (@ts-nocheck, never imported) |
| INT-03 | Cloudflare R2 | presign `owner/product-media.ts:82,178`; backup upload `workers/backup/upload.ts:16`; verify `backup/r2-verify.ts:131` | out (S3 API; browser→R2 direct presigned PUT) | R2_* five; BACKUP_ENABLED + BACKUP_ENCRYPTION_KEY + 4 crons | media LIVE (clean 503 unconfigured); backups DARK | Falls back LocalFsStorageProvider (`server.ts:306`). Rust: aws-sdk-s3 or object_store crate; presign contract must match FE uploader |
| INT-04 🔴 | Plisio crypto (ADR-0017) | adapter `lib/payments/plisio.ts:35`; registry `lib/payments/registry.ts:12`; webhook `routes/payments-webhook.ts:13` (404 when off); order-create fork `orders.ts:654`; refunds queue `owner/refunds.ts` | out (`plisio.net` invoice) + in (HMAC webhook, `plisio.ts:69`) | PAYMENTS_PREPAID_ENABLED ∧ PAYMENTS_CRYPTO_ENABLED (both drift, both off); PLISIO_SECRET_KEY (drift!) | fully DARK, zero FE | `payments` (integer minor units + residual CHECK) + `payment_events` append-only ledger; RLS FORCE dual-policy + DEFINER `payment_location_by_provider_ref` (mig 083). Refund manual (`refund_due`→`refund_sent`). NEEDS-HUMAN: live Plisio verify_hash validation |
| INT-05 | Routing/ETA (ORS) | `packages/platform/src/routing-provider.ts:120` circuit-breaker + haversine fallback (never rejects); consumers `workers/courier-events.ts:6`, `customer/orders.ts:8` | out (`api.openrouteservice.org`) | ROUTING_PROVIDER/BASE_URL/API_KEY (no key ⇒ 401 ⇒ silent haversine) | LIVE (degraded-by-default without key) | + separate CLIENT-side OSRM demo fetch `MenuPage.tsx:476-483` (`router.project-osrm.org`) for menu ETA — distinct path, easy to lose |
| INT-06 | Map tiles | `packages/ui/.../MapLibreBase.tsx:19` → `tiles.openfreemap.org/styles/liberty` (VITE_TILE_* seam, ADR-GEO-SEAMS) | out (browser) | FLAG-C15/C16 | LIVE | Inconsistency: `AnalyticsPage.tsx:499` hardcodes `basemaps.cartocdn.com` bypassing tileConfig + CSP list — fix during port, don't copy |
| INT-07 🔴 | GPS guard + customer live-location (P0 privacy) | `lib/courier-gps.ts:9-13` (active statuses exclude 'assigned'; 24h retention); enforced `courier/shifts.ts:305,382`; purge `workers/courier-cron.ts:38`; customer share = pure WS relay `websocket.ts:458-476`, NO DB write, auto-stop on DELIVERED | — | COURIER_GPS_MAX_DIST_KM | LIVE | The consent boundary (`accepted/picked_up` only) and no-persist relay are 🔴 privacy invariants — port with red→green proof each |
| INT-08 | Analytics export (ETHICAL-STOP) | client-only `apps/web/src/lib/exportCSV.ts` (exportCSV/JSON/JSONL, strips `_` fields); AnalyticsPage uses CSV+JSON; Dashboard/CRM/Couriers pages CSV-ONLY (PII tier — JSON deliberately withheld) | none (client serialization) | — | LIVE (analytics tier only) | The withholding IS the behavior: council brief `docs/design/owner-data-export-ai-council-brief.md:26-31` blocks JSON on PII pages until council. No server export endpoint exists — do not invent one in the port |
| INT-09 🔴 | GDPR erase + anonymizer | request `owner/gdpr.ts:33`; worker `workers/anonymizer-gdpr.ts:8` (FOR UPDATE SKIP LOCKED); **N1 backstop** `:67-94` — never `completed` without re-reading `customers.anonymized_at`; unconfirmed ⇒ `failed` + bus event, never false-complete; retention worker `anonymizer-retention.ts:13` | — | ANONYMIZER_RETENTION_CRON | LIVE. DEFINER erase = OPERATOR-GATED DRAFT NOT APPLIED (`docs/design/audit-fix-rls-reliability/migration-drafts/1790000000088_gdpr-erase-definer.ts`) | Tables: gdpr_erasure_requests, anonymization_audit_log. No customer self-service (owner-mediated by design). N1 backstop is a 🔴 port-verbatim behavior |
| INT-10 | PII primitives | `lib/pii-redactor.ts:8` (fail-closed regex scrub → Pino `logger.ts:3,7` + Sentry beforeSend `sentry.ts:2,17` + menu-region + backup-verify); `pii-mask.ts` (display masking, ~11 route consumers); `pii-cipher.ts:1` (AES-256-GCM, COURIER_PII_ENCRYPTION_KEY); `pii-leak-detector.ts` (TEST-time SSR scanner); `redactUrlSecrets` (`logger.ts:18-39`) | — | COURIER_PII_ENCRYPTION_KEY | LIVE | 🔴 cipher must round-trip the SAME key+format (data is shared across stacks during overlap); redactor semantics = fail-closed (error → redact everything) |
| INT-11 | Voice stack (ADR-0015) | `packages/voice/src/*` (WhisperProvider sink-free by construction; TransformersTranscriber dynamic-import `Xenova/whisper-base` q8); kill `lib/voice-flag.ts:11-12`; `GET /api/public/voice-config` (`public/voice-config.ts:11`, no-store, under /api so SW never pins); CSP connect-src widened to R2 only while enabled (`spa-shell.ts:151-159`) | out (model fetch from R2) | VOICE_CONTROL_ENABLED ∧ ¬VOICE_KILL (both drift) | DOUBLE-DARK: flag off AND MicFab never mounted (`docs/design/voice-fe-mount/resolution.md` design-stage; VoiceMount.tsx doesn't exist) | Port as-is dark or defer whole stack to post-rebuild — matrix row either way |
| INT-12 | Compliance gates | `/compliance` 15 files; CI step `ci.yml:50` `compliance:gate` → `scripts/compliance-gate.ts` (A: PII-migration↔data-map; B: service envs↔subprocessors.md; C: no raw PII on log/bus/queue sinks; D: DPIA for high-risk); + `guardrail:license` env-classification gate (§2) | — | — | LIVE | **Gaps found:** gate's SERVICE_ENV misses PLISIO_SECRET_KEY, TRANSLATION_ENDPOINT (LibreTranslate), MEM0_OLLAMA_URL — absent from subprocessors.md too. Fix before port; port the gate itself against the Rust config struct |
| INT-13 | Email (Resend, ops-only) | `notifications/adapters/email.ts:26` hand-rolled fetch `api.resend.com/emails`; sole caller `workers/access-request-notify.ts:31` | out | RESEND_API_KEY (absent ⇒ `email-disabled` result) | LIVE-if-key | Deliberately NOT in the tenant dispatcher — ops alert only. No SMS provider exists anywhere (OTP scaffold only) |
| INT-14 | AI menu-import chain | `lib/ai-ocr-parser.ts:82-109` provider union ollama/groq/openai/openrouter/zen/mock/heuristic; Tesseract.js in-process OCR; LibreTranslate `lib/libretranslate-provider.ts:5` (3-failure ⇒ pass-through degrade); brand-extractor `lib/brand-extractor.ts:1` (owner-URL fetch, SSRF-guarded incl. 169.254.169.254 block, `:252`) | out (groq/openai/openrouter/opencode-zen/localhost) | LLM_*/GROQ_*/OPENAI_*/OPENROUTER_*/OPENCODE_ZEN_*/TRANSLATION_*; MENU_GROUNDING_ENABLED dark | LIVE, degrades to zero-dep heuristic without keys | Rust: sidecar-vs-crate is Lane A's OCR decision; SSRF guard must port with its test corpus; LibreTranslate not in subprocessors.md (gap) |
| INT-15 | Observability | Sentry backend-only `lib/sentry.ts:54` (full PII redaction pipeline + tag allowlist; no FE SDK); Plausible script tag `apps/web/index.html:34` (cookieless); first-party telemetry `POST /api/telemetry(+/abuse)` (`public/telemetry.ts:37,84`) → analytics_events/cwv/abuse_log | out + in | SENTRY_DSN, GIT_SHA | LIVE | Rust: sentry crate + the SAME beforeSend redaction semantics (fail-closed); keep first-party telemetry endpoint contract (FE beacons) |
| INT-16 | Messenger deep-links (ADR-0016) | `apps/web/src/lib/messenger.ts:48` — tel/t.me/wa.me/viber/signal.me/SimpleX href generator; `couriers.messenger_kind` column | href only, no fetch | — | LIVE | Required "Communication" selector at checkout — pure FE port, but the 6-kind table is a behavior census item |
| INT-17 | Dead/inert scaffolds | Turnstile plugin `plugins/turnstile.ts` (never registered, zero call sites); mem0/Ollama `lib/memory.ts:15` (initialized `server.ts:293-297`, never consumed); `adapters/push.ts`; vestigial CSP hosts `cdn.jsdelivr.net`, `en.wikipedia.org` (no code refs) | — | MEM0_* | INERT | Explicit RETIRE rows in the matrix — the map must prove they were dropped on purpose, not lost |

Static `<a href>` externals (google.com/maps search, search.google.com review link,
maps.app.goo.gl placeholder, instagram/facebook owner-entered fields): no API calls — port as
content. `google_rating/google_review_count/google_maps_url` on `location_themes` are
owner-entered/scraped, no live Places API anywhere.

---

## §6 Realtime client contract — what the FE actually consumes

**Extraction:** `grep -rn "useWebSocket(" apps/web/src` → **5 call sites**; filtered
`grep -rhoE "\.type === '[a-zA-Z_.]+'" apps/web/src` census → **12 FE-handled WS message types**
(after excluding non-WS unions); server-side `messageBus.publish` census → **27 business types**
on room-shaped channels (+5 control types emitted in `websocket.ts`:
`auth_success`/`subscribed`/`error`/`client_location`/`client_location_stop`). Room grammar:
`order:<id>`, `location:<id>:dashboard`, `location:<id>:couriers`, `courier:<id>`,
`courier:<id>:shift`. NOTE: `BUS_CHANNELS.*` (`apps/api/src/lib/registry.ts:1-45`) is a separate
INTERNAL bus namespace consumed by notification/worker subscribers only — never browser-reachable;
do not conflate the two in the port.

| WS-ID | Type | Payload (1-line) | Consumer surface | FE file:line |
|---|---|---|---|---|
| WS-01 | `auth_success` | `{type,role}` | hook-internal handshake | `apps/web/src/lib/useWebSocket.ts:70-77` |
| WS-02 🔴 | `order.status` | `{orderId,status,statusAt*,locationId,timestamp}` (also wrapped `{room,data}`) | admin+storefront+courier | `admin/DashboardPage.tsx:149-151`, `client/OrderStatusPage.tsx:267-288`, `courier/DeliveryPage.tsx:174-177` |
| WS-03 🔴 | `order.created` | PII-free claim-check `{orderId,locationId,status,total,currency,shortId,itemCount}` | admin | `DashboardPage.tsx:141-148` |
| WS-04 | `order.message` | chat `{data:{id,…}}` | storefront+courier | `OrderStatusPage.tsx:290-295`, `DeliveryPage.tsx:165-171` |
| WS-05 | `order.route` | `{payload:{polyline}}` | storefront map | `OrderStatusPage.tsx:239-245` |
| WS-06 | `order.courier_updated` | `{payload:{position,courierName?,phoneMasked?,status?}}` | storefront map | `OrderStatusPage.tsx:247-265` |
| WS-07 | `courier.position_updated` | `{payload:{courierId,position}}` | admin live map | `DashboardPage.tsx:169-170` |
| WS-08 | `courier.shift_updated` | `{payload:{courierId,status}}` | admin live map | `DashboardPage.tsx:171-180` |
| WS-09 | `task_assigned` | `{payload:CourierTask}` | courier tasks | `courier/TasksPage.tsx:89-98` |
| WS-10/11 | `client_location` / `client_location_stop` | customer GPS relay `{payload:{lat,lng}}` / `{}` | courier | `DeliveryPage.tsx:156-164` |
| WS-12 | `error` | silently ignored | storefront | `OrderStatusPage.tsx:232` |

**Reconciliation (the silent-loss class this census exists for):**
- **Published-but-unhandled: 19 types** — `assignment.created`, `binding_changed`,
  `assignment_aborted/expired`, `offer_sent/expired/declined`, `task_offered`, `shift.opened`,
  `preflight.signal_raised/dismissed/acknowledged`, `dwell.alert_created/acknowledged`,
  `dwell.escalation_tier_changed`, `gdpr.erasure_completed`, `customer.contact_revealed`
  (`owner/reveal-contact.ts:64-67`), `courier.assignment_status_changed`, `order.picked_up`.
  Dropped client-side with no warn and no fallback poll (grep-verified zero FE hits); FE
  compensates via REST polling. **Rebuild decision per type: consume, or stop publishing —
  never port blind.**
- Handled-but-never-published: **none** (all 12 confirmed emitted server-side).

**Behavioral contract to preserve:**
- Reconnect/backoff — TWO implementations: main hook infinite retry
  `min(2000·1.5^n, 15000) + jitter(0..1000)`, attempt-reset on `auth_success`/`online`/`focus`/
  `visibilitychange`, no reconnect on clean close 1000/1005 (`lib/useWebSocket.ts:86-141`);
  status-widget capped at 5 attempts, `min(2^n·1000·jitter(0.5–1.5), 30000)`, then emits
  `status:offline`/`fallback:needed` events, no reconnect on 1008 auth-fail
  (`apps/api/src/client/status/ws.ts:53-71`). Server Pg-LISTEN reconnect
  `min(1000·2^n, 30000)` (`packages/platform/src/message-bus.ts:101-105`).
- `_truncated` — NOTIFY payloads >8000 bytes replaced by `{_truncated:true, type, data:{id}}`
  (`message-bus.ts:133-154`). **FE never checks the flag** (grep zero hits); resilience is an
  accident of refetch-on-any-event (`DashboardPage.tsx:143`, `OrderStatusPage.tsx:287`). Rust
  PgListener has the same NOTIFY payload cap — the port MUST keep the refetch-on-event pattern
  or make `_truncated` an explicit FE contract. This is precisely the silently-lost class.
- Cross-tab — Web Locks single-flight token refresh (lock name `dos-token-refresh`,
  `apiClient.ts:52-66`; prevents racing tabs from tripping refresh-reuse family-kill — pairs
  with AUTH-09); `storage`-event cart sync (`lib/CartProvider.tsx:66-80`, key
  `dos_cart_<locationId>`). No BroadcastChannel, no leader election anywhere (grep-verified).
- WS auth: dual-channel (deprecated `?token=` + in-band `{type:'auth'}`), usage instrumented
  redacted (`websocket.ts:179-189`); URL-secret log redaction SHIPPED
  (`lib/logger.ts:13-40,107-118`). See AUTH-12 for the Sec-WebSocket-Protocol addendum path.

---

## §7 Proof-net census — the entire test estate, classified for the rebuild

All counts from commands run 2026-07-04 on `fix/audit-remediation` @ `ae9f5360`
(`.claude/worktrees/*` excluded everywhere).

### §7.1 Census (extraction command → count)

| Estate | Count | Extraction command |
|---|---|---|
| Playwright spec files | **174** | `find e2e -name '*.spec.ts' \| wc -l` (tests/ 170 = 149 flat + a11y 2 + admin 6 + client 10 + courier 3; visual/ 3; lifecycle-e2e/ 1) |
| Playwright test call-sites | **1,194** (1,029 `test(` + 158 `.skip` + 7 `.fixme` + 0 `.only`) | `grep -rhE "^\s*(test\|test\.only\|test\.skip\|test\.fixme)\(" e2e --include='*.spec.ts' \| wc -l` |
| Specs asserting HTTP contract directly | **92 files / 784 `request.*` calls** (53% of suite) | `grep -rl "request\.\(get\|post\|put\|patch\|delete\|fetch\)(" e2e --include='*.spec.ts' \| wc -l` |
| CSS-class-coupled specs (`locator('.`) | **21 files / 48 occurrences** | `grep -rl "locator('\." e2e --include='*.spec.ts' \| wc -l` |
| Visual net | **3 spec files / 27 `toHaveScreenshot` sites → 162 comparisons** (27 × 2 locales al/en × 3 breakpoints 390/768/1280); ledger row #11 says "180" — live count is 162 | per-file `grep -c toHaveScreenshot`; `playwright.visual.config.ts` |
| Visual baselines committed | **0** — `__screenshots__/` does not exist; `.github/workflows/visual.yml` runs compare-mode and would fail on first trigger | `git ls-files \| grep __screenshots__ \| wc -l` |
| axe/a11y | **4 spec files** (2 dedicated in `e2e/tests/a11y/` incl. the red-proof suite that plants broken DOM to prove the sense fires; sq/en/uk loops) | `grep -rl "axe\b\|AxeBuilder" e2e --include='*.spec.ts'` |
| Unit tests (node:test) | **181 files in `test:unit` glob (184 repo-wide); 1,198 cases / 53 suites / 81 skipped / 26.1s** (authoritative run, `05-dx…md:18`; static line-start grep = 861 — node:test counts nested `t.test()` subtests) | glob from `package.json:35`; `find . -name '*.test.ts' -not -path '*/node_modules/*' …` |
| — breakdown | api 124 files/458 sites · web 15/91 · ui 11/157 · voice 5/25 · platform 3/6 · worker 1/1 · loop-harness 21/122 (harness, not product) · ccc 1/1 | per-dir `find`+`grep` |
| — escaping the glob | 3 files: `tools/eslint-plugin-local/__tests__/rules.test.ts` (glob matches `tests/` not `__tests__/`), `e2e/rites/song-of-singularity.test.ts`, `.opencode/plugin/harness.test.ts` | `find` − glob diff |
| RLS adversarial | `phase5/rls-adversarial.test.ts` — **8 IDOR tables** cross-tenant raw-PK sweep (skips honestly without DB envs) + `verify:rls` script — **25 tenant tables** default-deny | `grep -A10 IDOR_TABLES …`; `packages/db/scripts/verify-rls.ts` |
| Mutation testing | **0 framework** (no Stryker). "Mutants" = documented manual discipline: hand-broken source re-run against the suite, "N fail" recorded (ledger #21/#39-42; ADR-0013 = 25 tests (12+13) + 2 hand mutants) | `grep -i stryker package.json` → none |
| Budgets | `.size-limit.json`: storefront entry 250 kB (map excluded) + map chunk 1.2 MB; `lighthouserc.cjs`: perf ≥0.8, a11y ≥0.9, LCP ≤2.5s, CLS ≤0.1, TBT ≤300ms on `/s/demo` + checkout ×3 runs. **Neither wired into any CI workflow** (grep of workflows → 0 hits) | `cat .size-limit.json lighthouserc.cjs` |
| verify/guardrail/test scripts | **19 `verify:*` + 6 `guardrail:*` + 17 `test:*`** of 70 root scripts; `verify:all` = **25 composite gates** (audit doc said 23 — 2 added since, live drift) | `grep -oE '"verify:[a-z-]+"' package.json`; `grep -c "{ name:" scripts/verify-all.ts` |
| ESLint local rules | **26 defined / 25 wired** (`no-process-exit` defined, never enabled — dead rule) | `tools/eslint-plugin-local/src/index.js` vs `eslint.config.js` |
| CI reality | 3 workflows (ci/visual/skill-security). `test:unit` in CI: **ZERO**. Post-deploy E2E in CI: **4 of 174 specs** (deploy-validation, flow-core-lifecycles, telegram-webhook, telegram-full-flow). Pre-commit: lint→3 guardrails→i18n-parity(cond)→typecheck→build→fly validate→Docker build; **no unit tests** | `ci.yml`, `.husky/pre-commit` |
| Other estates | i18n-parity gate (`scripts/i18n-parity.ts`, sq/en/uk, hard-fail); boot schema-guard; migration-ordering gate (157 migrations); 8 `apps/api/scripts/verify-*` static contract gates; injection corpus (21 files/5 categories); LLM-persona driver harness (`e2e/personas` 23 + driver — separate from deterministic net); regression ledger **67 rows**; **37 ADRs** | §9 of the agent transcript; `grep -cE "^\| [0-9]+ \|" docs/regressions/REGRESSION-LEDGER.md` |

### §7.2 Classification: PARITY-ORACLE / NEEDS-REBASE / STACK-BOUND

**PARITY-ORACLE — runs as-is against Rust+Astro (the rebuild's language-independent contract):**
- **151 of 174 Playwright specs** (174 − 23 rebase set below). Everything asserting URLs,
  user-visible text (getByRole/getByText), HTTP status/envelope (`request.*` — 92 files, incl.
  `error-contract.spec.ts`, `flow-security-contracts.spec.ts`, `admin-platform-authz.spec.ts`
  "owner JWT is 403 on EVERY /api/admin endpoint", `courier-room-authz-isolation.spec.ts`
  ADR-0013 positive control, `no-cookies-invariant.spec.ts`), and DB effects. **Precondition:**
  the Rust stack must port AUTH-08 dev-auth (specs mint staging tokens through it) and keep
  `/s/:slug`, `/admin/*`, `/courier/*` URL space (mission rule: never invent routes).
- **RLS suites** — `verify:rls` (25 tables) + `rls-adversarial` (8 IDOR tables): pure SQL against
  the unchanged DB; zero porting.
- **lifecycle-e2e/critical-lifecycle.spec.ts** — the single full customer→owner→courier→cash trace.
- **i18n-parity gate** — stays as-is IF the catalog SSOT format is kept (Lane B decision);
  otherwise rebase the extractor, keep the assertion (3-locale key parity).
- **Injection corpus + guardrail:corpus-reachability** — harness-level, stack-agnostic.

**NEEDS-REBASE — semantic assertion ports, selector/pixel re-baselines (23 spec files):**
- **21 CSS-coupled files** (`locator('.` — list in agent §1.5: admin/dashboard, menu-manager,
  supplies, client/cart, menu, menu-interaction, courier/full-coverage, ui-polish, undo-redo,
  live-smoke, media-render, flow-* UI specs, real-session-menu, owner-fixes-batch…): rewrite the
  48 class-selectors to role/testid; every `expect(...).toBeVisible/ContainText` survives.
- **3 visual specs / 162 comparisons**: re-baseline against Astro/Svelte pixels with human
  review (§8d). **BLOCKER: baselines must first be locked on the CURRENT stack** (today 0 PNGs
  committed — there is no "before" to diff a rewrite against; execute
  `docs/operating-model/proposed-visual-ci/APPLY.md` before Phase A).
- **Budgets**: size-limit paths re-pointed at Astro output chunks (values carry over: 250 kB
  entry / 1.2 MB map); lhci config unchanged (URL-based). Wire both into CI at last (today
  neither runs).

**STACK-BOUND — replaced by Rust/Svelte tests; the INVARIANTS below must be re-covered 1:1
(this list IS the Rust test backlog):**

| Invariant cluster (Node files today) | Rust/Astro re-cover target |
|---|---|
| 🔴 Money: `money-tax`, `order-pricing`, `order-total-composition`, `order-total-route-composition`, `fee-parity`, `settlements-catchup`, `refund-due-spine`, `promotions-money-render`(FE), `ssr-jsonld-price` | Rust: integer-minor-unit vectors + fee server-mirror parity + total-composition property tests (port the exact vectors, then add proptest); JSON-LD price in Astro SSR test |
| 🔴 State machine: `order-machine-transitions`, `orders-status-patch-guards`, `grace-cancel-lifecycle`, `acquisition-state-machine`, `dispatch-recovery`, `customer-cancel-after-dispatch`, `deliver-*` (completion/handshake/drift) | Rust: `assertTransition` as exhaustive-match enum — invalid transition unrepresentable; port every guard vector incl. cash-as-proof HOLD |
| 🔴 Idempotency: `order-canonical` (buildRequestHash), asserted inside `orders-authz`, `order-persistence`, `venue-open`, `dispatch-recovery`, 9 more | Rust: canonical-hash test vectors byte-identical to Node's (same key → same hash across stacks during overlap) |
| 🔴 WS/realtime authz (ADR-0013): `courier-room-authz` (12), `websocket-authz` (13), `courier-relay-guard`, `websocket-churn` + 12 `*-authz` route tables | Rust: tri-state subscribe+fan-out tests + repeat the 2 hand-mutant red-proofs; per-route authz table tests |
| 🔴 Auth/JWT: `jwt-alg-kid-pin`, `argon2-params-lock`, `auth-refresh-race`, `auth-refresh-role`, `courier-session-binding`, `dev-guard`, `boot-guard-prod`, `platform-admin(-gate)` | Rust: alg/kid pinning, argon2 param lock (same m/t/p), refresh-rotation race vectors (AUTH-09 409-vs-family-kill), session bind, dev-guard fail-closed, boot guard |
| GDPR/PII: `anonymizer-fail-closed`, `anonymizer-gdpr-backstop` (N1), `anonymizer-gdpr-worker-provenance`, `pii-cipher`, `pii-redactor`, `pii-leak-detector`, `courier-history-pii`, `menu-region-pii`, `ocr-redaction`, `p0-privacy`, `logger-url-redaction` | Rust: fail-closed redaction (error → redact-everything), N1 backstop, PII cipher round-trip with SAME key format (data is shared!), URL-secret redaction in tracing layer |
| Error contract: `send-error`, `rate-limit-envelope` | Rust: IntoResponse envelope conformance + code superset test (§3) |
| SSR/shell: `spa-proxy(-authz)`, `spa-shell`, `ssr-client-shell`, `ssr-escaping`, `subdomain-rewrite`, `theme-renderer`, `preview-render` | Astro: escaping/XSS tests on SSR menu, subdomain rewrite, theme/branding render (derivePalette port) |
| i18n/theme/FE hooks: `ui` 11 files (i18n 19 tests, palette.contrast, fonts, characteristics, hooks), `web` 15 files (dashboard-utils, undo-redo, focus-trap, no-fabricated-fallback, voice adapter) | Svelte island tests (vitest): same behavioral assertions incl. AA-contrast checks on derived palettes and the no-fabricated-fallback rule |
| Voice (dark): `packages/voice` 5 files (capability-table, confirmation-gate, matcher, providers) + `voice-flag` | Port only with the voice surface (dark); capability table = data file, test ports verbatim |
| Platform: `message-bus-dispatch/notify`, `queue-provider-reliability`, `r2-storage`, `image-key/url`, `keyset-pagination`, `client-ip-ratelimit`, `metrics`, `health-*` | Rust: PgListener 8000-byte `_truncated` behavior, queue retry/DLQ semantics (Lane A tool), R2 presign, keyset pagination, rate-limit, health truthfulness |
| Guardrail culture: 26 ESLint rules, 8 `verify-*` static gates, 25 `verify:all` gates | clippy deny-set + custom lints + cargo-deny (Lane D map); each rule gets an explicit port/retire row in the matrix — a silently dropped lint is a silently dropped invariant |
| Harness (NOT product): `tools/loop-harness` 21 files, injection corpus, ccc | stays Node — no port |

**Split summary: 151 PARITY / 23 REBASE / 181-unit-file estate STACK-BOUND** (+ scripts/gates
per the last two rows).

**Proof-net gaps to fix BEFORE Phase A (they gate the rebuild's own verifiability):**
1. Lock visual baselines on the current stack (0 committed today; visual.yml would fail).
2. Put `test:unit` in CI (REBUILD-PLAN P0.2) — the invariant inventory above is only
   trustworthy if it runs.
3. Triage the 158 `test.skip` + 7 `fixme` — each is either dead (delete) or an unproven
   behavior that the rebuild would silently lose (matrix row).
4. Wire size-limit + lhci into CI (defined, never enforced).
5. Decide the 4-of-174 post-deploy CI subset → §8's per-surface slices supersede it.

---

## §8 Completeness methodology — how "nothing missed" becomes provable

The principle: **the map is data, not prose.** Every census in this doc (and in inventory/10–13)
reduces to machine-extractable ID lists; completeness = three set-differences being empty, checked
in CI, per phase. Narrative review never certifies coverage — only the gate does.

### §8a Master traceability matrix

One machine-readable file: `docs/design/rebuild-plan/inventory/traceability.csv` (CSV so it diffs
line-per-row and scripts parse it; a rendered .md view can be generated). Columns:

```
ID, class, current_artifact, behavior, rebuild_target, proof_artifact, phase, redline, status
```

- **ID** — stable, namespaced: `ROUTE-*` `PAGE-*` `WS-*` `JOB-*` `TBL-*`/`POL-*` `FLAG-*` `ENV-*`
  `ERR-*` `AUTH-*` `INT-*` `KEY-*` (i18n) `GUARD-*` `TEST-*` (namespaces already used across
  inventory/10–14; this doc contributes FLAG/ENV/ERR/AUTH/INT/WS/TEST rows).
- **current_artifact** — `file:line` in the Node codebase (or `pg_catalog` ref for TBL/POL).
- **behavior** — one line, user/system-observable (what a reviewer would miss if dropped).
- **rebuild_target** — `crate::module::item` / `apps/astro/src/...` / `KEEP` (unchanged, e.g. DB)
  / `RETIRE(reason)` — RETIRE is a first-class target: dead code (§1/§2/§5 inert rows) must be
  dropped *on record*, never silently.
- **proof_artifact** — the spec file / Rust test / gate / contract-diff that proves the port
  (PARITY spec name, or the §7.2 invariant-cluster test, or "map-gate lane N").
- **phase** — A (storefront-read spike) / B1 (auth) / B2 (catalog/admin) / B3 (orders/money 🔴) /
  B4 (realtime 🔴) per `06-*.md §3`.
- **redline** — 🔴 ⇒ Triadic Council APPROVED required before status can pass MAPPED.
- **status** — ratchet-ordered: `UNMAPPED → MAPPED → BUILT → PROVEN → CUTOVER`. Status may never
  decrease (guard in the gate script).

### §8b The map-coverage gate (CI-fails on unmapped / unbuilt / orphan)

`tools/rebuild-map-gate/` — a script pair per namespace, plus one differ. Design:

```
extract-old.<ns>  → IDs from the Node codebase        extract-new.<ns> → IDs from Rust/Astro
──────────────────────────────────────────────────────────────────────────────────────────
routes   fastify route registrations (grep + a        utoipa OpenAPI dump (cargo run --bin
         route-table dump run against the booted       openapi-json) → paths×methods
         app: GET /api/dev/routes or fastify.printRoutes)
pages    react-router route config parse               Astro file-route glob (src/pages/**)
flags    §1a grep + §1b schema∪drift list              Rust config-struct field dump (derive
                                                       macro emits JSON) + Astro PUBLIC_* grep
envs     EnvSchema keys ∪ process.env drift grep       same config-struct dump
errors   §3 sendError grep (+ ad-hoc code: grep)       error-enum variant dump (strum/serde)
ws       §6 publish-type grep + FE handled-type grep   WS message enum variants (serde tags)
                                                       + Svelte handler census
i18n     catalog SSOT key parse                        Paraglide/ported-catalog key parse
tables   pg_catalog query (tables+policies) — DB       SAME extractor, same DB (unchanged) —
         is shared, so old=new by construction;        gate asserts sqlx offline metadata
                                                       references ⊆ live schema
jobs     pg-boss registrations grep                    Rust queue job-registry dump
guards   eslint-local rule list + verify:all gate      clippy.toml deny-set + cargo-deny +
         names + husky stages                          CI job list (Lane D mapping table)
tests    spec/list of e2e + unit invariant clusters    cargo test --list + Playwright list
```

Gate logic (one differ, three failure classes, run per-PR in both repos + nightly):
1. **UNMAPPED** — `extract-old ∖ matrix.IDs ≠ ∅` → someone added Node behavior without a map row
   (or the initial census missed it — either way it surfaces *mechanically*).
2. **UNBUILT** — `{rows: phase ≤ current ∧ status ≥ BUILT} ∖ extract-new ≠ ∅` → the matrix
   claims progress the new codebase doesn't contain.
3. **ORPHAN** — `extract-new ∖ matrix.IDs ≠ ∅` → scope creep in the rebuild; new behavior needs
   a row (and, if 🔴-adjacent, a council) before it exists.
Plus two invariants: status monotonicity (no row regresses) and RETIRE-verification
(`RETIRE` rows must NOT appear in extract-new).

The extractors are deliberately dumb (grep/AST-lite/one SQL query/one cargo run) — auditable in
minutes; sophistication lives in the census regexes already proven in this doc (each §
carries its extraction command precisely so the gate can reuse it verbatim).

### §8c Per-surface cutover DoD (Phase B gate, per surface)

A surface (auth / catalog-admin / orders-money / realtime / storefront-read) flips its proxy route
to Rust only when ALL of:
1. **E2E slice green** — the surface's Playwright subset (from §7.2 PARITY list, tagged per
   surface in the matrix) passes with `VITE_BASE_URL` → the Rust+Astro staging deployment,
   including its error-path specs (`error-contract.spec.ts` always in every slice).
2. **Contract diff empty** — recorded Node responses (status + envelope shape + headers incl.
   `x-correlation-id`) vs Rust responses for the surface's routes: zero semantic diff
   (oasdiff/schemathesis against the utoipa spec + replay of the E2E slice's `request.*` calls).
3. **Invariant tests green** — the surface's STACK-BOUND cluster (§7.2 table) re-covered in Rust,
   each ported test proven **red→green** (break the Rust impl, watch it fail) per the regression-
   ledger discipline; 🔴 surfaces additionally repeat the hand-mutant red-proofs (ADR-0013 style).
4. **Map-coverage gate zero-diff** for the surface's namespaces at its phase.
5. **a11y + budgets** — axe specs green; size-limit + lhci thresholds met on the new bundles
   (both gates must first be wired into CI — §7.3 gap 4).
6. **🔴 council + rollback** — council APPROVED recorded for red-line surfaces; proxy flip-back
   rehearsed once on staging (strangler seam bidirectional until final acceptance).

### §8d Final acceptance (whole-system, before prod cutover)

1. **Full 174-spec run** green against the complete Rust+Astro staging — with the skip-list
   frozen: `test.skip` count must be ≤ the pre-rebuild baseline snapshot (158); any new skip is
   a red flag, not a workaround.
2. **Visual re-baseline review** — all 162 comparisons re-baselined against the new FE; every
   diff human-approved (requires locking current-stack baselines FIRST — §7.3 gap 1 — so there
   is an honest "before").
3. **48h staging soak** — error-rate, RSS (expect ~10–30 MB), pool wait, WS reconnect churn,
   job DLQ depth all within current-stack envelope; no Sentry novel-error classes.
4. **Reliability-gate L0–L11** (`.claude/skills/reliability-gate/SKILL.md`) — one real order
   traced entry→creation→notify→CONFIRMED→PREPARING/READY→IN_DELIVERY→DELIVERED→post-delivery
   (WS/feedback/ratings) + L11 cross-surface matrix, on the new stack: **GO** verdict required.
5. **RLS suites** (verify:rls 25 tables + adversarial 8 IDOR tables) green — same DB, so these
   prove the Rust tenancy plumbing (SET LOCAL) reproduces withTenant exactly.
6. **Map-coverage gate globally zero-diff** at phase=ALL, and the 67 regression-ledger rows each
   re-verified on the new stack or explicitly re-proven (a ported guardrail per row).

---

## §9 Behavior classes NOT fully enumerated (honest residue)

1. **FE per-page error-code branching** — no central mapApiError exists (§3); the per-page
   `if (err.data?.code === …)` chains were sampled (checkout, onboarding), not exhaustively
   cataloged page-by-page. Gate lane: `errors` extractor should also grep FE `data?.code ===`
   sites so the port can't drop a branch.
2. **Ad-hoc error-shape sites** — 51 bypass sites counted; each site's exact body shape not
   individually cataloged (the Rust single-exit design makes this moot, but the E2E slices must
   cover the FE-visible ones).
3. **Unit-test case-level inventory** — 1,198 is the authoritative *run* count; static grep gives
   861/1,092 (nested `t.test()` subtests are not statically countable). Mapping done at
   invariant-cluster level (§7.2), which is the level the Rust backlog needs.
4. **WS payload schemas** — no Zod on WS messages; §6 payload shapes are inferred from handler
   usage. The Rust port should make them a typed enum — that act itself completes this census.
5. **Internal BUS_CHANNELS event census** — flagged as a distinct namespace (§6) consumed by
   notification workers; the full internal-event census belongs to the backend/jobs lane (10/12),
   not repeated here.
6. **Root scripts estate** — 70 scripts counted and categorized (19 verify / 6 guardrail /
   17 test), not row-by-row mapped; REBUILD-PLAN A18 (70→35 consolidation) should precede the
   matrix rows for them.
7. **Full 119-env consumer table** — the doc carries the 32 classified + dead/drift lists; the
   complete per-var consumer mapping lives in the extraction commands (re-derivable) and the
   Lane E working transcript.
8. **158 `test.skip` semantics** — counted, not individually triaged (§7.3 gap 3 makes triage a
   pre-Phase-A action item).

---

*Lane E census complete: 19 client flags + 22 server flags · 119 envs (32 classified, 13 dead,
24 drift) · 68+8 error codes (311 sendError sites, 51 bypasses) · 12 auth flows · 17 integrations
(35 hosts reconciled) · 12 vs 27 WS types (19 published-unhandled) · 174 E2E specs (151 parity /
23 rebase) · 181 unit files → 13 STACK-BOUND invariant clusters · methodology: matrix + 3-way
map-coverage gate + 6-check surface DoD + 6-check final acceptance.*
