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

<!-- Empty as of 2026-07-20: all 4 prior rows (apps/api/**, packages/db/**, packages/ui/**,
     e2e/tests/behavioural-invariants.spec.ts) pruned by weekly librarian curation — their
     TRIGGER paths were deleted wholesale by the legacy-thin-layer removal (REGRESSION-LEDGER
     row #21, 2026-07-13; see root CLAUDE.md "drop js"). See
     docs/reflections/ARCHIVE/2026-06-22-read-public-menu-stale-base.reflection.md for detail. -->
