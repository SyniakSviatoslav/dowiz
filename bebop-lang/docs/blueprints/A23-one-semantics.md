Status: 2026-09-12, research pass (read-only, NO code written, NO run executed). Quoted with
`path:line` or derived and marked so. Grounded at HEAD `d75172c` (working tree: `M tools/bpref.py`, the
`set` fix, UNCOMMITTED). Roadmap row A23 (new, harness + `tools/bpref.py`, no compiler change). Delivers
language item 4 (one semantics, not two) in the only form that is cheaper than the problem: the
semantics is defined once by `bebop.bp`; every other implementation is a checker over a DECLARED subset,
the subset is a number in the battery, and disagreement is a battery RED, not a four-day surprise.

# A23 -- bpref demoted to a measured differential judge

## 0. Three corrections this blueprint rests on

**(0a) The language is implemented four times, not two.** `bebop.bp` (7,720 lines, the emitter);
`tools/bpref.py` (736 lines, "the executable grammar and semantics reference", `docs/LANGUAGE.md:1`, the
fuzz DIVERGE judge, `bench/fuzz/fuzz.sh:3`); `formal/Bebop/Semantics.lean` (607 lines, F4's definitional
interpreter, never run on-box -- `docs/VERIFIED-STATE-2026-09-12.md`: "the Lean semantics have not been
run against them"); `tools/typecheck.py` (177 lines, the T48 type rules over bpref's AST). Plus
`bench/fuzz/gen.py` (399 lines) which must learn every shape or the fuzzer never emits it (`gen.py:8-10`
lists what it avoids: strings, unary, return/break).

**(0b) bpref's evaluator is executed by NOTHING in the battery.** `tools/battery.sh:33-46` runs
`std_par.sh`, `construct_parity.sh`, `parity_driver.sh`, `pool_parity.sh`, `run_all.sh`, `invariants.sh`,
census, `check_abi`, `diag_check`, `check_words`, `f8_dt`. The word `bpref` appears in `construct_parity.sh`
only in comments (13 times, e.g. `:80,89,138`), in `invariants.sh:53` for rung (vii) which uses bpref's
PARSER through `typecheck.py`, and in 1 of 128 `bench/oracles/*.py`. Its evaluator runs in fuzz batches
(A4: "runs LAST") and when a worker types `python3 tools/bpref.py <file>` to derive an EXPECT. So the
`set`-node defect landed in `c99cddb` (2026-09-11), made `c70_qdsl` red, and was found by hand on
2026-09-12 (journal 1789244400) -- and it is STILL uncommitted at HEAD.

**(0c) bpref carries program-specific knowledge.** `bpref.py:576-620`: nine `qdsl_*` "builtins"
(`qdsl_char`, `qdsl_len`, `qdsl_ws`, `qdsl_nsz`, `qdsl_alloc`, `qdsl_set`, `qdsl_kind`, `qdsl_nch`,
`qdsl_ch`) with bodies that re-implement functions `c70_qdsl.bp` and `selfhost/std/qdsl.bp` DEFINE. They
are shadowed today only because `builtin_or_call:573` checks `self.fns` first; a program that omits one
of those names gets bpref's copy silently -- and the copy carried the `arr = list(arr)` bug too (journal
1789244400: "two more copies of the same line sat in the qdsl_alloc / qdsl_set shims").

---

## 1. Scope

### A23 IS

1. **Commit the `set` fix** (`bpref.py:536-547`) -- it is the first thing, and it is a one-line `git add`.
2. **Delete the nine program-specific shims** (`bpref.py:576-620`). An oracle that knows a program's
   function names is not an oracle; `qdsl.bp` defines them and bpref interprets them.
3. **bpref declares its grammar and REFUSES the rest with a named exit**: any form outside its documented
   surface (`bpref.py` docstring is the grammar per T39, HISTORY.md T39) raises `UNSUPPORTED:<form>` with
   exit code 3 (distinct from a Python traceback's 1 and from the program's own `SystemExit` codes), so a
   construct using a feature bpref lacks counts as `unsupported`, never as PASS and never as DIVERGE.
   Today an unknown form is a `SyntaxError('unexpected top-level ...')` (`:245`) -- a traceback the
   fuzzer classifies as `BPREF-ERROR` (`fuzz.sh:7`), which conflates "bpref is broken" with "bpref does
   not implement this".
4. **A battery lane `bench/vs_rust/bpref_parity.sh`**: for every construct with a numeric EXPECT (89
   positive files; `RUNFAIL`/`COMPILEFAIL` rows skipped), run bpref; report `bpref_parity: agree=<a>
   unsupported=<u> disagree=<d> native=<n>` where `native` counts constructs whose EXPECT header says
   bpref cannot compute them (host-only builtins, `sys_*`, `clock_ms`, threads). RED on `d > 0`. Wired
   into `battery.sh` as a lane with its own `line()` regex.
5. **Every EXPECT names its derivation**: `bench/parity_constructs/<c>.bp` line 2 carries `// EXPECT
   <value> from: bpref | hand: <method> | native: <why bpref cannot>`; `tools/arch_check.py` gains a check
   `expect-derivation` counting constructs without one, ratchet 0. (After A24 the EXPECT itself moves
   into the file; this header is its first half.)

### A23 IS NOT

1. **Not "generate bpref from the compiler's tables."** Not attempted; the compiler has no tables to
   generate from (60-odd text-directed `emit_*` fns over 43 fntab bases at 315 sites); a generable form
   is a table-driven front end = the A12 rung, ~2,000 lines, refuted for speed. Costed at 2-3 weeks of the
   `bebop.bp` lane and not scheduled.
2. **Not an interpreter mode inside `bebop.bin`.** A second walk of the same grammar in Bebop, same
   cost as 1, against a 768-fn cap with 295 used. Not scheduled.
3. **Not deleting bpref.** It is the only oracle that runs on this box for the fuzzer, and its 700 lines
   are cheaper to keep honest than to replace. What is deleted is its AUTHORITY: it stops being "the
   semantics reference" in `LANGUAGE.md:1` and becomes "a checker over the subset it declares".
4. **Not F4 (Lean).** When the Lean run produces a results file (F4's design), it joins `bpref_parity`'s
   table as a second column under the same rule: agree / unsupported / disagree, per construct.
5. **Not a change to how EXPECTs are chosen.** Hand derivation stays the honest default for anything
   bpref cannot run; the row only makes the choice VISIBLE and counts it.

---

## 2. The gate

### 2.1 Files

| file | what |
|---|---|
| `bench/vs_rust/bpref_parity.sh` | the lane; one line of output |
| `tools/battery.sh` | one `line bpref.log '^bpref_parity:' 'disagree=0'` |
| `tools/bpref.py` | the `set` fix committed; shims deleted; `UNSUPPORTED` exit 3; docstring = the declared grammar |
| `tools/arch_check.py` | `expect-derivation` check, ratchet 0; `bpref-shims` check: `grep -c "qdsl_" tools/bpref.py` == 0, ratchet 0 |
| `bench/parity_constructs/*.bp` | derivation headers on all 89 positive constructs |
| `docs/LANGUAGE.md:1` | "tools/bpref.py is the executable grammar and semantics reference" -> "tools/bpref.py is a differential checker over the subset its docstring declares; `bebop.bp` defines the language" |

### 2.2 The number

```
bpref_parity: agree=<a> unsupported=<u> disagree=0 native=<n>     (a + u + n == 89)
expect-derivation: 0 constructs without a derivation (ratchet 0)
bpref-shims: 0 (ratchet 0)
```

Expected on day one, derived: `native` >= the constructs using `sys_*`/`clock_ms`/threads/`crc32*`/`scan`
(`c42_crc32`, `c45_crc32x`, `c78_scan`, `c84_run`, `c94_fsync`, `c110_fence`, `c97-c99`, the `c_gb_*`
and `c_st_bytes` files: ~15); `unsupported` = 0 today (bpref implements the whole current surface per
F0's four-way diff, `ROADMAP.md:184`, minus `hvham`/`hvham2` which are `native`); `agree` = the rest.

### 2.3 How each assertion goes RED

| # | assertion | goes RED when |
|---|---|---|
| A | `disagree=0` | bpref and `bebop.bin` differ on any construct -- the `c70` class, caught in one battery instead of four days. Which side is wrong is decided the 2026-09-12 way (shrink; a hand-derived third value), never by editing the EXPECT to match either |
| B | `a + u + n == 89` | a construct was neither run nor classified (the lane skipped a file) |
| C | a mutant bpref that re-introduces `arr = list(arr)` in `set` makes `disagree >= 1` (`c70_qdsl`) | the lane does not actually run bpref's evaluator on `c70` (e.g. it is misclassified `native`) |
| D | `bpref-shims: 0` | someone adds a program's fns to the oracle again |
| E | `expect-derivation: 0` | a new construct lands with a bare EXPECT |
| F | a construct using a form bpref lacks (after A18/A21/A22 land) is counted `unsupported`, not `agree` | bpref silently evaluates a form it half-implements (the `SyntaxError` -> `UNSUPPORTED` refusal must be at the PARSER for the new keyword, not a downstream KeyError) |

---

## 3. Which existing work A23 sits on

| existing | verdict | reason |
|---|---|---|
| `tools/bpref.py` `set` fix (working tree) | **COMMITTED first** | uncommitted at HEAD |
| `bpref.py:576-620` shims | **DELETED** | program knowledge in the oracle |
| `bpref.py` docstring (T39: "its docstring is the grammar", `gen.py:3-4`) | **KEPT as the declaration** the `UNSUPPORTED` exit refers to | |
| `bench/fuzz/fuzz.sh` classes (`:7`: OK DIVERGE COMPILEFAIL CRASH TIMEOUT BPREF-ERROR ...) | **EXTENDED**: `BPREF-UNSUPPORTED` as its own class (exit 3), distinct from `BPREF-ERROR` | a traceback and a declared gap are different facts |
| `bench/fuzz/gen.py` | UNCHANGED here (it already emits only what bpref documents) | |
| `tools/builtin_surface.py` (F0: recovers builtin names BY HASH from the dispatch ladder and diffs four lists) | **THE MODEL** for the grammar diff: a `tools/grammar_surface.py` that lists `emit_factor`'s keyword tests and `emit_body_classify`'s statement forms and diffs them against bpref's docstring is ~60 lines and is step 3 | the surface census already exists for builtins |
| `construct_parity.sh` EXPECT comments (`:80,89,134,138,141`) | **MOVED** into the construct headers (derivation) | co-located with the code an agent edits |
| `docs/LANGUAGE.md:1` | **CHANGED** (one sentence) | the authority statement is the defect's root |
| D5b's witness (`selfhost/attic/expr_compile.bp`, 46/73 non-vacuous) | UNTOUCHED; named as the only in-tree route to a second INDEPENDENT executor without CPython | `docs/TRUST-CHAIN.md` §4a |
| F4's `formal/` | UNTOUCHED; joins the table when it runs | |

---

## 4. Constraints as design input

1. **No compiler change**: WORKER-CARD "non-codegen change" gates apply (typecheck 0 findings,
   `run_all` mismatch=0 missing=0, one full `std_golden.sh`), no chain.
2. **bpref runs under `ulimit -s`/recursion limits** (`bpref.py:72` `DepthError`); the lane must classify
   `BPREF-DEPTH` as `native`-with-reason, not as disagree, and print which.
3. **The lane's cost**: 89 bpref runs at the fuzzer's measured 7.3 programs/s (journal 1789244400) =
   ~12 s; `c70_qdsl`'s 339 lines are the slowest. Acceptable in the serial battery.
4. **Every new language row** (A18, A21, A22, A16, A24) must state in its blueprint which of `bpref
   node | UNSUPPORTED` it lands with; the `expect-derivation` check makes a construct with neither
   impossible to land.
5. **The oracle's own honesty**: an `UNSUPPORTED` exit must happen at parse time for the new keyword,
   before any evaluation, or a half-evaluated program can print a number.

---

## 5. Steps, in order, each with a number that can kill it

### Step 1 -- commit, delete, declare (0.5 day)

**Do:** commit the `set` fix; delete the shims; add the `UNSUPPORTED` exception + exit 3 at the parser's
unknown-form sites (`:245` and `factor()`'s fallthrough); update the docstring.
**Expected:** `python3 tools/bpref.py bench/parity_constructs/c70_qdsl.bp` prints 41000 (the user's
`qdsl_*` fns run, the shims are gone); a file with `theorem x` prints `UNSUPPORTED:theorem` rc=3.
**Kills:** `c70_qdsl` prints something else (a shim was load-bearing: then the construct's fn was
never interpreted and the EXPECT is suspect -- re-derive by hand).

### Step 2 -- the lane (1 day)

**Do:** `bpref_parity.sh`, the 89 derivation headers (`bpref` for the ones bpref computes, `hand:`
copied from the existing `construct_parity.sh` comments for `c73`/`c74`-class, `native:` for the ~15),
`battery.sh` line, `arch_check` checks.
**Expected:** `bpref_parity: agree=~74 unsupported=0 disagree=0 native=~15`; `battery: GREEN`.
**Kills:** `disagree >= 1` on day one -- that is a REAL finding (a second `c70`), filed, shrunk, and the
lane stays RED until the guilty side is fixed; do not classify it away.

### Step 3 -- the grammar diff (0.5 day, optional but cheap)

`tools/grammar_surface.py` in F0's shape: keyword and statement forms recovered from `emit_factor` /
`emit_body_classify` by their character tests vs bpref's docstring; `grammar_surface: <n> forms, <m>
declared by bpref, <k> unsupported` -- so "bpref supports X" is a diff, not a belief.

---

## 6. The honest ceiling

- **This does not make the semantics single.** It makes the DIFFERENCE between the implementations a
  number, per construct, in every battery. That is what the honesty floor can afford; a single
  semantics is the A12-class rewrite, costed and not scheduled.
- **bpref's stubs stay stubs**: `sys_*`/`clock_ms` return 0 (`bpref.py:669`), so ~15 constructs are
  `native` forever unless someone models syscalls in Python -- which would be a fifth semantics.
- **The fuzzer's coverage shrinks with every feature bpref refuses**, by exactly the `unsupported`
  count. That is the honest trade of not maintaining feature parity: named per row, visible in the
  number, reversible per feature when a bpref node is worth its 30-80 lines.
- **A second independent executor** (the D5b witness at 73/73, or F4 running) is the only thing that
  turns "agree" into evidence about the language rather than about two programs written by the same
  people. Not this row's cost.

---

## 7. VERDICT format for an A23 worker

```
VERDICT: GREEN|RED
step: <1-3>
bpref_parity: agree=<a> unsupported=<u> disagree=<d> native=<n>   (sum <89>)
c70_qdsl via bpref (shims deleted): <value> (41000)
UNSUPPORTED probe: `theorem x` -> rc=<3> text <UNSUPPORTED:theorem>
mutation: `arr = list(arr)` re-inserted -> disagree=<>=1> naming <c70_qdsl>
arch_check: expect-derivation <0> bpref-shims <0>
battery: GREEN|RED   std_golden <p>/<f>   typecheck <0> findings   run_all mismatch=<0> missing=<0>
journal: <one line, WORKER-CARD format, with COST:>
open: <deviations>
```
