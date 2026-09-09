#!/usr/bin/env bash
# C2 refutation probe driver (ROADMAP C2, blueprint §5 step 1 / §7). Measures BOTH halves
# of the C2 gate in ONE pinned run, arms interleaved, folds cross-checked:
#   size  -- 1M three-field records as OBJECTS vs as COLUMNS (arena_used*8), against a
#            sqlite twin of the same 1M three-field rows AND of the sbench 4-column+index
#            table (which is where the roadmap's "34.1 MB" actually comes from).
#   point -- 10^6 PK point lookups per arm at k=1 and k=3 fields read, median of REPS
#            interleaved reps on one pinned core, plus today's sbench PK-lookup row
#            re-derived on this box (the roadmap's "450 ns" is the number under test).
# PROBE ONLY: no live-path switch, no std_golden gate row, no codegen change.
# env: BEBOP_BIN, BEBOP_TMP, REPS (default 3), PIN (default 4).
set -u
cd "$(dirname "$0")/../.."
ulimit -s 65536 2>/dev/null
T=${BEBOP_TMP:-/tmp/opencode}; mkdir -p "$T"; BB=${BEBOP_BIN:-./bebop.bin}; REPS=${REPS:-3}
# PIN: this script wants ONE cpu number for `taskset -c`, but tools/slot.sh exports the SAME
# name as a COMMAND PREFIX ("taskset -c 4,5,6") for the shell benches that run `$PIN ./cmd`.
# Under a slot the bare form became `taskset -c "taskset -c 4,5,6"` -> "failed to parse CPU
# list" on stderr, an EMPTY stdout, and a full run of blank medians that reads as a measurement
# rather than a harness fault. That is the collision tools/perf.py:30-39 already carries a fix
# for (silent since slot.sh landed 2026-09-08); accept either form and keep the first core.
PIN=${PIN:-4}; case "$PIN" in *-c*) PIN=$(printf %s "$PIN" | sed -E 's/.*-c[[:space:]]+([0-9,-]+).*/\1/'); esac
PIN=${PIN%%,*}; PIN=${PIN%%-*}; [ -n "$PIN" ] || PIN=4
./seed/build/seed "$BB" compile bench/vs_rust/std_tests/scolp.bp "$T/scolp.bin" >/dev/null 2>&1 || { echo "COMPILEFAIL scolp"; exit 1; }
./seed/build/seed "$BB" compile bench/vs_rust/std_tests/sbench.bp "$T/sbench.bin" >/dev/null 2>&1 || { echo "COMPILEFAIL sbench"; exit 1; }
# L12: a run that prints NOTHING is a harness fault (bad PIN, missing .bin, a trap), never a
# measurement -- say so instead of feeding an empty string into the medians.
r() { local out; out=$(taskset -c "$PIN" ./seed/build/seed "$1" "$2" "$3" | tail -1)
      [ -n "$out" ] || { echo "GUARD: $(basename "$1") $2 $3 produced no output (PIN=$PIN)" >&2; exit 1; }
      printf %s "$out"; }
p() { r "$T/scolp.bin" "$1" "$2"; }
s() { r "$T/sbench.bin" "$1" "$2"; }
med() { printf '%s\n' "$@" | sort -n | awk '{a[NR]=$1} END{print a[int((NR+1)/2)]}'; }
rm -f scolp_o.store scolp_c.store scolp_o.store.tmp scolp_c.store.tmp sbench.store sbench.store.tmp
echo "== build (arms hold identical data: row i = (fu(i),fv(i),fw(i)) in both) =="
echo "build_obj_ms $(nice -n 10 taskset -c "$PIN" ./seed/build/seed "$T/scolp.bin" o t | tail -1)"
echo "build_col_ms $(nice -n 10 taskset -c "$PIN" ./seed/build/seed "$T/scolp.bin" c t | tail -1)"
echo "sbench_insert_ms $(nice -n 10 taskset -c "$PIN" ./seed/build/seed "$T/sbench.bin" insert t | tail -1)"
echo "== size (bytes, arena_used*8; fresh build = already compacted, nothing superseded) =="
OZ=$(p z f); CZ=$(p Z f); SZ=$(s z f)
echo "obj_bytes $OZ"; echo "col_bytes $CZ"; echo "sbench_obj_bytes $SZ"
echo "obj_blocks $(( $(stat -c %b scolp_o.store) * 512 )) col_blocks $(( $(stat -c %b scolp_c.store) * 512 ))"
echo "== folds (must be equal arm to arm) =="
echo "fold_k1 obj=$(p a f) col=$(p A f)"
echo "fold_k2 obj=$(p e f) col=$(p E f)"
echo "fold_k3 obj=$(p b f) col=$(p B f)"
echo "== point lookup, ns per op, 10^6 ops, interleaved x$REPS on core $PIN =="
A1=(); B1=(); A2=(); B2=(); A3=(); B3=(); SL=()
for r in $(seq 1 "$REPS"); do
  A1+=("$(p a t)"); B1+=("$(p A t)"); A2+=("$(p e t)"); B2+=("$(p E t)"); A3+=("$(p b t)"); B3+=("$(p B t)"); SL+=("$(s lookup t)")
done
echo "obj_k1_ns  reps=${A1[*]} median=$(med "${A1[@]}")"
echo "col_k1_ns  reps=${B1[*]} median=$(med "${B1[@]}")"
echo "obj_k2_ns  reps=${A2[*]} median=$(med "${A2[@]}")"
echo "col_k2_ns  reps=${B2[*]} median=$(med "${B2[@]}")"
echo "obj_k3_ns  reps=${A3[*]} median=$(med "${A3[@]}")"
echo "col_k3_ns  reps=${B3[*]} median=$(med "${B3[@]}")"
echo "sbench_pk_ns reps=${SL[*]} median=$(med "${SL[@]}")  (roadmap says 450)"
echo "sbench_pk_fold $(s lookup f)"
echo "== sqlite twins (same 1M rows; journal_mode=OFF so no -wal/-journal counted) =="
taskset -c "$PIN" python3 - "$T" <<'PY'
import os, sqlite3, sys
M = (1 << 64) - 1
def lcg(x): return (x * 6364136223846793005 + 1442695040888963407) & M
def fu(i): return ((lcg((i * 2654435761) & M) >> 33) - (1 << 30))
def fv(i): return ((lcg((i * 40503 + 7) & M) >> 33) - (1 << 30))
def fw(i): return ((fu(i) + (1 << 30)) >> 21) * 1024 + ((fv(i) + (1 << 30)) >> 21)
T = sys.argv[1]
rows = [(fu(i), fv(i), fw(i)) for i in range(1000000)]
def mk(path, ddl, ins, data, extra=None):
    for f in (path, path + '-journal', path + '-wal'):
        if os.path.exists(f): os.remove(f)
    c = sqlite3.connect(path); c.execute('PRAGMA journal_mode=OFF'); c.execute('PRAGMA synchronous=OFF')
    c.execute(ddl); c.executemany(ins, data)
    if extra: c.execute(extra)
    c.commit(); c.execute('VACUUM'); c.close(); return os.path.getsize(path)
a = mk(T + '/scolp_3col.sqlite', 'CREATE TABLE r(a INTEGER, b INTEGER, c INTEGER)', 'INSERT INTO r VALUES(?,?,?)', rows)
b = mk(T + '/scolp_sbench.sqlite', 'CREATE TABLE p(id INTEGER PRIMARY KEY, u INTEGER, v INTEGER, cell INTEGER)',
       'INSERT INTO p VALUES(?,?,?,?)', [(i, u, v, w) for i, (u, v, w) in enumerate(rows)], 'CREATE INDEX ic ON p(cell)')
print('sqlite_3field_bytes', a)
print('sqlite_sbench_shape_bytes', b)
# independent oracle for the probe folds
x = 777; f1 = 0; f2 = 0; f3 = 0
for _ in range(1000000):
    x = lcg(x); i = (x >> 20) % 1000000
    u, v, w = rows[i]; f1 += u; f2 += u + v; f3 += u + v + w
def s64(z): z &= M; return z - (1 << 64) if z >> 63 else z
print('oracle_fold_k1', s64(f1)); print('oracle_fold_k2', s64(f2)); print('oracle_fold_k3', s64(f3))
PY
echo "== done =="
