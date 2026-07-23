#!/bin/bash
# ═══ RESOURCE WATCHDOG — 95% utilization with auto-stop ═══
# Run: watch -n 2 ./scripts/resource_watchdog.sh
# Or: while true; do ./scripts/resource_watchdog.sh; sleep 2; done

set -euo pipefail

THRESHOLD_CPU_PCT=95
THRESHOLD_RAM_PCT=95
THRESHOLD_RAMDISK_PCT=95
THRESHOLD_DISK_FREE_PCT=5

# ── CPU ──
cpu_used=$(top -bn1 | grep "Cpu(s)" | awk '{print 100 - $8}' | cut -d. -f1)
cpu_pct=${cpu_used:-0}

# ── RAM ──
ram_info=$(free | awk '/^Mem:/ {printf "%.0f", $3/$2*100}')
ram_pct=${ram_info:-0}

# ── RAM disk ──
ramdisk_pct=$(df /dev/shm 2>/dev/null | awk 'NR==2 {printf "%.0f", $3/$2*100}')
ramdisk_pct=${ramdisk_pct:-0}

# ── Disk ──
disk_free_pct=$(df / 2>/dev/null | awk 'NR==2 {printf "%.0f", $4/$2*100}')
disk_free_pct=${disk_free_pct:-100}

# ── Status ──
status="OK"
alerts=""

if [ "$cpu_pct" -gt "$THRESHOLD_CPU_PCT" ]; then
    status="THROTTLE"
    alerts="$alerts CPU:${cpu_pct}%>${THRESHOLD_CPU_PCT}%"
fi

if [ "$ram_pct" -gt "$THRESHOLD_RAM_PCT" ]; then
    status="THROTTLE"
    alerts="$alerts RAM:${ram_pct}%>${THRESHOLD_RAM_PCT}%"
fi

if [ "$ramdisk_pct" -gt "$THRESHOLD_RAMDISK_PCT" ]; then
    status="CLEAN"
    alerts="$alerts RAMDISK:${ramdisk_pct}%>${THRESHOLD_RAMDISK_PCT}%"
    # Auto-clean incremental artifacts
    cargo sweep --incremental 2>/dev/null || rm -rf /dev/shm/cargo-target/debug/incremental 2>/dev/null
fi

if [ "$disk_free_pct" -lt "$THRESHOLD_DISK_FREE_PCT" ]; then
    status="CRITICAL"
    alerts="$alerts DISK:${disk_free_pct}%<${THRESHOLD_DISK_FREE_PCT}%"
    # Emergency: clean cargo home cache
    cargo sweep --time 0 2>/dev/null || true
fi

echo "[WATCHDOG] status=$status CPU=${cpu_pct}% RAM=${ram_pct}% RAMDISK=${ramdisk_pct}% DISK=${disk_free_pct}% free${alerts}"

# Signal to external orchestration
if [ "$status" = "CRITICAL" ]; then
    echo "[WATCHDOG] CRITICAL — signal to kill least-util agent"
    exit 2
elif [ "$status" = "THROTTLE" ]; then
    echo "[WATCHDOG] THROTTLE — no new agents"
    exit 1
fi
exit 0
