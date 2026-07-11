# G02 — Remote History Scrub, Secrets CI Gate, and the Rewrite-Aware Merge-to-Main

> Research + execution blueprint, 2026-07-11. Read-only session: nothing in the repo, remote, or
> working tree was modified; this file is the only artifact created.
> **SAFETY RULE honored throughout: no secret value appears in this document.** Secrets are referred
> to only by commit hash, branch, file path, or memory-file name.
> Sources: audit `docs/research/2026-07-11-full-project-audit-dowiz-bebop.md` (§1 risks 3+6, §6.3
> item 3, §7.4, §9 recs 3+4, App. A), memory corpus (`secrets-exposure-incident-2026-07-03.md`,
> `open-source-goal-adr020-2026-07-03.md`, `merge-to-main-plan-2026-07-02.md`), live git/`gh`
> evidence gathered in this session (all read-only).

---

## 1) Gap & evidence

**G02 = three coupled debts** blocking ADR-020 (open-source flip) and the eventual prod merge:

**(a) Origin still hosts secret-bearing history.** The 2026-07-05 scrub
(`git-filter-repo --replace-text`, per `secrets-exposure-incident-2026-07-03.md` §REMEDIATED)
rewrote **local** history only. Verified live in this session:

- `git ls-remote --heads origin` → **35 branches** on GitHub (`SyniakSviatoslav/dowiz`, PRIVATE,
  0 forks, default branch `main`, `main` NOT protected — all verified via `gh api`).
- The 4 secret-introducing commits named in the incident memory (`72fde8a6`, `bccfd324`,
  `d5eef9cb`, `84b95d66`) are **all ancestors of `origin/main` (`c8b2d5a0`)** — verified with
  `git merge-base --is-ancestor`. Every remote branch that descends from that lineage carries them.
- Credentials are rotated dead (operator, 07-05) → no live exposure; but ADR-020 hard gate #1
  ("secrets git-HISTORY scrub… over ALL history") is open until origin is clean.

**(b) History is trifurcated, not bifurcated.** Verified geometry (this session):

| Lineage | Root/tip | Where it lives |
|---|---|---|
| **L1 pre-scrub** (secret-bearing) | tip `c8b2d5a0` (origin/main, 07-03) | origin: 32 of 35 branches; locally only via remote-tracking refs |
| **L2 filter-repo scrubbed** (07-05 10:14) | e.g. `e4434515` = scrubbed twin of `c8b2d5a0` | local branches: `main` (`2be9e692`), `fix/audit-remediation` (`a8e5844e`), `integrate/merge-to-main`, 26 `worktree-agent-*`, etc. **Not on origin at all.** |
| **L3 fresh-root snapshot** (07-05 19:15) | root `a7d198db` "chore(sovereign-core): clean-history snapshot of local tree (secrets dropped)" | local `feat/sovereign-core-phase-zero` (100 commits), `feat/paleo-dinosaur-digs` (104 commits); on origin: `feat/sovereign-core-phase-zero` (`330ff4ed`), `backup-wip-2026-07-08` (`77811204`) |

Crucial correction to the audit's §7.4 framing: **L3 (paleo) shares no commit with L2 either** —
`git merge-base e4434515 feat/paleo-dinosaur-digs` = none; `rev-list --left-right --count` =
856 / 104. Paleo is not "post-scrub main lineage"; it is a deliberate orphan snapshot. The
"local vs origin fix/audit-remediation diverged 997/941" figure is L2-vs-L1 divergence (verified:
`fix/audit-remediation...origin/fix/audit-remediation` = 997/941). So the merge-to-main is a
**cross-root operation whichever pair you pick** — hence "rewrite-aware" (§4 Phase 3).

**(c) `pnpm verify:secrets` is a false-green.** Reproduced live in this session:
`pnpm verify:secrets` prints `⚠ gitleaks not installed, skipping` (skip path at
`scripts/verify-secrets.ts:22-23`) and **exits 0 with all other checks green**. `which gitleaks`
is empty. CI runs it twice (`.github/workflows/ci.yml:47` standalone + inside
`pnpm verify:all --ci` at line 41 via `scripts/verify-all.ts:21`), both silently green. Flagged
since 07-08 in `docs/design/dowiz-brand/EXPANSION-PLAN.md` §"Security scanners" as
"🔴 FIX FIRST… highest value, zero risk". A `.gitleaksignore` exists (2 annotated entries,
Jun 14); **no `.gitleaks.toml`** in the repo.

---

## 2) Research findings

### 2.1 Full remote-branch classification (all 35, verified)

Method: remote tip SHA looked up in `.git/filter-repo/commit-map` (1,014-commit old→new map,
written 07-05 10:14 — it survives, which makes the whole operation deterministic), local object
probes, ancestry checks, and `gh api compare` for tips that don't exist locally.

**Clean already (3) — no scrub action needed:**
- `feat/sovereign-core-phase-zero` (`330ff4ed`, L3; ancestor of local tip `28cf82eb`, which is 6 ahead)
- `backup-wip-2026-07-08` (`77811204`, L3; ancestor of local paleo)
- `telemetry/plane` (`bd6c5049`) — **orphan lineage** (GitHub: "no common ancestor with main");
  filter-repo left it byte-identical (ref-map old==new `7d937078`), i.e. no secrets found in it
  on 07-05; remote has extra telemetry-publish commits since. Verify with gitleaks in Phase 2,
  keep as-is.

**Pre-scrub L1, MERGED into old main → dead, DELETE (13):**
`backup/pre-deep-check-2026-06-18`, `chore/agentic-tooling-integration`, `feat/agentic-system`,
`feat/golive-remediation`, `feat/mvp-sensor-seams`, `feat/plane-telemetry-closed-loop`,
`feat/product-media-seam`, `feat/v1-hardening`, `fix/design-system-consistency`,
`fix/plane-telemetry-send-events`, `fix/ui-translations`, `infra`, `integrate/merge-to-main`
(each remote tip verified `merge-base --is-ancestor` → `origin/main`; each also has a local L2
scrubbed twin via commit-map/ref-map).

**Pre-scrub L1, historical, unmerged, local L2 twin exists → DELETE from origin (6):**
`chore/design-system-prune`, `chore/design-system-prune-t2` (both → twin `651d5197`, local),
`docs/plane-status-2026-07-02` (→ `7f8618c9`), `fix/plane-report-capture-json-corruption`
(→ `d2079ea5`), `fix/plane-telemetry-publish-data-loss` (→ `f97488bb`),
`governance/plane-maintainer-2026-07-04` (→ `9e1221a0`). Nothing is lost: twins are local branches.

**Pre-scrub L1, created in the CLOUD after 07-05 → no local twin exists (11):**
`chore/harness-curation-2026-07-06`, `chore/persist-dep-baseline`,
`chore/plane-maintainer-2026-07-07`, `chore/plane-telemetry-resolve-hydrate`,
`docs/plane-status-2026-07-05`, `fix/plane-telemetry-publish-drops-predictions`,
`plane-maintainer/2026-07-06-digest`, `plane-maintainer/2026-07-08-daily-run`,
`plane-maintainer/2026-07-10`, `plane-maintainer/channel-liveness-guard-20260711`,
`dependabot/npm_and_yarn/esbuild-0.28.1`. All verified via `gh api compare`: **1–4 commits ahead
of old main, 0 behind, merge base `c8b2d5a0`** → they carry the full secret-bearing history. All
are docs/governance digests or trivial fixes; each backs an **open PR** (see 2.4). DELETE +
optionally cherry-pick their few commits onto the new main later (cherry-pick is content-based,
safe across rewrites; everything is preserved in the Phase-2 bundle regardless).

**The two load-bearing replacements (2):**
- `main` (`c8b2d5a0`) → scrubbed twin **`e4434515`** exists locally (tip of the `worktree-agent-*`
  branches). Verified: `git diff c8b2d5a0 e4434515` touches **exactly the 12 known secret-bearing
  files** (`.agents/tmp/check-jobs.mjs`, `.agents/tmp/check-session.mjs`,
  `.agents/tmp/test-connections.cjs`, `apps/api/fix-db.js`,
  `docs/ops/PROD-UNBLOCK-RUNBOOK-2026-07-03.md`, `packages/db/scripts/check-job-details.ts`,
  `check-job-schema.ts`, `check-notify-jobs2.ts`, `check-schemas.ts`, `test-connections.js`,
  `packages/platform/test-notify2.cjs`, `test-notify3.cjs`) — REDACTED markers confirmed present
  in the scrubbed blobs. Same tree otherwise ⇒ force-pushing it is deploy-equivalent.
- `fix/audit-remediation` (remote `914acf8a`) → local L2 `fix/audit-remediation` (`a8e5844e`).
  The remote branch contains S1–S10 + the GDPR trio in old hashes; the local branch contains their
  scrubbed twins — verified via commit-map: `5ded9f19→3bf42379`, `58caf4f4→ded7cf9d`,
  `d6b3473e→69359a08`, S10 `2ebdf513→033c867b`, all four `--is-ancestor` of local
  `fix/audit-remediation`. **Delta at risk:** ~8 cloud "re-verify audit-remediation backlog" doc
  commits (07-09 → 07-11) exist only on the remote (salvageable by cherry-pick from the bundle).

### 2.2 ⚠ An ACTIVE writer is pushing pre-scrub lineage to origin *today*

`origin/fix/audit-remediation` last push **2026-07-11 12:07:30Z** (author `Claude
<noreply@anthropic.com>`, login `claude`): a scheduled cloud loop committing "re-verify
audit-remediation backlog #N — still complete, no drift" at ~00:07 / 06:07 / 12:07 UTC (≈ every
6h). A second scheduled agent (plane-maintainer) creates a **new L1-based branch + PR daily at
~06:15Z** (evidence: PR cadence #12–#23). Neither is a GitHub Actions workflow (`.github/workflows/`
has only `ci.yml`, `openwiki-update.yml` (cron 04:37, wiki only), `skill-security.yml`,
`visual.yml`) — they are external scheduled Claude cloud sessions. **Any scrub executed without
pausing these loops will be re-dirtied within hours**, and a stale cloud clone doing a normal push
after the scrub would *recreate* deleted L1 branches. The freeze window (Phase 2) is therefore not
optional ceremony — it is the difference between a scrub and a Sisyphus loop.

### 2.3 Backups — what actually exists (verified on disk)

| Artifact | Date | Contents (verified) | Verdict |
|---|---|---|---|
| `/root/dowiz-backup-2026-07-08.bundle` | Jul 8 21:33, 35.8 MB | `git bundle list-heads` → **exactly 1 ref**: `refs/heads/backup-wip-2026-07-08` (L3 WIP) | **NOT a full backup** — single-branch |
| `/root/restore-dowiz-2026-07-08.tar.zst` | Jul 8 21:36, 52.6 MB | working-tree snapshot under `restore/dowiz/` | tree-only, no refs |
| `/root/dowiz.tar.gz` | **Jun 17** 15:14, 4.9 MB | pre-incident tree snapshot (`dowiz/.agents/`, `.auth/`, …) | stale; likely contains pre-rotation secrets in-tree — treat as sensitive, keep offline |
| `scratchpad/dowiz-backup-pre-scrub.bundle` (named in incident memory) | — | **MISSING** — scratchpad was ephemeral; not found on disk | the pre-scrub full backup no longer exists locally |

⇒ **Today, the only complete copy of L1 pre-scrub history is origin itself** (plus partial local
remote-tracking refs for 4 branches). Phase 2 MUST take a full `--mirror` bundle before any push.
Note the inversion: that bundle *is itself a secret-bearing artifact* — `chmod 600`, offline, with
an operator retention decision (§6).

### 2.4 GitHub surface (verified via `gh`)

- **15 open PRs** (#6, #8, #10–#18, #20–#23), **all base=main**, all heads = L1 cloud/governance
  branches above. Deleting a head branch auto-closes its PR; closed PRs keep `refs/pull/N/head`
  server-side forever (user cannot delete those refs).
- **`main` is unprotected** (`protected:false`) — force-push is technically possible today; also
  means nothing prevents an accidental push→prod.
- **CI deploys prod on any push to main**: `ci.yml` `on.push.branches: [main]`; `deploy` job at
  line 133–135 gated only by `if: github.ref == 'refs/heads/main'`. **A force-push of main IS a
  prod deploy trigger.** CI checkout is default depth (shallow) — a full-history gitleaks scan in
  CI would need `fetch-depth: 0` and would go RED against pre-scrub origin (sequencing matters,
  Phase 1).
- Open issues: **#9** (guardrail scripts missing from main → `verify:all` red on main — interacts
  with Phase 3 slice content), **#19** (cloud sandbox egress — the scheduled agents are already
  degraded; pausing them for the freeze is cheap).
- 0 forks, repo private → no fork-network contamination; the classic "purge fork network" problem
  does not exist here. Residual server-side risk is only dangling objects + `refs/pull/*`.

### 2.5 Scrub tooling actually used on 07-05 (verified)

`git-filter-repo --replace-text` (incident memory §REMEDIATED; corroborated by
`.git/filter-repo/{commit-map,ref-map,changed-refs,first-changed-commits}` timestamped
2026-07-05 10:14). 1,014 commits mapped; 5 literal passwords + a host-URL regex →
`REDACTED`-style replacements (marker confirmed in scrubbed blobs, e.g.
`git show e4434515:apps/api/fix-db.js`). HEAD lineage `876fd5a3→9c6644e9`. No tags exist (local or
remote — verified), no `refs/replace`. **The commit-map is the rosetta stone for Phase 2 — do not
lose `.git/filter-repo/`** (it maps every L1 remote tip to its L2 twin).

### 2.6 verify-secrets.ts — deeper defects than "not installed" (all at `scripts/verify-secrets.ts`)

1. **Skip-not-fail** (lines 21–24): missing binary ⇒ warn + continue ⇒ CI false-green. In CI this
   must be a hard failure.
2. **Bogus flag** (line 26): `gitleaks detect --source . --verbose -i "."` — `-i` is not a valid
   gitleaks v8 flag; the moment gitleaks IS installed, this line will error and (correctly but
   confusingly) fail the check for the wrong reason.
3. **Label lies about scope** (line 31): "no secrets in working tree" — but `gitleaks detect`
   scans **git history** of the checkout by default (worktree scanning is `--no-git`/`gitleaks dir`).
4. `execSync` 30s timeout + default 1 MB `maxBuffer` will not survive a 2,000-commit history scan
   with `--verbose`.
5. `findGitleaks()` (lines 99–124) probes mostly Windows paths (`where.exe`, WinGet, USERPROFILE)
   — written on another box; harmless but dead weight on Linux CI.

### 2.7 G01 interplay (GDPR trio → prod) — ordering analysis

Facts: the GDPR fixes exist in **three forms** — L1 originals on `origin/fix/audit-remediation`
(`5ded9f19`, `58caf4f4`, `d6b3473e`), L2 twins reachable from local `fix/audit-remediation`
(`3bf42379`, `ded7cf9d`, `69359a08`), and as **content already inside paleo's tree** (verified:
`delivery_photo_key` present in paleo's anonymizer). `origin/main` has none of it.

**Recommendation: G01 lands BEFORE the force-push.** Rationale:
1. **Harm asymmetry**: GDPR erasure incompleteness is live legal exposure on real users (audit
   risk #1, rec #1, ETHICAL-STOP class); the dirty remote is latent hygiene — creds dead since
   07-05, repo private, 0 forks. The urgent thing ships first.
2. **The scrub must cover the final pre-OSS state anyway**: every push of old-lineage content
   re-dirties origin, so the force-push belongs *after the last L1 writer* — which is G01's
   cherry-pick plus the paused cloud loops — and as close to the ADR-020 flip as practical.
3. **Blast-radius isolation**: G01 is a small cherry-pick + monitored prod deploy on a known
   lineage. Doing it simultaneously with a 35-branch force-push wave would stack two risky
   operations on the same deploy pipeline in one window.
4. **Determinism is preserved**: after G01 lands (new origin/main tip M1), the scrubbed main
   candidate is mechanically `e4434515 + cherry-pick of the L2 twins` (general rule:
   `e4434515 + cherry-pick(c8b2d5a0..M1)` using commit-map twins where they exist), with a
   tree-diff proof (Phase 2 step 3) that catches any divergence.

The only argument for scrub-first — "merge geometry gets simpler" — is false: G01 via cherry-pick
onto origin/main works identically on L1, and Phase 2 explicitly re-bases the scrubbed candidate on
whatever main's final tip is.

---

## 3) Options & tradeoffs

### 3.1 For the remote scrub (Phase 2 vehicle)

| Option | Mechanics | Pros | Cons |
|---|---|---|---|
| **A. In-place force-push + prune** (recommended) | `push --force-with-lease` the 2 replacements, `push --delete` the 30 dead/L1 branches, keep the 3 clean | Keeps repo identity (issues, PR history, CI secrets, Fly wiring, remote URL); matches incident-memory + EXPANSION-PLAN §OSS step (3) | Dangling L1 objects + `refs/pull/*` remain fetchable **by SHA** on GitHub until Support-side gc → needs a GitHub Support ticket before the PUBLIC flip |
| **B. Fresh-repo swap** | rename `dowiz`→`dowiz-archive-private`; create new `dowiz`; push only clean refs; re-add Actions secrets (`DATABASE_URL_MIGRATIONS/SESSION`, `FLY_API_TOKEN`, `DEV_AUTH_SECRET`, …) | **Zero** server-side residue by construction — strongest possible ADR-020 posture; trivially verifiable | Loses issues #9/#19 + PR history (small); must re-wire CI secrets + any webhooks/schedules; renamed-repo redirects die when the name is reused (intended here) |
| **C. Delete-repo + recreate** | as B but destroys the archive too | nothing over B | destroys the private history archive; strictly worse |

A is the default; **B is genuinely competitive here** because the repo is private with 0 forks and
only 2 open issues — the classic costs of a swap are near zero, and it deletes the entire
"dangling objects on GitHub" risk class that otherwise requires trusting a Support gc. Operator
decision D3 (§6).

### 3.2 For the merge-to-main (Phase 3)

| Option | Mechanics | Pros | Cons |
|---|---|---|---|
| **M1. Adopt L3 as main** | force-push paleo → main | Preserves paleo's 104-commit history as *the* history; one step | Push = **prod deploy of the entire staging superset** (rebuild, sovereign, flags) — unvalidated (audit risk 4: staging cutover state unknown); severs main from 2,000-commit history; second main force-push; CI on paleo tree is red without the uncommitted plane-guard fixes (audit §6.1) |
| **M2. Curated tree-squash slices** (recommended) | build commits ON TOP of scrubbed main whose **tree** equals validated slices of paleo's tree (`git commit-tree paleo^{tree} -p main` for the final slice; earlier slices = subsystem checkouts); normal (non-force) pushes | main lineage stays continuous; **each slice = one reviewable, stageable, deployable prod change**; the 500+ add/add conflict problem (HANDOFF-2026-07-07) vanishes — there is no merge, only tree adoption; matches audit rec #1 "merge by tree, not by hash" | Flattens per-commit history into slices (mitigant: paleo branch is kept forever as the detailed record); slice curation is real work |
| **M3. Graft-and-bake** | `git replace --graft a7d198db <scrubbed-main-tip>` + `filter-repo` to bake → replay L3 atop L2 | one unified full history; normal merge thereafter | a **third** hash rewrite — invalidates origin/sovereign+backup-wip (another force-push wave), every commit hash in memory/docs/h_t frames pointing at L3, and any local clone; high cost for cosmetic benefit |

**Recommend M2**, with slice #1 = the harness/guardrail scripts that close GH #9 (so main's CI is
green before anything else), and money/GDPR/kernel slices individually operator-gated because each
main push deploys prod. M1 remains available later as an "endgame" once the full paleo tree has
been staging-revalidated — but it is not designable as safe today.

### 3.3 For the CI gate (Phase 1 scope)

Per EXPANSION-PLAN research (already council-distilled in-repo): raw pinned gitleaks binary, **not**
`gitleaks-action` (paid license for org private repos). Two scan modes with different sequencing:
worktree+staged scanning is safe to enforce **immediately**; full-history scanning (`--log-opts
"--all --full-history"`, `fetch-depth: 0`) goes RED against pre-scrub origin **by design** — it
becomes the post-Phase-2 release gate, not a day-1 CI default.

---

## 4) Recommended execution blueprint (phased)

Legend: 🔒 **OPERATOR-GATED** (irreversible / red-line — never autonomous; per
`never-bypass-human-gates` memory) · ⚙ agent-executable · **VbM** = falsifiable proof with its RED
case stated · Effort: S ≤ 2h · M ≤ 1 day · L > 1 day.

### Phase 1 — gitleaks install + CI gate (independent of everything; do first)

| # | Action | Gate | VbM proof (GREEN / RED) | Effort |
|---|---|---|---|---|
| 1.1 | ⚙ Install gitleaks: download the pinned v8.x linux-x64 release tarball, verify its published SHA-256 checksum, place at `/usr/local/bin/gitleaks`. (An install is a change to the box, not the repo — if even that is out-of-policy for agents, it is a 5-minute operator step.) | ⚙ (box) | `gitleaks version` prints the pinned version. RED: checksum mismatch ⇒ abort install. | S |
| 1.2 | ⚙ Fix `scripts/verify-secrets.ts`: (i) lines 21–24 — when `process.env.CI` is set, missing gitleaks ⇒ `check(false)` **hard fail**, not skip; (ii) line 26 — replace the invalid `-i "."` invocation with two explicit calls: `gitleaks dir .` (worktree, matches the existing label) and `gitleaks git --log-opts="-n 200"` (recent history), both with `--exit-code 1`, `maxBuffer` ≥ 16 MB, timeout ≥ 120s; (iii) drop the Windows-only path probes. | ⚙ (repo edit, normal PR) | Unset PATH copy of gitleaks + `CI=1 pnpm verify:secrets` ⇒ **exit 1** (this is the RED that today wrongly exits 0 — reproduced in this session). GREEN: with binary installed, exits 0 on a clean tree. | S |
| 1.3 | ⚙ CI: add an "Install gitleaks (pinned, checksum-verified)" step in `ci.yml` before line 41 (`verify:all --ci`), so both the standalone step (line 47) and the verify-all inner step get a real binary. Keep default shallow checkout for now (full-history CI scan would be RED against pre-scrub origin — see 3.3; that flip is step 2.9). | ⚙ | CI run on a throwaway branch shows the gitleaks step green and `verify:secrets` reporting a real scan (not "skipping"). RED: delete the install step in a scratch branch ⇒ `verify:secrets` fails (proves 1.2's hard-fail is live in CI, not just locally). | S |
| 1.4 | ⚙ **Canary RED proof** (the falsifiable heart of this phase): in a scratch worktree, write `scratch/canary.txt` containing a synthetic GitHub-PAT-shaped token — `ghp_` + 36 random alphanumerics generated at test time (never a real credential; random so no entropy-rule false-negative). Run `pnpm verify:secrets` ⇒ **must exit 1** naming the file. Delete the canary ⇒ exit 0. Record both outputs in the PR description. | ⚙ | Stated above — this IS the RED/GREEN pair. If the canary does NOT fail the scan, the gate is a false-green and MUST NOT be declared done. | S |
| 1.5 | ⚙ Optional hardening (recommended, cheap): add `gitleaks protect --staged` to `.husky/pre-commit` (currently absent — verified), and require a `# reason+reviewer` comment on any new `.gitleaksignore` line (guardrail script). | ⚙ | Stage the 1.4 canary ⇒ commit is rejected. RED = canary commits successfully. | S |

**Exit criterion:** CI can no longer be green without a real secrets scan. (This also becomes the
standing tripwire that would catch any post-scrub resurrection of secret-bearing blobs in new
commits — see 2.2 risk.)

### Phase 2 — remote scrub: freeze → backup → force-push/prune → verify

**Precondition: G01's GDPR trio is on origin/main and deployed** (see §2.7; if the operator
reverses that ordering decision — D1 — Phase 2 runs identically, just without the twin
cherry-picks in 2.3).

| # | Action | Gate | VbM proof (GREEN / RED) | Effort |
|---|---|---|---|---|
| 2.1 | 🔒 **Freeze window opens.** Operator pauses ALL scheduled cloud writers: the ~6-hourly "re-verify audit-remediation backlog" loop (next runs ≈ 00:07/06:07/12:07/18:07 UTC), the daily ~06:15Z plane-maintainer run, Dependabot, and any other Claude routine that pushes to this repo. Also disable GitHub Actions for the window (`gh api -X PUT repos/SyniakSviatoslav/dowiz/actions/permissions -f enabled=false`) so no push in this phase triggers a surprise prod deploy. No human pushes either. | 🔒 | Two `git ls-remote --heads origin` snapshots ≥ 60 min apart are byte-identical. RED: any hash moved ⇒ a writer is still live ⇒ do not proceed. | S (+wait) |
| 2.2 | ⚙ **Final full backup** (the 07-08 bundle is single-ref — §2.3 — so this is mandatory): `git clone --mirror git@github.com:SyniakSviatoslav/dowiz.git /root/dowiz-origin-mirror-2026-07-XX` then `git -C <mirror> bundle create /root/dowiz-origin-pre-forcepush-2026-07-XX.bundle --all`; `chmod 600` both. **This bundle contains the secret-bearing history — offline only, never committed, retention = operator decision D5.** | ⚙ | `git bundle verify` OK **and** `git bundle list-heads | wc -l` == 35 == current `ls-remote` count, hashes matching the 2.1 snapshot. RED: count/hash mismatch ⇒ re-clone. | S |
| 2.3 | ⚙ Build the two replacement refs in a scratch clone (never in the live `/root/dowiz` working tree): (a) `main-scrubbed` := `e4434515` + `cherry-pick` of the L2 twins of everything G01 landed (`3bf42379`, `ded7cf9d`, `69359a08` +fixups; general rule `e4434515 + twins(c8b2d5a0..M1)` via `.git/filter-repo/commit-map`); (b) `fix/audit-remediation` := local L2 tip `a8e5844e` (optionally + cherry-picks of the 8 remote-only "re-verify" doc stamps from the mirror — decision D4). | ⚙ | **Tree-diff proof:** `git diff --name-only <origin/main-final> main-scrubbed` lists **exactly the 12 files** of §2.1 (the REDACTED set) and nothing else. RED: any 13th path or any missing path ⇒ STOP, the candidate is wrong. | S–M |
| 2.4 | 🔒 Close the 15 open PRs (#6, #8, #10–#18, #20–#23) with a one-line comment pointing at this blueprint + the bundle path. (Branch deletion would auto-close them anyway; explicit closure is honest bookkeeping.) | 🔒 | `gh pr list --state open` == the empty set (or only PRs deliberately kept). RED: an open PR whose head is about to be deleted. | S |
| 2.5 | 🔒 **The force-push wave** (irreversible on GitHub's side): from the scratch clone, with per-branch `--force-with-lease=<branch>:<sha-from-2.1-snapshot>`: push `main-scrubbed → main`, `a8e5844e → fix/audit-remediation`; then `git push origin --delete` the **30** L1 branches (13 merged-dead + 6 historical-with-twins + 11 cloud-created, per §2.1 tables). Leave untouched: `feat/sovereign-core-phase-zero`, `backup-wip-2026-07-08`, `telemetry/plane`. | 🔒 | Lease failure on ANY branch ⇒ full stop (means 2.1 was violated). GREEN: all pushes accepted; `ls-remote` == expected 5-branch keep-set. RED case exists by construction: a moved remote ref fails the lease. | M |
| 2.6 | ⚙ **Reachability + leak verification on a FRESH clone** (never the local repo — its old objects would false-positive): `git clone` + fetch all heads; then (a) `git cat-file -e` for `72fde8a6`, `bccfd324`, `d5eef9cb`, `84b95d66`, `c8b2d5a0`, `914acf8a` ⇒ **all must FAIL** (objects unreachable ⇒ not transferred); (b) `gitleaks git --log-opts="--all --full-history"` over the fresh clone ⇒ 0 findings (honoring `.gitleaksignore`); (c) operator-held pattern file (the rotated literals, from the operator's password manager — **never committed, never echoed**) at `/tmp/leak-patterns.txt`: `git grep -f /tmp/leak-patterns.txt $(git rev-list --all)` ⇒ 0 hits; `shred -u` the file after. | ⚙ | **RED case (mandatory, run FIRST):** the identical (b)+(c) commands against a clone restored from the 2.2 bundle **must flag > 0** — proving the detectors detect. Only then does 0-on-fresh-clone mean anything. | M |
| 2.7 | 🔒 **Server-side residue**: dangling L1 objects and `refs/pull/N/head` remain fetchable **by SHA** on GitHub after any force-push (verified concept: `gh api repos/…/commits/72fde8a6…` returns 200 today and will keep doing so). Before the ADR-020 PUBLIC flip: file a GitHub Support request to gc/purge cached views (per GitHub's sensitive-data procedure and EXPANSION-PLAN §OSS step 3) — OR adopt Option B (fresh-repo swap, §3.1), which deletes this class entirely. | 🔒 | Falsifiable probe: `gh api repos/SyniakSviatoslav/dowiz/commits/<old-sha>` ⇒ expect 404/422 after purge. RED (= today's state): returns 200. **ADR-020 gate #1 is NOT satisfiable while this probe returns 200 on a to-be-public repo.** | S (+Support latency) |
| 2.8 | 🔒 **Thaw**: re-enable Actions; manually dispatch CI on `main` and watch it green; **do not resume any scheduled cloud loop until its workspace is re-cloned from the new origin** (a stale clone's normal push resurrects deleted L1 branches — §2.2); retarget or retire the "re-verify" loop (its branch now has new history). Watch `dowiz.fly.dev/livez` during the first main CI run (tree ≈ identical + GDPR, so deploy should be a near-no-op — but the pipeline has known health-timing landmines, `merge-to-main-plan` memory). | 🔒 | CI green on new main; livez 200 sustained. Post-window tripwire: daily `ls-remote` diff for 7 days ⇒ no unexpected ref. RED: any resurrected branch ⇒ its pusher's clone is stale ⇒ re-pause that loop. | S–M |
| 2.9 | ⚙ Flip the CI secrets gate to full-history mode (now safe): `fetch-depth: 0` + `gitleaks git --log-opts="--all --full-history"` as a required check; keep the fast worktree scan for PR latency. | ⚙ | A scratch branch grafting a canary secret into a **historical** commit (e.g. amend + force-push to the scratch branch only) goes RED in CI. GREEN on clean main. | S |

### Phase 3 — rewrite-aware merge-to-main (design accepted now; execution after staging re-validation)

| # | Action | Gate | VbM proof | Effort |
|---|---|---|---|---|
| 3.1 | ⚙ Land the uncommitted gate fixes sitting in paleo's working tree (`scripts/plane-guard.mjs` P7, `scripts/verify-all.ts`, `reactAction.test.ts` — audit §6.1) so the L3 tree itself is CI-green. | ⚙ | `pnpm verify:all --ci` green on paleo. RED: P7 red on any other checkout of the branch today. | S |
| 3.2 | ⚙ Slice #1 — harness/guardrail scripts to main (closes GH **#9**): tree-squash exactly the 9 missing guardrail scripts + verify-all wiring onto main. | ⚙ push, 🔒 because it deploys prod | CI (incl. the 3 currently-hard-failing gates) green on main. RED: today's main CI with the same gates = red. | S–M |
| 3.3 | 🔒 Subsequent slices in operator-priority order (each = `git commit-tree` adoption of a validated subtree of paleo onto main, staging-rehearsed first, one prod deploy each): candidates — sovereign-core kernel + checkout (needs migrations 085–089 disposition), rebuild `rebuild/` + cutover harness (inert without `CUTOVER_RUST_UPSTREAM`), brand/landing. **Never a raw `git merge paleo`** — cross-root ⇒ the documented 500+ add/add wall (HANDOFF-2026-07-07). | 🔒 each | Per slice: staging Playwright suite green + tree-identity check `git diff <slice-commit> <staging-validated-ref> -- <subtree>` == empty. RED: any unexpected path in the slice diff. | M–L per slice |
| 3.4 | 🔒 Endgame option (only after full-tree staging validation): adopt L3 as main (option M1) if the operator wants paleo's commit granularity as the permanent history; otherwise final slice = full paleo tree squash, and paleo/sovereign branches are kept permanently as the detailed history record. | 🔒 | Final tree assert: `git rev-parse main^{tree} == feat/paleo-dinosaur-digs^{tree}`. RED: hash inequality. | M |

---

## 5) Risks & rollback

1. **Force-push goes wrong / wrong ref pushed** → Rollback = the 2.2 mirror bundle:
   `git clone <bundle> restore && cd restore && git push --force origin 'refs/heads/*:refs/heads/*'`
   restores all 35 branches byte-identically (restoring the *dirty* state is acceptable — creds are
   dead; you are back exactly where you started). Individual PR head branches can also be restored
   via each closed PR's "Restore branch" button within GitHub's retention window. **This entire
   rollback exists only if 2.2 ran — the missing pre-scrub bundle (§2.3) is the cautionary tale.**
2. **Surprise prod deploy** (main push triggers `deploy`) → prevented in-window by disabling
   Actions (2.1); on thaw, the first CI run is manually dispatched and watched. If a deploy breaks
   prod: known recovery per `docs/ops/PROD-UNBLOCK-RUNBOOK-2026-07-03.md` + the
   `merge-to-main-plan` memory (`flyctl machine restart …`, health-grace caveat). The 2.3 tree-diff
   proof bounds the deploy delta to the 12 REDACTED files + GDPR content.
3. **Stale cloud clones resurrect L1** after the scrub (normal, non-force push recreates a deleted
   branch with old history) → schedules stay paused until re-cloned (2.8); 7-day `ls-remote`
   tripwire; Phase 1's CI gitleaks gate turns any resurrected secret-bearing blob RED on the next
   PR; the 2.9 full-history gate makes it structural.
4. **Lease race** (a writer slips through the freeze) → `--force-with-lease` per branch pinned to
   the 2.1 snapshot SHAs fails closed; full stop + re-freeze rather than `--force`.
5. **Dangling objects on GitHub after Phase 2** → not fixable client-side; the 2.7 probe is the
   gate. If Support gc is slow or unsatisfying, Option B (fresh-repo swap) is the guaranteed path —
   decide before the public flip, not after (post-flip, scraping bots archive within minutes).
6. **Secret-bearing backup artifacts on the box** (`/root/dowiz.tar.gz` Jun-17 tree,
   the new pre-forcepush bundle, `restore-dowiz-2026-07-08.tar.zst`) → chmod 600, offline copies
   only, operator retention decision D5. They do not block ADR-020 (not in the repo), but they are
   the last remaining copies of live-format secrets material.
7. **Local repo `/root/dowiz`** still holds L1 objects via remote-tracking refs — untouched by this
   op (leave-as-found). After Phase 2 an ordinary `git fetch --prune` re-points tracking refs; old
   local objects are harmless on a private box and MUST NOT be gc'd casually — `.git/filter-repo/`
   and the local L1 tracking refs are the audit trail that made this blueprint verifiable.
8. **Phase-3 wrong-lineage merge** (someone runs `git merge` across roots) → the M2 design never
   merges by hash; slices are tree-adoptions with tree-identity proofs. HANDOFF-2026-07-07's
   "don't force-push without understanding" is answered by this document: the only main force-push
   is 2.5, once, with the 2.3 tree proof.

---

## 6) Operator decision points

| # | Decision | Options | Blueprint default |
|---|---|---|---|
| **D1** | G01 ordering | GDPR-to-prod **before** scrub vs after | **Before** (§2.7 — harm asymmetry + determinism preserved) |
| **D2** | Freeze window scheduling | pick date/time; requires pausing the ~6-hourly re-verify loop + daily plane-maintainer + Dependabot | next low-activity window ≥ 2h, immediately after G01 deploy is verified |
| **D3** | Scrub vehicle | **A** in-place force-push + GitHub Support gc vs **B** fresh-repo swap | A for now; **re-decide at the ADR-020 flip if the 2.7 probe still returns 200** — B is the guaranteed-clean path and unusually cheap here (0 forks, 2 issues) |
| **D4** | The 8 remote-only "re-verify" doc commits on `origin/fix/audit-remediation` (07-09→07-11) | salvage by cherry-pick from bundle vs accept loss | accept loss (pure doc stamps; recoverable from bundle at any time) |
| **D5** | Retention of secret-bearing offline artifacts (pre-forcepush bundle, `dowiz.tar.gz`, tar.zst) | keep offline indefinitely / keep until ADR-020 flip + 30d soak / destroy after Phase 2 verification | keep until flip + 30d, then destroy the bundle; `dowiz.tar.gz` (Jun-17) has no remaining purpose — destroy sooner |
| **D6** | The 15 open PRs' content | pure close vs close + cherry-pick the useful ones (e.g. #23 channel-liveness, #21 hydrate fix) onto new main | close all; cherry-pick #21/#23 onto new main post-thaw (content-based, rewrite-safe) |
| **D7** | Protect `main` after Phase 2 | add branch protection (required checks, no force-push) — currently **unprotected** | YES — after 2.8, with `verify:all --ci` + the secrets gate as required checks; revisit force-push allowance only for 3.4 |
| **D8** | Phase-3 endgame | M2 slices only vs eventual M1 lineage adoption | M2 now; M1 decision deferred until full-tree staging validation exists |
| **D9** | Retarget or retire the "re-verify audit-remediation backlog" cloud loop (its raison d'être — the backlog — is confirmed complete 8× over) | retire / retarget at new lineage | retire; its job is done and it is the main freeze-window hazard |

---

*Prepared by a read-only research session, 2026-07-11. Every classification above is reproducible
from: `git ls-remote --heads origin`, `.git/filter-repo/{commit-map,ref-map}`, `git merge-base
--is-ancestor`, `gh api repos/SyniakSviatoslav/dowiz/{compare,commits,branches,pulls}`, and
`git bundle list-heads` on the artifacts named in §2.3.*
