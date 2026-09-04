# Legacy & Opportunity Audit — .bp world ONLY (target: zero C)

Status: 2026-09-04 SUPERSEDED-BY ROADMAP.md (counts are pre-zero-C; the live corpus audit is ROADMAP "TERMINAL-GOAL CLOSURE" F-A..F-I and T38)

Scope: everything that must survive C elimination — selfhost/*.bp,
bench/vs_rust/kernels/*.bp, and the QUALITY OF CODE the .bp compiler
emits. C-side findings excluded except where they mark a hole that .bp
must fill post-exit. ANALYSIS ONLY — no changes made.

## 0. Topology today

- 11,898 lines of .bp across 72 files; **only expr_compile.bp (3,298 L)
  is in the live build graph** (compilewords/parity/bench reference it).
- **8,576 lines (72%) are dormant**: 70 files never loaded by anything —
  earlier-generation compilers and experiment slices (full_compiler.bp,
  parity.bp, selftest_exec.bp, expr_parser.bp, parser.bp, lexer.bp,
  aarch64.bp+aarch64_data.bp [alt backend B2], wasm.bp+wasm_data.bp,
  gpu_fpga.bp, ...).
- The C runner (exec_words) is the only non-.bp piece the plan still
  needs short-term; long-term it lowers to a tiny loader or raw
  kernel-executed flat binary (already our artifact format!).

## 1. Dead weight (hygiene, zero runtime risk)

| item | size | action |
|---|---|---|
| 70 dormant selfhost files | 8,576 L | archive to /legacy or delete |
| parity.bp alone | 1,169 L, 44×zeros(4096) superseded by shell parity_driver (44 programs live there now) | archive |
| full_compiler.bp vs expr_compile.bp | duplicate generation: em/em_prologue/em_epilogue/read_ident/is_alpha/self_check all redefined | archive |
| fp_expr_stepX-style leftovers | done partly (5 fns removed earlier) | sweep again |

Win: repo focus; compilemany/prelude surfaces stop seeing ghosts;
zero perf effect (files never loaded) — pure hygiene + honest map.

## 2. Emitted-code legacies (biggest REAL performance items)

### 2.1 Legacy stack-machine tier (K4/K2 gap)
Any expression containing an ARRAY or a CALL bails from the fpC
register allocator to the push/pop stack machine (str/ldr [sp] pairs,
2 words per value movement, values round-trip memory).
- Measured: K4 arith-chain 1.6× slower than Rust; K2 fib 3.4–4.2×
  slower (recursion = calls ⇒ always legacy).
- Replace with: fast-path coverage for (a) array load/store as
  base+offset vregs, (b) calls with args in registers (ABI x0–x7
  already used by fpC call path — extend to nested/array contexts).
- Expected: array kernels close most of the gap → ~1.3–1.6× on K4;
  fib improves additionally via (2.3).

### 2.2 Fixed-size prologue/epilogue (artifact bloat)
Every function unconditionally emits save/restore of x19..x28
(10 callee-saved = 5 stp + 5 ldp words) + fixed 16384-byte frame.
OPT-G1 patches unused pairs to NOPs but THE WORDS STAY IN THE STREAM.
- k1.bin = 140 B of which ~17 words (68 B) are prologue/epilogue
  bookkeeping ≈ 49% of the artifact!
- Replace with: pre-scan body for bound-count & spill need BEFORE
  emitting prologue (symbol count is known from `let` scan), emit
  exactly the needed pairs, frame sized from max push depth.
- Expected: k1 → ~90–100 B; whole 256-byte club gets headroom;
  I-cache footprint of every fn shrinks.

### 2.3 Recursion without tail-call optimization (K2)
fib's self-calls are non-tail, but the general mechanism is absent:
no sibling-call recognition (`return f(...)` shape → b instead of bl
after popping frame).
- Replace with: sibcall pattern when call is the entire result
  expression of a fn (already detectable: result expr == call).
- Expected: deep-recursion benches stop paying frame-per-level when
  shaped as tails; combined with 2.1 register-args, K2 gap narrows to
  ≤1.5×.

## 3. Compiler-self legacies (selfcompile = 73 s today)

| legacy | cost | replacement | expected |
|---|---|---|---|
| sym_lookup linear scan per identifier occurrence | O(n) × thousands lookups/pass | hash→slot index in stab (name%251 probe) | O(1); selfcompile −15–25% |
| str_len(s) O(n) re-evaluated (44 call sites, many in loops over same string) | quadratic-ish in bodies | cache len in cell once per fn entry | −5–10% |
| skip_ws called per token char-walk (77 sites) | hot | fuse into read_ident/tokenizer pass | −5–8% |
| two parallel let-emitters (emit_let_in legacy + OPT-D fast @1375) diverged historically (source of the seq/compound bug family) | correctness + maintenance | ONE parameterized emitter | removes a whole bug class |
| tree-walk interpreter for compile itself | dominant 73 s | closure-conversion/threaded eval, or per-fn parse-tree memoization | 1.5–2× compiler speed |

## 4. True parallelism (post-C, via TERM_SYSCALL clone/futex — primitive EXISTS)

Today: ZERO parallelism in the .bp world. pool.c (C) has a ready
persistent-worker pool with parallel_for and is consumed by 12 C files —
after C exit its role must be reborn in .bp/syscalls. Best first
targets, embarrassingly parallel:

| workload | decomposition | expected |
|---|---|---|
| compilemany N files | one compiler instance per worker (state isolated per process today; per-thread needs arena split) | ÷ cores on cold builds |
| k7 nearest-key search | hamming per key independent | ÷ cores (NEON already vectorizes within key) |
| hv_stream window encoding | windows independent until bundle stage | ÷ cores |
| diff_fuzz / parity iterations | independent processes already; trivially scriptable parallel today | free wall-clock |

Note: TRUE shared-memory threads inside one JIT program require the
runner to expose clone/futex syscalls to emitted code — TERM_SYSCALL
exists; design = spawn workers pointing at the same flat-binary page,
work-splitting via arena counters (lock-free, single-writer slots).

## 5. Memory footprints (.bp-relevant)

| item | today | better |
|---|---|---|
| afv_arena (C static) | 8 M slots ×16 B = 128 MB VAS, touched lazily | post-C: .bp bump arena in the SAME mmap the runner already provides (x27/x28) — identical semantics, no C static |
| stab literal arrays | 128×i64 per site (fine) | keep |
| zeros(4096)×44 in parity.bp | dormant file | dies with §1 |
| exec_words runner arena | 64 MB mmap lazy | keep (flat-binary loader keeps it) |
| artifact format | decimal text .full AND raw .bin exist | standardize on .bin (4 bytes/word, no text) — 387 B k1 text → 140 B real |

## 6. Priority order (impact × confidence)

1. **P0 §1 archive dormants** — 72% line reduction, zero risk.
2. **P0 §2.2 upfront-sized prologue** — artifacts −30–50%, helps the
   256-byte club immediately; moderate risk (layout math), gates cover.
3. **P0 §2.1 fast-path arrays+calls** — largest steady-state perf win;
   staged: arrays-in-vregs first, then call args.
4. **P1 §3 symtab/str_len/fused ws** — selfcompile 73 s → ~55 s est.
5. **P1 single let-emitter merge** — kills the seq/compound bug family
   permanently.
6. **P2 §4 parallel compilemany + k7** — after §3 arena isolation.
7. **P2 sibcall** — K2-class shapes.
8. **Standardize .bin artifacts everywhere** — drop decimal .full from
   default paths (keep parser for compat).

Everything above is replacement-by-better, not rewrite: the fast path,
SWAR popcount, NEON builtins, arena contract, flat binaries and the
compile-once cache are already the new generation — this audit marks
what still carries the old generation on its back.
