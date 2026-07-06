# Sovereign Core — the f64→i64 money boundary for pricing extraction (GRAND-PLAN 0b-1)

Status: PROPOSAL (design-time, no code). Authors: System Architect. Red-line: YES (money).
Grounds: `GRAND-PLAN.md §0b-1`, `DECISIONS.md`, `LEAD-REVIEW.md F2`,
`rebuild/crates/domain/clippy.toml` (disallowed-types f64/f32), current
`rebuild/crates/api/src/routes/orders/pricing.rs` (884 lines, read in full).

---

## 1. Problem + non-goals

**Problem.** Move the pure money composition out of the shell into the sovereign core
(`dowiz-core` = `rebuild/crates/domain`, lib `domain`) as `kernel::pricing`, WITHOUT letting a
single `f64` compile into the core — the core's `clippy.toml` `disallowed-types = [f64, f32]` (live,
commit 83ac471e, enforced by `sovereign-gate.sh` Gate 2) makes any float a hard build failure. Three
functions/type-clusters carry floats today and cannot move as-is:

1. `apply_tax(subtotal: i64, tax_rate: f64, …)` — computes `rate_micro = round(tax_rate·1e6)`
   inside, via the shell helper `super::round_f64_to_i64`.
2. `distance_km(f64,f64,f64,f64) -> f64` — Haversine trig; GRAND-PLAN pins it to the shell (float
   determinism hazard native↔wasm).
3. `DeliveryTier { max_distance_km: f64, … }`, `FeeLocation { lat/lng: Option<f64>, … }`, and
   `resolve_delivery_fee`/`delivery_fee_for_order`, which call `distance_km` and float-compare
   `dist <= tier.max_distance_km`.

The open question the plan defers to this council: **the exact f64→i64 boundary** — new signatures,
km→meter rounding rule + its effect on tier-selection, where conversion happens, and the core error
type.

**Non-goals.** No behavior change beyond the mechanical move + the boundary integerization (red-line
money — parity is the whole point). No composition of pricing into `decide` (that is 0b-3). No
migration (pure code move; forward-only, no schema touch). No newtype money refactor. No change to
`distance_km`'s math, `Lek`, or the wire error codes.

---

## 2. Back-of-envelope

**Scale / connections.** N/A for connection budget — this is a pure-function code move, zero new
runtime, zero new queries, zero new connections (API+worker+analytics+migrations budget unchanged).
The only "scale" that matters is the numeric domain of the two float boundaries:

- **Tax rate micro-scale.** `rate_micro = round(rate·1e6)`. Real rates ≤ 1.0 ⇒ `rate_micro ≤ 1e6`.
  `subtotal ≤ int4 max ≈ 2.147e9`. `subtotal·rate_micro ≤ ~2.15e15 « i64::MAX (9.2e18)` — ~4000×
  headroom (this is the REV-S5-4 finding, unchanged). Moving the multiply into core with an already-
  integer `rate_micro` param changes nothing about this budget.

- **Distance meters.** Earth's max great-circle ≈ 20 000 km = 2e7 m « i64. A realistic Albanian
  delivery radius ≤ ~50 km = 5e4 m. `distance_km` already rounds to 3 dp (`(d·1000).round()/1000`),
  i.e. it is ALREADY quantized to whole meters before any compare. So the "integer meters" boundary
  adds **zero** new rounding error to the distance operand — the meter value is exact.

**Tier-flip BOE (Q6).** The compare is `distance ≤ max_distance`. distance is quantized to whole
meters by construction. Tier maxima in `delivery_tiers.max_distance_km` are `numeric` (schema:
`1780338982014_location_commerce.ts:18`, **no scale cap**). For any tier config with ≤ 3-dp km
precision (= meter granularity — every realistic config: 1, 1.5, 2.5, 5 km), `round(dist·1000) ≤
round(max·1000)` is **bit-identical** to `dist_km ≤ max_km`, because both operands scale by exactly
1000 with no residual. **Counter-example (the honest one):** a tier authored to sub-meter precision,
e.g. `max_distance_km = 1.2345` (= 1234.5 m) with a delivery at exactly 1.235 km (1235 m): float
`1.235 ≤ 1.2345` = FALSE (not covered); integer round-half-up `1235 ≤ round(1234.5)=1235` = TRUE
(covered) — the outcomes differ. This requires a 4-dp tier boundary AND a delivery landing within
0.5 m of it. Sub-meter delivery tiers are operationally meaningless (Haversine on 5-dp coords + a
spherical-earth radius is not sub-meter accurate; couriers do not route to the half-meter). It is
**operationally unreachable but not schema-forbidden** → accepted-risk row R1 with a cheap
compensating gate (owner-UI/validate `max_distance_km` to ≤ 3 dp, or a DB CHECK on scale).

---

## 3. Options (≥2 per open question, with the concept named)

### Q1 — New signatures; does the shell `pricing.rs` survive?

- **Option 1A — "Integer-boundary snapshot + thin shell adapter" (CHOSEN).** Core fns take
  pre-integerized inputs (`rate_micro: i64`, `distance_m: Option<i64>`, `DeliveryTier.max_distance_m:
  i64`). `routes/orders/pricing.rs` survives as a **thin shell adapter (shim)** that owns the two
  float boundaries (`distance_km`, `round_f64_to_i64`-based rate conversion, f64→i64 tier/pin
  conversion) and exposes adapter fns with the **same f64 signatures pg.rs uses today**, delegating
  to core. Concept: *Anti-Corruption Layer / boundary snapshot* — floats are marshalled to integers
  at the shell edge; the core sees only integers.
  *Trade:* pg.rs is essentially untouched (adapter keeps its signatures); `shifts.rs`'s
  `crate::routes::orders::pricing::distance_km` import is untouched (distance_km stays in the shim);
  diff is small and reversible. Cost: one extra hop (adapter → core), trivially inlined.

- **Option 1B — "Dissolve the shim; inline conversions in pg.rs."** Delete `pricing.rs`; pg.rs
  converts floats inline and calls `domain::kernel::pricing::*` directly. Concept: *no indirection*.
  *Trade:* one fewer file, but (a) pg.rs (a hotspot, health 1.0/10 area) grows float-marshalling
  noise on the money path; (b) `shifts.rs`'s `distance_km` import breaks and must be re-homed (a
  second, unrelated edit surface); (c) the float boundary is scattered across call sites instead of
  fenced in one module — harder to audit "where do floats touch money." Rejected: worse blast radius
  on a red-line file, and it disperses the very boundary this council exists to contain.

### Q2 — km→meters rounding rule

- **Option 2A — "Symmetric round-half-up (nearest meter) on both operands" (CHOSEN).**
  `to_m(km) = round_half_up(km·1000)` applied to BOTH the distance and each tier max, in the shell
  adapter, reusing the existing `round_f64_to_i64` (half-away-from-zero = Node `Math.round`, already
  the codebase's one f64→i64 convention). Concept: *quantize-then-compare, symmetric*. Because
  distance is already meter-quantized, this is bit-identical to the old float compare for all
  ≤3-dp tier configs (see §2). *Trade:* the sole residual divergence is the sub-meter-tier
  counter-example (R1) — vanishingly rare, meaningless in practice.

- **Option 2B — "Asymmetric floor(max)/ceil(dist)" (or truncation).** Bias the rounding so a point
  on the boundary is deterministically excluded/included. Concept: *conservative boundary bias*.
  *Trade:* CHANGES the common-case result at every exact whole-km boundary (a delivery at exactly
  1000 m against a 1.0 km tier would flip depending on direction), breaking the existing
  `resolve_fee_*` tests and, worse, changing real charges for the most common tier configs. Rejected:
  it "fixes" an unreachable edge by breaking the reachable centre.

### Q3 — Where the conversion happens

- **Option 3A — "In the shell adapter module (`routes/orders/pricing.rs`)" (CHOSEN).** All three
  conversions (rate→rate_micro, pin+loc→distance_m, tier km→m) live in the shim's adapter fns.
  pg.rs passes the same f64s it has today; the shim integerizes. Concept: *single float chokepoint*.
  *Trade:* the "where do floats meet money" answer is one file. Best auditability.

- **Option 3B — "In pg.rs before the call."** pg.rs does the conversions and calls core directly.
  Concept: *convert at the data source (right after the SQL read)*. *Trade:* pg.rs already reads the
  f64 rows (tax_rate, tiers, lat/lng), so this is "natural" — but it spreads float logic across the
  crown-jewel INSERT function and couples the SQL layer to the rounding convention. Rejected for the
  same reason as 1B: red-line file blast radius + dispersed boundary. (3B is the fallback if the
  council rejects keeping `pricing.rs`.)

### Q4 — The core error type / error-code serialization

- **Option 4A — "Core returns `PricingError { code: domain::ErrorCode, message }`" (CHOSEN).** Move
  `PricingError` into `domain` with `code: ErrorCode` (a domain type — NOT a shell `&'static str`).
  Every wire code the shim needs already exists in `domain::error::ErrorCode` (verified:
  `ProductNotFound, ProductUnavailable, ModifierUnavailable, ModifierMinNotMet, ModifierMaxExceeded,
  DuplicateModifier, NotDeliverable, DeliveryNotConfigured, MinOrderNotMet`), and its `#[serde(rename_all
  = "SCREAMING_SNAKE_CASE")]` guarantees `ProductNotFound → "PRODUCT_NOT_FOUND"` — the exact wire
  string, by construction. Concept: *the core speaks the domain's own error vocabulary; the shell
  string is derived, not authored.* *Trade / bonus:* this **deletes** pg.rs's `pricing_code(&str) ->
  ErrorCode` mapper (pg.rs:987) and its test (pg.rs:1077) — a mirror/drift risk removed. pg.rs
  changes `Rejected(pricing_code(e.code), e.message)` → `Rejected(e.code, e.message)`. Slightly more
  than a mechanical move, but net-negative code and drift-eliminating.

- **Option 4B — "Keep `PricingError { code: &'static str }` local to core."** Core defines its own
  string-coded error; pg.rs keeps `pricing_code`. Concept: *minimal mechanical move*. *Trade:* keeps
  a shell wire-convention (SCREAMING_SNAKE strings) living in the core AND keeps the `&str→ErrorCode`
  mirror in pg.rs (drift risk — the exact D5 mirror-oracle smell). Rejected, but it is the strict
  "mechanical-move-only" fallback if the council wants the smallest possible red-line diff (then the
  ErrorCode conversion becomes a defer-flag folded into 0b-3, when pricing errors flow through
  `decide` anyway).

---

## 4. Decision (ADR-format → `docs/adr/ADR-sovereign-core-money-boundary-0b1.md`)

**1A + 2A + 3A + 4A.** Core `kernel::pricing` is integer-only; a thin shell adapter
(`routes/orders/pricing.rs`) is the single float chokepoint, converting rate→`rate_micro`,
pin+loc→`distance_m`, tier-km→`max_distance_m` via symmetric round-half-up (`round_f64_to_i64`), and
`distance_km` stays in that shim. Core errors carry `domain::ErrorCode`, deleting the pg.rs
`pricing_code` mirror.

**Tax-guard split (resolves Breaker HIGH — the old f64 short-circuit
`if subtotal == 0 || tax_rate <= 0.0 || !tax_rate.is_finite() { return Ok(0); }` at `pricing.rs:50`
cannot exist verbatim in i64-core).** The guard is split across the boundary by domain, NOT
duplicated, NOT dropped:

- **Core (primary, i64-domain):** `if subtotal == 0 || rate_micro <= 0 { return Ok(0); }` as the
  first line of `apply_tax`. This is the CHOSEN placement for the sign/zero arm because it protects
  **every** caller — today's adapter AND any future one — from a non-positive `rate_micro` reaching
  `checked_mul` (which would produce a negative `Lek` → `Err` → 5xx). The core "trust the caller" pos-
  ture is deliberately NOT relied upon for the sign invariant: an i64 guard here is one branch and
  closes the class permanently.
- **Shell adapter (float-domain, before conversion):** `if !tax_rate.is_finite() { return Ok(0); }`.
  This arm MUST stay in the float domain, because the core i64 guard is structurally blind to it:
  `+Infinity` converts to `round_f64_to_i64(INF·1e6) = i64::MAX` — a **positive** `rate_micro` the
  `rate_micro <= 0` core guard passes straight through into a `checked_mul` overflow → 5xx. NaN
  happens to convert to 0 (caught by luck by the core guard) but is guarded here explicitly rather
  than relied on. The float `tax_rate <= 0.0` arm is intentionally omitted from the shell (subsumed by
  the core `rate_micro <= 0`: a negative/zero rate rounds to a non-positive micro-rate).

Together the two arms reproduce OLD `Ok(0)` for all four exotic inputs — negative rate, zero rate,
±Infinity, NaN — restoring byte-parity on the money red line.

**Final core signatures (`domain::kernel::pricing`):**

```
// tax — rate arrives pre-scaled to micro-units (6 dp); no f64 in core.
// GUARD (core, i64-domain — replaces the old f64 short-circuit `subtotal==0 || tax_rate<=0.0`):
//   pub fn apply_tax(subtotal: i64, rate_micro: i64, price_includes_tax: bool) -> Result<i64, MoneyError> {
//       if subtotal == 0 || rate_micro <= 0 { return Ok(0); }   // FIRST LINE, before any checked_mul
//       … existing inclusive/exclusive checked arithmetic …
//   }
pub fn apply_tax(subtotal: i64, rate_micro: i64, price_includes_tax: bool) -> Result<i64, MoneyError>
pub fn compute_line_total(product_price: Lek, modifier_deltas: &[Lek], quantity: i64) -> Result<Lek, MoneyError>   // unchanged
pub fn compose_total(subtotal: Lek, delivery_fee: Lek, charged_tax: Lek, discount_total: Lek) -> Result<Lek, MoneyError>  // unchanged
pub fn charged_tax(tax_total: Lek, price_includes_tax: bool) -> Lek   // unchanged
pub fn compute_order_pricing(items: &[PricingItem], snapshot: &PricingSnapshot) -> Result<(Lek, Vec<PricedOrderItemRow>), PricingError>  // unchanged sig, PricingError.code now ErrorCode

// delivery — distance & tier bounds arrive as integer meters; no f64, no distance_km call.
/// INTEGER-METER core tier. Do NOT confuse with the shim's f64 `DeliveryTier`
/// (`routes::orders::pricing::DeliveryTier { max_distance_km: f64 }`) — grabbing the wrong one is a
/// 1000×-scale (km-vs-m) money bug. This type's bound is METERS.
pub struct DeliveryTier { pub max_distance_m: i64, pub fee: i64 }          // was max_distance_km: f64
/// INTEGER-METER / integer-Lek core location. Do NOT confuse with the shim's f64 `FeeLocation`
/// (`routes::orders::pricing::FeeLocation { lat: Option<f64>, lng: Option<f64>, … }`). This type has
/// NO lat/lng (they only fed `distance_km`, which stays in the shim).
pub struct FeeLocation  { pub delivery_fee_flat: Option<i64>, pub free_delivery_threshold: Option<i64>, pub min_order_value: Option<i64> }  // lat/lng REMOVED (they only fed distance_km)
pub fn resolve_delivery_fee(location: FeeLocation, distance_m: Option<i64>, tiers: &[DeliveryTier]) -> Result<Lek, PricingError>
pub fn delivery_fee_for_order(subtotal: Lek, is_pickup: bool, location: FeeLocation, distance_m: Option<i64>, tiers: &[DeliveryTier]) -> Result<Lek, PricingError>

pub struct PricingError { pub code: domain::ErrorCode, pub message: String }   // moved into domain
```

**Shell adapter signatures (`routes/orders/pricing.rs`) — unchanged from today, so pg.rs is
untouched except the `pricing_code` deletion:**

```
// SHIM f64 SHAPES — same names as the core i64 types on purpose (adapter impersonates the old sig).
/// SHELL f64 tier — km bound, NOT the core's `domain::kernel::pricing::DeliveryTier` (meters).
/// Do not glob-import `domain::DeliveryTier` where THIS f64 km shape is meant: 1000×-scale bug.
///   pub struct DeliveryTier { pub max_distance_km: f64, pub fee: i64 }
/// SHELL f64 location — carries lat/lng, NOT the core's `domain::kernel::pricing::FeeLocation`.
///   pub struct FeeLocation  { pub lat: Option<f64>, pub lng: Option<f64>, delivery_fee_flat, free_delivery_threshold, min_order_value }
pub fn distance_km(lat1,lon1,lat2,lon2: f64) -> f64                        // STAYS (shifts.rs import intact)
pub fn apply_tax(subtotal: i64, tax_rate: f64, price_includes_tax: bool) -> Result<i64, MoneyError> {
    // SHELL GUARD (float-domain, before conversion): ±Infinity/NaN have no faithful i64 image
    // (round_f64_to_i64(INF·1e6) = i64::MAX → later checked_mul OVERFLOW → 5xx). The old f64 guard's
    // `!tax_rate.is_finite()` arm MUST live here in the float domain — the core i64 guard cannot see it
    // (a +Inf rate maps to a POSITIVE i64::MAX, not a rate_micro ≤ 0). Reproduces OLD Ok(0).
    if !tax_rate.is_finite() { return Ok(0); }
    // Sign/zero (`tax_rate <= 0.0`) is NOT re-checked here: it is subsumed by the core guard
    // `rate_micro <= 0` (a negative or zero rate rounds to a rate_micro ≤ 0). Defense-in-depth: the
    // core guard also protects EVERY future caller, not just this adapter.
    domain::kernel::pricing::apply_tax(subtotal, round_f64_to_i64(tax_rate * 1e6), price_includes_tax)
}
pub struct DeliveryTier { pub max_distance_km: f64, pub fee: i64 }         // shell-side f64 row shape
pub struct FeeLocation  { pub lat: Option<f64>, pub lng: Option<f64>, delivery_fee_flat, free_delivery_threshold, min_order_value }
pub fn delivery_fee_for_order(subtotal, is_pickup, location: FeeLocation, pin: Option<(f64,f64)>, tiers: &[DeliveryTier]) -> …
  // adapter: distance_m = pin&loc → round_f64_to_i64(distance_km(...)·1000); map tiers km→m; call core
```

**Why.** It confines every float to one shell module (auditable), keeps the crown-jewel INSERT
(pg.rs, a health-1.0 hotspot) and the courier geo path (shifts.rs) untouched, satisfies the core
purity gate by construction, preserves byte-parity on every realistic input, and net-removes code
(the `pricing_code` mirror).

---

## 5. Data / migrations

**None.** Forward-only, pure code move. Zero schema change. `delivery_tiers.max_distance_km` stays
`numeric`; `tax_rate` stays `numeric`; the SQL casts in pg.rs (`::double precision`, `::float8`) are
unchanged — the f64 still enters the shell exactly as today, and is integerized in the adapter.
Money stays integer (`Lek` i64) end-to-end; RLS FORCE on `delivery_tiers`/`orders` unaffected (no
table touched).

---

## 6. Consistency + idempotency (byte-parity preservation)

The invariant is **byte-parity vs the hand-derived oracle vectors**, not a runtime consistency
property. Preservation plan:

- **Unchanged-signature fns** (`compute_line_total`, `compose_total`, `charged_tax`,
  `compute_order_pricing`) — tests **move byte-for-byte** into `kernel::pricing`'s `#[cfg(test)]`.
- **`apply_tax`** — core gets integer-`rate_micro` vectors (`0.075→75000`, `0.0745→74500`,
  `0.0744→74400`, `0.2→200000`, `0.0825→82500`) asserting the SAME numeric outputs. The **f64-rate**
  vectors (`ORDER_TOTAL_VECTORS`, `apply_tax_matches_node_money_tax_vectors`, LC1-inclusive,
  exclusive) STAY in the shim, now exercising `adapter.apply_tax(f64)` → proving the f64→rate_micro
  boundary is unchanged end-to-end. (This is a feature: the shim vectors become an independent
  oracle over the real float boundary — a D5-friendly non-mirror check.)
- **`apply_tax` guard vectors (NEW — the Breaker HIGH gap; each asserts byte-parity OLD `Ok(0)` ==
  NEW).** Core `#[cfg(test)]`, exercising the i64 guard directly:
  ```
  // fn apply_tax_core_guard_returns_zero_on_nonpositive_rate_and_zero_subtotal()
  assert_eq!(apply_tax(1000, -200000, false), Ok(0));   // negative rate_micro (was: guard tax_rate<=0.0)
  assert_eq!(apply_tax(1000, -1,      false), Ok(0));    // smallest negative
  assert_eq!(apply_tax(1000, 0,       false), Ok(0));    // zero rate_micro
  assert_eq!(apply_tax(0,    75000,   false), Ok(0));    // zero subtotal
  assert_eq!(apply_tax(1000, -200000, true),  Ok(0));    // inclusive branch, same guard
  ```
  Shim `#[cfg(test)]`, exercising the float-domain arm through the real adapter+`round_f64_to_i64`:
  ```
  // fn adapter_apply_tax_nonfinite_and_negative_rate_match_old_ok_zero()
  assert_eq!(adapter::apply_tax(1000, f64::INFINITY,     false), Ok(0));  // was: !is_finite → Ok(0); would 5xx-overflow without shell guard
  assert_eq!(adapter::apply_tax(1000, f64::NEG_INFINITY, false), Ok(0));
  assert_eq!(adapter::apply_tax(1000, f64::NAN,          false), Ok(0));
  assert_eq!(adapter::apply_tax(1000, -0.2,              false), Ok(0));  // negative f64 rate → rate_micro=-200000 → core guard → Ok(0)
  assert_eq!(adapter::apply_tax(1000, f64::INFINITY,     true),  Ok(0));  // inclusive branch
  ```
  These are the ONLY behavior the integerization was most likely to lose (Breaker: "the single
  behavior … no test can catch"); they are now the red→green proof for the guard split.
- **Delivery ladder** — core gets NEW integer vectors: `distance_m`/`max_distance_m` literals
  (e.g. tier `max_distance_m: 1000, 5000`; `distance_m: 1390` → tier2=400; `distance_m` huge →
  `NotDeliverable`; `None`→flat/`DeliveryNotConfigured`). The **f64 end-to-end** delivery tests
  (pin + `distance_km` + f64 tiers) STAY in the shim, proving adapter+core ≡ old behavior on the
  same coordinate fixtures the current tests use.
- Idempotency (order create) is untouched — `request_hash` is already core; this move does not touch
  the create idempotency branch.

Two runs, same inputs, byte-identical outputs — guaranteed because the core is now integer-only
(deterministic native↔wasm) and the shim's float→int conversion is the same `round_f64_to_i64` the
code already ships.

---

## 7. Failures + degradation

This is **compile-time** enforcement, not runtime. The failure this design defends against is a
silent native↔wasm replay divergence from a leaked float — and the defense is that such a leak
**does not compile** into core:

- Every external/float value crosses the boundary in the shell adapter; the core cannot name `f64`
  (`clippy.toml disallowed-types`), so a future edit that tries to pass a raw rate or km into core is
  a hard build error, not a runtime surprise — **fail-closed at build, zero cascade.**
- Runtime error paths are **preserved verbatim**: `apply_tax`'s `checked_*` overflow→`MoneyError`
  (surfaced as a 5xx-class bug in pg.rs, never a silent reduced charge); `resolve_delivery_fee`'s
  `NOT_DELIVERABLE`/`DELIVERY_NOT_CONFIGURED`; `compute_order_pricing`'s first-failure 422 codes.
  No external call is added (no timeout/fallback surface changes).
- `distance_km` remaining in the shell is itself a degradation choice: the one float-trig hazard is
  quarantined out of the replayable core; if it ever produced a native↔wasm-divergent meter count,
  it does so in the shell where replay determinism is not claimed, and only ever selects an i64 fee.

---

## 8. Security + tenant isolation

**N/A — why:** pure functions over already-authorized, in-transaction snapshot data. No tenant data
is read or crossed here (the RLS-scoped reads happen in pg.rs and are unchanged); no PII enters the
core (menu/price/quantity/rate/meters only — no customer identity); no secrets; no new surface. The
core remains IO/framework-free (no sqlx/axum/tokio — `lib.rs` crate law), so it cannot reach a tenant
boundary even accidentally.

---

## 9. Operability — how RED→GREEN proves the core actually catches an f64 leak

The load-bearing operability question (D5): prove the gate can go RED on the real defect.

- **Purity gate (the one that matters here).** `bash rebuild/scripts/sovereign-gate.sh` Gate 2
  (`CLIPPY_CONF_DIR scoped, cargo clippy -p dowiz-core --lib -- -D warnings`) + the wasm32 build.
  **RED proof:** on a throwaway branch, add `pub fn leak(x: f64) -> f64 { x }` to `kernel::pricing`
  (or revert `apply_tax` to take `tax_rate: f64`) → `cargo clippy -p dowiz-core --lib` emits
  `disallowed_types` on `f64` → Gate 2 FAILS the build → revert → GREEN. This demonstrates the ban is
  real on the moved module, not just on the pre-existing files.
  - **Scope caveat (honest bound — resolves Breaker MED over-claim).** `disallowed-types` fires on
    **named** `f64`/`f32` type positions (signatures, fields, generics, casts, aliases). It is a hard
    build failure for those — the real leak vector for this move (a caller re-adds `tax_rate: f64` or
    a float field). It does **NOT** catch an *inferred*-float intermediate with no `f64` token
    (`let r = 0.1 + 0.2;`). The claim is therefore "**any named f64 is a hard build failure**", not
    "no float expression can exist." The residual is doubly-fenced: an inferred float that re-enters
    the i64 money path needs a named `as i64` cast, which `clippy::as_conversions` (denied workspace-
    wide) trips. Core has zero float-literal code today (this move adds none), so the hole is a future-
    edit hazard, not a present gap. Sound bite: named-type ban is airtight; inferred-float purity
    leans on `as_conversions` at the i64 re-entry, not on `disallowed-types` alone.
- **Money parity gate.** `cargo test -p dowiz-core` (core vectors) + `cargo check -p api`
  (shim+pg.rs compile) + the shim's f64 end-to-end vectors. **RED proof (GRAND-PLAN's):** flip
  `charged_tax` to return `tax_total` in the moved core code → the LC1 core test AND the shim
  end-to-end vectors go red → revert.
- **Deployed-surface gate (0b-1 DoD).** Staging POST of a fixture cart (known items/modifiers/tax-
  mode/tier) → DB totals asserted vs hand-computed literals (non-mirror), with `x-dowiz-cutover`
  asserted to prove Rust serves the route. Distance boundary: include one delivery fixture whose
  meter distance sits mid-tier so the integer path is exercised on the real surface.
- **Observability / rollback.** No new metric needed (pure move). Rollback = revert the commit
  (forward-only, no migration to unwind). The change is not flagged (it is a like-for-like internal
  move behind the existing S5 surface); the launch gate is the existing S5 route already live.
- **F2 alignment.** This step is the first real exercise of the `disallowed-types` gate the LEAD-
  REVIEW F2 added specifically "so the invariant survives when pricing.rs eventually moves in." The
  RED proof above IS that survival test.

---

## 10. Open / accepted risks (owner)

- **R1 — Sub-meter tier divergence (ACCEPTED-RISK → DEFER-FLAG with a named re-visit trigger).** A
  tier authored to >3-dp km precision could, for a delivery within 0.5 m of the boundary, select a
  different tier than the old float compare (§2 counter-example). Schema allows it (`max_distance_km
  numeric`, no scale cap). Operationally unreachable (Haversine/earth-radius not sub-meter accurate;
  owners do not author half-meter tiers). **Grep confirms (Counsel §5): `max_distance_km` appears in
  NO owner-facing `*.tsx` editor — only server/seed/migration/tests.** So today tiers are
  engineer/seed-authored and R1 is not merely unreachable, it is **dead/unreachable — a defer-flag,
  not a blocker.** *Owner + re-visit condition:* System Architect flags this to whichever future spec
  first ships a tier-author UI (or any surface that lets a non-engineer set `max_distance_km`). **The
  compensating control (≤3-dp validation, or a `CHECK (scale(max_distance_km) <= 3)` DB constraint)
  becomes a Definition-of-Done line for that spec** — it must not ship the editor without it. Until
  such a surface exists, the risk stays dormant/unreachable; there is nothing to gate in 0b-1.
  **MISSING (deferred):** the ≤3-dp DB CHECK is intentionally NOT landed in 0b-1 (Counsel offered it
  as one-line cheap insurance; System Architect defers it to the owning spec to avoid an orphaned
  constraint no surface yet needs). Re-visit the instant a tier-editor spec opens.
- **R2 — ErrorCode coupling of the core (ACCEPTED).** Using `domain::ErrorCode` in `kernel::pricing`
  couples pricing to the (large) domain error taxonomy. Justified: it is a domain-owned type already
  exported by `lib.rs`, and it deletes the pg.rs `&str→ErrorCode` mirror. If the council prefers the
  strictest mechanical move, fall back to Option 4B and defer the ErrorCode conversion to 0b-3.
  *Owner:* this council.
- **R3 — git-blame on the move (ACCEPTED).** `git mv pricing.rs → kernel/pricing.rs` preserves blame
  on the money core (the valuable part); the re-created shell shim (distance_km + adapters) is
  genuinely new lines. Acceptable — parity tests, not blame, are the audit anchor. *Owner:* System
  Architect.
- **R4 — Float trig stays in the shell forever (ACCEPTED / by-design).** `distance_km` is not
  replayable-deterministic and never will be in the core (D2/F2). Delivery distance is therefore a
  shell-computed input to the core, not a core computation — the conservation corridor (0b-3) must
  treat `distance_m` as an untrusted-but-integer input, never recompute it. *Owner:* 0b-3 council.
- **R5 — `shifts.rs` second km→m convention (ACCEPTED / OUT-OF-SCOPE).** `shifts.rs:896` computes
  `distance_km(...) * 1000.0` (raw `*1000`, NOT `round_f64_to_i64`) for the geofence courier-ping — a
  second, unrounded km→m conversion with a different rounding convention than the money path. This is
  **not a money path** (it gates a courier-location ping, selects no fee, touches no `Lek`), is not in
  0b-1's scope, and is **left AS-IS — `shifts.rs` is UNTOUCHED by this move** (file-plan step 7). The
  §3 "single meter convention" claim is hereby narrowed to "single meter convention *on the money
  path*"; geofence meters are a separate, non-money concern. Not a blocker; no guardrail owed on a
  non-money surface. *Owner:* whichever future spec (if any) unifies geo distance handling — none is
  scheduled and none is needed for money parity.

---

## File plan (Q5)

1. `git mv rebuild/crates/api/src/routes/orders/pricing.rs
   rebuild/crates/domain/src/kernel/pricing.rs` (preserves blame on the money bulk).
2. In the moved core file: strip the shell-only items OUT (`distance_km`, the f64 `DeliveryTier`/
   `FeeLocation` shapes, the `super::round_f64_to_i64` call, the f64-signature bodies); change
   `apply_tax` to `rate_micro: i64` **and prepend the core guard `if subtotal == 0 || rate_micro <= 0
   { return Ok(0); }`** (§4); change `DeliveryTier→max_distance_m: i64`, `FeeLocation`
   (drop lat/lng), `resolve_delivery_fee`/`delivery_fee_for_order` to `distance_m: Option<i64>`;
   change `PricingError.code` to `ErrorCode`; **replace the two entropy-seeded collections in
   `compute_order_pricing` (resolves Breaker MED — first `RandomState` source in the entropy-free
   core) with ordered, entropy-free equivalents** (exact diff below); keep/rehome the integer-input
   tests + add the new integer delivery/rate_micro vectors.

   **BTreeMap/BTreeSet diff (`compute_order_pricing`, was `pricing.rs:233,239`):**
   ```
   - use std::collections::{HashMap, HashSet};            // (or inline `HashMap::new()` / `HashSet::new()`)
   + use std::collections::{BTreeMap, BTreeSet};
     …
   - let mut group_counts: HashMap<&str, i64> = HashMap::new();
   + let mut group_counts: BTreeMap<&str, i64> = BTreeMap::new();      // ordered by &str key; zero RandomState
     …
   - let mut seen = std::collections::HashSet::new();
   + let mut seen: BTreeSet<_> = BTreeSet::new();                       // ordered; entropy-free dedup
   ```
   Same API surface used today (`.get()`/`.entry()`/`.insert()` by key, `.contains()`/`.insert()` for
   dedup) — a drop-in. `BTree*` reads **no OS entropy** (no `RandomState`), satisfying core Law 2, and
   as a bonus gives a deterministic iteration order should any future edit iterate the map (the exact
   native↔wasm divergence the Breaker flagged as currently latent-but-unguarded). Cost: `O(log n)`
   vs `O(1)` lookup on ≤ handful-of-groups carts — immaterial (BOE unchanged; no new deps, `BTree*`
   is std).
3. Create NEW `rebuild/crates/api/src/routes/orders/pricing.rs` (thin shim): `distance_km`, the f64
   `DeliveryTier`/`FeeLocation` shapes, adapter fns delegating to `domain::kernel::pricing`, and the
   f64 end-to-end byte-parity oracle tests (moved from the old file).
4. `rebuild/crates/domain/src/kernel.rs` — add `pub mod pricing;`.
5. `rebuild/crates/domain/src/lib.rs` — re-export `kernel::pricing::{PricingError, DeliveryTier,
   FeeLocation, PricingItem, PricingSnapshot, ProductInfo, ModifierInfo, GroupInfo,
   PricedOrderItemRow, PricedModifierRow, …}` as needed by the shim.
6. `rebuild/crates/api/src/routes/orders/pg.rs` — delete `pricing_code` + its test (pg.rs:987,1077);
   change the two `Rejected(pricing_code(e.code), …)` sites to `Rejected(e.code, …)`. The
   `super::pricing::{…}` import and all adapter call sites stay (adapter keeps f64 signatures).
7. `rebuild/crates/api/src/routes/courier/shifts.rs` — UNTOUCHED (distance_km still at
   `crate::routes::orders::pricing::distance_km`).
8. `rebuild/crates/domain/Cargo.toml` — no new deps (std `BTreeMap`/`BTreeSet` — NOT `HashMap`/
   `HashSet`, see step 2 — plus existing `thiserror`/`serde`).

**Tests: byte-for-byte move vs new-vector.**
- Move verbatim → core: `compute_line_total_matches_node`, `compose_total_discount_seam_subtracts…`,
  `compose_total_rejects_a_negative_result`, all 5 `pricing_*` (compute_order_pricing) tests.
- New integer vectors → core: `apply_tax` (rate_micro literals), **`apply_tax_core_guard_returns_zero_on_nonpositive_rate_and_zero_subtotal`
  (the Breaker-HIGH guard vectors, §6)**, integer LC1 + exclusive properties,
  `resolve_delivery_fee`/`delivery_fee_for_order` with `distance_m`/`max_distance_m` literals.
- New shim vector: **`adapter_apply_tax_nonfinite_and_negative_rate_match_old_ok_zero`** (±Infinity/
  NaN/negative f64 rate → `Ok(0)`, §6) — the float-domain arm of the guard split.
- Stay in shim (f64 end-to-end oracle): `apply_tax_matches_node_money_tax_vectors`,
  `apply_tax_large_cart_full_rate_stays_exact_in_i64`, `order_total_composition_byte_parity…`,
  `lc1_inclusive_never_adds_tax…`, `exclusive_adds_exactly_the_extracted_tax`,
  `distance_km_zero_and_rounding`, `fee_ladder_*`, `resolve_fee_*`.
