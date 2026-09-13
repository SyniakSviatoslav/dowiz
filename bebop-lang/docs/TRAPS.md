Status: 2026-09-09 CURRENT (F1 census; supersedes the 2026-09-06 T120 header. Two corrections and two new sections at the bottom were derived from bebop.bp by `tools/trap_census.py`, not transcribed. Original header: T120, decision D11-M; runtime traps 80/81/87 are one `brk #code` word each and the entry stub's SIGTRAP handler prints the row's text, T90 2c; the single table of exit codes — a code that is not here is a bug)

# Exit codes

| code | who | meaning | where |
|---|---|---|---|
| 0..63 | program | `main`'s value modulo 256 is NOT the exit code: the seed prints the value and exits 0; a program exits non-zero only through `sys_exit` or a trap | seed.S |
| 8 | was | silent `brk #8` (pre-A13 for > 14-parameter fns); since A13 (2026-09-08) a compile-time exit 100 instead | — |
| 64 | bebop.bin | `check` without a file. It now PRINTS `0:0: error[E64]: unknown CLI command (compile, check, size, version, run-via-exec, cas) or check without a file` and exits 64 (`cli_exit`, ROADMAP A17, 2026-09-13); before that it printed nothing at all. **CORRECTED 2026-09-13 by triggering it rather than reading the source: the "unknown CLI command" half of this row is NOT an exit code.** `main` ends `let r5 = if known == 0 then 64 else 0; r1 + ... + r5 + ...`, so an unrecognised verb makes main RETURN 64, which the seed prints on STDOUT, and the process exits **0**. Verify: `seed bebop.bin notaverb` prints `64` and `echo $?` gives 0, while `seed bebop.bin check` prints the error line and gives 64. Two different mechanisms wore one number and one sentence here for as long as the row existed. The text above is kept as-is because it is what the compiler now prints, but only the `check`-without-a-file path can produce it | bebop.bp main, cli_check, cli_exit |
| 65 | bebop.bin | index past the end (F3 static bounds check, 2026-09-13): a subscript `a[i]` is rejected at compile time when `i` is a constant outside `0..a.len-1`. Raised to code 65 (marked as free by F1's 2026-09-09 census) when the rewrite from asserts into compile-time traps lands. `<line>:<col>: error[E65]: index past the end` on stderr (`emit_array_index`, `diag_exit`) | bebop.bp |
| 80 | program | arena exhausted: a `zeros` crossed x28 (T118); fires **65536 bytes earlier** since 2026-09-12 (telemetry blueprint step 1: x28 lowered by slab size). Protects a 64 KiB telemetry ring at immediate index (bebop.bp:6934-6947). `brk #80`, stderr `trap 80: arena exhausted (zeros crossed x28)` (T90 2c) | emit_zeros, entry_stub handler |
| 81 | — | **RETIRED 2026-09-08 (ROADMAP A6).** Was: frame heap exhausted — an array literal / enum ctor crossed the 16 KiB frame (T118). A6 allocates aggregates on the arena cursor x27 instead of a per-activation frame heap, so `emit_heap_trap` and every `brk #81` are deleted from the compiler; an aggregate that outgrows the 256 MiB arena now takes exit **80** with `zeros`. The code is left unassigned rather than reused, so an old binary that still traps 81 keeps its meaning. |
| 82 | program | SIGSEGV or SIGBUS in the program: stack overflow (deep recursion at 16 KiB per frame) or a wild access; the entry stub's handler writes `trap 82: SIGSEGV/SIGBUS (stack overflow or wild access)` and exits 82 on an alternate stack (T118b, T90 2c) | entry_stub |
| 83 | bebop.bin | code buffer exhausted: one fn emitted 65536 words (the planning-pass buffer) or the program 262144 (before 2026-09-06 this was a SIGSEGV = exit 82); `code buffer exhausted at <cap> words (one fn or the program)` on stderr | em / cap_exit |
| 87 | program | a call to a function the compiler never resolved was executed (T130; before it the call silently yielded 0); `brk #87`, stderr `trap 87: call to an unresolved function` | emit_call, entry_stub handler |
| 88 | bebop.bin | `use "cas://sha256:<hex>"`: the module's SHA-256 differs from its name (T80). Now PRINTS `0:0: error[E88]: CAS module hash does not match its name` and exits 88 (`cli_exit`, ROADMAP A17, 2026-09-13); before that it printed nothing. Triggering it needs a real mismatch: a 77-char `cas://sha256:` + 64 hex path whose `.bcas/<hex>.bp` hashes to something else | cas_verify, cli_exit |
| 89 | bebop.bin | REGISTER-MODEL-BLUEPRINT: a `let` binding's register collides with a live cs temp (bind the call result with a let first), or S + tsp > 64 (§5, this compiler spills no further), or the fn/enum-ctor table is full (**768** since the fntab relayout `8d4a477`; this row said 512 until 2026-09-12, when a research pass caught the drift and the compiler was checked directly -- `bebop.bp:4410` and `:6198` both read `if cnt[0] >= 768 then diag_exit(s, 0, 104)`, and `c123_capfns` is a 756-fn construct that compiles. It was **512** from A2 step 0 2026-09-06/07 -- this row said 256 until the F1 census diffed it against bebop.bp:3509-3513; the arrays are `zeros(512)`: `compile`/`compile_fn`/`compile_program_offs`'s fnames/fpos/sizes/starts arrays are fixed-size — a 257th fn used to overflow `starts[256]` into the next arena cell silently, shifting every later word by one and producing a SIGILL binary instead of a diagnostic), or expression nesting / pending operands > **512** (the window's entry list at **fntab[2800..4335]**; this row said 128 slots at fntab[2000..2383] until the F1 census diffed it against bebop.bp:3515-3516) | sym_bind call sites (via check_reg_collision), compile_fn_at, compile/compile_fn/compile_program_offs, vs_push |
| 90 | seed / bebop.bin | open failed (source, output or the .bin to run; since T129 also the compiler's output or `.use` temp). Now PRINTS `0:0: error[E90]: open failed (source, output or binary file)` and exits 90 (`cli_exit`, ROADMAP A17, 2026-09-13); before that it printed nothing. **The `source` case did not exist until 2026-09-13 even though this row had always claimed it.** `cli_compile` opened the source with `sys_open(psrc, ls, 0)` and went straight into `sys_slurp(fd, 400000)` with NO `fd < 0` check -- the one unguarded file open in the compiler -- so `compile /typo.bp out.bin` slurped an invalid fd, compiled the resulting EMPTY program and exited **0**. Found by triggering the diagnostic instead of trusting that it fired; the guard and a gate for it landed in the same commit (`diag_check.sh` now asserts all three CLI exits, 19 pass) | seed.S, cli_compile, cli_exit |
| 91 | seed | read failed | seed.S |
| 92 | seed | mmap failed | seed.S |
| 94 | seed | generic failure | seed.S |
| 95 | bebop.bin | expected `)` (emit_paren) or `in` (let-expression) | bebop.bp |
| 96 | bebop.bin | `++` in an expression: string concatenation is not in the surface (T42 d) | emit_expr |
| 97 | bebop.bin | fn body without a tail expression (T42 c) | compile_fn_at |
| 98 | bebop.bin | more than 17 `return` / 18 pending `break` in one fn (T99) | emit_return_stmt / emit_break_stmt |
| 99 | bebop.bin | reserved word used as a function name (T122, planned) | compile_fn_at |
| 100 | bebop.bin | fn with more than 14 parameters (parse_params) | bebop.bp |
| 101 | bebop.bin | unbound symbol (emit_let_chain / emit_compound_stmt / emit_ident / emit_array_index) | bebop.bp |
| 102 | bebop.bin | a `sys_` name inside a `kernel fn`: the C1 checked dialect forbids reaching the kernel from a kernel (ROADMAP C1 step 2, 2026-09-08); `<line>:<col>: sys_ name inside a kernel fn` on stderr. Marker-free fns are untouched — fntab[4643] is 0 for them, so an unmarked fn's emitted words are byte-identical | emit_ident, diag_exit |
| 103 | bebop.bin | parallel region: too many live symbols across a `sys_clone` spawn (binary 292b8953 only, 2026-09-12): this trap was emitted by the promoted binary 292b8953 but its source was lost in commit e5b64f7. CORRECTED 2026-09-12, later the same day: the `<= 8` limit is REAL and the earlier retirement note on this row was wrong. The limit is on symbols that must be KEPT across the spawn -- anything the register allocator cannot rematerialise, i.e. array handles and call results. Constants do NOT count. Measured 2026-09-12 on 7c7d1f77 with a probe whose child writes a marker and whose parent waits for it: 8 kept symbols -> both children correct; 9 -> ONE child silently lost; 11 -> BOTH lost. The failure is SILENT: no trap, no diagnostic, the children simply never run. (An earlier note this same day claimed the rule was retired because 32 live symbols worked -- that probe used i64 CONSTANTS, which are rematerialised and cost nothing, so it measured the wrong thing.) The withdrawn binary's own check was crude -- it counted in-scope symbols including constants -- but its INTENT was right and removing it left a silent failure uncovered. Re-landing a correct check (count only symbols kept across the spawn) is an OPEN row. The compiler was re-promoted 2026-09-12 to 7c7d1f77 (== compile(bebop.bp)), so this code is no longer emitted by anything in the tree. The number stays RESERVED -- do not reuse it | — |
| 104 | bebop.bin | too many fns (cap **768** since `8d4a477`; this row said 512 until 2026-09-12 -- the guard is `bebop.bp:4410,6198`): compile_program_offs guarding fnames/fpos/sizes/starts arrays (A13 second half; was exit 89) | bebop.bp |
| 108 | bebop.bin | `sys_mapb` is REFUSED (ROADMAP A7, 2026-09-12). Its emitter passed the path's LENGTH as openat's FLAGS (`and x3,x0,#0xffffffff` then `openat(-100, x2, x3, 0)`) and took the path pointer from a `str` handle whose bytes nothing copies into the arena, with no copy loop and no NUL terminator -- `emit_sys_open` and `emit_sys_rename` both marshal a byte-per-cell path into the 8 KiB IO scratch below x28 first. It wrote nine files named after raw addresses, ~160 MB, into git history. The builtin has no call site in the tree; it returns when A7 step 2 re-lands whole. | emit_sys_mapb |
| 109 | bebop.bin | `sys_clone` is REFUSED when the enclosing function has more than 8 symbols bound at the spawn point (ROADMAP A19, 2026-09-13). Measured: 8 symbols kept across a spawn -> both children correct, 9 -> one child SILENTLY lost, 11 -> both. Symbols 1-8 live in x19..x26 and are copied to the child; a 9th lives at a spill slot `[x15,#k*8]` on the parent's stack, and the child re-homes x15 onto its own fresh 12 MiB stack, so the spilled symbol reads 0 there -- no trap, no diagnostic, the child simply never writes. **The check is deliberately CONSERVATIVE and over-approximates in two ways, both MEASURED 2026-09-13 on `7bdc0f11`, not assumed.** (i) It counts `stab[0]`, every symbol bound so far, INCLUDING i64 constants -- which the register allocator rematerialises and which are therefore safe in practice, so a fn binding 8 constants before a spawn is refused although it would have run. (ii) In the near-universal idiom `let r = sys_clone(...)` the pending binder `r` is entered into `stab` BEFORE its RHS is emitted, so it is counted too, and the effective budget is **7 prior symbols, not 8**; with the call in tail position (no binder) it is 8. Both over-approximations are the direction ROADMAP A19 step 1 asks for ("conservative, CHECK 20's direction") and neither breaks the tree, and that was TESTED rather than spot-checked: all **25** `sys_clone` callers in the tree were compiled on `7bdc0f11` with the pre-A19 compiler as a CONTROL arm, and every file that compiled before still compiles (sconc, smw, b6core, sgraph2, gb_pool, gb_bfs_gen_addr_gate, nn4, pool, bebop.bp itself and the three D4-clone fuzz repros among them). The only movement is `bench/wip/gb_par_{mxm,mxv,reduce}.bp`, which exited **101** (unbound symbol) before A19 and exits **109** now: the spawn check fires before symbol resolution finishes, so in an already-broken file this diagnostic can MASK a more fundamental one. No gated file is affected, but do not read a 109 from a file you have not also compiled on a pre-A19 compiler as proof that the clone count is its only problem. Correcting (ii) is A19 step 1b. Contrast the WITHDRAWN trap 103, which refused working programs for a different and unfixable reason -- it used FUNCTION-WIDE counters (`fntab[5393]`+`fntab[5391]`) rather than a point-specific count. Boundary pinned from BOTH sides: `c142_clone8` (8 bound, compiles, 201) and `neg/c141_clone9` (9 bound, COMPILEFAIL:109). Message: `line:col: sys_clone: > 8 kept symbols (max 8)` on stderr (`emit_sys_clone`, `diag_exit`) -- note that text names the `stab` count, so under (ii) it reads one higher than the number of symbols a reader would count by hand | emit_sys_clone |
| 110 | bebop.bin | invalid syntax in contract position (ROADMAP F8 step 0, 2026-09-13): the compiler discards text between `)` and `{` in fn headers and skips unrecognised top-level text character-by-character, so arbitrary invalid characters were silently accepted in requires/ensures/theorem positions. The lexical validator `scan_inert` checks these regions and refuses characters outside the allow list: whitespace, identifier chars, and type/contract punctuation `- > < = + * / . : , ( ) [ ] ;`. Rejected immediately on discovery. Message: `line:col: error[E110]: invalid syntax in contract position -- use only identifiers, type names, and operators` on stderr (`scan_inert`, `compile_fn_at`, `collect_fns`, `diag_exit`) -- recovers by refusing the file, not by rewriting the contract. Negative control `c143_contract_garbage` (theorem garbage REJECTED), positive control `c144_contract_ok` (valid requires/ensures accepted). The check spans two syntactic positions: (i) in fn headers after `parse_params` returns, before the opening `{` (compile_fn_at:6146, the blind `skip_to`), and (ii) at top level, from the end of the `theorem` keyword to end-of-line (collect_fns:6023-6027, newly-recognized keyword since F8 step 0). Both emit exit code 110 when garbage is found. With both checks active, the ratchet for "garbage-accepting positions" improved from 4 to 0 (F8 step 0's f8_dt gate, RATCHET_VALUE in tools/f8_dt.py set to 0 after measurement). | scan_inert, compile_fn_at, collect_fns |
| 128+n | kernel | signal n: 11 = SIGSEGV (unchecked index, deep recursion), 7 = SIGBUS (misaligned sp — a compiler bug) | — |

## Store program exit codes (selfhost/prelude/store.bp)

These are exit codes emitted by the `store.bp` library program, not the compiler. They share the integer space but are a separate process.

| code | who | meaning | where |
|---|---|---|---|
| 85 | store.bp | `st_seal` refused a length that cannot be an object length (> 2^28 cells): the header cell has been overwritten. Without it `st_crc` simply walks the bogus length -- measured 2026-09-12, a clobbered PartTab header gave 1329743170 and the crc walked 10.6 GB out of a 64 KB mapping, so the process died with trap 82 in a different function from the one with the bug | st_seal |
| 86 | store.bp | `st_open` refused a store whose live superblock claims more cells than were mapped for it. st_open ftruncates to `size`, so opening an existing store at the WRONG size silently cuts the arena off mid-object and every later read traps 82 far from the cause. Checked BEFORE st_reopen_verify, which would otherwise die inside the object walk. Nine gb gates were red for this class on 2026-09-12 | st_open |
| 87 | store.bp | `st_open`/`st_map_ro` refused a store whose version cell does not match. Cell 1 has been written by every writer since the format existed and read by NOTHING, so a schema divergence was silent: B5 step 1 changed the layout under nine separate readers and each discovered it separately over four hours, always via `trap 82` several functions from the cause | st_open, st_map_ro |

## Compiler self-check exit codes

These are exit codes emitted by the compiler when internal assertions fail. They indicate bugs in the compiler, not in the program being compiled.

| code | who | meaning | where |
|---|---|---|---|
| 201 | bebop.bin | compiler self-check: an internal invariant failed. The diagnostic includes the check kind and site for debugging (ROADMAP A17 step 1, 2026-09-13). Format: `0:0: error[E201]: compiler self-check <kind>.<site>`. Kinds: 1 = window-mask register out of range, 2 = window mask corrupt past bit 7, 3 = cs-mask register out of range, 4 = cs mask corrupt past bit 7, 5 = window free-mask not fully released at a boundary, 6 = cs mask not released at a boundary, 7 = cs mask non-zero at a statement boundary (was 103), 8 = window not empty where it must be (was 107) | selfcheck_exit |

Gates: compile-time traps are `bench/parity_constructs/neg/*.bp` with `EXPECT=COMPILEFAIL:<code>`,
run-time traps with `EXPECT=RUNFAIL:<code>` (bench/vs_rust/construct_parity.sh); the fuzzer
(bench/fuzz/fuzz.sh) classifies a trap the oracle predicted as TRAP-OK.


---

## The exit-code space is NOT partitioned (F1, 2026-09-09)

This table opens by saying "a code that is not here is a bug". By its own rule
the compiler contains dozens. Derived with `python3 tools/trap_census.py --codes`:

**Fixed codes bebop.bp emits that this table does not carry:** 105 (`4937`), 106 (`5001`, `5025`), 107 (`5333`). All three
are register-model **self-checks** — assertions that the window mask
`fntab[4548]` is 255 and the cs mask `fntab[4573]` is 0 at a statement or loop
boundary. They are compiler-bug detectors, not language traps, and they are
invisible to every worker who has been told this table is complete. (Code 103
was a self-check at `bebop.bp:5325` but is now a user-facing diagnostic for the
parallel region live-symbol check at `emit_sys_clone`, landing ROADMAP B6.)

**Computed codes: four sites span forty.** `vs_mask_take` / `vs_mask_free` /
`vs_cs_take` / `vs_cs_free` (`bebop.bp:2271-2295`) exit with

| site | expression | span |
|---|---|---|
| `bebop.bp:2273,2279` | `sys_exit(99 + site)` | 100..119 |
| `bebop.bp:2271,2277` | `sys_exit(100 + site)` | 101..120 |
| `bebop.bp:2289,2295` | `sys_exit(119 + site)` | 120..139 |
| `bebop.bp:2287,2293` | `sys_exit(120 + site)` | 121..140 |

`site` is a caller-chosen constant in 1..20, so the compiler can exit with **any
code in 99..140**. That range contains every documented compile-time diagnostic:
99 (reserved word as fn name), 100 (>14 params), 101 (unbound symbol), 102
(`sys_` in a `kernel fn`), 104 (too many fns). **A worker who sees exit 100
cannot tell "fn with more than 14 parameters" from "vs_mask_take site 0 range
violation".** Two of these aliases are live in the tree today: code 102 means
both the kernel-fn diagnostic (`bebop.bp:310`, user-facing) and a window-mask
self-check (`4912`, `5024`, `5608`, `5815`); code 104 means both "too many fns"
and a cs-mask self-check (`5609`, `5816`).

### This blocks ROADMAP F2 as written

F2 plans to assign **105** to definite-assignment, **106** to loop-literal
escape, **107** to `zeros` in a `while` body, and codes up to 118 to the rest.
All three of those are already emitted by the self-checks above, and 108-118 sit
inside the computed span. F2 must pick a free range before it writes a line of
code, and the census says which codes are actually free:

> **65-79, 84, 85, 86, 93 — nineteen free codes below 99**, against F2's twelve.

The durable fix, which is zero words and should land with F2's first code:
**partition the space** — user-facing diagnostics below 99, compiler self-checks
above 200, and replace `sys_exit(base + site)` with a single self-check code plus
the site number on stderr, the way `diag_exit` already prints a position.

## The trap census (F1)

`tools/trap_census.py` derives the census from `docs/WORKER-CARD.md`,
`docs/LANGUAGE.md`, this file, `bebop.bp` and `bench/parity_constructs/neg/`, and
`tools/undef_census.py` counts the undefined-behaviour sites. Today:

    trap_unrep: 10/29           shadowable_builtins: 7 of 36
    trap_zero_word: 16 of 19 open rows   trap_needs_words: 3 of 19
    undef_census: 2             gen_avoid: 3 PROVISIONAL

A row is **closed** only when it has BOTH a landed mechanism AND a
`bench/parity_constructs/neg/` construct: a mechanism with no neg construct is
one refactor away from silently disappearing.

**Builtin shadowing is a short table, not a missing mechanism.** `fn char(...)`
is rejected (exit 99); `fn clz(...)` compiles rc=0 and `clz(8)` returns 60. The
T122 hash table at `bebop.bp:5753-5756` carries 45 names and misses exactly
seven of the 36 dispatched builtins: `clz`, `crc32`, `crc32x`, `sys_msync`,
`sys_run`, `sys_setaffinity`, `sys_wait4`. Adding seven hashes is the whole fix.
(A warning for whoever re-derives this: `bebop.bp` spells the same 64-bit hash
signed in the reserved table and unsigned in the dispatch ladder, so comparing
the literals as written reports 16 shadowable builtins. Normalise to unsigned
and it is 7. `trap_census.py` does.)

## What is NOT a trap: the `sgraph2.bp` twin

`selfhost/std/sgraph2.bp` and `bench/vs_rust/std_tests/sgraph2.bp` are
near-duplicates, and drift between them cost a lane a measurement. It does not
belong in this census. A trap is a program the LANGUAGE accepts and gives a
meaning no author intended; this is two files in a build system, and the
language is not involved — `tools/gen_selfsrc.sh std` exists precisely to keep
them identical and `invariants.sh` (v) gates the expansion. Filing it here would
make the trap count a bucket for anything that ever cost someone an hour, and a
count that measures everything measures nothing. It is a **harness defect**, it
belongs in the harness ledger, and the honest fix is the one the tree already
half-implements: make the `std_tests` copy a generated artifact nobody edits, so
the second file cannot exist in an edited state.
