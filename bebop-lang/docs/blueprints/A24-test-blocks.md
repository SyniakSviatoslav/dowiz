Status: 2026-09-12, research pass (read-only, NO code written, NO run executed). Quoted with
`path:line` or derived and marked so. Grounded at HEAD `d75172c`. Roadmap row A24 (new). The ELEVENTH
item, added under the agent framing after testing three candidates against the tree (RESEARCH §6.1):
a program states its own expected value, so the gate is read from the file an agent edits, not from a
260-line `case` table in another file. This is T81 (`test name { ... }` blocks with erasure, TASKS.md
OPEN) with a gate.

# A24 -- self-describing constructs: `test` blocks and a `bebop.bin test` verb

## 0. Three corrections this blueprint rests on

**(0a) The failure this closes is measured, not hypothetical.** Journal 1789244500 (2026-09-12): "Three
of the nine [failures] were holes, not regressions: `c70_qdsl` and `c70_qdsl_neg` (added by `c99cddb`)
and `neg/read_before_assign` (added by `4286ec1`) have NO EXPECT row, so the gate has been red on them
for days while everything upstream reported green." The expectation lives in `bench/vs_rust/
construct_parity.sh:49-238`, a `case` table an agent forgets to edit when it adds a file to
`bench/parity_constructs/`. A construct with no row is not a failure the harness can name: the `case`
falls through to `EXPECT` unset and `TRAP_MISMATCH ... want ?` (`:255`).

**(0b) "Erased" must mean "explicitly skipped", not "ignored".** `tools/f8_dt.py` passes today because
`collect_fns` (`bebop.bp:5870`) scans only for `fn ` and a top-level `theorem ...` line is never seen
(RESEARCH §1 8-10b). A `test` block that is "erased" by that mechanism would be erased by accident, and
a `test` block containing the text `fn ` inside would be compiled as a function. This row makes the
scanner SKIP `test NAME { ... }` by brace matching (`find_body_end:4962` already does that for bodies) --
and the negative construct proves it.

**(0c) The compiler's own self-test is dead and is the precedent to retire.** `self_check()`
(`bebop.bp:6368`) has no caller and its goldens are stale (ROADMAP A6: "`self_check()`'s c29/c30/c31/c32
goldens are now stale; it has no caller"). A `test` form the compiler runs on request replaces it; the
dead fn is deleted under the dead-function ratchet (`arch_check` CHECK 19).

---

## 1. Scope

### A24 IS

1. **A top-level form** `test NAME { <body> }` whose body is a fn body (statements + one tail
   expression, exit 97 rule unchanged). Under `compile` and `check` it is SKIPPED by `collect_fns`
   (brace-matched), so the emitted program is byte-identical to the same file without the block.
2. **A CLI verb** `bebop.bin test <src.bp>`: compiles the program with every `test` block turned into a
   fn `test_NAME` and a synthetic `main` that calls each in file order and prints `NAME <value>` per line
   (the seed prints `main`'s value; here `main` writes lines with `sys_write` and returns the count).
   Implemented as a textual rewrite in the `use_expand` shape (`bebop.bp:6869`): the expanded source is
   written to `<src>.test` (like `<out>.use`) and compiled by the unchanged emitter. Zero emitter change.
3. **Constructs become self-describing**: each `bench/parity_constructs/<c>.bp` carries `test <c> {
   <expr> }` and a header `// EXPECT <value> from: hand: <derivation> | bpref | native: <why>` (A23's
   derivation line). `construct_parity.sh` reads the header for the value and the `test` block through
   `bebop.bin test` for the run; the `case` table is deleted. A file without a header is a FAIL named
   `NO_EXPECT <c>`, never a fallthrough.
4. **`neg/` constructs keep their `EXPECT=COMPILEFAIL:<code>` in the header** (they have no value to
   test); `RUNFAIL` likewise.
5. **Two constructs**: `c133_testblock` (a program with a `test` block whose `compile` output is
   byte-identical to a twin file without it -- the `c111_kernelfn` "marked/unmarked twins are the same
   652 bytes" precedent, ROADMAP C1) and `c134_testfn_inside` (a `test` block containing the text
   `fn ` in a string; under `compile` it must NOT become a function: the program must compile and run
   to its twin's value, and `strings` of the two `.bin`s must match).

   > **CORRECTED 2026-09-13: this said `neg/c134_testfn_inside`, and `neg/` is the wrong lane for it.**
   > `construct_parity.sh` iterates `neg/*.bp` expecting a `COMPILEFAIL:<code>` (or `RUNFAIL`)
   > outcome (`:221`), so a construct that must COMPILE AND RUN -- which is exactly what this row
   > asks of c134 -- fails that lane by construction. It belongs in the positive set with a twin,
   > like c133. Two further constraints, both from the language rather than from taste: a `str`
   > literal is "only valid as an argument" (`docs/LANGUAGE.md:91`), so the `fn ` bytes must appear
   > as a CALL ARGUMENT (e.g. `str_len("fn ")`) and not as a bare statement, which does not parse;
   > and **c133's test block must itself contain a string literal**, or assertion A never exercises
   > the `scan_literals` half of the skip -- a block holding only `42` shifts no literal table and
   > would pass with that scanner untouched, which is the one failure mode §2.3 A calls out.

### A24 IS NOT

1. **Not a tautology guard.** A `test` whose body recomputes the oracle's closed form is as tautological
   in the file as it was in the table (the Haiku incident). What this row buys is co-location and a
   named hole; the derivation header (A23) is the reviewer's handle.
2. **Not `assert`/`expect` builtins inside ordinary code.** F8's `requires`/`ensures` are contracts with a
   proof obligation; this is a test harness form with a value. Different rows.
3. **Not run by `compile`.** The battery's `construct_parity.sh` calls `bebop.bin test`; `compile` never
   sees a test.
4. **Not a replacement for `std_golden.sh`'s gates**, which need arguments, files, stores and timeouts.
5. **Not T81's full ask** (erasure + `test` in std programs with the `.bt` token stream); the
   construct-lane form only.

---

## 2. The gate

### 2.1 Files

| file | what |
|---|---|
| `bench/parity_constructs/c133_testblock.bp` + `c133_testblock_twin.bp` (no test block) | byte-identity under `compile` |
| `bench/parity_constructs/neg/c134_testfn_inside.bp` | skip-by-brace proof |
| `bench/vs_rust/construct_parity.sh` | header-driven; `NO_EXPECT` failure; the `case` table removed |
| `bebop.bp` | `cli_test` (new verb), `collect_fns` skip, `test_expand` (textual) |

### 2.2 The number

```
construct parity: pass=<n> fail=0 no_expect=0      (the `no_expect` field is new and must print)
```

plus `cmp c133_test.bin c133_twin_test.bin` = identical (the byte-identity assertion inside the lane).

### 2.3 How each assertion goes RED

| # | assertion | goes RED when |
|---|---|---|
| A | `c133` and its twin compile to identical bytes | the skip in `collect_fns` leaves a word behind, or the literal scanner (`scan_literals:6133`) counts a string inside the test block (it must skip the block too, or the literal table shifts every `adr`) |
| B | `c134` runs to its twin's value | a `fn ` inside the block is collected (the skip is not brace-matched) |
| C | `no_expect=0` and a deliberately headerless copy of `c01_lit.bp` makes `no_expect=1` and the lane RED | the header parse falls through to a default |
| D | `bebop.bin test c01_lit.bp` prints `c01_lit 1000000065571` | the synthetic `main` prints the wrong block, or `sys_write` of the decimal is wrong (`put_num` exists, `:46`) |
| E | every existing construct's value under `bebop.bin test` equals its old `case` value (104/104) | the rewrite changed the program (it must not: the test block is appended, not spliced into the program's own `main` -- and `main` is renamed in the `.test` expansion, not deleted, so a construct whose fns call `main` still resolves) |

---

## 3. Which existing work A24 sits on

| existing | verdict | reason |
|---|---|---|
| `use_expand` (`bebop.bp:6869`) / `<out>.use` | **THE SHAPE**: `test_expand` writes `<src>.test` and hands it to the unchanged emitter | zero emitter change; a diagnostic position in `.test` is the same class as `.use` positions today |
| `collect_fns` (`:5870`), `scan_literals` (`:6133`), `find_struct`/`collect_ctors` | **EXTENDED** with a brace-matched skip of `test ` blocks (`find_body_end:4962` is the matcher) | all four whole-source scans must skip, or one sees the block |
| `cli_check` (`:7244`), `main`'s verb dispatch (`:7690+`, verbs `compile`/`check`/`size`/`version`/`run-via-exec`/`cas`) | **EXTENDED** with `test` | |
| `put_num` (`:46`), `emit_sys_write` | REUSED by the synthetic `main` (as source text, not emitter code) | |
| `self_check()` (`:6368`, dead) | **DELETED** | its job is this row's |
| `construct_parity.sh` `case` table (`:49-238`) | **DELETED**, replaced by the header read | the hole class |
| `neg/` `EXPECT=COMPILEFAIL` rows (`:206-238`) | **MOVED** into headers | |
| `tools/bpref.py` | **EXTENDED**: `test NAME {` parsed and run under a `--test` flag (a bpref node, 20 lines) so A23's `bpref_parity` runs the same blocks | |
| `bench/fuzz/gen.py` | UNCHANGED (fuzz programs have no tests) | |
| T81 (TASKS.md OPEN) | **PARTIALLY CLOSED** by this row; the `.bt` half stays open | |

---

## 4. Constraints as design input

1. **Byte-identity is the whole point.** A `test` block must cost zero words and zero literal-table
   entries under `compile`. Assertion A is the proof and a mutation (leave one scan unskipped) must break
   it.
2. **The `.test` expansion is text**, so positions in diagnostics point into it -- acceptable for a test
   verb; A17 §6 names the `.use` mapping cost.
3. **A `test` body is a fn body**: the exit-97 tail rule, the 14-param cap (0 params), the 768-fn cap
   (each test is a fn: a file with 500 tests is refused -- fine).
4. **`main` renaming**: the expansion renames the program's `main` to `test_main_` (a hash-safe
   identifier) and emits its own `main`; `idx_of_main` (`:6682`) finds the new one.
5. **Bootstrap**: two-stage in principle (a new top-level form); the compiler will not carry tests in
   `bebop.bp` (it has no `main` to rename... it does: `main` at `:7690`; a compiler test suite in-file is
   possible later and is not proposed).

---

## 5. Steps, in order, each with a number that can kill it

### Step 1 -- the skip and the byte-identity twin (one codegen commit)

**Do:** brace-matched skip in the four scans; `c133` + twin; `c134`.
**Expected:** fixpoint; WORD_DELTA 0 on 104/104; `cmp` identical for `c133`/twin; `c134` runs.
**Kills:** `cmp` differs by exactly the literal table (the literal scanner still counts the block's
strings) -- fix the scanner, not the test.
**Effort:** 1 day. **Gate-days:** 1.

### Step 2 -- `bebop.bin test` (same or next commit; the emitter is unchanged, so no chain if step 1's
chain covered the scans)

**Do:** `test_expand`, `cli_test`, the synthetic `main`.
**Expected:** `bebop.bin test c01_lit.bp` -> `c01_lit 1000000065571`.
**Kills:** a construct whose fns are named `test_*` already (grep: none) or whose `main` is called from
another fn (grep `main(` in constructs: check before renaming).

### Step 3 -- the lane reads headers (harness, 1 day)

**Do:** headers on 104 files (A23's derivation line + the value); `construct_parity.sh` rewritten
(~60 lines shorter); `no_expect` field; the headerless-copy mutation.
**Expected:** `construct parity: pass=106 fail=0 no_expect=0`; identical pass set to before.
**Kills:** any construct's value differs between the `case` table and its header (a transcription
error -- resolve by re-running, never by editing the header to match).

---

## 6. The honest ceiling

- **A test block does not know what it should compute any better than a `case` row did.** The
  derivation header is a comment; a reviewer reads it. What is gained: the value, its derivation and the
  code are in one file, and a MISSING one is a named failure instead of a silent `want ?`.
- **`bebop.bin test` runs on the compiled program**, so a `test` block cannot test a compile failure;
  `neg/` stays header-only.
- **Cost of the two-scan discipline**: every future whole-source scan (`hoist_scan`-class text scans are
  per-fn and unaffected) must skip `test` blocks too; `arch_check` could count `while j + 2 < strn` style
  whole-source loops and require the skip -- costed at 20 lines of python, optional.
- **Not general-purpose test infrastructure**: no fixtures, no setup, no expected-failure. An agent
  needs exactly `name -> value`, and a human gets the same.

---

## 7. VERDICT format for an A24 worker

```
VERDICT: GREEN|RED
step: <1-3>
fixpoint: gen3 == gen4 <md5>   bebop words <before> -> <after> (budget line yes|no)
byte-identity: cmp c133 vs twin <identical|differs at byte N>
constructs: pass=<n> fail=<n> no_expect=<0>; c134 <value> == twin <value>
test verb: `bebop.bin test c01_lit.bp` -> <line>
mutation: headerless c01 copy -> no_expect=<1> RED <yes|no>; unskipped scan -> cmp differs <yes|no>
bpref: `bpref.py --test c01_lit.bp` -> <line>
journal: <one line, WORKER-CARD format, with COST:>
open: <deviations>
```
