# BLUEPRINT P31 — Math-First Kernel residuals: S0–S7 + Master-Integration Tier A/B closure (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map §9). This phase IS
> `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.5.1's **P31** (sub-letters
> P31a–P31e). Framing, per §10.5.1 itself: CORE is ~90% done and NOT the bottleneck — this
> blueprint **formalizes and cross-links** existing material; it invents no new design. The real
> design work lives in the cited sources and is reused, never re-derived here:
> `docs/design/math-first-architecture-blueprint.md` (S0–S7 charter),
> `docs/design/BLUEPRINT-EIGENVECTOR-REFACTOR-PLAN-2026-07-17.md` (eigensolver design),
> `docs/design/MASTER-INTEGRATION-PLAN-2026-07-14.md` (A1/A2/B1–B5 Tier A/B),
> `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P-A-kernel-primitives.md` (Layer-A DoD, incl.
> §11's 2026-07-18 codegen-leg status correction — load-bearing for P31a/P31b below).
> Lower urgency than P34/P37/P38/P40/P41/P45 — nothing in P31 is on the critical path.
>
> **Headline ground-truth findings of this pass:** (i) the eigensolver consolidation (S1) and
> the BumpArena both landed on `main` **today** (`03ac0fefe`, `5d61d097a`) — cites below are
> fresh, not inherited; (ii) P31e's §10.5.1 premise "`online.rs` connected only by a comment"
> is **stale**: `evals.rs` E3 now drives `ScalarAdam` from real kernel signals (§0 row 15) —
> the remaining P31e gap is only the eqc-runtime-IR half; (iii) P31b remains a genuine
> WIRING-GAP (refusal arms live at `tools/eqc-rs/src/lib.rs:610-611`).

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Verified 2026-07-18 against `dowiz` `main` @ `76167336a7b2ed31fee38d7161d109d462763643`
(working tree carries docs-only modifications — zero `.rs` diffs, code cites unaffected).
All paths relative to `/root/dowiz/`.

| # | Claim | Fresh `file:line` (this pass) | Inherited cite (§10.5.1) | Status |
|---|---|---|---|---|
| 1 | S0 eqc compiler + generated organs | `tools/eqc-rs/src/lib.rs` (`Asin`/`Atan2`/`DivHalfUp` `:68,71,75`, `emit_int_checked_rust` `:393`); generator binary `tools/eqc-rs/src/bin/gen_kernel_organs.rs` (exists, verified `ls`); `kernel/src/eqc_gen.rs:8-13` `ema_next_f64`, `:26-40` `apply_tax_{exclusive,inclusive}_int` | §10.5.1: "16/16 tests" | **MATCH** — P-A §11.0 verified the full A1/A2/A3/A6 ledger 2026-07-18 (16 passed, 1 declared-ignored cross-arch leg); not re-run here, cites live |
| 2 | S1 eigensolver consolidation | `kernel/src/spectral.rs:239` `pub fn topk_symmetric(a: &Csr, k, iters) -> Decomp`; `kernel/src/householder.rs:387` `pub fn eigh_contig(a: &mut [f64], n) -> (Vec<Vec<f64>>, Vec<f64>)`; `eig2x2` `householder.rs:211` (**private** helper fn, bit-capture oracle test `:1010`); commit `03ac0fefe` "deterministic symmetric eigensolver + sparse topk (BLUEPRINT-EIGENVECTOR-REFACTOR R1-R3)" — landed **today**, ancestor of HEAD | §10.5.1: same symbols | **MATCH, sharpened** — `eig2x2` is private, not an exported API; the 3-duplicate-Jacobi dual-authority hazard is closed |
| 3 | S2 money = integer | `kernel/src/money.rs:22` `MONEY_SCALE_MICRO`; `apply_tax` `:269-287` (hand law, authority); shadow organs in `eqc_gen.rs` row 1 | §10.5.1: money.rs/domain.rs/cart.rs all i64 | MATCH (spot-verified; full-file i64 sweep inherited from P-A §0) |
| 4 | S4 zero-copy bridge (CORE side) | `engine/src/bridge.rs:69` `pub struct VertexBridge` | §10.5.1: same | **MATCH** |
| 5 | A2(mip) native retrieval | `kernel/src/retrieval/` = **2879 lines** (fresh `wc`); `kernel/src/lib.rs:177` `pub mod retrieval`; `retrieval/recall.rs:202` `pub struct PrimaryRecall`; `retrieval/index.rs:82` `pub struct TrigramIndex` (=B5) | §10.5.1: "wired at `kernel/src/lib.rs:151,176`" | **DRIFT** — module registration moved to `:177` (lib.rs grew); line-count 2879 exact match |
| 6 | B1 Kalman | `kernel/src/kalman.rs:149` `pub struct KalmanFilter`, `scalar()` `:188` | §10.5.1: same | **MATCH** |
| 7 | B2 micrograd | `kernel/src/micrograd.rs:193` `pub fn backward` | §10.5.1: `Value::backward` | **MATCH** |
| 8 | B4 backup organ | `kernel/src/backup.rs` = **702 lines** (fresh `wc`) | §10.5.1: "702 lines" | **MATCH exact** — still never exercised end-to-end (OPS/DELIVERY dependency, unchanged) |
| 9 | Kernel suite | `cd kernel && cargo test --lib` → **561 passed, 0 failed, 0 ignored** (run this pass, 0.74 s) | P-A §11.0: "540 passed" (2026-07-18, earlier) | **FRESH** — +21 since the P-A §11 pass (arena + eigensolver tests landed between); GROUND-TRUTH-2026-07-17's "452" is two generations stale |
| 10 | **NEW: BumpArena landed** | `kernel/src/arena.rs` (11 KB): `pub struct BumpArena` `:43`, `alloc_slice` `:75`, `reset` `:112`, `high_water` `:120`; `kernel/src/lib.rs:49` `pub mod arena`; commit `5d61d097a` "BumpArena + arena-aware CSR rebuild pass (BLUEPRINT W5 / Phase 28)" — landed **today** | not in §10.5.1 (drafted before it landed) | **NEW** — Phase-28/W5 scope, NOT a P31 build item; recorded so no P31 reader treats the arena as open, and because standard item 16 names it as reusable machinery |
| 11 | P31b gap is live | CORDIC exists: `tools/eqc-rs/src/cordic.rs:33` `pub const ITERS: u32 = 31` (pure-integer Q30; digest tests green per P-A §11.0); refusal arms live: `tools/eqc-rs/src/lib.rs:610` `Expr::Sin(_) => Err(IntEmissionUnsupported(…))`, `:611` `Expr::Cos(_)` — int-mode emission does NOT route through CORDIC. **Twin site:** the Q-format (fixed-point) subset refuses too, `:533-534` | §10.5.1: WIRING-GAP; P-A §11.3.3 names `:610-612` | **MATCH** — gap confirmed open; the `:533-534` twin is a new sharpening (see §3.2) |
| 12 | P31c genuinely not started | `grep -rn no_std kernel/src/lib.rs kernel/Cargo.toml` → **0 hits** (fresh) | §10.5.1: "zero `#![no_std]` anywhere" | **MATCH** |
| 13 | P31d genuinely not started | `grep -rli "kani\|z3"` over `kernel/ tools/ .github/` → only `node_modules` junk, **0 real hits** (fresh) | §10.5.1: "zero hits repo-wide" | **MATCH** |
| 14 | P31e learner organs | `kernel/src/online.rs:28` `pub struct LinearSGD`, `:82` `pub struct ScalarAdam`; `kernel/src/lib.rs:172` `pub mod online` | §10.5.1: "connected only by a comment" | see row 15 |
| 15 | **STALE CORRECTION: online.rs is no longer comment-only** | `kernel/src/evals.rs` E3 ("Self-adaptation") **drives** `ScalarAdam`: field `opt: crate::online::ScalarAdam` `:795`, constructed `:809`, proof note `:1253`; header `:746` "Un-strands the two STRANDED learner organs". `LinearSGD` remains comment-only (`:746,:771` prose; zero call sites outside `online.rs`, fresh grep). **Zero eqc runtime-IR in kernel** (fresh grep: no `Expr`/IR type) | §10.5.1 P31e: "the two are connected only by a comment" | **DRIFT** — half the §10.5.1 gap closed by the swarm (ScalarAdam driven by evals); the eqc-IR↔learner bridge (the actual S6/B3 item) remains fully open. §3.5's DoD is re-anchored accordingly |
| 16 | P06 key_V context | `tools/ci-truth/src/v1.rs:405+` `HybridSigner` (`signed() -> true` `:416`, real `bebop2-kv sign/verify` shell-out, fail-closed empty-sig on error); commit `58987d79d` "P06 key_V HybridSigner — real bebop2-kv sign/verify + TLV sig field" — landed **today** | older CORE drafts treated P06 as open | **CLOSED** — never a P31 dependency; recorded so no reader inherits "P06 still open" into P31 sequencing |

Ground truth is non-discussible; everything below builds on the fresh column only.

---

## 1. Scope — the five sub-letters, and what P31 deliberately does NOT own

**P31's single sentence:** close the last residuals of the math-first rewrite — index the DONE
body (P31a), route int-mode trig through the existing CORDIC (P31b), make the kernel
`no_std`-capable (P31c), start the machine-checked-proof ladder (P31d), and bridge eqc's
equations to the online learner (P31e) — all off the critical path.

| Sub | Absorbs | Status (fresh, §0) | Character |
|---|---|---|---|
| P31a | S0, S1, A3(mip), S2, S4, S7, A1(mip), A2(mip), B1, B2, B4, B5 | DONE | index-only, zero build |
| P31b | A6 residual | WIRING-GAP (rows 1, 11) | one emitter routing change + tests |
| P31c | S3 residual half (SIMD half DONE: `kernel/src/simd.rs`) | NOT STARTED (row 12) | feature-gated split |
| P31d | S5 | NOT STARTED (row 13) | inventory + first proof harness |
| P31e | S6 = B3 (same item, counted once) | PARTIAL (rows 14-15) | minimal runtime-IR + bridge |

**Anti-scope (binding):**
- **Do not touch any P31a organ.** No "improving" DONE code while wiring gaps exist elsewhere
  (§10.5.1's own rule). The A3 authority flip (`apply_tax` callers → generated organs) stays
  operator-docket **R-4** gated — P-A §11.3.6, unchanged here.
- **Do not hand-edit `kernel/src/eqc_gen.rs`** — it is a compiled artifact (its header is the
  enforcement; P-A §11.2's provenance argument).
- **Do not rewrite CORDIC** (P31b wires, never rebuilds), do not widen the integer-exact subset
  beyond Sin/Cos, do not touch float-mode emission.
- **Do not chase whole-kernel `no_std`** — I/O organs get feature-gated, not rewritten (P31c).
- **Do not attempt whole-kernel verification** — ladder = cheapest rung first, money organs
  before anything else; proof tooling is CI-only, never a build dependency (P31d).
- **Do not port all of eqc-rs into the kernel** — P31e takes the minimal IR subset the existing
  organs need; compiled organs stay canonical, IR is only the learning surface.
- **The arena (§0 row 10) is Phase-28/W5 scope** — P31 neither extends nor depends on it.
- Nothing here blocks or serializes P34/P37/P38/P40/P41/P45.

---

## 2. Predefined types & constants (standard item 4)

P31a needs none (index-only). P31c/P31d introduce features/harnesses, not domain types. The two
sub-letters that introduce anything are named here before any implementation:

```rust
// ── P31b: no new public types. The emitter decision, stated as spec ──
// emit_int_checked_rust's Sin/Cos arms stop refusing and emit a SELF-CONTAINED
// inline Q30 CORDIC routine into the generated output (the RGB-seed /
// procedural-encoding pattern, standard item 16: equations stored as data).
// DECART one-liners for the rejected alternatives:
//   - kernel grows a `cordic` module → rejected: breaks the proven isolation
//     invariant "generated code has zero runtime dependency on the compiler
//     crate" only in spirit, but DOES add a second authority for the same
//     routine (tools/eqc-rs/src/cordic.rs stays the source of truth);
//   - emitted code calls `eqc::cordic` → rejected: creates the runtime dep the
//     isolation grep (P-A §11.0: `grep eqc kernel/Cargo.toml` → none) forbids.
// The emitted routine's digest must equal cordic.rs's pinned CORDIC_SINCOS_DIGEST
// (same Q30, same ITERS=31) — one implementation, two homes, digest-locked.

// ── P31e: the ONLY new production types (kernel/src/eq_ir.rs, minimal) ──
/// Runtime image of an eqc-compiled equation: enough to represent the already-
/// generated organ class (EMA + the two tax forms), nothing more (anti-scope).
pub struct EqIr {
    pub ops: Vec<IrOp>,          // postfix walk of the Expr tree, fixed at build
    pub params: Vec<f64>,        // the LEARNABLE slots (e.g. α in EMA)
}
/// Op vocabulary = exactly the Expr subset the generated organs use today.
pub enum IrOp { Const(f64), Param(u16), Var(u16), Add, Sub, Mul, Div, DivHalfUp }
/// Determinism law (DoD-4): with params untouched, `EqIr::eval` output is
/// bit-identical (`to_bits()`-exact) to the corresponding eqc_gen organ.
```

---

## 3. Build items — spec → RED test → code, each with an adversarial case (items 3, 5)

### 3.1 P31a — DONE ledger (zero build; the index is the deliverable)

The one-line ledger stands as written in §10.5.1, with §0's fresh cites replacing the inherited
ones (notably: retrieval registration `lib.rs:177` not `:151,176`; kernel suite **561** not
452/540). Two open paperwork items are **named, not owned**: (i) `REGRESSION-LEDGER.md` row 25's
CORDIC-consumption claim mismatches live source, (ii) no ledger rows exist for A2/A3 parity
pins — both flagged in P-A §11.3.4-5, owned there. S7 stays PARKED→PROTOCOL; A1(mip) stays
MOOT; B4's end-to-end exercise stays an ECOSYSTEM/OPS dependency. No RED test exists because no
change exists — the falsifier for this section is §5's ledger row (any P31a file diff = scope
violation).

### 3.2 P31b — CORDIC int-mode emission (A6 residual)

**Spec:** §2's inline-emission decision. `emit_int_checked_rust`'s `Sin`/`Cos` arms
(`lib.rs:610-611`) emit the Q30 CORDIC routine instead of `Err(IntEmissionUnsupported)`.
**The `:533-534` twin (Q-format/fixed-point subset):** out of §10.5.1's charter (int-mode only).
It stays refusing **unless** the implementer records a dated decision to wire it too — CORDIC is
natively Q30, so wiring it is plausibly cheaper, but that is a recorded decision, never a silent
scope expansion (standard item 19's honesty clause).

**RED test (eqc-rs suite, next to the existing 16):**
```rust
#[test]
fn int_mode_sin_cos_routes_through_cordic() {
    // RED today: emit_int_checked_rust(Sin(x)) == Err(IntEmissionUnsupported).
    // GREEN: emitted source contains the inline Q30 routine, and evaluating the
    // emitted organ at the cordic_digest fixture angles reproduces
    // cordic::cordic_sincos's exact outputs (integer-exact, not approx).
}
```
**Adversarial:** (i) reuse the `cordic_digest_has_teeth` pattern — an `ITERS−1` mutation of the
emitted routine must break the digest parity (teeth proven, not asserted); (ii) the OTHER
refusal arms (`Sqrt`/`Exp`/`Asin`/`Atan2`, `lib.rs:609,612-615`) must STILL refuse — a test
asserting each returns `Err`, so the subset cannot silently widen.
**DoD:** §10.5.1 P31b's two items kept 1:1 (emitted-output inspection + 16/16 baseline green
plus the new test), sharpened in §5.

### 3.3 P31c — no_std kernel (S3 residual)

**Spec (from §10.5.1, unchanged):** `#![cfg_attr(not(feature = "std"), no_std)]` split; `std`
default feature; I/O-bound modules (`backup.rs`, `retrieval/memory_store.rs`, `intake.rs`'s
file legs if any, `evals.rs` where it touches I/O) gated behind `std`, not rewritten.
**First DoD step is the alloc-boundary inventory, produced mechanically:** add the `cfg_attr`
gate on a scratch branch and let `cargo build --no-default-features` enumerate every `std`
surface as compile errors — the error list IS the inventory (write it into the PR description,
not a separate doc; §10.5.1's explicit instruction).
**RED:** `cargo build --no-default-features` today does not even exercise the property (no
feature exists) — the RED is structural: the CI check of §5 does not exist. **GREEN:** the
build passes with the gate live and the full suite (561 baseline, §0 row 9) still green under
default features — zero behavioral drift.
**Adversarial:** a deliberately added `use std::vec::Vec;`-free violation (e.g. `std::fs` call)
inside a non-gated module on a scratch branch must fail the `--no-default-features` CI build —
proving the ratchet has teeth before it is trusted.

### 3.4 P31d — Verification ladder (S5)

**First deliverable IS the inventory + tool choice** (§10.5.1: "the first deliverable IS the
blueprint"). To keep that deliverable from starting blank, the candidate-invariant list is
pre-seeded here (rank = red-line severity × proof cheapness; the executor refines, not
re-derives):
1. `apply_tax_exclusive_int`/`apply_tax_inclusive_int` (`eqc_gen.rs:26-40`): no overflow, never
   negative, over the documented input domain — money red-line, checked-int arithmetic makes it
   the cheapest rung.
2. `money.rs` ledger append/reversal: conservation — `ledger_balance` returns to 0 after a
   posting and its reversal leg.
3. `order_machine::fold_transitions`: no panic for any input sequence; determinism (same input,
   same output).
4. `geo.rs::ema_next` / `eqc_gen::ema_next_f64`: output bounded by min/max of (prev, x) for
   α ∈ [0,1].
5. `MONEY_SCALE_MICRO` scaling round-trip: representable amounts survive scale/unscale with
   zero loss.

**Tool choice** (kani vs z3) must be an honest falsifiable comparison per the tech-selection
rule (`rust-native-bare-metal-decision-2026-07-14`) — expected default is kani (proofs live
next to the Rust source, CI-runnable), but the comparison is run, not assumed.
**RED (mandated by §10.5.1 DoD-3):** a deliberately introduced off-by-one in a scratch branch
makes the landed proof fail — the proof is demonstrated RED-able before it counts.
**Adversarial:** the proof harness run against a widened input domain (outside the documented
one) is EXPECTED to fail or be vacuous — recording that boundary honestly is part of the
deliverable (a proof that passes on ℤ unbounded would be claiming too much).

### 3.5 P31e — Equation-IR at runtime + online-learner bridge (S6 = B3)

**Re-anchored on §0 row 15:** the "learner is stranded" half of §10.5.1's framing is done
(evals.rs E3 drives `ScalarAdam`); what remains is exactly the S6/B3 item: **no runtime
representation of eqc's equations exists for a learner to adjust.** The §10.5.1 DoD-3 phrasing
("the comment-only link … replaced by an actual call path") is re-scoped: the call path that
must newly exist is **IR ↔ learner** (`eq_ir.rs` symbol referenced from `online.rs`-driving
code), not merely "online.rs has any consumer" (that is now trivially true and would be a
fake-green).

**Spec:** §2's `EqIr`/`IrOp` — postfix eval, learnable `params`, op vocabulary frozen to the
generated-organ class. A short design note (which parameters are learnable per organ; EMA's α
is the pilot) precedes code, per §10.5.1.
**RED tests:**
```rust
#[test] // DoD-4 determinism pin — written FIRST, red against a stub eval
fn eq_ir_ema_bit_identical_to_compiled_organ() {
    // EqIr for ema_next over the 21-case EMA_PARITY_FIXTURES (geo.rs:518):
    // with params = [α] untouched, eval == eqc_gen::ema_next_f64, to_bits()-exact.
}
#[test] // DoD-2 — the bridge does something real
fn scalar_adam_updates_eq_ir_param() {
    // ScalarAdam (online.rs:82) minimizes a loss over EqIr's α across a synthetic
    // series; assert α moved AND post-update eval differs from the compiled organ
    // (learning is observable), while a zero-lr run stays bit-identical.
}
```
**Adversarial:** (i) an `IrOp` sequence that underflows the eval stack (malformed program) must
return `Err`, never panic or produce a partial value — fail-closed eval; (ii) teeth: corrupt one
`Const` in the EMA program and assert the determinism pin FAILS — proving the bit-identical
test binds the program, not just the inputs.
**DoD:** §10.5.1's four items kept, with DoD-3's grep target sharpened to the IR bridge (§5).

---

## 4. Cross-cutting obligations (items 6, 8, 9, 11–16)

- **Hazard-safety as math (6):** P31b — the digest lock (emitted CORDIC ≡ `cordic.rs` by pinned
  digest) makes a divergent second trig authority unrepresentable-in-CI, the exact drift class
  A4/A5 existed to kill (P-A §11.1). P31d — converts red-line prose invariants into
  machine-checked artifacts; its RED-ability requirement guards against vacuous proofs. P31e —
  the bit-identical pin means the learning surface cannot silently fork the Law: with learning
  off, IR ≡ compiled organ or CI is red. P31c — `no_std` removes ambient-I/O reachability from
  gated modules at the type-system level.
- **Scaling axes (8):** P31e's `EqIr` scales in ops-per-program (today ≤ ~20 for EMA/tax; the
  postfix Vec walk is O(ops) with zero allocation per eval after construction) — the stated
  break point is "organs with branching/loops", which the op vocabulary cannot represent by
  design (that is the anti-scope, not a TODO). P31c/P31d have no data shapes.
- **Linux-discipline verdicts (9):** compiler-as-gatekeeper via digest/parity pins =
  **ALREADY-EQUIVALENT**; CI-only proof tooling (P31d) = **EXTENDS** (new machinery, justified:
  red-line money organs); feature-gated `no_std` split = **REINFORCES** (one codebase, explicit
  configuration, no fork).
- **Isolation/bulkhead (11):** P31b touches only `tools/eqc-rs` (+ regenerated `eqc_gen.rs`
  output pasted per the established provenance flow) — kernel manifest untouched. P31e adds one
  kernel module with zero new external deps. P31d's tooling is CI-only. Blast radius of each is
  its own test suite.
- **Mesh-networking awareness (12):** **N/A for P31 except one line** — P31c is the soft
  prerequisite for embedding the kernel in WASM/mesh substrates (§10.5.1); nothing here touches
  transport, payloads, or gossip. Stated, not padded.
- **Rollback/self-healing vocabulary (13):** P31 claims only **Self-Termination /
  unrepresentable-state** (refusal arms, fail-closed IR eval, digest locks, exhaustive subsets).
  No Self-Healing or Snapshot-Re-entry is claimed — nothing here is a recovery mechanism.
- **Error-propagation gates (14):** the specific gates, per item: P31b — the still-refusing-arms
  adversarial + digest parity in the eqc CI job; P31c — the `--no-default-features` CI build;
  P31d — the proof harness in CI; P31e — the bit-identical parity pin in `cargo test --lib`.
- **Living memory (15):** A2(mip)'s retrieval stack (§0 row 5) IS the living-memory organ —
  cross-ref `internal-retrieval-living-memory-arc-2026-07-14`; P31 adds no new data with
  temporal/topological access patterns.
- **Tensor/spectral + eqc reuse (16):** this phase is largely ABOUT that machinery — S1 feeds
  `spectral.rs`/`spectral_cache`, P31b extends the eqc emitter, P31e stores equations as data
  (the RGB-seed/procedural pattern, §2). The Phase-28 arena (§0 row 10) is available to future
  CSR consumers but not used by any P31 item.

---

## 5. DoD — falsifiable, RED→GREEN, per item (item 2)

§10.5.1's DoD kept 1:1, sharpened with commands; P31e DoD-3 re-anchored per §0 row 15.

| Item | RED (fails before) | GREEN (passes after) | Command / falsifier |
|---|---|---|---|
| P31a | n/a — index only | ledger cites match live source (this file §0) | falsified by any P31a-organ file diff attributed to P31 (`git log --oneline -- kernel/src/money.rs …` shows none tagged P31) |
| P31b-1 | `emit_int_checked_rust(Sin(..))` → `Err` (`lib.rs:610`, live) | emitted int-mode output contains the inline Q30 routine | inspect generated output; falsified by any remaining `IntEmissionUnsupported("sin…")` arm |
| P31b-2 | new test absent | `cd tools/eqc-rs && cargo test --release` green: 16/16 baseline + `int_mode_sin_cos_routes_through_cordic` + still-refusing sweep + ITERS−1 teeth | falsified by teeth check not failing under mutation |
| P31c-1 | no `no_std` gate exists (§0 row 12) | `cargo build --no-default-features` green in `kernel/` | the command itself |
| P31c-2 | — | `cargo test --lib` under default features: 561-baseline green, zero drift | count regression = red |
| P31c-3 | — | I/O modules feature-gated, not rewritten (diff inspection: `#[cfg(feature = "std")]` only, no logic changes) | falsified by any I/O-organ body diff |
| P31c-4 | no CI check | `--no-default-features` build step in `.github/workflows/ci.yml` + the §3.3 adversarial recorded RED once | job absent = not done |
| P31d-1 | no inventory | ≥5 invariants ranked, tool chosen by falsifiable comparison (§3.4 seed refined) | doc grep for the comparison table |
| P31d-2 | no proof harness | ≥1 proof (money organs first) running in CI | CI job green |
| P31d-3 | — | scratch-branch off-by-one makes the proof FAIL (recorded) | falsified by no RED-run evidence |
| P31e-1 | no runtime-IR type (§0 row 15) | `kernel/src/eq_ir.rs` represents EMA + both tax forms | `grep -n "pub struct EqIr" kernel/src/eq_ir.rs` |
| P31e-2 | — | `scalar_adam_updates_eq_ir_param` green (learner demonstrably moves a param) | named test in `cargo test --lib` |
| P31e-3 | only evals→ScalarAdam exists (not the eqc bridge) | a real symbol path IR↔learner: `grep -rn "eq_ir::" kernel/src/ \| grep -v eq_ir.rs` ≥ 1 hit in learner-driving code | falsified if the only consumers are tests |
| P31e-4 | — | `eq_ir_ema_bit_identical_to_compiled_organ` green (learning off ⇒ bit-identical) + teeth mutation red-run recorded | falsified by teeth not failing |

Regression rows (item 17) for `docs/regressions/REGRESSION-LEDGER.md` on completion:
"P31b emitted-CORDIC digest parity — guardrail: eqc-rs test", "P31c no_std build —
guardrail: CI `--no-default-features` step", "P31e IR≡organ determinism pin — guardrail:
kernel unit test". (Also fold the row-25 correction flagged in P-A §11.3.4 when touching the
ledger — one visit, per the not-revisit-twice rule.)

---

## 6. Benchmark plan (item 10) — light, per the lower-urgency framing

- **P31b:** codegen is build-time; no runtime hot path changes. The only number worth one
  criterion run: emitted-CORDIC sincos vs `f64::sin/cos` at the digest fixture angles, recorded
  once in the eqc-rs test output (context, not a gate).
- **P31c:** zero runtime change by construction (gates only); the proof is the 561-baseline
  staying green + existing kernel benches flat (same flatness discipline as P34 §6.3). Build-
  matrix time cost of the extra CI step: note it, don't gate it.
- **P31d:** proof runtime is CI-budget, not product perf — record wall-clock per proof so the
  ladder's cost curve is known before adding rungs.
- **P31e:** ONE criterion bench: `eq_ir/eval_ema` vs `eqc_gen/ema_next_f64` — the interpreter
  tax, measured not estimated. IR is the learning surface, not the hot path; compiled organs
  stay canonical (anti-scope), so the number is recorded in `kernel/benches` history, not
  budgeted against.

---

## 7. Links to docs & memory (item 7)

Depends on / cites: `CORE-ROADMAP-STANDARD-2026-07-17.md` (contract) ·
`MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.5.1 (the P31 charter this file
formalizes; cross-reference summary at its end: S0–S7/A/B unit → P-number map) ·
`docs/design/math-first-architecture-blueprint.md` (S0–S7 source; NOT re-derived) ·
`docs/design/BLUEPRINT-EIGENVECTOR-REFACTOR-PLAN-2026-07-17.md` (S1 design; R1–R3 landed
`03ac0fefe`) · `docs/design/MASTER-INTEGRATION-PLAN-2026-07-14.md` (A/B tier source) ·
`BLUEPRINT-P-A-kernel-primitives.md` §11 (A1/A2/A3/A6 live status + the two ledger paperwork
flags §11.3.4-5 + the A3/R-4 authority gate §11.3.6) · `docs/regressions/REGRESSION-LEDGER.md`
(item-17 mechanism; row 25 needs correction per P-A §11.3.4) ·
`docs/design/GROUND-TRUTH-2026-07-17.md` (test-count lineage; superseded by §0 row 9's 561).
Memory: `math-first-architecture-arc-2026-07-14` (S0..S7 arc) ·
`integration-research-tf-attention-circuit-kalman-arc-2026-07-14` (Kalman-first; B1–B5) ·
`internal-retrieval-living-memory-arc-2026-07-14` (§4 item 15) ·
`rust-native-bare-metal-decision-2026-07-14` (P31d tool-choice discipline; DECART one-liners
§2) · `verified-by-math-2026-07-07` (P31d is this rule converted into artifacts) ·
`performance-priority-over-minimal-change-2026-07-17` (scoped to perf only — P31 is NOT a perf
arc; minimal-change bias applies here). Supersedes: nothing — §10.5.1 remains the charter,
this file the execution spec.

---

## 8. Hermetic principles honored (item 20)

- **P2 CORRESPONDENCE** (one concept, one primitive): one eigensolver family
  (`topk_symmetric`/`eigh_contig`, §0 row 2) replacing three Jacobi forks; one trig authority
  (CORDIC, digest-locked across its two homes, §3.2); one equation source of truth
  (eqc `Expr`) for compiled organ AND runtime IR (§3.5's bit-identical pin).
- **P6 CAUSE-AND-EFFECT** (determinism as law): every P31 deliverable carries a determinism
  falsifier — digest pins, `to_bits()`-exact parity, proof harnesses; none is asserted prose.
- (Other principles not load-bearing here; not claimed decoratively, per Anu/Ananke.)

---

## 9. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 — 16 rows, fresh cites, 3 drifts + 2 today-landed commits + 1 stale-premise correction (row 15) |
| 2 DoD | §5 — RED→GREEN per item, §10.5.1 kept 1:1, P31e-3 re-anchored with justification |
| 3 spec/TDD | §2 spec first; §3 RED tests precede code per sub-letter |
| 4 predefined types | §2 — `EqIr`/`IrOp` + the P31b emission decision as spec; P31a/c/d need none (stated) |
| 5 adversarial tests | §3.2 (still-refusing sweep, ITERS−1 teeth), §3.3 (std-leak injection), §3.4 (off-by-one RED, domain honesty), §3.5 (stack underflow, Const-corruption teeth) |
| 6 hazard-safety as math | §4 first bullet — unrepresentability arguments per sub-letter |
| 7 links | §7 |
| 8 scaling axes | §4 — EqIr ops axis + its designed break point; others N/A stated |
| 9 Linux discipline | §4 — three verdicts in the adopted framework |
| 10 benchmarks | §6 — measured-not-estimated, flatness for gate-only changes, interpreter tax pinned |
| 11 isolation | §4 — per-item blast radius; kernel manifest untouched by P31b |
| 12 mesh awareness | §4 — explicit near-N/A with the one real line (P31c enables embedding) |
| 13 rollback vocabulary | §4 — Self-Termination only, claimed precisely |
| 14 error-propagation gates | §4 — named CI/compile gate per sub-letter |
| 15 living memory | §4 — retrieval IS the organ; no new temporal data |
| 16 tensor/spectral/eqc | §4 — the phase is the machinery; arena noted, unused |
| 17 regression ledger | §5 tail — three named rows + the row-25 co-fix |
| 18 agent instructions | §10 |
| 19 reuse-first | preamble + §2 DECART rejections + anti-scope (wire, never rebuild) |
| 20 Hermetic citations | §8 |

---

## 10. Clear instructions for other agentic workers (item 18 — zero session context assumed)

All work in `/root/dowiz`. P31 sub-letters are independent lanes (parallelizable), all
off-critical-path — never preempt P34/P37/P38/P40/P41 lanes for these. Before starting ANY
sub-letter, re-run its §0 row's verification (ground truth is non-discussible; this repo's
swarm closes gaps fast — see §0 rows 10/15/16 for items that closed between drafts).

1. **T1 (P31b).** Verify RED: `tools/eqc-rs/src/lib.rs:610-611` still refuses Sin/Cos in
   `emit_int_checked_rust`. Implement §2's inline-emission decision; add
   `int_mode_sin_cos_routes_through_cordic` + the still-refusing sweep + ITERS−1 teeth
   (§3.2). Regenerate any affected organ via `tools/eqc-rs/src/bin/gen_kernel_organs.rs`
   (never hand-edit `eqc_gen.rs`). If you decide to also wire the `:533-534` Q-format arms,
   record a dated decision in this file — silence is not a decision. Acceptance:
   `cd tools/eqc-rs && cargo test --release` green with the three new tests.
2. **T2 (P31c).** Scratch-branch the `cfg_attr` gate; harvest the compile-error inventory into
   the PR description; feature-gate I/O modules (`#[cfg(feature = "std")]` only — zero logic
   diffs); add the CI `--no-default-features` step; run the §3.3 std-leak adversarial once and
   record its RED. Acceptance: §5 P31c rows 1-4.
3. **T3 (P31d).** Refine §3.4's seeded invariant list (≥5, ranked); run the kani-vs-z3
   comparison honestly; land ONE money-organ proof in CI; perform and record the off-by-one
   RED run. Do NOT add proof tooling to `[dependencies]` — CI-only. Acceptance: §5 P31d rows.
4. **T4 (P31e).** Write the short design note (learnable params per organ; α pilot); create
   `kernel/src/eq_ir.rs` per §2; write the two RED tests of §3.5 FIRST; wire ScalarAdam
   (already driven by `evals.rs` — reuse that pattern, `evals.rs:792-809`, do not invent a new
   optimizer or a new driver shape). Acceptance: §5 P31e rows 1-4, including the sharpened
   grep falsifier for row 3.
5. **T5 (close-out).** Append the three REGRESSION-LEDGER rows (§5 tail) and fix row 25 in the
   same visit (P-A §11.3.4). Update §10.5.1's P31 statuses if any sub-letter closed. Push after
   every milestone (`worktree-remote-push-collision-avoidance-2026-07-18`).

**Stop-and-flag conditions:** (i) any P31a organ diff; (ii) any `apply_tax` caller flip
(R-4 operator gate); (iii) hand edits to `eqc_gen.rs`; (iv) a §0 row failing re-verification
(stale ground truth — re-audit before building); (v) any impulse to make `EqIr` the default
execution path.
