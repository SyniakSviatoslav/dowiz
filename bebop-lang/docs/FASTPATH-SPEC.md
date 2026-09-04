# SPEC — R6 Fast-Path Tier (register-aware emitter, "faster than Rust")

Status: 2026-09-04 SUPERSEDED-BY ROADMAP.md (R6.2 v5 folding LANDED 2026-09-03; the untyped register window R6.1/T13 was RETIRED by operator decision 2026-09-04 — register residency is the typed Z2 bank, ROADMAP T25/T26/T35). Historical card kept for the failure modes it records.
Original status: NOT STARTED (blocked on R6.0 root-cause). This card replaces the
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

## R6.0 — RESOLVED (journal 1788288244, commit 4ad49f2)
The clean R6.1 rebuild does NOT reproduce the zeroing. Base-pointer logs
prove push and pop see the SAME fntab (matching b16/b8 bytes) and the
depth survives across calls; zero traps across ptab/fntab/self-test
arrays. The old signature was an artifact of the WIP (per-fn reset,
emit_bl flush, or misthreaded site in compile/compile_fn/emit_expr_words),
now superseded by the committed threading. REMAINING: re-verify fixpoint
bb2==bb3 and 44 gates on a stable box.

## R6.0-archive — root-cause the (1) write failure FIRST (blocking, ~1 session)
STATUS: sharpened this session (journal 1788288243). The failure is REAL
and now LOCATED to its signature, not yet its mechanism:
- trap cascade built: push write-reread checks (slot + depth) NEVER trap —
  the pushes' writes land and are CONFIRMED inside push; pop checks trap
  EVERY time (s==0, d==0) — so the model zone is ZEROED between the push's
  return and the pop's entry;
- the COMPILED push and pop are provably correct (full disasm: right base
  register per signature — push param3=x21, pop param4=x22; right
  3800/3801+d indexing; str/ldr [x2,x1,lsl#3]);
- standalone probes (user-level big-index writes, writes+call+read) all
  PASS — the corruption is COMPILER-CONTEXT-SPECIFIC.
Next-session hypotheses, in order:
- the sizing pass uses PTAB, the real pass FNTAB — is the pass-2 pop
  reading an array the pass-2 push did not write (ptab/fntab aliasing)?
- the caller's arg-prep stack (sub sp per arg) vs the callee's frame:
  verify the caller's fntab register survives the callee's param binding
  (x19-x26 saves) — disasm the push/pop prologues completely.
- run ONE push+pop pair at compile_program_to's top (outside emission) and
  observe whether the zone survives even without intervening emissions.

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

## R6.2 — model-driven folding (DESIGN PROVEN, implementation deferred)
Status (journal 1788288252): the design is validated end-to-end on small
probes (both-const and right-const retract+re-emit are semantically exact),
but the first implementation carried a layout-sensitive compile-time crash
(heisenbug: vanished with ladder prints; emit_binop reached ~35 bindings —
the spill-machinery class per L15). Reverted per L14/L15.

Next-session plan (single-variable diffs, one commit each):
1. Extract the fold into helper fns, each <=15 bindings (the proven range:
   emit_match_arm=15 works, b36=36 works as a USER fn, but the compiler's
   own 35-binding emit_binop crashes) — both-const path ONLY first.
2. Re-run the full battery (fixpoint, 44 gates, parity, construct 24/24)
   and regenerate frozen artifacts + the c1-c8 startup checksums (folding
   changes emitted streams; compile() user-call returns 0 — the checksums
   must be recomputed from the compiler's own startup path).
3. Then add the right-const imm12 path (the i+1 increment win) as a second
   commit.

## R6.2-archive — constant folding (source-level, not word-level)
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
