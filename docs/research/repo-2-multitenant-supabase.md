# Repo Dossier 2 — Multi-Tenant React + TS + Supabase Food-Ordering (Structural Twin)

> Research target: ONE strong OSS multi-tenant React+TS+Supabase food-ordering project, mapped onto DeliveryOS.
> Method: GitHub topic/keyword search + raw-file reads (no clone). Every claim cites a file/URL.
> Date: 2026-06-22. Reuse verdict gates everything — **license-first**.

---

## Pick

**`roshanx0/restaurant-ordering-saas`** — https://github.com/roshanx0/restaurant-ordering-saas
> "Multi-tenant restaurant ordering platform with QR menus, real-time orders, and admin dashboard. Built with React + TypeScript + Supabase."

This is the closest structural twin found: multi-tenant + Supabase Postgres + RLS + real-time + admin dashboard + per-restaurant slug storefront. It mirrors DeliveryOS's `/s/:slug` per-restaurant (NOT marketplace) shape almost 1:1 at the concept level.

**Runner-up:** `masterabdullah95/quickbite-eats` (React + Supabase, customer storefront + admin) — but it is **single-tenant**, so it loses the key multi-tenancy comparison and is dropped. Other near-results (`arnobt78/...MERN`, `YunmeiYe/food-ordering-app`) are MongoDB/Mongo-Next, not Supabase, so off-stack.

---

## R1 — Identity & License  →  VERDICT: **COPYABLE (with attribution)**

| Field | Value | Source |
|---|---|---|
| Slug | `roshanx0/restaurant-ordering-saas` | repo URL |
| Stars | 21 | GitHub API `/repos/...` |
| Forks | 3 | GitHub API |
| Created | 2025-12-07 | GitHub API `created_at` |
| Pushed | 2025-12-07 | GitHub API `pushed_at` |
| Updated | 2026-06-12 | GitHub API `updated_at` |
| Language | TypeScript 90.8% / PLpgSQL 6.9% | repo languages |
| Open issues | 0 | GitHub API |
| **License** | **MIT** (`spdx_id: MIT`, `LICENSE` file present) | GitHub API + `/LICENSE` |

**Recency caveat:** all real work landed in a single window (created and last-pushed both 2025-12-07; the 2026-06 "updated" is metadata-only). It is a **young, low-star, essentially single-author snapshot, not an actively maintained project.** Treat it as a high-quality *reference design*, not a dependency.

**License verdict:** MIT → **copyable**. We MAY lift code verbatim provided we keep the MIT copyright/permission notice. No copyleft, no all-rights-reserved. (Had there been no LICENSE → `avoid`/patterns-only; that gate does not bite here.)

---

## R2 — Stack & Topology

| Layer | This repo | DeliveryOS | Compat |
|---|---|---|---|
| Frontend | React 18 + TS + **Vite** | React 18 PWA + Vite SSR | ✅ same base; we add SSR + PWA |
| Routing | React Router v6 | (own SSR routing) | ⚠️ different |
| Styling | Tailwind CSS | Tailwind + shadcn/ui | ✅ Tailwind shared |
| State | (component/service-level; no Zustand/TanStack) | Zustand + TanStack Query | ⚠️ they're thinner |
| Backend | **Supabase only** (PG + Auth + Realtime), no app server | Fastify monolith + Vite SSR + worker | ❌ major divergence |
| Queue | none | pg-boss (Postgres) | ❌ N/A in twin |
| Realtime | **Supabase Realtime** (`postgres_changes`) | own `ws`, Supabase Realtime OFF | ❌ divergence (see R5) |
| DB | Supabase Postgres | Supabase PG17 + RLS | ✅ same engine |
| Validation | none visible (no Zod) | Zod strict shared | ⚠️ we're stricter |

Topology shape: this twin is a **"thick-client + Supabase-as-backend"** app — the React SPA talks straight to Supabase via the anon key, RLS does the access control, no application tier. DeliveryOS deliberately put a **Fastify tier + own ws + pg-boss** in front of Postgres. So the twin is architecturally *simpler/flatter*; we borrow its **data shapes and RLS ideas**, not its topology.

Files: `package.json`, `src/config/supabase.ts`, `vite.config.ts`, `tailwind.config.js`, `src/services/restaurantService.ts`, `src/services/adminService.ts`.

---

## R3 — Data Model & Multi-Tenancy  ← KEY COMPARISON

Source: `database/setup.sql` (verbatim CREATE TABLE / CHECK / POLICY read).

**8 tables:** `registration_requests`, `restaurants`, `users`, `menu_categories`, `menu_items`, `orders`, `admin_users`, `notifications`.

### Multi-tenancy mechanism
- **Single flat tenant key: `restaurant_id UUID REFERENCES restaurants(id) ON DELETE CASCADE`** on every tenant-owned table (`users`, `menu_categories`, `menu_items`, `orders`, `notifications`).
- Tenant = `restaurants` row; **no `organizations` parent, no `locations` layer** — one restaurant = one tenant = one storefront.
- Storefront addressed by `restaurants.slug` (UNIQUE) → public route `/:slug` (their analog of our `/s/:slug`).
- Isolation enforced by **RLS** (no schema-per-tenant, no separate DBs).
- `UNIQUE(restaurant_id, order_number)` and `UNIQUE(restaurant_id, name)` scope natural keys per tenant — good pattern.
- Indexes are all tenant-prefixed: `idx_orders_status (restaurant_id, status)`, `idx_orders_created_at (restaurant_id, created_at DESC)`, etc. — correct composite-index discipline for a tenant-keyed table.

### vs DeliveryOS
| | Twin | DeliveryOS |
|---|---|---|
| Tenant hierarchy | `restaurants` (flat) | `organizations → locations` (2-level) |
| Tenant key on rows | `restaurant_id` | **`location_id`** |
| Isolation | RLS via `auth.uid()→users.restaurant_id` subquery | RLS on `location_id` |
| Menu shape | `menu_categories → menu_items` (sizes/addons as JSONB) | category/item/**modifier** (relational) |
| Order line items | `orders.items` **JSONB blob** | relational order/order-item |

**Verdict on the model:** The twin's tenancy is a **degenerate single-level case** of ours (their `restaurant_id` ≈ our `location_id`, with `organizations` collapsed away). The *pattern* (tenant-id FK on every table + tenant-prefixed composite indexes + per-tenant unique constraints) is directly **adoptable**. Two divergences we should NOT copy: (a) **JSONB `items` / `sizes` / `addons`** — DeliveryOS uses relational modifiers (Zod-validated, queryable, price-auditable); keep relational. (b) their **flat tenant** — we need the org layer for multi-location owners.

### RLS policies (verbatim, the meat of R3)
```sql
-- public read of menu + active restaurants; public can INSERT orders
CREATE POLICY "Public can view available menu items" ON menu_items FOR SELECT USING (is_available = TRUE);
CREATE POLICY "Public can view active restaurants"   ON restaurants FOR SELECT USING (is_active = TRUE AND status = 'active');
CREATE POLICY "Public can create orders"             ON orders FOR INSERT WITH CHECK (TRUE);
-- owner isolation: auth.uid() must be a users row mapped to that restaurant
CREATE POLICY "Restaurant owners can manage orders"  ON orders FOR ALL USING (
  auth.uid()::text IN (SELECT id::text FROM users WHERE restaurant_id = orders.restaurant_id));
CREATE POLICY "Restaurant owners can manage menu"     ON menu_items FOR ALL USING (
  auth.uid()::text IN (SELECT id::text FROM users WHERE restaurant_id = menu_items.restaurant_id));
```
Pattern to adopt: **public anon SELECT gated by a status flag** (`is_available`, `status='active'`) + **owner write gated by a uid→tenant subquery**. This is exactly the storefront-public / owner-private split DeliveryOS needs, expressed as RLS.

⚠️ **But note the latent bug (see R8):** the owner policies key on `auth.uid()` (a Supabase Auth identity) while login is done against the **custom `users`/`admin_users` tables via RPC** (`password_hash` columns, RPC functions that bypass RLS). If sessions aren't actually Supabase-Auth users whose `auth.uid()` equals `users.id`, these `auth.uid()`-based policies match **nothing** and owner data is reachable only through RLS-bypassing RPC. This is a **multi-tenant RLS pitfall to learn from, not copy.**

---

## R4 — Order State Machine  vs  DeliveryOS 10-state COD

Twin `orders.status` CHECK: **`pending → accepted → preparing → ready → completed`**, plus terminal `cancelled`, `rejected` (**7 states**).
Timestamp columns mark transitions: `accepted_at, preparing_at, ready_at, completed_at, cancelled_at` (event-sourcing-lite; good pattern to mirror).
`order_type ∈ (qr, counter, phone, table)`; `payment_status ∈ (pending, paid, failed)`.

**Enforcement:** CHECK constraint only — **no transition guard / no state-machine** (any status can be set to any other; trigger only auto-generates `order_number`). Transitions are enforced in app code (`updateOrderStatus()`), not the DB.

**vs DeliveryOS (10-state COD):** the twin's 7 states are a **subset** of ours and align on the happy path (pending→accepted→preparing→ready→...). We additionally model courier/delivery + COD-specific states (e.g. assignment, en-route, delivered, plus cash-collected/auto-cancel sweeps). **Adopt:** the per-state `*_at` timestamp columns (clean audit + analytics). **Don't adopt:** the "any→any" looseness — DeliveryOS enforces legal transitions (state machine), which the twin lacks.

---

## R5 — Real-Time  →  **DIVERGENCE (expected & deliberate)**

Twin **USES Supabase Realtime**:
```sql
ALTER TABLE orders REPLICA IDENTITY FULL;
ALTER PUBLICATION supabase_realtime ADD TABLE orders;   -- also restaurants, menu_items, registration_requests
```
Client subscribes via `supabase.channel().on('postgres_changes', { filter: restaurant_id }).subscribe()` and **refetches** on change (`restaurantService.ts`).

**DeliveryOS deliberately disabled Supabase Realtime and runs its own `ws`** + Redis pub/sub. So this is a **known divergence, not a defect in either**. Why DeliveryOS chose own ws (for the record):
- A Fastify tier already exists → it can own the socket, auth it with our 15m JWT, and fan out via Redis pub/sub across instances — no second auth surface.
- Own ws lets us push **derived/business events** (state-machine transitions, courier assignment, claim-checked privacy-minimized payloads) rather than **raw row CDC**, which would leak columns and bypass our Zod/serialization layer.
- Avoids `REPLICA IDENTITY FULL` write amplification on a hot `orders` table and avoids coupling client to physical schema.

**Compat takeaway:** the twin's `postgres_changes`+refetch pattern is the *anti-pattern* DeliveryOS routes around. Adopt only the **conceptual filter (`restaurant_id`/`location_id`-scoped subscription)** — re-express it as a topic on our own ws, not a Supabase channel. **SKIP** the Supabase Realtime wiring.

---

## R6 — Component System & Theming  →  relevance to `packages/ui` + `var(--brand-*)`

- Hand-rolled primitive set in `src/components/ui/` (`Button, Card, Modal, Input, Select, Badge, Alert, Loading, Textarea`) — **shadcn-shaped but NOT shadcn** (no CVA, no Radix). Variants are plain TS objects: `primary: "bg-accent text-white hover:bg-accent/90"` (`Button.tsx`).
- **Semantic tokens via Tailwind config**, but the values are **hardcoded hex** in `tailwind.config.js` (`accent #000000`, `accent-secondary #6366F1`, `bg #FFFFFF`, `text #0A0A0A`, `error #EF4444`, `success #10B981`, `warning #F59E0B`, …). **No CSS variables.**

**vs DeliveryOS** (`packages/ui`, shadcn/ui + Tailwind, colors via `var(--brand-*)`): the twin has the right **semantic-token instinct** (named `accent/bg/text/error` rather than raw colors) but stops short of **runtime theming** — hardcoded hex can't do per-tenant white-label branding. DeliveryOS's `var(--brand-*)` indirection is strictly better for our white-label/auto-branding requirement (derivePalette).
- **HONOR:** semantic token naming (accent/bg/text/success/warning/error) — matches our palette intent.
- **MAY-DEVIATE / actually superior in DeliveryOS:** runtime `var(--brand-*)` over compile-time hex (required for per-restaurant theming; do NOT regress to their hardcoded model).
- **N/A:** their Button variant object — we already have shadcn CVA.

---

## R7 — Checkout & Payments

- `orders` carries a **stored price breakdown**: `subtotal, tax, discount, total` (all `DECIMAL(10,2)`, each `CHECK (>= 0)`). Mirrors our breakdown intent.
- **No price computation on the server / service** — `restaurantService.createOrder()` just persists whatever totals the client sends (`total` is set at creation, not recomputed). **Trust-the-client pricing = anti-pattern** (tamperable totals). DeliveryOS must compute/validate totals server-side (Fastify + Zod), never trust client.
- **Payment abstraction:** essentially none — `payment_method TEXT` (free text), `payment_status ∈ (pending,paid,failed)`, `payment_transaction_id`. No provider integration, no gateway code. Effectively a **manual / cash-style** flow (set `paid` from the dashboard).
- **Cash-on-delivery:** not explicitly modeled, but the free-text `payment_method` + manual `payment_status` *accommodates* COD trivially (`payment_method='cash'`, mark `paid` on collection). This aligns with DeliveryOS's COD-first design.
- ⚠️ `DECIMAL` money: DeliveryOS standard is **integer minor units** (memory: "integer tax / money fix"). Do **not** adopt their `DECIMAL(10,2)`; keep integer cents.

---

## R8 — Patterns to ADOPT + Anti-Patterns / Supabase Pitfalls

### Adopt (MIT-clean)
1. **Tenant-id FK on every table + tenant-prefixed composite indexes** (`(restaurant_id, status)`, `(restaurant_id, created_at DESC)`) — map to `location_id` composites. → DeliveryOS `packages/db`.
2. **Per-tenant unique constraints** (`UNIQUE(restaurant_id, order_number)`, `UNIQUE(restaurant_id, name)`) → `UNIQUE(location_id, ...)`.
3. **Public-read-by-status-flag RLS** (`is_available=TRUE`, `status='active'`) for the storefront surface; **owner-write-by-uid→tenant subquery** RLS for admin. → our RLS policies on `location_id`.
4. **Per-state `*_at` timestamp columns** on orders (cheap audit + analytics, no extra event table).
5. **Slug-addressed storefront** + `idx_restaurants_slug` — we already do `/s/:slug`; their index discipline confirms it.
6. **Status-as-CHECK-enum** for small closed sets — fine as a defense-in-depth layer *under* an app state machine.

### Anti-patterns / DO NOT copy
1. **JSONB order/menu blobs** (`orders.items`, `menu_items.sizes/addons`) — unqueryable, unvalidated, un-auditable pricing. Keep relational + Zod.
2. **Client-set order totals** (no server recompute) — tamperable. Compute & validate in Fastify.
3. **Any→any order transitions** (CHECK only, no guard) — enforce a real state machine.
4. **Hardcoded hex theme tokens** — blocks white-label; keep `var(--brand-*)`.
5. **`DECIMAL` money** — keep integer minor units.

### Supabase pitfall list (learned from this repo)
- **P1 — `auth.uid()` RLS vs custom `users` table mismatch.** Repo logs users in via custom `users`/`admin_users` tables + RPC (`password_hash`), yet RLS policies key on `auth.uid()`. If the session isn't a real Supabase-Auth user whose `auth.uid() == users.id`, every owner policy matches zero rows → isolation effectively delegated to RLS-**bypassing** RPC functions. **Lesson:** RLS predicates must reference the *same* identity your sessions actually carry. DeliveryOS uses its own JWT (15m+refresh) + Fastify-enforced `location_id`, sidestepping this entirely.
- **P2 — `FOR INSERT WITH CHECK (TRUE)` on `orders`** (public can create any order, any `restaurant_id`, any totals). Open write surface; rely on server validation, not this.
- **P3 — `REPLICA IDENTITY FULL` on hot tables** for Realtime = write amplification + full-row CDC leakage of internal columns (`internal_notes`, `payment_transaction_id`). Reason DeliveryOS pushes curated events over own ws.
- **P4 — anon key drives all reads** → RLS is the *only* thing between anon and data; one missing/over-broad policy = tenant leak. A server tier (Fastify) gives defense-in-depth we should keep.
- **P5 — no Zod / runtime validation** on the Supabase boundary; JSONB blobs make this worse. Our shared strict Zod is the mitigation.

---

## R9 — Liftable (MIT-permitting) vs Rewrite

| Asset | Action | License note | DeliveryOS module touched |
|---|---|---|---|
| RLS policy *shapes* (public-by-flag / owner-by-subquery) | **adapt** (`restaurant_id`→`location_id`; fix to our JWT identity, not `auth.uid()`) | MIT — keep notice | `packages/db` migrations / RLS |
| Tenant-prefixed composite indexes + per-tenant uniques | **adopt** | MIT | `packages/db` |
| Per-state `*_at` order timestamps | **adopt** | MIT | `packages/db` orders schema |
| Order status enum (7 states) | **borrow as subset**, extend to 10 + add transition guard | MIT | order state machine |
| Price-breakdown columns (subtotal/tax/discount/total) | **adopt shape**, but integer minor units + server-computed | MIT | checkout (Fastify) |
| `restaurantService.ts` Supabase-Realtime subscription | **skip** (re-express on own ws) | MIT | `apps/api` ws / Redis |
| UI primitives (`Button` etc.) | **skip** — we have shadcn/ui + `var(--brand-*)` | MIT | `packages/ui` |
| Tailwind hardcoded-hex tokens | **skip** — regress from our runtime theming | MIT | `packages/ui` theme |
| JSONB items/sizes/addons | **skip / reject** — keep relational | — | menu + order schema |
| Supabase Auth client wiring | **skip** — we use own JWT 15m+refresh | — | `apps/api` auth |

**Net:** lift the **RLS multi-tenant patterns, index/constraint discipline, per-state timestamps, and price-breakdown column shape** (all MIT, all → `packages/db`); **reject** JSONB blobs, client-set totals, Supabase Realtime, hardcoded theming, and DECIMAL money. The twin is most valuable as a **validated RLS/data-model reference**, least valuable as runtime architecture (our Fastify+own-ws topology is intentionally heavier and safer).

---

## Citations
- Repo & metadata: https://github.com/roshanx0/restaurant-ordering-saas · GitHub API `/repos/roshanx0/restaurant-ordering-saas` (stars 21, forks 3, created/pushed 2025-12-07, updated 2026-06-12, MIT, TS 90.8%/PLpgSQL 6.9%).
- Schema/RLS/indexes/triggers/realtime: `database/setup.sql` (raw).
- Service/order/realtime: `src/services/restaurantService.ts`; auth/client: `src/config/supabase.ts`.
- UI/theming: `src/components/ui/Button.tsx`, `tailwind.config.js`.
- Topic search: https://github.com/topics/food-ordering-system?l=typescript ; runner-up `masterabdullah95/quickbite-eats`.
