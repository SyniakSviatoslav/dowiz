#!/usr/bin/env bash
# spawn_census.sh — measure process spawning during a chain run
# Counts peak concurrent processes and breakdown by program
# Usage: bash tools/spawn_census.sh <command...>
# Output: writes to stdout, also creates census_summary.txt
# Method: /proc polling (strace unavailable on this proot)
# Measurement notes:
#   - baseline: process count before command starts
#   - peak: max process count during run (50ms sampling interval)
#   - spawns estimate: peak - baseline (may be conservative if processes exit quickly)

set -u

if [ $# -lt 1 ]; then
  echo "Usage: bash tools/spawn_census.sh <command...>" >&2
  exit 1
fi

CMD=("$@")

# Create temp directory for sampling
CENSUS_TMP="/tmp/census_$$"
mkdir -p "$CENSUS_TMP"
trap "rm -rf '$CENSUS_TMP'" EXIT

PROC_SAMPLES="$CENSUS_TMP/proc_samples.txt"
FINAL_LOG="$CENSUS_TMP/final_procs.txt"

# Function to get current process count
proc_count() {
  ls /proc/[0-9]*/comm 2>/dev/null | wc -l
}

# Function to get process breakdown
proc_breakdown() {
  {
    for pid in /proc/[0-9]*/comm; do
      [ -r "$pid" ] 2>/dev/null && cat "$pid" 2>/dev/null
    done
  } 2>/dev/null | sort | uniq -c | sort -rn
}

# Baseline: record initial process count
baseline=$(proc_count)

echo "Starting process sampling (baseline: $baseline procs)..." >&2

# Start background sampling loop
(
  start_ns=$(date +%s%N)
  max_procs=$baseline
  while true; do
    now_ns=$(date +%s%N)
    elapsed_ms=$(( (now_ns - start_ns) / 1000000 ))
    count=$(proc_count)
    echo "$elapsed_ms $count" >> "$PROC_SAMPLES"
    [ "$count" -gt "$max_procs" ] && max_procs=$count
    sleep 0.05
  done
) &
SAMPLER_PID=$!

# Give sampler a moment to start collecting
sleep 0.1

# Run the actual command and time it
t0=$(date +%s%N)
"${CMD[@]}"
cmd_rc=$?
t1=$(date +%s%N)

# Stop the sampling loop
sleep 0.1  # Let sampler capture tail of execution
kill $SAMPLER_PID 2>/dev/null || true
wait $SAMPLER_PID 2>/dev/null || true

# Calculate elapsed time
elapsed_ns=$((t1 - t0))
elapsed_ms=$((elapsed_ns / 1000000))
elapsed_s=$(awk "BEGIN {printf \"%.2f\", $elapsed_ns / 1000000000}")

# Get peak from samples and also capture final state
peak_procs=$baseline
if [ -f "$PROC_SAMPLES" ] && [ -s "$PROC_SAMPLES" ]; then
  peak_procs=$(awk '{if ($2 > max) max = $2} END {print (max == "" ? 0 : max)}' "$PROC_SAMPLES")
fi
[ -z "$peak_procs" ] && peak_procs=$baseline

# Estimate total spawns as peak - baseline
estimated_spawns=$((peak_procs - baseline))
[ "$estimated_spawns" -lt 0 ] && estimated_spawns=0

# Get final process breakdown
proc_breakdown > "$FINAL_LOG"

# Get final process count
final_procs=$(proc_count)

# Create output summary
cat > ./census_summary.txt <<EOF
Census measurement (method: /proc polling, 50ms sampling)
==========================================================

COMMAND: ${CMD[*]}
EXIT CODE: $cmd_rc

TIMING:
  Elapsed: ${elapsed_ms}ms (${elapsed_s}s)

PROCESS COUNTS:
  Baseline (before): $baseline
  Peak (during): $peak_procs
  Final (after): $final_procs
  Estimated spawns (peak - baseline): $estimated_spawns
  Max concurrent procs vs Android cap (32): $peak_procs/32

PROCESS BREAKDOWN (final state):
EOF

cat "$FINAL_LOG" >> ./census_summary.txt

# Emit results to stdout
echo "Census summary (method: /proc polling, 50ms samples)"
echo "===================================================="
echo "Elapsed: ${elapsed_ms}ms (${elapsed_s}s)"
echo "Peak concurrent processes: $peak_procs"
echo "Baseline: $baseline"
echo "Estimated spawns: $estimated_spawns"
echo "Exit code: $cmd_rc"
echo ""
echo "Process breakdown (final):"
cat "$FINAL_LOG"

# Emit JSON for structured parsing
python3 << PYSCRIPT 2>/dev/null || true
import json

result = {
  "method": "proc_polling_50ms",
  "strace_available": False,
  "elapsed_ms": $elapsed_ms,
  "elapsed_s": float($elapsed_s),
  "baseline_procs": $baseline,
  "peak_concurrent_procs": $peak_procs,
  "final_procs": $final_procs,
  "estimated_spawns": $estimated_spawns,
  "cmd_exit_code": $cmd_rc,
  "android_cap_headroom": 32 - $peak_procs,
  "blind_spots": [
    "strace not available on this proot",
    "counts peak process delta, not individual execve syscalls",
    "process reuse between runs counted as multiple spawns",
    "brief spikes between 50ms samples may be missed",
    "program breakdown is from final state snapshot only",
    "sampling starts after command, may miss initial burst"
  ]
}

print(json.dumps(result, indent=2))
PYSCRIPT

exit $cmd_rc
