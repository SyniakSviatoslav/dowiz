# MVP Sensor-Bus + Manual-Bridges + North-Star Seams — Hardened Design Proposal

> Architect deliverable for the Triadic Council. **Design-time only — no production code.**
> Binding input: `docs/design/mvp-sensor-seams/brief.md`. North Star runtime (autopilot P1–P7) is
> OUT of scope (brief §7): we design **seams**, not runtime. Every claim below is grounded in real
> source (file:line cited); assumptions that turned out false are corrected inline.
>
> Companion ADRs: `docs/adr/0007-stock-decrement-in-order-txn.md` (v4),
> `docs/adr/0008-bom-recipe-polymorphic-seam.md` (v3),
> `docs/adr/0009-sensor-bus-event-log-and-promised-window.md` (v4).
> **Hardened after THREE Breaker + Counsel rounds — see `docs/design/mvp-sensor-seams/resolution.md` for each
> finding's disposition (fix / accept-risk+owner / defer-flag / human-needed), incl. round-3 R3-C1/R3-H1/R3-H2/
> R3-M1.**
>
> **★ ROUND-3 STRATEGIC DE-SCOPE (Option B):** the stock decrement/restock **RUNTIME** is **DEFERRED** to a
> named follow-up ("Stock-runtime (decrement + restock) follow-up") after it leaked THREE rounds running
> (C1 → R2-C1 → R3-C1, each via a different context boundary). This batch ships ONLY the inert
> `products.stock_remaining` column-SEAM (NULL=unlimited, zero runtime, zero regression). The binary
> `is_available` toggle (already shipped) covers the §2.3 limited-special MVP need; the §4 per-unit DoS surface
> shrinks to that owner-only toggle (no per-unit burn). The idempotency `state` lifecycle (ADR-0007 v4 §4) is
> stock-independent and DOES ship now. See resolution.md "RESOLVE round 3" for the full A-vs-B reasoning.

---

## 0. What the codebase already has (grounding — corrects the brief)

The brief assumes several things are absent or "already laid". Verified against source:

| Brief assumption | Reality (file:line) | Consequence for this design |
|---|---|---|
| `order_status_history` "already laid" (§1.1, §6.1) | **TRUE.** `packages/db/migrations/1780338982015_order_history.ts:5-21` creates it; `1790000000059_order-tracking-timestamps.ts:19` adds `comment`. RLS ENABLE+FORCE + `tenant_isolation` via `app_member_location_ids()`. | Reuse it — but it is a **status-transition** log keyed on the `order_status` enum (`to_status order_status NOT NULL`). It **cannot** carry `courier_geofence_enter` (not a status). See §1.1 / ADR-0009. |
| Per-transition timestamps must be added | **ALREADY PRESENT** as columns on `orders`: `confirmed_at` (`1780310074262_orders.ts:42`), `ready_at`/`delivered_at` (`1780695000000_order_timelines.ts`), `preparing_at`/`in_delivery_at`/`picked_up_at` (`1790000000059_order-tracking-timestamps.ts:12-15`). Written by `apps/api/src/lib/orderStatusService.ts:89-117`. | §1.1 timestamps for status events are **DONE**. The only NEW timestamps are the non-status sensor events: `courier_geofence_enter`, optional `geofence_enter_customer`, `picked_up` (already have `picked_up_at`). |
| `order_status_history` is append-only | **By convention only** — no trigger blocks UPDATE/DELETE. Written best-effort inside a SAVEPOINT (`orderStatusService.ts:128-139`). | "Append-only" is currently a discipline, not a constraint. ADR-0009 decides whether to harden it with a trigger. |
| `products.stock_remaining` does not exist | **Confirmed absent** (`1780310072731_menu.ts:18-31` — products has no stock col). `prep_time_minutes` **already added** (`1790000000065_products-prep-time.ts:337`). | §6.1 stock col is genuinely new. §5 prep-time pre-existing — only the category-default *preset* is new. |
| money is integer | **Confirmed** — `orders.subtotal/total` (`1780310074262_orders.ts:32-33`), `products.price` (`1780310072731_menu.ts:25`), `delivery_trace.total` (`1790000000027_delivery-trace.ts:16`) all `integer`. | All new money/qty columns stay `integer`/`numeric`; never float for money. |
| velocity / no_show infra exists (§4) | **Mostly built.** `velocity_events` + `customer_signals` (advisory, NEVER auto-block) at `1780421100057_anti-fake-signals.ts`; live throttle at `orders.ts:250-269`; `customers.no_show_count` at `1780310074262_orders.ts:13`. | §4 is ~80% shipped. The only NEW piece is the **one-tap abort** owner action (§4.2) and surfacing no_show as a soft-gate (preflight already consumes it — `orders.ts:324-328`). |
| funnel_events / ingredients / recipe_components exist | **Confirmed absent** (grep over all migrations: zero hits). | All three are genuinely new. |
| a venue geofence (polygon/radius) exists | **Confirmed absent.** `locations` has `lat`/`lng` (`orders.ts:116`) but no radius/polygon. `courier_positions` streams pings (`1780421100042_courier-positions.ts`). ADR-GEO-SEAMS confirms ETA is advisory haversine, no router per ping. | §1.1 `courier_geofence_enter` must be derived from `locations.lat/lng` + a **radius**, haversine on the ping. New: `locations.geofence_radius_m`. See §1.1. |

**Canonical RLS idiom (must copy EXACTLY):** the hardened recent migrations
(`1790000000054_product-media-seam.ts:63-71`, `1780421100057_anti-fake-signals.ts:36-41`) use:
```sql
ALTER TABLE t ENABLE ROW LEVEL SECURITY;
ALTER TABLE t FORCE  ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON t
  USING      ( location_id IN (SELECT app_member_location_ids()) )
  WITH CHECK ( location_id IN (SELECT app_member_location_ids()) );
REVOKE ALL ON t FROM anon, authenticated, service_role;
-- GRANT DML to deliveryos_api_user (guarded) + mirror orders' grants.
```
`withTenant` sets `app.user_id` (`packages/platform/src/auth/tenant.ts:11`); `app_member_location_ids()`
derives the tenant set from it. The `current_setting('app.current_tenant')` idiom is **legacy/courier-only** —
do NOT use it for new owner-facing tables. **EXCEPTION (Breaker C2):** `order_sensor_events` is written from
the courier ping handler (which sets ONLY `app.current_tenant` — `shifts.ts:337`, never `app.user_id`) AND
read by owners (who set ONLY `app.user_id`). Its policy MUST be the **disjunction of both idioms** or every
courier-context geofence INSERT is RLS-denied and silently swallowed by the best-effort SAVEPOINT. See §3.1
`…071` + ADR-0009 v3 §1.

---

## 1. Back-of-envelope (cold-start reality check)

**Scale:** ~30 orders/day/location (brief §0.2). Treat a busy single tenant as the unit; horizontal
across tenants is already the topology (RLS-per-location). No sharding decision is triggered at this
volume — **boring monolith holds** (ADR-0001).

### 1a. Does each sensor give a meaningful day-1 answer with ZERO history?

| Sensor (§) | Day-1 answer with zero history? | Verdict |
|---|---|---|
| §1.1 timestamps + promised_window | YES — each order self-describes; no aggregate needed. The *fuel* for P1/P2/P7 accrues, but the **record** is meaningful immediately (this order took N min). | Ship. Pure capture. |
| §1.2 route_distance_m + expected_delivery_min | YES — `road_distance × speed_const` needs zero history; it's a per-delivery baseline, computed at `delivered_at`. (Speed const = a baked-in default, like prep priors §5.) | Ship. Heuristic-first. |
| §1.3 funnel_events | PARTIAL — a single event row is meaningful (this session abandoned at checkout with a 45-min ETA shown). The *counterfactual* ("lost because ETA too long") needs ~weeks of rows to be statistically real, but the **observability** starts day 1. | Ship capture; analysis is later (out of scope). |
| §1.4 eta_cap_min | YES — a hard cap is a config constant, not a learned value. Fires day 1. | Ship. |
| §2.1 prep countdown / dispatch nudge | YES — `prep_time_minutes` already on products (default 15) + `dispatch_margin_min` config. Manual-first. | Ship (mostly FE). |
| §2.3 stock_remaining | YES — NULL=unlimited means zero behavioral change until an owner sets a number. | Ship. NULL-safe. |
| §3.2 atomic decrement | YES — only active when stock_remaining is non-NULL. | Ship. |
| §6.2 ingredients/recipe_components | N/A — **inert seam**, no runtime, no day-1 answer expected (brief §6.2: "рантайм FLAT/manual"). | Ship schema only. |

**Conclusion:** every NON-seam sensor gives a meaningful day-1 answer. The seam tables (§6.2)
deliberately give none — that is the point (schema-now, runtime-later).

### 1b. Row volume / growth / retention

- **`funnel_events`**: heaviest writer. 4 event types × sessions. Estimate ~5–15 menu_views per order
  + add_to_cart/checkout funnel ≈ **~10–50 rows/order** → at 30 orders/day ≈ **300–1500 rows/day/location**.
  Across, say, 50 tenants ≈ 15k–75k rows/day → ~5–27M rows/year. **Needs retention.** Decision:
  **90-day retention sweep** (a cron DELETE on `created_at`, mirroring the `velocity_events` 24h sweep
  pattern at `1780421100057_anti-fake-signals.ts:62`). Index `(location_id, created_at DESC)` per §6.1.
- **`order_status_history`**: ~6–8 transitions/order → ~240/day/location → ~90k/year/location. Modest;
  this is an audit log → **keep ≥1 year** (it's the P1/P7 fuel; cheap). No sweep now; revisit at scale.
- **`delivery_trace`**: 1 row/delivered order (`UNIQUE order_id`) → ~30/day/location. Trivial. Keep.
- **`ingredients` / `recipe_components`**: inert; owner-authored; tens–hundreds of rows/location. Trivial.

### 1c. Stock-decrement latency — now at CONFIRM, not create (Breaker C1 re-architecture)

The decrement moved from order-create to the **CONFIRM transition** (ADR-0007 v3 — the lifecycle fix for C1:
a PENDING order reserves nothing, so timeout-CANCEL/owner-REJECT of an unconfirmed order leak zero units). It
is **1 guarded `UPDATE … RETURNING` per distinct product, ordered by `product_id`** (deadlock-free, Breaker
M4), folded into the CONFIRMED guarded UPDATE that already runs (`orderStatusService.ts:89-94`). Adding ≤N
(N = distinct products, ~1–8) single-statement round-trips (~1–5 ms each) rides the *same* txn and COMMIT as
the confirm → cannot create a half-decremented order. **The create hot path is now LIGHTER than v1** (no
decrement at create). The restock is now an **UNBYPASSABLE DB trigger** (`BEFORE UPDATE OF status` on
`orders` — ADR-0007 v3, Breaker R2-C1): it fires on ANY writer that flips a `stock_committed=true` order to
CANCELLED/REJECTED — including the **raw customer-cancel UPDATE** (`customer/orders.ts:289`) that bypassed the
v2 service-method restock — and flips `stock_committed=false` in the same row write (idempotent, no
double-restock). It is gated on `BEFORE UPDATE OF status` (a pure-timestamp/notes UPDATE never enters the body)
and short-circuits on `stock_committed=false` (PENDING / already-restocked) with no query, so it is
hot-table-safe. Detail + the no-leak lifecycle matrix + the raw-route anti-cheat-green test +
race/deadlock/claim-first(single-txn `state` lifecycle)/crash-recovery proofs in ADR-0007 v3.

**`promised_window` set-once trigger cost (Breaker H5):** the BEFORE-UPDATE trigger body is a few comparisons
with **no query / no I/O** → sub-microsecond/row; the timeout sweep's bulk `UPDATE … status='CANCELLED'`
(`order-timeout-sweep.ts:67`) never touches the frozen columns so the `IS DISTINCT FROM` is always false (fast
path). It is hot-table-safe; app writes cannot bypass it; a migration/superuser write intentionally can (the
one logged escape hatch — §7). Pinned as a DoD micro-assertion.

---

## 2. Load-bearing decisions — ≥2 named options each

### 2.1 §3.2 Atomic stock decrement (🔴 money/order red-line)

**Concept: status-guarded conditional UPDATE inside the existing order transaction** (the same anti-race
primitive already used for status transitions at `orderStatusService.ts:90-117`).

| Option | Mechanism | Race-correctness on last unit | Multi-item / NULL / replay | Verdict |
|---|---|---|---|---|
| **A — Conditional UPDATE … RETURNING** (CHOSEN) | `UPDATE products SET stock_remaining = stock_remaining - $qty WHERE id=$pid AND location_id=$loc AND (stock_remaining IS NULL OR stock_remaining >= $qty) RETURNING id, stock_remaining` per distinct product. 0 rows → reject the line → ROLLBACK whole order. | **Exactly one winner.** The `WHERE … >= qty` predicate is evaluated under a row write-lock taken by the UPDATE; the second concurrent txn re-reads the post-decrement value and matches 0 rows. No `SELECT FOR UPDATE` race window. | NULL row → predicate's `IS NULL` branch matches → decrement of NULL stays NULL (no-op on unlimited). Multi-item: any 0-row line → ROLLBACK → whole order fails atomically (no partial decrement survives, since all decrements share the one txn). Replay: idempotency key short-circuits at `orders.ts:369-379` **before** any decrement → a retried key returns the stored order and never re-runs the UPDATE. | **CHOSEN.** Matches the brief's exact SQL §3.2, the codebase's existing guarded-UPDATE idiom, single round-trip, no extra lock statement. |
| B — `SELECT … FOR UPDATE` then check then UPDATE | Lock row, read, branch in app, UPDATE. | Correct but **2 statements + app round-trip** under lock → longer lock hold inside the hot txn (worsens the pool-wedge failure mode the 4.5 s timeout guards `orders.ts:106-112`). | Same semantics but more code, more lock time. | Rejected — strictly worse latency/lock profile for identical correctness. |
| C — Separate guarded statement, then verify rowcount in a 2nd query | UPDATE unconditionally, then SELECT to check. | **Racy** — the unconditional UPDATE can drive stock negative before the check; or needs a CHECK(>=0) that aborts the whole txn with a constraint error (ugly error mapping). | Constraint-error path is brittle. | Rejected — reintroduces oversell window or opaque errors. |

**Decision: Option A, applied at CONFIRM (not create) — Breaker C1.** Per-distinct-product (aggregate
quantity across duplicate line-items, `ORDER BY product_id` for deadlock-free lock ordering — Breaker M4). The
decrement rides the **CONFIRMED transition** (`orderStatusService.ts:89-94`), NOT order-create: stock is a
kitchen-commitment resource and the commitment is the confirm, so a PENDING order reserves nothing and
timeout-CANCEL/owner-REJECT of an unconfirmed order leak **zero** units. The one residual leak path
(CONFIRMED→REJECTED/CANCELLED) is closed by a **flag-guarded idempotent restock** (`orders.stock_committed`).
A `CHECK (stock_remaining IS NULL OR stock_remaining >= 0)` is the belt-and-suspenders invariant. On 0 rows →
the confirm rolls back with `422 { code:'OUT_OF_STOCK', error:'Product <name> is out of stock' }` (humane
cause-hint — Counsel #4). Idempotency is **claim-first** (the `idempotency_keys` row is claimed via
`ON CONFLICT DO NOTHING` BEFORE any write — Breaker H1) so a concurrent same-key pair never double-creates and
gets a clean 200-replay (not a 500). See ADR-0007 v3 for the full statements, the no-leak lifecycle matrix, the
race/deadlock/claim-first proofs, and composition with every existing rollback path.

### 2.2 §6.2 `recipe_components.parent_id` — POLYMORPHIC vs alternatives (the seam the brief most fears)

The brief's load-bearing claim: *"Рецепти посилаються на ВУЗОЛ → апгрейд manual→derived
міграційно-вільний"* (§6.2). I must **prove it holds** or revise the seam so it does.

| Option | Shape | manual→derived migration-free? | Concurrency-correctness (shared ingredient) | Verdict |
|---|---|---|---|---|
| **A — Polymorphic parent** (`parent_kind 'product'\|'ingredient'`, `parent_id uuid`, NO real FK) (brief's proposal) | One `recipe_components` table; a row says "node X consumes qty of ingredient Y". A product's recipe and an intermediate's recipe live in the SAME table, distinguished by `parent_kind`. | **YES, provably.** Today (manual): a product's availability = `min(manual stock_remaining)`. Later (derived): availability = `min(stock_remaining, floor over recipe_components of ingredient stock / qty_per_parent)`. The *recipe rows never change* — only the **reader** changes (a new SQL function replaces the manual `min`). No row rewrite, no parent_id remap, no kind flip. The derivation is a READ over existing rows. | Shared ingredient = ONE `ingredients` row; the atomic guard (ADR-0007 pattern) is applied to that **one node** → no double-spend under concurrency. A flattened/denormalised recipe would copy the ingredient into N product rows → concurrent orders would each decrement a *copy* → oversell. Single-node is the concurrency-correct shape. | **CHOSEN.** Proven migration-free; concurrency-correct; matches brief. Cost: no DB-level FK on `parent_id` (polymorphic) → integrity is app-enforced + a partial CHECK. |
| B — Two tables: `product_recipe` (FK products) + `intermediate_recipe` (FK ingredients) | Clean real FKs both sides. | **NO — NOT migration-free for the reader's UNION,** but worse: the moment an intermediate is *also* sold as a product (a batch sauce sold retail), or a product becomes an ingredient of a combo, you must MOVE rows between tables → the exact ret-migration the brief fears. | Same single-node property achievable. | Rejected — the table boundary is the wrong cut; real-world nodes cross 'product'/'ingredient' freely. |
| C — Nullable double-FK (`product_id uuid NULL REFERENCES products`, `ingredient_id_parent uuid NULL REFERENCES ingredients`, CHECK exactly-one) | Real FKs, no polymorphism ambiguity. | **YES** — reader UNIONs on COALESCE; flipping kind = null one col, set the other (an UPDATE, but in-place, not a cross-table move). | Same. | **Viable runner-up.** Strictly safer integrity (two real FKs + CHECK) at the cost of a wider table + a 2-col CHECK. Rejected only because it complicates the "one node" mental model and the partial unique/index story; documented as the fallback if the polymorphic integrity proves fragile. |
| D — Single ingredients-only tree, products modelled as ingredients | One table, products are `ingredients` rows with `kind='sellable'`. | YES but **conflates the menu/commerce model with the BOM model** — products already carry price/translations/media/modifiers. Forcing products into `ingredients` duplicates or splits the product identity. | Same. | Rejected — pollutes the existing rich `products` table's identity; large blast radius on the menu reader. |

**Decision: Option A (polymorphic node), with Option C as the documented fallback.** The seam is
**inert** at MVP (FLAT/manual runtime, brief §6.2). Integrity for the missing FK on `parent_id` is held by:
(1) `parent_kind` CHECK constraint to the enum set; (2) an application-layer assertion on write; (3) the
inertness — no reader dereferences `parent_id` until the North-Star derivation lands, and that reader can
add the integrity join then. **The migration-free claim is PROVEN above** (the upgrade changes the
*reader*, never the *rows*). See ADR-0008 for the full DDL, the partial-index strategy, and the explicit
"how the future derived reader plugs in" walkthrough that demonstrates zero schema change.

### 2.3 §1.1 `promised_window` immutability — column-on-orders (set-once) vs event row

| Option | Mechanism | "Set once at confirm, never changes" enforcement | Verdict |
|---|---|---|---|
| **A — Two columns on `orders` + DB guard** (CHOSEN) | `orders.promised_window_lo_min int`, `promised_window_hi_min int`; written exactly once in the CONFIRMED branch of `orderStatusService.ts:89-94` (which already does `confirmed_at = now()`). | **DB-enforced set-once** via a BEFORE UPDATE trigger: `IF OLD.promised_window_lo_min IS NOT NULL AND NEW.promised_window_lo_min IS DISTINCT FROM OLD.promised_window_lo_min THEN RAISE`. App writes it in the same guarded UPDATE that flips to CONFIRMED. Historical truth is immutable at the DB, not just by convention. | **CHOSEN.** Co-located with the existing `confirmed_at` write; single read for the customer page; the trigger makes immutability a hard invariant (matches the brief's "історична істина, незмінна"). |
| B — A row in an event/audit table | A `promised_window` row in `order_status_history` or a new table. | Append-only (no UPDATE) gives immutability for free, BUT: (1) `order_status_history.to_status` is a status enum — can't carry a window; (2) a separate table = an extra join on every customer page load. | Rejected — wrong table shape + read cost; the window is a property of the order, not an event. |

**Decision: Option A — but SPLIT into a frozen promise + a mutable live estimate (Counsel ESTOP-1).** The
set-once trigger conflated two concepts; v2 separates them so §2.4's collapsing window no longer silently
fights §1.1's immutability on the same field:
- **`orders.promised_window_{lo,hi}_min`** — the **promise as made at confirm**, frozen by the set-once
  BEFORE-UPDATE trigger. Measurement ground truth for §8 (P1/P2/P7 falsification). Immutable.
- **`orders.live_eta_{lo,hi}_min`** — the **live current estimate** the customer sees collapse through stages
  (§2.4 confirmed→cooking→picked_up→arriving). **MUTABLE** — explicitly NOT covered by the trigger. This is
  the client truth channel: a mis-set first promise no longer freezes the customer into a lie, because the
  customer reads `live_eta_*` (the truth as it evolves) while §8 reads the frozen `promised_window_*`.
  **The WRITER is specified (Breaker R2-M1), not just the column:** `live_eta_*` is recomputed at each stage
  transition co-located with the existing `*_at` stamp in `orderStatusService.ts:106-116` (PREPARING→remaining
  prep+travel; READY→travel only; IN_DELIVERY/PICKED_UP→remaining travel off the latest ping; geofence→arriving
  band) via the **same synthesis helper**, so it inherits the `min_window_width_min` floor on EVERY recompute
  (not just the confirm synthesis — Counsel R2.1) and the `eta_cap` ceiling. The recompute is best-effort
  within the transition (a recompute failure degrades to the prior live band, never back to the frozen first
  promise). Without this writer `live_eta == promised_window` forever and ESTOP-1 would be cosmetic; with it
  the customer-truth channel is actually live. A **confirm-time OUT_OF_STOCK rejection rides this same live
  channel** with the product-name cause-hint (Counsel R2-a), not only a swallowable 422.

The append-only-window-log (Counsel's steel-man §4) is the **recorded North-Star upgrade** — its first row IS
the frozen column; the two-column split is the lower-blast-radius MVP cut delivering both ethics now. This
also satisfies §0.4 **range-never-point** at schema-shape (only `_lo`/`_hi`, no point column) **and** value-
level (`min_window_width_min` floor + DB `CHECK (hi >= lo + 1)` — Breaker L2 / Counsel #1). See §5 + ADR-0009 v3.

---

## 3. Data / migrations (forward-only, RLS FORCE, integer money)

All node-pg-migrate, forward-only (`down()` = no-op or column-drop only, mirroring the repo's discipline
— PII tables never DROP, per `access-requests.ts:93`). Numbering continues the `1790000000xxx` series
(highest current = `1790000000065`). Proposed (Council may renumber on landing):

### 3.1 NOW-runtime migrations (§6.1)

| # (proposed) | Migration | DDL summary | RLS |
|---|---|---|---|
| `…066` | `sensor-order-columns` | `ALTER orders ADD promised_window_lo_min int, promised_window_hi_min int` (frozen) **+ `live_eta_lo_min int, live_eta_hi_min int` (mutable — ESTOP-1)** + **set-once trigger on the frozen pair ONLY** + `CHECK (hi >= lo + 1)` on each pair (range-never-point value-level, L2). **NOTE (Option B): `stock_committed` flag + the `orders_restock_on_terminal` restock trigger move to the Stock-runtime FOLLOW-UP — NOT in this migration** (the restock runtime leaked R3-C1 under FORCE-RLS; it lands with its SECURITY-DEFINER fix + anti-cheat-green DoD). | orders already RLS FORCE — additive cols inherit |
| `…066b` | `idempotency-claim-state` | `ALTER idempotency_keys ADD state text NOT NULL DEFAULT 'completed' CHECK(state IN ('claimed','completed')), claimed_at timestamptz` (R2-H2 — claim-first single-txn lifecycle; legacy rows default 'completed') | idempotency_keys already RLS FORCE |
| `…067` | `sensor-product-stock` | `ALTER products ADD stock_remaining int` (NULL=unlimited) + `CHECK (stock_remaining IS NULL OR stock_remaining >= 0)`. **Ships INERT this batch (Option B): the column-SEAM only — no runtime reads/writes it.** Decrement/restock runtime = the named follow-up. | products already RLS FORCE |
| `…068` | `sensor-delivery-trace-baseline` | `ALTER delivery_trace ADD route_distance_m int, expected_delivery_min int` (written by the DELIVERED ON-CONFLICT upsert, L3) | delivery_trace already RLS FORCE |
| `…069` | `sensor-location-caps` | `ALTER locations ADD eta_cap_min int NOT NULL DEFAULT 90, dispatch_margin_min int NOT NULL DEFAULT 5, material_shift_min int NOT NULL DEFAULT 5, otp_target_pct int NOT NULL DEFAULT 90, geofence_radius_m int NOT NULL DEFAULT 150, **min_window_width_min int NOT NULL DEFAULT 5** (range floor, L2/Counsel #1)` | locations: not tenant-RLS (it IS the tenant); additive |
| `…070` | `sensor-funnel-events` | `CREATE TABLE funnel_events (id bigserial PK, location_id uuid NOT NULL REFERENCES locations, session_ref text, event_type text CHECK(event_type IN ('menu_view','add_to_cart','checkout_start','checkout_abandon')), shown_eta_lo_min int, shown_eta_hi_min int, created_at timestamptz DEFAULT now())` + `INDEX(location_id, created_at DESC)` + `INDEX(created_at)` (retention sweep) | **ENABLE+FORCE + tenant_isolation USING+WITH CHECK** (copy product_media exactly) + REVOKE anon/authenticated/service_role + GRANT deliveryos_api_user. Public ingest is **per-IP rate-limited** (H4). |
| `…071` | `sensor-geofence-event-log` | `CREATE TABLE order_sensor_events (id bigserial PK, order_id uuid NOT NULL REFERENCES orders ON DELETE CASCADE, location_id uuid NOT NULL, event_type text CHECK(event_type IN ('courier_geofence_enter','geofence_enter_customer')), payload jsonb DEFAULT '{}', created_at timestamptz DEFAULT now(), UNIQUE(order_id, event_type))` | **ENABLE+FORCE + DUAL-CONTEXT `tenant_isolation` (Breaker C2):** USING+WITH CHECK = `location_id IN (SELECT app_member_location_ids()) OR location_id = NULLIF(current_setting('app.current_tenant',true),'')::uuid` — the courier ping handler sets ONLY `app.current_tenant`; a member-only WITH CHECK denies every geofence INSERT. + REVOKE + GRANT (mirror courier_positions). |

| `…071b` | `courier-one-active-assignment` | `CREATE UNIQUE INDEX courier_one_active_assignment ON courier_assignments (courier_id) WHERE status IN ('accepted','picked_up')` (R3-H2 — make "one active assignment per courier" a DB invariant for MVP so the geofence `order_id` read is over an at-most-one set; the geofence read also gains a deterministic `ORDER BY picked_up_at NULLS LAST, accepted_at, order_id`). **HARD flag:** drop/relax this + rebind geofence per-assignment-by-proximity BEFORE the P3 `courier_sequence` batch seam activates. | courier_assignments already RLS (ENABLE) — additive index |

> **Why a NEW `order_sensor_events` table (ADR-0009) and not `order_status_history`:** the geofence/customer-arrival
> events are NOT status transitions; `order_status_history.to_status` is the `order_status` enum and cannot
> represent them without polluting the enum (which the state machine `assertTransition` enforces —
> `orderStatusService.ts:75`). A separate append-only sensor table is the correct shape. The `UNIQUE(order_id,
> event_type)` enforces "geofence_enter рівно раз" (brief §1.1 acceptance) at the DB, idempotently.

### 3.2 SEAM migrations (§6.2 — inert, runtime FLAT/manual)

| # (proposed) | Migration | DDL summary | RLS |
|---|---|---|---|
| `…072` | `bom-ingredients-seam` | `CREATE TABLE ingredients (id uuid PK, location_id uuid NOT NULL REFERENCES locations, name text NOT NULL, kind text NOT NULL DEFAULT 'raw' CHECK(kind IN ('raw','intermediate')), is_batch_made bool NOT NULL DEFAULT false, unit text, current_stock numeric, tracking_mode text NOT NULL DEFAULT 'untracked' CHECK(tracking_mode IN ('untracked','manual','derived')), waste_pct numeric NOT NULL DEFAULT 0, reset_cadence text, last_set_at timestamptz, created_at timestamptz DEFAULT now())` | **ENABLE+FORCE + tenant_isolation** + grants |
| `…073` | `bom-recipe-components-seam` | `CREATE TABLE recipe_components (id uuid PK, location_id uuid NOT NULL REFERENCES locations, parent_kind text NOT NULL CHECK(parent_kind IN ('product','ingredient')), parent_id uuid NOT NULL, ingredient_id uuid NOT NULL REFERENCES ingredients(id), qty_per_parent numeric NOT NULL, unit text, created_at timestamptz DEFAULT now())` + `INDEX(location_id, parent_kind, parent_id)` + `INDEX(ingredient_id)` **+ AFTER DELETE `FOR EACH ROW` triggers on `products`/`ingredients` (Breaker H3) + AFTER TRUNCATE `FOR EACH STATEMENT` triggers (Breaker R2-M2 — FOR EACH ROW does NOT fire on TRUNCATE) that delete matching `recipe_components` rows** | **ENABLE+FORCE + tenant_isolation** + grants |

`courier_sequence` (brief §6.2): grep shows it is **not yet a column on orders** — if the brief claims
"уже в orders", verify on landing; if absent, it is a trivial `ALTER orders ADD courier_sequence int` seam
(inert). *(Flagged for the conductor — see §10.)*

**Money/qty types:** `stock_remaining`/`*_min`/`route_distance_m`/`*_radius_m` = `int`; ingredient
`current_stock`/`qty_per_parent`/`waste_pct` = `numeric` (fractional units — 1.5 kg flour — are not money,
so numeric is correct; money stays integer everywhere).

---

## 4. Consistency · idempotency · failure · degradation · security

### 4.1 Idempotency

- **Order create — claim-first, single-txn `state` lifecycle (Breaker H1 + R2-H2)**: the `idempotency_keys`
  row is **claimed via `INSERT … ON CONFLICT (location_id, key) DO NOTHING RETURNING key` BEFORE any write**
  (not stored at `:655` near COMMIT as today), with `state='claimed'`; the claim, the order INSERT, and the
  `UPDATE … SET order_id, state='completed'` all commit in **ONE txn**. R2-H2 showed the txn placement was
  unpinned (same-txn vs separate-txn are mutually exclusive for the guarantee, and a separate-txn claim
  crash-poisons the key). v3 pins **single-txn**: the composite-PK unique index (`1790000000029:11`)
  serializes — a concurrent peer's `ON CONFLICT DO NOTHING` blocks on the index lock until the owner txn
  commits, then sees a `state='completed'` key with a valid `order_id` and **re-enters the existing replay
  SELECT** to return the **full order body 200-replay** (never a bare 200, never a 500 — R2-H2 part 3 closed).
  A crash leaves the key **recoverable** (single-txn rollback removes the claim; a guarded stale-`claimed`
  reclaim `DELETE … WHERE state='claimed' AND claimed_at < threshold RETURNING` lets exactly one retry win → no
  double-create — R2-H2 part 2 closed; no permanently-poisoned key — R2-H2 part 1 closed). Combined with the
  decrement moving to CONFIRM (C1), a double-tap can neither double-create nor double-decrement. Proven in
  ADR-0007 v3.
- **Geofence event**: `UNIQUE(order_id, event_type)` + `INSERT … ON CONFLICT (order_id, event_type) DO NOTHING`
  → idempotent "exactly once" even under GPS-jitter re-crossing (Breaker M3 — clause pinned, no 23505).
- **delivery_trace baseline (§1.2, Breaker L3)**: `delivery_trace` is `UNIQUE(order_id)` + `ON DELETE CASCADE`,
  written by the DELIVERED handler `ON CONFLICT DO NOTHING` (`1790000000027_delivery-trace.ts:5-6,12` —
  re-verified). The §1.2 baseline columns (`route_distance_m`, `expected_delivery_min`) are folded into that
  **same idempotent DELIVERED upsert** (`… ON CONFLICT (order_id) DO UPDATE SET …`), not a separate plain
  INSERT → a re-fired DELIVERED never double-writes or 23505s.

### 4.2 Failure + degradation — **sensors must NEVER fail the order** (brief §0.1 observe-don't-control)

| Sensor write | Failure handling |
|---|---|
| `funnel_events` INSERT (public funnel) | Fire-and-forget on a **separate** request path (NOT inside the order txn), **per-IP rate-limited** (Breaker H4 — uniform 200/204 preserved; over-cap events dropped server-side). Failure → log + drop. The funnel is observation; a lost funnel row never blocks a sale. |
| `order_sensor_events` geofence INSERT | Written in the IN_DELIVERY ping handler (`shifts.ts:336-378`, which sets `app.current_tenant`) as `INSERT … ON CONFLICT (order_id, event_type) DO NOTHING` (M3 pinned) **best-effort in a SAVEPOINT** (mirror `orderStatusService.ts:128-139`). The **dual-context RLS** (C2 fix, §3.1 `…071`) makes the WITH CHECK pass via `app.current_tenant` — so the SAVEPOINT now guards a *real* failure, not a guaranteed RLS denial. **The `order_id` is the courier's OWN active assignment (Breaker R2-H1):** it is read from `courier_assignments WHERE courier_id = <self>` (`shifts.ts:365-369`, extended to return `order_id`), NEVER from the ping payload — so a courier cannot forge a geofence on a colleague's order at the same venue (the RLS validates `location_id` only; order-ownership is enforced app-side where the assignment is already known). A failed sensor insert ROLLS BACK TO SAVEPOINT and the ping/position update still succeeds. **DoD asserts the geofence row IS present after a crossing** (C2 silent-loss) **AND that a forged colleague-order_id is impossible** (R2-H1 scope). |
| `promised_window` write at confirm | Part of the CONFIRMED guarded UPDATE (`orderStatusService.ts:90-94`). If it fails the confirm fails — but it's just two int columns on an UPDATE that already runs; effectively cannot fail independently. Acceptable to couple (it IS order state, not a sensor). |
| `delivery_trace` baseline (§1.2) | Already best-effort/idempotent at DELIVERED. A failed baseline never un-delivers the order. |
| **stock decrement** | The **ONE intentional exception** — it is NOT a sensor, it is order *correctness*. A 0-row result MUST reject the line (oversell prevention is the feature). This is by design and is the only sensor-adjacent write allowed to fail an order. |

**No cascade:** every new external/compute touch (geofence haversine, ETA baseline) is local CPU on data
already in-hand (ping lat/lng vs `locations.lat/lng`) — **zero new network calls**, so zero new circuit-breaker
surface. The geofence compute is `O(1)` haversine per ping (brief §1.2 explicitly forbids running a router).

### 4.3 Security · tenant isolation · PII

- **funnel_events PII + anti-flood/poison (Breaker H4, M2; Counsel #5)**: `session_ref` is an opaque client-
  minted session id (NOT a customer id, NOT a phone). **Unlinkability designed-in (M2)**: it is **never written
  onto an order row** and **never logged on the order-create path** (grep-gate: no `session_ref` in
  `orders.ts`), and the FE **rotates** it at order submission so the pre-order funnel session and the order are
  not the same token. RLS FORCE + REVOKE anon/authenticated/service_role keeps it off the Supabase Data API.
  **Anti-enumeration**: the public ingest returns a **uniform 200/204** regardless of validity (mirror
  `access-requests.ts`). **Anti-flood (H4)**: the ingest is **per-IP rate-limited** (reuse the existing rate-
  limiter; ~60 events/min/IP — generous for a real session, lethal to a flood) and the 90-day retention sweep
  DELETEs in **bounded batches** (`LIMIT` loop) so it can't lock-contend with live writes. **Anti-poison (H4)**:
  the §8.2 padding-creep counter-metric is computed over **distinct `session_ref`** (one session ≈ one vote)
  and is **advisory** input to a human/loop, never a direct autopilot actuator — a single-IP flood can't steer
  the brake. Funnel analytics are disclosed in the `/compliance` SoT + storefront privacy notice (Counsel #5).
- **RLS on every new table**: funnel_events, ingredients, recipe_components get ENABLE+FORCE +
  `tenant_isolation` USING+WITH CHECK via `app_member_location_ids()` + REVOKE + GRANT, **in the same
  migration** that creates the table (brief §6 / repo discipline). **`order_sensor_events` is the ONE
  exception (Breaker C2)**: it is written from the courier ping context (which sets only `app.current_tenant`)
  AND read by owners (who set only `app.user_id`), so its policy is the **disjunction of both tenant idioms**
  (`app_member_location_ids()` OR `app.current_tenant`), still ENABLE+FORCE — see §3.1 `…071`. A member-only
  WITH CHECK would deny every geofence INSERT and the SAVEPOINT would swallow it silently.
- **§4 no-OTP surface (Breaker M5)**: the live throttle is **phone-only** today (`if (phoneHash)`
  `orders.ts:250-261`); `clientIpHash` is computed at `:247` but never gates. v2 **adds the IP half** — a
  parallel `if (clientIpHash)` velocity gate mirroring the phone block, using the already-computed hash — so a
  phone-rotating attacker is bounded by IP (the brief's "velocity-ліміти (phone+IP)" finally made whole). And
  critically, with **decrement-at-CONFIRM (C1)** a flood of never-confirmed PENDING orders decrements **zero**
  stock, so the create-time DoS-on-availability is neutralised at the lifecycle level; the throttle now bounds
  PENDING-spam (dashboard noise + abort load), not stock. no_show stays advisory in preflight
  (`orders.ts:324-328`); `customer_signals` NEVER auto-blocks (`anti-fake-signals.ts:104`). The NEW piece is
  the **one-tap owner abort** — a human action (brief §0.6), owner-authenticated, via the existing guarded
  state machine. No auto-cancel, no auto-penalty.
- **§2.1 dispatch nudge (Counsel #3)**: courier-facing **advisory** only; non-compliance is **NOT** recorded
  as an owner-visible compliance signal — "courier owns the moment" true in lived experience, not just code.

---

## 5. Invariants as enforceable contracts

| Invariant (brief §) | Where enforced (not just documented) |
|---|---|
| **range-never-point — both bounds** (§0.4) | Schema-level: `orders` has ONLY `_lo_min`/`_hi_min` for BOTH the frozen `promised_window_*` and the live `live_eta_*` — no point column. **Value-level (Breaker L2 / Counsel #1):** `locations.min_window_width_min` (DEFAULT 5) is the **floor** — synthesis enforces `hi := max(hi, lo + min_window_width_min)` after the `eta_cap` ceiling clamp; DB `CHECK (hi >= lo + 1)` rejects a literal point; the Zod response schema rejects `lo == hi`. Honest below (no "1–2 min") + useful above (eta_cap) = the complete contract. |
| **promise frozen vs live-truth split** (§1.1 / §2.4, ESTOP-1) | The customer page reads the **mutable `live_eta_*`** (truth as it collapses through stages); the §8 metric reads the **frozen `promised_window_*`** (set-once trigger). §2.4 honesty no longer fights §1.1 immutability — they are different columns. Append-only window-log = recorded North-Star upgrade. |
| **agents-declare-privately** (§3.1) | The kitchen prep estimate (`prep_time_minutes`, owner-set) and courier timing are **internal** inputs; the customer page reads ONLY the synthesised window/live-eta off the order — never the raw per-agent numbers. Enforced by the read path: the public order-status endpoint selects the window columns, not prep_time. |
| **human-decides-consequential** (§0.6) | No auto-cancel/refund/penalty exists or is added. `customer_signals` is COMMENT-documented "NEVER used for auto-block" (`anti-fake-signals.ts:104`). The §4.2 abort is an owner tap. The §2.1 dispatch nudge is courier-advisory, NOT an owner compliance signal (Counsel #3). The future courier rating is normalized + advisory, never auto-deactivation (Counsel #2, North-Star). |
| **observe-don't-control** (§0.1) | Every sensor write is best-effort/non-blocking (§4.2 table) EXCEPT the stock decrement (at CONFIRM), which is a *shared-resource* control (the one place control is licensed — brief §0.1). Geofence/funnel/baseline are pure observation and cannot fail the order. |
| **set-once promised_window** (§1.1) | DB BEFORE UPDATE trigger raising on any change to the **frozen pair** after first write (ADR-0009 v3). App writes cannot bypass; a migration/superuser write intentionally can (the one logged correction escape hatch — H5/§7). `live_eta_*` is NOT covered (mutable by design, recomputed per stage — R2-M1). |
| **geofence exactly once, OWN order** (§1.1) | `UNIQUE(order_id, event_type)` on `order_sensor_events` + `ON CONFLICT (order_id, event_type) DO NOTHING` (M3 pinned). Writes land via the **dual-context RLS** (C2); the `order_id` is the courier's **own** active assignment, read server-side, never payload-supplied (R2-H1) — a courier cannot stamp a colleague's order. |
| **no oversell + no leak** (§3.2) | The conditional UPDATE predicate (at CONFIRM) + `CHECK(>=0)` + the no-leak lifecycle matrix + **UNBYPASSABLE DB-trigger restock** (ADR-0007 v3 — fires on any status writer incl. the raw customer-cancel, R2-C1); race+lifecycle+raw-route+claim-first+crash-recovery+deadlock tests assert exactly-one success and zero leak on any terminal path. |
| **bias-free reconstruction** (§8.1) | Durations computed only over both-endpoints-non-NULL orders, segmented by fulfilment type, dwell conditional on a geofence row, each AVG reporting `n` (Breaker M1 — ADR-0009 §4b). **Late-within-band customer-cost metric named NOW (collection only; centering decision deferred — Counsel R2.3):** `late_within_band_rate = delivered_at > promised_window_hi / live_eta_hi`, derivable from laid columns, so the customer's cost reaches autopilot-design as a peer of the funnel's venue-cost — not one-wired-one-hypothetical. |
| **eta_cap absolute** (§1.4) | The synthesis path clamps the window to `locations.eta_cap_min`; hitting the cap raises an owner signal (not silent). Enforced in the window-compute helper + an owner alert. |

---

## 6. Operability

- **Event-log growth**: funnel_events 90-day cron sweep (new), **DELETEing in bounded batches** (`LIMIT` loop)
  so it can't lock-contend with live writes (Breaker H4); the public ingest is **per-IP rate-limited** so an
  attacker can't drive the table size. order_sensor_events + order_status_history kept (cheap). Add the sweep
  predicate index `funnel_events(created_at)`.
- **Geofence compute cost**: `O(1)` haversine per IN_DELIVERY ping, on data already loaded. Negligible; no
  router (brief §1.2). No new provider → no new breaker (ADR-GEO-SEAMS posture preserved).
- **Health**: no new external dependency → the existing degraded-vs-down health truth is unchanged. The
  seams (§6.2) are inert → invisible to health.
- **Rollback / flag**: the inert §6.2 seams need no flag (no reader). The §1.3 funnel ingest + §1.1 geofence
  capture should land behind a cheap `SENSORS_ENABLED`-style boolean (default ON is safe since they're
  non-blocking, but a kill-switch lets us silence a misbehaving sensor without a deploy). Stock decrement
  (§3.2) is gated implicitly by `stock_remaining IS NULL` (no owner number → no behavior).
- **Migrations**: forward-only; each runs on staging DB FIRST (boot-guard FATAL-exits otherwise — repo Ship
  Discipline). The set-once trigger and the new tables are idempotent (guarded — copy product_media's
  `IF NOT EXISTS` / `DROP POLICY IF EXISTS` style) so a retried release_command is safe.

---

## 7. Open / accepted risks

| Risk | Class | Owner | Disposition |
|---|---|---|---|
| Polymorphic `recipe_components.parent_id` has no declared DB FK | accepted (seam inert) | Architect | **Mitigated** (Breaker H3): an AFTER DELETE trigger on products/ingredients cascades the orphan rows (the delete-side guard v1 lacked); the integrity join lands with the derived reader; Option C (double-nullable-FK) is the documented exit. |
| Introducing a NEW intermediate/batch node later needs a recipe re-point | accepted, documented | Architect → North-Star lead | **Honest re-scope** (Breaker H2): the manual→derived **reader swap is migration-free [proven]**; introducing an intermediate is a deliberate, owner-driven data backfill through the BOM UI — NOT a hidden ret-migration. Documented in ADR-0008 v3. |
| Ingredient cycle enterable now; recursive reader has no cycle guard yet | **defer-flag (MISSING)** | North-Star lead | L1: the future `available_units()` reader MUST carry depth-cap + visited-set memo AND validate **pre-existing** rows (data can be authored before the guard). Recorded MISSING in ADR-0008 v3 until the reader lands. |
| funnel ingest flood + counter-metric poisoning | accepted w/ mitigation; residual | Ops | **Mitigated** (Breaker H4): per-IP rate-limit + batched 90-day sweep + per-session-distinct advisory signal (human-review before actuation). Residual: a distributed botnet at scale — non-load-bearing at cold-start; revisit (PoW / signed session_ref) only if the funnel ever directly actuates the loop. |
| `session_ref` time-correlation to an order's customer | accepted | Ops | M2: session_ref never on orders + rotated at submit + disclosed in `/compliance`/privacy notice; residual timing-correlation needs DB-admin access to both tables and there is no identity column to join to. |
| Set-once trigger privileged-write bypass asymmetry | accepted | Architect | H5: app writes cannot bypass; a migration/superuser write intentionally can — that is the one logged correction escape hatch for a mis-set frozen promise (ESTOP-1 (b)). Stated, not oversold as an unqualified hard invariant. |
| `courier_sequence` "already in orders" claim unverified (grep: absent) | **needs confirmation** | Architect/Conductor | Verify on landing; trivial additive seam if absent. |
| Geofence radius default (150 m) is a guess | accepted | Owner-tunable | `locations.geofence_radius_m` is owner-configurable; 150 m is a sane urban default (cold-start heuristic). |
| Customer shown a frozen-wrong promise with no repair path | **RESOLVED — split adopted** (ESTOP-1) | Architect | The frozen `promised_window_*` (metric) and the mutable `live_eta_*` (customer truth channel) are now different columns; the customer always sees the current truth. No human-needed disposition remains for the schema. |
| Where inside the honest band the promise sits; no customer-side cost signal vs the owner's OTP knob | **human-needed (recorded open question)** | Product + North-Star lead | Counsel §5: the owner holds the conservativeness knob; the funnel measures only the venue's lost-cart cost, never the customer's late-within-band cost. NOT a seam blocker — carry to autopilot-design time + add a customer-side cost signal (late-within-band rate) before the loops self-reinforce. |
| §4 OTP deferred → worst case 1 wasted portion | **Closed by de-scope** (Option B) | Owner | With NO per-unit decrement runtime this batch, an order flood cannot burn availability (nothing decrements `stock_remaining`); the only availability lever is the owner-only `is_available` toggle. Worst case = PENDING noise, bounded by the phone+IP velocity throttle (M5). The per-unit DoS surface re-enters only with the Stock-runtime follow-up. |
| **Stock decrement/restock RUNTIME (the area that leaked C1→R2-C1→R3-C1)** | **DEFERRED to a named follow-up** (Option B) | Architect → Stock-runtime follow-up lead | Three rounds, three context-boundary leaks (lifecycle-after-COMMIT → raw writers → FORCE-RLS firing-context). De-scoped from this batch; ships ONLY the inert `stock_remaining` column-seam. The runtime lands in a focused follow-up with the R3-C1 SECURITY-DEFINER restock fix + race+leak proof against the REAL empty-context handler. See resolution.md round 3. |
| R3-C1: restock trigger `UPDATE products` RLS-denied in the customer-cancel empty context (FORCE-RLS) | **DEFERRED WITH THE RUNTIME** (Option B); fix pre-designed | Stock-runtime follow-up lead | A SECURITY INVOKER trigger restocks 0 rows under `products` FORCE-RLS when `app.user_id` is unset (verified `customer/orders.ts:255-319` sets neither tenant idiom). Pre-designed fix: `SECURITY DEFINER` restock fn, tenant scope derived from the ORDER ROW (`NEW.location_id`, no caller input — abuse-safe), anti-cheat-green DoD vs the real handler (assert the row MOVED, no member-context harness, FORCE-RLS active). ADR-0007 v4 §3. |
| R3-H1: `order_items.product_id ON DELETE SET NULL` drops a deleted-product restock line | **DEFERRED WITH THE RUNTIME**; accept-risk in follow-up | Stock-runtime follow-up lead | A hard-deleted product has no `stock_remaining` counter to restock into → the line drop is unobservable; multi-item orders restock the live lines correctly. Snapshot rejected as over-engineering. ADR-0007 v4 §3. |
| R3-H2: geofence `order_id` non-deterministic / wrong order for a batched (multi-active-assignment) courier | **RESOLVED — singular + deterministic** (R3-H2) | Architect | `LIMIT 1`-no-ORDER-BY over a courier who can hold N active assignments (`courier_assignments` had only `UNIQUE(order_id)`). v4: `courier_one_active_assignment` partial-unique (one active per courier, MVP) + deterministic `ORDER BY`. HARD flag: per-assignment-by-proximity rebind MUST precede P3 `courier_sequence`. ADR-0009 v4 §2a. Owner of the P3 flag: North-Star/batch lead. |
| R3-M1: `live_eta` width-floor after the cap pushes `hi` above `eta_cap` | **RESOLVED — cap last** (R3-M1) | Architect | v3 floor-after-cap let `hi` exceed `eta_cap` (`lo=92→hi=97>90`). v4: clamp `lo := min(lo, eta_cap − width)` first, floor, then `hi := min(hi, eta_cap)` LAST → cap is absolute, floor still honored, no inversion. ADR-0009 v4 §3a/§4. |
| Intra-tenant geofence forgery (courier stamps a colleague's order_id) | **RESOLVED — order-assignment scope** (R2-H1) | Architect | The C2 dual-context RLS validates `location_id` only. v3 derives the geofence `order_id` from the courier's OWN `courier_assignments` row (`shifts.ts:365-369`), never from the ping payload → a courier can only stamp the order they are delivering. ADR-0009 v3 §2a. |
| Claim-first idempotency crash-poison / replay-body / unpinned txn | **RESOLVED — single-txn `state` lifecycle** (R2-H2) | Architect | v2 left the claim's txn placement unpinned (mutually exclusive guarantees) and a separate-txn claim could crash-poison the key. v3 pins single-txn with a `state {claimed→completed}` column: crash → recoverable (guarded stale-claim reclaim, no double-create), replay → full order body. ADR-0007 v3 §4. |
| `live_eta_*` schema-only (no writer) → live==frozen, ESTOP-1 cosmetic | **RESOLVED — writer specified** (R2-M1) | Architect | v2 named a mutable column but no writer. v3 specifies the per-stage recompute (co-located with the existing `*_at` stamp) via the same synthesis helper, with the `min_window_width_min` floor on EVERY recompute (Counsel R2.1) — the customer-truth channel is actually live. ADR-0009 v3 §3a. |
| AFTER-DELETE FOR-EACH-ROW trigger misses TRUNCATE → orphan recipe rows | **RESOLVED — statement-level companion** (R2-M2) | Architect | `FOR EACH ROW` DELETE triggers do not fire on TRUNCATE. v3 adds AFTER TRUNCATE `FOR EACH STATEMENT` triggers; soft-delete-without-purge is by-design (restorable; future reader filters live state). FK-equivalence claim narrowed to DELETE+TRUNCATE paths, not over-claimed. ADR-0008 v3. |

---

## 8. Sequence (per brief §10 priority)

1. §6 seam migrations (highest irreversibility — `…072/073`, incl. the H3 cascade triggers) + §1.1 frozen/live
   columns + set-once trigger + `order_sensor_events` with **dual-context RLS** + geofence determinism
   (`…066/071/071b`, incl. the `courier_one_active_assignment` partial-unique, R3-H2). 2. §1.2 baseline +
   §1.3 funnel (rate-limited) + §1.4 cap + width-floor with **cap-last clamp** (`…068/069/070`, R3-M1) — fix
   measurement bias before dirty data accrues. 3. **`…067` ships the INERT `stock_remaining` column-seam only
   (Option B)** + claim-first idempotency `state` lifecycle (`…066b`, stock-independent) red→green. **The stock
   decrement/restock RUNTIME is a SEPARATE named follow-up** ("Stock-runtime (decrement + restock)"), gated on
   ADR-0007 v4's race+lifecycle+raw-route-restock-anti-cheat-green (R3-C1, SECURITY-DEFINER) tests red→green. 4.
   §2 manual bridges (mostly FE + the §4.2 abort endpoint + the IP velocity gate). 5. §3/§4/§5 in parallel (prep
   priors preset, abort, no_show soft-gate surfacing).

**DoD-gate (brief §63) — for THIS batch (Option B):** every order has a full timestamp trail + frozen
`promised_window` + a mutable `live_eta` **the customer reads, recomputed per stage with the width-floor**
(R2-M1) **and a cap that is truly absolute (clamp `lo` first → floor → cap last, R3-M1)**; every delivery has a
normalised baseline (bias-free reconstruction contract) **and the late-within-band customer-cost metric is
derivable** (Counsel R2.3); funnel logged + rate-limited + capped; **claim-first idempotency (single-txn `state`)
+ crash-recovery** (R2-H2) green; **geofence-PRESENCE test** (C2 silent-loss) **+ order-assignment-scope test**
(R2-H1) **+ singular-active-assignment partial-unique + deterministic geofence-binding test** (R3-H2);
**orphan-TRUNCATE test** (R2-M2); range-never-point at value-level (width floor + CHECK + absolute cap, on
confirm AND live recompute); the inert `stock_remaining` column exists with **zero runtime regression**; zero
auto-decisions/loops/recursive-BOM. The seam tables exist with FLAT runtime and **zero regression at NULL/inert**
(the kill criterion: an order with all sensors off behaves byte-identically to today).

**DoD-gate for the SEPARATE Stock-runtime follow-up (NOT this batch):** decrement-at-confirm with green
race + **no-leak-on-any-terminal-path** + **raw-route-restock-ANTI-CHEAT-GREEN against the REAL empty-context
`/orders/:orderId/cancel` handler under FORCE-RLS, asserting the row VALUE moved** (R2-C1/R3-C1, SECURITY-DEFINER
restock) + deadlock tests. This gate is the follow-up's, recorded here so the runtime cannot ship without it.
