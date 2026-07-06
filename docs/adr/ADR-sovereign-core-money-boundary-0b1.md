# ADR — The f64→i64 boundary for extracting pricing into the sovereign core (0b-1)

- Status: DRAFT (council-pending; red-line money)
- Date: 2026-07-06
- Deciders: System Architect (proposes); breaker + counsel round pending
- Context source: `docs/design/sovereign-core-money-boundary-0b1/proposal.md`
- Supersedes/relates: GRAND-PLAN §0b-1; LEAD-REVIEW F2 (the `disallowed-types` f64 ban);
  DECISIONS D2 (deterministic replayable core). Does not contradict any prior ADR (001–019).

## Context

`kernel::pricing` must move from the shell (`rebuild/crates/api/src/routes/orders/pricing.rs`) into
`dowiz-core` (`rebuild/crates/domain`, lib `domain`), which forbids `f64`/`f32` at compile time
(`crates/domain/clippy.toml disallowed-types`, enforced by `sovereign-gate.sh` Gate 2 + wasm32).
Three float carriers block a verbatim move: `apply_tax`'s `tax_rate: f64`; the Haversine
`distance_km` (pinned to the shell by GRAND-PLAN); and the `DeliveryTier`/`FeeLocation`/
`resolve_delivery_fee`/`delivery_fee_for_order` cluster that float-computes and float-compares
distance. The plan defers the exact boundary to this decision.

## Decision

Adopt **integer-boundary snapshot + a thin shell adapter** (proposal options 1A/2A/3A/4A):

1. **Core is integer-only.** Moved fns take pre-integerized inputs:
   - `apply_tax(subtotal: i64, rate_micro: i64, price_includes_tax: bool)` — the rate arrives scaled
     to micro-units (6 dp), computed in the shell.
   - `DeliveryTier { max_distance_m: i64, fee: i64 }`; `FeeLocation` drops `lat`/`lng` (they only fed
     `distance_km`); `resolve_delivery_fee(location, distance_m: Option<i64>, tiers)` and
     `delivery_fee_for_order(subtotal, is_pickup, location, distance_m: Option<i64>, tiers)` compare
     integer meters (`distance_m <= tier.max_distance_m`).
   - `compute_line_total`, `compose_total`, `charged_tax`, `compute_order_pricing` move
     signature-unchanged.

2. **km→meters = symmetric round-half-up** (`round_f64_to_i64(km * 1000)`, half-away-from-zero =
   Node `Math.round`), applied to BOTH distance and each tier max in the shell adapter. `distance_km`
   already quantizes to whole meters (3-dp km), so for every tier config with ≤3-dp precision this is
   bit-identical to the old float compare.

3. **Conversion lives in the shell adapter** (`routes/orders/pricing.rs` survives as a thin shim).
   The shim owns `distance_km`, the f64 `DeliveryTier`/`FeeLocation` row shapes, the
   rate→`rate_micro` and pin/tier→meter conversions, and keeps its current f64 adapter signatures so
   `pg.rs` and `shifts.rs` (which imports `distance_km`) are effectively untouched. It is the single
   float chokepoint.

4. **Core errors carry `domain::ErrorCode`.** `PricingError { code: ErrorCode, message: String }`
   moves into `domain`. `ErrorCode`'s existing `SCREAMING_SNAKE_CASE` serde reproduces the exact wire
   strings; every needed code already exists in `domain::error`. This deletes the `pricing_code(&str)
   -> ErrorCode` mirror in `pg.rs`.

## Consequences

- Positive: no float can compile into core (build-time fail-closed, zero cascade); byte-parity on
  every realistic input; the crown-jewel INSERT (`pg.rs`) and courier geo path (`shifts.rs`)
  untouched; net-negative code (the `pricing_code` mirror removed); first real exercise of the F2
  purity gate.
- Negative / accepted: a sub-meter-precision tier boundary could diverge from the old float compare
  for a delivery within 0.5 m of it (accepted-risk R1; operationally unreachable; cheap compensating
  ≤3-dp tier-config validation recommended). Core now references `domain::ErrorCode` (accepted R2;
  fall back to a local `&'static str` error + defer to 0b-3 if the council wants the strictest
  mechanical move). git-blame on the shim's re-created lines is lost (R3).

## Alternatives rejected

- 1B/3B — dissolve the shim, integerize inside `pg.rs`: disperses the float boundary across a
  red-line hotspot and breaks the `shifts.rs` `distance_km` import.
- 2B — asymmetric floor/ceil rounding: changes the common whole-km boundary result and breaks the
  existing fee-ladder parity for the sake of an unreachable edge.
- 4B — keep `&'static str` codes in core: leaves a shell wire-convention in the core and the
  `&str→ErrorCode` drift-mirror in `pg.rs` (kept only as the strict-mechanical-move fallback).

## Verification (RED→GREEN)

- Purity: add an `f64` to `kernel::pricing` → `cargo clippy -p dowiz-core --lib` `disallowed_types`
  fails Gate 2 → revert.
- Parity: flip `charged_tax` to `tax_total` → core LC1 test + shim end-to-end vectors red → revert.
- Deployed: staging fixture-cart POST (incl. one mid-tier delivery distance) → DB totals vs
  hand-computed literals, `x-dowiz-cutover` asserted.

## Forward-only

Pure code move; no migration, no schema touch. Rollback = revert the commit.
