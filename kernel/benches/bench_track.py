#!/usr/bin/env python3
"""bench_track — dynamic benchmark tracker for a Rust/criterion crate.

Runs `cargo bench` and parses criterion's stable TEXT output (the
`name  time:   [low mean high]` lines), compares each benchmark's mean
against benches/baseline.json (committed reference), prints a degrade/upgrade
delta table, appends a timestamped row to benches/BENCH_HISTORY.md
(git-ignored, so no repo churn), and exits non-zero when any benchmark
regresses beyond --threshold percent.

This is the "autotrack" mechanism: run it on every CI build / scheduled cron to
know the moment a hot path degrades (or silently improves).

Criterion 0.5 note: the `--output-format json` CLI flag was removed; text
parsing is version-stable and needs no special flags.

Usage:
    python3 benches/bench_track.py                 # run + compare + log
    python3 benches/bench_track.py --no-run        # compare baseline to itself (CI smoke)
    python3 benches/bench_track.py --threshold 15  # looser regression gate
"""
import argparse
import json
import os
import re
import subprocess
import sys
from datetime import datetime

UNIT_NS = {"ns": 1.0, "µs": 1e3, "us": 1e3, "ms": 1e6, "s": 1e9}
# criterion prints: "name  time:   [low unit mean unit high unit]"
LINE_RE = re.compile(
    r"^(?P<name>\S+)\s+time:\s+\[(?P<lo>[\d.]+)\s+(?P<lu>\w+)\s+"
    r"(?P<mean>[\d.]+)\s+(?P<mu>\w+)\s+(?P<hi>[\d.]+)\s+(?P<hu>\w+)\]"
)


def run_bench(crate: str) -> dict:
    cmd = [
        "cargo", "bench", "--bench", "criterion",
        "--", "--warm-up-time", "1", "--measurement-time", "2", "--sample-size", "10",
    ]
    out = subprocess.run(cmd, cwd=crate, check=True, capture_output=True, text=True)
    # criterion streams timing lines to stderr, not stdout.
    text = out.stdout + out.stderr
    means = {}
    for line in text.splitlines():
        m = LINE_RE.match(line.strip())
        if not m:
            continue
        unit = m.group("mu")
        mean_ns = float(m.group("mean")) * UNIT_NS[unit]
        means[m.group("name")] = mean_ns
    if not means:
        raise RuntimeError("bench_track: no benchmark timing lines parsed from cargo bench output")
    return means


def load_baseline(crate: str) -> dict:
    with open(os.path.join(crate, "benches", "baseline.json")) as f:
        return json.load(f)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--crate", default=".")
    ap.add_argument("--threshold", type=float, default=10.0)
    ap.add_argument("--no-run", action="store_true",
                    help="skip cargo bench; compare baseline to itself (smoke)")
    args = ap.parse_args()

    base = load_baseline(args.crate)
    cur = base if args.no_run else run_bench(args.crate)

    print(f"{'benchmark':26} {'baseline_ns':>12} {'current_ns':>12} {'delta':>9}  verdict")
    rows = []
    for name, bmean in base.items():
        cmean = cur.get(name)
        if cmean is None:
            print(f"{name:26} {bmean:12.2f} {'-':>12} {'MISSING':>9}  !!")
            rows.append((name, bmean, None, None))
            continue
        delta = (cmean - bmean) / bmean * 100.0
        if delta > args.threshold:
            verdict = "REGRESS"
        elif delta < -args.threshold:
            verdict = "improve"
        else:
            verdict = "ok"
        print(f"{name:26} {bmean:12.2f} {cmean:12.2f} {delta:+8.1f}%  {verdict}")
        rows.append((name, bmean, cmean, delta))

    # Append to the (git-ignored) rolling history so trends are visible over time.
    hist = os.path.join(args.crate, "benches", "BENCH_HISTORY.md")
    ts = datetime.now().isoformat(timespec="seconds")
    with open(hist, "a") as f:
        f.write(f"\n## {ts}\n")
        for name, bmean, cmean, delta in rows:
            if cmean is None:
                f.write(f"- {name}: baseline {bmean:.2f}ns, current MISSING\n")
            else:
                f.write(f"- {name}: {bmean:.2f}ns -> {cmean:.2f}ns ({delta:+.1f}%)\n")

    # Exit non-zero on any regression beyond threshold.
    for name, _b, _c, delta in rows:
        if delta is not None and delta > args.threshold:
            print(f"\nREGRESSION: {name} +{delta:.1f}% > {args.threshold}% threshold")
            sys.exit(1)
    print("\nOK: no regression beyond threshold.")


if __name__ == "__main__":
    main()
