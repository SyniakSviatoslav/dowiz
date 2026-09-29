#!/usr/bin/env bash
# run_all.sh — third column of the golden gate: oracle == frozen (std_golden.sh).
# Prints "<gate> <oracle> <frozen> OK|SELF-FROZEN|MISMATCH|MISSING" per gate + a summary.
# RUST=1 additionally re-runs spectral_golden/generator (cargo) and diffs golden.txt byte-exact.
# 2026-09-06 (dev-loop parallelism): the oracles run J at a time on the little cores (they
# never touch the compiler, so they must not compete with the std shards on the A78s), and
# a run whose inputs (every oracle file + the frozen list) equal the last GREEN run's is
# REPLAYED from $RUN_ALL_MEMO (FORCE=1 reruns; RUST=1 never memoizes).
cd "$(dirname "$0")/../.." || exit 1
D=bench/oracles
FROZEN=$(grep -E '^gate [A-Za-z0-9_]+ -?[0-9]+ ' bench/vs_rust/std_golden.sh | awk '{print $2, $3}')
MEMO=${RUN_ALL_MEMO:-$HOME/.cache/bebop/run_all}; mkdir -p "$MEMO"
# W-BATGREEN (2026-09-29): the key now also covers the Rust oracles' inputs. money.py and ordfsm.py
# are thin wrappers around PRODUCTION Rust (bench/oracles/rust -> crates/dowiz-core), so a change to
# dowiz-core's money.rs could move their value while every .py stayed byte-identical -- and the memo
# would have replayed the old GREEN. A memo is only as honest as the inputs its key reads.
RS=bench/oracles/rust; CORE=../crates/dowiz-core
KEY=$({ cat "$D"/*.py; ls "$D"/*.self-frozen 2>/dev/null; echo "$FROZEN";
        find "$RS/src" "$RS/Cargo.toml" "$RS/Cargo.lock" "$CORE/src" "$CORE/Cargo.toml" -type f 2>/dev/null | LC_ALL=C sort | xargs cat 2>/dev/null; } | sha256sum | cut -c1-16)
if [ "${FORCE:-0}" != 1 ] && [ "${RUST:-0}" != 1 ] && [ -s "$MEMO/$KEY" ]; then
  sed '$ s/$/ (memo: inputs unchanged since the last GREEN run)/' "$MEMO/$KEY"; exit 0
fi
# W-BATGREEN (2026-09-29): BUILD the Rust oracles ONCE, here, outside the per-oracle timeout.
# MEASURED: the built money/ordfsm binaries run in 188/166 ms, yet under the battery both printed
# ERR -- the 300 s below was spent compiling dowiz-core --release (13 MB rmeta, jobs=2, and pinned
# to the LITTLE cores by the taskset further down) inside `cargo run`, every time any lane touched
# dowiz-core. The timeout bounds the ORACLE's computation; it was never meant to bound a compiler
# build, and a build that outlives it read as a wrong value. The build is unpinned (it is not an
# oracle), in the battery's slot, and LOUD: a failed build is named with its rc and first error.
BLOG=${BEBOP_TMP:-/tmp/opencode}/run_all.rust-build.log; mkdir -p "$(dirname "$BLOG")"
if ls "$D"/*.py >/dev/null 2>&1 && grep -lq 'cargo' "$D"/*.py; then
  bs=$(date +%s)
  if (cd "$RS" && cargo build --release -q --bins) > "$BLOG" 2>&1; then
    echo "rust oracles: built ($(( $(date +%s) - bs )) s, bench/oracles/rust --release --bins)"
  else
    echo "RUST_ORACLE_BUILD FAILED rc=$? ($(( $(date +%s) - bs )) s): $(grep -m1 -E '^error' "$BLOG" || tail -n 1 "$BLOG") -- full log $BLOG"
  fi
fi
one() {  # one <gate> <frozen>
  local g=$1 f=$2 o e rc
  if [ -f "$D/$g.py" ]; then
    # stderr is KEPT (was 2>/dev/null): an ERR used to carry no reason at all, which is how a
    # compiler build hiding inside a timeout read as a wrong oracle value for days.
    e=$(mktemp); timeout 300 python3 "$D/$g.py" > "$e.out" 2> "$e"; rc=$?; o=$(tail -n 1 "$e.out")
    if [ "$o" = "$f" ]; then echo "$g $o $f OK"
    elif [ -n "$o" ]; then echo "$g $o $f MISMATCH"
    else  # the reason goes BEFORE the verdict word: SUMMARY counts lines ending in ' MISMATCH'
      local why; [ "$rc" = 124 ] && why=timeout-300s || why=$(tail -n 1 "$e" | tr -s ' \t' '__' | cut -c1-80)
      echo "$g ERR(rc=$rc:${why:-no-output}) $f MISMATCH"
    fi; rm -f "$e" "$e.out"
  elif [ -f "$D/$g.self-frozen" ]; then echo "$g - $f SELF-FROZEN"
  else echo "$g - $f MISSING"; fi
}
export -f one; export D
LITTLE=$(awk '/^processor/{p=$3} /CPU part/ && $NF=="0xd05"{print p}' /proc/cpuinfo | paste -sd,)
OUT=$(echo "$FROZEN" | ${LITTLE:+taskset -c $LITTLE} xargs -P "${J:-4}" -n 2 bash -c 'one "$@"' _; echo "XARGS_RC=$?")
XRC=$(sed -n 's/^XARGS_RC=//p' <<<"$OUT"); OUT=$(grep -v '^XARGS_RC=' <<<"$OUT" | sort)
echo "$OUT"
# W-BATGREEN (2026-09-29), MEASURED: procguard SIGKILLed one oracle at 33 procs, xargs stopped
# ("xargs: bash: terminated by signal 9") after 35 of the 124 frozen gates, and this script printed
# `SUMMARY ok=35 self-frozen=0 mismatch=0 missing=0`, exited 0 AND wrote that to the memo -- a
# GREEN over 28% of its inputs that every later run would have replayed. Every gate must report
# exactly once, or the run is NOT MEASURED: no SUMMARY a battery regex can read as green, rc 1,
# and no memo.
NF=$(grep -c . <<<"$FROZEN"); NR=$(grep -c . <<<"$OUT")
if [ "$NR" != "$NF" ] || [ "${XRC:-1}" != 0 ]; then
  echo "SUMMARY NOT MEASURED: $NR of $NF oracle gates reported (xargs rc=${XRC:-none}) -- a killed or truncated run, rerun it"
  exit 1
fi
OK=$(grep -c ' OK$' <<<"$OUT"); SF=$(grep -c ' SELF-FROZEN$' <<<"$OUT"); MM=$(grep -c ' MISMATCH$' <<<"$OUT"); MISS=$(grep -c ' MISSING$' <<<"$OUT")

RUST=""
if [ "${RUST:-0}" = 1 ]; then
  G=bench/vs_rust/spectral_golden; R=${BEBOP_TMP:-/tmp/opencode}/golden.regen.txt
  if (cd "$G/generator" && cargo run --release >"$R" 2>/dev/null) && cmp -s "$R" "$G/golden.txt"
  then RUST=" golden.txt=BYTE-EXACT"
  else RUST=" golden.txt=DIFF(only-in-golden=$(diff "$R" "$G/golden.txt" | grep -c '^>') only-in-generator=$(diff "$R" "$G/golden.txt" | grep -c '^<'))"; MM=$((MM+1)); fi
fi
SUM="SUMMARY ok=$OK self-frozen=$SF mismatch=$MM missing=$MISS$RUST"
echo "$SUM"
[ $((MM+MISS)) -eq 0 ] && [ -z "$RUST" ] && { echo "$OUT"; echo "$SUM"; } > "$MEMO/$KEY"
[ $((MM+MISS)) -eq 0 ]
