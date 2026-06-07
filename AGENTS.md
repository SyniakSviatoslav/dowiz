# DeliveryOS / dowiz — Agent Context

> Last updated: 2026-06-07 · Source of truth: `DeliveryOS-As-Built-Summary-v1.md` (2026-06-04)
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
e2e/          Playwright (92 tests × 3 breakpoints = 276)
migrations/   node-pg-migrate (delegates to packages/db/migrations)
.agents/
  rules/      always-on rules (design-system.md, graphify.md)
  workflows/  graphify.md
  skills/     deliveryos-theme, component-builder, screen-builder, deliveryos-ui
graphify-out/ Knowledge graph (run `graphify query "..."` before raw grep)
src/screens/  Static HTML mockups (legacy design reference only)
```

**Canonical reference doc** (read first when you need broad architecture context):
`DeliveryOS-As-Built-Summary-v1.md` — phase status, stack, shims, security posture, must-fix list.

## 3. Skill router (call the right skill, save tokens)

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
| Configure opencode itself | `customize-opencode` | Only for `.opencode/`, agents, MCP |

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

## 9. Known broken / must-fix before pilot (verified 2026-06-07)

| # | Item | Status | Evidence |
|---|---|---|---|
| 1 | Courier routes (assignments + shifts) registered in `server.ts` | ✅ FIXED | `server.ts:527-528` — both registered with prefix `/api/courier` |
| 2 | No per-phone order throttle (FX-4) | 🔴 STILL BROKEN | `orders.ts:50` — only global IP rate limit (10/min), no phone-based throttle |
| 3 | Operational pool connects as `postgres` superuser — RLS bypassed | ⚠️ CONFIG ISSUE | `packages/db/src/index.ts` — uses `DATABASE_URL_OPERATIONAL` env var; code does not hardcode role, but no guardrail prevents superuser connection |
| 4 | Theme/notification owner routes lack auth | ✅ FIXED | `owner/themes.ts:12-13`, `owner/notifications.ts:11-12` — both have `verifyAuth` + `requireRole(['owner'])` hooks |
| 5 | Idempotency dedup missing `location_id` scope (FX-5) | ✅ FIXED | `orders.ts:286-287,301,529-531` — all 3 ops (SELECT/DELETE/INSERT) scope by `location_id` |
| 6 | `MapLibreBase.innerHTML` → use `textContent` (XSS) | ✅ FIXED | `MapLibreBase.tsx:115` — uses `el.textContent = label` |
| 7 | No custom error handler (FX-6 leakage) | ✅ FIXED | `server.ts:459-478` — strips stack traces, returns safe messages + correlation ID |
| 8 | `statement_timeout` + acquire-timeout not set (FX-9) | ✅ FIXED | `packages/db/src/index.ts:28-29,48-49` — 10s operational, 30s session; `connectionTimeoutMillis: 5000` on both |

**Remaining blockers: per-phone throttle (FX-4) + DB role guardrail.** Security posture: HOLDS 12 · WEAK 3 · BROKEN 1.

## 10. Common commands

```powershell
pnpm dev:all                                # api(3000) + worker + ui(3001)
pnpm verify:env; pnpm verify:db; pnpm verify:rls; pnpm verify:secrets
pnpm migrate:up                             # apply migrations
pnpm migrate:create "<name>"                # new migration in packages/db/migrations
pnpm test:phase4 ; pnpm test:phase5         # stage tests
pnpm verify:launch                          # full pre-launch gate
pnpm lint ; pnpm typecheck ; pnpm format
pnpm backup:verify ; pnpm backup:drill ; pnpm backup:list
npx playwright test                         # e2e (chromium × 3 breakpoints)
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
