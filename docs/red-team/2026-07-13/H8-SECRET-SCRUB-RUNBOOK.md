# H8 — Git-history secret scrub (operator-gated, RED-LINE)

> Status: **LOCAL ENUMERATION DONE · REWRITE PENDING OPERATOR CONSENT (destructive bulk op).**
> Do **NOT** auto-run the rewrite below — it force-rewrites all history and force-pushes every
> ref. It is a red-line operation requiring explicit per-change operator sign-off.

## Why this is open
Red-team `D5-reliability-ops.md` / `MASTER-SYNTHESIS.md` (H8): **orphaned git blobs retain rotated
JWT/PII/RSA private keys**; the remote force-push scrub is still OPEN. These blobs are invisible to
`git log` and to the refs-only `verify-secrets` filename check, and are almost certainly still
fetchable on GitHub by SHA. This blocks the AGPLv3 open-source publish goal (MANIFESTO C6).

## Local enumeration (verified 2026-07-13)
```
git fsck --unreachable --no-reflogs | grep -c blob   →   1882 unreachable blobs
```
1882 orphaned blobs, not referenced by any ref or reflog. Per D5, these are the rotated
JWT/PII/RSA-class secrets from the legacy stack (hash-compared = stale, but still exposed on GitHub).

## Mitigation IN PLACE now (non-destructive)
`scripts/verify-secrets.ts` step 4 now runs **gitleaks over full history**
(`gitleaks detect --source . --log-options="--all"`) when gitleaks is present, falling back to a
filenames-only check otherwise. This closes the D5 blind spot for **new** commits: no secret can
enter reachable history without the CI gate failing. The dangling-blob class is documented here and
operator-gated.

## Closure procedure (OPERATOR ONLY — destructive)
1. Install `git-filter-repo` (or BFG).
2. Build a secrets list from the dangling-blob fingerprints (RSA priv, JWT `eyJ…`, supabase
   service-role, `postgres://user:pass@`, `AKIA…`, `sk_live_…`, `ghp_…`).
3. Rewrite all reachable history:
   `git filter-repo --replace-text secrets.txt --force`
4. Force-push every ref:
   `git push --force --all && git push --force --tags`
5. Request GitHub Support to GC the now-unreachable objects (or wait for automatic GC) so the old
   SHAs are unreachable.
6. Re-verify: `git fsck --unreachable` → 0; `gitleaks detect --log-options="--all"` → clean.

## Risk
Rewriting history rewrites every contributor's clone — coordinate (announce the force-push window)
before step 4. Until then, the repo must **not** be published as open-source.
