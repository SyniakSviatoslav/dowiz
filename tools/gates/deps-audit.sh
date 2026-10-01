#!/bin/sh
# DEPS-AUDIT -- no known high/critical advisory in any lockfile we ship.
#
# WHY THIS EXISTS. `git push` printed "GitHub found 6 vulnerabilities" for days and nothing on the
# box could list them: `gh` is absent, there is no API token, cargo-audit is not installed and
# building it is a heavy job on a 32-process box. W-DEPS (2026-09-30) reproduced the list from the
# lockfiles themselves; this gate keeps it reproduced.
#
# WHAT IT RUNS (both offline-first, both plain python, no cargo, no npm):
#   tools/deps/rustsec.py   every Cargo.lock vs a git checkout of rustsec/advisory-db
#                           ($RUSTSEC_DB, default ~/.cache/dowiz-deps/advisory-db; refreshed by
#                           tools/deps/refresh.sh). DB missing, < 500 advisories, or last commit
#                           older than $DEPS_DB_MAX_AGE_DAYS (default 7) -> REFUSED.
#   tools/deps/npm_audit.py every package-lock.json vs the npm registry's bulk advisory endpoint,
#                           cached per lock set ($NPM_ADVISORY_CACHE); a cached answer older than
#                           2 days is refetched, and an unreachable registry with no fresh cache
#                           -> REFUSED. DEPS_OFFLINE=1 forbids the fetch.
#
# RUSTSEC SEVERITY is computed from the advisory's CVSS 3.x vector; a CVSS 4.0 vector is not scored
# and counts as HIGH (fail closed) -- that is why four wasmtime advisories read "high" here.
#
# A GATE THAT CANNOT MEASURE NEVER PASSES: exit 0 GREEN, 1 RED (a high/critical finding), 2 REFUSED
# (no data, stale data, unparsed range, expired or unused ignore). Both checkers carry a --self-test
# and were proved on planted versions (once_cell 1.0.0 / zerocopy 0.7.30 in a Cargo.lock copy,
# minimist 1.2.5 in a package-lock copy -> reported; the patched boundary -> not reported).
#
# DATED IGNORES live in tools/deps/audit-ignore.txt (RustSec only): an entry past its date, or
# matching nothing, REFUSES the gate.
set -u
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT" || exit 2
off=""; [ "${DEPS_OFFLINE:-0}" = 1 ] && off="--offline"
rc=0
python3 tools/deps/rustsec.py --self-test >/dev/null || { echo "deps-audit: REFUSED: rustsec self-test failed"; exit 2; }
python3 tools/deps/npm_audit.py --self-test >/dev/null || { echo "deps-audit: REFUSED: npm_audit self-test failed"; exit 2; }
python3 tools/deps/rustsec.py --max-age-days "${DEPS_DB_MAX_AGE_DAYS:-7}" --ignore tools/deps/audit-ignore.txt --fail-on high .
r=$?; [ $r -gt $rc ] && rc=$r
python3 tools/deps/npm_audit.py $off --fail-on high .
r=$?; [ $r -gt $rc ] && rc=$r
case $rc in
  0) echo "deps-audit: GREEN (no unignored high/critical advisory in any Cargo.lock or package-lock.json)";;
  1) echo "deps-audit: RED (high/critical advisory above; bump it or add a DATED line to tools/deps/audit-ignore.txt)";;
  *) echo "deps-audit: REFUSED (the audit could not measure; see above -- tools/deps/refresh.sh)"; rc=2;;
esac
exit $rc
