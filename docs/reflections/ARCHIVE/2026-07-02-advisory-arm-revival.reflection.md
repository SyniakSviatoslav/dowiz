---
CONTEXT:   P1–P3 of the meta-loop audit (human-approved): forcing functions for the advisory
           arm (Stop reflection-gate, staged), harness self-measurement (harness-events.jsonl
           + agent-health-pass), loop-telemetry repair (finite breaker, registry SoT sync,
           metrics tracked in git), ledger/registry integrity guardrails, librarian's first
           curation run since 06-23, broader default permissions.
DECISIONS: Event log = one JSONL line per hook decision (inline 8-line shell fn, no sourcing);
           Stop-gate pulse = "any INBOX reflection <8h" not per-commit matching (forgiving beats
           nag-then-bypass); version-gated V2 armament cases so verify:all stays green while
           hooks v2 await operator apply; registry sync = union-merge (never deletes rich
           hand-authored entries); duplicate ledger rows suffixed (7b…) so existing "#N" refs
           stay valid; CERTIFIED-without-report flagged honestly instead of silently re-certed;
           CronCreate rejected for the weekly librarian (session-only, 7d expiry) → durable
           /schedule routine instead.
WHERE:     docs/governance/agent-health-2026-07-02.md (first report); staged-p1/ scratchpad;
           ledger #48. Live proof en route: the librarian's hours-old lesson (docs-only-no-
           staging-deploy) was auto-injected by pre-edit-lessons.sh during this very change's
           ledger edit — first end-to-end Tier-2 injection with curated content.
WHY:       Root confirmed from P0 generalizes: the system only sustains behaviors that are
           either hook-enforced or produce an artifact some gate checks. The advisory arm had
           neither — its steps ended in prose obligations ("librarian curates", "truncate after
           ship", "call finalize") with no artifact a machine ever inspected, so every one of
           them silently stopped. Measurement is the precondition for the ratchet: prune/promote
           decisions need hit-counts, armament needs decision logs, loop improvement needs run
           history — none existed, so the loop could not even observe its own death.
CONFIDENCE: high
NEXT-TIME: When designing any recurring obligation, specify in the same breath (a) the artifact
           it must produce, (b) the deterministic checker that inspects it, and (c) the event
           line it logs — or expect it to die within a week. Follow-ups: guard-bash false-
           positives on read-only commands whose PAYLOAD mentions protected paths + redirects
           (route around via script files; consider payload-aware refinement); worktree
           isolation for parallel sessions (2nd shared-checkout commit collision); autoupgrade
           session-telemetry collector still emits zero-cost rows.
LINK:      docs/regressions/REGRESSION-LEDGER.md #48 ; scripts/agent-health-pass.mjs ;
           [[meta-loop-audit-2026-07-02]]
---

**Curation note (librarian, 2026-07-05 daily pass):** Challenged fresh. The causal claim
("obligations that end in prose die; only hook-enforced or gate-checked behavior survives") is
sound and CONFIRMED — but it is not a new finding: it is the SAME root already distilled into
`docs/lessons/2026-07-02-gate-state-file-expiry.md` (source: the sibling reflection
`2026-07-02-governance-gates-rot-open.reflection.md`, ledger #47), and this reflection's own
guardrails (`guardrail-ledger-integrity.mjs`, `loops-registry-sync.mjs`, `agent-health-pass.mjs`,
hook-v2 armament) are already a complete, standing ledger row (#48) — verified present and
green on this pass (`scripts/agent-health-pass.mjs` runs; `guardrail-gate-armament.mjs`,
`guardrail-ledger-integrity.mjs`, `loops-registry-sync.mjs` all exist). No new TRIGGER/ACTION
survives that isn't already covered by the existing lesson + ledger row. DISCARDED as a
duplicate rather than distilled into a second lesson — writing a near-identical lesson would
violate "one atomic lesson" / the store-must-not-grow bias for zero marginal signal. Archived,
not promoted separately.
