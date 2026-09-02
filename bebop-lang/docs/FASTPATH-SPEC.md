# SPEC — R6 Fast-Path Tier (register-aware emitter, "faster than Rust")

Status: NOT STARTED (blocked on R6.0 root-cause). This card replaces the
failed R4/R5 attempts with the exact failure modes recorded so the next
session does not repeat them.

## Goal
Close the 4.4–13.7× arithmetic gap vs Rust (bench/vs_rust/REPORT.md) by making
the emitter register-aware. Done-check (honest, falsifiable): in-process
clock_ms harness (same shape as REPORT.md's), K1/K4 kernels ≥ 1.0× vs the
recorded Rust medians (5.37/6.35 ms), all 42 gates + parity + construct +
fixpoint green, kernels bit-exact.

## What was already tried and WHY it failed (journal 1788288234, this session)
1. **R4 model-in-fntab virtual slots** (push/pop keep values in x9–x13,
   flush-on-bl): the model cells (fntab[3800..3801]) were READ correctly but
   WRITES silently did not land — the very first pop already saw slot 0.
   Root cause NEVER isolated (disasm of the compiled push showed correct
   str [x2,x1,lsl#3]). ABANDONED.
2. **R4 post-pass peephole** (rewrite [sub,str][ldr,add] pairs into movs in
   the FINAL word stream, branch-target marking): layout-sensitive SIGSEGV
   of the self-compile at ~1/4 of the stream, position moved with any code
   change. Found + fixed a real bug (cbz imm19 decode masked 14 bits) —
   not sufficient. ABANDONED.
3. **R4 micro-cancel with runtime branch** (emitter NOPs the right operand's
   trailing push and emits a runtime b.eq to skip the matching ldr/add):
   CORRECT but 2–8× SLOWER — store-to-load forwarding makes the sp-relative
   ldr/str nearly free, and one extra mispredictable branch per arithmetic
   op costs more than it saves. REVERTED (measurement: k1 5.6→36ms,
   k4 7→61ms in-process).
4. **R4 register-window, compile-time leaf check** (walk back ≤4 leaf words
   from the right push; [mov x9,x0] for the left, movs for the pops, NO
   runtime branches): semantically CORRECT — 42/42 gates, K1–K4 bit-exact.
   Perf unproven (env timing noise 2–20×). KEPT OUT for now (see below).
5. **R5 cancellations + constant folding** (compound/let/while/cond adjacent
   push-pop NOP + literal folding in emit_binop): layout-sensitive crash
   again; even one cancellation alone crashed a rebuild. REVERTED.

## The hard lesson (binding for the next attempt)
**Do not rewrite the emitted word stream in place.** This compiler compiles
ITSELF: every word-level rewrite changes the stream the next generation
compiles, and the failure mode is layout-sensitive in a way the fixpoint
generation check does not catch. In-place rewrites are banned for R6.

## R6.0 — root-cause the (1) write failure FIRST (blocking, ~1 session)
Minimal repro: a 20-line .bp whose compiler-run shows the model write lost.
Instrument push/pop with sys_write hex dumps; verify with the OLD binary and
the NEW binary separately. Hypotheses to check in order:
- fntab array param alias: is the fntab passed to push the SAME array the
  reset wrote (compile_fn_at's fntab vs the ptab of the sizing pass)?
- the `let dbg = zeros(24)` debug added calls (sys_write) INSIDE push — the
  old compiler's own call ABI may clobber the model cells via the arena
  (x27 bump) — check x27/x28 cursor interaction with the fntab allocation.
- emit-time `if … then em(…) else …` evaluating BOTH branches at compile
  time (the branchless-cond lore) — if em() ran twice, n advances twice.

## R6.1 — the register-aware protocol (no word rewrites)
Track the value stack at EMIT TIME in emitter-owned state (not in the emitted
stream). State = per-emission-point: depth + "top is in x0" flag. Concretely:
- `push` → mark slot; emit mov x(9+depth),x0 ONLY when the value must
  survive (depth < 5) or fall back to memory. The decision is compile-time;
  NO runtime branch.
- `pop rd` → emit mov rd, x(slot) (register case) or the ldr (memory case).
- flush-on-bl: materialize live slots before every bl (bottom-up, memory
  layout identical to today's pushes).
- The model lives in the fntab/emitter arrays ONLY IF R6.0 proves the write
  path safe; otherwise thread a dedicated `stm: [i64]` through the emit_*
  signatures (mechanical, ~40 call sites — the earlier script exists).

## R6.2 — constant folding (source-level, not word-level)
In emit_apply_*/emit_binop: when BOTH operands are compile-time constants
(the emitter knows: the literal emitter just ran), compute the folded value
in the EMITTER and emit ONE literal — never rewrite already-emitted words.
Requires the emitters to report "my result is constant c" up the tree
(a cell [is_const, value] per emission) — that is the deleted fpC tier's
"lazy constant folding with materialization points".

## R6.3 — verification ladder (all must pass)
1. fixpoint gen2==gen3 (byte-exact, no layout lottery);
2. std_golden 42/42 + parity 9/0 + construct 20/20;
3. K1–K4 kernels bit-exact vs the frozen values;
4. in-process clock_ms benchmark (REPORT.md shape), median of ≥9 runs,
   K1/K4 vs Rust medians — ship the numbers, whatever they are.

## Fallback if R6 cannot land safely
Keep the stack machine (its ops are forwarding-cheap) and publish the honest
status: correctness complete, performance gap documented. No fabricated
maturity (Q12).
