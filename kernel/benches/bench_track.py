#!/usr/bin/env python3
"""bench_track — thin delegation wrapper around the native benchmark tracker.

This script is intentionally a THIN WRAPPER. All benchmark parsing, baseline
comparison, and regression gating live in the pure-Rust, zero-dep native
tracker at `tools/telemetry/native-trackers` (built with
`cargo build --release` there). Per AGENTS.md, native telemetry + benchmarks
are mandatory per wave, so the Rust tracker is the ONLY path — there is no
python fallback parser (the old python parser duplicated the Rust logic and
drifted, which is what produced the spurious FAIL-CLOSED "MISSING" reads for
empirical_identify / token_bucket).

What it does:
  - Locates the `native-trackers` binary (PATH or the canonical repo location).
  - Delegates: `native-trackers bench <crate> [--threshold N] [--bench <name>]`.
  - Forwards the native tracker's exit code unchanged (0 ok, 1 regression,
    2 usage/IO error). If the binary isn't built, exits 2 with a clear message.

Usage:
    python3 benches/bench_track.py                 # run + compare + log (via native)
    python3 benches/bench_track.py --threshold 15  # looser regression gate
    python3 benches/bench_track.py --bench criterion  # specific bench target
"""
import argparse
import os
import shutil
import subprocess
import sys

BIN_NAME = "native-trackers"


def _repo_root():
    # benches/bench_track.py -> ../.. == crate root; ../../.. == repo root
    return os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


def _native_dir():
    return os.path.join(_repo_root(), "tools", "telemetry", "native-trackers")


def _find_native():
    """Return path to the native-trackers binary, or None if not built."""
    bin_path = shutil.which(BIN_NAME)
    if bin_path is not None:
        return bin_path
    cand = os.path.join(_native_dir(), "target", "release", BIN_NAME)
    if os.path.isfile(cand):
        return cand
    return None


def main() -> None:
    ap = argparse.ArgumentParser(
        description="Delegate benchmark tracking to the native Rust tracker.")
    ap.add_argument("--crate", default=".",
                    help="crate directory to benchmark (default: current dir)")
    ap.add_argument("--threshold", type=float, default=10.0,
                    help="regression gate percentage (default: 10)")
    ap.add_argument("--bench", default=None,
                    help="specific bench target name (default: all in crate)")
    ap.add_argument("--build-native", action="store_true",
                    help="build native-trackers if missing, then run")
    args = ap.parse_args()

    bin_path = _find_native()
    if bin_path is None:
        native_dir = _native_dir()
        if args.build_native and os.path.isdir(native_dir):
            print(f"bench_track: building native-trackers in {native_dir} ...",
                  file=sys.stderr)
            rc = subprocess.call(["cargo", "build", "--release"], cwd=native_dir)
            if rc != 0:
                print("bench_track: native-trackers build failed", file=sys.stderr)
                sys.exit(2)
            bin_path = _find_native()
        if bin_path is None:
            print(
                "bench_track: native-trackers binary not found.\n"
                f"  Build it: cd {native_dir} && cargo build --release\n"
                "  (native tracking is the mandatory per-wave path; no python "
                "fallback parser exists by design.)",
                file=sys.stderr)
            sys.exit(2)

    cmd = [bin_path, "bench", args.crate, "--threshold", str(args.threshold)]
    # Pick the bench target: agent-adapters uses `fuel_bench`; everything else
    # uses the default `criterion` (kernel/llm/crates-bebop). Callers may also
    # pass `--bench <name>` explicitly to override.
    if args.bench is not None:
        bench = args.bench
    elif "agent-adapters" in os.path.abspath(args.crate):
        bench = "fuel_bench"
    else:
        bench = None  # native-trackers defaults to `criterion`
    if bench is not None:
        cmd += ["--bench", bench]
    sys.exit(subprocess.call(cmd))


if __name__ == "__main__":
    main()
