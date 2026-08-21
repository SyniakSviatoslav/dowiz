## 10. STATE SNAPSHOT — one-look current position (as of 2026-08-21)

Snapshot table — read this first; everything below is the detailed derivation.

- Toolchain: alive. `./build/bebopc run ../selfhost/expr_compile.bp self_check` → 0.
- Self-compile HALTS: `selfcompile expr_compile.bp` hangs >550s. Root cause:
  seed input + tiny corpus (270 lines, 0 constructs) + no tactic/judgement
  opcode table → deterministic infinite eval loop. Fixable. BLOCKED.
- C reference: 84 .c files, 78 .h headers, ~810 fn defs (heuristic grep).
- .bp corpus: 136 files (61 top-level + 73 std + 2 samples), ~675 fn decls.
  Largest: expr_compile.bp (71 fn), selftest_exec.bp (63 fn), wasm.bp (21 fn),
  codegen.bp (23 fn), typecheck.bp (24 fn), parser.bp (18 fn), lexer.bp (10 fn).
- Strict sweep: expr_compile.bp PASS. typecheck.bp FAIL (1/136). Full sweep not run.
- Fuzzing: 300k inputs, 0 crashes/hangs/signal-aborts. Gap to 1M target.
  Current fuzz.c covers lexer+parser+AST destructor ONLY.
- Verification: 3 machine-checked theorems (all refl). 0 enforced contracts.
  Verifier NOT wired. Coverage ~0.4% proven, 0.0% auto-verified.
- Working .bp backends: NONE at full parity yet. expr_compile.bp (71 fn)
  single-expression → AArch64 + exec-tested. aarch64.bp (4 fn) stub. wasm.bp
  (21 fn) partial. codegen.bp (23 fn) skeleton.
- Not started backends: vir.bp (NEON), gpu_fpga.bp (GPU/FPGA contract) — files
  do NOT exist yet (PLAN_B §B2-5/B2-6 spec only).

Phases:
- Phase 0 (gate, BLOCKING): fix selfcompile hang (P0.1), fix typecheck.bp strict
  (P0.2), full 136-file sweep (P0.3).
- Phase 1 (B1 front-end, PLAN_B B1-1..B1-8): lexer/parser/typecheck/codegen/in-
  fra/pipeline → full C parity + strict pass + end-to-end compile.
- Phase 2 (B2 backends, PLAN_B B2-1..B2-8): aarch64 (4 fn → full), wasm (21 fn
  → full), vir (new), gpu_fpga (new), bench, parity.
- Phase 3 (B3 closure): full self-compile (stable checksum, bit-match C), all
  self-tests ported, fuzz extended toward 1M, docs regenerated, release pushed.

This plan supersedes ROADMAP.md / PLAN_B.md / ROADMAP_SELFHOST.md on conflict.
