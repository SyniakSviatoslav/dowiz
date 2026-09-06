Status: 2026-09-06, owner main session (Fable fork), grounded at HEAD b1c0175 with the A1 worker tree; depends on A5 (x17 reserve; the x14 region already carved from the arena per activation)

# A6 Pointer-free step 2: aggregates allocate on the arena cursor; computed frames (B4 merged); x14 / T118 / exit 81 deleted

## 0. Goal

Array literals, struct literals and enum ctors allocate at the arena cursor x27 (the same bump every `zeros` uses) with LIFO release at `ret` and mark/reset on `while` back-edges; the per-activation 16 KiB frame becomes `F = 80 + 8*marks + 8 + 8*(S + tsp)` bytes from the facts the register model publishes. Gates: c67_deeprec (recursion depth 10^5 without a trap), c33/c34/c40 re-frozen with identical values, fuzz TRAP-81 class = 0 by construction (the trap no longer exists) and TRAP-82 = 0, docs/PERF.md `selfcompile_maxrss` and the RSS column of honest.sh reported (expect -16 KiB per live activation: recursion-heavy programs only).

## 1. Scope

In: emit_array_lit (bebop.bp:2953, verified), emit_struct_lit (:403), emit_enum_ctor (:509), emit_enum_ctor_nullary (:533): the `mov x0,x14 ; add x14,x14,#8n ; T118 trap (5 words at :432/:523/:542/:2991, verified)` prefix becomes `mov x0,x27 ; add x27,x27,#8n ; cmp x27,x28 ; b.ls ; brk #80` (the emit_zeros trap, 3 words) + the A5 index conversion; the fn-level release: prologue stores x27 into the frame's fn-mark cell, epilogue restores it (only for fns whose facts say has_agg); T43 (emit_while_stmt bebop.bp:3593: pmark nop patched to `str x14,[sp,#80+8*d]`, `ldrw` reset words at :42-43/:66-67 verified) switches to x27 with the SAME slot logic and the same `loop_alloc_safe` text scan (bebop.bp:3413); `count_word(insns, ..., mov x0,x14 = 2853045216)` at emit_while_stmt:39 and compile_fn_at (the `real_alloc` fact) scan for `mov x0,x27` instead; emit_prologue_sized/emit_epilogue_sized (bebop.bp:3316/3334): `sub sp,sp,#16384` -> `sub sp,sp,#F` (F <= 4095 fits the imm12; F > 4095 -> `sub sp,sp,#(F>>12),lsl #12` + `sub sp,sp,#(F&4095)` two words -- possible only with 64 slots + 20 marks: F = 80+160+8+512 = 760, so one word always; keep the two-word form as a guard), `add x15,sp,#(80 + 8*marks + 8)` instead of `#256`; emit_prologue/emit_epilogue (the planning-pass unsized pair) keep today's exact words and the arithmetic correction in compile_fn_at accounts for the difference (B1 mechanism, bebop.bp:4225-4242 verified: fw = vc + 256*alloc + 512*cs_hi + 8192*tsp -- add `marks` (max while depth, fntab[3661] high-water) as a fifth fact: fw + 2^20 * marks, marks <= 21). emit_sys_clone (bebop.bp:1159) stops re-homing x14. Exit 81 row removed from docs/TRAPS.md and the emit_paren table; gen.py stops predicting TRAP-81.
Out: escape analysis beyond today's `loop_alloc_safe` (a literal that escapes a loop iteration keeps today's behaviour: no reset, the arena grows -- exit 80 instead of exit 81 when it runs out; LANGUAGE.md loop-release rule unchanged); returning a literal from a fn stays undefined (it is today: the frame heap dies at ret; now the cursor is restored at ret -- same class). Fixed points: `zeros` semantics; the store; bpref (allocation is invisible to it).

## 2. Preconditions

A5 landed: x27/x28/x17 as described, the frame heap region carved from the arena (A5 step 1) -- A6 simply removes that region and lets aggregates use the cursor directly; facts vc/alloc/cs_hi/tsp published; the §3.14 invariants.

## 3. Design

**Allocation site** (all four emitters): `mov x0,x27 ; add x27,x27,#8n ; cmp x27,x28 ; b.ls +2 ; brk #80 ; sub x0,x0,x17 ; lsr x0,x0,#3` (7 words; today 7 with the T118 trap + A5's 2 = 9, so -2 per literal) then the element stores through the index (A5 forms). Big literals: `add x27,x27,#imm` imm12 covers n <= 511 cells (today's limit too, bebop.bp comment "valid for <= 511 elements").

**Release at ret.** Prologue (has_agg fns only): `str x27,[sp,#(80 + 8*marks)]` (the fn-mark cell right after the while marks); epilogue: `ldr x27,[sp,#...]` before `ret`. `return e;` paths jump to the epilogue (patch_jumps 3662 list) so they release too. Arena growth inside a fn between the mark and ret -- including `zeros` calls -- is also released: **this changes `zeros` semantics inside such functions** (today `zeros` memory is permanent). Rule: the fn-mark/release is emitted only when the fn body contains an aggregate literal AND no `zeros` (text scan: the word `zeros(`); fns with both keep permanent allocation (no release) exactly like today's zeros (the literal then leaks like a zeros would). bebop.bp itself: `zeros` and literals coexist in many fns (emit_half's `let p = [1, 65536, ...]` next to `zeros(64)` in compile_fn_at) -> those fns get NO release; only literal-only fns release. Census in step 0 tells how many fns of each kind exist (expect: most literal fns are literal-only).

**Loops.** T43 unchanged in structure: pmark `str x27,[sp,#80+8*d]`, reset `ldr x27,[sp,#80+8*d]` at the back-edge and the exit, decided by `loop_alloc_safe`; the alloc scan looks for `mov x0,x27` (asm text; derive the word). Nested loops: depth <= 20 as today.

**Frame.** F = 80 (fp/lr + 4 pairs, as today's layout) + 8*marks + 8 (fn mark) + 8*(S + tsp), rounded up to 16; x15 = sp + 80 + 8*marks + 8; slot addressing `[x15,#k*8]` unchanged; the 16 KiB constant `sub sp,sp,#16384` (0xd14013ff, bebop.bp:3279 emit_prologue verified) survives only in the planning-pass unsized prologue (throwaway buffer; its word count is what matters). Stack guard (TRAP-82 = SIGSEGV on the guard page) unchanged; with F ~ 100-800 B the reachable recursion depth grows 20-160x.

**Facts.** fw = vc + 256*alloc + 512*cs_hi + 8192*tsp + 2^20*marks + 2^25*has_agg_release (bebop.bp:4242 verified encoding; decoders fw_vc/fw_alloc/fw_cshi/fw_tsp at bebop.bp:~4102 gain fw_marks/fw_rel). `alloc` now means "uses x27 for aggregates" (drives the mark/release words and nothing else; the x14/x15 pair save around `bl` -- emit_bl's `stp x15,x14` -- becomes `stp x15,x27`? NO: x27 is a process register like today (callees bump and, if they release, restore; a callee that does not release leaves x27 advanced = permanent allocation, correct); x14 disappears from emit_bl's save: `stp x15,x14,[sp,#-16]!` becomes a single `str x15,[sp,#-16]!` / `ldr x15,[sp],#16` (2 words as today; only x15 is caller-saved and needed) -- has_spills = vc > 8 or tsp > 0 only.

**Invariants.** Planning/emission agreement: every new word is decided by text/facts, never by a register; both passes emit the same count at the allocation sites (the unsized prologue differs by the arithmetic correction as today). The fn-mark cell is written before any allocation and read after the tail value is in x0 (epilogue order: restore x27, restore pairs, ret).

## 4. Files and functions touched

| file:fn | change | anchor |
|---|---|---|
| bebop.bp:emit_array_lit / emit_struct_lit / emit_enum_ctor / emit_enum_ctor_nullary | x27 allocation + brk 80 trap | 2953, 403, 509, 533 |
| bebop.bp:emit_while_stmt, loop_alloc_safe, count_word callers | x27 mark/reset; new alloc word | 3593, 3413, 3472 |
| bebop.bp:emit_prologue_sized / emit_epilogue_sized / emit_bl / compile_fn_at (+_facts/_total_saved) / fw_* | computed F, x15 offset, fn mark, facts marks/has_agg_release, single x15 save | 3316, 3334, 552, 4225-4242, ~4102 |
| bebop.bp:emit_sys_clone | drop x14 re-homing | 1159 |
| bebop.bp: exit-code table comment (emit_paren), docs/TRAPS.md, bench/fuzz gen.py/bpref TRAP-81 prediction | exit 81 removed | emit_paren 2765; TRAPS.md |
| tools/check_abi.py | new stub/prologue forms allowlisted; `b1_facts` comment block updated | ~169-174 |
| docs/LANGUAGE.md | Frame heap paragraph -> arena aggregates + release rule | -- |

## 5. Steps

0. Census (python): fns with literals only / literals + zeros / neither; while bodies with literals (T43 sites). Report in the journal.
1. Allocation sites + T43 on x27 + fn-mark release (facts extended) -- one chain commit; c33/c34/c40 re-frozen with identical VALUES (WORD_DELTA recorded); c67_deeprec added.
2. Computed frames (F, x15 offset, single x15 save at bl, x14 gone everywhere) -- second chain commit; check_abi; TRAPS.md; gen.py.
Leave each uncommitted for the main session.

## 6. Constructs, oracles, twins

| construct | source | EXPECT | exercises |
|---|---|---|---|
| c67_deeprec | `fn d(n: i64) -> i64 { if n == 0 then 0 else 1 + d(n - 1) } fn main() -> i64 { d(100000) }` | 100000 (bpref recursion limit: raise its limit for the oracle or use the closed form as EXPECT) | 10^5 activations x F <= 8 MB stack: F must be <= 80 B for this fn (vc = 1, no marks, no slots -> F = 96 -> 9.6 MB > 8 MB default stack: use depth 60000 -> EXPECT 60000) |
| c33_loopalloc / c34_loopescape | re-frozen | same values | mark/reset on x27 |
| c40_struct, c10_struct, c11_enum, c12_match | re-frozen | same | allocation sites |
| c69_litrelease | a literal-only fn called 10^6 times in a loop (arena must not grow: fold + `sys_arena_base`-relative cursor check via a second call's returned index equal to the first's) | bpref value + the index equality | fn-mark release |

Fuzz: one 2000-seed batch on the candidate must show TRAP-81 = 0 (the class cannot occur) and TRAP-82 = 0.

## 7. Gates

- chain `--codegen` GREEN twice (steps 1, 2); WORD_DELTA lines; word_budget for growth (expect a net DECREASE: -2 per literal, -1 per bl in has_spills fns, -1 prologue word); census_allow if `b.ls` counts move.
- docs/PERF.md: selfcompile_maxrss reported; honest.sh RSS column; K2H (fib) ms: expect a small gain (16 KiB `sub sp` -> ~100 B; fewer cache lines touched per call).
- `bench/fuzz/fuzz.sh 2000 <start>` on the candidate: TRAP-81 = 0, TRAP-82 = 0, DIVERGE = 0.
- RED: a std_test that relied on a literal surviving its fn (undefined today, would now read released memory): the std_golden lane finds it; fix the program (it is UB) and note it.

## 8. Risks and probes

| risk | probe | symptom |
|---|---|---|
| a literal escapes its fn and was accidentally working (frame heap not yet overwritten) | std_golden; c34_loopescape | value changes after A6 -- classify as UB per LANGUAGE.md, fix the test |
| release in a fn that also calls `zeros` (would free permanent cells) | text scan rule; census | data corruption in the store gates |
| F rounding / x15 offset mismatch between passes | c21_param13, c53_param9 (spilled params through x15) | wrong 9th param |
| stack guard reached later than expected | c67 depth | SIGSEGV = TRAP-82 |
| clone children: x27 per thread (each thread has its own arena? today clone re-homes x27/x28 -- keep that; the fn-mark release inside a thread restores the thread's cursor) | pool_parity lane | thread crash |

## 9. VERDICT format

```
VERDICT: GREEN|RED
census: literal-only fns <n>, literal+zeros fns <n>, T43 sites <n>
step1 fixpoint <md5>; step2 fixpoint <md5>
bin_words <b> -> <a>; WORD_DELTA summary; c67/c69 EXPECT; c33/c34/c40 values unchanged: yes
frames: F range <min>..<max> bytes; x15 offset formula verified by c21/c53
fuzz batch: TRAP-81 0, TRAP-82 0, DIVERGE 0
RSS: selfcompile_maxrss <b> -> <a>; K2H ms <b> -> <a>
journal: <lines>
open: <anything>
```

## 10. Worker prompt skeleton

<context> repo, this blueprint, A5 facts (x17, x27 semantics), the register-model facts and pitfalls, harness commands. </context>
<constraints> two chain commits (allocation+release, then frames); no bpref change except the TRAP-81 prediction; words via as+objdump; leave uncommitted. </constraints>
<output_format> §9. </output_format>
<task> A6 steps 0-2; report. </task>
