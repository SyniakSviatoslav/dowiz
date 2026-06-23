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
