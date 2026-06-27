# DeliveryOS (dowiz) — Architecture Map

> Onboarding map + maintainer's index. Created to mitigate the bus-factor-1 risk (repowise reports
> a single owner across all ~1,665 files). Start here, then follow the linked ADRs for the *why*.

## What it is
A multi-tenant food-delivery platform: a public storefront (`/s/:slug`), an owner admin (`/admin/*`),
a courier app, and a customer order-tracking flow. pnpm monorepo, TypeScript end to end. ~197k NLOC.

## Topology

```
apps/
  api/      Fastify server (HTTP + WS). The product backend. Entry: apps/api/src/server.ts
  web/      React SPA (storefront + admin + courier + customer). Entry: apps/web/src/main.tsx
  worker/   Background job runner (pg-boss). Entry: apps/worker/src/index.ts
packages/
  db/           pg pools (operational vs session) + migrations (node-pg-migrate) + RLS
  platform/     cross-cutting infra: message bus, JWT (RS256), withTenant, queue provider
  domain/       pure business logic (order state machine, money) — zero deps
  config/       env schema + loadEnv (boot-guard)
  shared-types/ zod contracts shared BE↔FE
  ui/           React component library + i18n catalog + useWebSocket hook
tools/
  ccc/                 dev-only AST code search (ADR-0012) — `pnpm ccc`
  eslint-plugin-local/ the project's custom lint rules (the executable invariant gates)
```

Dependency direction (no cycles): `config → db → platform`; `domain`, `shared-types` are leaf;
`ui` depends on `shared-types`. **`apps/api` and `apps/web` never import each other.**

## Request → order, the critical path
`/s/:slug` (SSR shell, `spa-proxy.ts`) → menu from `read_public_menu()` (SECURITY DEFINER) →
`POST /api/orders` (`routes/orders.ts`: rate-limit → optional OTP → preflight signals → server-
authoritative pricing/tax → modifier validation → idempotent txn insert → outbox jobs) → WS push
to owner dashboard + customer track page (`useWebSocket`).

## Invariants (the red lines)
See **`docs/agent-rules/INVARIANTS.md`** — each invariant links to its executable gate (lint rule /
verify script / test / pre-commit hook). Money is integer minor units; tenant tables are RLS-scoped;
realtime only via `useWebSocket`; JWT is RS256; secrets never hardcoded; `crypto.randomUUID` for tokens.

## Error contract
One envelope `{ code, message, fields?, correlationId, retryAfterMs?, status, error }` from
`lib/api-error.ts` (`buildErrorEnvelope`), emitted by `setErrorHandler` + `reply.sendError`
(`lib/reply-send-error.ts`) + rate-limit + notFound. `code` is the stable BE↔FE contract — guarded by
`pnpm verify:error-contract`. See ADR-0010.

## Decisions (ADRs — the *why*)
`docs/adr/` — notably **0010** (error envelope), **0011** (menu-parse eval + grounding + redaction),
**0012** (agent token-economy / ccc), GEO-SEAMS, NOTIFICATION-CONSOLIDATION, p0-privacy-hardening,
soft-access-gate, 0004 (owner-token revocation). Design records: `docs/design/`.

## Verify gates (run before trusting a change)
`pnpm typecheck` · `pnpm verify:rls` · `pnpm verify:migrations` · `pnpm verify:error-contract` ·
`pnpm verify:menu-parse` · `pnpm verify:ccc-secrets` · `pnpm verify:secrets` · `pnpm lint`.
Pre-commit runs lint→typecheck→build→Docker; it does **not** run the unit suite — run targeted
`node --test --import tsx apps/api/tests/<spec>` for logic changes.

## Deploy
Staging: `flyctl deploy -a dowiz-staging --remote-only`. Prod: push to `main` (CI). Migrations run on
the staging DB first (red-line; `packages/db/` is protected — see staged artifacts under `docs/`).
Topology + flags: see the deploy-topology memo / `packages/config` env schema.

## Known hotspots (repowise health)
`routes/orders.ts` (ccn 156), `routes/spa-proxy.ts` (ccn 174), `server.ts` (ccn 116) — large, low test
coverage; decomposition tracked in the project analysis. Health 7.85/10, 42 alert files.
