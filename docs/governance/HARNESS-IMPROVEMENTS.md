# Harness Improvements — PROPOSED diffs (operator applies by hand)

> Target named in `docs/design/harness/SYSTEMS-MAP.md`'s Harness-Improvements-proposals row
> (backlog item 5): "reduce harness friction... that the operator applies by hand (protected
> files)". Every file this doc proposes to touch is a protected zone (`.claude/hooks/`,
> `.claude/settings.json`, `.github/workflows/`) — the autonomous-continuation task that wrote
> this doc is explicitly forbidden from editing those files itself. This is a **reviewable diff
> document**, not a change. Nothing here is applied. The operator reviews each proposal, edits
> the real file by hand (or asks an agent to, in a session with that authorization), and re-runs
> the relevant guardrail (`guardrail-hook-matchers.mjs`, `guardrail-gate-armament.mjs`,
> `lint:gates`) before committing.

Each proposal below states: the **friction observed** (with evidence), the **exact diff**, and
the **residual risk** the operator is accepting by applying it.

---

## P1 — move the Docker build out of pre-commit, into CI

**Friction observed.** `.husky/pre-commit` step "5/5" runs `docker build -t dowiz-check .` on
**every commit**, guarded only by a disk-space threshold
(`scripts/docker-disk-guard.sh`, added after a real incident —
`docs/incidents/2026-06-28-local-pg-disk-crash.md`, where ~50 uncommitted-layer commits filled
`/` and crashed the local Postgres data dir mid-WAL). The build's own comment already
acknowledges it's non-authoritative: "Fly.io builds in cloud — skipping" is the fallback on any
local Docker/network failure, and `.github/workflows/ci.yml`'s `deploy` job already performs the
real, authoritative build via `flyctl deploy --remote-only` on merge to `main`. So the local
build is a slow (multi-minute), disk-risky, non-blocking-on-failure check that duplicates a
build CI does anyway — the worst combination of "costs time" and "proves nothing between
here and prod parity" (a local Docker daemon can differ in base-image cache state from Fly's
remote builder).

**Exact diff.**

`.husky/pre-commit` — delete step "5/5" (the whole `echo "5/5: Checking Fly.io Build (Docker)..."`
block through its closing `fi`), keep step "4/5" (Fly config validate) as the last step,
renumbering it if desired:

```diff
-echo "5/5: Checking Fly.io Build (Docker)..."
-if command -v docker >/dev/null 2>&1; then
-  # DISK GUARD (regression fix — docs/incidents/2026-06-28-local-pg-disk-crash.md): ...
-  bash scripts/docker-disk-guard.sh 15
-  AVAIL_GB=$(df -BG --output=avail / 2>/dev/null | tail -1 | tr -dc '0-9')
-  if [ -n "$AVAIL_GB" ] && [ "$AVAIL_GB" -lt 8 ]; then
-    echo "Low disk (${AVAIL_GB}G free after prune) — skipping the local Docker build to protect the disk (cloud build still gates deploy)."
-  elif docker build -t dowiz-check . 2>&1; then
-    echo "Fly.io (Docker) build successful!"
-  else
-    echo "Warning: Docker build failed (local network/env issue). Fly.io builds in cloud — skipping."
-  fi
-  bash scripts/docker-disk-guard.sh 20
-else
-  echo "Docker is not installed or not running. Skipping Fly.io build check."
-fi
-
 echo "=== All checks passed! ==="
```

`.github/workflows/ci.yml` — add a `docker-build` job to the `validate` stage (build-only, no
push, no deploy credentials needed) so a broken Dockerfile is still caught before merge, just
once per PR instead of once per local commit:

```diff
 jobs:
   validate:
     runs-on: ubuntu-latest
     steps:
       - uses: actions/checkout@v4
       ...
       - name: Compliance gate (privacy-invariants)
         run: pnpm compliance:gate
+
+      - name: Docker build (parity check, no push)
+        run: docker build -t dowiz-ci-check .
```

**Residual risk the operator accepts.** A locally-broken Dockerfile is no longer caught until
push (was: every commit). This is the intended trade — `deploy` already gated the *authoritative*
build; this proposal removes a slow, disk-risky, redundant local copy of that same check and
relies on CI's `validate` job (which already blocks `deploy` via `needs: validate`) to catch it
before merge instead. `scripts/docker-disk-guard.sh` stays in the repo (still useful standalone,
e.g. before a manual `flyctl deploy` from a dev machine) but is no longer invoked by the hook.

---

## P2 — tier the protect-paths guard: hard-block only true red-lines, allow-with-log the rest

**Friction observed.** `.claude/hooks/protect-paths.sh`'s `PROTECTED` regex is flat — it
hard-blocks (`exit 2`, no override short of a human editing the hook) *any* edit whose path
matches, with no distinction between e.g. a schema migration (genuinely red-line: irreversible,
data-affecting) and a one-line `devDependencies` version bump or a new script entry in a
`package.json` (currently blocked outright by `/package\.json$` with zero tier — even adding an
npm script to `apps/api/package.json` requires the operator to hand-edit it). Same shape in
`guard-bash.sh`'s Bash-mirror `PROTECTED` pattern. The regression-ledger and AUTONOMOUS-STATUS
history already show every autonomous run hand-writing the *literal same* environment-warmup
workaround (`pnpm install --frozen-lockfile && pnpm -r build`) once per fresh container because
no gate distinguishes "this touches `packages/db/migrations/`" from "this reinstalls exactly what
`pnpm-lock.yaml` already pins" — both currently sit behind the identical hard wall.

**Exact diff.** Split `PROTECTED` into a `HARD_BLOCK` tier (unchanged: migrations, `.github/`,
`.claude/{hooks,commands,agents,settings}`, `fly.toml`, `Dockerfile`, `pnpm-lock.yaml`,
`packages/{db,shared-types}/`, `/contracts/`, `.contract.`, `/.env`) and an `ALLOW_WITH_LOG` tier
(new: bare `package.json` edits that are *pure dependency-version or script-field* changes,
formatter/lint config files, and `**/*.test.*` / `**/__tests__/**` globs) that logs to
`.claude/logs/harness-events.jsonl` (the same sink `guard-bash.sh`/`red-line-doubt-gate.sh`
already write to) and passes through instead of blocking:

```diff
-PROTECTED='(^|/)(migrations|\.github|\.claude)/|(^|/)(fly\.toml|Dockerfile|pnpm-lock\.yaml)$|/package\.json$|packages/shared-types/|packages/db/|/contracts/|\.contract\.|/\.env'
-
-if echo "$REL" | grep -qE "$PROTECTED"; then
-  echo "BLOCKED: '$REL' is in a protected zone (contracts/schema/infra/governance). This is an IMPROVEMENT requiring manual approval." >&2
-  exit 2
-fi
+# HARD_BLOCK: irreversible / red-line surfaces — still exit 2, no override short of a
+# human editing this file. `package.json` itself moved OUT of this tier (see ALLOW_WITH_LOG);
+# `packages/shared-types/` and `packages/db/` still hard-block because a version/script edit
+# there is indistinguishable from a contract/schema edit by path alone.
+HARD_BLOCK='(^|/)(migrations|\.github|\.claude)/|(^|/)(fly\.toml|Dockerfile|pnpm-lock\.yaml)$|packages/shared-types/|packages/db/|/contracts/|\.contract\.|/\.env'
+
+if echo "$REL" | grep -qE "$HARD_BLOCK"; then
+  echo "BLOCKED: '$REL' is in a protected zone (contracts/schema/infra/governance). This is an IMPROVEMENT requiring manual approval." >&2
+  exit 2
+fi
+
+# ALLOW_WITH_LOG: dep/version/formatter/test-glob edits — logged, not blocked. A malicious or
+# careless edit here still shows up in `agent-health-pass.mjs`'s harness-events read and in
+# `git diff`; it just no longer costs a full stop for routine work.
+ALLOW_WITH_LOG='/package\.json$|(^|/)\.(prettierrc|eslintrc)|(^|/)eslint\.config\.|\.test\.[jt]sx?$|/__tests__/'
+if echo "$REL" | grep -qE "$ALLOW_WITH_LOG"; then
+  HEV_LOG="$ROOT/.claude/logs/harness-events.jsonl"
+  mkdir -p "$(dirname "$HEV_LOG")" 2>/dev/null || true
+  printf '{"ts":"%s","hook":"protect-paths","event":"allow-with-log","target":"%s"}\n' \
+    "$(date -Iseconds)" "$(printf '%s' "$REL" | tr '"\\' '..')" >>"$HEV_LOG" 2>/dev/null || true
+fi
```

Mirror the same `HARD_BLOCK`/`ALLOW_WITH_LOG` split in `guard-bash.sh`'s section "2) protect-paths
parity" `PROTECTED` variable (it currently duplicates the flat pattern for the Bash lane).

**Residual risk the operator accepts.** A `package.json` scripts-field edit, a formatter-config
tweak, or a new test file can now land without a manual-approval stop — same trust boundary the
operator already extends to any other non-red-line source edit. The genuinely dangerous
`package.json` case — adding/removing a *dependency* (which mutates `pnpm-lock.yaml`) — is still
caught, because `pnpm-lock.yaml` itself stays in `HARD_BLOCK` and `guard-bash.sh` already
separately blocks `pnpm add`/`pnpm remove`/`npm install <pkg>` regardless of which tier
`package.json` sits in (see its section "5) dependency mutations").

---

## P3 — context-aware gate nudges (reduce false-positive friction on `red-line-doubt-gate.sh`)

**Friction observed.** `red-line-doubt-gate.sh`'s `REDLINE` pattern matches on **path substring
only** — e.g. `(^|/)(price|money|payment|cash|ledger|tax|payout|refund|invoice)` fires on any
file whose path contains "price", including files that are demonstrably not money-logic (a
config constants file named `pricing-copy.ts` holding only UI strings, or a test fixture named
`ledger.fixture.ts`). Every hit demands the same fixed doubt-pass narrative regardless of whether
the file is actual business logic or incidental naming — a `docs/`, `*.test.*`, or `*.md` path
under a red-line substring still gets the full advisory prompt, training the same suppress-the
-prompt reflex a too-loud gate always produces.

**Exact diff.** Add two cheap, structural exclusions ahead of the `REDLINE` match — no new red
-line surfaces slip through (both exclusions are narrower than the sets they exempt from doubt,
not a widening of what counts as red-line):

```diff
 is_redline=0
-printf '%s' "$REL" | grep -Eiq "$REDLINE" && is_redline=1
+# Context-aware nudge: docs/tests/fixtures under a red-line substring still name-match, but
+# carry none of the runtime risk the doubt-pass exists for — skip the prompt, not the gate
+# (a genuine business-logic change to these globs would live outside docs/tests anyway).
+EXCLUDE='\.(md|mdx)$|(^|/)(docs|e2e|__tests__)/|\.(test|spec|fixture)\.[jt]sx?$'
+if ! printf '%s' "$REL" | grep -Eiq "$EXCLUDE"; then
+  printf '%s' "$REL" | grep -Eiq "$REDLINE" && is_redline=1
+fi
 # routine reversible edit → ZERO output, pass clean
 [ "$is_redline" -eq 0 ] && exit 0
```

**Residual risk the operator accepts.** A red-line-named file under `docs/`, `e2e/`,
`__tests__/`, or a `.test./.spec./.fixture.` glob no longer gets the doubt-pass reminder. This is
scoped narrowly on purpose: none of these globs can themselves *be* the production auth/money/RLS
code the gate protects (a `.test.ts` file doesn't ship; `docs/` isn't executed) — at worst this
proposal delays the prompt to whenever the corresponding non-excluded source file is touched,
which still fires normally. `IRREVERSIBLE` (the harder, human-confirm-gated migrations set) is
untouched by this diff.

---

## P4 — agent-init health-check + retry (warm the container once, not once per run)

**Friction observed.** Every recent autonomous-continuation run on this branch has hit and
hand-fixed the *same* cold-container problem — quoting `docs/governance/AUTONOMOUS-STATUS.md`
verbatim, flagged four runs in a row: "this fresh container again had no `node_modules/` and no
`dist/` anywhere — ran `pnpm install --frozen-lockfile` ... then `pnpm -r build` once before
`pnpm -r typecheck`/`lint:gates` would pass. Same fix as the prior N runs; flagging again in case
this is worth a `docs/governance/HARNESS-IMPROVEMENTS.md` proposal... to warm this once per
container instead of once per run." There is currently no `SessionStart` hook registered in
`.claude/settings.json` at all (only `PreToolUse`/`PostToolUse`/`UserPromptSubmit`/`Stop`) — the
warmup is discovered fresh by whichever agent happens to run first, spending several minutes of
every session on a deterministic, idempotent step.

**Exact diff.** New `.claude/hooks/agent-init-warmup.sh`:

```bash
#!/usr/bin/env bash
# agent-init-warmup.sh — SessionStart hook. Idempotent container warmup: if node_modules/dist
# are already present (warm container, or a prior session already ran this), this is a fast no-op.
# Advisory/best-effort only — never blocks session start (exit 0 always); a failed warmup here
# just means the first real command re-discovers and fixes it manually, same as today.
set -uo pipefail
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "${CLAUDE_PROJECT_DIR:-$PWD}")"
cd "$ROOT" || exit 0

MARKER="$ROOT/.claude/state/warmup-done"
mkdir -p "$(dirname "$MARKER")" 2>/dev/null || true

if [ -d node_modules ] && [ -f "$MARKER" ]; then
  exit 0  # already warm this container, already recorded — nothing to do
fi

RETRIES=2
attempt=1
while [ "$attempt" -le "$RETRIES" ]; do
  if pnpm install --frozen-lockfile >/dev/null 2>&1 && pnpm -r build >/dev/null 2>&1; then
    date -Iseconds > "$MARKER"
    exit 0
  fi
  attempt=$((attempt + 1))
done
# Both attempts failed — leave no marker so the next session/step retries; the agent's own
# first real command will hit the same failure and surface it normally (route-around-environment
# rule in CLAUDE.md already covers this: don't block, report, use alternatives).
exit 0
```

`.claude/settings.json` — register it under a new `SessionStart` array:

```diff
   "hooks": {
+   "SessionStart": [
+    {
+     "hooks": [
+      {
+       "type": "command",
+       "command": "bash \"$CLAUDE_PROJECT_DIR/.claude/hooks/agent-init-warmup.sh\""
+      }
+     ]
+    }
+   ],
    "PreToolUse": [
```

**Residual risk the operator accepts.** The first session in a fresh container now spends its
warmup time at session start (background-ish, before the agent's first real command) instead of
mid-task at the first `pnpm typecheck`/`build` call — net time spent is the same, but it moves
earlier and is no longer narrated inline in every run's proof section. Fully backward compatible:
a warm container (marker present, `node_modules/` present) makes this an early `exit 0`, and any
failure mode falls through to exactly today's behavior (the agent's own first build/typecheck
call rediscovers and fixes it).

---

## P5 — research-lane token budgets

**Friction observed.** `docs/design/harness/SYSTEMS-MAP.md`'s Harness-Improvements-proposals row
names this as one of the friction classes worth a proposal, but there is currently no
`research-lane`-labeled subsystem, script, or config anywhere in the repo (`grep` across
`.claude/`, `scripts/`, `docs/design/harness/`, `docs/governance/` for "research-lane" / "token
budget" returns only that one SYSTEMS-MAP.md mention). The closest existing mechanism is the
`Workflow` tool's own `budget: {total, spent(), remaining()}` object (a caller-supplied `+500k`
-style directive per invocation) and per-skill research tools (`firecrawl-deep-research`,
`deep-research`, `last30days`) that have no repo-side cap of their own — an unbounded research
fan-out inside this repo's harness has no local backstop distinct from whatever budget the
calling session happens to pass in.

**Exact diff.** This is a **design gap, not a code diff** — there is no existing research-lane
mechanism to tier, so nothing here is a diff against a real file. Proposed spec for the operator
to scope before any code is written:

- A repo-local default ceiling (e.g. an env var `RESEARCH_LANE_TOKEN_BUDGET`, read by whichever
  script eventually orchestrates a `deep-research`/`firecrawl-deep-research` run from inside this
  repo's harness — none currently does) so a research-shaped task launched *without* an explicit
  `+Nk` directive from the operator still gets a sane default instead of running unbounded.
- This should compose with, not duplicate, the `Workflow` tool's own `budget.total`/`remaining()`
  — the repo-local default should only apply when the caller supplied no explicit budget,
  matching the existing `budget.total ? ... : ...` guard pattern already documented for
  loop-until-budget workflows.

**Residual risk / why this is a design flag, not a diff.** Writing an enforcement point without
a concrete integration site (which script, which trigger) risks inventing a mechanism nobody
calls — worse than the current gap, because it would look done. This item stays a named,
scoped gap in `SYSTEMS-MAP.md` (already 🔴 PLANNED there) until a real research-orchestration
call site exists in this repo to attach a budget to.

---

## Summary

| # | Proposal | File(s) | Diff status |
|---|----------|---------|--------------|
| P1 | Docker build: pre-commit → CI | `.husky/pre-commit`, `.github/workflows/ci.yml` | exact diff above |
| P2 | Tier protect-paths: hard-block vs allow-with-log | `.claude/hooks/protect-paths.sh`, `.claude/hooks/guard-bash.sh` | exact diff above |
| P3 | Context-aware red-line-doubt-gate nudges | `.claude/hooks/red-line-doubt-gate.sh` | exact diff above |
| P4 | Agent-init warmup + retry (SessionStart) | `.claude/hooks/agent-init-warmup.sh` (new), `.claude/settings.json` | exact diff above |
| P5 | Research-lane token budgets | *(no existing call site)* | design spec only — not a diff |

None of these have been applied. Applying P1/P4 requires touching `.github/workflows/` and
`.claude/settings.json` respectively — both protected zones — so even the operator should re-run
`node scripts/guardrail-hook-matchers.mjs` and `node scripts/guardrail-gate-armament.mjs` after
hand-applying P2–P4, and re-verify `pnpm lint:gates` end-to-end before committing.
