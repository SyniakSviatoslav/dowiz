Status: 2026-09-06, owner main session (Fable fork), grounded at HEAD b1c0175 with the A1 worker tree; depends on A1 (register model: SYM/REG tags, register-parameterised ldr/str forms in emit_array_get/set), A2 step 0 (fn cap 512), A3 landed (LIN) -- ROADMAP order A5 after A3
Revalidated: 2026-09-08 (light lane, docs only) against bebop.bin 7d8262a1 / bebop.bp md5 3c1b319f5779a30d4dd556cb03fb6370, i.e. AFTER A14 landed (2026-09-07, commit 21e92aa). A14 moved the frame layout this blueprint was drafted against: the temp-slot bound is 256 (was 64), the x15 spill region is 2048 B (was 512 B), and x14 now starts at sp+0x900 (was sp+0x300). Every offset, word and line number below is the post-A14 value. Line numbers are from that md5 and drift by a few lines per landed commit -- the FUNCTION NAMES are the real anchors, grep them.

# A5 Pointer-free step 1: one address space, `[i64]` values are x17-relative indices

## 0. Goal

Every `[i64]` VALUE in data is an integer index of a cell in ONE reserved address space whose base lives in x17; addresses exist only inside the instruction that reads or writes (`ldr xd,[x17,xt,lsl #3]`). Gates: chain GREEN with constructs c69_index_roundtrip / c92_ptrfree; K5 <= +8 % (docs/PERF.md selfcompile_wall vs the A3 row); K6 nn.bp scan ns/row <= +20 % (bench/tq_sqlite/run.sh); std_golden 99/99 (the store's G1-G8 gates run there); RSS unchanged.

## 1. Scope

In: the reserve + x17 setup in the entry stub; `zeros` returning an index; array get/set/literal forms; every `sys_*`/builtin that takes or returns cells converts index<->address inside its own words; `sys_mmap` placing file maps INSIDE the reserve so store bases are indices too; bpref: arrays as indices (it does NOT model them that way today -- see §3 "bpref"); a python census (step 0). Out: `str` values (raw pointers until A7); the frame heap x14 (A6); typed tables (A8); u32 indices (A8). Fixed points: emit_bl, the prologue/epilogue word PAIRING discipline (x17 is not saved/restored: it is a process constant like x27/x28, set once by the stub, re-homed for clone children exactly like x27/x28 -- emit_sys_clone, bebop.bp:1152, verified); the store's on-disk format (refs are object-relative offsets already, LANG-DB §4b); every construct's VALUE.

## 2. Preconditions

A1-A3 promoted. x17/x18 are unused: `grep -c 'x1[78]' seed/seed.S` == 0 (re-verified 2026-09-08) and the register model uses x16 only as the parallel-move scratch (REGISTER-MODEL §1.1); the stub's arena setup is in `entry_stub` (bebop.bp:5980, the 131-word stub; T90) -- the worker reads it before step 1.

`sys_arena_base` exists (`emit_sys_arena_base`, bebop.bp:1128) but READ ITS BODY FIRST: it emits a single `mov x0,x27` (2853897184), so it returns the arena CURSOR, not the arena base; `sys_arena_end` returns x28. Under the index model both must return INDICES, and "base" keeps meaning "cursor" -- do not silently redefine it, the store and pool code depend on the cursor reading.

The T118 arena trap words in `emit_zeros` (`cmp x27,x28` 3944481663 ; `b.ls +2` 1409286217 ; `brk #80` 3558869504, bebop.bp:5608-5611, verified) stay.

## 3. Design

**Address space.** At start the stub reserves R = 4 GiB (2^32 bytes = 2^29 cells; the u32 index space A8 will need) with `mmap(NULL, R, PROT_NONE, MAP_PRIVATE|MAP_ANONYMOUS|MAP_NORESERVE)`, then commits the arena as today INSIDE it: `mmap(base, 256 MiB, PROT_READ|PROT_WRITE, MAP_FIXED|...)` (or mprotect). x17 = base; x27 = base (bump cursor), x28 = base + 256 MiB (end) as today. File maps: `sys_mmap` takes the next slot from the top of the reserve downward; the stub keeps the next-slot address in a cell inside the first arena page: `[x17,#0]` = next file-map address (cells 0..15 of the arena are the stub's, `zeros` starts after them; index 0 stays "null" for `ref` semantics) and maps with MAP_FIXED there. Step 0 probes that a 4 GiB PROT_NONE reserve succeeds under this proot (a 20-line .bp calling sys_mmap with the flags, or python `mmap` with the same flags); fallback 1 GiB (2^27 cells) if it fails, recorded in the journal.

**Index = (address - x17) >> 3.** `zeros(n)`: today's words allocate at x27 and return the address in x0; add `sub x0,x0,x17 ; lsr x0,x0,#3` before the push (2 words; `emit_zeros`, bebop.bp:5590, verified). Array get/set (`emit_array_get` bebop.bp:3391, `emit_array_set` bebop.bp:3410, verified): base entry is an INDEX; dynamic index: `add xt,x<base>,x<idx> ; ldr xd,[x17,xt,lsl #3]` (t = vs_alloc, freed); constant index c: `add xt,x<base>,#c ; ldr xd,[x17,xt,lsl #3]` (c < 4096) -- 2 words instead of 1 (the +1 add of RESEARCH-NOPOINTERS §1.1; the flat IR's LICM removes it later); same for `str`. Array literal / struct literal / enum ctor: A6 moves them to the arena cursor, but in A5 they still allocate on x14 and hand out a POINTER, so A5 must convert THAT too -- see "Frame heap" below, which is what makes the conversion legal at all.

`sys_*` and builtins with cell arguments convert on entry: `add x<a>,x17,x<a>,lsl #3` per cell argument (1 word each; the arguments are already delivered to x0..x2 by vs_deliver), and builtins that RETURN cells (zeros, sys_readbuf, sys_slurp, sys_mmap, str_to_cells) convert on exit (2 words). `sys_arena_base()` returns the cursor as an index, `sys_arena_end()` the end index. `sys_clone(flags, stack_top)`: stack_top is an index -> converted. Futex/atomic builtins: converted. `char(s,i)`, `str_len`: `str` stays a raw pointer (A7).

**Frame heap (the step that A14 changed).** Today's frame, from `emit_prologue` (bebop.bp:3662) and its own header comment (bebop.bp:3657-3660), verified word by word:

| region | bytes | words / evidence |
|---|---|---|
| `stp x29,x30,[sp,#-16]!` ; `mov x29,sp` | above the frame | 2847898621 / 2432697341 |
| `sub sp,sp,#16384` | the whole frame = 16384 B | 3510637567 = 0xd14013ff, sh=1 imm12=4 |
| [sp+0, sp+80) callee-saved x19..x28 | 80 | 5 `stp` pairs, 2835370995 = `stp x19,x20,[sp,#0]` .. 2835641339 |
| [sp+80, sp+248) T43 while marks, depth <= 20 -> 21 cells | 168 | slot = `[sp,#80+8*depth]`, emit_while_stmt:4317 `ldepth <= 20`, patch words 4177527790 / 4181722094 + (10+ldepth)*1024 |
| [sp+248, sp+256) unused | 8 | -- |
| [sp+256, sp+2304) x15 spill region, 256 slots | 2048 | `add x15,sp,#256` = 2432959471; bound `s0 + tsp0 > 256 -> diag_exit 89` at compile_fn_at_facts, bebop.bp:5082 |
| [sp+2304, sp+16384) frame heap, x14 bumps up | **14080** | `add x14,sp,#0x900` = 2435056622 (bebop.bp:3672 unconditional, :3713 when has_alloc, :1170 in emit_sys_clone) |

So the per-activation frame heap is **14080 bytes today, not 15616** (it was 15616 before A14 moved x14 from sp+0x300 to sp+0x900: 1536 bytes, not 1280, left the heap -- see the note in §8). The exit-81 bound is not a separate number: `emit_heap_trap` (bebop.bp:1964) computes it as `x15 + 12288 + 3840` = x15 + 16128 = sp + 16384, i.e. the top of the frame, and traps `brk #81`. Since the A4-round-2 bugfix that trap allocates its scratch through `vs_alloc` and frees it with `vs_mask_free` (5 emitted words, one call, four call sites: bebop.bp:438 / 525 / 540 / 3374) instead of hardcoding x2.

The frame heap lies on the machine stack, OUTSIDE the reserve, so `(sp - x17)` is a huge negative number and `lsr` of it is garbage. **A5 therefore moves the frame heap INTO the reserve**, and only then may aggregates be handed out as indices:

- `emit_prologue_sized` (bebop.bp:3699), when `has_alloc == 1`, emits the carve instead of the single `add x14,sp,#0x900`:
  `mov x14,x27 ; add x27,x27,#12288 ; add x27,x27,#1792 ; cmp x27,x28 ; b.ls +2 ; brk #80` -- **6 words**, because 14080 does not fit one imm12: an ADD immediate carries 12 bits either unshifted (<= 4095) or shifted left 12 (a multiple of 4096), so the bump splits 12288 + 1792, exactly the split `emit_heap_trap` already uses for 16128 = 12288 + 3840. Derive all six with as+objdump.
- `emit_epilogue_sized` (bebop.bp:3717) releases LIFO: `sub x27,x27,#12288 ; sub x27,x27,#1792` (2 words). Its signature today is `(insns, n, vc, cs_hi)` -- it must gain `has_alloc`, and the call site at bebop.bp:5175 must pass `fw_alloc(fw0)`.
- The T118 bound is no longer a constant offset from x15. `emit_heap_trap` must compare x14 against the region END, which is now a runtime value: store it once in the prologue into the free cell at `[sp,#248]` (the 8 bytes between the mark region and the x15 base, reserved above) and make the trap `ldr xt,[sp,#248] ; cmp x14,xt ; b.ls +2 ; brk #81` (4 words, still one vs_alloc scratch). Do not try to keep the `x15 + 16128` form: after the carve it is meaningless.
- The B1 arithmetic correction in `compile_fn_at_total_saved` (bebop.bp:5036-5051) MUST be updated in the same edit: `saved_x14 = 1 - real_alloc` (bebop.bp:5049) counts exactly one prologue word saved when the fn does not allocate. With the carve the emission pass emits 6 words where the unsized planning prologue emits 1, so the term becomes `saved_x14 = 1 - 6 * real_alloc`, and a new `saved_rel = 0 - 2 * real_alloc` term covers the epilogue release. Get this wrong and every `bl` offset in the image is wrong -- it is the single most likely RED in step 1.
- `emit_prologue` / `emit_epilogue` (the unsized planning pair, bebop.bp:3662 / 3677) keep today's exact words and counts. They are a throwaway buffer; only the correction above makes the two passes agree.
- T43 is untouched in structure: the mark is still x14 into `[sp,#80+8*depth]`, because x14 is still the aggregate cursor -- it just points into the arena now.

Consequences the worker must accept and write into the journal: (i) exit 81 SURVIVES A5 -- the region is still fixed-size, only its home moved, so ROADMAP A14b's two seeds stay red until A6; (ii) a has_alloc activation now costs 14080 arena bytes, so deep recursion in an allocating fn exhausts the 256 MiB arena (exit 80) at ~18 600 nested activations, where today it hits the stack guard (exit 82) first; c09_recursion / c26_selfrec must be re-run for this reason, not only for the LIFO check.

**Ordering constraint against A6 (2026-09-08).** The carve is coherent, but it is scaffolding with a two-commit lifetime: A6 deletes x14, the carve, the release and `emit_heap_trap` outright and lets aggregates bump x27 directly. A5 must therefore NOT be promoted as the end of a session with A6 unscheduled: land A5 and A6 step 1 back to back, in that order, in the same worktree. A5 may not claim `TRAP-81 = 0` in any form -- that acceptance line belongs to A6 (see A6 §0/§7), and A14b's seeds 100671/100828 are A6's gate, not A5's. If the main session ever needs A5 alone on main for longer than one session, say so explicitly in the journal line, because the box then carries a compiler in which a 14080-byte fixed region is charged to the arena instead of the stack for every allocating activation.

**Pointer census (step 0, python, parallel-safe).** Extend tools/typecheck.py (tools/typecheck.py:19 `is_ref`, verified) with two findings: (a) arithmetic on a `[i64]`-typed value other than `+ literal`/`- literal` cell stepping (which stays meaningful in index units -- but any `* 8`/`+ 8` BYTE arithmetic on an array value must be found and rewritten first); (b) an `[i64]` value stored into a store cell (LANG-DB §6 "absolute pointers creeping back"). Run over bebop.bp, selfhost/, bench/vs_rust/std_tests: expected 0 findings of class (b) (the store uses object-relative refs) and a short list of (a) to fix by hand BEFORE step 1 (they are bugs under the new model).

**bpref.** RE-CHECKED 2026-09-08 and the earlier reading was wrong: `tools/bpref.py:500-507` does NOT model the arena as one list. `zeros` returns a fresh python list (`return [0] * args[0]`) and the interpreter tracks only a running cell COUNT in `self.arena_cells` so it can raise `SystemExit(80)` at the 256 MB capacity. Converting arrays to indices in bpref is therefore real work, not a confirmation: introduce one `self.arena = []` cell list, make `zeros` extend it and return the starting OFFSET, and route every array read/write, `sys_export`, `sys_mmap` and builtin that takes cells through that offset. Keep the `arena_cells` capacity check exactly as it is (rc 80 is a gated fuzz class). Parity then holds for `a + 1` cell stepping and for exporting/re-importing the arena image.

**Invariants.** x17 never changes after the stub (clone children inherit it); every `ldr/str` of a cell has x17 as Rn or a register derived from it inside the same emitter; no `[i64]` value is ever an address (census + c92); the planning/emission word counts agree (the added words are unconditional per form, and the has_alloc-conditional ones are covered by the total_saved correction above).

## 4. Files and functions touched

Anchors are function names; the line numbers are bebop.bp md5 3c1b319f (2026-09-08) and will have drifted.

| file:fn | change | anchor |
|---|---|---|
| bebop.bp:entry_stub | reserve + x17 + arena inside it + file-map cursor cell | bebop.bp:5980 (the 131-word stub, T90) |
| bebop.bp:emit_zeros | index conversion on return; T118 arena trap kept | 5590 (trap 5608-5611) |
| bebop.bp:emit_array_get / emit_array_set | add + x17 forms | 3391 / 3410 |
| bebop.bp:emit_array_lit / emit_struct_lit / emit_enum_ctor / emit_enum_ctor_nullary / emit_field_access | index conversion after `mov x0,x14` (2853045216, at :3368 / :436 / :519 / :534); field access = `add t,x<s>,#f ; ldr [x17,t,lsl 3]` | 3328 / 409 / 511 / 531 / 452 |
| bebop.bp:emit_heap_trap | bound is a frame cell, not `x15 + 16128` | 1964 (call sites 438, 525, 540, 3374) |
| bebop.bp:emit_prologue_sized / emit_epilogue_sized / compile_fn_at_total_saved / compile_fn_at | x14 region carved from the arena when has_alloc; epilogue release; `saved_x14` / `saved_rel` correction; epilogue_sized gains has_alloc | 3699 / 3717 / 5036-5051 / 5175 |
| bebop.bp:emit_sys_* (read, write, readbuf, slurp, mmap, munmap, msync, export, rename/open paths, clone, cond_set, futex_*, atomic_add), emit_crc32/crc32x, emit_hvham/hvham2, str_to_cells, emit_sys_arena_base/end | index<->address words | grep `^fn emit_sys_`; arena_base/end at 1128/1138, clone at 1152 |
| tools/bpref.py | arrays as offsets into one arena list (see §3) | bpref.py:500-507 |
| tools/typecheck.py | census findings (a)/(b) | typecheck.py:19+ |
| tools/check_abi.py | allow the new stub/prologue words; x17 as a documented fixed register | -- |
| docs/LANGUAGE.md | Memory model: indices | -- |

## 5. Steps

0. Census (python) + reserve probe (.bp or python mmap) + fix the (a) findings by hand as a plain (non-codegen) commit if any.
1. Stub + x17 + zeros/array forms + aggregates-into-arena (the carve, the release, the rewritten heap trap, the total_saved correction) + builtin conversions + bpref, ONE chain commit (`--codegen`); constructs c69_index_roundtrip / c92_ptrfree; WORD_DELTA lines (every construct with array access grows: word_budget lines with the reason "A5 +1 add per access"); census_allow if b.cond moves (it should not).
2. Perf rows: K5, K6 (bench/tq_sqlite/run.sh), RSS; honest.sh unchanged (K1H-K8H have no arrays) -- report.
Leave uncommitted for the main session.

## 6. Constructs, oracles, twins

Construct numbers checked against `ls bench/parity_constructs/` on 2026-09-08: c66, c70, c71, c72, c90, c91 are TAKEN. c69 is free; the old "c70_ptrfree" of this blueprint would have collided with c70_csel, so it is c92_ptrfree.

| construct | source | EXPECT | exercises |
|---|---|---|---|
| c69_index_roundtrip | `zeros(64)`, fill i*i, `sys_export` the arena range to a temp file, `sys_mmap` it back (lands at a different index), read back through index+offset and fold; also `let b = a + 3; b[0]` cell stepping | bpref | index arithmetic, export/mmap inside the reserve |
| c92_ptrfree | a program storing an array value into another array and reading through it (`t[0] = a; t[0][2]`) -- valid under both models; plus the census run as a battery lane: `python3 tools/typecheck.py --ptr-census bebop.bp selfhost bench/vs_rust/std_tests` must print 0 findings | bpref / census | no address escapes into cells |
| c33_loopalloc / c34_loopescape / c40_struct | re-frozen | -- | frame heap inside the arena + T43 reset |
| c09_recursion / c26_selfrec | re-frozen | -- | LIFO release of the carved region; the exit-80-instead-of-82 change above |

Twins: K6 (bench/tq_sqlite/run.sh, nn.bp/nnidx.bp) before/after; store gates G1-G8 via std_golden (sbench.sh, sgraph.sh rows optional).

## 7. Gates

- `PROC_CAP=30 BEBOP_TMP=$OUT tools/chain.sh bebop.bp $OUT --codegen`: GREEN.
- docs/PERF.md: selfcompile_wall <= 1.08 x the A3 row; bin_words growth budgeted (expect +3-6 %: one add per array access in bebop.bp itself).
- `bash bench/tq_sqlite/run.sh` (BEBOP_BIN set): nn ms/row <= 1.2 x the A3 value; folds unchanged.
- `bench/vs_rust/scrash.sh` (G5) and sbench.sh rows unchanged in value (store correctness under the new base).
- NOT a gate here: `TRAP-81 = 0`. Exit 81 still exists after A5 (§3); it is A6's acceptance line.
- RED: SIGSEGV in a builtin = a missing index->address conversion (probe: the construct that calls that builtin); a store gate value change = a `str`/`[i64]` confusion (paths are `str` -> raw; buffers are cells -> index); a wrong `bl` target or an immediate SIGILL = the `saved_x14`/`saved_rel` correction in compile_fn_at_total_saved.

## 8. Risks and probes

| risk | probe | symptom |
|---|---|---|
| 4 GiB PROT_NONE reserve refused by the kernel/proot | step 0 probe | fallback 1 GiB (2^27 cells), journal it |
| the two passes disagree on the has_alloc prologue/epilogue word count | c01 upward; any construct calling an allocating fn | SIGILL / wrong `bl` target -- fix compile_fn_at_total_saved |
| a builtin's hand words still use a raw cells pointer | grep every `emit_sys_*` for the delivered x0..x2 uses; construct per builtin family (c14_string, c42_crc32, pool_parity) | SIGSEGV/garbage |
| frame heap in the arena leaks across recursion | epilogue releases 14080 LIFO; c09_recursion, c26_selfrec | exit 80 after deep recursion |
| the region size is wrong (14080, NOT 15616 and NOT 16128) | c33_loopalloc allocating up to the old and new bounds | exit 81 earlier/later than today = a semantics change nobody asked for |
| clone child inherits x17 but its own x14 region | emit_sys_clone (bebop.bp:1152) re-homes x15/x14/x27/x28 today (words 2432959471 / 2435056622 at :1169-1170) -- x17 unchanged, the x14 word becomes the carve; pool_parity lane | child crash |
| `a + 8`-style byte arithmetic in the corpus | census (a) | wrong cell |
| bpref still returns list objects for arrays | bpref.py:500-507 | parity RED on c69 |

Note for the main session: ROADMAP row A14b says A14's wider slot region "takes 1280 bytes out of the 16 KiB frame". The verified figure is **1536** (x14 sp+0x300 -> sp+0x900 = 768 -> 2304; the slot region 512 -> 2048 B). The frame heap went 15616 -> 14080 B.

## 9. VERDICT format

```
VERDICT: GREEN|RED
reserve: 4GiB|1GiB (probe result)
census: findings (a) fixed: <n>; (b): 0
fixpoint: <md5>; bin_words <b> -> <a> (+%); word_budget lines: <n>
frame heap: carved <n> bytes/activation from the arena; total_saved correction verified by <construct>
K5: <before> -> <after> (gate +8 %)   K6 nn ns/row: <before> -> <after> (gate +20 %)
constructs: c69/c92 EXPECT + WORD_DELTA; c33/c34/c40/c09/c26 re-frozen
store gates: std_golden 99/99; scrash 100/100
exit 81: still present (expected; A6 deletes it)
journal: <line>
open: <anything>
```

## 10. Worker prompt skeleton

<context> repo, this blueprint, HEAD, the register model facts, the stub facts (read entry_stub first), the post-A14 frame table in §3, harness commands and traps, x17/x18 free (verified), the census tool. </context>
<constraints> `str` stays a raw pointer; one chain commit for step 1; every index<->address word derived with as+objdump into $OUT/words.objdump; no change to the bl pairing; the compile_fn_at_total_saved correction lands in the SAME edit as the carve; leave uncommitted. </constraints>
<output_format> §9. </output_format>
<task> A5 steps 0-2; report. </task>
