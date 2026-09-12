# AGENTS.md — Debugging & Process Laws (v2)

Supersedes v1 (same day, same author, more scars). Provenance: historical
audit of the FULL session — from ntt_filter and the sc-class hunt through
M1 seed, M2 syscalls, M3 self-bootstrap. Aggregate finding: **dozens of
avoidable cycles (hours) went to process failures, while genuinely hard
root causes fell fast whenever the method was disciplined.** The bottleneck
is procedure, not difficulty.

v1 rules survive here, re-homed: #1,#2,#6 → LAWS; #3,#4,#5,#7,#8 → LADDER/
HEURISTICS; #9,#10 → JOURNAL/KEEP. New in v2: the Occam Ladder protocol,
hypothesis discipline, parallel-agent protocol, symptom index, and rules
about rules themselves.

---

## 0. THE OCCAM LADDER — mandatory ordered protocol

Debug bottom-up. Never jump tiers; every skip in history cost hours.
Each tier lists its check and its typical cost.

**T0 · Evidence hygiene (seconds, always first)**
- Every printed value gets an explicit expected-vs-got comparison IN THE
  SAME BREATH. A diagnostic you don't check against an expectation is noise.
- Decompose suspicious results arithmetically ("which w,nr,ok produce this
  number?") before running anything new.
- Paired structures (names↔offsets, counts↔counts) get `assert len(a)==len(b)`
  and diff prints. Incident: 176 vs 138 both printed, mismatch ignored →
  entry jumped mid-function → SIGSEGV hunt that the assert would have ended.

**T1 · Identity (seconds)**
- Is the executed artifact the one just built? (timestamp, word count,
  path). Incident class: stale binaries and stale /tmp scratch files
  (y9/y8/z8.full) produced phantom mismatches repeatedly, historically.
- Scratch files are content-addressed or regenerated immediately before
  any comparison. Never reuse yesterday's .full.
- Cache model known (.becache key = crc32(compiler)+crc32(kernel)) — trust
  it without positive evidence of staleness; suspect your pipeline first.

**T2 · Known classes (minutes — consult BUGFIXES.md + this file)**
Ordered by historical hit rate for interp≠native/JIT divergence:
1. >8 live symbols → spill machinery. Shrink probe to ≤8 bindings FIRST.
   (syscall builtins × spills broke twice before this was tested.)
2. Fast-path bail-outs: if-with-call-in-condition retargets literal
   branches into `mov x0,x0` copies of the condition value (fpC).
3. IO scratch zone (x28-8192) overlap with live data; NUL termination of
   every buffer handed to the kernel, EVERY call (never trust fresh mmap).
4. Harness execution model: exec_words runs JIT TWICE (warmup+ref); state
   leaks between calls (scratch, arena cursor, fds).
5. Register protocol: x19–x26 symbols; x15 spill base; x27/x28 arena (never
   saved/restored by a prologue — T126: doing so rolled allocations back);
   caller-saved x0–x14 across bl; pop() already emits ldr+addSP.
6. Meta-language traps: nested `if` inside expressions segfaults the
   interpreted compiler; dangling/duplicate else-if links remap registers.

**T3 · Mechanical verification of artifacts (minutes)**
- Inserted words: re-disassemble the generated stream at the insertion
  site and diff against the reference block. Always. (~10 s.)
- Syscall wrappers: register table comment must exist and be complete
  (x0..x5,x8 each traced to a producer word). Missing x2=len shipped twice.
- Interp mirror present and semantically equal? One engine green proves
  nothing about the other.

**T4 · Bisection & minimal repro (minutes)**
- Shrink until the delta isolates ONE mechanism (p2/p4/p5 ladder style;
  io_probe variants A–E). Five tiny programs beat one accreting program.
- When several features flip behavior together, suspect the shared feature
  first (historical: shared dispatch chain corruption).
- Two consecutive failures along one design axis ⇒ STOP tuning it, pivot
  to a design that eliminates the constraint entirely (64K→16M→1M ENOMEM
  failures; file-mmap worked instantly).

**T5 · Deep tools (last resort)**
- gdb-on-JIT: anchor break on `__clear_cache` (post-mmap); never fixed
  addresses pre-run. Crash triage: `info proc mappings` → rwx map base →
  offset=pc−base → word#=offset/4 → disassemble that range of the .bin.
  exec_words is stripped — no symbol breaks.
- Single-stepping / instrumented builds only after T0–T4 exhausted.

---

## HYPOTHESIS DISCIPLINE (metacognition during the hunt)

- **Declare the space before probing.** Write down ≥3 candidate causes,
  ranked simplest-first, BEFORE the first experiment. If you cannot name
  three, you haven't understood the symptom yet — go read code.
- **Falsifiability per experiment.** Before running, state what result
  would KILL the hypothesis. An experiment that can't fail proves nothing.
- **Timebox per hypothesis:** two failed falsification attempts ⇒ drop it,
  move to next candidate (or fan out agents — see below). Historical cost:
  io_probe spiraled ~10 iterations because the spills candidate was tested
  last despite being documented lore.
- **One-line journal per experiment:** `H:<hyp> | DID:<action> | GOT:<x> |
  VERDICT:<confirmed/killed/inconclusive>`. This is the record later audits
  reconstruct from; it also enforces T0 hygiene mechanically.
- **Observed ≠ proven.** A rule derived while the system was in a broken
  state inherits the brokenness. Incident: "[RULE] STRICT branch evaluation
  confirmed mechanically" was written from a mangled dispatch-chain
  experiment; the lazy2 micro-test later proved branches ARE lazy. Mark
  conclusions OBSERVED (correlation) until reproduced on a clean state
  (mechanism).

## PARALLEL AGENT PROTOCOL

Single-threaded hunts through independent checks wasted wall-clock all
session. Fan out when:

- **≥2 independent hypotheses at the same tier** → one agent per hypothesis,
  each builds its own probe and returns a verdict. Example split for an
  interp≠JIT mismatch: agent A tests ≤8-symbol version (spills?), agent B
  greps BUGFIXES/docs for matching symptom classes, main thread runs the
  T1 identity checks.
- **Independent gates** (std_golden, parity driver, construct parity) →
  run concurrently, integrate verdicts.
- **Artifact prep**: reference-word extraction (asm→objdump→words) can run
  while another agent writes the emitter skeleton / interp mirror.

Isolation rules (hard-won):
- Each agent works in ITS OWN scratch namespace (`/tmp/opencode/<agent>-<topic>/`);
  shared mutable scratch caused real phantom bugs historically.
- Agents return structured verdicts ONLY: `VERDICT: <killed|confirmed|error>
  EVIDENCE: <exact command output lines>`. No narratives to re-read.
- Main thread integrates and owns all writes to the repo; agents never edit
  shared source concurrently.

## SYMPTOM → START-TIER INDEX (quick lookup)

| Symptom | Start at |
|---|---|
| Result decomposes into correct parts + garbage tail | T2.1 spills; T2.3 scratch |
| Works in interp, wrong in JIT | T2.1, T3 register table, T2.4 |
| Works outside gdb, crashes inside (or vice versa) | T2.4 harness model |
| Worked on first run, fails on second | T2.4 (warmup+ref) + T2.3 NUL |
| Words look right but behave wrong | T3 execution ground truth; T2.5 protocol |
| Entry/jump lands somewhere weird | T0 paired-count asserts (OFF table) |
| Everything breaks after a "small" emitter edit | T2.6 chain links; T3 diff |
| Silent SMALL wrong number (e.g. 7 vs 14M) | L11 entry identity FIRST; then spills |
| Crash site moves between fns across builds | name-map off-by-one: verify attribution (fnmap), don't trust it |
| Garbage array BASE from stack (x2=0/29) | T2.1 spills: check bind-store slot vs lookup-read slot in disasm |
| Compiler "traps" / exit 90-91 on trivial input | T1 artifact identity FIRST (empty-.bin class, journal 1788288248) |

## RULES ABOUT RULES (meta)

- A rule exists only WITH: incident link, trigger (when it applies),
  action, and rough cost of compliance. Narrative-only rules die unused —
  v1's "objdump-only constants" predates the very transcription errors it
  forbids, because it had no trigger attached to the moment of typing.
- Laws (mechanical, zero-tolerance: word pipeline, register tables,
  equality asserts, post-insert diff) are separated from heuristics
  (ladder order, pivots). Violating a law is a bug in the work; violating
  a heuristic is a judgment call to log.
- Sunset clause: if a rule fires false twice, rewrite or retire it in the
  same commit that discovers it. Keep the index short enough to recall at
  the decision moment.

## AGENT WORKFLOW (zero-C daemon — agentd replaced by spectral verification)

Agent work is gate-and-journal driven: every experiment states its
expected value (LAW L10), returns a VERDICT, and logs to `docs/exp.journal`.
Gates (`std_golden`, parity driver, construct parity) run via
`./seed/build/seed bebop.bin compile … && ./seed/build/seed <bin>` with
no external tooling.

- Cold-start rule: anything executed repeatedly lives in the self-hosted
  compiler pipeline (seed + bebop.bin); python/bash one-offs are for single
  use only.
- Spectral invariants replace C-oracle checks: the fixpoint (bb2 == bb3)
  and spectral drift (spectral_drift) are the primary correctness signals.

## TOOLING & NAVIGATION (agent work only — runtime stays zero-C)

- **Navigation is grep + AST-level parsing via the self-hosted compiler**:
  `grep/rg` for raw text hunts; `.bp` source is structured enough that
  regex suffices. No external C tools remain.
- **Recurring agent operations** run directly through the self-hosted
  pipeline (`./seed/build/seed bebop.bin compile …`); python one-offs
  are for single-use only (cold-start rule).

## LAWS (condensed, zero-tolerance)

L1. Words: asm → objdump → script → LE int → scripted insert → **disassembly
    diff at insertion site**. Hand-typing a constant is a defect.
    Dead letter (D13 item 7): mechanised by tools/check_words.py, run in battery.sh.
L2. Syscall/bl wrappers: full register table comment before emission;
    every argument register has a producer.
L3. Scripted source patches carry `assert old.count==1`; analysis tools
    run to completion and their outputs are sanity-checked before use.
L4. Buffers handed to the kernel are explicitly terminated every call.
L5. Both engines (interp + JIT) verified for any new builtin/emitter; a
    silently-wrong reference is reverted, not documented-and-kept
    (compound-ops precedent).
L6. Before touching a subsystem: load its living-memory nodes and read the
    causal map (who calls whom, which contracts bind). Debugging without
    the map repeats solved bugs (rule/map-before-work node).
L8. NO allocations inside while bodies (cells `[..]`, ctors, zeros): the
    frame-heap bump never resets per iteration -> monotonic escape -> SIGSEGV
    (collect_fns 3.9MB climb).
L9. Runtime self-source is a GENERATED artifact: always regenerate
    /tmp scratch .bp from selfhost/ in-repo in the SAME commit; hand-copied
    sources drift silently and poison entire debug ladders (2026-08-25:
    stale 158B K1 file read all day). Generator: tools/gen_selfsrc.sh.
L10. Every probe run states its expected value AT ISSUE TIME
     (auto-verdict+journal). A run without expectation is noise, not evidence.
L11. ENTRY IDENTITY for packed binaries: entry=0 means FIRST fn IN THE FILE,
     not "the interesting one". Before interpreting ANY result, confirm which
     fn executes (fnmap w<entry>). Incident class: probes returning callee's
     value for hours.

L12. ARTIFACT IDENTITY before use: any .bin used as a compiler or baseline
     must pass tools/guard_artifact.sh (size>0 + optional md5) FIRST; the
     three gate harnesses already preflight bebop.bin. exit 90/91 from the
     seed on a "trivial" input means invalid/empty .bin — never chase logic
     before T1. Incident: 2026-09-02 empty git-show artifact cost hours.

L13. IMMUTABLE BASELINES: golden binaries live in bench/golden/<name>-<rev>.bin
     with .sha256 sidecars and are never overwritten; extract only via
     tools/fetch_golden.sh (which verifies non-empty). bebop.bin promotion
     happens ONLY from a fixpoint-green rebuild; never cp an unverified
     binary over the working compiler.

L14. SINGLE-VARIABLE DIFFS: signature changes (adding a param/arg), model
     changes, and cap changes each land as their own commit with their own
     probe. Mixed commits produced the cascading-fix chain of 2026-09-02.
     Dead letter (D13 item 12): mechanised by tools/hooks/commit-msg.

L15. FALSIFIABLE PROBE BEFORE fpC/SPILL EDITS: before touching the
     branchless-cond or spill machinery, run (or add) the minimal probe for
     the construct (bench/parity_constructs/c22-c24 are the canonical
     shapes: match-binding, spilled-arg call, spilled-array-in-if). Fix
     only after the probe fails; then freeze the probe.
     Dead letter (D13 item 12): mechanised by tools/hooks/commit-msg.

L16. push/pop emit EXACTLY their canonical words — never extra words
     (check-traps in push/pop polluted the stream and cascaded into
     900-s self-compiles; journal 1788288246). Model state is
     bookkeeping-only: guarded slot writes (0<=d<96), depth clamped >=0.

L17. A `gate` line in std_golden.sh is accepted only with a committed
     independent oracle `bench/oracles/<gate>.py` in the SAME commit
     (T36, 2026-09-04); `bench/oracles/run_all.sh` must stay missing=0.

L7. str-vs-int comparisons in any analysis mirror of .bp code are banned:
    char() returns ints in Bebop; python mirrors must compare ord()s. The
    138-names=0 bug was exactly this class.

## LAW: FAILURES ARE LOUD (operator, 2026-09-12 — binding, no exceptions)

The dominant cost on this project is not fixing defects. It is finding out where a
symptom came from. Nearly every defect that cost a day was SILENT rather than subtle:

- a child hit `sys_exit(80)` and died; the parent waited on a flag for ever, so a dead
  thread and a slow one were indistinguishable and it was written off as "performance";
- past 8 kept symbols across a `sys_clone` the children are lost with NO trap and NO
  diagnostic — `sys_clone` returns valid TIDs and nothing ever writes;
- `got=` empty had THREE different causes in one day — a trap, a stale artifact, and the
  missing-binary guard — and all three printed identically;
- a fold of exactly 0 named none of the five guard factors that could have produced it;
- `st_commit_2pc` committed nothing at all and reported success;
- partitions wrote over each other and every write "succeeded".

So:

1. **Never let a failure be indistinguishable from success or from slowness.** If a thing
   can fail, its failure must SAY SO, with a code or a name, at the point it happens.
2. **Never discard stderr on a run.** `tools/arch_check.py` **loud-failure** enforces this
   for every gate and tool script; a fuzzer that classifies failures by exit code is the
   only exemption, and it is listed in `tools/arch_ratchet.txt` with its reason.
3. **Never report an empty result as empty.** `std_golden.sh`'s `gate()` turns an empty
   value into a NAMED cause using the run's exit code — `EMPTY(rc=82 trap ...)`,
   `EMPTY(rc=124 TIMEOUT -- a hang, or a child died and the parent waits for ever)`.
   Anything else that collects a result owes the same.
4. **A wait must be bounded and must name what it waited for.** An unbounded wait on a flag
   another thread sets converts every child-side failure into a hang. Bound it, and on
   expiry report WHICH worker never arrived.
5. **A guard that annihilates a value must be able to say which factor was false.** A fold
   of 0 from a product of five checks is a bug report with the useful part removed.
6. **A missing PREREQUISITE must name itself, never trap.** A gate that consumes an artifact an
   earlier block produced must assert the artifact is there before it runs. MEASURED 2026-09-12,
   one unchanged binary both ways: `gb_pool_test.bin` against a warm `$GBT` returns the golden
   `-4783772994166464769`; with `gb_gen.store` absent it dies `rc=82 trap: SIGSEGV/SIGBUS`. A
   missing input file was reported as a wild memory access — the diagnosis named the wrong
   subsystem, and that is worse than no diagnosis. `std_golden.sh`'s `need_file()` is the
   mechanism; `tools/arch_check.py` **prereq-guard** enforces it.
6. **An exit code is not a result.** `rc=0` with a fold of 0 is a failure. Quote the value.
7. **When two things must agree, print BOTH on disagreement**, never just the verdict —
   `MISMATCH(same=.../foreign=...)` is the shape to copy.

The test of this law is simple: if a failure required you to add instrumentation before you
could tell what happened, the instrumentation belongs in the code permanently.

## DEFECT TAXONOMY — every class that has cost this project time, and what now catches it

Written 2026-09-12 from the whole record. Each row is a class we have actually been bitten
by, not a hypothetical. **Where a check exists, it is mechanical and runs in `invariants.sh`
via `tools/arch_check.py` — read that file, do not re-derive these by hand.**

| # | Defect class | What it looked like | Caught by |
|---|---|---|---|
| 1 | **Claimed work that does not exist** | a roadmap row marked LANDED citing commit `c2f943e`, which is not a valid object; a dozen rows with no code behind them | no automatic check possible — require BOTH a roadmap marker AND a grep/commit in the tree, and say CONFIRMED / CLAIMED-ONLY / ROADMAP-OPEN per row |
| 2 | **The promoted artifact is not the source's** | `bebop.bin` was `292b8953` while `compile(bebop.bp)` was `7c7d1f77`; every gate number was measured against a compiler no source produces | `arch_check` **artifact-identity**. Note `chain.sh`'s fixpoint does NOT catch this: it proves the SOURCE has a fixpoint, not that the promoted binary IS it |
| 3 | **A number taken from a comment** | `<= 8 live symbols across a clone`; `KNOWN RED: compiler miscompile` on a gate that now passes; a harness model that predicted 400 cells per commit when it was 421 | discipline: a number in a comment is a HYPOTHESIS with a date on it. Re-measure before building on it |
| 4 | **Stale generated artifacts** | nine gb gates RED because a leftover `.store` no longer matched what `st_open` maps; trap 82, no output, reads exactly like a miscompile | `arch_check` **stale-store**, plus the battery clearing `$BEBOP_TMP/gb_*.store` before it starts |
| 5 | **A memo replay that skips a side effect** | the memo prints a PASSed gate's cached stdout WITHOUT running it, so the store other gates read is never written and they all fold to 0 | `arch_check` **producer-memo** |
| 6 | **Fitting a golden instead of deriving one** | setting a frozen expected value to whatever the program prints destroys the only independent check the gate has | discipline: build/run the ORACLE first. It decides which side is stale. slayout's oracle moved to meet bebop; schain's, sevolve's and scompact's agreed with the golden and bebop was wrong |
| 7 | **A probe that measures a race** | a parent read shared cells immediately after the spawn loop; its "201" was luck. Three conclusions were built on it, and all three were wrong | discipline: the parent MUST wait for what it reads, and every threaded measurement runs at least twice |
| 8 | **A probe that varies the wrong thing** | padding with `let a1 = 1;` proves nothing about spill pressure — constants are rematerialised. With symbols that must be KEPT the limit is 8, not 32 | discipline: vary the KIND as well as the count |
| 9 | **A guard factor annihilating a result** | a fold of exactly 0 is almost never arithmetic; it is a product of checks with one false factor | discipline: when a gate prints 0, print each factor separately before anything else |
| 10 | **A layout change without its dependents** | B5 step 1 added a 21-cell PartTab per commit and updated nothing that depended on it: five gates, two oracles and one harness model, found one at a time over a day | discipline: a change to a written format is not done until every reader, oracle, golden and harness model is re-derived in the SAME commit |
| 11 | **A gate that goes green for the wrong reason** | a `live` change made sevolve match its golden by restoring pre-B5 semantics — and broke schain, because `used` still counted the PartTab and `live` no longer did | discipline: any change that makes one gate agree while making another disagree is fitting, not fixing. Run the WHOLE battery before believing a single gate |
| 12 | **Wrong argument positions** | `seed smw.bin x 2 1000` makes `argv[2]` the string `"x"`, and `sm_atoi("x")` is 72 — so a "2 writer" run spawned 72. Every measurement taken that way was void | discipline: argv[0] is the seed loader, argv[1] the .bin, argv[2] the first user argument. Echo the parsed values back before trusting a run |
| 13 | **Files no one can review** | `bebop.bp` is 7700 lines; three prelude files passed 900 | `arch_check` **file-size** with a RATCHET in `tools/arch_ratchet.txt`: new files cap at 800 lines, existing offenders are listed as DEBT with the split that retires them, and the numbers may only ever go DOWN |
| 14 | **Nested function definitions** | bebop has no closures, so an indented `fn` is always a scope mistake | `arch_check` **nested-fn** |

## DEFECT TAXONOMY, PART 2 — the whole record, not one session

Mined 2026-09-12 from all 938 `docs/exp.journal` entries and 2937 lines of `HISTORY.md`.
Frequencies are journal GOT-field matches, so they measure how often a class was WRITTEN
ABOUT, which is a fair proxy for how often it cost time.

| Class | Journal hits | The shape it takes here | What catches it |
|---|---|---|---|
| **codegen / miscompile** | 69 | the largest class by far, and 38 % of HISTORY's defects. Characteristic form: two calls in ONE expression clobber each other — `sys_arena_end() + sys_arena_base()` evaluated to `base + base`, and one expression gave 0 where two `let`s gave the right answer. Also: register-window corruption between generations, array double-write producing zero output | differential testing against `tools/bpref.py` (the reference interpreter) over the construct corpus; `bench/parity_constructs`; the fuzzer. A miscompile that only shows in ONE spelling of an expression is invisible to a gate that uses the other spelling — vary the SPELLING, not just the value |
| **layout / off-by-N** | 39 | an offset convention broken by one writer while every reader keeps the old one. The PartTab header off-by-two is the type specimen: `st_alloc` writes a header at `off`, a writer wrote payload from `off`, and `st_len` then returned the low word of the store magic | `arch_check` **object-header**: only `st_alloc`/`st_seal` may write cells 0 and 1 of an object |
| **concurrency / race** | 31 | silent child loss above 8 kept symbols across a `sys_clone`; a probe whose parent read shared cells without waiting and measured a race; two writers sharing one spill frame | discipline: every threaded measurement waits, and runs at least twice. The 8-symbol rule is in `docs/TRAPS.md` and `WORKER-CARD.md` with its measurement |
| **memory / bounds** | 25 | `zeros()` past `x28`; an object written past its length clobbering the next one; a store mapped at a size the file no longer has | the traps themselves (80, 82) — but only once they are LOUD at the point of failure, see the law above |
| **silent failure** | 24 | a call to an undefined function compiled with rc=0; a literal past 2^63 silently accepted; an arena overflow that corrupted instead of trapping | each one was fixed by ADDING A TRAP (T130, trap 80, trap 87). That is the pattern: the fix for a silent failure is a diagnostic, not a workaround |
| **stale constant or doc** | 24 | `<= 8 live symbols`; `KNOWN RED: compiler miscompile` on a gate that passes; a harness model predicting 400 cells per commit when it was 421; a docstring describing gate behaviour it no longer had | `arch_check` **magic-constant** forces layout assumptions to be written as the arithmetic they are, so the next layout change can be checked against them |
| **stale artifact** | 21 | a `.store` from an earlier run that `st_open` cannot match; a `.gbpool` written by a different compiler; a promoted binary that is not the source's | `arch_check` **artifact-identity** and **stale-store**, plus the battery clearing generated stores |
| **no-op that reports success** | 11 | `st_commit_2pc` re-read the committed state, wrote it back and returned a generation number; partitions that all began at the same cursor and "successfully" overwrote each other | discipline: a function that publishes must be tested by READING BACK what it published, in a separate probe. `rc=0` proves nothing about a write |
| **hang / deadlock** | 10 | a child died and the parent waited on a flag for ever; a bounded resource exhausted inside a thread | the LOUD FAILURES law: bound every wait, name what it waited for |
| **fitted or unverified claim** | 10 | a roadmap row citing a commit that does not exist; a gate made green by reverting semantics rather than fixing a bug; a lane reporting a number it never ran | discipline: two independent sides must agree — gate and oracle, or measurement and derivation — before a number is believed |

**The three checks that would have caught the most, historically**: an identity gate on the
compiled artifact (the promoted binary IS the source's compiler — `arch_check`
artifact-identity); a loud trap wherever the compiler currently accepts something invalid;
and a version or shape field validated before any structured read, so a stale-layout read
announces itself instead of silently misinterpreting bytes.

**What the frequencies say about where to spend effort.** Codegen is the biggest class and
the hardest to mechanise, which is exactly why this project has a reference interpreter, a
construct corpus and a fuzzer — those three are the defence, and a new construct without a
`bench/parity_constructs` entry is an untested construct. Everything below codegen in that
table is now either mechanically checked or has a named discipline, and the checks live in
`tools/arch_check.py` inside `bench/vs_rust/invariants.sh`, not in this file.

## HOW TO WRITE THE NEXT FILE (architecture, not style)

1. **A new `.bp` file caps at 800 lines.** `arch_check` enforces it. If you are approaching
   the cap, you are writing two files: split along the seam you can NAME, not at the middle.
2. **Top-level functions only.** No nested definitions; the language has no closures and the
   register model has no frame for them.
3. **One named helper per concept, and give it the concept's name.** `st_region_start`,
   `st_parttab_root_p`, `do_cross_commit` are readable from their call sites. A block of
   inline arithmetic repeated twice is a helper you have not written yet.
4. **A function that spans a `sys_clone` keeps at most EIGHT symbols across the spawn**, and
   constants do not count — only what must be KEPT. Pack the rest into one `[i64]`; array
   handles are cell indices and `st_addr`/`st_cells` are identities, so a handle fits in a
   slot exactly. Over the line, children are lost SILENTLY: no trap, no diagnostic.
5. **Derive constants in the source, not in your head.** Write `2000 * 4 + 2003 + 5 + 21`,
   never `10029`. The derivation is what lets the next reader check it against a layout change.
6. **A gate's value is the deliverable, not its exit code.** `rc=0` with a fold of 0 is a
   failure. Quote the printed number, always.
7. **Show the gate can fail.** Break the feature on purpose, watch the number move, put it
   back, and quote all three values. A number that does not move is not measuring anything.

## KEEP (positive patterns, session-proven)

Distinct exit codes per failure branch + errno propagation (`neg x0`);
minimal-repro ladders; variant bisection A–E; `info proc mappings` crash
triage; execution-first verification through the real runtime path (seed)
— assembler-correct words can still be context-wrong; reverting silently-
wrong features instead of documenting around them.

## AGENT RULES (synthesised from provided principles — integrated before M3 resume)

I. INNER COMPASS (navigation & invariant orientation)
  1. First Principles vector — always derive from fundamentals, discard
     "general practice" / blind tradition.
  2. Zero target drift — keep focus on final invariant; any deviation must
     be justified & temporary.
  3. Truth over comfort — objective reality & math proofs > expectations.
  4. Lindy orientation — prefer principles/architectures that proved stable
     over time vs. fleeting trends.
  5. Extreme noise filter — ignore informational ballast with no direct
     benefit for current task.
  6. Value invariance — core ethics, honesty, engineering integrity rules
     don't change with external circumstances.
  7. Strategic refusal — able to decisively discard good ideas that don't
     lead to the main goal, to avoid resource dispersion.
  8. Internal locus of control — rely exclusively on own actions, analysis,
     code; minimize complaints about external obstacles.
  9. Declarations-action sync — system rules must execute directly in code
     & logic, no theory-practice gaps.
 10. Attention ecology — protect focus as scarcest resource; direct only to
      system bottlenecks.

II. INNER SYSTEM (architecture & operational mechanics)
 11. Zero bloat — build without unnecessary abstraction layers, runtimes,
      unnecessary dependencies.
 12. Deterministic execution — inputs must yield predictable stable results.
 13. Side-effect isolation — change in one module must not destabilize
      adjacent components.
 14. Critical path law — optimize only the limiting factor (bottleneck).
 15. Fail-fast architecture — detect errors & invalid states earliest,
      prevent cascade ruin.
 16. Closed feedback loops — design processes so execution results immediately
      re-calibrate input parameters.
 17. Scaling via simplification — system complexity is always architectural
      debt; real scaling achieves through simplification.
 18. Memory & context preservation — avoid work-environment fragmentation;
      keep context clean & structured.
 19. Emergent stability — design components minimally so their interaction
      yields a robust super-system.
 20. Ironclad invariants — every block/algorithm must have formally proven
      safety & correctness boundaries.

III. METACOGNITION (thinking about one's own thinking)
 21. Real consciousness monitoring — observe own thought process "from
      bird's-eye view", fix fatigue moments / tunnel-vision instances.
 22. Hunt cognitive biases — actively seek blind spots, desired-vs-real,
      fact-fitting to hypothesis in your reasoning.
 23. Pre-flight hypothesis validation — before complex action or writing
      code, clearly formulate falsification criteria.
 24. Automatic post-mortem — after task completion or bug collision, cold-
      blooded analysis: why did error occur, where did logic failure strike.
 25. Regular cognitive cache reset — stop early & take pauses to break
      looping on a wrong solution.
 26. Anti-egoistic audit — subject ideas you just invented & fell in love
      with to the hardest possible scrutiny.
 26. Cognitive ROI assessment — constantly analyze whether invested energy
      & time are worth final result.
 27. Rule versioning — treat your thinking algorithms like code subject to
      constant refactoring & optimization.
 28. Distance from problem — if stuck, exit to higher level: view problem
      not as executor but as architect.
 29. Presumption of own error — on failure, first search error in own
      assumptions/code, then in external factors.
 30. Debug reproducibility — never patch a bug without a minimal reproducible
      trigger (100% repeatable).

III. DEBUGGING PRINCIPLES (engineering trouble-shooting)
 31. Minimize test case (reductio ad absurdum) — reduce problem to atom:
      strip until only minimal instruction set reliably reproduces bug.
 32. Binary search — disable optimizations / code blocks one-by-one to
      instantly localize exact fault site.
 33. Scientific method over guesses — formulate concrete hypotheses:
      "Register X gets overwritten at instruction Y due to wrong stack offset".
 34. Single change only — one modification that either confirms or
      falsifies assumption; never multiple changes simultaneously.
 35. Direct machine code analysis — never trust high-level representation;
      always disassemble or dump raw memory; compilers often produce
      unexpected code.
 36. Control register state — for micro-benchmarkers fastest path is tracing
      register values at point of failure.
 37. Hunt uninitialized memory — most "mystical" bugs in native code without
      runtime protection reduce to random reading of register/stack garbage.
 38. Isolate side effects — verify function inputs & memory state before &
      after critical section strictly match spec.
 39. Make error reproducible — worst enemy is "floating" bug; must find
      exact minimal trigger that guarantees failure on 100% of attempts.
 40. Rubber duck method — explain problem aloud step-by-step (to person,
      rubber duck, or even object); usually self-defeating logic surfaces.
 41. Check assumptions — often the real problem: inputs are completely
      different from what we imagine them to be.
 42. 40-minute rule — if stuck on same code line >30-40 min, stop; close
      laptop, walk, drink coffee, switch task 15 min; fresh view often
      finds bug in 5 seconds.
 43. Keep debugging journal — record verified hypotheses, avoid hitting
      same wall twice: "Checked hypothesis H — no; moved register Y —
      behavior changed thus".

COLD-START RULE: Anything executed repeatedly lives in the daemon or in
compiled subcommands; python/bash one-offs are for single use only.

L18. NO IDLE WAITING (operator rule, 2026-09-06): while a shell, chain, battery or
     sweep runs in the background, the agent keeps working on an independent item
     (the next hunt, docs, the next patch in scratch) and polls the running work
     only alongside that work — never a turn that only waits. Trigger: any
     run_in_background / long gate. Cost: none; the box has 3 A78 cores and one
     writer. Incident: session 10 lost wall-clock to wait-only turns while a
     5-minute self-compile ran.
     Dead letter (D13 item 6): mechanised by a hookify sleep-poll block rule (Claude-config side).
L19. THE BOX IS GUARDED (operator priority, 2026-09-05): a Termux-side watchdog
(`boxguard status|log`, source /data/data/com.termux/files/home/boxguard.py) holds the
box at <= 450 % CPU (of 800 %), pauses nice-10 background work first (SIGSTOP/SIGCONT
rotation), kills the largest worker below 450 MB MemAvailable, halves the budget above
75 C, and kills any shell spinning at 100 % for 30 s with ppid 1 (a detached proot
tracee from a dead session -- the cause of the session deaths). Consequences: (a) a slow
step is usually `stopped=[...]` in `boxguard status`, not a regression -- read it before
timing anything; (b) never detach long work from a Claude shell (`nohup`, `&` + disown):
it dies or spins with the session's proot -- background shields run as runit services in
their own proot (`sv status $PREFIX/var/service/fuzzd`, tools/fuzzd.sh); (c) keep the
process count under 32 (Android phantom cap until ~/adbfix.sh is applied): fuzz J<=2,
one battery at a time, at most 3 parallel agents; (d) `ulimit -Sd` caps anonymous memory
at 3 GB per process -- an exit 137/MemoryError at that size is the cap, not the box.
     Dead letter, L19(c) (D13 items 1-2): mechanised by tools/reap.sh --check (runners refuse
     to start above 26 procs) and the Agent-spawn PreToolUse hook (Claude-config side).

L20. EXPERIMENT LOOP (2026-09-06, Karpathy autoresearch shape; the ralph-loop plugin runs it
unattended): one hypothesis per iteration; keep the change only if tools/chain.sh + battery
are GREEN and the metric is not worse, else `git checkout -- .`; every iteration writes ONE
docs/exp.journal line (H:/DID:/GOT:/VERDICT:) — the journal is the results.tsv; never pause
to ask whether to continue: stop only on the completion promise or max_iterations. Hard laws
(no cp onto bebop.bin, TASKS.md is generated, pkill -f literals, gate evidence in commits)
are also hookify rules in ~/.claude/hookify.*.local.md — edit the rule there, not the prose.

L21. REAP AFTER EVERY TASK (operator rule, 2026-09-06): when a task ends (a chain, a battery,
a fuzz batch, an agent, a commit) run `tools/reap.sh` and read the process count; anything it
lists -- a bash/python3/seed/xargs with ppid 1 running bench/ or tools/ work, or a parent whose
children are all zombies -- is a leftover of a dead session: `tools/reap.sh kill` it before the
next task starts. Reason: 2026-09-05 four such shells spun at 100 % each and the count hit 32 =
Android's phantom-process cap (the false "transient COMPILEFAIL rc=90"); 2026-09-06 an
orphaned invariants.sh with four zombie children sat at the cap again. Never `pkill -f` a
literal (L20 hookify rule); reap.sh kills by pid.

L22. ROLES (operator rule, 2026-09-06): the main session (Fable) is the analyst, planner and
orchestrator -- it reads the state, writes the spec/blueprint and the agent prompt, verifies the
VERDICT, commits and pushes. Roadmap work items (code, gates, journal lines) are executed by a
Sonnet agent from that spec; fact-gathering research reports may go to an Opus agent, but every
blueprint, spec and meta-prompt is written by the main session itself (operator 2026-09-06:
"блюпринти має писати лише fable, як і спеки"). Agent prompts follow
the Opus-5 prompting rules: goal + intent stated once, only the constraints the task really has,
no filler or verification nudges, concrete reference material in a <context> block, tags in the
order <context> <constraints> <output_format> <task>, one scope sentence. L19(c) caps still
hold: at most two agents at once, procs < 30.

L23. NOTHING IS "UNREACHABLE" UNTIL IT HAS BEEN TRIED (operator rule, 2026-09-09, binding):
     no row, report, journal line or commit message may call a goal unreachable, impossible or
     out of reach unless it has been attempted IN PRACTICE at least several times BY DIFFERENT
     METHODS. Arithmetic that bounds one mechanism bounds that mechanism, not the goal: say
     "this method gives X against a gate of Y" and name the methods not yet tried. A measured
     REFUTATION of a specific mechanism is not covered by this law and stays as it is -- what is
     banned is generalising from it to the goal.
     The rule was written because two live claims failed it the day it was made. D5 said
     two-stage DDC was "structurally impossible, not merely slow" because the witness had no
     `use` handling; one line to `emit_epilogue` then took the witness from 0 to 46 of 73
     non-vacuous agreements, and `use` was costed at 60-80 lines rather than being impossible.
     B4 said the ns/edge half was "unreachable by fixing promotion alone"; it is GREEN at 435.
     And the 2026-09-09 design study filed a self-verified checker as multi-year and out of
     reach by pricing row F4's cost into row F7's item, when F4 is paid for anyway.
     The honest forms are: "not attempted", "attempted by <methods>, best <number>", or
     "costed at <number> and not scheduled". Never "impossible" without the attempts behind it.
