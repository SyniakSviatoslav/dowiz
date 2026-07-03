# DeliveryOS — Architecture

> A high-level system map for contributors. Start here, then follow the linked
> ADRs in [`docs/adr/`](./adr) for the *why* behind each decision.

## What it is

A multi-tenant food-delivery platform with four user-facing surfaces:

- a public, per-tenant **storefront** at `/s/:slug` (menu → cart → checkout),
- an owner **admin** at `/admin/*`,
- a **courier** app, and
- a **customer** order-tracking flow.

It is a [pnpm](https://pnpm.io) monorepo, TypeScript end to end.

## Topology

```
apps/
  api/      Fastify server (HTTP + WebSocket) — the product backend.
            Entry: apps/api/src/server.ts
  web/      React SPA (storefront + admin + courier + customer).
            Entry: apps/web/src/main.tsx
  worker/   Background job runner. Entry: apps/worker/src/index.ts
packages/
  db/           pg pools (operational vs session) + migrations (node-pg-migrate) + RLS policies
  platform/     cross-cutting infra: message bus, JWT (RS256), withTenant, queue provider
  domain/       pure business logic (order state machine, money) — zero runtime deps
  config/       env schema + loadEnv (boot-guard)
  shared-types/ Zod contracts shared between backend and frontend
  ui/           React component library + i18n catalog + useWebSocket hook
tools/
  ccc/                 dev-only AST code search (ADR-0012) — `pnpm ccc`
  eslint-plugin-local/ the project's custom lint rules (executable invariant gates)
```

**Dependency direction (no cycles):** `config → db → platform`; `domain` and
`shared-types` are leaf packages; `ui` depends on `shared-types`. `apps/api` and
`apps/web` never import each other — they communicate only over the HTTP/WS
contract in `shared-types`.

## The critical path: request → order

`/s/:slug` (SSR shell served by `spa-proxy.ts`) → menu loaded via the
`read_public_menu()` SECURITY DEFINER function → `POST /api/orders`
(`routes/orders.ts`): rate-limit → optional OTP → preflight signals →
**server-authoritative** pricing/tax → modifier validation → idempotent
transactional insert → outbox jobs → WebSocket push to the owner dashboard and
the customer track page (via the `useWebSocket` hook).

Pricing is always derived on the server; the client never supplies prices. See
[`docs/pricing.md`](./pricing.md) for the exact formula and error codes.

## Invariants (the red lines)

See [`docs/agent-rules/INVARIANTS.md`](./agent-rules/INVARIANTS.md) — each
invariant links to its executable gate (lint rule / verify script / test /
pre-commit hook). In summary:

- Money is stored and computed in integer **minor units** (no floats).
- Tenant tables are **RLS-scoped** and `FORCE`d; DB roles carry no `BYPASSRLS`.
- Realtime is only consumed through the `useWebSocket` hook.
- JWTs are **RS256**; secrets are never hardcoded.
- Tokens use `crypto.randomUUID()`.

## Error contract

Every error is returned as one envelope —
`{ code, message, fields?, correlationId, retryAfterMs?, status, error }` —
built by `lib/api-error.ts` (`buildErrorEnvelope`) and emitted by the global
`setErrorHandler`, `reply.sendError` (`lib/reply-send-error.ts`), the rate
limiter, and the not-found handler. `code` is the stable backend↔frontend
contract, guarded by `pnpm verify:error-contract`. See ADR-0010.

## Decisions (ADRs — the *why*)

Architecture Decision Records live in [`docs/adr/`](./adr). Notable ones:
**0004** (owner-token revocation), **0010** (error envelope), **0011**
(menu-parse eval, grounding, redaction), **0012** (agent token-economy / ccc),
plus records on geo seams, notification consolidation, privacy hardening, and
the soft access gate. Longer design records are in [`docs/design/`](./design).

## Verify gates

Run these before trusting a change (see [`CONTRIBUTING.md`](../CONTRIBUTING.md)
for the full quality-gate policy):

```
pnpm typecheck            pnpm lint                 pnpm verify:rls
pnpm verify:migrations    pnpm verify:error-contract pnpm verify:menu-parse
pnpm verify:secrets       pnpm verify:ccc-secrets   pnpm verify:all
```

The pre-commit hook runs lint → typecheck → build. It does **not** run the unit
suite, so run targeted tests for logic changes, e.g.
`node --test --import tsx apps/api/tests/<spec>.test.ts`.

## Deployment model

DeliveryOS is containerized (see the root `Dockerfile`) and deploys as the API
server (which also serves the built web assets) plus the worker. Database
migrations are applied **before** the app boots — the config boot-guard fails
closed if the schema is behind. The environment contract is defined in
`packages/config`; copy `.env.example` and fill in your own values. The hosted
cloud is the supported way to run DeliveryOS in production (see the README).
