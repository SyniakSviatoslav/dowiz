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

L15. FALSIFIABLE PROBE BEFORE fpC/SPILL EDITS: before touching the
     branchless-cond or spill machinery, run (or add) the minimal probe for
     the construct (bench/parity_constructs/c22-c24 are the canonical
     shapes: match-binding, spilled-arg call, spilled-array-in-if). Fix
     only after the probe fails; then freeze the probe.

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
Sonnet agent from that spec; research and analysis reports by an Opus agent. Agent prompts follow
the Opus-5 prompting rules: goal + intent stated once, only the constraints the task really has,
no filler or verification nudges, concrete reference material in a <context> block, tags in the
order <context> <constraints> <output_format> <task>, one scope sentence. L19(c) caps still
hold: at most two agents at once, procs < 30.
