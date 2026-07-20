# BLUEPRINT — Item 35: Fixed-Point Number-Format + Rounding-Law Spec

- **Date:** 2026-07-19 · **Arc:** §H · **Status:** BLUEPRINT v1 — planning artifact, NO code.
- **Governing ruling (arc-wide):** *"безпека і передбачуваність понад швидкість"* — here the
  ruling *is* the spec: symmetric quantization (fewer moving parts, simpler proofs), per-tensor
  scalar scale (every layer law stays scalar-expressible), i32 accumulators with **proven** bounds,
  `div_half_up` rounding, **refuse-never-fall-back** on any unprovable bound.
- **Sources read this session:** synthesis §2 Q5 (the resolved format) + §4 (P2 Correspondence: the
  requantization reuses `eqc_gen`'s already-tested `div_half_up`); roadmap §H item 35 (lines
  533–539, "every law a checkable equation; i8×i8 MAC exhaustively proven over all 65 536 pairs;
  overflow-bound lemma falsifiable per layer"); `kernel/src/eqc_gen.rs:30` (`apply_tax_exclusive_int`
  — the committed, tested `div_half_up` half-up organ); `tools/eqc-rs/src/lib.rs:601-607` (the
  `DivHalfUp` emitter: `(a + b/2)/b` in i128); the CORDIC Q30 fixed-point precedent (synthesis §1.1,
  `REGRESSION-LEDGER.md` row 25).
- **Dependency gate:** **after item 34** (the pilot fixes the concrete layer shapes whose overflow
  bounds this spec must prove). **Gates items 36, 37, 38, 41** (all target this format).

---

## 1. Scope / goal + non-goals

**Goal.** A spec doc where **every law is a checkable equation**, fixing the number format end-to-
end: i8-symmetric weights & activations, per-tensor power-of-two scale, i32 accumulation with a
**proven** per-layer no-overflow bound, `div_half_up` requantization, saturating clamp, and the
refuse-never-fall-back contract when a bound is unprovable.

**Non-goals.** No exotic formats (Q4_K blocks, asymmetric affine, f16 — Q5-rejected: they optimize
size/accuracy-per-bit, the speed/size axis the ruling subordinates). **No dequantize-on-the-fly**
(rejected outright: it reimports IEEE-754 into the hot path). No per-channel scale (keeps every
layer a scalar equation eqc can express). This item is a *spec*; the executable emission is item 36,
the executable oracle is item 37.

## 2. Current-state grounding

- **`div_half_up` is already generated and tested.** `kernel/src/eqc_gen.rs:30`
  (`apply_tax_exclusive_int`) is the committed integer-exact half-up organ; the emitter is
  `tools/eqc-rs/src/lib.rs:601-607` (`{ let b=...; if b==0 { Err }; (a + b/2)/b }` in i128). The
  requantization rounding law is therefore **not new** — it is P2-Correspondence reuse.
- **The 65 536-pair standard is the house standard**, literally. Synthesis Q5 names it: the i8×i8
  product space is exactly 65 536 pairs — the same scale the source dialogue cites as the exhaustive
  standard. The MAC law is *provable*, not sampled.
- **Refuse-never-fall-back is eqc's established stance** (`emit_fixed_rust` / `emit_int_checked_rust`
  return typed `Err`, never a silent fallback — `lib.rs:360,393,413`). The overflow-bound refusal
  inherits it.

## 3. The laws (each a checkable equation)

Let `s_x` = per-tensor scale of tensor `x` (a positive power of two by preference, `s_x = 2^{-e}`).

1. **Quantize** (real `r` → i8 `q`): `q = clamp(round_half_up(r / s_x), Q_MIN, Q_MAX)`.
   **Symmetric ⇒ zero-point = 0.** Range decision in §6.
2. **Dequantize** (conceptual only — NEVER in the hot path): `r ≈ q · s_x`. Present so the oracle
   can relate integer results to reals; the engine never executes it.
3. **Multiply-accumulate** (the core law): for a dot of length `K`,
   `acc : i32 = Σ_{k=0}^{K-1} (a_k · w_k)`, with `a_k, w_k ∈ [Q_MIN, Q_MAX]` i8.
   Each product is exact in i16/i32; the sum is accumulated in i32.
4. **Overflow-bound lemma (per layer, falsifiable):** with `P_MAX = max product magnitude`, the
   accumulation is overflow-free in i32 iff `K · P_MAX ≤ 2^31 − 1`. For the restricted-symmetric
   range `[−127, 127]`: `P_MAX = 127² = 16 129`, so `K ≤ ⌊(2^31−1)/16 129⌋ = 133 144`. For the
   full range `[−128, 127]`: `P_MAX = 128² = 16 384 = 2^14`, so `K ≤ 2^17 − 1 = 131 071`. **Both
   ceilings are ≫ any toy-pilot layer width (`K ≤ 64`)** — the bound is comfortably met, and the
   spec REFUSES (typed `Err`) any layer whose `K` exceeds its ceiling.
5. **Requantize** (i32 `acc` → i8 for the next layer): with combined scale
   `S = (s_in · s_w) / s_out`:
   - if `S` is a power of two `2^{-r}`: `q_out = clamp(div_half_up(acc, 2^r), Q_MIN, Q_MAX)` —
     an arithmetic right shift with half-up rounding (the `eqc_gen` `div_half_up` organ verbatim).
   - else (general case, documented fallback): `q_out = clamp(div_half_up(acc · M, 2^{31}), …)`
     with a fixed i32 multiplier `M = round(S · 2^{31})` — the standard fixed-point-multiplier form,
     still `div_half_up`, still integer-exact.
6. **Saturating clamp** is **saturating, not wrapping**: `clamp(v, Q_MIN, Q_MAX)` returns the
   boundary, never a wrapped value. Overflow of the *clamp input* is a bug the item-4 lemma forbids.

## 4. Required proofs (5-point hardening-checklist mapping)

- **1 (oracle) — the load-bearing proof:** the i8×i8 MAC law **exhaustively proven over all 65 536
  `(a,w)` pairs** (literally the whole product domain), plus the per-layer overflow-bound lemma as
  a checkable test parameterized on `(K, P_MAX)`. A deliberately-wrong rounding (truncate instead
  of half-up) or a wrong `P_MAX` fails.
- **3 (debug/differential):** where a per-call reference exists (requantize vs a wide-accumulator
  i128 shadow), `debug_assert_eq!` — compiled out of release. The MAC/overflow lemma is
  corpus-style; those rows carry `N/A(corpus-oracle)` per CHECKLIST.md.
- **2 (dudect):** deferred — the format spec has no timing surface; item 43 owns the constant-time
  gate (public plane for the toy pilot ⇒ cheap branch).
- **4 (asm) / 5 (kani):** N/A for the spec; item 39 carries the asm spot-check on the emitted
  kernel, item 36's `emit_proof_program` carries the codegen self-assert.

## 5. Falsifiable acceptance criteria

1. A spec doc exists with **every law (§3) written as a checkable equation**.
2. The i8×i8 MAC law is **exhaustively tested over all 65 536 pairs** with **zero divergence** from
   the wide-accumulator (i128) shadow. **RED→GREEN:** injecting truncation-instead-of-half-up
   rounding turns it RED.
3. The overflow-bound lemma is a falsifiable per-layer test: `K · P_MAX ≤ 2^31−1` is asserted for
   each pilot layer, and a synthetic layer with `K` one past its ceiling is **REFUSED** (typed
   `Err`) — never silently accepted (**RED→GREEN**).
4. The requantization organ is `div_half_up` reused from `eqc_gen` (no second rounding impl) — a
   parity test asserts bit-equality with the committed `apply_*_int` half-up form on shared inputs.
5. Saturating clamp proven saturating (not wrapping) at the i8 boundaries by an enumerated
   boundary test.

## 6. Dependency gate + operator-decision-needed

- **Gate:** after item 34 (needs concrete layer shapes); gates items 36/37/38/41.
- **Operator-decision-needed — FLAGGED, two format choices with real consequences:**
  1. **i8 range: restricted-symmetric `[−127,127]` vs full `[−128,127]`.** Restricted gives *exact*
     symmetry (no `−128` asymmetry, cleanest proofs, `P_MAX=16 129`); full gives one more code and a
     power-of-two `P_MAX=2^14` (cleaner overflow arithmetic). **Architect recommendation:**
     restricted `[−127,127]` — exact symmetry is the more predictable choice the ruling favors, and
     `K≤133 144` is not a real constraint at pilot scale. Flagged for the operator/spec-author to
     confirm; **not invented**.
  2. **Power-of-two-scale-only vs general fixed-point multiplier `M`.** Power-of-two = pure
     arithmetic shift (fewest ops, most predictable); general `M` = broader accuracy but a multiply.
     **Recommendation:** power-of-two-preferred with the general-`M` path documented as the fallback
     (both are already `div_half_up`-expressible, so no new rounding surface either way). Flagged.
- Neither choice blocks downstream sequencing — both are expressible by the item-36 emitter; they
  change the constants, not the pipeline.
