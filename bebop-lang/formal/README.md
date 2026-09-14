# F4: Bebop Formal Verification Infrastructure

## Overview

This directory contains a Lean 4 definitional semantics for the Bebop language,
part of the F4 gate in the Phase F verification ladder.

**Gate:** `lean_conformance: 86/86`, `builtin_spec: 36/36` (ROADMAP F4 wording).
The `86` denominator is STALE. Counted 2026-09-14:
`ls bench/parity_constructs/*.bp | wc -l` = **101** and
`ls bench/parity_constructs/neg/*.bp | wc -l` = **20**, i.e. a **121**-construct
suite, all 121 carrying a `// EXPECT <v> from:` header. Measure against 121.

**Where this stands, 2026-09-14 (lane f4lean), all figures measured:**

| quantity | value | how |
|---|---|---|
| `lake build` | **rc=0**, "Build completed successfully (14 jobs)" | `lake build` through `tools/slot.sh` |
| constructs PARSED from disk | **121 of 121** | `lake exe parityrun <repo-root>` |
| constructs matching their `// EXPECT` | **120 of 121** | same run, eval fuel 4000000 |
| the one that does not | `c84_run` | STUCK at `sys_wait4`; see "c84_run" below |
| results file | `formal/results.txt`, emitted by `parityrun` | `lean_conformance 120/121` + the source hash |
| `lean_sources_sha256` reproducible outside Lean | yes | `find formal -name '*.lean' \| LC_ALL=C sort \| xargs sha256sum \| sha256sum` |
| theorems gaining a `sorryAx` | **0** | `grep -c sorryAx` over the build log |
| builtins executable | **13 of 41** | `zeros str_len char clock_ms clz crc32 crc32b crc32x hvham hvham2 scan` + the 13 modelled `sys_*` |
| builtin surface | **41**, from the compiler | `python3 tools/builtin_surface.py` ("compiler dispatches 41") |
| static rejection rules | **7**, in `Bebop/Reject.lean` | exits 65, 99, 100, 101, 102, 108, 109 |
| runtime traps raised | **3** | 80 (arena), 82 (call depth), 87 (unresolved call) |

**THE PROGRESSION, 87 -> 120, each number from a full 121-file run** (the group
names are lane f4lean's card):

| after | passes | what closed |
|---|---|---|
| base `fbe60a6` | 87 | -- |
| group 1, negative scoring | 94 | 7 `neg/` that the harness already refused correctly and scored as failures |
| group 2, `return`/`break` as expressions | 95 | c124_condreturn (c70_qdsl x2 moved UNSUPPORTED -> STUCK) |
| group 3, `Bebop/Reject.lean` | 106 | c113 c114 c120 c85 c93 c37 + c112 c140 c141 c52 read_before_assign |
| group 4+5, byte arena + syscalls + crc/scan | 117 | c68 c70 x2 c94 c97 c98 c99 c110 + c42 c45 c78 |
| group 6+7, `cas://` + call depth | 120 | c50_cas c51_casbad c48_stackovf |

**c84_run, the one left open, and the space searched.** Its value is
`((st[0] >> 8) & 255) + 1000 * (w == pid)` = 1035, where `st[0]` is the wait
status of a child that `sys_run` executed. Three ways to produce the 35 were
considered and all three are either out of reach or dishonest:

1. *Execute the image.* `sys_run(addr, size, argc, argv)` runs the aarch64
   machine code that `sys_mmap` mapped from `frozen/c01_lit.bin`. That needs an
   instruction-set model and a loader. This is a semantics of the LANGUAGE;
   machine code is not in it.
2. *Identify the `.bin` with the `.bp` of the same stem and evaluate that.* This
   assumes the compiler is correct, which is the property F4 exists to check.
   Circular.
3. *Read the number out of `c01_lit.bp`'s own `// EXPECT` header.* Fabrication.

A second, independent gap sits on top of the first: `sys_clone` is modelled as
the PARENT's view of a fork and the child is not executed, so even with
`sys_run` the model would have no child status to report. Both gaps are named
at the exact syscall that needs them (`Bebop/Syscalls.lean`'s default arm), and
the construct reports
`STUCK unmodelled builtin `sys_wait4``, never a value.

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
| `Bebop/Lexer.lean` | rc=0 | Tokenizer; 9 `#guard`s pinning longest-match (`>>>` before `>>`, `&&` vs `&`, `=>` vs `=`) |
| `Bebop/Parser.lean` | rc=0 | Recursive-descent `.bp` -> AST, TOTAL (fuel-recursive, no `partial` on the parse path), `ParseError` separating "unsupported by this parser" from "not a legal program"; 28 `#guard` grammar pins |
| `ParityRun.lean` | rc=0 (`lake exe parityrun`) | Reads the real `.bp` files + their `// EXPECT`, expands `use` INCLUDING `cas://` with its digest check, runs `Bebop.Reject` before evaluation, scores every file into exactly one bucket, COMPARES a negative's exit code against its `COMPILEFAIL:`/`RUNFAIL:` header, writes `results.txt`, exits 1 below the pass floor (now 120) |
| `Bebop/Basic.lean` | rc=0 | Core types: Val, Expr, Stmt, Program, State (+ State.lookup/bind/zeros/arenaRead/arenaWrite), TrapCode, Result (with `stuck` for a body that yields no value) |
| `Bebop/Builtins.lean` | rc=0 | 10 executable builtins + dispatch (imports Basic only) |
| `Bebop/Syscalls.lean` | rc=0 | 26 `axiom` sys_* specs, 5 footprints, and since 2026-09-14 a MODELLED `dispatchSyscall`: 13 names with a declared effect and value (`sys_arena_base/_end`, `open/close/fsync/msync`, `mmap/munmap/mprotect`, `clone` (parent view), `exit_thread_guard` (guard = 0 only), `write`), every other name `none` -- which is `stuck` with the name printed. NOT "0 for everything": that is one of the three fakes this directory closed |
| `Bebop/Reject.lean` | rc=0 | NEW 2026-09-14. The STATIC rejection pass, 7 rules: builtin-named `fn` (99), >14 params (100), `sys_*` in a `kernel fn` (102), `sys_mapb` (108), >8 symbols at a `sys_clone` (109), unbound symbol (101), literal index past a static length (65). Each rule's docstring names the negative it refuses AND the positive control that must survive it |
| `Bebop/Sha256.lean` | rc=0 | NEW 2026-09-14. SHA-256, pinned to three NIST vectors by `#guard`. Needed twice: to resolve `use "cas://sha256:<hex>"` (the two `.bcas` files in this tree are byte-identical, so ONLY the digest tells c50_cas from neg/c51_casbad) and to compute `results.txt`'s source hash |
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
- FIXED 2026-09-14 -- `builtinCrc32`/`builtinCrc32x` were the MSB-first update
  written with the reflected polynomial and truncated to 8 bits, with no final
  XOR: crc32("123456789") came back 15579374 where zlib gives 3421780262.
  There is now ONE reflected table (`crc32Table`/`crc32Step`/`crc32Of`) shared
  by crc32, crc32x and the newly added crc32b, pinned at elaboration time by
  `#guard` against zlib's values for "", "123456789" and "abc".
- FIXED 2026-09-14 -- **the byte arena exists**. `State.bytes` is an append-only
  `Array UInt8`; a string literal interns its UTF-8 bytes plus a NUL and
  evaluates to `(offset << 32) | length`, exactly as bpref's `'str'` node does.
  So `char` reads a real byte (it returned the handle's low byte for index 0
  and 0 after), `crc32b` exists at all, and `builtinScan` -- which used to read
  `pos` as if the pair were packed into the handle and then decline to read any
  byte, returning `posStart` unchanged -- is now bpref's loop rule for rule,
  both bounds and the write-back included.
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
- FIXED 2026-09-14 -- `||` and `&&` were modelled as plain bitwise `|||` /
  `&&&` (the pre-A26 bpref language). MEASURED on the promoted `bebop.bin`
  (sha256 3af3250...): `(2 && 1)*1000 + (2 || 1)*100 + (0 || 7)*10 + (0 && 7)`
  = **1110**, where bitwise gives 370 -- so the VALUE is logical, 0 or 1. But
  `let a = zeros(1); let r = 0 && bump(a); a[0] * 10 + r` = **70**, so both
  operands still run: NOT short-circuit (T125 unchanged). Both halves are now
  in `evalBinOp`. This also retires this tree's older note that `&&` is a
  constant zero -- false for the promoted binary.
- STILL A DOC GAP -- LANGUAGE.md:57-67 does not list `&&` or `||` at all, and
  their level had to be measured: `||` loosest, then `&&`, both ABOVE
  comparison (C-like), while comparison stays above `|`/`^`/`&` (NOT C).
  Every row is recorded with its probe in `Bebop/Parser.lean` §0.
- **NEW FINDING 2026-09-14 -- `bench/parity_constructs/c46_andor.bp`'s EXPECT
  header is STALE.** The file says `// EXPECT 111100`; the promoted compiler
  prints **101100**, and so does this Lean semantics. 111100 is exactly the
  value the program has under the PRE-A26 reading `&&` = `&`, `||` = `|` at
  bitwise precedence (checked arm by arm). So the header, not the semantics, is
  wrong -- and `construct_parity.sh` compares a run value against that header,
  so that construct's gate should be failing. `bench/` is not this lane's tree,
  so it is reported, not edited.
- OBSERVED, not this lane's to fix -- a bare `x = 5;` statement COMPILES and is
  a silent no-op: `fn main() -> i64 { let x = 1; x = 5; x }` prints **1**.
  In expression position the same assignment is refused (`if 1 then x = 9 else
  0` -> compile exit 101, "unbound symbol"). The parser here follows the
  refusal and rejects a variable assignment in expression position; it has no
  production for the bare statement form, so that shape is `invalid` here.
- SCORE, measured `lake exe parityrun <lane root>` at the default fuel, all 121
  files, 2026-09-14 after lane f4lean: **120 PASS · 1 STUCK** (`c84_run`).
  The 20 negatives all pass as REFUSALS and each prints its diagnosis in the
  run's "refusal diagnoses" section, because a refusal for the wrong reason
  scores the same as one for the right reason and the score cannot show it.
  The PREVIOUS score, for comparison:
  84 PASS · 14 STUCK · 10 VALUE_MISMATCH · 6 INVALID (all negative) ·
  3 FUEL · 2 UNSUPPORTED:`use` cas:// · 1 UNSUPPORTED:`return` in expression
  position · 1 LEX (the `@` in `neg/c143_contract_garbage`, correctly refused).
  The 14 STUCK and 4 of the VALUE_MISMATCH are SEMANTICS gaps already listed
  above (unmodelled `sys_*`, no byte arena, the wrong `crc32`/`crc32x`/`scan`);
  6 VALUE_MISMATCH are `neg/*.bp` that this model does not refuse because the
  static check involved (reserved-word shadowing, 15-parameter arity, static
  OOB, unbound symbol) is not implemented.
- FIXED 2026-09-14 -- **the arena was quadratic; it is now sparse and
  persistent.** `State.zeros` was `cells ++ Array.replicate n 0` (materialising
  n zeros) and `arenaWrite` was `Array.set!` (which copies whenever the
  reference is not unique). `Arena` is now `cursor : Nat` plus
  `cells : Std.TreeMap Nat Val`, where an allocated-but-unwritten cell READS as
  0 -- which is what `zeros` means -- so no zero is ever stored:

  | operation | was | now |
  |---|---|---|
  | `zeros(n)` | Theta(n), materialises n zeros | **O(1)**, `cursor += n` |
  | write | O(1) if uniquely referenced, O(n) if not | **O(log n) always** |
  | read | O(1) array index | **O(log n) always** |

  THE REPRESENTATION CHOICE WAS MEASURED, NOT REASONED. `Std.HashMap` was tried
  first and was WORSE than the array. Wall clock on a loop doing two 3-cell
  array literals per iteration (`c33_loopalloc`'s shape), timed from outside the
  process, ~160 ms of which is startup:

  | iterations | 2000 | 4000 | 8000 | 16000 | 32000 |
  |---|---|---|---|---|---|
  | `Array Val` (before) | 346 ms | 1195 ms | 4436 ms | 21703 ms | -- |
  | `Std.HashMap` | 457 ms | 1800 ms | 8065 ms | 47295 ms | 246918 ms |
  | `Std.TreeMap` (now) | 177 ms | 171 ms | 338 ms | **281 ms** | **494 ms** |

  The same loop with NO allocation was FLAT at 150-170 ms across the whole range
  in every version, so the interpreter was never the cost. `Array.set!` and
  `Std.HashMap.insert` are both copy-on-write-WHEN-SHARED, and this evaluator
  threads `State` functionally -- the `.while_` arm holds `result` while
  building `s3` from `result.state` -- so uniqueness is lost nearly every
  iteration and the amortised O(1) never applies. `Std.TreeMap` is a PERSISTENT
  balanced tree: `insert` allocates O(log n) nodes and shares every untouched
  subtree, so its cost does not depend on what else holds a reference. O(log n)
  guaranteed beats O(1)-if-unique/O(n)-in-practice. The O(log n) READ is the
  knowing cost of that trade; it is paid because the corpus got 47x FASTER, not
  slower -- 37840 ms -> 797 ms over all 121 files at fuel 20000, same corpus,
  same fuel, with an IDENTICAL pass set (36 non-PASS before, 36 after, empty
  diff in both directions).
- FIXED 2026-09-14 -- **a MODEL GAP could be hidden by an unused binding.**
  `c142_clone8.bp` scored `ok 201` while `sys_arena_base()` was entirely
  unmodelled: its `let base = sys_arena_base();` evaluated to `none`,
  `Stmt.let_` answered `.cont` and simply did not bind, and the tail never read
  `base`. `State.gap` is now set at every unmodelled-builtin site and
  `evalProgram` reports `stuck` even when a value came out. Controlled by
  mutation: inserting an unused `let zz = sys_slurp(0, 0);` into c27_zeroarg
  turns its `ok 7` into `STUCK unmodelled builtin `sys_slurp``.
- FIXED 2026-09-14 -- **`STUCK main yielded no value (unbound symbol,
  unresolved call, arena fault, or no tail expression)` named four causes at
  once**, and 12 constructs carried it simultaneously, which is why none of
  them was ever closed. `State.stuckWhy` is recorded at the SITE that first
  produced no value (first writer wins) and printed instead.
- The three FUEL rows resolved three different ways, which is why they were
  reported separately: `c33_loopalloc` was the quadratic one and now gives
  `ok 24999750000` -- its exact EXPECT -- at fuel 150000 in **1071 ms**, where
  before it did not finish in 60 s at fuel 100000; `c67_deeprec` was genuinely
  depth-bound and gives `ok 100000` at fuel 2000000 in 339 ms; and
  `neg/c48_stackovf` was still FUEL at 2000000, and the claim that it "always
  will be" was WRONG: the construct expects RUNFAIL:82, a stack overflow, and
  a model with no bounded stack cannot produce it.
  `Bebop.Semantics.callDepthLimit` (131072 activations) is that bound, separate
  from fuel, and c48 now reports `trap stackOverflow (exit 82)`. The control is
  in the lane verdict: a program that merely needs a lot of fuel (a
  10^9-iteration `while`) still reports FUEL, not 82.
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
