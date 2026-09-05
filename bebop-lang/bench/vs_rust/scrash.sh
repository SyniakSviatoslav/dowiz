#!/usr/bin/env bash
# G5 (T113): SIGKILL the scrash writer at a random moment, TRIALS times; every reopened
# store must parse (valid superblock, crc-clean chain of 100*g nodes, other superblock
# at g-1) and the bebop reader's fold must equal the oracle fold for that g.
# env: BEBOP_BIN, BEBOP_TMP, TRIALS (default 100). Prints the generation histogram.
set -u
cd "$(dirname "$0")/../.."
T=${BEBOP_TMP:-/tmp/opencode}; BB=${BEBOP_BIN:-./bebop.bin}; TRIALS=${TRIALS:-100}
./seed/build/seed "$BB" compile bench/vs_rust/std_tests/scrash.bp "$T/scrash.bin" >/dev/null 2>&1 || { echo "COMPILEFAIL scrash"; exit 1; }
rm -f scrash.store; t0=$(date +%s%N); ./seed/build/seed "$T/scrash.bin" w >/dev/null; full=$(( ($(date +%s%N) - t0) / 1000 ))
echo "full writer run: ${full} us for 10^4 generations"
python3 - "$T/scrash.bin" "$TRIALS" "$full" <<'PY'
import os, random, signal, subprocess, sys, time, collections
binp, trials, full = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
random.seed(20260905); hist = collections.Counter(); fails = 0; gens = []
for t in range(trials):
    try: os.remove('scrash.store')
    except FileNotFoundError: pass
    p = subprocess.Popen(['./seed/build/seed', binp, 'w'], stdout=subprocess.DEVNULL)
    time.sleep(random.uniform(0, 0.95 * full / 1e6)); p.send_signal(signal.SIGKILL); p.wait()
    par = subprocess.run(['python3', 'bench/oracles/scrash.py', '--parse'], capture_output=True, text=True)
    if par.returncode != 0: fails += 1; print('trial', t, 'PARSE FAIL', par.stderr.strip()[-200:]); continue
    g, fold = par.stdout.split(); g = int(g)
    rd = subprocess.run(['./seed/build/seed', binp], capture_output=True, text=True).stdout.strip().split('\n')[-1]
    exp = subprocess.run(['python3', 'bench/oracles/scrash.py', str(g)], capture_output=True, text=True).stdout.strip()
    if not (rd == fold == exp): fails += 1; print('trial', t, 'g', g, 'reader', rd, 'parse', fold, 'oracle', exp)
    gens.append(g); hist[g // 1000] += 1
print('generations reached (per 1000):', dict(sorted(hist.items())), 'min', min(gens) if gens else None, 'max', max(gens) if gens else None)
print('scrash: %d trials, %d failures' % (trials, fails)); sys.exit(1 if fails else 0)
PY
