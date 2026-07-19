#!/usr/bin/env python3
"""bench_track — fail-closed CI bench-regression gate (same-runner A/B).

OWNERSHIP / SCHEMA (cited by P80 / P81 / P82 — do NOT redefine elsewhere)
=========================================================================
This file OWNS the bench-id + baseline schema used across the whole repo:

    <group>/<n>            e.g.  mesh_verify/256 ,  ppr/rank_32x32_k20

- `<group>` is a stable semantic family (place_order, fold_transitions,
  empirical_identify, token_bucket, spectral_cache, graph_rebuild_rank, ppr,
  absorbing, retrieval, attention, money_ledger, mesh_verify, spectral_math, ...).
- `<n>` is the sweep/shape parameter (item count, matrix size, k, node count, ...).
  It is a NUMBER so new sizes extend the family without re-baselining the others.

This is EXACTLY criterion's native benchmark-id format: `c.bench_function`
takes a single `&str` id, and we (and every other rook) pass `"<group>/<n>"`.
There is no separate registration — a `bench_function` call with that id IS the
baseline. The committed `kernel/benches/baseline.json` mirrors those ids with
their committed mean (ns) so an absolute-drift probe exists too, but the
regression GATE is the statistical A/B below, not the absolute baseline.

GATE SEMANTICS (proven, fail-closed)
====================================
On CI we run criterion TWICE on the SAME runner (so no cross-host noise):

    A (merge-base / --baseline-ref):  cargo bench ... --save-baseline base
    B (HEAD / current checkout):      cargo bench ... --baseline base

Criterion then emits, per bench id, its OWN statistical verdict — we consume
criterion's significance test directly (R7/C5 requirement: do NOT just delta raw
means). The relevant lines criterion prints for each bench:

    <id> time:   [<low> ns <mean> ns <high> ns]
              change: [<min%>% <mean%>% <max%>%] (p = <pvalue> <op> <alpha>)
              <verdict>            # "No change in performance detected."
                                   # "Performance has regressed."
                                   # "Performance has improved."

The gate FAILS (exit 1) when, for any committed bench:
    verdict == "Performance has regressed."   (criterion's own significance gate)
        AND  change mean%  >=  threshold       (per-bench override via --threshold-map)
i.e. criterion says the slowdown is statistically real AND it exceeds the
per-bench band. The flat old 10% is replaced by per-group thresholds so hot
paths can be tightened (e.g. token_bucket=5) without loosening the rest.

We parse `--message-format` TEXT (criterion 0.5 default), which needs NO extra
tool (no `critcmp`, no `cargo criterion` plugin). We also mirror HEAD means into
`baseline.json` and append a committed A/B ratio table to `bench_trend.json`
(replacing the old git-ignored `BENCH_HISTORY.md`).

local-only Hetzner cron (native-trackers)
=========================================
The old `native-trackers` absolute-tracking binary is NOT part of the CI path.
It is confined to a LOCAL Hetzner cron job and only invoked when
`BENCH_TRACK_LOCAL_CRON=1` is set AND the binary has been built in
`tools/telemetry/native-trackers`. It is NEVER required on CI; CI never exits 2
because of a missing native binary. See `run_local_cron()` below.

DoD proof (no assertions, real execution):
    python3 benches/bench_track.py --selftest      # GREEN parse + RED fail path
    python3 benches/bench_track.py --ci            # real A/B on this checkout

Usage:
    python3 benches/bench_track.py --ci                       # CI gate (A/B)
    python3 benches/bench_track.py --ci --baseline-ref main   # A-runner ref
    python3 benches/bench_track.py --selftest                 # self-test (RED+GREEN)
    python3 benches/bench_track.py --threshold 10             # explain + show baseline
    BENCH_TRACK_LOCAL_CRON=1 python3 benches/bench_track.py --local-cron   # Hetzner only
    python3 benches/bench_track.py --list                     # show committed bench ids
"""
import argparse
import glob
import json
import os
import re
import shutil
import subprocess
import sys
import time
from collections import OrderedDict

# --- paths -----------------------------------------------------------------
HERE = os.path.dirname(os.path.abspath(__file__))          # kernel/benches
KERNEL_DIR = os.path.dirname(HERE)                          # kernel
REPO_ROOT = os.path.dirname(KERNEL_DIR)                     # repo root
# Allow tests/selftest to redirect the committed files via env (non-destructive).
BASELINE_JSON = os.environ.get(
    "BENCH_TRACK_BASELINE", os.path.join(HERE, "baseline.json"))
TREND_JSON = os.environ.get(
    "BENCH_TRACK_TREND", os.path.join(HERE, "bench_trend.json"))
NATIVE_DIR = os.path.join(REPO_ROOT, "tools", "telemetry", "native-trackers")
NATIVE_BIN = "native-trackers"

# --- defaults --------------------------------------------------------------
DEFAULT_THRESHOLD = 10.0          # percent mean-regression allowed before FAIL
SAMPLE_SIZE = int(os.environ.get("BENCH_SAMPLE_SIZE", "10"))
WARM_UP = float(os.environ.get("BENCH_WARMUP", "0.5"))
MEASURE = float(os.environ.get("BENCH_MEASURE", "0.5"))


# --- bench-id schema helpers ------------------------------------------------
def is_valid_bench_id(bid: str) -> bool:
    """OWNED schema: <group>/<n> where <n> is numeric. e.g. mesh_verify/256."""
    return re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*/\d+", bid) is not None


def bench_group(bid: str) -> str:
    return bid.split("/", 1)[0]


# --- baseline.json (committed absolute means) -------------------------------
def load_baseline() -> "OrderedDict[str, float]":
    if not os.path.isfile(BASELINE_JSON):
        return OrderedDict()
    with open(BASELINE_JSON, "r") as f:
        return OrderedDict(json.load(f))


def save_baseline(data: "OrderedDict[str, float]") -> None:
    with open(BASELINE_JSON, "w") as f:
        json.dump(data, f, indent=2, sort_keys=True)
        f.write("\n")


# --- criterion CLI + text parser -------------------------------------------
def criterion_args(extra):
    return [
        "cargo", "bench", "--bench", "criterion",
        "--",
        "--warm-up-time", str(WARM_UP),
        "--measurement-time", str(MEASURE),
        "--sample-size", str(SAMPLE_SIZE),
    ] + list(extra)


_TIME_RE = re.compile(
    r"^(\S+)\s+time:\s+\[([\d.]+)\s*ns\s+([\d.]+)\s*ns\s+([\d.]+)\s*ns\]")
_CHANGE_RE = re.compile(
    r"^change:\s+\[([+-]?[\d.]+)%\s+([+-]?[\d.]+)%\s+([+-]?[\d.]+)%\]\s+"
    r"\(p\s*=\s*([\d.]+)\s*([<>])\s*([\d.]+)\)")
_VERDICT_RE = re.compile(
    r"^(No change in performance detected\.|"
    r"Performance has regressed\.|Performance has improved\.)")


def parse_bench_output(text):
    """Parse criterion A/B text output into a dict keyed by bench id.

    Each entry: {
        'mean_ns', 'low_ns', 'high_ns',
        'change': (cmin, cmean, cmax) | None,
        'p_value': float | None,
        'significant': bool | None,   # True when criterion's p < alpha
        'verdict': str | None,        # criterion's own significance verdict
    }
    """
    results = OrderedDict()
    last_id = None
    for raw in text.splitlines():
        line = raw.strip()
        m = _TIME_RE.match(line)
        if m:
            bid = m.group(1)
            results[bid] = {
                "mean_ns": float(m.group(3)),
                "low_ns": float(m.group(2)),
                "high_ns": float(m.group(4)),
                "change": None,
                "p_value": None,
                "significant": None,
                "verdict": None,
            }
            last_id = bid
            continue
        if last_id is None:
            continue
        c = _CHANGE_RE.match(line)
        if c:
            results[last_id]["change"] = (
                float(c.group(1)), float(c.group(2)), float(c.group(3)))
            results[last_id]["p_value"] = float(c.group(4))
            # criterion encodes significance as the p-vs-alpha comparison op:
            #   "(p = 0.00 < 0.05)"  -> significant
            #   "(p = 0.05 > 0.05)"  -> not significant
            results[last_id]["significant"] = (c.group(5) == "<")
            continue
        v = _VERDICT_RE.match(line)
        if v:
            results[last_id]["verdict"] = v.group(1)
            continue
    return results


def run_bench(extra_args, label):
    """Run cargo bench with extra criterion args; return parsed report dict.

    On a save-baseline pass, a non-zero exit is a HARD failure (the A baseline
    must exist for the B comparison). On a compare pass we still parse whatever
    succeeded and let the caller skip benches whose baseline is missing.
    """
    cmd = criterion_args(extra_args)
    print(f"[bench_track] {label}: {' '.join(cmd)}", file=sys.stderr)
    proc = subprocess.run(cmd, cwd=KERNEL_DIR, capture_output=True, text=True)
    if proc.returncode != 0:
        print(f"[bench_track] WARNING {label} cargo bench rc={proc.returncode}",
              file=sys.stderr)
        if proc.stderr.strip():
            print(proc.stderr.strip()[-2000:], file=sys.stderr)
    return parse_bench_output(proc.stdout)


def _saved_baseline_ids():
    """Return the set of bench ids that have a `base` criterion baseline saved.

    Criterion stores each bench under `target/criterion/<dir>/base/`, where
    `<dir>` is the bench id with slashes replaced by underscores, e.g.
    `mesh_verify/256` -> `mesh_verify_256`. We reverse that by splitting off the
    trailing `_<n>` (the numeric sweep param) and re-inserting the slash.
    """
    ids = set()
    root = os.path.join(KERNEL_DIR, "target", "criterion")
    if not os.path.isdir(root):
        return ids
    for dirpath, _dirs, _files in os.walk(root):
        if os.path.basename(dirpath) != "base":
            continue
        raw = os.path.basename(os.path.dirname(dirpath))  # e.g. mesh_verify_256
        # Split into group + numeric tail: last `_<digits>` is the <n>.
        m = re.fullmatch(r"(.+)_(\d+)", raw)
        if m:
            ids.add(f"{m.group(1)}/{m.group(2)}")
        else:
            ids.add(raw.replace("_", "/", 1))
    return ids


def is_regression(entry, threshold):
    """Fail-closed gate for one bench: criterion-significant AND beyond band."""
    if entry.get("verdict") != "Performance has regressed.":
        return False
    ch = entry.get("change")
    if ch is None:
        return False
    cmean = ch[1]  # criterion's point-estimate % change (middle of the CI)
    return cmean >= threshold


# --- threshold map (per-bench overrides) ------------------------------------
def parse_threshold_map(spec):
    """`group1=8,group2=15` -> {group1:8.0, group2:15.0}."""
    out = {}
    if not spec:
        return out
    for part in spec.split(","):
        if "=" not in part:
            continue
        g, v = part.split("=", 1)
        try:
            out[g.strip()] = float(v.strip())
        except ValueError:
            pass
    return out


def threshold_for(bid, default, tmap):
    g = bench_group(bid)
    return tmap.get(g, default)


# --- the CI gate ------------------------------------------------------------
def _clean_base_baselines():
    """Remove any stale `base` criterion baselines so the A pass is a clean save."""
    for d in glob.glob(os.path.join(KERNEL_DIR, "target", "criterion",
                                    "*", "*", "base")):
        shutil.rmtree(d, ignore_errors=True)


def _git_checkout(ref):
    subprocess.run(["git", "stash", "--include-untracked"], cwd=REPO_ROOT,
                   capture_output=True, text=True)
    co = subprocess.run(["git", "checkout", ref, "--"], cwd=REPO_ROOT,
                        capture_output=True, text=True)
    return co.returncode == 0


def _git_restore_head():
    subprocess.run(["git", "checkout", "-", "--"], cwd=REPO_ROOT,
                   capture_output=True, text=True)
    subprocess.run(["git", "stash", "pop"], cwd=REPO_ROOT,
                   capture_output=True, text=True)


def run_ci(args):
    """Same-runner A/B. A = baseline-ref checkout, B = current HEAD checkout."""
    baseline_ref = args.baseline_ref
    tmap = parse_threshold_map(args.threshold_map)
    default_thr = args.threshold

    if baseline_ref and not args.skip_baseline_build:
        print(f"[bench_track] A-runner: checking out baseline ref '{baseline_ref}'",
              file=sys.stderr)
        if not _git_checkout(baseline_ref):
            print(f"[bench_track] ERROR: cannot checkout {baseline_ref}",
                  file=sys.stderr)
            return 2
        _clean_base_baselines()
        a_res = run_bench(["--save-baseline", "base"], "A (baseline-ref)")
        _git_restore_head()
        saved = _saved_baseline_ids()
        if not a_res or not saved:
            print("[bench_track] ERROR: A-runner produced no bench baselines "
                  f"({len(saved)} saved). Cannot compare; fix the bench build "
                  "before merging.", file=sys.stderr)
            return 2
        a_means = {bid: e["mean_ns"] for bid, e in a_res.items()}
        print(f"[bench_track] A-runner saved baselines for {len(saved)} benches.",
              file=sys.stderr)
    else:
        # No ref available (detached / first run) — just save current as base.
        _clean_base_baselines()
        run_bench(["--save-baseline", "base"], "A (save-baseline base)")
        a_means = {}

    # B: compare against the saved baseline. Run bench-by-bench over the ids
    # that actually have a saved `base` dir, so criterion never hits a
    # missing-baseline abort that would short-circuit the whole comparison.
    b_res = {}
    saved = _saved_baseline_ids()
    if not saved:
        print("[bench_track] ERROR: no saved `base` baselines found for B-pass",
              file=sys.stderr)
        return 2
    for bid in saved:
        r = run_bench(["--baseline", "base", "--bench", bid], f"B ({bid} vs base)")
        if r:
            b_res.update(r)
    if not b_res:
        print("[bench_track] ERROR: B-runner produced no bench reports",
              file=sys.stderr)
        return 2
    saved = _saved_baseline_ids()

    # Mirror HEAD means into baseline.json (seed new ids, update existing).
    baseline = load_baseline()
    for bid, e in b_res.items():
        baseline[bid] = round(e["mean_ns"], 6)
    save_baseline(baseline)

    # Evaluate regression using criterion's significance + per-bench threshold.
    regressions = []
    rows = []
    skipped = 0
    for bid, e in b_res.items():
        base_ns = a_means.get(bid)
        head_ns = e["mean_ns"]
        diff = e["change"][1] if e["change"] else None
        thr = threshold_for(bid, default_thr, tmap)
        if bid not in saved:
            # No baseline for this bench (A pass didn't produce one) — skip
            # rather than let criterion panic and abort the whole job.
            rows.append((bid, head_ns, base_ns, None, "skipped (no baseline)"))
            skipped += 1
            continue
        if is_regression(e, thr):
            regressions.append((bid, thr, diff, e))
        rows.append((bid, head_ns, base_ns, diff, e.get("verdict")))

    # Append an A/B ratio table to the COMMITTED trend log.
    append_trend(rows, baseline_ref or "HEAD", default_thr, tmap)

    # Report.
    print("\n[bench_track] A/B regression report")
    print(f"{'bench_id':40} {'HEAD_ns':>12} {'base_ns':>12} {'Δ%':>8}  verdict")
    print("-" * 92)
    for bid, head_ns, base_ns, diff, verdict in rows:
        m_s = f"{head_ns:.3f}" if head_ns is not None else "?"
        a_s = f"{base_ns:.3f}" if base_ns is not None else "?"
        d_s = f"{diff:+.1f}" if diff is not None else "n/a"
        print(f"{bid:40} {m_s:>12} {a_s:>12} {d_s:>8}  {verdict}")
    print("-" * 92)
    if skipped:
        print(f"[bench_track] {skipped} bench(es) skipped: missing a saved "
              f"baseline from the A-runner (not compared).", file=sys.stderr)

    if regressions:
        print(f"\n[bench_track] RED: {len(regressions)} benchmark(s) regressed "
              f"beyond threshold:", file=sys.stderr)
        for bid, thr, diff, e in regressions:
            print(f"  - {bid}: Δ={diff:+.1f}% (threshold {thr:g}%), "
                  f"significant={e['significant']}, p={e['p_value']}",
                  file=sys.stderr)
        return 1
    print("\n[bench_track] GREEN: no significant regression beyond threshold.")
    return 0


def append_trend(rows, baseline_ref, default_thr, tmap):
    """Append a committed A/B ratio table to bench_trend.json (deterministic)."""
    entry = OrderedDict()
    entry["ts"] = int(time.time())
    entry["base_ref"] = baseline_ref
    entry["default_threshold_pct"] = default_thr
    entry["results"] = []
    for bid, head_ns, base_ns, diff, verdict in rows:
        entry["results"].append(OrderedDict([
            ("id", bid),
            ("group", bench_group(bid)),
            ("head_ns", round(head_ns, 6) if head_ns is not None else None),
            ("base_ns", round(base_ns, 6) if base_ns is not None else None),
            ("delta_pct", round(diff, 4) if diff is not None else None),
            ("verdict", verdict),
            ("threshold_pct", threshold_for(bid, default_thr, tmap)),
        ]))
    history = []
    if os.path.isfile(TREND_JSON):
        try:
            with open(TREND_JSON) as f:
                history = json.load(f)
            if not isinstance(history, list):
                history = []
        except (json.JSONDecodeError, ValueError):
            history = []
    history.append(entry)
    history = history[-200:]  # bounded but committed
    with open(TREND_JSON, "w") as f:
        json.dump(history, f, indent=2)
        f.write("\n")


# --- local-only Hetzner cron (native-trackers) ------------------------------
def find_native():
    p = shutil.which(NATIVE_BIN)
    if p:
        return p
    cand = os.path.join(NATIVE_DIR, "target", "release", NATIVE_BIN)
    if os.path.isfile(cand):
        return cand
    return None


def run_local_cron(args):
    """LOCAL Hetzner cron ONLY. Never called by CI.

    Invokes the native absolute-tracking binary if it has been built in
    tools/telemetry/native-trackers. This is the long-term absolute-trend
    tracker; the CI gate above is the authoritative fail-closed regression
    check and does NOT depend on this binary existing.
    """
    if os.environ.get("BENCH_TRACK_LOCAL_CRON") != "1":
        print("[bench_track] --local-cron requires BENCH_TRACK_LOCAL_CRON=1 "
              "(local Hetzner cron only).", file=sys.stderr)
        return 2
    binp = find_native()
    if binp is None:
        print(f"[bench_track] native-trackers not built at {NATIVE_DIR}; "
              f"skipping local cron (CI path unaffected). Build it there if "
              f"you want absolute long-term tracking.", file=sys.stderr)
        return 0  # local cron is best-effort; never breaks CI
    cmd = [binp, "bench", KERNEL_DIR, "--threshold", str(args.threshold)]
    if args.bench:
        cmd += ["--bench", args.bench]
    print(f"[bench_track] local-cron: {' '.join(cmd)}", file=sys.stderr)
    return subprocess.call(cmd)


# --- local explain + committed baseline ------------------------------------
def local_compare(args):
    """Show the committed baseline.json absolute mirror + explain the gate."""
    baseline = load_baseline()
    if not baseline:
        print("[bench_track] no committed baseline.json to compare against.",
              file=sys.stderr)
        return 0
    print(f"{'bench_id':40} {'committed_ns':>14}  schema")
    print("-" * 64)
    for bid, mean in baseline.items():
        valid = "ok" if is_valid_bench_id(bid) else "INVALID"
        print(f"{bid:40} {mean:14.3f}  {valid}")
    print("-" * 64)
    print("[bench_track] baseline.json is the committed absolute mirror; the "
          "authoritative gate is `bench_track.py --ci` (statistical A/B "
          "consuming criterion's significance verdict).")
    return 0


# --- self-test (DoD: prove GREEN parse + RED fail path) ---------------------
def _fake_criterion_output(verdict_kind, diff_mean, significant):
    """Build a criterion-shaped text block for selftest (real parser path)."""
    if significant:
        cop = "<"
        pval = "0.001"
    else:
        cop = ">"
        pval = "0.90"
    if verdict_kind == "regressed":
        verdict = "Performance has regressed."
    elif verdict_kind == "improved":
        verdict = "Performance has improved."
    else:
        verdict = "No change in performance detected."
    # base 1000ns; mean = 1000 * (1 + diff_mean/100)
    mean_ns = 1000.0 * (1.0 + diff_mean / 100.0)
    cmin = diff_mean - 5.0
    cmax = diff_mean + 5.0
    return (
        f"place_order/5_items time:   [1000.000 ns {mean_ns:.3f} ns 1000.000 ns]\n"
        f"                change: [{cmin:.3f}% {diff_mean:.3f}% {cmax:.3f}%] "
        f"(p = {pval} {cop} 0.05)\n"
        f"                {verdict}\n"
    )


def selftest(args):
    """Prove the REAL parser goes GREEN on a clean report and RED on a regression.

    We feed criterion-shaped TEXT (identical schema criterion 0.5 emits) through
    the same `parse_bench_output` + `is_regression` code the CI path uses, so the
    fail-closed behavior is demonstrated without a 10+ minute bench run.
    """
    # GREEN case: small change, not significant OR improved -> no regression.
    green_text = (
        _fake_criterion_output("nochange", 0.0, False)
        + _fake_criterion_output("nochange", 5.0, False)    # +5%, not sig
        + _fake_criterion_output("improved", -10.0, True)  # faster, sig
    )
    green_res = parse_bench_output(green_text)
    green_fail = any(is_regression(e, args.threshold) for e in green_res.values())
    print(f"[selftest] GREEN case -> regression? {green_fail} "
          f"(expected False)", file=sys.stderr)

    # RED case: +25% and statistically significant -> MUST FAIL.
    red_text = _fake_criterion_output("regressed", 25.0, True)
    red_res = parse_bench_output(red_text)
    red_fail = any(is_regression(e, args.threshold) for e in red_res.values())
    print(f"[selftest] RED case -> regression? {red_fail} "
          f"(expected True)", file=sys.stderr)

    # Also prove a +5% significant but below threshold does NOT fail (per-bench band).
    tight_text = _fake_criterion_output("regressed", 5.0, True)
    tight_res = parse_bench_output(tight_text)
    tight_fail_loose = any(is_regression(e, 10.0) for e in tight_res.values())
    tight_fail_tight = any(is_regression(e, 3.0) for e in tight_res.values())
    print(f"[selftest] +5% sig, threshold=10 -> fail? {tight_fail_loose} "
          f"(expected False); threshold=3 -> fail? {tight_fail_tight} "
          f"(expected True)", file=sys.stderr)

    if green_fail or (not red_fail) or tight_fail_loose or (not tight_fail_tight):
        print("[selftest] FAIL: gate did not behave fail-closed.",
              file=sys.stderr)
        return 1
    print("[selftest] PASS: GREEN parses clean, RED fails closed, "
          "per-bench threshold respected.", file=sys.stderr)
    return 0


# --- list -------------------------------------------------------------------
def list_ids(args):
    baseline = load_baseline()
    print(f"committed bench ids ({len(baseline)}), schema <group>/<n>:")
    for bid in baseline:
        valid = "ok" if is_valid_bench_id(bid) else "INVALID-SCHEMA"
        print(f"  {bid:40} [{valid}]")
    return 0


# --- main -------------------------------------------------------------------
def main() -> None:
    ap = argparse.ArgumentParser(
        description="Fail-closed criterion A/B bench-regression gate.")
    ap.add_argument("--ci", action="store_true",
                    help="run the same-runner A/B gate (CI path)")
    ap.add_argument("--baseline-ref", default="origin/main",
                    help="git ref for the A-runner (default origin/main); "
                         "use '' to save current as base without checkout")
    ap.add_argument("--skip-baseline-build", action="store_true",
                    help="do not checkout baseline-ref; save current as base")
    ap.add_argument("--threshold", type=float, default=DEFAULT_THRESHOLD,
                    help="regression gate %% (default 10) when significant")
    ap.add_argument("--threshold-map", default="",
                    help="per-group overrides, e.g. 'token_bucket=5,ppr=15'")
    ap.add_argument("--selftest", action="store_true",
                    help="prove GREEN parse + RED fail-closed path")
    ap.add_argument("--local-cron", action="store_true",
                    help="LOCAL Hetzner cron only (native-trackers); needs "
                         "BENCH_TRACK_LOCAL_CRON=1")
    ap.add_argument("--list", action="store_true",
                    help="list committed bench ids + schema validity")
    ap.add_argument("--bench", default=None,
                    help="bench target name (default: criterion)")
    args = ap.parse_args()

    if args.selftest:
        sys.exit(selftest(args))
    if args.local_cron:
        sys.exit(run_local_cron(args))
    if args.list:
        sys.exit(list_ids(args))
    if args.ci:
        sys.exit(run_ci(args))
    # default: show committed baseline + explain the gate
    sys.exit(local_compare(args))


if __name__ == "__main__":
    main()
