Status: 2026-09-06, owner main session (Fable fork), grounded at HEAD b1c0175 with the A1 worker tree; depends on A1 (register model: SYM/REG tags, register-parameterised ldr/str forms in emit_array_get/set), A2 step 0 (fn cap 512), A3 landed (LIN) -- ROADMAP order A5 after A3

# A5 Pointer-free step 1: one address space, `[i64]` values are x17-relative indices

## 0. Goal

Every `[i64]` VALUE in data is an integer index of a cell in ONE reserved address space whose base lives in x17; addresses exist only inside the instruction that reads or writes (`ldr xd,[x17,xt,lsl #3]`). Gates: chain GREEN with constructs c69_index_roundtrip / c70_ptrfree; K5 <= +8 % (docs/PERF.md selfcompile_wall vs the A3 row); K6 nn.bp scan ns/row <= +20 % (bench/tq_sqlite/run.sh); std_golden 99/99 (the store's G1-G8 gates run there); RSS unchanged.

## 1. Scope

In: the reserve + x17 setup in the entry stub; `zeros` returning an index; array get/set/literal forms; every `sys_*`/builtin that takes or returns cells converts index<->address inside its own words; `sys_mmap` placing file maps INSIDE the reserve so store bases are indices too; bpref: arrays as indices (it already models the arena as one list -- tools/bpref.py:498 `zeros`, verified -- so an array value can become an integer offset into that list); a python census (step 0). Out: `str` values (raw pointers until A7); the frame heap x14 (A6); typed tables (A8); u32 indices (A8). Fixed points: emit_bl, prologue/epilogue words (x17 is not saved/restored: it is a process constant like x27/x28, set once by the stub, re-homed for clone children exactly like x27/x28 -- bebop.bp:1130 comment, verified); the store's on-disk format (refs are object-relative offsets already, LANG-DB §4b); every construct's VALUE.

## 2. Preconditions

A1-A3 promoted. x17/x18 are unused: `grep -n 'x17\|x18' seed/seed.S` is empty (verified 2026-09-06) and the register model uses x16 only as the parallel-move scratch (REGISTER-MODEL §1.1); the stub's arena setup is in `entry_stub` (bebop.bp, the 131-word stub; T90) -- the worker reads it before step 1. `sys_arena_base` builtin exists (bebop.bp:1135, verified). The T118 arena trap words in emit_zeros (`cmp x27,x28 ; b.ls ; brk #80`, bebop.bp emit_zeros, verified) stay.

## 3. Design

**Address space.** At start the stub reserves R = 4 GiB (2^32 bytes = 2^29 cells; the u32 index space A8 will need) with `mmap(NULL, R, PROT_NONE, MAP_PRIVATE|MAP_ANONYMOUS|MAP_NORESERVE)`, then commits the arena as today INSIDE it: `mmap(base, 256 MiB, PROT_READ|PROT_WRITE, MAP_FIXED|...)` (or mprotect). x17 = base; x27 = base (bump cursor), x28 = base + 256 MiB (end) as today. File maps: `sys_mmap` takes the next slot from the top of the reserve downward (a cursor cell in the stub's data area or a fixed register? use a cell at `[x17 - 8]`? no -- keep it simple: the stub keeps the next-slot address in a cell inside the first arena page, reserved by the stub: `[x17,#0]` = next file-map address (cells 0..15 of the arena are the stub's, `zeros` starts after them; index 0 stays "null" for `ref` semantics) and maps with MAP_FIXED there. Step 0 probes that a 4 GiB PROT_NONE reserve succeeds under this proot (a 20-line .bp calling sys_mmap with the flags, or python `mmap` with the same flags); fallback 1 GiB (2^27 cells) if it fails, recorded in the journal.

**Index = (address - x17) >> 3.** `zeros(n)`: today's words allocate at x27 and return the address in x0; add `sub x0,x0,x17 ; lsr x0,x0,#3` before the push (2 words; emit_zeros verified). Array get/set (emit_array_get bebop.bp:3010, emit_array_set :3029, verified): base entry is an INDEX; dynamic index: `add xt,x<base>,x<idx> ; ldr xd,[x17,xt,lsl #3]` (t = vs_alloc, freed); constant index c: `add xt,x<base>,#c ; ldr xd,[x17,xt,lsl #3]` (c < 4096) -- 2 words instead of 1 (the +1 add of RESEARCH-NOPOINTERS §1.1; the flat IR's LICM removes it later); same for `str`. Array literal / struct literal / enum ctor (A6 moves them; in A5 they still allocate on x14 and hand out a POINTER -- so A5 must convert THAT too: after `mov x0,x14 ; add x14,...` emit `sub x0,x0,x17 ; lsr x0,x0,#3` so aggregates are also indices (the frame heap lies inside the 16 KiB frame on the machine stack -- OUTSIDE the reserve, so (sp - x17) is a huge negative number: `lsr` of a negative is wrong. Therefore the frame heap must move INTO the reserve now, not in A6: the stub gives each activation... no -- simplest: A5 step 2 = the first half of A6: the frame heap becomes an arena-resident bump region per activation? That is A6's whole content. Decision: **A5 makes the frame heap x14 point into the arena**: the prologue's `add x14,sp,#1024` becomes `mov x14,x27`-style allocation of a 15.6 KiB region from the arena at fn entry when has_alloc (`mov x14,x27 ; add x27,x27,#16128 ; cmp x27,x28 ; b.ls ; brk #80`), released at the epilogue (`sub x27,x27,#16128` -- LIFO, since activations nest) and by the T43 back-edge reset exactly as today (the mark stores x14). Aggregates then live in the reserve and `(x14 - x17) >> 3` is a valid index. Cost: has_alloc fns +4 prologue words, +1 epilogue word; the 16 KiB frame's `sub sp` stays until A6 removes it. This also makes `escape` semantics identical to today (region dies at ret).)
`sys_*` and builtins with cell arguments convert on entry: `add x<a>,x17,x<a>,lsl #3` per cell argument (1 word each; the arguments are already delivered to x0..x2 by vs_deliver), and builtins that RETURN cells (zeros, sys_readbuf, sys_slurp, sys_mmap, str_to_cells) convert on exit (2 words). `sys_arena_base()` returns 16 (the first index after the stub cells) and `sys_arena_end()` the end index. `sys_clone(flags, stack_top)`: stack_top is an index -> converted. Futex/atomic builtins: converted. `char(s,i)`, `str_len`: `str` stays a raw pointer (A7).

**Pointer census (step 0, python, parallel-safe).** Extend tools/typecheck.py (tools/typecheck.py:19 `is_ref`, verified) with two findings: (a) arithmetic on a `[i64]`-typed value other than `+ literal`/`- literal` cell stepping (which stays meaningful in index units -- but any `* 8`/`+ 8` BYTE arithmetic on an array value must be found and rewritten first); (b) an `[i64]` value stored into a store cell (LANG-DB §6 "absolute pointers creeping back"). Run over bebop.bp, selfhost/, bench/vs_rust/std_tests: expected 0 findings of class (b) (the store uses object-relative refs) and a short list of (a) to fix by hand BEFORE step 1 (they are bugs under the new model).

**bpref.** Arrays are already positions in one list (bpref.py:498-506 verified: `zeros` appends to the arena list and returns its offset -- confirm; if it returns a python list object, change it to return the offset and index the global list). Parity then holds for `a + 1` cell stepping and for exporting/re-importing the arena image.

**Invariants.** x17 never changes after the stub (clone children inherit it); every `ldr/str` of a cell has x17 as Rn or a register derived from it inside the same emitter; no `[i64]` value is ever an address (census + c66); the planning/emission word counts agree (the added words are unconditional per form).

## 4. Files and functions touched

| file:fn | change | anchor |
|---|---|---|
| bebop.bp:entry_stub | reserve + x17 + arena inside it + file-map cursor cell | `fn entry_stub` (grep; the 131-word stub, T90) |
| bebop.bp:emit_zeros | index conversion on return | emit_zeros (verified body) |
| bebop.bp:emit_array_get / emit_array_set / vs_array_get_reg / vs_array_set_reg | add + x17 forms | bebop.bp:3010, 3029 |
| bebop.bp:emit_array_lit / emit_struct_lit / emit_enum_ctor(_nullary) / emit_field_access | index conversion after `mov x0,x14`; field access = `add t,x<s>,#f ; ldr [x17,t,lsl 3]` | bebop.bp:403, 450, 509, 533, 2953 |
| bebop.bp:emit_prologue_sized / emit_epilogue_sized / compile_fn_at facts | x14 region from the arena when has_alloc; epilogue release | bebop.bp:3316, 3334 |
| bebop.bp:emit_sys_* (read, write, readbuf, slurp, mmap, munmap, msync, export, rename/open paths, clone, cond_set, futex_*, atomic_add), emit_crc32/crc32x, emit_hvham/hvham2, emit_str_len? (no: str), str_to_cells, emit_sys_arena_base/end | index<->address words | bebop.bp:1011-1490 (grep `^fn emit_sys_`), 1135 |
| tools/bpref.py | arrays as offsets | bpref.py:498-506 |
| tools/typecheck.py | census findings (a)/(b) | typecheck.py:19+ |
| tools/check_abi.py | allow the new stub words; x17 as a documented fixed register | -- |
| docs/LANGUAGE.md | Memory model: indices | -- |

## 5. Steps

0. Census (python) + reserve probe (.bp or python mmap) + fix the (a) findings by hand as a plain (non-codegen) commit if any.
1. Stub + x17 + zeros/array forms + aggregates-into-arena + builtin conversions + bpref, ONE chain commit (`--codegen`); constructs c65/c66; WORD_DELTA lines (every construct with array access grows: word_budget lines with the reason "A5 +1 add per access"); census_allow if b.cond moves (it should not).
2. Perf rows: K5, K6 (bench/tq_sqlite/run.sh), RSS; honest.sh unchanged (K1H-K8H have no arrays) -- report.
Leave uncommitted for the main session.

## 6. Constructs, oracles, twins

| construct | source | EXPECT | exercises |
|---|---|---|---|
| c69_index_roundtrip | `zeros(64)`, fill i*i, `sys_export` the arena range to a temp file, `sys_mmap` it back (lands at a different index), read back through index+offset and fold; also `let b = a + 3; b[0]` cell stepping | bpref | index arithmetic, export/mmap inside the reserve |
| c70_ptrfree | a program storing an array value into another array and reading through it (`t[0] = a; t[0][2]`) -- valid under both models; plus the census run as a battery lane: `python3 tools/typecheck.py --ptr-census bebop.bp selfhost bench/vs_rust/std_tests` must print 0 findings | bpref / census | no address escapes into cells |
| c33_loopalloc / c34_loopescape / c40_struct | re-frozen | -- | frame heap inside the arena + T43 reset |

Twins: K6 (bench/tq_sqlite/run.sh, nn.bp/nnidx.bp) before/after; store gates G1-G8 via std_golden (sbench.sh, sgraph.sh rows optional).

## 7. Gates

- `PROC_CAP=30 BEBOP_TMP=$OUT tools/chain.sh bebop.bp $OUT --codegen`: GREEN.
- docs/PERF.md: selfcompile_wall <= 1.08 x the A3 row; bin_words growth budgeted (expect +3-6 %: one add per array access in bebop.bp itself).
- `bash bench/tq_sqlite/run.sh` (BEBOP_BIN set): nn ms/row <= 1.2 x the A3 value; folds unchanged.
- `bench/vs_rust/scrash.sh` (G5) and sbench.sh rows unchanged in value (store correctness under the new base).
- RED: SIGSEGV in a builtin = a missing index->address conversion (probe: the construct that calls that builtin); a store gate value change = a `str`/`[i64]` confusion (paths are `str` -> raw; buffers are cells -> index).

## 8. Risks and probes

| risk | probe | symptom |
|---|---|---|
| 4 GiB PROT_NONE reserve refused by the kernel/proot | step 0 probe | fallback 1 GiB (2^27 cells), journal it |
| a builtin's hand words still use a raw cells pointer | grep every `emit_sys_*` for the delivered x0..x2 uses; construct per builtin family (c14_string, c42_crc32, pool_parity) | SIGSEGV/garbage |
| frame heap in the arena leaks across recursion | epilogue releases `sub x27,#16128` LIFO; c09_recursion, c26_selfrec | exit 80 after deep recursion |
| clone child inherits x17 but its own x14 region | emit_sys_clone re-homes x14/x27/x28 today (bebop.bp:1130) -- add x17 unchanged; pool_parity lane | child crash |
| `a + 8`-style byte arithmetic in the corpus | census (a) | wrong cell |
| bpref returns list objects for arrays | bpref.py:498 | parity RED on c65 |

## 9. VERDICT format

```
VERDICT: GREEN|RED
reserve: 4GiB|1GiB (probe result)
census: findings (a) fixed: <n>; (b): 0
fixpoint: <md5>; bin_words <b> -> <a> (+%); word_budget lines: <n>
K5: <before> -> <after> (gate +8 %)   K6 nn ns/row: <before> -> <after> (gate +20 %)
constructs: c65/c66 EXPECT + WORD_DELTA; c33/c34/c40 re-frozen
store gates: std_golden 99/99; scrash 100/100
journal: <line>
open: <anything>
```

## 10. Worker prompt skeleton

<context> repo, this blueprint, HEAD, the register model facts, the stub facts (read entry_stub first), harness commands and traps, x17/x18 free (verified), the census tool. </context>
<constraints> `str` stays a raw pointer; one chain commit for step 1; every index<->address word derived with as+objdump into $OUT/words.objdump; no change to bl/prologue pairs; leave uncommitted. </constraints>
<output_format> §9. </output_format>
<task> A5 steps 0-2; report. </task>
