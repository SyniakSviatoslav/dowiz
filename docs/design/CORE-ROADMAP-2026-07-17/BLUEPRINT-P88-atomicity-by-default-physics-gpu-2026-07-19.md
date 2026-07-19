# BLUEPRINT P88 — Atomicity-by-default in the physics/GPU domain (2026-07-19)

> **Standalone POLICY + SPEC blueprint (dowiz `engine/` physics-render + the future `gpu` feature).**
> One coherent, independently-buildable unit against the 20-point contract in
> `CORE-ROADMAP-STANDARD-2026-07-17.md` §2. Research source:
> `docs/research/OPUS-PHYSICS-WAVE-ALGORITHMS-AND-TENSOR-SYNTHESIS-2026-07-18.md` §B1 (the
> "field step needs zero atomics" verdict this unit **overrules as a gating philosophy**) and
> `OPUS-PINGPONG-SHADOW-COPY-PROPAGATION-2026-07-18.md` (the single-writer structure the exemption
> rests on). Divergence recorded verbatim in `SYNTHESIS-PHYSICS-PERFORMANCE-VISION-2026-07-18.md`
> §2 row 4 (operator item C) and §4.4. Format precedent: `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md`.
> Grounding tree: `/root/dowiz` at HEAD, read live this pass.
>
> **One sentence:** in the physics/GPU render domain, **every cross-invocation shared-write site is
> atomic by default, and non-atomic access is a reviewable exemption granted only by a written
> structural single-writer proof** — reversing the research's evidence-gated "add atomics only after
> measured contention" default for this domain, while leaving the CPU-kernel domain's evidence-gated
> standing rule (E4/E12) untouched.

---

## VERDICT (stated up front, per session discipline)

**GO as a policy + spec unit; its implementation legs are P38 §4.2-gated.** This is the one physics
unit that is **write-first** (Master Ledger §4 item 13, §3 wave-1): it is pure policy text plus a
review artifact, has **no build dependency**, and it **constrains the shader design of P86 and P87**
which must not be written against a looser rule. The falsifiable content is the reconciliation in
§4.3 (default = atomic; exemption = structural proof) and the banned-primitive list (§4.4, float
`atomicAdd`). Nothing here silently takes the P38 §4.2 GPU-compute decision — until the operator
takes it, P88 is a written policy + a checklist template + the deterministic-reduction spec, with
zero GPU code compiled (the default build has no GPU adapter, `engine/src/bridge.rs:214-218`).

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

> "Ground truth is non-discussible." Every claim below was read from source **this pass**
> (`/root/dowiz`, HEAD), not inherited from the research sketch.

### 0.1 The stencil step is single-writer *by construction* today (the exemption's root)

`FieldFrame::step` (`engine/src/field_frame.rs:198-222`) is a **pure gather**: each output cell `i`
reads `u`, `u_prev`, `source`, and the 5-point Laplacian of `u` (`laplacian_into`,
`field_frame.rs:140-154`) and writes exactly one cell of `next_scratch` (`field_frame.rs:215`). No
cell writes any other cell. The buffer lifecycle is **ping-pong**: two `std::mem::swap`s rotate
`u_prev ← u`, `u ← next` (`field_frame.rs:217-221`). This is the structural single-writer property:
the read set (`u`) and the write set (`next_scratch`) are **disjoint buffers**, so there is no
shared-write site to make atomic. R14 B1's "adding atomics to the stencil would be a bug" is
**correct** — and P88 keeps that conclusion; it only changes *why* it is granted (a written proof,
not the absence of a contention bench).

### 0.2 The first real shared-write site is the energy reduction (test-only today; GPU-future)

`engine/src/field_energy.rs` computes the field's Dirichlet/kinetic/coupling energy as a **sum over
all cells** (`field_energy.rs:77-102`: `dirichlet_energy` folds `x·(Lx)` at `:80`; `field_energy`
folds kinetic `:91-98`, potential `:99`, coupling `:100`). This module is `#[cfg(test)]` (declared
in `lib.rs`; header `field_energy.rs:15-17`) — a CPU reference oracle, **not on any hot path today**.
It is nonetheless the **archetype of the first real GPU-atomics site** (R14 B1.2): a parallel
reduction over cells needs either a tree reduction or an `atomicAdd` accumulator. This is where the
atomicity policy first bites when the field step ports to GPU compute.

### 0.3 The determinism contract the policy must protect

- `compose()` is bit-deterministic across calls (`field_frame.rs:427-451`,
  `compose_returns_deterministic_frame`; `:453-493`, `allocfree_step_byte_identical`). The CPU field
  path is a **bit-exact oracle** — the reference every approximate GPU path is measured against
  (P38 §3.1 honesty split).
- The stability guard is **fail-closed**: `assert_stable` panics outside the 2-D Jury bound
  `dt < (2+2Γ)/(8c²−M)` (`field_frame.rs:70-98`), called before every step (`:199`). Any GPU port
  inherits this precondition unchanged.
- Floating-point summation is **non-associative**: `(a+b)+c ≠ a+(b+c)` in IEEE-754. A GPU
  `atomicAdd<f32>` accumulates in **arbitrary invocation-completion order**, so its result is
  **run-to-run non-deterministic**. This is the mathematical fact §4.4's ban rests on.

### 0.4 The GPU domain does not exist in the default build (the gate)

`engine/Cargo.toml:36` — `webgpu = []` / fallback flags are **empty** today; `bridge.rs:214-218`
(`e21_default_build_has_no_real_gpu_adapter`) pins that the default build has **no real GPU
adapter**, and P38 §4.2 names the field/particle-step-to-GPU-compute move as an **operator-owned
decision** (`BLUEPRINT-P38-webgpu-render-engine.md` §4.2; the "GPU compute for the field step now"
alternative is explicitly REJECTED at P38 `:180-182`). P88's **policy** is buildable now (it is
text + a checklist + a reduction spec); P88's **shader legs** wait on P38 §4.2.

### 0.5 The CPU-kernel domain this policy deliberately does NOT govern

The Performance Standing Rule (bench-gated rewrites, no blanket application; substance preserved in
`SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md` preamble) governs the CPU kernel. Its live rulings stand
**unchanged**: **E4** (kernel `SeqCst→Relaxed` — declined, no evidence) and **E12** (`Mutex→CAS` —
bench-first). The contention bench `OPUS-PERF-CONTENTION-BENCH-RESULTS` (branch-only,
`perf/contention-bench-2026-07-18`; registered by P90) folds into **this CPU boundary** when it
lands — it can move specific CPU sites; it **cannot** dissolve the GPU-domain default P88 sets.

---

## 1. Prior-art map — adopt, don't invent (standard §2 item 19)

| Prior art | What it is | How P88 uses it — and what it does NOT take |
|---|---|---|
| **Fixed-point / integer reduction for reproducible GPU sums** (e.g. Kahan-free deterministic `atomicAdd<i32>`; the "deterministic reduction" pattern in HPC/graphics) | scale floats to a fixed-point integer domain; integer add is **associative**, so an atomic accumulator is order-independent and bit-reproducible | **Adopt** for every physics/GPU reduction (§4.4). **NOT taken:** float `atomicAdd` — banned in-domain (§4.4). |
| **Ping-pong / double-buffering** (GPGPU stencil idiom; already the engine's native `field_frame.rs:217-221`) | disjoint read/write buffers per step ⇒ each cell is written by exactly one invocation | **Adopt as the exemption's structural proof** — the *only* sanctioned way to earn a non-atomic write in this domain (§4.3). **NOT taken:** "it's probably race-free" hand-waving. |
| **WireGuard/Linux-kernel "unsafe requires a comment" discipline** (`// SAFETY:` at every `unsafe`) | every escape from the safe default carries a written justification checked at review | **Adopt verbatim in shape** as the `// SINGLE-WRITER:` proof comment on every non-atomic shared write (§4.2, §4.5). |
| **The evidence-gated standing rule** (bench-per-claim; R14 B1's default) | never add synchronization the numbers don't justify | **Kept for the CPU domain (E4/E12), demoted to *measurement* in the GPU domain** — the bench still runs, but it documents cost, it no longer *grants permission* (§4.4 item, §2 divergence row 4). |

**What P88 invents:** nothing runtime. It adds one **review artifact** (the WGSL shared-write
checklist, §4.2) and one **spec** (the deterministic fixed-point reduction, §4.4). Both are process
+ design, zero new dependency, zero new primitive.

---

## 2. Scope — what P88 owns vs deliberately does NOT (standard §2 items 11, 18, 19)

### 2.1 P88 OWNS

1. **The atomicity-by-default rule** for the physics/GPU domain and its single legal exemption
   (structural single-writer proof), stated as a rule the stencil can live under (§4.3).
2. **The WGSL shared-write review checklist** — a per-shader artifact enumerating every
   `var<storage, read_write>` and `var<workgroup>` write and classifying it (a)/(b)/(c) (§4.2).
3. **The deterministic fixed-point reduction spec** — the pattern every physics/GPU reduction ships
   as, with a reproducibility (N-run bit-identical) test (§4.4).
4. **The in-domain ban on float `atomicAdd`** and the fixed-order / fixed-point alternatives (§4.4).
5. **The domain boundary statement** — physics/GPU only; the CPU kernel stays evidence-gated (§4.6).

### 2.2 P88 does NOT own (anti-scope — prevents collision & scope-creep)

- **The CPU kernel's synchronization policy.** E4 (`SeqCst→Relaxed`) and E12 (`Mutex→CAS`) stay
  evidence-gated under the Performance Standing Rule; P88 does not touch `kernel/` atomics.
- **`kernel/src/money.rs` and the CPU determinism/crypto oracle.** These are the **excluded red-line**
  (`SYNTHESIS-PHYSICS-PERFORMANCE-VISION-2026-07-18.md` §5): `money.rs` is `i64` exact minor-units
  arithmetic; the oracle (`compose()`, FSM/order projections, crypto verify) is a bit-exact
  determinism contract. **No P88 mechanism — atomic or fixed-point — is ever applied inside them.**
  Fixed-point reduction here is a *presentation-side* GPU technique, hard-walled from money/oracle.
- **The GPU port itself** (moving `step()`/particles to compute shaders) — that is P38 §4.2's
  operator-owned decision. P88 constrains *how* those shaders are written, it does not authorize them.
- **P86's channel-lease/pair-pool API and P87's mask plane** — P88 supplies the atomicity rule they
  inherit; it does not design their data structures.

### 2.3 Dependencies (named by artifact — standard §2 item 7)

**Hard inputs (in tree):** `FieldFrame::step` + the ping-pong swap (`engine/src/field_frame.rs`);
`field_energy.rs` reduction machinery (the reduction archetype); the P38 render-stack seam
(`engine/src/bridge.rs`, the `gpu` feature). **Consumers:** P86 (its shader design inherits §4.3),
P87 (its mask-update shader inherits §4.3/§4.4), and any future WGSL compute shader in the engine.
**Soft input:** `OPUS-PERF-CONTENTION-BENCH-RESULTS` (P90) — folds into the CPU boundary (§4.6),
not into this policy.

### 2.4 Honest reconciliation with the research (standard §2 item 6)

R14 B1 is **not wrong** and P88 does not claim it is. Its *conclusion* — the stencil step needs no
atomics — is preserved verbatim (§0.1). What P88 changes is the *default and the burden of proof*:
the research would grant the exemption because no bench showed contention; P88 grants it because
ping-pong is a **structural single-writer proof**, which is stronger (it holds regardless of load)
and reviewable. The operator's informed, domain-scoped exception to the evidence-gated rule
(`SYNTHESIS §2` row 4; `performance-priority-over-minimal-change-2026-07-17.md`) is the authority for
this; §4.6 keeps the exception scoped to exactly the physics/GPU domain.

---

## 3. Predefined types & constants — named BEFORE implementation (standard §2 item 4)

Fixed-point scale and the checklist verdict enum are named, never magic. New types live in a **new
module `engine/src/gpu_atomicity.rs`** (feature-gated `gpu`; pure CPU-side spec + the reduction
reference until the shaders land). The WGSL checklist is a doc artifact, but its verdict is a typed
enum so the reduction reference and its tests can assert on it.

```rust
// engine/src/gpu_atomicity.rs  (NEW; #[cfg(feature = "gpu")] for the shader-facing parts,
//                               the fixed-point reference + tests are always-compiled)

/// Fixed-point scale for deterministic reductions. Chosen so the field's value range
/// (|U| ≤ 1.0 by the frame_rgba clamp, field_frame.rs:234) maps to i32 with headroom for
/// N-cell accumulation without i32 overflow: 2^20 ≈ 1.05e6, and 512² = 262 144 cells ×
/// max |scaled cell| 2^20 fits in i64 accumulation (the on-GPU atomic is i32 per-workgroup,
/// promoted to i64 at the two-level merge — see §4.4). Named, not magic.
pub const FIXED_POINT_SHIFT: u32 = 20;
pub const FIXED_POINT_SCALE: f64 = (1u64 << FIXED_POINT_SHIFT) as f64; // 1_048_576.0

/// Classification of a single shared-write site in a physics/GPU shader (the checklist verdict).
/// Every `var<storage, read_write>` / `var<workgroup>` write MUST resolve to exactly one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedWriteClass {
    /// (a) the write is an atomic op (atomicAdd/atomicMax/… on an integer atomic).
    Atomic,
    /// (b) the write is separated from conflicting reads by a workgroupBarrier()/storageBarrier().
    BarrierSeparated,
    /// (c) the write is proven single-writer by construction (ping-pong, per-invocation-disjoint
    ///     output index). REQUIRES a written `// SINGLE-WRITER:` proof at the site.
    SingleWriterProof,
}

/// One row of the per-shader shared-write checklist (the review artifact, §4.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedWriteAudit {
    pub shader: &'static str,     // e.g. "field_step.wgsl"
    pub binding: &'static str,    // e.g. "var<storage, read_write> next: array<f32>"
    pub class: SharedWriteClass,  // MUST be set; a shader with any unclassified write is RED
    pub proof: &'static str,      // the // SINGLE-WRITER: text for class (c); "" otherwise
}
```

**Policy constant (the one-line ban surface):**

```rust
/// Float atomicAdd is BANNED in the physics/GPU domain (non-associative → non-deterministic sum,
/// §0.3/§4.4). This const documents the ban at a greppable site; the enforcement is the review
/// checklist + the reproducibility test (a float-atomic reduction fails N-run bit-identity).
pub const FLOAT_ATOMIC_ADD_BANNED_IN_DOMAIN: bool = true;
```

---

## 4. The policy in full — spec → RED test → artifact (standard §2 items 2, 3, 5)

### 4.1 The rule (the load-bearing statement)

> **Every cross-invocation shared-write site in physics/GPU code is atomic BY DEFAULT. Exemption is
> granted only by a STRUCTURAL PROOF of single-writer disjointness, recorded at the site.**

"Cross-invocation shared-write site" = any `var<storage, read_write>` or `var<workgroup>` location
that a shader invocation writes and that a *different* invocation (same dispatch or a later one)
reads or writes. A write whose output index is a pure function of the invocation id (gather/scatter
to a disjoint slot) is single-writer and eligible for exemption (c); everything else is atomic (a)
or barrier-separated (b).

### 4.2 The WGSL shared-write checklist (the standing review gate — the anti-scope-creep artifact)

For **each** landed physics/GPU compute shader, a checklist (a `Vec<SharedWriteAudit>`) enumerates
every shared write and classifies it. The shader does not merge until every write resolves to
`Atomic`, `BarrierSeparated`, or `SingleWriterProof` **with a non-empty `proof`**. This is the
domain's standing review artifact (parallel to P92's `FASTPATH_ELIGIBLE_SCOPES` allow-list: a
single, greppable, test-guarded surface).

- **RED `red_unclassified_shared_write_blocks_merge`:** a checklist row with a class-(c) verdict and
  an **empty** `proof` string fails the audit test — i.e. an exemption without a written proof is
  rejected. RED against a naive "trust me it's single-writer" entry; GREEN only with the proof text.
- **RED `red_stencil_step_is_single_writer_exempt`:** the `field_step.wgsl` (when it lands) checklist
  classifies its `next` buffer write as `SingleWriterProof` with the ping-pong proof text; the audit
  passes. This encodes §0.1 as a machine-checkable row, so a future edit that makes the step write a
  *shared* accumulator (a real bug) has no valid class and fails.

### 4.3 The reconciliation — the stencil under the rule (the design move)

Under §4.1 the pure-gather stencil step (`field_frame.rs:198-222`) does **not** get pessimal per-cell
atomics — its `next` write is class (c) `SingleWriterProof`. The proof text, recorded at the site:

```wgsl
// SINGLE-WRITER: ping-pong. Read set = `u` (prev buffer); write set = `next` (disjoint buffer).
// Output index i == global_invocation_id; each cell written by exactly one invocation. The two
// std::mem::swap-equivalent buffer rotations (CPU: field_frame.rs:217-221) keep read/write disjoint.
// No cross-invocation shared write exists → atomic-free is CORRECT, not merely un-benched.
```

This is stronger than the research's grant: it holds at **any** contention level, and a reviewer can
falsify it by pointing at a single cell written by two invocations. R14's conclusion survives as this
exemption's *justification*; its evidence-gated *default* does not govern this domain.

### 4.4 Reductions & scatters — atomic-and-deterministic from day one

- **The reduction spec.** The energy/Lyapunov reduction (the first real GPU-atomics site, §0.2) ships
  as a **two-level** pattern: (1) each workgroup reduces its cells into a **fixed-point `i32`** local
  accumulator via `atomicAdd` (integer add is **associative** → order-independent within the
  workgroup); (2) workgroup partials merge into an **`i64`** total (fixed-order tree or a single
  `atomicAdd<i32>` promoted at read), then de-scale by `FIXED_POINT_SCALE`. The sum is
  **bit-reproducible run-to-run**. This is where P87's quantization theme *buys* correctness:
  fixed-point is the mechanism that makes an atomic reduction deterministic.
- **Float `atomicAdd` is BANNED in-domain** (not merely discouraged): non-associative + arbitrary
  completion order = non-deterministic (§0.3). Only fixed-point atomics or fixed-order reduction
  trees are permitted.
- **Scatter passes** (future particle→grid deposition, spatial-hash inserts) are **born atomic** —
  no "probably race-free" first drafts. Their accumulation into grid cells is a many-writer site with
  no ping-pong disjointness, so class (a) is mandatory and, if it sums floats, must go through the
  fixed-point domain.

- **RED `red_fixed_point_reduction_is_run_deterministic`:** run the fixed-point reduction over the
  same cell buffer `N=32` times (simulated arbitrary accumulation order on CPU, or real dispatch when
  GPU lands) → all `N` sums bit-identical. RED against a float-accumulate reference (which drifts in
  the low bits under reordered summation); GREEN for the fixed-point path.
- **RED `red_float_atomic_reduction_is_nondeterministic` (the non-vacuousness proof):** the SAME
  N-run test over a *float*-accumulate reduction with a shuffled summation order **must** exhibit at
  least two differing sums — proving the ban is answering a real hazard, not a decorative rule. (This
  is the P88 analogue of `field_energy.rs`'s `energy_gate_catches_anti_diffusion` non-vacuousness
  discipline.)
- **RED `red_reduction_matches_cpu_oracle`:** the de-scaled fixed-point total matches the CPU
  `field_energy` oracle (`field_energy.rs:87-102`) within the fixed-point quantization tolerance
  (`≤ 2^-FIXED_POINT_SHIFT × N`), so the GPU reduction is *measured against* the authority, never
  replacing it.

### 4.5 The `// SINGLE-WRITER:` comment discipline (the smart index for the bug class)

Every class-(c) exemption carries a `// SINGLE-WRITER:` proof comment. A CI grep asserts that **every**
non-atomic write to a `var<storage, read_write>` in a physics shader is preceded by such a comment
(mirrors P38 §4.3's per-bug-class CI gates). The bug class this introduces — "a shared accumulator
written non-atomically because someone assumed disjointness" — becomes a **CI-time** failure (missing
comment or a comment a reviewer can falsify), not a runtime race.

- **RED `red_nonatomic_write_without_proof_comment`:** a physics shader with a non-atomic
  `read_write` store and no `// SINGLE-WRITER:` comment fails the CI grep. GREEN when the comment is
  added (and the review confirms it).

### 4.6 Domain boundary (explicit, per the operator's scoping)

This default governs the **physics/GPU domain only** — the engine's field/particle render compute.
The **CPU kernel domain** stays evidence-gated: **E4** (kernel `SeqCst→Relaxed` declined) and **E12**
(`Mutex→CAS` bench-first) stand unchanged under the Performance Standing Rule. When
`OPUS-PERF-CONTENTION-BENCH-RESULTS` (P90) lands, its data **folds into this boundary**: it can move
specific CPU sites (per E12's bench-first rule), it **cannot** dissolve the GPU-domain default. The
money/oracle red-line (§2.2) is outside both — no P88 mechanism touches it.

### 4.7 WebGL2 / CPU-floor fallback — the atomicity rule is WebGPU-compute-scoped (FE-16 floor)

P38's FE-16 fallback ladder is **WebGPU → WebGL2 → CPU `compose_field`** (`BLUEPRINT-P38-webgpu-render-engine.md`
§3.6, §12.3); a courier's mid-tier phone may reach only the WebGL2 rung. The atomicity-by-default rule
(§4.1) governs exactly one thing: **concurrent shared writes across GPU *compute* invocations** —
WGSL `var<storage, read_write>` / `var<workgroup>`, `atomicAdd`, `workgroupBarrier`, two-level
workgroup reductions. That concurrency **exists only on the WebGPU compute path**. Core WebGL2
(GLES 3.0) has **no compute shaders and no compute atomics at all**, and the CPU floor is
single-threaded. So the rule is **WebGPU-compute-only by construction**, and P88 states this plainly
rather than leaving it ambiguous (closing the META-GAP-AUDIT-2026-07-19 §1 G2 finding):

- **On the WebGPU rung (post-OD-11):** the full policy applies — atomic-by-default, the fixed-point
  deterministic reduction (§4.4), the `// SINGLE-WRITER:` structural exemption (§4.3).
- **On the WebGL2 and CPU-floor rungs:** the field step **and** the energy/Lyapunov reduction run on
  the **existing single-threaded CPU path** (`FieldFrame::step`, the `field_energy` fold — §0.1/§0.2),
  where there is **exactly one writer and zero cross-invocation concurrency**. Therefore **atomicity is
  moot** on these rungs: there is no shared-write site to make atomic, and the reduction is a plain
  sequential fold whose result is fixed by evaluation order. WebGL2 is used here only for
  *fragment-shader presentation* of the CPU-computed frame (or canvas2d `putImageData` on the CPU
  floor); it never runs the physics compute.

This is **option (i)** of the audit's G2 choice, taken explicitly and by name: **the compute legs are
WebGPU-only, and the WebGL2 floor never runs them — the CPU path handles the physics on that rung,
where no concurrent writer exists.** The alternative — a WebGL2-specific fragment multi-pass
(log-depth) reduction, option (ii) — is deliberately **NOT** adopted: the CPU fold is already the
bit-exact authority (P38 §3.1) and simpler, and standing up a second reduction implementation on
WebGL2 would add a redundant surface with its own determinism burden for zero benefit (the WebGL2 rung
is not asked to be the physics authority).

**Determinism corollary:** the CPU fold is sequential (fixed summation order) and thus already
run-to-run deterministic **without** fixed-point — the fixed-point machinery (§4.4, `FIXED_POINT_SHIFT`)
is an artifact of GPU-compute completion-order nondeterminism and is needed **only** on the WebGPU
compute rung. The CPU reduction is precisely the oracle the WebGPU reduction is measured against
(`red_reduction_matches_cpu_oracle`, §4.4).

**DoD addition — the FE-16 floor line (P38 §12.3 standing gate), previously absent (audit G2):**
**D-WEBGL2** — physics/field behavior is verified **CORRECT on the WebGL2 and CPU floors, not only on
WebGPU.** Falsifier: `compose_returns_deterministic_frame` + `allocfree_step_byte_identical` (the CPU
floor) stay green; and a forced-`navigator.gpu = undefined` degrade path (P38 §3.6 pattern) is asserted
to run the CPU physics with **no compute-atomics dependency reachable**. Any physics-correctness claim
that passes only under the WebGPU compute path, or a build that makes the field step *require* compute
atomics (unreachable on WebGL2), = **NOT done** regardless of WebGPU-rung green totals.

---

## 5. AI/system-hazard safety, grounded in math (standard §2 item 6)

The unsafe states are made **unrepresentable or CI-caught**, not policed by prose:

- **Non-deterministic reduction** (a GPU sum that differs run-to-run, silently corrupting the oracle
  comparison): unrepresentable in the sanctioned path because integer add is associative — the
  fixed-point accumulator's result is a pure function of the multiset of inputs, independent of
  completion order (§0.3, `red_fixed_point_reduction_is_run_deterministic`). A float accumulator that
  *would* be non-deterministic is banned at the checklist + caught by
  `red_float_atomic_reduction_is_nondeterministic`.
- **An un-proven data race** (a shared write assumed disjoint but not): CI-caught by the missing
  `// SINGLE-WRITER:` comment (§4.5) and by the empty-`proof` audit failure (§4.2). The exemption
  requires a *reviewable* structural argument, so an incorrect one is falsifiable at review, not at
  runtime.
- **Oracle corruption** (a GPU reduction feeding back into the authoritative CPU state): structurally
  impossible under the P38 §3.1 honesty split — the GPU path is **presentation-side only**; the CPU
  `compose()` remains the bit-exact authority (`field_frame.rs:427-451`). The reduction is *measured
  against* the oracle (`red_reduction_matches_cpu_oracle`), never written back into it.

Reachability is argued from associativity + the honesty split + CI gates — no policy/prose assurance.

---

## 6. DoD — falsifiable, machine-checkable (standard §2 item 2)

| # | Done when… | Falsifier (RED test / check) |
|---|---|---|
| D1 | the WGSL shared-write checklist exists as a typed artifact and rejects unproven exemptions | `red_unclassified_shared_write_blocks_merge` (M-checklist) |
| D2 | the checklist is **applied to the first landed physics/GPU compute shader** (the field step or reduction), every write classified | `red_stencil_step_is_single_writer_exempt` + the shader's own audit `Vec<SharedWriteAudit>` all-classified |
| D3 | the deterministic fixed-point energy reduction is implemented with an N-run bit-identity test | `red_fixed_point_reduction_is_run_deterministic` |
| D4 | float `atomicAdd` non-determinism is *demonstrated* (non-vacuousness) and banned | `red_float_atomic_reduction_is_nondeterministic` |
| D5 | the GPU reduction is measured against the CPU oracle within quantization tolerance | `red_reduction_matches_cpu_oracle` |
| D6 | every non-atomic physics shared write carries a `// SINGLE-WRITER:` proof (CI grep) | `red_nonatomic_write_without_proof_comment` |
| D7 | a contention/throughput microbench accompanies each atomic site (documents cost) | §7 bench output present in `docs/regressions/REGRESSION-LEDGER.md` |
| D-POLICY | the policy text + domain boundary are recorded and cited by P86/P87 as their inherited rule | this document referenced from P86/P87 §2.3 |
| D-NOREG | the CPU field oracle stays bit-identical (no policy change touches `compose()`) | `compose_returns_deterministic_frame`, `allocfree_step_byte_identical` stay green |
| D-WEBGL2 | field/reduction behavior is CORRECT on the WebGL2 + CPU floors (the atomicity rule is WebGPU-compute-only; those rungs run the single-threaded CPU path where atomicity is moot) — §4.7 | CPU-floor oracle green + forced-`navigator.gpu=undefined` runs the CPU physics with no compute-atomics dependency reachable (P38 §12.3 floor line) |

**Note (write-first):** D-POLICY, D1, and the *spec* for D3–D6 are deliverable **now** (CPU-side
reference + checklist). D2 and the *shader* legs of D3–D7 activate when P38 §4.2 is taken.

---

## 7. Benchmarks + telemetry + measurement (standard §2 item 10)

The standing rule's bench-per-claim survives here as **measurement**, no longer as **permission**
(§2.4). Each atomic site ships a microbench documenting its cost:

| Bench | Measures | Harness |
|---|---|---|
| `bench_reduction_fixed_point_vs_tree` | fixed-point `atomicAdd<i32>` vs a fixed-order reduction tree, per grid {128², 256², 512²} | P81 engine bench harness (the C2 substrate) |
| `bench_reduction_vs_cpu_oracle` | GPU reduction latency vs the CPU `field_energy` fold | P81 |
| `bench_stencil_step_no_atomic` | confirms the exempt stencil pays zero synchronization cost (baseline) | P81 |

Telemetry: emit per-dispatch `{reduction_ns, run_variance_bits}` through the P38 render metrics seam
so a determinism regression (a nonzero `run_variance_bits` = someone reintroduced a float atomic)
surfaces automatically (item 14), not at review time.

---

## 8. Cross-cutting obligations (standard §2 items 8, 9, 11–16, 20)

- **Schemas & scaling axis (item 8):** the reduction's scaling axis is **cells per grid** — the
  two-level pattern holds to `512² = 262 144` cells within `i32` per-workgroup + `i64` merge
  headroom (§3, `FIXED_POINT_SHIFT` note). Beyond ~`1024²` the merge tree gains a level (named break
  point), and `FIXED_POINT_SHIFT` may need lowering to preserve `i64` merge headroom — stated, not
  timeless.
- **Isolation / bulkhead (item 11):** the policy is a **review-time bulkhead** — a non-compliant
  shader never merges, so a race cannot reach the running system. The GPU path itself is bulkheaded
  from the CPU authority by the P38 §3.1 split (a GPU reduction bug corrupts a *displayed number*,
  never an order).
- **Mesh awareness (item 12):** **N/A, honestly** — the physics/GPU render is entirely node-local
  (P38 §4.4); atomicity is an on-device GPU concern with zero transport payload. Stated, not padded.
- **Rollback / self-healing as math (item 13):** **Self-Termination** = the checklist gate (a
  non-compliant shader is an unrepresentable *merge*, not a supervisor's veto). **Snapshot-re-entry /
  self-healing: NOT claimed** — this is a review policy, there is no runtime recovery to claim.
  Mechanical rollback: `gpu` feature OFF restores today's exact (atomic-free CPU) build.
- **Error-propagation / smart index (item 14):** the bug classes (a silent float atomic; an unproven
  non-atomic write) become **CI-time** failures (§4.2/§4.5) + a telemetry tripwire (§7), never a
  runtime surprise.
- **Living-memory awareness (item 15):** reductions are ephemeral per-frame values, deliberately not
  persisted; anything durable uses the CPU oracle path. Time-scoped by frame, not stored.
- **Tensor/spectral (item 16):** the reduction is a `xᵀLx` fold over the kernel's `+(D−A)` operator
  (`field_energy.rs:77-102`) — it reuses the spectral machinery; P89 (field eigenmodes) is the
  spectral *consumer*, P88 only fixes how its GPU reduction is summed.
- **Linux discipline (item 9):** **EXTENDS** — the `// SINGLE-WRITER:` proof-comment discipline is a
  new gate this repo adds, modeled on the kernel's `// SAFETY:` convention; **REINFORCES** the
  feature-gated-hardware-with-software-floor pattern; **ALREADY-EQUIVALENT** on fail-closed
  determinism (reuses the `assert_stable` + bit-oracle patterns).
- **Hermetic principles (item 20):** **Polarity / no-middle** — a shared write is either atomic or
  proven single-writer; there is no "probably fine" middle state that could carry a race.
  **Correspondence** — the fixed-point sum *is* a pure function of its inputs ("as the multiset, so
  the sum"), independent of order; determinism is structural, not asserted.

---

## 9. Standard-compliance map (all 20 points — standard §2)

| # | Standard item | Where satisfied |
|---|---|---|
| 1 | Ground truth, live `file:line` | §0 (stencil single-writer, energy reduction archetype, determinism contract, GPU-gate, CPU boundary) |
| 2 | Falsifiable DoD | §6 (D1–D-NOREG) |
| 3 | Spec→test→artifact, event-modeled | §4 (rule → checklist → RED tests; reduction as a two-level event) |
| 4 | Predefined types & constants | §3 (`SharedWriteClass`, `SharedWriteAudit`, `FIXED_POINT_SHIFT`) |
| 5 | Adversarial/breaking tests | §4.4 (`red_float_atomic_reduction_is_nondeterministic` non-vacuousness), §4.5 |
| 6 | Hazard-safety from math | §5 (associativity + honesty split + CI gates) |
| 7 | Links to docs & memory | §10 |
| 8 | Schemas with scaling axis | §8 (cells/grid; i32/i64 headroom break point) |
| 9 | Linux engineering discipline | §8 (EXTENDS/REINFORCES/ALREADY-EQUIVALENT verdict) |
| 10 | Benchmarks + telemetry | §7 (bench-as-measurement; run-variance telemetry) |
| 11 | Isolation / bulkhead | §8 (review-time bulkhead; P38 §3.1 GPU/CPU split) |
| 12 | Mesh awareness | §8 (N/A, node-local, stated) |
| 13 | Rollback/self-heal as math | §8 (self-termination = checklist gate; self-healing NOT claimed) |
| 14 | Error-propagation / smart index | §4.5 (CI grep), §7 (telemetry tripwire) |
| 15 | Living-memory awareness | §8 (ephemeral per-frame; durable = oracle path) |
| 16 | Tensor/spectral | §8 (xᵀLx reduction reuses kernel operator; P89 is the consumer) |
| 17 | Regression tracking | §6 D7 (bench recorded), D-NOREG (oracle bit-identity) |
| 18 | Clear worker instructions | §10 |
| 19 | Reuse-first, upgrade-if-needed | §1 (ping-pong / fixed-point / SAFETY-comment all adopted), §2.2 (anti-scope) |
| 20 | Hermetic principles | §8 (Polarity, Correspondence) |

---

## 10. Links to docs & memory + instructions for other agentic workers (standard §2 items 7, 18)

**Depends on / cites:**
- `SYNTHESIS-PHYSICS-PERFORMANCE-VISION-2026-07-18.md` §2 row 4 (the operator override), §4.4 (this
  unit's sketch), §5 (the money/oracle exclusion P88 honors).
- `MASTER-STATUS-LEDGER-2026-07-19.md` §1 (P88 row), §3 wave-1 (write-first), §4 item 13.
- `docs/research/OPUS-PHYSICS-WAVE-ALGORITHMS-AND-TENSOR-SYNTHESIS-2026-07-18.md` §B1 (the overruled
  gating philosophy — recovered, restore per Ledger §0), `OPUS-PINGPONG-SHADOW-COPY-PROPAGATION-2026-07-18.md`.
- `BLUEPRINT-P38-webgpu-render-engine.md` §3.1 (honesty split), §4.2 (operator-owned GPU-compute
  decision), §4.3 (per-bug-class CI gates model).
- `CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (the 20-point contract).
- Memory: `performance-priority-over-minimal-change-2026-07-17.md` (the scoped-perf-override authority
  for the domain exception), `ground-truth-over-proxy-2026-07-07.md`.

**Existing code this blueprint edits/extends (exact targets, dowiz — NOT bebop-repo):**
- **NEW** `engine/src/gpu_atomicity.rs` — §3 types (`SharedWriteClass`, `SharedWriteAudit`,
  `FIXED_POINT_SHIFT`); the fixed-point reduction reference + its N-run determinism + oracle-match
  tests (always-compiled); the shader-facing checklist under `#[cfg(feature = "gpu")]`.
- **EDIT (when P38 §4.2 taken)** the field-step / reduction WGSL shaders — each carries its
  `Vec<SharedWriteAudit>` checklist and `// SINGLE-WRITER:` proof comments.
- **REUSE unchanged** `engine/src/field_frame.rs` (ping-pong, the exemption root), `field_energy.rs`
  (the reduction oracle), `engine/src/bridge.rs` (the render seam + `e21` guard).
- **DO NOT TOUCH** `kernel/src/money.rs`, the CPU determinism/crypto oracle (`compose()`, FSM/order
  projections), or `kernel/` atomics (E4/E12 domain).

**For the worker with zero session context — exact acceptance path:**
1. **Write the policy first** — `gpu_atomicity.rs` types + the CPU-side fixed-point reduction
   reference + its determinism/non-vacuousness/oracle-match tests. This is deliverable with **no GPU
   feature** and constrains P86/P87; do it before either.
2. Wire the WGSL checklist artifact + the `// SINGLE-WRITER:` CI grep (a shell/CI check over
   `engine/**/*.wgsl`, dormant until a shader exists).
3. **Do NOT write any physics compute shader until P38 §4.2 is operator-approved.** When it is: the
   FIRST landed shader ships with a complete `Vec<SharedWriteAudit>`, every write classified, the
   stencil write proven class-(c), the reduction fixed-point + determinism-tested.
4. Add the D7 bench numbers + the D1/D3/D4 REDs-now-GREEN to `docs/regressions/REGRESSION-LEDGER.md`.
5. Anti-scope: never apply fixed-point/atomic mechanisms inside `money.rs` or the oracle; never grant
   a non-atomic exemption without a written `// SINGLE-WRITER:` proof; never let a float `atomicAdd`
   into a physics reduction.

---

## 11. Open operator-decision points (nothing silently taken)

| # | Decision | Blocks / affects | Default if unruled |
|---|---|---|---|
| OD-P88-1 | **P38 §4.2** — the field/particle-step-to-GPU-compute decision (operator-owned) | P88's *shader* legs (D2, D3-shader, D7); all of P86/P87 build | Policy + CPU reference land; no GPU shader written |
| OD-P88-2 | `FIXED_POINT_SHIFT` value (default 20) — trades reduction precision vs `i64` merge headroom at ≥1024² grids | D3/D5 tolerance | 20 (safe to 512²; revisit at 1024²) |
| OD-P88-3 | Whether the CPU-domain contention bench (P90/`OPUS-PERF-CONTENTION-BENCH-RESULTS`) is allowed to move any **E12** site once merged | CPU boundary (§4.6) — NOT the GPU default | E12 bench-first stands; GPU default unaffected either way |

---

*Cross-references: `SYNTHESIS-PHYSICS-PERFORMANCE-VISION-2026-07-18.md` (§2/§4.4/§5) ·
`SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md` (E4/E12, Performance Standing Rule) ·
`MASTER-STATUS-LEDGER-2026-07-19.md` (P88 row, wave sequence) ·
`BLUEPRINT-P38-webgpu-render-engine.md` (§3.1/§4.2/§4.3) · `BLUEPRINT-P87-...` (inherits §4.3) ·
`BLUEPRINT-P86-...` / `OPUS-PERF-ARENA-DEEPDIVE` (inherits §4.3) · `CORE-ROADMAP-STANDARD-2026-07-17.md`
§2 · source: `engine/src/field_frame.rs`, `engine/src/field_energy.rs`, `engine/src/bridge.rs` ·
memory: `performance-priority-over-minimal-change-2026-07-17.md`, `ground-truth-over-proxy-2026-07-07.md`.*
