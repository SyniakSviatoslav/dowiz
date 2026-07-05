# Contributing

Thanks for your interest in improving this project. This guide covers how to set
up the project, the quality gates every change must pass, and how we accept
contributions.

## Licensing & the DCO (no CLA)

This project is licensed under **AGPL-3.0-only**. Contributions are accepted
under the **same licence** — there is **no CLA** (Contributor Licence
Agreement) and no copyright assignment. You keep the copyright to your work.

We use the **Developer Certificate of Origin (DCO)**. Every commit must be
signed off, certifying you have the right to submit it under the project's
licence:

```
git commit -s -m "feat(scope): short summary"
```

The `-s` flag appends a trailer:

```
Signed-off-by: Your Name <you@example.com>
```

Use your real name and a reachable email. Read the DCO text at
<https://developercertificate.org/>. PRs without sign-off on all commits will be
asked to amend.

## Development setup

**Prerequisites:** Node.js (LTS), **pnpm**, and Postgres (local, Docker, or a
Supabase project) for anything touching the database.

```bash
pnpm install                 # install workspace deps
cp .env.example .env         # then fill in local values
pnpm verify:env              # sanity-check required env vars
pnpm migrate:up              # apply DB migrations
pnpm seed                    # load local seed data
```

Then start the apps you need (each in its own terminal — there is no root
`pnpm dev`):

```bash
pnpm --filter @deliveryos/api dev     # Fastify API + WebSocket server
pnpm --filter web dev                 # React app (Vite dev server)
pnpm --filter @deliveryos/worker dev  # background worker (optional)
```

The repo is a **pnpm monorepo**:

- `apps/api` — Fastify API + WebSocket server
- `apps/web` — React storefront (`/s/:slug`) + owner admin (`/admin`)
- `apps/worker` — background jobs (queue in Postgres)
- `packages/*` — `db`, `config`, `domain`, `platform`, `shared-types`, `ui`, `voice`

## Quality gates — `verify:all`

Every change must pass the full gate suite before it can be merged:

```bash
pnpm verify:all      # aggregate gate — run this before opening a PR
pnpm typecheck       # TypeScript, clean build
pnpm lint            # ESLint (incl. local guardrail rules)
pnpm test:unit       # unit + integration tests
pnpm verify:rls      # Row-Level Security must hold (tenant isolation)
pnpm verify:migrations
```

These are **verification, not optional style checks.** A red gate blocks the
change. Never weaken, skip (`.only`/`.skip`), inflate timeouts, or otherwise
"cheat green" — a gate that is disarmed to pass is a regression in itself.

## Ship discipline

Non-trivial changes follow the full loop to completion — "code written" is not
"done":

1. **Commit** — a contextual commit (intent + decisions) on a **feature
   branch**, never straight to `main`. The pre-commit hook (lint → typecheck →
   build) must pass.
2. **Deploy** — to **staging** first. If the change adds migrations, run them on
   the staging DB before deploy (the boot-guard fails closed otherwise).
3. **Validate** — run the relevant end-to-end (Playwright) tests against the
   deployed staging URL, plus the change's unit/integration tests and
   `pnpm typecheck`. **Paste the proof.**
4. **Feature-flag** anything not ready to launch (default off). Deploying dark
   code to verify is fine; launching is a separate, explicit act. Production
   ships only on explicit approval / merge to `main`.

## The red→green guardrail rule

> A fix is not **done** without a deterministic **guardrail** that fails before
> the fix and passes after it.

For any bug fix or behaviour change:

1. Write a test (or lint rule / hook) that **fails on the current, broken code**
   (**red**).
2. Apply the fix so the guardrail **passes** (**green**).
3. Include both in the same change, so the bug cannot silently return.

"It should work" is not proof. Proof is an assertion that fails when the code is
wrong. Changes with a **UI surface** need an E2E assertion on real DOM elements;
**API-only** changes need at least one request-level assertion.

## Pull requests

- Keep PRs focused; describe **intent and decisions**, not just the diff.
- Ensure `pnpm verify:all` is green and all commits are DCO-signed.
- Link any related issue/ADR. Architectural changes should reference or add an
  ADR in `docs/adr/`.
- Security-sensitive reports go through [`SECURITY.md`](./SECURITY.md), **not** a
  public PR/issue.

## Code of conduct & ethics

Be respectful and collaborative — see [`CODE_OF_CONDUCT.md`](./CODE_OF_CONDUCT.md)
for the full standard we hold each other to.

This project also carries a standing **ethics charter**: it must never be built
into military, warfare, weapons, or surveillance-for-harm applications.
Contributions serving those ends will be refused. See the ethics section of
[`CODE_OF_CONDUCT.md`](./CODE_OF_CONDUCT.md).
