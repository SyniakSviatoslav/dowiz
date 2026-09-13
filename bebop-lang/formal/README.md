# F4: Bebop Formal Verification Infrastructure

## Overview

This directory contains a Lean 4 definitional semantics for the Bebop language,
part of the F4 gate in the Phase F verification ladder.

**Gate:** `lean_conformance: 86/86`, `builtin_spec: 36/36`

**Status (measured 2026-09-13):** all 8 modules ELABORATE on-box under Lean 4.33.1
with the plain `lean` binary (see Building). Before that date nothing under
`formal/` had ever elaborated: a nested-comment token in Basic.lean, two import
cycles (Builtins <-> Semantics <-> Syscalls), a toolchain pin naming a Lean that
is not on the box, ~60 API/keyword/typing errors from code that was never run, and
one `#guard` that was false. The conformance harness RUNS and, since the
tail-expression rule landed later the same day, reports **5 of 5 samples PASS**
with real values (`ok 1000000065571`, `ok 34`, `ok 45`, `ok 6`, `ok 119`), each
sample re-encoded from its `bench/parity_constructs/*.bp` rather than from the
construct's derivation comment. Before the rule every sample printed `ok 0`
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
| `Bebop/Syscalls.lean` | rc=0 | 26 `axiom` sys_* specs, 5 footprints, `dispatchSyscall` placeholder returning `(s, 0)` (imports Basic only) |
| `Bebop/Semantics.lean` | rc=0 | Fuel-bounded evaluator (5 `partial def` in one `mutual` block; `execBody` is the tail-expression rule) |
| `Bebop/Traps.lean` | rc=0 | 24-row trap table, `#guard` x3 |
| `Bebop/Conformance.lean` | rc=0 | 80 + 14 EXPECT rows, 97 oracle entries (the `oracleCount` constant says 121), 5 inline-AST samples transcribed from the `.bp` files |
| `Bebop/Theorems.lean` | rc=0 | 7 `axiom` statements + 5 `#guard` sample checks; 0 `theorem` |
| `Bebop.lean` | rc=0 | Root module importing all |
| `harness.lean` | rc=0 (`lean --run`) | Prints the 5 sample verdicts: 5/5 PASS (2026-09-13, after the tail rule); 4.6 s |
| `lakefile.lean` | not exercised | Lake project config |
| `lean-toolchain` | -- | `leanprover/lean4:v4.33.1` (the Lean on the box; v4.12.0 predates `Int64`) |

Counts by `grep -c` on 2026-09-13: `sorry` 0 in code (the word occurs in two
comments), `axiom` 26 (Syscalls) + 7 (Theorems), `theorem` 0, `partial def` 4,
`#guard` 3 (Traps) + 5 (Theorems). The old "Compiles" / "6 sorry" table was
written without a build and was wrong in both directions.

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

`lake build` (Lake 5.0.0 from the same toolchain) also works on-box: measured
2026-09-13, rc=0, "Build completed successfully (10 jobs)", 79 s wall, no
process left behind. Lake 5 has no jobs flag, but the import DAG is a chain
with one fork (Builtins || Syscalls), so at most two `lean` processes run at
once. `.lake/` is the build directory and is not part of the source.

## Known content defects (elaborate, but wrong or vacuous) -- measured 2026-09-13

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
- STILL FAKE -- fuel exhaustion is invisible: `execStmts`/`loop` answer `.cont`
  at fuel 0, so `while 1 { 0 }; 0` evaluates to `ok 0` (probed with fuel 1000)
  where `bebop.bin` never terminates. A non-terminating program can "pass".
- STILL WRONG -- the caller's environment is lost after a call: the `.call` arm
  returns the callee's state, so `let x = 5; let y = f(1); x + y` is `stuck`
  (probed; was `ok 0` before `stuck` existed). Any construct that reads a local
  after a call will disagree with `bebop.bin`. One-line fix in the `.call` arm
  (`env := s1.env` on the way out), not applied: outside the tail-rule row.
- STILL FAKE -- `checkExpected` accepts ANY `.trap` or ANY `.rejected` when the
  expectation is `none`; a negative construct expecting exit 97 passes on exit 100.
- `dispatchSyscall` returns `some (s, 0)` for every `sys_*` name, including
  names that are not syscalls (probed: `sys_this_does_not_exist [] = some 0`,
  `sys_exit [7] = some 0`): a silent oracle, the defect A23 removed from bpref.
  It is consulted BEFORE user functions, so a user `fn sys_x` is shadowed too.
- `builtinCrc32`/`builtinCrc32x`: the table builder shifts LEFT, tests bit 7 and
  masks to 8 bits before XORing a 32-bit polynomial, and the final XOR is
  missing. Probed: crc32("123456789") = 15579374; zlib gives 3421780262
  (0xCBF43926). Visibly wrong, so it cannot fake a pass.
- `builtinChar` returns the handle's low byte for index 0 and 0 after (probed:
  3 0 0 for a length-3 handle where bpref reads 97 98 99); `builtinScan` never
  advances (probed: 0 where bpref gives 3). No byte arena is modelled.
- `Conformance.lean` header says 75 + 11 constructs / 121 oracles / 36 builtins;
  the tables hold 80 + 14 rows and 97 oracle entries; `oracleCount` (121) and
  `builtinCount` (36) are typed constants, not sizes.
  The tree has 100 + 20 constructs and 38 builtins (F3 blueprint §2.5).
- `Theorems.lean` proves nothing: 7 `axiom`s checked by `#guard` on sample values.

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
