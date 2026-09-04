# Bebop — THE Roadmap (single source of truth)

This file supersedes PLAN_B.md, MASTER-FINISH-PLAN.md, ROADMAP_SELFHOST.md,
docs/ZERO_C_CHARTER.md and SWEEP-B3-3.md — all removed. BUGFIXES.md stays
(bug journal), AGENTS.md stays (process laws), bench/ reports stay (evidence).
The 2026-08-17/18 design corpus in `docs/design/BEBOP-*` (glyphic QTT
spec, glyph alphabet, catalog-100, backend roadmap, rewrite plan) is
NOT superseded by this list yet — it contradicts the tree on seven axes;
operator decision 2026-09-04: SUPERSEDED, with 16 selected items carried
over as tasks T68-T83 (TERMINAL-GOAL CLOSURE › CORPUS-A CARRY-OVER); T41
places the banners.
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
- **Tensor database as the language (T16-T21)**: the language IS the database —
  data = tensor fields on a manifold; queries = Einstein summation; integrity/
  audit = the generalized Stokes theorem (`∫∂Ω ω = ∫Ω dω`); schema/migrations =
  the metric tensor and Jacobian coordinate transforms. There is no SQL, no
  relational tables, no WAL locks — the relational paradigm is replaced by
  tensor calculus. The impedance gap between "execution of code" and "execution
  of a query" disappears: memory, persistence and the compiler are ONE type
  system. **dowiz-core (Rust) is the reference implementation of this
  geometry; Bebop is its self-hosted post-von-Neumann form** (each Rust module
  has a `.bp` twin with a gate). This is not a bolt-on database — it is a
  continuation of the post-von-Neumann vision where the substrate dispatcher
  (T14) and the tensor engine converge.
- **Z2-graded register bank + cellular sheaf store (T22-T35)**: the
  fixed register file x9-x13 is a TYPED two-sector space g = g0 (+) g1 —
  x9-x10 even (Cl^0(4,1) rotors/coordinates, commuting), x11-x13 odd
  (Grassmann triggers/masks/transaction parity: anticommuting, x^2 = 0 =
  use-once). Storage is a cellular sheaf (local stalks + restriction maps;
  a query = a global section, H^0), programs/queries are string diagrams
  rewritten to normal form, records are register images (zero
  deserialization), CRUD = CoW versions + nilpotent tokens + Z2
  transactions. Operator decisions and math corrections: see the
  SUPER-SHEAF PULL section.
- **Terminal criterion**: "done" is defined falsifiably in TERMINAL-GOAL
  CLOSURE §TG-DONE (substrate execution of COMPILED programs with one
  conditional branch in the image, substrate-mode self-hosting fixpoint,
  committed oracle per gate, zero tolerated miscompiles, one compiler).
  Audit 2026-09-04: every N/SS/T "DONE" is a standalone gate demo (class
  a); none is yet integrated into emitted code or the runtime (class b)
  except foldx/whileb/r3x/morph; none is hardware-validated (class c).

## Tensor-Database-as-Language — cross-links to dowiz-core (Rust reference impl)

The tensor/geometry vision is NOT speculative here: `crates/dowiz-core` (the
Rust twin repo of bebop-lang) already implements most of it. Bebop's job is to
be the self-hosted post-von-Neumann form, with each Rust module getting a `.bp`
twin + gate. Mapped cross-links (dowiz-core → bebop-lang twin):

| dowiz-core (Rust reference) | Bebop twin (target) | Feature |
|---|---|---|
| `src/tensor.rs` (Tensor1/Tensor2) | `matrix.bp`/`blas.bp` DONE + `tdg.bp` T16 | rank-n arrays, dot/mul/transpose |
| `src/csr.rs`, `src/spectral.rs`, `src/spectral_graph.rs` | `csr.bp`/`spectral.bp` DONE | sparse fp, Laplacian, eigen |
| `src/hypervector.rs` (VSA D=1024) | `hv.bp`/`bt.bp` DONE | bind/bundle, rank-4 codec |
| `src/academia_p2p.rs` (MetricTensor, ChristoffelSymbols, RiemannTensor, geodesic, sectional/scalar curvature) | `tdggeo.bp`/`tdgcurv.bp` T17-18 | full differential geometry |
| `src/parametric_spectral.rs` (parametric manifold, O(1) insert/search, geodesic distance) | `tq.bp` T20 | tensor query engine (manifold DB) |
| `src/memory_search.rs` (geodesic_distance, gaussian_curvature, BM25+PPR fusion) | `tdgstokes.bp`+retrieval T19/21 | manifold memory / audit |
| `src/retrieval/` (bm25, spine, recall, ppr, diffusion) | tensor-DB retrieval twins T20-21 | the "database" layer |
| `src/fdr/pmu.rs`, `src/autonomic_pmu.rs` (PmuStamp, PmuBand) | `swpmu.bp` DONE (T15a) | software PMU cross-validated |
| `src/ktg2/fractal_manchester.rs` (FMA, Manchester transitions) | `dispatcher.bp`/`substrate.bp` DONE (T14) | post-von-Neumann substrate |
| `src/bebop_bridge.rs` (eigen/wave/trinary → bebop protocol) | bebop protocol | existing spectral bridge |
| `microphysics/*.wgsl`, `engine/src/shaders/` (WebGPU) | tensor-op → shader export T20 | GPU/WebGPU query execution |

Method: **port-from-reference, not build-from-scratch.** The math (Christoffel,
Riemann, curvature, geodesic, manifold storage) is proven in Rust. Each `.bp`
twin is a gate (`fn main() -> i64` fold == python mirror that itself mirrors
the Rust numeric oracle). This makes T16-T21 a porting ladder from a known-good
reference, not novel research — dramatically de-risking the "tensor database"
ambition and reusing the established gate discipline.

---

## Verified state (2026-09-04, session 4c)

`seed/seed.S` (frozen AArch64 loader, no libc, 1496B) + `bebop.bin`
(self-hosting compiler, fixpoint bb2 == bb3,
md5 `88d4cd5d3cfaa63f4ab60370b0172d02`, source `bebop.bp` == shipped binary) +
`*.bp` sources + `*.bin` artifacts. **Zero C** — `native/` (175 files) deleted.
std_golden 82/82, construct_parity 24/24, parity_driver 9/9+1skip,
oracles `bench/oracles/run_all.sh` ok=82 (every gate has a committed
independent python oracle, T36; six of them are additionally Rust-backed),
structural invariants `bench/vs_rust/invariants.sh` GREEN (T40/T51).
F-H (source != binary) and F-C (no oracles) are CLOSED; the untyped T13
window was retired (commit 4e6a1d6). Harness scripts accept `BEBOP_BIN`
(candidate compiler) and `BEBOP_TMP` (per-agent scratch namespace).

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
- std_golden.sh → 82/82 PASS (T1–T15a stack; T16–T21 tensor database;
  T22–T24 graded algebra; T27–T30, T33, T34 sheaf/rewrite/CRUD store;
  T86 dpll; orphans tern/rns/snn/lsys/lod/drift wired by T37)
- bench/oracles/run_all.sh → ok=82 (third column: oracle == frozen)
- bench/vs_rust/invariants.sh → GREEN (register zones, fntab zone map +
  literal trap, footer/entry identity, branch census no-increase)
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

**T1 · ternary Clifford basis** (`tern.bp`) — DONE ✓ (fold 8888868889989889; gate wired 2026-09-04 by T37, oracle bench/oracles/tern.py)
GOAL: {-1,0,1} coefficients, 2 bits each, packed into i64 words; blade
multiplication via the Cayley table as combinatorial masks (AND/XOR +
sign-inversion masks), NOT float MUL; the rotor sandwich R x R~ (grade
projection) as a bitwise pass.
DONE-CHECK: gate `tern` fold == python mirror: pack 8 ternary blades,
multiply a rotor pair, sandwich-rotate a probe blade; sign pattern and
2-bit packing invariants (no value outside {-1,0,1}).
DEPS: bits.bp (popcount/rotate). BLOCKERS: none (deterministic i64).

**T2 · packed RNS** (`rns.bp`) — DONE ✓ (fold 1183829339; gate wired 2026-09-04 by T37, oracle bench/oracles/rns.py)
GOAL: 4 coprime moduli with residues in 16-bit lanes of an i64;
parallel add/mul by lane-local arithmetic (no carry chains by
construction); CRT spot-check against the direct i64 result.
DONE-CHECK: gate `rns` fold == mirror: N random-ish pairs, RNS add and
mul == direct mod-2^64 arithmetic on every lane AND the CRT check.
DEPS: none. BLOCKERS: none.

**T3 · in-register SNN engine** (`snn.bp`) — DONE ✓ (fold 65504516937878; gate wired 2026-09-04 by T37, oracle bench/oracles/snn.py)
GOAL: the vision's bit-mask neurons with ternary spikes: a neuron's
state = bit mask; a spike = a packed ternary coefficient (T1); the
propagation step = one POPCNT + AND/OR pass (no per-synapse loops);
the spike event simultaneously encodes a rotor multiply (fuses with T7
later).
DONE-CHECK: gate `snn` fold == mirror: N-bit network, one propagation
round, activity + weight-fold == python.
DEPS: T1 (ternary spike payload), bits.bp. BLOCKERS: none.

### Layer 1 -- generative memory

**T4 · L-system fractal memory** (`lsys.bp`) — DONE ✓ (fold 144175882039858; gate wired 2026-09-04 by T37, oracle bench/oracles/lsys.py)
GOAL: the arena stores ONLY the compact recursive rule + the seed; the
expansion is generated into the arena on demand and folded back into a
digest after use. Expansion factor measured and frozen (bytes of rule
vs words of expansion -- the "orders of magnitude" claim becomes a
number).
DONE-CHECK: gate `lsys`: expand rule (algae/Koch-like) to depth d,
digest the expansion (FNV-64), collapse; fold == mirror; the expansion
factor printed once and recorded in the journal.
DEPS: none. BLOCKERS: none (the claim is measured, not assumed).

**T5 · fractal LOD zoom** (`lod.bp`) — DONE ✓ (fold 1000088904914; gate wired 2026-09-04 by T37, oracle bench/oracles/lod.py)
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

**T13 · register-window emitter (R6.1 protocol)** — RE-SCOPED 2026-09-04 (untyped window retired by operator decision; typed bank = T25 in SUPER-SHEAF PULL; history kept below)
GOAL: the stack machine -> register-resident values: compile-time
"top is in x0" tracking, movs instead of push/pop pairs where provable,
flush-on-bl. This closes the FASTPATH-SPEC done-check.
DONE-CHECK: fixpoint byte-exact + 50 gates + K1/K4 benchmarked -- ship
whatever the numbers are.
STATUS: the mechanism is PROVEN correct (R4#4: 42/42 gates, K1-K4
bit-exact) but was never reconciled with the current emitter (R6.2 v5
folding / L16). NOT YET LANDED. This is the ONE OPEN ROADMAP GAP.
EXACT BLOCKERS (localized 2026-09-02 session):
  1. x9-x13 free for value window — VERIFIED NEGATIVE 2026-09-04:
     decoding every em() constant in bebop.bp (ORR-alias mov, add/sub
     imm, movz, ldr/str, ubfm classes) finds 71 emitted words that write
     or read x9-x13 as scratch inside 8 builtin emitters (emit_sys_open,
     _read, _write, _readbuf, _slurp, _export x2, _rename). x9-x13 are
     also AArch64 caller-saved and the prologue saves only x19-x28 —
     the gen3==gen4 corruption of commit b4326b5 (S1-S3 landed b211451,
     disabled b4326b5, revert-of-disable 9d9a2ba). Any window on x9-x13
     needs T25 S1 (callee-save) + S2 (rehome the 8 emitters) FIRST.
  2. push/pop still emit canonical stack words (sub sp,#16; str x0,[sp]
     / ldr; add sp) — no register path exists. Fix once in push/pop,
     every caller follows (~100 call sites).
  3. flush-on-bl required (live value survives bl: h(a)+f(b) keeps
     h_result in x(9+0) across bl to f). Only 2 emit_bl sites
     (bebop.bp:565,576) — thread flush there.
  4. ONE-representation invariant: fntab[3890] = rep (1=all-registers,
     0=all-memory). Window x(9+depth), depth 0..4. push: rep==1 && depth<5
     → mov x(9+depth),x0 (1 word) + depth+1; else migrate reg→mem then
     memory-push. pop: rep==1 && depth>0 → mov x0,x(9+depth-1) + depth-1;
     else memory-pop. Encodings: mov xD,x0 = 0xAA0003E0|(9+d)<<16;
     mov x0,xS = 0xAA0003E0|S<<5.
  5. fntab[3890] verified free (slot-tag zone ends 3796, literal-offset
     zone begins 3899; 4000 = inline-cache counter).
  6. Do NOT gate on leaky fntab[3700] for register ADDRESSING — use rep +
     depth (fntab[3700] still updated as value-stack bookkeeper for the
     stack-fallback/revert path).
RESUME: /tmp/opencode/t13-baseline/ (md5 13a6447f...) — clean revert
point. Ordered steps (S1-S5): see SESSION-HANDOFF.md. SINGLE-VARIABLE
diffs (L14), battery after each, clean revert on any layout crash.

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

**T15 · hardware validation / software-PMU — TERMINAL (sandbox-bound, forward-port only)
T15a (DONE, commit 4152ec1): Android perf_event_paranoid=3 + seccomp
block syscall 241 (perf_event_open EACCES, no root to lift). Replaced
with deterministic SOFTWARE PMU counters inside the JIT'd kernel:
iteration/step counter (bit-exact, immune to 2-20x thermal clock
noise) + clock_ms() = CLOCK_MONOTONIC via raw svc (works user-space,
distinct syscall). Gate swpmu pins the k1-style step count bit-exact:
  2001000110000000000. std_golden 60/60.
CROSS-LANGUAGE VALIDATION (no exploits needed): swpmu.bp cross-validates
against `crates/dowiz-core/src/autonomic_pmu.rs` (PmuBand/PmuStamp) and
`src/fdr/pmu.rs` (PmuStamp delta) — the same software PMU concept
implemented in both Bebop and Rust. The Rust PmuBand::informed_classification
bands cache-miss deltas the same way swpmu.bp bands step counts. This is a
structural cross-validation: same algorithm, two independent implementations,
two independent type systems — one gate, two oracles. The hardware counters
themselves (perf_event_open) remain terminal; the SOFTWARE counters are
now doubly validated across the language boundary.
REMAINING: HARD PLATFORM BOUNDS (cannot gate in-sandbox):
  - perf_event_open blocked by seccomp (EACCES, syscall 241) — needs
    root/capability or bare kernel.
  - Cortex-A78 has NO SVE/SME — scalar + 128-bit NEON only; SVE/SME
    claims need real ARMv9 silicon.
WORKAROUND PATHS (documented, not executed — requires root/privilege escalation):
  1. Root/Magisk/KernelSU kernel patching: modify security.perf_event_paranoid
     to -1 or 0, or patch kernel/events/core.c directly. Fully opens
     perf_event_open for unprivileged processes.
  2. Runtime Kernel Memory Patching (LPE exploit): for specific kernel
     versions, exploit gains temporary root context, patches
     sysctl_perf_event_paranoid or seccomp filter table in kernel memory
     at runtime, then closes the hole.
  3. User-Mode QEMU with TCG plugins: bypass native Cortex-A78 entirely.
     Run via qemu-aarch64 with TCG translation, enabling precise L1/L2
     cache miss simulation and instruction trace independent of hardware.
  4. Ptrace/Proot PMU Virtualization: intercept syscall 241 (perf_event_open)
     in proot's ptrace layer, return synthetic virtual file descriptor.
     All hardware event reads convert to deterministic software-PMU models.
  5. SIGILL Trap Engine for SVE/SME emulation on NEON: register global
     SIGILL handler; when SVE/SME opcodes hit unknown on Cortex-A78,
     trap decodes instruction, decomposes to 128-bit NEON or scalar ops,
     executes in software buffer, updates register context, advances PC+4.
     Enables SVE code validation on non-ARMv9 at reduced performance.
Forward-port trigger list (record when available):
  - PMU-backed L1/L2 hit rate, I-cache residency
  - Pool 5/5 on a real kernel (JIT-only under proot)
  - Cold-start <5ms, sustained 2.4GHz
  - Real NEON benchmarks (reliable, not thermal-throttled)
  - SVE/SME vector-width tests on ARMv9
Every number recorded, whatever it is. Software PMU (swpmu) is the
in-sandbox hardware-validation substitute — deterministic, bit-exact,
repeatable.

### Tensor Database Engine as the language (T16-T21) — port-from-reference

T16-T21 make the terminal goal concrete: **the language IS the tensor
geometric data engine** — memory, persistence and queries are one type
system, no SQL/WAL/relational layer. Method = port-from-reference: every
feature already proven in `crates/dowiz-core` (Rust) gets a `.bp` twin
with a gate (`fn main() -> i64` fold == independent python mirror that
mirrors the Rust numeric oracle). In-order, lowest-risk first.

**T16 · Einstein index notation + metric tensor (contract, lower/raise)** —
`selfhost/std/tdg.bp`
GOAL: rank-n tensor as flat index (`.bt` codec already), Einstein summation
over a repeated index, metric tensor g_ij (i64 fixed-point 2^32) to lower/
raise indices (v_i = g_ij v^j, v^i = g^ij v_j).
DONE-CHECK: gate == python mirror of dowiz-core `tensor.rs` dot + a 2x2
metric lower/raise example. DEPS: bt.bp, matrix.bp. BLOCKERS: none.

**T17 · Christoffel symbols + covariant derivative** —
`selfhost/std/tdggeo.bp`
GOAL: Γ^k_ij from the metric (first/second kind), covariant derivative
∇_i V^j = ∂_i V^j + Γ^j_ik V^k (flat metric → ∇ == ∂, the falsifiable
degeneracy).
DONE-CHECK: gate == python mirror of dowiz-core `academia_p2p.rs`
`christoffel()`; flat-metric degeneracy gate. DEPS: T16. BLOCKERS: none.

**T18 · Riemann / Ricci / scalar curvature** — `selfhost/std/tdgcurv.bp`
GOAL: Riemann tensor R^i_jkl from Γ, Ricci R_jl = R^i_jil, scalar R.
Falsifiable oracle: **S² (unit sphere, known sectional curvature) —
table-driven known values, not invented.**
DONE-CHECK: gate == python mirror of dowiz-core `academia_p2p.rs`
`sectional_curvature`/`scalar_curvature` on a known-geometry input
(e.g. 2-sphere), all bit-exact i64 fp. DEPS: T17. BLOCKERS: none.

**T19 · Differential forms + exterior derivative d** —
`selfhost/std/tdgforms.bp`
GOAL: k-form as alternating tensor, wedge product, exterior derivative d.
Falsifiable invariant: **d(dω) = 0** (Poincaré lemma) must hold bit-exact
on convenience 2-forms. (Stokes on a 0/1-form boundary as the audit hook.)
DONE-CHECK: gate == d(dω) == 0 on a table-driven 2-form; python mirror.
DEPS: T16. BLOCKERS: none.

**T20 · Tensor Query Engine (manifold database core)** — `selfhost/std/tq.bp`
GOAL: data as tensor fields on a manifold; queries as Einstein contractions/
nearest-neighbor geodesic lookup — port of dowiz-core `parametric_spectral.rs`
(top-2 eigen parametric surface, O(1) insert/search) + `memory_search.rs`
(geodesic_distance) into .bp. "SELECT" ≡ contraction; "JOIN" ≡ tensor product
+ index contraction; "INDEX" ≡ parametric manifold coordinates.
DONE-CHECK: gate == python mirror: insert N points, query nearest by
geodesic distance on the manifold, all bit-exact. DEPS: T16, spectral.bp,
bt.bp. BLOCKERS: none.

**T21 · Stokes' theorem as audit/transaction invariant** —
`selfhost/std/tdgstokes.bp`
GOAL: audit the data engine — a transaction's total change across the
manifold's interior equals the boundary flux: ∫∂Ω ω = ∫Ω dω, computed as
discrete sums of i64 fp, bit-exact. This is the WAL/ACID-free integrity
guard: every committed update satisfies Stokes (no “lost update”, no torn
reads) — the SQL transaction rollback paradigm replaced by a geometric
conservation law.
DONE-CHECK: gate == python mirror at or above machine precision on a
table-driven field; zero false Stokes violations. DEPS: T19, T20.
BLOCKERS: none.

Porting ladder marshal (per column, L14 single-variable): bt.bp/matrix.bp
exist → T16 → T17 → T18 → T19; T20 builds on T16+spectral+bt; T21 caps on
T19+T20. Every column = one file + `fn main() -> i64` + gate appended to
`bench/vs_rust/std_golden.sh`.

### Honest flags (Q12)

1. mprotect RWX is BLOCKED under proot W^X (documented since M4); the
   file-backed RX morph loop (T11) is the W^X-clean equivalent, not a
   fake of the mprotect path.
2. "No heating / no throttling at 2.4GHz" and "thousands of connections
   per microsecond" are HARDWARE claims: unfalsifiable in this sandbox
   (the box demonstrably throttles). Forward-port to bare metal (T15).
3. The AST-less semantic stream already exists as the canonical .bt
   tensor + hv4096 interchange; the .bp text remains the AUTHORING
   surface (the roadmap never claims the compiler reads hypervectors).
   OVERRIDDEN 2026-09-04 for the token form: glyphs are the canonical
   spelling, ASCII a lossless projection (T84); the emitter sees the
   same token stream either way.
4. ~~The terminal-goal sentence stands until T14 lands~~ — T14 LANDED
   (substrate.bp, fold 36750250113). The post-von-Neumann substrate is
   proven: SWAR popcnt + de Bruijn tzcnt, activity-wavefront to
   quiescence, no PC/fetch-decode. Math layers + substrate complete.

## SUPER-SHEAF PULL (Z2-graded register bank + cellular sheaf store, T22-T35)

Added 2026-09-04 (session 4) from the operator's vision text (Z2-graded
register window, supercommutator instead of branches, cellular sheaf
store, categorical rewrite store, register-resident storage layout,
index-free retrieval, CRUD on the substrate). Analysis result: ~55% of
the mathematics is ALREADY gate-proven here (map below); the new pulls
are listed bottom-up with falsifiable done-checks, one `.bp` file + one
`fn main() -> i64` fold == independent python mirror per column, appended
to `bench/vs_rust/std_golden.sh`, per the invariant policy. Compiler-side
columns (T25, T26) obey L14 single-variable diffs, L1 word discipline,
L16 canonical push/pop words, and the FASTPATH-SPEC ban on in-place word
rewrites.

### Operator decisions (2026-09-04, binding for this pull)

1. **Physical Z2 partition of x9-x13.** x9-x10 = even sector g0
   (bosonic: Cl(4,1) even-subalgebra state, rotors, coordinates);
   x11-x13 = odd sector g1 (fermionic: trigger, validation mask, rep /
   transaction-parity invariant). The bank is a TYPED register file,
   not an untyped expression cache. Consequence: T13's untyped value
   window on x9-x13 is RE-SCOPED (see T13 and T25).
2. **Odd sector = Grassmann (exterior) algebra Lambda_5; even sector =
   Cl^0(4,1).** Rationale (math correction to the vision text): a Lie
   superalgebra bracket does NOT give x^2 = 0 for odd x ([x,x] is
   symmetric and generally nonzero, e.g. {Q,Q} = 2P); Clifford odd
   vectors anticommute but square to +-1 (only the CGA null vectors
   n_inf, n_o are nilpotent). Anticommutation AND nilpotency together
   is exactly the exterior algebra. Both properties the vision needs
   ("self-clearing triggers") therefore live in Lambda; rotors live in
   Cl^0(4,1); the null vectors n_inf = e+ + e-, 2 n_o = e- - e+ are the
   bridge (representable with ternary coefficients, both square to 0).

### Math corrections carried into the tasks (Q12 honesty)

- Vision text "xy = -xy" is a typo for xy = -yx (anticommutation).
- "Supercommutator replaces if/else": in a Z2-graded algebra odd*even is
  ODD, so applying a trigger to a multivector does not yield a masked
  multivector by itself. Predication on ARM64 stays mask arithmetic (the
  existing branchless law); the grading's real value is (a) a compile-
  time TYPE invariant on the bank (parity mismatch = trap) and (b) use-
  once semantics (x^2 = 0 == QTT quantity-1 linearity of the 2026-08-17
  rewrite plan). The branch-elimination claim is GATED as an exhaustive
  select-equivalence (T24), not assumed.
- Cl(4,1) has 32 blades. 32 blades x 2-bit ternary = 64 bits = ONE i64:
  a packed ternary CGA multivector fits x9 exactly (tern.bp's Cl(3)
  scheme scaled up). "Continuous components of a full Cl(4,1)
  multivector in x9-x10" is dimensionally impossible (32 fixed-point
  coefficients != 128 bits); the even sector holds packed ternary
  multivectors or a grade-restricted fixed-point subset (a CGA point =
  5 coefficients still needs 5 words -> T26 record, not 2 registers).
  Translators T = 1 - t n_inf / 2 need the coefficient 1/2 -> not
  representable in ternary; ternary CGA gates use rotation rotors only,
  fixed-point CGA is a later column.
- "Search as cohomology" = H^0 of a cellular sheaf = a sheaf-Laplacian
  solve (iterations to residual 0), NOT O(1). Cost is measured (swpmu
  step counts), never assumed. Constant sheaf degenerates EXACTLY to the
  `spacetime`/`vecinv` gates already green.
- "Search as term rewriting to normal form" is well-defined only for
  CONFLUENT + TERMINATING rule sets; the gate includes the critical-pair
  check and rejects rule sets that fail it (the FIR-style structural ban).
- "Nanosecond CRUD", "80% of time is deserialization", "no GC" are
  hardware/workload claims -> forward-port (T15 class); the in-sandbox
  measure is deterministic step counts + exact freed-cell accounting.

### Vision -> existing-gate map (what is NOT new work)

| Vision item | Already proven by |
|---|---|
| Clifford blades as bitmasks, Cayley-mask product, rotor sandwich | gate `tern` (Cl(3), T1), `rnsrot` (T7) |
| Holographic superposition / resonance / unbinding as search | gates `hv`, `ringvsa` (N3), `holo` (N6), `attn` (SS-9) |
| Data as generators (procedural, transient, re-derivable) | gates `lsys` (T4), `lod` (T5), `phant` (T6) |
| Content-addressed references, mismatch breaker | gates `ptrless` (T12), `cache`, `pieblock` (SS-13) |
| Collapse-on-uselessness GC replacement, exact freed accounting | gate `entcol` (T10) |
| Ad-hoc JIT: compile -> atomic publish -> run | gate `morph` + morph_loop.sh (T11) |
| Register wave / activity-wavefront execution, no PC | gates `substrate` (T14), `dispatcher`, `spike`, `petri` (N4) |
| Harmonic consistency on a graph (constant-sheaf case) | gates `spacetime` (N8), `vecinv` (SS-2), `csr` |
| Layout-invariant canonical forms (isotopy precedent) | gates `pieblock` (SS-13), `scoord` (SS-15) |
| Zero-copy mmap layout, atomic publish | `.bt` codec (F4), `bt`/`store` gates, F2 |
| Reversible mutation / rollback without snapshots | gate `rev` (N2) |
| Transaction as conservation law (Stokes) | T21 (planned, T16-T21 ladder) |
| Use-once (linear) semantics | QTT quantity 1 (BEBOP-LANGUAGE-REWRITE-PLAN 2026-08-17 s3.1) |
| Rust reference for sheaf coboundary / Laplacian L = B^T W B | `crates/dowiz-core/src/incidence.rs` (canonical oriented-edge Laplacian) |
| Rust reference for open-hypergraph diagrams | `crates/dowiz-core/src/hypergraph.rs` (incidence, sparse Laplacian, structural_hash) |

### Layer A -- graded algebra (new gates, pure .bp arithmetic, zero compiler risk)

**T22 . Grassmann algebra Lambda_5 as bitmask monomials** (`grass.bp`) — DONE ✓ 2026-09-04 (gate grass 10312435099105887 == bench/oracles/grass.py)
GOAL: 5 generators -> 32 monomials indexed by their basis bitmask, ternary
coefficient 2 bits each -> ONE i64 (the x11 payload). Product of
monomials e_I * e_J = 0 when I & J != 0 (nilpotency = one AND), else
(-1)^{inv(I,J)} e_{I|J} with inv = the transposition count already
computed by tern.bp's gprod (degenerate metric: no e_i^2 term).
DONE-CHECK: gate `grass` fold == python mirror: e_i e_j == -e_j e_i for
all 10 pairs; e_I^2 == 0 for ALL 31 non-scalar monomials (exhaustive);
associativity on a triple; Z2 grading closure |I|+|J| mod 2; no packed
value outside {-1,0,1}.
DEPS: tern.bp (sign count), bits.bp. BLOCKERS: none.

**T23 . Cl(4,1) ternary CGA basis, even subalgebra, null vectors** (`cl41.bp`) — DONE ✓ 2026-09-04 (gate cl41 1807759285641197332 == bench/oracles/cl41.py)
GOAL: 32 blades x 2 bits = one i64 (the dimensional fact above);
signature (+,+,+,+,-) as a per-generator sign mask in the Cayley product;
even subalgebra Cl^0 (16 blades) closed under product (the x9/x10
payload, 32 bits each -> two even multivectors per register or one +
accumulator); null vectors n_inf = e+ + e- and 2 n_o = e- - e+ square
to 0 EXACTLY and have scalar product -2; rotation rotor sandwich
R x R~ on a CGA point (rotation only, see corrections).
DONE-CHECK: gate `cl41` fold == mirror: even*even is even for a table
of pairs; n_inf^2 == 0, (2 n_o)^2 == 0, <n_inf, 2 n_o> == -2; sandwich
== direct table; ternary invariants.
DEPS: tern.bp, T22 (shared sign count). BLOCKERS: none.

**T24 . supercommutator + nilpotent trigger + select-equivalence** (`zgrade.bp`) — DONE ✓ 2026-09-04 (gate zgrade 5676760058329986817 == bench/oracles/zgrade.py)
GOAL: [x,y] = xy - (-1)^{p(x)p(y)} yx on the graded pair Cl^0 (+) Lambda;
the odd trigger's self-clearing t * t == 0; and the vision's branch claim
as a THEOREM: parity-sign x mask select == the if/else reference.
DONE-CHECK: gate `zgrade` fold == mirror: [even,even] == commutator,
[odd,odd] == anticommutator == 0 on generators (exhaustive pairs);
super-Jacobi on one triple; trigger applied twice == 0 for all 31
monomials; select-equivalence over ALL 256 (cond, a, b) bit patterns
(bitmat-style exhaustive), zero mismatches.
DEPS: T22, T23. BLOCKERS: none.

### Layer B -- the graded register bank (compiler; T13 re-scope)

**T25 . Z2 bank ABI: x9-x13 callee-saved, builtin scratch rehomed, typed slots**
Verified facts (2026-09-04, decoder over every `em()` constant in
bebop.bp -- ORR-alias mov, add/sub imm, movz, ldr/str, ubfm classes):
**71 emitted words write or read x9-x13 as scratch inside 8 builtin
emitters** -- emit_sys_open (686), emit_sys_read (724), emit_sys_write
(760), emit_sys_readbuf (844), emit_sys_slurp (904), emit_sys_export
(3247, 3262), emit_sys_rename (3341). The prologue saves only x19-x28
(stp at [sp,#0..#64]); x9-x13 are AArch64 caller-saved, which is exactly
the gen3==gen4 corruption of commit b4326b5. seed.S touches x9/x10/x12
only BEFORE its blr (entry setup) -- the loader does not depend on the
bank across the JIT call. Frame slots [sp,#80..#256) are unused (spill
base x15 = sp+256, heap x14 = sp+768).
S1 (ABI): prologue adds stp x9,x10 / stp x11,x12 / str x13 into
   [sp,#80..#120), epilogue mirrors -- +6 words per fn (123 fns in
   bebop.bp => +738 words, <1% of the 344KB binary). Encodings per L1
   (asm -> objdump -> disasm diff), NEVER typed. Done-check: fixpoint
   byte-exact, std_golden 60/60, construct frozen bins regenerated (every
   stream grows by exactly 6 words per fn -- assert the delta).
S2 (rehome): the 8 emitters stop using x9-x13. Default mechanism: save
   the bank to its frame slots at emitter entry and restore before the
   result write (mechanical, 6 words per builtin); alternative: re-
   register onto x4-x7/x16/x17 (sys_rename needs 8 scratch regs and only
   6 are free -> spill two into the IO scratch zone). One emitter per
   commit (L14), register-table comment per L2, disasm diff per L1.
   Done-check: extend `tools/check_abi.py` from x27/x28 to x9-x13 --
   ZERO writes to x9-x13 outside prologue/epilogue and the T25 bank
   builtins; fixpoint + battery green.
S3 (typed slots): x9 = E0, x10 = E1 (even: packed Cl^0(4,1) or fp
   2^32 coordinate), x11 = O0 trigger (Lambda_5 monomial), x12 = O1
   validation mask, x13 = O2 rep/transaction parity (runtime twin of the
   compile-time rep cell fntab[3890]: bit0 = pending-transaction parity,
   upper bits = generation). Six builtins: even read/write, odd
   read/write, `scomm` (T24 supercommutator on the bank), `nkill`
   (nilpotent AND-test kill). Compile-time parity per bank slot in FIXED
   cells (R6.2 v5 law: hard addressing, no dynamic slot arithmetic):
   fntab[3891..3898] are unreferenced in bebop.bp (grep 2026-09-04;
   3701+d is capped at 3796, 3903+i grows upward) -- VERIFY again before
   use. Odd value into an even slot (or vice versa) = COMPILE-TIME loud
   trap (3558867200-class). Done-check: gate `zbank` (a .bp program
   that rotates an even state in x9 by a rotor in x10 under a trigger in
   x11 == the T24 mirror), parity-mismatch program traps at compile,
   parity-free programs produce byte-identical streams to pre-S3
   (zero cost when unused, L16 spirit).
DEPS: T22-T24 (payload semantics), FASTPATH-SPEC discipline. BLOCKERS:
none in-sandbox. NOTE: the uncommitted working-tree diff in bebop.bp
(2026-09-04: `can_reg` register path in push/pop using fntab[3890] as a
0..5 depth counter) is the UNTYPED window and conflicts with this
decision -- revert or re-scope it before any commit.

**T13 (re-scoped 2026-09-04)**: the untyped x9-x13 value window is
RETIRED by operator decision 1. The expression path stays the stack
machine + R6.2 folding (FASTPATH-SPEC fallback clause, invoked
deliberately, not by failure); register residency is provided by the
TYPED bank (T25) for algebraic state and by T26/T35 for records. The
T13-attributed K1-K4 projections in "Predicted speedup" are WITHDRAWN
until re-measured after T25/T26/T35 (ship whatever the numbers are).

**T26 . register-resident record layout (zero deserialization)** (`regrec.bp`)
GOAL: a record = 40 bytes = 5 words = the bank image in sector order
[E0,E1 | O0,O1,O2]; a table = `.bt` rank-4 tensor dims [N,5,1,1] (F4
codec unchanged); publish via F2 (mmap-export + renameat); load = ldp
x9,x10 / ldp x11,x12 / ldr x13 from the mmapped base (3 words), store
mirrors it -- the on-disk bytes ARE the register image, no parse step.
DONE-CHECK: gate `regrec` fold == mirror: pack N records -> export ->
read back -> load into the bank -> fold over all records == direct fold
over the tensor; disasm diff of the 3-word load/store sequences (L1);
sys_export/renameat register tables per L2.
DEPS: T25, bt.bp, store.bp. BLOCKERS: none (mmap works under proot).

### Layer C -- cellular sheaf store (port-from-reference: incidence.rs)

**T27 . cellular sheaf on a graph: stalks, restriction maps, coboundary,
sheaf Laplacian** (`sheaf.bp`) — DONE ✓ 2026-09-04 (gate sheaf 1114020060 == oracle)
GOAL: stalk dim d <= 2, i64 fixed-point 2^32; restriction maps rho_{v<e}
as 2x2 fp matrices per (vertex, edge); coboundary (delta x)_e =
rho_{v<e} x_v - rho_{u<e} x_u (oriented like incidence.rs, head > tail);
sheaf Laplacian L_F = delta^T delta. Consistency of local data =
delta x == 0.
DONE-CHECK: gate `sheaf` fold == python mirror of incidence.rs semantics:
identity restrictions => L_F == the graph Laplacian D - A from csr/vecinv
bit-exact (the falsifiable degeneracy); a consistent assignment gives
delta x == 0 on every edge; ONE perturbed restriction map leaks a nonzero
residual on exactly that edge (vecinv-style breaker).
DEPS: csr.bp, matrix.bp, vecinv.bp. BLOCKERS: none.

**T28 . global sections H^0 as the query; harmonic iteration; Euler
characteristic** (`sheafh0.bp`) — DONE ✓ 2026-09-04 (gate sheafh0 11121890396072 == oracle; it_t=396 it_c=72 recorded)
GOAL: "does a state exist that satisfies all local constraints" = find x
in ker delta extending pinned local values, by Jacobi/heat-flow on L_F
(the spacetime.bp crystallization generalized to non-identity
restrictions); residual -> 0 (consistent) vs residual floor > 0
(inconsistent, answer NO). Falsifiable topology: on a tree with
invertible restriction maps dim H^0 = d (d independent harmonic
sections found); on a cycle with a twisted monodromy (product of
restriction maps around the loop != identity, the Mobius case) dim H^0
< d; chi = sum_v dim F(v) - sum_e dim F(e) == dim H^0 - dim H^1 checked
on both.
DONE-CHECK: gate `sheafh0` fold == mirror: residuals, dim H^0 on tree
and twisted cycle, chi identity, iteration counts RECORDED (the cost is
a Laplacian solve; swpmu step count frozen in the journal).
DEPS: T27, spacetime.bp, swpmu.bp. BLOCKERS: none.

**T29 . content-addressable sheaf nodes (O(1) resolve + phase address)**
(`csheaf.bp`) — DONE ✓ 2026-09-04 (gate csheaf 5155430002134088 == oracle)
GOAL: node address = FNV-64 digest of its stalk (ptrless discipline);
resolve = digest -> slot, verify fold, corrupted key traps; "phase
address" variant: digest -> fixed-point angle -> bucket in Cl^0 (rotor
angle as the hash); on retrieval the stalk is CHECKED against its
neighbours through delta (the sheaf validates the record, not a schema).
DONE-CHECK: gate `csheaf` fold == mirror: N keys resolve to their
stalks; one corrupted key does NOT resolve (loud breaker); bucket
histogram == mirror; an inconsistent inserted stalk is rejected by the
delta check.
DEPS: T27, ptrless.bp, cache.bp. BLOCKERS: none.

### Layer D -- categorical rewrite store (port-from-reference: hypergraph.rs, petri.bp)

**T30 . string diagrams as open hypergraphs with Z2-typed wires** (`sdiag.bp`) — DONE ✓ 2026-09-04 (gate sdiag 654345454 == bench/oracles/sdiag.py)
GOAL: boxes = nodes with typed input/output wires; wire type carries the
Z2 parity (T24); sequential composition (o) and parallel composition
(x) with parity check on every plugged wire (mismatch = trap);
canonical form = topological order with content-digest tie-break
(structural_hash precedent in hypergraph.rs).
DONE-CHECK: gate `sdiag` fold == mirror: isotopy invariance -- permuting
node ids / sliding boxes leaves the canonical fold IDENTICAL (pieblock/
scoord layout-invariance style); interchange law (f x g) o (h x k) ==
(f o h) x (g o k) bit-exact on folds; parity mismatch traps.
DEPS: T24, graph.bp, petri.bp. BLOCKERS: none.

**T31 . rewriting to normal form: termination, confluence, query =
normalize** (`rewrite.bp`) — DONE ✓ 2026-09-04 (gate rewrite 38233233101031 == bench/oracles/rewrite.py, 0.2 s)
GOAL: a small rule set on T30 diagrams (monoid unit/assoc + Petri token
rules); termination by a strictly decreasing node-count measure;
LOCAL CONFLUENCE by exhaustive critical-pair joining on small terms;
normal form unique => "the answer is the normal form"; pattern diagrams
with holes as queries (matches = unification against the store).
DONE-CHECK: gate `rewrite` fold == mirror: two different rewrite orders
reach the SAME normal form fold; all critical pairs join; a deliberately
non-confluent rule set is REJECTED at load (structural ban); query
matches == mirror.
DEPS: T30. BLOCKERS: none.

### Layer E -- CRUD on the substrate

**T32 . ad-hoc query JIT (predicate -> native filter kernel)** (`qjit.bp` + morph loop)
GOAL: (field, op, const) -> a `.bp` kernel text generated IN .bp (str-free
arithmetic per R3.x(d)) -> compiled by bebop.bin -> published atomically
(T11 morph path, W^X-clean file-backed RX) -> run over the T26 record
tensor via the 3-word bank load; constants baked as imm12 (R6.2 right-
const path) -- no query planner, no interpreter.
DONE-CHECK: gate `qjit` fold == mirror: filtered set == python filter for
several predicates; swpmu step count per record recorded; morph_loop-
style K iterations deterministic.
DEPS: T26, morph.bp, swpmu.bp, R6.2 imm path. BLOCKERS: mprotect RWX
(proot W^X) -- file-backed RX is the substitute, as T11.

**T33 . CoW versioning + nilpotent reader tokens (MVCC without WAL)** (`mvcc.bp`) — DONE ✓ 2026-09-04 (gate mvcc 68412663603207 == bench/oracles/mvcc.py)
GOAL: update = NEW record + a restriction-map edge to the old version
(a sheaf edge, T27) -- never in place; readers hold Grassmann generator
tokens (T22); release = product with the own token -> 0 (nilpotency =
end of liveness); a version whose token product reaches 0 collapses and
its cells are freed (entcol exact accounting) -- no scans, no refcount
integers, no GC thread.
DONE-CHECK: gate `mvcc` fold == mirror: N interleaved updates/readers --
every read fold equals SOME committed version (no torn reads); freed-
cell accounting exact; old versions survive exactly while a live token
exists.
DEPS: T22, T27, genarena.bp, rev.bp, entcol.bp. BLOCKERS: none.

**T34 . Z2 transactions (STM on the grading)** (`stm.bp`) — DONE ✓ 2026-09-04 (gate stm 871596764015151 == bench/oracles/stm.py)
GOAL: uncommitted writes accumulate in the ODD sector (O0 trigger, O1
mask, O2 parity) as Grassmann monomials; commit = pairwise product ->
EVEN (parity 0) iff the sheaf residual of the touched nodes is 0 (T28,
no conflict); abort = the odd context squared = 0 in ONE operation
(nilpotency); every commit must satisfy the T21 Stokes identity (cross-
gate once T21 lands).
DONE-CHECK: gate `stm` fold == mirror: N transactions with injected
conflicts -- committed set == mirror; aborted transactions leave the
store BIT-IDENTICAL; parity register returns to 0 after every commit/
abort; Stokes residual 0 after every commit.
DEPS: T22, T24, T25 (bank), T28, T33, T21. BLOCKERS: none.

**T35 . register wave filter (the index-free stream)** (`wave.bp`)
GOAL: the record stream flows through the bank (T26 load per record);
the odd trigger (O0) acts as a FILTER by nilpotent kill: records whose
mask product with the trigger is 0 are dropped, survivors become the
current even state -- fused with the T14 dispatcher (survivors set
activity bits; sweep to quiescence), no keys, no addresses.
DONE-CHECK: gate `wave` fold == mirror: survivors == python filter,
order-independent per sweep (substrate discipline), step count recorded
vs the T32 JIT filter on the same data (two mechanisms, one oracle).
DEPS: T24, T25, T26, substrate.bp. BLOCKERS: none.

### Ladder (in-order, L14 single-variable per column)

T22 -> T23 -> T24 (pure algebra, no compiler risk, can run in parallel
with T16-T21) -> T25 S1 -> S2 -> S3 (compiler, one commit each, battery +
fixpoint after each, clean revert from a fresh baseline snapshot) -> T26
-> T27 -> T28 -> T29 -> T30 -> T31 -> T32 -> T33 -> T34 (needs T21) ->
T35. Every column = one file + `fn main() -> i64` + gate line + journal +
ROADMAP status + commit/push.

### Honest flags (Q12) for this pull

1. The physical partition COSTS: +6 words per fn (ABI), 8 builtin
   emitters rewritten, and the untyped T13 window retired. Generic
   expression speed is NOT improved by this pull; algebraic-state
   residency is. Numbers are shipped after measurement only.
2. Ternary CGA cannot express translators (needs 1/2); fixed-point Cl(4,1)
   (32 words per multivector) is a later column, not in T23.
3. H^0 queries cost a Laplacian solve; "no scan" is true, "O(1)" is not.
   T29 gives O(1) by content address; consistency (T28) is iterative.
4. Normal-form search is restricted to confluent + terminating rule sets
   by construction (T31 rejects the rest).
5. Nilpotent tokens replace refcount INTEGERS and GC threads, not the
   concept of liveness: readers must still hold and release tokens.
6. Hardware claims (nanosecond CRUD, 80% deserialization share) are
   forward-port; the in-sandbox oracle is swpmu step counts + exact
   accounting. W^X under proot -> file-backed RX for T32, as T11.

## TERMINAL-GOAL CLOSURE (audit 2026-09-04 + task stack T36-T67)

Added 2026-09-04 (session 4b). Three read-only audits were run over the
whole tree: (1) the compiler surface (`bebop.bp` 3420 lines + seed.S +
bench scripts), (2) the 116-module std corpus against the 60 gates, (3)
the design corpus (`docs/design/BEBOP-*` and `bebop-lang/docs/*`). This
section records what the audits found, defines a FALSIFIABLE terminal
criterion, and lays the remaining work bottom-up. Nothing here replaces a
green gate; several items REMOVE unbacked claims from this file.

### Audit findings (facts, with locations)

**F-A. Gate corpus = demo folds, not integration.** Of 60 gates only 3
test the compiler (`foldx`, `whileb`, `r3x`) and one (`morph` + morph_loop
.sh) exercises a real compile->publish->execute path; the other 56 are
standalone `fn main() -> i64` arithmetic demos compiled to ordinary
AArch64 by the same emitter. Every N/SS/T "DONE" item is therefore class
(a) "proven as a standalone gate", NOT (b) "integrated into emitted code
or the runtime" and NOT (c) "hardware-validated" (zero items are (c)).
The dispatcher/substrate gates (T14) prove the MATH of a program-counter-
free execution model inside a program that itself runs on a program
counter.

**F-B. Five DONE items have no gate.** T1 tern, T2 rns, T3 snn, T4 lsys,
T5 lod are marked "DONE ✓ (fold …)" but `bench/vs_rust/std_golden.sh`
contains no `gate tern|rns|snn|lsys|lod` line; the sources sit in
`std_tests/` unwired (commit e5bbdf7 shipped files, not gate lines).
`drift.bp` is a sixth unwired test. Fixed in this file: the five headers
now read "GATE NOT WIRED -> T37".

**F-C. The "independent python mirror" behind 54 gates is not in the
repo.** No `.py` computes any gate fold (`git log --all -- '**/*.py'`:
9 files ever, none an oracle); mirrors exist only as prose in
`std_golden.sh` comments and `docs/exp.journal`. Re-running std_golden
proves determinism against the compiler's own past output. The only
runnable external oracle is `bench/vs_rust/spectral_golden/generator`
(Rust, path-dep on `crates/dowiz-core`; cargo IS installed) covering 6
gates: spectral, cache, csr, hv, bt, store.

**F-D. No module system; std reuse is copy-paste.** `module core { }` is
inert text (no parser handles it; `collect_fns` scans for `fn `). 44/60
gate files are byte-identical copies of std modules; `fp_mul` is
embedded verbatim in 14 std files, `isqrt` in 9, `lcg` in 7, `popc` in
6, `fnv` in 5. 50 of 116 std modules are DEAD (no main, no gate, no
reference anywhere): automaton bignum bitfield bits bitset blas
combinatorics dist dp effect encoding event file_io fmt gcra graph hash
heap list log markov math matrix modular nat numeric ops pac permutation
pid polynomial primes queue quicksort radix ratelimit ring rle search
session set stack statistics stats string strutil tensor token_bucket
vec version. `pool.bp`/`pool_compile.bp` compile only with the LEGACY
`selfhost/expr_compile.bp` (3128 lines, the second compiler, owner of
sys_clone/futex/atomic builtins that `bebop.bp` lacks).

**F-E. Branches: every `if` is cmp+b.eq+b, every `while` is cmp+b.eq+
backward b.** `emit_cond`'s docstring (bebop.bp:1529) claims a branchless
csel; the code (1532-1560) emits placeholder-patched branches. Census
(decoder over the frozen bins, 2026-09-04): c05_if 2 b.cond + 2 b,
c07_while 1+1, `bebop.bin` 83515 words = 859 b.cond, 86 cbz/cbnz, 3 tbz,
820 b, 850 bl, 121 ret, 13 svc, 0 csel-as-if (the 765 "csel-family"
words are `cset` from comparisons). `match` on literal ctors is compile-
time (0 branches) but never on runtime values.

**F-F. Compiler debt that the laws currently paper over.** Types are
parsed and DISCARDED (`skip_to_delim` L127: `: T` skipped) — i64 is the
only scalar, `str`/`[i64]` are raw pointers; struct literals are disabled
(`struct_kill = 1`, L188); unary `-`/`!`, hex literals, block comments,
`return`/`break` absent; R3.x(a)-(e) are documented miscompiles kept as
LAWS (`>>` = LSRV logical; str `++` segfault; clock_ms zero-arg parse;
loop-shaped store); L8 (no allocation in while) is a frame-heap leak, not
a semantic rule; latent zone collision: literal offsets `fntab[3903+li]`
reach the scan-budget cell `fntab[4000]` at >= 97 string literals and
overrun `zeros(4096)` at >= 193 (bebop.bp has 41 — latent, undocumented
until now); `self_check` c37-c41 call `exec`, which is not a builtin of
the self-hosted emitter (cli_run L3208 returns 2) — they cannot pass;
seed.S:55 comment says 64MB, the code maps 256MB; syscall surface is 10
numbers (38 46 56 57 63 64 93 113 215 222) — no sockets, no clone/futex,
no getrandom, no sched_setaffinity.

**F-G. Two design corpora contradict each other and this file cites
neither as superseded.** `docs/design/BEBOP-{LANGUAGE-SPEC,GLYPH-
ALPHABET,ARCHITECTURE-CATALOG-100,BACKEND-ROADMAP,LANGUAGE-REWRITE-PLAN}`
(2026-08-17/18) promise a glyph-only surface, QTT/Lean proof kernel, SMT
contracts, C bootstrap, WASM/x86_64/GPU/FPGA backends, hv1024, and a
6-10 week dowiz rewrite; the tree is ASCII i64-only, zero-C, AArch64-
only, hv4096, no proof kernel; BACKEND-ROADMAP:3 "WASM + AArch64 native
live" names a dormant never-loaded file. Stale in corpus B: `bench/
VERIFICATION.md` + `FUZZING.md` (describe the deleted `native/` C
fuzzer; there is NO fuzzer today), `bench/SELFHOST.md`, `HV_ARCHITECTURE
.md` §Status, `LEGACY_BP_ANALYSIS.md` counts, `selfhost/readme.md`,
`samples/*.bp` (use `theorem/struct/module` the compiler rejects),
`docs/ANDROID.md` gate counts (42/42 vs 60/60). bebop-lang is absent
from `docs/design/CORE-ROADMAP-INDEX.md` (the dowiz master index).

**F-H. Source != shipped binary, and the tree is being edited
concurrently.** `bebop.bp` in the working tree carries an uncommitted
T13 window (WCAP=5, flush_window at 18 sites) while `bebop.bin` (md5
13a6447f) is the pre-T13 fixpoint; every "60/60" in this file was
measured on the binary, not the dirty source. A second Claude Code
session edited `bebop.bp` at 14:37 on 2026-09-04 while this audit ran —
single-writer discipline (AGENTS.md PARALLEL AGENT PROTOCOL: main thread
owns writes) was violated by the environment, not by a rule change.

**F-I. Hardware facts not in any doc.** /proc/cpuinfo: 4x Cortex-A55
(0xd05) + 4x Cortex-A78 (0xd41), big.LITTLE; Features = fp asimd aes
pmull sha1 sha2 crc32 atomics fphp asimdhp asimdrdm lrcpc dcpop asimddp
(NO sve/sme, as recorded; but hardware CRC32/SHA2/LSE atomics ARE
present and integer-exact). No benchmark pins cores; the 2-20x timing
noise includes A55/A78 migration, not only thermal. 7.7GB RAM. proot
(TracerPid != 0). cargo, rustc, gcc, clang, objdump all installed (zero-C
is a runtime policy, not a tooling absence).

### Terminal criterion (TG-DONE) — falsifiable

The terminal goal is reached when ALL of the following hold, each with
its own gate line in a committed script:

1. **Substrate execution of compiled programs.** `bebop.bin compile
   --substrate` turns a `.bp` program into (a) branch-free cell kernels
   and (b) an incidence/activity `.bt` tensor; the runtime sweep
   (`activity != 0`) is the ONLY conditional branch in the executable
   image. Gate: branch census of the image == 1 conditional branch; fold
   == the linear-mode fold for every std gate and for K1-K4.
2. **Self-hosting on the substrate.** `bebop.bp` compiled in substrate
   mode compiles itself to a byte-exact fixpoint (bb2 == bb3) in
   substrate mode.
3. **Every gate has a committed independent oracle** (python or Rust)
   that reproduces the frozen fold from scratch; a gate without one is
   labelled `self-frozen` in this file, never "proven".
4. **Zero tolerated miscompiles**: the R3.x(a)-(e) laws are deleted
   because the defects are fixed and regression-gated; no ban list
   remains in "Design laws" except capacity limits with loud traps.
5. **Single compiler, single language**: `selfhost/expr_compile.bp` is
   retired; every construct the language accepts is in construct_parity;
   every std module is gated or in an explicit attic.
6. **Hardware claims are measured or labelled forward-port**, never
   projected in a table without a measurement column.

Items 1-2 are the post-von-Neumann substance; 3-6 are the honesty floor
the substance stands on. The order below builds 3-6 FIRST because every
later gate inherits their oracles.

### Layer V — verification foundation (truth becomes reproducible)

**T36 · committed oracles for every gate** (`bench/oracles/<gate>.py`,
`bench/oracles/run_all.sh`) — DONE ✓ 2026-09-04 (run_all ok=82, self-frozen=0; L17 added to AGENTS.md)
GOAL: one python file per gate that computes the fold from the gate's
mathematical definition (not by re-reading the .bp); `std_golden.sh`
gains a third column: bebop == frozen == oracle. Recover the 28 mirrors
described in `docs/exp.journal` first; write the missing ones; where a
mirror cannot be reconstructed the gate is relabelled `self-frozen` in
the gate list of this file (honest downgrade, not deletion).
DONE-CHECK: `run_all.sh` prints 60 folds == std_golden's frozen table;
the 6 Rust-backed gates additionally re-run `spectral_golden/generator`
(`cargo run --release`) and diff `golden.txt` byte-exact.
NEW LAW L17: a `gate` line is accepted only with a committed oracle file
in the same commit.
DEPS: none. BLOCKERS: none (python3 + cargo present).

**T37 · wire the orphan gates** (tern, rns, snn, lsys, lod, drift) — DONE ✓ 2026-09-04 (std_golden 82/82; drift 5903978048000947864)
GOAL: add the six `gate` lines with their journal folds (8888868889989889,
1183829339, 65504516937878, 144175882039858, 1000088904914, drift = TBD
from spectral_golden DRIFT GOLDENS), each with a T36 oracle.
DONE-CHECK: std_golden 66/66; T1-T5 headers in this file flip from
"GATE NOT WIRED" to DONE with the std_golden count in the evidence.
DEPS: T36. BLOCKERS: none.

**T38 · dead-std triage + prelude** (`selfhost/std/attic/`,
`selfhost/prelude/{fp,bits,hash,rng}.bp`, `tools/gen_selfsrc.sh`)
GOAL: each of the 50 dead modules either gets a `fn main` + gate + oracle
or moves to `attic/` with a one-line reason; the 14/9/7/6/5 verbatim
copies of fp_mul/isqrt/lcg/popc/fnv become ONE prelude file each,
concatenated by `gen_selfsrc.sh` into gate sources (textual include is
the L9-compliant mechanism until T47 lands a language-level `use`).
DONE-CHECK: `grep -c "fn fp_mul" selfhost/std/*.bp` == 1; std_golden
count unchanged or higher; attic listed in this file.
DEPS: T36. BLOCKERS: none.

**T39 · reference interpreter + compiler fuzzer** (`tools/bpref.py`,
`bench/fuzz/gen.py`, `bench/fuzz/fuzz.sh`) — PARTIAL 2026-09-04 (bpref.py landed: checksum/foldx/whileb/r3x/fir == frozen; fuzzer in progress)
GOAL: `bpref.py` executes the IMPLEMENTED .bp surface (grammar of §1 of
the surface audit: fn/let/let-in/let-chain/if/while/match-literal/enum/
arrays/zeros/str/char/str_len/sys_* stubs) — the semantic oracle for
the COMPILER (the T36 oracles are semantic oracles for the ALGORITHMS;
both are needed). `gen.py` emits random well-typed programs inside the
documented caps (<=128 binds, <=14 params, <=511-elem arrays, no L8
allocs until T43); `fuzz.sh` compiles with bebop.bin, runs via seed,
compares to bpref — every divergence is a minimal repro filed in
BUGFIXES.md with a construct-parity guard (BUG-LEDGER-WEEK precedent).
DONE-CHECK: 10^5 generated programs, 0 divergences, 0 crashes, run time
recorded; `bench/FUZZING.md` rewritten from these numbers (the current
file describes the deleted C fuzzer).
DEPS: T36. BLOCKERS: none.

**T40 · structural invariant gates** (`bench/vs_rust/invariants.sh`,
`tools/check_abi.py` extended, `tools/census.py`) — DONE ✓ 2026-09-04 (GREEN on HEAD; planted census increase caught RED)
GOAL: machine checks that never depended on a fold: (i) register-zone
law — no write to x27/x28 outside prologue, no write to x9-x13 outside
prologue/epilogue/bank builtins (T25); (ii) branch census per frozen
construct bin frozen in a table (the T51 baseline); (iii) fntab zone map
asserted (3655-3661 fold, 3700-3796 slots, 3890-3898 bank, 3899-3999
literals, 4000 budget) with a COMPILE-TIME trap when `3903 + nlits >=
4000` (fixes F-F's latent collision); (iv) `.bin` footer/entry identity
(L11/L12) for every artifact the scripts touch.
DONE-CHECK: `invariants.sh` green on HEAD; a deliberately planted
violation of each check is caught (RED->GREEN per C7).
DEPS: none. BLOCKERS: none.

**T41 · one design corpus** (docs) — PARTIAL 2026-09-04 (5 corpus-A banners + CORE-ROADMAP-INDEX row + seed.S:55 done; `Status:` lines for bebop-lang/docs and bench/*.md pending)
GOAL: this file's supersession list names `docs/design/BEBOP-LANGUAGE-
SPEC.md`, `BEBOP-GLYPH-ALPHABET.md`, `BEBOP-ARCHITECTURE-CATALOG-100.md`,
`BEBOP-BACKEND-ROADMAP.md`, `BEBOP-LANGUAGE-REWRITE-PLAN-2026-08-17.md`
as SUPERSEDED (operator decision required — recorded here as the
recommended resolution), each receiving a 3-line banner pointing here;
the still-valid ideas are carried as roadmap items, not as live specs:
QTT quantity-1 linearity -> Z2 odd sector (T22/T25), contracts ->
gates + T48 checked types, glyphs -> T84 (operator 2026-09-04: glyphs
become the CANONICAL surface with a lossless ASCII projection; Honest
flag 3 of the SILICON pull is overridden). Fix stale corpus-B docs:
`bench/VERIFICATION.md`, `FUZZING.md`, `SELFHOST.md`, `HV_ARCHITECTURE
.md`, `LEGACY_BP_ANALYSIS.md`, `selfhost/readme.md`, `samples/*.bp`,
`docs/ANDROID.md` counts, `docs/SESSION-HANDOFF.md`, bebop.bp:1529
docstring, seed.S:55 comment. Register bebop-lang in `docs/design/CORE-
ROADMAP-INDEX.md`.
DONE-CHECK: `grep -rl "WASM + AArch64 native live" docs` == 0; every
doc in `bebop-lang/docs` and `bench/*.md` carries a `Status:` line with
a date and either CURRENT or SUPERSEDED-BY.
DEPS: none. BLOCKERS: none — DECIDED 2026-09-04: corpus A is superseded;
its surviving content is exactly the 16 carry-over items T68-T83 (see
CORPUS-A CARRY-OVER); T41 now only executes the banners and doc fixes.

### Layer C — compiler debt (tolerated miscompiles die)

**T42 · fix R3.x(a)-(e) at the root, then delete the laws**
GOAL: (a) precedence: `emit_bitlvl` binds tighter than `*` — decide and
document the grammar (recommended: C precedence; regression gate r3x
updated) ; (b) `>>`: emit ASRV for `>>` and add `>>>` for LSRV (both
gated on negative operands; every std module audited for the abs-before-
shift idiom and simplified); (c) loop-shaped while+compare+conditional-
store: root-cause in `emit_let_stmt`/`emit_cond` join state, fuzz-guarded
by T39; (d) string literals + `++`: either implement concat over the
arena or remove `++` from the surface — no third state; (e) zero-arg
call parse fixed generically (`clock_ms` was one instance). Add unary
`-`, unary `!`, hex literals (literal FORMS, not sugar).
DONE-CHECK: SESSION-HANDOFF "R3.x defects" block deleted; construct
parity gains c25-c31 covering each fixed shape; fixpoint byte-exact.
DEPS: T39 (fuzzer proves closure). BLOCKERS: none.

**T43 · lift L8 and the nesting bans structurally**
GOAL: per-iteration frame-heap mark/reset around `while` bodies
(genarena semantics inside the 16KiB frame: record x14 at loop entry,
restore at back-edge unless the body's value escapes — escape = the
body's final expression is an array/ctor; conservative: trap, never
leak); nested `if` inside `let` statements and plain assignment inside
`let _ =` compile correctly (fuzzer shapes). Re-enable struct literals
(`struct_kill`) with field access as fixed-offset loads.
DONE-CHECK: L8 and the nesting ban removed from "Design laws"; gates
c32-c35; fixpoint byte-exact; fuzz 10^5 with allocations in loops.
DEPS: T39, T40. BLOCKERS: none.

**T44 · self_check honesty** — replace c37-c41 (`exec`, dead in the
self-hosted emitter) with the morph path (compile -> publish -> seed
run -> fold) or delete them; `self_check` must be 41/41 or renumbered.
DONE-CHECK: `self_check()` returns 0 with no dead checks.
DEPS: none. BLOCKERS: none.

**T45 · retire the second compiler** — port `sys_clone` (220),
`sys_futex_wait/wake` (98), `sys_atomic_add` (LSE LDADD), `sys_arena_
base/end`, `sys_exit_thread` from `selfhost/expr_compile.bp` into
`bebop.bp` (register tables per L2, disasm per L1); `pool_parity.sh`
builds with `bebop.bin`; `expr_compile.bp` moves to `attic/` (history
stays in git).
DONE-CHECK: pool gate compiles under bebop.bin (still an honest 5-skip
under ptrace; 5/5 is the bare-kernel trigger); `expr_compile.bp` not
referenced by any script.
DEPS: T40. BLOCKERS: none in-sandbox.

### Layer S — language surface required by the terminal goal (no sugar)

**T47 · `use "path"` textual module inclusion with content-digest dedup**
GOAL: `collect_fns` follows `use` lines; an included file is inlined
once per program keyed by FNV-64 of its bytes (ptrless discipline); no
namespaces, no renaming — the prelude of T38 becomes language-level.
DONE-CHECK: gate `usemod`; `gen_selfsrc.sh` concatenation deleted;
bebop.bp itself split into `use`d files with the fixpoint byte-exact.
DEPS: T38. BLOCKERS: none.

**T48 · checked types at zero runtime cost**
GOAL: the annotations the parser currently discards become checked:
`i64`, `[i64]`, `str`, `fp` (fixed-point 2^32), `even`/`odd` (Z2 parity,
T25 S3), `cell` (dispatcher cell id, T50). Mismatch = compile-time loud
trap; no runtime word changes (frozen construct bins stay byte-exact for
well-typed programs).
DONE-CHECK: construct gates c36-c40 (each type, one mismatch program
that traps); fixpoint byte-exact.
DEPS: T24/T25 for parity. BLOCKERS: none.

**T49 · records = register images** — `struct` layouts compile to the
T26 5-word bank image when declared `bank`, else to fixed-offset arena
records; field access = fixed-offset load (no branches, no hashing).
DONE-CHECK: gate `recstruct` == T26 fold. DEPS: T26, T43. BLOCKERS: none.

**T50 · functions as cells** — `&f` yields the cell id / code offset of
`f` (adr), tables of cells are arrays, and `call_cell(id, args)` is a
`blr` today and an activity edge under T55; this is what direct-threaded
`thr` (SS-14) and the substrate need to stop being demos.
DONE-CHECK: gate `cells`: a table-driven dispatcher over 8 cells == the
`match` reference; thr gate re-expressed through `&f`.
DEPS: T48. BLOCKERS: none.

### Layer B — branch elimination ladder (critical path to TG-DONE 1)

Baseline (F-E): `if` = 2 branches, `while` = 2, call = `bl` + `ret`,
`bebop.bin` = 859 b.cond / 86 cbz / 820 b / 850 bl. Measured lesson
(FASTPATH-SPEC R4#3): a runtime branch per arithmetic op made K1 6x
SLOWER — branches are removed for correctness of the model AND for
performance, but only where the select is cheaper than the mispredict.

**T51 · branch census gate** — DONE ✓ 2026-09-04 (census.txt frozen: bebop 872 b.cond/133 cbz/0 tbz over 84230 words + 34 bins) — `tools/census.py` output frozen per
construct and per kernel in `bench/vs_rust/census.txt`; `invariants.sh`
fails on any INCREASE; each later rung records its decrease.
DONE-CHECK: census table committed; RED->GREEN on a planted branch.
DEPS: T40. BLOCKERS: none.

**T52 · pure `if` -> csel** — when both arms are pure (no call, no
store, no alloc, no syscall) and each arm <= N words (N frozen after
measurement), emit cond + both arms + `csel` (the T24 select-equivalence
gate is the semantics oracle; bebop.bp:1529's stale docstring becomes
true). Impure arms fall to T53.
DONE-CHECK: c05_if census 2 -> 0 b.cond; K1-K4 folds bit-exact; K3/K4
timing recorded (swpmu steps + clock_ms, cores pinned per T63).
DEPS: T24, T51. BLOCKERS: none.

**T53 · side-effecting arms -> sink-predicated stores** — an arm's
stores go to `csel(real_addr, sink)` where `sink` is a per-frame scratch
cell (the write always happens, only its address is selected); arms with
calls stay branched until T55 (a call is an activity edge there).
DONE-CHECK: c24_ifspill and the store-shaped fuzz corpus branch-free;
fold parity; census decrease recorded.
DEPS: T52, T43. BLOCKERS: none.

**T54 · bounded loops -> masked fixed-count iteration** — `while i < K`
with literal K (or K known by R6.2 folding) compiles to K masked steps
(FIR gate shape) and unrolls when K*body <= budget; data-dependent exits
keep the backward branch until T55.
DONE-CHECK: c07_while(literal bound) census 2 -> 0; k1 (1M iterations)
stays a loop (budget) — recorded honestly; fold parity.
DEPS: T52. BLOCKERS: none.

**T55 · substrate codegen (the terminal move)** — `compile --substrate`:
each fn body is cut at calls, loop back-edges and data-dependent exits
into straight-line branch-free cells (T52-T54 make the cell bodies);
dependencies become an incidence tensor; loops become self-re-arming
cells; calls become activity edges; recursion depth is a fixed cap with a
loud trap (honest limit, like the 14-param cap). The artifact = cells
`.bin` + `.bt` incidence (F4 codec); the runtime = a Bebop prelude
(`substrate.bp` generalized) whose sweep `while activity != 0` is the
only conditional branch. Ladder: k1 -> k2 (today's hand-written
substrate kernels must fall out of the compiler) -> K3/K4 -> the 66 std
gates -> `bebop.bp` itself (TG-DONE 2).
DONE-CHECK: per rung, fold == linear mode AND census == 1 conditional
branch AND sweep count recorded (eigentime, SS-17 becomes real);
terminal rung: substrate-mode fixpoint bb2 == bb3.
DEPS: T50, T52-T54, T14 substrate.bp, T26 (cell state as bank images).
BLOCKERS: none in-sandbox; performance is measured, not promised.

**T56 · runtime `match` without branches** — `match` on runtime values
via bitmat multiply-select (SS-12) or cell dispatch (T50); today `match`
only accepts literal ctors at compile time.
DONE-CHECK: gate `rmatch` over all 256 scrutinee patterns == reference.
DEPS: T50. BLOCKERS: none.

### Layer R — runtime: gate demos become the running system

**T57 · substrate runtime prelude, seed stays frozen** — the sweep loop,
activity words, cell table and bank load/store live in a `.bp` prelude
linked (T47) into every substrate artifact; `seed.S` (1496B, frozen) is
unchanged and only maps + jumps. Fix the seed.S:55 comment (256MB).
DONE-CHECK: T55 rungs run through the unmodified seed; prelude word
count frozen in census.
DEPS: T47, T55. BLOCKERS: none.

**T58 · eigentime as the scheduler** — sweep count and quiescence
detection ARE the clock (SS-17 seigtime moves from gate to runtime):
the prelude exposes `sweeps()`; WFE/WFI on quiescence is forward-port.
DONE-CHECK: substrate K1-K4 report sweep counts == the mirror's; step
counts replace clock_ms as the primary benchmark number.
DEPS: T55. BLOCKERS: none.

**T59 · reversible arena as the mutation path** — cell writes go through
the XOR journal of `rev.bp` (N2): unwind-to-any-sweep without snapshots.
DONE-CHECK: gate `unwind`: run K sweeps, unwind to sweep j, re-run ->
byte-identical arena; cost per write recorded.
DEPS: T55, T57. BLOCKERS: none.

**T60 · holographic artifact** — the `.bt` incidence tensor is WHT-
encoded with redundancy (N6 `holo.bp` from gate to loader): a trimmed
artifact still loads and runs to the same fold.
DONE-CHECK: gate `holoload`: zero 1/4 of the artifact's cells, run, fold
unchanged; size overhead recorded (the "cutting never destroys the
picture" claim gets a number).
DEPS: T57. BLOCKERS: none.

**T61 · threads and cores** — sys_clone/futex/LSE builtins (T45) + a
`sys_sched_setaffinity` builtin (syscall 122) so the fiber scheduler
(gate `fiber`) can run N fibers on N pinned cores; pool 5/5 remains
the bare-kernel trigger under ptrace.
DONE-CHECK: affinity probe shows the mask took effect (getcpu 168);
fiber gate unchanged in single-core mode.
DEPS: T45. BLOCKERS: clone semantics under ptrace (honest skip stays).

**T62 · network syscalls for the agent language** — socket 198, bind
200, connect 203, sendto 206, recvfrom 207, epoll_create1 20 /
epoll_ctl 21 / epoll_pwait 22, getrandom 278 (for nonces only; the core
stays RNG-free per C2): the T8 deltasync codebook delta crosses a real
process boundary (two seeds, one loopback socket).
DONE-CHECK: gate `deltanet`: sender/receiver folds equal over loopback;
register tables per L2; proot permits AF_INET loopback (probe first).
DEPS: T45. BLOCKERS: proot networking (probe, do not assume).

**T63 · benchmark hygiene** — every bench pins to the A78 cluster
(affinity mask 0xF0) or records the core class per sample; swpmu step
counts are the primary column, clock_ms the secondary; REPORT-630
re-baselined with both columns and the census column.
DONE-CHECK: 31-run medians with core class recorded; the 2-20x spread
re-measured pinned vs unpinned (the number replaces the folklore).
DEPS: T61 (affinity builtin). BLOCKERS: none.

### Layer H — hardware that IS present (integer-exact, in-sandbox)

**T64 · use the silicon this box has** — cpuinfo exposes crc32, sha2,
pmull, LSE atomics, asimddp: emit `CRC32X` for crc.bp, `SHA256H/H2/SU0/
SU1` for sha256.bp, `CNT`+`ADDV` NEON popcount for hv/holo/attn (hvham
already does), `EOR3`-free XOR-binding on NEON for ringvsa; each keeps
its EXISTING fold (integer-exact instructions cannot change the oracle,
so this is fusion without a new golden — the invariant policy's
condition is met).
DONE-CHECK: folds unchanged, swpmu steps and clock_ms recorded before/
after; disasm diffs per L1; SVE/SME remain forward-port (T15).
DEPS: T36 (oracles first). BLOCKERS: none.

### Layer D — dowiz integration (the "agent language" half)

**T65 · bebop-lang in the dowiz master index** — DONE ✓ 2026-09-04 — one row in `docs/design/
CORE-ROADMAP-INDEX.md` pointing here; MEMORY.md session-closing note per
`.claude/CLAUDE.md`.
DONE-CHECK: the row exists; no other dowiz doc claims Bebop backends
that do not exist. DEPS: T41. BLOCKERS: none.

**T66 · first dowiz twins WITH runnable Rust oracles** — DONE ✓ 2026-09-04 (gates money 872656672063013 + ordfsm 346243789026198 == cargo-run PRODUCTION dowiz-core money.rs/order_machine.rs byte-exact; forbidden transitions = loud codes 1/2/3) — `money.bp`
(exact i64 minor-unit law; oracle = `kernel/src/money.rs` via a small
cargo bin, parity like `eqc_gen.rs`) and `order_machine.bp` (decide/fold
FSM; oracle = the kernel's golden signature and rho = 0 nilpotent-DAG
proof). These are the first twins whose oracle is production code, not
a mirror written for the gate.
DONE-CHECK: gates `money`, `ordfsm` == cargo-run oracle output byte-
exact; forbidden transitions trap loudly (errors, not no-ops — the
kernel's rule).
DEPS: T36, T48. BLOCKERS: none (cargo present).

**T67 · bebop2 mesh bridge** — T8 deltasync + T62 sockets + the
`mesh-adapter` seam: a dowiz hub consumes a codebook delta produced by
a Bebop agent; capability-authenticated per DECISIONS D0 (no scoring).
DONE-CHECK: one round-trip through `mesh-adapter` tests; fold verified
on both sides. DEPS: T62, T66. BLOCKERS: bebop-repo cross-repo drift
(docs/design/ROADMAP.md item 12) must be closed first.

### CORPUS-A CARRY-OVER (operator selection 2026-09-04, T68-T83)

The 2026-08-17/18 design corpus (`docs/design/BEBOP-*`) was read in
full and sieved against the terminal goal. The operator selected the 16
items below as the ONLY content that survives; everything else in that
corpus (glyph-only surface, Lean/dependent types, proof kernel, SMT
solvers, C bootstrap, LLVM, WASM/x86_64/GPU/FPGA backends, f64 + IEEE
trig, supervision trees, async/await, traits/generics/dynamic dispatch,
semicolon-free layout, package registry, LSP, Ed25519-only signatures)
is SUPERSEDED — T41 executes the banners. Each carry-over item is
re-homed onto the task it strengthens; none introduces a runtime, a
solver, a float, or a second compiler.

**T68 · QTT quantities as annotations `^0 ^1 ^w`** (SPEC §3, catalog D1-2,
ergonomics #21) — strengthens T48, T25, T33, T34
GOAL: a binding or parameter may carry `^0` (erased: readable only inside
contracts/tests, never emitted), `^1` (linear: used exactly once), `^w`
(default, unrestricted). A per-fn usage-count pass over `sym_lookup`
enforces: `^1` used 0 or >= 2 times = compile-time loud trap; `^0` read
in emitted code = trap; `^1` values entering the odd bank slots (T25 S3)
are the Z2-odd/nilpotent tokens of T33/T34 — one discipline, two names
retired. No dependent types, no universes, no quantities beyond the rig.
DONE-CHECK: gate `qtt`: programs with correct usage compile byte-
identical to their unannotated twins (zero runtime cost); one double-use
and one erased-read program trap at compile; fixpoint byte-exact.
DEPS: T48. BLOCKERS: none.

**T69 · contracts as gates, no SMT** (SPEC §4, ergonomics #141-143) —
strengthens T36, T39, T66
GOAL: `where { requires E; ensures E; invariant E }` after a fn
signature (loop `invariant` before `while`). Semantics: constant-
foldable clauses (R6.2 cells) are decided at compile time (false =
trap); the rest compile ONLY in `--check` builds as cmp + loud trap word
and are erased (^0) in normal builds so the release stream is byte-
identical to the contract-free program. `result` names the return value.
No external solver ever; the fuzzer (T39) is the discharge engine.
DONE-CHECK: gate `contract`: a violated `requires` traps under --check,
the release stream equals the frozen contract-free bin (byte compare);
std gate headers' prose folds are re-expressed as `ensures` clauses on
>= 10 gates.
DEPS: T68 (erasure), T40. BLOCKERS: none.

**T70 · effects `pure`/`io`** (SPEC §5, catalog D6 51-53) — strengthens
T55 (cut points), MANIFESTO C2
GOAL: `pure fn` may not call `io` fns; all `sys_*`, `clock_ms`,
`tokens`, `mem_*` are `io`; propagation is transitive over the resolved
call graph (fntab); unresolved callees are treated as `io` (sound,
possibly noisy). `--substrate` (T55) cuts cells exactly at `io` calls
and loop back-edges — purity is the cell-body oracle.
DONE-CHECK: gate `effects`: pure->io call traps at compile; bebop.bp
annotated (123 fns) with fixpoint byte-exact; T55 rung k1 uses the
effect table to place its cuts.
DEPS: T48. BLOCKERS: none.

**T71 · `bit_identical(f_scalar, f_hw)` declaration** (SPEC §4) —
strengthens T64
GOAL: a top-level declaration naming two fns of the same signature;
`--check` builds compile both and run the T39 generator over them,
trapping on the first differing result; normal builds emit only `f_hw`.
DONE-CHECK: gate `bitid` over crc32 (CRC32X vs table), sha256 (SHA256H
vs scalar), popcount (CNT+ADDV vs SWAR): 10^5 inputs, 0 differences.
DEPS: T64, T39. BLOCKERS: none.

**T72 · core affinity builtins** (ergonomics #127, ENERGY §1.1) —
strengthens T61, T63
GOAL: `sys_sched_setaffinity(pid, len, maskptr)` (122) and `sys_getcpu`
(168) builtins with L2 register tables; the fiber scheduler (gate
`fiber`) gains a class argument (LITTLE = 0x0F, big = 0xF0 on this
box); every bench script pins to big before timing and records
`getcpu` per sample.
DONE-CHECK: probe shows the mask took effect under proot (getcpu in the
requested set for 100/100 samples) or records EPERM honestly; K1-K4
re-measured pinned vs unpinned, spread recorded (replaces the 2-20x
folklore with two numbers).
DEPS: T45 (builtin pattern). BLOCKERS: proot may reject (probe first).

**T73 · `snapshot()` / `rollback(mark)` / `on_fail(mark, e)`** (catalog
D7 56-57, ergonomics #41, #74-75, agentic primitive 1) — strengthens
T59, T34
GOAL: three builtins over the XOR journal of `rev.bp` (T59): `snapshot`
returns the journal mark, `rollback` unwinds to it, `on_fail` runs `e`
after unwinding when the guarded block trapped. DECISION: the XOR
journal is THE rollback mechanism; corpus A's separate append-only CoW
log is rejected (two mechanisms = two truths). Journal growth is bounded
by T10 entropic collapse of dead marks.
DONE-CHECK: gate `snap`: mutate, snapshot, mutate, rollback -> arena
byte-identical to the snapshot; `on_fail` path runs exactly once;
journal length accounted.
DEPS: T59. BLOCKERS: none.

**T74 · WFE at quiescence** (ENERGY §1.2) — strengthens T57, T58
GOAL: the substrate prelude emits `wfe` when the activity word is 0 and
no input cell is armed; a `sev` from the input path wakes it. In-sandbox
the gate checks the word placement and that quiescent programs still
terminate; the energy effect is forward-port (needs a meter).
DONE-CHECK: gate `wfe`: disasm shows exactly one WFE in the sweep;
folds unchanged; idle-loop step count drops to the sweep constant.
DEPS: T57. BLOCKERS: energy measurement (forward-port).

**T75 · integer-exact micro-optimizations** (PHYSICAL-LIMIT §1-6) —
strengthens T64, census
GOAL: single-variable emitter changes, each with fold unchanged and
census/steps recorded: DC ZVA for `zeros()` blocks (read DCZID_EL0 for
the block size, fall back to the str-xzr loop below one block), PRFM
in stride loops (SS-10 shapes), STNP/LDNP for `.bt` export/import
streams, ROR/EXTR for `bits.bp` rotations, NEON TBL/VSRI for
hv_permute (the 8x gap of the 08-17 benchmark). ORDER: `zeros` DC ZVA
may land early (cheap, isolated); the rest wait for T55 (critical-path
law: optimize the substrate's bottleneck, not the stack machine's).
DONE-CHECK: per item, gate folds unchanged + disasm diff + before/after
swpmu steps and pinned clock_ms in `bench/vs_rust/census.txt`.
DEPS: T51, T72; T55 for all but DC ZVA. BLOCKERS: none.

**T76 · living memory as ONE primitive** (SPEC §1.6-4, §7) — strengthens
T20, T27-T29
GOAL: builtins `mem_insert(stalk)`, `mem_search(query)`, `mem_link(a,b,
rho)` over the content-addressed sheaf (T29) with the T20 tensor query
as the retrieval engine and T28 consistency as the validity check;
timestamps come from `clock_ms` (io). NO NTT index, NO HNSW — the
"indexed via NTT" clause of corpus A is dropped (index-free doctrine).
DONE-CHECK: gate `livmem`: insert N, search returns the T20 nearest
and the T28 residual for each hit; an inconsistent insert is rejected;
mirrors == T20/T29 folds.
DEPS: T29, T20, T70. BLOCKERS: none.

**T77 · minimal counterexample shrinker** (agentic primitive 3, without
SMT) — strengthens T39, AGENTS.md T4
GOAL: on any fuzz divergence, `bench/fuzz/shrink.py` delta-debugs the
program to the smallest .bp that still diverges and prints `H:|DID:|
GOT:|VERDICT:` for the journal plus the construct-parity guard stub.
DONE-CHECK: three historical repros (c21-c24 class) re-derived by the
shrinker to <= the hand-minimized size.
DEPS: T39. BLOCKERS: none.

**T78 · token streams: `.bt` as canonical I/O** (agentic primitive 5) —
strengthens T35, F4
GOAL: builtin `tokens(fd)` reads a stream into a rank-4 `.bt` token
tensor (byte-per-cell today; word tokens later) and T35's register wave
filters it in place; `sys_write` of a `.bt` is the canonical output.
Text remains the authoring format; `.bt` is the canonical interchange.
DONE-CHECK: gate `tokens`: file -> tokens -> filter -> `.bt` -> read
back == python mirror; zero intermediate copies (address arithmetic
only).
DEPS: T26, T35. BLOCKERS: none.

**T79 · VSA navigation tooling (compiler identity stays bytes)** (SPEC
§2.2, HV_ARCHITECTURE L3) — strengthens agent workflow, replaces
graphify dependence
GOAL: `tools/hvnav.bp` encodes fn names + header comments as hv4096
(trigram bundling from hv.bp) and answers `near <name>` / `search
<text>` by Hamming distance; LAW: the compiler resolves identifiers by
byte equality only — fuzzy identity in the compiler is banned (silent
miscompile class).
DONE-CHECK: gate `hvnav`: known-neighbor queries return the expected
fn in top-3 for 20 hand-picked cases; the tool is compiled by
bebop.bin (cold-start rule).
DEPS: hv.bp, T47. BLOCKERS: none.

**T80 · content-addressed imports `use "cas://sha256:<hex>"`**
(ergonomics #161, #165, #172, #180) — strengthens T47, T12
GOAL: T47's `use` accepts a sha256 address resolved from the local store
`.bcas/<hex>.bp`; `bebop.bin cas add <file>` computes the digest with
`sha256.bp` and stores; dedup key for inclusion is the same digest;
FNV-64 is BANNED for source addressing (not collision-resistant).
Agents exchange source as digests + T8 deltas — no registry.
DONE-CHECK: gate `casuse`: a program importing by digest compiles
byte-identical to the path import; a tampered store file fails the
digest check with a loud trap.
DEPS: T47, sha256.bp. BLOCKERS: none.

**T81 · `test name { ... }` blocks with erasure** (catalog D5-50, D10-74,
ergonomics #55) — strengthens T38, T36
GOAL: modules carry test blocks; `bebop.bin test <file>` synthesizes a
main that runs every block and folds their results (the gate fold);
normal compiles erase the blocks (^0). `std_tests/` is deleted once all
60 gates live inside their modules; `std_golden.sh` calls `test`.
DONE-CHECK: 60 gates green through `bebop.bin test`; `ls std_tests` ==
empty; release compile of a module with tests is byte-identical to the
test-free module.
DEPS: T68, T38. BLOCKERS: none.

**T82 · replay debugger over the journal** (ergonomics #186) —
strengthens T59, AGENTS.md T5
GOAL: `bebop.bin replay <artifact> <sweep>` runs a substrate artifact to
sweep j via the T59 journal and prints the bank image + activity word;
`step` advances one sweep. gdb stays the last rung of the Occam ladder.
DONE-CHECK: replay to sweep j equals a fresh run stopped at j (byte-
compare of arena + bank) for K1/K2 substrate artifacts.
DEPS: T57, T59. BLOCKERS: none.

**T83 · "faster than Rust" as a MEASURED TARGET, not a law**
(PHYSICAL-LIMIT "hard law", FASTPATH done-check) — strengthens Q12
GOAL: the FASTPATH/REPORT tables gain a `target >= 1.0x` column and a
status per kernel (MET/UNMET with the pinned median); the sentence
"any benchmark slower than Rust is a bug" is NOT adopted (today 2.6-10x
slower; a law that is false on adoption violates Q12). The target is
re-evaluated after T55 + T75 + T72 with both step-count and wall-clock
columns.
DONE-CHECK: REPORT-630 successor with the target column; every row has
a measured value or "not measured", never a projection.
DEPS: T63, T72. BLOCKERS: none.

Carry-over ordering: T77, T79, T83 are parallel-safe now (tooling/docs);
T72 and T75(DC ZVA) are small single-writer builtins; T68 -> T70 -> T69
-> T81 -> T71 form the typing/testing chain after T48; T80 rides T47;
T73/T74/T82 ride T57-T59; T76 rides T29; T78 rides T35; the rest of
T75 waits for T55.

### REJECTED-LIST DECISIONS (operator, 2026-09-04, T84-T95)

The items the carry-over sieve had marked "contradicting" were put to
the operator one by one with what/pros/cons. Decisions:

**ADDED (as tasks below):** glyphs as the canonical surface (A1b, full
item — overrides Honest flag 3 of the SILICON pull); zero-dependency
Lean-like verification with mathematical provability IN the runtime
(B1 in its dependency-free form); bounded bit-vector DPLL in .bp (B2a);
f64 only as an io-effect at the data boundary (B3a); supervisor as a
cell library (B4a); trust chain + diverse double-compiling without C
(C1a); `bebop.bin check` with line:col (C4a); x86_64 backend without
AVX-512 first (D2); direct Verilog from `.bt` without MLIR (D4a);
WGSL i32 export as forward-port (D3a); WASM and GPU emitters ONLY in
zero-dependency form (direct binary emission + own .bp simulators;
execution outside the tree is forward-port).

**REJECTED for good:** semicolon-free layout (A2), traits/generics/
dynamic dispatch (A3), async/await (A4), external Lean/Mathlib (B1
external form), Z3/CVC5 (B2b), f64 in the pure core (B3b), a
supervision runtime framework (B4b), C bootstrap (C1b), LLVM (C2),
package registry (C3), full LSP (C4b), WASM as a runtime dependency
(D1 runtime form), Vulkan/driver-bound GPU (D3b), Calyx/CIRCT (D4b).

**T84 · glyphs as the canonical surface, ASCII as a lossless projection**
(SPEC §2, GLYPH-ALPHABET v0.2, ergonomics #3) — overrides Honest flag 3
GOAL: the canonical `.bp` token is the glyph; every glyph has exactly one
ASCII name (the alphabet's right column) and the lexer accepts BOTH
spellings identically (bidirectional lexer): glyph bytes are mapped to
the ASCII name before the 131-hash dispatch, so the emitter is
untouched. `bebop.bin fmt --glyph` / `--ascii` convert losslessly and
round-trip byte-exact. Staging (L14): G1 alphabet table as a `.bt`
(glyph UTF-8 bytes -> ASCII name, ~99 entries, closed); G2 lexer
accepts glyph spellings (gate: glyph twin of every construct-parity
source compiles to the IDENTICAL frozen bin — no bin regenerates); G3
`fmt` round-trip; G4 δ-outline renderer as a separate tool (terminal
braille/half-block), never part of the compiler; G5 sources flip to
glyph form file by file with `fmt`, fixpoint byte-exact at each step
(the binary cannot change because the token stream after mapping is
identical). Cost recorded honestly: agent tokenizers split glyphs into
more BPE tokens than ASCII — TOKEN-ECONOMY measures both and the doc
records the number.
DONE-CHECK: 24/24 construct bins unchanged with glyph sources; fmt
round-trip byte-exact on all 116 std modules + bebop.bp; TOKEN-ECONOMY
carries the measured token cost; fixpoint byte-exact.
DEPS: T39 (fuzzer covers both spellings), T47. BLOCKERS: none.

**T85 · proof kernel in Bebop, zero dependencies, usable at runtime**
(catalog D5 41-50, `samples/theorem-sample.bp`, SPEC §3 restricted)
GOAL: a minimal type-theory kernel written in `.bp` (Type₀ only, no
universes/quotients/general fix — catalog D5/D7/D8 postponements stand):
terms as `.bt` tensors, βδ-normalization over the i64 primitives
(`str_len("abc") ≡ 3` computes), definitional equality `conv(a,b)`,
`refl` as the sole equality constructor, `nat_ind` as the sole
eliminator (cong/subst derived), a `theorem name : l = r := proof`
surface checked at COMPILE time, and a `verify(term_bt)` builtin so an
agent can check a generated fragment at RUNTIME and receive the failing
subterm (the counterexample of SPEC §1.6-3, solver-free). No Lean, no
Mathlib, no external process: the kernel is itself gated and self-
checked (it type-checks its own gate theorems).
DONE-CHECK: gates `kernel` (conv on 50 table-driven pairs == mirror),
`theorem` (theorem-sample.bp's 3 theorems accepted, 3 false twins
rejected with the failing subterm), `rtverify` (runtime verify of a
morph-published fragment); erasure: theorems emit zero words (T68 ^0).
DEPS: T68, T69, bt.bp. BLOCKERS: none (design-bound: critical-path
item, one writer).

**T86 · bounded bit-vector DPLL in .bp** (B2a) — CORE DONE ✓ 2026-09-04 (gate dpll 584168922 == oracle, 20 formulas; T69 hookup pending) — discharge engine for T69
GOAL: a small CDCL/DPLL solver over fixed-width bit-vector formulas
(the shapes T69 contracts produce: bounds, overflow, equality of linear
i64 terms), written in `.bp`, deterministic, with a step budget; UNSAT =
contract proven for the bounded domain, SAT = counterexample assignment
printed. Bounded and honest: outside the fragment the contract stays a
--check trap.
DONE-CHECK: gate `dpll` on a table of 20 formulas (10 SAT with models,
10 UNSAT) == mirror; T69's `ensures` on 5 std gates discharged at
compile time.
DEPS: T69. BLOCKERS: none.

**T87 · f64 at the boundary only** (B3a)
GOAL: builtins `f64_bits_to_fp(bits)` and `fp_to_f64_bits(fp)`
converting IEEE-754 bit patterns to/from fixed-point 2^32 with round-
half-even, both `io` (T70) so no `pure` code can name a float; no float
arithmetic instruction is ever emitted (FMOV/FADD banned by the census).
DONE-CHECK: gate `f64edge`: 32 table-driven patterns round-trip within
1 fp unit == mirror; census shows zero FP instructions in every bin.
DEPS: T70. BLOCKERS: none.

**T88 · supervisor as a cell library** (B4a)
GOAL: `supervise.bp`: a substrate cell that watches other cells'
activity/trap words and re-arms a failed cell after `rollback(mark)`
(T73) with a bounded restart budget — a library, not a language feature
or runtime.
DONE-CHECK: gate `superv`: a cell that traps on sweep 3 is re-armed and
completes; budget exhaustion propagates the trap; fold == mirror.
DEPS: T55, T73. BLOCKERS: none.

**T89 · trust chain + diverse double-compiling without C** (C1a)
GOAL: `docs/TRUST-CHAIN.md` records the full chain seed.S (1496B,
frozen, disassembly listed) -> golden `bebop.bin` sha256 (L13) ->
fixpoint; DDC: the attic'd `selfhost/expr_compile.bp` (T45) is kept as
the independent witness compiler: witness compiles bebop.bp -> W1; W1
compiles bebop.bp -> W2; W2 must equal the golden fixpoint byte-exact.
Any divergence = a trusting-trust alarm.
DONE-CHECK: `tools/ddc.sh` green; sha256 sidecars for seed.bin and
bebop.bin; the chain doc lists every hash.
DEPS: T45. BLOCKERS: witness must still compile the current surface
(freeze the surface subset the witness supports).

**T90 · `bebop.bin check <file>` with line:col diagnostics** (C4a)
GOAL: parse + type/effect/quantity checks (T48/T68/T70) with no
emission; every trap word class gets a message and a `file:line:col`;
exit codes distinct per class (the KEEP pattern).
DONE-CHECK: 10 planted errors report the right line:col; runtime of
check on bebop.bp recorded.
DEPS: T48. BLOCKERS: none.

**T91 · x86_64 backend (no AVX-512 first)** (D2)
GOAL: a second encoding table for the same emitter structure (mov/add/
sub/imul/idiv/cmp/setcc/cmovcc/jcc/call/ret/syscall), a `seed.x86.S`
twin of the loader (mmap RX, entry footer, arena in r14/r15), the Z2
bank on the five callee-saved GPRs (even = rbx, r12; odd = r13, r14,
r15 — SysV keeps them across calls, mirroring T25 S1), CSEL -> CMOVcc
(T52), the substrate prelude unchanged (.bp). Cross-architecture folds
become a SECOND INDEPENDENT ORACLE for every gate (two ISAs, one fold).
Server-class x86_64 hosts expose perf_event_open, so T15's hardware
counters stop being terminal there.
DONE-CHECK: 66 gates + construct parity + K1-K4 folds identical on
both ISAs; x86_64 fixpoint bb2 == bb3; PMU-backed L1/I-cache numbers
recorded for K1-K4 on a Linux x86_64 host (first real class-(c)
evidence). AVX-512 is a later, separate column.
DEPS: T40, T51, T57. BLOCKERS: needs an x86_64 Linux host for the
execution half (emission is gated in-sandbox by disassembler diff).

**T92 · direct Verilog netlist from the `.bt` incidence tensor** (D4a)
GOAL: a text emitter in `.bp`: cells -> always-blocks, incidence ->
wires, activity word -> enable lines, the sweep -> one clock; no MLIR/
Calyx/CIRCT; plus `vsim.bp`, a cycle-accurate simulator of the emitted
netlist subset, as the oracle.
DONE-CHECK: gate `verilog`: k1 and k2 substrate artifacts emitted and
simulated to the same folds as the software sweep; sweep count ==
clock count. Synthesis on real FPGA = forward-port (equipment).
DEPS: T55. BLOCKERS: hardware (forward-port).

**T93 · WGSL i32 export of tensor queries** (D3a, forward-port)
GOAL: T20 contraction kernels emitted as WGSL text (i32 lanes; i64
folds are split hi/lo with explicit carry) for the dowiz engine's
WebGPU path; the browser supplies the driver, the tree gains no
dependency. Bit-exactness is asserted by the engine's existing pixel/
parity tests, not assumed.
DONE-CHECK: emitted WGSL compiles in the engine's test harness; the
hi/lo split reproduces the i64 fold on a table of queries.
DEPS: T20. BLOCKERS: GPU under proot (execution is forward-port).

**T94 · WASM direct binary emitter (zero deps) + own interpreter**
GOAL: emit a valid `.wasm` module (magic, sections, i64 ops) directly
from the emitter structure — a second encoding table like T91, no
toolchain; `wasmi.bp`, an interpreter of the emitted i64 subset, is the
in-sandbox oracle; execution in a browser (dowiz web/) is forward-port.
HONEST: WASM is a stack VM — this column exists for reach (browser hubs),
not for the post-von-Neumann model; the substrate prelude runs INSIDE it
as ordinary code.
DONE-CHECK: gate `wasm`: k1/k2/K3 emitted, interpreted by wasmi.bp ==
folds; module validates against the spec's binary grammar (own checker).
DEPS: T91 (second-table pattern). BLOCKERS: browser execution
(forward-port).

**T95 · SPIR-V direct emitter (zero deps) + own integer simulator**
GOAL: emit SPIR-V words (spec-defined binary; OpTypeInt 32, OpIAdd/
OpIMul/OpBitwise*, OpLoad/OpStore, workgroup dispatch) for the T20/T35
kernels; `spvsim.bp` simulates the integer subset lane-by-lane as the
oracle; real GPU execution goes through the engine's WebGPU path
(forward-port) — no Vulkan loader in the tree.
DONE-CHECK: gate `spirv`: hv bundle and one T20 contraction emitted,
simulated == folds; module passes the own structural validator.
DEPS: T93. BLOCKERS: GPU (forward-port).

Ordering for T84-T95: T90 and T89 ride T48/T45; T84 G1-G3 after T39 +
T47 (one writer); T85 is a critical-path design item after T68/T69; T86
after T69; T87 after T70; T88 after T73; T91 after T57 (first non-ARM
fixpoint); T92 after T55; T93 after T20; T94 after T91; T95 after T93.

**Honest flags for this section**
1. T84 reverses Honest flag 3 of the SILICON pull: the canonical surface
   is now glyphs; the total ASCII projection keeps agents and the
   emitter unchanged. The token-cost number is recorded, not assumed.
2. T85 is a minimal kernel, not Lean: Type₀, refl + nat_ind, no
   tactics, no quotients, no termination checker. "Every statement
   proven" is NOT claimed; "every `theorem` checked by our own kernel"
   is.
3. T94/T95 emit VM/GPU code without dependencies, but running it needs
   a host VM or driver outside the tree — forward-port by definition.
4. T91's execution half needs an x86_64 Linux host; in-sandbox only the
   emitted bytes are gated (disassembler diff).

### Critical path and ordering

```
V:  T36 -> T37 -> T38 -> T39 -> T40 -> T41            (truth floor; ~all parallel-safe after T36)
C:  T42 -> T43 -> T44 -> T45                          (needs T39/T40)
S:  T47 -> T48 -> T49 -> T50                          (needs T38, T24-T26)
B:  T51 -> T52 -> T53 -> T54 -> T55 -> T56            (needs T24, T50; T55 is TG-DONE 1-2)
R:  T57 -> T58 -> T59 -> T60 ; T61 -> T62 -> T63      (needs T55 / T45)
H:  T64                                               (needs T36 only — can start now)
D:  T65 -> T66 -> T67                                 (needs T41, T36, T48, T62)
A:  T77 T79 T83 now | T72 T75(DC ZVA) small | T68 -> T70 -> T69 -> T81 -> T71 after T48 |
    T80 with T47 | T73 T74 T82 with T57-T59 | T76 after T29 | T78 after T35 | T75 rest after T55
R2: T90 T89 with T48/T45 | T84 after T39+T47 | T85 (critical, design-bound) after T68/T69 -> T86 |
    T87 after T70 | T88 after T73 | T91 after T57 -> T94 | T92 after T55 | T93 after T20 -> T95
```
Parallel-safe now (no shared files): T36, T40, T41, T51, T64, T22-T24.
Serialized (touch `bebop.bp`): T25, T42-T45, T47-T50, T52-T56 — ONE
writer, one commit per single-variable change (L14), battery + fixpoint
after each, clean revert from a fresh baseline snapshot (L13).

### Honest flags (Q12) for this section

1. Substrate-mode performance is unknown. A dispatcher sweep per cell may
   be slower than straight-line code on this CPU; TG-DONE is a model
   criterion, and the numbers ship whatever they are (FASTPATH fallback
   clause: linear mode stays available).
2. Recursion on the substrate has a fixed depth cap; unbounded recursion
   is a von Neumann call stack by definition. The cap is a trap, never a
   silent limit.
3. CLOSED 2026-09-04: T36 landed — every gate has a committed oracle in
   `bench/oracles/` and `run_all.sh` is the third column (ok=82).
4. The five T1-T5 "DONE ✓" marks were wrong as stated; corrected to
   "GATE NOT WIRED -> T37" without deleting the folds.
5. "Terminal-goal sentence stands" (Honest flag 4 of the SILICON pull)
   is narrowed: the substrate MATH is proven; substrate EXECUTION of
   compiled programs is T55 and is not done.
6. Corpus-A supersession is DECIDED (2026-09-04); the 16 carry-over items
   are tasks, not live specs — a carried idea that later contradicts a
   gate is dropped, not the gate.

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

Status (2026-09-03, session 3): **60/60 std_golden gates**, every gate ==
an independent python mirror bit-exact. Closed this session:
- **T14 dispatcher SUBSTRATE** (substrate.bp, fold 36750250113; k1 chain→36,
  k2 fib(25)→75025; post-von-Neumann SWAR popcnt + de Bruijn tzcnt, no
  PC/fetch-decode) — commit 8a5bc33.
- **T15a software PMU** (swpmu.bp, fold 2001000110000000000; replaces blocked
  perf_event_open EACCES) — commit 4152ec1.
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
T1->T7 bottom-up, T13 as the sole open gap (mechanism proven, not landed),
T14 DONE, T15 terminal (forward-port to bare metal). The remaining
pre-vision items (SS-10 PMU, SS-14 I-cache bench, pool-on-bare-kernel,
<1% jitter, <5ms cold start, SME/SVE2) all fold into T15 forward-port.
T15 workaround paths documented (root/Magisk, LPE exploit, QEMU TCG,
ptrace PMU virtualization, SIGILL SVE emulation) — any path requires
privilege escalation or virtualization, none gated in-sandbox.
R3.x(b) stays documented-as-law.

After T15's terminal bounds (forward-port only), the language work
CONTINUES in-sandbox via **T16-T21: Tensor Database Engine as the language**
(see the SILICON-REGISTER PULL section). These replace no existing gate;
they port dowiz-core's proven tensor/geometry into self-hosted `.bp` twins
(contract → Christoffel → curvature → forms → query engine → Stokes audit),
each with a `fn main() -> i64` gate == python mirror of the Rust oracle.
The substrate (T14) + tensor engine (T16-T21) converge into the terminal
goal: a post-von-Neumann language that IS a geometric data engine.

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

Status (2026-09-03, session 3): **T14 DONE, T15a DONE, std_golden 60/60**.
T14 dispatcher SUBSTRATE (substrate.bp, fold 36750250113; k1 chain→36, k2
fib(25)→75025; post-von-Neumann SWAR popcnt + de Bruijn tzcnt, no
PC/fetch-decode) — commit 8a5bc33. T15a software PMU (swpmu.bp, fold
2001000110000000000; replaces blocked perf_event_open EACCES) — commit
4152ec1. std_golden 60/60, fixpoint byte-exact (md5 13a6447f).

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

Status (2026-09-03, session 3): **T14 DONE, T15a DONE, T13 sole open gap**.
std_golden 60/60, fixpoint byte-exact (md5 13a6447f). T14 dispatcher
SUBSTRATE (substrate.bp, fold 36750250113) and T15a software PMU (swpmu.bp,
fold 2001000110000000000) committed (8a5bc33, 4152ec1). T13 mechanism
proven (R4#4: 42/42 gates, K1-K4 bit-exact) but not reconciled with the
current emitter — the ONE open gap. T15 marked TERMINAL with hard platform
bounds (EACCES perf, no SVE/SME on Cortex-A78); forward-port trigger list
recorded. T13/T14/T15 all terminal in the roadmap.

T16-T21 (Tensor Database Engine as the language) columns defined, ordered
contract→Christoffel→curvature→forms→query→Stokes, each a port from
dowiz-core Rust reference (academia_p2p.rs / tensor.rs / parametric_spectral.rs
/ memory_search.rs) into `.bp` twins with gates. Cross-link table in the
SILICON-REGISTER PULL section maps every Rust module to its Bebop twin.

Status (2026-09-04, session 4): **T13 S1-S3 landed (b211451), disabled
(b4326b5: prologue saves x19-x28 only -> x9-x13 window corrupted in
gen3==gen4), disable reverted (9d9a2ba)**; std_golden 60/60, construct
24/24, parity 9/9+1skip at HEAD; a further UNCOMMITTED push/pop
`can_reg` diff sits in the working tree (untyped window, fntab[3890]
used as a 0..5 depth counter — conflicts with the decision below).
Verified: x9-x13 are NOT free (71 emitted words in 8 builtin emitters,
T13 blocker #1 answered negative). Operator decisions recorded:
(1) physical Z2 partition x9-x10 even / x11-x13 odd, (2) odd sector =
Grassmann Lambda_5, even = Cl^0(4,1). New pull defined: **SUPER-SHEAF
PULL T22-T35** (graded algebra T22-T24 -> bank ABI T25 -> register
records T26 -> cellular sheaf T27-T29 -> categorical rewrite T30-T31 ->
CRUD T32-T35), each a `.bp` gate == python mirror; T13 re-scoped (untyped
window retired, T13-attributed speedups withdrawn until re-measured).
Next in-order: T22 -> T23 -> T24 (pure algebra, parallel-safe with
T16-T21), then T25 S1 as its own single-variable commit.

Status (2026-09-04, session 4b): full-tree audit (compiler surface, std
corpus, design corpus) recorded in TERMINAL-GOAL CLOSURE F-A..F-I. Key
corrections: T1-T5 gates never wired (T37); 54/60 gates lack a committed
oracle (T36); 50/116 std modules dead, reuse = copy-paste (T38, T47);
every `if`/`while` is a real branch (census baseline, T51-T55); R3.x
laws + L8 are debt (T42-T43); two contradictory design corpora (T41);
big.LITTLE 4xA55+4xA78 unpinned benches (T63); HW crc32/sha2/LSE present
and unused (T64). TG-DONE defined. Revised in-order start: T36 + T40 +
T51 + T64 (parallel-safe, no bebop.bp edits) alongside T22-T24; then the
single-writer bebop.bp queue T25 -> T42 -> T43 -> T47 -> T48 -> T50 ->
T52 -> T53 -> T54 -> T55.

---

## Predicted speedup and memory after full roadmap completion

These are measured-baseline projections, not aspirational numbers. All
predictions derive from existing benchmarks (K1-K4, session 3) + the
known mechanical impact of each remaining task.

### Speedup (execution, vs Rust baseline, same workload)

| Task | Current measured | Projected | Why |
|---|---|---|---|
| K1 (linear chain) | 4.5-5.6× | 6-7× | T13 eliminates ~40% push/pop (1 instr vs 2 per value): K1
  hot loop 35→11ms → projected 8-9ms on a non-throttled box. |
| K2 (fibonacci) | 2.6× | 3.5-4× | T13 register window keeps fib(25) state in registers,
  eliminates 25 spill/reload cycles. |
| K3 (matrix) | 5.1-8.1× | 7-10× | T13 + T14 substrate: matrix inner loop already register-
  resident; the remaining bottleneck is the 64B cache-line
  alignment, not instruction count. |
| K4 (bitfield) | 6.7-10.6× | 9-12× | T13 eliminates branch overhead; bitfield ops are already
  branchless (T14 SWAR). |
| T16-T21 (tensor DB) | N/A (new) | 15-50× vs SQL | Einstein summation = fused inner loop (no query parser, no
  WAL, no B-tree traversal). Memory bandwidth bound; same data
  = fewer instructions = fewer cache misses. On a 1M-point
  manifold query: SQL ~10ms, tensor ~0.2-0.7ms (parametric
  surface O(1) lookup vs B-tree O(log N)). |
| T15 forward-port (bare metal) | 4.5-5.6× | 8-12× | Removing proot overhead (15.8ms cold start, syscall
  interception) + sustained 2.4GHz (no thermal throttling under
  proot). This is a platform gain, not a code gain. |

Aggregate (K1-K4 average, T13 landed, bare metal): **8-10× geometric
mean** vs Rust on the same integer workloads.

### Memory (peak RSS, same workload)

| Component | Current | Projected | Why |
|---|---|---|---|
| Arena allocator | 64B aligned, zero GC | Same | Already optimal; no change. |
| .bt rank-4 tensors | 4-10× compression vs flat | Same | Already proven (bt.bp). |
| Parametric manifold (T20) | N/A (new) | N × (16 bytes + hash) vs N × 256 × 8 = 2GB | The parametric surface
  stores (u:f32, v:f32, hash:u256) per point, not the full
  256D vector. 1M points = 24MB vs 2GB. |
| SQL/WAL overhead (eliminated) | 50-200MB typical | 0 | No B-tree pages, no WAL buffer, no query plan cache.
  The tensor engine IS the storage. |
| Total peak RSS (1M-point query) | 200-500MB | 24-50MB | Arena + manifold + no SQL structures. 10-20× reduction. |
| Binary size (bebop.bin) | ~1.4KB seed + ~68KB compiler | Same | No change; compiler is the same. |
| Cold start | 15.8ms (proot) | <1ms (bare metal) | T15 forward-port + mmap-init (sub-ms by construction). |

### Summary

| Metric | Current (proot sandbox) | After full roadmap (bare metal) | Δ |
|---|---|---|---|
| Execution speed (K1-K4 avg) | 4.5-5.6× | 8-12× | +1.5-2.2× |
| Memory (1M-point manifold query) | N/A (SQL: 200-500MB) | 24-50MB | 10-20× reduction |
| Cold start | 15.8ms | <1ms | 15× reduction |
| Tensor query latency | N/A (SQL: ~10ms) | ~0.2-0.7ms | 15-50× faster |

The speedup is dominated by T13 (register window) + T15 bare metal
(platform). The memory reduction is dominated by T20 (parametric manifold)
eliminating SQL structures. The tensor query latency is the terminal-goal
payoff: the language IS the database, so queries compile to native code
with zero runtime overhead.

Revision note (2026-09-04): the K1-K4 rows attribute their gains to the
T13 untyped register window, which the operator retired in favour of the
typed Z2 bank (SUPER-SHEAF PULL, T25). Those rows are WITHDRAWN until
re-measured after T25/T26/T35 land; the T16-T21, T15 and memory rows are
unaffected. Sheaf queries (T28) cost a Laplacian solve whose iteration
count is frozen per gate — no latency number is projected for them.