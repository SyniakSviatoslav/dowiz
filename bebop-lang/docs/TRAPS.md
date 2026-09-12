Status: 2026-09-09 CURRENT (F1 census; supersedes the 2026-09-06 T120 header. Two corrections and two new sections at the bottom were derived from bebop.bp by `tools/trap_census.py`, not transcribed. Original header: T120, decision D11-M; runtime traps 80/81/87 are one `brk #code` word each and the entry stub's SIGTRAP handler prints the row's text, T90 2c; the single table of exit codes — a code that is not here is a bug)

# Exit codes

| code | who | meaning | where |
|---|---|---|---|
| 0..63 | program | `main`'s value modulo 256 is NOT the exit code: the seed prints the value and exits 0; a program exits non-zero only through `sys_exit` or a trap | seed.S |
| 8 | was | silent `brk #8` (pre-A13 for > 14-parameter fns); since A13 (2026-09-08) a compile-time exit 100 instead | — |
| 64 | bebop.bin | unknown CLI command (`compile`, `check`, `size`, `version`, `run-via-exec`, `cas`) or `check` without a file | bebop.bp main |
| 80 | program | arena exhausted: a `zeros` crossed x28 (T118); fires **65536 bytes earlier** since 2026-09-12 (telemetry blueprint step 1: x28 lowered by slab size). Protects a 64 KiB telemetry ring at immediate index (bebop.bp:6934-6947). `brk #80`, stderr `trap 80: arena exhausted (zeros crossed x28)` (T90 2c) | emit_zeros, entry_stub handler |
| 81 | — | **RETIRED 2026-09-08 (ROADMAP A6).** Was: frame heap exhausted — an array literal / enum ctor crossed the 16 KiB frame (T118). A6 allocates aggregates on the arena cursor x27 instead of a per-activation frame heap, so `emit_heap_trap` and every `brk #81` are deleted from the compiler; an aggregate that outgrows the 256 MiB arena now takes exit **80** with `zeros`. The code is left unassigned rather than reused, so an old binary that still traps 81 keeps its meaning. |
| 82 | program | SIGSEGV or SIGBUS in the program: stack overflow (deep recursion at 16 KiB per frame) or a wild access; the entry stub's handler writes `trap 82: SIGSEGV/SIGBUS (stack overflow or wild access)` and exits 82 on an alternate stack (T118b, T90 2c) | entry_stub |
| 83 | bebop.bin | code buffer exhausted: one fn emitted 65536 words (the planning-pass buffer) or the program 262144 (before 2026-09-06 this was a SIGSEGV = exit 82); `code buffer exhausted at <cap> words (one fn or the program)` on stderr | em / cap_exit |
| 87 | program | a call to a function the compiler never resolved was executed (T130; before it the call silently yielded 0); `brk #87`, stderr `trap 87: call to an unresolved function` | emit_call, entry_stub handler |
| 88 | bebop.bin | `use "cas://sha256:<hex>"`: the module's SHA-256 differs from its name (T80) | cas_verify |
| 89 | bebop.bin | REGISTER-MODEL-BLUEPRINT: a `let` binding's register collides with a live cs temp (bind the call result with a let first), or S + tsp > 64 (§5, this compiler spills no further), or the fn/enum-ctor table is full (**512** since A2 step 0 2026-09-06/07 -- this row said 256 until the F1 census diffed it against bebop.bp:3509-3513; the arrays are `zeros(512)`: `compile`/`compile_fn`/`compile_program_offs`'s fnames/fpos/sizes/starts arrays are fixed-size — a 257th fn used to overflow `starts[256]` into the next arena cell silently, shifting every later word by one and producing a SIGILL binary instead of a diagnostic), or expression nesting / pending operands > **512** (the window's entry list at **fntab[2800..4335]**; this row said 128 slots at fntab[2000..2383] until the F1 census diffed it against bebop.bp:3515-3516) | sym_bind call sites (via check_reg_collision), compile_fn_at, compile/compile_fn/compile_program_offs, vs_push |
| 90 | seed / bebop.bin | open failed (source, output or the .bin to run; since T129 also the compiler's output or `.use` temp) | seed.S, cli_compile |
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
| 85 | store.bp | `st_seal` refused a length that cannot be an object length (> 2^28 cells): the header cell has been overwritten. Without it `st_crc` simply walks the bogus length -- measured 2026-09-12, a clobbered PartTab header gave 1329743170 and the crc walked 10.6 GB out of a 64 KB mapping, so the process died with trap 82 in a different function from the one with the bug | st_seal |
| 86 | store.bp | `st_open` refused a store whose live superblock claims more cells than were mapped for it. st_open ftruncates to `size`, so opening an existing store at the WRONG size silently cuts the arena off mid-object and every later read traps 82 far from the cause. Checked BEFORE st_reopen_verify, which would otherwise die inside the object walk. Nine gb gates were red for this class on 2026-09-12 | st_open |
| 87 | store.bp | `st_open`/`st_map_ro` refused a store whose version cell does not match. Cell 1 has been written by every writer since the format existed and read by NOTHING, so a schema divergence was silent: B5 step 1 changed the layout under nine separate readers and each discovered it separately over four hours, always via `trap 82` several functions from the cause | st_open, st_map_ro |
| 104 | bebop.bin | too many fns (cap 512): compile_program_offs guarding fnames/fpos/sizes/starts arrays (A13 second half; was exit 89) | bebop.bp |
| 128+n | kernel | signal n: 11 = SIGSEGV (unchecked index, deep recursion), 7 = SIGBUS (misaligned sp — a compiler bug) | — |

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
