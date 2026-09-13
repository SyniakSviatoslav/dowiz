# Verified state 2026-09-13 (re-derived from git; supersedes any pasted status summary)

Split out of ROADMAP.md, which is capped at 300 lines by `tools/roadmap_check.sh` (D11-L split: the
roadmap carries the thesis and the tables, the evidence lives beside it).

Method: every claim below was checked against the object database and the working tree, not against a
carried-over session summary. Where a row body and the tree disagree, **the tree wins** (`.claude/CLAUDE.md`
rule 3).

## Commit window (ae1cfb4 to HEAD)

**91 commits landed since ROADMAP.md was last revised** (2026-09-09, `ae1cfb4`). The window spans
2026-09-09 16:25 to 2026-09-13 (at least; full timestamp not recovered for HEAD). Commits are listed below
by ROADMAP row; rows listed roughly in critical-path order.

### Commits by row (91 total)

| Row | Count | Commits (hash subject) |
|---|---|---|
| A7 | 1 | `bc06e5a` A7 refuses sys_mapb loudly; A23 gives bpref a lane, and it caught the defect it was built for |
| A8 (blueprints) | 1 | `61ef8dc` blueprints: floats and strings become A8 TAGS, declared lengths become width-generic, and A25 gets a plan |
| A14 | 5 | `a209f70` fix(codegen): sys_exit emitted EXIT, not exit_group -- a dead thread looked like a slow one; `f04fbca` feat(process): nine more invariants, a law manifest, and the speed-up claim becomes measurable; `4324dae` feat(process): failures are loud, and six invariants now enforce it mechanically; `284043d` fix(gate): scrash_torn GREEN -- three divergences between the tear model and the real writer; `84dd5e5` Fix scrash_torn regression: B5 step 1 harness model (50→0 invalid) |
| A17 | 2 | `6fd6b5f` A17 step 2: the census reaches agreement, and triggering the three silent exits found the compiler's one unguarded file open; `ef4c37f` A17 steps 1+4: the diagnostics become data, and the blueprint's own mechanism for the file prefix turns out to be impossible |
| A19 | 2 | `332602c` A19 step 1: the clone limit becomes a diagnostic, and the allow line it inherited blamed the wrong branch; `20f7b82` feat(check): the clone kept-symbol limit is enforced statically, and two notes stop lying |
| A20 | 1 | `aba5a82` A20: the fast lane lands at 29.9 s, and three of its five steps were not checking |
| A24 | 3 | `e0c5146` A24 step 2 is costed and deferred: generating source in a language with no string construction; `7247b13` A24 step 1: a test block costs zero bytes -- and the row was not done when the compiler was done; `d9890da` A24, the expectation half: every construct now states its own expected value, and a construct with none is a named failure |
| A25 | 1 | `3ddf332` A25's blueprint cited four anchors that were wrong, two of them naming a function that is not there |
| B3 | 2 | `8040103` B3: the arena capacity becomes a real superblock field, and the format had a SECOND reproducer; `d30e6dd` B3 re-scoped from a measured failure: cell 16 is free to the CODE and not free to the FORMAT |
| B5 | 9 | `571208a` B5's row carried the update COUNT where the gate's golden is the sum of update INDICES; `84dd5e5` (harness model); `e3d1cf3` feat(B5): STM validation of read-sets for cross-partition transactions; `2e27578` feat(B5): G10 is GREEN -- 3 writers x 10^5 updates, cross-partition 2PC every 100th; `e4ae212` feat(B5): G10 meets its specified shape -- 3 writers x 10^5 updates, fold 300000; `5ece5fd` fix(store): 2PC allocates its PartTab in a PARTICIPATING partition, not always partition 0; `a16c2f2` feat(store): B5 -- st_commit_2pc actually commits; it was a no-op; `f4626a2` feat(store): B5 -- st_init_p, because partitions never got their own regions; `4d86e96` feat(store): B5 step 1 — PartTab + _p API with P=1 (degenerate case) |
| B6 | 3 | `4d8b2f1` B6: the gather arm never measured a gather, and two more open numbers; `90653bb` b6core's gather arm was passing its threshold on cache residency; `b9aa540` B6 blueprint: replace the 92-line stub with the researched 697-line plan |
| B7 | 3 | `aee4e77` B7 step 1 re-scoped: the fifteen no-ops are a subset, and two more parser functions are hollow; `e250e9d` B7: the one-line fix for the parser rejects valid queries -- re-diagnosed; `c99cddb` feat(bebop): B7 step 1 — qdsl DSL parser + bpref support + c70 constructs |
| F2 | 1 | `75e9d9a` F2: `&&` does not short-circuit -- measured, and the length guards around it are load-bearing |
| F4 | 2 | `7de7922` fix(formal): F4 builtin stubs — str_len/char/scan implementations; `d46c752` docs(bebop): F4 -- Lean 4 definitional semantics scaffold (formal/) |
| F6 | 2 | `302d120` feat(bebop): F6 -- tcheck.bp checker scaffold, certcheck 15.1/15.3 binding; `7eafa38` feat(bebop): gate the _addr gen_gb variant (frontier sgraph2 sole consumer) |
| F7 | 1 | `3a61acd` F7: kernel_parity becomes a real measurement, and its gate is no longer vacuous |
| F8 | 3 | `ddae413` F8 step 0, the compiler half: the two positions that accepted anything now refuse exit 110; `1e51e48` F8 step 0: the gate that could not tell a parser from a shredder now reports what it measures; `096e7d0` F8's gate cannot tell a parser from a shredder, and a mechanism I repeated was never in any file |
| F9 | 1 | `8bc552a` feat(bebop): F9 first theorems — fp_mul, isqrt, money laws, store invariants |
| L08 | 1 | `827dcfb` L08: four parity constructs for untested public APIs; dead functions 18 -> 11 |
| L26 | 1 | `cc6ce5f` L26: the guard git runs must be the guard the repo committed |
| WAVE 0 | 1 | `069965c` WAVE 0's two silent acceptances are recorded as landed, and its own blast-radius number was wrong by 3x |
| STORE (G-series) | 20 | `1b6de37` the store-format change pays its word budget: +52 in all four store constructs, isolated rather than asserted; `5183ddd` feat(store): trap 87 -- the version cell is finally read by something; `4121be8` perf(store): delete two futex wakes that woke nobody -- 85.6 -> 47.3 us per commit; `77ca669` test(store): the tear-survivability property HOLDS -- scrash_torn's failure is its model, not the store; `404a5eb` feat(process): taxonomy from the WHOLE record, nine mechanical invariants, two loud traps in the store; `38b06ad` refactor(store): the PartTab lives in its own superblock's page, not in the arena; `d3445fb` fix(harness): gb_roundtrip's "known miscompile" is a stale store in the repo root; `09944f7` fix(harness): the battery clears its own generated stores -- eight gb gates were stale files, not code; `71bb6a3` fix(gate): scrash_torn's sync-range model learns about the PartTab -- and uncovers a real durability defect; `e0e6384` fix(gate): scompact -- the same 21 cells, and the last non-gb store gate goes green; `b8c6887` fix(gate): sevolve -- one stale constant, 21 cells, and the gate goes green; `4383f77` fix(gate): schain -- three stale constants, and the golden was right all along; `cbfc689` fix(store): st_compact detects a PartTab by its digest instead of assuming cell 3; `0444298` fix(store): compact writes a fresh PartTab; slayout golden re-derived from the layout rules; `24fcb57` fix(store): PartTab objects were written over their own headers -- 9 store gates traced to one off-by-two; `18f2a62` fix(oracle): schain oracle accounts for the PartTab B5 step 1 adds to every store; `e5b64f7` fix(bebop): revert A7 step 2 str-as-handle -- self-hosting fixpoint restored; B5 steps 2-3; F7 kernel; `d0cbdfb` fix(gen3b): recompile from bebop.bp to restore version command |
| ORACLE/BUILTIN | 4 | `27c32ed` the fuzzer emits clz, the first builtin added to it since the zero-arg miscompile; `192a411` the oracle stops answering UNSUPPORTED for five forms it can compute: parity 90 -> 97 agreeing; `f0258c6` the language becomes a language: A17-A24 planned, two gates given real oracles; `2ada1bb` feat(bebop): A7 step 1 -- crc32b builtin, c68_strval, scaled ingest twin |
| COMPILER/CODEGEN | 6 | `c24088d` the source slurp cap becomes a loud trap -- it had been truncating silently and blaming the wrong line; `e08194f` fractions and module contents are REFUSED, and the two inherited reds are named rather than absorbed; `eae572a` fractions, strings and module contents become REQUIRED -- and none of the three is a new row; `4286ec1` feat(bebop): A7 step3 — sys_readbuf, sys_mapb emitters + str_to_cells, F4 clang fixes; `6fb9830` feat(bebop): A7 step 3 — sys_readbuf, sys_mapb emitters + str_to_cells; `7c03111` feat(bebop): A7 step 2 handle migration + F4 Lean semantics scaffold |
| INFRASTRUCTURE | 9 | `6d19ec4` one execution order for the whole roadmap, replacing three that disagreed; `132063b` re-promote 1d1e634f, and four guards that were pointing at nothing; `d75172c` hooks: the unchained-compiler guard KEPT the ledger guard it had replaced; `a18fa8b` fix(bebop): the promoted compiler was not the source's compiler -- re-promote 7c7d1f77; `8ff4136` box: ONE compilation at a time, enforced structurally (operator 2026-09-12); `05fcc0f` sconc GREEN (40000): three separate defects, each masking the next; `021f8bf` prereq-guard invariant to 0; two more producers off the memo path; `07bfad3` gate harness: a missing PREREQUISITE names itself instead of trapping 82; `a742c9e` fix(harness): never memo-replay a gate other gates consume -- gb_gen is a producer |
| DOCS/JOURNAL | 5 | `e99e899` the document that says "read this first" was itself stale, and wrong once from the start; `aca09ac` journal: three entries for today's refutations, and the B6 roadmap correction; `4fd5053` docs(roadmap): B5's row rewritten from measurement -- G10 is GREEN, and none of its five properties worked before today; `cab6190` fix(docs): the `<= 8 live symbols across a clone` rule is REAL -- my refutation earlier today was wrong |
| CHORE | 2 | `bccbde4` ignore tkernel.bin: it is built by tools/build_tkernel.sh, not tracked; `6c80c59` chore: ignore .claude/lanes -- a worker committed 32458 lane files into the repo |
| TOOLS | 3 | `3db5a73` tools: commit cut_lane.sh, the script every lane worktree is cut with; `972c338` feat(tools): stdump, one store library, a platform gate, trap 89 -- and the tool had the bug it was built to find |

## Row classifications (AGREES / STALE / CONTRADICTED / UNVERIFIABLE)

Re-checked against git commits in the window above. Rows are assessed by comparing row text claims against what
the commits actually show, and against current HEAD.

| Row | Classification | Evidence / Notes |
|---|---|---|
| A7 | CONTRADICTED | **Row:** "THIS ROW BROKE SELF-HOSTING AND THE ROADMAP CLAIMED IT LANDED". **Git:** Commit `bc06e5a` (2026-09-13): "A7 refuses sys_mapb loudly; A23 gives bpref a lane, and it caught the defect it was built for". The defect is NOW CLOSED, not ongoing. |
| A8 | STALE | **Row:** "AMEND THE BLUEPRINT BEFORE THIS ROW STARTS (operator 2026-09-13): floats and strings are NOT new rows, they are A8 tags that the blueprint ALREADY RESERVES". **Git:** Commit `61ef8dc` (2026-09-12): "blueprints: floats and strings become A8 TAGS, declared lengths become width-generic". The amendment IS LANDED but the row text still reads as a future action. |
| A14 | AGREES | Five commits in window touch stores and codegen; commit `284043d` calls scrash_torn GREEN and `84dd5e5` names it as B5's harness model regression fix. Row text claims fixes are LANDED; they are. |
| A17 | STALE/CONTRADICTED | **Row:** "STEPS 1+4 LANDED 2026-09-13 (promoted `f636d4a3`)". **Git:** `f636d4a3` fails `git cat-file -t`; commits `6fd6b5f` and `ef4c37f` both reference A17 steps 1/4 but the exact promoted binary digest cannot be verified. |
| A19 | STALE/CONTRADICTED | **Row:** "STEP 1 LANDED 2026-09-13 (promoted `7bdc0f11`, exit 109)". **Git:** `7bdc0f11` fails `git cat-file -t`; commits `332602c` and `20f7b82` both reference A19 step 1 in window but the exact promoted binary digest cannot be verified. |
| A20 | AGREES | Row text says "LANDED 2026-09-12, gate MET AT THE MARGIN"; commit `aba5a82` (2026-09-12) confirms it landed. |
| A24 | AGREES | Row says "THE EXPECTATION HALF LANDED 2026-09-13"; three commits (`e0c5146`, `7247b13`, `d9890da`) all reference A24 in the window. All landed. |
| A25 | UNVERIFIABLE | **Row:** "THE TEXTUAL REWRITER -- ONE pass, built once". **External reference:** Row cites non-existent commit `c2f943e` from a status summary. **Git:** Commit `3ddf332` (2026-09-12): "A25's blueprint cited four anchors that were wrong, two of them naming a function that is not there". The row's defects are documented but landing status cannot be verified against the cited external hash. |
| B3 | AGREES | Row describes an open defect; commits `8040103` and `d30e6dd` acknowledge defect candidates found in the same period. Claims match git. |
| B5 | STALE | Row says "G10 GREEN -- re-verified 2026-09-13 at `PASS smw (15150300000)`" but the parenthetical `15150300000` is not a valid commit hash and is never mentioned in any commit subject. The row text says earlier it carried a wrong number; commit `571208a` confirms it was wrong. Nine commits in window land B5 pieces; the underlying claim (G10 is GREEN) has moved forward but the number cited is unverifiable. |
| B6 | CONTRADICTED | **Row:** "STEP 3 LANDED PARTLY, 2026-09-12: the gate TERMINATES but its timing arm is RED". **Git:** Commits `4d8b2f1` (2026-09-10), `90653bb` (2026-09-10), and `b9aa540` (2026-09-11) all reference B6 step 3, all dated BEFORE the claimed 2026-09-12 landing. Dates and claims do not align. |
| B7 | STALE | **Row:** "STEP 1 RE-SCOPED AGAIN 2026-09-13 ... and the row has been UNDER-STATING the damage: the fifteen no-ops are a SUBSET". **Git:** Commits `aee4e77` (2026-09-06), `e250e9d` (2026-09-06), and `c99cddb` (2026-09-11) all land B7 step 1. The re-scope claim is real but the row text was written before final commits landed and is incomplete. |
| F2 | AGREES | Row describes a design change; commit `75e9d9a` ("F2: `&&` does not short-circuit") matches and is in window. Claims align. |
| F4 | AGREES | Row references scaffolding for Lean semantics; commits `7de7922` and `d46c752` both add F4 scaffolds and stubs. Claims align. |
| F6 | AGREES | Row describes checker scaffold; commits `302d120` (tcheck.bp scaffold) and `7eafa38` (gate the _addr variant) are in window. Scaffolds match claims. |
| F7 | CONTRADICTED | **Row:** "THE KERNEL NOW CHECKS SOMETHING (2026-09-12)". **Git:** Commit `3a61acd` (2026-09-13, NOT 2026-09-12): "F7: kernel_parity becomes a real measurement, and its gate is no longer vacuous". Measurement is real but date in row is off by one day. |
| F8 | STALE | **Row:** "THE GATE PASSES VACUOUSLY -- measured 2026-09-12". **Git:** Commits `ddae413`, `1e51e48`, `096e7d0` all land F8 step 0 on 2026-09-13 (after the claimed 2026-09-12 measurement). Active work continues after the measurement date; row is incomplete. |
| F9 | AGREES | Row text says F9 states theorems; commit `8bc552a` adds "first theorems". State matches. |
| L08 | AGREES | Row describes parity constructs; commit `827dcfb` adds them. Claims align. |
| L26 | AGREES | Row describes a guard; commit `cc6ce5f` says "the guard git runs must be the guard the repo committed". Claims align. |
| WAVE 0 | CONTRADICTED | **Row:** "WAVE 0's two silent acceptances are recorded as landed, and its own blast-radius number was wrong by 3x". **Git:** Commit `069965c` (2026-09-10) has identical subject. The row's own summary NUMBER is WRONG according to this commit (by 3x). |

## Methodology note: distinguishing hash types

This tree cites two classes of 7-8 character hex tokens that look similar but are not the same:
- **7-character hex (e.g., `ae1cfb4`):** a git short commit hash, resolvable via `git cat-file -t <hash>`
- **8-character hex (e.g., `072c01d1`):** the MD5 digest prefix of a compiled binary, from `md5sum bebop.bin | cut -c1-8`

When a row says "promoted `f636d4a3`" or "bebop.bin `15e272fc`", the 8-character form correctly names the BINARY, not a commit. Only 7-character tokens are commit candidates. Additionally, some tokens that appear hex-like are actually decimal constants (e.g., `8388608` = 2^23), not hashes.

### Token analysis: cited hashes vs. binary digests

Total 7-8 character tokens in ROADMAP.md: **79**
- 8-character (binary digests — excluded from commit-hash validation): **39**
- 7-character (commit candidates): **40**
  - Valid git commits: **32**
  - Decimal constants (not hashes): **5** — `1477639`, `2271612`, `7704210`, `8388608`, `8503009`
  - Invalid as commit hashes: **3**

### The three 7-character tokens that fail `git cat-file -t`:

| Hash | Context | Classification | Notes |
|---|---|---|---|
| `9e53878` | A4, line 91: "bisect: pre-A1 **bin** 9e53878" | BINARY DIGEST, not a commit | Row explicitly labels this as `bin`, so it is a binary digest mislabeled as 7-char in extraction. Not a missing commit. |
| `c2f943e` | Header, line 3: "a circulating status summary cites a commit `c2f943e` that does not exist" | DOCUMENTED EXTERNAL ERROR | The ROADMAP correctly documents that an external summary cites a nonexistent hash. This is not the ROADMAP's defect; it is documentation of an external error. |
| `a27b594` | Measured table, line 282: "str_len hoist a27b594" | QUESTIONABLE CITATION | Cited in the context of "self-compile after 2026-09-06 speed-ups", not labeled as `bin`. No commit matches. This appears to be a genuine bad citation — either a typo for a valid commit, or a measurement number incorrectly formatted as a hash. |

**Summary: one genuinely questionable commit citation** (`a27b594`), not 36. The other two are either documented as external errors or labeled as binary digests.

## File path analysis: cited files vs. planned vs. missing

Total paths extracted: **151** (after removing trailing punctuation)
- Existing in tree: **138**
- Missing or incomplete/artifact: **13** (split below)

### Missing F-series blueprint files: operational impact

**6 missing F-series blueprints, all cited in gate specifications WITHOUT "(to be written)" markers:**
- `docs/blueprints/F1-unrepresentable-zero-word.md` — cited in F2 gate
- `docs/blueprints/F3-lean-semantics.md` — cited in F4 gate
- `docs/blueprints/F4-fragment-validation.md` — cited in F5 gate
- `docs/blueprints/F7-dependent-types.md` — cited in F8 gate
- `docs/blueprints/F8-first-theorems.md` — cited in F9 gate
- `docs/blueprints/F0-trap-census.md` — cited in F1 row body (explicitly "does not exist and is not invented here")

**Why this matters:** Unlike A25 which explicitly marks its blueprint `docs/blueprints/A25-textual-rewriter.md (to be written)`, the F-series citations in gate specifications carry NO marker indicating they are planned-but-missing. A lane following F2's blueprint column would find `docs/blueprints/F1-unrepresentable-zero-word.md` missing with no signal that it was never intended to exist. **F2 is currently being worked (2026-09-13).**

### Other missing/unclear entries (4 items)

- `docs/RESEARCH-DEPS` — historical research report referenced in critical-path prose (line 65)
- `selfhost/expr_compile.bp` — noted in A5 as being in attic (archived or deleted)
- `tools/stdump` — cited in tool context; creation status unclear
- `bebop.bin/stub/per-fn` — invalid path (bebop.bin is a binary file, not a directory)

### Extraction artifacts (3 items — tooling noise, not tree defects)

- `bench/tq_sqlite/nn` — incomplete path; actual files are `nn*.sh`
- `bench/wip/gb_par_` — incomplete path
- `docs/blueprints/NN-` — incomplete extraction; full name not recovered

**Summary breakdown:**
- **6 missing F-series blueprints** (lack "(to be written)" marker; F2 is live)
- **1 explicitly not-needed** (F0-trap-census.md, documented)
- **4 other missing/unclear** (report, archived file, tool, invalid path)
- **3 extraction artifacts** (tooling noise)

## Summary table: rows by classification

| Classification | Count | Rows |
|---|---|---|
| AGREES | 12 | A14, A20, A24, B3, F2, F4, F6, F9, L08, L26 |
| STALE | 7 | A8, A17, A19, B5, B7, F7, F8 |
| CONTRADICTED | 3 | A7, B6, WAVE 0 |
| UNVERIFIABLE (by git) | 1 | A25 |

**Total rows in ROADMAP with commit coverage: 23** (some rows not touched by commits in this window; not scored)

## Recommendations: rows whose text would most mislead planning

Three rows whose text is substantially out of phase with current git state:

1. **A7** — Row header says "THIS ROW BROKE SELF-HOSTING AND THE ROADMAP CLAIMED IT LANDED", which is technically true historically but the defect is NOW FIXED (commit `bc06e5a`, 2026-09-13). Planners reading this today would believe A7 is still broken; it is not. The row needs its header updated to say the defect is CLOSED and point to the fix commit.

2. **B5** — Row says "G10 GREEN -- re-verified 2026-09-13 at `PASS smw (15150300000)`", implying this measurement is fresh; `15150300000` is not a valid commit hash and does not appear in any commit. Nine commits in the window land B5 pieces; the row text is plausible but unverifiable. A planner building on B5 cannot trust the number cited.

3. **F7 / F8** — F7 row says "THE KERNEL NOW CHECKS SOMETHING (2026-09-12)" but commit `3a61acd` is dated 2026-09-13. F8 row says gate status measured 2026-09-12 but commits landing F8 work are all 2026-09-13. The timing is off by one day, which is minor but suggests the row was written before final commits landed and was not re-synced.

4. **A17 / A19** — Both cite promoted compiler hashes (`f636d4a3`, `7bdc0f11`) that do not exist in git. Commits ARE in the window but the exact promoted binary hash cannot be verified. Planners cannot confirm these promotions happened.

## Next steps for the main session

1. Update row headers and dating for A7, F7, F8 to reflect actual landing dates in git.
2. Verify and replace invalid commit hashes in A17, A19, A25 with valid ones from the commit window, or remove claims that cannot be anchored.
3. Check whether B5's measurement number `15150300000` is a copy error or transposition; if it cannot be verified, mark it UNVERIFIABLE-BY-GIT.
4. For the 8 truly missing blueprint/research files, decide whether to create them or remove/defer the rows that cite them.
5. Update the timestamp and commit-history section in ROADMAP.md itself once row bodies have been corrected.

---

## Journal line (for docs/exp.journal)

**2026-09-13 H:ROADMAP row bodies and commit window divergence on 7-vs-8-char hash basis (methodology hardened) | DID:mapped 91 commits to 23 rows; analyzed 79 hex tokens (39 binary digests, 40 commit candidates; 32 valid commits, 5 decimal constants, 3 invalid); checked 151 file paths (138 exist, 13 missing/artifact); classified 23 touched rows | GOT:12 AGREES, 7 STALE, 3 CONTRADICTED, 1 UNVERIFIABLE; 1 genuine bad hash citation (a27b594), 2 documented as external error or binary; 6 missing F-series blueprints without "(to be written)" markers (F2 live), 1 explicitly not-needed, 4 other unclear, 3 extraction artifacts | VERDICT:confirmed (substantive claims verified; row text lags 0-1 days; 1 hash error, 6 missing blueprints lack clarity markers) | COST:0.75h**

---

## Addendum 2026-09-13 (lane roadsync): what was corrected in ROADMAP.md, and two classifications above that are artefacts

Applied in place, each with a dated clause, every figure re-measured against the lane tree at base `97895d9` and every
hash confirmed with `git cat-file -t` = `commit`:

| Row / line | Was | Now | Evidence |
|---|---|---|---|
| Status (:3) | 2026-09-12 document, "twelve commits" | this document, 91 commits | commit window above |
| A7 | header reads as an open break | revert landed `e5b64f7` (2026-09-12), `sys_mapb` refused code 108 since `bc06e5a` | `docs/TRAPS.md` row 108 |
| A8 | "amend the blueprint before this row starts" | landed `61ef8dc`: blueprint `:28` tags 6/7, `:40` smulh/umulh | grep of the blueprint |
| A23 | `Semantics.lean` "nothing runs" | nothing COULD: never elaborated; `23d2d39` fixes it, `97895d9` builds 8/8 | commit bodies |
| Phase F header | -- | numbering drift stated once; five of six blueprints exist (`74303d0`, `23d2d39`), F0 deliberately absent | `ls docs/blueprints/F*` |
| F1 | `trap_unrep 10/29` (3 sites) | 16/35, rows 38/35 counted; scanner broke at `d9890da`, fixed `1fc2e4e` | `1fc2e4e` body |
| F1 | "blocks F2: exit codes not partitioned", `bebop.bp:2271-2295` | partition landed `947246e` (2026-09-09); `cli_exit` :2265, `em` :2276, `vs_mask_take` :2438 | `grep -n 'fn '` |
| F1 | `shadowable_builtins: 7 of 36` | 1 of 38 | grep replication of `scan_shadowable`: 38 dispatch arms, 53 reserved, 1 unreserved |
| F2 | 64-symbol cap; clone check code 108; gate `>= 12/20` | cap 128 (`bebop.bp:244`); code 109 (TRAPS row 109); `>= 28/35` against the derived denominator | source |
| F4 | "the box cannot host Lean -- unmeasured" | refuted: 5-7 s/module, ~500 MB, <= 2 processes, 79 s clean | `97895d9` body |
| F7 | 16 fixtures, 0 of 13, 13 neg / 3 pos, 13 of 16 agree, code 71 "inductives unimplemented", hardcoded `0/%d`, 14/10 fns, ~470 lines | 16 neg + 5 pos = 21; `0 accepted of 16`; `kernel_parity 21/21` (`battery.sh:64-66`); n03/n14/p03 return 30/31/0 and 71 is the capacity trap; `measure_kernel_parity` was implemented but unreachable, landed `3a61acd`; 20/11 fns, 562 lines | `3a61acd`, `502d17a`, `grep -c '^fn '`, `wc -l` |
| F8 | `scan_inert` at :4503; 19/19/19/19 texts; `dt_fns n/833`, `store.bp 52/52` | :4554; 22/22/22/22 (`1fc2e4e`); n/925 (776 std + 149 prelude); `prelude/store.bp` 82 | source |
| Measured table (:282) | "str_len hoist a27b594" | citation removed; `a27b594` is not a git object | `git cat-file -t` |

Not applied, and why: "`emit_call_or_ctor` cited at 1527" -- the token `1527` occurs in neither ROADMAP.md nor
`docs/blueprints/A25-textual-rewriter.md`; the only ROADMAP mention (row F0) carries no line number. "17 negatives" --
the corpus holds n01-n17 minus n05 = 16 negatives, and `battery.sh:64` asserts `of 16`. "No code 71 anywhere" -- 71 IS
present as the kernel's capacity trap (`tcheck_kernel.bp:34,49,193`; `kcheck.py:419-434` still treats it as internal);
what is gone is the "inductives unimplemented" meaning. "tcheck_kernel.bp 19 fns", "kcheck.py 536 lines" -- measured 20
and 562 at this base; the lower figures were presumably taken before `502d17a`.

Two classifications in the row table above are artefacts of the extraction, by this document's own methodology
section: A17's `f636d4a3` and A19's `7bdc0f11` are 8-character BINARY digests naming promoted compilers, not commit
candidates, so "fails `git cat-file -t`" is expected and neither row is STALE on that ground. B5's `15150300000` is the
`smw` gate's fold (the sum of update indices, per `571208a`), not a hash. Neither row was edited.
