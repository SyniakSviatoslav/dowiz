# F4: Bebop Formal Verification Infrastructure

## Overview

This directory contains a Lean 4 definitional semantics for the Bebop language,
part of the F4 gate in the Phase F verification ladder.

**Gate:** `lean_conformance: 86/86`, `builtin_spec: 36/36`

**Status:** SCAFFOLD -- architecture complete, core types and interpreter implemented,
builtins/syscalls/traps/conformance as stubs with `sorry` for unimplemented parts.

**Runs OFF-BOX:** Lean 4 cannot run under the box's 3GB/32-process caps.
The Lean run emits a results file bound by sha256 to the `.lean` sources;
the chain-side Python step recomputes the comparison.

## Files

| File | Lines | Status | Purpose |
|------|-------|--------|---------|
| `Bebop/Basic.lean` | ~200 | Compiles | Core types: Val, Expr, Stmt, Program, State, TrapCode, Result |
| `Bebop/Semantics.lean` | ~230 | Compiles | Fuel-bounded definitional interpreter (evalExpr, execStmt, evalProgram) |
| `Bebop/Builtins.lean` | ~170 | 6 sorry | 10 executable builtins (zeros, clz, clock_ms fully defined; rest stubs) |
| `Bebop/Syscalls.lean` | ~150 | All sorry | 26 axiomatised sys_* with declared footprints |
| `Bebop/Traps.lean` | ~180 | Compiles | 24-row trap table, static rejection checker, gate predicates |
| `Bebop/Conformance.lean` | ~200 | 2 sorry | 86-construct harness + 121 oracle interface |
| `Bebop.lean` | ~12 | Compiles | Root module importing all |
| `lakefile.lean` | ~7 | Compiles | Lake project config |
| `lean-toolchain` | 1 | -- | Pinned Lean version (v4.12.0) |

**Total Lean code:** ~1,150 lines
**Compiles without sorry:** Basic.lean, Semantics.lean, Traps.lean, Bepop.lean, lakefile.lean
**Contains sorry:** Builtins.lean (6), Syscalls.lean (26 axioms), Conformance.lean (2)

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
# Requires Lean 4 (runs off-box)
cd formal
lake build
```

## What Compiles vs What Has sorry

### Compiles (no sorry)
- `Basic.lean`: All types, enums, structures
- `Semantics.lean`: evalExpr, execStmt, execStmts, evalProgram (fuel-bounded)
- `Traps.lean`: 24-row trap table, gate predicates, static rejection types
- `Bebop.lean`: Root import

### Contains sorry (stubs)
- `Builtins.lean`: `builtinCrc32`, `builtinCrc32x`, `builtinHvham`,
  `builtinHvham2`, `builtinScan` (5 sorry in implementations)
- `Syscalls.lean`: 26 axiom declarations + `dispatchSyscall` stub
- `Conformance.lean`: `runTestCase` (parser not yet implemented),
  `checkResult` (needs full Result matching)

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
