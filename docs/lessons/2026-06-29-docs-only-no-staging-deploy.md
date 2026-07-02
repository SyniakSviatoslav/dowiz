---
TRIGGER: docs/**
CAUSE: >
  Ship Discipline (commit → staging deploy → Playwright/unit validation) is scoped to changes
  with a runtime surface (product code, config, migrations). A change confined to `docs/**` /
  `*.md` / `*.mdx` (regression ledger, lessons, reflections, ADRs, research notes) has zero
  runtime footprint — nothing is served, built, or executed differently — so a staging deploy
  of a docs-only change verifies nothing that the deploy itself didn't already prove for the
  prior commit.
ACTION: >
  When the full set of changed files in a commit is docs/** and/or *.md/*.mdx (no code, no
  config, no migration, no eslint-plugin-local rule, no test file) → skip the staging-deploy +
  Playwright-validation steps of Ship Discipline; commit is sufficient. The instant ANY
  non-docs file is included in the same change, the full ship-discipline loop (commit → deploy
  → validate) applies to the whole change — do not split a mixed change to dodge it.
LINK: docs/reflections/ARCHIVE/self-improve-2026-06-29.md (PROPAGATE item 2)
SCOPE: docs/**, *.md, *.mdx files ONLY, and only when they are the entire diff of the change.
  Never applies to `.claude/**` (governance-relevant even though markdown-adjacent), and never
  applies once a single code/config/test/migration file joins the same change.
STATUS: active
---

# Docs-only edits need no staging deploy

Source: reflection `self-improve-2026-06-29.md` (already self-applied during that session for
CI-gate/doc-only phases).

Ship Discipline exists to catch runtime regressions before they reach users — it has no signal
to offer on a change that touches only `docs/**`/`*.md` (ledger rows, lessons, ADRs, research
notes), because nothing served by the app changed. Deploying anyway wastes a build/deploy cycle
without adding proof. The moment a change also touches product code, config, a migration, or a
test/guardrail file, treat the whole change as code and run the full loop — this lesson never
licenses skipping validation for a change that happens to include one doc file among others.
