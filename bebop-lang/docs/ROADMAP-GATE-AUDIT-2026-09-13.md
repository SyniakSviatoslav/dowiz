# ROADMAP gate audit — 2026-09-13 (lane `roadgates`)

The question asked of every OPEN row: **this row states a gate. Would that gate actually
detect the row being false — and does anything RUN it?**

Scope: every row in `ROADMAP.md` lines 86–203 whose task cell does not open with a
LANDED / CLOSED / REFUTED verdict. That rule yields **40 open rows** of 58 total
(A×16, B×8, C×5, D×2, E×1, F×8). All 40 are in the table; depth varies and is marked.

Method, and what was actually run (read-only, no compile, no heavy job):
* `python3 tools/trap_census.py` and `--rows` — real run, rc=0.
* `python3 tools/tv_fragments.py bebop.bin` — real run, rc=1.
* `python3 tools/kcheck.py --corpus bench/kernel_neg` — real run, rc=0 (twin only; `tkernel.bin` absent in this lane).
* `python3 tools/builtin_surface.py`, `python3 tools/undef_census.py` — real runs, rc=0.
* A monkeypatched in-process re-run of `trap_census.build()` to test the census's
  sensitivity to work landing (below, F2).
* `ls` / `grep` for every cited instrument and every automatic caller.
Nothing was compiled. Where a claim needs a compile I say so and mark it unverified.

The four automatic runners in this tree are `tools/chain.sh` → `tools/battery.sh` →
(`bench/vs_rust/std_golden.sh` via `std_par.sh`, `construct_parity.sh`, `parity_driver.sh`,
`pool_parity.sh`, `bench/oracles/run_all.sh`, `bpref_parity.sh`, `kcheck.py`,
`invariants.sh`, `diag_check.sh`, `check_words.py`, `f8_dt.py`, `trap_census.py`), plus
`tools/perf.py` from `chain.sh`. **Anything not reachable from that list runs in no lane.**
`tools/roadmap_check.sh` — the only thing that could catch a stale gate cell — is itself
called by nothing, and checks only the TG-DONE `ok=` count, never a phase row's gate cell.

---

## Table

Legend: **E** instrument exists · **W** wired to an automatic runner · **F** would go red if
the row were false · **N** gate's number still derivable.

### F-series (done first, per the brief)

| row | gate, quoted | E | W | F | N | verdict + evidence |
|---|---|---|---|---|---|---|
| F2 | ``trap_unrep >= 12/20`` … ``16/35 today, this row's twelve on top of it = >= 28/35``; ``gen_avoid`` shrinks by 12 | yes | yes | **no** | **no** | **STALE-NUMBER + WRONG-QUANTITY.** `tools/trap_census.py` exists and IS wired (`battery.sh:53,73`). But (a) the denominator MOVES WITH the numerator: `trap_census.py:614-621` appends a fresh `code-N` closed row for every code that gains a `neg/` construct and has no curated row. Proved by monkeypatching `scan_neg_expects` to simulate the F2 lane landing a `neg` for the nesting-cap trap (code 95): the `nesting-cap` row stayed **OPEN**, a duplicate `code-95` closed row appeared, and the line read **`trap_unrep: 17/36`** — not 17/35. `>= 28/35` is therefore not reachable as literal arithmetic. (b) Of the 23 curated rows, **exactly one** (`unresolved-call` → `c52_undef`) carries a `negname`; the other 22 have `negname=None`, so `has_neg` is False and `closed` is hard-wired False no matter what lands. The numerator cannot move on the work this row describes without a human editing `trap_census.py`. (c) `battery.sh:73` asserts `'^trap_unrep: 16/35$'` — an **exact-equality freeze**, so the first trap F2 closes turns the battery **RED**. (d) `gen_avoid` shrinks by 12 is unsatisfiable: `tools/undef_census.py` measures `gen_avoid: 3 PROVISIONAL`. Also `unresolved-call` (this row's item "115") is ALREADY counted closed, so the "twelve on top" double-counts. |
| F3 | ``bounds_census: checked+proven+hoisted = 100 % of index sites``; … ``stepping_sites: 0``; ``trap_unrep >= 18/20`` | **no** | no | no | **no** | **ABSENT-INSTRUMENT + STALE-NUMBER.** `grep -rl bounds_census tools bench formal selfhost` → nothing; same for `stepping_sites`. Both appear only in docs/blueprints. And `trap_unrep >= 18/20` is the **dead denominator** shape exactly: the census counts **35**, so this clause can be neither met nor failed as written. Note F3 is BLOCKED pending a width-generic blueprint, so the absence of the census is expected — the stale `/20` is not. |
| F4 | ``lean_conformance: c/86``; ``builtin_spec: b/36``; ``bpref_diff: 0``; ``lean_results_hash == sha256(formal/*.lean)`` | part | **no** | no | part | **UNWIRED + ABSENT-INSTRUMENT.** `formal/` exists (8 modules incl. `Conformance.lean`, `Semantics.lean`, `Theorems.lean`) and `lean_conformance` appears inside three `.lean` files. But no script emits a `lean_conformance:` line, nothing emits `builtin_spec` or `bpref_diff`, the "~70-line python step in battery.sh" the row describes is **not in `battery.sh`**, and no runner checks the results hash. Blind-sample risk on the one half that could be built cheaply: `bpref_diff` uses `tools/bpref.py` as oracle, and `builtin_surface.py` reports **bpref impl 12 / stub 24 / absent 2 of 38**, while `bpref.py:748` returns 0 for `clock_ms` or ANY name starting `sys_`. A differential against an oracle that stubs 24 of 38 builtins and silently zeroes every syscall is insensitive to most of what it is for. |
| F5 | ``tv_fragments: v/t`` over the 86 constructs; ``trace_zone_words: 0`` | yes | **no** | n/a | n/a | **UNWIRED.** `tools/tv_fragments.py` exists and is now HONEST — run today on `bebop.bin` it printed `tv_fragments: NOT MEASURED (no trace: bebop.trace absent …)` at **rc=1**, and its docstring says it has no rc=0 path. That is the correct state for an unbuilt row. But `grep -rn tv_fragments` outside the file itself finds **zero callers**: not `battery.sh`, not `chain.sh`, not `std_golden.sh`. Nothing emits `trace_zone_words` at all. So the row's only artifact still runs in no lane — the same condition that let its predecessor print `PASS (0/0)` for its whole life. |
| F6 | ``cert_checked`` — obligations discharged and machine-checked …; ``checker_neg 0/N`` on deliberately corrupted certificates | part | **no** | **no** | n/a | **ABSENT-INSTRUMENT (gate name) + UNWIRED.** `tools/certgen.py` (12 KB) and `tools/certcheck.py` (15 KB) exist and the row documents real measured negatives. But **nothing in the tree emits `cert_checked` or `checker_neg`** (`grep -rl` over `tools bench formal selfhost` → nothing), and neither script has an automatic caller. The four negatives the row cites were measured by hand on 2026-09-09 and have not run since. A regression in `certcheck.py` today is invisible. |
| F7 | ``kernel_checked`` — theorems whose terms the Bebop kernel accepts, counting to total | **no** | no | **no** | n/a | **WRONG-QUANTITY (the "wrong computer" pattern, one level up) + STALE-NUMBER.** The GATE COLUMN names `kernel_checked`; nothing emits it, and there are zero theorems, so the stated gate is vacuous at 0/0. What the battery actually asserts is three DIFFERENT quantities the gate column never names: `kernel_neg: 0 accepted of 21`, `kernel_neg_bin: 0 accepted of 21`, `kernel_parity: 28/28` (`battery.sh:65-67`). Separately, the row's own prose is **stale by today's own count**: it says "the corpus is 16 negatives + 5 positives = 21 … `kernel_parity: 21/21`, all three asserted by `tools/battery.sh:64-66`". Measured now: `bench/kernel_neg` holds **28 fixtures = 21 negatives + 7 positives**; `kcheck.py` prints `kernel_neg: 0 accepted of 21` / `kernel_pos: 0 rejected of 7`; the assertions are at **:65-67**, and expect **28/28**. **Blind-sample gap:** `kernel_pos` is PRINTED and NOT ASSERTED. A kernel that rejected every term would satisfy `kernel_neg` and `kernel_neg_bin` outright; only `kernel_parity 28/28` catches it, and only through the Python twin — if the twin regresses the same way, parity agrees and the battery stays green. |
| F8 | ``dt_fns: n/925`` selfhost fns with checked types; ``elab_neg: 0 accepted of N``; ``K5_typed <= 1.5x K5`` | **no** | no | **no** | **yes** | **ABSENT-INSTRUMENT + WRONG-QUANTITY.** Nothing emits `dt_fns`, `elab_neg` or `K5_typed`. `tools/f8_dt.py` IS wired (`battery.sh:52,72`) but measures a different quantity — a ratchet at 0 garbage-accepting positions — and says so in words. So step 0 has a real gate and (a)–(d) have none. The denominator IS derivable and correct: re-counted today, `selfhost/std` 776 `^fn` + `selfhost/prelude` 149 = **925**, `money.bp` 17, `store.bp` 82. One path slip: the cell implies `fp.bp` is under `selfhost/std`; it is `selfhost/prelude/fp.bp` (2 fns, so the 2/2 is right). **`K5_typed <= 1.5x K5` is a classic blind-sample trap**: if annotation parsing were silently skipped, K5_typed would equal K5 and the gate would pass at its best possible value. It needs a positive that proves annotations were read. |
| F9 | ``theorems: >= 3`` … each with a kernel-checked term and an LRAT certificate; ``theorem-sample.bp``/``theorem-false.bp`` retired or re-homed | **no** | no | **no** | n/a | **ABSENT-INSTRUMENT — and this is the row that answers the false-theorem question.** Nothing emits `theorems:`. `samples/theorem-false.bp` contains `theorem bad : 1 + 1 = 3 := refl` and compiles rc=0. Reading `bebop.bp:4554` (`scan_inert`) and its two call sites (`:6098-6099` theorem line, `:6254-6255` fn header): the validator is **lexical** — it rejects only characters outside `{whitespace, identifier chars, - > < = + * / . : , ( ) [ ] ;}`. Every character in `theorem bad : 1 + 1 = 3 := refl` is on the allow list, so F8 step 0 passes it by construction and is right to (it never claimed to check truth). No other gate looks at it: F7's kernel reads `.core` fixtures, not `.bp` theorems; F6's `cert_checked` does not exist; F9's `theorems:` does not exist. **A false theorem is green today and no row's gate can currently go red on it.** The row that OWNS it is F9, seconded by F6. |

### B-series

| row | gate, quoted | E | W | F | N | verdict + evidence |
|---|---|---|---|---|---|---|
| B1 | ``1000 torn trials, 0 invalid reopens (same harness over sqlite WAL); commits/s row; recovery row`` | yes | yes | part | yes | **SOUND (with one soft half).** `bench/vs_rust/scrash_torn.sh` and `scrash.sh` are both called from `std_golden.sh`, and `gate scrash_torn 0` is asserted; `std_golden` is wired through `std_par.sh` at `battery.sh:40` and asserted at `:58`. The torn-trial half would go red on a lost durability guarantee. The "commits/s row" and "recovery row" halves are REPORT rows — they record numbers, nothing thresholds them, so a 10x durability-path regression that still reopens cleanly is invisible. |
| B2 | ``join >= 10x sqlite AND >= 0.7x best Rust on both key distributions``; scan twin rows; CSR build profile row | yes | **no** | yes* | yes | **UNWIRED.** `bench/vs_rust/twins_b2.sh` implements the gate EXACTLY (`:260-261`, `gate = 'MET' if (r_sql >= 10 and r_rust >= 0.7)`, and `rust_best` correctly includes `rust_csr` so an algorithm win cannot be credited as a language win). Callers: `bench/tq_sqlite/join_sqlite.py` and `bench/vs_rust/B2-PREP.md` only — **no automatic runner**. The gate is well-designed and has not run since 2026-09-09. *F is yes only when someone runs it by hand. |
| B3 | ``G9a round-trip; G9b LAGraph-style folds BFS/PR/TC/CC/SSSP == python oracle; first-query latency <= 1 ms (tier 0) and specialised <= 50 ms; each kernel a construct`` | part | part | part | yes | **UNWIRED (half).** The pool-identity half is wired: `gate gb_pool`, `gate gb_pool_reuse`, `gate gb_pool_abi` are in `std_golden.sh:972-995`. The fold and latency halves are not: `bench/vs_rust/gbpool.sh` has **no caller** (only a `.bp` mentions it), and no runner thresholds "<= 1 ms" or "<= 50 ms". The row's own open defect — `st_alloc` (`store.bp:324`) bumping `tx[2]` with no bound test while its twin `st_alloc_p` (`:561`) exits 80 first — has no gate at all: nothing would go red if `st_alloc` walked off the mapping again. |
| B4 | gate cell is an adjudication paragraph: ``ns/edge MET at 435 vs <= 500 … the <= 10 ms stall clause is RENEGOTIATED to the measured 97 ms bound`` | n/a | n/a | **no** | part | **WRONG-QUANTITY (a results cell, not a gate).** The column carries verdicts and superseded numbers rather than a threshold anything can be tested against. The folds it names (`nbr0 500446467359` etc.) are real committed goldens, but the cell does not say which script asserts them. A renegotiated-in-place bound (10 ms → 97 ms) is a gate that moved to meet the measurement; it should be re-stated so the next measurement can fail it. |
| B5 | ``G10: 3 writers x 10^5 updates on disjoint partitions -- MET, 303000, ~10.5 s, reproduced`` | yes | yes | yes | **no** | **STALE-NUMBER, in the gate cell only.** The row's own body says 303000 is wrong and the golden is `15150300000`; the GATE COLUMN was never updated. Verified against the committed source: `bench/vs_rust/std_golden.sh:728` reads `gate smw 15150300000`. The instrument is sound and wired (std_golden → `battery.sh:40,58`) and would detect a lost update. The cell would mislead anyone re-deriving it. Related real hazard the row records: `run_all.sh` selects gates with `grep -E '^gate [A-Za-z0-9_]+ -?[0-9]+ '`, so a golden written as a shell variable silently DROPS out of the oracle check instead of failing. |
| B6 | ``K6 scan x1.4-2.2 on 3 cores; BFS/PR rows; no lost updates with B5`` | yes | **no** | **no** | **no** | **WRONG-QUANTITY + UNWIRED + STALE-NUMBER — three at once.** (a) `bench/vs_rust/b6core.sh` has **no caller anywhere** (`grep -rln b6core.sh tools bench .github` → nothing). (b) What IS wired is `std_golden.sh:740 gate b6core 485239494593740800` — a deterministic FOLD VALUE at W=1. A fold value cannot detect a speedup regression; the multi-core claim is not the quantity being gated. (c) The roadmap number and the instrument's number DISAGREE: the row says `x1.4-2.2`, `b6core.sh:141` enforces `scan >= 2.50x, gather >= 1.80x`. (d) The row's own body reports scan **1.00x** and gather **1.36x** today — RED by the instrument, invisible to the battery. The gather arm's earlier green was the textbook blind sample: `idx[i] & (n-1)` at n=10^7 masked to 16384 distinct cells, so it measured L2 residency, not a gather. |
| B7 | ``Q6 >= 10x, Q1 >= 5x sqlite native; first/repeat latency rows; rank-3 construct`` | yes | **no** | **no** | ? | **UNWIRED + blind sample set.** `bench/tq_sqlite/tpch_twin.sh`, `tpch_q6.bp`, `tpch_q1.bp` exist; callers are two `.md` files — **no automatic runner**. Worse, the row's own step-1 finding says the thing being gated is hollow: `qdsl_ckw` is a stub that always returns 1, `qdsl_ident` consumes to end of string, and "the frozen golden 41000 encodes whatever THAT parse produces". A speed gate over a parser that accepts malformed queries measures the wrong program, and the frozen golden actively resists the fix. Ranked high because it is a correctness hole wearing a performance gate. |
| B8 | ``byte-exact Rust oracles; the G-rows of this workload in REPORT-honest / RESULT-sgraph`` | part | part | **no** | n/a | **WRONG-QUANTITY (gate points at documents) + vacuous-success risk.** "the G-rows in REPORT-honest / RESULT-sgraph" names two markdown reports, not a threshold. The "byte-exact Rust oracles" half runs through `bench/oracles/run_all.sh`, which **memoises**: `run_all.sh:8,12,15` replays a stored GREEN from `$RUN_ALL_MEMO` when inputs are unchanged, so `SUMMARY ok=117` can be green with no oracle having executed; `money`/`ordfsm` shell out to `cargo` and reportedly went unexercised in three consecutive runs (carried forward from today's oracle audit; not re-measured here). |

### A-series

| row | gate, quoted (abridged) | E | W | F | N | verdict + evidence |
|---|---|---|---|---|---|---|
| A1 | ``push_words == 0; k4 <= 13 … k4_ms <= 1.15x Rust twin; c55-c61`` | yes | yes | yes | yes | **SOUND** (effectively closed; it trips the open-filter only because the cell opens lowercase "register model, one commit (landed…)"). Word counts ride `construct_parity`/`word_budget`, which are wired. |
| A4 | ``0 CRASH/DIVERGE, TRAP-82 = 0`` | yes | part | yes | yes | **SOUND via chain only.** TRAP-82 is counted by `bench/fuzz/fuzz.sh:68-73` and thresholded by `tools/perf.py:156` (`{v} TRAP-82 … 0 tolerated`), and `perf.py run` is invoked from `chain.sh:55`. `fuzz.sh` itself is NOT in `battery.sh`, so a plain battery cannot go red on this. |
| A5 | ``chain GREEN; c69_index_roundtrip, c70_ptrfree; K5 <= +8 %, K6 ns/row <= +20 %`` | **no** | part | part | yes | **ABSENT-INSTRUMENT.** `bench/parity_constructs/c69_index_roundtrip.bp` exists; **`c70_ptrfree` does not exist** — the c70 slot is occupied by `c70_csel.bp`, `c70_qdsl.bp`, `c70_qdsl_neg.bp`. Half of a gate on a row declared COMPLETE cites a construct that was never created (or was renamed without the cell following). Same shape A19 confessed for `c126`/`c127`. |
| A6 | ``step 1 MET: brk #0x51 = 0 …; Step 2 gate: c67_deeprec (10^5 recursion), fuzz TRAP-81 = 0, TRAP-82 = 0, RSS row`` | yes | part | part | yes | **UNWIRED (half).** `c67_deeprec.bp` exists and runs under `construct_parity` (wired). The fuzz halves run only through `fuzz.sh`, which no battery lane calls, and **`fuzz.sh:71-73` explicitly scores TRAP-81 as a PASS** — so "fuzz TRAP-81 = 0" cannot fail in the harness that measures it. "RSS row" is a report row with no threshold. |
| A7 | ``c68_strval; 100 MB ingest twin: raw <= 1.5x best Rust, maxrss <= 1.5x file size`` | yes | part | part | yes | **UNWIRED (half).** `c68_strval.bp` exists and is wired via `construct_parity`. `bench/vs_rust/ingest_twin.py` and `ingest.bp` exist but have no automatic caller, so neither ratio is ever taken. |
| A8 | ``G7 file size <= 1.2x sqlite; K6 ns/row -2x; typecheck oracle == bpref`` | yes | part | **no** | yes | **WRONG-QUANTITY / blind sample set — the bpref clause.** `tools/typecheck.py` is wired (via `invariants.sh`, `bpref_parity.sh`). But "typecheck oracle == bpref" leans on `tools/bpref.py`, and measured today: `builtin_surface.py` reports **bpref impl 12 / stub 24 / absent 2 of 38** builtins, and `bpref.py:748-749` is `if name == 'clock_ms' or name.startswith('sys_'): return 0`. So `sys_this_does_not_exist(1,2)` evaluates to 0 rather than raising `UnsupportedForm`. Agreement with an oracle that stubs two thirds of the builtin surface and silently zeroes every syscall is not evidence the types are right. Inherited by A9, A23 and F4. |
| A9 | ``K5 -10 % (scan); K6 ns/row <= 4 with a Rust scan twin; each builtin a construct + bpref stub`` | yes | part | part | yes | **UNWIRED (perf halves) + inherits A8's bpref blindness.** "each builtin a construct + bpref **stub**" literally gates on a stub existing, which is exactly the state `builtin_surface.py` reports as `stub 24`. A stub that returns 0 agrees with anything. |
| A11 | ``(a) decision gate 20 % of battery wall; (b) byte-identical lane summaries x3`` | **no** | no | **no** | ? | **ABSENT-INSTRUMENT.** Nothing measures a battery-wall fraction per lane, and nothing compares lane summaries for byte identity. Both clauses are unfalsifiable as written. |
| A16 | ``the core after defunctionalisation and monomorphisation is first-order and monomorphic, PROVEN by a census rather than asserted`` | **no** | no | **no** | n/a | **ABSENT-INSTRUMENT (expected — row not started).** No census script exists for first-orderness/monomorphism. The gate is well-shaped (it demands a census, not an assertion) but has no instrument yet; flagging so it is written WITH the feature, not after. |
| A17 | ``grep -c 'trap 81'`` = 0 (step 3) | **no** | **no** | **no** | yes | **UNWIRED + WRONG-QUANTITY.** `grep -rn "trap 81\|TRAP-81"` over `tools bench` finds it ONLY inside `bench/fuzz/fuzz.sh` (a counter) and `perf.py` (a journal field) — no runner greps a binary or a log for it, and `fuzz.sh:71` says TRAP-81 "is by design and stays a pass". The gate as written would need a target to grep; none is named. |
| A19 | ``neg/c141_clone9`` COMPILEFAIL:109 and ``c142_clone8`` = 201 … ``c128_clone12`` = 201 (step 2) | yes | yes | yes | yes | **SOUND.** `bench/parity_constructs/neg/c141_clone9.bp` and `c142_clone8.bp` both exist and run under `construct_parity.sh` (wired, `battery.sh:41,59`). `c128_clone12` is absent but belongs to step 2, which has not landed — the gate naming its own deliverable is correct. This row is the model: it corrected its own `c126`/`c127` phantoms in place. |
| A18 | ``c124_condreturn`` MATCH at a hand value; ``neg/c125_noopif`` COMPILEFAIL:<code>; WORD_DELTA 0 on c05/c35/c36/c70_csel/c71 | no* | no | n/a | yes | **Gate names its own deliverable (not a defect).** `c124`/`c125` absent because the row is unstarted; `c70_csel.bp` and the WORD_DELTA machinery exist. The "hand value (smallest-prime-factor table, not the program's own division)" clause is exactly right — it forbids the self-frozen comparison that `b6core`'s old `fold_par == fold_seq` fell into. |
| A21 | ``c129_multiret`` MATCH at a hand value; ``neg/c130_multiret_arity`` COMPILEFAIL:<code>; … a double-x0-push mutant FAILS c129 | no* | no | n/a | yes | **Gate names its own deliverable; well-shaped.** The "a double-x0-push mutant FAILS c129" clause is a mutation check written INTO the gate — the single best defence against the blind-sample pattern found elsewhere today. Keep it. |
| A22 | ``c131_structs2`` MATCH … (weights chosen so no field permutation is invariant); ``neg/c132_untagged_field``; slayout/schain/sevolve/scompact/scrash_torn/smw unchanged | no* | no | n/a | yes | **Gate names its own deliverable; well-shaped.** The permutation-invariance clause is a real falsifiability design. The named store gates all exist in `std_golden.sh` and are wired. |
| A24 | ``construct parity: pass=n fail=0 no_expect=0``; ``neg/c134_testfn_inside``; a headerless c01 copy -> ``no_expect=1`` RED | yes | **part** | **no** | yes | **UNWIRED (half) — a concrete, cheap hole.** `construct_parity.sh:115` DOES print `no_expect=$NO_EXPECT`. But `battery.sh:59` asserts only `'^construct parity: pass=[1-9][0-9]* fail=0'` — the regex stops at `fail=0` and never looks at `no_expect`. So `no_expect=99` reads GREEN in every battery, and the row's own "headerless c01 copy → RED" test would NOT go red today. `c133_testblock.bp`, `c133_testblock_twin.bp` and `c134_testfn_inside.bp` all exist. Current `no_expect` value unverified (needs a compile). |
| A25 | ``rewriter_idempotent``: flattening a flattened source is a fixpoint; ``use_expand`` and ``mod_expand`` share one implementation (one ``grep``-able entry point); … 0 words added to ``bebop.bin`` | no* | no | n/a | yes | **Gate names its own deliverable.** Note the "one `grep`-able entry point" clause is a gate on SOURCE SHAPE, which `tools/arch_check.py` (wired via `invariants.sh`) is the right home for — worth saying so in the row so it does not land unwired. |

### C / D / E

| row | gate, quoted (abridged) | E | W | F | N | verdict + evidence |
|---|---|---|---|---|---|---|
| C1 | ``10^4 hostile generated kernels -> 0 SIGSEGV/TRAP-82 and 100 % loud traps; K6 ns/row with checks <= 1.2x without; sgraph2 frontier fold unchanged`` | part | **part** | **no** | yes | **UNWIRED on the soundness half.** `c110_fence.bp`, `c111_kernelfn.bp` and `neg/c112_kernelsys` exist and run under `construct_parity` (wired), so the dialect check is gated. But **nothing generates or runs 10^4 hostile kernels in any automatic runner** — TRAP-82 counting lives in `fuzz.sh`/`perf.py`, neither of which drives a hostile-kernel corpus. The `<= 1.2x` timing clause has no instrument. `sgraph2.sh` IS called from `std_golden.sh`, so the fold half is wired. Ranked high: this is the row that stands between a hostile kernel and a SIGSEGV. |
| C2 | ``REFUTATION STEP GREEN (typecheck 0 findings, oracles ok=114 mismatch=0 missing=0, std_golden 114/0)``; ``<= 1.2x SAME-SHAPE sqlite (<= 25.3 MB; measured 24.0 MB) and point lookup <= 2x … (<= 362 ns; measured 138 ns at k=3)`` | yes | **no** | part | **no** | **STALE-NUMBER + UNWIRED.** The cell's `ok=114` / `std_golden 114/0` are stale: measured today `grep -c '^gate ' bench/vs_rust/std_golden.sh` = **117**, and `battery.sh:63` expects `ok=[1-9][0-9]*`. The size/latency half is measured by `bench/vs_rust/scolp.sh`, which has **no caller** — so the 25.3 MB and 362 ns guards are never re-taken. The guards themselves are well-derived (the row corrected its own 450 ns → 170 ns baseline). |
| C4 | ``fold equality maintained == recomputed, before AND after compaction; maintenance below noise; scrash_torn 0/50`` | yes | part | part | yes | **SOUND (partial), one clause soft.** `scrash_torn` and `scompact.bp` are both reachable from `std_golden.sh` (wired). "maintenance below noise" has no threshold and no instrument — unfalsifiable as written. Not re-measured here. |
| C5 | ``on Zipf input: within 1.1x of the better fixed flavour AND better than 1.5x of the worse one; fold identical to the single-flavour run`` | **no** | no | n/a | yes | **ABSENT-INSTRUMENT (expected — row not started).** No Zipf flavour-switching harness exists. The gate's shape is good: two-sided, so a switcher that always picks one flavour fails the second clause. |
| C6 | ``none until the trigger fires: a target node with a DAX device AND a harness that can cut power to it`` | n/a | n/a | n/a | n/a | **SOUND BY DECLARATION.** The gate says explicitly that there is none and names the precondition. This is the honest form of an absent gate and should be the template for F5/F1/F0-style deliberate non-wiring. |
| D4 | ``traversal 1.06x REFUTED; write side -8.0 MB and -40 pct stall CONFIRMED; log row +28 pct REFUTED, so nothing lands until A8 emits the unpack`` | n/a | n/a | **no** | part | **WRONG-QUANTITY (a results cell, not a gate).** Like B4, the column holds verdicts. There is no threshold a future measurement could fail, so the row cannot regress detectably; it can only be re-argued. |
| D5 | ``docs/TRUST-CHAIN.md`` names every artifact hash … AND states what is NOT witnessed; ``tools/ddc.sh --gate`` prints the measurement and exits RED with ``DDC: NOT ESTABLISHED`` (deliberately NOT wired into battery.sh or std_golden.sh, or it would fail every run) | yes | **no, by design** | yes | yes | **SOUND.** Both artifacts exist (`docs/TRUST-CHAIN.md` 20 KB, `tools/ddc.sh` 12.7 KB), `ddc.sh:179` prints `DDC: NOT ESTABLISHED` and `:32` documents the deliberate non-wiring in the script itself. The gate is falsifiable, runnable by hand, and its unwiredness is declared in BOTH the row and the instrument. This is the best gate in the open set and the pattern F5 should copy. |
| E4 | ``bytes/slot <= 3 with ns/slot not above A8's u32, fold identical -- the same gate as D4, reached earlier`` | **no** | no | n/a | part | **ABSENT-INSTRUMENT (expected — row not started).** Inherits D4's problem by reference: "the same gate as D4" points at a cell that currently holds verdicts rather than a threshold, so the inherited gate is underspecified. |

---

## Ranked: rows whose gate would NOT catch the row being false

Ranked by consequence. Soundness failures outrank performance regressions; within a tier,
"green today while the thing is broken" outranks "runs nowhere".

1. **F9 (with F6 and F8 (a)–(d)) — a false theorem is green and no gate can go red on it.**
   `samples/theorem-false.bp`'s `theorem bad : 1 + 1 = 3 := refl` passes `scan_inert`'s
   character allow-list by construction (`bebop.bp:4554`, call site `:6098`). `theorems:`,
   `cert_checked`, `checker_neg`, `elab_neg` — none of the four has a producer anywhere in
   the tree. This is the phase's whole purpose and it is ungated end to end.
2. **F7 — the kernel's own gate is not the gate the row states, and the positives arm is
   unasserted.** `kernel_checked` has no instrument (vacuous at 0/0). The battery asserts
   `kernel_neg`/`kernel_neg_bin`/`kernel_parity` instead; `kernel_pos: 0 rejected of 7` is
   printed and never asserted, so a reject-everything kernel is caught only through the
   Python twin. Same family as today's "wrong computer" finding, one level up.
3. **F2 — the trap gate is arithmetically unreachable and progress on it turns the battery
   RED.** Proved by re-running the census with a simulated landing: 16/35 → **17/36**.
   22 of 23 curated rows can never close without a human editing `trap_census.py`.
   `battery.sh:73` freezes the line at exactly `16/35`. `gen_avoid` shrink-by-12 against a
   measured 3.
4. **C1 — the hostile-kernel soundness gate runs nowhere.** "10^4 hostile kernels → 0
   SIGSEGV/TRAP-82" has no generator and no runner. The row's dialect constructs are gated;
   the memory-safety claim is not.
5. **F3 — bounds soundness: `bounds_census` and `stepping_sites` do not exist, and
   `trap_unrep >= 18/20` has a dead denominator** (the census counts 35).
6. **A8 / A9 / F4 / every bpref-leaning row — the differential oracle is systematically
   insensitive.** `bpref.py:748` returns 0 for any `sys_*`; `builtin_surface.py` measures
   **stub 24 / impl 12 / absent 2 of 38**. Agreement with this oracle is weak evidence.
7. **B7 — `Q6 >= 10x` gates a query engine whose parser the row itself found hollow**
   (`qdsl_ckw` always returns 1; `qdsl_ident` runs to end of string), the frozen golden
   41000 encodes the stub's parse, and `tpch_twin.sh` has no automatic caller.
8. **B6 — three defects at once**: the instrument has no caller, the wired `gate b6core`
   checks a fold VALUE (cannot detect a speedup regression), and the roadmap's `x1.4-2.2`
   disagrees with the instrument's `>= 2.50x`. Today's 1.00x is RED and invisible.
9. **B3 — `st_alloc`'s missing bounds check has no gate.** The pool-identity gates are
   wired; the defect the row is open FOR is not.
10. **A24 — `no_expect=0` is printed and not asserted** (`construct_parity.sh:115` vs
    `battery.sh:59`). A one-line regex fix; until then the row's own RED test cannot fire.
11. **F5 — instrument honest, wired to nothing.** Zero callers; `trace_zone_words` absent.
12. **B2 — the gate is implemented exactly right and runs in no lane.**
13. **A17 — `grep -c 'trap 81' = 0` names no target, and the one harness that counts
    TRAP-81 scores it a pass by design.**
14. **A5 — `c70_ptrfree` does not exist** (the c70 slot holds `c70_csel`/`c70_qdsl`), on a
    row declared COMPLETE.
15. **A11 — both clauses unfalsifiable** (no battery-wall fraction, no lane-summary compare).
16. **B8 / B4 / D4 / E4 — gate cells that hold reports or verdicts instead of thresholds**;
    B8 additionally inherits `run_all.sh`'s memoised replay.
17. **C2 / B5 / F0 / F1 — stale numbers inside gate cells** (see below). Lowest consequence:
    the instruments are sound and wired; the cells mislead the reader.

### Stale numbers found in gate cells (all verified against the tree today)

| where | cell says | tree says | source |
|---|---|---|---|
| B5 gate | `MET, 303000` | `gate smw 15150300000` | `bench/vs_rust/std_golden.sh:728` |
| C2 gate | `oracles ok=114 … std_golden 114/0` | 117 gates | `grep -c '^gate ' std_golden.sh` = 117 |
| F0 gate | `builtins known to all four: 36 of 36` … `34 of 36 today` | **35 of 38** | `python3 tools/builtin_surface.py` |
| F1 gate | "**NOT wired into battery.sh**" | **it IS wired** | `battery.sh:53` runs it, `:73` asserts it |
| F3 gate | `trap_unrep >= 18/20` | denominator is 35 | `python3 tools/trap_census.py` |
| F7 prose | "16 negatives + 5 positives = 21", "battery.sh:64-66", `kernel_parity 21/21` | 21 neg + 7 pos = 28; `:65-67`; expects `28/28` | `kcheck.py` run + `battery.sh` |
| F8 gate | `fp.bp` under `selfhost/std` | `selfhost/prelude/fp.bp` | `find . -name fp.bp` |

F1's confirmed-correct halves, for the record: `shadowable_builtins: 1 of 38` and
`undef_census: 2` both re-derived today and match the cell.

---

## What I did not reach

* **No row was compiled or benchmarked.** Every "would it go red" judgement about a
  threshold (K5, K6, Q6, ns/row, file size) is from reading the instrument, not from
  running it. Rule 2 forbade the heavy jobs.
* **A24's current `no_expect` value is unverified** — the regex hole in `battery.sh:59` is
  certain; whether any construct is currently missing its EXPECT header needs a
  `construct_parity.sh` run.
* **`tkernel.bin` is absent in this lane tree**, so `kernel_neg_bin` and `kernel_parity`
  were `NOT MEASURED` here. F7's binary-side numbers are taken from `battery.sh`'s expects
  (`0 accepted of 21`, `28/28`), not re-measured.
* **`run_all.sh`'s memoised-replay and the `money`/`ordfsm` cargo shell-outs were not
  re-measured** — carried forward from today's oracle audit as stated, and B8's verdict
  rests partly on it. Worth an independent check.
* **A2, A3, A10, A12–A15, A20, A23, C3, D1–D3, E1–E3, F0, F1** were read but not given
  full rows: their task cells open with a LANDED/REFUTED verdict and they fall outside the
  stated scope. F0 and F1 appear above only in the stale-number table, because their gate
  cells make claims about wiring that the tree contradicts.
* **`tools/roadmap_check.sh` is itself unwired** (no caller but `split_roadmap.py`) and
  checks only `ROADMAP.md <= 300 lines` (currently **292** — 8 from the cap), TASKS.md
  regeneration, T-id presence, and the TG-DONE `ok=` count. It cannot catch any of the
  stale gate cells above. That is the cheapest single fix suggested by this audit, and it
  is a fix, so this lane did not make it.
