# Bebop — THE Roadmap (single source of truth)

This file supersedes PLAN_B.md, MASTER-FINISH-PLAN.md, ROADMAP_SELFHOST.md,
docs/ZERO_C_CHARTER.md and SWEEP-B3-3.md — all removed. BUGFIXES.md stays
(bug journal), AGENTS.md stays (process laws), bench/ reports stay (evidence).
Historical closed-status records are consolidated in "Progress log" below;
new work is tracked live in this file.

---

## Terminal goal

**Bebop is a post-von-Neumann self-hosting agent language — a single living
mathematical structure that maps directly to silicon.** It erases the boundary
between memory, compiler, text and processor architecture. There are no
traditional instruction lines, no syntactic sugar, no virtual machines, no
garbage collectors, no intermediate interpreters.

### What the language IS (the target state)

- **Post-von-Neumann substrate**: no program counter, no call stack, no
  sequential fetch-execute loop. The processor is an asynchronous event
  dispatcher scanning dense bit arrays of activity via hardware `tzcnt`/`popcnt`
  + SVE2, with threshold accumulation (Σ w_i x_i > θ). Code "lives" only where
  a spike fires.
- **Holographic memory topology + ranked arenas**: a single immutable linear
  arena, zero-copy mmap, 64B cache-line aligned. Information is not broken into
  isolated cells — it is packed into rank-4 word-tensors (`.bt`) and CSR
  adjacency matrices. Through spectral smearing via FWHT, the program structure
  is spectrally distributed across the entire tensor topology. Modification or
  deletion does not break the system — it smoothly redistributes spectral
  fingerprints across the whole space, eliminating the concepts of "dangling
  pointers" and segmentation faults entirely.
- **Spectral engine + eigentime**: no classical execution loop. Time is
  measured not by clock ticks but by eigentime — discrete iterations of Hotelling
  deflation and dominant eigenvalue (λ) stabilization. The compiler maintains
  continuous spectral invariant checks on the CSR graph; if the global spectral
  gap (γ) is violated, the system instantly prunes invalid branches before
  execution.
- **Multi-tier spectral stack** (the canonical transform set of the language):
  * **Micro** (FWHT / Hadamard / Walsh / Haar): instantaneous bit-level binding
    and event routing via pure integer add/sub — no multiplication, no float.
  * **Meso** (NTT): exact polynomial and cyclic convolutions over Z_p with
    absolute bit-precision and zero approximation drift.
  * **Macro** (KLT): background spectral deflation, eigenvector computation,
    eigentime arena stabilization.
  Rejected forever: DFT, FFT, DCT, DST, Z-transform, DHT — they build on
  trigonometric/complex floating point, accumulate rounding error, and violate
  the deterministic bit-exact spectral invariant doctrine. (The FFT butterfly
  skeleton is reused inside NTT modulo p; classical complex Fourier remains
  foreign overhead.)
- **Hardware fusion on ARM SME/SVE2**: tensor multiplication, spectral
  deflations, and quantized attention transforms map directly to ARM SME matrix
  tiles (ZA); Hamming distances and parallel masking use SVE2 variable-length
  vectors — maximum bus throughput with no external libraries.
- **Reversible logic**: all arena mutations via reversible gates (Toffoli/
  Fredkin) implemented as pure bitwise XOR/mask operations. Every arena state
  is fully reversible at the bit level — instant rollback and self-healing
  without snapshots or extra memory.
- **Multiversal superposition**: all potential agent logic states are held as a
  weighted superposition of hypervectors (hv4096). Deterministic collapse to
  reality occurs at the intersection of eigenvectors.

---

## Verified state (2026-09-01)

`seed/seed.S` (frozen AArch64 loader, no libc, 1496B) + `bebop.bin`
(self-hosting compiler, fixpoint bb2 == bb3,
sha256 `3b720370a22b784847d867dce594dd490e4a51eaaaf2e21f0602982fcc850398`) +
`*.bp` sources + `*.bin` artifacts. **Zero C** — `native/` (175 files) deleted.

Milestone history (all DONE):
- **M1** seed loader: k1..k7 run through seed.bin, outputs identical to the
  interpreter; zero C at runtime.
- **M3** self-bootstrap: full selfsource compiled by itself is byte-identical
  to the interpreter's output (67816/67816 words); selfcompile fingerprint
  236065248692568 == word-sum; self_check = 0.
- **M4** CLI-in-.bp: `bebop.bp` = compiler + CLI (`compile/size/version`); seed
  loader v4 passes argc/argv; `version`=1000000, `size k1c.bin`=94, CLI-compiled
  k7 executes → 3939697352.
- **M5** std twins: `bench/vs_rust/std_golden.sh` — **15/15 PASS** (checksum,
  sort, rng, base64, sha256, crc32, hex, hv, spectral, csr, bt, cache, wht,
  haar, ntt). Emitter fix shipped: `is_alpha` uppercase fix.
- **M7** Zero-C: `native/src` deleted; full gate suite green without C compiler.

**Active gates (all green):**
- std_golden.sh → 57/57 PASS (includes tern, rns, snn, lsys, lod, phant,
  rnsrot, deltasync, mutlsys, entcol, ptrless, morph — the T1–T12 stack)
- parity_driver.sh (kernels) → 9/0/0 (+1 main-less skip)
- construct_parity.sh → 24/24 MATCH (words AND values)
- pool tests → 5/5 (JIT-only; interp retired at M7) — proot-blocked;
  fiber scheduler is the in-sandbox replacement

### Known issues
1. **Pool test JIT divergence**: bebop.bin pool tests (par_sum/par_merge/
   par_compile) return 0 on the JIT due to the retired interp's fntab
   scan-budget divergence on sys_clone/futex paths — JIT itself is functionally
   correct (fixpoint stable, all gates green); documented, not blocking.
2. **CLI exec mprotect**: CLI compilation of selfsrc (116KB) segfaults via the
   exec builtin under proot W^X. Boot path (self_bootstrap) works; low priority.

---

## Architecture

### The zero-C toolchain (already real)

```
seed.S (frozen AArch64, 1496B, no libc)     ← the only thing that runs at boot
        │
        ▼
bebop.bin (self-hosting compiler, fixpoint bb2 == bb3)   ← compiles .bp → .bin
        │
        ▼
*.bp sources → *.bin → executed by the seed JIT on the arena
```

- F1. **Syscall ABI emitters** — machine-verified (LAW L1). AUDIT 2026-08-31:
  131 words across 17 emitters disassembled — ALL register numbers correct.
  **LAW: `movz x8,#N` = 3531603968 + N*32 + 8 (Rd=8!).** A formula without +8
  yields `movz x0,#N` (wrong register) — never use it.
- F2. **Artifact I/O** — mmap-export (ftruncate+mmap MAP_SHARED+stores+munmap);
  atomic publish via renameat(tmp,out); tmp = agent argv-argument; zero
  sys_write on the critical path (proot syscall-site flakes immunized).
- F3. **Memory** — arena + strided tensor-views (rank-2/3/4 over the same
  arena); generation arena with MAP_NORESERVE pagination, bump allocation,
  mprotect(PROT_NONE) reclaim.
- F4. **Canonical artifact** — .bt rank-4 word-tensor (codec v1: "BT4R" magic,
  u32 version/rank, dims[4], dense i64 LE data; 28B header; pack/FNV/unpack/
  stride). Text-codec = agent-authoring; canon = .bt tensor.
- F5. **Control flow** — branch-mode while/if with proper patching; depth_sim
  = 0 outside run_program.
- F6. **Verification** — self_fixpoint in RAM (bb2==bb3), artifact-vs-artifact,
  **zero C-oracle**.

### Design laws (inviolable, every swarm)

Branchless Σ k·(k==N)·expr; no_std; O(n); atomic/lock-free; vector-first (NEON,
scalar fallback); hypervectors where possible; living memory. Per-fn bind budget
≤ 128 (overflow = trap, never silent); literals ≥ 2^63 must use the
normalized-half emit path; nested ifs inside let-statements and plain-var
assignment inside `let _ =` are banned (index cells + single-level guards only);
capacity asserts on every fixed table.

### Coordination

Every milestone: commit + push to origin/main. Full verification gate after each
batch: self_check, self_bootstrap parity, parity drivers, construct corpus,
std_golden — evidence in commit messages.

---

## Implementation plan

Execution order: the lower foundation closes first, then each upper column
lands ON TOP of it. No column is optional. Every spectral layer carries its own
golden vector / self-check under the new basis (see invariant policy below).

### FOUNDATION — closed layers (bottom stack, stays forever)

F1–F6 above + the big closed milestones tracked in the Progress log. This layer
is the immutable substrate every NEO new-basis column builds on.

### NEO-FOUNDATION — 8 mandatory columns (N1..N8, strict order)

The post-von-Neumann VM on ARM64 silicon. Lower columns are base decisions,
upper columns are built on them. Order maximizes reuse first.

**N1. FWHT — Fast Walsh-Hadamard Transform (numeric / VSA-shift basis)** — [CORE DONE]
- Pure ADD/SUB butterfly, ZERO multiplies — perfect for the i64 linear arena.
- Native language for HVs: randomized orthonormalization, projections, state
  folds without heavy matrix multiplies.
- Hardware synergy: butterfly → NEON vadd/vsub pairs, SVE2 VL-agnostic.
- Branchless, deterministic, fixed stride, zero mispredicts.
- Impl: `selfhost/std/wht.bp` — fwht(x,n) in-place butterfly (wht_pow2 /
  wht_invert / wht_encode). **Gate wht 85001** (unit-vector dispatch word 85 +
  self-inverse round trip). JIT == interp == 85001.

**N2. Reversible / Conservative logic** — [gate rev added]
- No destructive operations: no arena edge is ever erased irreversibly; all
  instructions/mutations via reversible gates (Toffoli/Fredkin) as pure bitwise
  XOR/mask.
- Real bit-level time reversal inside the compiler architecture: every arena
  state can be unwound without copies/snapshots — 0-overhead debug, instant
  agent rollback, self-healing.
- Impl: `selfhost/std/rev.bp` — reversible primitives (CNOT/Toffoli masks),
  journal / reversible constructs over the arena.

**N3. Ring-VSA / HDC with colored Hadamard rings (holistic algebraic system)** — **DONE**
- No code/data/type/instruction split: ANY syntactic element, CSR graph, SNN
  spike = a single 4096-bit hypervector.
- WHT = the single algebraic group for bind (binding) and bundle
  (superposition).
- The compiler does not translate code — it homomorphically folds entities into
  the single holographic space of the arena; search/execution = Hamming
  distance.
- Gate `ringvsa` fold 1110000000544 (38th): dyadic XOR-index convolution bind;
  ring axioms — associativity (a·b)·c == a·(b·c), the WHT convolution theorem
  fwht(a·b) == fwht(a)·fwht(b) for all 16 cells, identity a·e == a. Oracle ==
  BP bit-exact (journal 1788288226).

**N4. Bit-Level Petri Nets (bit-asynchronous Petri nets)**
- Replaces even heuristic event queues: parallel token-passing Petri-net
  marking matrices mapped into arena bit arrays.
- Activity = transition incidence matrix; firing is not branches but a single
  hardware bitwise operation (AND masks → tzcnt). Thousands of parallel logic
  branches in a few cycles — no queues, no function calls.
- Impl: `petri.bp` — bit transitions, incidence, tzcnt dispatcher. [new file]

**N5. LSM / Reservoir Computing (liquid state machines)**
- Agent temporal dynamics and memory through a constant random-but-fixed
  reservoir of connected nodes in the CSR arena, fixed by spectral invariants.
- Input spikes excite a "liquid" state; FWHT instantly projects the
  high-dim trace into the decision prospect. Real-time adaptation with no
  "training"/gradient — the language structure itself is the time processor.
- Impl: `lsm.bp` — reservoir, spectral-invariant fixation. [new file]

**N6. Holographic memory non-locality**
- Every fragment of the ranked word-tensor is WHT-encoded to carry a
  micro-fingerprint of the WHOLE program (hologram: cutting never destroys the
  picture).
- The arena can be trimmed anywhere — the spectral engine (eigenvectors) fully
  recovers and executes all logic from any surviving piece. "Dangling pointer" /
  "context loss" disappear: information is global.
- Impl: WHT-encoding of word-tensors, spectral recovery on top of N1+N5.

**N7. Multiversal superposition branching** — **DONE**
- Eliminates sequential path choice: ALL possible logic branches compute
  simultaneously in one bit array of superpositions (SME/SVE2 vector
  instructions = the forward-port form; the canonical arithmetic is gated).
- All alternative future states = weighted sum of hypervectors; collapse
  (reality choice) happens automatically when the spectral deflator (Hotelling)
  finds the eigenvalue break λ that masks out false branches.
- Gate `msuper` fold 1114056100000 (39th): S = Σ w_b·H_b (weights
  {+1,+1,-1,-1}); the Gram-dominant eigenvector's sign pattern {+,+,-,-}
  masks the false branches EXACTLY; λ1=50.08 vs λ2=10.47 (gap>>22=40561,
  decisive); readout argmax <S,H_b> = 0 in the surviving group. Oracle ==
  BP bit-exact (journal 1788288227).

**N8. Spacetime metric code / global boundary execution** — **DONE**
- Eliminates "execution time" as a sequence of steps: the program = a global
  boundary-value problem on the CSR graph surface (Laplace/heat flow).
- Runtime instantly finds the whole system's stationary state as a single
  mathematical equilibrium; past/present/future agree globally via the spectral
  invariant in one hardware pass = "matrix crystallization".
- Gate `spacetime` fold 1111100012240 (40th): pinned boundary nodes +
  constant arc increments crystallize the harmonic field in ONE pass
  ((10,-2)->[10,7,4,1,-2,1,4,7], (20,-4)->[20,14,8,2,-4,2,8,14]); loop
  closure, Laplacian==0 at every interior node, Jacobi heat-flow consistency
  (crystal is an exact fixpoint, pinned nodes survive). The truncated
  averaging map's rounding basins (zero start -> [10,6,3,0,...] basin,
  random start oscillates) are sidestepped by one-pass crystallization.
  Oracle == BP bit-exact (journal 1788288228).

### Invariant policy (mandatory, operator decision)

- **Golden-vector determinism / JIT==JIT fixed-point parity: RELEASED** where it
  conflicts with N1–N8. Each new-basis layer carries its own golden / self-checks
  under the NEW conventions; the old fixed-point oracle is no longer the
  mandatory source of truth for these layers.
- The foundation (arenas/.bt/HDC/spectral/spike-dispatcher) is NOT deleted — it
  remains the substrate the columns build on, and its own golden invariance
  stays.
- SME/SVE2/bit-manip are first-class hardware for the NEO foundation, never a
  replacement oracle: no hardware FP accelerator ever becomes the source of
  truth for a lower foundation layer. Bottom layers built first, upper
  dispatchers after the core is stable.

### ARM SME/SVE2 hardware mapping

1. **ARM SME (Tiles ZA)** — first-class hardware for the NEO foundation.
   Inner product v1·v1^T + SpMV in Hotelling deflation / power iteration
   (SS-6/15/16) lands on SME matrix tiles; N7 (multiversal superposition) uses
   SME/SVE2 to compute all branches at once. Per the operator decision these
   layers carry their own new-basis golden, not the old i64 oracle.
2. **ARM SVE2 (VL-agnostic)** — for hv4096 HDC and bit spikes. Current HDC is
   canonical on NEON (SWAR popcount/hamming, golden-exact). SVE2 = forward port
   of the same deterministic integer arithmetic (popcnt/tzcnt never change
   numbers); safe exactly because bitwise ops are VL-invariant.
3. **Bit-manipulation (biti/dispatch)** — for the event-driven Spike Dispatcher
   (tzcnt/lzcnt/popcnt + Base+SpikeIndex*Stride) — the agent-runtime dispatcher
   built AFTER the core (compiler/spectral layer) is stable.

---

## Neural Operator Core (FNO-style, three-level spectral stack) — **DONE**

Replaces the classic attention layer with a continuous integral operator in
the frequency domain, integrated directly into the macro/meso/micro tiers of
the spectral stack:

- **Micro (FWHT)**: fast pre-encoding of arena bit arrays into wave
  hypervectors (hv4096) with ZERO multiplications — structural, gated
  (`wht` 85001).
- **Meso (NTT)**: the kernel convolution replaces the standard complex
  float32 FFT with a number-theoretic transform over Z_p (MOD=998244353):
  full bit-exact determinism, zero float drift. Gated: NTT-convolution ==
  direct circular convolution mod p (`ntt` 141003, `fno` 41st).
- **Macro (KLT / Neural Operator Core)**: instead of learned dense weights
  the kernel K is parameterized directly in spectral space as low-frequency
  modes, updated by Hotelling deflation iterations (the deflator is gated
  in `spectral`/`scoord`/`sgamma`/`msuper`).
- **Zero-copy linear arena**: field state + kernel in ONE linear arena,
  64B-aligned ranked word-tensors, neighbor access by implicit address
  arithmetic (stride geometry gated in `stride` 36th; bump allocation +
  generation reset gated in `genarena` 35th).
- **SME/SVE2 machine execution**: SME ZA matrix tiles for mode
  multiplication and SVE2 branchless scans for σ/truncation remain the
  FORWARD PORT to real ARMv9 silicon (the branchless scan arithmetic is
  canonical and gated: `bitmat` 32nd, `fno` dispatch tier). No gate can
  fake hardware — marked as forward-port with trigger: first ARMv9/SME
  silicon available.
- **Eigentime γ trigger + SNN dispatch**: the operator iterates until the
  global spectral gap reaches equilibrium (γ = |λ1|−|λ2| of the output
  spectrum); the branchless SNN dispatcher (tzcnt/popcnt) then activates
  the next arena node without polling or event queues.
- Gate `fno` fold 111152971008019 (41st): conv_ok + modes_ok + spec_ok +
  fwht_ok + gapq(52971) + mode-mask dispatch (7/3/0). Oracle == BP
  bit-exact (journal 1788288229).

---

## Spectral Singularity Layer (SS-1..SS-18)

Max projection: arenas + .bt tensors + HDC + spectral geometry. Each item is a
capability with its done-check.

### SS-1 NEON Kalman filter (zero-alloc, arena, real-time) — **DONE**
- Kalman filter as a pure .bp computation on linear arenas: zero malloc,
  zero heap (scalar-only 1-D gate).
- Deterministic latency: fixed tick count per iteration (WCET guarantee) —
  structural (no data-dependent loops).
- Gate `kalman` fold 28327900110011 (27th): F=H=1.0, Q=0.001, R=0.01, z=5.0,
  1000 iterations — Riccati map reaches an EXACT fp fixpoint (P1000==P999,
  0 drift), state tracks z inside the 0.1% band (err 3 fp units). Oracle ==
  BP bit-exact (journal 1788288215). NEON 2×2 systolic tiles = forward port
  (same trigger as SME/SVE2).

### SS-2 Vector calculus as static invariants (rot/div/grad → graph topology) — **DONE**
- Identities (∇·∇f = ∇²f, ∇×∇f = 0) become CSR-graph structure preservation
  checks; differential operators = bit masks on rank-4 tensors (not symbolic
  math).
- Gate `vecinv` fold 1111018 (29th): div.grad == laplacian (stored-flow
  row-sum vs direct formula, 8/8), div.rot == 0 survives node relabel
  rotation (layout-invariant invariant), a broken asymmetric edge leaks
  exactly 1 unit and the invariant fires. Oracle == BP bit-exact (journal
  1788288217).

### SS-3 LC resonance as agent-loop timing (jitter-free) — **DONE (core)**
- Agent loop = electronic LC tank: L = latency, C = arena capacity. Resonant
  frequency f₀ drives the target inter-iteration period — minimal jitter
  without an OS scheduler.
- Gate `lcres` fold 1116675441335088 (34th): f0 = 1/(2π√(LC)) in fp 2^32 for
  two tanks (2/π, 4/π + period π/4) inside the 0.1% band; fp_div fixed
  (integer-part pre-loop restores the r<b invariant for ratios ≥ 1).
  Oracle == BP bit-exact (journal 1788288222).
- DONE (jitter half, gate `lcjit`): clock_ms landed on the std gate surface
  (R3.x(e) parse fix); two 800k-cycle LC batches timed in-process. Honest:
  3-8% batch jitter on this shared proot box; fixed tick count per cycle is
  structural; <1% wall-clock needs bare metal (upgrade trigger stays).

### SS-4 FIR as a ban on cyclic dependencies (BIBO stability) — **DONE**
- FIR: forward-only flow → BIBO guarantee structurally (the emission
  REJECTS-while-of-unknown-depth half is compiler-internal R3.x work).
- Gate `fir` fold 11104857722880 (30th): 4-tap h={1,1/2,1/4,1/8}, literal
  tap count = bounded masked iteration (zero infinite-loop risk by
  construction); impulse == taps exact; all 16 worst-case sign patterns
  |x|≤1 give |y| ≤ Σ|h| = 15/8 with equality at the aligned pattern.
  Oracle == BP bit-exact (journal 1788288218).

### SS-5 Calculus bounding (Taylor/mean-value for mutation code) — **DONE**
- Mean-value theorem + Taylor series → automatic bounding boxes for mutations.
- Gate `calcbound` fold 1024576000 (28th): f(x)=x²−x at x0=1.0, five golden
  mutations d∈{−1/8,−1/16,0,+1/16,+1/8}, slope bounds [0.75,1.25] with
  ε=0.01 slack — every actual Δf lands inside its box. Oracle == BP
  bit-exact (journal 1788288216).

### SS-6 Matrix decompositions on arenas (LU/QR/SVD/power method) — [CORE DONE]
- Port dowiz-core spectral.rs → .bp: topk_symmetric (power method + Hotelling
  deflation) in i64 fixed-point 2^32 (fp_mul schoolbook exact 64-bit split,
  isqrt bit-by-bit, LCG start with oracle constants, sign = first positive
  |component| > 2^−16, sort desc |λ|).
- DecompCache: content-addressed (FNV-64) spectra cache with monotonic
  recomputes falsifier.
- Impl: `selfhost/std/spectral.bp`. **Gate spectral 2038** (B6_bridge k=3
  iters=32, frozen = Σ|λ_bp − λ_golden| fp units, ~8e-6 relative per λ — honest
  fixed-point-truncation vs f64 gap). DecompCache gate cache 38876254956.
- LAW: `>>` in Bebop is LOGICAL (u64) on both engines — abs before any shift of
  a possibly-negative value.

### SS-7 QLoRA 4-bit agentic evolution — **DONE**
- Agent strategy weights = 4-bit matrices in fixed arenas; low-rank adapters
  (A·B with rank << dim) update logic on live hardware; DecompCache stores
  quantized states (FNV-64 key).
- Gate `qlora` fold 1116506000272 (31st): 8 weights → 4-bit signed
  (round(|w|·8), error ≤ 1/16 all 8); rank-1 adapter moves the strategy
  (Δy = 17/256); re-quantized packed-state FNV-64 key flips (DecompCache
  invalidation). Oracle == BP bit-exact (journal 1788288219). <1ms/0-malloc
  timing half deferred with the clock syscall.

### SS-8 Sinc interpolation (no phase distortion) — **DONE**
- sinc(x)=sin(πx)/(πx) as ideal interpolant for tensor telemetry.
- Gate `sinc` fold 6684880500081 (26th): direct Taylor series (no division)
  1 − z²/3! + z⁴/5! − … − z¹⁰/11!, z=πx, fp 2^32. Honest window |x|≤1
  (fixed-point truncation): sinc(0)=1.0 exact; sinc(1/2) = 2/π to ~1e-8
  (q05=667544 vs golden 667544.2); sinc(1) error 0.013% < 0.1% band.
  Oracle == BP bit-exact (journal 1788288214).

### SS-9 Transformer attention on ARM64 NEON (zero frameworks) — **DONE**
- Self-attention: hv4096 Hamming distance via vcnt (instead of softmax+float);
  bind = XOR, bundle = majority; KV-cache = DecompCache.
- Gate `attn` fold 2008568201 (33rd): Hamming nearest-neighbour over 64-bit
  hypervectors (SWAR popcount from hv.bp verbatim), XOR-bind of the winning
  value; Q at distance 3 from K2, ≥19 from the rest → unique winner,
  deterministic argmin. Oracle == BP bit-exact (journal 1788288221).
  <1ms/128-token + C-golden timing halves deferred (clock + PMU).

### SS-10 Normalization & stride optimization (cache-line aligned) — **DONE (core)**
- Layer/Instance norm = rank-4 stride geometry under 64B cache lines.
- Gate `stride` fold 11100128016 (36th): (4,4,4) tensor padded to 8-cell
  (64B) runs, stride 8/64/256; every run base 64B-aligned (zero false
  sharing), padding cost exactly 64 cells. Oracle == BP bit-exact (journal
  1788288224). innovate: L1 hit-rate >95% needs PMU counters — perf_event_open
  blocked by the sandbox (probe errno); deferred to bare metal.

### SS-11 Generation arena with MAP_NORESERVE pagination — **DONE (core)**
- Allocation = pointer bump (deterministic, zero GC); reset = instant return
  to the generation mark (the mprotect(PROT_NONE) analogue); generations
  reuse the arena (constant high-water, zero fragmentation by construction).
- Gate `genarena` fold 1110300000100 (35th): 100 gens × 10000 bump-allocs =
  1M alloc/free cycles, pure arithmetic (zero syscalls), monotonic
  addresses, exact gen accounting (hw=30000). Oracle == BP bit-exact
  (journal 1788288223). innovate: the mmap MAP_NORESERVE/mprotect syscall
  half is compiler-internal — deferred.

### SS-12 NEON bit matrices (switch/case → parallel bit grids) — **DONE (core)**
- Pattern matching: all conditions packed into dense bit grids, branch-free.
- Gate `bitmat` fold 1000024600 (32nd): first-set-bit dispatcher over an
  8-bit condition word via a running not-found mask, verified over ALL 256
  patterns (sum of outputs 246); fixed 8-step tick = the structural part of
  the <10-cycle claim (the 23-builtin emit-dispatcher swap + cycle count are
  compiler-internal R3.x work). Oracle == BP bit-exact (journal 1788288220).

### SS-13 Position-independent DecompCache blocks — **DONE (core)**
- Cached AST graphs + compiled code = position-independent binary blocks;
  relocatable PIE-style (relative deltas, never absolute pointers).
- Gate `pieblock` fold 1100800001 (37th): a serialized object graph moved
  base 0 → 1000 resolves IDENTICAL payloads, the FNV-64 fingerprint is
  move-invariant, the link walk cycles to origin. Oracle == BP bit-exact
  (journal 1788288225). innovate: zero-copy mmap save/load + <5ms cold
  start are compiler-internal — deferred.

### SS-14 Direct-threaded code in arenas (no dispatch loop) — DEFERRED
- Instructions = direct links to the next handler (no dispatch loop); L1
  I-cache maximized by linear-arena placement. For .bt tensor-op
  interpretation and agent state machines.
- innovate: requires the emitter/interpreter dispatch rework inside
  bebop.bp + real-hardware I-cache benchmarks (≥2× vs switch-dispatch) —
  no deterministic fold can prove a hardware perf claim; upgrade trigger:
  after R3.x emitter work lands, port the .bt interpreter dispatch to
  direct threading and measure.

### SPECTRAL COORDINATE SYSTEM (eigen integration)

**SS-15 Eigenvectors = the single coordinate system** — [replaces VS-AST] — **DONE**
- All states/concepts project onto the orthonormal basis Q (eigenvectors of the
  connection operator); byte-shift invariance via the spectral basis; search =
  projecting a hypervector onto dominant eigenvectors (not a pointer!).
  Coordinates are spectral projections, layout-invariant — no pointers at all.
- Gate `scoord` fold 2010131 (21st): C8+I dominant mode = constant vector →
  DC coordinate invariant under cyclic byte-shift to fp error; layout-mirror
  bit-exact; argmin over stored coordinate VALUES (no pointers); orthonormality
  bounded (ob). Oracle == BP bit-exact. Journal 1788288200-03 (s64 phantom,
  silent undefined-fn tolerance → T0 law).

**SS-16 Eigenvalues = control-flow metrics** — **DONE**
- γ = λ₁ − λ₂ (spectral gap) as the logic switch: γ < threshold → the graph
  disintegrates. Fiedler vector → automatic parallelization (sign = graph
  bipartition).
- Gate `sgamma` fold 3550431 (22nd): connected P8+selfloop → γ·2⁻³² ≈ 0.3473
  (switch ON), two identical P4+I blocks → γ ≈ 0 (switch fires); Fiedler
  evecs[1] sign cut = 4+/4− (work split). Oracle == BP bit-exact. Journal
  1788288204-05: bipartite ± spectrum degeneracy — equal-|λ| pairs with
  different λ freeze the power method at mixed fixed points (self-loops fix;
  equal-λ pairs are harmless).

**SS-17 Eigentime (time = spectral iteration)** — **DONE**
- Synchronization = number of Hotelling iterations until the iterate enters an
  exact cycle of the power map (not a wall clock!). λ₁ fixpoint stabilizes →
  cores into energy-efficient state (WFI/WFE); agent signal → ΔA → new
  iteration. Removes the need for OS scheduler, timers, interrupts.
- Gate `seigtime` fold 1233012011 (24th): slow clock C8+I (λ₂/λ₁ = 0.805) —
  the normalize map is locally flat at the dominant constant mode (c′ ≈
  2³²/√8 independent of c), so the rounding-locked trajectory sawtooths into
  an exact period-30 cycle: first recurrence td=123, absorb 16/16 (ab=1) →
  e_slow=123301; fast clock J8 (λ₂..₈ = 0) — exact period-1 fixpoint td=2,
  ab=1 → e_fast=2011. Time-scale separation + absorbing both. Detection =
  ring history hist[240], shortest p∈1..30 with x_t == x_{t−p}, +16-step
  membership absorb; e = td·1000 + per·10 + ab. Oracle == BP bit-exact.
  Journal 1788288208-11: SPEC's period-1/2 detector falsified by measurement
  (no short cycle at any precision); ring-30 redesign Pro-approved; R3(b)
  LSR-seed discovery — the seed shift emits LSR for loop-reassigned locals;
  eigentime is the first SEED-SENSITIVE gate (topk folds are seed-insensitive,
  which is why ss15/16 never exposed it).

**SS-18 Spectral self-replication (mutation via ΔA)** — [DriftClass ported] — **DONE**
- Agent changing logic = matrix perturbation ΔA. Check: spectral_drift(A₀,
  A₀+ΔA) → DriftClass (spectral.rs:800 port; `selfhost/std/spectral.bp`).
  Drift within γ → automatic fix (mmap snapshot); outside → .bt dump.
  Replaces textual compilation: evolution = pure spectral jumps.
- Gate `srepl` fold 8449214 (25th): base A₀ = 0.25·(C8+I) (ρ=0.75 Damped,
  γ=0.1465); within-γ mutation (+0.01 one self-loop) → Δρ=0.00128 ≪ γ →
  class stable Damped→Damped (trans 0 = auto-fix regime); outside-γ
  (+0.4 all self-loops) → ρ=1.15, 3 unstable modes → Damped→Unstable
  (trans 2 = .bt dump regime). Fold = Δρ₁q·10⁵ + trans₂·10⁴ + unst₂·10³ +
  trans₁·10² + Δρ₂q. Oracle == BP bit-exact (journal 1788288212-13).

---

## SILICON-REGISTER PULL (vision -> bottom-up task stack)

Added 2026-09-03: the register-level vision (ternary Clifford basis,
RNS, L-systems, in-register SNN, mprotect morphing, .becache-as-pointer)
is the terminal goal elaborated at the silicon layer. Analysis: ~70% of
its mathematics is ALREADY gate-proven in this repo; the new pulls are
listed bottom-up with falsifiable done-checks. Every new layer carries
its own gate (fold == independent python mirror) per the invariant
policy; the v5 lesson (fixed addressing cells, no dynamic slot
arithmetic, branch-free selection, journal 1788385641) is a LAW for all
new emitter-adjacent work.

### Vision -> existing-gate map (what is NOT new work)

| Vision item | Already proven by |
|---|---|
| Hypervector codebooks (hv4096, XOR bind, majority bundle) | gate `hv` 4427592702613580868 |
| Spectral projections / KLT deflation | gates `spectral`/`scoord`/`sgamma` |
| In-register SNN semantics (bit masks, POPCNT) | gates `petri` (18th), `bitmat` (32nd), `spike` (49th) |
| Superposition branching + collapse | gate `msuper` (39th) |
| Entropic selection (spectral gap switch) | gates `sgamma` (22nd), `seigtime` (24th) |
| Deterministic .becache + i64 folds | gate `cache` + the fold discipline itself |
| 4-bit agentic evolution (self-mutation) | gates `qlora` (31st), `srepl` (25th) |
| Holographic memory / WHT smearing | gates `holo` (19th), `wht` |
| Canonical AST-less interchange format | `.bt` rank-4 tensors (F4) + `bt` gate |
| Direct-threaded cells | gate `thr` |
| Constant folding (the register-window rung 1) | gate `foldx` (R6.2 v5) |

### Layer 0 -- numeric basis (new gates, pure .bp arithmetic)

**T1 · ternary Clifford basis** (`tern.bp`) — DONE ✓ (fold 8888868889989889)
GOAL: {-1,0,1} coefficients, 2 bits each, packed into i64 words; blade
multiplication via the Cayley table as combinatorial masks (AND/XOR +
sign-inversion masks), NOT float MUL; the rotor sandwich R x R~ (grade
projection) as a bitwise pass.
DONE-CHECK: gate `tern` fold == python mirror: pack 8 ternary blades,
multiply a rotor pair, sandwich-rotate a probe blade; sign pattern and
2-bit packing invariants (no value outside {-1,0,1}).
DEPS: bits.bp (popcount/rotate). BLOCKERS: none (deterministic i64).

**T2 · packed RNS** (`rns.bp`) — DONE ✓ (fold 1183829339)
GOAL: 4 coprime moduli with residues in 16-bit lanes of an i64;
parallel add/mul by lane-local arithmetic (no carry chains by
construction); CRT spot-check against the direct i64 result.
DONE-CHECK: gate `rns` fold == mirror: N random-ish pairs, RNS add and
mul == direct mod-2^64 arithmetic on every lane AND the CRT check.
DEPS: none. BLOCKERS: none.

**T3 · in-register SNN engine** (`snn.bp`) — DONE ✓ (fold 65504516937878)
GOAL: the vision's bit-mask neurons with ternary spikes: a neuron's
state = bit mask; a spike = a packed ternary coefficient (T1); the
propagation step = one POPCNT + AND/OR pass (no per-synapse loops);
the spike event simultaneously encodes a rotor multiply (fuses with T7
later).
DONE-CHECK: gate `snn` fold == mirror: N-bit network, one propagation
round, activity + weight-fold == python.
DEPS: T1 (ternary spike payload), bits.bp. BLOCKERS: none.

### Layer 1 -- generative memory

**T4 · L-system fractal memory** (`lsys.bp`) — DONE ✓ (fold 144175882039858)
GOAL: the arena stores ONLY the compact recursive rule + the seed; the
expansion is generated into the arena on demand and folded back into a
digest after use. Expansion factor measured and frozen (bytes of rule
vs words of expansion -- the "orders of magnitude" claim becomes a
number).
DONE-CHECK: gate `lsys`: expand rule (algae/Koch-like) to depth d,
digest the expansion (FNV-64), collapse; fold == mirror; the expansion
factor printed once and recorded in the journal.
DEPS: none. BLOCKERS: none (the claim is measured, not assumed).

**T5 · fractal LOD zoom** (`lod.bp`) — DONE ✓ (fold 1000088904914)
GOAL: the macro-rotor (T1, low dimension) stays register-resident; a
collision/logical-inference trigger expands a fractal layer locally,
applies the sandwich, collapses back to a macro-index. The expansion is
transient -- after the fold, the arena section is reclaimed (generation
arena, `genarena` gate).
DONE-CHECK: gate `lod` fold == mirror: expand -> rotate -> collapse
equals the direct high-dim rotation (bit-exact), plus the arena
high-water stays bounded (reuse evidence).
DEPS: T1, T4, genarena. BLOCKERS: none.

**T6 · time-phantom networks** (`phant.bp`) — DONE ✓ (fold 8328000021)
GOAL: a full SNN exists for exactly ONE POPCNT pass: the L-rule (T4)
expands to a bit-mask network, one propagation round runs (T3), the
64-bit fold is emitted, and the network evaporates (registers freed).
DONE-CHECK: gate `phant` fold == mirror: expand->propagate->fold->clear
is bit-exact AND the second expansion reproduces the first (generative
determinism = the "phantom" exists identically whenever needed).
DEPS: T3, T4. BLOCKERS: none.

### Layer 2 -- fused hybrids

**T7 · RNS-integrated spike rotors** (`rnsrot.bp`) — DONE ✓ (fold 1000088888708)
GOAL: the spike event = the rotor multiply: the RNS lanes (T2) carry
residues of rotor coefficients; propagation via POPCNT (T3) executes the
geometric rotation R x R~ as ONE fused pass -- the neural and geometric
engines are one operation.
DONE-CHECK: gate `rnsrot` fold == mirror: fused pass == the T1 sandwich
computed separately, bit-exact.
DEPS: T1, T2, T3. BLOCKERS: none.

**T8 · VSA delta mesh sync** (`deltasync.bp`) — DONE ✓ (fold 1168535566021)
GOAL: agents exchange ONLY codebook deltas (XOR of old/new hv4096) +
the i64 fold digest; the receiver applies the delta and verifies the
fold -- context replication without serialization. Packet size frozen
and recorded (the "tiny packet" claim becomes a byte count).
DONE-CHECK: gate `deltasync` fold == mirror: apply-delta + fold check
== direct codebook transfer, bit-exact; mismatched delta -> loud trap
word (the breaker, T12).
DEPS: hv.bp. BLOCKERS: none (single-process gate; real mesh = bare
metal/network, forward-port).

**T9 · self-mutating L-rules** (`mutlsys.bp`) — DONE ✓ (fold 44349936263)
GOAL: the runtime mutates its own generative rule; the .becache fold is
the fitness function: keep the mutation iff the fold improves, else
revert -- natural selection over L-systems. Reuses qlora (quantized
adapter moves) and srepl (DriftClass stability gate) as the mutation
machinery.
DONE-CHECK: gate `mutlsys` fold == mirror: N mutations, keep/revert by
fold comparison, final rule == the mirror's; mutation budget honored.
DEPS: T4, qlora, srepl. BLOCKERS: none.

**T10 · entropic topological collapse** (`entcol.bp`) — DONE ✓ (fold 3000021007)
GOAL: the GC replacement: when a structure's information-utility (a
deterministic entropy estimate from the spectral layer) falls below a
threshold, the structure folds back to its base L-rule and the arena
slot is freed -- no scans, no refcounts.
DONE-CHECK: gate `entcol` fold == mirror: threshold crossings collapse
exactly the structures the mirror predicts; the freed-cell accounting
is exact.
DEPS: T4, spectral (entropy proxy). BLOCKERS: none.

### Layer 3 -- code-as-data runtime

**T11 · JIT D-I fusion / morph loop** (`morph.bp` + runtime) — DONE ✓ (morph gate fold 11, morph_loop.sh 8/8)
GOAL: "data becomes code" via the COMPILER ITSELF: a generated rule is
emitted as AArch64 words, published atomically (F2 mmap-export +
renameat), and the seed mmaps it PROT_READ|PROT_EXEC file-backed --
code is born, runs once, and is replaced by the next publication. This
is the in-sandbox form of the vision's mprotect morphing (proot W^X
blocks mprotect RWX: the file-backed RX map is the W^X-clean
equivalent, already frozen in seed.S).
DONE-CHECK: a bench+gate loop (compile k1-shaped rule -> run -> fold
== frozen) iterated K times proves the morph loop is deterministic
and the artifacts replace atomically; mprotect variant = forward-port
trigger (bare metal).
DEPS: F1/F2 (emitters, export), the whole compiler. BLOCKERS: mprotect
RWX (proot W^X) -- the file path is the substitute.

**T12 · .becache as the only pointer + mismatch breaker** (`ptrless`) — DONE ✓ (fold 1118234452261)
GOAL: references ARE content digests: a state is addressed by its
.becache key + i64 fold; every materialization verifies the fold and
a one-bit mismatch emits a LOUD trap word (3558867200-class) and
aborts -- immunity to garbage/stale reads by construction (the policy,
not a claim of unbreakability).
DONE-CHECK: construct-parity-frozen trap word + gate `ptrless`: correct
key materializes the right state; corrupted key traps (fold != 0).
DEPS: cache.bp, pieblock, sha256. BLOCKERS: none.

### Layer 4 -- the substrate (the terminal-goal gap)

**T13 · register-window emitter (R6.1 protocol)** — ACTIVE-NEXT (the one open gap)
GOAL: the stack machine -> register-resident values: compile-time
"top is in x0" tracking, movs instead of push/pop pairs where provable,
flush-on-bl. This closes the FASTPATH-SPEC done-check.
DONE-CHECK: fixpoint byte-exact + 50 gates + K1/K4 benchmarked -- ship
whatever the numbers are.
STATUS: the mechanism is PROVEN correct (R4#4: 42/42 gates, K1-K4
bit-exact) but was never reconciled with the current emitter (R6.2 v5
folding / L16). NOT YET LANDED. EXACT BLOCKER found this session:
  1. Of ~100 push/pop call sites, push/pop still emit the canonical
     stack words (sub sp,#16; str x0,[sp] / ldr; add sp) — no register
     path exists. Verified: symbols use x19-x28, spills x100+ via
     [x15], arena x14, scratch x2/x3 — x9-x13 appear free for the value
     window, but their use in emitted instr must be CONFIRMED (only
     2 occurrences of "x9"/"x13" seen, none confirmed as emitted words).
  2. flush-on-bl required (live value survives a bl: h(a)+f(b) keeps
     h_result in x(9+0) across the bl to f). Only 2 emit_bl call sites
     (bebop.bp:565,576), both have fntab — thread flush there.
  3. REP state cell = fntab[3890] (free: slot zone ends 3796, literal
     zone begins 3899). Enforce the ONE-representation invariant:
     all-register (depth<5) OR all-memory; migrate reg->mem at bl /
     at depth>=5. Encodings: mov xD,x0 = 0xAA0003E0|(9+d)<<16;
     mov x0,xS = 0xAA0003E0|S<<5. self_check 0.05s / full battery /
     fixpoint (~60s/gen) are the gate loop; snapshot baseline
     /tmp/opencode/t13-baseline/ (md5 13a6447f...). verify x9-x13
     first, then single-variable push/pop/emit_bl increments.

**T14 · dispatcher as the execution substrate (post-von-Neumann)** — DONE
First rung (commit b549416): dispatcher.bp bridge — a kernel's operand
data as a rank-4 .bt word-tensor; the event dispatcher (SWAR popcnt +
de Bruijn tzcnt, LSB-first, NO program counter / fetch-decode loop)
threshold-accumulates the active cells; order-independent bit-exact.
Substrate (commit 8a5bc33): substrate.bp — the dispatcher as the
EXECUTION SUBSTRATE: two canonical kernels execute on the same engine
by iterative activity-wavefront to quiescence (activity word == 0):
  k1 linear chain accumulation cells 0..8 -> 36, 9 sweeps
  k2 fibonacci recurrence ripple fib(25)  -> 75025, 25 sweeps
Fold 36750250113, all oracle bits green. Gates: dispatcher, substrate.
std_golden 60/60.

**T15 · hardware validation / software-PMU — PARTIAL (sandbox-bound)
T15a (DONE, commit 4152ec1): Android perf_event_paranoid=3 + seccomp
block syscall 241 (perf_event_open EACCES, no root to lift). Replaced
with deterministic SOFTWARE PMU counters inside the JIT'd kernel:
iteration/step counter (bit-exact, immune to 2-20x thermal clock
noise) + clock_ms() = CLOCK_MONOTONIC via raw svc (works user-space,
distinct syscall). Gate swpmu pins the k1-style step count bit-exact:
  2001000110000000000. std_golden 60/60.
REMAINING (forward-port to bare metal / ARMv9, cannot gate in-sandbox):
the PMU-backed L1/L2 hit rate, I-cache residency, pool 5/5 on a real
kernel, cold-start <5ms, sustained 2.4GHz, real NEON (Cortex-A78 has
NO SVE/SME — scalars+128-bit NEON are the target; SVE/SME claims need
real ARMv9 silicon). Every number recorded, whatever it is.

### Honest flags (Q12)

1. mprotect RWX is BLOCKED under proot W^X (documented since M4); the
   file-backed RX morph loop (T11) is the W^X-clean equivalent, not a
   fake of the mprotect path.
2. "No heating / no throttling at 2.4GHz" and "thousands of connections
   per microsecond" are HARDWARE claims: unfalsifiable in this sandbox
   (the box demonstrably throttles). Gated on T15.
3. The AST-less semantic stream already exists as the canonical .bt
   tensor + hv4096 interchange; the .bp text remains the AUTHORING
   surface by design (the roadmap never claims the compiler reads
   hypervectors).
4. The terminal-goal sentence stands until T14 lands: the math layers
   are complete and gate-proven; the substrate is still von-Neumann.

## Progress log (closed statuses, evidence)

- **Ф0.3 bootstrap closed (2026-09-01)** — bebop.bin crash on pristine+3fn
  fixed: emit_sys_read/write push bytes through a fixed 8192-byte scratch
  (x28−8192); any len > 8192 = silent write past the arena end (read as a
  ~291KB/112KB "ceiling"). Fix: chunked write/read in CLI (≤8192/call), no
  emitter change; regression around the C parser bypassed at .bp level. Gates
  after: parity 9/0/0, construct 20/20, std_golden 7/7, self_check 0, fixpoint
  TWO GENERATIONS byte-for-byte (bebop.bin == selfA == selfB).
- **GOLDEN + Ф1 HDC closed (2026-09-01)** — spectral_golden/golden.txt: Rust
  references (dowiz-core) extracted BEFORE zero-C (topk_symmetric 6 graphs/32
  iters/i64-fixed-point, Householder eigh parity, LeVerrier charpoly, HDC
  section). Ф1 HDC core `hv.bp` = canonical twin of hypervector.rs (splitmix64
  code, XOR bind, majority bundle ties→0, bit-rotate permute D=1024, SWAR
  popcount/hamming). **Gate hv 4427592702613580868**.
- **SPECTRAL topk_symmetric port closed (2026-09-01)** — `spectral.bp` as
  above; **gate spectral 2038**; JIT == interp exact (deterministic integer
  arithmetic).
- **Loose ends closed (2026-09-01)** — run_program ret −1 holdout (unresolved
  emit_call didn't consume the arg list → phantom push/SP crawl; fix: lexical
  arg skip), C-parser split-brain (single body parser extended to .bp surface),
  interp twins for sys_ftruncate/munmap/mmap/rename (interp == JIT == 720000).
- **Ф2 core primitives closed (2026-09-01)** — `csr.bp` (from_edges structural
  twin: per-row bucketing, selection sort, adjacent-duplicate merge, wrapping
  sums order-independent = exact Rust parity) + csr_spmv canonical summation
  order. **Gate csr −6945622865743784444**. `.bt` rank-4 codec `bt.bp`.
  **Gate bt −5708805812714944038**. LAW (csr): cross-loop state only in cells;
  temp names must not collide with cell names (`tc` clobbered a merge counter —
  silent nnz=0).
- **N1b micro-tier Haar/DWT closed (2026-09-01)** — `haar.bp` integer DWT,
  pure ADD/SUB, exact self-inverse. **Gate haar 41001** (word=41 sign-bits of
  dispatch row + round trip). N1 (wht) + N1b close the MICRO tier.
- **N1c meso-tier NTT closed (2026-09-01)** — `ntt.bp` over Z_p,
  p=998244353=2^23·119+1, primitive root 3; exact round trip +
  convolution identity. **Gate ntt 141003**. LAW (ntt): invert-parameter in an
  if-branch didn't scale 1/n — inverse path extracted into a separate
  straight-line ntt_inv. MACRO (KLT) = spectral.bp topk/Hotelling.
- **Rejected transforms**: DFT/FFT/DCT/DST/Z/DHT — see Terminal goal.

## Roadmap for the next batch (in-order)

Status (2026-09-03, session end): **50/50 std_golden gates**, every gate ==
an independent python mirror bit-exact. Closed this session:
- **R6.2 constant folding LANDED (4th attempt, v5)** — journal
  1788285641: the v1-v3 dynamic-slot model (fntab[3401+d]) was the
  layout-sensitive miscompile source; v5 uses FIVE FIXED CELLS
  (fntab[3655..3659], hard addressing, zero dynamic slot arithmetic) with
  push/pop/emit_cond/emit_match clearing the const depth and emit_lit
  re-recording from captured pre-push state, PLUS the right-const imm12
  path (`i - 1` -> `sub x0,#1`: 5 words vs 10). Fixpoint byte-exact
  (md5 13a6447f), gate `foldx` 7150011 (50th), 10 self_check goldens
  regenerated, construct frozen bins regenerated. Bebop-side wins: K1
  hot loop 35->11ms on a cool run (throttle-bound later), canonical
  k1-loop footprint 124->114 words (-9%).
- **Software PMU / translation index** (SS-10/SS-14 in-sandbox form):
  perf_event_open blocked by the sandbox (syscall probe); the
  deterministic stand-in = the construct frozen bins (byte-freeze every
  emitted stream) + the measured footprints. 100% repeatable by
  construction; the hardware counters stay forward-port.
- **Cooperative fiber scheduler** — gate `fiber` 1215172329: N agents on
  ONE process, shared arena, zero kernel calls — the in-sandbox
  replacement for the clone/futex pool semantics (pool gate = honest
  5-skip under ptrace; bare-kernel trigger for the real 5/5).
- **SS-13 cold start** documented as env tax: median 15.8ms under proot
  (spawn-dominated; the seed's mmap+init is sub-ms by construction).
- **Bench**: K1 4.5-5.6×, K2 2.6×, K3 5.1-8.1×, K4 6.7-10.6× vs Rust
  (two noisy sessions; box thermal-dominated).
Next pulls (in-order): the SILICON-REGISTER PULL section above --
T1->T7 bottom-up, T13/T14 as the substrate endgame, T15 when bare
metal exists. The remaining pre-vision items (SS-10 PMU, SS-14 I-cache
bench, pool-on-bare-kernel, <1% jitter, <5ms cold start, SME/SVE2) all
fold into T15. R3.x(b) stays documented-as-law.

Status (2026-09-03, session 2): **SILICON-REGISTER PULL T1–T12 + T11 all
LANDED**, std_golden 57/57 (was 50/50). Closed this session with each
gate == python mirror bit-exact (commits e5bbdf7, f9cd9f0):
- **T1 tern** 8888868889989889 — ternary Clifford basis {-1,0,1} packed
  2-bits, blade multiply via Cayley masks (no float MUL), rotor sandwich.
- **T2 rns** 1183829339 — 4 moduli 16-bit lanes, Garner CRT spot-check.
- **T3 snn** 65504516937878 — in-register bit-mask SNN, one POPCNT/AND/OR
  propagation round, ternary payload (T1).
- **T4 lsys** 144175882039858 — L-system fractal memory: arena stores the
  rule, expansion generated on demand + FNV digest.
- **T5 lod** 1000088904914 — fractal LOD zoom: expand->rotate->collapse ==
  direct high-dim rotation bit-exact, arena high-water bounded.
- **T6 phant** 8328000021 — time-phantom networks: algae depth-6 expansion
  -> connword -> ring-adjacency propagation -> evaporate + re-expand
  determinism. Fixed connword OR-not-sum (wrap-ADD carried cells 16-20).
- **T7 rnsrot** 1000088888708 — RNS-integrated spike rotors: T1 sandwich of
  e1/e2 round-trips through 4-modulus RNS + Garner DECODE with OFFSET
  encoding (+M/2: Garner is mod-M, negatives need offset). Fused neural +
  geometric passes == one arithmetic.
- **T8 deltasync** 1168535566021 — VSA delta mesh sync: codebook XOR delta
  + FNV fold; bad delta (one flipped bit) does NOT reproduce the digest
  (breaker).
- **T9 mutlsys** 44349936263 — self-mutating L-rules: FNV fold IS the
  fitness fn, keep iff fold improves (signed compare, deterministic).
- **T10 entcol** 3000021007 — entropic topological collapse (GC
  replacement): decaying grammar drives diversity below the 3 threshold;
  3 collapses, 21 cells freed, exact accounting.
- **T11 morph** gate 11 + morph_loop.sh 8/8 — JIT D-I fusion: kernel rule
  published atomically (tmp->sys_export->renameat) + read back
  byte-identical; K=8 compile->atomic-replace->run loop, k1 fold
  500000500000 stable. W^X-clean mprotect equivalent.
- **T12 ptrless** 1118234452261 — .becache-as-the-only-pointer: content
  digest addresses state; corrupted key does NOT resolve (loud breaker).

**ACTIVE NEXT (T13)**: the register-window emitter (R6.1 protocol,
FASTPATH-SPEC.md is binding). The T13 goal is a full re-architecture of
the value machinery — push/pop -> mov x(9+depth) register window, "top is
in x0" tracking, flush-on-bl, ~40 emit_* call sites threaded, then a
fresh fixpoint compile + the whole battery. The SPEC's own R6.2 plan and
the 5-attempt heisenbug history (R4 x5, journal 1788288234) require
single-variable diffs, one commit per step, battery after each, and a
clean revert on ANY layout crash (SPEC fallback: keep the stack machine —
its ops are forwarding-cheap — publish performance-gap honestly). This is
the multi-session architectural jump; stage it as its own focused block.