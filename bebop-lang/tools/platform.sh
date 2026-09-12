#!/bin/bash
# Platform fragility early warning system (2026-09-13).
# Seven one-line runtime probes measuring hardware/kernel/environment assumptions.
# Frozen values in tools/platform.txt guide gate decisions; deviations fail fast.
# See tools/platform.txt for WHAT BREAKS on each value change.

set -eu

pagesize=$(getconf PAGESIZE)
kernel_release=$(uname -r)
core_count=$(getconf _NPROCESSORS_ONLN)

# LSE atomics (ldaddal for sys_atomic_add)
lse_atomics=$(grep -c '^Features.*atomics' /proc/cpuinfo || echo 0)

# CRC32 extension (crc32x for optional acceleration)
crc32=$(grep -c '^Features.*crc32' /proc/cpuinfo || echo 0)

# TracerPid from /proc/self/status: nonzero means under debugger/proot.
# Emitted as a BOOLEAN, not as the pid (2026-09-12). The pid is the proot process's, so it
# changes with every session -- freezing it made rung (0) print "PLATFORM CHANGED" and turn
# invariants RED in every fresh session, for a value no gate depends on. What the gates
# actually assume is `are we traced at all`, because tracing is what inflates latency 10-100x.
traced=$([ "$(grep '^TracerPid:' /proc/self/status | awk '{print $2}')" = 0 ] && echo 0 || echo 1)

# Mount options for the filesystem holding stores
# (This store may be /tmp/L07/scrash.store; check /tmp mount or current working directory)
store_mount=$(df . | tail -1 | awk '{print $NF}')
mount_opts=$(mount | grep " $store_mount " | grep -o 'nobarrier' || echo 'barrier-enabled')

# Output as name=value pairs
echo "pagesize=$pagesize"
echo "lse_atomics=$lse_atomics"
echo "crc32=$crc32"
echo "traced=$traced"
echo "fsync_mode=$mount_opts"
echo "kernel_release=$kernel_release"
echo "core_count=$core_count"
