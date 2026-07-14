# Self-Ecosystem Evolution — protocol

> Operator-granted 2026-07-14. The agent may **correct, modify, and upgrade its own machinery**
> (the harness/governance/tooling it runs on) — but the capability is **self-limiting**: it can
> never lower the product-safety floor. Activation stays with the human; every change is audited.

## The boundary

| Bucket | Paths | Rule |
|---|---|---|
| **Self-ecosystem** — the agent's own machinery | `.claude/**` (hooks, agents, skills, commands, `settings.json`, non-token `state/`), the agent's lessons/ledger/reflections, `tools/` | Agent-modifiable **when** `.claude/state/self-mod-enabled` exists. Every edit appended to `.claude/logs/self-mod.log`. |
| **Red-line floor** — product safety / infra | `packages/db/**`, `migrations/**`, `packages/shared-types/**`, `/contracts/`, `*.contract.*`, `.env`, `.github/**` (CI), `fly.toml`, `Dockerfile`, `pnpm-lock.yaml`, `package.json` | **Unconditional human-gate.** Never self-modifiable, even with the capability ON. |
| **Meta-red-line** — the enforcement itself | the authorization tokens; the red-line patterns inside the gates; the wiring of the red-line hooks | The agent may *refactor* gate files, but `verify-safety-floor.sh` fails loud if any product red-line stops being enforced. The floor may only be **strengthened or scope-corrected, never removed** (monotonic ratchet). |

## Why it is safe (autonomy ≠ arbitrariness)

1. **Operator-only activation.** `self-mod-enabled` is in the red-line set — the agent can neither
   create the token nor edit it. Only a human turns the capability on. The auto-mode classifier
   independently reserves activation for the human, so this is enforced twice.
2. **Self-limiting.** `verify-safety-floor.sh` (a PostToolUse hook + standalone check) asserts 22
   invariants: every product red-line (cookie / insecure-random / float-money / PII / migrations /
   auth / RLS / deploy / CI) is still enforced and every red-line hook is still wired. A self-mod
   that lowers the floor trips it.
3. **External backstop.** `.github/workflows/safety-floor.yml` runs the floor invariant in CI on
   every push. It lives in `.github/` — a red-line path — so it catches even tampering with the
   local hook.
4. **Fully audited.** Every self-ecosystem edit is one row in `.claude/logs/self-mod.log`.
5. **Instantly revocable.** `rm .claude/state/self-mod-enabled` turns the capability off.

## Discipline for a self-mod (every time)

1. **Scope, don't weaken.** Prefer a scope correction (remove a false positive) over relaxing a
   check. If a change touches the red-line floor or its enforcement → it is out of bounds; escalate
   to the human.
2. **Assert the patch.** Anchor edits so a stale anchor aborts rather than silently corrupts.
3. **Prove red→green.** Exercise the changed gate against fixtures; a real red-line must still exit
   non-zero, the fixed false-positive must pass.
4. **Floor stays green.** `bash .claude/hooks/verify-safety-floor.sh` must pass after the change
   (the PostToolUse hook runs it automatically; CI runs it again).
5. **The audit log is the record.** Review it periodically; a self-mod you can't explain is a bug.

## Files

- `.claude/hooks/protect-paths.sh` — the lane (RED-LINE block · self-eco token-gate · audit).
- `.claude/hooks/verify-safety-floor.sh` — the 22-check floor invariant.
- `.github/workflows/safety-floor.yml` — the human-owned CI backstop.
- `.claude/state/self-mod-enabled` — the operator's on-switch (operator-only).
- `.claude/logs/self-mod.log` — the append-only audit trail.
