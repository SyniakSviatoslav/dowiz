# OPERATOR-APPLY — the one protected line that unblocks the voice FE unit

- **Date:** 2026-07-04 · **Status:** BLOCKED on protect-paths (`apps/web/package.json` is a protected zone)
- **Context:** the voice FE unit (council-APPROVED, dark behind `VITE_VOICE_ENABLED`) is fully staged in the
  working tree: `packages/ui/src/voice/*` (39/39 tests) + barrel export + halo CSS are committed-ready;
  `apps/web/src/lib/voice/*` (19/19) is present but **held uncommitted** because it imports
  `@deliveryos/voice`, which is not declared in `apps/web/package.json`. With pnpm strict linking, a
  clean install (CI / remote Docker build) cannot resolve that import — local runs only work via
  workspace hoisting. Committing it before this line lands would break the staging deploy build.

## The change (apply + approve)

`apps/web/package.json` → `dependencies`, after `"@deliveryos/ui": "workspace:*",`:

```json
    "@deliveryos/voice": "workspace:*",
```

then from the repo root:

```
pnpm install   # updates pnpm-lock.yaml (also protected)
```

## What unblocks after it lands (next session, one unit)

1. Commit `apps/web/src/lib/voice/*` (adapter, 19/19 unit tests) + the 5 confirmation-gate
   boolean-contract fixes in `packages/voice` (already in tree, tests green).
2. Wire the MicFab mount into `ClientLayout` per `FE-INTEGRATION-PLAN.md` (flag-gated
   `VITE_VOICE_ENABLED` + WebGPU/config render-predicate) — connects the PR-2 gate to the PR-3 MicFab.
3. Ledger row for the voice FE unit. Everything stays DARK (flag default off).

## Why not worked around

- Vite/tsc resolve the import per-package under pnpm strict `node_modules`; there is no safe
  hoisting-independent alternative short of vendoring the package (rejected: duplicates a workspace lib).
- protect-paths guards `package.json` + lockfile — this is an operator-approval act by design.
