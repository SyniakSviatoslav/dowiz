Status: 2026-09-13, written against `ddb534f` (bebop.bin `072c01d1`) for ROADMAP.md:196 (row F2), which cites this file. Every `bebop.bp:NNN` below was read at that line on this commit; a claim this lane could not measure is marked SPECULATIVE. Nothing here was compiled or run through `bebop.bin` -- the compile token was held by another lane -- so every executed probe went through `tools/bpref.py` and says so.

# F1 Unrepresentable zero-word traps: the planning-scan rejections

## 0. Goal

Row F2 (ROADMAP.md:196) lists the traps that can be closed in the planning scan the compiler already runs -- a `diag_exit(s, pos, code)` with a `<line>:<col>` and **zero emitted words** -- and gates them as `trap_unrep >= 12/20`; construct parity `86+12/0`; fixpoint `gen3 == gen4`; `bin_words` delta `<= 0` (one `brk #87` word removed) with a `word_budget` line; `gen_avoid` shrinks by 12; battery GREEN. This blueprint grounds each of the row's items in the compiler as it stands today, records which parts of the row and of its F1 predecessor have gone stale, assigns codes from the space that is actually free, and sequences the work so that the census instrument is trustworthy before any number is claimed against it.

## 1. Where the inputs stand today

### 1.1 `docs/blueprints/F0-trap-census.md` is deliberately absent

Row F1 (ROADMAP.md:195) says so in its first sentence: "The census is a script that RE-DERIVES, because the document it was meant to transcribe was not in the tree -- `docs/blueprints/F0-trap-census.md` does not exist and is not invented here". The replacement is `tools/trap_census.py` (landed `25c0835`, 2026-09-09: "the trap census refuses to transcribe a document that is not in the tree"), which re-derives every number on every run from `docs/WORKER-CARD.md`, `docs/LANGUAGE.md`, `docs/TRAPS.md`, `bebop.bp` and `bench/parity_constructs/neg/`, and prints the absence itself (`tools/trap_census.py:613`, "NOT PRESENT: docs/RESEARCH-VERIFICATION-2026-09-09.md, docs/blueprints/F0-trap-census.md"). The first half of that line is now stale -- `docs/RESEARCH-VERIFICATION-2026-09-09.md` exists (its §0 item 1 and §3, lines 7 and 49, are the origin of "24 rows / 20 open / 16 zero-word") -- but the second half is the decision, not a gap. Do not write F0; grep for the file should land here.

### 1.2 The F1 row's census numbers, re-derived 2026-09-13

`python3 tools/trap_census.py` on `ddb534f`:

| number | row F1 (2026-09-09) | today | status |
|---|---|---|---|
| `trap_unrep` | 10/29 | **0/20** | MOVED, and the move is an INSTRUMENT ARTEFACT (§1.4), not a regression |
| `trap_zero_word` | 16 of 19 open | 17 of 20 open | moved by one row each way (the 6 derived `code-N` rows vanished with the neg scan) |
| `trap_needs_words` | 3 of 19 | 3 of 20 | holds |
| `undef_census` | 2 | 2 (`tools/undef_census.py`) | holds |
| `shadowable_builtins` | 7 of 36 | **1 of 38** (`sys_mapb` only, `--shadow`) | moved: six of the seven hashes landed; the survivor is the refused builtin (`bebop.bp:7735`, exit 108) |
| `gen_avoid` | 3 PROVISIONAL | 3 PROVISIONAL | holds; `avoid` still occurs 0 times in `bench/fuzz/gen.py` and no `# LANG-AVOID:` marker exists, so the F2 gate "`gen_avoid` shrinks by 12" cannot move as worded |
| `diag_texts` (`--texts`) | -- | **22 codes, 22 in source, 22 in binary, 22 documented** | row F8 (ROADMAP.md:202) still says 19; the row lags by three codes (111, 112, 113 landed after it was written) |

### 1.3 The exit-code partition has LANDED; the F1 row's blocker is closed

Row F1's "FINDING THAT BLOCKS F2" cites `vs_mask_take`/`free` and `vs_cs_take`/`free` at `bebop.bp:2271-2295` exiting `sys_exit(99/100/119/120 + site)`. On `ddb534f`:

- `bebop.bp:2271-2295` is the tail of `cli_exit` and `fn em` -- the range no longer names those functions.
- `fn vs_mask_take` is at `bebop.bp:2438`, `vs_mask_free` 2444, `vs_cs_take` 2454, `vs_cs_free` 2460, and every failure arm calls `selfcheck_exit(kind, site)` (`bebop.bp:2439-2463`), which prints `0:0: error[E201]: compiler self-check <kind>.<site>` and exits **201** (`fn selfcheck_exit`, `bebop.bp:2240-2249`).
- Landed in `947246e` (2026-09-09, "one self-check code instead of a span of forty"); documented as the 201 row at `docs/TRAPS.md:56`.
- `trap_census.py --codes` today: "exit codes the compiler can produce but docs/TRAPS.md does not carry: (none)" and no computed range.

Consequences for this row: **105, 106 and 107 are free again** (they were self-check aliases; the self-checks moved to 201), so the F2 row's original assignment stands except for 112, which WAVE 0 took for module contents (`bebop.bp:6107`, `docs/TRAPS.md:36`). The free list, derived on `ddb534f` as the codes neither in `docs/TRAPS.md`'s tables nor emitted by `diag_exit`/`sys_exit`/`cli_exit` in `bebop.bp`: **66-79, 84, 93** below 99 (sixteen, not the row's nineteen: 65 went to F3's static bounds check at `bebop.bp:4296`, and 85/86 are `store.bp`'s at `docs/TRAPS.md:46-47`), and **105-107, 114-127** above. The "user diagnostics below 99" half of the partition proposal in `docs/TRAPS.md:108-111` was not adopted -- 108-113 all landed above 99 after `947246e` -- so this row keeps the F2 numbering and takes **114** for the `&&` family in place of 112. `84` is reserved by `docs/blueprints/F2-bounds-by-type.md:85`; `93` by the F2 row for arity.

Three places still carry the pre-`947246e` text and should be corrected when this row lands: `docs/TRAPS.md:65-111` (the whole "NOT partitioned" section, including the `2271-2295` anchor at :79 and the free list at :106), `docs/TRAPS.md:119` and :129 (`trap_unrep: 10/29`, `7 of 36`, `bebop.bp:5753-5756` -- the T122 table is at `bebop.bp:6234`), and ROADMAP.md:195's own text.

### 1.4 The census instrument has two stale parts, and step 0 repairs them

**(a) The neg scan reads a format that no longer exists.** `scan_neg_expects` (`tools/trap_census.py:106-118`) matches lines of the form `name) EXPECT=COMPILEFAIL:<code>;;` in `bench/vs_rust/construct_parity.sh`. Since `d9890da` (2026-09-13, "every construct now states its own expected value") the expectations live in each construct's header -- `construct_parity.sh:5` and `extract_expect` at :22-26 read `// EXPECT <value> from:` -- so the scan finds nothing: "bench/.../neg: 0 codes have a neg construct". Twenty `neg/*.bp` files carry such headers today (e.g. `neg/c141_clone9.bp:8` `COMPILEFAIL:109`, `neg/c52_undef.bp:3` `RUNFAIL:87`). Every `closed` verdict requires `has_neg` (`trap_census.py:568`), so `trap_unrep` fell from 10/29 to 0/20 the day the header format moved, and no gate noticed because the census is not wired into the battery (row F1's own decision). The F2 gate `trap_unrep >= 12/20` is unmeasurable until the scanner reads the headers.

**(b) `ROWS` is a transcribed table inside a script that calls itself derived.** `tools/trap_census.py:191-253` hard-codes 23 rows with their mechanism text. Five are contradicted by `ddb534f`: `live-symbols-across-clone` (:195, "none: not detected" -- landed as exit 109 at `bebop.bp:1312`, neg `c141_clone9`); `exit-code-space-unpartitioned` (:236), `exit-102-overloaded` (:240), `exit-105-106-undocumented` (:243) and `fn-cap-code-conflict` (:247) -- all four describe the pre-`947246e` code space, and their anchors (`2271-2295`, `310 vs 4912`, `4937,5025`, `2656,4172`) no longer point at what they name (102 is emitted at `bebop.bp:316` and `:5726` today; 104 at two `diag_exit` sites; there is no `sys_exit(105)`). Their `has_mech` verdict is computed from the mechanism string's first word (`trap_census.py:567`), so the table's prose decides the count. This is the same defect the script was written to refuse.

Step 0 of this row therefore lands before any trap: the scanner reads construct headers; the five stale rows are re-derived (four become closed-by-201 or n/a, one becomes closed with `c141_clone9`); the re-derived baseline is printed and recorded here, not assumed. Only then does `>= 12/20` mean anything.

## 2. The family: what "unrepresentable at zero words" means

A row belongs here when (i) the program is accepted today and given a meaning no author intended, (ii) the decision can be made from the text and the symbol table during the planning pass, and (iii) the rejection is a `diag_exit` with a position -- the form every code from 95 to 113 already takes (`fn diag_exit`, `bebop.bp:72-112`, text ladder at :92-107). The cost is compiler lines and compile time; the emitted program is unchanged except where a runtime trap word is deleted (§3.7). The oracle matters for each row in one of two ways: where `tools/bpref.py` already refuses the shape (definite assignment, §3.1) the compiler is the lagging twin; where bpref accepts it identically (`&&`, arity: §3.6, §3.10) the parity gate can never see the trap and the `neg/` construct is the only instrument.

## 3. The rows, grounded

Each item of ROADMAP.md:196, with what the compiler does today at a named function, the check, and what was measured.

### 3.1 Definite assignment (105)

**Today.** `docs/LANGUAGE.md:53-54`: reading a symbol no `let` has executed yet "is UNDEFINED (whatever the register holds)". Symbols are function-scoped triples in `stab` (`fn sym_bind`, `bebop.bp:223`); the cap is **128**, not 64 -- `is_full = if cnt >= 128` at `bebop.bp:244`, and `stab = zeros(385) // 1 + 3*128` at `bebop.bp:4455`, `:5941`, `:6216`. The row's "one i64 bitmask over the 64-symbol cap" is therefore two words, or one byte per symbol; the mechanism is unchanged. **Measured (bpref):** `fn main() -> i64 { let i = 0; while i < 0 { let x = 5; let i = i + 1; 0 }; x }` -> `bpref error: KeyError: 'x'`, rc 2. The oracle refuses; the compiler, per LANGUAGE.md, does not. The census row (`trap_census.py:222`) records that the fuzzer avoids the shape for exactly this reason.

**Existing neg is misfiled.** `neg/read_before_assign.bp:1` says "F2 (exit 105): read before assignment" but its body is an identifier-hash-collision probe and its header (`:4`) expects `COMPILEFAIL:101` -- unbound symbol, which is what it gets today. It does not exercise definite assignment and must not be credited to this row; the real construct is the loop-scoped shape above.

**Check.** A per-symbol "assigned on every path" bit, set by `emit_let_stmt`/`emit_let_chain` at binding, cleared at loop entry for symbols first bound inside the body and at the `else`-less arms of `if`, read by `emit_ident` (`bebop.bp:307`). Precision is the open question: Bebop `let` REBINDS (`LANGUAGE.md:41-43`), so the bit tracks "some `let` for this name dominates this read", which is a dataflow fact the one-pass emitter approximates by scope nesting. SPECULATIVE: the approximation's false-rejection rate on the 544-file control arm is unmeasured; the F8 step 0 precedent (one file changed) is the shape of the measurement.

### 3.2 Loop-literal escape (106) and 3.3 `zeros` in a `while` body (107)

**Today, and this is the fact the row asks to "verify first".** `emit_while_stmt` (`bebop.bp:5292`) already scans the body: `allocs = count_word(...)` counts allocation words, `safe = loop_alloc_safe(...)` (`bebop.bp:5410`, the function at `:4785`) returns 0 when a `let` in the body binds a bare literal to an OUTER symbol or stores it into an array (`bad` accumulates at the `outer + store` line), and `do_reset = slot_ok * allocs * safe` (`:5411`). An UNSAFE verdict therefore disables the per-iteration arena reset **silently**: the escaping aggregate stays valid and the loop leaks arena until exit 80 (`emit_zeros` trap words, `bebop.bp:6733`). Under A6 this is a leak, not a corruption, which is why no construct has caught it.

**Check.** Both codes are one line at the same site: `if allocs > 0 and safe == 0 then diag_exit(s, <the escaping let>, 106)`; and for 107, `emit_zeros` reads the live loop depth `fntab[5246]` (written at `bebop.bp:5298-5299`, restored at `:5440`, zeroed per fn at `:6203`; no reader exists in `emit_zeros` today) and refuses when it is non-zero. `loop_alloc_safe` must return the offending position instead of a 0/1 for 106 to carry a `<line>:<col>` (a two-line change to its `bad` accumulation).

**Cost that must be measured before landing:** 107 as a flat rule enforces law L8 (`docs/WORKER-CARD.md:20`, "no `zeros` inside a while body") on the compiler itself. A cheap proxy on `ddb534f` -- `zeros(` at indentation >= 4 in `bebop.bp` -- gives 1 site against 93 at fn top level; the real count needs the control arm. SPECULATIVE until compiled.

### 3.4 More than 8 kept symbols across `sys_clone` -- LANDED, the row's code is stale

The row assigns 108. It landed as **109** (A19, 2026-09-13): `if stab[0] > 8 then diag_exit(s, srcpos, 109)` at `bebop.bp:1312` in `emit_sys_clone` (`:1298`), text at `:104`, row at `docs/TRAPS.md:33`, pinned both sides by `c142_clone8.bp:10` (positive, 201) and `neg/c141_clone9.bp:8` (`COMPILEFAIL:109`). 108 is `sys_mapb` refused (`bebop.bp:7735`). Nothing to build; the census row at `trap_census.py:195` must be re-derived to closed (§1.4b), and the row's own two over-approximations (constants counted; the pending binder counted) are A19 step 1b, not this row.

### 3.5 A true message at the nesting cap

**Today.** All seven `diag_exit(..., 95)` sites (`bebop.bp:851, 3726, 4005, 4028, 5532, 5906, 5922`) print "expected `)` or `in`" (`:92`); none is the nesting cap. The window cap itself is `if fntab[5387] >= 512 then sys_exit(89)` at `bebop.bp:2824` inside the register model, with **no position and the wrong text** (89's text at `:97` is about a live call temp). The other positionless 89 is `bebop.bp:918` (`vs_find_unplaced`). Row F1's `nesting-cap` census row (`trap_census.py:197`) already names the wrong-cause message.

**Check.** A dedicated text for the cap and a position. `vs_push` has no `s`/`pos`; the cheapest route is the statement position the emitter already keeps for diagnostics, stashed in a `fntab` cell at each statement boundary and read at :2824 and :918. SPECULATIVE: which cell is free is a register-model map question (`bebop.bp:2283-2296` lists the taken ones).

### 3.6 `&&` and `||`: the precedence trap (MEASURED 2026-09-13)

**The grammar.** `emit_cmp` (`bebop.bp:3597`) parses each comparison operand with `emit_bor` (`:3598`, and again for the right operand in `emit_apply_cmp` at `:3592`); the tier comment at `:3513` is "cmp > `|` > `^` > `&` > `<<` `>>` > + - > * / %"; `emit_band` (`:3547`) consumes a single `&` (`ch == 38` at `:3554`, `emit_apply_bits(..., adv=1, lvl=1)` at `:3501`). So `&` binds TIGHTER than `<`, and `&&` is not a token: `c >= 65 && c <= 90` parses as `c >= (65 & <second &> c) <= 90`. `docs/LANGUAGE.md:57-70` lists the precedence tiers and mentions neither `&&` nor `||` (0 occurrences in the file). `bench/parity_constructs/c46_andor.bp:1-2` records the `&`-tier design on purpose ("bebop.bin consumes one `&`, the right operand parses the second, no short circuit") but not the consequence.

**Measured through bpref** (which models `'&&'` as `&` at the `&` tier: `tools/bpref.py:111`, `:120`):
- `fn isup(c) { if c >= 65 && c <= 90 then 1 else 0 }` over c = 32, 65, 97, 0, 90 -> **11111**: the character-class test is a tautology.
- `c46_andor.bp` -> **111100**, the frozen golden; digit two is `q = if 3 > 5 && 5 < 9`, which a logical AND scores 0 and the golden scores **1**. The golden freezes the mis-parse. Under logical `&&`/`||` the construct's value is 101100.
- `if 0 == 1 && boom() == 1 then 5 else 9` with `boom` calling `sys_exit(77)` -> exit **77**: no short circuit, the right operand runs with a false left operand (the F2 row's own probe, reproduced).
- `6 && 3` -> **2** on bpref. Reading the compiler: the second `&` reaches `emit_factor`'s fallback `emit_num` (`bebop.bp:4418`), which reads zero digits and emits a literal 0 without advancing (`:3458-3466`), after which `emit_band`'s loop consumes the second `&`; that gives `(6 & 0) & 3 = 0`. SPECULATIVE until run on `bebop.bin`: if it prints 0, the oracle and the compiler disagree on `&&` outside the 0/1 case and the parity gate has never had a construct that could show it. This is the one probe the fixing lane should run first.

**Where it bites.** `selfhost/std/qdsl.bp:35` `if (c >= 65 && c <= 90) then 1 ...` and `:63` `if (c >= 48 && c <= 57)` are exactly the tautological shape, in the production query parser. With the regex "comparison, operand, `&&`/`||`, operand, comparison" over every `.bp` outside comments this lane counts **102** sites in six files: `c70_qdsl.bp` 36, `selfhost/std/qdsl.bp` 35, `c70_qdsl_neg.bp` 24, `c46_andor.bp` 5, and the `morph.bp` twins 1 each; `bebop.bp` has 0 (its one hit, `:3182`, is a comment). The operator's count of 34 in four files used a criterion this lane could not reproduce; the files agree, the number does not, and the lane fixing the compiler should publish its regex. `c70_qdsl.bp`'s golden 41000 (`:3`) was frozen over 36 such sites and must be re-derived with the fix, as must c46's.

**Check, in either outcome of the fix in flight.** (A) If `&&`/`||` stay `&`-tier: one test in `emit_band`/`emit_bor` -- `ch == 38 and ch1 == 38` (and `124`/`124`) -> `diag_exit(s, pos[0], 114)`, text "`&&` is not an operator: `&` is bitwise and binds tighter than a comparison; parenthesise each comparison or nest `if`". Two lines, zero words, and the 102 sites are rewritten. (B) If the fix makes `&&`/`||` a loosest tier with short circuit: the mis-parse becomes unrepresentable by grammar, the `LANGUAGE.md:57` table gains the tier, the neg construct is replaced by the re-goldened c46 plus a `c46b_andor_short` that exits 0 where today's exits 77, and `tools/bpref.py:120` changes with it or the oracle diverges. Either way the census row is "logical-and-is-bitwise", source `c46_andor.bp:1-2`, and it closes only with a construct.

### 3.7 Unresolved call at compile time (115)

**Today.** `collect_fns` (`bebop.bp:6023`) records every `fn` name and position before emission (`fnames[cnt[0]] = name` at `:6088`); `fntab_lookup` (`:675-688`) returns -1 for a name not in the table; `emit_call_resolve` (`:5853`) folds that into `okres`, and `emit_call` (`:5878`) emits the `brk #87` word at **`:5893`** when `okres == 0` -- the only site emitting that word (`3558869728` occurs once in the source). The trap fires at run time (`docs/TRAPS.md` 87 row; `neg/c52_undef.bp:3` expects `RUNFAIL:87`).

**Check.** At :5893 the name is already known to be absent from a complete table, so `diag_exit(s, pos[0], 115)` replaces the `em(...)` -- the word is deleted, which is the F2 gate's "`bin_words` delta <= 0". `c52_undef` flips from `RUNFAIL:87` to `COMPILEFAIL:115`; 87 stays documented for old binaries. Builtins and enum ctors are dispatched before this path (`emit_call_or_ctor`, `:1867`), so they are unaffected.

### 3.8 Literal `sys_clone(_, 0)` (116)

**Today.** `emit_sys_clone` (`bebop.bp:1298`) parses both arguments with `emit_cmp` and checks only the symbol count (§3.4). `docs/WORKER-CARD.md:24`: "never stack_top 0: the child rebinds x14/x15/x27/x28 to [stack_top, +12 MiB)". Census row `clone-stack-top-0` (`trap_census.py:219`): none.

**Check.** After the comma, `skip_ws` then `char == 48` followed by `)` -> `diag_exit(116)`. Literal only; a computed 0 is a runtime fact this row does not claim.

### 3.9 `use` off column 0 (118)

**Today.** `use_scan` (`bebop.bp:7045`) recognises an include only at beginning of line: `is_use = bol * ...` at `:7054`. A `use "..."` preceded by a space is not an include; the text then reaches `collect_fns`, which advances one character over anything it does not recognise, so the line vanishes silently and the only symptom is a missing `<out>.bin.use` (`docs/WORKER-CARD.md:21`; census row `use-line-not-seen`, `trap_census.py:209`).

**Check.** In the same loop: `use "` with only spaces/tabs between the last newline and `u` -> `diag_exit(srcv, i, 118)`. A comment line (`//` before it) does not match, so `// use "x"` stays legal.

### 3.10 Argument-count mismatch at a call site (93) -- MEASURED, and the oracle agrees with the compiler

**Today.** The `bl` path `emit_bl_call` (`bebop.bp:830`) counts arguments as it emits them (`nargs` at `:833`, `:843`) and uses the count only for register placement; the callee's parameter count exists only when its own header is parsed (`parse_params`, `:277-303`, which caps at 14 -> exit 100 at `:302`), and `fntab` holds names and start offsets (`fntab_lookup`, `:675-688`), no arity. **Measured (bpref):** `fn f(a, b, c) -> i64 { a*100 + b*10 + c }` called `f(1, 2, 3, 4, 5)` -> **123**, rc 0 -- the oracle discards the extra arguments too, so `bpref_diff` can never surface this. In the tree: `selfhost/std/qdsl.bp:165` declares `qdsl_cc` with SEVEN parameters and `:223, :236, :265, :287` call it with EIGHT, so `kl` receives the fifth keyword character, as the row says.

**Check.** `collect_fns` records `parse_params`' count beside the name (one more parallel array, the arity table A21 also needs -- land once); `emit_bl_call` compares `nargs` after the closing `)` (`:851` is where it already validates the `,`/`)` shape) and exits 93 with the call's position. Too few arguments is the same comparison.

### 3.11 Positions on every remaining exit-89 site

Six `diag_exit(..., 89)` sites carry a position (`bebop.bp:2349, 2355, 4490, 5957, 6187, 6448`); two `sys_exit(89)` do not (`:918`, `:2824`, §3.5). Both are inside the register model with no source handle; the fix is the stashed statement position of §3.5.

## 4. Code assignment

| code | trap | status on `ddb534f` |
|---|---|---|
| 93 | argument-count mismatch | free; reserved by row F2 |
| 105 | read before assignment | free again since `947246e` |
| 106 | loop-literal escape (unsafe `loop_alloc_safe` verdict) | free again |
| 107 | `zeros` in a `while` body | free again |
| 109 | > 8 kept symbols across `sys_clone` | LANDED (row said 108) |
| 114 | `&&`/`||` mis-parse (option A) | free; replaces the row's 112, taken by module contents |
| 115 | unresolved call at compile time | free |
| 116 | literal `sys_clone(_, 0)` | free |
| 118 | `use` off column 0 | free |
| (89 sites) | nesting cap / unplaced arg, with position and true text | existing code, new text |

Every new code gets a `diag_exit` text (`bebop.bp:92-107` ladder), a `docs/TRAPS.md` row, and one `neg/` construct with `// EXPECT COMPILEFAIL:<code> from:` in its header (`construct_parity.sh:5`); `trap_census.py --texts` must stay at agreement (22 today) plus the count landed.

## 5. Gates

| step | name | lands | gate | predicate |
|---|---|---|---|---|
| 0 | census instrument repair | `scan_neg_expects` reads `// EXPECT` headers; five stale `ROWS` re-derived; baseline recorded in this file | `trap_census.py` prints a non-zero `bench/.../neg` code count; `trap_unrep` baseline `k0/20` recorded; `--texts` still 22/22/22/22 | success |
| 1 | delete-a-word traps | 115 (§3.7), 116 (§3.8), 118 (§3.9) | `neg/c52_undef` -> `COMPILEFAIL:115`; two new negs MATCH; `bin_words` delta <= 0 with a `word_budget` line; fixpoint `gen3 == gen4` | success |
| 2 | arity | 93 (§3.10) with the arity array in `collect_fns` | `neg/c_arity_over`, `neg/c_arity_under` MATCH; the four `qdsl.bp` sites corrected and `c70_qdsl` re-goldened; A21 consumes the same array | success |
| 3 | `&&` family | 114 (option A) or the re-goldened c46 + short-circuit construct (option B) | `c46_andor` golden re-derived by hand from logical semantics (101100 under B); bpref and bebop.bin agree on `6 && 3`; the 102 sites rewritten or made correct by grammar | success |
| 4 | loop and allocation | 106, 107 (§3.2-3.3) | 544-file control arm: files changing exit status listed by name; negs MATCH; L8 violations in `bebop.bp` counted, not proxied | success |
| 5 | definite assignment | 105 (§3.1) | `neg/read_before_assign` REWRITTEN to the loop shape; control arm as step 4; `read_before_assign` census row closed | success |
| 6 | positions and texts | §3.5, §3.11 | no `sys_exit(89)` without a preceding position; `--texts` agreement holds | success |
| all | row F2 gate | | `trap_unrep >= k0 + 9` of 20 (nine rows above; 109 already counted at step 0); construct parity `86+N/0`; battery GREEN | success |

The row's `gen_avoid shrinks by 12` is not gated: `bench/fuzz/gen.py` contains no `avoid` string and no `# LANG-AVOID:` markers (§1.2), so the number is a human judgement until the one-line marker convention lands.

## 6. Files and functions touched

| file:fn | change | anchor |
|---|---|---|
| tools/trap_census.py:scan_neg_expects | read `// EXPECT (COMPILEFAIL\|RUNFAIL):<code>` from `bench/parity_constructs/neg/*.bp` headers | `:106-118`; format at `construct_parity.sh:22-26` |
| tools/trap_census.py:ROWS | re-derive the five stale rows; drop the `NOT PRESENT` line's first half | `:191-253`, `:613` |
| bebop.bp:emit_call | `diag_exit(115)` replaces the `brk #87` word | `:5893` |
| bebop.bp:emit_sys_clone | literal-0 stack top -> 116 | `:1298`, after the second-argument `emit_cmp` |
| bebop.bp:use_scan | indented `use "` -> 118 | `:7054` (`is_use = bol * ...`) |
| bebop.bp:collect_fns, emit_bl_call | arity array beside `fnames`; compare `nargs` at the `)` -> 93 | `:6088`; `:833`, `:843`, `:851` |
| bebop.bp:emit_band, emit_bor | `&&`/`||` -> 114 (option A) or new tier (option B) | `:3554`; `:3515`; tier comment `:3513` |
| bebop.bp:emit_while_stmt, loop_alloc_safe | unsafe verdict -> 106 with the escaping `let`'s position | `:5410-5411`; `:4785` |
| bebop.bp:emit_zeros | `fntab[5246] != 0` -> 107 | `:6733`; depth written `:5298-5299` |
| bebop.bp:sym_bind, emit_ident, emit_let_stmt | assigned-bit per symbol (128 cap, two words) -> 105 | `:223`, `:244`; `:307`; `:5466` |
| bebop.bp:vs_push (:2824), vs_place_args (:918) | position + true text at the two positionless 89s | `:2824`, `:918` |
| bebop.bp:diag_exit | one text line per new code | `:92-107` |
| docs/TRAPS.md | rows for 93/105/106/107/114/115/116/118; correct :65-111, :119, :129 | `:56` (201 row is the model) |
| docs/LANGUAGE.md | `&&`/`||` line in the precedence table | `:57-70` |
| selfhost/std/qdsl.bp | eight-argument `qdsl_cc` calls -> seven; char-class tests parenthesised per comparison | `:165`; `:223, :236, :265, :287`; `:35`, `:63` |
| bench/parity_constructs/neg/ | one construct per code; `read_before_assign.bp` rewritten | header format `construct_parity.sh:5` |

## 7. Cost and schedule

- Step 0: ~40 lines of Python, half a day; it is the only step whose output is a number this row is measured by.
- Steps 1-3: ~120 lines of `bebop.bp` (the row's "~300 lines total" covered §3.1-3.3 as well), one `bebop.bp` lane, ~1 week, each step behind the compile token and a battery.
- Steps 4-5: ~120 lines plus the control arm each; the control arm (544 files, before/after, named diff) is the cost, not the check. ~1 week. SPECULATIVE: the false-rejection rate of the definite-assignment approximation decides whether step 5 is a week or a month.
- Step 6: ~30 lines, contingent on a free `fntab` cell.
- Words: zero added; one word (`brk #87`) removed per unresolved call site, which in a program that compiles is zero sites, so the delta is exactly 0 on every gated file.
- The row's "~3 weeks" holds if the `&&` fix in flight lands as option B; option A adds the 102-site rewrite and two re-goldenings.

## 8. Out of scope

- The three word-costing rows (unchecked bounds, read past end, scanner aliasing): `docs/blueprints/F2-bounds-by-type.md`.
- A19 step 1b (the clone check's two over-approximations, `docs/TRAPS.md:33`).
- Whether user diagnostics should move below 99: `947246e` chose not to; this row follows the binary as promoted.
- Fixing `&&` itself: a lane is doing it; this row specifies the check and the constructs that must exist whichever way it lands.

## 9. Attribution

- Row text and gate: ROADMAP.md:196 (row F2); predecessor and census decision: ROADMAP.md:195 (row F1); `diag_texts` 19: ROADMAP.md:202 (row F8); builtin-surface precedent: ROADMAP.md:194 (row F0).
- Census numbers today: `python3 tools/trap_census.py`, `--texts`, `--shadow`; `python3 tools/undef_census.py`; all on `ddb534f`.
- Partition landed: `git log -1 947246e` (2026-09-09); `bebop.bp:2240` (`selfcheck_exit`), `:2438-2463`; `docs/TRAPS.md:56`.
- Header format change: `git log -1 d9890da` (2026-09-13); `bench/vs_rust/construct_parity.sh:5, :22-26`; `tools/trap_census.py:106-118`, `:568`.
- Census created: `25c0835` (2026-09-09); A17 step 2 agreement: `6fd6b5f` (2026-09-13). Every 7-character hash above returns `commit` from `git cat-file -t`; `072c01d1` is `md5sum bebop.bin | cut -c1-8`, the digest form `tools/battery.sh:56` builds.
- Free-code list: derived by intersecting `docs/TRAPS.md` table rows with `diag_exit`/`sys_exit`/`cli_exit` literals in `bebop.bp` (comments stripped); 65 at `bebop.bp:4296`, 85/86 at `docs/TRAPS.md:46-47`, 84 at `docs/blueprints/F2-bounds-by-type.md:85`.
- Precedence: `bebop.bp:3513` (tier comment), `:3547-3554` (`emit_band`), `:3597-3598` (`emit_cmp`), `:3590-3592` (`emit_apply_cmp`), `:4418` and `:3458-3466` (`emit_factor` fallback into `emit_num`); `docs/LANGUAGE.md:57-70`; `bench/parity_constructs/c46_andor.bp:1-2, :7-11`; `tools/bpref.py:111, :120`.
- bpref measurements (this lane, 2026-09-13): char-class probe 11111; c46 111100; boom probe exit 77; `6 && 3` = 2; arity probe 123; read-before-assign probe `KeyError: 'x'`. `bebop.bin` was not run by this lane.
- Site count 102 in six files: `grep -rnE` with the regex described in §3.6 over `--include='*.bp'`, comment lines excluded; `bebop.bp:3182` is the comment hit.
- Symbol cap 128: `bebop.bp:244`, `:4455`, `:5941`, `:6216`; UNDEFINED read: `docs/LANGUAGE.md:53-54`; law L8 and clone rules: `docs/WORKER-CARD.md:20, :21, :24`.
- Loop allocation: `bebop.bp:5410-5411`, `:4785`; loop depth `fntab[5246]`: `:5298-5299`, `:5440`, `:6203`; `emit_zeros` `:6733`.
- Clone check: `bebop.bp:1312`, `docs/TRAPS.md:33`, `c142_clone8.bp:10`, `neg/c141_clone9.bp:8`; `sys_mapb` 108: `bebop.bp:7735`.
- Unresolved call: `bebop.bp:675-688`, `:5853`, `:5878`, `:5893`; `neg/c52_undef.bp:3`.
- `use` at column 0: `bebop.bp:7054`; `collect_fns` `:6023`, name record `:6088`.
- Arity: `bebop.bp:277-303`, `:830-851`; `selfhost/std/qdsl.bp:165, :223, :236, :265, :287`.
- Exit-89 sites: `bebop.bp:918`, `:2824` (no position); `:2349, :2355, :4490, :5957, :6187, :6448` (with position); 95 sites: `:851, :3726, :4005, :4028, :5532, :5906, :5922`.
- Misfiled neg: `bench/parity_constructs/neg/read_before_assign.bp:1, :4`.
- Origin of "24 rows / 20 open / 16 zero-word": `docs/RESEARCH-VERIFICATION-2026-09-09.md:7, :49, :80`.
