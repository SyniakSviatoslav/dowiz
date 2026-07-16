#!/usr/bin/env bash
# scripts/v5c-reexec.sh — V5-C independent re-execution verifier (S6).
# BLUEPRINT-P01-ci-truth-floor.md §2.8 (Wave 0; the P06 precondition harness).
#
# WHAT: re-executes a diff range's tests in a CLEAN, INDEPENDENT git worktree
# (never the shared working tree) and emits RED|GREEN + a rationale JSON.
# It is the "second party" that re-runs the claim instead of trusting a
# self-reported GREEN — the structural answer to self-certified "done" gates.
#
# UNSIGNED in Phase 1. Phase 6 wraps THIS SAME runner with ML-DSA key_K/key_V
# signatures + a merge gate (P01 §5: "the same runner"). No crypto is done here.
#
# RED-LINE GATE (per blueprint `if: red-line paths touched`): the full re-exec
# runs ONLY when the diff range touches a red-line surface —
#   money.rs / order_machine.rs / event_log.rs / anything auth-related.
# Otherwise it emits verdict SKIP with a reason and exits 0 (dev-time fence:
# re-executing every green diff would be wasteful; the money/order/auth core is
# where an independent re-run earns its cost).
#
# ISOLATION: uses `git worktree add` to a fresh temp dir checked out at the head
# SHA, runs `cargo test --offline` for kernel AND engine there, then removes the
# worktree (trap-guaranteed). The shared tree is never mutated — this is the
# standing "independent worktree" law, not a same-tree checkout.
#
# USAGE
#   scripts/v5c-reexec.sh [<base>] [<head>]     # defaults: origin/main HEAD
#
# EXIT 0  — GREEN (both suites pass) or SKIP (no red-line path touched).
# EXIT 1  — RED (a suite failed) — this is the merge-gating signal.
# EXIT 2  — usage / git resolution error.
set -euo pipefail

BASE_REF="${1:-origin/main}"
HEAD_REF="${2:-HEAD}"

# --- resolve refs to SHAs (base tolerates a missing origin/main in CI) -------
resolve() { git rev-parse --verify --quiet "${1}^{commit}"; }

HEAD_SHA="$(resolve "${HEAD_REF}")" || { echo "v5c-reexec: cannot resolve head '${HEAD_REF}'" >&2; exit 2; }
BASE_SHA="$(resolve "${BASE_REF}")" || true
if [[ -z "${BASE_SHA:-}" ]]; then
  # Fallbacks so the harness still runs when 'origin/main' isn't a local ref.
  BASE_SHA="$(resolve main || resolve "${HEAD_SHA}~1" || true)"
fi
if [[ -z "${BASE_SHA:-}" ]]; then
  echo "v5c-reexec: cannot resolve base '${BASE_REF}' (nor main / HEAD~1)" >&2
  exit 2
fi

# --- red-line detection over the diff range ---------------------------------
# money / orders / event_log / auth (auth also covers otp/jwt/login surfaces).
REDLINE_RE='money\.rs|order_machine\.rs|event_log\.rs|auth|otp|jwt'
CHANGED="$(git diff --name-only "${BASE_SHA}" "${HEAD_SHA}")"
REDLINE_HITS="$(printf '%s\n' "${CHANGED}" | grep -Ei "${REDLINE_RE}" || true)"

# JSON array from a newline-separated list of ASCII-safe repo paths / test ids.
json_array() {
  local first=1 item
  printf '['
  while IFS= read -r item; do
    [[ -z "${item}" ]] && continue
    item="${item//\"/}"          # defensive: strip any stray quote
    if [[ ${first} -eq 1 ]]; then first=0; else printf ','; fi
    printf '"%s"' "${item}"
  done <<< "${1}"
  printf ']'
}

if [[ -z "${REDLINE_HITS}" ]]; then
  echo "V5C-VERDICT: SKIP"
  printf '{"verdict":"SKIP","signed":false,"base":"%s","head":"%s","red_line_paths":[],"reason":"no red-line path (money.rs/order_machine.rs/event_log.rs/auth) touched in base..head; V5-C re-exec is red-line-gated per BLUEPRINT-P01 §2.8"}\n' \
    "${BASE_SHA}" "${HEAD_SHA}"
  exit 0
fi

# --- clean independent worktree at the head SHA -----------------------------
WORKTREE="$(mktemp -d "${TMPDIR:-/tmp}/v5c-reexec.XXXXXX")"
cleanup() { git worktree remove --force "${WORKTREE}" >/dev/null 2>&1 || rm -rf "${WORKTREE}"; }
trap cleanup EXIT

git worktree add --detach "${WORKTREE}" "${HEAD_SHA}" >/dev/null 2>&1 \
  || { echo "v5c-reexec: git worktree add failed" >&2; exit 2; }

run_suite() {  # $1=manifest-relative-path  $2=logfile  -> echoes "exit passed failed"
  local manifest="$1" log="$2" ex
  set +e
  ( cd "${WORKTREE}" && cargo test --offline --manifest-path "${manifest}" ) > "${log}" 2>&1
  ex=$?
  set -e
  local p f
  p="$(grep -oE '[0-9]+ passed'  "${log}" | awk '{s+=$1} END{print s+0}')"
  f="$(grep -oE '[0-9]+ failed'  "${log}" | awk '{s+=$1} END{print s+0}')"
  echo "${ex} ${p} ${f}"
}

KLOG="${WORKTREE}.kernel.log"
ELOG="${WORKTREE}.engine.log"
trap 'cleanup; rm -f "${KLOG}" "${ELOG}"' EXIT

read -r K_EXIT K_PASS K_FAIL < <(run_suite kernel/Cargo.toml "${KLOG}")
read -r E_EXIT E_PASS E_FAIL < <(run_suite engine/Cargo.toml "${ELOG}")

# Failing test ids (empty on GREEN) — parsed from both suite logs.
FAILING="$( { grep -hE '\.\.\. FAILED' "${KLOG}" "${ELOG}" 2>/dev/null \
            | sed -E 's/^test ([^ ]+) \.\.\. FAILED.*/\1/'; } || true )"

VERDICT="GREEN"
EXIT_CODE=0
if [[ "${K_EXIT}" -ne 0 || "${E_EXIT}" -ne 0 ]]; then
  VERDICT="RED"
  EXIT_CODE=1
fi

echo "V5C-VERDICT: ${VERDICT}"
printf '{"verdict":"%s","signed":false,"base":"%s","head":"%s","red_line_paths":%s,"suites":{"kernel":{"ran":true,"exit":%s,"passed":%s,"failed":%s},"engine":{"ran":true,"exit":%s,"passed":%s,"failed":%s}},"failing_tests":%s,"note":"UNSIGNED (Phase 1); Phase 6 wraps this runner with ML-DSA key_K/key_V signatures"}\n' \
  "${VERDICT}" "${BASE_SHA}" "${HEAD_SHA}" \
  "$(json_array "${REDLINE_HITS}")" \
  "${K_EXIT}" "${K_PASS}" "${K_FAIL}" \
  "${E_EXIT}" "${E_PASS}" "${E_FAIL}" \
  "$(json_array "${FAILING}")"

exit "${EXIT_CODE}"
