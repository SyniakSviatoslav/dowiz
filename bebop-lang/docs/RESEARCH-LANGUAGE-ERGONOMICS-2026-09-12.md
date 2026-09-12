Status: 2026-09-12, research pass (read-only; NO compile, NO gate, NO chain, nothing written under
/root/dowiz). Ground truth is HEAD `d75172c` (2026-09-12 20:54 +0200, `git log -1`), working tree
DIRTY: `M ROADMAP.md`, `M bebop.bin`, `M bench/fuzz/gen.py`, `M tools/bpref.py` (+11 lines: the `set`
fix of journal 1789244400), 90 `bench/parity_constructs/frozen/*.bin` and `docs/exp.journal` (+2). Every
claim below carries a `path:line` I read or a `git log` line I ran; where a roadmap row and the tree
disagree, the tree wins, and the disagreement is listed in §1.

# Language ergonomics: ten required items, what the tree says, and the executable plan

## 0. What this document decides

Ten items became REQUIRED features on 2026-09-12. This document (a) verifies the evidence behind each,
(b) names the exact functions in `bebop.bp` each one touches, (c) gives the cheapest design that is
correct for a ONE-PASS self-hosting compiler with its cost in fns / words / gates, (d) says what each
breaks, (e) answers the operator's three open questions (§7), and (f) re-judges every item for the
PRIMARY audience, which is an agent, not a human (§6). The blueprints are in
`docs/blueprints/A16-closures-generics-hof.md` and `A17`..`A24`; the roadmap changes are in
`ROADMAP-PATCH.md`; the one-page verdict is `VERDICT.md`.

Numbers used throughout (all re-derived today):

| fact | value | where |
|---|---|---|
| compiler | `bebop.bp` 7,720 lines, **295** `fn`, fn cap **768** | `grep -c '^fn '`; `bebop.bp:4410`, `:5893` |
| promoted binary | 43,491 words, bcond 1214 / cbz 124 / tbz 0 | `bench/vs_rust/census.txt:2` |
| constructs | 89 positive + 15 `neg/` = **104**, EXPECT table 260 lines | `ls bench/parity_constructs`, `construct_parity.sh:49-238` |
| std gates | **117** | `grep -c '^gate ' bench/vs_rust/std_golden.sh` |
| chain wall / cpu | 104-139 s / 54-82 s (CG=1, with battery) | `docs/PERF.md:9-10`, `bench/perf.csv:1418,1473` |
| self-compile | 1.5-1.7 s cold, 0.07 s `.becache` hit | `ROADMAP.md:290` |
| one std-gate compile | 346 / 113 / 106 ms cold / warm / floor | `ROADMAP.md:291` |
| slots | **1** heavy job on the box, global flock | `tools/slot.sh:35-37` |
| bpref | 736 lines; 9 program-specific shims; `sys_*`/`clock_ms` stubbed to 0 | `tools/bpref.py:576-620`, `:669` |
| third semantics | `formal/Bebop/Semantics.lean` 607 lines (F4, never run on-box) | `wc -l formal/Bebop/*.lean` |
| params | 14 max, 9-14 in x8..x13; symbols 1-8 in x19..x26, 9+ spilled to `[x15,#k*8]` | `parse_params:297`, `sym_bind:242-247` |

---

## 1. Premises verified against the tree

Legend: CONFIRMED = read in the tree at the cited line; CLAIMED-ONLY = asserted in a document or
prompt, no code/journal evidence found; NOT-IN-TREE = contradicted by the tree.

| # | premise (from the brief) | verdict | evidence |
|---|---|---|---|
| 1a | `c70_qdsl.bp` is 339 lines and packs `(p << 32) \| node` by hand | CONFIRMED | `wc -l` = 339; pack at `:42,46,61,65,101,114,117,152`, unpack `& 0xffffffff` at `:77,98,109,118,145,191,198,204,211,231,248,257,277,295` |
| 1b | the idiom is general, not one file | CONFIRMED, wider | `<< 32` packing at **75** sites in 12 selfhost files (`store.bp`, `fp.bp`, `sha256.bp`, `gb_run.bp`, `sgraph2.bp`, `qdsl.bp`, ...); `& 0xffffffff`/`4294967295` at **68** sites |
| 2a | everything is `[i64]` with hand offsets `a[node+3+ci]` | CONFIRMED | `c70_qdsl.bp:26-30`; `store.bp:862` `st_get = base[obj + 2 + i]`; PartTab cells 16/17/18 hard-coded in `st_parttab_root/used/gen` (`store.bp:269-283`); 34 `base[... + 1x]` sites in `store.bp` |
| 2b | B5 step 1's 21-cell PartTab turned FIVE gates red on baked constants | PARTLY CONFIRMED | +21 constant class: **four** gates -- slayout (journal 925), schain (930, "+21*1000"), sevolve (934, "exactly 21"), scompact (935, "21"). scrash_torn (943/944) was red on a **harness-model** divergence (root-cell formula, tear application, root_p), not on a +21 constant. The costlier layout defect was the **+2 object header**: `st_parttab_write` wrote from `base[pt_off+0]` over the header (journal 924) and `tools/stdump` read from `parttab_off` instead of `+2` (journal 948) -- the same off-by-two twice in one day |
| 2c | structs exist | CONFIRMED, narrower than documented | `emit_struct_lit` (`bebop.bp:447`), `emit_field_access` (`:500`), postfix `.` (`:4356-4368`) all resolve against the FIRST `struct` declaration only: `find_struct` (`:354`) returns the first, `struct_name_hash` (`:418`) is cached once in `fntab[5541]`, `field_index` (`:388`) searches that one. `c40_struct` has one struct; `c10_struct` contains **no struct at all** (it tests call composition, `c10_struct.bp:1-4`). The compiler itself declares no `struct`/`enum` (`grep -n '^struct \|^enum ' bebop.bp` = 0). `docs/LANGUAGE.md:16` says "literals disabled (T43 rest)" -- stale, they are enabled under the guard at `emit_ident:315-330` |
| 3a | `qdsl_query` writes `let _ = if ok == 0 then 0 else 0;` where a return was meant | CONFIRMED | 15 occurrences in `c70_qdsl.bp:190-303`; 24 more in `selfhost/std/qdsl.bp`; 70 tree-wide in 4 files |
| 3b | `c70_qdsl_neg` gets 1 where its header asks for 89 | CONFIRMED | header `c70_qdsl_neg.bp:1-2` ("returns 89", "returns 0 for invalid input"); `construct_parity.sh:158` `EXPECT=1`; journal 1789244500 |
| 3c | `return`/`break` exist but the author could not use them | CONFIRMED, cause found | `return`/`break` are recognised ONLY as body items (`emit_body_classify:5481-5484`, dispatch `emit_body:5518-5521`); `if` is an EXPRESSION whose arms go through `emit_cmp` into one register `d` (`emit_cond_branch:3780-3892`), so a `return` inside an arm is parsed as an identifier and exits 101. Written down independently in `selfhost/tcheck_kernel.bp:13-15`. Caps: 17 `return` / 18 `break` per fn, exit 98 (`emit_return_stmt:4715`, `emit_break_stmt:4729`) |
| 4a | bpref's `set` node copied the array; `poke` gave 0 vs 9 | CONFIRMED, fix UNCOMMITTED | `tools/bpref.py:536-547` (comment names the defect); `git status` shows `M tools/bpref.py`; journal 1789244400. Not in any commit yet |
| 4b | every feature costs two implementations | CONFIRMED, it is FOUR | `bebop.bp` (emitter), `tools/bpref.py` (evaluator, 736 lines), `formal/Bebop/Semantics.lean` (607 lines, F4), `tools/typecheck.py` (types, 177 lines) -- plus `bench/fuzz/gen.py` (399 lines) which must learn every shape. bpref carries **9 program-specific builtins** (`qdsl_*`, `bpref.py:576-620`) that are dead for `c70` only because user fns take precedence at `:573` |
| 4c | the second implementation rots quietly | CONFIRMED, and why | bpref's EVALUATOR is executed by NO battery lane: `tools/battery.sh` runs it nowhere, `invariants.sh:53` uses only its parser (typecheck), 1 of 128 oracles imports it, and fuzz batches "run LAST" (ROADMAP A4). It rotted from `c99cddb` (2026-09-11) to 2026-09-12 unobserved |
| 5a | errors are exit codes; diag text is i64 cells, one char per cell | CONFIRMED for compile-time | `diag_exit` `bebop.bp:75-107`, 11 codes at `:92-102` as `[101, 120, ...]` literals; `cap_exit:2172-2183`; `selfcheck_exit:2205-2216` (exit 201). 33 `diag_exit(` + 21 `sys_exit(` + 20 `selfcheck_exit(` sites in the compiler |
| 5b | neither `grep` nor `strings` finds a message | HALF TRUE | `strings -n 8 bebop.bin` finds the **4 runtime** trap texts (80/81/82/87, in the entry stub) and **0 of the 11 compile-time** texts. The stub still ships `trap 81: frame heap exhausted`, a code A6 RETIRED (TRAPS.md row 81) |
| 5c | "what does 103 mean" | CONFIRMED as a class | TRAPS.md row 103 is 20 lines of history; exit-code space was unpartitioned until `bebop.bp:3607-3640` partitioned it (self-checks -> 201; free below 99: 65-79, 84-86; 65 now taken by F3 static bounds `:4232`). TRAPS.md rows 85/86/87 are **store.bp program** exit codes that overlap the compiler's free range -- different process, same number, an agent reading `rc=86` must know which |
| 6a | > 8 kept symbols across `sys_clone` loses children silently | CONFIRMED, mechanism found | journal 1789219189: 8 ok / 9 one child lost / 11 both, silent, TIDs valid. Mechanism: symbols 9+ live at `[x15,#k*8]`; the child **re-homes x15 to `sp+256` on a FRESH 12 MiB stack** (`emit_sys_clone:1296-1304`, words `:1305-1307`), so every spilled symbol reads as zero in the child. `arch_check.py:check_clone_kept_symbols (def at :523 at HEAD, :552 in the working tree -- the file is being edited)` CHECK 20 enforces `<= 8` statically (regex, conservative) |
| 6b | "spill slots live in a frame SHARED between parent and child ... stomp" | NOT-IN-TREE | `selfhost/std/pool.bp:6-7`: "spilled slots are per-thread stack memory and are NOT shared". The failure is the opposite: the child's frame is EMPTY, not shared |
| 6c | "b6core hung for 120 s" | **CONFIRMED after this pass** (main session, 2026-09-12): `bench/perf_fn/gates.txt` recorded `b6core_test 120199 miss rc=124` and a direct run returned rc=143; the join waited on a flag nothing set, fixed in `132063b`. Was CLAIMED-ONLY only because the journal line had not been written yet. | commit `05fcc0f` says b6core is "red for a correctness reason (ok=0 at W=1)"; no journal line or commit mentions a 120 s hang. The **60 s hang** in the tree is journal 1789233153 (`sys_exit` emitted EXIT not exit_group: OLD rc=124 at 60,260 ms, FIXED rc=80 in **516 ms**) |
| 7a | every codegen change costs chain + re-freeze + budget + census + battery, one slot | CONFIRMED | `tools/chain.sh:33-46`, `tools/battery.sh:33-46`, `construct_parity.sh:27-36` (FREEZE/WORD_DELTA/word_budget), `slot.sh` SLOTS=1 |
| 7b | "516 s" per chain | NOT-IN-TREE | measured chain wall is **104-139 s** (`docs/PERF.md:9`, `perf.csv:1418,1473`); commit `e5b64f7` reports 186 s. 516 is the **milliseconds** figure of journal 1789233153 |
| 8-10 | A16 REQUIRED since 2026-09-09; LANGUAGE.md:132 a defect | CONFIRMED | `ROADMAP.md:104`; `docs/LANGUAGE.md:130-138` |
| 8-10b | LANGUAGE.md: "SURFACE SYNTAX IS parsed and erased as of A16 Phase 1" | NOT-IN-TREE | added by `4286ec1` (2026-09-11) with `A16-parsing-annotations.md` ("IMPLEMENTATION BEGUN") and `tools/f8_dt.py`. `bebop.bp` has **0** matches for `annotation|generic|closure|requires|ensures|theorem`. `f8_dt.py` passes because `compile_fn_at:6041` `skip_to(s, pos, 123)` discards EVERYTHING between `)` and `{`, and `collect_fns:5870` scans only `fn ` so a top-level `theorem ...` line is never seen. It is a gate that measures parser laxness, exactly the failure class of 3a |
| cap | fn cap 768, ~300 fns | CONFIRMED | 295; TRAPS.md row 89 still says **512** (stale) |
| cap | 14 params, <= 8 kept across clone, 511-element literals, 8 MiB child arena | CONFIRMED | `parse_params:297`; CHECK 20; `LANGUAGE.md:80`; `emit_sys_clone:1308-1313` |
| honesty | `push_words == 0` is a gate | CONFIRMED | `invariants.sh:76-78` |

Two more things found that nobody asked about: (i) `tools/trap_census.py` today prints `trap_unrep:
11/30`, `shadowable_builtins: 1 of 38` -- the ROADMAP F1 cell (`10/29`, `7 of 36`) is stale; (ii)
`bench/parity_constructs/c10_struct.bp` is misnamed and gates nothing about structs.

---

## 2. Where a language feature lands in `bebop.bp` (the map every design below uses)

One pass, text-directed, no AST. Each expression emitter leaves exactly one window entry; each
statement emitter leaves none (`docs/REGISTER-MODEL-BLUEPRINT.md` §1.2).

| layer | functions (line) | what a feature touches |
|---|---|---|
| program scan | `collect_fns` 5870 (finds `fn NAME(`), `collect_ctors` 725, `find_struct` 354 (FIRST struct only), `scan_literals` 6133 | a new top-level form must be skipped here or it is silently ignored (the f8_dt lesson) |
| per-fn header | `compile_fn_at` 5982: reserved-word table 6004-6006, `parse_params` 272 (`skip_to_delim` discards types), `skip_to(... 123)` 6041 discards everything to `{` | return types / generic params / annotations are DISCARDED today |
| body items | `emit_body_classify` 5460 (`let`/`while`/`return`/`break`/compound/expr by first chars), `emit_body` 5499 (window-empty assert per item, `selfcheck_exit(8,1)`) | statement-level forms |
| statements | `emit_let_stmt` 5313 (+ F3 length side channel `fntab[5410+lk]` at 5324), `emit_while_stmt` 5139, `emit_return_stmt` 4708 (`vs_deliver(1)` -> x0, `b` placeholder in `fntab[5247..]`), `emit_break_stmt` 4725, `emit_compound_stmt` 5409 | multi-value `let`, conditional return |
| expressions | `emit_cmp` 3533 -> `emit_bor/bxor/band/shift` -> `emit_expr` 3565 -> `emit_term` 3590 -> `emit_factor` 4305 (keywords `if`/`let`/`match`, `[`, `"`, ident, number; postfix `.` loop 4356-4368) -> `emit_ident` 302 (call / struct-literal guard / index / var) -> `emit_call_or_ctor` 1831 (38-arm builtin ladder by name hash, then ctor, self, `bl`, unresolved) | new keywords go in `emit_factor`; new builtins in the ladder AND the reserved table AND `tools/builtin_surface.py` |
| `if` | `emit_cond` 3703 -> `emit_cond_csel` 3737 (pure arms) / `emit_cond_branch` 3780 (arms via `emit_cmp`, both delivered into register `d`, window state saved/restored at 3787-3790 / 3814-3818) | an arm is an expression; a `return` inside it does not exist |
| calls | `emit_bl_call` 807 (args -> `vs_park` + `vs_place_args` 873 -> `bl` -> push `REG x0` at 857), `vs_deliver` 935 (builtin operands -> x0..x(n-1)), `emit_bl` 628 (x15 save/restore), `emit_call` 5725 (unresolved callee, `brk #87`) | a second return register is `REG x1` pushed after the `bl`; x1..x7 are free there (`emit_bl_call:855-857`) |
| window model | `vs_push` 2752 (512-entry list at `fntab[3700..]`), `vs_pop` 2788, `vs_alloc` 2447, `vs_park` 2585, `vs_to` 2877, `vs_bind` 2924 (`str` to `[x15,#slot]` for symbols >= 9), `vs_mat_*` 2818 | zero-word features push CONST entries; anything else costs `mov`/`ldr` words |
| aggregates | `emit_array_lit` 4002, `emit_struct_lit` 447, `emit_enum_ctor` 563 -- all bump the ARENA cursor x27 (A6), LIFO-released at `ret`, reset at `while` back-edges when `loop_alloc_safe` 4632 says so; `emit_zeros` 6506 (permanent) | a struct VALUE is a cell index; a returned struct is an allocation per call |
| fn table | `fntab_lookup` 652 (name -> start offset), per-fn facts `fntab[2900+i]` (b1_facts), zone map `tools/check_abi.py:205-212`; free cells today: 5237-5239, 5285-5289, 5538-5539, 5601-5999, 7000-8191 (verify with `check_abi.py --fntab`) | return arity, struct tables, fn-value tables |
| diagnostics | `diag_exit` 75 (line:col + text from i64 arrays 93-103 -> `sys_write(2,...)`), `cap_exit` 2172, `selfcheck_exit` 2205, runtime traps = `brk #code` + the entry stub's SIGTRAP handler (`entry_stub` 6904, texts in the stub) | compile-time texts are compiler code (0 words in programs); runtime texts are in the stub (**every** binary, 104 `word_budget` lines if it moves) |
| self-hosting | `tools/chain.sh`: gen2 = old bin compiling new source; gen3 = gen2 compiling it; gen4 = gen3 compiling it; fixpoint = gen3 == gen4 | at the commit that ADDS a grammar form, `bebop.bp` must not USE it (the old promoted binary must still parse the new source); it may use it from the next commit on (§7 Q3) |

---

## 3. The ten items

Each: what is missing (evidence) / where it lands / cheapest one-pass design / cost / what it breaks /
interactions. Costs are estimates anchored to counts in the tree; "gate-days" = one worker day per
chain-gated commit on the one slot at 104-139 s per chain plus construct re-freezes.

### 3.1 Item 1 -- tuples / multiple return values (row A21, `docs/blueprints/A21-multi-return.md`)

**Missing.** A function returns one i64 in x0 (`compile_fn_at:6085` `vs_to(x0)`; `emit_return_stmt:4713`
`vs_deliver(1)`). Two values are packed into one cell by hand: 75 `<< 32` sites, 68 mask sites (§1).
The pack loses range (32-bit halves) and the unpack is written at every consumer (`c70_qdsl.bp:77,98,...`).

**Lands in.** `emit_return_stmt` (deliver k operands), `compile_fn_at` tail (a `(a, b)` tail
expression), `emit_paren` 3657 (a `,` after the first expression = tuple literal, only in tail/return
position), `emit_bl_call:857` (push `REG x0`, `REG x1`, ... after the `bl`), `emit_let_stmt` (`let (a, b)
= call;` destructuring: bind top-down in reverse), `collect_fns` (record the declared return arity from
`-> (` so a call site can be checked), `tools/bpref.py` (`tuple` node, `let-tuple`), `tools/typecheck.py`
(arity), `bench/fuzz/gen.py` (emit the shape), `formal/` (a rule).

**Cheapest correct design: a register calling convention, not a value.** `return (a, b)` and a tail
`(a, b)` deliver operands to x0..x(k-1) through the SAME `vs_deliver`/`vs_place_args` path builtins use
(`vs_deliver:935`): zero new machinery on the callee side. On the caller side, after `bl`, x1..x7 are free
and dead (`emit_bl_call:862-863`), so pushing `REG x1` is legal in the model and costs **0 words**; `let
(p, node) = f(...)` pops two entries into two symbols (two `mov`/`str`, the same words a `let` costs
today). `emit_bl` saves/restores only x15 (`:628-651`), so x1 survives the return. The callee's epilogue
restores x19-x26/fp/lr only (`emit_epilogue_sized:4551`), so x1 survives that too. k <= 7 by the window;
propose k <= 3 as the documented cap (x0..x2), exit with a diagnostic above it.

A tuple is NOT a first-class value: it cannot be stored in an array, passed as an argument, or bound to
one name. That is what keeps it one-pass and zero-word; a stored pair is a struct (item 2).

**Arity safety (agent-first).** The one-pass problem: at `let (a, b) = f(x)` the caller must know `f`
returns 2. The destructuring pattern SAYS 2, so the emitter needs no lookup to generate code; the risk
is a mismatch (f returns 1, x1 is garbage) which is silent. Fix at zero words: `collect_fns` runs before
any fn is compiled and can read `-> (i64, i64)` from the header text into a per-fn arity cell (a new
fntab zone, 768 cells); `emit_let_stmt` compares and exits with a new diagnostic. A callee whose
`return (a, b)` arity disagrees with its own header is diagnosed in `emit_return_stmt`.

**Cost.** ~120-180 lines, +3-4 fns (parse tuple tail, destructuring bind, arity record/check, bpref
node), +~150-250 `bin_words` in the compiler (the parser code), **0 words** in programs that do not use
it (WORD_DELTA 0 on all 104 constructs), 2 constructs (positive + `neg/` arity mismatch), 1 chain +
freeze. bpref +40 lines, typecheck +15, gen.py +20, Lean +1 rule. **~1 week, 2 gate-days.**
Two-stage bootstrap: the compiler may use `let (a, b) =` from the commit after it lands.

**Breaks.** Nothing frozen (constructs without tuples emit identical words). `emit_paren`'s exit 95 for
`(a, b)` outside tail/return position must stay a diagnostic, or the fuzzer's `(e)` shapes change.

**Interaction.** Does NOT depend on structs; structs do not replace it (§7 Q1). Composes with A16's
`call_fn` (same x0/x1 convention).

### 3.2 Item 2 -- structs with named fields and a derived layout (row A22, `docs/blueprints/A22-struct-layout.md`; adjacent to A8/F3)

**Missing.** (a) Only the FIRST struct in a program is usable (`find_struct:354`, `fntab[5541]`); with two
structs, the second's literal is parsed as a block and `.f` resolves against the first -- silently wrong.
(b) No `sizeof`/offset constants: layouts are hand-numbered (`store.bp:269-283`, PartTab "cell 16/17/18
for P=1"; `c70_qdsl.bp:26-30`). (c) No struct-typed symbols: `p.f` cannot know `p`'s struct without a type
tag, and types are discarded (`parse_params` -> `skip_to_delim`).

**Lands in.** `collect_ctors`-shaped `collect_structs` (new; a struct table in a free fntab zone: name
hash, decl pos, field count), `emit_ident` (`NAME.field` / `NAME.size` where NAME is a declared struct ->
push CONST; `NAME {` for ANY declared struct), `emit_field_access` (resolve against the symbol's struct
tag instead of the first struct), `emit_let_stmt`/`parse_params` (record a per-symbol struct tag: the F3
length side channel `fntab[5410+lk]` at `emit_let_stmt:5324` is the exact precedent -- a per-symbol cell
written on every `let` from the RHS text), `tools/bpref.py` (`structs` already keyed by name at
`bpref.py:222-233`; add offsets), `typecheck.py`.

**Cheapest correct design, in the order it pays:**

- **Step 1 -- layout constants, no inference (zero words).** `T.f` = the 0-based field index of `f` in
  `struct T`, `T.size` = its field count, both compile-time CONST pushes (`vs_push kind 1`, materialised
  only when consumed, folded into `add #imm` by `vs_try_addsub_imm:2997`). `a[i * Node.size + Node.child]`
  replaces `a[node+3+ci]`; `base[pt + 2 + PartTab.used]` replaces "cell 17". This is the whole PartTab
  defect class made nameable, and it works on `[i64]`-backed objects, store objects and CSR rows alike --
  which is the object model the thesis names ("persisted objects ARE the in-memory objects"). It does
  not need A8.
- **Step 2 -- any struct, typed symbols.** `NAME {` for every declared struct; a per-symbol struct tag
  from (i) a struct-literal RHS, (ii) a `: T` parameter type read in `parse_params` (today skipped), (iii)
  a `-> T` return type recorded by `collect_fns`; `p.f` resolves through the tag; a `.f` on a symbol with
  no tag is a diagnostic instead of "field of the first struct". This IS A8 step 1's "4th stab cell"
  (`docs/blueprints/A8-typed-tables-u32.md` §3) restricted to struct names -- write it as A8 step 1a so
  A8 inherits it rather than duplicating it.
- **Not proposed:** struct values in registers (T49 "records = register images", HISTORY.md:1744, OPEN)
  -- a struct is a cell index into an arena block (`emit_struct_lit:483-489`), and that is the store's
  layout too; changing it would change the store.

**Cost.** Step 1: ~80-120 lines, +2 fns, +~120 `bin_words`, 0 program words, 1 construct, 1 chain. Step
2: ~150-200 lines, +3 fns, stab widened 3 -> 4 cells (`zeros(385)` -> `zeros(513)` at
`compile_fn_at:6007` and every `stab[1 + 3*i]` site: `sym_lookup:172-174`, `sym_bind:249`), +~250
`bin_words`, 0 program words, 2 constructs (+1 `neg/`: `.f` on an untagged symbol). **Step 1 ~1 week,
step 2 ~2 weeks (it is A8's first step).** Two-stage bootstrap for the compiler's own use; the compiler
uses no structs today and need not.

**Breaks.** `c40_struct` (one struct) must stay byte-identical -- step 2's resolution for a first-struct
program must produce the same index. The `fntab[5541]` guard against `while i < n {` (`emit_ident:315-330`)
must be kept per struct name.

**Interactions.** Subsumes nothing of item 1 (§7 Q1). Step 2 = A8 step 1a; F3's `[T; n]` lengths and
this share the per-symbol side-channel mechanism. Store: `st_parttab_cells`/`_used`/`_gen`/`_root`
rewritten over `PartTab.*` with the six store gates byte-identical is the natural acceptance.

### 3.3 Item 3 -- early return / real control flow (row A18, `docs/blueprints/A18-early-return.md`)

**Missing.** `return e;` and `break;` exist as body items only (T99, `emit_body:5518-5521`). `if` is an
expression with a mandatory `else`, both arms delivered into one register (`emit_cond_branch`). So the
ONLY way to leave early on a condition is `if c then (return v) else ...`, which does not parse: `return`
in arm position is read as an identifier (`emit_factor:4344-4355` has no such keyword) -> exit 101, or
with an earlier compiler a silent dead symbol. The author of `qdsl_query` wrote a no-op instead, 15 times,
and shipped a parser that accepts garbage (`c70_qdsl_neg` = 1). `tcheck_kernel.bp:13-15` documents the
same wall and chose "branchless error propagation with uid 0 as an absorbing sentinel" -- a whole
error-handling style chosen because control flow was missing.

**Lands in.** `emit_factor` (recognise `return`/`break` keywords in expression position, exactly where
`if`/`let`/`match` are recognised at `:4316-4343`), a shared `emit_return_expr` that reuses
`emit_return_stmt`'s placeholder list (`fntab[5247..]`) and then pushes a dummy `CONST 0` entry so the
enclosing arm's `vs_to(d)` has an operand (1-2 dead words after an unconditional `b`); `emit_cond_branch`
already saves and restores window/mask/cs/slot state around each arm (`:3786-3789`, `:3813-3817`), which is
what makes an arm that never falls through safe for the other arm. `break` in an arm patches to the
enclosing loop's exit like the statement form (`patch_jumps(5265)`), landing on the T43 `x27` reset word
exactly as a statement-level `break` does.

**Cheapest correct design.** (1) `return e` / `break` as EXPRESSIONS of type "never", allowed anywhere
an expression is; (2) an agent-facing lint: `let _ = if c then K1 else K2;` where both arms are literals
and the value is discarded is a compile-time diagnostic (a free code below 99) -- it is the exact shape
that shipped the accepting parser, and a human placeholder idiom is not worth a silent no-op; (3) NOT
proposed now: statement-level `if c { ... }` without `else` (a block parser, ~200 lines) -- costed, not
scheduled; `if c then return x else 0` is explicit and an agent writes it without complaint.

**Cost.** (1) ~60-90 lines, +1-2 fns, +~80 `bin_words`, **0-2 words per use** in programs, 1 construct;
(2) ~30 lines in `emit_let_stmt`, +1 `neg/` construct, 1 diag text. **3-5 days, 1 chain.** Two-stage for
the compiler's own use (the compiler has 316 `let _ = if ... then ... else 0;` conditional-effect lines
that would read better as early exits, but it must not use the form in the landing commit).

**Breaks.** `d09_returns` (diag lane, exit 98 cap) unchanged. The 17/18 pending-jump caps become
reachable faster inside arms -- raise both to 64 (the "jumps" zone `fntab[5247..5284]` is 38 cells; move
it to the free 5601+ zone or cap-check). `bench/fuzz/gen.py:10` currently avoids return/break entirely;
after this row it should emit them in arm position, or the fuzzer never sees the new code.

**Interactions.** Unblocks B7 step 1's fix (`qdsl_query` rewritten with real early returns, then
`c70_qdsl_neg` re-derived to its header's 89 -- that number is B7's gate, not this row's). Interacts with
A21: `return (a, b)` in arm position uses the same path.

### 3.4 Item 4 -- one semantics, not two (row A23, `docs/blueprints/A23-one-semantics.md`)

The full analysis is §4. Summary: the language is implemented four times; the second (bpref) is the
fuzz judge and the source of most EXPECT values but is executed by no battery lane, so it rots until a
construct happens to be re-derived. "Generating bpref from the compiler's own tables" is **not
attempted** and costed at a table-driven rewrite of the front end (~2,000 lines, A12-class); the
scheduled form is **demotion with a measured subset**: bpref stays the differential judge over the
grammar it declares, refuses (`UNSUPPORTED:<form>`, a named exit) anything else, is RUN mechanically in
the battery over all 104 constructs, carries no program-specific shims, and every EXPECT in
`construct_parity.sh` names its derivation. Cost ~2 days, harness only.

### 3.5 Item 5 -- real diagnostics (row A17, `docs/blueprints/A17-diagnostics-as-data.md`)

**Missing.** Compile-time messages are i64 array literals inside `diag_exit` (`bebop.bp:92-102`): not
grep-able in source by their text as prose, not in the binary at all (`strings` finds 0 of 11), and every
new code costs an array + a length in a hand-summed `mlen` expression (`:103`) -- an off-by-one prints
garbage (A13 blueprint §4 warns exactly this). Runtime traps ARE text in the stub (4 found), including a
retired one. Output format today: `<line>:<col>: <text>` on stderr, exit code = the code
(`diag_check.sh:11-15` parses fields 1,2 with `cut -d:`) -- which is already machine-readable but carries
no code token and no file name. Exit-code space partitioned on 2026-09-09 (`bebop.bp:3607-3640`), but
TRAPS.md still lists 512 as the fn cap and rows 85-87 as store codes that alias the free compiler range.

**Lands in.** `diag_text` (take a `str`, copy with `char()` -- `str` literals are legal as arguments
today, `LANGUAGE.md:91`), `diag_exit` (one table: code -> literal), `cap_exit`, `selfcheck_exit`, and the
entry stub's trap texts (`entry_stub:6904`, every binary -- step 3 only). `tools/trap_census.py` already
derives the code census; extend it to assert code == text == TRAPS.md row.

**Cheapest correct design.** (1) Messages as string literals: `diag_text(buf, at, "expected `)` or
`in`")`; `str_len` gives the length, so `mlen` disappears. A 30-char message costs ~8 data words as a
literal against ~60-90 emitted words as an array literal (each element is a store + constant
materialisation, `emit_array_lit:4002`); the compiler SHRINKS by an estimated 500-800 words. (2) One
stable line format on stderr for every compiler exit: `<file>:<line>:<col>: error[E<code>]: <text>`,
the shape every agent already parses for gcc/rustc; JSON is costed and NOT chosen (escaping arbitrary
source text costs code in a language with no string ops, and buys nothing an agent's regex does not
already do). `<file>` is `argv[3]` in `cli_compile`, available. (3) Runtime traps: the stub texts gain
the same `error[E80]` token and drop the retired 81 -- this changes EVERY binary's words (the stub is
copied into each), so it is a separate commit with 104 `word_budget` lines, like A5's +41. (4)
`docs/TRAPS.md` regenerated from the table by `trap_census.py` (the census already refuses undocumented
codes), never edited by hand.

**Cost.** (1)+(2): ~80 lines changed, 0 new fns, `bin_words` -500..-800, 0 program words, the diag
lane (10 `diag_neg` + 15 `neg/`) re-frozen for the new format (their `// EXPECT line:col code` headers
stay valid, the parser in `diag_check.sh` changes 1 line). **1-2 days, 1 chain.** Single-stage
bootstrap (uses only existing grammar) and the compiler uses it immediately. (3): 1 chain + 104 budget
lines, ~1 day. (4): a python change, 0.5 day.

**Breaks.** `diag_check.sh:13` (`cut -d: -f1,2` still works if the file name is prefixed? No: field 1
becomes the file -- change to `-f2,3`). Every tool that greps `^<n>:<n>:` -- `bench/fuzz/fuzz.sh`
classifies by exit code only (`fuzz.sh:7`), unaffected.

**Interaction.** Every later row adds codes through this table. A16's `call_fn` range trap is a new
runtime text (stub change, step 3's budget).

### 3.6 Item 6 -- loud failures (row A19, `docs/blueprints/A19-loud-clone.md`)

**Missing.** A clone-spanning fn with > 8 kept symbols loses children silently (journal 1789219189).
The compiler's own check (exit 103) was withdrawn because it counted constants (TRAPS.md row 103);
`arch_check.py` CHECK 20 counts syntactically and conservatively (the fn body). Mechanism (§1 6a): symbol
9+ is a spill slot, the child's x15 is re-homed to a fresh stack. Second measured silent class: a child
thread's `sys_exit` used to be EXIT not exit_group (fixed, journal 1789233153) -- the unbounded-wait
check now exists (`arch_check` unbounded-wait, ratchet 0).

**Lands in.** `emit_sys_clone:1275` (the emitter knows `stab[0]` = symbols bound so far, and the planning
pass publishes `vc` per fn); `compile_fn_at_facts:5961` (per-fn facts); a new diag code; a `neg/`
construct; and for the correct-by-construction alternative, the child re-home words at `:1305-1313`.

**Cheapest correct design.** Step 1 (loud, 1-2 days): at every `sys_clone` site, if the fn's final
symbol count `vc > 8` (published fact, `fw_vc`) OR any spilled symbol is read after the spawn, exit with a
diagnostic naming the fn and the count. Counting `vc > 8` alone is conservative (it refuses a fn with
9 symbols of which the 9th is dead after the spawn) -- that is the right direction (CHECK 20's own
argument, `arch_check.py:519-522`), and a refused program is repaired by the `env: [i64]` shape B6's
blueprint §4 mandates (22 `env: [i64]` sites already use it). Step 2 (correct, ~1 week): the child copies
its parent's spill region before re-homing -- `S + tsp` slots are a published fact, so the copy is a
fixed loop of ~8 words per spawn site; then symbols 9+ read correctly in the child and the limit
disappears. Step 2 makes step 1's diagnostic unnecessary; land step 1 first because it is a day and B6 is
blocked on the silence, not on the limit.

**Cost.** Step 1: ~40 lines, +1 diag text, +1 `neg/` + 1 positive construct, +3 bcond in the compiler
(a `census_allow` line -- the same +3 the withdrawn check cost, `census_allow.txt` last row), 0 program
words. Step 2: ~60 lines, +8-12 words per `sys_clone` site (there are few: `pool.bp`, `smw.bp`, `nn4.bp`,
`b6core.bp`, `gb_run.bp`, `sconc.bp`), 3 constructs, 1 chain.

**Breaks.** Step 1 refuses any existing clone-spanning fn with > 8 symbols: `smw.bp` was rewritten to 4
(journal 1789220830); `b6core.bp`, `nn4.bp`, `sconc.bp`, `pool.bp`, `gb_run.bp` must be checked by the
worker BEFORE the diag lands (CHECK 20 at ratchet 0 says they pass today).

### 3.7 Item 7 -- a faster feedback loop (row A20, `docs/blueprints/A20-fast-lane.md`)

**Measured today.** Chain wall 104-139 s of which the three self-compiles are ~5 s (1.5-1.7 s each) and
the rest is the battery (`std_par.sh` 117 gates, 104 constructs compiled + run + `cmp`, `parity_driver`,
`pool_parity`, `run_all` 128 oracles, `invariants.sh` 9 rungs, census, `check_abi`, `diag_check`,
`check_words`, `f8_dt`) plus `perf.py` (~60 s, `chain.sh:50`, `PERF=0` skips). Serialised through one
slot. The gate memo (`std_golden.sh:27-54`) replays PASSes keyed by `.bin` md5 -- useless after a codegen
change because every `.bin` changes.

**Cheapest design (harness only, no compiler change).** A `a new `fast.sh` under `tools/` (TO BE CREATED by A20)` lane that a worker runs per
edit: `bebop.bin check` (T90 2b, `cli_check:7244`, syntax + diagnostics, no output file, ~0.1 s) on the
edited files; then gen2 only (one self-compile, ~2 s) and `construct_parity.sh` against gen2 WITHOUT
freeze (~104 x 0.1 s = ~10 s); `typecheck.py`; `check_words.py`. Target **<= 30 s wall**, printed as one
line `fast: pass=<n> fail=<n> <s> s`. It is a STRICT SUBSET and never promotes: the chain stays the merge
criterion. A second, cheaper win inside the chain: run `perf.py` only on promotion (`PERF=0` for the
worker's chain), saving ~60 s of 104-139. Not proposed: per-fn memoisation (A10, refuted then
re-examined -- its own row).

**Cost.** ~80 lines of bash, 0.5-1 day, no chain. Structured output for the battery (a `--json` summary,
~40 lines of python over the existing `line()` regexes in `battery.sh:47-63`) is optional and costed at
0.5 day; the existing `PASS name (golden)` / `FAIL <check>: msg` lines are already line-parseable.

### 3.8 Items 8, 9, 10 -- generics, closures, higher-order functions (row A16, `docs/blueprints/A16-closures-generics-hof.md`, superseding `A16-parsing-annotations.md`)

**Missing.** Nothing of A16 is in the tree (§1 8-10b). The existing blueprint plans annotation PARSING
first and defers emission; its "IMPLEMENTATION BEGUN" status is not backed by code. The prior costing
(`docs/RESEARCH-PROOF-DESIGN-2026-09-09.md` §3) established the two facts this design rests on: the
emitter has **0 `blr` sites** (`:9`) and a closure packs into one i64 as fn index + env cell index (`:81`).

**Cheapest correct design, in order (agent-first, §6):**

- **Step 1 -- function values and indirect calls (HOF; this is T50 "functions as cells", OPEN,
  HISTORY.md:1749).** `&f` is a compile-time constant: the fn's start offset in words, known to
  `fntab_lookup` at emission (planning pass: emit a fixed 2-word `movz/movk` so both passes agree on
  size). `call_fn(v, args...)` delivers `args` to x0..x(k-1) and `v` to x<k> through `vs_deliver(k+1)`,
  then `adr x16, #-(4*n[0])` (the image base: PC-relative, in range because the program cap is 262,144
  words = 1 MiB = `adr`'s reach, `cap_exit`), `add x16, x16, x<k>, lsl #2`, `blr x16`, with `emit_bl`'s
  x15 save/restore around it, result `REG x0` (and x1 under A21). **5-6 words per call site, 0 words
  elsewhere, zero allocation.** Position-independent, so it survives `sys_run` of a foreign image. A
  range check (`cmp x<k>, #words; b.lo; brk #<new>`) is +3 words per site and a new runtime trap text.
  bpref: `('fnref', name)` and `call_fn` by table. This gives `map`, `fold`, `sort_by`, a table-driven
  dispatcher, and lets `emit_call_or_ctor`'s 38-arm `if` ladder become a table in a later generation.
- **Step 2 -- closures as fn value + EXPLICIT environment.** A closure is `(fn value, env)` where `env`
  is an ordinary `[i64]` the caller builds -- an array literal (arena cursor, released at `ret`, reset at
  loop back-edges when `loop_alloc_safe` holds) or `zeros` when it must escape. `call_fn` passes `env`
  as the first argument by convention; the packed one-i64 form `(fn << 40) | env` is sugar over that with
  2 unpack words. **No capture inference.** This is the shape 22 sites in the tree already use
  (`env: [i64]`), it is the shape B6's blueprint §4 mandates for every parallel region, and it is what an
  agent writes reliably (§6). It trades away implicit capture and `|x| x + a` syntax; that is stated.
- **Step 3 -- generics as erasure.** `fn f[T](x: T) -> T` parses (`compile_fn_at` skips `[..]` after the
  name; `collect_fns` unaffected because it matches `fn NAME` then reads an ident); T is a type variable
  for `typecheck.py` (A8's oracle) and nothing else, because every value is an i64 or a cell index and
  `[T]` compiles to `[i64]` for every T. 0 words. Constructs prove a generic `map` over two element
  "types" runs unchanged.
- **Step 4 -- monomorphisation, only when codegen depends on T.** That happens the day A8 lands
  `[u32]` (4-byte loads). Then instantiation is textual, the `use_expand` shape (`bebop.bp:6869`), 8-12
  fns per RESEARCH-PROOF-DESIGN `:155`. Costed, **not scheduled** before A8 step 2.

**Cost.** Step 1: ~150 lines, +3 fns, +~200 `bin_words`, 5-9 words per use, 2 constructs + 1 `neg/`,
1 chain: **1-2 weeks**. Step 2: ~40 lines of sugar + 2 constructs: **3 days**. Step 3: ~30 lines +
typecheck rules: **3 days**. Step 4: 8-12 fns, ~2 weeks, after A8. Total scheduled: **~3-4 weeks**, which
is the month the operator expected; the capture-inference closure the human reader wants is what is left
out, and it is "costed at ~2 weeks (escape analysis over `loop_alloc_safe`, a lambda-lifting text pass)
and not scheduled".

**Breaks.** `blr` enters the census (0 today): add a `blr` column or an allow line -- `tools/census.py`
counts b/bl/ret, and a new instruction class must be visible to the honesty floor, not hidden. The
`brk #87` unresolved-call path is unchanged (`&f` of an undefined name is a compile-time 101).

**Interactions.** Needs A21 for multi-value returns through `call_fn`; needs A17 for its trap text;
needs A8 before step 4. F7's kernel cost is unchanged by steps 1-3 (they add no logic; the kernel is
CIC by fiat).

---

## 4. Item 4 in depth: what it takes to stop maintaining the semantics twice

**How many implementations there are.** Four executable ones plus a generator:

| artifact | lines | role | who runs it mechanically |
|---|---|---|---|
| `bebop.bp` | 7,720 | THE compiler; defines the language by what it accepts and emits | every gate |
| `tools/bpref.py` | 736 | "the executable grammar and semantics reference" (`LANGUAGE.md:1`); the fuzz DIVERGE judge; source of most hand EXPECTs | fuzz batches only (A4: run LAST); nothing in `battery.sh` |
| `formal/Bebop/Semantics.lean` | 607 | F4 definitional interpreter | nobody (off-box, never run) |
| `tools/typecheck.py` | 177 | T48 type census over bpref's AST | `invariants.sh:53` rung (vii) |
| `bench/fuzz/gen.py` | 399 | must learn every shape or the fuzzer never exercises it | fuzz |

**Why the second rotted.** `c99cddb` (B7 step 1) added `qdsl_*` shims to bpref (`bpref.py:576-620`) --
a second definition of nine user functions inside the ORACLE -- and copied the `arr = list(arr)` line
into them. Nothing ran bpref over the constructs; construct parity compares `bebop.bin` against a frozen
EXPECT, not against bpref (`construct_parity.sh:37-42`; the word `bpref` there is only in comments).
The disagreement surfaced when a human ran both sides by hand (journal 1789244400). Four days.

**Option A -- generate bpref from the compiler's own tables.** Not attempted. The compiler has no
tables to generate from: the grammar is 60-odd text-directed `emit_*` fns over 43 fntab bases at 315
sites, dispatching on first characters and name hashes. Generating an interpreter requires the grammar in
data form, i.e. a table-driven front end -- the A12 rung by another name (`docs/blueprints/A12-...`),
refuted for speed and ~2,000 lines. Costed at 2-3 weeks of bebop.bp-lane time plus a re-derivation of
every construct; **not scheduled**, because it buys "one semantics" only if the generator itself is not a
second semantics, and a table interpreter is one.

**Option B -- an interpreter mode inside `bebop.bin` sharing the parser.** Not attempted. The
emitters consume text and produce words; an evaluator would have to be a second walk of the same
grammar with the same 60 dispatch points. Same cost as A, in Bebop, against a 768-fn cap with 295 used.
Not scheduled.

**Option C -- demote bpref to a differential harness with a MEASURED subset (chosen).**

1. Commit the `set` fix (it is uncommitted at HEAD).
2. Delete the nine program-specific shims (`bpref.py:576-620`). An oracle that knows a program's
   function names by heart is not an oracle. `qdsl.bp` defines them; bpref interprets them.
3. bpref declares its grammar and REFUSES the rest: any form outside it raises `UNSUPPORTED:<form>`
   with a stable exit (not a Python traceback), so a construct using a new feature is counted as
   `unsupported`, never as PASS and never as DIVERGE.
4. A battery lane `a new `bpref_parity.sh` under `bench/vs_rust/` (TO BE CREATED by A23)`: for every construct with a numeric EXPECT, run bpref;
   report `bpref_parity: agree=<a> unsupported=<u> disagree=<d>`, RED on `d > 0`. This is the number that
   would have gone red on `c99cddb` within one battery.
5. Every EXPECT line in `construct_parity.sh` names its derivation: `bpref`, `hand:<closed form>`, or
   `native:<why bpref cannot>` -- an `arch_check` check counts the unlabelled ones, ratchet 0.
6. New language features land with the bpref node OR with an `UNSUPPORTED` entry and a hand-derived
   EXPECT; the row's blueprint says which. The fuzzer emits only what bpref supports (as today).

What this costs: ~2 days, harness + `bpref.py`, no chain. What it saves: the four-day class, and it
converts "bpref supports X" from a belief into a per-construct count. What it does NOT do: it does not
make the semantics single. The honest sentence for the roadmap is: **the semantics is defined once, by
`bebop.bp`; every other implementation is a checker over a declared subset, and the subset is a number.**
Lean (F4) joins under the same rule when it runs.

**Where a real second executor would come from, if wanted:** D5b's witness (`selfhost/attic/
expr_compile.bp`, 46/73 non-vacuous agreements) is a second COMPILER in Bebop -- Thompson-diverse,
in-tree, no CPython. Bringing it to 73/73 on today's grammar is the D5b row (~60-80 lines for `use`,
then return/break, then builtins), and it would be the executor an agent should trust over bpref. Not
this row's cost; named because it is the only path to "one language, two independent executors" that
does not add a Python interpreter to the trust root.

---

## 5. Interactions, in one table

| pair | relation |
|---|---|
| structs (A22) vs tuples (A21) | independent mechanisms; neither subsumes the other (§7 Q1) |
| tuples (A21) -> HOF (A16) | `call_fn` returns through the same x0/x1 convention; land A21 first or A16's results are single |
| structs step 2 (A22) = A8 step 1a | one mechanism (per-symbol type/struct tag in stab); write it once under A8 |
| F3 lengths, A22 tags | same side-channel shape (`fntab[5410+lk]`); F3 arm (ii) declared lengths ride the same cell |
| diagnostics (A17) -> every row | every new code goes through the table; A16/A19 add runtime trap texts = stub change = 104 budget lines (A17 step 3) |
| early return (A18) -> B7 | B7's open defect closes only after A18 (`c70_qdsl_neg` -> 89 is B7's number) |
| loud clone (A19) -> B6 | B6's b6core is red at ok=0; A19 step 1 turns a silent lost child into a named refusal |
| fast lane (A20) -> all | reduces the per-edit cost of every other row; no dependency |
| one-semantics (A23) -> all | each row declares bpref support or UNSUPPORTED; A23 first so the counter exists |
| generics step 4 <- A8 step 2 | monomorphisation is needed only once `[u32]` makes codegen depend on T |
| A16 step 1 <- A17 step 3 | the `call_fn` range trap is a stub text |
| diag table vs frozen stub words | compile-time texts: 0 program words; runtime texts: every binary (A17 step 3 is a separate commit) |

Bootstrap order (§7 Q3): A17 (single-stage), A20/A23 (no compiler change), A19 step 1 (single-stage:
adds a check, not a form), then the two-stage forms A18, A21, A22, A16, A19 step 2.

---

## 6. Design for an agent, not a human

Operator framing, 2026-09-12 (binding): Bebop is a full general-purpose language whose PRIMARY audience
is agents; where the two audiences pull apart, optimise for the agent, but never license a language only
a machine can write. Each of the coordinator's starting points is checked against the tree, then each
item is re-judged: RAISES / LOWERS / LEAVES its order index.

**Checked starting points.**

- *An agent reads compiler output more than tutorials; `exit 89` costs a tool round.* CONFIRMED and
  worse: `exit 89` has THREE meanings in one row (`TRAPS.md` row 89: register collision, `S+tsp > 64`,
  table full) and `docs/TRAPS.md` still says the fn cap is 512 while the compiler says 768
  (`bebop.bp:102` text "cap 768"). An agent that greps the source for a message finds `[101, 120, 112,
  ...]`. A17 is the cheapest row in the plan and the highest-leverage one for this audience. Also
  confirmed: `rc=86` means "store version mismatch" in a compiled program (TRAPS.md row 86) and is in the
  compiler's declared FREE range (`bebop.bp:3623`) -- two processes, one number; the format must carry
  which process spoke.
- *Silent failure costs an agent more (it reports GREEN).* CONFIRMED by the tree's own record: journal
  1789227104 mined 938 entries -- silent-failure 24, no-op-that-reports-success 11, hang 10 -- and the
  2026-09-12 law "FAILURES ARE LOUD" (AGENTS.md:246-289) was written from agent sessions. The two
  measured instances are 6a (kept-symbol loss) and journal 1789233153 (EXIT vs exit_group). The claimed
  "shared frame stomp" is not one of them (§1 6b).
- *Ambiguity an agent resolves by guessing.* CONFIRMED with a count: 70 no-op `if X == 0 then 0 else
  0` lines in 4 files, all agent-written (B7 step 1, `c99cddb`). And the parser laxness that lets
  `fn f(x: i64) requires true ensures ... {` compile (§1 8-10b) is the same class: a form the agent
  believed was checked was silently discarded. Every "accept and ignore" path is an agent trap.
- *Verbosity is cheaper for an agent than implicitness.* CONFIRMED by usage: 22 explicit `env: [i64]`
  sites, `smw.bp` deliberately packing everything into one env (journal 1789220830), B6 §4 mandating it.
  No capture syntax exists and nobody has asked for one in the journal; what they asked for was a
  function VALUE (T50, OPEN since 2026-09-04).
- *Feedback per edit.* CONFIRMED; the number is 104-139 s per chain, not 516 s, but it is one slot on a
  phone and a worker's context window is the scarce resource (WORKER-CARD "Token economy").
- *General-purpose use must hold.* Where this plan trades human legibility away it is named below.

**Per item.**

| item | row | agent-first judgement | order index | disagreement named |
|---|---|---|---|---|
| 5 diagnostics | A17 | RAISES to **1**. Stable code token + file:line:col + text + the fix, in the gcc line shape every agent tool parses; texts as string literals so `grep` finds them in source AND `strings` in the binary. JSON rejected on cost (§3.5). | 1 | none: the gcc shape reads fine for humans |
| 7 fast loop | A20 | RAISES to **2** (parallel-safe with A17; harness only). A 30 s fast lane per edit is a context window saved. | 2 | none |
| 6 loud failures | A19 | RAISES to **3** for step 1 (a day; B6 is waiting). Step 2 (make it work) stays later. | 3 | none |
| 4 one semantics | A23 | RAISES to **4**. An agent reads `bpref.py` as the spec (LANGUAGE.md:1 says so); a rotten spec misleads an agent more than a human, who runs it. Demotion + measured subset. | 4 | humans like a readable Python reference; it stays, as a checker with a number |
| 3 early return | A18 | RAISES to **5**: an explicit `return` in an arm removes the guess; the no-op lint refuses the guess. | 5 | the lint refuses a placeholder idiom humans sometimes write on purpose (`if c then 0 else 0` as a stub). Trade taken: a stub costs a comment |
| 11 self-describing constructs | A24 | NEW, order **6** (§6.1) | 6 | none |
| 1 tuples | A21 | LEAVES (7). Explicit `let (a, b) = f()` is as good for both audiences; the pack idiom is a range bug waiting (32-bit halves). | 7 | none |
| 2 structs | A22 | LEAVES for step 1 (8), LOWERS step 2 behind A8. `T.f`/`T.size` constants are verbose but unambiguous -- an agent writes `a[i*T.size + T.f]` without a type system; a human wants `p.f`. Step 2 restores `p.f` for humans on typed symbols. | 8 (step 1), A8 (step 2) | **yes**: step 1 is a human-legibility trade (offset arithmetic stays visible). It is temporary and it is the store's own idiom today |
| 10 HOF | A16 s1 | RAISES relative to closures: fn values + `call_fn` first (T50). | 9 | none |
| 9 closures | A16 s2 | LOWERS: explicit env only; capture inference costed and not scheduled. | 10 | **yes**: humans want `\|x\| x + a`; agents write `[a]` and pass it. Trade taken and stated |
| 8 generics | A16 s3-4 | LOWERS: erasure is free; monomorphisation waits for A8's `[u32]`. Every value is an i64 or a cell index, so generics buy the CHECKER, not the program. | 11 (erasure), after A8 (mono) | humans expect generics to mean something at runtime; here they cannot until cell widths differ |

### 6.1 The eleventh item: self-describing constructs (row A24, `docs/blueprints/A24-test-blocks.md`)

Tested against the tree, not assumed. Three candidates:

- **A machine-readable diagnostic protocol.** Load-bearing, but it is ONE mechanism with item 5 -- folded
  into A17 (the format is the protocol). Not a separate row.
- **A way for a program to state its own contract so a gate can be generated.** Load-bearing and
  measured: journal 1789244500 found THREE constructs with **no EXPECT row at all** (`c70_qdsl`,
  `c70_qdsl_neg`, `neg/read_before_assign`), red for days while everything upstream reported green,
  because the expectation lives in a 260-line `case` table in another file (`construct_parity.sh:49-238`)
  that an agent forgets to edit. T81 (`test name { ... }` blocks with erasure, TASKS.md OPEN) is exactly
  this. Design: a top-level `test NAME { <expr> }` form, compiled only under `bebop.bin test <file>` (a
  4th CLI verb beside `compile`/`check`/`cas`), which runs each block and prints `NAME <value>`; erased
  under `compile` (`collect_fns` must SKIP it explicitly -- the f8_dt lesson says "ignored" is not
  "erased"). The construct's EXPECT then sits in the construct file as `test c124 { early(5) * 1000 +
  nested(20) }` plus a `// EXPECT 15041 hand: ...` header, and `construct_parity.sh` reads the header
  instead of the table. It does not stop a tautological test (nothing does mechanically), but it puts
  the expectation next to the code an agent edits and makes a missing one a parse error instead of a
  missing `case` arm. Cost: ~80-120 lines in `bebop.bp` (+2 fns), ~30 lines of harness; **3-4 days**;
  two-stage bootstrap is irrelevant (the compiler has no tests of its own; `self_check()` at `:6368` is
  the dead precedent). Order 6, after A18 (tests want `return`) -- or before, it does not depend on it.
- **Structured output from every tool in `tools/`.** Checked: `arch_check.py` prints `FAIL <check>:
  <msg>` / `note: ...` lines (`:678-683`), `std_golden.sh` prints `PASS name (golden)` / `FAIL ...`
  (`gate():81-`), `battery.sh` reduces everything to one regex per lane and `battery: GREEN|RED`. Already
  line-parseable; a `--json` summary is 40 lines and folded into A20 as an optional step. Not load-bearing.

So the eleventh row is **A24 self-describing constructs (T81)**. Verdict row added.

**What is traded away from human legibility, in total:** struct field access by offset constant until
A8 (temporary); closures without capture syntax (permanent, until someone costs the lifting pass);
a lint that refuses a constant-arm placeholder `if` (permanent, by design). Nothing else in this plan is
harder for a human than for an agent.

---

## 7. The three questions

### Q1. Do named-field structs make tuples unnecessary?

**No. They are different mechanisms in this object model, and the cheap one is the tuple.**

A struct VALUE is a cell index into an arena block: `emit_struct_lit` bumps x27 by `8*fcnt`
(`bebop.bp:475-476`), stores the fields, returns the index (`:489-490`). Returning a struct therefore
costs an allocation per call, released only at the ALLOCATING fn's `ret` (A6 LIFO) -- and the value is
dead the moment that fn returns, so a "return a struct" is a use-after-release unless it is `zeros`'d,
which is permanent arena. Inside a loop it is `zeros` in a loop (L8, trap 80 far from the cause). That is
exactly what `qdsl_alloc` does today and why the node tree lives in a caller-owned `a: [i64]`.

A multi-value return is a REGISTER convention: x0 and x1 are both caller-saved, x1 is free after every
`bl` (`emit_bl_call:855-857`), the callee already places k operands into x0..x(k-1) for every builtin
(`vs_deliver:935`), and nothing between the callee's tail and the caller's bind touches x1
(`emit_epilogue_sized`, `emit_bl`). Cost: 0 words. The 14-parameter cap and the x0..x7 tagging do not
constrain it: returns use the same window the arguments used, and k <= 3 is far under both.

What structs subsume is the OTHER half of the pack idiom: two values STORED in one cell (`sgraph2.bp`,
`store.bp` packing `off << 32 | len`). That is a two-field struct or a two-cell object, and A22 step 1's
`T.f` constants make it nameable without changing the layout. So: A21 for "return position + node",
A22 for "store position + node"; neither replaces the other; both are cheap; A21 is cheaper.

### Q2. Can closures exist without heap allocation here?

**Yes, for the useful 80 %, and the tree already writes them that way.** The thesis' "no runtime
cells for ordinary code" (T55) is about a substrate, not about function values; the cost of a closure in
THIS model is exactly the cost of its environment:

| shape | what it costs | where it lives |
|---|---|---|
| fn value `&f` | 2 words at the use (movz/movk), 0 allocation | a register / any cell |
| `call_fn(v, args)` | 5-6 words per site (`adr` base, `add`, `blr`, x15 save/restore) | code |
| environment as an array literal `[a, b]` | 2 + fcnt words, arena cursor, released at `ret`, reset per iteration in loops when `loop_alloc_safe` holds | frame-lifetime arena |
| environment that ESCAPES (stored, returned, sent to a thread) | `zeros(k)`: permanent arena; in a loop it is trap-census row 5 | permanent arena |
| packed `(fn << 40) \| env` in one i64 | +2 unpack words | any cell (the store included) |

So a closure needs heap only when its environment must outlive the frame that made it, which is the
case in any language. The 80 % -- a comparator for `sort_by`, a per-row predicate for a scan, a reducer
for `fold`, a worker body for B6 -- has a frame-lifetime environment and costs no `zeros`. The
defunctionalisation route (an `apply_T` `if`-chain per arrow type, RESEARCH-PROOF-DESIGN §3.2) is NOT
proposed: it needs a whole-program pass to enumerate the lambdas, and a direct `blr` costs the same 3-5
words without it; the census gains a `blr` column instead.

What is traded away: capture inference (`|x| x + a` finding `a` and building the env), which is the
part an agent gets wrong and a human likes. Costed at ~2 weeks (a lambda-lifting text pass in the
`use_expand` shape plus escape classification over `loop_alloc_safe`'s scan) and not scheduled. Of the
ten items, this trades away nothing else: HOF (10) is delivered whole; generics (8) never needed
closures; tuples/structs are unaffected.

One thing it does NOT deliver without a whole-program pass: a `call_fn` site whose callee is COMPUTED
cannot be checked for arity at compile time. The A17 format plus a runtime range trap on the fn index
make a wrong index loud; a wrong arity is a wrong VALUE, caught by the construct, and the honest
statement is "arity of an indirect call is checked by the type census (A8), not by the emitter".

### Q3. What is the honest bootstrap order?

The rule from `tools/chain.sh`: gen2 is built by the OLD promoted `bebop.bin` from the NEW source. So at
the commit that introduces a grammar form, `bebop.bp` must not contain that form; from the commit after
promotion it may. That is a two-stage bootstrap for every syntactic feature and it is already the
project's practice (A1b, A5 step 1b). Per item:

| item | row | stage | can the compiler use it, and should it |
|---|---|---|---|
| 5 diagnostics as strings | A17 | **single** (uses `str` args, legal since T43) | yes, in the same commit -- it IS the compiler's own diag path; `bin_words` shrinks |
| 7 fast lane | A20 | none (harness) | -- |
| 4 one semantics | A23 | none (harness + bpref) | -- |
| 6 loud clone step 1 | A19 | **single** (a check, not a form) | the compiler never clones; no |
| 11 test blocks | A24 | two-stage in principle; the compiler will not use it | no (`self_check` is dead) |
| 3 early return in arms | A18 | **two-stage** | yes, generation N+1: 316 `let _ = if ... else 0;` conditional-effect lines and 33 `diag_exit` guards would read as `if c then return ...`; measurable as a `bin_words` delta, worth doing only if it shrinks |
| 1 multi-return | A21 | **two-stage** | yes, N+1: `emit_call_resolve` packs `pb*2+okres` (`:5722`), `emit_body` returns `n[0]*2 + lastf` (`:5612`), `compile_fn_at_facts` packs five facts into one word (`fw_*:5955-5959`) -- exactly the hand-packing the row removes |
| 2 struct layout | A22 | **two-stage** | N+1 for `fntab` zones as named layouts (43 bases, 315 sites); a large cleanup, own row, not required |
| 6 loud clone step 2 (frame copy) | A19 | **two-stage** (changes clone words) | no |
| 10 fn values | A16 s1 | **two-stage** | N+1: the 38-arm builtin ladder as a hash table -> `call_fn`; K5 measurable |
| 9 closures (explicit env) | A16 s2 | **two-stage** | no need |
| 8 generics (erasure) | A16 s3 | **two-stage** | no need |
| 8 monomorphisation | A16 s4 | after A8 | no |

The single-stage rows are exactly the ones an agent gains from first, which is the ordering in
`ROADMAP-PATCH.md`. Every two-stage row's landing commit must have WORD_DELTA 0 on all 104 constructs
(no construct uses the new form) and a fixpoint at gen2 == gen3 == gen4 when the emitter change adds no
words to old programs -- the A1b precedent -- or gen3 == gen4 with a stated `bebop` word_budget line
when the compiler's own words change (the new parser code).

---

## 8. Cost summary

| row | item(s) | lines | new fns | `bin_words` (compiler) | program words | constructs | chains | days | stage |
|---|---|---|---|---|---|---|---|---|---|
| A17 | 5 (+11 protocol) | ~80 | 0 | -500..-800 | 0 (step 3: stub, every binary) | re-freeze 25 diag/neg | 1 (+1 for step 3) | 1-2 (+1) | single |
| A20 | 7 | ~80 bash | 0 | 0 | 0 | 0 | 0 | 0.5-1 | -- |
| A19 s1 | 6 | ~40 | 0 | +~40, +3 bcond | 0 | 2 | 1 | 1-2 | single |
| A23 | 4 | ~60 py + 40 bash | 0 | 0 | 0 | 0 | 0 | 2 | -- |
| A18 | 3 | ~120 | 1-2 | +~120 | 0-2 per use | 2 | 1 | 3-5 | two |
| A24 | 11 | ~120 | 2 | +~150 | 0 | 1 | 1 | 3-4 | two |
| A21 | 1 | ~150 | 3-4 | +~200 | 0 | 2 | 1 | 5-7 | two |
| A22 s1 | 2 | ~100 | 2 | +~120 | 0 | 1 | 1 | 5 | two |
| A22 s2 (= A8 s1a) | 2 | ~180 | 3 | +~250 | 0 | 2 | 1 | 10-15 | two |
| A16 s1 | 10 | ~150 | 3 | +~200, `blr` census | 5-9 per site | 3 | 1 | 7-10 | two |
| A16 s2 | 9 | ~40 | 0 | +~40 | 2 per unpack | 2 | 1 | 3 | two |
| A16 s3 | 8 | ~30 (+typecheck) | 0 | +~30 | 0 | 2 | 1 | 3 | two |
| A16 s4 | 8 | 8-12 fns | 8-12 | ? | per instantiation | 2 | 1+ | ~10, after A8 | two |
| A19 s2 | 6 | ~60 | 0 | +~60 | +8-12 per clone | 1 | 1 | 5 | two |

Fn budget: 295 + (0+0+0+0+2+2+4+2+3+3+0+0+12+0) = **323** against 768. `bin_words` net: roughly
43,491 - 650 + 1,200 = ~44,050 before A16 s4, within +1.3 %.

What is NOT in this plan, and why (L23 form): capture-inference closures -- costed ~2 weeks, not
scheduled; `if` statements without `else` -- costed ~200 lines, not scheduled; a table-driven front end
that would make bpref generable -- costed ~2,000 lines, not scheduled; JSON diagnostics -- costed, a
worse fit than the gcc line shape for a language without string operations; struct values in registers
(T49) -- would change the store's layout, not scheduled.
