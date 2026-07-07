# Reflection — Ground truth over proxy reasoning: remove the council + proxy-reasoning layer

CLASSIFICATION: build (harness change)
DATE: 2026-07-07

## CONTEXT
Operator directive, three messages, final and non-negotiable: "remove council and proxies from
everywhere except memories, this session"; "harness must remain, but ground truth over proxy
reasoning". The arc was seeded by [[next-arc-agent-tooluse-hook-antirot-2026-07-07]]: proxies
(review agents, advisory nudges) were "bringing more problems than solving" — most visibly the
0-tool-use degenerate subagent returns that looked green while doing nothing (a proxy trusted
without a ground-truth check).

## DECISIONS
- Defined the boundary mechanically, not by vibe: **ground truth = a deterministic verdict read off
  the real artifact** (glob/pattern gate, failing test, re-read of source, canonical bytes);
  **proxy = a stand-in** (a 2nd model's opinion / an advisory reasoning-nudge / a cached datum).
  Ground-truth checks are the harness → KEEP. Proxies → REMOVE.
- REMOVED: council Triad (counsel/system-architect/system-breaker) + /council + design-convergence
  loop + serious-gate.sh; critic/review proxy agents (cause/pattern/ratchet-critic, librarian,
  research-verifier, invariant-guardian, security-sentinel, test-scout); advisory-injection hooks
  (pre-edit-lessons, route-request, loop-detector) + their settings.json registrations.
- KEPT: protect-paths, guard-bash, agent-dispatch-gate, post-edit-gates, context-budget-guard,
  require-classification, red-line-doubt-gate, distill-nudge; circuits; loops; playwright test
  agents; all memory/knowledge files.
- Updated the two arming guardrails (hook-matchers, gate-armament) to EXPECT the new set — not
  weakened; both still prove every surviving edit/Bash gate covers its lane and denies stale
  clearances. Elevated §0·GP (ground truth over proxy) to the GOVERNING principle of the
  model-agnostic playbook; demoted the reviewer-decorrelation moves (U4/§3/X5) to conditional.
- Fable examination reframed: using Fable (a proxy) to examine proxies is itself proxy reasoning.
  The examination was done by a ground-truth pass (read + classify + measure every hook vs the
  `_hev` log). A one-shot decorrelated Fable audit stays available but only as a human-gated
  exception via `.claude/state/fable-override`.

## WHERE
`.claude/agents/*`, `.claude/hooks/*`, `.claude/settings.json`, `.claude/commands/*`,
`scripts/guardrail-hook-matchers.mjs`, `scripts/guardrail-gate-armament.mjs`, `loops/registry.md`,
`docs/operating-model/model-agnostic-playbook.md`, `AGENTS.md`. Commits: `f1255ad5` (machinery) +
this docs commit.

## WHY (causal — why it arose, not just where)
The proxy layer grew because every past incident produced a *reflex to add a watcher* (a critic, a
nudge, a council gate) instead of a *check*. A watcher is cheap to add and feels like safety, but it
substitutes a second opinion for a verdict — and an unverified opinion is exactly the meta-root of
the ledger ("a PROXY mistaken for GROUND TRUTH"). The 0-tool-use degenerate returns made it
concrete: a proxy (the subagent) was trusted with no ground-truth check on its output, so "done"
meant nothing. The fix is not a better watcher; it is to prefer the deterministic check whenever one
can be written, and to let a proxy only *signal* where none can. Removing the opinion-machinery
forces that discipline structurally rather than asking for it.

## CONFIDENCE
High on the boundary (ground-truth gates vs proxies) and on the removals being safe: git-recoverable,
ground-truth-verified (no live `.claude` file references a removed proxy), both guardrails green,
full pre-commit incl. Docker build passed. Medium on completeness of the "everywhere" scrub — live
machinery + binding policy (AGENTS.md, playbook) are done; softer historical mentions in loops/*.yaml
prose and governance docs remain as history (they name the proxies as past practice, not as a live
mandate).

## NEXT-TIME
When an incident tempts a new watcher, first ask "can I write a deterministic check for this?" — if
yes, write it and promote it to a circuit/gate (the ratchet); only if no check is possible does a
signalling proxy earn a place, and it must ship with a ground-truth check on its own output.

## LINK
[[next-arc-agent-tooluse-hook-antirot-2026-07-07]] · [[metacognition-transfer-2026-07-06]] ·
[[model-routing-policy-2026-07-03]] · docs/operating-model/model-agnostic-playbook.md §0·GP
