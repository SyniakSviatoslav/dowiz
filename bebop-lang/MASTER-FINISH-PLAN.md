# BEBOP — MASTER FINISH PLAN (one artifact, full scope)

Updated: 2026-08-21 · Owner: bebop-lang workstream · Status: ACTIVE execution driver.

**Precedence (D8):** newest approved decision outranks older. This doc supersedes
`ROADMAP.md` / `PLAN_B.md` / `ROADMAP_SELFHOST.md` wherever they conflict; those remain
detailed spec sheets referenced by ID (B1-x / B2-x batch contracts live in PLAN_B.md).
Operator ruling 2026-08-21: the full language spec wishlist (former "5.1.x") is
**committed scope**, not backlog. It is organized below as Waves W1–W13.

---

## 0. Ground truth (verified live 2026-08-21 — commands + outputs)

| Claim | Evidence |
|---|---|
| Toolchain alive | `/lib/ld-linux-aarch64.so.1 ./build/bebopc run ../selfhost/expr_compile.bp self_check` → `0` |
| Self-host compile loop | **REGRESSED/UNKNOWN**: `bebopc selfcompile expr_compile.bp` hangs >550 s (SELFHOST.md claims checksum `123745140208`; commit d6b4b25 closed it). Root-cause = Phase 0 gate. |
| C reference size | ~495 fn defs in `native/src/*.c`, 481 sigs in 78 `.h` headers |
| .bp corpus | 134 files (61 top + 73 std), 675 `fn` decls; many are thin stubs (<1 KB) |
| Formal verification | 3 machine-checked theorems; 0 enforced requires/ensures; auto-verifier NOT wired into pipeline; `verifier_prove` placeholder; `qtt_prove_induction` stub (`bench/VERIFICATION.md`) |
| Fuzzing | 300k inputs, 0 crashes/hangs, PASS (`bench/FUZZING.md`) |
| Benchmarks | 12 result dirs r1–r12; mean-only reporting, no σ/p95/p99 |
| Working backends (.bp) | ONLY `expr_compile.bp` (single-expression → AArch64, exec-tested). `aarch64.bp`=5 LOC, `wasm.bp`=opcode consts, `codegen.bp`=toy IR — all stubs |

Living memory: `/root/.hermes/memories/MEMORY.md` (⚠ AGENTS.md's
`/root/.claude/projects/-root-dowiz/memory/` does not exist in this env — hermes file is canonical).

## 0.1 Environment laws (binding on every wave)

- Host: aarch64 linux, proot/f2fs — run binaries via `/lib/ld-linux-aarch64.so.1` when direct exec misbehaves.
- Bebop authoring: NO hex literals; reassign needs prior `let x=v`; array mutation wrapped `(let _ = a[i]=v in 0)`; no idents containing `if(`/`if `; BRANCHLESS LAW Σ k·(k==N)*expr in hot loops; real branches only for if/else & while; modules <~25 fns (split!); every module exports `self_check()->i64` = 0 on pass, including checksum AND exec tests.
- `exec(words,count,arg0)->i64`: mmap+call `long fn(long x0)` — one i64 arg only; multi-operand shims take a pointer to a stack block built via movz/movk inside emitted code.
- Zero external deps (C bootstrap: gcc + libc only). No LLVM/Rust in the compiler path.
- Verifier ≠ implementer (different agent/model). Commit only own files (Autopilot concurrent).

---

## 1. Dependency graph (re-derived)

```
P0 baseline gate ──► P1 B1 front-end unification ──► P2 B2 backends ──► P3 B3 self-host closure
        │                                                   ▲                    │
        └── IR contract (codegen.bp) is part of P2 but consumed by P1 output ◄─────┘
P3 ──► Waves W1..W13 (full language completion; W-waves parallelize after P3,
       except W6-stdlib/W9-tooling pieces that can start once P2 wasm+a64 land)
```

Hard edges: backends consume the unified IR (B1-6). Stdlib waves need data-plane
backends (B2-2/B2-4). Contracts/SMT need parser hooks (W2 after B1-2). Everything else
parallel-safe at file granularity.

---

## 2. Phase 0 — Baseline gate (NOW)

Goal: restore and pin a green toolchain before any new code.

| # | Task | Done-check (falsifiable) |
|---|---|---|
| P0.1 | Root-cause `selfcompile` hang (bg probe running; suspects: proot f2fs fd issue, OOM-thrash on 83 KB source × pool resets, infinite loop in two-pass layout for >N fns) | `selfcompile expr_compile.bp` prints `123745140208` ≤120 s wall |
| P0.2 | Pin run-protocol: all CI/local invocations use ld-linux wrapper script `tools/bp.sh` | `tools/bp.sh check ../selfhost/expr_compile.bp` exit 0 |
| P0.3 | Re-run strict + full test sweep, record numbers in bench/SELFHOST.md header | `strict` PASS count recorded; `run ... self_check` = 0 for expr_compile + selftest_exec |

## 3. Phase 1 — B1 front-end unification (PLAN_B B1-1..B1-8)

One .bp front-end whose output feeds the shared IR. Swarms per PLAN_B.md §B1.
Done-check: `check` + `strict` PASS on all touched .bp; parity vs C lexer/parser/typecheck
on golden corpus ≥135/135; `compiler_main.bp` drives lex→parse→type→IR end-to-end on samples/.

## 4. Phase 2 — B2 backends (PLAN_B B2-1..B2-8) — CURRENT EXECUTION TARGET

| Batch | Module(s) | Scope | Done-check |
|---|---|---|---|
| P2.a | `codegen.bp` | REAL flat quad IR contract (decimal opcodes, arrays of i64), documented encoding | self_check=0; IR round-trip tests |
| P2.b | `aarch64.bp` (+`aarch64_data.bp`) | Full native backend: prologue/epilogue, stack-machine ALU, cmp/cset, branch-if + bl two-pass layout, while loops, struct/enum/array/string via x14 arena, syscalls | emitted code executed via `exec` bridge: arithmetic/calls/loops/data each return expected value; bit-parity vs native.c encodings on golden set |
| P2.c | `wasm.bp` (+`wasm_data.bp`) | Valid WASM MVP: types/func/export/code sections, LEB128, i32/i64 const+arith, locals, control flow; then memory+data segments | structural validator (own .bp code) accepts module; C `codegen.c` cross-check on golden programs |
| P2.d | `vir.bp` | NEON vector-first ops (ADD/SUB/MUL 2D·4S, FADD/FSUB/FMUL 2D, LD1/ST1), umulh2 synthesis | exec-bridge runs hand-encoded NEON shims; results match scalar reference |
| P2.e | `gpu_fpga.bp` | VIR lowering slice → compute/Calyx contract (config not fork, M5) | golden VIR program lowers to both targets' IR deterministically |
| P2.f | `bench_compile.bp` + `bench_selfhost.c` | compile-time + runtime benches with p50/p95/p99, σ over ≥10 runs | report generated, committed under bench/ |
| P2.g | `parity.bp` | bit-match checker aarch64(.bp) vs native.c outputs | 0 mismatches on golden corpus |

## 5. Phase 3 — B3 closure

Full-language self-compile: compiler_main.bp compiles the whole selfhost/ tree;
all 79 MODS self-tests ported/green; fuzz 1M inputs; docs updated; release tag.
Done-check: `selfcompile` over full tree prints stable checksum twice (reproducible);
strict sweep 135+/135+; SELFHOST.md regenerated from live run.

---

## 6. Waves W1–W13 — FULL LANGUAGE COMPLETION (committed scope)

Each wave: deliverables → concrete modules; DoD = falsifiable command + verifier sign-off.
Waves start after P3 unless noted; intra-wave items parallelize as swarms.

### W1 Types & numeric tower (spec 5.1.1, 5.1.18, 5.1.28)
uint/u8/u32/u64 family; v8 SIMD vector type; tribool (three-state logic, no bare bool at
boundaries); varint; uint256/512 (limb-based); complex; ration; SO3 rotations;
ndarray/tensor (rank+shape); z_axis/point/vector/quaternion geometry; range;
option/result; continuation/coroutine CPS types; effect types.
Modules: `std/types/*.bp` + typecheck.bp extensions + backend lowering rules.
DoD: per-type self_check incl. exec tests; typecheck rejects ill-formed samples (negative tests committed).

### W2 Contracts & formal verification (5.1.2, 5.1.10)
requires/ensures parsed post-parser pre-typecheck; forall/exists; transforms
(`a in [0,255] => result in [0,65025]`); loop invariants; SMT integration.
DECART REQUIRED before adopting solver (Z3 vs CVC5 vs Alt-Ergo vs own branchless
decision-procedure core): candidates×criteria table + probe; zero-dep law pushes toward
embedding a small own solver first, external solver as optional adapter.
Wire auto-verifier into pipeline before codegen; replace `verifier_prove` placeholder;
real induction for `qtt_prove_induction`. Lean 4: type-soundness + memory-safety +
compiler-correctness (partial) proofs; coverage metrics (% fns with contracts, % theorems).
DoD: `bebopc verify file.bp` gates compilation; counterexample reported on seeded-bug probe; coverage % published in bench/VERIFICATION.md.

### W3 Macros & metaprogramming (5.1.3, 5.1.25)
Hygienic macros; pattern-matching macros; template instantiation (monomorphization);
AST-transformation macros; derive (Debug/Clone/PartialEq/Eq/Hash).
DoD: derive generates compiling code for 5 traits on sample structs; hygiene probe (capture attempt) fails to compile.

### W4 Concurrency (5.1.4, 5.1.26, 5.1.30)
pthread-like threads; SPSC/MPMC channels; Mutex/RwLock; atomics (load/store/CAS/fetch_add/sub);
Barrier/Condvar; async/await (poll/register); stackful coroutines; parallel iterators
(zip/map/filter/reduce). Branchless+atomic law applies.
DoD: SPSC stress 10M msgs zero-loss (Q4 dead-letter counters); CAS contention bench included in W13 harness.

### W5 Backends & portability (5.1.5, 5.1.14)
x86_64 native (port existing byte-verified `x86_64.c` encoder → .bp); ARM32; RISC-V;
PowerPC; MIPS; WASM SIMD; JS/emscripten target; Windows (MSVC/MinGW ABI notes); Android NDK; iOS.
Each target = config entry (M5 capability, never hard-coded fork) + parity.bp golden set.
DoD: hello+i64-arithmetic golden binary per target validated by available emulator/validator; absent-emulator targets ship structural validation only (named absence).

### W6 Standard library (5.1.6, 5.1.21, 5.1.27, 5.1.31–33)
io; fs; net (TCP/UDP/DNS); os (env/args/exit); time; rand (PRNG+CSPRNG);
hash (SHA2/SHA3/BLAKE3 — KAT-gated real impls only, Q-security); crypto (AES-GCM,
X25519, Ed25519 — real KAT vectors, never fake); compress (deflate/gzip); regex;
json+xml; http client/server; async runtime; collections (hashmap/btree/deque/ringbuffer);
iter adapters; option/result/error; format; math_ext (Gamma/Bessel/Zeta); stats;
linalg (micro-BLAS parity); fft (NTT heritage); signal (filters/convolution);
polynomial; bignum/bigint/rationals.
DoD: per-module self_check + KAT suites for hash/crypto pass; json round-trip golden; http fetch against loopback server.

### W7 Compiler optimizations (5.1.8)
Constant folding; DCE; inlining; loop unrolling; CFG construction; SSA; register
allocation (graph coloring); instruction scheduling; peephole; LTO; PGO.
Order: folding+DCE+peephole first (cheap, branchless-friendly), SSA+regalloc after IR stabilizes.
DoD: opt passes gated by flag; golden programs show ≤ same runtime, identical semantics (parity.bp); no pass breaks exec tests.

### W8 Runtime (5.1.9, 5.1.29)
Optional GC modes (refcount / mark-sweep / generational — selectable, arena default);
stack unwinding; panic handler; backtraces; CPU/mem profiler; execution trace.
DoD: panic path prints function+line via source_map; GC mode toggle survives stress test without leak growth (RSS measured).

### W9 Tooling (5.1.7, 5.1.36)
LSP (hover/goto-def/completion/rename); debugger (breakpoints/step/watch/memory inspector);
package manager; formatter; linter; test framework (unit+property); bench framework;
doc generator; cross-compiler driver; FFI bindings generator.
DoD: LSP speaks initialize+hover over stdlib sample (scripted client test); formatter idempotent on selfhost tree (`fmt fmt == fmt`).

### W10 Build & deploy (5.1.11, 5.1.38, 5.1.39)
CMake glue; pkg-config; cross-compilation profiles; static/dynamic linking toggles;
install/uninstall targets; CI (GitHub Actions) running P0 gates + strict sweep + fuzz smoke; deployment recipes.
DoD: fresh-clone → `make && tools/bp.sh check …` green in CI log; reproducible-build check (two builds, identical checksum).

### W11 Testing & QA hardening (5.1.12, 5.1.37, 5.1.45)
Unit/integration suites (.bp + C); property-based (QuickCheck-style generators in .bp);
continuous fuzzing (AFL++-style harness extension of fuzz.c, target 1M+); coverage
(gcov/lcov for C; own instrumenting counter for .bp interpreter); mutation testing
(own mutator first — DECART before adopting mutmeister); stress/regression corpora.
DoD: coverage % published; mutation score ≥ threshold set after first honest measurement; regression corpus auto-runs in CI.

### W12 Benchmarks & performance evidence (paste §2 — committed)
Statistical harness: p50/p95/p99 + σ, ≥30 reps (extend bench_selfhost.c; hyperfine optional adapter — DECART).
Suites: CoreMark-PRO (license check), STREAM, LMbench-latency equivalents, CLBG set, Dhrystone/Whetstone.
Concurrency: Amdahl scalability 1..N cores, spawn cost, context-switch, SPSC/MPMC under load, async overhead.
Memory: peak RSS vs Rust, alloc rate, per-object overhead, binary size, startup, stack/heap tradeoff.
Energy: joules/op via Power module, idle PSCI states.
Compile-time: build speed vs Rust same-project, incremental, .text size, reproducible builds.
DoD: bench/FINAL-REPORT.md regenerated from live runs; every number carries n, σ, p95/p99, machine tag.

### W13 Docs, ergonomics, security, debugging UX (5.1.13, 5.1.15–17, 5.1.34–36, 5.1.43–44, 49–238 cleanup)
Language reference (formal grammar+semantics); stdlib docs; tutorial; examples;
design rationale; migration guide; FAQ. Pattern matching UX; informative errors with context;
/// doc comments + doctest. Sandboxing (capability-based); FFI safety checks;
constant-time crypto discipline; side-channel resistance notes; zeroization helpers.
trace builtin; terse output mode; stack traces; breakpoints/step/watch/memory inspector (with W9 debugger).
Note: paste sections 5.1.49–5.1.238 are degenerate filler ("Види моделей" repeated) — their
intent is absorbed here; no separate artifacts will be created for them. ⚠ CORRECTED.
DoD: docs build scripted from repo; every stdlib module has doc header + example; error-message golden tests.

---

## 7. Execution order & swarm map

1. NOW: P0 (single-threaded, root-cause discipline) → P2.a IR contract.
2. Then swarm burst A (parallel-safe): P2.b aarch64 · P2.c wasm · P2.d vir (independent files, shared IR frozen by P2.a).
3. Burst B: P2.e gpu_fpga · P2.f bench · P2.g parity (consume A outputs).
4. P1 front-end swarms (B1-1..8) can run parallel to burst A AFTER IR contract freezes (they target different files).
5. P3 closure single-threaded integration + external verifier agent.
6. Waves: W7+W2-infrastructure first (they harden the compiler itself), then W1/W6/W4 (language surface),
   W5/W9/W10 (reach), W8/W11/W12/W13 (evidence+UX) — multiple bursts, verifier rotation each.

Every burst: plans pushed before code; results written back to living memory with provenance;
different-agent verification; commit only own files.

## 8. Two-question doubt check (mandatory ritual, answered now)

Least confident (un-investigated gaps):
1. selfcompile hang root cause — could be env (proot) or a real regression from swarm-A commits; not yet bisected.
2. True state of B1 fragments (lexer/parser/typecheck .bp parity vs C) — counts known, behavioral parity unmeasured.
3. Whether `exec` bridge suffices for multi-fn backend tests (4096-word cap, single x0 arg) — design constraint unproven for large programs.
4. wasm.bp validator-vs-browser gap — no independent WASM validator in-tree yet.
5. Lean 4 toolchain availability in this env for W2 proofs — unverified.
6. CoreMark/CLBG licensing + availability offline — may force substitutes (named absence).
7. Timeline realism of W1–W13 — no estimates attached yet (deliberate: sequence first, estimate per-burst).

Biggest thing missing: **a pinned, executable definition of "the language is finished"** —
this plan's answer is P3 done-checks + per-wave DoDs above; if any DoD proves unfalsifiable
in practice, it gets rewritten BEFORE its wave starts, not after.

## 9. DECART ledger (required before each adoption)

| Adoption | Status |
|---|---|
| Z3/CVC5/Alt-Ergo (W2) | DECART pending — own-solver-first bias due to zero-dep law |
| hyperfine (W12) | optional adapter; own harness primary |
| gcov/lcov (W11) | C-side standard, low risk; .bp coverage must be own tool |
| mutmeister (W11) | DECART pending — own mutator bias |
| CoreMark/CLBG/Dhrystone (W12) | license+availability check required; substitutes named if blocked |
| AFL++ (W11) | extend existing fuzz.c first; adopt only if extension insufficient |
