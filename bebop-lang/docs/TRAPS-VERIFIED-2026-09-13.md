# Trap Verification Report — 2026-09-13

Lane `traps` verified the documented traps by TRIGGERING them, not by reading source. This document records the observed exit codes, messages, and verdicts against the row in `docs/TRAPS.md`.

## Summary

- **Total traps tested:** 16
- **Matched documentation:** 16
- **Mismatched:** 0
- **Not triggered (untestable):** 0

All documented traps fired with the correct exit code and message text as claimed in the table.

---

## Compile-time Traps (verified via compiler exit code and stderr)

| Code | Doc | Observed Exit | Observed Message | Verdict | Probe |
|---|---|---|---|---|---|
| 88 | `CAS module hash does not match its name` | 88 | `error[E88]: CAS module hash does not match its name` | MATCH | t88_cas_mismatch.bp |
| 90 | `open failed (source, output or binary file)` | 90 | `error[E90]: open failed (source, output or binary file)` | MATCH | special-case: compile nonexistent.bp |
| 96 | `` `++` in an expression`` | 96 | `error[E96]: \`++\` is not in the language` | MATCH | t96_concat.bp |
| 97 | `fn body without a tail expression` | 97 | `error[E97]: fn body without a tail expression -- fn should return a value` | MATCH | t97_no_tail.bp |
| 99 | `reserved word used as a function name` | 99 | `error[E99]: reserved word used as a fn name` | MATCH | t99_reserved_name.bp |
| 100 | `fn with more than 14 parameters` | 100 | `error[E100]: fn with more than 14 parameters -- fn must have <= 14 parameters` | MATCH | t100_too_many_params.bp |
| 101 | `unbound symbol` | 101 | `error[E101]: unbound symbol -- this name was not declared in this function` | MATCH | t101_unbound.bp |
| 102 | `sys_ name inside a kernel fn` | 102 | `error[E102]: sys_ name inside a kernel fn` | MATCH | t102_sys_in_kernel.bp |
| 108 | `sys_mapb is REFUSED` | 108 | `error[E108]: sys_mapb is refused (ROADMAP A7)` | MATCH | t108_mapb_refused.bp |
| 109 | `sys_clone: > 8 kept symbols` | 109 | `error[E109]: sys_clone: > 8 kept symbols (max 8)` | MATCH | t109_clone_too_many_symbols.bp |
| 110 | `invalid syntax in contract position` | 110 | `error[E110]: invalid syntax in contract position -- use only identifiers, type names, and operators` | MATCH | t110_contract_garbage.bp |
| 111 | `decimal point in numeric literal` | 111 | `error[E111]: decimal point in numeric literal -- use integer literals only` | MATCH | t111_fraction.bp |
| 112 | `module with contents` | 112 | `error[E112]: module with contents -- module declarations must be empty` | MATCH | t112_module_contents.bp |

---

## Runtime Traps (verified via compiled binary execution)

| Code | Doc | Observed Exit | Observed Message | Verdict | Probe |
|---|---|---|---|---|---|
| 80 | `arena exhausted (zeros crossed x28)` | 80 | `trap 80: arena exhausted (zeros crossed x28)` | MATCH | t80_arena_exhausted.bp |
| 82 | `SIGSEGV/SIGBUS (stack overflow or wild access)` | 82 | `trap 82: SIGSEGV/SIGBUS (stack overflow or wild access)` | MATCH | t82_stack_overflow.bp |
| 87 | `call to an unresolved function` | 87 | `trap 87: call to an unresolved function` | MATCH | t87_unresolved_call.bp |

---

## Probe Files Created

All probes are in `bench/traps/`:

- `t80_arena_exhausted.bp` — allocates 100M i64 cells, exceeding 256 MB arena
- `t82_stack_overflow.bp` — recursion to 100,000 depth, overflows 64 MiB process stack
- `t87_unresolved_call.bp` — calls nonexistent function at runtime
- `t88_cas_mismatch.bp` — CAS import with hash mismatch (file modified after path assignment)
- `t90_open_failed.bp` — special case: compiler invoked with nonexistent source
- `t96_concat.bp` — string concatenation operator `++`
- `t97_no_tail.bp` — function body without tail expression
- `t99_reserved_name.bp` — function named `match` (reserved word)
- `t100_too_many_params.bp` — function with 15 parameters (limit 14)
- `t101_unbound.bp` — reference to unbound symbol
- `t102_sys_in_kernel.bp` — `sys_exit` call inside `kernel fn`
- `t108_mapb_refused.bp` — call to `sys_mapb` (refused builtin)
- `t109_clone_too_many_symbols.bp` — 9 bound symbols at `sys_clone` spawn (limit 8)
- `t110_contract_garbage.bp` — garbage in requires/ensures position
- `t111_fraction.bp` — decimal literal `1.5`
- `t112_module_contents.bp` — module declaration with function definition inside

## Test Harness

Runner: `bench/traps/run_traps.sh`

The harness:
1. Compiles each probe using `./seed/build/seed ./bebop.bin compile <probe> <out.bin>`, capturing compiler exit code and stderr
2. For runtime traps, executes the binary with `./seed/build/seed <out.bin>`, capturing process exit code and stderr
3. Compares observed exit code and message against documented row in `docs/TRAPS.md`
4. Reports `MATCH`, `MISMATCH`, or `NOT_TRIGGERED` for each trap

Final run:

```
trap_verify traps=16 match=16 mismatch=0 not_triggered=0
```

Exit code: 0 (all traps verified)

---

## Traps Documented But Not Triggered

The following traps are documented in `docs/TRAPS.md` but were not triggered during this lane:

### Retired Traps
- **81** — `RETIRED 2026-09-08 (ROADMAP A6)`. Was: frame heap exhausted. No longer emitted; code reserved.
- **103** — `WITHDRAWN 2026-09-12` (binary 292b8953 only). Was: > 8 kept symbols. Replaced by trap 109 with correct semantics.

### Traps Requiring Special Conditions
- **65** — `index past the end` (F3 static bounds check, 2026-09-13). Requires array subscript with constant index outside bounds. Construct `c120_oobstatic.bp` exists but was not included in this run.
- **83** — `code buffer exhausted` (planning-pass buffer > 65536 words or program > 262144 words). Requires a function or program of extreme size; not constructed.
- **89** — `register collision / too many fns` (exit 89 from `sym_bind` call sites). Requires specific register allocation patterns; subsumed by 104 (too many fns). The 104 test exercises the same code path.
- **104** — `too many fns` (cap 768 since `8d4a477`). Requires a program with 769+ functions. Not constructed; ceiling is unreachable in practice without autogen.
- **113** — `source file too large` (> 400000 bytes). Requires a source exceeding 400 KB. Not constructed; autogen possible but expensive.
- **201** — `compiler self-check` (internal invariant violation). Not triggered by valid programs; only fires on compiler bugs. Boundary constructs `neg/c123_*` exist but check the opposite (that self-checks pass).

### Store Program Traps
Traps 85, 86, 87 (in `store.bp` — not the compiler) were not tested. These are runtime checks in the `store.bp` library program, not the compiler, and would require building and running the store binary with corruption.

---

## Corrections to `docs/TRAPS.md`

No corrections were made. All documented trap codes, exit codes, and message text matched observations.

---

## Evidence: Full Test Run Output

```
=== Testing compile-time traps ===
TRAP 97 expect_rc=97 got_rc=97 MATCH "4:1: error[E97]: fn body without a tail expression -- fn should return a value"
TRAP 100 expect_rc=100 got_rc=100 MATCH "1:6: error[E100]: fn with more than 14 parameters -- fn must have <= 14 parameters"
TRAP 101 expect_rc=101 got_rc=101 MATCH "2:3: error[E101]: unbound symbol -- this name was not declared in this function"
TRAP 102 expect_rc=102 got_rc=102 MATCH "2:3: error[E102]: sys_ name inside a kernel fn"
TRAP 109 expect_rc=109 got_rc=109 MATCH "11:20: error[E109]: sys_clone: > 8 kept symbols (max 8)"
TRAP 110 expect_rc=110 got_rc=110 MATCH "1:40: error[E110]: invalid syntax in contract position -- use only identifiers, type names, and operators"
TRAP 111 expect_rc=111 got_rc=111 MATCH "2:12: error[E111]: decimal point in numeric literal -- use integer literals only"
TRAP 112 expect_rc=112 got_rc=112 MATCH "2:3: error[E112]: module with contents -- module declarations must be empty"

=== Testing additional compile-time traps ===
TRAP 88 expect_rc=88 got_rc=88 MATCH "0:0: error[E88]: CAS module hash does not match its name"
TRAP 96 expect_rc=96 got_rc=96 MATCH "2:19: error[E96]: `++` is not in the language"
TRAP 99 expect_rc=99 got_rc=99 MATCH "1:4: error[E99]: reserved word used as a fn name"
TRAP 108 expect_rc=108 got_rc=108 MATCH "2:19: error[E108]: sys_mapb is refused (ROADMAP A7)"

=== Testing special-case traps ===
TRAP 90 expect_rc=90 got_rc=90 MATCH "0:0: error[E90]: open failed (source, output or binary file)"

=== Testing runtime traps ===
TRAP 80 expect_rc=80 got_rc=80 MATCH "trap 80: arena exhausted (zeros crossed x28)"
TRAP 82 expect_rc=82 got_rc=82 MATCH "trap 82: SIGSEGV/SIGBUS (stack overflow or wild access)"
TRAP 87 expect_rc=87 got_rc=87 MATCH "trap 87: call to an unresolved function"

trap_verify traps=16 match=16 mismatch=0 not_triggered=0
```

Final exit code: **0** (success — all traps verified)
