# autoupgrade — run memory

## 2026-06-27 · run #1 (REPORT-ONLY, §8 step 2)

First run on the v3-FINAL harness. MAP grounded in real telemetry (codeburn + fs scan), CLASSIFY fail-safe, NO apply (oracle not yet built).

**Candidates (4):**
- Class A (auto-eligible once oracle proven, 3):
  - `ghost-mcp:claude_design` — codeburn: 0/20 tools used. Action: `claude mcp remove 'claude_design'`. Reversible.
  - `ghost-mcp:claude_ai_Notion` — codeburn: 0/18 tools used across 462 sessions (~$46/mo cached-prefix). Action: `claude mcp remove 'claude_ai_Notion'`. Reversible.
  - `config-bloat:CLAUDE.md` — large file re-read into every prefix. Action: distill → linked docs. Reversible (med blast).
- Class B (propose-only, human/DB-owner, 1):
  - `staged-security:SECURITY-DEFINER-search-path.migration.ts` — firm boundary (security/schema) → NEVER autonomously applied.

**Verdict:** firm boundary held (security migration → B; dev-loop/MCP/config → A). Report-only — nothing applied.

**Classifier regression fixed this run:** the ghost-MCP prune briefly mis-classified as B because the free-text regex matched "session" in "loaded every session" (auth-session false-positive). Fix: broad boundary list matches the structured `area`; tight list matches free text. Regression test added.

**Carry forward → run #2:** none of the Class-A actions are applied until the machine oracle (§2: green + RLS/auth/secret assertions + benchmark-replay ≥5% speedup + recorded revert) and atomic rollback are built and proven (§8 steps 3–4). `applyCandidate()` throws by design until then.

## 2026-06-27 · run #2 (ORACLE-GATED AUTO-APPLY, `--apply`, §8 steps 3–4 enabled)

Oracle built (`oracle.ts`, 7 tests: KEEP + every rollback path) and Class-A auto-apply wired behind
`--apply`. **Verdicts: 0 kept · 0 rolled-back · 3 skipped.**
- ghost-mcp `claude_design` / `claude_ai_Notion` → SKIPPED: account-managed (claude.ai connector),
  not removable/re-addable via the local mcp CLI → NOT loop-reversible. The loop won't apply what it
  can't atomically revert. (Prune manually if desired — the win is real, ~$46/mo.)
- config-bloat `CLAUDE.md` → SKIPPED: protect-paths-gated + prefix-size win isn't benchmark-replayable.
- Class B (SECURITY-DEFINER migration) → never reached apply (firm boundary).

**Outcome:** the oracle correctly KEPT NOTHING — every current candidate is unprovable (not
reversible by the loop, or no runnable benchmark). The gate works as designed (only keep what's
proven). **NEXT:** a reversible+benchmarkable Class-A adapter (worktree + benchmark-replay) so real
repo-perf candidates can be auto-kept; widen only after several clean runs (§8 step 6).

## 2026-06-27 · adapter built (the next step)

`benchmark.ts` + `repo-apply.ts` (`makeRepoHooks`): apply patch → measure benchmark before/after →
green+security → **atomic revert via `git checkout`** (refuses if tree dirty). `repo-apply.test.ts`
(5) proves the FULL path end-to-end on a real throwaway git repo: a 20%-faster change is **KEPT on
disk**; tests-RED / security-regression / no-speedup → **atomically ROLLED BACK** (exact bytes
restored); dirty-tree → refused. Wired into `buildHooks` for `repo-perf:` candidates. The oracle can
now genuinely keep a real repo change. ponytail ceiling: git-checkout (not worktree) → not
concurrency-safe; upgrade to worktree when >1 loop runs. **NEXT:** a MAP source emitting repo-perf
candidates with DETERMINISTIC MECHANICAL patches (slow-query→index, slow-test→cache) — NEVER
autonomous LLM patches (§0 injection). The apply/gate/revert is complete+proven; the missing piece is
a trustworthy SOURCE of patches.

## 2026-06-27 · the gap closed — MAP source for repo-perf (the loop is now end-to-end functional)

`detectors.ts` `configTuneDetector`: the safe, mechanical MAP source. Operator declares tunables in
`loops/autoupgrade.tunables.json` (knob file + `find` regex w/ capture-group-1 value + a BOUNDED set
of safe candidate values + a benchmark + optional green/security cmds). The detector emits a
`repo-perf:tune:<id>:<value>` Candidate per non-current value, each carrying a RepoPerfSpec (mechanical
regex value-swap + benchmark + git-revert). The loop tries each value, benchmarks, and the oracle
KEEPS only the ≥5%-faster one; else atomic rollback. **The operator bounds the search (safety); the
loop searches+measures+keeps (autonomy).** This is the ONLY autonomous repo-mutation path — mechanical,
bounded, reversible, benchmarked; NEVER an autonomous LLM patch (§0).

`detectors.test.ts` (5) proves the FULL pipeline on a real git repo: declared tunable → classify A →
oracle apply → benchmark 50% faster → KEPT on disk; slower value → atomic rollback; no declaration →
[] (safe default, no auto-tuning without opt-in). Wired into mapCandidates. Example:
`loops/autoupgrade.tunables.example.json`. **The autoupgrade loop is now end-to-end functional:**
MAP (incl. real repo-perf) → CLASSIFY → ORACLE → KEEP|ROLLBACK → §5 report. Total 56 harness tests.
