---
TRIGGER: .github/workflows/**
CAUSE: >
  A failing DB-connection step (ESSLREQUIRED/AUTH/HOST) was diagnosed by assuming the "obvious"
  store (Fly runtime secrets) instead of tracing the failing JOB to the exact secret it reads.
  CI's migrate step (`.github/workflows/ci.yml:153`, `DATABASE_URL_MIGRATIONS:
  ${{ secrets.DATABASE_URL_MIGRATIONS }}`) reads a GITHUB ACTIONS secret, NOT the Fly secret of
  the same name — three same-named copies exist (Fly runtime, GitHub Actions, Supabase role
  password) with no single source of truth. The Fly-side secret was correctly re-set three times
  against a failure whose real source was the stale GitHub secret; each cycle cost a full
  deploy-and-observe loop (~2min, prod-touching) instead of a ~5s local trace+repro.
ACTION: >
  When a DB/secret-dependent CI or deploy step fails (ESSLREQUIRED/AUTH/HOST/timeout) → cause:
  Fly runtime secrets, GitHub Actions secrets, and Supabase role passwords can each hold a
  stale/different copy of "the same" URL → do: (1) `grep` the failing job's actual workflow/config
  for the exact secret reference BEFORE mutating any store (e.g. `grep DATABASE_URL_MIGRATIONS
  .github/workflows/ci.yml` — confirms which store the job reads); (2) run
  `node scripts/ci-connection-preflight.mjs` with the SAME secret value the failing job reads —
  it classifies SSL/AUTH/HOST in seconds, off-prod; (3) only then mutate the store that step 1
  named. Never use the deploy target itself as the diagnostic harness — a deploy-and-observe
  cycle is the most expensive way to answer "which store, which value."
LINK: scripts/ci-connection-preflight.mjs ; .github/workflows/ci.yml:153 ;
  docs/reflections/ARCHIVE/2026-07-03-trace-config-source-before-mutating.reflection.md ;
  docs/regressions/REGRESSION-LEDGER.md #52
SCOPE: DB-connection-dependent CI/deploy failures across the 3 secret stores (Fly runtime /
  GitHub Actions / Supabase role passwords) ONLY. Not general debugging guidance.
STATUS: active
---

# Trace a CI/deploy connection failure to its secret STORE before mutating any store

Source: reflection `docs/reflections/ARCHIVE/2026-07-03-trace-config-source-before-mutating.reflection.md`
(now archived — fully promoted here + into the deterministic guardrail below).

The 2026-07-03 prod-deploy saga lost real time to re-setting the *Fly* secret three times against
an `ESSLREQUIRED` that actually came from the *GitHub Actions* runner. One `grep` at the first
failure would have shown the store in ~5 seconds; a local `ci-connection-preflight.mjs` run gives
the same answer instantly, off-prod. `scripts/ci-connection-preflight.mjs` is the deterministic
backstop (connects with the exact secret a job would use, classifies SSL/AUTH/HOST) — this lesson
is the advisory nudge to reach for it (and to trace-first) before the next multi-secret-store
incident repeats the pattern.
