---
TRIGGER: packages/db/migrations/**
CAUSE: >
  "Tested on staging" (or "applies clean to a fresh DB") is NOT proof a migration is safe on
  PROD when staging has silently drifted from prod via out-of-band DDL. 2026-07-03: migrations
  077-082 `GRANT ... TO dowiz_app` (role existed on staging, NOT on prod) and migration 077's RLS
  policy keyed `telegram_connect_tokens.owner_id` (prod's live column was still `user_id`) — both
  invisible until `migrate:up` actually ran against prod. This is a RECURRING class: the
  2026-06-20 outage (ledger #3) was schema drift between what CI deployed and what the DB head
  actually was, same root shape ("the validation target was not the deploy target").
ACTION: >
  Before merging/deploying any migration → cause: staging / a fresh DB are stand-ins that can
  silently diverge from prod's actual schema, roles, and grants → do: run
  `node scripts/ci-migration-preflight.mjs` (LIGHT mode — extracts every table/role/column the
  PENDING migrations reference and asserts each exists against `SOURCE=prod`, read-only) AND
  `node scripts/ci-schema-drift.mjs` (diffs staging vs prod's column sets, scoped to
  migration-referenced tables) BEFORE the migration runs against prod. A migration that only ran
  clean on staging or a throwaway fresh DB is not proof it will run clean on prod — only a
  preflight against prod's actual schema/roles is.
LINK: scripts/ci-migration-preflight.mjs ; scripts/ci-schema-drift.mjs ;
  packages/db/migrations/1790000000077_rls-nobypassrls-phase1-policies.ts:102-106 (owner_id drift) ;
  docs/regressions/REGRESSION-LEDGER.md #3 (2026-06-20, first occurrence), #52 (this occurrence's gate)
SCOPE: packages/db/migrations/** ONLY — any migration file, since drift can affect any
  table/role/column/grant it references. Does not apply to application code.
STATUS: active
---

# Prod≠staging schema drift is a RECURRING class — preflight against the real deploy target

Source: `docs/reflections/ARCHIVE/ci-pre-prod-verification-2026-07-03.md` (P3, "highest-value
gap"); prior occurrence `prod-outage-schema-drift-2026-06-20` (ledger #3).

Two occurrences now share one shape: the DB you validated against (staging, or a fresh migrated
DB) was not the DB you deployed to (prod), and the two had quietly diverged (a role that exists
on one, not the other; a column renamed on one, not the other). `scripts/ci-migration-preflight.mjs`
+ `scripts/ci-schema-drift.mjs` are the deterministic backstop — built this session, self-tested
(the migration-preflight extractor is proven to catch the exact `telegram_connect_tokens.owner_id`
+ `dowiz_app` drift class with `--self-test`, no DB required) but **not yet wired into CI**
(`.github/workflows/ci.yml` is a protect-path — wiring is staged as
`docs/proposals/ci-pre-prod-verification-wiring.md` for operator application). Until wired, this
lesson is the advisory reminder to run them by hand before a migration touches prod.
