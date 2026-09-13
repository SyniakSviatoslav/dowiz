#!/usr/bin/env bash
# build_tkernel.sh -- reproducible build of the kernel binary (selfhost/tkernel.bp)
# This script ensures the kernel is built from source and cached in the tree,
# not in /tmp which is ephemeral and not tracked.

set -u
cd "$(dirname "$0")/.." || exit 1

# Output path in the tree (not /tmp)
TKERNEL_BIN="${TKERNEL_BIN:-./tkernel.bin}"
SEED="${SEED:-./seed/build/seed}"

# Guard: seed binary must exist
if [ ! -x "$SEED" ]; then
    echo "build_tkernel.sh: ERROR seed binary not found or not executable: $SEED" >&2
    exit 1
fi

# Guard: source must exist
if [ ! -f selfhost/tkernel.bp ]; then
    echo "build_tkernel.sh: ERROR source not found: selfhost/tkernel.bp" >&2
    exit 1
fi

# If binary already exists and is non-empty, skip rebuild
if [ -s "$TKERNEL_BIN" ]; then
    echo "build_tkernel.sh: $TKERNEL_BIN already built (skipping rebuild)"
    exit 0
fi

# Build via cc.sh (which handles slot serialization)
# cc.sh takes: <bin0> <src.bp> <out.bin> where bin0 is the bootstrap bebop compiler
bash tools/cc.sh ./bebop.bin selfhost/tkernel.bp "$TKERNEL_BIN"
