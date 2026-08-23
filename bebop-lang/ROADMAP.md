# Bebop Language — Roadmap: from self-host expr kernel to full language

Status: drafted 2026-08-20. Point A closed today; Point B planned for a few days
of heavy swarm parallelization. Every milestone = commit + push to origin/main.

---

## Point A — Self-host expr kernel closes the loop  [CLOSE TO READY — selfcompile HALTS]

The .bp compiler (`selfhost/expr_compile.bp`, 71 fns) parses a tiny language and
emits real AArch64 machine code, executes it via the `exec` builtin, and
`selfcompile` produces a deterministic checksum of the machine code for its own
source: **123745140208**.

Status: `self_check` → 0 ✅; `strict` PASS; `check` 0 errors. BUT `selfcompile`
**HALTS >550s** (root cause not yet bisected — could be proot env or real regression).

Swarms completed (8 swarms, today):
1. codegen for array literals / arr[i] / arr[i]=v / string literals /
   str_len / char / let...in  (swarm-0, encoding-verified against objdump)
2. native.c reference parity for TERM_ARRAY / TERM_STR / *_GET / *_SET /
   STR_LEN / STR_CHAR  (swarm-1)
3. arena/pool hardening + exec edge-case bounds  (swarm-2)
4. fuzzer (fuzz-selfhost, 300k inputs, 0 crashes, 0 hangs)  (swarm-3)
5. throughput benchmark (typecheck/self_check/codegen words-per-sec)  (swarm-4)
6. bench/SELFHOST.md status report  (swarm-5, DONE — honest gaps documented)
7. AArch64 disassembler/verifier tool  (swarm-6)
8. selftest_exec.bp execution regression suite (63 fns)  (swarm-7)

Gate (partial):
- `check`/`strict`/`self_check`/`make test` green ✅
- self-compiled image bit-matches native reference for the arith/conditional
  subset ✅
- string/array builtins: self-consistent but NOT independently cross-checked
  against native.c for bit-identity — PRIMARY OPEN WORK ITEM
- `selfcompile` HALTS on full source — root cause open (proot or regression)

## Point B — Full language front-end + backends, self-hosted  [~3 days]

---

## Point B — Full language front-end + backends, self-hosted  [~3 days]

The real compiler today lives in C (`native/src/*.c`, 82 modules / 25k LOC).
The .bp port (`selfhost/*.bp`) is fragmentary: lexer/parser/typecheck/eval are
real, but aarch64/wasm/emitter/codegen are 5-55 LOC stubs. Point B ports the C
compiler into self-hosted .bp and adds the missing backends.

Design laws (inviolable, every swarm): branchless Σ k·(k==N)·expr, no_std,
O(n), atomic/lock-free, vector-first (NEON, scalars as fallback), hypervectors
everywhere possible, living memory.

### B1 — Front-end unification (Day 1)

Goal: one pipeline `lexer.bp → parser.bp → typecheck.bp → codegen.bp →
aarch64.bp` that compiles the full glyphic+VSA surface, mirroring the C front
end (`lexer.c`, `parser.c`, `expr.c`, `qtt.c` typechecker).

Swarms (parallel):
- B1a: full lexer.bp parity with C lexer.c (tokens, strings, comments, glyphs,
  escape sequences, line/col tracking, source_map).
- B1b: full parser.bp parity with C parser.c (modules, fn decls, struct/enum
  decls, ADTs, dependent Pi types, generics, match, let/let-in, while, arrays,
  lambda `\x.`, field access, precedence).
- B1c: full typecheck.bp parity with C qtt.c infer/check (QTT quantities
  0/1/ω, linearity, Pi/Σ, universes, generics, dependent types, conv/norm
  kernel). Also fix typecheck.bp strict failure (the 1/135 file).
- B1d: unified AST → codegen.bp IR (lowering to a small typed IR the backends
  consume; single source of truth, no duplicated lowering).

### B2 — Backends (Day 2)

Swarms (parallel):
- B2a: aarch64.bp full native backend = parity with C native.c + codegen.c
  (the entire construct set: arithmetic, comparison, branch if, calls/bl,
  closures, structs, enums, match, arrays, strings, syscalls, alloc, float).
- B2b: wasm.bp backend emitting valid WebAssembly (MVP: i32/i64, locals,
  control flow, memory ops; validate with a wasm parser).
- B2c: NEON/SIMD vector-first backend (micro-BLAS: restrict+FMA+4-accum+
  transpose+tiling+unroll+64B align; vir_umulh2 synthesis for missing
  vector umulh).
- B2d: GPU/FPGA first slice — vir.c (vector IR) lowering: emit a VIR
  representation of the hot kernels (NTT, hypervector, living-memory) as the
  single source for GPU/FPGA later; document the emit contract.

### B3 — Verification + closure (Day 3)

Swarms (parallel):
- B3a: self-compile the FULL compiler (not just expr) — the composed pipeline
  compiles its own source end-to-end; checksum stable + bit-matches native.
- B3b: full test suite — port native self-tests to .bp selftests; 135/135
  typecheck clean + strict PASS; fuzz ~1M inputs no crash.
- B3c: honest benchmark vs C (compile throughput, codegen words/sec, output
  binary correctness on the dowiz algorithm kernels).
- B3d: docs — update bench/SELFHOST.md with B state, architecture, ABI, and
  remaining gaps; final commit + push.

---

## Definition of done (Point B)

- `selfhost/` contains a complete, self-hosting compiler (lexer+parser+typecheck
  +codegen+aarch64+wasm) that compiles the full glyphic+VSA+QTT surface.
- The compiler compiles itself (checksum-stable, bit-matches the C reference).
- 135/135 .bp files typecheck + strict clean.
- Backends: aarch64 (native parity), wasm (validated), NEON (vector-first),
  GPU/FPGA (VIR slice + emit contract).
- fuzz (1M) + benchmark + docs all green; every milestone committed+pushed.

---

## Stage B status update — 2026-08-23 (PLAN_B.md supersedes on conflict)

Executed via PLAN_B.md swarms; every item below is verified by command output
recorded in git history (see `git log --oneline` + PLAN_B STATUS lines).

DONE:
- Phase 0 gate: 144/144 .bp files strict+check green (SWEEP-B3-3.md; was
  0/136 check + 17 strict fails before emit_call fix).
- B1 front-end slices: lexer.bp (full bp_lex parity), parser.bp (item-level),
  expr_parser.bp (10/10 goldens), typecheck green.
- B2 backend slices (exec-verified goldens): aarch64.bp 4/4, aarch64_data.bp
  4/4, wasm.bp MVP module, wasm_data.bp memory ops, vir.bp NEON ADD/SUB 2D +
  MUL 4S, gpu_fpga.bp (WGSL kernels + Verilog ADD_2D skeleton, self_check 0).
- B2-7 selfhost-bench: honest R=10 medians (compile_program ~1KB ≈ 89 ms,
  ~11.4 KB/s; typecheck suite 0.19 ms); .bp work-counter harness.
- B2-8 parity: 20/20 constructs byte-identical to live C goldens; real failure
  accumulation (old dead-store bug fixed).
- B3-1: full_compiler.bp stable deterministic selfcompile.
- Evaluator root causes fixed en route: 255-char string truncation + chr()
  aliasing (string arena); cross-invocation env aliasing under in_while
  (boundary markers).

OPEN (honest gaps):
- Backends are SLICES (4-construct goldens), not full-language parity with
  native.c — the Definition-of-Done bar above still stands.
- B3-2: only lexer/parser/expr_parser self-tests ported; most native modules
  lack .bp twins.
- B3-4 fuzz: ~630k documented inputs toward 1M; fuzz_selfhost has ~0.28%
  bounded-slow inputs (2s watchdog) with root cause NOT yet localized (not the
  while-cap). Division lowering defect in C codegen.c (default->ADD) unfixed.
- Lexer has no string token: '{'/'}' inside .bp string literals break parsing;
  codegen text emits braces via chr(123)/chr(125) until BP_TOK_STR lands.
