# Math-First Architecture — The Full-Rewrite Blueprint

> Status: **v2 (2026-07-14)**. Grounded in a four-lane live inspection of the whole
> code surface (dowiz kernel, bebop math, toolchain, low-level/hardware). Every
> claim marked **PROVEN** (a test/build was run) · **MEASURED** (a value was read
> off the machine) · **RESEARCHED** (external source) · **ESTIMATED** (judged).
> Companion artifacts: `tools/eqc/` (the equation compiler, built + proven this
> turn), `docs/design/psyonic-spectral-kernel.md` (the organ roadmap),
> `docs/design/bebop-autonomous-self-evolution.md` (one consumer of this foundation).
>
> **v2 binds the operator's non-negotiable architecture (§1.5): decentralized ·
> non-AI · mesh · local-first · Data-Oriented-Design · bridges-&-adapters · with
> spectral-waves as the applied math principle** — and reconciles this substrate
> rewrite with the product plan in `docs/design/MASTER-ROADMAP-10-PHASES-2026-07-14.md`.
> The rewrite is the *engine* under that roadmap's P2 (kernel math authority) and
> P9 (growth substrate); it never front-runs its INVARIANT — *build DOWN from the
> first real order, not UP from the protocol.*

## 0. Thesis (one sentence)

**Stop writing instructions; implement laws.** The kernel's internal logic is a set
of *equations* authored once as a machine-checkable source of truth and **generated**
into Rust (no hand-transcription), carried in **integer fixed-point where determinism
is required** and float where only dynamics are, crossed to the outside world through
**zero-copy shared memory** (not RPC/proxies), and driven down toward the metal by
**SIMD** — with the honest boundary that FPGA/ASIC buy this workload nothing. The one
invariant that governs the whole descent: *a result must be reproducible before it is
fast* — determinism is the floor, speed is the optimisation on top of it.

## 1. What "math-first" means precisely

From `psyonic-spectral-kernel.md` §1, sharpened by the inspection:

| Part | Is | Discipline |
|---|---|---|
| **Body** | the bare-metal Rust kernel | equations *run* here; zero abstraction distance formula↔instruction |
| **Skeleton** | red-lines: integer money, the FSM Law, auth/RLS/PQ-crypto | rigid; the field never deforms it 🔴 |
| **Mind** | spectral / dynamical operators (`|λ|` vs 1) | metacognition = the spectrum of the system's own behaviour |
| **Senses** | thin code bridges (I/O, net, DB, WASM, LLM judge) | the only external ground; stays code, stays minimal |
| **Metabolism** | the reverse-engineering loop | converts legacy tissue → proven kernel-math tissue |

"Math-first" is not "use more math." It is: **the internal logic has no state that is
not described by an equation, and no equation that reaches the repo unproven.** Side
effects, hidden state, and transcription bugs are the enemy — each is a place where the
system can do something the equations don't say.

## 1.5 Architecture invariants (non-negotiable — the Ferrari's spec)

Seven properties every phase must preserve. They are not aspirational — each already
has a load-bearing organ in the tree; the rewrite's job is to keep them true *by
construction*, not by convention.

1. **Decentralized · mesh · local-first.** Authority is per-node, not central. State is
   grow-only CRDT + content-addressed event log (`kernel::event_log`,
   `event_id=SHA3(prev‖pubkey‖seq‖payload)`); consensus is a lattice-join fixpoint (pull
   anti-entropy + Merkle equality), *not* a coordinator. A fresh node computes offline
   from its own log; sync is reconciliation, never a round-trip to a server. (Master-P6:
   the mesh *protocol* stays PARKED until a real dowiz flow carries it — but the
   substrate stays mesh-shaped so nothing has to be un-built later.)
2. **Non-AI in the core loop.** The kernel is deterministic pure functions — **no LLM in
   the decision path.** An LLM is only a *sense* at the edge (advisory, outside the loop),
   exactly as `resonator`'s actors are pure `fn` pointers with no I/O. Reproducibility and
   auditability both die the moment a probabilistic model sits inside the loop.
3. **Data-Oriented Design (DOD).** Data is laid out as it lives in memory — flat,
   contiguous, struct-of-arrays where it's iterated. This is the *same change* as the
   heap-kill (§5): the `Vec<Vec<f64>>` matrices become one contiguous `Vec<f64>`/stack
   array, which is simultaneously the DOD layout, the alloc-free path, and the SIMD
   prerequisite. Names track physical quantities (`force = mass * acceleration`), so the
   code *is* the documentation. (Also honours DoD-as-Definition-of-Done via §benchmark: a
   phase isn't "done" until its speed gate is green.)
4. **Bridges & adapters (engine vs chassis).** The math core is pure `f(x)→y` and knows
   nothing of network/disk/user; every I/O concern lives in a thin adapter — the *only*
   place "legacy" is allowed. Swap TCP→RDMA, JSON→shared-memory, or one transport for
   another without touching the engine. Already real: `engine/src/bridge.rs` (numeric
   adapter), the `wasm.rs` marshalling shell, the `proto-wire`/`proto-cap`/`proto-crypto`
   protocol adapters, and the wasip2 capability-isolated ports. Pillar C hardens this.
5. **Spectral-waves = the applied math principle.** The recurring operator across every
   layer is the **graph-Laplacian spectrum + wave/heat propagation** and the `|λ|`-vs-1
   readout: loop-health & drift (`spectral` γ/SLEM, `markov`), UI field (`field`
   `u(t)=Σe^{−λ_k t}⟨u0,φ_k⟩φ_k`), interference (`coherence` `|ψ₁±ψ₂|²`), brittleness
   (`wavefield` graph-Fourier notch, algebraic connectivity λ₂), and — when a contact
   graph exists — mesh mixing-time. One math principle, applied everywhere it recurs.
6. **Manifest-in-code.** Every math module carries, at its top, the equation it implements
   and its physical/mathematical justification (the convention already pervades the tree:
   the closed form is written above the fn). No organ is a black box you have to reverse
   in a month.
7. **Benchmark-as-speedometer.** Speed is the default state, gated in CI: each organ has a
   micro-benchmark vs a naïve reference; a regression (12ns→13ns) must be *explained*, not
   waved through. This is quality control, not premature optimisation — and it is the
   falsifiable Definition-of-Done for every rewrite phase.

## 2. Ground truth — where we are today (the 4-lane survey)

**Kernel organs already math-first (dowiz, PROVEN inventory):** `money` (integer i64/i128,
`SCALE=1_000_000`, zero float — the red line held), `cart` (checked integer), `domain`
(order-total law), `geo` (haversine/bearing/ETA — f64 dynamics, **stack-only, no heap**),
`spectral` (Faddeev-LeVerrier + Durand-Kerner eigensolver), `markov` (attractor detector,
this session), `absorbing` (fundamental-matrix funnel, this session), `order_machine` (FSM
graph laws + golden-signature gate), `event_log` (SHA3 content-addressing).

**The honest gaps the survey found:**

1. **Translation gap — no equation source of truth.** Every equation is hand-written
   Rust; there is **no** `build.rs`, **no** proc-macro, **no** equation IR in either repo
   (MEASURED). The "spec" lives only as a doc-comment above each fn. → **Pillar A (eqc).**
2. **Determinism is convention, not mechanism.** The money/float split is correct and
   documented, but the *only* float that touches money (`tax_rate: f64` →
   `(tax_rate*1e6).round()`, `money.rs:33`) is still a float, and there is **no scaled
   fixed-point type anywhere** outside crypto (MEASURED — grep for `Q16/Q32/FixedPoint`
   returns nothing in either repo). → **Pillar B.**
3. **The WASM bridge is serialize-and-copy.** Every kernel↔JS call crosses as a JSON or
   CSV **String** (`wasm.rs`), one alloc+copy each way. A real zero-copy design exists but
   is **unwired** (`engine/src/bridge.rs` `VertexBridge`, a `&[f32]` staging contract with
   zero deps). → **Pillar C.**
4. **No machine-checked proofs available.** lean/coq/z3/kani/creusot/prusti are **all
   absent** (MEASURED); z3 + coq are apt candidates, not installed. Today's ceiling is
   generated **parity/property tests**. → **Pillar D.**
5. **The kernel is not tuned to the metal.** dowiz-kernel is plain **std** (not no_std),
   has **exactly one** `#[inline]` in the whole crate (on a non-math fn), **no**
   `target-cpu`/RUSTFLAGS/`.cargo/config`, and `absorbing`/`spectral`/`markov` allocate a
   fresh `vec![vec![0.0;n];n]` **per call** (MEASURED). → **§4–5.**
6. **Numeric-kernel duplication = live silent-drift risk.** The Jacobi eigensolver exists
   **four times** across bebop (`field.rs`, `kalman.rs`, `lyapunov.rs`, `wavefield.rs`) and
   has **already drifted** (one copy lost a degenerate-rotation guard); the `exp` shim
   exists **three times**; dowiz adds its own FL+DK. This is the exact hazard the kernel
   exists to kill. → **the rewrite's consolidation target (§7).**

**Assets to build ON, not from scratch:** bebop2-core is *already* correctly `no_std`-gated
(the PQ-crypto core compiles to wasm with an **empty import section**) — the template dowiz
should copy; `engine/src/bridge.rs` is a ready zero-copy contract; SymPy's `rust_code` printer
is a working codegen backend (PROVEN, §3A).

## 3. The four pillars

### Pillar A — Equation → code (`eqc`): the translation gap, closed [BUILT + PROVEN]

`tools/eqc/` (this turn). You author the equation once as a SymPy expression; `eqc` emits
Rust. **Dual emission from one source:** the f64 variant (SymPy's `rust_code`) *and* a
**fixed-point Q-format** variant (a custom printer over the expr tree: `Add→I₁+I₂`,
`Mul→(I₁·I₂)/2^SHIFT` in i128, `Pow(n≥0)→repeated mul`, consts scaled), plus a
**self-asserting proof program** that `rustc`-compiles and runs.

PROVEN end-to-end (`test_eqc.py`, generate→rustc→run→assert vs SymPy):

```
✓ quad → f64+fixed proven     ✓ tax → f64+fixed proven     ✓ hyp(sqrt) → f64 proven, fixed REFUSED
ALL EQC PROOFS PASSED (3/3 equations, rustc-compiled + executed)
```

It is **build-time, offline** codegen emitting committed, hand-inspectable Rust — *not* a
runtime transpiler/proxy (respecting `bebop2/ARCHITECTURE.md`'s standing directive). The
generated money law comes out crystalline:
`tax_fixed(sub,rate) = (((rate as i128)*(sub as i128))/4294967296i128) as i64` — bitwise
identical on every target. **This is how we resolve the "translation gap" (your point 4):
the equation is the source of truth, the parity test is the guard, and the four drifting
Jacobi copies become one generated organ.**

### Pillar B — Fixed-point determinism: the crystalline kernel

Float is not deterministic across targets for *transcendentals* — `sin/cos/atan2/asin/ln/
log2/sqrt/hypot` are not guaranteed bit-identical between native x86_64 and wasm32 (no FMA
by default), and reordered float reductions differ bit-for-bit because IEEE addition is
non-associative (RESEARCHED). But **integer/fixed-point is associative and exactly
specified** — same result on every CPU, every wasm engine, regardless of `target-cpu`.

Strategy (evidence-backed by the kernel's own convention):

- **Money/consensus/FSM-signature paths → integer fixed-point, always.** Already true for
  `money`/`cart`. Next: eliminate the last `tax_rate: f64` by taking an integer
  basis-points/micro-rate from config (removes the one float adjacent to money). `eqc`'s
  fixed printer generates exactly these.
- **Advisory/dynamics paths (`geo`, `spectral`, `markov`, `absorbing`) → f64 is fine** —
  they never feed a charged amount or a gated verdict. Keep them float; document the
  cross-target tolerance (already `1e-6`/`1e-9` in tests).
- **One watch-item (MEASURED risk):** `order_machine::FSM_GOLDEN_SIGNATURE` must compare
  spectral values with an **epsilon**, never bitwise float equality — otherwise enabling
  `target-cpu=native` fleet-wide could flip it. Confirm before §5's target-cpu step.

Fixed-point makes the kernel "crystalline" exactly where crystallinity is load-bearing,
and leaves float where only smoothness matters. That split is the resolution of the
determinism↔speed tension (§4.5), not a compromise.

### Pillar C — Zero-copy bridges: shared memory, not RPC

Today every kernel↔host call is a String round-trip (`wasm.rs`, MEASURED). The math-first
endgame for the bridge (your point 2): the host writes equation **inputs directly into a
shared linear-memory region**; the kernel (already-compiled instructions) reads them, runs
the equation, writes outputs to another region; latency ≈ memory-bus, no serialise step.

Concretely, in the WASM reality we ship on:
- WASM already exposes its linear memory as a JS `ArrayBuffer`. Zero-copy = pass **offsets
  into wasm memory**, not JSON strings; read results back by view, not by `JSON.parse`.
- `engine/src/bridge.rs`'s `VertexBridge` is the pattern already designed (a pre-allocated
  `&[f32]` staging buffer, one write per frame, zero JSON in the loop) — **wire it** as the
  kernel's numeric boundary, replacing the CSV-string "flat bridge" in `wasm.rs`.
- The batch shape matters more than the per-call shape: the win is passing *many* points
  (a courier fleet, a matrix) across the boundary once, not one scalar per call.

This is the WASM-native approximation of the "memory-mapped shared interface" — true
`SharedArrayBuffer`/mmap is available on the native (non-wasm) embedding for host processes.

### Pillar D — Formal-verification ladder (honest about the ceiling)

The blueprint's verification is a **ladder**, and the survey pins exactly which rungs are
reachable now (MEASURED — all proof assistants absent):

1. **Analytic ground truth** — diagonal / cycle Cₙ (roots of unity) / path-Laplacian /
   nilpotent / stochastic (λ=1) fixtures. *Available now.*
2. **Legacy-oracle parity** — freeze the legacy corpus as shared JSON; kernel must match
   until legacy is deleted (the R1 markov pattern). *Available now.*
3. **eqc differential/parity tests** — generated f64-vs-fixed-vs-SymPy equivalence within
   ε, emitted beside each organ. *Available now (Pillar A).*
4. **Property/invariant** — `Σλ=tr`, `Πλ=(−1)ⁿc₀`, Gershgorin, `B` rows sum to 1,
   `(I−Q)N=I`. *Available now.*
5. **Machine-checked contracts** — `cargo-kani` (bounded model checking) / `z3` SMT /
   `creusot`. **GATED: requires provisioning** (`apt-get z3`, `cargo install kani` — one
   step each, out of scope until the operator green-lights the install). Until then, do not
   claim machine-checked proof; claim tested-to-ε.

The direct-LaTeX→Rust mapping you asked about (point B) is exactly what `eqc` gives: the
Rust *is* the equation, so "logical mismatch" bugs (formula right, code wrong) are removed
by construction — the residual risk (does the equation model the world?) is what the
oracle/property tests cover.

## 4. The performance descent: CPU → SIMD → (FPGA / ASIC)

### 4.1 The asm reality (your `cargo-show-asm` question, MEASURED)

`cargo-asm` isn't installed, and fat-LTO dead-strips the math fns (no ABI export) — so they
can't be `objdump`'d out of the release artifact as built (a finding in itself: there is no
ABI-stable export boundary for the kernel's math today). A standalone probe compiled at the
kernel's release profile gives the concrete numbers:

- **`haversine` = 45 x86_64 instructions + 5 libm calls** (`sin`,`cos`,`asin`). The
  *calls* dominate, not the surrounding arithmetic; LLVM already strength-reduced
  `sin(x).powi(2)` → a single `mulsd`.
- **A 2×2 stack matmul = 36 instructions, ZERO calls, fully unrolled.** This is the
  "beautiful math → a handful of instructions" ideal in the flesh — and the lesson: it is
  clean precisely because it is *stack-only, transcendental-free, and statically bounded.*

The takeaway steers the rewrite: the wins are (a) remove the libm calls / heap allocs, then
(b) SIMD — in that order.

### 4.2 Stage 1 — SIMD (the real win)

| Organ | Change | Expected |
|---|---|---|
| `absorbing`/`spectral` matmul | **flatten `Vec<Vec<f64>>` → contiguous `Vec<f64>`** first (kills N heap allocs + pointer-chasing), then AVX2 `_mm256_fmadd_pd` | flattening alone is the bigger win at n≤10; SIMD 3–4× on top (RESEARCHED/ESTIMATED) |
| `geo` batched haversine | vectorize the arithmetic; trig needs a SIMD math lib (`sleef`/polynomial) — `core::arch` gives no vector `sin` | real only when **batched** over a fleet (which the tracking use-case has) |
| integer/fixed-point organs | `i64x4`/`i128` SIMD (`_mm256_add_epi64`, WASM `i64x2`) | **fast AND bitwise-deterministic** — no tension |

Stable-Rust caveat (MEASURED): `std::simd` is still **nightly-only** on `rustc 1.96.1`; the
stable path is raw `core::arch` behind `#[target_feature]` + `is_x86_feature_detected!`
runtime dispatch, or the `wide` crate (weigh against the kernel's zero-dep invariant).
**WASM SIMD128** (`+simd128`, f64x2/i64x2) is the portable, cross-fleet-deterministic middle
ground — one fixed width on every runtime, unlike `target-cpu=native`.

### 4.3 Stages 2–3 — FPGA / ASIC: the honest verdict (RESEARCHED)

**Not justified for this workload.** Every organ is either I/O-bound (event log, intake,
WS/HTTP) or operates on matrices small enough (n≤10 lifecycle states, a handful of couriers)
that the whole computation finishes in ns–µs — dwarfed by network/DB latency by **5–6 orders
of magnitude**. No tool compiles this kernel's Rust (with `Vec`, closures, `f64`) to HDL;
RustHDL/Veryl are separate hardware DSLs, not a descent from existing source. FPGA's win only
appears when saturating memory bandwidth or doing millions of matmuls/sec — not this shape.
The *only* organ that could ever reach FPGA territory (a huge high-frequency geo-fence sweep)
is solved by CPU-SIMD + batching long before hardware synthesis pays for its engineering
cost. **We name the ceiling honestly: the real speed lives at Stage 1, and pretending
otherwise is hype.** (ASIC is the same argument, one order harder.)

### 4.5 The determinism ⟷ speed tension, resolved

`-C target-cpu=native` + float-SIMD changes instruction selection and reduction order →
machine-specific last-bit float results → disqualifying for money/consensus. Resolution
(already the kernel's implicit design): **integer fixed-point + integer-SIMD is bit-identical
regardless of order or CPU** → full speed *and* determinism on the paths that need it; enable
`target-cpu=native`/float-SIMD **only** on the self-labeled "dynamics, never money" organs.

## 5. Near-term Rust levers (your point: no_std, no-heap, inline, target-cpu)

Concrete, low-risk, MEASURED-gap-closing steps for dowiz-kernel (bebop2-core already does
most of this):

1. **`#![no_std]` + `alloc` gating**, mirroring bebop2-core's `#[cfg(feature="host")]`
   split — the f64 analytic organs behind a feature, a pure core that compiles to wasm with
   an empty import section. (Biggest structural change; do it per-organ.)
2. **Kill the heap churn**: flatten `Vec<Vec<f64>>`→`Vec<f64>` in `absorbing`/`spectral`/
   `markov`; for n≤10, stack arrays (`[[f64; N]; N]`) or `SmallVec`-style avoid the alloc
   entirely. (This is *also* the SIMD prerequisite — one change, two wins.)
3. **`#[inline(always)]` on the leaf math** (`Complex::{add,mul}`, matmul inner, haversine)
   — `eqc` already emits it. Measure with the asm probe before/after.
4. **`.cargo/config.toml` with per-profile `target-cpu`** — `native` for the local/staging
   host build of the dynamics organs; **baseline** for anything whose output is pinned
   (after confirming §3B's golden-signature epsilon). Never `native` on the wasm artifact.
5. **`panic = "abort"` + `strip`** on release (smaller, faster, no unwind tables).

## 6. How we feed the kernel equations (your direct question)

**Hardcode-generated, not JIT.** Recommendation, with reasons:

- The equation is authored as a SymPy expression (or an `organ.eq.py` spec); `eqc` generates
  Rust **at authoring time**; the generated `.rs` is **committed** and compiled by the normal
  `cargo`/`wasm-bindgen` pipeline into CPU instructions (or wasm). This is "hardcode" in the
  sense that ships — but the *source of truth is the equation*, and the Rust is a build
  artifact you can diff and inspect.
- **Not runtime JIT / dynamic WASM-load of equations.** JIT reintroduces exactly what
  math-first removes: a runtime translator with its own state, entropy, and attack surface
  (`bebop2/ARCHITECTURE.md`'s directive), plus non-determinism and a fat dependency. The
  determinism floor (Pillar B) and the "no runtime proxy" rule both forbid it.
- The one place dynamic loading is *already* correct is the **WASM-component boundary for
  untrusted ports** (`cargo-component`, wasip2, zero-ambient-authority) — but that's for
  *capability-isolated I/O adapters*, not for the kernel's own equations. Equations are
  trusted, proven, and baked; ports are sandboxed and loaded. Keep the two separate.

So: **author as equation → generate → commit → compile to instructions.** The kernel's math
becomes, as you put it, a logic circuit — and the generation step guarantees the circuit
matches the equation.

## 7. The full-rewrite plan (phased, forward-only)

These are **substrate (engine) phases**, labelled `S0…S7`, orthogonal to the *product*
phases `P1…P10` in `MASTER-ROADMAP-10-PHASES-2026-07-14.md`. They obey that roadmap's
governing INVARIANT — **build DOWN from the first real order, not UP from the protocol** —
so the rewrite advances the engine *only as far as a real flow needs it*, never as a
front-run of the product. Each phase is independently shippable, proven red→green, never
weakens a red-line, and preserves all seven §1.5 invariants.

| Substrate phase | Serves master phase | §1.5 invariant advanced |
|---|---|---|
| S0 Foundation (eqc) | P2 · P9 | manifest-in-code, non-AI |
| S0.5 Atomic test | P9 | benchmark-as-speedometer |
| S1 Consolidate numeric core | P2 | spectral-waves, benchmark |
| S2 Crystallise money 🔴 | P1 | (skeleton) determinism |
| S3 DOD + no_std + SIMD | P2 · P8 | Data-Oriented-Design |
| S4 Zero-copy bridge | P3 · P4 | bridges-&-adapters |
| S5 Verification lift | P1 · P9 | (all — proof) |
| S6 Equation IR | P9 | manifest-in-code |
| S7 Mesh-shaped substrate | P6 (PARKED) | decentralized · mesh · local-first |

- **S0 — Foundation [DONE this turn].** `eqc` built + proven (Pillar A); this blueprint;
  the 4-lane ground-truth inventory.
- **S0.5 — The atomic test (the "garage" proof-of-concept).** Before scaling: take the ONE
  most-critical, most-intensive math block (§8.5 argues this is the **spectral eigensolve**),
  write a `criterion` micro-benchmark comparing the current impl to a naïve reference, and
  record the ns. This is the first datum on the speedometer and the template for the
  benchmark-as-CI-gate (§1.5.7). Small, concrete, falsifiable.
- **S1 — Consolidate the numeric core (the spectral organ).** One generated eigensolver/
  matmul organ replaces the **4 drifting Jacobi copies + FL+DK + the 3 exp-shims** — the
  single highest-leverage move (it removes the live silent-drift hazard *and* is the
  spectral-wave heart, §1.5.5). Parity-gated in CI against every current copy until the
  copies are deleted (doctrine rung 2). Benchmark-gated (S0.5).
- **S2 — Crystallise money 🔴.** Remove the last `tax_rate: f64`; generate `money` organs
  (float+fixed) with `eqc`; emit the parity test beside each. Red-line: integer-only,
  operator-gated review.
- **S3 — DOD + no_std + heap-kill + SIMD (§5, §1.5.3).** Per-organ `#![no_std]` gating;
  flatten `Vec<Vec<f64>>`→contiguous/stack (the DOD layout = the alloc-free path = the SIMD
  prerequisite, one change); `#[inline]`; `.cargo/config` target-cpu; integer-SIMD on
  fixed-point, float-SIMD on dynamics only. Measure each with the asm probe + the benchmark.
- **S4 — Zero-copy bridge (Pillar C, §1.5.4).** Wire `engine/src/bridge.rs` as the numeric
  adapter; replace the CSV/JSON string boundary with wasm-memory offsets; keep JSON only for
  structured domain objects. The engine never learns about the transport — only the adapter
  changes.
- **S5 — Verification lift (Pillar D).** Provision z3 + cargo-kani (operator-gated install);
  machine-checked contracts on the money + FSM organs; eqc/oracle rungs for the rest.
- **S6 — Equation IR.** Every kernel math organ has an `organ.eq.py` source of truth; `eqc`
  regenerates all + their parity tests in one pass; a CI gate fails if a committed `.rs`
  diverges from re-generation — **drift becomes impossible, not merely detected.**
- **S7 — Mesh-shaped substrate (§1.5.1).** Keep the kernel's math authority local-first and
  content-addressed so the mesh *protocol* (master-P6) can carry it when a real dowiz flow
  exists — never before. No coordinator is introduced; sync stays a CRDT lattice-join.
  PARKED behind the product, by the same invariant that parks master-P6.

## 8. What this foundation underpins

- **bebop autonomous self-evolution** (`bebop-autonomous-self-evolution.md`) — its resonator
  loop, verify-self-mod floor, and reversibility invariant all sit on top of proven kernel
  math; the driver's convergence governor *is* the spectral `|λ|`-vs-1 readout. The math-first
  foundation is what makes the self-evolution deterministic and auditable.
- **The psyonic-spectral organ roadmap** (R2–R8) — every ASCEND organ becomes an `eqc`
  spec (generate + parity-test), so the roadmap's "collapse legacy → one equation" is
  mechanised, not hand-done.

## 8.5 The foundational equation — where the engine starts

*Your question: from which equation does the engine begin — the point where computation
gets intensive enough to justify a custom architecture?*

**The engine's `F = ma` is the spectral operator on the connection graph.** Concretely:
the graph Laplacian `L = D − A`, its heat/wave propagator
`u(t) = Σ_k e^{−λ_k t} ⟨u₀, φ_k⟩ φ_k`, and the single readout `|λ|` vs the unit circle
(`< 1` damped · `= 1` resonant · `> 1` unstable), with the spectral gap `γ = 1 − |λ₂|` as
the master dial (mixing-time `τ ≈ 1/γ`, iteration-budget `k ≈ ln(1/tol)/ln(1/|λ₂|)`).

Three reasons this, and not something else, is the heart:

1. **It recurs at every layer** — loop-health & self-drift (`spectral`, `markov`), the UI
   field (`field` heat-kernel), interference (`coherence`), brittleness (`wavefield` λ₂),
   and mesh mixing-time (when a contact graph exists). One operator, applied everywhere —
   the definition of a foundational equation (§1.5.5).
2. **It is the computational hot-spot** — the eigensolve is the O(n³) intensive kernel,
   the exact place the survey found the pain (the **4× duplicated + drifted Jacobi**, plus
   FL+DK, re-run from scratch on every query). "Where computation becomes intensive enough
   to build custom architecture" is *literally the eigensolve.*
3. **Its output is a control signal, not just a number** — the `|λ|`-vs-1 verdict drives
   freeze/rollback, iteration budgets, and drift gates. The engine doesn't just compute;
   it reads its own spectrum and decides.

So the **atomic test (S0.5)** is: benchmark the one consolidated spectral eigensolve organ
against the four naïve copies it replaces, in `criterion`, and record the ns won. That is
the "garage" proof-of-concept — one critical function, one benchmark — and it doubles as
the first S1 parity-and-speed gate.

(The two **skeleton** equations — the integer money law `total = Σ pᵢqᵢ`, `tax =
⌊sub·rate / SCALE⌋`, and the FSM adjacency Law `decide` — are foundational too, but they
are the rigid *skeleton*, not the *engine/mind*. They get crystallised, S2, and pinned by
a "Manifesto of Constants" — a single `const` module documenting every scale factor,
threshold, and physical constant with its justification, §1.5.6.)

## 9. Red-lines (never yield, even under full autonomy)

1. The mind never overrides the skeleton: **no** float on money, **no** touching
   auth/RLS/migrations/PQ-crypto without the operator. 🔴
2. Math is the authority: an equation enters the kernel only PROVEN (analytic / oracle /
   property / eqc-parity RED→GREEN), never prose.
3. Codegen is build-time and committed — **never** a runtime transpiler/JIT/proxy.
4. Governance self-mod stays human-`!`-activated: the agent designs, proves, and generates;
   the operator lands anything that mutates the harness or crosses the credential wall.
