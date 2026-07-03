# Lessons Index

Machine-parsable lesson table. The PreToolUse hook
`.claude/hooks/pre-edit-lessons.sh` greps this file: for each row whose
`TRIGGER` glob matches the path being edited (or matches a known error
signature), it injects that lesson's `ACTION` + `LINK` before the edit.

Format contract (DO NOT change column order — the hook parses by column):
`| TRIGGER | file |`, one row per lesson. TRIGGER is a path glob or an
error signature. file is repo-relative.

| TRIGGER | file |
|---------|------|
| apps/api/src/routes/auth/** | docs/lessons/2026-06-22-inline-vs-plugin-local-login.md |
| packages/db/migrations/**read*public*menu** | docs/lessons/2026-06-22-read-public-menu-redefine.md |
| packages/ui/src/theme/**.css | docs/lessons/2026-06-23-css-comment-star-slash.md |
| e2e/tests/behavioural-invariants.spec.ts | docs/lessons/2026-06-23-contrast-gate-skip-images.md |
| File has not been read yet | docs/lessons/2026-06-29-read-tool-before-edit.md |
| docs/** | docs/lessons/2026-06-29-docs-only-no-staging-deploy.md |
| .claude/state/** | docs/lessons/2026-07-02-gate-state-file-expiry.md |
| .claude/hooks/** | docs/lessons/2026-07-02-gate-state-file-expiry.md |
| packages/db/migrations/**role** | docs/lessons/2026-07-03-rotate-prod-role-staging-rehearsal.md |
| packages/db/migrations/**grant** | docs/lessons/2026-07-03-rotate-prod-role-staging-rehearsal.md |
| .github/workflows/** | docs/lessons/2026-07-03-secret-store-provenance-trace.md |
| ESSLREQUIRED | docs/lessons/2026-07-03-secret-store-provenance-trace.md |
| packages/db/migrations/** | docs/lessons/2026-07-03-prod-staging-schema-drift.md |
