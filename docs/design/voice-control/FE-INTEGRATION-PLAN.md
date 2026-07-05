# Voice FE — scoped integration plan (rebase-first, gated)

> The voice FE exists as preserved snapshots (`sandbox-snapshots/`), NOT on-branch. The three source
> worktrees are 14 commits behind HEAD; a naive merge reverts 240 files of audit remediation. This is
> the **real** integration: rebase the FE onto current HEAD, wire it, gate it, prove it. Red-line-adjacent
> (voice→cart) — runs the serious-gate / triadic council per the harness before landing.

## Preconditions (done)

- ✅ Bytes preserved inert: `docs/design/voice-control/sandbox-snapshots/{a271-voice-ui,ad77-voice-adapter}.tar.gz` (commit `a43485d0`).
- ✅ Engine + PR-0 guardrails already on `main`: `packages/voice` (read-only), `no-voice-app-import`,
  `no-voice-engine-callback`, `capability-table` (ledger #62/#63).
- ✅ Staleness now gated: `guardrail-sandbox-staleness.mjs` (ledger #68).

## Steps

1. **Fresh sandbox on HEAD.** `node scripts/sandbox-swarm-gate.mjs new voice-fe --apply` (branch `ssg/voice-fe`, base HEAD).
2. **Extract the snapshots** into it (purely additive new paths — no conflicts):
   `packages/ui/src/voice/*` (UI) + `apps/web/src/lib/voice/*` (adapter).
3. **Compile-gap probe (do this FIRST).** `pnpm typecheck` in the sandbox. The FE was authored against
   `c8b2d5a0`; 14 commits of drift touched `packages/voice` types, `packages/ui` exports, and the cart
   store. The typecheck delta *is* the integration work-list — resolve against current signatures, do
   not downgrade types.
4. **Wire the three tiers** (respect ADR-0015 §6): `packages/voice` (engine, write-incapable) →
   `apps/web/src/lib/voice` (adapter, `ConfirmationGate` the SOLE write sink) → `packages/ui/src/voice`
   (MicFab/ReadBack/Confirm UI). The engine must never import an app/fetch/Cart mutator
   (`no-voice-app-import` enforces this — keep it green).
5. **Flag it dark.** `VITE_VOICE_ENABLED` default-OFF; deploying dark is fine, launch is a separate act.
6. **GATE** the diff: `node scripts/sandbox-swarm-gate.mjs plan voice-fe` → invariant-guardian +
   security-sentinel + the §4 rubric. Voice→cart is red-line-adjacent → **serious-gate / triadic council**
   clearance before merge.
7. **Mandatory Proof:** Playwright E2E on staging (`/s/:slug` mic → partial transcript → read-back →
   confirm → cart line appears via `toBeVisible`), `packages/ui` voice unit (state-machine +
   equal-affordance, already in the snapshot), `pnpm typecheck` whole-repo.
8. **Ship discipline:** commit → staging deploy (`VITE_VOICE_ENABLED` build-arg) → validate → merge.
   Add a ledger row (the wiring is a behavior change) + `guardrail-sandbox-staleness --ci` green
   (the sandbox resolved).

## Why not this session

Steps 3–7 are a focused, red-line-adjacent block that deserves its own session + council clearance —
not a tail-end rush after a large harness build. The scoping above IS the first real step; execution
starts at step 1 on request.
