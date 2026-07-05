---
TRIGGER: .claude/state/**
CAUSE: >
  A gate whose release condition lives in a mutable state file (a clearance/allowlist line with
  no expiry) depends on a discipline step ("truncate after ship") to stay tight — and every
  discipline-triggered cleanup step in this system has, at some point, stopped being performed.
  Once that happens, "the hook is registered and runs" gets silently mistaken for "the hook
  denies": nothing measures armament (does it ever say DENY?), only registration (does it
  execute?) — so hundreds of blind ALLOWs read as silence-equals-health. Separately, when such a
  gate over-blocks a legitimate case (e.g. a staging deploy), the fix-under-pressure was to
  unregister it wholesale rather than narrow the matching rule — an over-broad gate converts to
  NO gate.
ACTION: >
  When adding or editing a gate whose release condition is a state file under `.claude/state/**`
  (or the hook reading it under `.claude/hooks/**`) → cause: an un-expiring clearance line rots
  open and registration alone doesn't prove the gate can still deny → do: (a) every clearance
  entry carries its OWN expiry embedded in the state line itself (fail-closed compare against
  wall-clock in the hook, not a separate cleanup step to remember), (b) ship a hermetic armament
  test that SIMULATES a DENY case (not just "the hook is registered in settings.json"), (c) emit
  one log line per decision (ALLOW/DENY) so a blind-open streak is visible in
  `.claude/logs/classification.log` data, not assumed absent. If the gate over-blocks a
  legitimate case, narrow the matching rule — never unregister/remove the gate.
LINK: docs/regressions/REGRESSION-LEDGER.md #47 ; scripts/guardrail-gate-armament.mjs
SCOPE: Gates whose release condition is persisted in `.claude/state/**` (clearance/allowlist
  files) ONLY. Does not apply to stateless, purely static-pattern gates (e.g. an eslint rule)
  that have no persisted release condition to rot.
STATUS: active
---

# A gate released by a state file needs expiry IN the state, not a cleanup step

Source: reflection `2026-07-02-governance-gates-rot-open.reflection.md` (P0 of the 2026-07-02
meta-loop audit, human-approved). CONFIDENCE: high in the source reflection.

serious-gate and the red-line gate were de-facto open since 06-21/06-23: 400+ blind "ALLOW
cleared" decisions followed the last real DENY, because the per-line clearance state had no
expiry and its cleanup depended on a discipline step that, like every discipline-triggered step
in this system, stopped happening. Registration ("the hook runs") was mistaken for armament
("the hook denies") because nothing measured the latter. Separately, guard-bash was removed
wholesale from settings.json when it over-blocked legitimate staging deploys, rather than
precision-fixed — the correct move per CLAUDE.md is always to narrow, never unregister.

The fix pattern for any future state-backed gate: expiry lives IN the state (not remembered
externally), an armament test proves DENY still fires (not just that the hook executes), and a
log line per decision makes blind-open visible in data instead of silent.
