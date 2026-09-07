Status: 2026-09-06, owner main session (Fable fork), grounded at HEAD b1c0175 with the A1 worker tree; depends on A5 (x17 reserve; the x14 region already carved from the arena per activation)
Revalidated: 2026-09-08 (light lane, docs only) against bebop.bin 7d8262a1 / bebop.bp md5 3c1b319f5779a30d4dd556cb03fb6370, i.e. AFTER A14 landed (2026-09-07, commit 21e92aa). A14 moved the frame layout this blueprint was drafted against (slot bound 64 -> 256, x15 region 512 -> 2048 B, x14 sp+0x300 -> sp+0x900) and, as a direct consequence, handed A6 an INHERITED OBLIGATION it was not written with: ROADMAP row A14b parks fuzz seeds 100671 and 100828 on A6 -- see §0a. Line numbers are from that md5; the FUNCTION NAMES are the real anchors, grep them.

# A6 Pointer-free step 2: aggregates allocate on the arena cursor; computed frames (B4 merged); x14 / T118 / exit 81 deleted

## 0. Goal

Array literals, struct literals and enum ctors allocate at the arena cursor x27 (the same bump every `zeros` uses) with LIFO release at `ret` and mark/reset on `while` back-edges; the per-activation 16 KiB frame becomes `F = 80 + 8*marks + 8 + 8*(S + tsp)` bytes from the facts the register model publishes. Gates: c67_deeprec (deep recursion without a trap), c33/c34/c40 re-frozen with identical values, **TRAP-81 = 0 by construction (the trap no longer exists) and the two A14b seeds green (§0a)**, TRAP-82 = 0, docs/PERF.md `selfcompile_maxrss` and the RSS column of honest.sh reported.

## 0a. Inherited obligation from A14b (added 2026-09-08 -- binding)

A14 grew the x15 temp-slot region from 512 to 2048 bytes and moved x14 from sp+0x300 to sp+0x900, taking **1536 bytes** out of the per-activation frame heap (15616 -> 14080 B; note ROADMAP row A14b says "1280 bytes" -- the verified number is 1536, see §3 "Where the 16 KiB goes today"). Two seeds of A14's 330-repro corpus sweep now compile cleanly and then trap at runtime:

| repro | expected | today (verified 2026-09-08 on bebop.bin 7d8262a1) |
|---|---|---|
| `~/.cache/bebop/fuzzd/repros/UNSUPPORTED-89-100671.bp` | `expected=6` (header line 1) | compile rc=0; run rc=**81**, `trap 81: frame heap exhausted (array literal or enum ctor)` |
| `~/.cache/bebop/fuzzd/repros/UNSUPPORTED-89-100828.bp` | `expected=4` (header line 1) | compile rc=0; run rc=**81**, same text |

Reproduce exactly like this (bebop.bin is an image, not an executable):

```
nice -n 10 taskset -c 0-3 ./seed/build/seed bebop.bin compile ~/.cache/bebop/fuzzd/repros/UNSUPPORTED-89-100671.bp $OUT/100671.bin
nice -n 10 taskset -c 0-3 ./seed/build/seed $OUT/100671.bin ; echo rc=$?
```

**A6's acceptance line for this pair:** both seeds must compile rc=0, run rc=0 and print their recorded `expected=` value (100671 -> 6, 100828 -> 4) on the A6 candidate, and the class must be gone, not merely under budget: `TRAP-81 = 0`. "Gone" is checkable two ways and both are required -- (i) the two runs above, (ii) no `brk #81` word survives anywhere in a produced image: `objdump -D -b binary -m aarch64 $OUT/gen4.bin | grep -c 'brk\s*#0x51'` == 0 (0x51 = 81; the same shape as the trap-87 check on the WORKER-CARD).

Honest caveat to carry into the VERDICT: A6 does not bound allocation, it re-homes it. A literal inside a `while` body that `loop_alloc_safe` refuses to reset still leaks, and after A6 it leaks into the 256 MiB arena instead of a 14080-byte frame region -- exit **80**, not 81, and ~18 600x later. If either seed comes back as rc=80 rather than its expected value, that is a NEW finding for the main session (an escape-analysis item, out of scope per §1), not a licence to widen the arena.

## 1. Scope

In: `emit_array_lit` (bebop.bp:3328), `emit_struct_lit` (:409), `emit_enum_ctor` (:511), `emit_enum_ctor_nullary` (:531): the `mov x0,x14` (2853045216) + `add x14,x14,#8n` + T118 trap prefix becomes `mov x0,x27 ; add x27,x27,#8n ; cmp x27,x28 ; b.ls +2 ; brk #80` (the emit_zeros trap, 3 words) + the A5 index conversion.

**The T118 trap is no longer five inline words per site.** Since the A4-round-2 bugfix (bebop.bp:1946-1962) it is one helper, `emit_heap_trap` (bebop.bp:1964), called from four sites (bebop.bp:438, 525, 540, 3374); it emits 5 words and takes its scratch register from `vs_alloc` / `vs_mask_free` rather than hardcoding x2. A6 **deletes the whole function and all four calls**. Two consequences the worker must not miss: (i) the deletion returns a window register at every literal site, which can only relieve register pressure -- if an exit-89 construct changes behaviour, that is why; (ii) `vs_settle_flags` in emit_array_lit (bebop.bp:~3364) STAYS -- the replacement `cmp x27,x28` clobbers NZCV exactly like the trap it replaces, and that call is what keeps a trailing FLAGS element correct.

Also in: the fn-level release: prologue stores x27 into the frame's fn-mark cell, epilogue restores it (only for fns whose facts say has_agg_release); T43 (`emit_while_stmt` bebop.bp:4311: pmark nop patched to `str x14,[sp,#80+8*d]` at :4425, word 4177527790 + (10+ldepth)*1024, reset `ldrw` = 4181722094 + (10+ldepth)*1024 at :4398, depth gate `ldepth <= 20` at :4317) switches to x27 with the SAME slot logic and the same `loop_alloc_safe` text scan (bebop.bp:3796); the alloc scan `count_word(insns, body_start, n[0], 2853045216)` at emit_while_stmt:4395 and the identical scan in `compile_fn_at_total_saved` (bebop.bp:5040, the `real_alloc` fact) look for `mov x0,x27` instead of `mov x0,x14` -- derive that word with as+objdump.

`emit_prologue_sized` / `emit_epilogue_sized` (bebop.bp:3699 / 3717): `sub sp,sp,#16384` (3510637567 = 0xd14013ff, sh=1 imm12=4) -> `sub sp,sp,#F`; `add x15,sp,#(80 + 8*marks + 8)` instead of the constant `add x15,sp,#256` (2432959471, :3671 / :3712); the `add x14,sp,#0x900` word (2435056622, unconditional at emit_prologue:3672, has_alloc-conditional at emit_prologue_sized:3713) is deleted. `emit_prologue` / `emit_epilogue` (the planning-pass unsized pair, bebop.bp:3662 / 3677) keep today's exact words and the arithmetic correction in `compile_fn_at_total_saved` (bebop.bp:5036-5051) accounts for the difference (B1 mechanism; fw published at bebop.bp:5052). `emit_sys_clone` (bebop.bp:1152) stops re-homing x14 (drop the 2435056622 word at :1170; keep x15 at :1169 and x27/x28). Exit 81 row removed from the exit-code table comment in bebop.bp (above `emit_paren`, bebop.bp:2980-2981), from docs/TRAPS.md, and from the entry stub's trap-text table (`entry_stub`, bebop.bp:5980 -- the texts live at the end of the 131-word stub); gen.py stops predicting TRAP-81.

Out: escape analysis beyond today's `loop_alloc_safe` (a literal that escapes a loop iteration keeps today's behaviour: no reset, the arena grows -- exit 80 instead of exit 81 when it runs out; LANGUAGE.md loop-release rule unchanged); returning a literal from a fn stays undefined (it is today: the frame heap dies at ret; now the cursor is restored at ret -- same class). Fixed points: `zeros` semantics; the store; bpref (allocation is invisible to it apart from A5's arena list).

## 2. Preconditions

A5 landed: x17/x27/x28 as described, the frame heap region carved from the arena (A5 step 1, 14080 bytes/activation, 6 prologue words + 2 epilogue words, its own rewritten `emit_heap_trap` bound in the `[sp,#248]` cell) -- A6 removes that region wholesale and lets aggregates use the cursor directly, which is why A5 §3 says the two must land back to back. Facts vc/alloc/cs_hi/tsp published (bebop.bp:5052); the §3.14 invariants.

## 3. Design

**Where the 16 KiB goes TODAY** (post-A14, every number verified in bebop.bp at the md5 above):

| region | bytes | evidence |
|---|---|---|
| [sp+0, sp+80) callee-saved x19..x28, 5 `stp` pairs | 80 | emit_prologue:3666-3670, 2835370995 = `stp x19,x20,[sp,#0]` .. 2835641339 |
| [sp+80, sp+248) T43 `while` mark cells, depth 0..20 = 21 cells | 168 | slot `[sp,#80+8*depth]`, emit_while_stmt:4317/4398/4425 |
| [sp+248, sp+256) unused | 8 | -- |
| [sp+256, sp+2304) x15 spill region, 256 slots of 8 B | 2048 | `add x15,sp,#256` = 2432959471 (:3671, :3712); bound `s0 + tsp0 > 256 -> diag_exit 89`, compile_fn_at_facts:5082 (**64 before A14**) |
| [sp+2304, sp+16384) frame heap, x14 bumps up | **14080** | `add x14,sp,#0x900` = 2435056622 (:3672, :3713, :1170); trap bound `x15 + 12288 + 3840` = sp+16384, emit_heap_trap:1964 |
| total `sub sp,sp,#16384` | 16384 | 3510637567 |

That is the budget A14b's two seeds overflow: 14080 bytes of literals/ctors live in one activation. Before A14 they had 15616 and passed.

**What A6 replaces it with.** No frame heap at all. `F = 80 + 8*marks + 8 + 8*(S + tsp)`, rounded up to 16, with marks <= 21 and S + tsp <= 256, so **F ranges 80 .. 2304 bytes** -- a 7x to 200x shrink, and 2304 < 4095 still fits one `sub sp,sp,#F` imm12 (keep the two-word `sub ..,lsl #12` + `sub ..` fallback as a guard; it can no longer fire, and the guard's own words must be derived if written). x15 = sp + 80 + 8*marks + 8 (which is exactly today's sp+256 when marks = 21, a useful sanity check); slot addressing `[x15,#k*8]` unchanged. Aggregates move to the 256 MiB arena, which is 18 600x the region they had, and their exhaustion class becomes exit 80 (`zeros`'s existing trap), not a new one. Stack guard (TRAP-82 = SIGSEGV on the guard page) unchanged; with F ~ 100-2300 B the reachable recursion depth grows 7-200x.

**Allocation site** (all four emitters): `mov x0,x27 ; add x27,x27,#8n ; cmp x27,x28 ; b.ls +2 ; brk #80 ; sub x0,x0,x17 ; lsr x0,x0,#3` (7 words; today 7 = 2 + emit_heap_trap's 5, plus A5's 2 = 9, so -2 per literal). Big literals: `add x27,x27,#imm` imm12 covers n <= 511 cells (today's limit too, bebop.bp comment "valid for <= 511 elements").

**Release at ret.** Prologue (has_agg_release fns only): `str x27,[sp,#(80 + 8*marks)]` (the fn-mark cell right after the while marks); epilogue: `ldr x27,[sp,#...]` before `ret`. `return e;` paths jump to the epilogue via `patch_jumps` on the list based at **fntab[4412]** (compile_fn_at:5174, and the same base at :4977) so they release too. Arena growth inside a fn between the mark and ret -- including `zeros` calls -- is also released: **this changes `zeros` semantics inside such functions** (today `zeros` memory is permanent). Rule: the fn-mark/release is emitted only when the fn body contains an aggregate literal AND no `zeros` (text scan for the word `zeros(`); fns with both keep permanent allocation (no release) exactly like today's zeros (the literal then leaks like a zeros would). bebop.bp itself has many fns where `zeros` and literals coexist -> those get NO release; only literal-only fns release. Census in step 0 tells how many fns of each kind exist (expect: most literal fns are literal-only).

**Loops.** T43 unchanged in structure: pmark `str x27,[sp,#80+8*d]`, reset `ldr x27,[sp,#80+8*d]` at the back-edge and the exit, decided by `loop_alloc_safe`; the alloc scan looks for `mov x0,x27`. Nested loops: depth <= 20 as today (fntab[4411], the LIVE depth, incremented at emit_while_stmt:4315-4316 and reset in compile_fn_at).

**Facts -- the packing changed under A14, re-derive it.** Today (bebop.bp:5052):
`fw = vc + 256*alloc + 512*cs_hi + 8192*tsp`, decoders `fw_vc`/`fw_alloc`/`fw_cshi`/`fw_tsp` at bebop.bp:5060-5063. Field map: vc bits 0-7, alloc bit 8, cs_hi bits 9-12, tsp bits 13+. **Since A14 tsp can reach 256, so 8192*tsp reaches 2^21 and the tsp field occupies bits 13..21.** The 2^20 slot this blueprint originally reserved for `marks` is INSIDE tsp's range and would corrupt every wide-slot fn. Use instead:

```
fw = vc + 256*alloc + 512*cs_hi + 8192*tsp + 4194304*marks + 134217728*rel      (2^22, 2^27)
fw_tsp(fw)   = let t = ((fw / 256) / 2) / 16 in t - (t / 512) * 512             // was unbounded; must now mask 9 bits
fw_marks(fw) = let m = fw / 4194304 in m - (m / 32) * 32                        // marks <= 21, 5 bits
fw_rel(fw)   = fw / 134217728                                                    // has_agg_release, 1 bit
```

`marks` is the fn's max `while` nesting depth (high-water), and it does not exist today: fntab[4411] holds the LIVE depth only. Add a high-water cell next to it -- fntab[4578] and [4579] are free in the register-model map (bebop.bp:1722-1731 documents 4547/4548/4573-4577, 4591/4592 are the A14 arm bases; confirm with `grep -n 'fntab\[4578\]' bebop.bp` == empty before using it) -- bumped where fntab[4411] is incremented and zeroed where it is reset, then published by `compile_fn_at_total_saved` alongside real_vc/real_alloc/real_cs_hi/real_tsp.

`alloc` now means "uses x27 for aggregates" (drives the mark/release words and nothing else). x27 stays a process register: callees bump it and, if they release, restore it; a callee that does not release leaves x27 advanced = permanent allocation, correct. x14 disappears from `emit_bl` (bebop.bp:546): `stp x15,x14,[sp,#-16]!` (2847882223) / `ldp x15,x14,[sp],#16` (2831236079) become a single `str x15,[sp,#-16]!` / `ldr x15,[sp],#16` (still 2 words; only x15 is caller-saved and needed), and `has_spills` reduces to `vc > 8 or tsp > 0` -- update the matching `real_needs_save_raw` / `needs_save0raw` expressions (bebop.bp:5045 and 5070), which today read `(if vc > 8 then 1 else alloc) + (if tsp > 0 then 1 else 0)`, and the `saved_x15` / `saved_x14` / `saved_calls` terms of `compile_fn_at_total_saved` (bebop.bp:5048-5051) with them. Those five lines are the whole two-pass agreement; a mistake there is a wrong `bl` target, i.e. SIGILL, not a wrong value.

**Invariants.** Planning/emission agreement: every new word is decided by text/facts, never by a register; both passes emit the same count at the allocation sites (the unsized prologue differs by the arithmetic correction as today). The fn-mark cell is written before any allocation and read after the tail value is in x0 (epilogue order: restore x27, restore pairs, ret).

## 4. Files and functions touched

Anchors are function names; line numbers are bebop.bp md5 3c1b319f (2026-09-08) and will have drifted.

| file:fn | change | anchor |
|---|---|---|
| bebop.bp:emit_array_lit / emit_struct_lit / emit_enum_ctor / emit_enum_ctor_nullary | x27 allocation + brk 80 trap; keep vs_settle_flags | 3328, 409, 511, 531 |
| bebop.bp:emit_heap_trap | DELETED, with its 4 call sites | 1964; calls at 438, 525, 540, 3374 |
| bebop.bp:emit_while_stmt, loop_alloc_safe, count_word callers | x27 mark/reset; new alloc word; marks high-water | 4311, 3796, 4395, 5040 |
| bebop.bp:emit_prologue_sized / emit_epilogue_sized / emit_bl / compile_fn_at_total_saved / compile_fn_at_facts / fw_* | computed F, x15 offset, fn mark, facts marks/rel, single x15 save | 3699, 3717, 546, 5036-5052, 5065-5082, 5060-5063 |
| bebop.bp:emit_sys_clone | drop the x14 re-home word 2435056622 | 1152 (word at 1170) |
| bebop.bp: exit-code table comment above emit_paren, entry_stub trap texts, docs/TRAPS.md, bench/fuzz gen.py TRAP-81 prediction | exit 81 removed | 2980-2982; entry_stub 5980; TRAPS.md |
| tools/check_abi.py | new prologue/epilogue forms allowlisted; `b1_facts` comment block updated | ~169-174 |
| docs/LANGUAGE.md | Frame heap paragraph -> arena aggregates + release rule | -- |

Stale comments in bebop.bp that A6 should fix while it is in there (they say 64 where the code says 256, both left over from A14): the exit-code table above emit_paren says "S+tsp > 64 (§5)" (bebop.bp:2972) and there is a second "past `s0 + tsp0 > 64`" comment at bebop.bp:4706. Neither is load-bearing; both mislead the next worker.

## 5. Steps

0. Census (python): fns with literals only / literals + zeros / neither; while bodies with literals (T43 sites); the marks high-water distribution over bebop.bp's own fns (it decides the F range you will report). Report in the journal.
1. Allocation sites + T43 on x27 + fn-mark release (facts extended, emit_heap_trap deleted) -- one chain commit; c33/c34/c40 re-frozen with identical VALUES (WORD_DELTA recorded); c67_deeprec added; **the two A14b seeds run here already** (they only need the frame heap gone, not the computed frame).
2. Computed frames (F, x15 offset, single x15 save at bl, x14 gone everywhere) -- second chain commit; check_abi; TRAPS.md; gen.py.
Leave each uncommitted for the main session.

## 6. Constructs, oracles, twins

Construct numbers checked against `ls bench/parity_constructs/` on 2026-09-08: c66, c70, c71, c72, c90, c91 are TAKEN. c67 is free and keeps its name; the old "c69_litrelease" of this blueprint collided with A5's c69_index_roundtrip, so it is c93_litrelease.

| construct | source | EXPECT | exercises |
|---|---|---|---|
| c67_deeprec | `fn d(n: i64) -> i64 { if n == 0 then 0 else 1 + d(n - 1) } fn main() -> i64 { d(N) }` | N (bpref recursion limit: raise its limit for the oracle or use the closed form as EXPECT) | pick N from the MEASURED F of `d` (vc = 1, no marks, no slots, no agg -> F = 80 + 8 = 88 -> 96 after the 16-round; 8 MB default stack / 96 B ~ 87 000 activations, so N = 60000 with margin -- re-derive N from the F your build actually emits, do not copy this number) |
| c33_loopalloc / c34_loopescape | re-frozen | same values | mark/reset on x27 |
| c40_struct, c10_struct, c11_enum, c12_match | re-frozen | same | allocation sites |
| c93_litrelease | a literal-only fn called 10^6 times in a loop (the arena must not grow: compare the index returned by the first call with the index returned by a later one -- equal, via `sys_arena_base()` which returns the CURSOR, see A5 §2) | bpref value + the index equality | fn-mark release |
| A14b pair (not constructs -- repro files) | `UNSUPPORTED-89-100671.bp` / `-100828.bp` | 6 / 4 | §0a; the exit-81 class is gone |

Fuzz: TRAP-81 = 0 is provable without a batch (the `brk #0x51` objdump count of §0a); the 2000-seed batch, if the operator's fuzz freeze permits one at all, additionally shows TRAP-82 = 0.

## 7. Gates

- chain `--codegen` GREEN twice (steps 1, 2); WORD_DELTA lines; word_budget for growth (expect a net DECREASE: -2 per literal, -1 per bl in has_spills fns, -1 prologue word); census_allow if `b.ls` counts move (they will: one `b.ls` moves from emit_heap_trap to the allocation site per literal, net zero, but the census counts sites not classes -- write the reason on the census_allow line).
- **A14b acceptance:** seeds 100671 and 100828 compile rc=0, run rc=0, print 6 and 4 respectively; `TRAP-81 = 0`, proven by both checks in §0a.
- docs/PERF.md: selfcompile_maxrss reported; honest.sh RSS column; K2H (fib) ms: expect a small gain (16 KiB `sub sp` -> <= 2304 B; fewer cache lines touched per call). Timing rows are only valid when no other lane is running -- see PARALLEL-LANES §1.
- RED: a std_test that relied on a literal surviving its fn (undefined today, would now read released memory): the std_golden lane finds it; fix the program (it is UB) and note it.

## 8. Risks and probes

| risk | probe | symptom |
|---|---|---|
| a literal escapes its fn and was accidentally working (frame heap not yet overwritten) | std_golden; c34_loopescape | value changes after A6 -- classify as UB per LANGUAGE.md, fix the test |
| release in a fn that also calls `zeros` (would free permanent cells) | text scan rule; census | data corruption in the store gates |
| the `marks` field collides with `tsp` in fw | a fn with tsp near 256 and a nested while (bebop.bp's own compile_fn_at) | wrong frame size / SIGILL -- see §3 "Facts" |
| F rounding / x15 offset mismatch between passes | c21_param13, c53_param9 (spilled params through x15) | wrong 9th param |
| the two-pass word correction (needs_save / saved_x14 / saved_calls) | any construct calling an allocating fn | SIGILL or a wrong `bl` target |
| an A14b seed comes back rc=80 instead of its value | §0a caveat | a loop-escaping literal; a NEW finding for the main session, not a wider arena |
| stack guard reached later than expected | c67 depth | SIGSEGV = TRAP-82 |
| clone children: today emit_sys_clone re-homes x15/x14/x27/x28 -- keep x27/x28, drop x14; the fn-mark release inside a thread restores the thread's cursor | pool_parity lane | thread crash |

## 9. VERDICT format

```
VERDICT: GREEN|RED
census: literal-only fns <n>, literal+zeros fns <n>, T43 sites <n>, marks high-water max <n>
step1 fixpoint <md5>; step2 fixpoint <md5>
bin_words <b> -> <a>; WORD_DELTA summary; c67/c93 EXPECT; c33/c34/c40 values unchanged: yes
frames: F range <min>..<max> bytes; x15 offset formula verified by c21/c53
A14b: 100671 rc=0 value=6; 100828 rc=0 value=4; TRAP-81 = 0 (brk #0x51 count 0)
TRAP-82 0; DIVERGE 0 (if a batch was run)
RSS: selfcompile_maxrss <b> -> <a>; K2H ms <b> -> <a>
journal: <lines>
open: <anything>
```

## 10. Worker prompt skeleton

<context> repo, this blueprint (§0a and §3's two tables first), A5 facts (x17, x27 semantics, the carve A6 deletes), the register-model facts and pitfalls, harness commands. </context>
<constraints> two chain commits (allocation+release, then frames); no bpref change except the TRAP-81 prediction; words via as+objdump; the fw repacking and the needs_save/total_saved edits land with the code that needs them; leave uncommitted. </constraints>
<output_format> §9. </output_format>
<task> A6 steps 0-2, including the A14b acceptance of §0a; report. </task>
