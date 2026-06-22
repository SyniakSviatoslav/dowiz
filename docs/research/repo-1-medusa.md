# Repo Dossier 1 — medusajs/medusa

> Research target: **`medusajs/medusa`** — Node/TS commerce platform.
> Purpose: extract patterns/code we can reuse for **DeliveryOS** (white-label, per-restaurant
> food-ordering; Fastify monolith + Vite SSR + pg-boss worker; Supabase PG17; own `ws`;
> React 18 PWA; RLS tenant isolation; `organizations → locations → …`; cash-on-delivery; 10 order states).
> Every claim below cites a file path, release, or docs URL. License gates everything — see R1.

---

## R1 — Identity & License

| Field | Value | Evidence |
|---|---|---|
| Exact slug | `medusajs/medusa` | https://github.com/medusajs/medusa |
| Stars | ~34.5k | repo landing page |
| Recency | Latest release **v2.16.0**, released ~June 18 2026; `develop` actively committed | https://github.com/medusajs/medusa/releases/tag/v2.16.0 |
| Primary langs | TypeScript ~85.5%, JavaScript ~14.3% | repo language bar |
| License | **MIT** | `LICENSE` (raw) |

**LICENSE contents (verbatim head):**
```
The MIT License (MIT)
Copyright (c) 2021 Medusajs
Permission is hereby granted, free of charge, to any person obtaining a
```
Standard MIT: use / modify / distribute / sublicense freely; "as-is", no warranty.

### Reuse verdict: **`copyable (MIT + attribution)`**
Source code may be lifted verbatim into DeliveryOS provided the MIT copyright + permission
notice is preserved (e.g. a NOTICE / third-party-licenses entry). No copyleft, no patent
clause complications. This is the most permissive outcome — both *patterns* and *literal code*
are on the table, subject only to the attribution line.

---

## R2 — Stack & Topology

Medusa v2 is a **modular monolith** built as a Yarn-workspaces + Turborepo monorepo, NOT a
microservice mesh. Evidence: `turbo.json`, `.yarnrc.yml`, `packages/` at repo root.

| Concern | Medusa v2 | Evidence |
|---|---|---|
| HTTP framework | **Express ^4.21** | `packages/medusa/package.json` deps |
| ORM | **MikroORM 6.6.x** (Postgres) | v2.16.0 changelog (MikroORM bumped to 6.6.14, CVE fix) |
| DB | PostgreSQL | docs / module data-models |
| Node engine | **>=20** | `packages/medusa/package.json` engines |
| Jobs / scheduling | `node-schedule` + **Workflow Engine** (`@medusajs/workflow-engine-inmemory` / `-redis`) | package.json deps |
| Events | **Event Bus** (`event-bus-local` / `event-bus-redis`) | package.json deps |
| Cache | `cache-inmemory` / `cache-redis` | package.json deps |
| Auth | `jsonwebtoken ^9` | package.json deps |
| File upload | `multer ^2` | package.json deps |
| Module split | `packages/modules/*` — 35 modules | `packages/modules` listing |

**Module catalogue** (`packages/modules`): `analytics, api-key, auth, cart, currency, customer,
file, fulfillment, index, inventory, link-modules, locking, notification, order, payment,
pricing, product, promotion, providers, rbac, region, sales-channel, settings, stock-location,
store, tax, translation, user, workflow-engine-*, event-bus-*, cache-*`.

**Topology mapping onto DeliveryOS:**
- Medusa = **one process, many in-proc modules** linked by an in-memory or Redis event bus +
  a workflow engine. DeliveryOS is also a monolith (Fastify) + one separate worker — *architecturally
  similar shape*, so module-boundary ideas transfer.
- **Divergence (skip):** Medusa's distributed-grade machinery (Redis workflow engine, Redis event
  bus, Redis locking, MikroORM) is heavier than DeliveryOS's chosen stack: pg-boss-in-Postgres for
  jobs, Redis **pub/sub only**, raw SQL/lightweight access on Supabase PG17, own `ws` server. Do NOT
  import `@medusajs/framework` — it would drag MikroORM + the DI container + Redis assumptions.
- The **module ⇒ data-model ⇒ workflow** layering is the borrowable idea, not the runtime.

---

## R3 — Data Model & Multi-Tenancy

**Core entities** (modules, each owns its tables): `Store`, `SalesChannel`, `Region`, `Cart`,
`LineItem`, `Order`, `Product`, `ProductVariant`, `Customer`, `Fulfillment`, `Payment`,
`Pricing`, `StockLocation`, `Inventory`, `Promotion`, `Tax`. Cross-module relations are made via
the **link-modules** package (no foreign keys across module boundaries; "module links" instead).

**Entity graph** (from DeepWiki order-management + docs):
`Customer → Order`; `Order → LineItem`; `LineItem → ProductVariant`; `ProductVariant → Product`;
`Cart → Order` (cart precedes order at checkout); `Order → Fulfillment`; `Order → Payment(Collection)`.
`Region` carries currency/tax/fulfillment config; `SalesChannel` scopes product availability;
`StockLocation` scopes inventory.

**Multi-tenancy approach — the critical comparison:**
- Medusa **does NOT do row-level multi-tenancy** out of the box. The **Store Module** allows multiple
  stores in one instance, and `SalesChannel` + `sales_channel_id` on orders is the documented filter
  for channel/tenancy separation (docs: Store Module; Sales-Channel module).
- Medusa's own guidance for true SaaS multi-tenancy is **instance-per-tenant** (separate Medusa
  instance + DB per tenant) — see medusajs.com multi-tenant blog and discussion #11671. Community
  recipes bolt on **Postgres RLS** manually (rigbyjs multi-tenancy guide) — i.e. RLS is *not* native.

**Closeness to DeliveryOS `organizations → locations`:**
- **Conceptually close but inverted in rigor.** DeliveryOS makes `location_id` the hard tenancy key
  enforced by **RLS** at the DB; Medusa makes `sales_channel_id` a *soft application-level filter*
  with no DB enforcement, and pushes hard isolation to instance-per-DB.
- Closest analogues: DeliveryOS `organizations` ≈ Medusa `Store`; DeliveryOS `locations` ≈ Medusa
  `SalesChannel`/`StockLocation` (a sellable venue with its own catalog/inventory).
- **Takeaway — borrow the entity vocabulary, REJECT the isolation model.** DeliveryOS's RLS-enforced
  `location_id` is *stronger* than Medusa and must stay. Do not weaken to app-level filtering.
  Touches module: **locations**, **menu**, **orders**.

---

## R4 — Order State Machine

Medusa does **not** use one linear status. It tracks **three orthogonal status axes** on an order,
each its own enum. Verbatim values from `packages/core/utils/src/order/status.ts` and
`packages/core/types/src/order/common.ts`:

**`OrderStatus`** (6): `pending · completed · draft · archived · canceled · requires_action`

**`PaymentStatus`** (10): `not_paid · awaiting · authorized · partially_authorized · captured ·
partially_captured · partially_refunded · refunded · canceled · requires_action`

**`FulfillmentStatus`** (8): `not_fulfilled · partially_fulfilled · fulfilled · partially_shipped ·
shipped · partially_delivered · delivered · canceled`

Plus `ReturnStatus`, `ClaimType`, `ClaimReason` for post-purchase flows.

**Who triggers transitions:** there is no explicit FSM guard table — transitions are driven by
**workflows** (`@medusajs/core-flows`), e.g. `createOrderWorkflow`, `cancelOrderWorkflow`
(cancels fulfillments → refunds/cancels payment → cancels order, in order). Status fields are
recomputed from sub-entity state (creating a fulfillment flips `not_fulfilled → partially/fulfilled`;
shipping flips to `partially_shipped/shipped`; delivering to `partially_delivered/delivered`).
Sources: docs Order-Fulfillment user guide; DeepWiki 4.4 order-management; issue #9292 (status-update logic).

**COMPARE to DeliveryOS 10-state cash-on-delivery machine:**
- **Fundamentally different model.** DeliveryOS = **one linear 10-state FSM** with explicit
  transitions. Medusa = **3 independent multi-valued axes** with no single ordered path. The two
  are not isomorphic; do **not** map 1:1.
- For COD, most Medusa `PaymentStatus` values are dead weight (no auth/capture split, no partial
  captures, no refund tiers for cash). DeliveryOS collapses payment to essentially paid-on-delivery.
- Medusa's `FulfillmentStatus` *delivery* tail (`shipped → delivered`, with `partially_*` variants)
  is the **closest analogue to courier progression** and is worth reading for naming — but DeliveryOS
  has no partial-shipment concept (single restaurant, single order), so the `partially_*` states are N/A.
- **Verdict — adapt naming, reject structure.** Keep the single linear FSM. **N/A** as a UX
  convention (Medusa exposes no customer-facing state UX worth honoring here). Touches: **orders**, **couriers**.

---

## R5 — Real-Time

- **Core Medusa has no first-party realtime transport for storefronts.** Realtime is achieved via the
  **Event Bus** (Redis/local) for server-side reactions, not a client push channel. There is no
  bundled WebSocket server for end users.
- The food-delivery reference (**medusa-eats**, R-extra below) implements live order/delivery tracking
  with **Server-Sent Events (SSE)** on top of a **long-running Medusa Workflow**, plus
  automatic notifications — NOT WebSockets (medusa-eats README).

**Relevance to DeliveryOS (own `ws`, Supabase Realtime OFF, Redis pub/sub only):**
- DeliveryOS's choice of a **dedicated `ws` server + Redis pub/sub fan-out** is *more capable* than
  Medusa core (which has none) and a different transport than medusa-eats (SSE). **Skip** Medusa here
  for transport.
- **One borrowable idea:** medusa-eats' "the order lifecycle IS a long-running workflow, and every
  state change emits an event the client subscribes to" — DeliveryOS already does the event side via
  Redis pub/sub → `ws`. Confirm rather than adopt. Touches: **orders**, **couriers**.

---

## R6 — Component System

Medusa's admin UI is a **published, MIT React design system** (separate repo `medusajs/ui`, mirrored
into the monorepo admin):
- **`@medusajs/ui`** — React components/hooks/utils, built on **Radix UI primitives**.
- **`@medusajs/ui-preset`** — a **Tailwind CSS preset** defining the design **tokens** (the shared
  config consumed via Tailwind's `presets`).
- **`@medusajs/icons`** — icon set.
- Tokens also published as a **Figma** library.
Sources: https://docs.medusajs.com/ui ; https://github.com/medusajs/ui ; npm `@medusajs/ui-preset`.

**Relevance to DeliveryOS `packages/ui` + CSS-var theming:**
- **Strong architectural validation of DeliveryOS's split** (`packages/ui` = components +
  `ui-preset`-style token layer). Medusa proves the "components on Radix + Tailwind preset of tokens"
  pattern at scale.
- **CRITICAL DEVIATION — theming mechanism.** Medusa tokens live in a **Tailwind preset** (build-time,
  one brand). DeliveryOS requires **per-tenant runtime branding** and mandates colors ONLY via CSS
  variables `var(--brand-*)`. Medusa's preset cannot do runtime multi-tenant recoloring, so DeliveryOS
  must keep its CSS-var indirection. **Adapt the structure, REJECT the static-preset token delivery.**
- shadcn/ui (DeliveryOS) and `@medusajs/ui` are **both Radix-based** — component *patterns* (dialog,
  table, badge, status pill) are directly readable/portable. **MAY-DEVIATE** on visual styling
  (different brand), **borrow** structural component code where useful (MIT-clean). Touches: **ui**, **branding**.

---

## R7 — Checkout & Payments

**Payment-provider abstraction** — clean and directly instructive for COD:
- A payment provider is a module whose service extends **`AbstractPaymentProvider`** (from
  `@medusajs/framework/utils`), implementing: `initiatePayment, authorizePayment, capturePayment,
  cancelPayment, refundPayment, getPaymentStatus, retrievePayment, updatePayment`.
- **Manual / "system" provider** (`pp_system`, npm `medusa-payment-manual`) is the built-in
  **placeholder** that does NOT auto-process — it **delegates settlement to the merchant**. This is
  the exact shape of **cash-on-delivery**.
- Flow: a **PaymentSession** is created per provider on the cart → authorized → becomes a `Payment`
  on the order → later captured/refunded. Price/fee/tax breakdown lives in the **Pricing** + **Tax**
  + **Promotion** modules and on `LineItem`/`Cart` totals, computed at cart time.
Sources: https://docs.medusajs.com/resources/commerce-modules/payment/payment-provider ;
medusajs.com manual-payment integration ; npm `medusa-payment-manual`.

**Relevance / takeaway:**
- **BORROW the `AbstractPaymentProvider` + "system/manual" pattern conceptually.** DeliveryOS is
  cash-on-delivery today; modeling COD as one provider behind a thin interface (`initiate / capture
  (=mark-collected) / cancel / refund`) gives a clean seam to add Albanian online payment later
  without touching order code. This is the single most reusable Medusa pattern for DeliveryOS payments.
- **Reject** the full PaymentSession/PaymentCollection machinery (auth/capture/partial split) — for
  COD it's overkill; collapse to a `paid_on_delivery` boolean/timestamp + a provider hook. Touches: **orders**.

---

## R8 — Patterns to Adopt + Anti-Patterns

**Adopt (license-clean, MIT):**
1. **Provider abstraction for settlement** (`AbstractPaymentProvider`, manual provider) → model COD
   as a provider. (orders)
2. **Module = data-model + service + workflows** boundary discipline — keeps order logic out of HTTP
   handlers. Mirrors DeliveryOS's "business logic keyed on `location_id`" intent. (orders/menu)
3. **Multi-axis status thinking** — separating *payment* state from *fulfillment/delivery* state is a
   sound idea even inside a single linear FSM (e.g. DeliveryOS can carry a `paid` flag orthogonal to
   the courier-progress state). (orders)
4. **UI = Radix components + Tailwind token preset** split — validates `packages/ui`. (ui)
5. **Workflow-orchestrated cancellation ordering** (cancel fulfillments → settle payment → cancel
   order) — a correctness pattern worth copying for DeliveryOS order-cancel. (orders/couriers)

**Anti-patterns / do-NOT-import:**
1. **Soft, app-level tenancy** (`sales_channel_id` filter, instance-per-tenant for real isolation) —
   weaker than DeliveryOS RLS. Keep RLS. (locations)
2. **Heavy runtime deps** — MikroORM + DI container + Redis workflow engine + Redis event bus +
   Redis locking. Conflicts with pg-boss-in-Postgres / Redis-pub-sub-only / Supabase. (infra)
3. **Static Tailwind-preset tokens** — incompatible with per-tenant runtime `var(--brand-*)` branding. (branding)
4. **Over-rich status enums** for a cash, single-venue flow (10 payment states, partial-shipment
   states) — YAGNI for DeliveryOS. (orders)
5. **No bundled storefront realtime** — core gives nothing; do not look to Medusa for `ws`. (orders)

---

## R9 — Liftable (license-permitting) vs Rewrite

License gate: **MIT — lifting literal code is permitted with attribution.**

| Item | Decision | Why / DeliveryOS module |
|---|---|---|
| `AbstractPaymentProvider` interface shape + manual-provider semantics | **LIFT (adapt)** — copy the method seam, trim to COD | orders |
| Order status enum *naming* (delivery tail: `shipped/delivered`) | **BORROW vocabulary** — re-author as one linear 10-state FSM | orders/couriers |
| `cancelOrderWorkflow` ordering logic | **BORROW pattern** — re-implement in pg-boss worker / handler | orders |
| `@medusajs/ui` Radix component bodies (badge/status-pill/table/dialog) | **LIFT selectively** — restyle to `var(--brand-*)`, keep MIT notice | ui |
| Sales-channel / store tenancy model | **REWRITE / REJECT** — DeliveryOS RLS `location_id` is stronger | locations |
| MikroORM data-models, DI container, workflow engine, event bus | **REWRITE / SKIP** — incompatible runtime; use Supabase + pg-boss + own `ws` | infra/orders |
| Tailwind `ui-preset` token delivery | **REWRITE** — must be runtime CSS vars, not build-time preset | branding/ui |
| SSE realtime (from medusa-eats) | **SKIP** — DeliveryOS uses own `ws` + Redis pub/sub | orders/couriers |

---

## R-extra — Medusa-based food-delivery build (located)

**`medusajs/medusa-eats`** — "An Uber Eats-style food delivery platform, running Medusa 2.0 and
Next.js 14." https://github.com/medusajs/medusa-eats
- **License:** MIT. **Stars:** ~251. **Stack:** Next.js 14 + Tailwind frontend; Medusa 2.0 backend;
  ~99% TypeScript. Built at a Paris hackathon; **explicitly an architectural reference, not maintained**.
- **Three-sided** (customer / restaurant / driver) via a **custom Restaurant module** + a
  **long-running Medusa Workflow** that drives the whole delivery lifecycle (order → notify restaurant
  → find driver → driver claims route → update customer).
- **Realtime = Server-Sent Events** (not WebSockets) + automatic notifications.
- **Most relevant artifact for DeliveryOS:** it's the clearest demonstration of how to extend a
  commerce core with a per-venue **Restaurant module** and a courier-claim flow on top of Medusa's
  order model — i.e. the same problem DeliveryOS solves natively. Worth a deeper read for the
  restaurant/driver entity design and the workflow-as-lifecycle pattern (couriers, orders), while
  ignoring its SSE/Next.js transport choices.

---

### Sources
- Repo + LICENSE: https://github.com/medusajs/medusa · raw `LICENSE`
- `packages/medusa/package.json`, `packages/modules/` listing
- `packages/core/utils/src/order/status.ts`, `packages/core/types/src/order/common.ts`
- Release v2.16.0: https://github.com/medusajs/medusa/releases/tag/v2.16.0
- Docs: order-fulfillment user guide; Store / Sales-Channel / Payment-Provider modules; `@medusajs/ui`
- DeepWiki: https://deepwiki.com/medusajs/medusa/4.4-order-management
- Multi-tenancy: medusajs.com multi-tenant blog; discussion #11671; rigbyjs RLS guide
- Food-delivery: https://github.com/medusajs/medusa-eats ; medusajs.com "Announcing Medusa Eats"
