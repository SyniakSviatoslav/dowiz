# GATE PROVENANCE AUDIT — 2026-09-13

**The one question, asked of every assertion in the tree:**
*Does this assertion measure the thing its name claims — and WHICH COMPONENT computes the number?*

This is not the falsifiability axis (can the line go red), which a sibling lane is auditing on
`tools/battery.sh`'s 15 `line` calls. This axis is **provenance**: who computes the number, does
the name match the computer, and — the question that found every one of today's five defects —
**what does the gate do when its instrument is ABSENT?** The right answer is RED.

Base commit `6726f9a`. 71 assertions audited, 34 mismatched. Nothing in this document was fixed.

---

## 0. READ THIS FIRST — what is live right now

**No gate is currently green over a known-live unsound term.** What follows are gates that are
green *today* while measuring nothing, or that would stay green if a soundness bug were
introduced tomorrow. Four are demonstrated with a command; all four were run in this lane.

### 0.1 A gate is printing the word MISSING today and returning 0

`tools/check_abi.py:381-383` prints:

```
literal trap (6000 + nlits >= 7000): MISSING at bebop.bp:? (compile-time trap owned by bebop.bp)
```

`guarded` is computed at `check_abi.py:373` and **is never appended to `errs`** — compare
`:374-378`, where `check_registered` and `check_perfn` results *are*. So `check_fntab` returns 0,
`invariants.sh:42` passes, and `invariants: GREEN` prints with the word MISSING in the log.

The trap it names does not exist:

```
$ grep -c "7000" bebop.bp
0
```

The compile-time guard that is supposed to refuse a program whose literal count pushes
`fntab[6000+k]` past 6999 is absent from the compiler, the checker says so in plain English, and
no gate is red. **Rank: SOUNDNESS** — silent `fntab` overflow past the registered zone map is the
b1_facts defect class `check_abi.py:220-226` exists to prevent.

### 0.2 The F8 gate is GREEN with no compiler at all

```
$ cd /root/dowiz/.claude/lanes/provenance
$ python3 tools/f8_dt.py /nonexistent_compiler.bin
REJECTED [requires/ensures garbage 1]: compile exit 90
REJECTED [requires/ensures garbage 2]: compile exit 90
REJECTED [theorem garbage 1]: compile exit 90
REJECTED [theorem garbage 2]: compile exit 90
f8_dt: 0 discarded (not parsed), 0 garbage-accepting positions (ratchet 0)
f8_dt gate: PASS -- ratchet holds at 0.
$ echo $?
0
```

The gate's only measurement is "did the compiler reject this garbage". A compiler that rejects
*everything* — or is not there — scores a perfect ratchet. `tools/f8_dt.py:123` catches every
exception without incrementing the counter, and the positive arm (`positive_pass`) is printed at
`:130` and **asserted by nothing**, so there is no positive control to notice that nothing
compiled. `tools/battery.sh:24` guards `[ -s "$BIN" ]`, so inside the battery the file exists —
but "exists" is not "is a compiler". **Rank: CORRECTNESS.**

### 0.3 The F7 kernel-soundness line is GREEN with a binary that is not a kernel

```
$ TKERNEL_BIN=./bebop.bin python3 tools/kcheck.py --corpus bench/kernel_neg | grep '^kernel_'
kernel_neg: 0 accepted of 21
kernel_internal: 0 of 28  (a CRASH is not a rejection and is never scored as one)
kernel_pos: 0 rejected of 7
kernel_neg_bin: 0 accepted of 21      <-- battery regex ' 0 accepted of 21' MATCHES: GREEN
kernel_parity: 0/28                   <-- battery regex '28/28' does not match: RED
```

`kernel_neg_bin` is the line that was added today to fix the canonical `kernel_neg` defect, and it
has inherited half of it. `tools/kcheck.py:611` counts only `if not is_internal and
kernel_verdict == 0` — an instrument that **answers nothing is scored as having rejected
everything**. The line that names binary soundness is green while the binary is `./bebop.bin`.

The battery survives only because `kernel_parity` is pinned at `28/28` and catches it. Remove or
weaken that one line and `kernel_neg_bin` becomes a permanent green. **Rank: SOUNDNESS.**

### 0.4 `./tkernel.bin` is never compared to `selfhost/tkernel.bp`, and is built exactly once

```
tools/battery.sh:26   if [ ! -s "./tkernel.bin" ]; then
tools/build_tkernel.sh:26   if [ -s "$TKERNEL_BIN" ]; then
tools/build_tkernel.sh:27       echo "build_tkernel.sh: $TKERNEL_BIN already built (skipping rebuild)"
```

The battery builds the kernel only if the file is missing; `build_tkernel.sh` then also skips if
the file is non-empty. `tkernel.bin` is **untracked** (`git cat-file -e HEAD:tkernel.bin` →
`does not exist in 'HEAD'`) and a full-tree grep finds **no check anywhere** that compares it to
its source — no `guard_artifact` call, no `arch_check` artifact-identity rung, nothing:

```
$ grep -rn "tkernel" tools/*.py tools/*.sh bench/
tools/kcheck.py:687, tools/battery.sh:26,27,28,46, tools/build_tkernel.sh   # and nothing else
```

Edit `selfhost/tkernel.bp` to weaken the elimination restriction added in `6726f9a`, do not
rebuild, and both F7 binary gates measure the *old* binary. The only staleness signal is indirect:
`kernel_parity` compares the binary to the twin, so a stale kernel is caught **iff** one of the 28
committed fixtures discriminates. A hole closed in the source for which no fixture exists is
invisible. `arch_check.py:87-99` does exactly this job for `bebop.bin` — the kernel has no
equivalent. **Rank: SOUNDNESS, conditional on fixture coverage.**

### 0.5 `typecheck census: 0 findings` is the success string a total parser breakage also prints

`bench/vs_rust/invariants.sh:55` asserts the last line equals exactly
`typecheck census: 0 findings`. `tools/typecheck.py:170-173`:

```python
    try:
        p = bpref.Parser(src); p.program()
    except Exception as ex:
        print('%s: PARSE %s' % (f, ex)); continue
```

A file bpref cannot parse contributes zero findings and `continue`s. Land a surface syntax
`bpref.py` rejects and all ~200 corpus files print `PARSE …`, `total` stays 0, and the rung prints
its success string — a log that looks *better* than before. `invariants.sh:54`'s `| tail -1`
discards the `PARSE` lines and the exit code alike. The negative control at `:56-57` is armed
backwards: `bench/typecheck_neg/` holds exactly **one** file, and if it is deleted the glob stays
literal, `open()` raises outside the try, and the `FileNotFoundError` line is `!=` the target
string — so **the control passes because the corpus is gone**. **Rank: SOUNDNESS.**

---

## 1. Four patterns, not three

The three named in the brief all recur. A fourth is distinct and is the most common defect in this
tree.

| pattern | shape | examples found here |
|---|---|---|
| **WRONG COMPUTER** | the name says compiler/kernel/binary; a Python twin or a text scan computes it | `kernel_neg`; `typecheck census`; every `check_abi`/`arch_check` rung that greps `bebop.bp` text; `honest.sh`'s K6 row, scraped out of a markdown file |
| **UNREACHABLE INSTRUMENT** | correct code that never runs; a fallback prints something number-shaped | `trap_census.py:352 scan_binary_texts` (8-byte guard, **no callers**); `trap_census.py:89` and `:102` anti-vacuity `die()`s, both on the code path `--texts` does not take; `certcheck.py:312` `cert_path` assigned and never used |
| **STALE CONTRACT** | producer changed format, consumer parses the old one, zero reported as success | `invariants.sh:97` greps `"RESERVED but still emitted"`, one word wide of `WITHDRAWN`; `check_abi.py:372` exact-text anchor `fntab[6000 + lcnt[0]] =`; `invariants.sh:106` names a ratchet key that does not exist |
| **ORPHAN INSTRUMENT** *(new)* | the whole tool is correct and maintained, **no gate calls it**, and documents quote its number as if it were enforced | see §1.1 |

### 1.1 The fourth pattern: orphan instruments

A mechanical sweep — for each script, is its basename referenced by any *other* `.sh` or `.py`? —
found **38 verdict-producing scripts that no script calls.** The ones that matter:

| tool | what it asserts | quoted in |
|---|---|---|
| `tools/roadmap_check.sh` | **the only pin on the std gate count** — `:22` `g=$(grep -c '^gate ' std_golden.sh); grep -q "ok=$g" ROADMAP.md` | referenced only in a *comment* in `tools/split_roadmap.py:2` |
| `tools/tv_fragments.py` | F5 translation validation | 3 docs (already diagnosed in `docs/blueprints/F4-fragment-validation.md:19-27`) |
| `tools/mutate_compiler.py` | whether any gate detects a mutated emitter | `tools/mutation_coverage.txt`; **`tools/mutate_gate.sh` does not call it** (verified: no `.py` invocation in the file) |
| `tools/journal_lint.sh` | no unfalsifiable journal claim | `tools/hooks/pre-push:9` — a hook whose installation is unverified |
| `tools/undef_census.py`, `tools/lin_census.py`, `tools/check_encodings.py` | verdict lines | `docs/VERIFIED-STATE-2026-09-12.md` and others |
| `tools/certcheck.py` | certificate soundness | no caller found in this tree |

`roadmap_check.sh` closes §2.1's hole exactly — and nothing runs it. `ROADMAP.md:53` asserts
`ok=117 self-frozen=0`; `tools/battery.sh:62` asserts only `self-frozen=0 mismatch=0 missing=0`
and **never asserts `ok=117`**. The document asserts a number the gate does not.

**The provenance failure of an orphan instrument is total:** the number in the document has no
producer at all at the time it is read.

---

## 2. The table

Columns: **assertion** · **name claims** · **who computes it (verified anchor)** · **mismatch?** ·
**green while the named thing is broken?** · **rank**.
Every `file:line` below was printed with `sed -n` before being written down. Rows marked
**[2nd]** were read by a parallel reader and spot-verified by me at the cited line; rows marked
**[unverified]** say what is missing.

### 2.1 `tools/battery.sh` — the 15 asserted lines, plus the two unasserted ones

Verdict helper for all of them: `battery.sh:55`
`line() { local l; l=$(grep -E "$2" "$T/$1" | tail -n 1); [ -n "$l" ] || { l="MISSING ($1)"; red=1; }; echo "$l" | grep -qE "$3" || red=1; echo "  $l"; }`
A missing log or a missing line is correctly RED. **All 15 regexes currently match their
producer's live format** — I checked each against the producing `echo`; there is no stale contract
*inside* battery.sh today. The defects are all upstream, in what the producers count.

| # | assertion | name claims | who computes it | mismatch? | green while broken? | rank |
|---|---|---|---|---|---|---|
| 1 | `std_golden: 117 pass, 0 fail (J=1 shards)` | 117 golden gates ran and agreed | **`tools/std_par.sh:82`** — `P=$(grep -c '^PASS ' "$T/all.log"); F=$(grep -c '^FAIL ' …)` over `:53` `cat "$T"/shard*.log \| grep -E '^(PASS\|FAIL) '`. The goldens themselves are **literals typed into `std_golden.sh`** (`gate checksum 96354`) | **YES** — the number is a line count over concatenated shard logs, not a count of gates that exist | **YES, demonstrated.** A shard that dies contributes no lines. With every shard dead, `all.log` is empty: `std_golden: 0 pass, 0 fail (J=1 shards)` — and `' 0 fail'` matches. **Green with 0 of 117 gates run.** `std_par.sh` knows the gate count (it prints `shard k: N gates` at `:39`) and never compares. The one check that would catch it, `roadmap_check.sh:22`, is an orphan (§1.1) | **CORRECTNESS** (hides everything the 117 gates cover) |
| 1b | `PASS <name> (RETRIED standalone, pinned)` | the gate passes | `std_par.sh:62` — `sed -i "s/^FAIL $name:.*/PASS $name (RETRIED…)/" "$T/all.log"` | **YES** — a FAIL is rewritten into a PASS in the log the summary counts | a genuinely flaky miscompile passes on the second try and is counted a pass; the only trace is `perf.py record battery_flakes` (`:80`), which no gate asserts | CORRECTNESS |
| 2 | `construct parity: pass=120 fail=0 no_expect=0` | 100 positive + 20 negative constructs compiled, ran and matched | `bench/vs_rust/construct_parity.sh:112`; the compile is `./seed/build/seed $BEBOP_BIN compile` — the real candidate | NO on the computer | **`pass=` is not asserted.** Delete 90 constructs: `pass=30 fail=0` is green. Coverage shrinks in silence. Note this script *did* get the trap-after-correct-output fix (`:55-67`, `RUNFAIL`/`RUNNOISE`) — it is the only one of five runners that did | CORRECTNESS |
| 3 | `diag: 19 pass, 0 fail` | every `bench/diag_neg/*.bp` produced the hand-counted `line:col` and code | `bench/vs_rust/diag_check.sh:75`; positions from `// EXPECT` headers at `:11` | NO on the computer | `pass=` unasserted; empty the `diag_neg/` corpus and the 9 hardcoded checks still yield `0 fail` | CORRECTNESS |
| 4 | `parity: pass=14 fail=0 skip=…` | the K-kernels agree with the reference | **`bench/vs_rust/parity_driver.sh:26-59`** — a `case` of **hand-typed literals**. There is no Rust twin in this path despite the directory name `vs_rust` | **YES** — "parity" reads as agreement between two implementations; it is a frozen-value regression against numbers typed into the gate | **YES, two ways.** (a) `:24` `IVAL=$(timeout 30 … \| tail -1)` — **the exit code is discarded through the pipe.** A kernel that prints the correct value and *then* traps is a MATCH. `construct_parity.sh:55-58` documents this exact defect, measured today, and fixed it *there*; `parity_driver.sh` did not get the fix. (b) `:60` `*) EXPECT="";;` — a kernel with no `case` arm that prints nothing (i.e. traps before output) compares `""` to `""` and **MATCHes**. All 14 kernels have arms today, so (b) is latent | **CORRECTNESS** |
| 5 | `pool_parity: 5 pass, 0 fail` | 4 threads compiling produce the same code as 1 thread | `bench/vs_rust/pool_parity.sh:176`; the reference `WANT` comes from `par_expect` (`:46-54`), which re-runs **the same program** with `par_compile(1,…)` | **YES** — the name claims parallel/serial agreement; the measurement is a program compared to itself. If `par_compile` silently ran serially the sum is still `4*w` and the gate passes. Only gate 3 (`par_tids`, `:174`) is evidence of real threads | exit codes discarded at `:65,77,118,160,173` (all `\| tail -1`) — same trap-after-output hole as row 4 | CORRECTNESS |
| 6 | `SUMMARY ok=117 self-frozen=0 mismatch=0 missing=0` | every gate's golden was independently confirmed by an oracle | `bench/oracles/run_all.sh:38`; the oracle is `:20` `o=$(timeout 300 python3 "$D/$g.py" 2>/dev/null \| tail -1)` — a genuinely independent Python computation. **The frozen side is scraped out of the gate script's source text:** `:11` `FROZEN=$(grep -E '^gate [A-Za-z0-9_]+ -?[0-9]+ ' bench/vs_rust/std_golden.sh \| awk '{print $2, $3}')` | **YES** | **YES, demonstrated.** Change `std_golden.sh`'s gate-line format (a variable golden, different spacing) and `FROZEN` is empty; `xargs` runs nothing; `OK=SF=MM=MISS=0`; the line becomes `SUMMARY ok=0 self-frozen=0 mismatch=0 missing=0` — **the battery's regex matches, exit 0, and the memo at `:40` caches that green**. `ok=` is never asserted. Also: `:20` discards the oracle's exit code through `\| tail -1`; and **11 oracle `.py` files are never executed** because `one()` iterates `FROZEN`, not the oracle directory (`c68_strval`, `gb_lagraph`, `ingest_maxrss`, `ingest_small`, `join_twin`, `lag_common`, `scan_twin`, `sgraph`, `sgraph2`, `storelib`, `tpch`) | **CORRECTNESS** |
| 7 | `bpref_parity: agree=100 unsupported=0 disagree=0 error=0 total=100` | the differential oracle and the compiler agree on every construct | `bench/vs_rust/bpref_parity.sh:71`. Genuinely differential — no EXPECT is read. Well built: `:44-47` deliberately avoids the `\| tail -1` rc bug; `:61` rejects an empty answer from a zero exit | **YES, on `unsupported`** | **YES.** `tools/bpref.py:800` is `except BaseException as ex: … sys.exit(2)` — **any** exception in the evaluator becomes rc 2, which `:51-56` scores `unsupported`. The battery asserts only `disagree=0 error=0`. So `agree=0 unsupported=100 disagree=0 error=0` is **GREEN** — the oracle answering nothing at all. This lane exists (`:5-9`) precisely because bpref's `set` node was silently wrong for four days; the lane can now go silent the same way. `total=` and `agree=` are unasserted | **CORRECTNESS** |
| 8 | `kernel_neg: 0 accepted of 21` | the kernel rejects all 21 unsound terms | **`tools/kcheck.py:681`**, counting `run_file` (`:508`) — **the Python twin**. The canonical defect; battery's own comment at `:64` now says "(Python reference)" | **YES**, by construction and now by admission | yes — this is the documented original. Retained for twin/kernel cross-refutation | SOUNDNESS (mitigated by row 9) |
| 9 | `kernel_neg_bin: 0 accepted of 21` | **the kernel binary** rejects all 21 | `tools/kcheck.py:597 measure_kernel_neg_bin`, via `run_kernel_on_fixture` (`:570`) → `subprocess.run([seed, kernel_bin, fixture])`. Correct computer | **YES, partially** | **YES, demonstrated (§0.3).** `:611` counts only `verdict == 0` and treats every internal/crash/timeout as a rejection. A kernel that crashes on all 21, or a binary that is not a kernel, scores `0 accepted of 21`. Caught today only by row 10 | **SOUNDNESS** |
| 10 | `kernel_parity: 28/28` | twin and binary agree on all 28 fixtures | `tools/kcheck.py:617 measure_kernel_parity`. Correct computer, and the count is pinned (a 22nd negative fixture breaks both `of 21` and `28/28` → RED). **Absent-instrument behaviour is correct**: `:700-701` print `NOT MEASURED (kernel binary absent)` and `:702` returns 1 | NO | this is the load-bearing F7 line and the only staleness signal on `tkernel.bin` (§0.4). Caveat: `kcheck.py:704` `return 1 if (accepted or rejected_pos or internal) else 0` considers **only twin results** — kcheck exits **0** while reporting `kernel_parity: 0/28`. Safe today because the battery reads lines, not the exit code; a latent trap for any future consumer | CORRECTNESS (latent) |
| 11 | `ABI ok <bin>: x27/x28 clean, x9-x13 allowlisted sys=N argpass=M` | the binary obeys the register-zone law | `tools/check_abi.py:415`, from `check_bin` `:164-196` over the hand-rolled decoder `writes()` `:81-112` — reads the real binary | **YES** **[2nd]** — three structural blind spots: `:183` `for i in range(PRO_N, len(span) - EPI_N)` with `(10, 8)` at `:38` means **any function ≤ 18 words is not checked at all**; `argpass_window` (`:46-59`) grants a 13-word amnesty before every `bl`; `sys_allow` (`:134-141`) harvests every `em(insns,n,<int>)` from all 28 `emit_sys_*` into one global word allowlist applied to every function of every binary | `sys=N argpass=M` are printed and **asserted nowhere** — the amnesty can swallow the corpus with no signal. In `battery.sh:49` check_abi's **exit code is discarded** (no `\|\| red=1`); `tail -n 1` of lines matching `ABI` is safe with one argument but would hide an earlier `ABI FAIL` if the call ever grew a second | **SOUNDNESS** |
| 12 | `invariants: GREEN` | 10 structural rungs all held | `bench/vs_rust/invariants.sh:157`, `fail` accumulated across the rungs | see §2.2 — three rungs can be green while measuring nothing | see §2.2 | SOUNDNESS |
| 13 | `words: PASS (no bebop.bp diff, or no new em()/st[] literal >= 0x1000)` | hand-typed machine words were verified against an objdump listing | `tools/check_words.py:50`, over `:46` `subprocess.run(['git','diff','HEAD','--','bebop.bp'], cwd=ROOT, capture_output=True)` | **YES, three ways** | (a) The gate measures **uncommitted** literals only. Once committed, `git diff HEAD` is empty → PASS for ever. Every hand-typed word in the tree's history is unverified by construction. (b) **In a lane it measures the wrong tree.** `ls -d .git` in this lane → absent; `git rev-parse --show-toplevel` from here → `/root/dowiz`. A lane's `check_words` diffs the *main* repo's `bebop.bp`, and takes a read lock on the main index doing it. (c) If git fails or is absent, `stdout` is empty → `lits=[]` → **PASS**. Instrument absent → green | **CORRECTNESS** |
| 14 | `boxguard: lcjit -- absent (box daemons removed 2026-09-07, operator)` | the box was healthy while the timing gate ran | `tools/std_par.sh:70` — `bstat=$(command -v boxguard >/dev/null 2>&1 && boxguard status … \|\| echo "absent (…)")` | **YES** | **This line is permanently green by design.** The battery's regex is `'.'` (`battery.sh:70`) — any non-empty string. `boxguard` is not installed on this box, so the assertion is satisfied by the literal word "absent". Inverted failure mode: it goes **RED** if `std_golden.sh` ever loses its single `# timing` marker (`:508`), i.e. red when there is nothing to measure, green when the instrument is gone | COSMETIC |
| 15 | `f8_dt gate: PASS -- ratchet holds at 0` | dependent surface syntax parses and erases correctly | `tools/f8_dt.py:151` | **YES** — and `battery.sh:52,71` label the lane "dependent surface types parsing + erasure" while `f8_dt.py:151` itself says **"NOT an erasure claim"**. The battery's comment overclaims what its own tool prints | **YES, demonstrated (§0.2).** Green with a nonexistent compiler | CORRECTNESS |
| 16 | `census: <bin> <bcond> <cbz> …` (`battery.sh:72`) | the branch census of the candidate | `battery.sh:48` `python3 tools/census.py "$BIN" \| tail -n 1` | **YES** | **Asserted by nothing.** There is no `line census.txt …` among the 15. The exit code is additionally lost through `\| tail -n 1`. A reader of `battery: GREEN` sees a census row in the summary block and reasonably takes it as checked; the load-bearing census is the separate `--check` in `invariants.sh:51`, and nothing tells the reader which of the two numbers is the gate | COSMETIC |
| 17 | `battery: GREEN` | every lane measured and agreed | `battery.sh:74`, `red` accumulated by `line()` | **YES** — it means "15 regexes matched", which as rows 1, 6, 7, 9, 13 and 15 show includes several ways of matching nothing | see above | SOUNDNESS |

### 2.2 `bench/vs_rust/invariants.sh` — the 10 rungs behind `invariants: GREEN`

| rung | assertion | who computes it | mismatch? | green while broken? | rank |
|---|---|---|---|---|---|
| preflight | `tools/guard_artifact.sh "$BEBOP_BIN"` (`:26`) | `guard_artifact.sh:6,8` — `[ ! -f "$f" ]` and `[ "$sz" -le 0 ]`. The **stale** half is `:9-11`, `if [ $# -ge 2 ]`, and the sole caller passes **one argument** | **YES** — the script's own header (`:2-3`) says "empty/**stale** .bin"; the wired behaviour is "exists and non-zero bytes" | a `bebop.bin` from three commits ago, or a 4-byte text file, passes the guard, and every number in the run then describes the wrong compiler. This is the "promoted binary != source" class exactly | **SOUNDNESS** |
| (i)+(iv) | `ABI ok` over ~70 bins (`:40`) | as row 11 | YES | as row 11 | SOUNDNESS |
| (iii) | `fntab zones: N constant bases, all in …` + `literal trap …: MISSING` (`:42`) | `check_abi.py:363-365` `re.findall(r"fntab\[(\d+)", line)` over **bebop.bp text** | **YES** — "all in …" is an unconditional `print`, not a conditional. Only *numeric-literal* bases are visible: `fntab[base + i]` with a variable base is invisible to the zone map, the registry and the per-fn check | **§0.1: it is failing in English and passing in exit code today.** Separately, the anchor at `:372` is an exact-text match on `fntab[6000 + lcnt[0]] =` — reformat that line and it degrades to `MISSING at bebop.bp:?`, still green | **SOUNDNESS** |
| (ii) | `census: 112 bins, no conditional-branch increase` (`:51`) | `census.py:89`, from `census()` `:22-33` — a real mask-match over the `.bin` words | NO on the headline | **Best-built gate in the set**: `census.py:86-87` turns "in table but not measured" into RED, which defeats the empty-set hole. Residual: if `census.txt` is emptied or its format moves, `table` and `seen` are both empty and `:89` prints `census: 0 bins, no conditional-branch increase`, rc 0. And a DECREASE (`:84-85`) prints but does **not** set `rc` | CORRECTNESS |
| (vii) | `typecheck census: 0 findings` (`:55`) + negative control (`:57`) | `typecheck.py:177`, over **bpref's AST** (`:171`) | **YES** — wrong computer *and* crash-goes-green | **§0.5.** Plus `REF_TOLERANT` (`typecheck.py:17`), an allowlist whose own comment records erasing 74 findings then 3 more; the gate asserts `0 findings` and never asserts that the allowlist stopped growing | **SOUNDNESS** |
| (texts) | `codes emitted with no diagnostic text: 0 (ratchet 0)` (`:109`) | `invariants.sh:69` — `NO_TEXT_COUNT=$(echo "$TEXTS_OUT" \| grep -c "with NO diagnostic text")`, compared to the literal `RATCHET_VALUE=0` (`:84`). Producer is `trap_census.py:533` | **YES** — `TEXTS_RC` is captured at `:62` and **never referenced again** (`grep -n TEXTS_RC` returns only line 62). `DT` at `:66` likewise assigned and never read | **YES, demonstrated.** A traceback from `trap_census.py` contains none of the five greps: `NO_TEXT_COUNT=0`, `HAS_OTHER_FAILURES=0`, rung prints `(ratchet 0)`, `fail` stays 0. Worse **[2nd]**: with `--src` unresolvable *and* `docs/TRAPS.md` moved, `trap_census.py:564` finds `0 == 0 == 0 == 0` and exits **0** with a single line. **Lowering this ratchet to 0 today converted a red-on-crash into a green-on-crash** — at ratchet 3, a crashed tool gave `0 < 3` → "ratchet improved" → `fail=1`. The stale-contract exposure is live: `trap_census.py` has no strict flag parsing (`:574` `if '--texts' in argv`), and `python3 tools/trap_census.py --nosuchflag` returns **rc 0** and prints a *different* report | **SOUNDNESS** |
| (texts) b | `if echo "$TEXTS_OUT" \| grep -q "RESERVED but still emitted"` (`:97`) | producer `trap_census.py:560`, where the word comes from `:438` `how = 'WITHDRAWN' if 'WITHDRAWN' in ln else 'RESERVED'` — **chosen from the prose of the TRAPS.md row** | **YES** **[2nd]** | one natural consistency edit to `docs/TRAPS.md`'s row for code 103 (row 33 already calls it "the WITHDRAWN trap 103") flips `how`, the grep matches nothing, `TEXTS_RC` is discarded, and a re-emitted retired code ships green. A stale contract one word wide | CORRECTNESS → SOUNDNESS once tripped |
| (v) | `expansion: 133 gate sources checked; drift=0 unregistered=0 lost-twin=0 grew-twin=0` (`gen_selfsrc.sh:83`) | `gen_selfsrc.sh:64` `cmp -s "$src" "$t"`; authority from `:47` reading `std_tests/GENERATED.txt` | PARTIAL **[2nd]** | **Delete `GENERATED.txt`** and `:48` stops firing, `ent` is always empty, so LOST-TWIN (`:52`) and GREW-TWIN (`:57`) can never fire — three of the four checks the `:24-28` comment says the manifest exists to provide switch off silently, leaving only the byte `cmp`, exit 0. `$n` is printed and never asserted: delete 50 gate sources and it prints `83 … drift=0` and exits 0. The positive path is genuinely honest — checker and fixer share one implementation (`:70`) | CORRECTNESS (SOUNDNESS for the manifest path) |
| (viii) | `seed: .text identical (N B; …)` (`:130`) | `as`/`ld`/`objcopy`/`cmp` on `seed/seed.S` | NO | **correct RED-on-absent** — `:131` explicitly folds "or as/ld missing" into `SEED DRIFT` and `fail=1`. One of two rungs in the file that get absence right | — |
| (ix) | `push_words: 0` (`:135-136`) | `perf.py size` piped into a `python3 -c` literal-eval | NO | a perf.py failure makes `PW` empty, `[ "$PW" = 0 ]` is false, `fail=1`. **Correct RED-on-absent** | — |
| (0) | platform identity `diff <(bash tools/platform.sh) …` (`:150`) | `:149` `if [ -f tools/platform.txt ]; then` | **YES** | **Missing frozen file ⇒ the entire rung is skipped in silence.** No note, no fail. Every latency and durability assumption below it is then unvalidated. (`platform.txt` is present today, 22 lines) | CORRECTNESS |
| (x) | `arch_check: all invariants hold` (`:155`) | `arch_check.py:715` | **YES** — see §2.3 | see §2.3 | SOUNDNESS |

### 2.3 `tools/arch_check.py` — 23 rungs behind one green line **[2nd, spot-verified]**

Structurally sound in one respect: all 23 `check_*` functions defined are actually called from
`main()` (`:684-706`) — no unreachable check function. The problems are the seams.

**Four defects that affect the whole file:**

- **(a) `bp_sources()` empty ⇒ seven checks go green at once.** `:32-47` walks `ROOT`; seven
  checks iterate it and report a count (`:54, :67, :210, :347, :512, :555, :655`). An empty walk
  makes all seven report 0 and pass, and `check_file_size` prints
  `note("largest .bp: None at 0 lines (ratchet 8500)")` — a green line naming `None`.
- **(b) Coverage can shrink invisibly.** Nothing asserts how many checks ran. The meta-guard
  `check_laws_have_enforcement` (`:410-413`) builds its `have` set by **grepping arch_check.py's
  own source text** for `fail("name"` — so a check whose function exists but is no longer *called*
  still satisfies the law manifest. Comment out one line of `main()` and the law is still
  "enforced".
- **(c) Four checks return silently when their input is absent** — no note, no fail:
  `:136`, `:151`, `:371-372`, `:453`. Rename `bench/vs_rust/std_golden.sh` and **three**
  invariants vanish with zero output. (`check_prereq_guarded` at `:598-600` does it right — it
  notes the skip.)
- **(d) `check_binary_matches_source` writes `/tmp/arch_check_gen.bin` (`:87`)** — a fixed global
  path, not `$BEBOP_TMP`. Two lanes running arch_check concurrently race on one file and may md5
  each other's output, on the single most important identity check in the tree. And `:89`
  **skips with a NOTE** when `seed` is absent: `artifact-identity: skipped, no seed binary`, and
  the run still prints `all invariants hold`.

**The rungs whose name outruns their computer (SOUNDNESS-ranked only):**

| rung | assertion | who computes it | why the name outruns it |
|---|---|---|---|
| `artifact-identity` | `bebop.bin == compile(bebop.bp) == <md5>` `:99` | `:90-94` — **actually runs the compiler and md5s both**. The one rung where the named component computes the number | skips-with-a-note on absent seed (d); `/tmp` race (d) |
| `object-header` | `0 writers touch cells 0/1 of an allocated object` `:230` | `:222-226` regexes requiring the array to be named `*base*`/`new`/`old`/`b2` **and** the index to be a parameter named `off`/`*_off`/`obj` | rename the parameter `o` or the array `buf` and the writer is invisible; the incident at `:199-203` ("one off-by-two, nine red gates, a day to find") recurs while it prints `0 writers` |
| `builtin-coverage` | `N emitted builtins, M uncovered` `:320` | `:311-312` `def called(b): return (b + "(") in text or (b[4:] + "(") in text` over concatenated corpus text | `b[4:]` strips `sys_`, so `sys_read` is "covered" by any `read(` **anywhere, including a comment**. Nothing executes; no differential test is consulted. The fail text claims "no differential test can ever see a miscompile in them", so its inverse reads as "these are all differentially tested" — which this code cannot establish |
| `gate-oracle` | `117 gates, 0 without an oracle` `:382` | `:374-375` — **`os.path.exists(bench/oracles/<name>.py)` only** | asserts a *filename*. Hypothetical: `touch bench/oracles/foo.py` (0 bytes) makes a gate "oracled". Does not check that `run_all.sh` runs it — and §2.1 row 6 found 11 oracles it never runs |
| `law-manifest` | `N laws, all with a named enforcement` `:426` | `:410-413`, grepping arch_check's own source | see (b) |
| `scripted-patch-assert` | `N tool(s) without an anchor assert` `:499` | `:492` `if "count(" not in txt and "assert" not in txt` | **the word `assert` anywhere in the file** — a docstring, a `# TODO assert this` — makes a patcher compliant. The incident it guards (an anchor matching 0 times ⇒ a commit claiming a change it did not make) recurs behind a comment |
| `clone-symbols` | `every spawn site keeps <= 8 symbols across it` `:583` | `:553-576`; the cap is `r.get("max_clone_kept_symbols", 8)` | phrased as a proven property, printed whenever `len(bad) <= max_clone_violations`. **And the `8` is a hardcoded Python default — `max_clone_kept_symbols` is not in `arch_ratchet.txt`.** The number governing the tree's worst silent-failure mode lives at `arch_check.py:553`, not in the ratchet file anyone would audit |
| `unbounded-wait` | `N unbounded wait(s) on shared memory (ratchet 0)` `:680` | `:663` `if not re.search(r"\b(cells\|base\|locks)\s*\[", cond): continue` | **only** while-conditions indexing an identifier literally named `cells`, `base` or `locks` are examined. Spin on `flags[0]`, `sh[0]`, `q[i]`, `st[0]` → skipped before analysis → `0 unbounded wait(s)` against a ratchet of **0** reads as proof |
| `loud-failure` | `no silenced runs in gate or tool scripts` `:196` | `:191` — requires the line to also mention `$SEED` | the docstring (`:174-175`) promises two things; **the "exit code thrown away by a pipeline" half is not implemented**. `invariants.sh:134` does exactly what the docstring forbids and is not flagged. Scope is `tools/*.sh` + `bench/vs_rust/*.sh` only |
| `arch_check: all invariants hold` `:715` | all 23 evaluated and passed | `main()` | means "nothing appended to `fails`", which includes every silent `return` in (c), every skip in (d), and every empty-corpus zero in (a) |

**Ratchets whose "good" value is 0** — where "0 findings" and "the tool produced no output" are
indistinguishable: `max_unbounded_waits=0` (`:674`), `max_unguarded_prereqs=0` (`:609`), plus four
caps defaulted to 0 **in code and absent from `arch_ratchet.txt`** (`max_undocumented_traps` `:333`,
`max_alloc_in_loop` `:357`, `max_gates_without_oracle` `:376`, `max_clone_violations` `:577`), plus
the shell-side `RATCHET_VALUE=0` at `invariants.sh:84`, which is the worst instance because there
the instrument crashing *produces* the 0.

`check_ratchets_are_read` (`:627-639`) proves file→code and **cannot prove code→file**, which is
why those five are invisible. And `invariants.sh:106` instructs the reader to lower
`max_codes_emitted_no_text in tools/arch_ratchet.txt` — **that key does not exist in that file.**

### 2.4 Outward — gate numbers outside the battery **[2nd, spot-verified]**

| assertion | name claims | who computes it | mismatch? | green while broken? | rank |
|---|---|---|---|---|---|
| `trap_unrep: 16/35` (`trap_census.py:699`) | fraction of language traps closed with a mechanism **and** a regression test | `:599-601` — `has_mech = not mech.startswith('none')` where `mech` is a **hand-typed English sentence** in the `ROWS` literal (`:216-278`); the derived half (`:611-617`) appends rows with `has_mech=True, closed=True` **hard-coded** | **YES** | add one `neg/*.bp` for an already-caught code and both numerator and denominator rise — the ratio moves toward 100% with no trap closed. Delete the `unresolved-call` mechanism from `bebop.bp` entirely and the row stays `closed`, because `has_mech` reads the sentence `'brk #87 at run time (T130)'`. Not wired into the battery (`ROADMAP.md:195`), which is the only mitigation | SOUNDNESS |
| `die('NOT MEASURED: neg/ has constructs but no EXPECT headers…')` (`:589-590`) | the `EXPECT=` → `// EXPECT` regression of `d9890da` cannot recur | `:587-588` — `if has_neg_dir and not negs` | **YES** | all-or-nothing. Change the header form in 19 of 20 constructs: one still matches, `negs` is non-empty, no `die`, and `trap_unrep` reports a smaller denominator at rc 0. The exact defect is blocked **only at literally zero** | CORRECTNESS |
| `diag_texts: … k in binary` (`:515`) | the diagnostic string is present in the compiled binary | `:507-510` — a bare `if text.encode() in bin_content`, **no minimum length**. The vetted implementation with the 8-byte guard, `scan_binary_texts` at `:352`, **has no callers** | **YES** — unreachable instrument beside live loose code | give a code a 3-character text; those bytes occur by chance in a 200 KB image, `k` counts it, `n==m==k==d`, rc 0, while the compiler never emits the string | CORRECTNESS |
| `certcheck: OK` + `bound to <obl> (sha256 verified)` (`certcheck.py:267,269`) | this certificate refutes the named obligation | `check()` `:117` opens **only** `cert_path` (`:125`); the obligation is used only at `:260` for `obl_oid` — its clauses are never parsed and the cert's own `c` lines are never compared to it | **YES** | write `p 1 1 / c 1 1 / c 2 -1 / r 3 0 1 2 / x <real-oid>`: the chain closes, the empty clause derives, the oid matches → `certcheck: OK`. **A two-clause tautology certified against an arbitrary obligation** | SOUNDNESS |
| `certificates verified: N` then `return 0` (`certcheck.py:340-342`) | `--verify-root <root> <cert> <obls…>` checked that certificate | `:312` binds `cert_path = args[2]` and **never uses it**; `:332` guesses `<obl>.cert` and `:334` `if os.path.exists(…)` **skips silently**; `:337` `rc = check(...)` assigns a value never read | **YES** | `--verify-root <root> proof.cert a.obl b.obl` with no `a.cert`/`b.cert` on disk prints `obligations: 2`, `certificates verified: 0`, **rc 0**. Zero certificates checked; the file the operator named was never opened. `n_ok` is never asserted equal to `len(obl_paths)` | SOUNDNESS |
| "Merkle root construction" (`certcheck.py:43-46,76`) | a Merkle tree with inclusion proofs | `:86-93` — `leaves.sort()` then one flat `sha256(b''.join(leaves))` | YES (naming) | the value is deterministic and fit for an equality gate, so the gate works; `--merkle`'s promise of per-obligation proofs cannot be met | COSMETIC |
| `tv_fragments: NOT MEASURED (…)` (`tv_fragments.py:75,79`) | F5 is not implemented and this tool refuses to print a number | `main()` `:61`; `load_bin` `:40` genuinely parses the footer | **NO** | **no rc-0 path exists; the word PASS does not occur.** Already diagnosed and rewritten per `docs/blueprints/F4-fragment-validation.md:19-27`. **This is the model the rest of the tree should follow** | — |
| `journal_lint: clean` (`journal_lint.sh:20`) | no newly added journal line makes an unfalsifiable claim | `:18` — `while … done < <(git diff origin/main..HEAD -- docs/exp.journal \| grep -E '^\+[^+]')` | **YES** | a failing `git diff` (no `origin/main`, detached HEAD, fresh clone) feeds the loop **zero lines**: `bad=0`, `clean`, exit 0, push allowed. Nothing counts the lines examined. Also `:11` fires only on `transient\|flaky\|not reproducible` — "intermittent" is unlinted — and `:12` accepts the two-character string `rc=`, including `rc=?`. And run from a lane it locks the **main** repo's index | CORRECTNESS |
| `MET` / `UNMET` (`honest.sh:47`) | a pass/fail verdict on the D1(a) ≤ 2.0x gate | `:46` `ratio = bm/rm …` | **YES** | table prose. The script never exits non-zero on `UNMET`; nothing consumes it | CORRECTNESS |
| `K5 self-compile … {k5:.2f} s` (`honest.sh:61`) | a measured cold self-compile time | `:59` `t=time.time(); subprocess.run([...], capture_output=True); k5v.append(time.time()-t)` — the `CompletedProcess` is discarded; neither rc nor the existence of `k5.bin` is checked | **YES** | a syntax error in `bebop.bp`, or a stale `BB`, aborts in 20 ms and the row publishes **`0.02 s`** — the best K5 in the project's history. **A failure is reported as an improvement** | **SOUNDNESS** |
| `K6 nnidx scan 1M … \| {k6} ms \| sqlite scan 183 ms … \| store faster` (`honest.sh:62`) | a measured comparison against sqlite | `:50` — a regex scraped out of the **committed markdown** `bench/tq_sqlite/RESULT.md`; `:51` `except Exception: k6='?'`. The baselines and the verdict `store faster` are **string literals** | **YES — wrong computer is a markdown file** | rename or reformat `RESULT.md` and the row prints `\| ? ms \| … \| store faster \|`. The verdict is asserted with no number at all | **SOUNDNESS** |
| `mutation: N sensitive, M insensitive` (`mutate_gate.sh:51`) | N std gates provably detect an operator change | `:50` awk over `std_golden.sh` \| xargs \| `tee report.txt`, then two `grep -c` | **YES** | change the gate-line syntax → awk emits nothing → `report.txt` empty → `mutation: 0 sensitive, 0 insensitive`, rc 0 (the header says "exit 0 always"). Nothing asserts the two counts sum to 117. A gate whose base fails to compile (`:17`) appears in **neither** count | CORRECTNESS |
| `<gate> sensitive (c/t mutants changed the fold…)` (`mutate_gate.sh:47`) | the gate's fold moved, so the gate is load-bearing | `:43-45`: a mutant that **fails to compile** counts as changed (`:43`) — the *parser* made the kill; `:44` `got=$(timeout 120 … 2>/dev/null \| tail -1)` — **rc lost through the pipe**, so a SIGSEGV/trap/timeout prints nothing and counts as changed; **the unmutated baseline is never executed** and never compared to `$want` | **YES** | live: four gate sources carry **two** `gate` lines each (`gb_pool.bp`, `gb_gen.bp`, `scrash.bp`, `sround.bp`) and `one_gate` compares only `\| tail -1`, so at most one of each pair's `$want` can ever match — the other gate reports `sensitive` **unconditionally, mutation or not**. Generalised: point `BEBOP_BIN` at a binary that miscompiles everything and all 117 rows report `sensitive` | **SOUNDNESS** |
| `emit_hvham SURVIVED pass=95==95` (`mutation_coverage.txt:7`) | no gate detects a corruption of that emitter | not this file — a static committed transcript. Producer `mutate_compiler.py`'s `sites()` (`:28-38`) takes only the **first** `em(insns,n,<literal>)` per emitter | **YES**, and the file says so at `:33-38`: for `emit_sys_cond_set` the mutated word is `2332102177` while "the word that actually publishes is `4177526819` … and has **NEVER** been mutated by `sites()`" | a SURVIVED row reads as "coverage gap, add a gate" when it may mean "we mutated a word that does not affect the result". The `95` baseline is frozen text; gates added since make `pass=95==95` meaningless, and nothing re-derives it | CORRECTNESS |
| `roadmap_check: GREEN` (`roadmap_check.sh:23`) | the roadmap's numbers match the tree | `:22` `g=$(grep -c '^gate ' std_golden.sh); grep -q "ok=$g" ROADMAP.md` — the **only** pin on the gate count anywhere | — | **no script calls it** (§1.1). Also `:11-12` *regenerates `TASKS.md` in place* as part of "checking" — a checker that mutates the tree. Today: `g=117` and `ROADMAP.md:53` contains `ok=117`, consistent | CORRECTNESS |
| `chain: fixpoint gen3 == gen4 <md5>` + `grep -q 'battery: GREEN'` (`chain.sh:46`) | three-generation self-hosting fixpoint and a green battery | `chain.sh:43-47`; the battery's **exit code is discarded** (`:39,41`) and the verdict is a grep for the string | NO — the grep is the honest choice here; a missing `battery.log` makes grep fail → exit 1 | — | — |
| `perf: N metrics, R runs, A alerts` (`perf.py:356-357`) | performance regressions were checked | `report()` `:321`; alerts only for rows with `valid == "1"` (`:349`) | NO | `:323` `if not R: print("perf.csv empty"); return 0` — an absent/empty `perf.csv` is GREEN with zero metrics. **Reachability verified, not assumed**: `bench/perf.csv` has 2069 rows, 1840 `valid=1` / 229 `valid=0` across 121 metrics, so the alert path is live on this box | CORRECTNESS |

---

## 3. Ranked findings — what is worth acting on

**Ranked by (can it hide a soundness bug) × (how little has to go wrong).**

1. **`check_abi.py:373` — `guarded` is computed and never appended to `errs`.** The gate prints
   `literal trap …: MISSING` today and exits 0, and `grep -c 7000 bebop.bp` is `0`. One line:
   append it. *This is the only finding in this document where a gate is printing a failure right
   now.*
2. **`kcheck.py:611` — an instrument that answers nothing is scored as a rejection.** Count
   internals into `kernel_neg_bin`'s denominator, or print a companion
   `kernel_neg_bin_internal: N of 21` and assert it 0. Demonstrated in §0.3. Today only
   `kernel_parity: 28/28` stands between this and a permanent green.
3. **`tkernel.bin` has no identity check and is built once, ever** (`build_tkernel.sh:26`,
   `battery.sh:26`). `arch_check.py:87-99` does this for `bebop.bin`; the kernel — the component
   whose whole job is soundness — has nothing. Add the same rung, or drop the `[ -s ]` skip so the
   battery rebuilds from source every run.
4. **`typecheck.py:172-173` — a `PARSE` exception must count as a finding.** A total parser
   breakage currently prints the exact success string (§0.5). And `bench/typecheck_neg/` has one
   file whose deletion makes the negative control pass.
5. **`invariants.sh:62` — `TEXTS_RC` is captured and discarded.** One line
   (`[ "$TEXTS_RC" = 0 ] || fail=1`) converts the whole `(texts)` rung from a grep-for-a-sentence
   into the four-way agreement `ROADMAP` A17 already claims it is, and closes four rows at once.
   Lowering that ratchet to 0 today turned a red-on-crash into a green-on-crash.
6. **`guard_artifact.sh` is called with one argument** (`invariants.sh:26`), so the "stale" half of
   the silent-artifact preflight is dead code. Pass an expected md5.
7. **Empty-set greens: assert the denominators.** Five gates go green on zero work, three of them
   demonstrated in this lane:
   `std_golden: 0 pass, 0 fail` · `SUMMARY ok=0 …` · `bpref_parity: agree=0 unsupported=100 …` ·
   `mutation: 0 sensitive, 0 insensitive` · `expansion: 83 gate sources checked`.
   The fix is uniform and cheap: assert the *pass/total* count, not only the failure count.
   `census.py:86-87` already shows how (`in table but not measured` → RED) — copy it.
8. **`bpref.py:800` `except BaseException → exit 2` is scored `unsupported`**, which the battery
   does not assert. Either assert `unsupported <= K`, or split rc 2 (a real oracle bug) from rc 3
   (a declared-unmodelled form). The lane's own header says why.
9. **`parity_driver.sh:24` and `pool_parity.sh:65,77,118,160,173` discard the exit code through
   `| tail -1`.** `construct_parity.sh:55-67` was fixed for exactly this today and documents the
   measured incident; four sibling runners were not. A construct that prints the right number and
   then SIGSEGVs is a MATCH in all of them.
10. **`honest.sh:59,62` publish numbers with no producer check** — a failed compile publishes as a
    record self-compile time; the K6 row's verdict is a string literal beside a number scraped from
    markdown. This is the "status claims are unverified" class, in the script whose name is
    `honest`.
11. **`certcheck.py:312`** — `cert_path` assigned and never used in `--verify-root`; a missing
    `<obl>.cert` skips silently; assert `n_ok == len(obl_paths)`.
12. **`mutate_gate.sh:43-45`** — the baseline is never run, compile failures are counted as kills,
    and rc is lost through a pipe. Four gates report `sensitive` unconditionally because their
    source carries two `gate` lines and only `tail -1` is compared.
13. **Orphan instruments (§1.1).** `roadmap_check.sh` holds the only pin on the gate count and
    nothing runs it. Either wire it into the battery or stop citing its numbers in `ROADMAP.md`.
14. **`arch_check.py:87` writes `/tmp/arch_check_gen.bin`**, a fixed global path, on the identity
    check — two lanes race. Use `$BEBOP_TMP`.
15. **`arch_check.py:553`** — `max_clone_kept_symbols` defaults to `8` in code and is **absent from
    `arch_ratchet.txt`**, along with four other enforced caps. `check_ratchets_are_read` proves
    file→code only and structurally cannot see this.

**Cosmetic but worth one commit each:** `battery.sh:52,71` label f8_dt "parsing + erasure" while
`f8_dt.py:151` explicitly disclaims erasure; `invariants.sh:103` names codes 64/88/90 that the
comment two lines above says were fixed; `invariants.sh:106` names a ratchet key that does not
exist; `battery.sh:70`'s `boxguard` regex is `'.'` and is satisfied by the word "absent".

---

## 4. What I could NOT determine, and what it would take

Stated as gaps rather than guesses. Three of my own working assumptions were corrected by checking
during this audit — the memo-key hash in `run_all.sh` is better keyed than I first read it, and the
`perf.py` alert path is reachable on this box (1840 of 2069 rows carry `valid=1`), which I had
provisionally listed as an unreachable instrument.

1. **Whether `kernel_parity: 28/28` actually discriminates a stale `tkernel.bin`.** §0.4's rank is
   conditional on the 28 committed fixtures covering the elimination restriction and the three
   holes closed in `6726f9a`. *Needs:* build `tkernel.bin` from the parent commit's
   `selfhost/tkernel.bp` and run `kcheck.py --corpus` against it — a compile, which this lane is
   forbidden to run (another lane holds the compile token).
2. **The live value of `trap_unrep` and the four-way `diag_texts` agreement.** I ran
   `trap_census.py --texts` only against nonexistent paths, to characterise the absent-instrument
   path. *Needs:* `python3 tools/trap_census.py --texts --src bebop.bp --bin bebop.bin` against the
   real artifacts. `ROADMAP` claims `18 codes … 18 documented`; `invariants.sh:79-83` claims 18 as
   of today; my absent-path run printed `22 documented`. **These three numbers do not agree and I
   did not resolve which is current.**
3. **Whether `tools/hooks/pre-push` is installed.** `journal_lint.sh`'s only caller is that hook.
   *Needs:* inspecting `.git/hooks` in `/root/dowiz`, which I did not touch (rule 1).
   `arch_check.py:110-111` skips its own hook check in every lane, so the gate cannot answer this
   either.
4. **Whether the 11 orphan oracles (§2.1 row 6) are dead or belong to gates that were renamed.**
   *Needs:* `git log --follow` on each, or a maintainer's answer. Their existence satisfies
   `check_gate_has_oracle` for no gate, so they are at minimum unenforced.
5. **`arch_check.py` has no CHECK 19** — the numbered headers jump from `:502` to `:540`. Whether a
   19th invariant was removed or this is pure renumbering is *unverified; needs*
   `git log -p -- tools/arch_check.py`.
6. **`check_abi.py`'s `sys_allow` and `argpass_window` blast radius.** I established the mechanism
   (a global word allowlist, a 13-word amnesty) but not how much of the corpus they currently
   excuse — `sys=N argpass=M` are printed and asserted nowhere, and I did not run check_abi over
   the ~70 bins. *Needs:* one `python3 tools/check_abi.py` run over `$BINS` and a sum of the two
   counters. Cheap; I skipped it only because the bins are produced by a compile.
7. **Whether `bpref.py` accepts everything `bebop.bin` accepts.** `typecheck census: 0 findings`
   rests entirely on this and nothing in the tree asserts it. *Needs:* a run of the
   `bpref_parity` lane, which requires compiling 100 constructs.

---

## 5. Provenance of this audit

Asked of itself. 71 assertions audited across `tools/battery.sh` and its 8 lanes,
`bench/vs_rust/invariants.sh`'s 10 rungs, `tools/arch_check.py`'s 23 rungs, and 16 outward
producers. **34 are mismatched** (the name does not describe the computer, or the gate can be green
with the named thing broken).

- Rows in §2.1, §2.2 and §0.1–§0.5 I read and anchored myself; every `file:line` was printed with
  `sed -n` before being written.
- Rows marked **[2nd]** in §2.2, §2.3 and §2.4 were read by two parallel readers over
  `arch_check.py`/`census.py`/`check_abi.py`/`typecheck.py` and
  `trap_census.py`/`certcheck.py`/`tv_fragments.py`/`journal_lint.sh`/`guard_artifact.sh`/`honest.sh`/`mutate_gate.sh`/`gen_selfsrc.sh`.
  I re-verified by hand the anchors behind every SOUNDNESS-ranked claim drawn from them —
  `check_abi.py:370-384`, `typecheck.py:166-177`, `guard_artifact.sh` (whole file),
  `honest.sh:44-63`, `certcheck.py:308-342`, `mutate_gate.sh:40-51` — and the `7000` and
  `typecheck_neg` facts. Anything I did not personally re-read at its line is marked **[2nd]** and
  should be treated as one reader's claim.
- Demonstrations in §0.2, §0.3 and §2.1 rows 1/6/7 were executed in this lane and their output is
  reproduced verbatim.
- No code was changed. No `git` command that writes or locks an index was run. No heavy job was
  started; the heaviest thing executed was 49 `.core` fixtures through `seed` (milliseconds each)
  and two Python scripts against nonexistent paths.
