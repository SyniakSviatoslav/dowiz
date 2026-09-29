# Shared by the eight wlog_* oracles (ROADMAP B8; W-BATGREEN 2026-09-29 -- the gates landed in
# 74906579 with their goldens taken from bench/oracles/rust/src/bin/wlog.rs but with NO
# bench/oracles/<gate>.py, so run_all.sh read MISSING for all eight and L17 was broken).
# The oracle is PRODUCTION Rust: wlog.rs replays wlog.bp's event stream and takes every transition
# decision from dowiz_core::order_machine::assert_transition, deriving the adjacency from it.
# Arguments mirror std_golden.sh's run: n=10000 orders, k=8 events each, then the update phase's
# 5000 appended events. Prints the value of ONE named line; a missing line or a failed cargo run
# exits non-zero with the reason on stderr.
import pathlib, subprocess, sys

def value(key, pick=None):
    r = subprocess.run(["cargo", "run", "--release", "-q", "--bin", "wlog", "--", "10000", "8", "5000"],
                       cwd=pathlib.Path(__file__).resolve().parent / "rust", capture_output=True, text=True)
    if r.returncode:
        sys.stderr.write(r.stderr); sys.exit(1)
    for line in r.stdout.splitlines():
        k, _, v = line.partition(" ")
        if k == key:
            print(v.split(",")[pick] if pick is not None else v); return
    sys.stderr.write("wlog oracle printed no '%s' line:\n%s" % (key, r.stdout)); sys.exit(1)
