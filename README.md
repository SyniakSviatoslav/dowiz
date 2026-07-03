# DeliveryOS

**A 0-commission, data-ownership food-delivery platform for small vendors.**

DeliveryOS gives each restaurant its own **branded ordering storefront**
(menu → cart → checkout → live order tracking) at `/s/:slug`, themed with that
restaurant's own brand — not the platform's. The aggregator model charges
vendors **25–35% commission** and owns the customer relationship. DeliveryOS
charges a **flat, predictable subscription with 0% transaction fees**, and the
diner data belongs to the **restaurant**.

> **Working repo name:** `dowiz`. The public brand name is an open decision —
> see [`TRADEMARK.md`](./TRADEMARK.md).

---

## Why this exists

- **0% commission.** Restaurants keep their margin. The enemy is the 25–35%
  aggregator cut; the wedge is a flat subscription whose ROI is obvious in
  days, not a tax on every order.
- **Your channel, your customers, your data.** Each vendor gets a boutique,
  white-label storefront that looks hand-made *for them*. Orders, customers, and
  insights stay with the restaurant — the platform is a **conduit, not a
  custodian**.
- **Built for small vendors.** Mobile-first, one-handed ordering for diners on
  flaky connections; a simple owner admin for staff. Primary market: Albania and
  EN/UK speakers (sq/en/uk).

## Pricing (subscription — 0% transaction fees, all tiers)

| Tier         | Monthly orders | Price       |
| ------------ | -------------- | ----------- |
| **Free**     | ≤ 50           | $0          |
| **Starter**  | ≤ 500          | $19 / mo    |
| **Pro**      | ≤ 2,000        | $39 / mo    |
| **Business** | Unlimited      | $59 / mo    |

There is **no per-order commission** on any tier. The **hosted cloud** is the
supported way to run DeliveryOS (see below).

## Architecture

DeliveryOS is a **pnpm monorepo** (TypeScript / Node.js end to end):

| Component      | Path            | Role |
| -------------- | --------------- | ---- |
| **API**        | `apps/api`      | Fastify HTTP API + WebSocket server (realtime order/courier updates) |
| **Web**        | `apps/web`      | React app — diner **storefront** (`/s/:slug`, SSR-friendly) and owner **admin** (`/admin`) |
| **Worker**     | `apps/worker`   | Background jobs (queue-in-Postgres — see `docs/adr/0001`) |
| **Packages**   | `packages/*`    | `db`, `config`, `domain`, `platform`, `shared-types`, `ui`, `voice` |

**Data & infra:**

- **Postgres** (Supabase-compatible) as the system of record, with
  **Row-Level Security** enforced for multi-tenant isolation.
- **Cloudflare R2** for media (menu/product photos, theme assets).
- **WebSockets** for live order and courier tracking.
- **Per-tenant theming** via brand tokens (`--brand-*`), so every storefront
  carries its restaurant's identity.

Design decisions are recorded as ADRs in [`docs/adr/`](./docs/adr).

## Hosted cloud is the supported path

The **hosted DeliveryOS cloud** always runs the latest release and is the
supported, recommended way to use the platform — restaurants get updates,
backups, and security fixes without operating infrastructure.

Self-hosting is fully supported by the licence (AGPL-3.0-only) and encouraged
for those who want it, but you are responsible for your own operations,
upgrades, and security. If you offer a **hosted service to third parties**, note
the AGPL §13 source-availability obligation and the brand rules in
[`TRADEMARK.md`](./TRADEMARK.md).

## Getting started (local development)

**Prerequisites:** Node.js ≥ 22, [pnpm](https://pnpm.io) 9, and a Postgres
database (local, Docker, or a hosted Postgres) for anything touching data.

```bash
pnpm install                          # install workspace deps
cp .env.example .env                  # then fill in local values
pnpm verify:env                       # sanity-check required env vars

pnpm migrate:up                       # apply DB migrations
pnpm seed                             # load local seed data

# start the apps (each in its own terminal)
pnpm --filter @deliveryos/api dev     # Fastify API + WebSocket server
pnpm --filter web dev                 # React app (Vite dev server)
pnpm --filter @deliveryos/worker dev  # background worker (optional)
```

There is no root `pnpm dev` — the apps are started per-package as above.

See [`CONTRIBUTING.md`](./CONTRIBUTING.md) for the full setup, the `verify:all`
quality gates, ship discipline, and the red→green guardrail rule, and
[`docs/ARCHITECTURE.md`](./docs/ARCHITECTURE.md) for a high-level system map.

## Why open source

- **Bus-factor / longevity.** A restaurant's ordering channel is
  business-critical. Open source means the platform can outlive any single
  operator — the vendor is never locked to a black box.
- **Data belongs to the client.** Transparency about how orders, addresses, and
  phone numbers are handled is a feature, not a liability. The code that touches
  a restaurant's customers is inspectable.
- **A collective tool.** The platform is built to serve small vendors, not to
  capture them.

Licensed under **AGPL-3.0-only** — the network-copyleft licence — so that
anyone offering it as a service keeps improvements open.

## Contributing & security

- **Contributing:** [`CONTRIBUTING.md`](./CONTRIBUTING.md) — DCO sign-off (no
  CLA), dev setup, quality gates, guardrail rule.
- **Code of conduct:** [`CODE_OF_CONDUCT.md`](./CODE_OF_CONDUCT.md) — how we
  work together.
- **Security:** [`SECURITY.md`](./SECURITY.md) — private disclosure, supported
  versions, security posture (RLS, Zod-strict validation, constant-time
  comparisons, plane-guard).
- **Architecture:** [`docs/ARCHITECTURE.md`](./docs/ARCHITECTURE.md) — system map
  and the critical order path. Design decisions live in [`docs/adr/`](./docs/adr).
- **Trademark / brand:** [`TRADEMARK.md`](./TRADEMARK.md).

## License

[**AGPL-3.0-only**](./LICENSE). Copyright © the DeliveryOS contributors.
