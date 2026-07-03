# CI pre-prod verification hardening — design proposal

**Status:** DESIGNED. Scripts implemented in `scripts/` (writable). CI wiring staged as a paste-in
proposal (`docs/proposals/ci-pre-prod-verification-wiring.md`) because `.github/` is protect-path.
**Source of failure:** the 2026-07-03 prod-deploy saga (six patterns P1–P6). Causal analysis:
`docs/reflections/INBOX/ci-pre-prod-verification-2026-07-03.md`.

**Principle (harness):** deterministic gates are authority; discipline is not. Every gate below FAILS
the PR/CI — none relies on someone remembering. Every gate exercises the ACTUAL deploy target (or a
prod-faithful clone) BEFORE the irreversible act (migrate + deploy), because the common root of P1–P6
is "the deploy target was never exercised before the deploy."

---

## Gate ↔ pattern map

| Pattern | What bit prod | Deterministic gate that catches it pre-prod | Where it runs |
|---|---|---|---|
| **P1** prod-as-test-bed | serial blocker discovery at deploy | `preflight` job (below) + staging-E2E-on-same-commit `needs:` before `deploy`; `verify:all` as required status check | new `preflight` + `staging-verify` jobs |
| **P2** secret-store fragmentation | fixed Fly secret for a CI problem | `ci-connection-preflight.mjs` connects with the secret **the job actually reads** and names the store on failure | `preflight` job (uses same `secrets.*` as `deploy`) |
| **P3** prod≠staging drift | 077 `owner_id` vs prod `user_id`; missing `dowiz_app` | `ci-migration-preflight.mjs` (LIGHT: assert pending-migration refs exist on prod; FULL: pg_dump→scratch→migrate) + `ci-schema-drift.mjs` (staging vs prod) | `preflight` job (read-only prod introspection) |
| **P4** SSL config untested | `require`→verify-full→cert reject | `ci-connection-preflight.mjs` connects with each URL's exact sslmode before migrate | `preflight` job |
| **P5** infra-change outage | "block non-SSL" downed runtime pools | `ci-connection-preflight.mjs --require-all` over OPERATIONAL+SESSION as a **manual preflight before flipping the toggle** (+ optional scheduled run) | ops runbook + optional `schedule:` workflow |
| **P6** post-deploy smoke too late + prod-incompatible | 0.1 dev-login can't pass on prod; runs after live | move mutating deploy-validation to **staging pre-prod**; prod post-deploy = read-only health/menu smoke (no dev-login) | `staging-verify` (pre) + slim prod smoke (post) |

---

## Per-pattern gate detail

### P1 — prod-as-test-bed → a `preflight` job + staging-on-same-commit
Two additive gates:
1. **`preflight` job** that runs the connection + migration preflights and that `deploy` **`needs:`** —
   so a broken secret / SSL / schema-drift fails the pipeline *before* `migrate:up` and `flyctl deploy`.
2. **`staging-verify` job** that deploys the SAME commit to staging and runs the E2E suite there; the
   prod `deploy` job `needs: [validate, preflight, staging-verify]`. This encodes Ship Discipline
   (commit→staging→validate→prod) as CI topology instead of a runbook step humans skip.
Together they remove the property that made prod the integration test: nothing reached prod that hadn't
already migrated + passed E2E against a prod-shaped target on the same commit.

### P2 — secret fragmentation → connection preflight in the deploying job
`ci-connection-preflight.mjs` reads `DATABASE_URL_MIGRATIONS / _OPERATIONAL / _SESSION` from env and
connects with each, running `select 1`. Wired into a job that receives the **same `secrets.*`** the
`deploy` job uses, it proves the GitHub-Actions copy connects — the exact copy `migrate:up` will use —
so a wrong/missing/stale secret fails in CI naming the store, not on prod naming the wrong store.
Output classifies SSL vs AUTH vs HOST and redacts the password.

### P3 — prod≠staging drift → migration preflight (see A/B below) + schema-drift diff
`ci-migration-preflight.mjs` determines the PENDING set (migration files not in prod's `pgmigrations`)
and validates each against prod's actual schema. `ci-schema-drift.mjs` diffs staging vs prod column
sets (scoped to migration-referenced tables) so silent staging drift — the thing that made "tested on
staging" a false proof — is itself a red CI signal. `--self-test` proves the extractor catches the 077
case (`telegram_connect_tokens` + `owner_id` + `dowiz_app`) with no DB. **This is the highest-value
gate** — it directly closes the schema/role drift that caused the most blockers.

### P4 — SSL untested → connection preflight uses connection-string sslmode
The preflight constructs `new Client({ connectionString })` and lets the repo's pg parse `sslmode` —
the *identical* parse path node-pg-migrate and the app use. So `sslmode=require` failing verify-full,
or a missing sslmode raising `ESSLREQUIRED`, reproduces in CI exactly as it would on prod, and the
error message hints at `sslmode=no-verify` for the self-signed Supabase pooler.

### P5 — infra-change outage → preflight the constraint before flipping it
Before enabling a DB-side constraint (e.g. "block non-SSL"), run
`DATABASE_URL_OPERATIONAL=… DATABASE_URL_SESSION=… node scripts/ci-connection-preflight.mjs --require-all`
against the RUNTIME pool URLs. If any runtime pool can't connect under SSL, it fails BEFORE the toggle
takes prod down. Add to the infra-change runbook as a required step; optionally a `schedule:` workflow
runs it hourly so runtime-secret rot is caught proactively, not at the next boot.

### P6 — smoke too late + prod-incompatible → split the smoke by target
- **Pre-prod (staging):** the mutating `deploy-validation.spec.ts` (which needs dev-login + writes test
  data) runs in `staging-verify` against staging — its authored target (`requireStaging(BASE)` already
  enforces this; it defaults BASE to staging). This gates the deploy.
- **Post-prod:** a NEW read-only prod smoke (health 200 + a published `/s/:slug` menu renders + an
  unauthenticated endpoint returns 401) that needs NO dev-login, so it can actually be green on prod
  and detect a bad deploy without the false-red that trained operators to ignore it.

---

## Back-of-envelope CI cost

| Gate | Extra CI minutes | Notes |
|---|---|---|
| `ci-connection-preflight` | ~0.2–0.5 min | three `select 1` connects; network-bound |
| `ci-migration-preflight` LIGHT | ~0.3–0.6 min | one read-only introspection + regex over pending files |
| `ci-migration-preflight` FULL | ~1.5–4 min | pg_dump --schema-only (prod) + load + migrate; scales with schema size |
| `ci-schema-drift` | ~0.3–0.6 min | two `information_schema` snapshots + diff |
| `staging-verify` (deploy + E2E) | ~6–12 min | dominated by staging `flyctl deploy` + Playwright; the real cost |
| prod read-only post-smoke | ~1–2 min | a handful of `request.*` assertions |
| **preflight job total (LIGHT)** | **~1–2 min** | the cheap, must-run tier — wire this first |

The preflight tier is <2 CI-min and pays for itself the first time it fails a PR instead of prod.
`staging-verify` is the expensive tier but it is the existing Ship-Discipline cost merely moved into CI.

---

## Migration preflight — Option A vs Option B (the big decision)

The migration preflight must validate PENDING migrations against **prod's actual schema**. Two ways:

### Option A — clone prod schema each run (`pg_dump --schema-only` → scratch → migrate)
Every CI run does `pg_dump --schema-only` from prod (read-only), loads it into an ephemeral scratch
Postgres (the `fresh-provision` job already stands one up), then runs `migrate:up` against it.
- **Pro:** highest fidelity — a REAL apply against prod's real schema; catches *any* drift class
  (columns, constraints, types, functions, roles-if-dumped), not just the ones a regex anticipates.
  Zero maintenance — the "snapshot" is always current because it's taken live.
- **Con:** CI needs read access to prod (a scoped read-only role, extra secret) + `pg_dump` on the
  runner; ~1.5–4 CI-min; a prod dependency in the CI critical path (prod down → CI blocked, though
  that's arguably correct).
- Implemented as `ci-migration-preflight.mjs --full` (auto-skips if `pg_dump`/`SCRATCH_URL` absent).

### Option B — maintained committed schema snapshot
Commit a `schema.sql` (dumped periodically) into the repo; CI loads THAT into scratch and migrates.
- **Pro:** no prod access from CI; fully hermetic; fast; snapshot is diffable in PRs.
- **Con:** the snapshot **drifts from prod** — which is the exact failure mode that caused P3. It is
  only as good as the last refresh, and staleness is invisible until it bites. Requires discipline
  (a scheduled refresh job) — and "requires discipline" is what we're trying to eliminate.

### Recommendation
**Adopt Option A as the authority** (FULL mode) for the pre-deploy gate — it cannot go stale, which is
the whole point after P3. Use the **LIGHT mode** (regex refs vs live prod `information_schema`, no
pg_dump, no scratch) as the **always-on cheap tier** in the `preflight` job — it needs only a
read-only prod connection and catches the missing-column / missing-role class that actually bit us.
If granting CI any prod access is unacceptable, fall back to Option B **plus** `ci-schema-drift.mjs`
run on a schedule (staging-vs-prod) so snapshot staleness is at least detected. Do NOT adopt B alone —
that reproduces P3.

---

## What is explicitly NOT solved by tooling (raise to operator)
- **Single source of truth for secrets (P2 root).** These gates DETECT a drifted/wrong secret; they do
  not unify the three stores. A real fix is one canonical store (e.g. Fly as SoT, GitHub secrets synced
  from it) — an operator/infra decision, flagged not implemented.
- **Read-only prod role for CI (needed by P3 LIGHT + A).** Requires the operator to mint a scoped
  `SELECT`-only role + add it as a GitHub secret. No credentials are hardcoded anywhere.
