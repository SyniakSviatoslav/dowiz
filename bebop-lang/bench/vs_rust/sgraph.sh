#!/usr/bin/env bash
# G8 (T117) stage 1: the CSR graph in the store vs sqlite: build, BFS from S sources
# (fold == python oracle for the same S), neighbour queries. env: BEBOP_BIN, BEBOP_TMP,
# NSRC (sources for the fold rows, default 3), NSRCB (sources for the bebop timing row, 100).
set -u
cd "$(dirname "$0")/../.."
ulimit -s 65536 2>/dev/null
T=${BEBOP_TMP:-/tmp/opencode}; BB=${BEBOP_BIN:-./bebop.bin}; NSRC=${NSRC:-3}; NSRCB=${NSRCB:-100}
BIG=$(awk '/^processor/{p=$3} /CPU part/ && $NF=="0xd41"{print p}' /proc/cpuinfo | tr '\n' ' ')
PIN=$(python3 -c "import os;u=sorted(os.sched_getaffinity(0));b=[int(x) for x in '$BIG'.split()];print(next((c for c in b if c in u),u[0]))")
./seed/build/seed "$BB" compile bench/vs_rust/std_tests/sgraph.bp "$T/sgraph.bin" >/dev/null 2>&1 || { echo "COMPILEFAIL sgraph"; exit 1; }
rm -f sgraph.store sgraph.store.tmp
bb() { taskset -c "$PIN" ./seed/build/seed "$T/sgraph.bin" "$@" | tail -1; }
build=$(bb b t); bfs_f=$(bb f f "$NSRC"); bfs_t=$(bb f t "$NSRCB"); nbr_f=$(bb n f); nbr_t=$(bb n t); size=$(stat -c %s sgraph.store)
ora=$(taskset -c "$PIN" python3 bench/oracles/sgraph.py "$NSRC"); ora_n=$(taskset -c "$PIN" python3 bench/oracles/sgraph.py nbr)
sq=$(taskset -c "$PIN" python3 bench/tq_sqlite/sgraph_sqlite.py "$NSRC")
g() { sed "s/.*$1=\([^ ]*\).*/\1/" <<<"$sq"; }
ok=$([ "$bfs_f" = "$ora" ] && [ "$bfs_f" = "$(g bfs_fold)" ] && [ "$nbr_f" = "$ora_n" ] && [ "$nbr_f" = "$(g nbr_fold)" ] && echo equal || echo "MISMATCH bebop $bfs_f/$nbr_f oracle $ora/$ora_n sqlite $(g bfs_fold)/$(g nbr_fold)")
out="# G8 sgraph stage 1 ($(date -u +%F), $(md5sum "$BB" | cut -c1-8), core $PIN): 1M nodes, 10M directed edge slots (5M pairs both ways), folds $ok

| row | store (bebop) | sqlite $(python3 -c 'import sqlite3;print(sqlite3.sqlite_version)') | sqlite / store |
|---|---|---|---|
| build (ms) | $build | $(g build_ms) | $(python3 -c "print(f'{$(g build_ms)/max($build,1):.1f}x')") |
| BFS, ns per edge (bebop over $NSRCB sources, sqlite level-synchronous over $NSRC) | $bfs_t | $(g bfs_ns_per_edge) | $(python3 -c "print(f'{$(g bfs_ns_per_edge)/max($bfs_t,1):.1f}x')") |
| neighbours of v, ns per query (10^5) | $nbr_t | $(python3 -c "print($(g nbr_us)*1000)") | $(python3 -c "print(f'{$(g nbr_us)*1000/max($nbr_t,1):.1f}x')") |
| file size (bytes) | $size | $(g size) | |

- folds: bebop BFS/neighbour == python oracle == sqlite for NSRC=$NSRC sources; the $NSRCB-source bebop row is timing only
- stage 2 (edge log + L0/L1 rebuilds, tombstone deletes, compaction, frontier SpMSpV) pending"
echo "$out" > bench/vs_rust/RESULT-sgraph.md; echo "$out"; [ "$ok" = equal ] || { echo "G8 FAIL"; exit 1; }
