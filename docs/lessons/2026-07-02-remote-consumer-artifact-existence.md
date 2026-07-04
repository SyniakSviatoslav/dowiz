---
TRIGGER: scripts/plane-*.mjs
CAUSE: >
  A remote trigger prompt (claude.ai `RemoteTrigger`) was updated to reference repo script paths
  (`scripts/plane-telemetry.mjs`, `scripts/plane-report.mjs`) while those scripts sat uncommitted
  on the authoring session's local disk. The remote consumer reads the pushed ref its checkout
  pulls from, never the local filesystem — so "the script exists" (true locally) and "the script
  is reachable to the remote consumer" (false, until pushed) were silently conflated. Root cause:
  two PRIOR sessions had deferred committing (collision-avoidance in a shared checkout — the same
  hazard as `docs/reflections/ARCHIVE/design-system-prune-collision-2026-07-02.md`), and the
  deferral became invisible background state that a later remote-facing wiring step assumed away.
ACTION: >
  Before updating or authoring anything that points a REMOTE consumer (a claude.ai trigger prompt,
  a webhook, a CI/workflow doc) at a repo script/file path → cause: the remote consumer resolves
  the path against the ref it pulls, not this session's working tree → do: (1) commit + push the
  referenced file to the branch the remote consumer actually reads BEFORE wiring the reference;
  (2) confirm with `git ls-remote` or `git log origin/<branch> -- <path>` that the exact path
  exists there; (3) if a commit is deliberately deferred (collision-avoidance), record WHO/WHEN
  unblocks it and re-verify at every subsequent remote-facing step — a deferred commit is debt
  with a fuse, not a settled state.
LINK: docs/reflections/ARCHIVE/2026-07-02-plane-telemetry-closed-loop.reflection.md ;
  docs/governance/plane-maintainer-agent.md ; docs/adr/ADR-plane-telemetry-and-calibration.md
SCOPE: ONLY edits to scripts under `scripts/plane-*.mjs` (or the plane-maintainer governance doc)
  that a remote trigger/webhook/CI wiring references. Not general "commit often" guidance, and not
  a substitute for the already-shipped durable-store fix (git-plumbing publish to the append-only
  `telemetry/plane` branch, ledger #49) which covers the sibling "durable-local illusion" root.
STATUS: active
---

# A remote consumer reads the pushed ref, not this session's disk — verify the path exists remotely before wiring it

Source: `docs/reflections/ARCHIVE/2026-07-02-plane-telemetry-closed-loop.reflection.md`
(causal root 2, "uncommitted-toolchain blind spot"). CONFIDENCE: high in the source reflection.

The 2026-07-02 plane-telemetry session shipped a trigger-prompt update referencing
`scripts/plane-telemetry.mjs` before that script was committed — the cloud-side trigger checks out
GitHub, so the whole uncommitted governance batch was invisible to it until a clean ship branch
was assembled. This is a second occurrence of the shared-checkout deferred-commit hazard already
seen in the design-system-prune session (that one broke a local commit; this one broke a remote
wiring reference) — recurrence-prone enough to warrant an advisory nudge, though no static gate
can see "is this referenced path pushed to the ref the remote consumer reads" without querying the
remote each time, so this stays Tier-2 (lesson), not a forced Tier-1 guardrail.
