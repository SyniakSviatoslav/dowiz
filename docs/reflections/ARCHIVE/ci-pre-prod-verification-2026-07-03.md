# Reflection: prod-as-test-bed — the 2026-07-03 deploy saga (six failure patterns, causal roots)

## CONTEXT
Merging the 275-commit integration branch to `main` triggered a prod deploy that failed serially:
7+ distinct blockers, each discovered only by deploying to prod and watching it break, each fixed,
re-deployed, next blocker surfaced. One infra toggle ("block non-SSL") caused a ~5-min prod OUTAGE.
Operator directive after: make it impossible for THESE CLASSES to be discovered on prod — they must
fail in CI/staging first. This reflection records the causal WHY per pattern (root, not symptom); the
deterministic gates that follow from each root live in `docs/design/ci-pre-prod-verification/proposal.md`
and the implemented scripts in `scripts/ci-*preflight*.mjs` + `scripts/ci-schema-drift.mjs`.

## WHY-causal (per pattern — root, not symptom)

**P1 · PROD-AS-TEST-BED.** Symptom: every blocker found on prod. Root: the deploy path is the FIRST
place the merged commit is exercised end-to-end against real infra. `verify:all` runs only in CI's
`validate` job (not pre-commit/pre-push), and the integration branch was merged to `main` while never
CI-green on the merge commit — so the deploy job became the integration test. Category: **absent gate,
not weak discipline.** There was no job that stood between "merge" and "prod migrate+deploy" that
exercised the same commit against a prod-shaped target. Serial discovery is the signature of a missing
fail-fast preflight: each fix only reveals the next because nothing ran them all together before prod.

**P2 · SECRET-STORE FRAGMENTATION.** Symptom: hours lost fixing the *Fly* secret for a *CI-side*
failure. Root: three secret stores (Fly runtime secrets, GitHub Actions secrets, Supabase role
passwords) each hold a copy of "the DB URL", with **no single source of truth and no check that the
copy each store holds actually connects.** The CI "Migrate Database" step reads
`${{ secrets.DATABASE_URL_MIGRATIONS }}` — a GitHub secret — but the operator's mental model pointed at
Fly. Category: **provenance ambiguity.** Nothing asserted "the secret THIS job will use connects,"
so the failure surfaced as a symptom in the wrong store and debugging chased the wrong copy.

**P3 · PROD≠STAGING DRIFT (highest-value gap).** Symptom: migration 077 created an RLS policy on
`telegram_connect_tokens.owner_id`, but prod's table has `user_id` (staging had drifted to `owner_id`);
and 077-082 `GRANT … TO dowiz_app`, but `dowiz_app` didn't exist on prod. Root: **the validation
targets were not the deploy target.** Migrations were proven on (a) staging and (b) a fresh clean DB —
neither of which equals prod's ACTUAL schema/roles. "Tested on staging" was a **false proof** because
staging had silently drifted (out-of-band DDL: the `owner_id` re-key, the `dowiz_app` bootstrap). The
deeper root is the same category as the plane-telemetry "durable-local illusion": correctness is a
property of the *deploy target's* schema, not of any convenient stand-in. Two stand-ins that agree with
each other but not with prod produce confident wrong answers.

**P4 · SSL/CONNECTION CONFIG NOT PREFLIGHTED.** Symptom: connection strings failed serially (no
sslmode → `ESSLREQUIRED`; `sslmode=require` → node-pg does verify-full → self-signed pooler cert
rejected; `sslmode=no-verify` → works). Root: **SSL semantics are driver-specific and untested until
first connect.** node-pg's interpretation of `require` (verify-full) differs from the operator's
expectation (encrypt, don't verify). Nothing connected with each URL *before* the step that needed it,
so the correct sslmode was found by trial-and-error against prod. Category: **untested config
assumption** — a config value's meaning was assumed, never asserted.

**P5 · INFRA-CHANGE OUTAGE.** Symptom: enabling Supabase "block non-SSL" took prod down ~5 min because
the OPERATIONAL Fly secret (`deliveryos_api_user@6543`) lacked sslmode → app couldn't boot. Root:
**an irreversible-in-effect infra constraint was flipped without a preflight that all runtime pools
satisfy it first.** Same class as P4 (untested SSL) but on the RUNTIME pools and triggered by an infra
toggle rather than a deploy. The constraint changed the environment under a running app; no gate
proved the app's live connection strings would survive the new constraint before it took effect.

**P6 · POST-DEPLOY VALIDATION TOO LATE + PROD-INCOMPATIBLE.** Symptom: the `deploy` job's post-deploy
smoke (`e2e/tests/deploy-validation.spec.ts`) runs AFTER prod is live (can't gate the deploy), and its
test 0.1 "local login returns a valid owner token" can never pass on prod (prod closes the DEV_AUTH
local-login backdoor, ADR-0003; the spec itself `requireStaging(BASE)`-guards and defaults BASE to
staging). Root: **the validation was authored for a mutating, dev-auth-enabled STAGING target but wired
to run against PROD** — a target/harness mismatch. A smoke that cannot be green on its target is not a
gate; it's noise that trains operators to ignore red. And running it post-deploy means even a real
failure can't prevent the bad deploy.

## COMMON ROOT (the systemic one)
Five of six (P1–P6 except the pure timing half of P6) reduce to ONE structural cause:
**the deploy target was never exercised before the deploy.** Different faces — schema (P3), roles (P3),
secrets/provenance (P2), SSL (P4/P5), end-to-end (P1) — but one shape: *validation ran against a
stand-in (staging / fresh DB / a different secret store / the operator's assumption), never against a
prod-faithful target, before the irreversible act (migrate + deploy).* The fix class is therefore ONE
idea applied per surface: **a preflight that connects to / clones from / runs against the actual
deploy target, and fails CI, positioned BEFORE migrate+deploy.**

## WHERE (implemented this session — scripts/ only; .github + packages/db staged as docs)
- `scripts/_pg-loader.mjs` — resolves the repo's pg@8.21.0 from the pnpm store; redaction + error
  classifier (SSL/AUTH/HOST/OTHER) shared by all three gates.
- `scripts/ci-connection-preflight.mjs` — P2/P4/P5: connects with each DATABASE_URL_*, `select 1`,
  names which url + whether SSL vs AUTH vs HOST failed. Proven: HOST-classifies a bad host, redacts
  password, exit 1 on fail / 0 on all-connect.
- `scripts/ci-migration-preflight.mjs` — P3: LIGHT mode (extract tables/columns/roles from PENDING
  migrations, assert they exist on SOURCE=prod read-only) + FULL mode (pg_dump --schema-only → scratch
  → migrate:up). `--self-test` PROVES the extractor captures telegram_connect_tokens/owner_id + dowiz_app
  (the exact 077 drift) with no DB.
- `scripts/ci-schema-drift.mjs` — P3: diffs two DBs' public column sets (scoped to migration-referenced
  tables), exit 1 on drift — makes "staging drifted from prod" a visible CI failure.
- `docs/design/ci-pre-prod-verification/proposal.md` — per-pattern gate design, CI-minute costs, the
  A-vs-B tradeoff for the migration preflight.
- `docs/proposals/ci-pre-prod-verification-wiring.md` — the exact staged `.github/workflows/ci.yml`
  edits (protect-path — operator pastes) + pre-push hook proposal + branch-protection note.

## CONFIDENCE
High on the causal analysis (each root is reproducible from the artifacts in the CI file + migration 077
+ deploy-validation spec, all read this session) and on the LIGHT-mode gates (self-test + dry-runs
green, red arms proven). Medium on FULL mode until it runs once in CI against a real prod pg_dump
(pg_dump is present locally; the scratch-DB apply path is implemented but unexercised end-to-end here).

## NEXT-TIME
- Before accepting "tested on X" as proof for a deploy to Y, ask: **is X byte-for-byte the deploy
  target's schema/roles/secrets/SSL, or a stand-in?** If a stand-in, the proof is conditional on
  zero-drift — so gate the drift.
- Every irreversible act (migrate, deploy, infra toggle) needs a preflight that exercises the ACTUAL
  target and fails BEFORE the act — never a post-hoc smoke.
- A validation that cannot be green on its wired target is worse than none: it normalizes red. Wire
  smokes to the target they were authored for.

## LINK
- [[merge-to-main-plan-2026-07-02]] · [[staging-db-access-2026-06-30]] · [[deploy-topology]]
- [[prod-outage-schema-drift-2026-06-20]] (P3 recurrence — schema drift bit prod BEFORE; this is the
  second occurrence → promote from lesson to gate)
- [[plane-telemetry-closed-loop-2026-07-02]] (durable-local illusion = same "property-of-the-target"
  category error as prod≠staging)
- docs/design/ci-pre-prod-verification/proposal.md · docs/proposals/ci-pre-prod-verification-wiring.md

---

**Curation note (librarian, 2026-07-06 weekly pass):** Council retro — cause-critic CONFIRMS the
common root across all six patterns (high confidence). Already fully distilled in a prior pass
into `docs/lessons/2026-07-03-prod-staging-schema-drift.md` and
`docs/lessons/2026-07-03-rotate-prod-role-staging-rehearsal.md` (ledger rows #51, #52); that
lesson's Source line already referenced this file as archived, but the move never happened —
performed now. CI wiring for `scripts/ci-connection-preflight.mjs` /
`scripts/ci-migration-preflight.mjs` / `scripts/ci-schema-drift.mjs` remains a staged operator
proposal (`docs/proposals/ci-pre-prod-verification-wiring.md`, protect-path) — carried forward
as a PR proposal, not enacted here (out of librarian's writable scope). No new lesson needed.
