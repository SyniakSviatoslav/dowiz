#!/bin/sh
# integration-matrix's proof: the gate is heard before it is trusted. Every case runs on a
# SCRATCH copy of the registry and baseline; the tree is never touched.
#   1. the registry as it is                                     -> pass (0)
#   2. a TESTED-INMEM row whose test was removed (ref renamed)    -> refuse (1)
#   3. the tested_inmem baseline above the count                  -> refuse (1)
#   4. a TESTED-INMEM row with its proof taken away               -> refuse (1)
#   5. 57 rows                                                    -> refuse (1)
#   6. a status outside the six                                   -> refuse (1)
#   7. a test named only in a comment                             -> refuse (1)
#   8. results mode, a log that shows every proof ok              -> pass (0)
#   9. results mode, the same log with one proof FAILED           -> refuse (1)
#  10. `live` set to the row's own in-memory cargo test           -> refuse (1)  (a mock is not live)
#  11. `live` a probe with no contract file                       -> refuse (1)
#  12. `live` a probe with its contract                           -> pass (0), live=1
#  13. the live baseline above the live count                     -> refuse (1)
#  14. a row without the `live` key                               -> refuse (1)
#  15. row 59 appended (W-LIVE's rows continue the numbering)     -> pass (0)
#  16. row 60 appended with 59 missing                            -> refuse (1)
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
S=$(mktemp -d)
trap 'rm -rf "$S"' EXIT
fail=0
cp "$REPO/tools/integrations/matrix.json" "$S/m.json"
cp "$REPO/tools/gates/integration-matrix.baseline" "$S/base"

want() { # want <rc> <label> [env assignments...]
  rc_want=$1; label=$2; shift 2
  env INTEGRATION_MATRIX="$S/m.json" INTEGRATION_MATRIX_BASELINE="$S/base" "$@" sh "$HERE/integration-matrix.sh" >"$S/out" 2>&1
  rc=$?
  echo "prove: $label -> rc=$rc (want $rc_want): $(grep -m1 '^REFUSED' "$S/out" || tail -1 "$S/out")"
  [ "$rc" -eq "$rc_want" ] || { cat "$S/out"; fail=1; }
  cp "$S/out" "$S/out.last"
  cp "$REPO/tools/integrations/matrix.json" "$S/m.json"
  cp "$REPO/tools/gates/integration-matrix.baseline" "$S/base"
}
edit() { # edit <python statement over the list `L`>
  python3 - "$S/m.json" "$1" <<'PY'
import json, sys
p, stmt = sys.argv[1], sys.argv[2]
d = json.load(open(p))
L = d["links"]
first = next(l for l in L if l["status"] == "TESTED-INMEM" and l["proof"]["kind"] == "cargo-test")
exec(stmt)
json.dump(d, open(p, "w"))
PY
}

want 0 "the registry as it is"
edit 'first["proof"]["ref"] = first["proof"]["ref"].rsplit("::", 1)[0] + "::nobody_wrote_this_test"'
want 1 "a TESTED-INMEM row whose test is gone"
n=$(sed -n 's/^tested_inmem=//p' "$S/base"); printf 'tested_inmem=%s\nlive=0\n' "$((n + 1))" > "$S/base"
want 1 "the tested_inmem baseline above the count"
edit 'first["proof"] = None'
want 1 "a TESTED-INMEM row with no proof"
edit 'L.pop()'
want 1 "57 rows"
edit 'L[0]["status"] = "WORKS_PROBABLY"'
want 1 "a status outside the six"
mkdir -p "$S/src"
printf '%s\n' '// #[test]' '// fn only_in_a_comment_test() {}' > "$S/src/c.rs"
edit 'first["proof"]["ref"] = "x::only_in_a_comment_test"'
want 1 "a test named only in a comment" INTEGRATION_MATRIX_SRC="$REPO/workers/api/src $REPO/crates $S/src"
python3 - "$S/m.json" "$S/log.ok" "$S/log.bad" <<'PY'
import json, sys
L = json.load(open(sys.argv[1]))["links"]
refs = [l["proof"]["ref"] for l in L if l["status"] == "TESTED-INMEM" and l["proof"]["kind"] == "cargo-test"]
open(sys.argv[2], "w").write("".join(f"test {r} ... ok\n" for r in refs))
open(sys.argv[3], "w").write("".join(f"test {r} ... {'FAILED' if k == 0 else 'ok'}\n" for k, r in enumerate(refs)))
PY
want 0 "results: every proof ok" INTEGRATION_MATRIX_RESULTS="$S/log.ok"
want 1 "results: one proof FAILED" INTEGRATION_MATRIX_RESULTS="$S/log.bad"
edit 'first["live"] = dict(first["proof"])'
want 1 "live = the row's in-memory test"
printf '#!/bin/sh\n# read-only probe (scratch)\n' > "$S/probe.sh"; printf '{"version": 1}\n' > "$S/contract.json"
edit "first['live'] = {'kind': 'probe', 'ref': '$S/probe.sh', 'contract': '$S/nope.json'}"
want 1 "live probe with no contract"
edit "first['live'] = {'kind': 'probe', 'ref': '$S/probe.sh', 'contract': '$S/contract.json'}"
want 0 "live probe with its contract"
grep -q 'live=1 of' "$S/out.last" || { echo "prove: a valid live probe was not counted"; fail=1; }
m=$(sed -n 's/^tested_inmem=//p' "$S/base"); printf 'tested_inmem=%s\nlive=1\n' "$m" > "$S/base"
want 1 "the live baseline above the live count"
edit 'del first["live"]'
want 1 "a row without the live key"
edit 'L.append(dict(L[-1], id=59, status="CONNECTED_UNPROVEN", proof=None))'
want 0 "row 59 appended"
edit 'L.append(dict(L[-1], id=60, status="CONNECTED_UNPROVEN", proof=None))'
want 1 "row 60 with 59 missing"

[ "$fail" -eq 0 ] && echo "integration-matrix.prove: the gate refuses a missing proof, a lost row, a bad status, a comment, a red run and a mock passed off as live" || echo "integration-matrix.prove: FAILED"
exit "$fail"
