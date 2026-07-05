SWEEP RULE — `@ts-ignore` → `@ts-expect-error`

Mechanical, well-scoped codemod (Tier 3). Replace each `// @ts-ignore` directive
comment with `// @ts-expect-error`, preserving any trailing explanation text on
the same line.

- Change ONLY the directive token `@ts-ignore` → `@ts-expect-error`.
- Do NOT touch the suppressed line, surrounding code, imports, or formatting.
- Do NOT add, remove, or reorder any other line.
- If the file contains no `@ts-ignore`, make NO change at all.

Rationale (for the proposal, not for you to act on): `@ts-expect-error` is the
preferred TS directive — it errors if the suppression ever becomes unnecessary,
so it cannot silently rot. Runtime behavior is identical.
