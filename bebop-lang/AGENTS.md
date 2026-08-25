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
5. Register protocol: x19–x26 symbols; x15 spill base; x27/x28 arena;
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
- **Independent gates** (parity driver, fuzz_selfhost, make test,
  selfcompile ×2) → run concurrently, integrate verdicts.
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

## KEEP (positive patterns, session-proven)

Distinct exit codes per failure branch + errno propagation (`neg x0`);
minimal-repro ladders; variant bisection A–E; `info proc mappings` crash
triage; execution-first verification through the real runtime path (seed)
— assembler-correct words can still be context-wrong; reverting silently-
wrong features instead of documenting around them.
