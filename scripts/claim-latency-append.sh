#!/usr/bin/env bash
# scripts/claim-latency-append.sh — V5-B claim-latency ledger appender.
# BLUEPRINT-P01-ci-truth-floor.md §2.7 (Wave 0; HERMETIC-REMEDIATION-PLAN §1 rows #7/#27).
#
# Appends ONE JSONL entry per commit to docs/ledger/claim-latency.jsonl:
#   {commit_sha, authored_ts, ci_observed_green_ts, delta_s, diff_loc}
#
#   commit_sha           full 40-hex of the commit being recorded
#   authored_ts          author time (unix epoch, `git show -s --format=%at`)
#   ci_observed_green_ts observation time = NOW (`date +%s`), when the caller/CI
#                        is confirming this commit's tree is GREEN
#   delta_s              ci_observed_green_ts - authored_ts (the claim latency)
#   diff_loc             lines changed in that commit (added+deleted, `--numstat`)
#
# Phase 1 builds ONLY the appender. The anomaly detector (the "52s on a 1610-line
# diff" flag) is Phase 8's consumer (P08 §4) — deliberately NOT built here.
#
# The appender is IDEMPOTENT: a commit_sha already present in the ledger is
# skipped, so re-running CI on the same commit never double-appends. Acceptance
# criterion 8 ("ledger grows by exactly one per NEW commit") holds on re-run.
#
# USAGE
#   scripts/claim-latency-append.sh                 # record HEAD (default)
#   scripts/claim-latency-append.sh <sha>           # record one commit
#   scripts/claim-latency-append.sh <base> <head>   # record each commit in base..head
#   scripts/claim-latency-append.sh <base>..<head>  # same, range form
#
# EXIT 0 on success (including "all already recorded"); non-zero on git/usage error.
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
LEDGER="${REPO_ROOT}/docs/ledger/claim-latency.jsonl"
mkdir -p "$(dirname "${LEDGER}")"
touch "${LEDGER}"

# --- resolve the set of commits to record (oldest-first) --------------------
commits=()
case "$#" in
  0)
    commits=("$(git rev-parse HEAD)")
    ;;
  1)
    if [[ "$1" == *".."* ]]; then
      mapfile -t commits < <(git rev-list --reverse "$1")
    else
      commits=("$(git rev-parse "$1^{commit}")")
    fi
    ;;
  2)
    mapfile -t commits < <(git rev-list --reverse "$1..$2")
    ;;
  *)
    echo "claim-latency-append: usage: $0 [<sha> | <base> <head> | <base>..<head>]" >&2
    exit 2
    ;;
esac

if [[ "${#commits[@]}" -eq 0 ]]; then
  echo "claim-latency-append: no commits in range — nothing to append." >&2
  exit 0
fi

# The observation time is captured ONCE, at invocation — this is the moment the
# caller (CI) is confirming green, shared across every commit recorded this run.
observed_ts="$(date +%s)"

appended=0
skipped=0
for sha in "${commits[@]}"; do
  full_sha="$(git rev-parse "${sha}")"

  # Idempotency: never append a commit already in the ledger.
  if grep -q "\"commit_sha\":\"${full_sha}\"" "${LEDGER}" 2>/dev/null; then
    skipped=$((skipped + 1))
    continue
  fi

  authored_ts="$(git show -s --format=%at "${full_sha}")"
  delta_s=$((observed_ts - authored_ts))

  # diff_loc = added + deleted across the commit's own diff (vs its first parent;
  # root commit diffs against the empty tree). Binary files show '-' in numstat —
  # counted as 0. Empty commits sum to 0.
  diff_loc="$(git show --numstat --format='' "${full_sha}" \
    | awk '{ a=$1; d=$2; if (a=="-") a=0; if (d=="-") d=0; s+=a+d } END { print s+0 }')"

  printf '{"commit_sha":"%s","authored_ts":%s,"ci_observed_green_ts":%s,"delta_s":%s,"diff_loc":%s}\n' \
    "${full_sha}" "${authored_ts}" "${observed_ts}" "${delta_s}" "${diff_loc}" >> "${LEDGER}"
  appended=$((appended + 1))
done

echo "claim-latency-append: appended ${appended}, skipped ${skipped} (already present) -> ${LEDGER}"
exit 0
