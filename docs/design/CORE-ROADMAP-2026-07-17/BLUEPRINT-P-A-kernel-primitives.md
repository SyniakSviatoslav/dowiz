# BLUEPRINT P-A — Core Kernel Primitives: equations-not-primitives wiring + Wave-1 correctness closure (2026-07-17)

> **Wave 2, Fable, planning document — writes no product code.** Written against the 20-point
> contract in `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map in §9 below —
> every point addressed, none skipped). This phase **IS mesh-masterwork Wave 1** as scheduled in
> `docs/design/bebop2-mesh-tensor-hermetic-2026-07-17/BLUEPRINT-BEBOP2-MESH-MASTERWORK-SYNTHESIS-V2.md`
> §F (lanes W1-L1, W1-L5, W1-L10) — it formalizes those lanes to the standard, it does not
> re-derive their verdicts. Grounding research: Batch 1
> (`10-BATCH1-kernel-tensor-memory-findings.md`) and Batch 8
> (`17-BATCH8-equations-not-primitives-zero-scripts-audit.md`), both read in full this pass.
> The already-decided eigenvector design (`docs/design/BLUEPRINT-EIGENVECTOR-REFACTOR-PLAN-2026-07-17.md`)
> is a hard constraint this blueprint sequences around and must not contradict (§4.4).

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

This repo has concurrent activity; every inherited citation was re-read against the working tree
on `feat/harness-llm-backend` this session. Matches and drifts, stated exactly:

| Claim | Fresh `file:line` (this pass) | Inherited cite | Status |
|---|---|---|---|
| `ema_next` = `prev + alpha*(sample-prev)` | `kernel/src/geo.rs:39-41` | Batch 8 row 1: `geo.rs:39-41` | **MATCH** |
| `compute_order_total` (money law, `checked_add` chain) | `kernel/src/domain.rs:129-145` | Batch 8 row 2: `domain.rs:95-111` | **DRIFTED +34 lines** (P07 ledger methods `post_earn`/`ledger_balance` inserted above at `domain.rs:86-118`); logic unchanged |
| `apply_tax` (micro-scale, half-up, i128 intermediates) | `kernel/src/money.rs:269-287`; `SCALE: i128 = 1_000_000` at `money.rs:21` | Batch 8 row 2 (`money::apply_tax`) | **MATCH** (function body as described) |
| eig2x2 quadratic, **copy 1** (deflation block in `eig_hessenberg`) | `kernel/src/householder.rs:224-229` (operands bound `:220-223`) | Batch 8 row 3 / W1-L5: `224-229` | **MATCH exact** |
| eig2x2 quadratic, **copy 2** (Wilkinson-shift block) | `kernel/src/householder.rs:245-250` (operands bound `:241-244`) | Batch 8 row 3: `246-250` | **DRIFTED −1 line** (`tr` is at `:245`); duplication verbatim-confirmed |
| `Complex` type used by both copies | `kernel/src/spectral.rs:43-48`, imported at `householder.rs:24` | — | verified |
| `slem_cached` hashes the **raw** adjacency | `kernel/src/spectral_cache.rs:113-130`; the raw hash is `let root = matrix_content_address(a);` at `:114` | V2 §A bridge-gap #1: `spectral_cache.rs:117-118` | **DRIFTED −3/−4 lines** (cited lines now hold the payload comment); bug confirmed present |
| `matrix_content_address` = FNV-1a over row-framed `f64::to_bits` | `kernel/src/spectral_cache.rs:94-108` | Batch 1 §1.2: `:98` | **MATCH** |
| Sole in-repo `slem_cached` caller (a row-stochastic Markov matrix) | `kernel/src/markov.rs:208-209` | — | verified this pass |
| `Csr::row_normalize` (dangling row → deterministic self-loop) | `kernel/src/csr.rs:125-152` | V2 §A: `csr.rs:125-152` | **MATCH** |
| `DriftClass` / `classify_drift` | `kernel/src/spectral.rs:316` / `:342` | V2 §A: `spectral.rs:313-352` | MATCH (within range) |
| eqc-rs `Expr` enum (Sym/Num/Sum/Prod/Pow/Sqrt/Sin/Cos/Exp) | `tools/eqc-rs/src/lib.rs:52-62` | Batch 8 §A | **MATCH** |
| `FixedPointUnsupported` typed refusal | `tools/eqc-rs/src/lib.rs:67-68` | Batch 8 §A | MATCH |
| Fixed emitter: truncating Mul, i128, neg-exponent refusal | `tools/eqc-rs/src/lib.rs:358-396` (neg-exp at `:376-380`) | Batch 8 §A | MATCH |
| `Sub` desugars to `self + (Num(-1.0) * rhs)` | `tools/eqc-rs/src/lib.rs:182-187` | — | verified (load-bearing for §3.1 bit-parity) |
| Default `scale_bits: 32` | `tools/eqc-rs/src/lib.rs:254` | — | verified |
| Refusal proven by test (`hyp_refuses_fixed_point`) | `tools/eqc-rs/src/lib.rs:411-420`; end-to-end in `tools/eqc-rs/tests/proof.rs` (emits→`rustc`→runs→self-asserts) | Batch 8 §A | **MATCH — the refusal path is already exercised** |
| eqc-rs roadmap already names: rounding modes, overflow guard, CORDIC, parity-`#[test]`-beside-organ, equation IR | `tools/eqc-rs/README.md` §Roadmap | Batch 8 §A (`README.md:96`) | MATCH |
| criterion harness wired, 4 benches | `kernel/benches/criterion.rs:12,60,76,91` (`place_order/5_items`, `fold_transitions/5_hops`, `empirical_identify/*`, `token_bucket/try_acquire_permit`); `native-trackers bench` + `baseline.json` per `CORE-ROADMAP-2026-07-17/P-H-audit-telemetry-regression-benchmarks.md` Area 3 (read this pass) | P-H audit | **MATCH** — harness exists; NOT CI-gated; baseline holds only 2 of 4 benches |
| `order_machine::spectral_radius` 1000-iter power iteration | `kernel/src/order_machine.rs:341` (`ITERS: usize = 1000` at `:353`) | Batch 8 row 8: `:311-361` | **DRIFTED ~+30 lines**; loop confirmed present |
| Branch-state correction | `kernel/src/lib.rs:69,96` now declares `pub mod incidence` and `pub mod stats` | Batch 1 appendix said `incidence.rs` absent on this branch | **DIVERGED — E1/E2 have merged onto `feat/harness-llm-backend` since Batch 1 was written**; Batch 1's cross-worktree caveat is obsolete |

Ground truth is non-discussible; everything below builds on the fresh column only.

---

## 1. Scope, wave mapping, and what this blueprint deliberately does NOT own

**P-A owns (build items §3–§4):**

| Item | Mesh-masterwork lane (V2 §F) | Content |
|---|---|---|
| A1 | W1-L1 (compiler leg) | eqc-rs extensions: `Asin`/`Atan2` nodes, integer-exact emission mode with `DivHalfUp` + checked overflow |
| A2 | W1-L1 (organ leg) | `geo::ema_next` generated via eqc-rs + bit-identical parity `#[test]` — the template organ |
| A3 | prep of W4-L1 (**no authority flip**) | money law generated as a **shadow** organ + exact-integer parity pin; flip stays R-4/operator-gated |
| A4 | W1-L5 | `householder.rs` eig2x2 dedup (the two verbatim quadratic copies → one helper) |
| A5 | W1-L10 | normalize-before-hash fix in `spectral_cache.rs::slem_cached` (doc-19 bridge-gap #1) |
| A6 | W1-L1 (CORDIC leg) | integer-CORDIC fixed-point sin/cos primitive, digest-pinned (doc 20 item 4 FLIP) |
| A7 | W1-L4 | `order_machine::spectral_radius` → proven `const` ρ=0 (a runtime loop rediscovering a theorem) |

**P-A cites-and-sequences but does not restate (standard item 19, reuse-first):** the BumpArena
(W2-L1, fully designed in `BLUEPRINT-CACHE-REFERENCE-GRAPH-TENSOR-ARENA-2026-07-17.md` §3.3) and
the eigenvector R1–R3 (`eigh_contig` + `topk_symmetric`, W2-L2, fully designed in
`BLUEPRINT-EIGENVECTOR-REFACTOR-PLAN-2026-07-17.md` §5) are P-A's Wave-2 continuation. Their
designs are decided; re-deriving them here would violate the standard's own reuse rule. The only
interaction P-A has with them is a **sequencing constraint** (§4.4).

**Adjacent Wave-1 lanes owned by sibling phases (cross-links, no double-build):** W1-L2
`append_raw` exactly-once → P-B; W1-L3 hydra hysteresis → P-C; W1-L11 drift-gate-the-arena-snapshot
→ P-B (it gates a *state/snapshot* admission, per the standard's §3 P-B row); W1-L6 zerocopy
AoS-label fix → one-comment change, folded into whichever phase first touches `engine/src/zerocopy.rs`
(named in Batch 1 §4); W1-L7/L8/L9 → P-G/P-B/hygiene respectively.

**Overlap note (honest):** the standard's §3 table lists "the normalize-before-hash fix" under
P-B's *New this pass* column, while the coordinator's Wave-2 dispatch assigns it here. Resolution:
**P-A implements it** (the fix lives entirely in the kernel-primitive file `spectral_cache.rs` and
its RED test is a cache-event assertion — §3.5); **P-B references it** as the `(c)→(a)` link of
its tile→normalize→hash→snapshot chain (V2 §A). One implementation, two consumers; the P-B
blueprint must cite this section instead of re-specifying it.

---

## 2. Predefined types & constants (standard item 4 — named BEFORE implementation)

Everything new in this blueprint, declared up front. No magic numbers, no stringly-typed slots.

```rust
// ── tools/eqc-rs/src/lib.rs — A1 extensions ─────────────────────────────────
pub enum Expr {
    // ... existing 9 variants (lib.rs:52-62) unchanged ...
    Asin(Box<Expr>),                    // f64-only; fixed/int emission MUST refuse
    Atan2(Box<Expr>, Box<Expr>),        // first BINARY function node; f64-only; refuses fixed/int
    DivHalfUp(Box<Expr>, Box<Expr>),    // integer-exact (a + b/2)/b — INT-MODE ONLY; f64 emission refuses
}

/// Typed refusal for the integer-exact path — the mirror image of
/// `FixedPointUnsupported` (lib.rs:67-68): some nodes are f64-only, some are int-only.
pub struct IntEmissionUnsupported(pub String);

impl Equation {
    /// Integer-exact emission mode: symbols are raw i64 (minor units / micro-rates,
    /// NOT Q-scaled reals), arithmetic in i128, every narrowing/overflowing step
    /// checked, return type Result<i64, &'static str>. Matches money.rs's BP-17
    /// fail-closed contract. Allowed nodes: Sym, Num(integral), Sum, Prod,
    /// Pow(n>=0), DivHalfUp. Refuses (Err, never a fallback): Sqrt/Sin/Cos/Exp/
    /// Asin/Atan2, non-integral Num.
    pub fn emit_int_checked_rust(&self) -> Result<String, IntEmissionUnsupported>;
}

// ── kernel/src/money.rs — single scale authority ────────────────────────────
// The existing private `SCALE` (money.rs:21) is promoted, ONE authority, no second def:
pub const MONEY_SCALE_MICRO: i128 = 1_000_000;

// ── kernel/src/eqc_gen.rs — NEW module (generated organs, committed artifacts) ──
// Registered in lib.rs beside the existing pub-mod list (lib.rs:7-166). Every fn
// carries the emitter's "GENERATED by eqc-rs" header. Hand-editing = drift; the
// S6 regenerate-and-diff CI gate (Batch 8 step 7) later makes that structural.
pub fn ema_next_f64(prev: f64, sample: f64, alpha: f64) -> f64;             // A2
pub fn apply_tax_exclusive_int(sub: i64, rate_micro: i64) -> Result<i64, &'static str>; // A3
pub fn apply_tax_inclusive_int(sub: i64, rate_micro: i64) -> Result<i64, &'static str>; // A3

// ── kernel/src/householder.rs — A4 ──────────────────────────────────────────
/// Closed-form eigenvalues of the 2×2 complex block [[a,b],[d,e]]:
/// tr = a+e; det = ae − bd; disc = tr² − 4·det; roots = (tr ± √disc)/2.
/// ONE authority for the quadratic that eig_hessenberg's two sites inlined.
fn eig2x2(a: Complex, b: Complex, d: Complex, e: Complex) -> (Complex, Complex);

// ── kernel/src/spectral_cache.rs — A5 ───────────────────────────────────────
/// Scale-invariant content address: every entry divided by the global pivot
/// (|first nonzero| in row-major order) before hashing; -0.0 → +0.0; NaN → one
/// canonical quiet-NaN bit pattern. NO transcendental on this path — IEEE-754
/// division is correctly rounded (cross-target bit-identical); sqrt/sin/etc.
/// are not (rng.rs:22-28 precedent, V2 §A constraint).
pub fn canonical_content_address(a: &[Vec<f64>]) -> String;
const CANONICAL_QUIET_NAN: u64 = 0x7ff8_0000_0000_0000;

// ── kernel/src/order_machine.rs — A7 ────────────────────────────────────────
/// The order-lifecycle FSM is a fixed DAG ⇒ nilpotent adjacency ⇒ ρ = 0
/// (Perron–Frobenius). Proven once by the retained iterative oracle in tests.
pub const FSM_SPECTRAL_RADIUS: f64 = 0.0;

// ── A6 (CORDIC), pinned from doc 20's measured artifact ─────────────────────
/// Cross-target determinism proof for the Q30 CORDIC sin/cos primitive:
/// digest over 51,471 samples, measured twice bit-identical
/// (20-BUILD-TEST-FIRST-REEXAMINATION.md:181,207-208).
pub const CORDIC_SINCOS_DIGEST: u64 = 0x9d1c_0e89_c65c_be08;

// ── kernel/src/geo.rs tests — A2 fixtures ───────────────────────────────────
const EMA_PARITY_FIXTURES: &[(f64, f64, f64)]; // incl. ±0.0, subnormal, ±INFINITY,
                                               // alpha ∈ {0.0, 1.0, -0.5, 1.5}, large-magnitude
```

Rejected alternative for A1 (DECART-style, one line each): Q-format `Rounding::HalfUp` +
`Scale::Dec` (the README's own roadmap phrasing) — rejected for the money organ because money
already lives in exact integers (`money.rs:269-287` computes on raw i64/i128, not on scaled
reals); wrapping an already-integer domain in Q-format re-scales it and invites double-rounding.
The integer-exact mode models `apply_tax` literally. Q-rounding-modes remain the named future
extension for *real-valued* fixed-point organs.

---

## 3. Build items — spec → RED test → code, each with an adversarial case (items 3, 5)

Spec-driven + event-driven TDD, per standard item 3: for each item the types above are the spec,
the RED test precedes the code, and where a state transition exists the test asserts the **event
sequence** (the `DecompCache.recomputes` counter is literally an event count — §3.5 leans on it).

### 3.1 A2 — `ema_next` generated organ (the template; zero compiler changes needed)

Batch 8 row 1's verdict stands verified: pure `±·`, "the ideal template organ. Zero blockers."
The equation, exact Expr-tree sketch:

```rust
// tools/eqc-rs/src/bin/gen_kernel_organs.rs (new bin beside demo.rs)
use eqc_rs::{Equation, Expr};
let (prev, sample, alpha) = (Expr::sym("prev"), Expr::sym("sample"), Expr::sym("alpha"));
let ema = Equation::new(
    "ema_next",
    &["prev", "sample", "alpha"],
    prev.clone() + alpha * (sample - prev),
);
println!("{}", ema.emit_f64_rust());   // → committed into kernel/src/eqc_gen.rs
```

Emitted body (given `Sub` desugaring, `lib.rs:182-187`):
`(prev + (alpha * (sample + (-1.0f64 * prev))))`.

**Bit-parity argument (hazard-safety-as-math, item 6):** `-1.0 * x` is an exact IEEE-754 sign
flip, and `a + (-b)` is bit-identical to `a - b` for all non-NaN operands — so the generated form
and the hand-written `geo.rs:40` body are **provably bit-identical**, not approximately equal.
The parity test asserts exactly that:

```rust
// kernel/src/geo.rs tests mod (beside ema_converges, geo.rs:356) — the README-roadmap
// "parity #[test] beside each organ" shape:
#[test]
fn ema_next_generated_parity_bit_identical() {
    for &(p, s, a) in EMA_PARITY_FIXTURES {
        assert_eq!(
            ema_next(p, s, a).to_bits(),
            crate::eqc_gen::ema_next_f64(p, s, a).to_bits(),
            "generated organ diverged from hand-written law at ({p},{s},{a})"
        );
    }
}
```

RED→GREEN: the test is written first against a stub `eqc_gen::ema_next_f64` returning `0.0`
(RED on every fixture); committing the real generated body turns it GREEN.

**Adversarial test (must-refuse, verified refusable):** the fixed-point subset must reject what
it cannot represent — and the refusal path is *already proven live* (`hyp_refuses_fixed_point`,
`lib.rs:411-420`, exercised end-to-end by `tests/proof.rs`). A2 adds a second, different-node
refusal so the boundary is tested per-class, not per-example:

```rust
#[test]
fn ema_with_negative_power_refused_by_fixed_subset() {
    let (p, s, a) = (Expr::sym("p"), Expr::sym("s"), Expr::sym("a"));
    // alpha⁻¹ smuggled in: 1/a is NOT fixed-point-representable (lib.rs:376-380)
    let bad = Equation::new("ema_bad", &["p", "s", "a"],
        p.clone() + a.pow(-1) * (s - p));
    assert!(bad.emit_fixed_rust().is_err()); // refusal, never a silent fallback
}
```

### 3.2 A1 — eqc-rs extensions (`Asin`/`Atan2`/`DivHalfUp` + integer-exact checked emission)

Needed by: A3 (money, `DivHalfUp` + checked int mode) and the haversine/bearing conversions
(Batch 8 row 4 — `asin`/`atan2` are the only missing nodes; f64-only, fixed correctly refused).
All three are on eqc-rs's own README roadmap — this is extension of the named plan, not new
scope. Implementation constraints:

- `Atan2` is the first binary function node — `free_symbols`, `eval`, `to_rust_f64` each gain a
  two-child arm; the fixed emitter refuses it (extend the refusal match `lib.rs:391-394`).
- `DivHalfUp(a, b)` emits `((A) + (B) / 2) / (B)` in i128 in int mode only (exactly the half-up
  form `money.rs:280,284` uses); `emit_f64_rust` **refuses** it via a panic-free typed error
  (f64 division would silently change semantics — the honest boundary cuts both ways).
- `emit_int_checked_rust` emits `checked_mul`/`checked_add`/`i64::try_from` chains mirroring
  `money.rs:267-286`'s BP-17 contract; division-by-zero guard emitted before every `DivHalfUp`.

RED→GREEN: extend `tests/proof.rs` with an `atan2` proof program (compiles, runs, self-asserts
against `Expr::eval` — the existing referee pattern) and an int-mode proof for `DivHalfUp`.

**Adversarial tests:** (i) `emit_fixed_rust` on an `Atan2` expression → `Err` (verify the new
node joins the refused set — this is the prompt-required "equation that SHOULD be refused,
verify it actually refuses", extended to the new nodes); (ii) `emit_int_checked_rust` on an
expression containing `Sqrt` → `Err(IntEmissionUnsupported)`; (iii) `emit_f64_rust` on
`DivHalfUp` → typed refusal; (iv) generated checked code at `sub = i64::MAX, rate_micro = 2_000_000`
must return `Err`, never wrap (mirrors the existing `apply_tax` overflow test corpus,
`money.rs:493`).

### 3.3 A3 — money law as a generated SHADOW organ (parity pin now; authority flip R-4-gated)

**Red-line discipline (item 6 + memory `never-bypass-human-gates`):** `apply_tax`'s body and its
callers are **not touched**. P-A generates the equation-form organ and pins exact-integer parity
against the hand-written law; the authority flip (callers switch to the generated organ; `tax_rate:
f64` → integer basis-points) is W4-L1 under operator docket **R-4** (V2 §F Wave 3/4). Zero
behavior change ships in P-A — which is precisely why it is safe to build now.

Both legs of `apply_tax` (`money.rs:277-285`) are expressible in the A1 integer mode:

```rust
// exclusive: tax = (sub·rate_micro + S/2) / S           [S = MONEY_SCALE_MICRO]
// inclusive: net = (sub·S + (S+rate_micro)/2) / (S+rate_micro);  tax = sub − net
let (sub, rate) = (Expr::sym("sub"), Expr::sym("rate_micro"));
let s = Expr::int(1_000_000);
let tax_excl = Equation::new("apply_tax_exclusive", &["sub", "rate_micro"],
    Expr::div_half_up(sub.clone() * rate.clone(), s.clone()));
let net = Expr::div_half_up(sub.clone() * s.clone(), s + rate);
let tax_incl = Equation::new("apply_tax_inclusive", &["sub", "rate_micro"],
    sub - net);
// emit_int_checked_rust() for both → kernel/src/eqc_gen.rs
```

Parity `#[test]` shape (exact integers — money admits no tolerance):

```rust
// kernel/src/money.rs tests mod, beside the existing apply_tax corpus (money.rs:456-493):
#[test]
fn apply_tax_generated_parity_exact_integers() {
    for &(sub, rate, incl) in MONEY_TAX_FIXTURES { // includes every existing corpus case
        let rate_micro = (rate * 1_000_000.0).round() as i64; // same boundary conv as money.rs:274
        let want = apply_tax(sub, rate, incl);
        let got = if incl { crate::eqc_gen::apply_tax_inclusive_int(sub, rate_micro) }
                  else    { crate::eqc_gen::apply_tax_exclusive_int(sub, rate_micro) };
        match want {
            Ok(v)  => assert_eq!(got.unwrap(), v),          // exact, not approx
            Err(_) => assert!(got.is_err()),                 // both refuse, or RED
        }
    }
}
```

RED→GREEN: stub organs returning `Ok(0)` are RED on the first nonzero fixture. **Adversarial:**
the overflow fixture (`i64::MAX`, rate 2.0 — `money.rs:493`'s case) where *both* paths must
`Err`; and a divergence-hunting property sweep (deterministic grid over sub ∈ {0, 1, 999, 10⁶,
i64::MAX/2}, rate_micro ∈ {0, 1, 200_000, 999_999}) where any single mismatch is RED — this is
the test *designed to break* the claim that half-up-in-i128 was transcribed correctly.

### 3.4 A4 — eig2x2 dedup (W1-L5): the verbatim-duplication drift hazard

Both copies, fresh-verified (§0): `householder.rs:224-229` (deflation) and `:245-250` (Wilkinson
shift) compute `tr = a.add(e); det = a.mul(e).sub(b.mul(dd)); disc = tr.mul(tr).sub(det.mul(4));
sq = disc.sqrt(); r1 = (tr+sq)·½; r2 = (tr−sq)·½` in `Complex` arithmetic, token-for-token. Not
eqc-able (complex arithmetic — Batch 8 row 3's boundary holds); the fix is the classic
single-authority consolidation:

- Add the private `eig2x2` helper (§2 signature) whose body is the **identical operation
  sequence** — same ops, same order — and replace both inline blocks with calls. The deflation
  site keeps its realification post-step (`:230-236`); the shift site keeps its
  closer-to-corner selection (`:251-254`).
- **Byte-identity is structural, not reviewed-for:** because the helper is a pure motion of an
  identical expression DAG (no reassociation, no operand reorder), the compiled arithmetic is
  the same sequence of `Complex` ops — the falsifier below would catch any accidental deviation.

RED→GREEN + regression: (i) all 8 existing hand-oracle/parity tests (`householder.rs` tests mod)
stay green; (ii) a new **bit-capture falsifier** runs `eigenvalues_contig` on the P₃/K₃ fixtures
plus one complex-pair fixture and asserts `to_bits`-equality against values captured from the
pre-refactor build (captured in the RED commit, asserted in the GREEN commit). **Adversarial:**
(iii) the rotation matrix `[[0,−1],[1,0]]` (eigenvalues ±i — forces the `disc < 0` complex-`sqrt`
branch through the helper) and a `disc = 0` repeated-root fixture (`[[1,1],[0,1]]`'s bottom
block) — the two numerically nastiest paths through the shared code, asserted bit-identical
pre/post.

### 3.5 A5 — normalize-before-hash (W1-L10, doc-19 bridge-gap #1): the ordering bug

**The bug, precisely (fresh cites):** `slem_cached` content-addresses the **raw** adjacency
(`spectral_cache.rs:114`) via `matrix_content_address` (`:94-108`, FNV-1a over `f64::to_bits`).
Two nodes that build the same logical tile at different scale — `W` and `c·W` — hash to
different content-ids, so the `DecompCache` never hits cross-node and Merkle roots never
converge, **silently** (V2 §A/§E; `19-SYSTEM-COHERENCE-AND-AUTHORITY-BOUNDARY-REDO.md` Part 1).
Additional latent case found this pass: `-0.0` and `+0.0` have different `to_bits`, so two
bit-different-but-value-identical tiles also never converge.

**Design (refining V2's parenthetical, with reasons):** V2 offered "`row_normalize`d **or**
integer-scaled canonical." For `slem_cached`'s general-matrix contract the right canonicalizer is
**global-pivot scaling, not row-stochastic normalization**: row-normalizing (`csr.rs:125-152`
semantics) would equate matrices that differ by *per-row* scales — which have genuinely different
spectra — collapsing distinct operators onto one cache key (a correctness bug worse than the one
being fixed). Global-pivot scaling equates exactly the uniform-scale family `{c·W : c > 0}`,
which is the family whose spectra differ only by the known factor. Row-stochastic normalization
remains the right canonical form where the consumer's semantic object *is* the stochastic
operator (PPR, the P-B tile chain) — P-B's chain should use `row_normalize` per V2 §A; this
item fixes `slem_cached` specifically.

Mechanics:

- `canonical_content_address` (§2): find the global pivot `p` = |first nonzero entry| in
  row-major order; hash the entries `x / p` with the existing FNV-1a framing; map `-0.0 → +0.0`
  and any NaN → `CANONICAL_QUIET_NAN` (deterministic in release; `debug_assert!` flags NaN input
  in debug). Zero matrix ⇒ no pivot ⇒ hash raw (well-defined).
- **Why this is sound (hazard-safety-as-math, item 6):** IEEE-754 mandates correctly-rounded
  division — `fl(x/p)` depends only on the real quotient, so for an exactly-scaled family
  (`c·x` representable exactly, e.g. integer-count-derived weights, any power-of-two `c`)
  `fl((cx)/(cp)) ≡ fl(x/p)` **bitwise, on every conforming target**. No summation appears on
  the hash path (a fixed-order sum would *not* commute with scaling), and no transcendental
  (not correctly-rounded, `rng.rs:22-28`) — the two constraints V2 §A states, both honored
  structurally.
- `slem_cached` ordering change: key = `canonical_content_address(a)`; the eigensolve runs on
  the **pivot-scaled canonical operator** (so the cached payload is scale-free and cross-node
  meaningful); the wrapper returns `slem_canonical · p` to preserve the caller-facing contract.
  Blast radius: exactly one in-repo caller (`markov.rs:208-209`, a row-stochastic matrix), whose
  result may move by ULPs (division + remultiplication); acceptance below pins the bound.

RED→GREEN, **event-sequence form** (standard item 3 — asserted on the cache's event counter,
the same falsifier discipline as the two existing tests `spectral_cache.rs:146-203`):

```rust
#[test]
fn slem_cached_scale_invariant_key_and_payload() {
    let w = fixture_tile();                       // integer-valued entries (exact-scale family)
    let mut cache = DecompCache::new();
    let s1 = slem_cached(&mut cache, &w);
    let w2 = scale(&w, 2.0);                      // exact (power of two)
    let w3 = scale(&w, 3.0);                      // exact (small integers)
    let s2 = slem_cached(&mut cache, &w2);
    let s3 = slem_cached(&mut cache, &w3);
    assert_eq!(cache.recomputes(), 0);            // EVENT SEQUENCE: 1 fill, 0 recomputes.
                                                  // TODAY: 2 recomputes — this is the RED.
    assert!((s2 - 2.0 * s1).abs() < 1e-12 && (s3 - 3.0 * s1).abs() < 1e-12);
}

#[test]
fn neg_zero_and_pos_zero_are_the_same_tile() {    // RED today (to_bits differ)
    assert_eq!(canonical_content_address(&tile_with(0.0)),
               canonical_content_address(&tile_with(-0.0)));
}
```

**Adversarial tests (designed to break the fix):** (i) *over-normalization guard* — `D·W` with
**distinct per-row** factors must NOT collide with `W` (different spectra ⇒ different key); this
is the test that would go RED if an implementer "helpfully" switched to row-stochastic
normalization; (ii) NaN-bearing tile: two runs produce the identical id (determinism under
corruption), and debug builds assert; (iii) all-zero tile: id is stable and distinct from any
nonzero tile; (iv) markov regression pin: `|slem_new − slem_old| ≤ 1e-9` on the markov fixture
suite, existing markov/spectral suites stay green.

### 3.6 A6 — integer-CORDIC fixed-point sin/cos primitive (W1-L1 third leg)

Doc 20 item 4 (FLIP, measured): a Q30 CORDIC sin/cos with a zero-float runtime kernel is
bit-identical across runs — digest `0x9d1c0e89c65cbe08` over 51,471 samples, asserted twice
(`20-BUILD-TEST-FIRST-REEXAMINATION.md:181,207-208`, artifact `reexam-builds/item4_cordic.rs`).
P-A's job: promote that proven artifact from `reexam-builds/` into `tools/eqc-rs` as the
fixed-point-transcendental substrate (the README-roadmap "CORDIC/LUT" line), so `Sin`/`Cos` gain
an *integer* emission path instead of refusing — extending the representable subset without
touching the f64 dynamics path. DoD: the digest test asserts `CORDIC_SINCOS_DIGEST` on this
host; the W1-L1 cross-arch leg (x86_64 + aarch64 both emit the digest) is the done-check when an
aarch64 runner is available — if none is, the digest constant + single-host assertion land now
and the cross-arch run is an explicitly-open checklist line, not silently claimed (honesty per
doc 20's own protocol). **Adversarial:** an input outside the CORDIC convergence range
(|θ| > π/2 pre-reduction) must go through the deterministic range-reduction, and a
deliberately-wrong iteration count must change the digest (proving the digest actually has
teeth — run once with `ITERS−1`, assert `≠ CORDIC_SINCOS_DIGEST`, then restore).

### 3.7 A7 — `spectral_radius()` → proven const (W1-L4)

Fresh: `order_machine.rs:341` runs a 1000-iteration power iteration (`ITERS` at `:353`) on the
fixed lifecycle FSM adjacency at every call. The lifecycle graph is a compile-time-constant DAG ⇒
nilpotent adjacency ⇒ **ρ = 0 exactly** (Perron–Frobenius / nilpotency; the same theorem family
the FSM drift-gate already relies on — memory `fsm-graph-analysis`: ρ≈0 ⟺ acyclic). Replace the
runtime loop with `FSM_SPECTRAL_RADIUS` (§2); **retain the iterative code as the test-side
oracle** (adapter rule: older = adapters, no purging): a `#[test]` runs the old loop and asserts
it agrees with the const to 1e-12. **Adversarial:** mutate a copy of the edge list to add one
back-edge (making the graph cyclic) and assert the oracle then reports ρ > 0 while the golden
FSM-signature test (the existing gate) rejects the mutated graph — proving the const is guarded
by the signature, not by faith. RED→GREEN: the oracle-vs-const test is written against the loop
first (GREEN trivially), then the loop is removed from the runtime path; the RED direction is
the back-edge mutation test, which fails if anyone silently changes the FSM without updating
both the signature and the const.

---

## 4. Cross-cutting design obligations (items 6, 8, 9, 11–16)

### 4.1 Hazard-safety as math (item 6)

Every P-A safety claim is a reachability argument, not prose: A2/A3 — divergence between law and
organ is unrepresentable *in CI* because the parity test asserts bit/integer equality (the bug
class "transcription drift" becomes a red test, not a runtime surprise); A1 — unrepresentable
equations are **typed refusals** (`FixedPointUnsupported` / `IntEmissionUnsupported`), never
fallbacks, so a money-grade organ silently degrading to float is not a reachable state; A4 —
after dedup there is no second copy to drift (the hazard's state space is deleted, not
monitored); A5 — cross-node id-mismatch of an exactly-scaled tile family is unreachable by the
correctly-rounded-division argument (§3.5). Per V2 §D's authority accounting, every P-A check is
in the **internal-arithmetic invariant class where authority genuinely dissolves** — no watchdog,
no anchored second party needed; nothing here touches the tamper leg (P06/key_V unaffected and
not blocked by this phase).

### 4.2 Schemas & scaling axes (item 8)

`canonical_content_address` is O(n²) over a dense `Vec<Vec<f64>>`: fine at the current consumer
scale (markov's 10-state matrices; dense diagnostics n ≤ 32). Its stated break point: dense
hashing at n ≳ 10³ — at which point tiles live in `Csr` inside the Phase-28 arena and the
content-address moves to the CSR byte layout (Batch 1 A1; not built here, axis named).
`eqc_gen.rs` scales by organ count, one fn each, no runtime state. `MONEY_SCALE_MICRO` is fixed
by the money contract; a currency needing finer resolution than micro-units is the (named,
unlikely) change trigger.

### 4.3 Isolation / bulkhead (item 11), mesh awareness (item 12), rollback (item 13), living memory (item 15)

- **Isolation:** eqc-rs runs at authoring time only — generated code has zero runtime dependency
  on the compiler crate (`lib.rs:12-16`), so an eqc-rs defect cannot propagate into a running
  kernel except through a committed, reviewed, parity-gated diff. `DecompCache` is `&mut`-only
  (`spectral_cache.rs:36`), no shared mutable state.
- **Mesh:** all P-A items are node-local. A5 is the *precondition* for cross-node convergence
  (Merkle-root agreement over the existing ≤1 MiB SyncFrame transport, V2 §A propagation layer)
  but ships and is testable entirely locally; no transport change, no new payload budget.
- **Rollback (item-13 vocabulary, used precisely):** P-A claims only the **Self-Termination /
  unrepresentable-state leg** (typed emission refusals; checked money arithmetic; negative
  totals refused at `money.rs:310-315`). No Self-Healing (no redundancy math here) and no
  Snapshot-Re-entry claims — those belong to P-B/P-C. Every item is mechanically reversible:
  delete `eqc_gen.rs` + its tests (A2/A3), inline the helper back (A4), revert `slem_cached`'s
  key fn (A5), restore the loop (A7).
- **Living memory (item 15):** the content-address IS the recall key of the spectral cache — the
  canonicalization makes recall keyed by the tile's *invariant* content rather than its
  incidental scale, the same content-not-location principle as
  `internal-retrieval-living-memory-arc-2026-07-14`'s recall design.

### 4.4 Non-contradiction with the eigenvector plan (hard constraint)

`BLUEPRINT-EIGENVECTOR-REFACTOR-PLAN-2026-07-17.md` §3.1 Option A requires `eig_hessenberg`/
`qr_step` stay **byte-identical for the values-only path** while `eigh_contig` is added. A4
touches `eig_hessenberg` — the resolution is ordering plus a structural guarantee: **A4 lands
BEFORE W2-L2 (eigh_contig) starts**, and A4's bit-capture falsifier (§3.4) proves the values-only
path emits identical bytes post-dedup, so the eigenvector plan's precondition ("old path
byte-identical", its §5.4.7) holds against the deduped base. A5 fills no `Decomp` basis slot and
changes no `Decomp` type — consistent with that plan's "spectral_cache.rs needs no change at
all *for the vector work*" (its §5.1; A5's change is to the *keying*, orthogonal to the basis
slot). `topk_symmetric`/`eigh_contig` signatures are untouched by P-A.

### 4.5 Linux-discipline verdict framework (item 9)

Applying `BLUEPRINT-LINUX-ENGINEERING-PRINCIPLES-ADOPTION-2026-07-17.md`'s categories, not
re-deriving them: A4 and A7 are **ALREADY-EQUIVALENT** discipline ("one implementation of one
concept" / "don't compute at runtime what is known at build time"); A2/A3's
generated-code-is-committed-and-reviewed model **REINFORCES** the kernel's no-generated-magic
rule (emitted Rust is hand-inspectable, diff-reviewed like any patch); the S6
regenerate-and-diff gate is the **EXTENDS** item (deferred, trigger = ≥3 organs through eqc-rs,
Batch 8 step 7).

---

## 5. DoD — falsifiable, RED→GREEN, per item (item 2)

A P-A item is DONE iff its rows below are all demonstrably true; none is a prose checkbox.

| Item | RED (fails before) | GREEN (passes after) | Permanent regression test (item 17) |
|---|---|---|---|
| A1 | new proof-program tests absent/failing; `Atan2` in fixed mode does not yet Err | `cargo test` in `tools/eqc-rs` green incl. new refusal + proof tests | `eqc-proofs` CI job (`.github/workflows/ci.yml:30-42`, already wired) |
| A2 | `ema_next_generated_parity_bit_identical` RED vs stub | bit-identical on all fixtures | same test, permanent, in kernel suite |
| A3 | `apply_tax_generated_parity_exact_integers` RED vs stub; overflow-divergence sweep RED | exact-integer parity on full corpus incl. both-Err cases | same test; ledger row (below) |
| A4 | bit-capture falsifier records pre-refactor bits; adversarial complex-pair/repeated-root fixtures pinned | 8 existing hand-oracle tests + bit-capture + adversarial all green post-dedup | bit-capture test, permanent |
| A5 | `slem_cached_scale_invariant_key_and_payload` RED (recomputes = 2 today); `neg_zero…` RED | recomputes = 0, payload scales linearly; over-normalization guard green | both tests + the guard, permanent |
| A6 | digest test absent; `ITERS−1` mutation shows digest has teeth | `CORDIC_SINCOS_DIGEST` asserted on host; cross-arch line explicitly open or closed | digest test, permanent |
| A7 | back-edge mutation test RED against a graph change without signature update | const == oracle to 1e-12; runtime loop gone from hot path | oracle-vs-const + mutation test |

Each behavior-changing item (A3 parity pin, A5, A7) adds a row to
`docs/regressions/REGRESSION-LEDGER.md` under the kernel-native guardrail types the P-H audit
Area 2 defines (`cargo-test` unit + CI-gate), honoring the ledger's standing ratchet rule
(ledger `:7-9`): guardrail with red→green proof BEFORE "done".

---

## 6. Benchmark plan (item 10) — existing harness only, nothing new built

Harness confirmed by the P-H audit (read this pass): criterion wired (`kernel/Cargo.toml`,
`kernel/benches/criterion.rs` — 4 benches), baseline-diff regression tracking via
`native-trackers bench kernel --threshold N` (exit 1 on regression) with `bench_track.py`
fallback, history in `kernel/benches/BENCH_HISTORY.md`. P-A adds **two benches and zero
infrastructure**:

1. `spectral_cache/slem_cached_10x10_hit` and `spectral_cache/canonical_address_32x32` — added
   in the RED commit (pre-A5) so `native-trackers bench` auto-seeds the pre-change baseline;
   post-A5 run must show: hit path unchanged within threshold (hashing is the only added work on
   a hit), canonical address ≤ 2× the raw `matrix_content_address` cost (one divide pass added —
   the eigensolve dominates the miss path by orders of magnitude, so end-to-end `slem_cached`
   cost must be flat within noise). Numbers go into `BENCH_HISTORY.md`, not prose estimates.
2. A2/A3/A4/A7 need no new bench: A2's generated body is instruction-identical (bit-parity
   proves it), A4 is expression motion, A7 strictly deletes 1000 iterations (the existing
   `place_order`/`fold_transitions` benches guard against accidental regression via the standard
   threshold run). One honest measurement to record: `fold_transitions/5_hops` before/after A7
   if `spectral_radius` sits on any measured path — if it does not, record "no measured consumer
   on a bench path" in `BENCH_HISTORY.md` rather than inventing a number.

Telemetry hook (item 10's second half): the bench-regression CI job is P-H's deliverable (P-H
audit Area 3 verdict); P-A's benches are written `bench_track`-compatible so they are gated the
day that job lands — nothing to build here, dependency named.

---

## 7. Links to docs & memory (item 7)

Depends on / cites: `CORE-ROADMAP-STANDARD-2026-07-17.md` (the contract) ·
`BLUEPRINT-BEBOP2-MESH-MASTERWORK-SYNTHESIS-V2.md` §A/§D/§E/§F (wave authority; W1-L1/L5/L10) ·
`10-BATCH1-…` + `17-BATCH8-…` (grounding audits) · `19-SYSTEM-COHERENCE-…` (bridge-gap #1
provenance) · `20-BUILD-TEST-FIRST-REEXAMINATION.md` (CORDIC digest) ·
`BLUEPRINT-EIGENVECTOR-REFACTOR-PLAN-2026-07-17.md` (§4.4 constraint) ·
`BLUEPRINT-CACHE-REFERENCE-GRAPH-TENSOR-ARENA-2026-07-17.md` (Wave-2 continuation, cited not
restated) · `P-H-audit-telemetry-regression-benchmarks.md` (bench harness ground truth) ·
`docs/regressions/REGRESSION-LEDGER.md` (item-17 mechanism) ·
`HERMETIC-ARCHITECTURE-PRINCIPLES.md` (§8). Memory files: `bebop2-mesh-masterwork-2026-07-17`
(execution-model rule: equations via eqc-rs, scripts→0) ·
`sovereign-architecture-19-phase-roadmap-2026-07-17` (P06 unaffected by P-A — no new blocker
introduced) · `anu-ananke-strict-discipline-feedback-2026-07-17` (style discipline applied
throughout) · `verified-by-math-2026-07-07` · `never-bypass-human-gates-2026-06-29` (A3's R-4
gate) · `fsm-graph-analysis` (A7's theorem) · `internal-retrieval-living-memory-arc-2026-07-14`
(§4.3). Supersedes: nothing — this is Wave-1's formalization, additive over V2 §F.

---

## 8. Hermetic principles honored (item 20 — explicit, per principle)

- **P1 MENTALISM** (spec is source, code derived): the entire eqc premise — the `Expr` tree is
  the spec, `eqc_gen.rs` is the derived artifact (`lib.rs:1-16`); A2/A3 make two real kernel
  organs live under this law.
- **P2 CORRESPONDENCE** (one concept, one primitive): A4 (one quadratic authority), A7 (one
  ρ-truth: the const, oracle-pinned), A5 (one canonical identity per logical tile), and §2's
  `MONEY_SCALE_MICRO` single scale authority.
- **P6 CAUSE-AND-EFFECT** (determinism as law): bit-parity tests (A2), exact-integer parity
  (A3), correctly-rounded-division argument + no-transcendental hash path (A5), digest-pinned
  CORDIC (A6) — every determinism claim has a falsifier, none is asserted.
- **P7 GENDER** (paired creation, no self-certification): every generated organ is verified by
  an **independent second path** — `Expr::eval` (a tree-walking interpreter, a different code
  path than the emitter, `lib.rs:119-124`) referees the proof programs, and the hand-written law
  referees the generated organ in-kernel. A7's const is refereed by the retained iterative
  oracle. Nothing in P-A certifies itself.

(P3/P4/P5 are not load-bearing for these items and are not claimed decoratively, per the
Anu/Ananke discipline.)

---

## 9. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 (fresh cites + drift table) |
| 2 DoD | §5 |
| 3 spec/event-driven TDD | §2 (spec first), §3 per-item RED tests, §3.5 event-counter assertion |
| 4 predefined types/consts | §2 |
| 5 adversarial/breaking tests | §3.1–3.7, one+ per item, incl. must-refuse verified |
| 6 hazard-safety as math | §4.1, §3.5 soundness argument |
| 7 links docs/memory | §7 |
| 8 scaling axes | §4.2 |
| 9 Linux discipline | §4.5 |
| 10 benchmarks+telemetry | §6 |
| 11 isolation/bulkhead | §4.3 |
| 12 mesh awareness | §4.3 |
| 13 rollback/self-heal vocabulary | §4.3 |
| 14 error-propagation gates | §5 (named CI gates per item), §4.1 |
| 15 living memory | §4.3 |
| 16 tensor/spectral + eqc reuse | §1 (arena/eigenvector cited-not-restated), §3.6 CORDIC-as-data |
| 17 regression ledger | §5 (ledger rows named) |
| 18 agent-executable instructions | §10 |
| 19 reuse-first | §1, §2 (rejected alternative), §6 (no new harness) |
| 20 Hermetic citations | §8 |

---

## 10. Clear instructions for other agentic workers (item 18 — zero session context assumed)

Execute in this order; T1 is the DoD-critical path's first buildable unit. Every task names its
files, its acceptance command, and its gate. Do not reorder T5 after any eigenvector (`eigh_contig`)
work — §4.4.

1. **T1 (A2, smallest first — no compiler changes needed).** Create
   `tools/eqc-rs/src/bin/gen_kernel_organs.rs` with the §3.1 `Equation` (copy the sketch
   verbatim). Run it; paste its stdout into a new file `kernel/src/eqc_gen.rs`; add
   `pub mod eqc_gen;` to `kernel/src/lib.rs` (alphabetical slot near `event_log`, see the
   pub-mod list at `lib.rs:7-166`). In `kernel/src/geo.rs`'s existing `mod tests`
   (starts `geo.rs:323`), add `EMA_PARITY_FIXTURES` (§2 contents) and the
   `ema_next_generated_parity_bit_identical` test (§3.1 verbatim). RED first: commit the test
   against a stub `ema_next_f64` returning `0.0`, run `cargo test -p kernel ema_next_generated`
   — must FAIL; then commit the generated body — must PASS. Also add
   `ema_with_negative_power_refused_by_fixed_subset` (§3.1) to `tools/eqc-rs/src/lib.rs`'s test
   mod. Acceptance: both tests green; `cd tools/eqc-rs && cargo test --release` green.
2. **T2 (A4).** In `kernel/src/householder.rs`: BEFORE editing, add the bit-capture test (§3.4)
   asserting `eigenvalues_contig` output bits on P₃ (fixture at `householder.rs:435-443`), K₃,
   `[[0,-1],[1,0]]`, and `[[1,1],[0,1]]`; run and record the bits in the test (GREEN). Then add
   `fn eig2x2` (§2 signature; body = the exact op sequence at `householder.rs:224-229`) and
   replace BOTH inline blocks — `:224-229` and `:245-250` — with calls (keep `:230-236`
   realification and `:251-254` shift-selection in place). Acceptance:
   `cargo test -p kernel householder` — all 8 pre-existing tests plus the bit-capture test green,
   zero bit drift.
3. **T3 (A5).** In `kernel/src/spectral_cache.rs`: add `canonical_content_address` +
   `CANONICAL_QUIET_NAN` (§2, mechanics §3.5 — pivot = |first nonzero| row-major; `-0.0→+0.0`;
   NaN→canonical; zero matrix hashes raw; reuse the FNV framing from `:94-108`). Write the two
   RED tests (§3.5 verbatim) + the four adversarial tests; run
   `cargo test -p kernel spectral_cache` and CONFIRM `slem_cached_scale_invariant…` FAILS with
   `recomputes == 2` (this proves the bug live). Then change `slem_cached` (`:113-130`): key on
   the canonical address, eigensolve the pivot-scaled operator, return `slem · p`. Acceptance:
   all new tests green; `cargo test -p kernel markov` green; the markov ULP-pin
   (`|Δslem| ≤ 1e-9`) green. Do NOT touch `matrix_content_address` itself (other consumers keep
   raw semantics).
4. **T4 (benches for T3).** Add the two §6 benches to `kernel/benches/criterion.rs` in the T3
   RED commit; run `python3 kernel/benches/bench_track.py` (or
   `tools/telemetry/native-trackers bench kernel`) before and after T3's GREEN commit; append
   both results to `kernel/benches/BENCH_HISTORY.md`. Acceptance: post-change run exits 0
   (no regression beyond threshold).
5. **T5 (A1).** In `tools/eqc-rs/src/lib.rs`: add the three `Expr` variants, the
   `IntEmissionUnsupported` type, and `emit_int_checked_rust` (§2 + §3.2 constraints — extend
   `free_symbols`/`eval`/`to_rust_f64`/`emit_fixed`'s refusal arm for the new nodes). Add the
   four adversarial refusal tests (§3.2) and an `atan2` + `div_half_up` proof-program test in
   `tools/eqc-rs/tests/proof.rs` (copy the existing `compile_and_run` pattern, `proof.rs:19-46`).
   Acceptance: `cd tools/eqc-rs && cargo test --release` green (this is CI job `eqc-proofs`,
   `ci.yml:30-42`).
6. **T6 (A3 — shadow only; DO NOT modify `apply_tax` or any caller).** Promote `SCALE` to
   `pub const MONEY_SCALE_MICRO` in `kernel/src/money.rs:21` (keep one definition). Extend
   `gen_kernel_organs.rs` with the two §3.3 equations; append their `emit_int_checked_rust`
   output to `kernel/src/eqc_gen.rs`. Add `apply_tax_generated_parity_exact_integers` (§3.3)
   + the overflow/property adversarial sweep to `kernel/src/money.rs`'s test mod (corpus base:
   `money.rs:456-493`). RED vs stubs first, then GREEN. Add a REGRESSION-LEDGER row ("money law
   parity pin — guardrail: kernel unit test, CI cargo-test"). The authority flip is **forbidden
   here** — it is W4-L1, operator docket R-4.
7. **T7 (A7).** In `kernel/src/order_machine.rs`: add `FSM_SPECTRAL_RADIUS` (§2); move the
   power-iteration body (`:341-` incl. `ITERS` at `:353`) into the test mod as
   `fn spectral_radius_oracle()`; make `spectral_radius()` return the const; add the
   oracle-vs-const test and the back-edge mutation test (§3.7). Acceptance:
   `cargo test -p kernel order_machine` green; grep confirms no 1000-iter loop on the runtime
   path. Ledger row added.
8. **T8 (A6).** Copy `docs/design/bebop2-mesh-tensor-hermetic-2026-07-17/reexam-builds/item4_cordic.rs`
   into `tools/eqc-rs/src/cordic.rs` (module `pub mod cordic;`); add the digest test asserting
   `CORDIC_SINCOS_DIGEST` (§2) and the `ITERS−1` teeth test (§3.6). Wire `Sin`/`Cos` int-mode
   emission to route through it ONLY if T5 landed; otherwise land the module + digest test alone.
   If no aarch64 runner exists, add the cross-arch check as a `#[ignore]`d test whose doc comment
   states the activation condition (the Batch-1 §5.6 deferred-seam convention). Acceptance:
   digest test green on host.
9. **T9 (close-out).** Run the full suites: `cargo test -p kernel && cargo test -p engine &&
   (cd tools/eqc-rs && cargo test --release)`. Verify every §5 DoD row. Append the P-A ledger
   rows to `docs/regressions/REGRESSION-LEDGER.md`. Do not mark P-A done if any adversarial test
   was weakened, `#[ignore]`d (other than T8's declared cross-arch seam), or had its tolerance
   inflated — the ledger's ratchet rule (`REGRESSION-LEDGER.md:13-18`) applies verbatim.
