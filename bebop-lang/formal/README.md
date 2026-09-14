# F4: Bebop Formal Verification Infrastructure

## Overview

This directory contains a Lean 4 definitional semantics for the Bebop language,
part of the F4 gate in the Phase F verification ladder.

**Gate:** `lean_conformance: 86/86`, `builtin_spec: 36/36` (ROADMAP F4 wording).
The `86` denominator is STALE. Counted 2026-09-14:
`ls bench/parity_constructs/*.bp | wc -l` = **101** and
`ls bench/parity_constructs/neg/*.bp | wc -l` = **20**, i.e. a **121**-construct
suite, all 121 carrying a `// EXPECT <v> from:` header. Measure against 121.

**Where this stands, 2026-09-14 (lane l4lean), all figures measured:**

| quantity | value | how |
|---|---|---|
| `lake build` | **rc=0**, clean, 81 s, 10 jobs | `rm -rf .lake/build && lake build` |
| modules elaborating standalone | 9 of 9 (incl. `harness.lean`) | `lean <file>` per file, all rc=0 |
| import cycles | **none** | the graph is a DAG; the 2026-09-13 "still open" claim was already false at 7edffdf |
| constructs actually RUN | **10 of 121** | 10 `#guard`ed inline-AST samples; there is no `.bp` parser |
| constructs declared as data | 94 of 121 (80 pos + 14 neg rows) | 21 positive + 6 negative names missing |
| builtins executable | **10 of 37** | `zeros str_len char clock_ms clz crc32 crc32x hvham hvham2 scan` |
| builtins with an axiom spec only | 26 (`dispatchSyscall` returns `none` for all) | `Syscalls.lean` |
| builtins absent entirely | 1 (`crc32b`) | grep over `formal/` |
| statement forms modelled | 8 of 8 | every `Stmt` has an `execStmt` arm |
| expression forms modelled | 28 of 28 | 19 operators + 7 primaries + 2 postfix |
| type checking | **0** | `Ty` is declared and never read; `grep -n 'Ty\.' Semantics.lean` is empty |
| F9 theorems | **6 proved, 1 axiom, 0 sorry** | `#print axioms` on each, in the build log |

**Status (measured 2026-09-13):** all 8 modules ELABORATE on-box under Lean 4.33.1
with the plain `lean` binary (see Building). Before that date nothing under
`formal/` had ever elaborated: a nested-comment token in Basic.lean, two import
cycles (Builtins <-> Semantics <-> Syscalls), a toolchain pin naming a Lean that
is not on the box, ~60 API/keyword/typing errors from code that was never run, and
one `#guard` that was false. The conformance harness RUNS and, since the
tail-expression rule landed later the same day, reports **5 of 5 samples PASS**
with real values (`ok 1000000065571`, `ok 34`, `ok 45`, `ok 6`, `ok 119`), each
sample re-encoded from its `bench/parity_constructs/*.bp` rather than from the
construct's derivation comment. Later the same day the three fakes that could
make a wrong evaluator look right (syscalls answering 0, silent fuel exhaustion,
caller env lost after a call) were closed and pinned by 5 probe fixtures
(`lean_probes: 5/5`); the 5/5 did not move, because none of the five samples
touched any of the three. Before the rule every sample printed `ok 0`
(`Stmt.exprStmt` discarded its value; `evalProgram` answered 0 unless a `ret`
fired), so 0/5 -> 5/5 is one rule plus one rebind fix, not five. The gate
numbers are still 0: nothing in `tools/` runs this, and 5 hand-built ASTs are
not 100 constructs (see "Known content defects").

**Runs ON-BOX:** one `lean` process per module, 4-9 s and ~0.5 GB each.
The "cannot host Lean under the 3GB/32-process caps" claim that used to stand
here was never measured and is refuted (docs/blueprints/F3-lean-semantics.md §3).

## Files

| File | Elaborates (lean 4.33.1, 2026-09-13) | Purpose |
|------|------|---------|
| `Bebop/Basic.lean` | rc=0 | Core types: Val, Expr, Stmt, Program, State (+ State.lookup/bind/zeros/arenaRead/arenaWrite), TrapCode, Result (with `stuck` for a body that yields no value) |
| `Bebop/Builtins.lean` | rc=0 | 10 executable builtins + dispatch (imports Basic only) |
| `Bebop/Syscalls.lean` | rc=0 | 26 `axiom` sys_* specs, 5 footprints, `dispatchSyscall` = `none` for every name (nothing modelled; was `some (s, 0)` for any `sys_*` until 2026-09-13) (imports Basic only) |
| `Bebop/Semantics.lean` | rc=0 | Fuel-bounded evaluator (5 `partial def` in one `mutual` block; `execBody` is the tail-expression rule; every fuel-0 arm sets `State.fuelOut` and `evalProgram` reports `Result.fuelExhausted`; the `.call` arm restores the caller's env) |
| `Bebop/Traps.lean` | rc=0 | 24-row trap table, `#guard` x3 |
| `Bebop/Conformance.lean` | rc=0 | 80 + 14 EXPECT rows, 97 program entries now carrying the **verbatim** `.bp` source (88 of 97 were invented programs until 2026-09-14), **10** inline-AST samples each with a `#guard`, 5 probe fixtures (§5b) pinning the three fakes closed 2026-09-13. `oracleCount` is now `oracleEntries.size`, not a typed 121 |
| `Bebop/Theorems.lean` | rc=0 | **6 `theorem` + 1 `axiom`**, 0 `sorry`; `#print axioms` on all 6 in the build log. Two of the old 7 axioms were FALSE and are now theorems with the hypotheses that make them true |
| `Bebop.lean` | rc=0 | Root module importing all |
| `harness.lean` | rc=0 (`lean --run`) | Prints the 5 sample verdicts and the 5 probe verdicts as `lean_conformance: 5/5` and `lean_probes: 5/5`; exits 1 if either is short |
| `lakefile.lean` | not exercised | Lake project config |
| `lean-toolchain` | -- | `leanprover/lean4:v4.33.1` (the Lean on the box; v4.12.0 predates `Int64`) |

Counts by `grep -c` on 2026-09-14: `sorry` 0 in code, `axiom` 26 (Syscalls) +
**1** (Theorems), `theorem` **9** (6 F9 statements + `st_len_masks_digest` +
2 bridging lemmas), `partial def` 4, `#guard` 3 (Traps) + 5 + 4 (Theorems) +
**10** (Conformance samples). `grep -c sorryAx` over the whole build log: 0.

## Architecture

```
                    Bebop.lean (root)
                         |
         +---------------+---------------+
         |               |               |
    Basic.lean    Semantics.lean    Conformance.lean
   (types, AST)   (interpreter)    (86 tests + 121 oracles)
                         |
         +-------+-------+-------+
         |       |       |       |
    Builtins  Syscalls  Traps
   (10 exec)  (26 ax)  (24 rows)
```

### Design Decisions

1. **Fuel-bounded termination:** Every recursive call in the interpreter
   decrements a fuel counter. Zero fuel = undefined result. This guarantees
   termination in Lean's totality checker without well-founded recursion.

2. **One-cell-array memory model:** Arrays are offsets into a flat arena
   (A5 step 1b). The frame heap is a separate array for array literals.

3. **Function-scoped bindings:** `let` REBINDS a fn-scoped register.
   No block scoping, no shadowing (LANGUAGE.md:41-43).

4. **Wrapping arithmetic:** All i64 operations wrap on overflow.
   `x/0 = 0`, `x%0 = x` (LANGUAGE.md:66).

5. **Control flow signals:** break/return are modelled as Signal values
   propagated through statement execution.

## Building

```bash
# On-box. Lean 4.33.1 at /root/s30/outC4/lean/bin (the only Lean here).
cd formal
export LEAN_PATH=$PWD/.lake/build/lib/lean
mkdir -p .lake/build/lib/lean/Bebop
for m in Basic Builtins Syscalls Semantics Traps Conformance Theorems; do
  /root/s30/outC4/lean/bin/lean -o .lake/build/lib/lean/Bebop/$m.olean Bebop/$m.lean || break
done
/root/s30/outC4/lean/bin/lean -o .lake/build/lib/lean/Bebop.olean Bebop.lean
LEAN_PATH=$PWD/.lake/build/lib/lean /root/s30/outC4/lean/bin/lean --run harness.lean
# prints the 5 sample verdicts (LEAN_PATH is needed: `lean --run` does not read lakefile.lean)
```

Both `lake build` and `lean --run` are heavy jobs (lake ~1 GB, lean ~0.25-0.5 GB
per process) and must go through the box's slot, or three lanes end up computing
at once and the Android phantom-process killer picks a victim (it happened
2026-09-13 at 37 procs):

```bash
cd formal
PERF=0 ../tools/slot.sh lean-build /root/s30/outC4/lean/bin/lake build
PERF=0 ../tools/slot.sh lean-harness env LEAN_PATH=$PWD/.lake/build/lib/lean \
  /root/s30/outC4/lean/bin/lean --run harness.lean
```

`lake build` (Lake 5.0.0 from the same toolchain) also works on-box: measured
2026-09-13, rc=0, "Build completed successfully (10 jobs)", 79 s wall, no
process left behind. Lake 5 has no jobs flag, but the import DAG is a chain
with one fork (Builtins || Syscalls), so at most two `lean` processes run at
once. `.lake/` is the build directory and is not part of the source.

## Known content defects (elaborate, but wrong or vacuous) -- measured 2026-09-13, updated 2026-09-14

- FIXED 2026-09-13 -- tail-expression rule. `Semantics.lean` `execBody` runs a
  function body as statements + ONE tail expression (LANGUAGE.md:27-28; mirrors
  `tools/bpref.py` `run_body`); it is the only place a statement sequence yields
  a value, and both consumers of a body (the `.call` arm and `evalProgram`) use
  it. `execStmts` is unchanged, so a `while` body's tail is still discarded and a
  non-final `exprStmt` is still dropped (both probed). A body that yields no
  value is `Result.stuck`, never `ok 0`.
- FIXED 2026-09-13 -- rebind. `State.bind` pushed and `State.lookup` found the
  FIRST entry, so `let x = x + 1` and `x += 1` never took effect; c07_while looped
  on `i = 0` until the fuel ran out and printed `ok 0`. `bind` now updates in place.
- FIXED 2026-09-13 -- three of the five samples encoded programs other than
  their `.bp` (c01 `1000000000000+65536+36+1`, c07 `s += i` to 10, c08
  `add(3,4)`); they gave 1000000065573 / 0 / 7 once the tail rule made values
  visible. Re-transcribed from the `.bp` files. The `oracleEntries` strings for
  c01/c02 still carry the old wrong programs (data only; the blueprint deletes
  that table).
- FIXED 2026-09-13 -- `dispatchSyscall` answered `some (s, 0)` for every name
  beginning `sys_`, real or invented (probed: `sys_this_does_not_exist [] =
  some 0`, `sys_exit [7] = some 0`), and was consulted BEFORE user functions,
  so `fn sys_x() { 42 }` ran as `ok 0` (bpref: 42). It is now `none` for every
  name: a syscall call is `stuck`, never a value, until that syscall is modelled
  (F3 blueprint §4.4 names `sys_write`/`sys_exit` as the two to model first).
  None of the 5 samples calls a `sys_*`, so the count did not move; 8 of the
  100 positive constructs do and will be `stuck` until modelled. Probes
  p01a/p01b/p01c. NOTE: `tools/bpref.py:748` still has its own
  `startswith('sys_') -> 0` fallback (run 2026-09-13: `sys_this_does_not_exist()`
  prints 0); the blueprint's "A23 removed it" is not true on this tree.
- FIXED 2026-09-13 -- fuel exhaustion was invisible: every fuel-0 arm answered
  `.cont`/`none` and the run carried on, so `while 1 { 0 }; 0` was `ok 0` at
  fuel 1000 where `bebop.bin` never terminates (bpref: 5 s timeout, rc=124).
  Now every fuel-0 arm sets the sticky `State.fuelOut` and `evalProgram`
  reports `Result.fuelExhausted fuel` whatever value came out afterwards --
  the tools/kcheck.py `whnf` rule ("a checker that timed out into 'yes' would
  be unsound"). It is not a `.trap`, so the lax trap arm below cannot accept
  it. Probe p02.
- FIXED 2026-09-13 -- the caller's environment was lost after a call: the
  `.call` arm returned the callee's state, so `let x = 5; let y = f(1); x + y`
  was `stuck` (`ok 0` before `stuck` existed; bpref: 7). The arm now restores
  `env := s1.env` on the way out (arena and `fuelOut` stay the callee's), as
  bpref's `call` does by building a fresh env dict. Probe p03.
- STILL FAKE -- `checkExpected` accepts ANY `.trap` or ANY `.rejected` when the
  expectation is `none`; a negative construct expecting exit 97 passes on exit 100.
  (The two arms added for the probes, `stuck`/`fuel exhausted`, match the verdict
  string exactly and widen nothing.)
- `builtinCrc32`/`builtinCrc32x`: the table builder shifts LEFT, tests bit 7 and
  masks to 8 bits before XORing a 32-bit polynomial, and the final XOR is
  missing. Probed: crc32("123456789") = 15579374; zlib gives 3421780262
  (0xCBF43926). Visibly wrong, so it cannot fake a pass.
- `builtinChar` returns the handle's low byte for index 0 and 0 after (probed:
  3 0 0 for a length-3 handle where bpref reads 97 98 99); `builtinScan` never
  advances (probed: 0 where bpref gives 3). No byte arena is modelled.
- FIXED 2026-09-14 -- `Conformance.lean`'s counts. The header said 75 + 11
  constructs / 121 oracles; the tables held 80 + 14 rows and 97 entries.
  `oracleCount` is now `oracleEntries.size` (derived), the 121 lives in
  `oracleProgramTarget` where it cannot be mistaken for a measurement, and
  `suiteConstructCount = 121` / `executedConstructCount = 10` record what is on
  disk versus what actually runs. `builtinCount` (36) is still a typed constant;
  LANGUAGE.md's table lists 37 call names and one of them (`crc32b`) is modelled
  nowhere, which is where the 36-vs-37-vs-38 disagreement comes from.
- FIXED 2026-09-14 -- **88 of 97 `oracleEntries.source` strings were invented
  programs**, not the files they name. Only 8 of the 96 that have a real `.bp`
  matched it (whitespace/comment-normalised). Some were not even in the
  language: `c05_if` used brace `if` arms where Bebop has `if c then a else b`.
  Others disagreed with their own recorded value: `c08_call` was `add(3,4)`
  against 6, `c14_string` `str_len("hello")` against 8, `c02_arith` `a + b * 2`
  = 16 against 34 (the same mis-transcription already fixed in `sample_c02` and
  left standing here). The VALUES were all right -- 76 of 76 agreed with the
  real `.bp`'s `// EXPECT` -- so every source was replaced, by script, with the
  byte-for-byte file content.
- FIXED 2026-09-14 -- **`match` bound its payload binder to the arena OFFSET,
  not the payload value.** `c12_match` (`match some(5) { none => 0,
  some(x) => x + 1 }`, EXPECT 6) evaluated to `ok 1`. Only payload-binding arms
  were affected, so the nullary `c11_enum` looked fine. Fixed by `arenaRead`;
  `c11 c12 c22 c25 c96` are now `#guard`ed samples and `c96_enumpay`, the
  runtime match on a VARIABLE, gives `ok 8503009`.
- FIXED 2026-09-14 -- **`Theorems.lean`'s axiom set was INCONSISTENT.**
  `isqrt_correct` asserted `r*r <= s` with no sign hypothesis (false at s = -1)
  and `s < (r+1)^2` over wrapping Int64 (false at s = i64::MAX, where
  `(r+1)*(r+1)` wraps to -9223372036709301616); `cursor_monotone` asserted
  `old + 2 + len >= old` with no hypothesis on `len` (false at len = -10, and
  at old = i64::MAX). A false axiom proves anything, so nothing in the file or
  in any importer was safe. Both are now theorems over `Int64.toInt` with the
  needed hypotheses, and their counterexamples are kept as `#guard`s.
- STILL OPEN -- `fp_mul_correct` is the one remaining `axiom`. 64x64
  multiplication is out of `bv_decide`'s reach and the limb argument is
  unwritten; `sorry` was deliberately NOT used. A 1024-pair LCG differential
  (`fp_mul_sweep`) agrees, which refutes nothing and proves nothing.
- STILL OPEN -- `addov_correct` and `st_len_masks_digest` are proved by
  `bv_decide`, which in Lean v4.33.1 checks its LRAT certificate by COMPILED
  evaluation (`nativeEqTrue`) and records a generated axiom
  (`..._native.bv_decide.ax_*`). They are machine-checked with the Lean
  compiler and cadical in the trust root, NOT kernel-checked. The F9 gate asks
  for both; only the certificate half is there.
- STILL OPEN -- **no type checking at all.** `Ty` is declared in `Basic.lean`
  and never read by the evaluator (`grep -n 'Ty\.' Bebop/Semantics.lean` is
  empty). Every F1 static rejection that is a type error is therefore
  unmodelled, and the 6 missing negative rows include the ones that need it.
- STILL OPEN -- `.lor` / `.land` (`||` / `&&`) are modelled as plain bitwise
  `|||` / `&&&`, i.e. the pre-A26 bpref language. LANGUAGE.md's precedence
  table does not list `&&` or `||` at all, and the compiler's `&&` is reported
  to be a constant zero, so this rule matches neither document. It belongs to
  whoever owns `bebop.bp`, but F4 cannot be "the WHOLE language" until the
  three agree.
- STILL OPEN -- 21 positive and 6 negative construct names have no row:
  positive `c124_condreturn c133_testblock(+_twin) c134_testfn_inside(+_twin)
  c135_testblock_braces(+_twin) c142_clone8 c144_contract_ok c145_fraction_ok
  c146_module_empty_ok c146_module_empty_nospace_ok c70_qdsl c70_qdsl_neg
  c97_arena_operand_miscompile c98_arena_base_operand c99_arena_end_operand
  c_gb_degree c_gb_kernel_accessors c_gb_vec_setbit c_st_bytes`;
  negative `c140_mapb_refused c141_clone9 c143_contract_garbage
  c145_fraction_garbage c146_module_contents_garbage read_before_assign`.

## Relationship to Other Phases

- **F4 is PARALLEL to F1/F2** -- it does NOT depend on them.
- F0-F2 produce 20 rejected bug classes in 5-9 weeks.
- F4 (this) produces the normative semantics for the whole language.
- F3's differential testing against bpref is the runtime gate.
- F4's Lean semantics is the compile-time/definitional gate.
- The certificate checker (F5) consumes QF_BV obligations discharged here.

## Trust Boundary

This Lean semantics is NOT in the trust root for the final system.
The trust root is:
1. `seed/seed.S` (1,480 bytes): loads and jumps
2. `bebop.bin` (171,320 bytes): the compiler
3. `tcheck.bp` (future): the certificate checker

The Lean semantics is a PRODUCER: a wrong semantics yields a rejected
conformance test, never an accepted false one. This is the de Bruijn
criterion applied to the verification infrastructure.
