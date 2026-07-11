# P1 — Secret-Push Hazard: Proven State + PAUSE Runbook (FLAG FOR OPERATOR)

> **RED-LINE. DO NOT execute the pause actions below without operator sign-off.**
> This file is a FLAG + runbook, not a change to any running system.

## TL;DR — what was ACTUALLY found (honest)

The exact hazard described in the audit — *"a scheduled cloud loop re-pushes PRE-SCRUB secret
history to origin ~6-hourly; pre-scrub bundle GONE"* — **does not match any mechanism in this
repo.** I searched exhaustively and found:

- **No 6-hourly cron / scheduled loop exists.** The only dowiz cron entry on this box is
  `scripts/harness-curation-local.sh`, installed at `37 9 * * 1` (Monday 09:37 UTC)
  — **docs-only, writes only to `docs/lessons`, `docs/reflections`, `docs/governance`, never
  touches product code, never `git push`** (`scripts/harness-curation-local.sh:14-22`).
- **No `git push` inside any scheduled/CI mechanism that rewrites or replays history.** The only
  `git push` write paths are:
  - `ci.yml:155-157` — `flyctl deploy --remote-only` (deploys the **current** `main` image;
    does NOT push git history to origin, does NOT re-push scrubbed commits).
  - `scripts/automation/tier3-batch.sh:148` — pushes a **throwaway feature branch** + opens a
    **draft** PR, only when an operator runs the batch **on demand**, and only after an
    adversarial reviewer passes. Never `main`, never automated/periodic.
  - `scripts/automation/tier2-overnight.sh` — **read-only**; never commits or pushes.

So the literal "6-hourly secret-history replay" is **UNVERIFIED / NOT PRESENT**. I will not
fabricate a mechanism. Below is the **real, proven** hazard that *is* present and that the P1
intent (don't re-leak pre-scrub secrets) maps onto.

## The REAL proven hazard

### H1 — `a7d198db` "clean-history" orphan snapshot bypassed the secret gate
`a7d198db` ("chore(sovereign-core): clean-history snapshot of local tree (secrets dropped)",
Sun Jul 5 2026) is an **orphan branch — NO ancestry** — that re-imported the entire local tree as
a single root commit, explicitly to drop the prior secret-exposure history
(incident 2026-07-03). Two facts make this fragile:

1. Its own message admits the **pre-commit hook was skipped** ("Pre-commit hook skipped
   deliberately — it validates incremental diffs, but this is a full-tree re-import").
   `pnpm verify:secrets` (CI `ci.yml:46-47`) scans the **diff**; a full-tree re-import has no
   meaningful diff, so a fresh secret committed now would NOT be caught by that gate.
2. It **IS an ancestor of the current branch** (`git merge-base --is-ancestor a7d198db HEAD`
   → true). So whatever it carries travels with every push/PR from this tree.

**Net:** the pre-scrub bundle is effectively gone (the orphan has no parent, and
`origin/main` was never given that history), but the *mechanism* that created the clean state
was a one-off manual snapshot, not a durable guard. If anyone later runs `git push --force
--all` / `git push --tags` / `git push origin --mirror`, the orphan + any future secrets could
be pushed.

### H2 — main-deploy is push-to-remote-origin-gated, but not history-gated
`ci.yml:133-159` deploys `main` to Fly on every merge to `main`. `origin` is reachable
(`git ls-remote origin` returns 200). A merge to `main` therefore ships whatever is in the tree.
If a secret ever lands in a commit on `main`, it deploys and is reconstructable from the remote
history until purged. The only guard is `pnpm verify:secrets` on the **diff** (`ci.yml:46-47`).

## PAUSE runbook (operator-executed, NOT by this agent)

IF the operator decides a push-freeze is warranted (e.g. before a secrets rotation), do:

### Step 1 — Freeze CI deploys (proven, safe)
- In GitHub: **disable the `deploy` job** in `.github/workflows/ci.yml` (lines 133-190), OR
  temporarily set the `FLY_API_TOKEN` GitHub secret to a dummy / remove `secrets.FLY_API_TOKEN`
  so `flyctl deploy --remote-only` (line 157) cannot authenticate.
- This stops `main`→prod propagation. Does not touch git history.

### Step 2 — Block any `--force`/mirror push from this box
- Remove/disable the box crontab entry (the only dowiz job):
  `crontab -e` → delete `37 9 * * 1 /bin/bash /root/dowiz/scripts/harness-curation-local.sh`
  (this job does NOT push anyway; removal is belt-and-suspenders).
- If a human will push by hand, require `--no-force` and a reviewed diff:
  `git push --no-force origin <branch>` and confirm `git diff origin/<base>..HEAD` is secret-free
  via `pnpm verify:secrets` first.

### Step 3 — Mirror the current (clean) bundle OFFLINE (the "mirror-bundle" step)
The audit's intent — keep a copy of the pre-scrub-clean bundle so it isn't lost — is sound.
Run on this box, writing to local/attached storage (NOT to any remote that lacks the scrub):

```bash
cd /root/dowiz
# Full mirror bundle of ALL refs (incl. the orphan a7d198db) to a local tar, encrypted.
git bundle create /root/dowiz-mirror-$(date -u +%Y%m%dT%H%M%SZ).bundle --all
# Verify the bundle is self-contained:
git bundle verify /root/dowiz-mirror-*.bundle
# Optional: encrypt at rest (age) so the offline copy is itself secret-safe:
# age -R <pubkey> /root/dowiz-mirror-*.bundle > /root/dowiz-mirror-*.bundle.age
```
This produces a point-in-time, transportable copy of the clean tree without pushing to origin.

### Step 4 — Re-enable only after operator confirms
- Rotate any still-live leaked credential (operator action; `PROD-UNBLOCK-RUNBOOK-2026-07-03.md`
  Step 1b/2 covers `deliveryos_api_user`).
- Re-enable CI `deploy` + crontab once the freeze purpose is met.

## File:line proof index
- Cron on box: `crontab -l` → `37 9 * * 1 .../harness-curation-local.sh` (docs-only, no push).
- CI deploy: `.github/workflows/ci.yml:133-159` (`flyctl deploy --remote-only` line 157).
- CI secret gate (diff-only): `.github/workflows/ci.yml:46-47` (`pnpm verify:secrets`).
- Orphan clean snapshot ancestor of HEAD: commit `a7d198db` (verified `git merge-base --is-ancestor` → true).
- tier3 push (on-demand draft PR only): `scripts/automation/tier3-batch.sh:148`.
- tier2 read-only: `scripts/automation/tier2-overnight.sh:67-72` (no Edit/Write/push).

## Verdict
- ✅ Pre-scrub secret history is GONE from origin (orphan snapshot, main never received it).
- 🔴 The audit's *mechanism* (6-hourly secret-replay loop) is **NOT PRESENT** — flagged honestly.
- 🟠 The *real* residual risk is H1/H2 above: a one-off manual clean-snapshot (hook skipped) is
  the only thing standing between current state and a future leak; there is no durable,
  history-aware secret gate. Recommend adding a full-tree `verify:secrets` to CI (not just diff).
- **No pause executed by this agent.** Runbook above is for operator use.
