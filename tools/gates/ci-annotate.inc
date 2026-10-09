#!/bin/sh
# A RED CARGO STEP NAMES ITS OWN FAILURE. Job logs need a token to read; annotations do not
# (/check-runs/<job_id>/annotations is public). kernel and dowiz-core sat red on every push from
# 2026-10-05 to 2026-10-08 with nothing but "exit code 101" visible, because the one thing that
# fails there (x86_64 glibc) is the one thing the aarch64 dev box cannot run natively.
#
#   sh tools/gates/ci-annotate.inc <title> <command...>
#
# Runs the command, keeps its exit code, and on failure prints up to 20 `panicked at`, `FAILED`,
# `error[`/`error:`, node e2e `FAIL ` and `...Error: ` lines as ::error annotations titled <title>. Outside GitHub Actions it
# only runs the command. Named .inc so run-all's tools/gates/*.sh glob never runs it as a gate.
set -u
title="$1"; shift
out="$(mktemp)"
"$@" > "$out" 2>&1
rc=$?
cat "$out"
if [ "$rc" != 0 ] && [ -n "${GITHUB_ACTIONS:-}" ]; then
  # CI sets CARGO_TERM_COLOR=always: strip the colour codes first, or `^error` never matches
  # (gen-vocab's E0063 went unannotated on be1ed8e1).
  sed 's/\x1b\[[0-9;]*[A-Za-z]//g' "$out" > "$out.plain"; mv "$out.plain" "$out"
  grep -E -A1 'panicked at|\.\.\. FAILED|^error(\[|:)|^FAIL |^[A-Za-z]*Error: ' "$out" | grep -v '^--$' | head -20 | while IFS= read -r line; do
    printf '::error title=%s (rc=%s)::%s\n' "$title" "$rc" "$(printf '%s' "$line" | tr -d '\r')"
  done
fi
rm -f "$out"
exit "$rc"
