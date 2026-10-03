#!/bin/sh
# INTEGRATION-MATRIX -- EVERY LINK BETWEEN TWO SYSTEMS HAS A STATUS, "TESTED-INMEM" HAS A PROOF,
# AND "LIVE" IS A SEPARATE CLAIM THAT ONLY A LIVE PROBE CAN MAKE.
#
# Operator 2026-10-02: "з 58 зв'язків між системами лише 7 з'єднані й доведені а мають бути
# усі з'єднані та доведені, це пріоритет". The registry is tools/integrations/matrix.json, one
# entry per link, numbered as docs/research/2026-10-02-system-integration-check.md section 1
# (1..58; later rows continue the numbering, 59.. by lane W-LIVE).
#
# Operator 2026-10-03: "не просто мокати дані, а перевіряти усе на живому сайті з реальними
# підключеннями, детальними контрактами та схемами і версіями й описом". So a row carries TWO
# claims, counted and ratcheted apart:
#   status TESTED-INMEM  a check in this repo (in-memory route test, node test, gate) drives the link;
#   live                 null, or {kind: "probe", ref: <read-only live probe script>, contract: <the
#                        row's contract/schema/version file>, ...}. An in-memory test is NEVER live.
#
# WHAT IT REFUSES (exit 1, each named):
#   * ids that are not exactly 1..N (N >= 58), a duplicate, a status outside the six;
#   * a TESTED-INMEM row with no proof, or whose proof does not RESOLVE:
#       cargo-test  `module::path::test_name` -- a `fn test_name(` under a `#[test]` in
#                   workers/api/src or crates/** (a name only in a comment does not count);
#       node-test   the file exists and is a `*.test.mjs`;
#       gate        the script exists under tools/gates;
#       probe       the script exists (read-only live probes, only where nothing else can);
#     the same for every `also` entry (`node:<path>` is a node test, otherwise a cargo test) and
#     for a proof given on a row that is not TESTED-INMEM -- a stale reference is stale whatever the status;
#   * a row without a `live` key; a `live` that is not null and not a probe (cargo-test, node-test and
#     gate are refused BY NAME: they run against a mock, not the live site), or whose probe script or
#     contract file does not exist;
#   * fewer TESTED-INMEM rows than `tested_inmem=<n>`, or fewer live rows than `live=<n>`, in the
#     baseline (both may only rise).
#
# WHAT IT DOES NOT DO: run cargo or node. run-all must stay fast, and the proofs run in their own
# suites (`cargo test --lib` in workers/api, the node tests). Existence is not green, so there is a
# second mode that READS a test run instead of trusting one:
#   INTEGRATION_MATRIX_RESULTS=<cargo test output> sh tools/gates/integration-matrix.sh
# refuses every TESTED-INMEM cargo-test proof that the log does not show as `test <ref> ... ok` --
# missing, FAILED and ignored alike -- and prints how many it saw green.
set -eu
cd "$(dirname "$0")/../.."
MATRIX=${INTEGRATION_MATRIX:-tools/integrations/matrix.json}
BASELINE=${INTEGRATION_MATRIX_BASELINE:-tools/gates/integration-matrix.baseline}
SRC=${INTEGRATION_MATRIX_SRC:-workers/api/src crates}
RESULTS=${INTEGRATION_MATRIX_RESULTS:-}

[ -f "$MATRIX" ] || { echo "integration-matrix: no registry at $MATRIX"; exit 2; }
base=$(sed -n 's/^tested_inmem=\([0-9][0-9]*\)$/\1/p' "$BASELINE" 2>/dev/null | head -1)
lbase=$(sed -n 's/^live=\([0-9][0-9]*\)$/\1/p' "$BASELINE" 2>/dev/null | head -1)
[ -n "$base" ] && [ -n "$lbase" ] || { echo "integration-matrix: no 'tested_inmem=<n>' and 'live=<n>' in $BASELINE"; exit 2; }

set +e
python3 - "$MATRIX" "$base" "$lbase" "$RESULTS" $SRC <<'PY'
import json, os, re, sys
matrix, base, lbase, results, src = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), sys.argv[4], sys.argv[5:]
STATUSES = ["TESTED-INMEM", "CONNECTED_UNPROVEN", "DISCONNECTED", "OFF_BY_FLAG", "NEEDS_SECRET", "BROKEN"]
KINDS = ["cargo-test", "node-test", "gate", "probe"]
N = 58
bad = []
try:
    links = json.load(open(matrix, encoding="utf-8"))["links"]
except Exception as e:
    print(f"integration-matrix: {matrix} is not a registry: {e}")
    sys.exit(1)

# Every #[test] function name in the source roots, comments stripped first (a gate that counts
# the comment recording a deletion measures its own epitaph).
tests = set()
for root in src:
    for d, _, files in os.walk(root):
        if "/target" in d or "/node_modules" in d:
            continue
        for f in files:
            if not f.endswith(".rs"):
                continue
            try:
                lines = open(os.path.join(d, f), encoding="utf-8", errors="replace").read().split("\n")
            except OSError:
                continue
            code = [re.sub(r"//.*", "", l) for l in lines]
            for i, l in enumerate(code):
                m = re.match(r"\s*(?:pub(?:\([a-z]+\))?\s+)?(?:async\s+)?fn\s+([a-z0-9_]+)\s*\(", l)
                if not m:
                    continue
                j, attrs = i - 1, []
                while j >= 0 and (code[j].strip().startswith("#[") or not code[j].strip()):
                    attrs.append(code[j].strip())
                    j -= 1
                if any(a.startswith("#[test]") for a in attrs):
                    tests.add(m.group(1))

def resolve(kind, ref):
    if not isinstance(ref, str) or not ref.strip():
        return "empty ref"
    if kind == "cargo-test":
        name = ref.split("::")[-1]
        if "::" not in ref:
            return "a cargo-test ref is module::path::name"
        return None if name in tests else f"no #[test] fn {name} in {' '.join(src)}"
    if kind == "node-test":
        return None if ref.endswith(".test.mjs") and os.path.isfile(ref) else "no such node test file"
    if kind == "gate":
        return None if ref.startswith("tools/gates/") and os.path.isfile(ref) else "no such gate under tools/gates"
    if kind == "probe":
        return None if os.path.isfile(ref) else "no such probe script"
    return f"unknown proof kind {kind!r} (one of {', '.join(KINDS)})"

ids = [l.get("id") for l in links]
top = max([i for i in ids if isinstance(i, int)] + [N])
if sorted(i for i in ids if isinstance(i, int)) != list(range(1, top + 1)) or len(ids) != top:
    missing = sorted(set(range(1, top + 1)) - set(ids))
    dup = sorted({i for i in ids if ids.count(i) > 1})
    bad.append(f"ids are not exactly 1..{top} (at least 1..{N}; count {len(ids)}, missing {missing}, duplicated {dup})")
N = top
live = 0
count = {s: 0 for s in STATUSES}
proven_cargo = []
for l in links:
    i, st, proof = l.get("id"), l.get("status"), l.get("proof")
    for key in ("name", "producer", "consumer"):
        if not l.get(key):
            bad.append(f"#{i}: no {key}")
    if st not in STATUSES:
        bad.append(f"#{i}: status {st!r} is not one of {', '.join(STATUSES)}")
        continue
    count[st] += 1
    if "live" not in l:
        bad.append(f"#{i}: no `live` key (null until a live probe passes)")
    lv = l.get("live")
    if lv is not None:
        lk = lv.get("kind") if isinstance(lv, dict) else None
        if lk in ("cargo-test", "node-test", "gate"):
            bad.append(f"#{i} live is a {lk}: an in-memory/repo check is not a live check (only a probe on the live site is)")
        elif lk != "probe":
            bad.append(f"#{i} live {lv!r} is not null and not a probe")
        else:
            why = resolve("probe", lv.get("ref"))
            c = lv.get("contract")
            if why:
                bad.append(f"#{i} live probe {lv.get('ref')}: {why}")
            elif not (isinstance(c, str) and os.path.isfile(c)):
                bad.append(f"#{i} live probe has no contract file ({c!r}): a live row names its contract/schema/version")
            else:
                live += 1
    if st == "TESTED-INMEM" and not proof:
        bad.append(f"#{i} TESTED-INMEM with no proof")
        continue
    if proof:
        why = resolve(proof.get("kind"), proof.get("ref"))
        if why:
            bad.append(f"#{i} {st} proof {proof.get('kind')} {proof.get('ref')}: {why}")
        elif st == "TESTED-INMEM" and proof.get("kind") == "cargo-test":
            proven_cargo.append((i, proof["ref"]))
    for a in l.get("also") or []:
        kind, ref = ("node-test", a[5:]) if a.startswith("node:") else ("cargo-test", a)
        why = resolve(kind, ref)
        if why:
            bad.append(f"#{i} also {a}: {why}")

seen = ""
if results:
    try:
        log = open(results, encoding="utf-8", errors="replace").read()
    except OSError as e:
        bad.append(f"results {results}: {e}")
        log = None
    if log is not None:
        green = 0
        for i, ref in proven_cargo:
            if re.search(r"^test " + re.escape(ref) + r" \.\.\. ok$", log, re.M):
                green += 1
            else:
                bad.append(f"#{i} TESTED-INMEM but {ref} is not 'ok' in {results}")
        seen = f"; seen green {green} of {len(proven_cargo)} cargo proofs"

for b in bad:
    print("REFUSED " + b)
line = (f"tested-inmem={count['TESTED-INMEM']} of {N}; live={live} of {N}; connected-unproven={count['CONNECTED_UNPROVEN']}; "
        f"disconnected={count['DISCONNECTED']}; off-by-flag={count['OFF_BY_FLAG']}; "
        f"needs-secret={count['NEEDS_SECRET']}; broken={count['BROKEN']} (baseline tested_inmem={base} live={lbase}{seen})")
if bad:
    print(f"integration-matrix: {len(bad)} refusal(s); {line}")
    sys.exit(1)
if count["TESTED-INMEM"] < base:
    print(f"integration-matrix: tested-inmem fell below the baseline; {line}")
    sys.exit(1)
if live < lbase:
    print(f"integration-matrix: live fell below the baseline; {line}")
    sys.exit(1)
print(f"integration-matrix: {line}")
PY
rc=$?
exit $rc
