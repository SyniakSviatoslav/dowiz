Status: 2026-09-13, written for the ROADMAP row that cites it -- **row F4, ROADMAP.md:198**, whose blueprint column reads `docs/blueprints/F3-lean-semantics.md` (the row letter and the file name differ by one; F_PHASE_STATUS.md:276 records the same file as missing). Base commit `ddb534f`. Every `file:line` below was re-derived with `sed -n` against that tree on 2026-09-13; a line that could not be confirmed is written "anchor not located". Binary under test: `bebop.bin` digest `072c01d1` (8 hex = md5 prefix, `tools/battery.sh:56`; not a commit hash).

# F3 Lean semantics: make the third implementation run, then gate it

## 0. Goal, and the number today

The row asks for an executable definitional semantics of the whole language in Lean 4, gated by `lean_conformance: c/86`, `builtin_spec: b/36`, `bpref_diff: 0` and a results file hash-bound to the `.lean` sources. The material exists: `formal/` holds 2,853 lines of Lean across seven modules plus a driver. **The number today is 0 on every gate, and this blueprint is built around the reason: nothing in the tree has ever elaborated `formal/`, and the committed files cannot be elaborated as they stand.** Three defects, each verified by running Lean on-box (§2), keep the development from building under ANY Lean version; two more would make the harness report FAIL the first time it ran (§2.4). The work is therefore not "write more Lean" -- it is (1) make what exists build, (2) give it a front end that is not a fourth parser, (3) make its verdict a number the chain recomputes, and only then (4) close the gap to the whole language.

The ROADMAP's own framing governs the value of the result: Phase F's header (ROADMAP.md:191) demotes Lean "from producer to cross-check oracle". So F3 buys a third independent executor of the language -- next to `bebop.bin` and `tools/bpref.py` -- and the only one that can later carry a proof. It buys no theorem by itself (§4.7).

## 1. What is in the tree (measured, not read from the README)

| file | lines | `sorry` | `axiom` | `partial def` | `#guard` | note |
|---|---|---|---|---|---|---|
| `formal/Bebop/Basic.lean` | 246 | 0 | 0 | 0 | 0 | types; `import Lean` at :13 (Init suffices, §2.3) |
| `formal/Bebop/Semantics.lean` | 607 | 0 | 0 | 4 | 0 | evaluator; the ROADMAP's "607 lines" is exact |
| `formal/Bebop/Builtins.lean` | 324 | 0 (the word occurs once, in a comment at :2) | 0 | 0 | 0 | 10 executable builtins, dispatch at :282 |
| `formal/Bebop/Syscalls.lean` | 398 | 0 (comment at :377) | 26 | 0 | 0 | `dispatchSyscall` :385 returns `(s, 0)` for every `sys_*` |
| `formal/Bebop/Traps.lean` | 193 | 0 | 0 | 0 | 3 | 24-row table at :37; `#guard` at :114-116 pins 4 closed / 20 open / 24 |
| `formal/Bebop/Conformance.lean` | 698 | 0 | 0 | 0 | 0 | 75 + 11 EXPECT rows (:43, :218), 121 oracle entries (:266), 5 inline-AST samples (:555-660) |
| `formal/Bebop/Theorems.lean` | 387 | 0 | 7 | 0 | 5 | F9's "7 theorems" are 7 `axiom`s (:105, :169, :220, :231, :287, :295, :305) checked by `#guard` on sample values (:372) |
| `formal/harness.lean` | 89 | 0 | 0 | 0 | 0 | `#eval` driver for the 5 samples; never wired |
| `formal/lean-toolchain` | 1 | | | | | pins `leanprover/lean4:v4.12.0` |

Counts by `grep -c`; the README's table (`formal/README.md:21-32`) says "Compiles" for five of these files and "6 sorry" for Builtins -- neither is current, and "Compiles" was never true (§2).

History (read-only `git log -- bebop-lang/formal`, main repo): scaffold in `7c03111` (2026-09-10), builtin stubs `7de7922`, Theorems `8bc552a` (2026-09-11), harness + `tools/tv_fragments.py` in `4286ec1` (2026-09-11). Every hash verified `git cat-file -t` = commit. No commit touches `formal/` after 2026-09-11.

What runs it: **nothing.** `grep -rn -i "lean\b|lake\b|results.json|formal/"` over `tools/battery.sh`, `tools/chain.sh`, `tools/*.py`, `tools/*.sh`, `bench/*.sh`, `bench/vs_rust/*.sh` finds only `tools/arch_check.py:277` (the cited-file regex admits `.lean` names) and `tools/certgen.py:6-15,211-214` (uses the `cadical` binary shipped inside the Lean toolchain). No battery lane, no chain step, no script invokes `lean` or `lake`. The A23 row (ROADMAP.md:108) says so in words -- "a third implementation that nothing runs" -- and it is still true at `ddb534f`.

## 2. Why nothing runs it: three build-breaking defects, verified on-box

These were found by copying the files to a scratch directory and running the Lean that IS on the box (§3), one process at a time, never `lake build`.

### 2.1 A nested comment that never closes -- `formal/Bebop/Basic.lean:25`

```
/-- Signed truncating division. LANGUAGE.md:66: x/0 = 0, MIN/-1 = MIN. -/
```

Lean block comments nest, and `MIN/-1` contains `/-`. The comment opened at :25 therefore swallows the rest of the file, and `lean Basic.lean` reports `Basic.lean:247:0: error: unterminated comment` (measured: rc=1, 6.9 s). This one token means **`Basic.lean` has never elaborated under any Lean 4 toolchain, and every module imports it.** Fix: `MIN / -1` (one space). With that change alone the file elaborates: rc=0, 12.6 s, peak RSS 535 MB (§3).

### 2.2 Import cycles -- `Builtins.lean:9`, `Semantics.lean:22`, `Syscalls.lean:20`

`grep -n "^import" formal/Bebop/*.lean`:

- `Builtins.lean:9  import Bebop.Semantics` and `Semantics.lean:22 import Bebop.Builtins`
- `Syscalls.lean:20 import Bebop.Semantics` and `Semantics.lean:23 import Bebop.Syscalls`

Lean's module system refuses cycles, so `lake build` cannot succeed on this graph regardless of the comment fix (measured: after producing `Basic.olean`, `lean Builtins.lean` stops at `Builtins.lean:8` because `Semantics.olean` does not exist, and it never can). The cause is visible in the code: `Builtins.lean` needs `State.zeros`/`arenaRead`/`arenaWrite`, which live in `Semantics.lean:61-79`, while `Semantics.lean:259,263` calls `dispatchBuiltin`/`dispatchSyscall`. Fix: move the `State` memory operations (`Semantics.lean:37-79`) into `Basic.lean` (or a new `Memory.lean` under `Basic`), and make `Semantics` the top of the DAG. Nothing else in `Builtins`/`Syscalls` references the evaluator.

### 2.3 The toolchain pin names a Lean that is not here, and the code needs a newer one

`formal/lean-toolchain` pins `v4.12.0`. `~/.elan` does not exist. The only Lean reachable on the box is `/root/s30/outC4/lean/bin/lean`, **version 4.33.1** (`lean --version`, measured), the same toolchain `tools/certgen.py:211` already uses for `cadical`. `Basic.lean:21` builds `Val` on `Int64`; from memory (unpinned, unverifiable on this box), `Int64` entered Lean core after 4.12, so the pinned toolchain would reject the file even with §2.1-2.2 fixed. Fix: pin the toolchain that is actually run and record its hash in the results file (§4.6); the row already asks for "the toolchain hash".

### 2.4 Two defects that would surface the first time the harness ran

- `Conformance.lean:576-592` (`sample_c02`) encodes `let a = 10; let b = 3; a + b * 2` (= 16) against `positiveExpectations[1]` = 34, and the docstring above it (:569-575) reads "Wait -- construct_parity.sh says c02_arith EXPECT=34. But `10 + 3 * 2 = 16`, not 34. Let me re-read the .bp file." The real `bench/parity_constructs/c02_arith.bp:3` is `20 + 22 - 2 * 5 / 2 - 7 % 4`. The sample would FAIL; nobody saw it because nothing ran.
- `Semantics.lean:436`: `match s.lookup "_scrutinee"` -- the `matchExpr` rule has no scrutinee in the AST (`Basic.lean:88`, `matchExpr (arms : Array MatchArm)`) and reads it from a magic environment key that no rule ever binds. Every `match` therefore evaluates to `none`. LANGUAGE.md:80-81 makes `match` compile-time over a literal constructor; the rule must carry the scrutinee.

### 2.5 Claims in the tree that these measurements contradict

- `formal/README.md:13` and `F_PHASE_STATUS.md:124`: "Lean 4 cannot run under the box's 3GB/32-process caps". Refuted by §3; the ROADMAP row itself flags this as "unmeasured", correctly.
- `formal/README.md:21-32` "Compiles" / "Compiles without sorry"; `F_PHASE_STATUS.md:101-112` "COMPILES". Refuted by §2.1-2.2: no file ever compiled.
- `formal/README.md:8`, `Conformance.lean:675,681`: 86 constructs, 121 oracles, 36 builtins. The tree has **100 positive + 20 negative constructs** (`ls bench/parity_constructs/*.bp`, `neg/*.bp`), **128** files in `bench/oracles/*.py`, and **38 builtin dispatch arms** (`tools/builtin_surface.py`, run 2026-09-13: "compiler dispatches 38, resolved 38, unresolved 0; typecheck 37, bpref impl 12 / stub 24 / absent 2, LANGUAGE 37; known to all four: 35 of 38", the two bpref absentees being `hvham`/`hvham2` and `sys_mapb` the one missing from typecheck/LANGUAGE).
- `Traps.lean:37-116`: 24 rows / 20 open / 4 closed, pinned by `#guard`. `tools/trap_census.py` (run 2026-09-13) prints `trap_rows: 23 total, 20 counted (3 excluded)`, `trap_zero_word: 17 of 20`, `trap_needs_words: 3 of 20`, `trap_unrep: 0/20`. The Lean table is a transcription and will drift; §4.5 derives it instead.
- `F_PHASE_STATUS.md:276-277` cites the rows at "line 188/189"; they are at 198/199 today.

## 3. On-box Lean, measured (this is the premise the row leaves open)

All runs: `/root/s30/outC4/lean/bin/lean` 4.33.1, one process, scratch copies, peak RSS sampled from `/proc/<pid>/status` every 100 ms, wall from `date +%s%N`. Box: 7 cores, 7.5 GB total, 2.3 GB available at the time (`free -m`).

| probe | rc | wall | peak RSS | note |
|---|---|---|---|---|
| 4-line file: `Int64` + `BitVec 64` `#eval` | 0 | 11.4 s | (not captured: the sampler watched `timeout`) | proves `Int64`/`BitVec` are in Init on 4.33.1 |
| `Basic.lean` verbatim minus `import Lean` | 1 | 6.9 s | 474 MB | `unterminated comment` (§2.1) |
| same, `MIN/-1` -> `MIN / -1` | 0 | 12.6 s | 535 MB | elaborates |
| same, `-o Basic.olean` | 0 | 15.7 s | 547 MB | produces the olean the next module needs |
| `Builtins.lean` against that olean | 1 | 10.0 s | 459 MB | stops on the cycle (§2.2) |

So a single module costs ~7-16 s and ~0.5 GB on this box under proot; seven modules built serially are of the order of two minutes and never more than one `lean` process plus `lake`. That is inside every cap the box has (3 GB per process; the phantom-process killer counts processes, not memory -- and `lake build -j1` is 2-3 processes). **The row's off-box premise is refuted for building.** What is NOT measured: the cost of RUNNING the conformance harness (§4.6 and §8), and `import Lean` (Basic.lean:13), which loads the whole frontend and was dropped for every probe because Init already provides `Int64`.

## 4. Design

### 4.1 One front end, not a fourth parser

`Conformance.lean:10` says the interpreter runs "WITHOUT a parser (inline AST construction)", and the README lists "parser not yet implemented". Writing a Bebop parser in Lean would be the language's fourth parser (`bebop.bp`, `tools/bpref.py:148 class Parser`, and `tools/typecheck.py:8` which imports bpref's). Instead: `tools/bpref.py` gains `--dump-ast <file.bp>` that prints the tree its parser already builds (node kinds `arr assign bin block call ctor field get if letin match neg not num set str var`, from `grep -o "return ('[a-z_]*'"` and the `Interp.ev` dispatch) as one S-expression after `expand_use` (`bpref.py:762`), and `formal/Bebop/Reader.lean` (~150 lines, estimate) turns that text into `Program`. The `.use` text is then what A25 §6 already designates as the one program text for all consumers (`docs/blueprints/A25-textual-rewriter.md` §6, §8: "formal/Bebop/Conformance.lean | run `.use` text through the semantics").

Cost of the choice, stated: bpref's parser becomes shared between two of the three executors, so a parse defect is shared. It is not shared with `bebop.bin`, which keeps its own parser, and `lean_conformance` compares against `bebop.bin`'s ANSWER (§4.2), so a shared parse defect still surfaces as a disagreement with the compiler. The A23 alternative -- generating a front end from the compiler's own tables -- is costed there at ~2,000 lines and "not scheduled"; this blueprint does not schedule it either.

### 4.2 Agreement, not EXPECT

`bench/vs_rust/bpref_parity.sh:11-13` states the rule the tree learned on 2026-09-13: "Not a golden: AGREEMENT. ... No EXPECT is read, so this lane cannot be satisfied by editing a table." `Conformance.lean:43-248` is exactly such a table, transcribed by hand (all 100 positive constructs carry `// EXPECT ... from: hand`, measured `grep -h "// EXPECT" | grep -o "from: *[a-z]*" | sort | uniq -c` -> `100 from: hand`). The gate here is therefore defined the way `bpref_parity` is: for each construct, `bebop.bin`'s printed value (or its exit code for `neg/`) versus the Lean evaluator's `Result`, with `unsupported` kept and named (§4.5), and the `positiveExpectations` table deleted rather than maintained.

### 4.3 The rules gap (what "whole language" means against LANGUAGE.md)

The evaluator covers the C-precedence expression tiers (`Semantics.lean:87-107` mirrors LANGUAGE.md:60-68 including `>>` logical / `>>>` arithmetic and non-short-circuit `&&`/`||`, the latter now MEASURED true of the compiler by the F2 row), let/let-in/compound/while/return/break statements, calls, arrays as arena cell indexes (A5 step 1b), struct and enum literals, field access. Missing or wrong against LANGUAGE.md:13-83 and the compiler:

| form | status in Lean | source of truth |
|---|---|---|
| `match` scrutinee | broken (§2.4) | LANGUAGE.md:80-81; `bebop.bp:1945 emit_match`, `:1958 emit_match_lit` |
| `str` values as `(off << 32) \| len` handles, `char`/`str_len`/`crc32b`/`scan` over real bytes | `builtinChar` "returns 0 for idx > 0" (F_PHASE_STATUS.md:117); no byte arena | A7 (ROADMAP A7 rows); `bpref.py` `self.bytes` (:744-747) |
| `kernel fn` (sys_ refused, exit 102), `unchecked fn`, `theorem` lines (F8 step 0, exit 110 via `scan_inert`, `bebop.bp:4554`), `test` blocks, `module m { }` | absent | `bench/parity_constructs/neg/c112_kernelsys.bp`, `c143_contract_garbage.bp` |
| static rejections 97/99/100/101/104 and F2's planned 105-108/115/116/118 | `Traps.lean:124-153` declares an inductive, no checker | `tools/trap_census.py` output; `bebop.bp:72 diag_exit` |
| frame-heap reset per `while` iteration (T43) | modelled at `Semantics.lean:517,524` by clearing `frameArrays` only | LANGUAGE.md:112-118 |
| capacity traps 80/82/89 | 80 modelled (`State.zeros`), 82/89 not modellable | counted `native-only` (§4.5) |

### 4.4 Builtins: 38, partitioned by what can be checked

`builtin_spec` counts a builtin as specified only when (a) it is executable in Lean AND agrees with `bebop.bin` on a native golden construct, or (b) it is an axiom with a declared footprint (`Basic.lean:204 Footprint`, `Syscalls.lean:61-317`). The row's "10 exec + 26 axiom = 36" is 38 today (`sys_mapb`, `crc32b` joined since the row was written). Executable candidates: `zeros str_len char clock_ms clz crc32 crc32x crc32b hvham hvham2 scan` (11); `sys_write`/`sys_exit` are the two syscalls bpref implements and the Lean side should too (stdout capture and exit code are what the gate compares). The remaining 25 stay axioms, with the 7 threading builtins (`Syscalls.lean:279-317`) outside the single-thread semantics as the row rules ("no theorem crosses a clone"). `dispatchSyscall`'s `(s, 0)` placeholder (`Syscalls.lean:385-396`) must become `none` -> `unsupported`, the way `bpref.py:748-750` was made to raise `UnsupportedForm` by A23 -- returning 0 for an unmodelled syscall is precisely the silent-oracle defect A23 removed from bpref.

Bound on what the semantics can answer, measured: 8 of 100 positive constructs and 36 of 133 `bench/vs_rust/std_tests/*.bp` call some `sys_*`; 2 and 8 respectively use `sys_clone`/`sys_run`/`sys_wait4`. So the reachable ceiling without a syscall model is 92/100 constructs and roughly 97/133 std programs; the "121 oracle programs" of the row are the Python oracles in `bench/oracles/` (128 today), which compute expected values for the std gates -- the programs themselves are the `std_tests`.

### 4.5 Negatives and unsupported forms, named not discarded

20 `neg/` constructs carry `EXPECT COMPILEFAIL:<code>` or `RUNFAIL:<code>` (`bench/vs_rust/construct_parity.sh:79-111`). A negative counts as conformant when the Lean static checker returns `.rejected code` with the same code from the AST. Codes that are capacity or register-model facts (80 arena, 82 stack, 89 register pressure) are reported `native-only:<why>` and excluded from the denominator, exactly as `bpref_parity` reports `unsupported` with its reason (`bpref_parity.sh:51-56`). The trap table is not transcribed into Lean again: `tools/lean_conformance.py` (§4.6) reads `tools/trap_census.py`'s output and checks that every code the Lean checker can emit is one the census knows -- the census stays the authority.

### 4.6 The results file, the hash binding, and the ratchet

Because §3 shows the build fits on-box, the primary path is on-box: a compiled harness (`lake exe conformance`, native via the toolchain's `leanc`/`clang` at `/root/s30/outC4/lean/bin`; **unmeasured**, §8) run behind `tools/slot.sh` like every other heavy job, writing `formal/results.txt` (new): one line per program `name <value|exit:N|unsupported:<why>>`, a header with `sha256` over `formal/**/*.lean` in sorted order and the toolchain's `lean --version` string. The row's off-box mechanism is kept as the fallback for a box that cannot run it: the file is committed, and the chain-side step refuses a file whose source hash does not match the tree.

`tools/lean_conformance.py` (new, ~70 lines, the row's estimate; `tools/check_words.py` at 69 lines is the size precedent) recomputes the comparison against `bebop.bin`'s answers produced by `bpref_parity.sh`'s own compile-and-run loop (`bpref_parity.sh:36-41`), prints `lean_conformance: c/N unsupported=u native_only=k` and `builtin_spec: b/38`, and holds a RATCHET on `c` the way `tools/f8_dt.py:24` holds `RATCHET_VALUE` -- a count that falls without the ratchet being lowered is RED. It is wired into `tools/battery.sh` as a `line` only once c > 0 with the ratchet at the real number; the F0/F1 precedent (ROADMAP.md:194-195) is to keep a script standalone while its number would be a false RED, never to wire a vacuous PASS (the F8 lesson, ROADMAP.md:202).

### 4.7 What `partial def` forecloses, and Theorems.lean

All four evaluator functions are `partial def` (`Semantics.lean:114,132,466,538`). `partial` makes them opaque to the kernel: nothing can be proved ABOUT `evalExpr`, so this semantics is, today, a third interpreter and not a proof object. The fuel argument is already threaded (`Basic.lean:246 Fuel := Nat`), so restating them as structural recursion on fuel is mechanical but not free: the `let rec go` loops inside (`:119,:248,:305,:390,:440,:506,:542`) must each become fuel-decreasing too. Costed at 1-2 weeks; NOT in this blueprint's schedule (§7) because no gate in the row needs it. It becomes necessary the day F7's self-verification ladder (ROADMAP.md:201) or any whole-language theorem is attempted.

`Theorems.lean` should be read for what it is: 7 `axiom`s whose statements are exercised by `#guard` on hand-picked values (`:115`, `:177`, `:372`). That is a test, and a useful one, but F9's gate (`theorems: >= 3 ... each with a kernel-checked term and an LRAT certificate`, ROADMAP.md:203) is not met by an axiom, and the F9 commit message (`8bc552a`: "7 theorems ... proofs deferred to F7 kernel") should not be quoted as theorems landed. This blueprint changes nothing in Theorems.lean; it records the state so the F9 row can be corrected.

## 5. Gates

| step | name | lands | gate | predicate |
|---|---|---|---|---|
| 0 | build | §2.1-2.3 fixed; `import Lean` dropped; toolchain pinned to the one on the box; `Memory` split | `lean_build: 8/8 modules, sorry=0, partial=4, axiom=26 (Syscalls) + 7 (Theorems)` printed by `tools/lean_conformance.py --build-only` | `lake build` rc=0 on-box behind `slot.sh`; the counts are re-derived by grep, not typed |
| 1 | reader | `bpref.py --dump-ast`; `formal/Bebop/Reader.lean` | `ast_roundtrip: 120/120` (every construct parsed by bpref and read by Lean without error; N derived from `ls`) | 120/120 |
| 2 | match + strings + statics | §4.3 rows closed | `lean_conformance: c/N unsupported=u native_only=k` with c ratcheted | c never falls; end state c + k = N |
| 3 | builtins | §4.4 | `builtin_spec: b/38` (denominator read from `tools/builtin_surface.py`, never typed) | b ratcheted; end 38/38 |
| 4 | results + hash | `formal/results.txt`; `tools/lean_conformance.py` in `battery.sh` | `lean_results_hash: match` (sha256 over sorted `formal/**/*.lean` + toolchain string) | RED on mismatch; RED if the file is older than any `.lean` it names |
| 5 | bpref diff | Lean vs bpref over constructs + std_tests + fuzz seeds via `bench/fuzz/fuzz_batch.py`'s forked bpref (`fuzz_batch.py:28`) | `bpref_diff: 0 over 120 + 133 + >= 10^4 seeds` | 0 disagreements; a disagreement names the program and both values, as `bpref_parity.sh:67` does |

## 6. Files and functions touched

| file:fn | change | anchor |
|---|---|---|
| `formal/Bebop/Basic.lean:Val.sdiv` docstring | `MIN/-1` -> `MIN / -1` (nested comment) | `Basic.lean:25` |
| `formal/Bebop/Basic.lean` | drop `import Lean`; receive `State.lookup/bind/zeros/arenaRead/arenaWrite` from Semantics | `Basic.lean:13`; `Semantics.lean:37-79` |
| `formal/Bebop/Builtins.lean`, `Syscalls.lean` | remove `import Bebop.Semantics` (cycle) | `Builtins.lean:9`, `Syscalls.lean:20` |
| `formal/Bebop/Syscalls.lean:dispatchSyscall` | `(s, 0)` placeholder -> `none` (unsupported), `sys_write`/`sys_exit` executable | `Syscalls.lean:385-395` |
| `formal/Bebop/Basic.lean:Expr.matchExpr`, `Semantics.lean` match arm | add the scrutinee; delete the `_scrutinee` lookup | `Basic.lean:88`, `Semantics.lean:421-459` |
| `formal/Bebop/Reader.lean` (new) | S-expression -> `Program` | none yet |
| `tools/bpref.py:Parser`, `main` | `--dump-ast` after `expand_use` | `bpref.py:148`, `:762`, `:784` |
| `formal/Bebop/Conformance.lean` | delete `positiveExpectations`/`negativeExpectations`/`oracleEntries` tables and the 5 inline samples; keep `runProgram`/`checkExpected` | `Conformance.lean:43, :218, :266, :524-534, :555-660` |
| `formal/Bebop/Traps.lean` | table derived from `tools/trap_census.py` output, not transcribed; `#guard` counts removed | `Traps.lean:37-116` |
| `formal/lean-toolchain`, `formal/README.md` | pin the toolchain actually run; delete "Compiles"/"cannot host" claims | `lean-toolchain:1`, `README.md:13,21-32` |
| `tools/lean_conformance.py` (new, ~70 lines) | chain-side comparison, hash check, ratchet | precedent `tools/check_words.py` (69 lines), `tools/f8_dt.py:24` |
| `tools/battery.sh` | one `line` for `lean_conformance`, one for `lean_results_hash` (only after step 2 has c > 0) | `battery.sh:57-70` (`line` calls) |
| `formal/harness.lean` | replaced by the `lake exe` driver | `harness.lean:1-11` |
| `docs/TRUST-CHAIN.md` | name the Lean toolchain + results file as UNTRUSTED producers (the row's own requirement) | `TRUST-CHAIN.md:158-172` names bpref; no Lean entry today |

No `bebop.bp` edit. No frozen artifact changes. No chain token beyond the slot the on-box `lake build` takes.

## 7. Cost and schedule (the row says 8-12 weeks; this agrees, and says why)

| step | estimate | reasoning |
|---|---|---|
| 0 build | 2-3 days | three mechanical fixes (§2), one module split, one pin; each verified by a 15 s on-box run |
| 1 reader | 1 week | bpref's parser exists; the Lean reader is a recursive descent over one S-expression form; 120 constructs are the test |
| 2 rules gap | 3-4 weeks | the §4.3 table: `match`, byte arena for `str`, five surface forms, the static checker (~300 lines of Bebop logic restated, the research's own estimate at `docs/RESEARCH-VERIFICATION-2026-09-09.md:131`) |
| 3 builtins | 1-2 weeks | 11 executable, each against a native golden construct; footprints for 25 |
| 4 results + chain step | 1-2 weeks | includes the FIRST measurement of a compiled Lean executable on this box (§8) |
| 5 bpref diff | 1 week | `fuzz_batch.py` already forks bpref per seed; the Lean side is one more subprocess per seed IF the executable starts in milliseconds (§8) |
| **total** | **8-11 weeks** | |

Calibration, from `docs/RESEARCH-VERIFICATION-2026-09-09.md:271`: the tree's 1-row-per-lane-day velocity "does NOT transfer to F3 ... their bottleneck is a proof assistant". The measured 7-16 s per module means every edit costs a quarter-minute to check, not the sub-second loop `bebop.bp` lanes have.

**What must land first:** step 0. Until `lake build` exits 0 on-box, every number is 0 and every claim about `formal/` is unverifiable -- the README's "Compiles" was written without a build, and this blueprint would be the same kind of document if it scheduled step 2 before step 0.

**What is NOT proposed here:** a Lean parser for Bebop; a semantics of Linux (the 25 axiomatised syscalls stay axioms); any theorem about programs (F9) or about the evaluator (needs §4.7, costed 1-2 weeks, unscheduled); a verified compiler; Mathlib; generating bpref from the compiler's tables (A23, unscheduled); any change to `bebop.bp`; wiring `lean_conformance` into the battery while it would be a vacuous PASS or a false RED.

## 8. Speculative, labelled

- **Compiled harness start-up and per-program cost** (§4.6, step 5): unmeasured. If `lake exe` cannot be produced on-box (the `4286ec1` commit title says "F4 clang fixes", which suggests someone hit the C backend and left no measurement), the fallback is `lean --run` at ~10 s per invocation, which makes 10^4 seeds ~28 h and forces the fuzz differential off-box or batched (one process, many programs). The step-5 gate is stated at 10^4 seeds because the row says so; the mechanism that reaches it is not yet chosen.
- **`Int64` absence in v4.12.0** (§2.3): from memory, not verified on this box; irrelevant once the pin follows the toolchain actually run.
- **Reader size ~150 lines**: estimate.
- **Peak RSS under `lake`** (as opposed to bare `lean`): not measured; expected within 10 % of the bare numbers.
- **Fragment of std_tests answerable without syscalls (~97/133)**: an upper bound from a `grep -l sys_`, not a run.

## 9. Stale or contradicted in the citing row (ROADMAP.md:198), for the operator

- "runs OFF-BOX ... the box cannot host Lean under its 3 GB/32-process caps -- unmeasured": measured now; a module elaborates in 7-16 s at ~0.5 GB (§3). Off-box stays the fallback, not the premise.
- "86 constructs", "121 oracle programs", "36 builtins (10 exec + 26 axiom)": 120 constructs, 128 oracle scripts / 133 std programs, 38 builtins today (§2.5).
- "conformance harness runs the 86 constructs": the harness runs 5 inline-AST samples (`harness.lean:54-76`), one of which encodes the wrong program (§2.4).
- "no bebop.bp edit": still true of this blueprint.
- `Semantics.lean` "607 lines": exact.

## 10. Attribution and evidence

- Citing row: ROADMAP.md:198 (F4); neighbours read in full: :108 (A23), :191 (Phase F header, Lean demoted), :194 (F0), :195 (F1), :199 (F5), :200 (F6), :201 (F7), :202 (F8), :203 (F9).
- Research source: `docs/RESEARCH-VERIFICATION-2026-09-09.md` §5.1-5.4 (:98-141), §10 ladder (:252-274, F3 rung at :261), §14 "Not determined here: ... whether Lean 4 checks within the box's 3 GB cap" (:309).
- House rules applied: agreement not EXPECT (`bench/vs_rust/bpref_parity.sh:11-18`); ratchet in the script (`tools/f8_dt.py:24`); hold a gate out rather than wire a false green (ROADMAP.md:194-195); cited files must exist (`tools/arch_check.py:268-290`, ratchet `tools/arch_ratchet.txt` `max_missing_citations = 20`).
- Measurements: `lean --version`; five `lean` probes (§3) on scratch copies under `/tmp/claude-0/.../scratchpad/probe`; `python3 tools/builtin_surface.py`; `python3 tools/trap_census.py`; `grep -c` for `sorry`/`axiom`/`partial def`/`#guard`; `git log`/`git cat-file -t` read-only against `/root/dowiz`.
- Layout facts used: `.bin` = words + LE64 entry footer (`seed/seed.S:2`); `bebop.bin` `072c01d1` = 44,970 words in file, code_end 44,020 (`tools/check_abi.py:62 load_bin`).
- Not verified here: the internals of `tools/certgen.py` beyond its use of the toolchain's `cadical`; any Lean 4.12.0 behaviour.
