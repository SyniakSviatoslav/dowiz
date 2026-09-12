# Verified state 2026-09-12 (re-derived from git; supersedes any pasted status summary)

Split out of ROADMAP.md, which is capped at 300 lines by `tools/roadmap_check.sh` (D11-L split: the
roadmap carries the thesis and the tables, the evidence lives beside it).

Method: every claim below was checked against the object database and the working tree, not against a
carried-over session summary. Where a row body and the tree disagree, **the tree wins** (`.claude/CLAUDE.md`
rule 3). Row bodies further down were NOT rewritten except A6 -- read them with this section in hand.

**Ground truth.** HEAD = `4286ec1` (2026-09-11 21:48). That commit carried 9 pointer-named binary blobs,
~260 MB, that an A7-step3 run wrote into the repo root -- their names are raw `0xa8`-prefixed addresses, so a
filename argument was handed a pointer instead of a string; **that emitter path is an open defect, tracked
under A7**. On 2026-09-12 the blobs were removed from the index and moved out of the repo (along with
`sbench.sqlite-wal` and a stray control-char filename, and `*.sqlite-wal` / `*.sqlite-shm` were added to
`.gitignore`); they existed in no other commit, so amending `4286ec1` removes them from history entirely. Local `main` is **95 commits ahead of `origin/main`
(`f6da66d`)**, 0 behind; push is blocked for want of a key, so *nothing since 2026-09-07 is on the remote*.
13 stale `.claude/worktrees/agent-*` checkouts are detached at ancestors of `main` -- dangling lanes, no
divergent work.

**Counts.** 53 task rows (A 19, B 8, C 6, D 6, E 4, F 10), classified by the rows' own
LANDED / REFUTED / CLOSED / OPEN markers: **27 done, 13 in progress, 11 not started, 2 deliberately parked**
(A4's fuzz window runs last; C6 is "CONDITIONAL -- do not start").

**Landed after this file's last edit (`ae1cfb4`, 2026-09-09 16:25) and not yet folded into the row bodies:**
`2ada1bb` A7 step 1 (crc32b builtin, c68_strval, scaled ingest twin) · `7eafa38` gate the `_addr` gen_gb
variant · `302d120` F6 tcheck.bp checker scaffold + certcheck 15.1/15.3 binding · `d46c752` F4 Lean 4
definitional-semantics scaffold · `7c03111` A7 step 2 handle migration · `c99cddb` B7 step 1 (qdsl parser +
bpref support + c70 constructs) · `7de7922` F4 builtin stubs (str_len/char/scan) · `6fb9830` + `4286ec1`
A7 step 3 (sys_readbuf, sys_mapb emitters, str_to_cells) · `4d86e96` B5 step 1 (PartTab + `_p` API, P=1
degenerate) · `8bc552a` F9 first theorem statements · `d0cbdfb` recompile to restore the version command.
Earlier and equally unfolded: `7704210` + `fe06c54` A6 step 2A/2B (computed frames, `emit_bl` saves x15).

**A status summary claiming a commit `c2f943e` ("A8+A9+A13+A15+A16+B5+B6+B8+F3+F7+F8+F9") is circulating.
That commit does not exist** -- `git cat-file -t c2f943e` fails, and it is absent from history, the reflog,
the stashes and all 13 worktrees. Only A13 and A15 of that list are real (both 2026-09-08, unrelated). The
rest, checked by grep on 2026-09-12:

| claimed | evidence in the tree | verdict |
|---|---|---|
| A8 typed tables / u32 cells | nothing in `bebop.bp` | NOT IMPLEMENTED |
| A9 `cmp_mask` / `sum64` / `umulh` | nothing; only `scan` landed (2026-09-08) | NOT IMPLEMENTED |
| A16 closures / generics / dependent types | no parser support | NOT IMPLEMENTED |
| B5 step 2/3 `st_commit_p`, `st_commit_2pc`, O_EXCL locks | `store.bp` has `st_commit{,_m,_sync,_batch,_c,_at}` and **no** `_p` / `_2pc`; step 1 (`4d86e96`) is real | STEPS 2-3 NOT IMPLEMENTED |
| B6 `par_run` multi-core | `par_run` appears only inside comments of `gb_par_{mxm,mxv,reduce,scan}.bp`; never defined or called | NOT IMPLEMENTED |
| B8 `wlog.bp` end-to-end workload | no `wlog*` file exists; only `B8_PREP_ANALYSIS.md` | NOT IMPLEMENTED |
| F3 bounds-by-type / trap 84 | no "trap 84" in `bebop.bp` | NOT IMPLEMENTED |
| F5 translation validation | `tools/tv_fragments.py` tooling only | NOT STARTED as a gate |
| F7 dependent-calculus kernel | `selfhost/tcheck_kernel.bp` is real but ~30 fns against the ~170 this row's own gate needs, and it is referenced nowhere else -- unwired, untested | SKELETON ONLY |
| F8 annotation parsing / VC emission | `tools/f8_dt.py` gate exists; `bebop.bp` has **zero** matches for `requires` / `ensures` / `theorem` / `invariant`, so the gate would fail if run | GATE WITHOUT A COMPILER |
| F9 machine-checked theorems | `formal/Bebop/Theorems.lean` states 7 items as Lean **`axiom`s, 0 proved theorems**; the F9 gate demands a kernel-checked term plus an LRAT certificate | STATED, NOT PROVED |

Two further claims from the same summary also fail: "B7 step2 GREEN" (only step 1 exists, `c99cddb`) and
"10+ uncommitted files" (the tree is clean). "B4 GREEN" is half-true at best -- B4's own body says step 3 is
its only open half. "F4 = 121/121 oracle entries" is true only as *declared fixtures* in `Conformance.lean`;
the Lean semantics have not been run against them, and F4 carries no LANDED marker.

**What that means for the critical path:** the whole of Phase F is earlier than it looks. F7 is the load-bearing
row -- F9 cannot become real proofs until the kernel exists, and F8's gate cannot pass until `bebop.bp` parses
the annotation syntax. Nothing in A8/A9/A16/B5(2-3)/B6/B8/F3 has been started in code despite design docs
(`B5_IMPLEMENTATION_DESIGN.md`, `F2_IMPLEMENTATION_PLAN.md`, `F_PHASE_STATUS.md`, `A16-parsing-annotations.md`)
existing for several of them. Design documents are not evidence; a gate number in a committed script is.

