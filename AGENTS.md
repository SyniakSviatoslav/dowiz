# DeliveryOS / dowiz — Agent Context

> Last updated: 2026-06-09 · Source of truth: `DeliveryOS-As-Built-Summary-v1.md` (2026-06-04)
> Reading this file is mandatory. Skip everything else until a router below tells you otherwise.

## 1. What this is (TL;DR — 30s read)

**dowiz / DeliveryOS** — Albanian-market SaaS delivery platform for restaurants. Three roles: Client (orders), Owner (manages), Courier (delivers). Mobile-first, 77% cash payments. ~50-restaurant pilot, solo developer, N=1 on Supabase Free + Fly.io single instance + Cloudflare DNS/CDN.

**This is NOT a static HTML mockup.** It is a full TypeScript monorepo (Phase 0–5 shipped, 67 migrations, 92 E2E tests all green). Static mockups under `src/screens/*.html` are kept only as design reference.

## 2. Repo map (where things live)

```
apps/
  api/        Fastify 5 backend (routes/, lib/, notifications/, workers/, plugins/)
  web/        React 18 + Vite 6 + Tailwind frontend (18 screens, all three roles)
  worker/     pg-boss jobs (Phase 5 anonymizer, backup, cron)
packages/
  config/     ESLint, TS, Tailwind shared config (+ verify-env.ts)
  core/       Domain primitives
  db/         67 migrations, RLS helpers, seed.ts
  domain/     Business logic / pure functions
  platform/   QueueProvider (pg-boss), MessageBus (pg NOTIFY/LISTEN)
  shared-types/  Zod schemas + TS types crossing api↔web
  ui/         Shared React component library (CSS variables only)
docs/
  audit/      phaseN-exit.md gates, vulnerabilities.md, inventory.md, flows/
  phase4/     anti-fake, OTP, signals-ui, dashboard, security
  phase5/     anonymizer, GDPR, retention-policy, backup, launch-checklist
  adr/        Architecture decision records
  harness/    Harness self-improvement: model-rotation.md, failure-mode-ledger.md, episodes/
e2e/          Playwright (92 tests × 3 breakpoints = 276)
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
3. Run `pnpm lint` + `pnpm typecheck` after changes

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

## 7. Phase 4 anti-fake rules (P26) — server-side

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

## 9. Known broken / must-fix before pilot (updated 2026-06-10)

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

**Remaining blockers: DB role guardrail only.** Security posture: HOLDS 13 · WEAK 1 · BROKEN 0.

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

### 13.2 Test quality rules — tests must fail when code is broken

| Anti-pattern (DO NOT) | Correct pattern (DO) |
|---|---|
| `expect([200,201,400,500]).toContain(status)` | `expect(status).toBe(200)` — test the exact expected status. If a flow can legitimately return 400 (e.g., validation), test that as a separate case. |
| Test only auth (`expect not 401`) and ignore the response body | Test auth AND the response body: verify it matches the expected schema. |
| Use `attributes?: any` type on frontend | Define proper types and validate at the boundary: `ProductAttributes` with `taste?`, `bom?`, `stock_count?` |
| Create a slug from display name instead of using the API slug | Always use the `slug` field returned by the API. Generate from name ONLY as fallback. |
| Form env var names from memory or convention | Copy-paste exact var names from `.env.example` or `verify:env.ts`. Verify against Fly secrets. |
| Assume a method exists on a provider/interface | Read the actual interface file before calling any method. |

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
