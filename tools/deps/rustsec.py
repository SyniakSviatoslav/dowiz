#!/usr/bin/env python3
"""rustsec.py -- match every Cargo.lock in the tree against the RustSec advisory DB, offline.

cargo-audit is not installed on the dev box and building it is a heavy job, so this is the small
part of cargo-audit that the dependency gate needs: read each lock's registry packages, read each
advisory's [versions] patched/unaffected requirements, and report every (package, version) that is
covered by neither. Requirement syntax is Cargo's (the `semver` crate's VersionReq): a bare
`1.2.3` means `^1.2.3`, a comma is AND, and a pre-release version only matches a comparator
that names a pre-release on the same major.minor.patch.

    python3 tools/deps/rustsec.py [--db DIR] [--max-age-days N] [--fail-on high] [--json]
                                  [LOCK_OR_DIR ...]          # default: every Cargo.lock under .
    python3 tools/deps/rustsec.py --self-test

DB: --db, else $RUSTSEC_DB, else ~/.cache/dowiz-deps/advisory-db (tools/deps/refresh.sh clones
it). A DB that is missing, is not a git checkout, holds fewer than 500 advisories or whose last
commit is older than --max-age-days (default 7) is a REFUSAL (exit 2): a checker that cannot
measure never passes.

Severity comes from the advisory's CVSS 3.x vector (base score computed here). A CVSS 4.0 vector
is not scored by this file and counts as `high` (fail closed) -- the output says `cvss4` beside it.
An advisory with `informational = "unsound" | "unmaintained" | "notice"` is reported as `info`
and never fails the run. Withdrawn advisories are skipped.

--ignore FILE: lines `ID LOCK_DIR YYYY-MM-DD reason...` (# comments). A matching finding is printed
as IGNORED and does not fail the run. An entry past its date, malformed, or matching no finding is a
refusal -- an ignore list is a dated debt, not a mute button.

Exit: 0 no finding at or above --fail-on; 1 at least one; 2 refusal (DB, parse, no lock found).
"""
import argparse
import json
import math
import os
import re
import subprocess
import sys
import time
import tomllib

SEV_RANK = {"info": 0, "none": 0, "low": 1, "moderate": 2, "high": 3, "critical": 4}


def refuse(msg):
    print(f"rustsec: REFUSED: {msg}", file=sys.stderr)
    print(f"rustsec: REFUSED: {msg}")
    sys.exit(2)


# ---------------------------------------------------------------- semver (Cargo flavour)

VER_RE = re.compile(r"^\s*v?(\d+)(?:\.(\d+))?(?:\.(\d+))?(?:-([0-9A-Za-z.-]+))?(?:\+[0-9A-Za-z.-]+)?\s*$")


def parse_version(s):
    m = VER_RE.match(s)
    if not m or m.group(2) is None or m.group(3) is None:
        raise ValueError(f"not a full semver version: {s!r}")
    pre = tuple(m.group(4).split(".")) if m.group(4) else ()
    return (int(m.group(1)), int(m.group(2)), int(m.group(3)), pre)


def _pre_key(pre):
    # semver precedence: no pre-release > any pre-release; identifiers compared numerically
    # when numeric, numeric < alphanumeric, shorter prefix < longer.
    if not pre:
        return (1,)
    parts = []
    for ident in pre:
        parts.append((0, int(ident), "") if ident.isdigit() else (1, 0, ident))
    return (0, tuple(parts))


def vkey(v):
    return (v[0], v[1], v[2], _pre_key(v[3]))


def cmp(a, b):
    ka, kb = vkey(a), vkey(b)
    return (ka > kb) - (ka < kb)


COMP_RE = re.compile(r"^\s*(>=|<=|>|<|=|\^|~)?\s*(\d+)(?:\.(\d+|\*|x))?(?:\.(\d+|\*|x))?(?:-([0-9A-Za-z.-]+))?\s*$")


def parse_comparator(s):
    """-> list of (op, version) primitive bounds, plus the pre-release anchor (or None)."""
    s = s.strip()
    if s == "*":
        return [], None
    m = COMP_RE.match(s)
    if not m:
        raise ValueError(f"unparsed requirement {s!r}")
    op = m.group(1) or "^"
    maj = int(m.group(2))
    mi = m.group(3)
    pa = m.group(4)
    mi = None if mi in (None, "*", "x") else int(mi)
    pa = None if pa in (None, "*", "x") else int(pa)
    pre = tuple(m.group(5).split(".")) if m.group(5) else ()
    lo = (maj, mi or 0, pa or 0, pre)
    anchor = lo if pre else None
    if op == "=":
        if mi is None:
            return [(">=", (maj, 0, 0, ())), ("<", (maj + 1, 0, 0, ()))], anchor
        if pa is None:
            return [(">=", (maj, mi, 0, ())), ("<", (maj, mi + 1, 0, ()))], anchor
        return [("=", lo)], anchor
    if op in (">", "<=") and (mi is None or pa is None):
        # >1.2 means >=1.3.0 ; <=1.2 means <1.3.0
        up = (maj + 1, 0, 0, ()) if mi is None else (maj, mi + 1, 0, ())
        return [(">=" if op == ">" else "<", up)], anchor
    if op in (">", ">=", "<", "<="):
        return [(op, lo)], anchor
    if op == "~":
        up = (maj + 1, 0, 0, ()) if mi is None else (maj, mi + 1, 0, ())
        return [(">=", lo), ("<", up)], anchor
    # caret
    if maj > 0 or mi is None:
        up = (maj + 1, 0, 0, ())
    elif mi > 0 or pa is None:
        up = (0, mi + 1, 0, ())
    else:
        up = (0, 0, pa + 1, ())
    return [(">=", lo), ("<", up)], anchor


def req_matches(req, v):
    """Cargo VersionReq: comma-separated comparators, all must hold."""
    bounds, anchors = [], []
    for part in req.split(","):
        if not part.strip():
            raise ValueError(f"empty comparator in {req!r}")
        b, a = parse_comparator(part)
        bounds += b
        if a is not None:
            anchors.append(a)
    for op, bv in bounds:
        c = cmp(v, bv)
        ok = {"=": c == 0, ">": c > 0, ">=": c >= 0, "<": c < 0, "<=": c <= 0}[op]
        if not ok:
            return False
    if v[3]:
        # a pre-release only matches a requirement that names a pre-release of the same x.y.z
        return any(a[:3] == v[:3] for a in anchors)
    return True


# ---------------------------------------------------------------- CVSS 3.x base score

_W = {
    "AV": {"N": 0.85, "A": 0.62, "L": 0.55, "P": 0.2},
    "AC": {"L": 0.77, "H": 0.44},
    "UI": {"N": 0.85, "R": 0.62},
    "CIA": {"H": 0.56, "L": 0.22, "N": 0.0},
}


def _roundup(x):
    i = round(x * 100000)
    return i / 100000.0 if i % 10000 == 0 else (math.floor(i / 10000) + 1) / 10.0


def cvss3_score(vec):
    m = dict(p.split(":", 1) for p in vec.split("/")[1:])
    changed = m["S"] == "C"
    pr = {"N": 0.85, "L": 0.68 if changed else 0.62, "H": 0.5 if changed else 0.27}[m["PR"]]
    iss = 1 - (1 - _W["CIA"][m["C"]]) * (1 - _W["CIA"][m["I"]]) * (1 - _W["CIA"][m["A"]])
    impact = 7.52 * (iss - 0.029) - 3.25 * (iss - 0.02) ** 15 if changed else 6.42 * iss
    expl = 8.22 * _W["AV"][m["AV"]] * _W["AC"][m["AC"]] * pr * _W["UI"][m["UI"]]
    if impact <= 0:
        return 0.0
    return _roundup(min((1.08 if changed else 1.0) * (impact + expl), 10))


def sev_of_score(s):
    if s == 0:
        return "none"
    if s < 4:
        return "low"
    if s < 7:
        return "moderate"
    if s < 9:
        return "high"
    return "critical"


def severity(adv):
    info = adv.get("informational")
    if info:
        return "info", f"informational={info.strip()}"
    vec = adv.get("cvss")
    if not vec:
        return "moderate", "no-cvss(assumed moderate)"
    if vec.startswith("CVSS:3."):
        s = cvss3_score(vec)
        return sev_of_score(s), f"cvss3={s}"
    if vec.startswith("CVSS:4."):
        return "high", "cvss4(unscored, fail-closed as high)"
    return "high", f"unknown-cvss({vec[:12]}) fail-closed"


# ---------------------------------------------------------------- DB and locks


def default_db():
    return os.environ.get("RUSTSEC_DB") or os.path.expanduser("~/.cache/dowiz-deps/advisory-db")


def db_age_days(db):
    try:
        out = subprocess.run(["git", "-C", db, "log", "-1", "--format=%ct"], capture_output=True, text=True, check=True).stdout.strip()
        return (time.time() - int(out)) / 86400.0
    except Exception as e:  # noqa: BLE001
        refuse(f"{db} is not a readable git checkout of rustsec/advisory-db ({e}); run tools/deps/refresh.sh")


def load_db(db):
    root = os.path.join(db, "crates")
    if not os.path.isdir(root):
        refuse(f"no advisory DB at {db} (missing {root}); run tools/deps/refresh.sh")
    by_pkg = {}
    n = 0
    for dirpath, _, files in os.walk(root):
        for f in files:
            if not f.endswith(".md"):
                continue
            p = os.path.join(dirpath, f)
            text = open(p, encoding="utf-8").read()
            m = re.search(r"```toml\n(.*?)\n```", text, re.S)
            if not m:
                refuse(f"advisory without a toml block: {p}")
            try:
                d = tomllib.loads(m.group(1))
            except tomllib.TOMLDecodeError as e:
                refuse(f"advisory toml does not parse: {p}: {e}")
            n += 1
            adv = d.get("advisory", {})
            if adv.get("withdrawn"):
                continue
            ver = d.get("versions", {})
            title = next((ln[2:].strip() for ln in text[m.end():].splitlines() if ln.startswith("# ")), "")
            by_pkg.setdefault(adv["package"], []).append(
                {
                    "id": adv["id"],
                    "aliases": adv.get("aliases", []),
                    "patched": ver.get("patched", []),
                    "unaffected": ver.get("unaffected", []),
                    "adv": adv,
                    "title": title,
                }
            )
    if n < 500:
        refuse(f"advisory DB at {db} holds {n} advisories (< 500): not a real checkout")
    return by_pkg, n


def tracked_locks(d, name):
    """Every tracked <name> under directory d, or None when d is not inside a git work tree."""
    try:
        r = subprocess.run(["git", "-C", d, "ls-files", "-z", "--", ":(glob)**/" + name, name],
                           capture_output=True, check=True)
    except (OSError, subprocess.CalledProcessError):
        return None
    return [os.path.join(d, f) for f in r.stdout.decode().split("\0") if f and os.path.isfile(os.path.join(d, f))]


def find_locks(paths):
    out = []
    for p in paths:
        if os.path.isfile(p):
            out.append(p)
            continue
        # Only the locks git tracks: the tree also holds other lanes' worktrees and scratch copies
        # (.claude/worktrees/agent-*), whose stale locks we do not ship.
        tracked = tracked_locks(p, "Cargo.lock")
        if tracked is not None:
            out.extend(tracked)
            continue
        for dirpath, dirnames, files in os.walk(p):
            dirnames[:] = [d for d in dirnames if d not in ("target", "node_modules", ".git")]
            if "Cargo.lock" in files:
                out.append(os.path.join(dirpath, "Cargo.lock"))
    return sorted(out)


def lock_packages(path):
    try:
        d = tomllib.load(open(path, "rb"))
    except tomllib.TOMLDecodeError as e:
        refuse(f"{path} does not parse: {e}")
    pk = []
    for p in d.get("package", []):
        src = p.get("source", "")
        if src.startswith("registry+") or src.startswith("sparse+"):
            pk.append((p["name"], p["version"]))
    return pk


def vulnerable(entry, v):
    for r in entry["patched"] + entry["unaffected"]:
        if req_matches(r, v):
            return False
    return True


def check(locks, by_pkg):
    findings = []
    for lock in locks:
        for name, ver in lock_packages(lock):
            for e in by_pkg.get(name, []):
                try:
                    v = parse_version(ver)
                    hit = vulnerable(e, v)
                except ValueError as ex:
                    refuse(f"{lock}: {name} {ver} vs {e['id']}: {ex}")
                if hit:
                    sev, why = severity(e["adv"])
                    findings.append(
                        {
                            "lock": lock,
                            "package": name,
                            "version": ver,
                            "id": e["id"],
                            "aliases": e["aliases"],
                            "severity": sev,
                            "why": why,
                            "patched": e["patched"],
                            "title": e["title"],
                        }
                    )
    return findings


def load_ignores(path):
    out = []
    for i, ln in enumerate(open(path, encoding="utf-8"), 1):
        ln = ln.split("#", 1)[0].strip()
        if not ln:
            continue
        f = ln.split(None, 3)
        if len(f) < 4 or not re.fullmatch(r"\d{4}-\d{2}-\d{2}", f[2]):
            refuse(f"{path}:{i}: want `ID LOCK_DIR YYYY-MM-DD reason`, got {ln!r}")
        if time.strftime("%Y-%m-%d") > f[2]:
            refuse(f"{path}:{i}: ignore of {f[0]} in {f[1]} expired on {f[2]}: fix it or re-decide it")
        out.append({"id": f[0], "dir": os.path.normpath(f[1]), "until": f[2], "why": f[3], "used": 0})
    return out


def apply_ignores(findings, ignores, path):
    for f in findings:
        d = os.path.normpath(os.path.dirname(f["lock"]))
        for ig in ignores:
            if ig["id"] == f["id"] and (d == ig["dir"] or d.endswith(os.sep + ig["dir"])):
                f["ignored"] = f"until {ig['until']}: {ig['why']}"
                ig["used"] += 1
    for ig in ignores:
        if not ig["used"]:
            refuse(f"{path}: ignore of {ig['id']} in {ig['dir']} matches no finding any more: delete the line")


# ---------------------------------------------------------------- self-test


def self_test():
    V = parse_version
    cases = [
        (">= 1.44.2", "1.44.2", True), (">= 1.44.2", "1.44.1", False),
        (">= 1.38.2, < 1.39.0", "1.38.5", True), (">= 1.38.2, < 1.39.0", "1.39.0", False),
        ("^0.4.1", "0.4.9", True), ("^0.4.1", "0.5.0", False), ("^0.0.3", "0.0.4", False),
        ("0.3.2", "0.3.9", True), ("0.3.2", "0.4.0", False), ("1.2.3", "1.9.0", True),
        ("=1.0.0", "1.0.1", False), ("= 1.0", "1.0.7", True), ("~1.2", "1.2.9", True), ("~1.2", "1.3.0", False),
        ("< 0.2.5", "0.2.4", True), ("> 2.0", "2.0.9", False), ("> 2.0", "2.1.0", True), ("<= 1.2", "1.2.9", True),
        (">= 0.10", "0.10.0", True), (">= 2", "2.0.0", True),
        (">= 1.0.0-alpha.3", "1.0.0-alpha.4", True), (">= 1.0.0-alpha.3", "1.0.0-alpha.2", False),
        (">= 1.0.0", "1.0.0-rc.1", False),  # pre-release does not match a release comparator
        (">= 0.9.0", "1.1.0-beta.1", False),
        ("< 1.0.0-rc.1, >= 0.9.0", "1.0.0-alpha.1", True),
    ]
    bad = 0
    for req, ver, want in cases:
        got = req_matches(req, V(ver))
        if got != want:
            bad += 1
            print(f"self-test FAIL: {req!r} vs {ver}: got {got}, want {want}")
    order = ["1.0.0-alpha", "1.0.0-alpha.1", "1.0.0-alpha.beta", "1.0.0-beta", "1.0.0-beta.2", "1.0.0-beta.11", "1.0.0-rc.1", "1.0.0"]
    for a, b in zip(order, order[1:]):
        if cmp(V(a), V(b)) >= 0:
            bad += 1
            print(f"self-test FAIL: precedence {a} < {b}")
    for vec, want in [
        ("CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H", 9.8),
        ("CVSS:3.1/AV:N/AC:H/PR:N/UI:N/S:U/C:N/I:N/A:H", 5.9),
        ("CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:C/C:H/I:H/A:H", 10.0),
        ("CVSS:3.1/AV:L/AC:L/PR:L/UI:N/S:U/C:L/I:N/A:N", 3.3),
        ("CVSS:3.0/AV:N/AC:L/PR:L/UI:R/S:C/C:L/I:L/A:N", 5.4),
    ]:
        if cvss3_score(vec) != want:
            bad += 1
            print(f"self-test FAIL: {vec} -> {cvss3_score(vec)}, want {want}")
    try:
        req_matches(">>= 1", V("1.0.0"))
        bad += 1
        print("self-test FAIL: a garbage requirement parsed")
    except ValueError:
        pass
    print(f"rustsec: self-test {'FAIL' if bad else 'ok'} ({len(cases)} requirement cases, {bad} failed)")
    return 1 if bad else 0


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("paths", nargs="*")
    ap.add_argument("--db", default=default_db())
    ap.add_argument("--max-age-days", type=float, default=7)
    ap.add_argument("--fail-on", default="high", choices=["low", "moderate", "high", "critical"])
    ap.add_argument("--json", action="store_true")
    ap.add_argument("--ignore")
    ap.add_argument("--self-test", action="store_true")
    a = ap.parse_args()
    if a.self_test:
        sys.exit(self_test())
    age = db_age_days(a.db)
    if age > a.max_age_days:
        refuse(f"advisory DB {a.db} is {age:.1f} days old (> {a.max_age_days}); run tools/deps/refresh.sh")
    by_pkg, n = load_db(a.db)
    locks = find_locks(a.paths or ["."])
    if not locks:
        refuse("no Cargo.lock found")
    npk = sum(len(lock_packages(l)) for l in locks)
    findings = check(locks, by_pkg)
    if a.ignore:
        apply_ignores(findings, load_ignores(a.ignore), a.ignore)
    if a.json:
        print(json.dumps(findings, indent=1))
    else:
        for f in findings:
            tag = "IGNORED" if f.get("ignored") else "FINDING"
            print(f"{tag} {f['severity']:<8} {f['id']} {f['package']} {f['version']} {f['lock']} [{f['why']}] "
                  f"patched={f['patched']} aliases={f['aliases']} :: {f['title']}" + (f" [{f['ignored']}]" if f.get("ignored") else ""))
    worst = [f for f in findings if SEV_RANK[f["severity"]] >= SEV_RANK[a.fail_on] and not f.get("ignored")]
    counts = {}
    for f in findings:
        counts[f["severity"]] = counts.get(f["severity"], 0) + 1
    nign = sum(1 for f in findings if f.get("ignored"))
    print(f"rustsec: {nign} ignored by {a.ignore}; ", end="", file=sys.stderr if a.json else sys.stdout) if a.ignore else None
    print(f"rustsec: {len(locks)} locks, {npk} registry packages, {n} advisories (db {age:.1f} d old); "
          f"{len(findings)} findings {counts}; {len(worst)} at or above {a.fail_on}", file=sys.stderr if a.json else sys.stdout)
    sys.exit(1 if worst else 0)


if __name__ == "__main__":
    main()
