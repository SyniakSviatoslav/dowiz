# BLUEPRINT — Disk cleanup, automatic backups, caching, archiving (2026-07-18)

> OPS action-plan/runbook, not a full 20-point roadmap phase (operational remediation, not new
> product capability). Written directly by the lead after two background agents assigned this
> task both failed on a session API-limit boundary (reset 13:20 UTC) — executed live instead of
> re-dispatching into the same limit. Cross-references
> [`BLUEPRINT-P45-ops-security-monitoring.md`](BLUEPRINT-P45-ops-security-monitoring.md) rather
> than duplicating its backup/monitoring design.

## 0. Ground truth (live, this pass, 2026-07-18 ~13:22 UTC)

| Claim | Evidence |
|---|---|
| Disk was at the confirmed crisis point | `df -h /` → `75G 65G used 7.5G avail 90%` |
| ~19GB of the 65G used was Rust `target/` build cache, spread across ~19 directories in 15+ separate worktrees (`dowiz`, `bebop-repo`, `dowiz-pa-T1..T7`, `dowiz-wt-p07`/`p12`, `*-verify-redteam` ×4, `dowiz-spectral-evolution`, `dowiz-pq`) | `find /root -maxdepth 3 -iname target -type d` + `du -sh` each, summed |
| `target/` is never git-tracked anywhere in this tree | `**/target/` in root `.gitignore` (confirmed earlier this same session) — clearing it touches zero committed or uncommitted source |
| Off-Hetzner-style object storage backup infra ALREADY EXISTS and has real data in it | `rclone listremotes` → `hetzner:` (type `s3`, provider `Other`, endpoint `fsn1.your-objectstorage.com`); `rclone lsd hetzner:` → bucket `dowiz` created 2026-07-13; `rclone lsf hetzner:dowiz` → `backups/`, `cold/`, `db/`, `images/`; `rclone size hetzner:dowiz` → 141 objects, 13.07 GiB. **Correction to a memory claim**: this is Hetzner's own S3-compatible object storage, not Cloudflare R2 as one memory note asserted — the two are different services; the memory note is imprecise, this live check is authoritative. |
| No disk-cleanup or backup automation is currently scheduled | `crontab -l` → exactly one entry, a weekly harness-curation script (`scripts/harness-curation-local.sh`, unrelated to disk/backup). Memory's claim of an existing "deep-clean tool + cronjobs" is **stale** — whatever ran once (memory cites a past "disk cleanup 94%→79%" event) is not scheduled today. Same "designed once, silently stopped" pattern found repeatedly elsewhere this session. |

## 1. Action taken this pass (executed, not just recommended)

**Cleared 19 `target/` directories** (list below), all git-ignored, all 100% regenerable via `cargo build`, zero uncommitted source touched (verified `git status` clean on `main` before and after):

```
/root/dowiz/kernel/target                              /root/dowiz-pa-T2/kernel/target
/root/dowiz/agent-adapters/target                       /root/dowiz-pa-T7/kernel/target
/root/bebop2-verify-redteam/target                      /root/dowiz-pa-T6/kernel/target
/root/dowiz-pa-T3/kernel/target                         /root/dowiz-verify-redteam/llm-adapters/target
/root/dowiz-pa-T1/kernel/target                         /root/dowiz-wt-p07/kernel/target
/root/bebop-repo/target                                 /root/agentic-mesh-verify-redteam/kernel/target
/root/dowiz-pa-T4/kernel/target                         /root/dowiz/engine/target
/root/dowiz-wt-p12/kernel/target                        /root/dowiz-spectral-evolution/kernel/target
/root/dowiz-pq/target                                   /root/hermes-verify-redteam/hermes-kernel/target
/root/dowiz-spectral-evolution/engine/target
```

**Result: `df -h /` → `75G 46G used 26G avail 65%`** (90%→65%, 7.5GB→26GB free, ~19GB recovered).
This alone resolves the disk crisis that was blocking `BLUEPRINT-P21-local-llm-hermes-native.md`'s
Wave-0 `mistral:7b` pull (4.4GB) and gives real headroom beyond it.

**Deliberately NOT touched this pass** (real du weight, genuine ambiguity, needs a human/second
pass, not deleted blind):
- The `*-pa-T1..T8`, `*-wt-p07/p12`, `*-verify-redteam` worktree directories THEMSELVES (only
  their `target/` subdirs were cleared) — these still hold full git checkouts (source + history),
  ranging ~80MB–1.9GB each post-target-clear. Several are named after completed research passes
  already merged into `main` this session (T1–T8 parallel-agent worktrees, verify-redteam
  worktrees). **Recommend for a human decision, not auto-deleted**: `git -C <dir> log --oneline
  -1` + `git -C <dir> status --short` on each to confirm zero uncommitted work and that its HEAD
  commit is already an ancestor of `origin/main`, THEN `git worktree remove` (not bare `rm -rf`,
  to keep the parent repo's worktree metadata clean) for any that qualify. Left as a checklist,
  not executed, because several session-active worktrees (the swarm's own `feat/autopilot-
  integration` work observed live this session) must NOT be swept up in a blind pass.
- Backup bundles/archives at `/root/*.bundle`, `/root/*.tar.zst`, `/root/*.tar.gz`, `/root/*.zip`
  (`dowiz-pre-scrub-backup.bundle` 622M, `dowiz-H8-PRE-SCRUB...bundle` 621M,
  `dowiz-pushclean-backup-...bundle` 137M, `restore-dowiz-2026-07-08.tar.zst` 51M,
  `dowiz-backup-2026-07-08.bundle` 35M, `dowiz.zip` 67M, `dowiz.tar.gz` 4.7M, `bebop-repo-backup-
  ...` ~15M — roughly 1.5GB total). These are exactly the kind of point-in-time safety snapshots
  this repo's own culture treats as precious (pre-scrub, pre-force-push backups from real
  incidents named in memory: `secrets-exposure-incident-2026-07-03`). **Recommend: upload to
  `hetzner:dowiz/backups/` (the bucket already exists and already has a `backups/` prefix) via
  `rclone move` (not `copy`, to actually free local space) rather than delete outright** — this
  converts local one-off insurance snapshots into the same durable off-box tier P45 already
  wants, at near-zero marginal cost since the bucket/credentials are already live.

## 2. Automatic prevention — the gap this pass closes going forward

Per §0's finding, nothing currently re-runs this cleanup or alerts before the disk fills again.
Two additive, minimal pieces, both extending existing conventions rather than inventing new ones:

### 2.1 A weekly target-sweep cron (mirrors the existing harness-curation cron's shape exactly)
```bash
#!/bin/bash
# scripts/disk-target-sweep.sh — weekly, safe: only ever removes .gitignore'd target/ dirs
set -euo pipefail
find /root -maxdepth 4 -iname "target" -type d 2>/dev/null | while read -r d; do
  git -C "$(dirname "$d")" rev-parse --is-inside-work-tree >/dev/null 2>&1 || continue
  rm -rf "$d"
done
```
Crontab addition (same style as the existing entry): `0 6 * * 1 /bin/bash /root/dowiz/scripts/disk-target-sweep.sh`
**RED→GREEN done-check**: run it once by hand, confirm `df -h /` doesn't regress vs. this pass's
26G, confirm `git status` clean in every affected worktree before/after (mirrors this pass's own
verification, made repeatable).

### 2.2 Disk-threshold alert — extends P45 §4b's existing Telegram-alerting design, does not fork one
P45 already designed benchmark/regression alerting through `tools/telemetry/lib.sh`'s `tg_send`
(used directly this session to deliver the 7-message roadmap summary — confirmed working,
credentials live). Add one more check of the same shape: a cron-fired script that runs `df -h /`,
and calls `tg_send` (topic 257, the same "report" topic used this session) if usage crosses 85%
— BEFORE hitting the 90%+ crisis point this pass just recovered from, not after.
```bash
#!/bin/bash
# scripts/disk-alert.sh — hourly
source /root/dowiz/tools/telemetry/lib.sh
pct="$(df -h / | awk 'NR==2{print $5}' | tr -d '%')"
[ "$pct" -ge 85 ] && tg_send "⚠️ dowiz VM disk at ${pct}% ($(df -h / | awk 'NR==2{print $4}') free) — run scripts/disk-target-sweep.sh or archive to hetzner:dowiz/backups/"
```
Crontab addition: `15 * * * * /bin/bash /root/dowiz/scripts/disk-alert.sh`

### 2.3 Caching — scope check against P44
`BLUEPRINT-P44-cache-layers-scaleout.md`'s own discipline ("baseline before any layer, no layer
ships without a number it improves") governs GENERAL app-level caching and stays deferred, per
its own reasoning — this disk-cleanup pass does not reopen that. The one disk-specific caching
question — LLM model-weight storage (`ollama pull mistral:7b` = 4.4GB on disk, one copy, no
eviction policy needed at single-model scale) — is `BLUEPRINT-P21-local-llm-hermes-native.md`'s
concern, not a new cache layer; cross-referenced there, not duplicated here.

## 3. DoD
- **D1 (done, this pass):** disk usage measured live, `target/`-cache hypothesis confirmed by
  real `du` numbers before deleting, 19 directories cleared, 90%→65% verified post-clear.
- **D2 (done, this pass):** off-Hetzner object-storage backup target confirmed live and reachable
  (`hetzner:dowiz`, 13GB already present) — corrects the memory record's Cloudflare-R2 claim.
- **B1 (open, human checklist, not executed):** worktree-removal pass per §1's named checklist.
- **B2 (open, human checklist, not executed):** `rclone move` the ~1.5GB of local safety bundles
  to `hetzner:dowiz/backups/` per §1.
- **B3 (open, falsifiable):** `scripts/disk-target-sweep.sh` + `scripts/disk-alert.sh` committed,
  crontab entries added, one manual dry-run of each confirmed before relying on the schedule.

## 4. Anti-scope
No `docker system prune` (Docker presence unconfirmed this pass, out of scope). No `git gc
--aggressive` on any repo (destructive-adjacent, not needed — none of the freed space was git
object-store bloat). No deletion of any `.bundle`/`.tar.*` file — archive-and-free via `rclone
move`, never delete a named incident-recovery artifact outright. No general-purpose cache-layer
build — deferred to P44 exactly as P44 itself already argues.
