# H8 — GitHub-side GC: VERIFIED NOT NEEDED (2026-07-13)

> Definitive verdict: the two RSA private-key blobs are **NOT retrievable from
> GitHub**. A `gh api` existence probe returned HTTP 404 for both SHAs:
>
> ```
> gh api "repos/SyniakSviatoslav/dowiz/git/blobs/478ee4459bed085d58977feb7916dcf72180e318" -> 404
> gh api "repos/SyniakSviatoslav/dowiz/git/blobs/fa8cda34e6fde18565015e6299a24b4c274118a0" -> 404
> ```
>
> A 404 from `/git/blobs/{sha}` means the object does not exist in GitHub's
> object store → it was never pushed (the blobs were always dangling/unreachable
> locally and were removed by the local `git gc --prune=now`). **No GitHub
> Support ticket is required.** Open-source publish is unblocked at the repo
> level.

## Why 404, not 200
- The blobs were dangling-only in the local repo (not referenced by any ref).
- The push of `feat/decentralized-pq-protocol` was a fast-forward of a branch
  that never contained those SHAs, so they were never transmitted to GitHub.
- Therefore GitHub never stored them; the local purge was sufficient.

## Local state (still true)
- `git fsck --unreachable --no-reflogs | grep -c blob` → **0**
- `pnpm verify:secrets` → GREEN

See `H8-SECRET-SCRUB-RUNBOOK.md` for the full local remediation record.
