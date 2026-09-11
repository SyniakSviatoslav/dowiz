# F2 Implementation Plan: Zero-Word Trap Implementations

**Status**: NOT STARTED — next F-phase work after F1
**Branch**: bebop.bp lane
**Estimated**: ~300 lines, ~3 weeks
**Gate**: `trap_unrep >= 12/20`; construct parity `86+12/0`; fixpoint `gen3 == gen4`; `bin_words delta <= 0` (one `brk #87` word removed) with a word_budget line; `gen_avoid` shrinks by 12; battery GREEN

---

## 1. Exit-Code Space Partitioning (Precondition — 0 words, lands first)

**Problem** (from TRAPS.md §F1): `vs_mask_take/free` and `vs_cs_take/free` emit `sys_exit(99+site)` through `sys_exit(120+site)` for `site in 1..20`, spanning **99..140**. This aliases every documented compile-time diagnostic (99, 100, 101, 102, 104). Codes 102 and 104 each mean two things today. Codes 103/105/106/107 are emitted and documented NOWHERE.

**Fix**: Partition the space before writing any F2 diagnostic code.
- User-facing diagnostics: **65–93** (19 free codes per census, F2 needs 12)
- Compiler self-checks: **201** (single code, site printed on stderr as `compiler self-check <kind>.<site>`)
- `selfcheck_exit` already implemented at bebop.bp:2196, exits 201

**Files**: `bebop.bp` (vs_mask_take, vs_mask_free, vs_cs_take, vs_cs_free, selfcheck_exit), `docs/TRAPS.md`
**Lines**: ~30
**Neg constructs**: none (infrastructure)

---

## 2. Definite Assignment — Exit 105 (was 65)

**Source**: `docs/WORKER-CARD.md:20` "Reading a symbol that no `let` has executed yet ... is UNDEFINED"; `docs/LANGUAGE.md:53` UNDEFINED; trap_census row `read-before-assign`

**Mechanism**: One `i64` bitmask over the 64-symbol cap (`fntab[5387]` max 512 fns, but per-fn symbols ≤ 64). Track assigned symbols in a 64-bit mask; on every symbol read, check the bit. Reject at parse/planning time with `diag_exit(s, pos, 105)` — **0 machine words**.

**Emitter changes** (`bebop.bp`):
- `emit_ident` / `emit_var_or_ctor`: add assigned-mask check before returning register
- `emit_let_stmt` / `emit_let_chain`: set bit on bind
- New per-fn fact: `assigned_mask` in fntab (one cell per fn, or packed in existing fact zone)

**Neg construct**: `bench/parity_constructs/neg/read_before_assign.bp` — `fn main()->i64{let x=i64; x}` (or similar minimal case) with `EXPECT=COMPILEFAIL:105`

**Lines**: ~40

---

## 3. Loop-Literal Escape — Exit 106 (was 66)

**Source**: `docs/WORKER-CARD.md:20` "no `zeros` inside a while body (L8)"; trap_census row `zeros-in-while`; `loop_alloc_safe` already computes escape (bebop.bp:4617)

**Mechanism**: At `while` planning (in `compile_fn` / `emit_while_stmt`), call `loop_alloc_safe` on the loop body text. If it returns 0 (escape detected), reject with `diag_exit(s, loop_pos, 106)`. **Verify A6's current per-iteration status first** — A6 moved aggregates to arena cursor x27; the escape semantics changed (leak → arena growth → exit 80). F2 must reject the *source pattern* regardless of where it leaks.

**Emitter changes**:
- `emit_while_stmt` (or `compile_fn` planning pass): invoke `loop_alloc_safe(s, body_start, body_end, stab, cnt0)`; on 0 → `diag_exit(s, pos, 106)`
- Reuse existing `loop_alloc_safe` (text scan, conservative: `let OUTER = bare` or store to outer symbol)

**Neg construct**: `bench/parity_constructs/neg/loop_literal_escape.bp` — `fn main()->i64{let a=zeros(8); while 1<2 { let _ = a[0]=1; }; 0}` with `EXPECT=COMPILEFAIL:106`

**Lines**: ~25

---

## 4. `zeros` in `while` Body — Exit 107 (was 67)

**Source**: `docs/WORKER-CARD.md:20` (L8); trap_census row `zeros-in-while`; distinct from escape — this is the *allocation itself* inside a loop, even if it doesn't escape.

**Mechanism**: In `emit_while_stmt` (or planning scan), detect `zeros(N)` call in loop body (text scan or AST walk). Reject with `diag_exit(s, pos, 107)`. Zero words — pure compile-time rejection.

**Emitter changes**: Add scan in `emit_while_stmt` before emitting loop header.

**Neg construct**: `bench/parity_constructs/neg/zeros_in_while.bp` — `fn main()->i64{while 1<2 { let _=zeros(8); }; 0}` with `EXPECT=COMPILEFAIL:107`

**Lines**: ~15

---

## 5. > 8 Live Symbols Across `sys_clone` — Exit 108 (was 68)

**Source**: `docs/WORKER-CARD.md:20` "exit 89 register pressure or > 8 live symbols across a clone (let-bind, split fns)"; trap_census row `live-symbols-across-clone`

**Mechanism**: At `sys_clone` call site (in `emit_call_or_ctor` / `emit_sys_clone`), count live symbols in window mask (`fntab[5388]`) + cs mask (`fntab[5389]`). If `popcount(~fntab[5388] & 0xFF) + popcount(fntab[5389] & 0xFF) > 8`, reject with `diag_exit(s, pos, 108)`. Zero words.

**Emitter changes**: Add check in `emit_sys_clone` (or `emit_call_or_ctor` dispatch for `sys_clone`).

**Neg construct**: `bench/parity_constructs/neg/live_symbols_clone.bp` — 9+ live let-bindings across a `sys_clone` call with `EXPECT=COMPILEFAIL:108`

**Lines**: ~20

---

## 6. True Message for Nesting Cap at Exit 95 Sites

**Source**: trap_census row `nesting-cap` — "diag_exit(...,95) but the message is 'expected )' -- WRONG CAUSE for the nesting case"; TRAPS.md shows exit 95 used for multiple parse errors (missing `)`, missing `in`, parameter list issues)

**Mechanism**: Distinguish the nesting-cap case (window depth > 512 at `fntab[2800..4335]`) from other parse errors. At each `diag_exit(s, pos, 95)` site, pass a distinct code or augment the message. Since exit codes are partitioned, assign a new code for nesting cap (e.g., **70**) and keep 95 for genuine "expected ) or in".

**Emitter changes**:
- `emit_paren` (bebop.bp:3653): genuine "expected )" → keep 95
- `compile_fn_at` / `vs_push` (window depth check): new code **70** "expression nesting too deep (cap 512)"
- `parse_params` / `emit_let_stmt` "expected in": keep 95 or assign **71** "expected 'in' after let-expression"

**Neg constructs**: 
- `neg/nesting_paren.bp` `EXPECT=COMPILEFAIL:95`
- `neg/nesting_cap.bp` `EXPECT=COMPILEFAIL:70`
- `neg/nesting_in.bp` `EXPECT=COMPILEFAIL:71`

**Lines**: ~25

---

## 7. `&&` / `||` Rejected or Short-Circuited — Exit 112 (was 69)

**Source**: ROADMAP F2 row; trap_census has no row yet. Bebop has no short-circuit; `&&`/`||` are bitwise `&`/`|` in BIN (bpref.py:94). Language spec says they are bitwise (LANGUAGE.md:61-63). Two options:
- **Reject**: `diag_exit(s, pos, 112)` "`&&`/`||` not in surface; use `&`/`|`"
- **Short-circuit**: emit branchless csel sequence (costs words — against F2's zero-word budget)

**Decision**: **Reject** (zero words, matches "rejected or short-circuited" with preference for reject). The fuzzer doesn't generate them; gen.py must stop avoiding the shape.

**Emitter changes**: In `emit_binop` / `cmp` parsing, detect `&&`/`||` tokens → `diag_exit(s, pos, 112)`.

**Neg construct**: `bench/parity_constructs/neg/logical_andor.bp` — `fn main()->i64{1 && 2}` with `EXPECT=COMPILEFAIL:112`

**Lines**: ~15

---

## 8. Unresolved Call at Compile Time via `fntab_lookup` — Exit 115 (was 72)

**Source**: ROADMAP F2 row "unresolved call at compile time via fntab_lookup at end of program (exit 115, deletes the brk #87 word)"; trap_census row `unresolved-call` currently RUNFAIL:87 (brk #87 at runtime)

**Mechanism**: After all functions compiled (end of `compile_program`), scan fntab for unresolved calls (entries with `val == 0` at `fntab[3668]` / call-site table). For each, `diag_exit(s, srcpos, 115)` with the call site position. **Deletes the `brk #87` word** from emit_call (runtime trap) — saves 1 word in bin_words.

**Emitter changes**:
- `compile_program` / `compile_program_offs`: after fn loop, iterate call sites; unresolved → `diag_exit`
- `emit_call_or_ctor`: remove the `brk #87` emission path (the `tgt0 == 0` branch)

**Neg construct**: `bench/parity_constructs/neg/unresolved_call_compile.bp` — `fn main()->i64{missing_fn()}` with `EXPECT=COMPILEFAIL:115`
- Update `c52_undef` to `EXPECT=COMPILEFAIL:115` (was RUNFAIL:87)

**Lines**: ~30 (net -1 word → word_budget line: `c52_undef -1 unresolved call now compile-time`)

---

## 9. Literal `sys_clone(_, 0)` — Exit 116 (was 73)

**Source**: `docs/WORKER-CARD.md:24` "never stack_top 0: the child rebinds x14/x15/x27/x28 to [stack_top, +12 MiB)"; trap_census row `clone-stack-top-0`

**Mechanism**: In `emit_sys_clone`, if second argument is literal 0 → `diag_exit(s, pos, 116)` "`sys_clone` stack_top must not be 0". Zero words.

**Emitter changes**: Add check in `emit_sys_clone`.

**Neg construct**: `bench/parity_constructs/neg/clone_stack_zero.bp` — `fn main()->i64{sys_clone(17, 0)}` with `EXPECT=COMPILEFAIL:116`

**Lines**: ~10

---

## 10. `use` Off Column 0 — Exit 118 (was 74)

**Source**: ROADMAP F2 row; `use` must be line-initial (LANGUAGE.md:17). Currently not checked.

**Mechanism**: In parser (`parse_use` or top-level loop), if `use` not at column 0 → `diag_exit(s, pos, 118)` "`use` must start at column 0". Zero words.

**Emitter changes**: Add column check where `use` is parsed (bebop.bp:5738 area).

**Neg construct**: `bench/parity_constructs/neg/use_not_col0.bp` — `  use "x.bp"` (indented) with `EXPECT=COMPILEFAIL:118`

**Lines**: ~10

---

## 11. Positions on Every Remaining Exit-89 Site

**Source**: ROADMAP F2 row; trap_census row `register-pressure` and `fn-cap-511` — exit 89 messages lack position or have wrong message.

**Mechanism**: Audit all `diag_exit(s, ..., 89)` sites (bebop.bp:4172, 2656, and others). Ensure each passes a valid source position and a specific message ("register pressure: let binding collides with live cs temp", "too many fns (cap 512)", etc.). Use existing `diag_exit` infrastructure.

**Emitter changes**: Update each exit-89 site with proper position and message.

**Neg constructs**: One per distinct exit-89 cause (extend existing `c89_letlive`, add new for fn-cap)

**Lines**: ~30

---

## 12. gen.py Stops Avoiding Each Rejected Shape

**Source**: ROADMAP F2 row "gen.py stops avoiding each rejected shape"

**Mechanism**: For each of the 12 new neg constructs, remove the avoidance logic in `bench/fuzz/gen.py` that currently skips generating those patterns. The fuzzer should now produce them, and they should be caught by the new compile-time rejections (TRAP-OK in fuzz classification).

**Files**: `bench/fuzz/gen.py`
**Changes**: Remove avoidance for: read-before-assign, loop-literal-escape, zeros-in-while, live-symbols-clone, &&/||, unresolved-call, clone-stack-zero, use-off-col0
**Lines**: ~20

---

## 13. One `neg/` Construct Per Code (`EXPECT=COMPILEFAIL:<code>`)

**Summary of 12 new neg constructs needed**:

| Code | Trap | Neg Construct File | Expect |
|------|------|-------------------|--------|
| 105 | definite assignment | `read_before_assign.bp` | COMPILEFAIL:105 |
| 106 | loop-literal escape | `loop_literal_escape.bp` | COMPILEFAIL:106 |
| 107 | zeros in while | `zeros_in_while.bp` | COMPILEFAIL:107 |
| 108 | >8 live across clone | `live_symbols_clone.bp` | COMPILEFAIL:108 |
| 70 | nesting cap (new) | `nesting_cap.bp` | COMPILEFAIL:70 |
| 71 | expected in (new) | `nesting_in.bp` | COMPILEFAIL:71 |
| 112 | &&/|| rejected | `logical_andor.bp` | COMPILEFAIL:112 |
| 115 | unresolved call (compile-time) | `unresolved_call_compile.bp` | COMPILEFAIL:115 |
| 116 | sys_clone(_, 0) | `clone_stack_zero.bp` | COMPILEFAIL:116 |
| 118 | use off col 0 | `use_not_col0.bp` | COMPILEFAIL:118 |
| 95 | genuine paren/in | `nesting_paren.bp` | COMPILEFAIL:95 |
| 89 | register pressure / fn cap | extend existing / new | COMPILEFAIL:89 |

**Total**: 12 new neg constructs (plus updates to existing)

---

## 14. Word Budget & Fixpoint

- **Net `bin_words` delta ≤ 0**: Only change adding words is definite-assignment mask tracking (1 fntab cell per fn = 0 code words). Only change removing words: `brk #87` deletion in `emit_call` (-1 word). **Net: -1 word**.
- **word_budget.txt line**: `c52_undef -1 unresolved call now compile-time`
- **Fixpoint**: `gen3 == gen4` must hold after all changes (run `tools/chain.sh --codegen`)

---

## 15. Timeline (3 Weeks)

| Week | Work | Deliverable |
|------|------|-------------|
| 1 | Exit-code partitioning (precondition); definite assignment (105); loop-literal escape (106); zeros in while (107) | 4 traps implemented + 4 neg constructs; partition committed |
| 2 | Live symbols across clone (108); nesting cap messages (70, 71, 95); &&/|| reject (112); unresolved call compile-time (115) | 4 more traps + 4 neg constructs; `brk #87` removed |
| 3 | sys_clone(_,0) (116); use off col 0 (118); positions on exit-89; gen.py avoidance removal; all 12 neg constructs; fixpoint verification; battery GREEN | All 16 traps done; `trap_unrep >= 12/20`; construct parity 86+12/0; fixpoint; bin_words delta ≤ 0; gen_avoid -12; battery GREEN |

---

## 16. Verification Checklist (Gate Criteria)

- [ ] `trap_unrep >= 12/20` (was 11/30, target 12/20 after F2)
- [ ] Construct parity: `86+12/0` positive + negative (run `construct_parity.sh`)
- [ ] Fixpoint: `gen3 == gen4` (run `tools/chain.sh --codegen`)
- [ ] `bin_words delta <= 0` with word_budget line for -1 word
- [ ] `gen_avoid` shrinks by 12 (verify `bench/fuzz/gen.py` avoidance lines removed)
- [ ] Battery: `SERIAL=1 BEBOP_TMP=$OUT FREEZE=1 SRC=bebop.bp tools/battery.sh ./bebop.bin $OUT/bat` → `battery: GREEN`
- [ ] All 12 new neg constructs in `bench/parity_constructs/neg/` with correct `EXPECT=COMPILEFAIL:<code>`
- [ ] `docs/TRAPS.md` updated with new exit codes and messages
- [ ] `tools/trap_census.py --rows` shows updated counts

---

## 17. File Inventory (Exact Paths)

| File | Changes |
|------|---------|
| `bebop.bp` | All emitter changes (12 traps, partitioning, messages) |
| `docs/TRAPS.md` | Exit code table updated; partition documented |
| `bench/vs_rust/construct_parity.sh` | 12 new EXPECT lines in neg section |
| `bench/parity_constructs/neg/*.bp` | 12 new neg construct files |
| `bench/fuzz/gen.py` | Remove 12 avoidance patterns |
| `bench/parity_constructs/word_budget.txt` | One line: `c52_undef -1 unresolved call now compile-time` |
| `tools/trap_census.py` | No changes needed (derives from sources) |

---

## 18. Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Partitioning breaks existing self-checks | `selfcheck_exit` already uses 201; verify all computed-code sites migrated before F2 diagnostics land |
| Definite assignment mask exceeds 64 symbols | Cap is 64 per fn (window 8 + cs 8 = 16, plus params/locals ≤ 48); 64-bit mask sufficient |
| `loop_alloc_safe` false positives | Conservative by design; false positive = reject valid program (safe); false negative = leak (caught by exit 80) |
| Unresolved call at compile time misses dynamic cases | All calls are resolved at compile time in this compiler; no dynamic loading |
| Fixpoint fails due to word count drift | Only -1 word change; run chain.sh incrementally per trap |

---

## 19. Self-Critique (2-Question Ritual)

**What am I least confident about?**
1. Exact fntab cell for definite-assignment mask — need to verify per-fn fact zone layout in `compile_fn_at` / `compile_program_offs`
2. Whether `loop_alloc_safe` text scan has access to full loop body text at `emit_while_stmt` (it receives `s, b0, b1, stab, cnt0` — confirm `b0/b1` are body bounds)
3. Whether gen.py avoidance removal is purely deleting `if` guards or requires restructuring — need to audit gen.py for each pattern

**What's the biggest thing I'm missing?**
- The interaction between exit-code partitioning and existing `diag_exit` calls that use codes 105/106/107 as self-checks — must ensure those are fully migrated to 201 before assigning 105/106/107 to user diagnostics. The census says 105/106 are emitted at bebop.bp:4937/5025 as cs-mask self-checks; those must move to 201 first.

---

## 20. Execution Order (Single-Variable Commits)

Per AGENTS.md L14 and ROADMAP critical path: each trap lands as its own commit with its own probe. Order:

1. **Partition exit codes** (migrate self-checks to 201) — `selfcheck_exit` infra already there
2. **Definite assignment (105)** — mask + check + neg
3. **Loop-literal escape (106)** — `loop_alloc_safe` integration + neg
4. **Zeros in while (107)** — scan + neg
5. **Live symbols across clone (108)** — count + neg
6. **Nesting cap messages (70, 71, 95)** — message disambiguation + 3 neg
7. **&&/|| reject (112)** — token check + neg
8. **Unresolved call compile-time (115)** — end-of-program scan + remove brk #87 + neg (update c52_undef)
9. **sys_clone(_,0) (116)** — literal check + neg
10. **use off col 0 (118)** — column check + neg
11. **Exit-89 positions** — audit all sites + neg
12. **gen.py avoidance removal** — 12 patterns
13. **Fixpoint + battery** — chain.sh --codegen, battery.sh

Each commit: `tools/chain.sh --codegen` → fixpoint → `construct_parity.sh` → `battery.sh` → promote.