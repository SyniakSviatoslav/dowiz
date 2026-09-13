# CRITICAL-PATH AUDIT — the eight TG-DONE criteria (2026-09-13)

Lane `critpath`, base commit `4a188ff`. Read-only: no code changed, no heavy job run, one
compile attempted and abandoned (see METHOD). Every `file:line` below was printed before it
was written down.

The question asked of each row, following the battery audit of the same day:

> **Does this criterion's stated gate actually measure the criterion — and who computes the number?**

Ground truth established first, because six of the eight rows depend on it:

| fact | value | how established |
|---|---|---|
| project root | `/root/dowiz/bebop-lang` (NOT `/root/dowiz`, which is the git root) | `git rev-parse --show-toplevel` |
| promoted compiler | `bebop.bin` md5-8 = **`072c01d1`** | `md5sum bebop.bin` |
| its source | `bebop.bp` md5-8 = **`c6bcd3a2`** | `md5sum bebop.bp` |
| manifest row | `072c01d1  3ddf332+dirty  c6bcd3a2  2026-09-13  yes` | `compilers/MANIFEST.tsv` |
| git history | **SHALLOW**, 240 commits, earliest 2026-09-06 | `git rev-parse --is-shallow-repository` |

**A correction to the audit brief, made before anything was concluded from it.** Four of the
"commits" cited in the eight rows — `94e47998` (row 1), `c3f58e8e` (row 2), `08cd9e46` (row 4),
`d785e062` / `e14dd55e` (row 8) — do **not** resolve via `git cat-file -t`. They are **not**
missing commits. They are **md5-8 digests of `bebop.bin`**: `honest.sh:36` prints
`bebop.bin {md5}` with `[:8]`, `chain.sh:34-35` prints `gen2/gen3/gen4 $(md5sum | cut -c1-8)`,
`fuzz.sh:65` prints `bin=%s` the same way, and `tools/hooks/pre-commit` names `08cd9e46` as a
*binary* ("08cd9e46 shipped against a source that produced 2822d7b5"). `94e47998` also appears
in `docs/exp.journal` as a `bin=` key with 1,500 fuzz seeds. So the rows are not citing
phantoms — they are citing **artifacts**, and the real question is whether those artifacts are
still the ones in the tree. Mostly they are not.

---

## The eight rows

| # | criterion / claimed number | named gate — EXISTS? | who computes the number (verified) | WIRED? | last actually MEASURED | can it be green while the criterion is false? |
|---|---|---|---|---|---|---|
| **1** | honest kernels vs Rust twin. `2026-09-06 (REPORT-honest.md, 94e47998, REPS=100): K1H 1.8x, K2H 3.3x, K3H 3.4x, K4 1.6x` | `bench/vs_rust/honest.sh` **EXISTS** (63 lines). But the row's sibling artifact `REPORT-honest.md` is at `bench/vs_rust/`, **not** `docs/` | an inline `python3` heredoc, `honest.sh:22-63`. Ratio at `:48`; the verdict string `"MET" if ratio <= 2.0 else "UNMET"` at `:51` | **NO.** `grep -l honest.sh tools/battery.sh tools/chain.sh tools/std_par.sh bench/vs_rust/std_golden.sh` → empty. Nothing runs it | **2026-09-06**, on `bebop.bin 94e47998`. Last commit touching `REPORT-honest.md` is `0f29018`, 2026-09-07. Promoted binary is now `072c01d1` — **~13 promotions later** (`MANIFEST.tsv`) | **YES, and it is.** The gate has no exit code — the heredoc's status is discarded and `honest.sh` always exits 0 — so "≤ 2.0x" is enforced by nobody. Worse, the row transcribes the **oldest** section of its own artifact: `REPORT-honest.md` already carries newer rows (`:31` B1 `1a3b2cc2` K2H 2.6x/K3H 2.4x; `:131` A2 commit 2 `9694b780`, 2026-09-07) that the ROADMAP never picked up. The row also silently drops **K8H**, which `honest.sh:33` measures |
| **2** | linear self-hosting fixpoint, `gen3 == gen4 c3f58e8e (2026-09-06)` | "every codegen commit (AGENTS law)" — the real gate is `tools/chain.sh:43-47` | `chain.sh:43` `m3`/`m4` = `md5sum \| cut -c1-8` of `gen3.bin`/`gen4.bin`; the verdict is `:44` `[ -n "$m4" ] && [ "$m3" = "$m4" ]` | **YES**, and it is the best-wired row of the eight: `chain.sh` runs it, `pre-commit` refuses a `bebop.bp` staged without `bebop.bin` and refuses a `bebop.bin` with no `MANIFEST.tsv` row, and `arch_check.py:82-99` **artifact-identity** re-compiles and compares — reached from `invariants.sh:155`, which `battery.sh:47` runs and `battery.sh:69` asserts GREEN | **TODAY.** `MANIFEST.tsv` row `072c01d1 … c6bcd3a2 … yes`, and I re-derived `md5sum bebop.bp` = `c6bcd3a2`. The shipped binary **is** this source's binary | **Green while the compiler is WRONG: YES, by construction.** See §2 below — this is not a defect, it is what a fixpoint is. Green while the *promoted artifact* is not the source's: **no longer**, that hole was named (`AGENTS.md:300`) and closed. Two residual escape hatches: `arch_check.py:89` returns a **`note`** (not a `fail`) when `seed/build/seed` is absent, and `chain.sh:35` runs `gen()` inside `( … ) &`, so its `exit 1` leaves the subshell only — recovered at `:44` by the `-n "$m4"` guard, which is correct but load-bearing and undocumented |
| **3** | one oracle per gate, none self-frozen. `ok=117 self-frozen=0` | `bench/oracles/run_all.sh` **EXISTS** (41 lines) | **Two different computers, and the row conflates them.** (a) The *published* `ok=117` is **not** run_all's output — the row says so itself: "`grep -c '^gate ' bench/vs_rust/std_golden.sh` = 117". I re-derived it: 117. That is a count of **gate definitions in a shell script**. (b) run_all's own `ok=` is `run_all.sh:29` `grep -c ' OK$'`, comparing each oracle's stdout to the frozen literal at `std_golden.sh` — **neither side touches the compiler** (`run_all.sh:2` says so: "oracle == frozen"). (c) The "every gate has an oracle" half is `arch_check.py:371-383` `check_gate_has_oracle` — a real ratcheted `fail` at `:378`, ratchet 0 (absent from `arch_ratchet.txt`, so `r.get(…, 0)`). I verified: **0 of 117 gates lack an oracle `.py`** | (b) and (c) **YES** — `battery.sh:44` runs run_all, `:63` asserts the SUMMARY; `arch_check` via `invariants.sh:155`. (a) is checked only by `tools/roadmap_check.sh:22`, which is **`grep -q "ok=$g" ROADMAP.md`** — a string-presence test on a markdown file, and `roadmap_check.sh` is itself wired into **nothing** | The gate-count half: re-derived **2026-09-12**. The oracle half: **replayed, not run** — see the payload | **YES — DEMONSTRATED, and this is the sharpest live hole in the eight.** See F1 below. `run_all.sh:12-16` memoises on `KEY = sha256(all oracle .py + the frozen list)` (`:13`). That key contains **nothing about the environment, the toolchain, or the tree**. I computed it in both trees: lane `add93d1555c1a4fd` == main `add93d1555c1a4fd`, and the memo file exists. Running it in this lane took **300 ms** and printed `ok=117 … missing=0` including `money` and `ordfsm`, whose Rust binaries **do not exist in this tree at all**. `battery.sh:63`'s regex is unanchored at the end, so the appended `(memo: …)` marker matches — the battery **cannot distinguish a measurement from a replay** |
| **4** | zero tolerated miscompiles; zero-arg-builtin miscompile CLOSED "on the PROMOTED compiler `08cd9e46`"; construct parity 104/104 | `construct_parity` **EXISTS** (`bench/vs_rust/construct_parity.sh`); `fuzz` — see row 8 | `construct_parity.sh` tail: `PASS`/`FAIL` counters against each construct's `// EXPECT:` line; `:end-3` now prints `NOT MEASURED` at 0 constructs | construct_parity **YES** (`battery.sh:41`, asserted `:59`). fuzz **NO** | construct parity: every battery. **But `08cd9e46` is not the promoted compiler and never was a chained one**: `MANIFEST.tsv` records it `fixpoint = no`, "UNCHAINED by `05fcc0f`, which moved bebop.bp … without re-promoting". Current binary is `072c01d1` | **YES, three ways.** (i) The row's evidence is pinned to a **repudiated binary**. (ii) The three guards' EXPECTs are sourced "from `tools/bpref.py`" (`c98`/`c99` headers) and bpref models **every** `sys_*` as 0 — harmless here only because the author deliberately made `c98`/`c99` address-independent, which is good design but means the guard tests the *shape*, not the *value*. (iii) `tools/mutation_coverage.txt`: **10 of 32 compiler emitters SURVIVED mutation** — "nothing did". `emit_hvham`, `emit_hvham2`, `emit_sys_read`, `emit_sys_write`, `emit_sys_arena_end`, `emit_sys_futex_wait_guard`, `emit_sys_exit_thread_guard`, `emit_sys_setaffinity`, `emit_sys_msync`. Ten emitters can be corrupted and the whole battery stays green |
| **5** | single compiler, single language. `expr_compile.bp in attic; 38 constructs` | **`attic` DOES NOT EXIST.** `ls attic` → no such file. The file is `selfhost/attic/expr_compile.bp`. `ROADMAP.md:169` (row D5) already says so: "the row pointed at the wrong path" | **Nobody.** "attic" is a *directory*, not a gate — there is no script, no function, no binary that computes a number for this row. `arch_check.py:36` merely *skips* `/attic/` when scanning. The only mechanical half is `construct_parity` | half **YES** (construct_parity), half **N/A** (no instrument exists) | The "38 constructs" number: **stale by ~3x**. `ls bench/parity_constructs/*.bp` = **100**, plus `neg/` = 20; row 4 of the same table says **104**. Three numbers for one quantity in one document | **YES — this row cannot go red.** A criterion whose gate is a directory listing has no failure mode. "Single compiler, single language" is not measured by anything: nothing counts languages, nothing asserts `expr_compile.bp` stays retired, nothing notices if a second front-end lands |
| **6** | hardware claims measured, never projected. `no projected row remains` | `bench_pinned.sh` — **the path as written does not exist**; it is `bench/vs_rust/bench_pinned.sh`. `REPORT-pinned.md` likewise is at `bench/vs_rust/`, not `docs/` | **Nobody.** `grep -rn 'projected\|PROJECTED' tools/*.py tools/*.sh` → **zero hits**. No script inspects the Measured table for projected rows. `bench_pinned.sh` itself computes timings honestly (it verifies `taskset` via `Cpus_allowed_list` and prints the result, `:24-26`) but asserts nothing | **NO.** `bench_pinned.sh` appears in no runner | `REPORT-pinned.md:1` — **"Status: 2026-09-04 CURRENT"**, `bebop.bin md5 104b629124d4e243642137ff73b37154`. Nine days old, and on a binary that predates the register model, csel, constant hoisting and 13 promotions | **YES.** The claim is self-certifying prose. And the ROADMAP's own **Measured table** (`:239-262`) contradicts the artifacts it cites: its sbench block reads `insert 880 ms / PK 450 ns / scan 2.7 us`, while `bench/vs_rust/RESULT-sbench.md` (2026-09-07, bin `e9159318`) reads **`400 ms / 190 ns / 1200 ns`**. Every row differs by ~2x |
| **7** | the store. `G1-G6 green in std_golden (99 gates …); G7 sbench 17x insert, 450 ns PK, 30x window scan, 2.5x size loss; G8 stage 1 … stage 2 running` | **"G1-G8" is not a gate name.** No gate in `std_golden.sh` is called `g1`…`g8`; I listed all 117. The G-labels are ROADMAP prose mapping informally onto `store`, `slayout`, `sround`, `sround_reopen`, `scompact`, `schain`, `scrash`, `scrash_torn`, `smw`, `sevolve`, `sconc`, `sdiag`, `srepl`. `sbench.sh` (G7) **exists** at `bench/vs_rust/sbench.sh` | the store gates: `std_golden.sh` folds via `std_par.sh`. **G7: `sbench.sh`, and its numbers are transcribed by hand** into the ROADMAP | store gates **YES** (`battery.sh:40` → `std_par.sh`, asserted `:58`). **`sbench.sh`: wired into nothing** | **G7 is stale in all four numbers**, against its own named artifact: `RESULT-sbench.md` gives insert **24.4x** (row says 17x), PK **190 ns** (row says 450), window scan **42.8x** (row says 30x), size after compaction **~2.1x** (row says 2.5x). "99 gates" is stale against 117. "G8 stage 2 running" is a **process status from 2026-09-05** with no resolution in the row | **YES, and it has been caught once already inside this same document.** `ROADMAP.md:138` (row C2, 2026-09-09) states: *"'today's 450 ns' is stale by 2.6x — the sbench PK row re-derived on this box is **170 ns** … and 450 made the regression guard nearly three times too permissive."* The TG-DONE table **still says 450 ns**, four days later. A correction was published in the same file and never propagated to the row that decides the thesis |
| **8** | fuzz at scale. target `>= 10^5 seeds ON THE PROMOTED BINARY (docs/PERF.md fuzz_seeds_on_bin, keyed by md5; 28.5k on d785e062, e14dd55e counting)`, `0 CRASH/DIVERGE`, `TRAP-82 = 0` | `bench/fuzz/fuzz.sh` **EXISTS** (70 lines) | `perf.py:295-318` `fuzz()`: parses `docs/exp.journal` for `H:fuzzd batch` lines, buckets seeds **by `bin=` md5**, then `:312` `d = per.get(cur, {…zeros…})` and `:314` records `fuzz_seeds_on_bin` | **NO** for `fuzz.sh` (it is in no runner). `perf.py fuzz` is reached from `chain.sh:55`. The background shield `fuzzd` is **not running** (no `pid`, no `next` seed file) and its enforcement hook `tools/hooks/pre-push` — the one that refuses a push while `ALERT` exists — is **not installed**; only `pre-commit` is in `.git/hooks/` | I replayed `perf.py`'s own arithmetic over `docs/exp.journal`: **48,100 seeds total across 9 binaries** — `d785e062` 28,500, `e14dd55e` 7,000, … `94e47998` 1,500. Seeds on the **promoted** `072c01d1`: **ZERO.** `docs/PERF.md:58` confirms it in every column: `fuzz_seeds_on_bin \| seeds \| 0 \| 0 \| … \| 0 -> 0` | **YES — LIVE, and it is green right now.** `perf.py:312`'s default dict supplies `seeds=0, trap82=0` for an unfuzzed binary and `:314-317` record them with `st = {"valid": 1}`. So `fuzz_trap82 = 0` — the row's **ALERT-class** metric — reads "zero SIGSEGV tolerated" when the truth is "no fuzzing has ever run on this compiler". An ALERT that fires on `> 0` can never fire on a metric that is 0 because nothing measured it. Separately: `fuzz.sh` has **no pass/fail exit status at all** — its last statement is a pipeline ending in `head -40` (`:70`), which returns 0 unconditionally |

---

## §2 — Criterion 2, asked properly: what does a fixpoint prove?

The brief asked for a plain answer. Here it is.

**`gen3 == gen4` proves reproducibility, not correctness.** It says: the compiler is a fixed
point of its own compilation. A miscompilation that is *self-consistent* — one the compiler
applies to its own source and to the test programs identically — reproduces itself perfectly
and the fixpoint closes on it. This is Thompson's construction, and no fixpoint survives it.

**This is not my inference; the tree already knows it and wrote it down twice.** I found both
while chasing the row:

- `ROADMAP.md:169` (row D5): *"`gen3 == gen4` is a reproducibility gate, not a trust gate:
  Thompson survives any fixpoint by construction."*
- `AGENTS.md:300`: *"Note `chain.sh`'s fixpoint does NOT catch this: it proves the SOURCE has a
  fixpoint, not that the promoted binary IS it."*

**Is there a gate that would catch a change that is self-consistent and wrong?** Yes — but only
three, and each has a named weakness:

1. **`std_golden.sh`'s frozen folds.** The compiler's output is compared to a literal in a
   shell script. Independent *iff* that literal was not produced by the compiler. That is
   exactly what criterion 3's `self-frozen=0` protects, and `arch_check.py:378` enforces
   (ratchet 0, currently 0 violations — verified). This is the real defence, and it is sound
   **provided the oracles actually execute** — which F1 shows they need not.
2. **`bpref_parity.sh`** — genuine differential agreement between the compiler and a Python
   evaluator, reading no EXPECT table ("cannot be satisfied by editing a table", `:12`). The
   strongest instrument in the tree. Wired at `battery.sh:45` — **as of 2026-09-13**; its own
   header records that bpref's evaluator "ran in no battery lane until" that day.
3. **`kcheck.py`'s `kernel_neg_bin`** (`battery.sh:66`) — the kernel binary's own acceptance,
   added after the Python twin's line stayed green through two soundness holes.

And one gate that **cannot** catch it, despite its name: `run_all.sh` compares *oracle to frozen
literal*. The compiler is on neither side. That is stated honestly at `run_all.sh:2`, but the
ROADMAP's placement of it as the "one oracle per gate" evidence invites the opposite reading.

**Verdict on row 2: sound today, and the best-defended of the eight.** The two historical holes
(promoted ≠ source; no provenance record) were found, named, and closed with `MANIFEST.tsv` +
`pre-commit` + `arch_check` artifact-identity. I verified the current state end-to-end:
`bebop.bin` `072c01d1`, `bebop.bp` `c6bcd3a2`, manifest row `fixpoint = yes`. What the row
should stop claiming is that this proves correctness. It proves *identity*.

---

## Findings, ranked by what they could hide

### SOUNDNESS — could hide a wrong answer from the compiler or the reference

**F1 — `run_all.sh`'s memo makes criterion 3 unfalsifiable in a lane tree. LIVE. Demonstrated.**

The memo key (`run_all.sh:13`) is `sha256(cat bench/oracles/*.py ; ls *.self-frozen ; frozen list)`.
It contains nothing about the machine, the toolchain, the compiler, or any **data file an
oracle reads** (12 oracles call `open()`). The memo lives at `$HOME/.cache/bebop/run_all` — one
**global** cache shared by every lane worktree. So a GREEN written from the main tree replays
verbatim in a tree where the oracles could not possibly run.

Exact demonstration, safe to re-run (it is a cache hit, ~300 ms, executes no oracle):

```sh
cd /root/dowiz/.claude/lanes/critpath
ls bench/oracles/rust/target/release/money        # -> No such file or directory
time bash bench/oracles/run_all.sh | tail -1
# SUMMARY ok=117 self-frozen=0 mismatch=0 missing=0 (memo: inputs unchanged since the last GREEN run)
grep -E '^(money|ordfsm) ' <(bash bench/oracles/run_all.sh)
# money   872656672063013 872656672063013 OK
# ordfsm  346243789026198 346243789026198 OK
```

`money.py` and `ordfsm.py` are `subprocess.run(["cargo","run","--release",…])` wrappers over
production Rust. In this tree there is no `target/`; a from-scratch build does not finish
(I aborted at 60 s). They are reported OK anyway. And `battery.sh:63`'s regex
`^SUMMARY ok=[1-9][0-9]* self-frozen=0 mismatch=0 missing=0` is unanchored at the end, so the
`(memo: …)` suffix matches: **the battery has no way to tell a measurement from a replay.**

Why this is soundness and not hygiene: criterion 3 is the tree's *honesty floor* — "every gate
has an oracle and a number that agrees with itself is not evidence". Under the memo, 117 gates'
worth of oracle evidence can be produced by a cache lookup. The two cargo oracles are the ones
that matter most, because they are the only gates whose oracle is **production Rust** rather
than a Python re-implementation.

*Extension of the brief's lead:* the lead said `money`/`ordfsm` "were not exercised in three
consecutive battery runs". I can sharpen and partly correct that — in the **main** tree they
*did* run at 2026-09-13 15:05 (`bench/oracles/rust/target/release/money.d` mtime matches the
memo file `add93d1555c1a4fd` exactly). The defect is not that they never run; it is that
**whether they ran is not observable from the gate's output**, and in any lane tree they
provably did not.

*What "one oracle per gate, none self-frozen" actually guarantees under a memo:* that at some
past instant, on some machine, in some tree, each of 117 gates had a file named
`bench/oracles/<gate>.py` whose last line matched a literal. It guarantees nothing about the
current environment, and `arch_check`'s half only checks the file **exists** — an oracle whose
body is `print(42)` satisfies it.

---

**F2 — `tools/bpref.py:748` gives every `sys_*` name the value 0, including names that do not exist. LIVE.**

```
748:        if name == 'clock_ms' or name.startswith('sys_'):
749:            return 0
```

Verified and **extended beyond the brief's lead**: the hole is scoped precisely to the `sys_`
prefix, and it is a *wildcard*, not a stub list.

```sh
cd /root/dowiz/.claude/lanes/critpath
printf 'fn main() -> i64 { sys_this_does_not_exist(1,2) + 5 }\n' > /tmp/ghost.bp
python3 tools/bpref.py /tmp/ghost.bp;  echo "rc=$?"   # -> 5   rc=0
printf 'fn main() -> i64 { totally_bogus(1) + 5 }\n'   > /tmp/bog.bp
python3 tools/bpref.py /tmp/bog.bp;    echo "rc=$?"   # -> UNSUPPORTED:builtin totally_bogus   rc=3
```

An unknown builtin **correctly** raises `UnsupportedForm` (`:750`) — unless it starts with
`sys_`, in which case the reference semantics silently fabricates 0 at rc=0.

**Which criteria lean on bpref:** 4 (construct EXPECTs — `c98`/`c99` headers say verbatim
"from `tools/bpref.py`"), 8 (it is the fuzz DIVERGE judge, `fuzz.sh:2`), and the differential
gate `bpref_parity` that criterion 2's defence rests on (§2 item 2).

**The consequence nobody has written down.** `bpref` models the *real* `sys_arena_base()` as 0
too — verified, it prints `0`. The zero-arg miscompile that row 4 closed was
`str_len("ab") - sys_arena_base()` returning 0 instead of **-8205**, i.e. the true value is
~8207. So on that exact expression bpref's answer is neither the correct one nor the buggy one.
bpref **could not have adjudicated the defect row 4 is proudest of**, and cannot adjudicate the
next one in that family. `ROADMAP.md:169` (D5) independently records the same stub at its old
line number — *"`bpref.py:659` stubs every `sys_*` to 0"* — as a reason bpref cannot serve as a
DDC witness. Nobody connected that to bpref's role as the DIVERGE judge.

**And it degrades silently on growth.** Every new `sys_*` builtin added to the compiler is
*automatically* modelled as 0 by the reference, with no `UNSUPPORTED`, no error, no gate. The
oracle gets quietly wronger each time the language grows.

---

**F3 — Ten compiler emitters survive mutation: the battery does not see them. Severity: soundness.**

`tools/mutation_coverage.txt`, header: *"KILLED = some gate noticed; SURVIVED = nothing did."*
22 KILLED, **10 SURVIVED**:

```
emit_hvham  emit_hvham2  emit_sys_read  emit_sys_write  emit_sys_arena_end
emit_sys_futex_wait_guard  emit_sys_exit_thread_guard  emit_sys_setaffinity  emit_sys_msync
```

Criterion 4 is "zero tolerated miscompiles". For these ten emitters the battery would not
**detect** a miscompile, so "zero tolerated" is not a measurement over them — it is silence.
Note `emit_sys_write` and `emit_sys_msync` in that list: the store's durability argument
(criterion 7) is built on `msync` ordering.

*Honest caveat, and it matters.* I ordered the commits: the sweep was recorded in `05fcc0f`
(2026-09-12 **20:16**), and `c98`/`c99` — the arena-operand guards — landed in `132063b`
(2026-09-12 **23:00**). So the `emit_sys_arena_end SURVIVED` verdict **predates its guard** and
may now be stale in the favourable direction. The other nine have no such excuse that I found.
`mutate_gate.sh` is wired into nothing and its header says `exit 0 always`, so nothing re-runs
this.

*And the instrument itself is partial* — recorded in the file's own footer:
*"`sites()` mutates only the FIRST literal `em(insns, n, N)` per emitter … the word that
actually publishes is `4177526819` = `str x3,[x1]` … and has NEVER been mutated."* So KILLED
means "its first emitted word is covered", not "the emitter is covered". Both verdicts are
weaker than the column headings suggest.

---

### CORRECTNESS — could hide a wrong number, not a wrong answer

**F4 — Criterion 8's headline is 0 on the binary that ships, and the metric that should say so is the one that hides it. LIVE.**

Replaying `perf.py:295-311`'s own parser over `docs/exp.journal`:

```
d785e062  28500 seeds      e9159318   3000      deef28e0   1000
e14dd55e   7000            405f33a8   2500      05aa91ac   1000
9017813e   3100            94e47998   1500      70cddb59    500
TOTAL: 9 bins, 48,100 seeds.      Promoted binary 072c01d1:  0 seeds.
```

`docs/PERF.md:58` agrees in every column: `fuzz_seeds_on_bin | seeds | 0 | 0 | … | 0 -> 0`.

The mechanism is `perf.py:312`:

```python
cur = md5(binpath); d = per.get(cur, {"seeds": 0, "bad": 0, "trap": 0, "trap82": 0, "rates": [0.0]})
```

A binary nothing has fuzzed falls to the default dict, and `:314-317` record those zeros with
`st = {"valid": 1}`. The row's ALERT-class metric `fuzz_trap82` therefore reads **0** —
indistinguishable from "fuzzed hard, no SIGSEGV". An alert threshold of `> 0` is unreachable
from a metric that is 0 for want of measurement. Textbook vacuous success.

Compounding it: `fuzzd` is **not running** (`~/.cache/bebop/fuzzd` has no `pid`, no `next`), and
`tools/hooks/pre-push` — the hook that refuses a push while `ALERT` exists — is **not installed**
in `.git/hooks/` (only `pre-commit` is). And `fuzz.sh` has no exit status: `:70` ends in a
pipeline terminating in `head -40`, which always returns 0.

Also worth saying plainly: 48,100 seeds across **all** binaries is under half the 10^5 target,
and the target is per-binary.

---

**F5 — Criterion 7's G7 cell carries a number its own document corrected four days ago.**

TG-DONE row 7: `17x insert, 450 ns PK lookup, 30x window scan, 2.5x size loss`.
`bench/vs_rust/RESULT-sbench.md` (2026-09-07, bin `e9159318`): `24.4x, 190 ns, 42.8x, ~2.1x`.
**All four differ.** And `ROADMAP.md:138` (row C2, 2026-09-09) already published the correction:

> *"'today's 450 ns' is stale by 2.6x — the sbench PK row re-derived on this box is 170 ns
> (RESULT-sbench.md already said 190 on 2026-09-07), and 450 made the regression guard nearly
> three times too permissive."*

The critical-path table still says 450 ns. The same stale block also sits in the Measured table
(`ROADMAP.md:256-262`), which is criterion **6**'s evidence — so one un-propagated correction
falsifies two rows. `sbench.sh` is wired into nothing, so nothing would ever re-derive it.

---

**F6 — Criteria 1 and 6 are measured by scripts that no runner invokes and that cannot fail.**

`honest.sh`, `bench_pinned.sh`, `sbench.sh`, `sgraph.sh`: none appears in `battery.sh`,
`chain.sh`, `std_par.sh` or `std_golden.sh` (only `sgraph2.sh` is, via `std_golden.sh`).
`honest.sh` computes `"MET" if ratio <= 2.0 else "UNMET"` at `:51` and then throws the verdict
away — its exit status is the heredoc's, and nothing reads the text. `REPORT-pinned.md:1` says
*"Status: 2026-09-04 CURRENT"* on `bebop.bin 104b6291`; the tree ships `072c01d1`.

Criterion 6's claim "no projected row remains" is computed by **nobody**:
`grep -rn 'projected\|PROJECTED' tools/*.py tools/*.sh` returns zero hits.

---

### COSMETIC — wrong text, no hidden defect

**F7 — Three cited paths do not exist as written.** Row 6 cites `bench_pinned.sh` and
`REPORT-pinned.md`; both live under `bench/vs_rust/`. Row 5 cites `attic`; `ls attic` fails —
it is `selfhost/attic/`. The tree already ratchets this class:
`arch_ratchet.txt` `max_missing_citations = 20`, annotated *"A citation that points at nothing
reads as evidence."* These three belong in that count.

**F8 — Row 5's "38 constructs" is stale by ~3x** (actual: 100 positive + 20 neg; row 4 of the
same table says 104). **Row 7's "99 gates"** is stale against 117. **Row 7's "G8 … stage 2
running"** is a process status from 2026-09-05 carried for eight days with no resolution.

---

## A fifth pattern

The four named patterns (wrong computer / unreachable instrument / stale contract / vacuous
success) cover most of what I found. F1 and F3 need a fifth:

> **5. Cached or partial verdict, reported as a whole-set measurement.**
> The instrument is correct, wired, and did run — *once, elsewhere, or over a subset* — and the
> gate reprints that verdict as though it were this run, over everything.

It is distinct from *unreachable instrument* (the code there really does run) and from *vacuous
success* (there really was a measurement). What is falsified is the verdict's **scope**:

- `run_all.sh`'s memo replays a GREEN from another tree and another day, and the summary line
  cannot be distinguished from a fresh one (F1);
- `mutation_coverage.txt` reports per-emitter verdicts derived from mutating only the **first**
  `em()` literal in each emitter, and the column header says KILLED/SURVIVED unqualified (F3);
- `perf.py`'s `fuzz_seeds_on_bin` reports a per-binary figure computed from a **journal**, so
  "0" means "nothing in the log matched this md5", not "measured zero" (F4).

The diagnostic question that separates it from the other four: *if this gate had never run in
this environment, would its output differ?* For all three, no.

---

## What I could NOT determine, and what I would need

1. **Whether the nine other mutation survivors are still survivors.** I established only that
   `emit_sys_arena_end`'s verdict predates its `c99` guard by 2h44m. *Needs:* a re-run of
   `tools/mutate_compiler.py` / `mutate_gate.sh` on `072c01d1` — a heavy job, barred in this
   lane. Until then F3 is "≤ 10 survivors", not "10".

2. **Whether `money`/`ordfsm` can complete inside `run_all.sh`'s `timeout 300` in a cold tree.**
   I aborted at 60 s. If they cannot, then the memo is not merely hiding a replay — it is the
   only reason criterion 3 is green anywhere. *Needs:* one timed `cargo build --release` in a
   scratch tree (heavy; slot-gated).

3. **Whether the fuzz corpus would even exercise the `sys_` hole.** `gen.py:128` emits
   `clock_ms()`, `sys_arena_base()`, `sys_arena_end()` — but always in the form `builtin * 0`
   (ROADMAP row 4: "the program value stays deterministic"). `* 0` neutralises the value on
   both sides, so agreement is guaranteed regardless of what either computes. My reading is
   that fuzz covers the **emit path** and is structurally **value-blind** on these three
   builtins, and cannot be un-blinded while bpref returns 0 for them. *Needs:* confirmation
   from whoever wrote the `* 0` whether value coverage was intended to follow.

4. **Whether the compiler rejects a fabricated `sys_*` name.** I did not compile — a `.bp`
   compile is slot-gated work and this lane is barred from it. If the compiler *rejects* it, F2
   is confined to EXPECT derivation and to real `sys_*` values. If it *accepts* it, compiler and
   oracle agree on a fabricated program and `bpref_parity` would be green on nonsense.
   *Needs:* one command in a slot-holding lane —
   `./seed/build/seed ./bebop.bin compile /tmp/ghost.bp /tmp/ghost.bin; echo rc=$?`

5. **What `G1`…`G8` are actually bound to.** I verified no gate bears those names and listed
   the 13 store-ish gates in `std_golden.sh`, but I found no file mapping label → gate.
   `docs/LANG-DB-DESIGN.md §5` is cited by `RESULT-sbench.md` as holding the G7 pass rule; I did
   not read §5 in full. *Needs:* that section, or a written mapping. Until one exists, "G1-G6
   green" cannot be checked by anyone, which is itself the finding.

6. **Pre-2026-09-06 history.** The repo is shallow (240 commits). Any claim about when a number
   was *first* measured before 2026-09-06 is beyond what I can verify here. *Needs:*
   `git fetch --unshallow`.

7. **Whether `d785e062`'s 28,500 seeds were clean.** I counted seeds from the journal; I did not
   audit the DIVERGE/CRASH columns per batch. The three most recent batch lines include one
   `DIVERGE=2` and one `TRAP-82=1`, both `VERDICT:ALERT`, on bins `e9159318` and `05aa91ac`.
   Whether those were resolved I could not establish. *Needs:* the `ALERT` file history, which
   does not exist in `~/.cache/bebop/fuzzd` today.

---

## METHOD

Read-only throughout. `git log` / `git show` / `git cat-file` against `/root/dowiz` only —
no command that writes or locks the index. One deviation to record: verifying F1 required
running `bench/oracles/money.py`, which started a `cargo` build and created
`bench/oracles/rust/target/`. I aborted it at 60 s and removed the directory
(`rm -rf bench/oracles/rust/target`); the lane tree is otherwise byte-unchanged apart from this
file. `bash bench/oracles/run_all.sh` was run once — it is a 300 ms cache hit and executed no
oracle, which is the finding. `python3 tools/bpref.py` was run on three one-line fixtures.
No compile, no chain, no battery, no slot.
