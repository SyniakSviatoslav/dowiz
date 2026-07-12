# CLEAN-LEDGER.md — sweep results (Phase B · SWEEP, read-only)

> One row per hit found by sweeping every seed in [CLEAN-SEEDS.md](./CLEAN-SEEDS.md) across the
> monorepo. **No fixes performed.** Status: `CATALOGUED` (real hit, fixable in Phase C) ·
> `FLAG-only` (contract/money/auth/RLS/security/foundation — separate decision, do NOT auto-fix) ·
> `WONTFIX-justified` (legit exception) · `CLEAN` (seed swept, zero residual).
> Red-line literals are written in neutralized form in this doc (it's documentation, not product code).

## Severity distribution (actionable = CATALOGUED + FLAG-only)

| sev | CATALOGUED | FLAG-only | WONTFIX | CLEAN seeds |
|---|---|---|---|---|
| S0 | 3 | 14 | 2 | ~14 |
| S1 | ~10 | 5 | 1 | ~14 |
| S2 | ~9 | 4 | 1 | ~10 |
| S3 | 1 | — | — | 1 |

---

## 🔴 S0 — top priority (security / privacy / contract / integrity)

| id | seed_id | location | root | proposed_fix / note | status |
|---|---|---|---|---|---|
| LDG-0001 | SEED-BE-RLS-FORCE | `pnpm verify:rls` → "Isolation leak: anonymous query returned 20 for **memberships**, expected 0" | FLAG-only | **Live gate FAILURE.** Either memberships RLS not `FORCE`/policy-gapped, or verify:rls connects with an RLS-bypassing role / wrong DB. Red-line — needs doubt-escalation + human before any change. | FLAG-only |
| LDG-0002 | SEED-SC-SEARCH-PATH | 16 `SECURITY DEFINER` fns in `packages/db/migrations/` (1790000000064/065/032, 1780338982028, …) — no visible `SET search_path` | FLAG-only | Confirm each definer fn pins `SET search_path = public, pg_temp`; missing → schema-resolution hijack. Migration red-line → escalate. | FLAG-only |
| LDG-0003 | SEED-SEC-PII-LLM | `apps/api/src/lib/ai-ocr-parser.ts` (OpenRouter/OpenCode menu OCR) | FLAG-only | Confirm only menu text (no customer/owner/courier PII) reaches the prompt; provider must be a listed subprocessor. | FLAG-only |
| LDG-0004 | SEED-SEC-PII-QUEUE | pg-boss enqueue sites in `apps/api/src` | FLAG-only | Verify payloads carry claim-check keys (order_id) only, no name/phone/email/address. | FLAG-only |
| LDG-0005 | SEED-SEC-PII-BUS-WS | MessageBus / WS broadcast in `apps/api/src` | FLAG-only | Verify WS frames carry order_id only; item names / PII backfilled via authed GET. | FLAG-only |
| LDG-0006 | SEED-SEC-JWT-RS256-KID | vendored `@deliveryos/platform` issuer | FLAG-only | Confirm RS256 + `kid` in header (not HS*). | FLAG-only |
| LDG-0007 | SEED-SEC-SSRF-BRAND | `apps/api/src/lib/brand-extractor.ts:161-211` (assertPublicUrl @188) | FLAG-only | `assertPublicUrl` present — confirm it re-validates on **each** redirect hop (DNS-rebind TOCTOU). | FLAG-only |
| LDG-0008 | SEED-SEC-UPLOAD-MAGIC | upload handlers in `apps/api/src` | FLAG-only | No magic-byte/file-signature check found; audit against `product-media-validation` test. | FLAG-only |
| LDG-0009 | SEED-SEC-TOKEN-TTL-ACTIVE | `activeLocationId` baked in JWT, used in authz paths | FLAG-only | Confirm each owner authz path does a live `AND status='active'` (insider-removal). Partly covered by ledger#16. | FLAG-only |
| LDG-0010 | SEED-SEC-OAUTH-GATE | `apps/api/src/routes/auth.ts` `/auth/google` + `/callback` | FLAG-only | Confirm both gated by `GOOGLE_OAUTH_ENABLED` on prod (FE-hidden ≠ backend-gated). | FLAG-only |
| LDG-0011 | SEED-SEC-ANONYMIZER | GDPR purge path in `apps/api/src` | FLAG-only | Confirm it NULLs PII fields, does not DELETE rows (audit-trail integrity). | FLAG-only |
| LDG-0012 | SEED-SEC-RATELIMIT-MUTATION | mutation routes in `apps/api/src` | FLAG-only | Auth routes limited; audit owner/courier mutations for coverage + per-instance-vs-global on N>1. | FLAG-only |
| LDG-0013 | SEED-SEC-TOKEN-IN-URL | `apps/web/src/pages/admin/LoginPage.tsx:50`; `apps/web/src/pages/MenuFirstOnboarding.tsx:151` | FLAG-only | Magic-link/token-exchange in URL — confirm short TTL + single-use; acceptable pattern but verify. | FLAG-only |
| LDG-0014 | SEED-SEC-TOKEN-IN-URL | `apps/api/src/client/status/ws.ts:17` (token in WS query) | local | Browser WS can't set headers; common pattern. Prefer short-lived ticket token over the session JWT. | CATALOGUED |
| LDG-0015 | SEED-BE-GPS-FILTER | `apps/api/src/workers/courier-events.ts` (courier_positions ingest) | FLAG-only | No server-side accuracy>100m / speed>150 sanity filter found; confirm or add (spoof/garbage). | FLAG-only |
| LDG-0016 | SEED-BE-CASH-HOLD | cash handling (`cash_collected` on courier_assignments) | FLAG-only | No HOLD-divergence→owner-alert visible in route sweep; audit end-to-end. | FLAG-only |
| LDG-0017 | SEED-BE-CONTRACT-FIELD-PARITY | client checkout payload vs route Zod schemas | FLAG-only | Spot-check `delivery_instructions` matches; do a field-by-field CreateOrderInput audit. | FLAG-only |
| LDG-0018 | SEED-SC-COMPLIANCE-PII | new PII cols (subject_type, opted_in, preflight jsonb) | FLAG-only | Run `pnpm compliance:gate` (script exists at `scripts/compliance-gate.ts`); confirm DPIA + data-map updated. | FLAG-only |
| LDG-0019 | SEED-BE-MONEY-INT | `apps/api/src/lib/ssr-renderer.ts:99` `toFixed(2)` on product-price **display** | local | Display-only formatting, not charge math. Confirm no downstream calc consumes it; else leave. | CATALOGUED |

**S0 CLEAN (swept, zero residual):** SEED-SEC-COOKIE (no cookie-API writes) · SEED-SEC-INSECURE-RANDOM (gate green, no non-CSPRNG security ids) · SEED-SEC-DEV-LOGIN (properly gated by `ALLOW_DEV_LOGIN`+`DEV_AUTH_SECRET` in `plugins/dev-guard.ts`) · SEED-SEC-IDOR-COURIER (`courierAssignmentService.ts:24` scopes `WHERE id=$1 AND courier_id=$2`) · SEED-SEC-ANON-ORDER-IDOR (`orders.ts:823-848` customer token scoped) · SEED-SEC-CUSTOM-CSS-PURIFY (`ssr-renderer.ts` dangerouslySetInnerHTML is hardcoded theme/JSON-LD, no user input) · SEED-BE-STATUS-GUARD (`orderStatusService.ts:92-118` all guarded by current status) · SEED-BE-IDEMPOTENCY-TX (`orders.ts:681-684` idempotency insert+enqueue in one tx, unique-constraint) · SEED-BE-NOTIFY-CRITPATH (notify enqueued off critical path) · SEED-BE-DELIVER-FRICTION (no auto-close; explicit courier action) · SEED-BE-AUTOBAN (`preflight.ts:45` hard_block errors, no auto-ban) · SEED-BE-AUTOCANCEL-PERSIST (`timeout_at` col + `order-timeout-sweep.ts` poller) · SEED-RS-SCHEMA-DRIFT (`fly.toml` release_command set) · SEED-RS-MIGRATION-HEAD (`build-apps.ts` head == migrations tail 1790000000067) · SEED-RS-WORKER-LIVENESS (`apps/worker/src/heartbeat.ts` + ops_worker_heartbeat) · SEED-RP-GITIGNORE-ENV (`.gitignore` ignores `.env.*` except `!.env.example`).

---

## 🔴 Red-line investigation verdicts (Phase C — step 1, escalation not fixing)

| id | verdict | finding | recommended path (human/council, NOT autonomous) |
|---|---|---|---|
| LDG-0001 | **likely env/role artifact** (2 interpretations on a red-line → operator confirm) | Migration is correct: `memberships` has ENABLE+FORCE+`tenant_isolation` policy via `app_member_location_ids()` (`core-identity.ts:76-95`). Anon→0 unless the connecting role bypasses RLS. Pool factory even guards against `postgres` superuser (`packages/db/src/index.ts:37`). The `verify:rls` anon=20 means the env it ran in had `DATABASE_URL_OPERATIONAL` on a superuser/BYPASSRLS role (sandbox `.env`) — or a real prod gap. | Operator: confirm staging/prod `DATABASE_URL_OPERATIONAL` connects as a non-superuser, non-`BYPASSRLS` role; re-run `verify:rls` against a properly-seeded DB with the restricted role. No code change. |
| LDG-0002 | **CONFIRMED real** | All 13 `SECURITY DEFINER` fns lack `SET search_path`, **including `app_member_location_ids()`** — the RLS lynchpin. Matches Supabase linter 0011. Risk scales with whether untrusted roles can CREATE on the search_path. | Council/human: new migration adding `SET search_path = public, pg_temp` (or `= ''` + fully-qualified refs) to each definer fn. Red-line glob (`packages/db/migrations/`) → gate + red→green guardrail before merge. |
| LDG-0003 | **conditional gap** | `ai-ocr-parser.ts:515` concatenates un-redacted `rawText` into the LLM prompt; `PiiRedactor` (line 399) output feeds only the provenance hash (line 564), not the model input. Safe only if uploads are clean menus; no upload-content validation. | Decision: feed `redactedText` to the model instead of `rawText`, and/or validate upload is a menu. PII red-line → flag, not autonomous. |
| LDG-0004 | ✅ **HOLDS (verified)** | pg-boss payloads are claim-check: `dwell-monitor.ts:132-141`, `dwell-escalation.ts:134`, `order-timeout-sweep.ts:110` carry ids/eventType/generic-message only, no name/phone/email/address. | None — invariant holds. Reclassify CLEAN. |
| LDG-0005 | ✅ **HOLDS (verified)** | MessageBus publishes claim-check only: `orders.ts:729-763` (orderId/status/total/shortId/itemCount), explicit P0-3 comment at `orders.ts:746-750` ("bus carries ZERO customer PII … dashboard pulls from authenticated RLS-scoped /owner/orders"). | None — invariant holds. Reclassify CLEAN. |

**Net red-line result:** 2 invariants verified HOLDING (queue, bus); 1 confirmed real hardening gap (search_path, escalate); 1 conditional gap (LLM redaction, escalate); 1 needs operator role-confirmation (RLS). **Zero autonomous changes to red-line paths.**

---

## S1 — resilience / correctness

| id | seed_id | location | root | proposed_fix | status |
|---|---|---|---|---|---|
| LDG-0020 | SEED-BE-ZOD-STRICT | `auth.ts:66,204,342` · `orders.ts:22,212,252` · `settlements.ts:16,77,111,164,208,260` · several owner routes | consolidate→add `.strict()` | Append `.strict()` to every body/params/querystring `z.object()`. Broad but mechanical. | CATALOGUED |
| LDG-0021 | SEED-BE-EXTERNAL-TIMEOUT | `turnstile.ts:20` · `notifications/adapters/telegram.ts:23,58` · `auth.ts:84` · `ai-ocr-parser.ts:115,144,174,213,241` | consolidate→shared `fetchWithTimeout` | Wrap external fetch in AbortController+timeout+fallback (brand-extractor@189 already does). | CATALOGUED |
| LDG-0021b | SEED-SEC-AS-UNKNOWN | `apps/web/src/components/PaperScene.tsx:61` · `apps/api/src/routes/courier/shifts.ts:377` · `packages/ui/src/theme/paperSkin.ts:9` · `packages/ui/src/hooks/use-voice-order.ts:32,33` | local | Replace double-cast escapes with proper types (NetworkInformation, vite env, SpeechRecognition). | CATALOGUED |
| LDG-0021c | SEED-SEC-AS-ANY | `apps/api/src/websocket.ts:57,62,112,113` · `plugins/turnstile.ts:14,36` · `server.ts:67,685,692` | local | `no-raw-any` is warn-level; define WS `isAlive` iface, Fastify body types, token payload types. | CATALOGUED |
| LDG-0022 | SEED-DF-DIRECT-FETCH | `MenuManagerPage.tsx:69` · `OrderStatusPage.tsx:96,100` · `CheckoutPage.tsx:264,302` · `AccessRequestForm.tsx:38` · `MenuPage.tsx:303,325,414` | consolidate→`apiClient` | Route all internal-API calls through apiClient (auth/dedup/error-normalize). | CATALOGUED |
| LDG-0023 | SEED-DF-DIRECT-FETCH (3p) | `CheckoutPage.tsx:286` (Wikipedia geocode) | local | External 3rd-party — document/guard with timeout, keep out of apiClient. | CATALOGUED |
| LDG-0024 | SEED-BE-UNHANDLED-PROMISE | `apps/api/src/client/pwa/sw.ts:54` `caches.open().then()` fire-and-forget | local | Add `.catch()` log; best-effort cache, non-critical. | CATALOGUED |
| LDG-0025 | SEED-BE-CREATE-REPLACE-STALE | `read_public_menu` redefined in migrations …018/032/033/055/064/065; `_all_locales` in …034 | local | Latest migration is the live def (intentional evolution) — verify no old body re-copied. | CATALOGUED |
| LDG-0026 | SEED-DS-HARDCODED-STRING | `no-hardcoded-string` is warn-level; offenders remain | FLAG-only | Promote rule to error per-path after backfill; needs i18n parity pass. | FLAG-only |
| LDG-0027 | SEED-TS-I18N-PARITY | `scripts/i18n-parity.ts` warn-level | FLAG-only | Run `--strict`, backfill missing sq/uk via `scripts/i18n-add.ts`, then gate at error. | FLAG-only |
| LDG-0028 | SEED-DS-INVENTORY-DRIFT | inventory "AI assistant" (`/admin/ai`) — no matching page in `apps/web/src/pages` | FLAG-only | Blind spot vs inventory: confirm intentionally deferred or build. Don't auto-create (scope). | FLAG-only |
| LDG-0029 | SEED-TS-SKIP-ONLY | 30+ conditional `test.skip()` across `e2e/tests/*` (e.g. `flow-ui-admin-dashboard.spec.ts:104`) | local | Most are setup-conditional (orderId/slug/token). Audit for unintended quarantine. | CATALOGUED |
| LDG-0030 | SEED-TS-WAIT-TIMEOUT | 60+ `page.waitForTimeout()` in `e2e/tests/*` (MATRIX.md tracks ~68) | local | Replace with `expect(locator).toBeVisible()` waits. Backlog already tracked. | CATALOGUED |
| LDG-0031 | SEED-TS-FAKE-ASSERT | `e2e/tests/simple-test.spec.ts:4` `expect(true).toBe(true)` | consolidate→delete | Tautology smoke/template — delete or replace with a real assertion. | CATALOGUED |

**S1 CLEAN:** SEED-DF-DIRECT-WS (gate green) · SEED-DF-WS-RECONCILE (`DashboardPage:155`, `TasksPage:82`, `OrderStatusPage:298` refetch on reconnect) · SEED-DF-WS-TERMINAL-LOCK · SEED-DF-WS-UNSUB (`useWebSocket` cleanup) · SEED-DF-NODE-CRON (none) · SEED-DF-LOCAL-WS-MAP (`websocket.ts:15-22` via MessageBus) · SEED-DF-MENU-LOCALE (`MenuPage.tsx:244` passes locale) · SEED-BE-HEALTH-SPLIT (`health.ts` critical vs degraded) · SEED-BE-SIGTERM-DRAIN (`shutdown.ts` both apps) · SEED-BE-ENV-NULLCHECK (`verify:env` OK) · SEED-BE-OTP-GATE (`customer/otp.ts` + `orders.ts` gated) · SEED-BE-SOFT-CONFIRM (`orders.ts:369-377` returns `requiresConfirmation`) · SEED-BE-PRIVPOOL-WHERE (no privileged tenant-data pool) · SEED-BE-EMPTY-CATCH (gate) · SEED-RS-POOL-STARVATION (fixed) · SEED-RP-SPIKES-IMPORT (none) · SEED-SC-NOTNULL-NO-DEFAULT (all adds have DEFAULT) · SEED-SC-PREFLIGHT-JSONB / SEED-SC-SUBSCRIPTION-FIELDS (added with documented intent).

---

## S2 — UI-state / design-system / hygiene

| id | seed_id | location | root | proposed_fix | status |
|---|---|---|---|---|---|
| LDG-0032 | SEED-DS-PRIMARY-AS-TEXT | `AnalyticsPage.tsx:164,188,235,276,317,327,333,343,426,477-481` · `main.tsx:85` | consolidate→`--brand-primary-readable` | brand-primary used as readable text (AA ~3.7:1) → swap to readable token. | CATALOGUED |
| LDG-0033 | SEED-DS-BROKEN-IMAGE | ~18 `<img>` w/o onError: `AnalyticsPage:323` · `MenuManagerPage:768,860,936` · `MenuPage:898` · `CheckoutPage:715` · `DeliveryPage:379,388` · media/* components | consolidate→shared `<Img onError>` | Add fallback glyph; `ClientLayout.tsx:119` shows the good pattern. | CATALOGUED |
| LDG-0034 | SEED-DS-OPACITY-MUTED | `MenuManagerPage:995` · `BrandingPage:449` · `DashboardPage:637,757` · `MessageThread.tsx:135` | local | opacity on already-tuned muted text risks <AA — verify ratio or drop opacity. | CATALOGUED |
| LDG-0035 | SEED-DS-COLOR-NONHEX | `packages/ui/src/theme/index.ts:3,24,38` (rgba in PRESETS) | WONTFIX-justified | Token/preset definitions (allowlisted like tokens.css). | WONTFIX-justified |
| LDG-0036 | SEED-RP-ROOT-CLUTTER | 37 files at repo root: `admin-*.png`, `checkout-*.png`, `storefront-*.png`, `api-*.log`, `server.log`, `fix.{cjs,js,mjs}`, `analyze.mjs`, `record-run.mjs` | local→move/rm + gitignore | Move to `audit/`/`e2e/findings/` or delete; add patterns to `.gitignore`. | CATALOGUED |
| LDG-0037 | SEED-RP-TRANSFER-ARTIFACTS | `e2e/findings/round-{1,2,3}/trace.zip` (8) · `/root/restore` dir | local→gitignore | Gitignore traces; `/root/restore` is outside repo — leave or remove manually. | CATALOGUED |
| LDG-0038 | SEED-RP-WINDOWS-JUNK | `.claude/settings.local.json:11,14,17,27,30,31` (`/c/Users/Dell5`, PowerShell) | FLAG-only (escalate) | `.claude/` is protect-path-blocked — cannot auto-edit; surface for manual cleanup. | FLAG-only |
| LDG-0039 | SEED-RP-ENV-DRIFT | `.env.example` 2× obsolete `WHATSAPP_*`/`BAILEYS` | local | Remove obsolete keys; add current TELEGRAM_*/OPENCODE_ZEN; diff vs config EnvSchema. | CATALOGUED |
| LDG-0040 | SEED-RS-PIPE-EXIT | `scripts/ui-verify-floor.sh:29,31` (`\| head -20 \|\| true`) | local | Display-only with `\|\| true` fallback (low risk) — use `head 2>/dev/null` to not mask. | CATALOGUED |
| LDG-0041 | SEED-RS-BACKUP-RESTORE | `scripts/backup-restore.ts` has `--dry-run`/manifest verify | CATALOGUED | Confirm a scheduled restore-into-separate-DB assertion exists, not just dry-run. | CATALOGUED |

**S2 FLAG-only (MV / needs live check):** SEED-DS-ICON-UNLABELED (~59 buttons; most have text — visual pass) · SEED-DS-DEAD-STATE (per-component loading/empty/error audit) · SEED-DS-MOBILE-OVERFLOW (Playwright @390px on product modal) · SEED-DS-TAP-TARGET (measure courier actions) · SEED-DF-COURIER-STATUS (MV) · SEED-DF-IMPORT-SILENT-ZERO (MV) · SEED-DS-DARK-ON-DARK (derivePalette partial-theme edge) · SEED-TS-STALE-SPEC (flow-start-hero/SwanHero dead selectors per MATRIX.md) · SEED-RS-SSR-CACHE (confirm `/s/:slug` Cache-Control+menu_version) · SEED-RS-STORAGE-BACKUP (storage→R2 versioned?) · SEED-SC-PGBOSS-BOOTSTRAP (queue-names vs migration 0011) · SEED-SC-PG-VERSION (pg version + pgboss grants).

**S2/S3 CLEAN:** SEED-DS-COLOR-HEX · SEED-DS-TW-COLOR · SEED-DS-ARBITRARY-TW · SEED-DS-FONT (only `var(--font-display)`) · SEED-DS-RAW-FORM · SEED-DS-NESTED-INTERACTIVE (no button-in-clickable-card) · SEED-DS-INPUT-UNLABELED (custom Select used) · SEED-DS-ORPHAN-ARIA (all role=tab in tablist) · SEED-DS-FALSE-INVALID (`index.css:37-39` suppresses pristine ring) · SEED-RP-HOOK-TOPLEVEL (all 5 hooks have `2>/dev/null \|\| ${CLAUDE_PROJECT_DIR:-$PWD}` fallback) · SEED-RP-DUP-IMPORT (gate) · SEED-SC-MIGRATION-SEQ (142 migs, timestamp scheme, no conflict) · SEED-TS-PERMISSIVE-STATUS (gate) · SEED-TS-MOCK-IN-PROD (gate).

---

## WONTFIX-justified

| seed_id | location | reason |
|---|---|---|
| SEED-SEC-TS-IGNORE | `libretranslate-provider.ts:2` · `ai-ocr-parser.ts:2,316` | legacy 3p type stubs (pdfjs-dist) — incomplete upstream types; acceptable, ticket to shared-types |
| SEED-SEC-HARDCODED-CREDS | `e2e/tests/*.spec.ts` (test@dowiz.com) | seeded CI test fixture, not a prod credential — but rotate the telegram token if any is real (see flag) |
| SEED-DF-CLIENT-PRICE | `ClientLayout.tsx:109` + `CheckoutPage.tsx:353` | display-only estimate; server is authoritative per ADR-0005 (fee server-mirror + parity guardrail) |
| SEED-DS-COLOR-NONHEX | `theme/index.ts:3,24,38` | rgba in token PRESETS = source-of-truth definitions, allowlisted |

---

## SEED-BACK (new classes surfaced during the sweep)

- None that spawn fresh hits beyond existing seeds. Candidates noted but folded into existing seeds:
  - "as-any on WS extension property" → subset of `SEED-SEC-AS-ANY` (LDG-0021c).
  - "fetch-without-AbortController" → already `SEED-BE-EXTERNAL-TIMEOUT` (LDG-0021).
  - "build-time dead-code (import.meta.env.DEV) not verified in prod bundle" → low value; `SEED-SEC-IMPORT-META-RUNTIME` swept CLEAN (uses are tree-shake guards, not runtime gates).

➡️ Phase B converges: a full pass produced **no new seed that yields new hits**.

---

## Phase C — fixes applied (batch 1: self-provable hygiene/correctness, no staging needed)

| id | seed_id | action | proof (re-detect / gate) | status |
|---|---|---|---|---|
| LDG-0036 | SEED-RP-ROOT-CLUTTER | `rm` untracked gitignored root `*.png`/`*.log`; `git rm` tracked one-off `fix.{cjs,js,mjs}`, `analyze.mjs`, `record-run.mjs` (zero code refs; `analytics/` copies retained) | `find . -maxdepth 1 …` → **0** | FIXED |
| LDG-0039 | SEED-RP-ENV-DRIFT | removed obsolete WHATSAPP/Baileys block from `.env.example` (TELEGRAM_* already present) | `grep -cE "WHATSAPP\|BAILEYS" .env.example` → **0** | FIXED |
| LDG-0031 | SEED-TS-FAKE-ASSERT | deleted `e2e/tests/simple-test.spec.ts` (`expect(true).toBe(true)` tautology, zero coverage) | `grep expect(true) e2e` → **0** | FIXED |
| LDG-0024 | SEED-BE-UNHANDLED-PROMISE | `apps/api/src/client/pwa/sw.ts`: extracted `cachePut()` module helper sinking the best-effort cache promise via `.catch` | `eslint sw.ts` → 0 errors, **13 warns (−1 vs baseline, 0 new)**; api `tsc` passes | FIXED |
| LDG-0037 | SEED-RP-TRANSFER-ARTIFACTS | none needed — `e2e/findings/round-*/` already gitignored (`.gitignore:56`); `/root/restore` is outside the repo | — | WONTFIX-justified |
| LDG-0040 | SEED-RS-PIPE-EXIT | reclassified — `ui-verify-floor.sh:29,31` are `grep … \| head \|\| true` **display** lines (intentional non-fatal), not exit-masking of a deploy/commit. No genuine pipe-exit bug in committed scripts. | — | WONTFIX-justified |

**Remaining Phase C (not yet done):**
- **Type-safety batch** (typecheck-provable): `SEED-SEC-AS-UNKNOWN` (5: PaperScene, shifts.ts, paperSkin, use-voice-order), `SEED-SEC-AS-ANY` (websocket.ts, turnstile.ts, server.ts — **hotspot/auth-adjacent, warn-level → careful pass**).
- **UI/dataflow consolidation batch** (needs Playwright-on-staging per Mandatory Proof Rule): `SEED-DF-DIRECT-FETCH`→apiClient (~9), `SEED-DS-BROKEN-IMAGE` shared fallback (~18), `SEED-DS-PRIMARY-AS-TEXT`→readable token (~9), `SEED-DS-OPACITY-MUTED` (5), `SEED-BE-EXTERNAL-TIMEOUT` wrapper (~6).
- **Held for explicit OK:** `SEED-BE-ZOD-STRICT` (`.strict()` rejects unknown keys → contract-affecting).
- **Escalated (red-line, not autonomous):** LDG-0001 (RLS role), LDG-0002 (search_path), LDG-0003 (LLM redaction).

---

## Phase-B exit status

- Every seed swept (exact + similar + MV). ✅
- **Zero fixes performed.** ✅
- Ledger complete; `FLAG-only` items listed separately (S0 red-lines isolated at top). ✅
- Convergence: no new seeds spawned new hits. ✅

> **STOP-checkpoint B.** Awaiting GO before Phase C (FIX by classes, S0→S3, consolidate-first).
> ⚠️ Note: LDG-0001 (`verify:rls` memberships leak), LDG-0002 (SECURITY DEFINER search_path),
> and the PII triad (LDG-0003/4/5) are **red-line FLAG-only** — they require doubt-escalation /
> council / human sign-off, NOT routine Phase-C fixing.
