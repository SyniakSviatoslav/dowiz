# BUG LEDGER — week ending 2026-09-02 (R3.x closure + R6.1)

Every defect from the week with: class, evidence, fix, and the MECHANICAL
guard that now exists to catch the class (per AGENTS v2 rules: a rule
lives only with its trigger and its check).

## Compiler-internal classes (bebop.bp)

### C1 · Model-zone clobber via unbounded depth index
- Symptom: model slot writes `fntab[3801+d]` land on the string-literal
  table (3899+) when the emulated stack depth d grows.
- Evidence: journal 1788288245/1788288246 — depth legitimately reaches
  99+ (array literals) and 230+ (the +1-per-unresolved-call leak).
- Fix: model relocated to 3700, slot write guarded `0 <= d < 96`.
- Guard: the guard IS in push (bebop.bp `let wsl = ...`).

### C2 · Stream-pollution cascade from check-traps in push/pop
- Symptom: self-sized compiles took >900 s vs 14 s; byte-output must stay
  pure (the emitter's only side effect on the stream must be the words).
- Fix: push/pop became bookkeeping-only (no `em` of trap words).
- Guard: c22/c23 probes freeze the emitted bytes; any new trap emission
  would change the frozen words. LAW: push/pop never emit extra words.

### C3 · Negative-depth OOB
- Pop at depth 0 made d=-1; deeper imbalance could index below the array.
- Fix: depth clamps at 0; slot index guarded both sides.
- Guard: the clamp + guard in pop/push (c21 probe: 13-param arithmetic).

### C4 · Param-cap silent truncation (pre-existing)
- Symptom: 13-param fns compiled to silently-wrong 440 (55<<3 stack
  desync) — both old and new compilers.
- Fix: cap 10 -> 14 in parse_params arrays and all three cap sites.
- Guard (NEW): a LOUD trap word 3558867200 is emitted when a fn declares
  >14 params (compile_fn_at / compile_fn / emit_call) — truncation can
  never again be silent. c21 freezes the 13-param case.

### C5 · 4-arg pop call from a 13-param spilled frame segfaults
- Symptom: match binding crash between ladder prints C and D; generic
  4-arg/spilled-arg probes pass, so the trigger is the specific call form.
- Fix: the pop's two words are inlined in emit_match_arm (byte-identical
  output, no call).
- Guard: c22_matchbind freezes the binding emission bytes + runtime 7.

## Process classes (tooling, this session)

### P1 · Empty-file artifact read as a real compiler
- What happened: `git show 8ec62df:bebop.bin` failed silently (repo root
  is dowiz; paths need the bebop-lang/ prefix), the redirect produced an
  EMPTY file, and every "old compiler" comparison ran it. exit 91 is the
  seed's invalid-.bin code — misread as a "loud trap" and chased as logic.
- Cost: hours of phantom conclusions (retracted in journal 1788288248).
- Guards (mechanical, now in place):
  - tools/guard_artifact.sh: size>0 (+optional md5) before ANY use.
  - tools/fetch_golden.sh: rev+path extraction that verifies non-empty.
  - All three harnesses (std_golden/parity/construct) run a zero-size
    preflight on bebop.bin and exit before any test.

### P2 · Baseline drift (cp over the working compiler)
- What happened: bebop.bin was promoted/swapped in place several times;
  at one point the disk binary did not correspond to the source tree.
- Guards:
  - Immutable golden baselines live in bench/golden/<name>-<rev>.bin with
    .sha256 sidecars; never overwritten.
  - Promotion rule: bebop.bin changes ONLY via a fixpoint-green rebuild.

### P3 · Cascading fixes without root-cause isolation
- What happened: guards -> cap -> signature reorder -> revert chains
  without declared hypotheses (the c12 hunt) until the ladder print
  localized the crash to one statement.
- Guard: AGENTS v2 HYPOTHESIS DISCIPLINE + new L14/L15 below.

## Guards added this commit
- bebop.bp: param-cap trap 3558867200 (3 sites).
- bench/parity_constructs/: c21_param13, c22_matchbind, c23_spillcall,
  c24_ifspill probes + frozen bins + EXPECT entries (construct 24/24).
- tools/guard_artifact.sh, tools/fetch_golden.sh.
- bench/golden/bebop-8ec62df.bin + .sha256 (the verified pre-R6.1
  compiler, md5 f78033586c617c5f03a7f9622c4ad548).
- Harness preflights in all three suites.
- AGENTS.md L12–L15.
