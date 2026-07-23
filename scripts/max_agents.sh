#!/bin/bash
# ═══ Maximum Parallel Agents Launcher ═══
# Uses: pre-built binary + taskset (no cargo lock)
# Runs N copies of test suite in parallel, each pinned to a core.

set -euo pipefail

AGENTS="${1:-8}"
KERNEL_DIR="/root/dowiz/kernel"
TEST_BIN=$(find /dev/shm/cargo-target/debug/deps -name "dowiz_kernel-*" -not -name "*.d" -type f 2>/dev/null | head -1)

if [ -z "$TEST_BIN" ]; then
    echo "Pre-building test binary..."
    cd "$KERNEL_DIR" && cargo test --no-run --lib 2>/dev/null
    TEST_BIN=$(find /dev/shm/cargo-target/debug/deps -name "dowiz_kernel-*" -not -name "*.d" -type f | head -1)
fi

echo "[MAX_AGENTS] Launching $AGENTS agents on $(nproc) cores..."
echo "[MAX_AGENTS] Binary: $TEST_BIN ($(du -h "$TEST_BIN" | cut -f1))"
echo "[MAX_AGENTS] Time: $(date '+%H:%M:%S')"

PIDS=()
START=$(date +%s)

for i in $(seq 1 "$AGENTS"); do
    CORE=$(( (i - 1) % $(nproc) ))
    taskset -c "$CORE" "$TEST_BIN" --test-threads=1 --skip hydra_runtime_probe \
        > /dev/shm/agent_${i}.log 2>&1 &
    PIDS+=($!)
done

echo "[MAX_AGENTS] $AGENTS agents running, waiting..."
for pid in "${PIDS[@]}"; do
    wait "$pid" 2>/dev/null || true
done

END=$(date +%s)
ELAPSED=$((END - START))

PASSED=0
FAILED=0
for i in $(seq 1 "$AGENTS"); do
    RESULT=$(grep "test result" /dev/shm/agent_${i}.log 2>/dev/null || echo "")
    if echo "$RESULT" | grep -q "ok\."; then
        PASSED=$((PASSED + 1))
    elif echo "$RESULT" | grep -q "FAILED"; then
        FAILED=$((FAILED + 1))
    fi
    rm -f /dev/shm/agent_${i}.log
done

echo "═══ RESULTS ═══"
echo "Agents:  $AGENTS"
echo "Cores:   $(nproc)"
echo "Time:    ${ELAPSED}s"
echo "Passed:  $PASSED/$AGENTS"
echo "Failed:  $FAILED/$AGENTS"
echo "Per core: $(python3 -c "print(f'{AGENTS/$ELAPSED:.1f}')" 2>/dev/null || echo "N/A") agents/sec"

# Resources
bash /root/dowiz/scripts/resource_watchdog.sh 2>/dev/null || true
