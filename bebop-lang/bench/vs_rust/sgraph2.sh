#!/usr/bin/env bash
# G8 (T117) stage 2: edge log + L0 rebuilds, tombstone deletes, compaction, BFS with
# tombstones — every fold against bench/oracles/sgraph2.py. env: BEBOP_BIN, BEBOP_TMP,
# NSRC (BFS sources for the fold, default 3). Appends to bench/vs_rust/RESULT-sgraph.md.
set -u
cd "$(dirname "$0")/../.."
ulimit -s 65536 2>/dev/null
T=${BEBOP_TMP:-/tmp/opencode}; BB=${BEBOP_BIN:-./bebop.bin}; NSRC=${NSRC:-3}
BIG=$(awk '/^processor/{p=$3} /CPU part/ && $NF=="0xd41"{print p}' /proc/cpuinfo | tr '\n' ' ')
PIN=$(python3 -c "import os;u=sorted(os.sched_getaffinity(0));b=[int(x) for x in '$BIG'.split()];print(next((c for c in b if c in u),u[0]))")
./seed/build/seed "$BB" compile bench/vs_rust/std_tests/sgraph2.bp "$T/sgraph2.bin" >/dev/null 2>&1 || { echo "COMPILEFAIL sgraph2"; exit 1; }
rm -f sgraph2.store sgraph2.store.tmp
bb() { taskset -c "$PIN" ./seed/build/seed "$T/sgraph2.bin" "$@" | tail -1; }
build=$(bb b t); nbr0=$(bb n f); bfs0_f=$(bb f f "$NSRC"); bfs0_t=$(bb f t "$NSRC"); fro_f=$(bb p f "$NSRC"); fro_t=$(bb p t "$NSRC")
log_ns=0; stall=0
for b0 in 0 20 40 60 80; do v=$(bb l t "$b0" 20); log_ns=$(( log_ns + v / 1000 )); st=$(( v % 1000 )); [ "$st" -gt "$stall" ] && stall=$st; bb c t >/dev/null; done
log_ns=$(( log_ns / 5 )); nbrlog=$(bb n f); nbr_t=$(bb n t)
del_ms=$(bb d t); nbr1=$(bb n f); bfs_f=$(bb f f "$NSRC"); bfs_t=$(bb f t "$NSRC"); size1=$(bb z f)
comp_ms=$(bb c t); size2=$(bb z f); nbr2=$(bb n f); bfs2=$(bb f f "$NSRC")
rm -f sgraph2h.store; hb=$(bb bh t); hq_f=$(bb fh f "$NSRC"); hq_t=$(bb fh t "$NSRC"); hp_f=$(bb ph f "$NSRC"); hp_t=$(bb ph t "$NSRC"); hn_f=$(bb nh f); hn_t=$(bb nh t)
hora=$(taskset -c "$PIN" python3 bench/oracles/sgraph2.py "all$NSRC" h); ho0=$(awk '$1=="nbr0"{print $2}' <<<"$hora"); hob=$(awk '$1=="bfs"{print $2}' <<<"$hora")
ora=$(taskset -c "$PIN" python3 bench/oracles/sgraph2.py "all$NSRC"); o0=$(awk '$1=="nbr0"{print $2}' <<<"$ora"); olog=$(awk '$1=="nbrlog"{print $2}' <<<"$ora"); o1=$(awk '$1=="nbr"{print $2}' <<<"$ora"); ob=$(awk '$1=="bfs"{print $2}' <<<"$ora")
ok=$([ "$nbr0" = "$o0" ] && [ "$bfs0_f" = "$fro_f" ] && [ "$nbrlog" = "$olog" ] && [ "$nbr1" = "$o1" ] && [ "$nbr2" = "$o1" ] && [ "$bfs_f" = "$ob" ] && [ "$bfs2" = "$ob" ] && echo equal || echo "MISMATCH nbr0 $nbr0/$o0 nbrlog $nbrlog/$olog nbr $nbr1,$nbr2/$o1 bfs $bfs_f,$bfs2/$ob")
out="
## stage 2 ($(date -u +%F), $(md5sum "$BB" | cut -c1-8), core $PIN): edge log, tombstones, compaction — folds $ok

| row | value |
|---|---|
| build L1 + empty log/L0/bitmap (ms) | $build |
| BFS on L1 before the log, $NSRC sources: queue vs frontier SpMSpV (push/pull, alpha 14), ns per edge slot | $bfs0_t vs $fro_t (folds $([ "$bfs0_f" = "$fro_f" ] && echo equal || echo DIFFER)) |
| insert 1M edges through the log, 100 batches with an L0 rebuild each (compaction every 20): amortized ns per edge / max batch stall ms | $log_ns / $stall |
| neighbours of v after the log, ns per query (L1 slice + L0 slice + tombstone bits, 3 slices) | $nbr_t |
| tombstone 10% of the L1 slots (one bitmap version) + commit (ms) | $del_ms |
| BFS with tombstones + log, $NSRC sources, ns per edge slot | $bfs_t |
| logical size before / after compaction (bytes) | $size1 / $size2 |
| compaction (ms) | $comp_ms |
| 1%-hub skewed graph (sgraph2h.store): build ms / BFS queue vs frontier ns per slot / neighbours ns per query | $hb / $hq_t vs $hp_t / $hn_t (folds $([ "$hq_f" = "$hp_f" ] && [ "$hq_f" = "$hob" ] && [ "$hn_f" = "$ho0" ] && echo equal || echo DIFFER)) |
- folds: neighbours before/after the log and before/after compaction, BFS before/after compaction == python oracle
- the hub variant's oracle is the same array BFS with the skewed generator (the log/tombstone phases run on the uniform graph only)"
# E9 (D12-A): G8 stage-2 phases as perf.csv rows
python3 tools/perf.py record --bin "$BB" sgraph2_build_ms "$build" ms "L1 + empty log/L0/bitmap"
python3 tools/perf.py record --bin "$BB" sgraph2_bfs_ns "$bfs0_t" ns "per edge slot, queue BFS before the log ($NSRC sources)"
python3 tools/perf.py record --bin "$BB" sgraph2_log_ns "$log_ns" ns "amortized per inserted edge through the log (max stall $stall ms)"
python3 tools/perf.py record --bin "$BB" sgraph2_compact_ms "$comp_ms" ms ""
echo "$out" >> bench/vs_rust/RESULT-sgraph.md; echo "$out"; [ "$ok" = equal ] || { echo "G8 stage 2 FAIL"; exit 1; }
