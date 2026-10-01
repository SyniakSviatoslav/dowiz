#!/usr/bin/env python3
"""npm_audit.py -- match every package-lock.json in the tree against npm's advisory DB, offline-first.

`npm audit` needs the network on every run and prints nothing useful when it cannot reach the
registry, so the dependency gate uses the same data a different way: collect every (name, version)
from each lock, POST them once to the registry's bulk advisory endpoint (the endpoint `npm audit`
itself uses), CACHE the answer keyed by the exact request, and match locally. A later run with the
same locks reads the cache and needs no network at all.

    python3 tools/deps/npm_audit.py [--cache DIR] [--max-age-days N] [--fail-on high] [--offline]
                                    [--json] [LOCK_OR_DIR ...]   # default: every package-lock.json under .
    python3 tools/deps/npm_audit.py --self-test

Cache: --cache, else $NPM_ADVISORY_CACHE, else ~/.cache/dowiz-deps/npm-bulk. A cached answer older
than --max-age-days (default 2) is refetched; if that fetch fails, or --offline is given and there is
no fresh answer, it is a REFUSAL (exit 2): a checker that cannot measure never passes.

Ranges are npm's (node-semver): `||` is OR, whitespace is AND, `a - b` is inclusive, a bare
version is `=`, `^`/`~`/x-ranges as usual. An unparsed range is a refusal, never a silent miss.

Exit: 0 no finding at or above --fail-on; 1 at least one; 2 refusal.
"""
import argparse
import hashlib
import json
import os
import subprocess
import re
import sys
import time
import urllib.request

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from rustsec import SEV_RANK, cmp, parse_comparator, parse_version  # noqa: E402

BULK_URL = "https://registry.npmjs.org/-/npm/v1/security/advisories/bulk"


def refuse(msg):
    print(f"npm_audit: REFUSED: {msg}", file=sys.stderr)
    print(f"npm_audit: REFUSED: {msg}")
    sys.exit(2)


# ---------------------------------------------------------------- node-semver ranges

HYPHEN_RE = re.compile(r"^\s*(\S+)\s+-\s+(\S+)\s*$")


def _set_matches(s, v):
    s = s.strip()
    bounds, anchors = [], []
    m = HYPHEN_RE.match(s)
    parts = [">=" + m.group(1), "<=" + m.group(2)] if m else s.split()
    if not parts:
        parts = ["*"]
    for p in parts:
        p = p.strip()
        if p in ("*", "x", "X", ""):
            continue
        if p[0].isdigit() or p[0] == "v":
            p = "=" + p.lstrip("v")
        b, a = parse_comparator(p)
        bounds += b
        if a is not None:
            anchors.append(a)
    for op, bv in bounds:
        c = cmp(v, bv)
        if not {"=": c == 0, ">": c > 0, ">=": c >= 0, "<": c < 0, "<=": c <= 0}[op]:
            return False
    if v[3]:
        return any(a[:3] == v[:3] for a in anchors)
    return True


def range_matches(rng, v):
    return any(_set_matches(s, v) for s in rng.split("||"))


# ---------------------------------------------------------------- locks


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
        tracked = tracked_locks(p, "package-lock.json")
        if tracked is not None:
            out.extend(tracked)
            continue
        for dirpath, dirnames, files in os.walk(p):
            dirnames[:] = [d for d in dirnames if d not in ("node_modules", "target", ".git")]
            if "package-lock.json" in files:
                out.append(os.path.join(dirpath, "package-lock.json"))
    return sorted(out)


def lock_packages(path):
    try:
        d = json.load(open(path, encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as e:
        refuse(f"{path} does not parse: {e}")
    pk = set()
    if "packages" in d:
        for key, e in d["packages"].items():
            if not key or e.get("link") or "version" not in e:
                continue
            name = e.get("name") or key.rsplit("node_modules/", 1)[-1]
            pk.add((name, e["version"]))
    else:  # lockfileVersion 1
        def walk(deps):
            for name, e in (deps or {}).items():
                if "version" in e and not e["version"].startswith(("file:", "git", "http")):
                    pk.add((name, e["version"]))
                walk(e.get("dependencies"))
        walk(d.get("dependencies"))
    if not pk and d.get("packages", {}).get("", {}).get("dependencies"):
        refuse(f"{path}: declares dependencies but no locked package was read")
    return sorted(pk)


# ---------------------------------------------------------------- advisories (cached)


def cache_dir(a):
    return a.cache or os.environ.get("NPM_ADVISORY_CACHE") or os.path.expanduser("~/.cache/dowiz-deps/npm-bulk")


def advisories(req, a):
    body = json.dumps(req, sort_keys=True).encode()
    key = hashlib.sha256(body).hexdigest()[:24]
    cdir = cache_dir(a)
    cpath = os.path.join(cdir, f"{key}.json")
    if os.path.exists(cpath):
        c = json.load(open(cpath))
        age = (time.time() - c["fetched_at"]) / 86400.0
        if age <= a.max_age_days:
            return c["advisories"], f"cache {age:.2f} d old"
    if a.offline:
        refuse(f"--offline and no answer younger than {a.max_age_days} d in {cdir} for this lock set; run without --offline")
    last = None
    for _ in range(2):  # the box's TCP connect is 4-11 s; one retry
        try:
            r = urllib.request.Request(BULK_URL, data=body, headers={"content-type": "application/json"}, method="POST")
            with urllib.request.urlopen(r, timeout=60) as resp:
                adv = json.load(resp)
            break
        except Exception as e:  # noqa: BLE001
            last = e
    else:
        refuse(f"advisory fetch from {BULK_URL} failed twice ({last}) and no fresh cache")
    if not isinstance(adv, dict):
        refuse(f"advisory endpoint answered a {type(adv).__name__}, not an object")
    os.makedirs(cdir, exist_ok=True)
    tmp = cpath + ".tmp"
    json.dump({"fetched_at": time.time(), "request_packages": len(req), "advisories": adv}, open(tmp, "w"))
    os.replace(tmp, cpath)
    return adv, "fetched now"


def check(locks, adv):
    findings = []
    for lock in locks:
        for name, ver in lock_packages(lock):
            for e in adv.get(name, []):
                try:
                    hit = range_matches(e["vulnerable_versions"], parse_version(ver))
                except ValueError as ex:
                    refuse(f"{lock}: {name} {ver} vs {e.get('url')}: {ex}")
                if hit:
                    sev = e.get("severity", "high")
                    if sev not in SEV_RANK:
                        sev = "high"  # unknown label: fail closed
                    findings.append({"lock": lock, "package": name, "version": ver, "id": e["url"].rsplit("/", 1)[-1],
                                     "severity": sev, "range": e["vulnerable_versions"], "title": e.get("title", "")})
    return findings


# ---------------------------------------------------------------- self-test


def self_test():
    V = parse_version
    cases = [
        (">=7.28.0 <7.29.1", "7.29.0", True), (">=7.28.0 <7.29.1", "7.29.1", False), (">=7.28.0 <7.29.1", "7.27.9", False),
        ("<4.17.21", "4.17.20", True), ("<4.17.21", "4.17.21", False), ("*", "0.0.1", True),
        ("1.2.3", "1.2.3", True), ("1.2.3", "1.2.4", False), ("1.2", "1.2.9", True), ("1.2", "1.3.0", False),
        ("1.0.0 - 1.2.0", "1.2.0", True), ("1.0.0 - 1.2.0", "1.2.1", False),
        ("<1.0.0 || >=2.0.0 <2.1.3", "2.1.2", True), ("<1.0.0 || >=2.0.0 <2.1.3", "1.5.0", False),
        ("^2.1.0", "2.9.9", True), ("^2.1.0", "3.0.0", False), ("~1.4.1", "1.4.9", True), ("~1.4.1", "1.5.0", False),
        (">=1.0.0 <1.0.4", "1.0.4-beta.1", False), (">=1.0.4-alpha <1.0.4", "1.0.4-beta.1", True),
    ]
    bad = 0
    for rng, ver, want in cases:
        got = range_matches(rng, V(ver))
        if got != want:
            bad += 1
            print(f"self-test FAIL: {rng!r} vs {ver}: got {got}, want {want}")
    try:
        range_matches(">>1", V("1.0.0"))
        bad += 1
        print("self-test FAIL: a garbage range parsed")
    except ValueError:
        pass
    print(f"npm_audit: self-test {'FAIL' if bad else 'ok'} ({len(cases)} range cases, {bad} failed)")
    return 1 if bad else 0


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("paths", nargs="*")
    ap.add_argument("--cache")
    ap.add_argument("--max-age-days", type=float, default=2)
    ap.add_argument("--offline", action="store_true")
    ap.add_argument("--fail-on", default="high", choices=["low", "moderate", "high", "critical"])
    ap.add_argument("--json", action="store_true")
    ap.add_argument("--self-test", action="store_true")
    a = ap.parse_args()
    if a.self_test:
        sys.exit(self_test())
    locks = find_locks(a.paths or ["."])
    if not locks:
        refuse("no package-lock.json found")
    req = {}
    for l in locks:
        for n, v in lock_packages(l):
            req.setdefault(n, set()).add(v)
    req = {n: sorted(vs) for n, vs in sorted(req.items())}
    adv, src = advisories(req, a)
    findings = check(locks, adv)
    if a.json:
        print(json.dumps(findings, indent=1))
    else:
        for f in findings:
            print(f"FINDING {f['severity']:<8} {f['id']} {f['package']} {f['version']} {f['lock']} range={f['range']!r} :: {f['title']}")
    worst = [f for f in findings if SEV_RANK[f["severity"]] >= SEV_RANK[a.fail_on]]
    counts = {}
    for f in findings:
        counts[f["severity"]] = counts.get(f["severity"], 0) + 1
    npk = sum(len(v) for v in req.values())
    print(f"npm_audit: {len(locks)} locks, {npk} package versions, advisories {src}; {len(findings)} findings {counts}; "
          f"{len(worst)} at or above {a.fail_on}", file=sys.stderr if a.json else sys.stdout)
    sys.exit(1 if worst else 0)


if __name__ == "__main__":
    main()
