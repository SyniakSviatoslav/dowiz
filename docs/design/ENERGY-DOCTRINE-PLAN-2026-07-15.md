# Plan: Energy-Doctrine Kernel Spine (stabilized core ZERO)

Date: 2026-07-15. Branch: bebop feat/verification-harness. Status: PLAN (push before code).

## Decoding "stabilized core ZERO programming" (first-principles; web research BLOCKED — Firecrawl down)
The kernel is already a physics engine. `L = D − A` Laplacian = circuit/field energy operator.
- Circuit energy: `E(u) = ½·uᵀL·u = ½·Σ_{edges}(u_i − u_j)²` ≡ resistor-network dissipation power
  (Ohm: P = Σ(ΔV)²/R). The graph IS the circuit; L IS the conductance matrix.
- Wave/field evolution: `u(t) = exp(−coeff·L·t)·u0` is a contractive (dissipative) heat/wave op.
  Eigenvalues of exp(−coeff·L·t) ∈ (0,1] for coeff>0 ⇒ `E(u(t))` is NON-INCREASING.
- ZERO = (a) ground state: uniform u ⇒ ∇u=0 ⇒ E=0; (b) zero-allocation steady state: matrix-free
  propagator, no heap churn in hot loop.
- STABILIZED = energy-non-increasing invariant: `E(t) ≤ E(0)` always; field relaxes to ZERO.

>> NEEDS OPERATOR CONFIRM: does "ZERO programming" name a specific external framework? If so,
   provide a ref/keyword and I'll align. Otherwise the above physics-energy decoding is the spec.

## Current gap
`ACCUM: Mutex<(usize, Vec<f64>)>` tracks `Σ|Δu|` per node (a dissipation *proxy*) but the true
field energy `E = ½uᵀLu` is never computed/asserted. Energy doctrine is implicit, not the spine.

## Changes (additive, zero new deps, no_std/empty-import safe)
1. Add `field_energy(u: &[f64]) -> f64` helper = `½·uᵀ·(L·u)` via `field_matvec_raw` (O(nnz), no alloc).
2. Add `ENERGY: Mutex<(E0, E_last, E_cum)>` ledger (baseline energy, last energy, cumulative dissipated).
   Updated inside `field_spectral`/`field_active` (only when buffers sized to n). Does NOT touch ACCUM
   (keeps `field_sensitivity` intact).
3. Extend `field_metrics` 5-tuple → 8-tuple: append `[5] E_last [6] E0 [7] stabilize_ratio=E_last/E0`.
   (caller passes n>=8; old n=5 still works, returns 0 with partial fill — keeps ABI backward-safe?
   No: extend contract, bump probe to 8. Document.)
4. Relabel ACCUM doc as "energy dissipation proxy (Σ|Δu|)" — keep for sensitivity.

## Falsifiable verification (Verified-by-Math)
- `test_field_energy_stabilizes`: build path graph, u0 with nonzero gradient, record E0, propagate
  via `field_spectral`, record E1 ⇒ assert `E1 ≤ E0 + 1e-9` (contractive invariant).
  Math: `u(t)=exp(−cLt)u0`, `E(t)=½u(t)ᵀLu(t)=½u0ᵀexp(−2cLt)u0 ≤ ½u0ᵀu0` and since exp(−cLt)≤1
  spectrally, `E(t)≤E(0)`. Red+GREEN.
- Existing 24 tests stay green. wasm empty-import gate re-verified (no new imports).

## Out of scope
- External "ZERO framework" (unknown; pending operator confirm).
- Event-level traces (decart #5 — future).
- Disk (already reclaimed 91%→88%).

## Files
- `rust-core/src/lib.rs` (field_energy helper, ENERGY ledger, field_metrics ext, tests)
- `rust-core/examples/metrics_probe.rs` (print E0/E_last/ratio)
- `dowiz/tools/telemetry/telemetry` (kernel subcommand: parse new tuple)
- `docs/design/ENERGY-DOCTRINE-PLAN-2026-07-15.md` (this)
