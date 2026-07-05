---
date: 2026-07-03
slug: trace-config-source-before-mutating
surface: prod-deploy / secrets / diagnostics
qualifies: ">=3 code files + red-line (prod DB, secrets, migrations)"
---

# Trace a failure to its actual config source (and reproduce it cheaply) BEFORE mutating config

## CONTEXT
Landing the 275-commit merge to prod. The deploy failed ~7 times in a row, each on a different
blocker (dowiz_app role, SSL/ESSLREQUIRED, migration 077 owner_id, post-deploy smoke). One
self-inflicted ~5-min prod outage occurred mid-way. Prod eventually landed (v405) and is stable.

## DECISIONS
- Created a bare `dowiz_app` role on prod (skipped BYPASSRLS after the operator's error).
- Set Fly secrets MIGRATIONS/SESSION/OPERATIONAL to `?sslmode=no-verify`.
- Skipped migration 077 on prod (marked applied) for the telegram_connect_tokens owner_id drift.
- Ran operator-authorized prod-secret writes via a script when the operator's `!` exec was broken.

## WHERE
Fly secret store vs GitHub Actions secret store; `.github/workflows/ci.yml:153`
(`DATABASE_URL_MIGRATIONS: ${{ secrets.DATABASE_URL_MIGRATIONS }}`); connection-string sslmode
parsing (node-pg-migrate vs pg.Client); Supabase block-non-SSL toggle vs OPERATIONAL secret.

## WHY (causal — not just where)
The time was not lost to the blockers themselves; it was lost to **acting on the first plausible
hypothesis without first tracing the failing component to the exact source it reads, and without
reproducing the failure in the cheapest available harness.**
- I re-set the *Fly* secret THREE times against an `ESSLREQUIRED` that came from the CI runner —
  which reads the *GitHub* secret. One `grep 'secrets.DATABASE_URL_MIGRATIONS' ci.yml` at the first
  failure would have shown the store in ~5 seconds. I assumed "the deploy uses Fly secrets" because
  that is the *obvious* store, and never traced the data-flow to confirm.
- I discovered the correct sslmode by deploy-and-observe (a ~2-min prod-touching cycle each) — yet
  when I finally ran a 3-second local `pg` connect + an empty-migration-dir `node-pg-migrate` run, it
  gave the exact answer (`no-verify` works, `require`=verify-full fails) instantly and off-prod.
- The outage: OPERATIONAL lacked sslmode; enabling block-non-SSL exposed it. I had not preflighted
  that every runtime pool connects under the new constraint before it took effect.
The common root: **the deploy target was used as the diagnostic harness.** Every guess cost a slow,
prod-risking cycle; every trace/local-repro would have cost seconds and touched nothing.

## CONFIDENCE
High. The Fly-vs-GitHub store confusion and the sslmode-by-deploy loop are directly observable in this
session's command history; the local repro that resolved it in seconds is the counterfactual proof.

## NEXT-TIME
Before mutating ANY config to fix a failure:
1. **Trace the failing component to the exact source it reads** — which secret store, which file line,
   which env var. Never assume the obvious store; `grep` the workflow/config for the symbol.
2. **Reproduce in the cheapest harness that isolates the failure** — a local connection test, an
   empty-migration-dir run — off the deploy target. The deploy is never the debugger.
3. **Preflight the invariant an infra change assumes** (e.g. "all pools carry sslmode") before flipping
   the switch that depends on it.
This is already being ratcheted into a gate: `scripts/ci-connection-preflight.mjs` connects with the
*exact secret the failing job reads* and classifies SSL/AUTH/HOST — turning "guess the store" into a
deterministic pre-deploy check.

## LINK
[[merge-to-main-plan-2026-07-02]] · docs/reflections/ARCHIVE/ci-pre-prod-verification-2026-07-03.md
(the gate-side reflection; this is the method-side) · docs/design/ci-pre-prod-verification/ ·
prod-outage-schema-drift-2026-06-20 (P3 recurrence).

---

**Curation note (librarian, 2026-07-05 daily pass):** Challenged fresh. The causal claim (secret
store confusion cost real time; the deploy target was used as the diagnostic harness) is
concrete and directly evidenced by this session's own command history — not a coincidence of
timing. This reflection was ALREADY fully promoted in a prior pass: `docs/lessons/2026-07-03-
secret-store-provenance-trace.md` names it as its source and its guardrail is ledger row #52,
both verified present and green this pass (`scripts/ci-connection-preflight.mjs` exists, its
sibling `scripts/ci-migration-preflight.mjs --self-test` passes 5/5). The prior pass's own
lesson text said "now archived" but the move never happened — completing that omission now.
Fixed the stale `docs/reflections/INBOX/...` self-reference in the sibling lesson to point at
this file's actual (now correct) ARCHIVE path. Nothing further to promote.
