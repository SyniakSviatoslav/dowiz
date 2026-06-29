# MVP Phase 0 — ship-blocker operator patches

All three touch protect-paths zones (`.github/`, `packages/db/migrations/`) → operator-applied. Staged
here ready to apply. Source: the MVP-ship plan + the CLEAN-seed/launch audit.

## 1. Pin the mutable GitHub Action to a SHA (`.github/workflows/ci.yml:150`)
`superfly/flyctl-actions/setup-flyctl@master` is a moving tag (supply-chain risk). Pin to the resolved SHA:
```diff
-        uses: superfly/flyctl-actions/setup-flyctl@master
+        uses: superfly/flyctl-actions/setup-flyctl@ed8efb33836e8b2096c7fd3ba1c8afe303ebbff1  # master @ 2026-06-29
```
(Also consider pinning `actions/checkout@v4`, `pnpm/action-setup@v3`, `actions/setup-node@v4` to SHAs.)

## 2. Close the RLS `WITH CHECK` asymmetry (migration)
`customers.anonymous_update` (mig `1780338981782`) has `USING` but no `WITH CHECK`. Artifact:
`docs/security/RLS-customers-with-check.migration.ts` — place as
`packages/db/migrations/<next>_customers-anon-update-with-check.ts`, apply staging-first, `pnpm verify:rls`.
Follow-up guardrail: lint every `FOR UPDATE` policy to require a `WITH CHECK`.

## 3. Apply the already-authored pg-privilege migrations (greens `verify:rls`)
`docs/security/OPERATIONAL-ROLE-nobypassrls.migration.ts` (MIG-ITEM2, FIRST) +
`docs/security/SECURITY-DEFINER-search-path.migration.ts` (MIG-ITEM1) — see
`docs/security/pg-privilege-hardening-OPERATOR-HANDOFF.md`. Until applied, the hot-path role keeps
BYPASSRLS and `verify:rls` (a launch-checklist gate) cannot pass.

## Order
Apply 2 + 3 together on staging (both RLS), run `pnpm verify:rls` + the lifecycle E2E, then prod. (1) is
independent and ships with the next CI change. All reversible (down()/git revert); no data mutation.
