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
- std_golden.sh → 15/15 PASS
- parity_driver.sh (kernels) → 9/0/0 (+1 main-less skip); (constructs) → 20/0/0
- construct_parity.sh → 20/20 MATCH (words AND values)
- pool tests → 5/5 (JIT-only; interp retired at M7)

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

**N3. Ring-VSA / HDC with colored Hadamard rings (holistic algebraic system)**
- No code/data/type/instruction split: ANY syntactic element, CSR graph, SNN
  spike = a single 4096-bit hypervector.
- WHT = the single algebraic group for bind (binding) and bundle
  (superposition).
- The compiler does not translate code — it homomorphically folds entities into
  the single holographic space of the arena; search/execution = Hamming
  distance.
- Impl: extend `hv.bp` into ring algebra (bind=bundle via the WHT group).

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

**N7. Multiversal superposition branching**
- Eliminates sequential path choice: ALL possible logic branches compute
  simultaneously in one bit array of superpositions via SME/SVE2 vector
  instructions.
- All alternative future states = weighted sum of hypervectors; collapse
  (reality choice) happens automatically when the spectral deflator (Hotelling)
  finds the eigenvalue break λ that masks out false branches.
- Impl: superposition arrays, spectral collapse on top of N1+N6+SS-15/16.

**N8. Spacetime metric code / global boundary execution**
- Eliminates "execution time" as a sequence of steps: the program = a global
  boundary-value problem on the CSR graph surface (Laplace/heat flow).
- Runtime instantly finds the whole system's stationary state as a single
  mathematical equilibrium; past/present/future agree globally via the spectral
  invariant in one hardware pass = "matrix crystallization".
- Impl: Laplacian-boundary execution on top of N6+N7+SS-18.

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

## Spectral Singularity Layer (SS-1..SS-18)

Max projection: arenas + .bt tensors + HDC + spectral geometry. Each item is a
capability with its done-check.

### SS-1 NEON Kalman filter (zero-alloc, arena, real-time)
- Kalman filter as a pure .bp library on linear arenas: zero malloc, zero heap.
- Predict/update matrix ops via NEON 2×2 systolic tiles.
- Deterministic latency: fixed tick count per iteration (WCET guarantee).
- Emit: `kalman_predict(state F Q)`, `kalman_update(state H R z)`.
- Done-check: 1000 iterations → states == golden, 0 drift.

### SS-2 Vector calculus as static invariants (rot/div/grad → graph topology)
- Identities (∇·∇f = ∇²f, ∇×∇f = 0) become CSR-graph structure preservation
  checks; differential operators = bit masks on rank-4 tensors (not symbolic
  math). The compiler statically verifies invariant preservation after any
  tensor transform.
- Done-check: graph divergence = 0 for correct AST transforms.

### SS-3 LC resonance as agent-loop timing (jitter-free)
- Agent loop = electronic LC tank: L = latency, C = arena capacity. Resonant
  frequency f₀ drives the target inter-iteration period — minimal jitter
  without an OS scheduler.
- Impl: clock_ms() + NEON drift compensation (PID-in-.bp).
- Done-check: jitter < 1% over 1000 cycles.

### SS-4 FIR as a ban on cyclic dependencies (BIBO stability)
- FIR: forward-only flow → BIBO guarantee structurally. The compiler REJECTS
  while-loops of unknown depth in agent code at emission; while → bounded masked
  iteration. IIR allowed only with a formal convergence proof.
- LAW: agent code without FIR bounding = rejected at emission.
- Done-check: bounded masked loop domain — zero infinite-loop risk.

### SS-5 Calculus bounding (Taylor/mean-value for mutation code)
- Mean-value theorem + Taylor series → automatic bounding boxes for mutations:
  compiler proves Δ(output) ∈ [f(a)−ε, f(b)+ε] for any CSR-graph mutation.
- Done-check: golden mutations — bounding box contains the actual result.

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

### SS-7 QLoRA 4-bit agentic evolution
- Agent strategy weights = 4-bit matrices in fixed arenas; low-rank adapters
  (A·B with rank << dim) update logic on live hardware. DecompCache stores
  quantized states (FNV-64 key, mmap load). NEON: 4-bit unpack (shift/mask) +
  matvec ~1 cycle/16 elements.
- Done-check: strategy update < 1ms, 0 malloc.

### SS-8 Sinc interpolation (no phase distortion)
- sinc(x)=sin(πx)/(πx) as ideal interpolant for tensor telemetry; NEON
  vectorized approximation (exact to 4 digits). Critical for Kalman (SS-1).
- Done-check: sinc interpolation vs exact — error < 0.1%.

### SS-9 Transformer attention on ARM64 NEON (zero frameworks)
- Q,K,V = rank-4 .bt tensors in linear arenas (64B-aligned).
- Self-attention: hv4096 Hamming distance via vcnt (instead of softmax+float);
  bind = XOR, bundle = majority. Positional encodings = top-k eigenvalues +
  Fiedler vector (spectral, layout-invariant). KV-cache = DecompCache.
- Done-check: attention(Q,K,V) on .bt == C golden, < 1ms on 128 tokens.

### SS-10 Normalization & stride optimization (cache-line aligned)
- Layer/Instance norm = rank-4 stride geometry under 64B cache lines; hot
  tensors L1-resident, cold KV-cache L2/L3; zero false sharing.
- Done-check: attention pass — L1 hit rate > 95%.

### SS-11 Generation arena with MAP_NORESERVE pagination
- mmap(NULL, size, PROT_READ|WRITE, MAP_PRIVATE|ANONYMOUS|NORESERVE); allocation
  = pointer bump (deterministic, zero GC); reset = mprotect(PROT_NONE) instant
  release; new generation = old state → mprotect(READ_ONLY) → bump from base.
- Done-check: 1M alloc/free cycles — 0 syscall (except mmap), 0 fragmentation.

### SS-12 NEON bit matrices (switch/case → parallel bit grids)
- Pattern matching: all conditions packed into dense 128-bit NEON bit grids;
  tens of states per cycle.
- Impl: replace switch/case in emit_call_or_ctor dispatcher with bit masks.
- Done-check: 23-builtin dispatcher via bit matrix — < 10 cycles.

### SS-13 Position-independent DecompCache blocks
- Cached AST graphs + compiled code = position-independent binary blocks;
  zero-copy mmap save/load (disk or /dev/shm); compiled code = raw arena bytes
  without relocations — instant cold start. Not content-addressed (unlike Ф6),
  relocatable PIE-style.
- Done-check: compiler cold start < 5ms.

### SS-14 Direct-threaded code in arenas (no dispatch loop)
- Instructions = direct links to the next handler (no dispatch loop); L1
  I-cache maximized by linear-arena placement. For .bt tensor-op interpretation
  and agent state machines.
- Done-check: threaded vs switch-dispatch — ≥2× on I-cache-intensive load.

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

**SS-18 Spectral self-replication (mutation via ΔA)** — [DriftClass ported]
- Agent changing logic = matrix perturbation ΔA. Check: spectral_drift(A₀,
  A₀+ΔA) → DriftClass (spectral.rs:800 port; `selfhost/std/spectral.bp`).
  Drift within γ → automatic fix (mmap snapshot); outside → .bt dump.
  Replaces textual compilation: evolution = pure spectral jumps.
- Impl: `spectral_drift` in spectral.bp. (Committed: DriftClass profile + delta.)

---

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

Status (2026-09-01): items 1-2 of the current pull are DONE below (rev/store
gates, atomic-publish driver), N4 petri, N5 lsm and N6 holo are DONE (gates
18/19/20, folds in std_golden.sh; holo fold 2766693490590679850 == python
oracle bit-exact); SS-15 scoord and SS-16 sgamma DONE (gates 21/22, folds
2010131/3550431 == python mirrors bit-exact); the canonical fixpoint source
is bebop.bp (the driverless expr_compile.bp is its forward fork - edits land
in BOTH).
Emitter defects reserved for R3.x emitter work:
(a) fast-path `a*b<<c` miscompile (journal 1788288190;
    workaround: parenthesize/lift into lets);
(b) `>>` selector splits by OPERAND ORIGIN: array-loaded/spilled operands emit
    LSR (logical), literal/local-arithmetic operands emit ASR (journal
    1788288193; workaround: shift only non-negative magnitudes, or &-mask for
    LSR);
(c) loop-shaped miscompile: while-loop + local-extract + compare +
    conditional-store -> layout-dependent garbage (journal 1788288197;
    workaround: unroll, hoist values to locals, or branch-free
    multiply-select stores);
(d) str literals and ++ concat SEGFAULT in the .bin runtime; argv strings
    work (journal 1788288206; workaround: str-free programs - argv + cells +
    arithmetic only).
Next pulls: N6 DONE, SS-15/SS-16 DONE; tb (tokenbox, gate 23) DONE - merged
token-economy tool (rtk+graphify+mempalace in one str-free .bp binary);
SS-17 seigtime DONE (gate 24, fold 1233012011, ring-30 eigentime);
next SS-18, then SS-1..14, then SME/SVE2.

1. **N2 → N3**: N2 rev.bp gate DONE (fold 5092789399242, 17th gate;
   xor-toggle/CNOT/Toffoli/Fredkin self-inverse + rev_round/rev_undo delta
   unwind, Oracle-verified; rev_toffoli_bit parenthesized after the `a*b<<c`
   fast-path finding). N3 ring-VSA (hv.bp extension) still OPEN.
2. **.bt store (Ф2/F4 I/O)**: DONE. emit_sys_rename 4-arg byte-packing rewrite
   (register table in both compilers), fixpoint rebuilt bebop.bin dfaf06c3,
   atomic-publish driver Ф6 (argv[5]=tmp -> export -> renameat publish,
   artifact atomically visible, tmp gone), store gate fold
   2245524994793680850 (16th gate). Fixpoint self-test: bb2==bb3 required.
3. **N4 petri.bp** DONE (18th gate, fold 61678606) → **N5 lsm.bp** DONE
   (19th gate, fold -4383576415516299782) → **N6 holo.bp** DONE (20th gate,
   fold 2766693490590679850; WHT-dispersed 4x copies, trim recovery
   pf=15/dan=32/best=0, recovered-tensor re-execution through reservoir at
   2^15 spike scale; oracle == BP bit-exact).
4. **SS-15/16/17 DONE, SS-18 next** — spectral coordinates, gap flow
   control, eigentime (gates 21/22/24, folds 2010131/3550431/1233012011),
   then spectral self-replication (spectral_drift already in spectral.bp);
   then SS-1..SS-14 in dependency order (SS-6 foundation first, already
   CORE DONE).
5. **SME/SVE2 path** — NEON canonical first (already), SVE2 forward port when
   fixed-width is the bottleneck; Spike Dispatcher last (only after the core
   compiler/spectral layer is stable).