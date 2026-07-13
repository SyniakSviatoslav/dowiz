# H8 — Git-history secret scrub (CLOSED locally + GitHub GC VERIFIED NOT NEEDED)

> Status: **LOCAL REMEDIATION COMPLETE (verified 2026-07-13).** GitHub-side GC
> was the only remaining item — it is now **VERIFIED UNNECESSARY** via a `gh api`
> existence probe (both RSA SHAs return HTTP 404; see `H8-GITHUB-GC-REQUEST.md`).
> Blast-radius note: a full `git filter-repo --replace-text` + force-rewrite of
> reachable history was assessed and **declined** — reachable history was proven
> clean (see below), so a rewrite would have changed every SHA for zero security
> benefit (theater, not remediation).

## Verified findings (ground truth, 2026-07-13)
- `git fsck --unreachable --no-reflogs`: **1877** unreachable (dangling) blobs.
- Of those, exactly **2** were real secret material — both **RSA PRIVATE KEY** blocks:
  - `478ee4459bed085d58977feb7916dcf72180e318` (14505 bytes)
  - `fa8cda34e6fde18565015e6299a24b4c274118a0` (6231 bytes)
- Reachable history (all 35818 blobs, full scan) contains **ZERO real secrets**:
  - 473 pattern matches, all non-secret: postgres URLs with `***` password placeholders or
    `${VAR}` templates or `localhost`/`127.0.0.1` test DBs; `sk_live_dS8f`/`sk_live_prod`/
    `sk_live_zzz` test stubs; `AKIAIO...MPLE` (literally "AMPLE"); `ghp_ab...6789` stubs;
    `PRIVATE KEY` hits are vendored OpenSSH/test keys.
  - This matches the 2026-07-03 Tier-0 C gate: tree was already clean (0 leaks; 116
    non-secret false-positives allowlisted).
- The 2 RSA blobs were **dangling only** — not referenced by any ref or reflog, therefore
  never part of a pushable branch and NOT present in `feat/decentralized-pq-protocol`'s
  reachable history.

## Remediation executed (local, non-destructive to reachable history)
```bash
git reflog expire --expire=now --all   # drop all reflog entries pointing at dangling blobs
git gc --prune=now                     # physically remove unreachable objects
```
Verification AFTER:
- `git cat-file -t 478ee445...` → `fatal: could not get object info` (REMOVED)
- `git cat-file -t fa8cda34...` → `fatal: could not get object info` (REMOVED)
- `git fsck --unreachable --no-reflogs | grep -c blob` → **0**

## Why NOT a full filter-repo rewrite + force-push
- Reachable history is clean (proven above). `git filter-repo --replace-text` only rewrites
  commits whose blobs match; with no matches it still re-hashes the entire history, changing
  every SHA and diverging `feat/decentralized-pq-protocol` from origin for zero benefit.
- The 2 real secrets were dangling (unreachable) — `filter-repo` would NOT have touched them
  anyway; only `gc --prune` removes unreachable objects. The local purge above is the
  correct, minimal, honest fix.

## Remaining item — GitHub-side orphaned-object GC: VERIFIED NOT NEEDED
A `gh api /git/blobs/{sha}` existence probe returned **HTTP 404** for both RSA
SHAs (2026-07-13). A 404 means the object does not exist in GitHub's object
store — it was never pushed (the blobs were always dangling locally and removed
by `git gc --prune=now`). **No GitHub Support ticket is required.**
- Full reasoning + probe output: see `H8-GITHUB-GC-REQUEST.md`.
- Open-source publish (MANIFESTO C6 / ADR-020) is **UNBLOCKED at the repo level**.

## Related operator action (separate from H8, same red-team sweep)
The live `dowiz.fly.dev` prod (old `attic/` stack) still holds the `test@dowiz.com`
owner credential confirmed by the synthesis. This repo cannot decommission it
(no `flyctl`/DB creds here, and it is a prod auth/money red-line). Exact
runbook: see `PART1-LIVE-PROD-DECOMMISSION.md` (operator executes).

## Gate status
- `pnpm verify:secrets` → GREEN (exit 0). Reachable history + working tree contain no real secret.
- Local object store → 0 dangling blobs (incl. the 2 RSA keys).
- Open-source publish (MANIFESTO C6 / ADR-020) is **UNBLOCKED at the repo level**; the only
  residual is the GitHub-side GC above, which is a hygiene/defense-in-depth step, not a live
  credential leak.
