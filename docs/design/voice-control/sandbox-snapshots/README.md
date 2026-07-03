# Voice FE — sandbox snapshots (preservation, NOT integration)

> **Why this exists.** On 2026-07-03 the voice front-end was found to exist **only as
> untracked working-tree files inside three throwaway agent worktrees**
> (`.claude/worktrees/agent-*`), all based on `c8b2d5a0` — **13 commits behind HEAD** —
> with **zero commits** on their branches. `scripts/sandbox-swarm-gate.mjs rm --apply`
> runs `git worktree remove --force`, which would have silently destroyed ~1,800 lines
> of council-reviewed work. These archives freeze the exact bytes on a durable, tracked
> branch so nothing is lost. **They are inert** (`.tar.gz`, invisible to lint/tsc/build)
> and are **not** wired into any live import path.

## Archives

| File | Source worktree | Base | Contents |
|------|-----------------|------|----------|
| `a271-voice-ui.tar.gz` | `agent-a271635061d020c67` | `c8b2d5a0` (13 behind HEAD) | The **UI** — `packages/ui/src/voice/` (MicFab, ConfirmChip, ReadBackPanel, DisambiguationChips, ErrorPill, PartialTranscriptPill, DisclosureSheet, VoiceSettingToggle, `state-machine.ts`, `useVoiceControl.ts`, `layout.ts`, `types.ts` + `__tests__/`), the PR-3 council docs, and `PHASE1-IMPLEMENTATION-PLAN.md` |
| `ad77-voice-adapter.tar.gz` | `agent-ad77f8d52cb4f2e8d` | `c8b2d5a0` (13 behind HEAD) | The **app-adapter** — `apps/web/src/lib/voice/` (`handlers.ts`, `gate.ts`, `menuContext.ts`, `types.ts`, `voiceAdapter.test.ts`) |

`agent-afde4fe249e5c3eb0` held no voice work (an abandoned stub) and is not snapshotted.

The two archives are **complementary, not competing**: `a271` = presentation, `ad77` =
engine→app glue. A real integration lands both.

## This is preservation, not a merge

A naive merge of these stale branches would **revert 240 files of audit-remediation work**.
Do **not** extract these to their live paths and commit. The real integration is a separate,
gated exercise:

1. `git worktree add` a fresh sandbox on **current HEAD**.
2. Extract these archives into it; **rebase/replay** the voice FE on top of HEAD (resolve
   the 13-commit drift — money/authz/RLS remediation touched shared surfaces).
3. Wire `packages/voice` (read-only engine) → `apps/web/src/lib/voice` (adapter) →
   `packages/ui/src/voice` (UI). Engine stays write-incapable; `ConfirmationGate` is the
   sole write sink (ADR-0015 §6; guardrails `no-voice-app-import`, `no-voice-engine-callback`).
4. Run the **Sandbox-Swarm-Gate** rubric over the diff (`node scripts/sandbox-swarm-gate.mjs
   checklist`) + invariant-guardian + security-sentinel.
5. Mandatory Proof: Playwright E2E on staging (`/s/:slug` mic → confirm → cart) +
   `packages/ui` unit + `pnpm typecheck`.
6. Flag `VITE_VOICE_ENABLED` default-OFF; deploy dark; launch is a separate explicit act.

## To extract (into a fresh sandbox only)

```sh
tar -xzf docs/design/voice-control/sandbox-snapshots/a271-voice-ui.tar.gz   -C <fresh-sandbox>
tar -xzf docs/design/voice-control/sandbox-snapshots/ad77-voice-adapter.tar.gz -C <fresh-sandbox>
```
