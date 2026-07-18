# BLUEPRINT P62 — Catalog & multi-vendor data model (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map §10). Component:
> **CATALOG / MULTI-VENDOR DATA MODEL**. Wave **W1** of the launch-blocker build sequence
> (`SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §5, row P62). Scope is fixed by that synthesis:
> §5's P62 row, the merchant-of-record ruling (§0.2-1), and — the load-bearing item — the
> **catalog leaf invariant X7** (§2/X7), which this blueprint *owns* as a single-owner contract.
> Structural template: `BLUEPRINT-P51-open-map-routing.md` (numbering mirrored). Research base:
> `OPUS-R5-MULTIVENDOR-ECOSYSTEM-OPS-2026-07-18.md` §3 (food-court prior art) and the operator
> rulings in `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §16.15/§16.16/§16.17/§16.46/
> §16.49. **Nothing here re-litigates a closed decision** — single-hub-DB row-scoping, free-form
> vendor catalogs, vendor-as-own-MoR, and shared courier pool all stand exactly as ruled; this
> document makes them buildable and binds the one invariant three downstream systems must share.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Working tree on `main`, 2026-07-18. All fresh reads. The single most load-bearing finding:
**the kernel already has the money authority, a trusted price catalog, a cart machine, and an
order aggregate — P62 EXTENDS them with two missing axes (currency-on-leaf, vendor_id) and does
NOT rebuild any of them.** The catalog leaf invariant is a *type upgrade to existing code*, not a
green-field module.

| Claim | Fresh `file:line` (this pass) | Status |
|---|---|---|
| Money authority EXISTS: `Money { minor: i64, currency: Currency }` — integer minor units, currency-tagged so cross-currency add is a **caught error, not silent unit confusion** | `kernel/src/money.rs:58-62` (struct), `:28-54` (`Currency` = All/Eur/Usd + `code`/`from_code`) | **VERIFIED — P62's leaf price IS this type, never a bare i64** |
| `Money::checked_add`/`checked_sub`/`checked_neg` are cross-currency fail-closed + overflow fail-closed (`Err`, never wrap/panic) | `kernel/src/money.rs:71-121` | **VERIFIED — the resolve-fold reuses this; cross-currency safety is FREE** |
| `compute_line_total(product_price, modifier_prices, quantity)` (checked, overflow→`Err`) and `apply_tax(subtotal, rate, incl)` (half-up, negative/overflow→`Err`) | `kernel/src/money.rs:307-320`, `:270-300` | VERIFIED — line/tax math is the single authority; P62 reuses, does not re-derive |
| P07 double-entry ledger + reversal conservation (`ledger_append`/`reversed_leg`/`ledger_sum`, Σ==0 on compensated order) | `kernel/src/money.rs:124-263` | VERIFIED — refunds against a multi-vendor order (P72) fold through this, unchanged |
| Trusted price catalog EXISTS but is **currency-blind AND vendor-blind**: `PriceEntry { base: i64, modifiers: BTreeMap<String,i64> }`, `PriceCatalog` keyed by `product_id: String`, `unit_price(product_id, modifier_ids) -> Result<i64>` fail-closed on unknown product | `kernel/src/catalog.rs:20-26`, `:29-32`, `:61-74` | **VERIFIED — THIS is the exact X7 gap: `i64` not `Money`, no `vendor_id`, flat key not a tree. P62 extends this module.** |
| Cart machine EXISTS: `CartLine { product_id, options, qty }`, `Cart::price<F: Fn(&str)->i64>` (overflow-safe subtotal), `Cart::reconcile<F: Fn(&str)->Option<i64>>` **drops delisted + re-prices survivors** | `kernel/src/cart.rs:11-17`, `:90-112`, `:117-129` | **VERIFIED — the unified cross-vendor cart is this same `Cart`; reconcile is the self-heal leg (§5.4)** |
| `format_money(minor, decimals, symbol)` — integer divmod, **no float**, `€0.01` never `€0.00` | `kernel/src/cart.rs:134-152` | **VERIFIED — the minor→decimal discipline the JSON-LD `Offer.price` string reuses (§4.5)** |
| Order aggregate EXISTS: `OrderItem { product_id: String, modifier_ids: Vec<String>, quantity: i64, unit_price: i64 }` — **no `vendor_id`, no per-line currency** | `kernel/src/domain.rs:30-35` | **VERIFIED — the one additive struct change: OrderItem gains `vendor_id` + `currency` (§3, §4.4)** |
| `place_order_priced(...)` **re-derives every line's `unit_price` from the trusted `PriceCatalog`, ignoring the caller value** → `price_trusted = true`; `place_order` legacy path → `price_trusted = false` | `kernel/src/domain.rs:198-240`, `:156-187`, flag `:55-59` | **VERIFIED — P62 extends this: `vendor_id` is ALSO catalog-authoritative (a client cannot forge which vendor's leg an item belongs to — §4.4 security property)** |
| Order status FSM: `OrderStatus` = Pending/Confirmed/Preparing/Ready/InDelivery/Delivered/Rejected/Cancelled/Scheduled/PickedUp/Refunding/CompensatedRefund; `assert_transition` legality table | `kernel/src/order_machine.rs:8-22`, transition guard imported at `domain.rs:25` | VERIFIED — KDS routing keys off item `vendor_id`, not off status; the FSM is untouched |
| Settlement is an **event append, idempotent by `order_id`** (ONE fold per order), amount-mismatch → `AmountMismatch` (never silently adjusts); every amount `i64` minor units | `kernel/src/ports/payment.rs:15`, `:81-90`, `:104-130`, `:367-390` (`decide_settlement`) | **VERIFIED — P62 DERIVES the per-vendor charge legs; P60/P72 EXECUTE them through this port (§2 boundary)** |
| Payment port trait + Wave-0 cash adapter | `kernel/src/ports/payment.rs:313` (`PaymentPort`), `:336` (`CashOnDeliveryPort`) | VERIFIED — the leg execution surface P60 owns; P62 feeds it, never calls it |
| `catalog`/`cart`/`money`/`domain`/`order_machine` are pure-`std`, registered in lib.rs; **default kernel build pulls NO serde/sqlx** | `kernel/src/lib.rs:35,38,178,185` (mods); serde only under `json-api`/`wasm`, sqlx only under `pgrust` | VERIFIED — the pure half of P62 builds NOW with zero server tier |
| JSON boundary is feature-gated: `json_api` is `#![cfg(feature = "json-api")]`, serde only here; `wasm` enables it as a superset | `kernel/src/json_api.rs:1-21`, `place_order_logic` `:154`, `apply_event_logic` `:203` | **VERIFIED — the schema.org JSON-LD projection lives HERE (feature-gated), never in the serde-free default kernel (§4.5)** |
| RLS mechanism EXISTS as a **council-gated proposal**, not invented here: `deploy/pgrust.toml` `rls = { cross_tenant = "deny" }` (the app-level tenant gate; "must never be flipped to allow in production"); the W13 `PgStore` sqlx boundary (`Handle`+`block_on`, explicit `migrate()`) is the adapter pattern | `deploy/pgrust.toml:20-25`; `kernel/src/retrieval/memory_store.rs:125-208` (`PgStore`); `kernel/Cargo.toml:43,82` (`pgrust = ["dep:sqlx","dep:tokio"]`, sqlx 0.8 cached) | **VERIFIED — P62 reuses this, adds no new RLS mechanism (§4.6)** |
| The tenant boundary is **`location_id`** (the hub), enforced NOBYPASSRLS + `FORCE ROW LEVEL SECURITY` + session GUC `app.member_location_ids`, **deny-on-unset**; `order_items` today has `order_id/product_id/price_snapshot/quantity` and **no `vendor_id`** | `BLUEPRINT-P-NATIVE-PGRUST-TENANT-REBUILD.md` §1-§2 (design), §2.2 (`order_items` columns); `docs/red-team/2026-07-13/D2-rls-data-governance.md` §2/R3 (fail-open tables) | **VERIFIED — `location_id` = the red-line cross-hub boundary (reused verbatim); `vendor_id` = a NEW intra-hub partition, NOT a second tenant boundary (§4.6)** |
| Food-court model is operator-ruled: **one hub, multiple vendors, one shared courier pool** (§16.15); **fully vendor-defined catalog, no fixed dowiz schema** (§16.17); **unified cart across vendors, one delivery, split required** (§16.46); vendor sets own tax rate in the free-form schema, dowiz calculates nothing tax-related (§16.49) | `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md:1985-1993`, `:2005-2013`, `:2301-2308`, `:2344-2345` | VERIFIED — the binding rulings; §16.15 explicitly names "the in-hub data model needs a vendor-scoping layer … not yet designed" — **P62 is that named gap** |
| Merchant-of-record ruling (post-research, CLOSED): **each vendor is their own MoR** (separate-charges-and-transfers mechanics); "dowiz never becomes a party to the money"; `settlement_split` → **per-vendor charge-leg derived from `order_item.vendor_id`**; N-leg auth-then-capture atomicity is P60/P72 | `SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md:29-41` (§0.2-1) | **VERIFIED — P62 owns the DERIVATION `order_item.vendor_id → ChargeLeg`; P60/P72 own the EXECUTION** |
| X7 invariant is a **single-owner contract**: "every purchasable leaf carries a resolvable price (integer minor units), currency, and `vendor_id`"; three consumers force it — unified cart (R5 §3.4), per-vendor charge leg (§0.2-1), schema.org JSON-LD (R1 §7, the AEO substrate; llms.txt is a forward-looking extra); "P62 owns the invariant; P60, P69, and the static-file pack consume it" | `SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md:190-198` (X7), `:432` (P62 row: "owns the leaf invariant") | **VERIFIED — this document is that ownership, made a type** |

Ground truth is non-discussible; everything below builds on this table only.

---

## 1. Design verdicts — the load-bearing decisions, argued not asserted

### 1.1 Row-scoped single hub DB, NOT schema-per-vendor (R5 §3.4, binding)

The operator ruling (§16.46) is a **cross-vendor** aggregate: one `order` spanning `order_item`s
with different `vendor_id`s, one delivery, one checkout. R5 §3.4's research reaches the
falsifiable verdict that schema-per-vendor **fights** this design: "the unified cart is a
cross-vendor aggregate … schema-per-vendor would make the single most important query (the cart)
a cross-schema join, fighting the model instead of expressing it," and "the courier pool is
explicitly shared (§16.15) — it belongs to the hub, not any vendor." The food-hall POS prior art
(GoTab/Tabski/Chowbus, R5 §3.2) is unanimous: **shared browse, single checkout, per-item kitchen
routing, tenant data-isolation as a filter not a physical boundary.** Therefore:

- **The isolation boundary between hubs is `location_id`** — the existing red-line RLS-FORCE
  boundary (`deploy/pgrust.toml:20`, pgrust-tenant blueprint §1). Reused verbatim; **not**
  re-invented.
- **`vendor_id` is a row-scoping COLUMN within one hub**, not a schema/DB boundary. A food-court
  hub is *one* trust and *one* fulfillment domain (R5 §3.4); its vendors are catalog partitions,
  not isolated tenants. `vendor_id` gives (a) per-vendor kitchen routing and (b) settlement-leg
  grouping — operational scoping and money attribution, **not** a cross-tenant security boundary
  on par with `location_id`. This distinction is stated as an invariant in §4.6 and tested.

### 1.2 The leaf invariant is a TYPE, and the "free-form" is above it (§16.17 ↔ X7)

§16.17 rules the catalog fully vendor-defined: no fixed dowiz taxonomy, vendors author their own
categories/modifiers/variants, viable for "flowers or goods" as well as food. The synthesis X7
adds the one non-negotiable floor: **every purchasable leaf carries a resolvable price (integer
minor units), a currency, and a `vendor_id`.** These are not in tension — they are two layers:

- **Above the leaf: a free-form adjacency-list tree.** `label` is vendor-authored text; there is
  no dowiz enum for "category". A node is a `Group` (non-purchasable) or a `Leaf` (purchasable).
- **At the leaf: the invariant, enforced by construction.** `NodeBody::Leaf(PriceableLeaf)` — the
  *only* purchasable node variant carries a `PriceableLeaf`, which *cannot exist* without
  `(Money, VendorId)`. An unpriced/uncurrencied/unattributed purchasable leaf is
  **unrepresentable**, not merely rejected at runtime (§5.1 hazard-safety, §9 Hermetic P1). This
  is the single most important design decision in the blueprint: the invariant is a type the
  compiler enforces, so the three consumers *cannot* each re-derive it differently.

R5 §7 (riskiest unknowns) named this exact seam — "the boundary between 'free schema' and 'enough
structure to check out' is undesigned" — and P62 draws it precisely at `PriceableLeaf`.

### 1.3 Reuse-first: extend `catalog.rs`, do NOT build a new module (standard item 19)

The existing `PriceCatalog` (`catalog.rs:29`) is *already* the "SINGLE authority on what a line
item COSTS" — its module doc is a money red-line. It has exactly two missing axes: **currency**
(it stores bare `i64`, not `Money`) and **`vendor_id`** (it is single-vendor by omission). P62's
whole schema change is: give each priced leaf a `Money` (which `money.rs:58` already defines) and
a `VendorId`. Building a parallel "multi-vendor catalog" module beside the trusted one would fork
the money authority — the exact anti-pattern item 19 forbids. **Verdict: extend `catalog.rs` and
`domain.rs`; add one small `vendor.rs` for the `VendorId` newtype; add the JSON-LD projection to
the already-serde-gated `json_api.rs`.** No new crate, no second money surface.

### 1.4 Single-vendor is N=1 of the same model, provably (binding scope)

The common single-vendor hub is **not** a separate simpler path — it is `N=1` of the multi-vendor
model with zero special-casing. Every P62 primitive is total over `1..N` vendors: `charge_legs` is
`group_by(vendor_id)`, whose output on a single-key input is a one-entry map; KDS routing is the
same group-by; the RLS predicate is identical (one `vendor_id` value in the location). There is no
`if n_vendors == 1` branch anywhere, and §6's DoD makes that a **machine-checked** gate (a source
audit that FAILS if such a branch is introduced — §4.7). This is the falsifiable form of "reduces
cleanly to single-vendor."

---

## 2. Scope — what P62 owns vs deliberately does NOT

**P62 owns (build items §4):**

| Item | Content |
|---|---|
| M1 | `kernel/src/vendor.rs` — `VendorId` newtype (the intra-hub partition identity) |
| M2 | **The X7 leaf invariant**: extend `catalog.rs` with `PriceableLeaf` (price:`Money` + `vendor_id` + kind + availability), the free-form `CatalogNode` tree, `PriceComponent`, and the `resolve_line` fold (cross-currency + cross-vendor + overflow fail-closed) |
| M3 | Free-form catalog tree ops: build/validate the vendor-authored adjacency tree (no dowiz taxonomy; the ONLY structural law is "a `Leaf` carries a `PriceableLeaf`") |
| M4 | `order_item.vendor_id` fan-out: extend `OrderItem` (+`vendor_id`, +`currency`), make `vendor_id` **catalog-authoritative** in `place_order_priced`; `charge_legs` (per-vendor settlement legs) + `kitchen_tickets` (per-vendor KDS routing) |
| M5 | schema.org **`Menu`/`MenuItem`/`Offer` JSON-LD** projection from catalog state (feature-gated), + the one integer minor→decimal price-string fn — the bot-facing AEO substrate consumed by P69's static pack |
| M6 | **RLS design** (not migration): the `vendor_id` column on `catalog_node`/`order_items`, and the two-layer policy predicate (outer `location_id` red-line boundary + inner opt-in `vendor_id` narrowing) — lands *through the pgrust council gate*, reusing the existing FORCE-RLS mechanism |
| M7 | The **N=1 no-special-path** guarantee as a machine-checked gate (property test + source audit) |

**P62 explicitly does NOT own:**

- **NOT payment execution / N-leg atomicity.** `charge_legs` *derives* per-vendor legs; **P60**
  owns the `PaymentProvider` port, idempotency contract, and the auth-all-then-capture atomicity
  (§0.2-1); **P72** owns the food-court N-leg checkout UX and partial-failure/void semantics. P62
  hands them a `Vec<ChargeLeg>` and never calls a provider. A diff that puts a `capture()` in
  `catalog.rs`/`domain.rs` is a scope violation regardless of test state.
- **NOT the merchant-of-record decision.** §0.2-1 is CLOSED (each vendor is own-MoR). P62 encodes
  its *consequence* (leg = grouped `order_item.vendor_id`); it does not re-open which entity is MoR.
- **NOT the storefront / checkout UI, nor emitting the static SEO pack.** **P69** renders the
  unified cart and *emits* the JSON-LD file; P62 supplies the projection function it calls.
- **NOT the owner menu-editor UI.** **P70** builds the vendor's catalog-authoring surface; P62
  supplies the tree types + validation it edits.
- **NOT executing the RLS migration.** The DDL (`vendor_id` column + `FORCE RLS` + predicate)
  lands **only** through the council-gated `BLUEPRINT-P-NATIVE-PGRUST-TENANT-REBUILD.md` flow
  (auth/money/RLS/migrations are red-line), never by default build. P62 designs the predicate;
  the pgrust blueprint owns its application. The pure-kernel half (types, resolver, fan-out,
  JSON-LD) builds NOW with no server tier.
- **NOT courier matching / dispatch / the shared pool's internals.** The shared `courier_pool` is
  hub-level (§16.15); the matcher (`bebop2 matcher`) and dispatch (**P65**) already own it. P62
  only records that the pool is hub-scoped, never vendor-scoped (one delivery serves the whole
  food-court order — §16.46).
- **NOT tax computation.** §16.49: the vendor sets their own rate inside the free-form schema;
  dowiz "calculates and tracks nothing tax-related." P62 carries the vendor's rate as opaque
  vendor-authored data and passes it to the existing `apply_tax`; it defines no tax policy.
- **NOT cross-currency food-court carts.** One order = one currency in Wave-0; a cart mixing
  vendors in different currencies is the §4-D operator-scoped market question (P72), refused
  fail-closed here (`Money` cross-currency guard), never silently converted.

---

## 3. Predefined types & constants (standard item 4 — named BEFORE implementation)

```rust
// ── kernel/src/vendor.rs — NEW module (the intra-hub vendor partition) ───────
/// Stable identity of a vendor WITHIN one hub (one `location_id`). A hub hosts
/// 1..N vendors (§16.15): N=1 is the common single-vendor case, N>1 is food-court.
/// `u64` (maps to SQL `BIGINT`) — a cheap group-by key for KDS fan-out + settlement
/// legs; NOT a `String` (product_id is String — a leaf; a vendor is the coarser axis).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VendorId(pub u64);

// ── kernel/src/catalog.rs — EXTEND (reuse-first item 19; NOT a new module) ───
/// Vendor-authored leaf id, free-form within its vendor scope (§16.17, no dowiz enum).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LeafId(pub String);
/// Vendor-authored tree-node id (free-form).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub String);

/// Routing / JSON-LD hint ONLY — imposes NO taxonomy on the tree (§16.17).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeafKind { Item, Variant, Modifier }

/// Vendor-controlled orderability. A SoldOut/Scheduled leaf is still PRICED (X7 holds)
/// but is not orderable — availability never nulls the invariant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability { Available, SoldOut, Scheduled }

/// ── X7 — THE CATALOG LEAF INVARIANT ──────────────────────────────────────────
/// The ONE type every purchasable leaf resolves to. Non-negotiable floor under
/// §16.17's free-form tree: the categories/modifiers/variants ABOVE a leaf are
/// vendor-authored and arbitrary; a LEAF is ALWAYS (price, currency, vendor). The
/// `price: Money` field carries BOTH minor-units AND `Currency` (money.rs:58); the
/// `vendor_id` is the single fan-out key. Constructed ONLY via `new` — an unpriced /
/// uncurrencied / unattributed purchasable leaf is UNREPRESENTABLE by type, not merely
/// rejected at runtime. Consumed VERBATIM by three systems (P69 cart, P60/P72 charge
/// legs, P69's schema.org pack); NONE may redefine it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceableLeaf {
    pub leaf_id: LeafId,
    pub vendor_id: VendorId,
    /// RESOLVED absolute price of one unit (base + chosen components folded at resolve time),
    /// currency-tagged. NEVER a bare i64, NEVER a float (money red-line, money.rs).
    pub price: crate::money::Money,
    pub kind: LeafKind,
    pub availability: Availability,
}
impl PriceableLeaf {
    /// The ONLY constructor. Total + refusing: price MAY be 0 (a free add-on) but NEVER
    /// negative (`assert_non_negative`, money.rs:323). Returns a typed `CatalogError`.
    pub fn new(leaf_id: LeafId, vendor_id: VendorId, price: crate::money::Money,
               kind: LeafKind, availability: Availability) -> Result<Self, CatalogError>;
}

/// A variant/modifier price contribution (§16.17 free-form). Absolute REPLACES the base
/// (e.g. size "Large" = 700); Delta ADDS to it (e.g. "extra cheese" = +150 — the existing
/// `PriceEntry.modifiers` surcharge model, catalog.rs:24, now currency-tagged).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriceComponent { Absolute(crate::money::Money), Delta(crate::money::Money) }

/// Free-form vendor-authored catalog tree node (§16.17). Adjacency list (parent ptr).
/// dowiz imposes NO structure on `label`/children; the ONLY law is: a `Leaf` carries a
/// `PriceableLeaf` (X7 by construction — a `Group` is not purchasable and has no price).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogNode {
    pub node_id: NodeId,
    pub vendor_id: VendorId,          // every node is vendor-scoped
    pub parent: Option<NodeId>,       // adjacency list; a root = None
    pub label: String,                // free-form, vendor-authored, NO dowiz enum
    pub body: NodeBody,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeBody { Group, Leaf(PriceableLeaf) }

/// Typed refusals — every failure names itself (never a partial tree / None-as-success).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogError {
    NegativePrice, CrossCurrency, CrossVendor, Overflow,
    UnknownLeaf(LeafId), LeafHasChildren(NodeId), CycleInTree(NodeId), DanglingParent(NodeId),
}

/// Fold a base leaf + chosen components into ONE resolved unit `Money`. The load-bearing
/// reuse: `Money::checked_add` (money.rs:71) makes cross-CURRENCY fail closed for FREE;
/// a component from a DIFFERENT vendor is refused (`CrossVendor`); overflow → `Overflow`.
/// This is where X7 becomes enforced, not merely declared.
pub fn resolve_line(base: &PriceableLeaf,
                    components: &[(VendorId, PriceComponent)]) -> Result<crate::money::Money, CatalogError>;

/// Validate a vendor's free-form tree: no cycle, no dangling parent, every `Leaf` is a
/// tree-leaf (a purchasable node with children is `LeafHasChildren`). Structure only —
/// NO taxonomy check (§16.17). Returns the vendor's `PriceableLeaf`s in deterministic
/// `LeafId` order (for reproducible JSON-LD + benches).
pub fn validate_tree(nodes: &[CatalogNode], vendor: VendorId) -> Result<Vec<PriceableLeaf>, CatalogError>;

// ── kernel/src/domain.rs — EXTEND: OrderItem gains the fan-out axes ──────────
// OrderItem gains:  pub vendor_id: crate::vendor::VendorId   // the SINGLE fan-out key
//                   pub currency:  crate::money::Currency     // per-line currency (leg grouping)
// (unit_price: i64 kept — the price snapshot; currency makes each line self-describing.)

/// One vendor's charge leg under the merchant-of-record ruling (§0.2-1): each vendor is
/// their OWN merchant-of-record, so each leg authorizes/captures against THAT vendor's own
/// provider account. DERIVED here (P62); EXECUTED by P60/P72. Deterministic `VendorId` order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChargeLeg {
    pub vendor_id: crate::vendor::VendorId,
    pub amount: crate::money::Money,   // Σ(unit_price × qty) for this vendor's items, currency-tagged
    pub line_count: usize,
}
/// Fan an order's items into per-vendor charge legs: `group_by(vendor_id)`, sum via
/// `Money::checked_add`. N=1 (single-vendor hub) → exactly ONE leg by the SAME code path
/// as N=30 — there is NO single-vendor branch (a group-by of a 1-key input is a 1-entry map).
pub fn charge_legs(order: &Order) -> Result<Vec<ChargeLeg>, String>;
/// KDS routing: the same `group_by(vendor_id)`, yielding each vendor's line items.
/// One food-court order fans to N kitchen views; a single-vendor order fans to one.
pub fn kitchen_tickets(order: &Order) -> std::collections::BTreeMap<crate::vendor::VendorId, Vec<&OrderItem>>;

// ── kernel/src/json_api.rs (feature = "json-api") — schema.org projection ────
/// Project a vendor's validated catalog into schema.org `Menu`/`MenuItem`/`Offer` JSON-LD —
/// the bot-facing AEO substrate (R1 §7). `Offer.price` = `price_to_decimal_string`, NEVER a
/// float; `Offer.priceCurrency` = `Currency::code()`. Consumed by P69's static SEO pack;
/// P62 supplies the STRING, P69 writes the FILE.
pub fn menu_jsonld(vendor: crate::vendor::VendorId, leaves: &[crate::catalog::PriceableLeaf]) -> String;
/// The ONE minor-units → schema.org decimal string. Integer divmod (reuses the
/// `cart::format_money` discipline, cart.rs:134 — no float); 2 places for ALL/EUR/USD.
pub fn price_to_decimal_string(m: crate::money::Money) -> String;
```

**RLS predicate (SQL — DESIGN only; lands through the pgrust council gate, §4.6):**

```sql
-- catalog_node, order_items each gain:  vendor_id BIGINT NOT NULL
-- (order_items' additive column mirrors the pgrust blueprint's "customer_devices gets
--  location_id" additive R5 fix — same convention.)
-- OUTER boundary (cross-hub, RED-LINE): location_id FORCE RLS, deny-on-unset (EXISTING).
-- INNER filter (intra-hub vendor scope): OPT-IN narrowing, NOT a second tenant boundary.
CREATE POLICY catalog_node_scope ON catalog_node USING (
  location_id = ANY (current_setting('app.member_location_ids')::bigint[])      -- outer, ALWAYS
  AND ( current_setting('app.vendor_scope', /* missing_ok */ true) IS NULL      -- hub-wide read
        OR vendor_id = current_setting('app.vendor_scope')::bigint )            -- vendor/KDS-scoped read
);
```

Rejected alternatives (DECART one-liners): **schema-per-vendor / DB-per-vendor** — rejected: the
unified cart becomes a cross-schema join (R5 §3.4); the shared courier pool has no home.
**A new `multi_vendor_catalog.rs` module** — rejected: forks the trusted-price money authority
(`catalog.rs`); item 19 demands extension. **Bare-i64 leaf price + a separate currency field** —
rejected: `Money` already binds them fail-closed (money.rs:58); splitting them re-opens the M5 gap
the money module exists to close. **`vendor_id` as a second FORCE-RLS tenant boundary** — rejected:
a food-court hub is one trust domain (R5 §3.4); vendor scope is opt-in narrowing, not isolation
(§4.6). **`f64` price in JSON-LD** — rejected: `price_to_decimal_string` integer divmod is the
`cart::format_money` discipline; a float on money is the red-line.

---

## 4. Build items — spec → RED test → code, each with adversarial cases (items 3, 5)

Dependency order: M1 → M2 → M3 → M4 → M5 → M6 → M7. M1–M5 + M7 are buildable NOW with zero
network / zero server tier (`catalog`/`cart`/`money`/`domain` are pure-`std`, §0). M6 is DESIGN in
Wave-0; its DDL executes only through the pgrust council gate.

### 4.1 M1 — `VendorId` (the intra-hub partition)

New `kernel/src/vendor.rs` per §3; register `pub mod vendor;` in `lib.rs` (alphabetical, near
`verify_retrieval`). `VendorId(pub u64)`, `Copy + Ord + Hash` (group-by key). RED→GREEN:
`vendor_id_ordered_stable` — a `BTreeMap<VendorId,_>` iterates in ascending id order (the
determinism `charge_legs`/`kitchen_tickets` rely on). **Adversarial:** `VendorId(0)` and
`VendorId(u64::MAX)` both usable as keys (no reserved sentinel — a sentinel would be a hidden
special case; §1.4).

### 4.2 M2 — the X7 leaf invariant (extend `catalog.rs`)

Add `PriceableLeaf` / `PriceComponent` / `CatalogNode` / `NodeBody` / `CatalogError` /
`resolve_line` per §3. `PriceableLeaf::new` rejects a negative price (`assert_non_negative`,
money.rs:323 → `NegativePrice`). `resolve_line` folds base + components via `Money::checked_add`.
RED→GREEN: `resolve_line_absolute_and_delta` — base 500 ALL + Delta(+150 ALL) = 650 ALL; an
`Absolute(700 ALL)` component overrides to 700 ALL. **Adversarial (designed to break the
invariant):** (i) a component in `Currency::Eur` on an `ALL` base ⇒ `CrossCurrency` (the test that
FAILS if someone unwraps the `Money` guard — teeth: it must be `Err`, not a coerced sum);
(ii) a component tagged `VendorId(B)` on a `VendorId(A)` leaf ⇒ `CrossVendor` (a client must not
splice another vendor's modifier onto this line); (iii) `Money::new(i64::MAX, ALL)` base +
`Delta(1)` ⇒ `Overflow` (never wraps); (iv) `PriceableLeaf::new(.., Money::new(-1, ALL), ..)` ⇒
`NegativePrice`; (v) a `Money::new(0, ALL)` price is ACCEPTED (a free add-on is valid — the
invariant is "resolvable + non-negative", not "positive").

### 4.3 M3 — free-form tree build + validate (§16.17, no taxonomy)

`validate_tree(nodes, vendor)` per §3: walk the adjacency list; reject a cycle (`CycleInTree`), a
dangling parent (`DanglingParent`), or a `Leaf` that has children (`LeafHasChildren` — a
purchasable node is a tree leaf). **No `label` check** — dowiz imposes no taxonomy (§16.17); a
vendor may name a category "🌮" or "Bouquets" freely. Returns the vendor's `PriceableLeaf`s in
`LeafId` order. RED→GREEN: `validate_tree_free_form_ok` — a 3-level vendor tree
(Group→Group→Leaf) with arbitrary labels validates and yields its leaves; a "flowers" vendor tree
(non-food labels) validates identically (the §16.17 "any small business" claim, made falsifiable).
**Adversarial:** (i) a node whose `parent` points to a missing `NodeId` ⇒ `DanglingParent`;
(ii) a 3-node cycle ⇒ `CycleInTree` (never infinite-loops — bounded walk); (iii) a `Leaf` node
that is also some other node's `parent` ⇒ `LeafHasChildren` (a priced thing cannot be a category);
(iv) two vendors' nodes in one slice ⇒ `validate_tree(_, A)` returns ONLY A's leaves (vendor
scoping at the pure layer, before any RLS).

### 4.4 M4 — `order_item.vendor_id` fan-out (extend `domain.rs`)

Add `vendor_id: VendorId` + `currency: Currency` to `OrderItem` (`domain.rs:30`). Extend
`place_order_priced` (`domain.rs:198`): re-derive `vendor_id` AND `currency` from the trusted
catalog leaf, **exactly as `unit_price` is re-derived today** — a `price_trusted` order is also
**vendor-trusted**. **Security property (state it, test it):** a client cannot forge which
vendor's charge leg an item belongs to — misattributing an item to another vendor's leg (a real
money-misrouting attack under separate-charges-and-transfers) is impossible when `vendor_id` comes
from the catalog, not the request. Add `charge_legs(order)` and `kitchen_tickets(order)` per §3,
both `group_by(vendor_id)`. RED→GREEN: `charge_legs_three_vendors` — an order with items from
vendors 1/2/3 yields exactly 3 legs, each summing its own items via `Money::checked_add`, in
ascending `VendorId` order; the leg totals sum to the order subtotal. **Adversarial:** (i) two
items same vendor ⇒ ONE leg with the summed amount (not two legs); (ii) an order whose items carry
mixed currencies within one vendor ⇒ `charge_legs` returns `Err` (cross-currency fail-closed — the
§4-D flag, never a silent conversion); (iii) an item with `unit_price × qty` overflowing i64 ⇒
`Err`, no wrap; (iv) `kitchen_tickets` for a food-court order routes each line to exactly its
vendor's view, no line duplicated or dropped (Σ line_counts == item count).

### 4.5 M5 — schema.org `Menu`/`MenuItem`/`Offer` JSON-LD (extend `json_api.rs`)

Add `menu_jsonld` + `price_to_decimal_string` per §3, under `#[cfg(feature = "json-api")]` (the
default kernel stays serde-free, §0). `price_to_decimal_string` is integer divmod (reuse the
`cart::format_money` discipline, cart.rs:134 — **no float ever touches a monetary value**). Each
`PriceableLeaf` → an `Offer { price, priceCurrency }`; each vendor's leaves → a `Menu` with
`MenuItem`s. RED→GREEN: `offer_price_string_integer_exact` — `Money::new(1, ALL)` → `"0.01"`,
`Money::new(1250, EUR)` → `"12.50"`, `Money::new(0, USD)` → `"0.00"` (the `€0.01`-never-`€0.00`
guarantee, applied to the bot surface); `menu_jsonld_valid_shape` — the output parses as JSON with
`@type: "Menu"` → `hasMenuItem[].offers.price` present for every leaf. **Adversarial:** (i) a leaf
priced `Money::new(i64::MAX, ALL)` renders its exact integer, never `+Inf`/scientific notation
(the float failure mode a naive `as f64` would produce — the test that FAILS if someone floats it);
(ii) a currency with a different minor-unit exponent is handled by `Currency` (ALL/EUR/USD are all
2-place Wave-0; the exponent is read from `Currency`, not hardcoded, so a 0-place currency added
later needs no JSON-LD change); (iii) a `SoldOut` leaf still emits an `Offer` with
`availability: "SoldOut"` (schema.org) — crawlers see the item, priced, marked unavailable (X7
holds through availability).

### 4.6 M6 — RLS design (two layers; lands through the pgrust council gate)

**This is DESIGN, not a migration.** The DDL executes ONLY through
`BLUEPRINT-P-NATIVE-PGRUST-TENANT-REBUILD.md`'s council-gated flow (RLS/migrations are red-line);
P62 supplies the column + predicate that blueprint applies. Two layers, reusing the existing
FORCE-RLS mechanism verbatim:

- **Outer (cross-hub, RED-LINE, unchanged):** `catalog_node` and `order_items` carry `location_id`
  (order_items via its `orders` FK-chain, per the pgrust blueprint §2.2 pattern);
  `FORCE ROW LEVEL SECURITY`; predicate `location_id = ANY(current_setting('app.member_location_ids')::bigint[])`;
  the connecting role is **NOBYPASSRLS**; **deny-on-unset** (an unset session context matches NO
  row — the D2 R3 fail-open lesson). This is the reused boundary; P62 adds nothing to it.
- **Inner (intra-hub vendor scope, NEW, opt-in):** both tables gain `vendor_id BIGINT NOT NULL`.
  The predicate ANDs an **opt-in narrowing**: `current_setting('app.vendor_scope', true) IS NULL
  OR vendor_id = current_setting('app.vendor_scope')::bigint`. Vendor/KDS connections SET
  `app.vendor_scope` → see only their own rows (the GoTab "tenants see only their own" pattern);
  hub-wide roles (customer storefront, owner, settlement) leave it UNSET → read across all vendors
  in the location (the unified cross-vendor cart is naturally expressible — the whole reason
  §1.1 rejects schema-per-vendor).

**Critical distinction (state + test):** the *unset vendor scope* WIDENS within the
already-`location_id`-scoped set (still one hub — safe), and is **NOT** the D2 R3 fail-open (which
was the OUTER boundary matching all rows across ALL hubs). The outer boundary is always
deny-on-unset; the inner filter is deliberately widen-on-unset. RED→GREEN (as `#[ignore]`d
DB-gated tests in the pgrust adapter, W13 pattern — GREEN when the server tier reactivates):
`vendor_scope_narrows_within_location` — vendor A's KDS connection sees only A's `catalog_node`
rows in its location; `hub_wide_reads_all_vendors` — an unset-vendor-scope connection sees all
vendors' leaves (unified cart); `unset_location_denies_all` — the OUTER deny-on-unset still fires
(a bare NOBYPASSRLS role with no `app.member_location_ids` sees 0 rows — the pgrust §4.4 probe,
inherited). **Adversarial:** a vendor A connection that sets `app.vendor_scope = B` still cannot
escape its `location_id` (the AND means outer boundary dominates) — cross-hub is unreachable
regardless of the inner filter.

### 4.7 M7 — the N=1 no-special-path guarantee (machine-checked)

Two teeth, so "single-vendor is N=1, zero special-casing" is falsifiable, not prose:

1. **Property test `single_vendor_is_degenerate_multivendor`:** run a single-vendor fixture
   (all items `VendorId(1)`) through the SAME `charge_legs`/`kitchen_tickets`/`validate_tree`
   functions the food-court path uses; assert `charge_legs` yields exactly ONE leg whose amount
   equals the whole-order subtotal, and `kitchen_tickets` yields one vendor view holding every
   item. Then a 3-vendor fixture through the identical functions yields 3 legs — **same call
   sites, no branch.**
2. **Source-audit test `no_single_vendor_special_path`** (the "smart index" gate, item 14; mirrors
   P51 §4.7's "grep the tool's input surface" and the repo's kernel-fence guards): a test that
   reads the source of `catalog.rs`/`cart.rs`/`domain.rs`/`vendor.rs` and FAILS if it finds any
   single-vendor special-case token — regex over `n_vendors\s*==\s*1`, `single.?vendor`,
   `is_food_court`, `vendors\.len\(\)\s*==\s*1`, `if.*only.*vendor`. Introducing a special path
   turns this RED. This is the task-mandated falsifiable test.

RED→GREEN: both absent before M4 lands; green after, and `no_single_vendor_special_path` stays a
permanent regression row (§6) so the guarantee cannot silently rot.

---

## 5. Cross-cutting design obligations (items 6, 8, 9, 11–16)

### 5.1 Hazard-safety as math (item 6)

Reachability arguments, not prose. **An unpriced purchasable leaf is unrepresentable:** the only
purchasable `NodeBody` variant is `Leaf(PriceableLeaf)`, and `PriceableLeaf` has no constructor
that omits `(Money, VendorId)` — the type system makes "a checkout-able thing with no
price/currency/vendor" a state that cannot be built (§9 Hermetic P1). **Cross-currency money
confusion is unreachable:** every fold goes through `Money::checked_add` (money.rs:71), which
fail-closes on currency mismatch — there is no code path from two differently-denominated leaves to
a summed integer. **A misrouted charge leg is unreachable from the client:** `vendor_id` on
`order_item` is re-derived from the trusted catalog (§4.4), so a request cannot reassign an item to
another vendor's money leg. **Cross-hub leakage is unreachable through the inner filter:** the RLS
predicate ANDs the outer `location_id` boundary (§4.6), which the inner `vendor_scope` cannot relax
(AND, not OR). **Money conservation is untouched:** refunds against a multi-vendor order fold
through the existing P07 ledger (`ledger_sum` → 0, money.rs:230), per-leg; P62 adds no new money
movement, only attribution.

### 5.2 Schemas & scaling axes (item 8)

- **`CatalogNode` tree:** axis = nodes / vendor. A menu is 10¹–10³ nodes; `validate_tree` is O(n)
  with an O(n) cycle walk. Break point: a single vendor > ~10⁴ nodes ⇒ lazy subtree load /
  pagination (not a Wave-0 concern; a menu is not a catalog-of-millions).
- **`VendorId` fan-out:** axis = vendors / hub. A food-court is realistically 2–30 vendors;
  `charge_legs`/`kitchen_tickets` are O(items) group-by with an O(vendors) map. No break in sight.
- **`charge_legs`:** axis = items / order. O(items). A 300-item order (30 vendors × 10 lines) is
  microseconds (§7 bench).
- **RLS session array:** axis = `location_id`s in `app.member_location_ids` (an owner's multi-hub
  reach, §16.18). `= ANY(array)` is fine to ~10³ hubs per owner; beyond that, a temp-table join
  (named future, not Wave-0).

### 5.3 Isolation (item 11), mesh awareness (item 12), living memory (item 15)

- **Isolation:** the catalog is **hub-local state** (§16.6 hub isolation). A vendor's bad tree
  cannot corrupt another vendor's — `validate_tree` is per-vendor and pure; the money authority is
  a shared *read-only* primitive (`money.rs`), not shared mutable state. The renderer/JSON-LD is a
  state consumer, never a mutator (P38 §4.3 bulkhead, inherited).
- **Mesh:** catalog is **NOT gossiped** — it is hub-local authored state served over P37's HTTP
  surface (same posture as P51's MapPack: a menu-sized blob has no business in the SyncFrame path).
  The static JSON-LD pack is a **build-time artifact** (P69 emits it), not a live mesh payload.
  `order_item.vendor_id` rides the existing order event on the P34/P37 wire — an additive field on
  an already-carried event, no new payload budget.
- **Living memory:** catalog leaves are content-addressable authored state; a superseded price
  **demotes, never mutates in place** — the historical price is preserved as the `unit_price`
  snapshot on `order_item` (the existing price-snapshot discipline, domain.rs:28). An order priced
  at leaf-version `v1` keeps `v1`'s numbers even after the vendor re-prices (demote-never-delete,
  matching the living-memory arc).

### 5.4 Rollback / self-healing vocabulary (item 13, used precisely)

- **Self-Termination leg claimed:** typed `CatalogError`/`CrossCurrency`/`CrossVendor`/`Overflow`
  refusals; `NodeBody::Leaf(PriceableLeaf)` making unpriced-purchasable unrepresentable; the
  `Money` cross-currency guard. Unsafe states are structurally absent, not policed.
- **Self-Healing leg claimed NARROWLY:** `Cart::reconcile` (cart.rs:117) already drops delisted
  leaves and re-prices survivors at current menu truth — P62 extends it to re-resolve
  `PriceableLeaf` (drop a leaf whose vendor delisted it; re-fold current components). A drifted
  cart heals to the current catalog. Claimed for the **cart projection only**, not for state.
- **Snapshot-Re-entry: NOT claimed.** The catalog is authored state in the event log; recovery =
  replay from the log, which is re-derivation, not snapshot re-entry. Mechanical rollback: every
  change is additive (`vendor.rs` new; `catalog.rs`/`domain.rs`/`json_api.rs` extended with new
  items + two struct fields) — reverting restores today's tree.

### 5.5 Linux discipline (item 9) + tensor/spectral/eqc (item 16)

Verdicts per the adoption framework: **ALREADY-EQUIVALENT** — one money authority (`money.rs`
shared by cart/catalog/charge-leg/JSON-LD), one price-string authority
(`price_to_decimal_string` = `format_money` discipline), one fan-out key (`VendorId`).
**REINFORCES** — the trusted-catalog invariant (`place_order_priced` re-derives price) is extended
to `vendor_id` (re-derive attribution too), a stable widening of an existing red-line.
**EXTENDS** — the "unrepresentable invalid state" doctrine to the catalog leaf (a new gate class:
X7 as a type, not a validator). **GAP** honestly named — the JSON-LD `Offer` shape must track
schema.org's evolving vocabulary; Wave-0 pins `Menu`/`MenuItem`/`Offer` (the stable core), and a
vocabulary drift is a projection-only change (no data-model impact). **Item 16 (tensor/spectral):**
deliberately NOT decoratively invoked — a menu is a plain adjacency-list tree; no Laplacian/spectral
machinery is load-bearing here (the Anu/Ananke discipline forbids ritual math). **eqc-rs:** the
resolve fold is integer `checked_add`; there is no closed-form equation to compile — reuse the
hand-written money authority, do not manufacture an equation.

---

## 6. DoD — falsifiable, RED→GREEN, per item (item 2)

| Item | RED (fails before) | GREEN (passes after) | Permanent regression (item 17) |
|---|---|---|---|
| M1 | no `VendorId`; ordering test absent | `vendor_id_ordered_stable`; 0/MAX both keyable | — |
| M2 | no `PriceableLeaf`; cross-currency/cross-vendor corpus RED by construction | `resolve_line` absolute+delta green; cross-currency ⇒ `CrossCurrency`; cross-vendor ⇒ `CrossVendor`; overflow ⇒ `Overflow`; negative ⇒ `NegativePrice`; zero-price accepted | leaf-invariant guard test (ledger row) |
| M3 | no `validate_tree`; tree tests absent | free-form tree (food + flowers labels) validates; cycle/dangling/leaf-has-children refused; per-vendor scoping | tree-validation test |
| M4 | `OrderItem` has no `vendor_id`; `charge_legs` absent | 3-vendor order → 3 legs (legs sum to subtotal); same-vendor items → 1 leg; mixed-currency ⇒ `Err`; overflow ⇒ `Err`; `vendor_id` catalog-authoritative (client cannot forge) | charge-leg + vendor-trusted tests (ledger rows) |
| M5 | no JSON-LD projection; `€0.01`-style corpus RED | `Offer.price` integer-exact strings; valid `Menu` shape; i64::MAX renders exact (no float); SoldOut still offered | no-float-on-money-JSONLD test (ledger row) |
| M6 | no `vendor_id` column / predicate (DB-gated `#[ignore]`) | vendor scope narrows within location; hub-wide reads all vendors; outer deny-on-unset still fires; inner cannot escape location | RLS narrowing test (DB-gated) |
| M7 | no N=1 property/audit test | single-vendor fixture = degenerate multivendor (same functions, 1 leg); source audit finds no special-case token | **`no_single_vendor_special_path`** (ledger row — the task's mandated gate) |

**Not-done clauses:** a bare-`i64` leaf price anywhere (must be `Money`) = NOT done regardless of
green totals (X7); a float on any monetary value (JSON-LD included) = NOT done; a
`vendor_id`-supplied-by-client path in `place_order_priced` = NOT done (forgeable leg attribution);
any `if n_vendors == 1` / single-vendor branch = NOT done (M7 audit RED); executing the RLS
migration outside the pgrust council gate = NOT done (red-line).

---

## 7. Benchmark plan (item 10) — existing Criterion harness, four benches, zero new infra

Reuse the kernel Criterion harness (P-A §6 / P51 §7 precedent). Add:
`catalog/resolve_line_10_components` (< 1 µs — the fold is checked adds); `catalog/validate_tree_1k_nodes`
(< 1 ms — O(n) walk); `domain/charge_legs_30v_300items` (< 50 µs — the food-court fan-out, the
"multi-vendor is cheap" claim made falsifiable); `json_api/menu_jsonld_500_leaves` (< 5 ms).
**The N=1 bench is the same `charge_legs` bench at `1v_10items`** — measured beside `30v_300items`
to prove the cost shape is one function scaling with items, not two code paths (item 14 / §1.4).
All added RED-commit-first so baselines auto-seed; results to `BENCH_HISTORY.md`, never prose
estimates. Telemetry: catalog-resolve + charge-leg counters ride the existing `native-trackers`
hooks (P-H's lane), so a fan-out cost regression surfaces without review.

---

## 8. Links to docs & memory (item 7)

Depends on / cites: `CORE-ROADMAP-STANDARD-2026-07-17.md` (contract) ·
`SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §0.2-1 (MoR ruling), §2/X7 (the owned invariant), §5
(P62 row + consumer assignment) · `OPUS-R5-MULTIVENDOR-ECOSYSTEM-OPS-2026-07-18.md` §3 (food-court
prior art; row-scoping verdict; §7 riskiest-unknown #4 = the leaf-invariant seam) ·
`MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §16.15/§16.16/§16.17/§16.46/§16.49
(binding rulings) · `BLUEPRINT-P-NATIVE-PGRUST-TENANT-REBUILD.md` (the RLS-FORCE mechanism reused;
M6 lands through its council gate) · `docs/red-team/2026-07-13/D2-rls-data-governance.md` §2/R3
(fail-open lesson; `order_items` columns) · `BLUEPRINT-P51-open-map-routing.md` (structural
template; static-asset-not-gossip posture) · `docs/regressions/REGRESSION-LEDGER.md` (five rows
named in §6) · `HERMETIC-ARCHITECTURE-PRINCIPLES.md` (§9). **Consumers (feed, do not modify):**
**P69** (customer storefront + checkout + the static SEO pack that WRITES the JSON-LD `menu_jsonld`
emits) · **P60** (payment adapter core — consumes `charge_legs`, owns the idempotency contract +
N-leg auth-then-capture) · **P70** (owner surface — edits the `CatalogNode` tree) · **P72**
(food-court N-leg checkout — consumes `charge_legs`, owns partial-failure/void). Memory:
`rust-native-bare-metal-decision-2026-07-14` (DECART §3; extend-don't-fork) ·
`anu-ananke-strict-discipline-feedback-2026-07-17` (no ritual spectral math, §5.5) ·
`verified-by-math-2026-07-07` · `test-integrity-rules-2026-06-27` (money-RLS-PII red-lines) ·
`never-bypass-human-gates-2026-06-29` (M6 stays behind the pgrust council gate). Supersedes:
nothing — additive; closes the §16.15 "vendor-scoping layer … not yet designed" gap.

**Note on transitive cites:** R1 §7 (schema.org JSON-LD as the AEO substrate) is cited through the
synthesis §2/X7 and §5; it was not read directly this pass — the JSON-LD *shape* is grounded in
schema.org's `Menu`/`MenuItem`/`Offer` vocabulary, and P69 (its emitter) owns the crawlability
proof.

---

## 9. Hermetic principles honored (item 20 — load-bearing only)

- **P1 MENTALISM** (spec is source): the X7 invariant is a *type* (`PriceableLeaf`), not a runtime
  validator — the schema is the spec, and an invalid leaf cannot be constructed (§5.1).
- **P2 CORRESPONDENCE** (one concept, one primitive): one money authority (`money.rs`), one leaf
  invariant consumed by three systems, one fan-out key (`VendorId`), one price-string authority,
  one resolver — no consumer re-derives.
- **P4 POLARITY** (one axis, two poles): single-vendor and food-court are the SAME model at
  `N=1` vs `N>1`, not two designs — the degenerate case is a pole of one axis, machine-proven
  (§4.7). This is the principle the task's "reduces cleanly to single-vendor" invokes.
- **P6 CAUSE-AND-EFFECT** (determinism as law): deterministic `VendorId`-ordered charge legs,
  deterministic `LeafId`-ordered JSON-LD, integer money end to end — every determinism claim
  carries a falsifier (§4, §6).
- **P7 GENDER** (paired verification, no self-certification): the invariant is refereed by three
  *independent* consumers (cart / charge-leg / JSON-LD); if any could redefine it the pairing
  breaks — the single-owner contract + the `no_single_vendor_special_path` audit are the
  self-certification refusal.

(P3/P5 not load-bearing here; not claimed decoratively.)

---

## 10. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 (fresh cites; the currency-blind/vendor-blind `catalog.rs` finding; the reused RLS mechanism) |
| 2 DoD | §6 |
| 3 spec/event-driven TDD | §3 spec-first; §4 RED-first; §4.4 charge-leg + §4.6 RLS assert on event/row sequences |
| 4 predefined types/consts | §3 |
| 5 adversarial/breaking tests | §4.2–4.7 (cross-currency, cross-vendor, overflow, cycle, dangling, i64::MAX-no-float, forged vendor_id, single-vendor-audit) |
| 6 hazard-safety as math | §5.1 (unrepresentable unpriced leaf; unreachable cross-currency/misrouted-leg/cross-hub) |
| 7 links docs/memory | §8 |
| 8 scaling axes | §5.2 (each with a named break point) |
| 9 Linux discipline | §5.5 (all four verdict classes incl. an honest GAP) |
| 10 benchmarks+telemetry | §7 |
| 11 isolation/bulkhead | §5.3 (hub-local, per-vendor pure validation) |
| 12 mesh awareness | §5.3 (catalog not gossiped; JSON-LD build-time asset; vendor_id additive on an existing event) |
| 13 rollback/self-heal vocabulary | §5.4 (two legs claimed precisely, one refused) |
| 14 error-propagation gates | §6 (ledger rows), §5.1 (typed refusals), §4.7 (the compile/CI-time N=1 audit) |
| 15 living memory | §5.3 (price snapshot demote-never-mutate) |
| 16 tensor/spectral + eqc reuse | §5.5 (spectral honestly NOT invoked; eqc N/A with reason) |
| 17 regression ledger | §6 (five rows) |
| 18 agent-executable instructions | §11 |
| 19 reuse-first | §1.3, §0 (`catalog`/`cart`/`money`/`domain`/`json_api` all extended, not rebuilt; four rejected alternatives §3) |
| 20 Hermetic citations | §9 |

---

## 11. Clear instructions for other agentic workers (item 18 — zero session context assumed)

Order below is the dependency order; T1–T5 + T7 are buildable today with zero network / zero
server tier (`catalog`/`cart`/`money`/`domain` are pure-`std`, §0). T6 is DESIGN-only in Wave-0.

1. **T1 (M1).** Create `kernel/src/vendor.rs` per §3 (`VendorId(pub u64)`, derive
   `Copy/Ord/Hash`). Register `pub mod vendor;` in `kernel/src/lib.rs` (alphabetical). Write
   `vendor_id_ordered_stable` first. Acceptance: `cargo test -p dowiz-kernel vendor` green.
2. **T2 (M2 — the invariant is the contract).** Extend `kernel/src/catalog.rs` with
   `PriceableLeaf`/`PriceComponent`/`CatalogNode`/`NodeBody`/`CatalogError`/`resolve_line` per §3
   (types verbatim). `price: crate::money::Money` — NEVER a bare i64. Write the RED tests first
   (§4.2): absolute+delta fold; cross-currency ⇒ `CrossCurrency`; cross-vendor ⇒ `CrossVendor`;
   overflow ⇒ `Overflow`; negative ⇒ `NegativePrice`; zero-price accepted. Do NOT touch the
   existing `PriceEntry`/`PriceCatalog` semantics — extend beside them (they stay the flat trusted
   lookup; `PriceableLeaf` is the currency+vendor-carrying leaf). Acceptance:
   `cargo test -p dowiz-kernel catalog` green.
3. **T3 (M3).** Add `validate_tree` to `catalog.rs` per §3 + §4.3 (cycle/dangling/leaf-has-children;
   NO label taxonomy; per-vendor scoping; deterministic `LeafId` order). Acceptance: catalog tests
   green including the flowers-vendor free-form fixture.
4. **T4 (M4).** Extend `kernel/src/domain.rs` `OrderItem` with `vendor_id` + `currency`; extend
   `place_order_priced` to re-derive BOTH from the trusted catalog leaf (the vendor-trusted
   security property, §4.4) — mirror the existing `unit_price` re-derivation exactly; the existing
   domain.rs tests will need the two new fields added to their fixtures. Add `charge_legs` +
   `kitchen_tickets` per §3. Write the RED tests first (§4.4). Acceptance:
   `cargo test -p dowiz-kernel domain` green; legs sum to subtotal; mixed-currency ⇒ Err.
5. **T5 (M5).** Extend `kernel/src/json_api.rs` (under `#[cfg(feature = "json-api")]`) with
   `menu_jsonld` + `price_to_decimal_string` per §3 (integer divmod, reuse the `cart::format_money`
   discipline — NO float). RED tests §4.5 (`0.01`/`12.50`/`0.00`; i64::MAX exact; valid `Menu`
   shape; SoldOut still offered). Acceptance: `cargo test -p dowiz-kernel --features json-api json_api` green.
6. **T6 (M6 — DESIGN only; do NOT run any migration).** Record the two-layer predicate (§3 SQL,
   §4.6) in `BLUEPRINT-P-NATIVE-PGRUST-TENANT-REBUILD.md`'s scope as the `catalog_node`/`order_items`
   rows (coordinate with that blueprint's owner — the DDL lands ONLY through its council gate).
   Write the DB-gated `#[ignore]`d RLS tests (§4.6) in `kernel/src/pgrust_tenant.rs`'s test module
   *when that module is created by the pgrust blueprint* — they go GREEN when the server tier
   reactivates. Do NOT add sqlx code to the default build. Acceptance: the predicate is recorded +
   the ignored tests compile.
7. **T7 (M7 — the mandated gate).** Add `single_vendor_is_degenerate_multivendor` (property) +
   `no_single_vendor_special_path` (source audit over `catalog.rs`/`cart.rs`/`domain.rs`/`vendor.rs`)
   per §4.7. Add the five §6 ledger rows to `docs/regressions/REGRESSION-LEDGER.md` (with
   `no_single_vendor_special_path` named as the permanent N=1 guarantee). Acceptance: both tests
   green; the audit test RED-proves by temporarily inserting a `if n_vendors == 1` line and
   confirming it fails, then removing it.
