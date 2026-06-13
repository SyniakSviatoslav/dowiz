# DeliveryOS / dowiz — Agent Context

> Last updated: 2026-06-13 (menu import preview fix) · Source of truth: `DeliveryOS-As-Built-Summary-v1.md` (2026-06-04)
> Reading this file is mandatory. Skip everything else until a router below tells you otherwise.

## 1. What this is (TL;DR — 30s read)

**dowiz / DeliveryOS** — Albanian-market SaaS delivery platform for restaurants. Three roles: Client (orders), Owner (manages), Courier (delivers). Mobile-first, 77% cash payments. ~50-restaurant pilot, solo developer, N=1 on Supabase Free + Fly.io single instance + Cloudflare DNS/CDN.

**This is NOT a static HTML mockup.** It is a full TypeScript monorepo (Phase 0–5 shipped, 67 migrations, 92 E2E tests all green). Static mockups under `src/screens/*.html` are kept only as design reference.

## 2. Repo map (where things live)

```
apps/
  api/        Fastify 5 backend (routes/, lib/, notifications/, workers/, plugins/)
  api/scripts/  Verify scripts: verify-orphans, verify-event-wiring, verify-no-raw-status-update, config-drift, release-gate
  api/scripts/radar/  FE-radar + interconnected-radar + probe harness (harness/auth.ts, order.ts, observe.ts)
  web/        React 18 + Vite 6 + Tailwind frontend (18 screens, all three roles)
  worker/     pg-boss jobs (Phase 5 anonymizer, backup, cron)
packages/
  config/     ESLint, TS, Tailwind shared config (+ verify-env.ts)
  core/       Domain primitives
  db/         71 migrations (latest: 00010 audit-drop-reasons, 00011 reconciliation queue), RLS helpers, seed.ts
  domain/     Business logic / pure functions
  platform/   QueueProvider (pg-boss), MessageBus (pg NOTIFY/LISTEN)
  shared-types/  Zod schemas + TS types crossing api↔web (+ queue-names.ts)
  ui/         Shared React component library (CSS variables only)
docs/
  audit/      phaseN-exit.md gates, vulnerabilities.md, inventory.md, flows/, bug-investigation-2026-06-12.md
  seo/        ssr-architecture.md

```
apps/api/src/
  plugins/
    turnstile.ts              Turnstile CAPTCHA verification plugin
    auth.ts                   Auth verification + role enforcement
  lib/
    resilience/rate-limit.ts  Token-bucket rate limiter (extended: ORDER/PROMO/AUTH_OPTS + recordAbuse)
    ssr-renderer.ts           Preact SSR renderer (NOW ACTIVE — was dormant)
    jsonld-builder.ts         JSON-LD builder (Restaurant+Menu+Breadcrumb+FAQPage)
    motion.ts                 Framer Motion config + springs + variants
  routes/public/
    ssr.ts                    SSR route handler (/s/:slug) — calls renderMenuPage()
    seo.ts                    robots.txt + sitemap index + sharded children
    telemetry.ts              Analytics event ingestion (POST /api/telemetry + /api/telemetry/abuse)
    pwa.ts                    Per-location PWA manifest
```

**New packages**: `framer-motion` (ui + web), `@cf/turnstile` verification via fetch

**New tables (migration 1790000000012):** `analytics_events`, `analytics_abuse_log`, `analytics_cwv`
  audit/      FLOW-RADAR-MAP.md, FLOW-RADAR-REPORT.md, FE-RADAR-REPORT.md, FE-RADAR-REPORT-v2.md
  audit/      RELEASE-GATE.md, RELEASE-GATE-RUN.md, RECON-CATALOG.md, CONFIG-DRIFT-MAP.md
  phase4/     anti-fake, OTP, signals-ui, dashboard, security
  phase5/     anonymizer, GDPR, retention-policy, backup, launch-checklist
  adr/        Architecture decision records (+ ADR-NOTIFICATION-CONSOLIDATION.md)
  harness/    Harness self-improvement: model-rotation.md, failure-mode-ledger.md, episodes/
e2e/          Playwright (92 tests × 3 breakpoints = 276, plus fe-radar.spec.ts, fe-radar-v2.spec.ts)
migrations/   node-pg-migrate (delegates to packages/db/migrations) 
.agents/
  rules/      always-on rules (design-system.md, graphify.md, research-first.md, harness-self-improvement.md)
  workflows/  graphify.md, harness-self-improvement (via .agents/rules/harness-self-improvement.md)
  skills/     deliveryos-theme, component-builder, screen-builder, deliveryos-ui
graphify-out/ Knowledge graph (run `graphify query "..."` before raw grep)
src/screens/  Static HTML mockups (legacy design reference only)
```

**Canonical reference doc** (read first when you need broad architecture context):
`DeliveryOS-As-Built-Summary-v1.md` — phase status, stack, shims, security posture, must-fix list.

## Key 2026-06-13 Learnings (major refactors completed)

### SSR was dormant — now active
Preact SSR renderer (`ssr-renderer.ts`) was fully implemented but NEVER CALLED. The route handler at `ssr.ts` sent the static SPA shell instead. **Fixed 2026-06-13.** Now every `/s/:slug` returns full SSR HTML with menu content, `<head>` tags, JSON-LD, hreflang, OG tags.

### No animation library existed — framer-motion added from scratch
Prior to 2026-06-13, all animations were CSS-only. Added framer-motion to both `web` and `ui` packages. Created: Pressable, AnimatedNumber, AnimatedCheck, CrossfadeOnLoad, LiveDot (battery-aware), springs + variants library, MotionConfig reducedMotion="user" wrapper.

### No Cloudflare edge/WAF — app-level only
App runs on Fly.io single instance. No Cloudflare WAF/DDoS/Bot Management available. All protection is app-level: rate-limit middleware + Turnstile plugin + existing anti-fake/signals system.

### No TimescaleDB — regular PostgreSQL
Analytics tables (`analytics_events`, `analytics_abuse_log`, `analytics_cwv`) use plain PG with proper indexes, not hypertables. Migration 1790000000012. Telemetry endpoint persists events via sendBeacon pattern. Zero PII, zero third-party trackers.

## 3. Skill router (call the right skill, save tokens)

**Before writing any code or answering any question, check this router for a matching skill. If one exists, load it first.** The skill provides purpose-built instructions that produce better results than raw prompting.

| You are asked to… | Load skill | Notes |
|---|---|---|
| Change colors, brand tokens, presets, dark mode, CSS vars | `deliveryos-theme` | Always read `tokens.css` + `presets.json` first |
| Build a UI component (Card, Button, Badge, FAB, Nav) | `component-builder` | See `examples/` in skill dir |
| Build a full screen / page mockup | `screen-builder` | HTML5 self-contained, CDN-only |
| Codebase / architecture question, "where is X", "how does Y flow" | use **graphify** — see §5 | NEVER raw grep first |
| Debug, fix bug, investigate error, root-cause | `gstack-openclaw-investigate` or `investigate` | |
| Plan review / scope / poke holes | `gstack-openclaw-ceo-review` or `plan-ceo-review` | |
| Brainstorm new direction before code | `gstack-openclaw-office-hours` or `office-hours` | |
| Weekly retro / what shipped | `gstack-openclaw-retro` or `retro` | |
| Pre-landing PR review | `review` | |
| Ship workflow (bump, tag, PR) | `ship` | |
| QA web app + fix bugs | `qa` (writes fixes) or `qa-only` (report only) | |
| Generate / update docs | `document-generate` or `document-release` | |
| Write context-aware git commit messages | `contextual-commit` | Complements surgical-changes discipline |
| Deep research on external topics, competitors, tech | `firecrawl-deep-research` | Uses Firecrawl for web research |
| Create / test / improve a SKILL.md skill | `skill-creator` | Meta: authoring harness skills |
| Playwright E2E testing (black-box / Python) | `webapp-testing` | Complements convergence loop; lower-level Playwright helper |
| Configure Cloudflare Workers / R2 / edge | `wrangler` or `workers-best-practices` | Directly relevant — you run Cloudflare |
| Choose visual aesthetic direction before coding | `frontend-design` | Direction-only; tokens/audit gates still authoritative |
| Clean AI-writing tells from docs/READMEs | `stop-slop` | Docs only; code slop handled by Karpathy P2 + lints |
| Configure opencode itself | `customize-opencode` | Only for `.opencode/`, agents, MCP |
| Improve the harness (rules, skills, tools, gates, memory) | harness-self-improvement | Always-on rule at `.agents/rules/harness-self-improvement.md`. Failure-mode ledger at `docs/harness/failure-mode-ledger.md`. |

## 4. Memory protocol (mempalace — `wing: dowiz`, 3099 drawers already)

**Before answering any question about past work / decisions / facts:**
1. `mempalace_search` with focused keywords (use `wing: dowiz`).
2. `mempalace_kg_query` for entities (people, projects, places).
3. Only then synthesize — never guess.

**After each session of real work:**
- `mempalace_diary_write` with `agent_name: opencode`, `wing: dowiz` — AAAK-compressed entry.
- File durable facts as drawers via `mempalace_add_drawer` (wing: `dowiz`, room: matches `mempalace.yaml`).
- Use `mempalace_check_duplicate` before adding to avoid bloat.

**When facts change:**
- `mempalace_kg_invalidate` old, `mempalace_kg_add` new (with `valid_from` date).

## 4b. Research-first protocol (mandatory before ANY code change)

Before writing or modifying code, agent MUST complete the research checklist in `.agents/rules/research-first.md`. This prevents the bugs we keep finding: duplicate routes, double-prefixed paths, copy-pasted utilities, inconsistent auth patterns, and conflicting error formats.

**Minimum required:**
1. Grep/graphify for existing implementations of what you're about to change
2. Check conventions (auth hooks, error format, CSS vars, route prefixes)
3. Check i18n: search `packages/ui/src/lib/i18n.ts` for existing translations before adding new keys; add every new key to ALL 3 locales
4. Run `pnpm lint` + `pnpm typecheck` after changes

Full checklist: `.agents/rules/research-first.md` (always-on rule, loaded automatically).

## 5. Graphify protocol (codebase questions)

Knowledge graph lives at `graphify-out/`. **Note: current graph is stale** (built when project was at `Documents\delivery\`; project moved to `dowiz`). When you need it, refresh first:

```powershell
graphify update .          # AST-only, free
# or full rebuild:
graphify .                 # if structure shifted significantly
```

Then query (cheaper than reading docs or grepping):
```powershell
graphify query "where is order placement throttled?"
graphify path "OrderRoutes" "AnonymizerService"
graphify explain "Phase 5 anonymizer"
```

Use `graphify-out/GRAPH_REPORT.md` only for broad architecture overview.

## 6. Non-negotiable design rules (UI / frontend)

1. All colors via CSS variables — see `design.md`, `.agents/skills/deliveryos-theme/resources/tokens.css`. **Zero hardcoded hex.**
2. **No cookies, anywhere.** `localStorage` / `sessionStorage` only.
3. **No `position: fixed`** in embed mode (`?embed=true`).
4. Theme switcher on every screen — cycles 6 presets via `:root` vars only.
5. Dark mode mandatory on every screen.
6. Implement all 4 component states (default, hover, active, disabled).
7. 🔴 **3-language i18n mandatory.** Every user-visible string uses `t('key', 'English fallback')`. All 3 locales (`sq`, `en`, `uk`) must have the key in `packages/ui/src/lib/i18n.ts`. No hardcoded English strings, no alerts without `t()`. Check `design-system.md` rule for full details.

## 7. Notification hardening rules (H-series, landed 2026-06-12)

1. 🔴 **All channels/queues/event types through typed registry.** Zero raw string channel or queue names. Use `BUS_CHANNELS.*`, `QUEUE_NAMES.*`, `orderChannel()`, `dashboardChannel()`, `courierChannel()`, `shiftChannel()` helpers from `apps/api/src/lib/registry.ts`.
2. 🔴 **Every event must be fully wired.** `NotificationEventType` union at `apps/api/src/notifications/provider.ts`. Each event needs render case (`render.ts`), data builder case (`workers/index.ts:buildTelegramData`), locale strings (`locales.ts`) — enforced by `never` guards on default cases (compile error if missing).
3. 🔴 **Every notification drop writes audit.** No silent returns in dispatch or Telegram send handlers. Audit statuses: `no_target`, `unknown_event`, `quiet_hours`, `dedup`, `target_inactive`, `prefs_disabled`, `order_not_found`, `circuit_open`, `rate_limited`, `sending`, `delivered`, `failed`. Table: `notification_outbox_audit`.
4. 🔴 **One canonical path per task.** All order status transitions through `updateOrderStatus()` (guarded by `verify-no-raw-status-update`). No direct `UPDATE orders SET status` bypass.
5. 🔴 **Dwell-monitor only (pending-aging removed).** One pending notification mechanism. Runs every 60s, not 5min.
6. 🔴 **Dashboard WebSocket `enabled: true`** with correct room `location:{tenantId}:dashboard`. Reconnect handler calls `fetchOrders()` for reconcile.
7. 🔴 **Event registry (`EVENT_REGISTRY`)** has per-event quiet-hours policy, render group, and target scope. `isEventAllowedDuringQuietHours()` replaces hardcoded allowlist.
8. 🔴 **`delivered` audit entry** written only after successful Telegram API response (not queue processing proxy).

## 7b. Phase 4 anti-fake rules (P26) — server-side

1. 🔴 **No auto-ban.** Signals are advisory only. No signal blocks order placement.
2. 🔴 **Reputation decays exponentially** (30-day half-life). Counter alone = no signal.
3. 🔴 **OTP = owner toggle, off by default.** Rate-limited (3/15min sends, 5/15min verifies).
4. 🔴 **Human-in-loop.** Acknowledge/dismiss are manual only.
5. 🔴 **Velocity = privacy-first.** Only `*_hash` (sha256), never raw phone/IP. 24h retention, tenant-scoped.
6. 🔴 **OTP token** = opaque 32B base64url, 15min TTL, order-scoped, single-use. **Not JWT.**
7. 🔴 **0 PII** in `customer_signals.evidence`, `velocity_events`, MessageBus events.
8. 🔴 **Cross-tenant signal query → 404**, not 403.
9. 🔴 **Zod `.strict()` on all endpoints. RS256 JWT only. 0 cookies.**

## 8. Phase 5 anonymizer rules (P30) — server-side

1. 🔴 **Single mechanism, two triggers.** `AnonymizerService.anonymize(scope, subject)` only. `RetentionTrigger` (cron) + `GdprErasureTrigger` (request) both call it.
2. 🔴 **Anonymize, NOT delete.** PII → NULL or anon-token. Business fields (totals, snapshots) remain. FK references preserved. **No `DELETE FROM customers/orders`.**
3. 🔴 **Storage + R2 coverage.** Avatar cleanup on anonymization. R2 manifest PII-free. R2 lifecycle ≤ DB retention.
4. 🔴 **Idempotent.** `anonymized_at IS NOT NULL → skip`.
5. 🔴 **Audit append-only, PII-free.** `anonymization_audit_log` — RLS + FORCE.
6. 🔴 **GDPR dedup.** UNIQUE partial index. Rate-limit 1/customer/24h.
7. 🔴 **0 PII** in pg-boss payload, MessageBus events, logs, audit, error messages.
8. 🔴 **Cross-tenant → 404**, not 403. Owner-only RBAC.
9. 🔴 **Zod `.strict()` on all GDPR endpoints. RS256 JWT only. 0 cookies.**

## 9. Known broken / must-fix before pilot (updated 2026-06-12)

| # | Item | Status | Evidence |
|---|---|---|---|
| 1 | Courier routes (assignments + shifts) registered in `server.ts` | ✅ FIXED | `server.ts:527-528` — both registered with prefix `/api/courier` |
| 2 | No per-phone order throttle (FX-4) | ✅ FIXED | `orders.ts:200-224` — 15min window, max 5 orders, 429 + retryAfterSeconds |
| 3 | Operational pool connects as `postgres` superuser — RLS bypassed | ⚠️ CONFIG ISSUE | `packages/db/src/index.ts` — uses `***REDACTED***` env var; code does not hardcode role, but no guardrail prevents superuser connection |
| 4 | Theme/notification owner routes lack auth | ✅ FIXED | `owner/themes.ts:12-13`, `owner/notifications.ts:11-12` — both have `verifyAuth` + `requireRole(['owner'])` hooks |
| 5 | Idempotency dedup missing `location_id` scope (FX-5) | ✅ FIXED | `orders.ts:286-287,301,529-531` — all 3 ops scope by `location_id` |
| 6 | `MapLibreBase.innerHTML` → use `textContent` (XSS) | ✅ FIXED | `MapLibreBase.tsx:115` — uses `el.textContent = label` |
| 7 | No custom error handler (FX-6 leakage) | ✅ FIXED | `server.ts:459-478` — strips stack traces, returns safe messages + correlation ID |
| 8 | `statement_timeout` + acquire-timeout not set (FX-9) | ✅ FIXED | `packages/db/src/index.ts:28-29,48-49` — 10s operational, 30s session; `connectionTimeoutMillis: 5000` on both |
| 9 | `storage.upload()` called but `StorageProvider` only has `put()` | ✅ FIXED | `spa-proxy.ts:255`, `owner/themes.ts:136` — changed to `storage.put(key, processed)` |
| 10 | `BrandingPage.tsx` generates slug from name instead of using `res.slug` | ✅ FIXED | `BrandingPage.tsx:34-36` — uses `res.slug` first, falls back to name generation |
| 11 | `Fly.io secret GROQ_API_KE` (typo) → fixed to `GROQ_API_KEY` | ✅ FIXED | Manual: `flyctl secrets set GROQ_API_KEY=...` + `unset GROQ_API_KE` |
| 12 | MenuPage reads `attributes.kcal` but data is in `attributes.bom[].kcal` | ✅ FIXED | `MenuPage.tsx` — `bomToNutrition()` aggregates from `bom[]`; `attrEntries` filters `bom`/`stock_count` |
| 13 | `ai-ocr-parser.ts` default Groq model `llama3.1:8b-instruct` (Ollama fmt) | ✅ FIXED | Default now `llama-3.1-8b-instruct` (Groq format); reads `GROQ_MODEL` env var |

| 14 | Telegram callback query auth — NULL user_id in notification targets | ✅ FIXED | `telegram-webhook.ts:131-173` — order-first location resolution, skip membership on NULL user_id |
| 15 | CONFIRMED/REJECTED order events not delivering Telegram notifications | ✅ FIXED | `orderStatusService.ts:77-83` — publish lifecycle events; `server.ts:469-485` — subscribe + tgSend |
| 16 | pg-boss v10 array callback mismatch — `work()` passes `Job[]` not `Job` | ✅ FIXED | `queue-provider.ts:45-52` — wrapper iterates array; all workers use `queue.work()` |
| 17 | pg-boss runtime role had DDL privileges (migrate:true) | ✅ FIXED | `server.ts:248` — `migrate: false`; migration 0009 revokes CREATE on public; queues pre-created |
| 18 | MessageBus used operational (transaction) pool — LISTEN/NOTIFY broken | ✅ FIXED | `server.ts:235` — explicit session pool; `message-bus.ts:72-103` — reconnect with backoff |
| 19 | Missing notification dedup — duplicate events created duplicate jobs | ✅ FIXED | `server.ts:438-445` — dedupKey from event:entity_id:location_id as singletonKey |
| 20 | Queue `createQueue()` not called for 6 of 10 queues | ✅ FIXED | `server.ts:260-269` — explicit createQueue for all 10 |
| 21 | Webhook secret-token validation too strict — broke existing connect flows | ✅ FIXED | `telegram-webhook.ts:35-47` — warn-only if header missing; full validate only if header present |
| 22 | `answerCallbackQuery` called late — Telegram loading spinner shown | ✅ FIXED | `telegram-webhook.ts` — answer at top of handler, follow-up message sent separately |
| 23 | The/notification owner routes lacked `locale` in PUT schema | ✅ FIXED | `notifications.ts:99-107` — added `locale: z.enum(['sq','en','uk']).optional()` |
| 24 | `PgMessageBus.connect()` didn't release old client before reconnect | ✅ FIXED | `message-bus.ts` — release() before creating new connection, reset isDegraded |
| 25 | Menu import preview strips prices/images — maps products/categories to `.name` only | ✅ FIXED | `menu-import.ts:136-143` — pass through full `CanonicalCategory[]`/`CanonicalProduct[]` objects instead of `map(p => p.name)` |
| 24 | `PgMessageBus.connect()` didn't release old client before reconnect | ✅ FIXED | `message-bus.ts` — release() before creating new connection, reset isDegraded |

**Remaining blockers: DB role guardrail only.** Security posture: HOLDS 25 · WEAK 1 · BROKEN 0.

## 10. Common commands

```powershell
pnpm dev:all                                # api(3000) + worker + ui(3001)
pnpm verify:env; pnpm verify:db; pnpm verify:rls; pnpm verify:secrets
pnpm verify:all                             # composite: env→db→rls→secrets→migrations→lint→typecheck
pnpm verify:migrations                      # check migration ordering (numeric prefix ≈ alpha)
pnpm migrate:up                             # apply migrations
pnpm migrate:create "<name>"                # new migration in packages/db/migrations
pnpm test:phase4 ; pnpm test:phase5         # stage tests
pnpm verify:launch                          # full pre-launch gate
pnpm lint ; pnpm typecheck ; pnpm format
pnpm backup:verify ; pnpm backup:drill ; pnpm backup:list
npx playwright test                         # e2e — auto-starts API if VITE_BASE_URL not set (chromium × 3 breakpoints)
```

## 11. Reference documents (deep dive when needed)

- `DeliveryOS-As-Built-Summary-v1.md` — **start here** for any architecture question
- `CONVENTIONS.md` — code conventions (binary in repo; open in editor)
- `docs/audit/inventory.md`, `docs/audit/vulnerabilities.md` — security & risk register
- `docs/audit/phaseN-exit.md` — phase gates
- `docs/audit/flows/core-flows.md` — Mermaid flow diagrams for all 17 flows
- `docs/phase4/*.md` — anti-fake, OTP, signals UI, dashboard
- `docs/phase5/*.md` — anonymizer, GDPR, retention, backup, launch-checklist
- `docs/connection-budget.md`, `docs/contract-map.md`, `docs/ai-governance.md`
- `docs/DeliveryOS-Context-Handoff-v4_5.md` — latest product context
- `docs/DeliveryOS-Service-Build-Plan-v4_4.md` — build plan
- `.agents/skills/*/SKILL.md` — load via skill router (§3)
- `docs/harness/` — model-rotation registry, failure-mode ledger, episode store

## 12. Karpathy Working Rules (harness — cross-model)

Enforced by opencode plugins at `.opencode/plugin/`. Rules are model-agnostic — guard hooks run identically across Claude, DeepSeek, GPT, Gemini.

| Principle | Rule | Enforcement | Mechanism |
|---|---|---|---|
| P1 | Think Before coding | Plan-gate — blocks edit/write/patch unless `plan.jsonc` exists | `karpathy-guards.ts` — `tool.execute.before` |
| P2 | Keep it Simple | Simplicity lints + monthly churn report | eslint config + `.opencode/churn-report.cjs` |
| P3 | Make Surgical Changes | Scope-block — blocks edits outside `scope.jsonc` | `karpathy-guards.ts` — `tool.execute.before` |
| P4 | Be Goal-Driven (already enforced by existing harness gates) | Progress-anchored entropy sensor resets on gate pass | `entropy-sensor.ts` — `tool.execute.after` |

### P1 plan-gate protocol
```jsonc
// .opencode/plan.jsonc — minimal plan required before any mutation
{ "approach": "one-line or multi-step",
  "files": ["apps/web/src/...", "packages/domain/src/..."],
  "keyTypes": ["OrderStatus", "DeliveryZone"],
  "boundaryCondition": "empty cart edge case" }
```

### P3 scope-block protocol
```jsonc
// .opencode/scope.jsonc — restrict edits to listed dirs
["apps/web/src/", "packages/ui/src/"]
```

### Simplicity P2: what gets flagged
- `max-depth: 4` — deeply nested code
- `max-params: 4` — functions with too many args
- `no-lonely-if` — unnecessary nesting
- `@typescript-eslint/prefer-optional-chain` — `?.` over `&&`
- `max-nested-callbacks: 3` — callback pyramid
- `object-shorthand` — concise object literals
- Churn report: `node .opencode/churn-report.cjs` — flags files with >20% churn rate over 100 commits.

### Escaping the guard / silencing
- Trivial work (single edit, no side effects) can skip plan.jsonc.
- scope.jsonc is inert when absent — only written to bound a focused run.
- lints are `warn` only — never fail CI.

## 13. Verification-first rules (NON-NEGOTIABLE — born from 6 production bugs that passed tests)

These rules were added after discovering that 6 real production bugs passed all E2E tests because the tests were permissive, methods were not verified against interfaces, env vars had typos, and data shapes were not validated against contracts.

### 13.1 Never invent — always verify

| Rule | Failure it prevents | How to verify |
|---|---|---|
| **Verify method names exist on the interface before calling** | `storage.upload()` was called but `StorageProvider` only has `put()` | Read the interface/type definition file. `grep` or `read` the actual interface before using any method. |
| **Never use `@ts-nocheck` to suppress errors in API routes** | `@ts-nocheck` hid the `storage.upload()` type error | Remove `@ts-nocheck`. Fix the actual type errors. Use `@ts-expect-error` for individual lines if absolutely necessary. |
| **Verify env var names match between code and deployment** | `GROQ_API_KE` was set on Fly (typo) instead of `GROQ_API_KEY` | Cross-reference `process.env.XXX` references with `flyctl secrets list` and `.env.example`. Add new env vars to `verify:env`. |
| **Verify API response shapes against actual live data** | Frontend read `attributes.kcal` but API returned `attributes.bom[].kcal` | For any API integration, `curl` or `fetch` the live endpoint and inspect the actual response before writing frontend parsing code. |
| **Verify slugs/IDs match the database before hardcoding** | `demo-location` was used but DB has `demo` | Query the actual data: `SELECT slug FROM locations` or call the live API before assuming a slug. |
| **Verify API payload field names against Zod schemas** | Test sent `productId` but API expects `product_id` (snake_case) | Read the Zod schema in `packages/shared-types/src/*.ts` before constructing API payloads. API uses snake_case, not camelCase. |

### 13.2 Test quality rules — tests must fail when code is broken

| Anti-pattern (DO NOT) | Correct pattern (DO) |
|---|---|
| `expect([200,201,400,500]).toContain(status)` | `expect(status).toBe(200)` — test the exact expected status. If a flow can legitimately return 400 (e.g., validation), test that as a separate case. |
| Test only auth (`expect not 401`) and ignore the response body | Test auth AND the response body: verify it matches the expected schema. |
| Use `attributes?: any` type on frontend | Define proper types and validate at the boundary: `ProductAttributes` with `taste?`, `bom?`, `stock_count?` |
| Create a slug from display name instead of using the API slug | Always use the `slug` field returned by the API. Generate from name ONLY as fallback. |
| Form env var names from memory or convention | Copy-paste exact var names from `.env.example` or `verify:env.ts`. Verify against Fly secrets. |
| Assume a method exists on a provider/interface | Read the actual interface file before calling any method. |
| Use camelCase in API payloads when backend expects snake_case | Read Zod schemas in `packages/shared-types/src/`. API uses snake_case (`product_id`, `location_id`, `address_text`), not camelCase. |

### 13.3 Pre-deploy verification checklist (MANDATORY before any deploy)

Before `git push` or `fly deploy`, verify ALL of the following:

1. **`pnpm typecheck` passes** with zero `@ts-nocheck` suppressions in `apps/api/src/routes/`
2. **`pnpm lint` passes** with zero new warnings
3. **Method existence**: every `provider.method()` call has a corresponding method in the interface file
4. **Env var coverage**: every `process.env.XXX` in production code has a corresponding check in `verify:env`
5. **API contract test**: at least one E2E test validates the response shape of each public endpoint
6. **Slug resolution test**: the slug returned by `/owner/settings` resolves to a 200 on `/public/locations/:slug/menu`
7. **Image upload test**: upload with auth returns 200 and the image URL is fetchable
8. **No permissive status assertions**: no test accepts `[200, 400, 500]` as passing

### 13.4 NX-specific verification rules (born from 20-issue notification audit)

These rules were added after the 2026-06-12 NX audit which found 20 issues in the Telegram notification subsystem caused by library API drift, topology confusion, incomplete event wiring, and permission gaps.

| Rule | Failure it prevents | How to verify |
|---|---|---|
| **Schema-first query verification** — before writing SQL JOINs, verify columns exist | `short_id`/`oi.created_at`/`currency` column mismatches | `SELECT column_name, data_type FROM information_schema.columns WHERE table_name = ?` or read the migration file for the table |
| **Library API pinning** — verify installed version's API before using it | pg-boss v10 `work(Job[])` vs v9 `work(Job)` | Read `node_modules/<pkg>/package.json` for installed version, then read type declarations or changelog for that version |
| **Event wiring completeness** — when adding new event type, verify all chain links | Missing subscriber/handler/locale/render/type for `order.confirmed`/`order.rejected` | Grep chain: publish → subscribe → handler → locale strings → render case → type union. Every link must exist before deployment. |
| **Connection lifecycle audit** — every connect() must have matching close() | `PgMessageBus.connect()` leaked old client on reconnect | Grep for `.connect()` calls; verify each has a corresponding `.close()`/`.release()` in error and normal paths |
| **Resilience-by-default for IPC** — every pg-boss send must have singletonKey | Duplicate notification jobs sending multiple Telegram messages | Verify `singletonKey` or `dedupKey` is set on every `queue.send()` call |
| **Backward compat first for webhooks** — start lenient, add strict after telemetry | Webhook secret-token validation broke existing connect flows | Log warning on first config mismatch; enforce strict validation only after verifying all producers updated (24h minimum) |
| **Topology verification** — know the port map before connecting | MessageBus used transaction pooler (LISTEN/NOTIFY broken) | Document which pool (session/transaction) each connection uses. Verify `inet_server_port()` returns correct port for each env var. |
| **Runtime privilege verification** — verify grants exist before depending on them | Default privileges didn't cover existing tables; missing DDL grants | Test `has_schema_privilege(user, schema, privilege)` and `has_table_privilege(user, table, privilege)` at startup |
| **Infrastructure pre-flight** — verify external deps exist at startup | Queues not created, webhook not set, secrets missing | For each external dependency, add a startup check that fails fast |
| **Don't repeat API calls** — on third inline fetch, extract a helper | 3+ inline fetch calls to Telegram API with inconsistent error handling | When same API host gets third inline `fetch(url, {...})`, extract shared helper with consistent error handling |

### 13.5 NX test gap checklist (verify after event/notification changes)

After ANY change to:
- Notification event types or delivery
- pg-boss queue/work/worker configuration
- MessageBus publish/subscribe wiring
- Telegram/Bot API interaction

Run the following (in order of confidence):

1. **`pnpm typecheck`** — zero errors
2. **`pnpm test:stage36`** — all NX stage tests pass (T-1 durability, T-2 off-critical-path, T-3 topology/privileges, T-4 idempotency)
3. **`npx tsx apps/api/scripts/verify-nx-flow.ts`** — full chain E2E: enqueue → process → audit trail
4. **Verify all 10 queues exist** — `SELECT name FROM pgboss.queue` should return all expected queue names
5. **Verify message delivery** — trigger an event, check `pgboss.job` for `completed` state, check `notification_outbox_audit` for the audit record
