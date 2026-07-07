# Fable harness audit — findings backlog (2026-07-07)

Decorrelated one-shot audit run ON FABLE (operator: "run on fable — purge the blockers"), read-only,
40 tool-uses, evidence-first. This is the actionable backlog: each item ends in a DETERMINISTIC
follow-up (a check/gate/test to write, or a file/process to delete) per §0·GP. Execute in a FRESH
session (this session hit the 300K context-budget gate).

## ROOT-CAUSE: 0-tool-use degenerate subagent returns
**Mechanism = injected-context ECHO** (not a hook, not routing): the subagent's first decode
*continues* the harness-injected trailing metadata (deferred-tools delta / skill listing /
system-reminder) instead of executing the task.
- Evidence: `~/.claude/projects/-root-dowiz/163d0c88-.../subagents/agent-ae2504fd9a2b85308.jsonl` = 4
  lines (user prompt intact, `deferred_tools_delta`, `skill_listing`, then ONE assistant turn, 55
  out-tokens/2842ms) whose text is `_context_relevance:\nSkills marked...` — a metadata continuation.
  Second: `agent-a79d88d9420462b48.jsonl` → `_id: ...\n\nThe system is Claude Code...<system-reminder>`.
  Control `agent-a9db49f45099f4d74.jsonl` (same attachments/model/session) = 88 lines, worked → rules
  out routing/attachment-as-sole-cause; no hook output in any subagent transcript → rules out a guard.
- **Base rate: 33 / 1301 subagent transcripts (2.5%) are 0-tool-use**, clustered. Signatures:
  `^_[a-z_]+:` metadata continuations, `<br>\n\nSystem:` fabricated turns, `<system-reminder>` echoes,
  mid-sentence fragments, + 6 API-error strings surfaced as "results" (`API Error: 529 Overloaded`,
  `You've hit your session limit`, `temporarily limiting requests`). Only ~5/33 are legit no-tool replies.
- Parent-side ground truth ALREADY exists: task-notification carries `<tool_uses>0</tool_uses>`
  (main transcript `163d0c88...jsonl:506`).
- **CHECKER TO BUILD** — `subagent-return-guard.sh`, event **SubagentStop** (+ belt PostToolUse
  `Agent|Task`): locate the stopped `agent-*.jsonl`, count `"type":"tool_use"` + assistant turns;
  **RED** (`{"decision":"block",...}` + `_hev`) when `tool_uses==0 && assistant_turns==1 &&
  final_text =~ /^(_[a-z_]+|<br>|<system-reminder>|API Error:|You've hit your session limit|.*temporarily limiting)|The system is Claude Code/`;
  WARN on 0-tool-use without signature. Fixtures from the two real degenerate transcripts (must red)
  + the good control (must green); pin registration in `guardrail-hook-matchers.mjs` MUST_COVER.

## FINDINGS (ranked; SEVERITY | AREA | evidence | follow-up)
1. **HIGH · guard-bash over-blocks** — `guard-bash.sh:70,86-96` greps the WHOLE command text (incl.
   quoted strings/commit messages/banners) for PROTECTED; ~10/12 recent blocks are false positives
   (read-only `curl`, `cat > scratchpad`, `git commit -F - <<EOF` of docs, `cat` of a state file).
   → rewrite mutation detection to parse redirect/mutator TARGETS (token after `>`/`>>`/`tee`/`cp`/`mv`)
   and match PROTECTED against targets only; whitelist `/tmp/claude-*`; add the 10 FP commands as
   exit-0 fixtures + 2 `.env`-write probes as exit-2 fixtures in `guardrail-token-gates.mjs`.
2. **HIGH · circuits NOT wired** — `scripts/run-circuits.mjs` + `circuits/registry.json` (4 circuits)
   have zero references in pre-commit / package.json / run-armaments / settings.json. → add
   `node scripts/run-circuits.mjs --staged` as a `run()` line in `scripts/run-armaments.sh` (1 line).
3. **HIGH · Fable deny disarmed** — `agent-dispatch-gate.sh:80` now default-warn (this session's purge),
   contradicting the fable-oneshot spec + AGENTS.md. → **OPERATOR DECISION**: the one-shot is done; the
   audit recommends re-arming (`:-deny` + flip guardrail assertion). Left purged pending operator call
   (they said "purge", not "temporarily"). Do NOT silently re-arm.
4. **HIGH · loop cert dishonesty** — `registry.md:13,21` two CERTIFIED rows say "звіт ВТРАЧЕНО"; rows
   14-24 cite 10 report files that don't exist (loops/reports/ has only 4). → write
   `guardrail-loop-registry-parity.mjs` (CERTIFIED ⇒ report exists; cited paths exist); add to
   run-armaments; demote report-less CERTIFIED rows to DRAFT.
5. **HIGH · loops reference removed machinery** — `sandbox-swarm-gate.yaml`/`skill-evolution.yaml`
   require `invariant-guardian`/`security-sentinel` verdicts; `registry.md:46` prescribes
   `reasoning=fable`; 11 loop YAMLs escalate to `/council`. → add a red-line forbid-circuit (glob
   `loops/**` + `.claude/skills/**`, pattern `/council|invariant-guardian|security-sentinel|serious-gate|design-council`),
   then fix the 13 files it reds on.
6. **MED · 0-tool-use checker absent** — see root-cause; build `subagent-return-guard.sh`.
7. **MED · require-classification bypass** — `require-classification.sh:37` checks only `git diff
   --quiet` (tracked, unstaged); staged-only + untracked-new files bypass the manifest requirement.
   → switch to `git status --porcelain -- apps/ packages/ spikes/`; add a staged-only red fixture.
8. **MED · external-script bypass** — `guard-bash.sh:78` + `protect-paths.sh:50` (abs paths outside
   repo exit 0): an agent can Write a script to scratchpad and execute it to mutate a protected zone.
   → post-hoc Stop/PostToolUse(Bash) check: `git status --porcelain` ∩ PROTECTED, red when a protected
   file changed with no fresh human-approval state file.
9. **MED · gates emit no telemetry** — `protect-paths.sh` + `context-budget-guard.sh` have no `_hev`
   call → block/fire rates unmeasurable (pruning is supposed to be measured). → add `_hev` lines;
   extend `guardrail-gate-armament.mjs:24` UTF-8/telemetry check to all registered hooks.
10. **MED · `_hev` UTF-8 corruption** — `cut -c1-200` byte-truncation splits multibyte UTF-8 →
    harness-events.jsonl has invalid bytes (reproduced: UnicodeDecodeError at 0xd0). Same defect in
    agent-dispatch-gate/red-line-doubt-gate/distill-nudge/require-classification. → single shared
    UTF-8-safe `_hev` helper; circuit: every harness-events line must parse as strict-UTF-8 JSON.
11. **LOW · guard-bash** blocks `npm install -g` (global installs never touch pnpm-lock). → exempt `-g/--global`.
12. **LOW · router hook not wired** — `registry.md:7` claims `tools/loop-harness/router-hook.sh`
    enforces, but it's not in settings.json. → register it or correct registry.md to "not wired".
13. **LOW · orphan state** — `.claude/state/serious-cleared` (14 grants, removed serious-gate) +
    `.claude/state/eye/` (never-installed eye). → delete both.
14. **LOW · loop-architect M11** mandates a standing OpenRouter cross-review proxy inside every cert.
    → make M11 on-demand; certification valid only when the report artifact exists (finding 4).
15. **LOW · docs-drift** — `AGENTS.md:222` claims settings.json pins `model: claude-opus-4-8`; project
    settings.json has NO model key (pin is in `~/.claude/settings.json`). → correct AGENTS.md.

## SURVIVING PROXIES (still opinion-based, no ground-truth check on their own output)
- `red-line-doubt-gate.sh:96-111` advisory doubt-pass arm (keep the irreversible-DENY arm) — capture
  the doubt-pass to a checkable state file, or drop the advisory arm.
- `loop-architect` "CERTIFIED" verdict — proxy until registry-parity guardrail enforces the artifact.
- `doubt-escalation` skill — rung 4 = removed `/council`; 0 measured invocations of 51 Skill calls.
- `playwright-test-planner` — acceptable input-proxy (generator+healer outputs ARE test-checked).
- **`~/.claude/settings.json repowise-augment` PostToolUse on every Bash|Read|Edit|Grep|Glob** — a
  cached-datum injection on every action; no ground-truth check on its own staleness claims, and it is
  exactly the injected-reminder text class the echo bug regurgitates. Measure cost/hit-rate; candidate
  to narrow to Edit/Write only.

## DEAD / ORPHANED (deletion candidates — deterministic-proof auto-deletion §7·B fodder)
- 10 nonexistent report files cited by `registry.md` rows 14-24.
- 13 files referencing removed machinery (11 loop YAMLs → `/council`; 2 loops → invariant-guardian/
  security-sentinel/serious-gate; registry.md:46,50; doubt-escalation SKILL.md:44-45).
- `.claude/state/serious-cleared`, `.claude/state/eye/`.
- `.claude/CLAUDE.md.bak-1782168703`, `.bak-1782288969` (stale backups in the live config dir).
- `docs/operating-model/proposed-eye/`, `proposed-circuit-wiring/` — wire (finding 2) or shelf-ware.

## TOP 5 (highest value first)
1. Build `subagent-return-guard.sh` (SubagentStop): red on tool_uses==0 && single turn && echo/API-error
   signature; fixtures from the 2 real degenerate transcripts.
2. Wire circuits: one `run()` line in run-armaments.sh → the whole KNOWLEDGE-AS-CIRCUITS layer becomes enforced.
3. Fix guard-bash target-based matching + scratchpad exemption (83% FP rate = over-broad gate → no gate).
4. `guardrail-loop-registry-parity.mjs` + red-line forbid-circuit for removed-machinery refs; fix the 13 files.
5. Operator decision on Fable re-arm; delete orphan state.

## STATUS — SHIPPED 2026-07-07 (all top-5 + VbM), proof: `bash scripts/run-armaments.sh` = 14/14 green
1. ✅ `.claude/hooks/subagent-return-guard.sh` (SubagentStop + belt PostToolUse Agent|Task) — blocks on
   tool_uses==0 && ≤1 turn && echo/API-error signature; loop-guard on stop_hook_active; fail-open.
   Fixtures `scripts/fixtures/subagent-return-guard/` (2 real degenerate signatures + control + legit-no-tool);
   armament `scripts/guardrail-subagent-return-guard.mjs`; registered in settings.json; pinned in
   `guardrail-hook-matchers.mjs` MUST_COVER.
2. ✅ circuits wired: `run-circuits.mjs` gained `--self-test` + `--warn-ok` (red-line blocks, warn advisory,
   no over-block); TWO run() lines in `run-armaments.sh`. Also fixed a latent `globToRe` globstar bug
   (`**/` required an intermediate dir → top-level `loops/*.yaml` were never checked = a circuit that
   could never fire; a VbM false-green). Registry now 6 circuits.
3. ✅ guard-bash target-based matching: PROTECTED/OVERRIDES now match WRITE TARGETS only (redirect dests +
   mutator path-args on a quote-stripped skeleton), /tmp/claude-* whitelisted. Measured: 0/7 FP on the
   over-block corpus, 0/5 missed real blocks (`scripts/probe-system-comparison.mjs`). New fixtures in
   `guardrail-gate-armament.mjs`.
4. ✅ `scripts/guardrail-loop-registry-parity.mjs` (CERTIFIED⇒report exists; cited paths exist) + red-line
   forbid-circuits (loops/**/*.yaml + .claude/skills/**); demoted rows 13/21 to DRAFT, blanked 10 bogus
   citations; fixed 14 files (0 removed-machinery refs remain). Both wired into run-armaments.
5. ✅ Fable RE-ARMED to `deny` default (one-shot consumed; human expiring-override + warn escape-hatch
   remain, all falsifiable); orphan `.claude/state/serious-cleared` + `.claude/state/eye/` deleted.
+ ✅ **Verified-by-Math** universal rule (operator 2026-07-07): `docs/operating-model/verified-by-math.md`
   + `scripts/guardrail-falsifiable-proof.mjs` (every enforced proof must be able to go RED — 12/12) +
   CLAUDE.md core + AGENTS.md §VbM.
Remaining (MED/LOW, not top-5): #7 require-classification staged/untracked, #8 external-script bypass,
#9/#10 gate telemetry + `_hev` UTF-8, #11 npm -g, #12 router-hook wiring/registry, #14 loop-architect M11,
#15 AGENTS docs-drift. The surviving-proxies + dead/orphaned lists still stand as backlog.
