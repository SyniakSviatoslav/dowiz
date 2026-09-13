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
one `#guard` that was false. The conformance harness now RUNS and reports
**0 of 5 samples PASS** -- every sample returns `ok 0` because the evaluator has
no tail-expression rule (`Stmt.exprStmt` discards its value; `evalProgram`
answers 0 unless a `ret` fires). The gate numbers are therefore still 0.

**Runs ON-BOX:** one `lean` process per module, 4-9 s and ~0.5 GB each.
The "cannot host Lean under the 3GB/32-process caps" claim that used to stand
here was never measured and is refuted (docs/blueprints/F3-lean-semantics.md §3).

## Files

| File | Elaborates (lean 4.33.1, 2026-09-13) | Purpose |
|------|------|---------|
| `Bebop/Basic.lean` | rc=0 | Core types: Val, Expr, Stmt, Program, State (+ State.lookup/bind/zeros/arenaRead/arenaWrite), TrapCode, Result |
| `Bebop/Builtins.lean` | rc=0 | 10 executable builtins + dispatch (imports Basic only) |
| `Bebop/Syscalls.lean` | rc=0 | 26 `axiom` sys_* specs, 5 footprints, `dispatchSyscall` placeholder returning `(s, 0)` (imports Basic only) |
| `Bebop/Semantics.lean` | rc=0 | Fuel-bounded evaluator (4 `partial def` in one `mutual` block) |
| `Bebop/Traps.lean` | rc=0 | 24-row trap table, `#guard` x3 |
| `Bebop/Conformance.lean` | rc=0 | 80 + 14 EXPECT rows, 97 oracle entries (the `oracleCount` constant says 121), 5 inline-AST samples |
| `Bebop/Theorems.lean` | rc=0 | 7 `axiom` statements + 5 `#guard` sample checks; 0 `theorem` |
| `Bebop.lean` | rc=0 | Root module importing all |
| `harness.lean` | rc=0 (`lean --run`) | Prints the 5 sample verdicts: 0/5 PASS today |
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
/root/s30/outC4/lean/bin/lean --run harness.lean      # prints the 5 sample verdicts
```

`lake build` (Lake 5.0.0 from the same toolchain) also works on-box: measured
2026-09-13, rc=0, "Build completed successfully (10 jobs)", 79 s wall, no
process left behind. Lake 5 has no jobs flag, but the import DAG is a chain
with one fork (Builtins || Syscalls), so at most two `lean` processes run at
once. `.lake/` is the build directory and is not part of the source.

## Known content defects (elaborate, but wrong or vacuous) -- measured 2026-09-13

- No tail-expression rule: `execStmt (.exprStmt e)` drops the value and
  `evalProgram` returns `.ok 0` unless a `ret` signal fires, so all 5 harness
  samples print `ok 0`. This is the first thing to fix before any gate number
  can be non-zero.
- `dispatchSyscall` returns `some (s, 0)` for every `sys_*` name (Syscalls.lean,
  section 7): a silent oracle, the defect A23 removed from bpref.
- `builtinCrc32`/`builtinCrc32x` table generation is not the reflected CRC32
  algorithm and the final XOR is missing; they will not agree with zlib.
- `builtinChar` returns 0 for any index > 0; `builtinScan` never advances
  (no byte arena is modelled).
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
- The certificate checker (F5 in this file's numbering; row F6 in ROADMAP.md at `97895d9` -- the drift is explained in docs/blueprints/F7-dependent-types.md §1) consumes QF_BV obligations discharged here.

## Trust Boundary

This Lean semantics is NOT in the trust root for the final system.
The trust root is:
1. `seed/build/seed` (1,480 bytes, assembled from `seed/seed.S`, whose source is 4,370 bytes; re-derived 2026-09-13): loads and jumps
2. `bebop.bin` (179,888 bytes at `97895d9` by `wc -c`; re-derived 2026-09-13, was 171,320): the compiler
3. `tcheck.bp` (future): the certificate checker

The Lean semantics is a PRODUCER: a wrong semantics yields a rejected
conformance test, never an accepted false one. This is the de Bruijn
criterion applied to the verification infrastructure.
