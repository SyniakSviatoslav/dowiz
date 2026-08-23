## 10. STATE SNAPSHOT — one-look current position (as of 2026-08-21)

Snapshot table — read this first; everything below is the detailed derivation.

STATE AS OF 2026-08-23 (supersedes the 2026-08-21 snapshot; evidence = git log +
SWEEP-B3-3.md + PLAN_B STATUS lines):

- Toolchain: alive. `./native/build/bebopc version` → `bebopc 0.1.0 (native C bootstrap)`.
- Corpus: **137 .bp files** (64 top-level + 73 std). Strict sweep: **137/137 PASS**,
  check sweep: **137/137 PASS** (SWEEP-B3-3.md, 2026-08-22; commit 55e01c0).
  The old emit_call blocker and 17 strict failures are FIXED.
- B1 front-end: lexer.bp / parser.bp / expr_parser.bp parity slices DONE, typecheck green.
- B2 backends (slices, exec-verified goldens): aarch64.bp 4/4 + aarch64_data.bp 4/4;
  wasm.bp MVP (valid module, interpreter-executed); wasm_data.bp memory ops;
  vir.bp NEON ADD/SUB 2D + MUL 4S 3/3; parity.bp 4/4 constructs byte-identical to C.
- B3-1: full_compiler.bp — stable deterministic selfcompile (commit b363534).
- Remaining: B2-6 gpu_fpga.bp (file absent), B2-7 selfhost-bench (absent),
  B2-8 parity full corpus (~20; only 4 done), B3-2 remaining native-self-test ports
  (lexer/parser only so far), B3-4 fuzz 300k→1M + typecheck/codegen/backends coverage,
  B3-5 real-runtime wasm validation, B3-6 NEON correctness+perf table, B3-7 docs,
  B3-8 release gate.
- Fuzzing: 300,000 inputs, 0 crashes, 0 hangs, 0 signal-aborts. Gap to 1M target.
  Current fuzz.c covers lexer+parser+AST destructor ONLY.
- Verification: 3 machine-checked theorems (all refl). 0 enforced contracts.
  Verifier NOT wired. Coverage ~0.4% proven, 0.0% auto-verified.

Phases:
- Phase 0 (gate): DONE — emit_call fixed; 137/137 strict+check green (SWEEP-B3-3).
- Phase 1 (B1 front-end, PLAN_B B1-1..B1-8): DONE — lexer/parser/expr_parser parity
  slices + typecheck green.
- Phase 2 (B2 backends, PLAN_B B2-1..B2-8): aarch64+wasm+vir slices done; REMAINING
  B2-6 gpu_fpga (new), B2-7 selfhost-bench (new), B2-8 full 20-construct corpus.
- Phase 3 (B3 closure): B3-1 stable selfcompile done; REMAINING B3-2 remaining ports,
  B3-4 fuzz→1M, B3-5 wasm runtime validation, B3-6 NEON verify, B3-7 docs,
  B3-8 release gate.

This plan supersedes ROADMAP.md / PLAN_B.md / ROADMAP_SELFHOST.md on conflict.
