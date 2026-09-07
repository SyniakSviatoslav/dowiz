#!/usr/bin/env bash
# G7 (T116): the store vs sqlite 3 (C API, ctypes, prepared statements) on the T100
# workload: insert 1M, PK lookup 10^5, cell-window scan 10^4, update 10^5 (one
# transaction), reopen, file size, compaction. Folds cross-checked between the two
# engines after every phase; every number is in-process wall on one pinned A78.
# env: BEBOP_BIN, BEBOP_TMP. Writes bench/vs_rust/RESULT-sbench.md.
set -u
cd "$(dirname "$0")/../.."
ulimit -s 65536 2>/dev/null
T=${BEBOP_TMP:-/tmp/opencode}; BB=${BEBOP_BIN:-./bebop.bin}
BIG=$(awk '/^processor/{p=$3} /CPU part/ && $NF=="0xd41"{print p}' /proc/cpuinfo | tr '\n' ' ')
PIN=$(python3 -c "import os;u=sorted(os.sched_getaffinity(0));b=[int(x) for x in '$BIG'.split()];print(next((c for c in b if c in u),u[0]))")
./seed/build/seed "$BB" compile bench/vs_rust/std_tests/sbench.bp "$T/sbench.bin" >/dev/null 2>&1 || { echo "COMPILEFAIL sbench"; exit 1; }
rm -f sbench.store sbench.store.tmp
bb() { taskset -c "$PIN" ./seed/build/seed "$T/sbench.bin" "$1" "$2" | tail -1; }
bbb() { taskset -c "$PIN" ./seed/build/seed "$T/sbench.bin" B t "$1" | tail -1; }  # B1: st_commit_batch, group of $1
sq() { taskset -c "$PIN" python3 bench/tq_sqlite/sbench_sqlite.py "$1"; }
declare -A B S
B[insert]=$(bb insert t); B[lookup_f]=$(bb lookup f); B[lookup]=$(bb lookup t); B[scan_f]=$(bb scan f); B[scan]=$(bb scan t)
B[update]=$(bb update t); B[lookup2_f]=$(bb lookup f); B[reopen]=$(bb reopen t); B[size1]=$(bb z f)
B[compact]=$(bb compact t); B[lookup3_f]=$(bb lookup f); B[size2]=$(bb z f); B[blocks2]=$(( $(stat -c %b sbench.store) * 512 )); B[durable]=$(bb y t)
B[dbatch10]=$(bbb 10); B[dbatch100]=$(bbb 100)
# B1 blueprint section 3 `recover` row: zero the LIVE generation's own exclusive arena tail
# (never the fallback generation's, since we bound it to [other_used, live_used)) in a copy
# of sbench.store, forcing st_open -> st_reopen_verify's real self-heal-and-fallback path
# (store.bp:131), then time N independent reopens from a FRESH copy each trial (reopening
# the SAME corrupted file repeatedly would see it already self-healed after trial 1), median
# wall time in us -- restores the pristine store afterward.
B[recover]=$(python3 - "$T" "$PIN" <<'PY'
import os, struct, subprocess, sys, time, zlib
T, PIN = sys.argv[1], sys.argv[2]
STORE = 'sbench.store'
d = open(STORE, 'rb').read()
MAGIC = 3554557610294396226
def cells(off, n): return list(struct.unpack('<%dq' % n, d[off * 8:(off + n) * 8]))
def valid(sb):
    c = cells(sb, 16); return c[0] == MAGIC and (c[15] & ((1 << 32) - 1)) == zlib.crc32(d[sb * 8:(sb + 15) * 8])
sbs = [sb for sb in (0, 512) if valid(sb)]
assert sbs, 'sbench.store has no valid superblock'
live = max(sbs, key=lambda s: cells(s, 16)[2])
other = [s for s in (0, 512) if s != live][0]
live_used = cells(live, 16)[4]
other_used = cells(other, 16)[4] if valid(other) else 1024
lo, hi = min(other_used, live_used), live_used
img = bytearray(d)
img[lo * 8:hi * 8] = b'\x00' * ((hi - lo) * 8)
times = []
for _ in range(11):
    open(STORE, 'wb').write(img)
    t0 = time.perf_counter()
    r = subprocess.run(['taskset', '-c', PIN, './seed/build/seed', T + '/sbench.bin', 'x', 't'], capture_output=True, text=True)
    times.append((time.perf_counter() - t0) * 1e6)
    assert r.returncode == 0, r.stderr
open(STORE, 'wb').write(d)
times.sort()
print(round(times[len(times) // 2], 1))
PY
)
read -r _ S[insert] _ <<<"$(sq insert)"; read -r _ S[lookup] S[lookup_f] <<<"$(sq lookup)"; read -r _ S[scan] S[scan_f] <<<"$(sq scan)"
read -r _ S[update] _ <<<"$(sq update)"; read -r _ S[lookup2] S[lookup2_f] <<<"$(sq lookup)"; read -r _ S[reopen] _ <<<"$(sq reopen)"
read -r _ S[size1] _ <<<"$(sq size)"; read -r _ S[compact] _ <<<"$(sq compact)"; read -r _ S[size2] _ <<<"$(sq size)"; read -r _ S[durable] _ <<<"$(sq durable)"; read -r _ S[durable_full] _ <<<"$(sq durable_full)"
read -r _ S[dbatch10] _ <<<"$(sq durable_batch10)"; read -r _ S[dbatch100] _ <<<"$(sq durable_batch100)"; read -r _ S[recover] _ <<<"$(sq recover)"
folds=$([ "${B[lookup_f]}" = "${S[lookup_f]}" ] && [ "${B[scan_f]}" = "${S[scan_f]}" ] && [ "${B[lookup2_f]}" = "${S[lookup2_f]}" ] && [ "${B[lookup3_f]}" = "${S[lookup2_f]}" ] && echo equal || echo "MISMATCH bebop ${B[lookup_f]}/${B[scan_f]}/${B[lookup2_f]}/${B[lookup3_f]} sqlite ${S[lookup_f]}/${S[scan_f]}/${S[lookup2_f]}")
r() { python3 -c "print(f'{$2/max($1,0.001):.1f}x')"; }
out="# G7 sbench ($(date -u +%F), $(md5sum "$BB" | cut -c1-8), core $PIN): store vs sqlite $(python3 -c 'import sqlite3;print(sqlite3.sqlite_version)') C API, folds $folds

| phase | store | sqlite | sqlite / store |
|---|---|---|---|
| insert 1M + index + commit (ms) | ${B[insert]} | ${S[insert]} | $(r ${B[insert]} ${S[insert]}) |
| PK lookup, ns each (10^5) | ${B[lookup]} | ${S[lookup]} | $(r ${B[lookup]} ${S[lookup]}) |
| 3x3 cell-window scan, ns each (10^4) | ${B[scan]} | ${S[scan]} | $(r ${B[scan]} ${S[scan]}) |
| update 10^5 in one transaction (ms) | ${B[update]} | ${S[update]} | $(r ${B[update]} ${S[update]}) |
| reopen + first record, us (100x) | ${B[reopen]} | ${S[reopen]} | $(r ${B[reopen]} ${S[reopen]}) |
| logical size after update, arena_used*8 (bytes) | ${B[size1]} | ${S[size1]} | $(r ${B[size1]} ${S[size1]}) |
| compaction / VACUUM (ms) | ${B[compact]} | ${S[compact]} | $(r ${B[compact]} ${S[compact]}) |
| logical size after compaction (bytes; file blocks allocated ${B[blocks2]}) | ${B[size2]} | ${S[size2]} | $(r ${B[size2]} ${S[size2]}) |
| durable commit, us each (1000 x one record version; store = msync of the appended pages + the superblock pages; sqlite = WAL synchronous=NORMAL, no fsync per commit) | ${B[durable]} | ${S[durable]} | $(r ${B[durable]} ${S[durable]}) |
| durable commit vs sqlite WAL synchronous=FULL (fsync per commit) | ${B[durable]} | ${S[durable_full]} | $(r ${B[durable]} ${S[durable_full]}) |
| durable batch 10, us per commit (B1: st_commit_batch, one msync per 10 commits; sqlite = 10 UPDATEs per COMMIT, synchronous=FULL) | ${B[dbatch10]} | ${S[dbatch10]} | $(r ${B[dbatch10]} ${S[dbatch10]}) |
| durable batch 100, us per commit (same, 100 commits per msync/COMMIT) | ${B[dbatch100]} | ${S[dbatch100]} | $(r ${B[dbatch100]} ${S[dbatch100]}) |
| recover, us (median of 11; reopen after a torn image -- store = st_open/st_reopen_verify self-heal past a zeroed live-generation tail; sqlite = open with a non-empty -wal after no checkpoint) | ${B[recover]} | ${S[recover]} | $(r ${B[recover]} ${S[recover]}) |

- pass rule (docs/LANG-DB-DESIGN.md §5 G7): PK lookup >= 3x sqlite native AND scan >= 5x; the file size is reported whatever it is (expected ~2.2x loss before compaction)
- store rows are in-process clock_ms deltas (insert/update/compact exclude the ~100 ms process floor and the open); sqlite rows go through ctypes (4 calls per lookup, ~8 us of ctypes per op, T100 measured ~19 us for the window query) inside one transaction with locking_mode=EXCLUSIVE, so its per-op rows are an upper bound on native sqlite by roughly that floor
- the store file is preallocated (256 MB ftruncate); the logical size is what a size-aware open would map"
# E9 (D12-A): the G7 phases as perf.csv rows (store side; the sqlite twin stays in RESULT-sbench.md)
for m in insert lookup scan update reopen size1 compact size2 durable dbatch10 dbatch100 recover; do python3 tools/perf.py record --bin "$BB" "sbench_$m" "${B[$m]}" "$([ $m = lookup ] || [ $m = scan ] && echo ns || { [ $m = reopen ] || [ $m = durable ] || [ $m = dbatch10 ] || [ $m = dbatch100 ] || [ $m = recover ] && echo us || { [[ $m = size* ]] && echo bytes || echo ms; }; })" "sqlite ${S[$m]}"; done
echo "$out" > bench/vs_rust/RESULT-sbench.md; echo "$out"
[ "$folds" = equal ] || { echo "G7 FAIL: fold mismatch"; exit 1; }
