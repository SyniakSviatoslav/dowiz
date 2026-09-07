# B1 std_golden gate oracle (docs/blueprints/B1-durability-torn-write.md): re-runs
# bench/vs_rust/scrash_torn.sh at TRIALS=50 (same trial count as the std_golden.sh
# `scrash_torn` gate) and prints the invalid-reopen count. Golden = 0. There is no
# closed-form/pure-math shortcut for "0 invalid reopens" (unlike scrash.py's fold, which
# is a pure LCG computation) -- proving durability IS running the harness, so this oracle
# genuinely re-executes it rather than recomputing an expected constant. run_all.sh invokes
# this with cwd = repo root (it cd's there before calling any bench/oracles/*.py), so plain
# relative paths work; BEBOP_TMP/BEBOP_BIN follow scrash_torn.sh's own env defaults unless
# already set by the caller.
import os, subprocess, sys

env = dict(os.environ)
env.setdefault('TRIALS', '50')
env.setdefault('BEBOP_TMP', '/tmp/opencode')
env.setdefault('BEBOP_BIN', './bebop.bin')
try:
    r = subprocess.run(['bash', 'bench/vs_rust/scrash_torn.sh'], env=env,
                        capture_output=True, text=True, timeout=180)
    last = r.stdout.strip().split('\n')[-1] if r.stdout.strip() else ''
    # "scrash_torn: 50 trials, N invalid reopens (bebop store, NGEN=1000)"
    n = int(last.split()[3])
except Exception as e:
    print('ERR', e, file=sys.stderr)
    n = -1
print(n)
