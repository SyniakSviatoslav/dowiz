#!/usr/bin/env bash
# verify-pa-wave1.sh — runs each P-A Wave-1 agent's acceptance gate FRESH on disk.
# Per swarm skill: do NOT trust subagent self-reports; re-run the actual gate here.
# Usage: bash scripts/verify-pa-wave1.sh
set -uo pipefail
ROOT=/root/dowiz
WT=("$ROOT/dowiz-pa-T1" "$ROOT/dowiz-pa-T2" "$ROOT/dowiz-pa-T3" "$ROOT/dowiz-pa-T5" "$ROOT/dowiz-pa-T7")
TAG=("T1-ema" "T2-eig2x2" "T3-canon" "T5-eqc-ext" "T7-spectral-const")
# acceptance command per worktree (the named cargo test gate from the blueprint §10)
CMD=(
  "cargo test -p kernel ema_next_generated"
  "cargo test -p kernel householder"
  "cargo test -p kernel spectral_cache && cargo test -p kernel markov"
  "cd $ROOT/tools/eqc-rs && cargo test --release"
  "cargo test -p kernel order_machine"
)
overall=0
for i in "${!WT[@]}"; do
  w="${WT[$i]}"; t="${TAG[$i]}"; c="${CMD[$i]}"
  echo "════════════════════════════════════════════════════"
  echo "VERIFY $t  @ $w"
  echo "branch: $(git -C "$w" rev-parse --abbrev-ref HEAD)  HEAD: $(git -C "$w" rev-parse --short HEAD)"
  echo "cmd: $c"
  ( cd "$w" && eval "$c" ) >/tmp/pa-verify-$t.log 2>&1
  rc=$?
  if [ $rc -eq 0 ]; then echo "  ✅ PASS (rc=0)"; else echo "  ❌ FAIL (rc=$rc) — tail:"; tail -n 12 "/tmp/pa-verify-$t.log"; overall=1; fi
done
echo "════════════════════════════════════════════════════"
echo "WAVE-1 VERIFY EXIT=$overall"
exit $overall
