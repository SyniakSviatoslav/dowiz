#!/usr/bin/env bash
# B1 prep -- G5b torn-write harness (docs/blueprints/B1-durability-torn-write.md).
# Sector-tear model (SQLite atomiccommit, RESEARCH-NOPOINTERS-SQL section 2.1): after
# each commit k, the pages that changed since commit k-1 (payload appended by k, plus
# the ONE superblock page toggled by k) may independently land as old|new|torn|zeroed
# on a real crash. This harness builds that crashed image directly (no real crash is
# reachable under proot/f2fs nobarrier, LANG-DB section 3) and reopens it with the REAL
# bebop reader (seed binary) AND bench/oracles/scrash.py --parse (unmodified, already
# generic in the generation number). invalid = parse fail, reader trap/timeout, picked
# generation not in {k-1,k}, or a fold mismatch among {reader, --parse, oracle(g)}.
#
# GAP (recorded per the B1 blueprint, not closed here): scrash.bp commits with plain
# st_commit (no msync/fsync -- store.bp:171 st_sync / :176 st_commit_sync exist but are
# unused by the writer, and sys_fsync does not exist yet, bebop.bp `grep fsync` = 0
# hits). Without that ordering barrier there is no in-writer guarantee that a
# generation's payload pages are durable before its superblock page is toggled, so a
# payload page torn/zeroed while ITS OWN (valid-crc) superblock page survives is a
# REAL failure mode here, not a harness bug -- see REPORT-g5b.md.
#
# env: BEBOP_BIN, BEBOP_TMP (=$OUT), TRIALS (default 1000).
# usage: scrash_torn.sh            # store (bebop) trials
#        scrash_torn.sh --sqlite   # sqlite WAL trials (same page-tear methodology)
set -u
cd "$(dirname "$0")/../.."
T=${BEBOP_TMP:-/tmp/opencode}; BB=${BEBOP_BIN:-./bebop.bin}; TRIALS=${TRIALS:-1000}
mkdir -p "$T"

if [ "${1:-}" = "--sqlite" ]; then
  python3 - "$T" "$TRIALS" <<'PY'
import os, random, sqlite3, struct, sys, subprocess

T, TRIALS = sys.argv[1], int(sys.argv[2])
A, C, M = 6364136223846793005, 1442695040888963407, (1 << 64) - 1
def s64(x): x &= M; return x - (1 << 64) if x >> 63 else x
db = os.path.join(T, 'torn.sqlite'); wal = db + '-wal'
NGEN = 300  # commits; small on purpose (filesystem-heaviness rule, AGENTS.md)

for f in (db, wal, db + '-shm', db + '-journal'):
    try: os.remove(f)
    except FileNotFoundError: pass

# one real run: capture wal length right after each commit (append-only while
# under the default auto-checkpoint threshold of 1000 pages -- never hit here).
con = sqlite3.connect(db, isolation_level=None)
con.execute('PRAGMA journal_mode=WAL'); con.execute('PRAGMA synchronous=NORMAL')
con.execute('CREATE TABLE t(id INTEGER PRIMARY KEY, v INTEGER)')
wal_len = [os.path.getsize(wal) if os.path.exists(wal) else 0]
v = 42
for k in range(1, NGEN + 1):
    v = (v * A + C) & M
    con.execute('BEGIN'); con.execute('INSERT INTO t VALUES(?,?)', (k, s64(v))); con.execute('COMMIT')
    wal_len.append(os.path.getsize(wal))
# snapshot the files BEFORE closing: connection close runs a checkpoint that can
# truncate/remove the -wal file, which would erase the very history we need.
wal_bytes = open(wal, 'rb').read()
db_bytes = open(db, 'rb').read()  # main db file: unchanged after the schema commit (no checkpoint)
con.close()

def expected_fold(g):  # acc = (acc*31 + v) & M rolling fold, ascending id (same shape as scrash.py)
    acc = 0
    x = 42
    for i in range(1, g + 1):
        x = (x * A + C) & M; acc = (acc * 31 + s64(x)) & M
    return s64(acc)

random.seed(20260906)
invalid = 0
trial_db = os.path.join(T, 'torn_trial.sqlite'); trial_wal = trial_db + '-wal'
for t in range(TRIALS):
    k = random.randint(1, NGEN)
    lo, hi = wal_len[k - 1], wal_len[k]
    variant = random.choice(['old', 'new', 'torn', 'zeroed'])
    if variant == 'old':
        tail = b''
    elif variant == 'new':
        tail = wal_bytes[lo:hi]
    elif variant == 'zeroed':
        tail = b'\x00' * (hi - lo)
    else:  # torn: a random truncation point inside the new tail (partial last frame)
        cut = random.randint(0, hi - lo)
        tail = wal_bytes[lo:lo + cut]
    open(trial_db, 'wb').write(db_bytes)
    open(trial_wal, 'wb').write(wal_bytes[:lo] + tail)
    for f in (trial_db + '-shm',):
        try: os.remove(f)
        except FileNotFoundError: pass
    bad = False
    try:
        tcon = sqlite3.connect(trial_wal[:-4], timeout=2)
        tcon.execute('PRAGMA journal_mode=WAL')
        rows = tcon.execute('SELECT v FROM t ORDER BY id').fetchall()
        tcon.close()
        g_seen = len(rows)
        acc = 0
        for (val,) in rows: acc = (acc * 31 + val) & M
        fold = s64(acc)
        if g_seen not in (k - 1, k): bad = True
        elif fold != expected_fold(g_seen): bad = True
    except Exception as e:
        bad = True
    if bad:
        invalid += 1
        print('trial', t, 'k', k, 'variant', variant, 'INVALID')
    for f in (trial_db, trial_wal, trial_db + '-shm', trial_db + '-journal'):
        try: os.remove(f)
        except FileNotFoundError: pass

for f in (db, wal, db + '-shm'):
    try: os.remove(f)
    except FileNotFoundError: pass
print('scrash_torn --sqlite: %d trials, %d invalid reopens (NGEN=%d commits sampled)' % (TRIALS, invalid, NGEN))
sys.exit(1 if invalid else 0)
PY
  exit $?
fi

# --- store (bebop) side ---
REPO_ROOT=$(pwd)
NGEN=1000  # bench/vs_rust/std_tests/scrash_small.bp -- small arena (4 MiB), same
           # writer/reader/digest/LCG as scrash.bp, generic bench/oracles/scrash.py reused
./seed/build/seed "$BB" compile bench/vs_rust/std_tests/scrash_small.bp "$T/scrash_small.bin" >/dev/null 2>&1 \
  || { echo "COMPILEFAIL scrash_small"; exit 1; }
( cd "$T" && rm -f scrash.store && "$REPO_ROOT/seed/build/seed" "$T/scrash_small.bin" w >/dev/null )
[ -s "$T/scrash.store" ] || { echo "WRITERFAIL: no scrash.store produced"; exit 1; }

python3 - "$T" "$TRIALS" "$NGEN" <<'PY'
import os, random, struct, subprocess, sys, zlib

T, TRIALS, NGEN = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
REPO = os.getcwd()  # scrash_torn.sh already cd'd to the repo root
SEED_BIN = os.path.join(REPO, 'seed/build/seed')
READER_BIN = os.path.join(T, 'scrash_small.bin')
STORE = os.path.join(T, 'scrash.store')
ORACLE = os.path.join(REPO, 'bench/oracles/scrash.py')
MAGIC = 3554557610294396226
M = (1 << 64) - 1

golden = open(STORE, 'rb').read()

def sb15(gen, root, cursor, live, sup):
    c = [MAGIC, 1, gen, root, cursor, 0, 0, live, sup, 0, 0, 0, 0, 0, 0]
    return c

def sb_bytes(gen, root, cursor, live, sup):
    c = sb15(gen, root, cursor, live, sup)
    raw = struct.pack('<15q', *c)
    crc = zlib.crc32(raw) & 0xffffffff
    page = raw + struct.pack('<q', crc)
    return page + b'\x00' * (4096 - len(page))

def slot_for(k):  # BYTE offset of the superblock page holding generation k (0 or 4096; cell index * 8)
    return 0 if k == 0 else (4096 if k % 2 == 1 else 0)

def sb_state_bytes(k):  # the 4 KiB superblock page as it looked right after commit k (k=0: fresh open)
    if k == 0:
        return sb_bytes(0, 0, 1024, 0, 0)
    return sb_bytes(k, 400 * k + 1020, 1024 + 400 * k, 400 * k, 0)

ZERO_PAGE = b'\x00' * 4096

def page_variant(old_page, new_page, choice, rng):
    if choice == 'old': return old_page
    if choice == 'new': return new_page
    if choice == 'zeroed': return b'\x00' * len(new_page)
    # torn: first-or-last 512 B sector from `new`, rest from `old` (SQLite atomiccommit)
    if rng.choice(('first', 'last')) == 'first':
        return new_page[:512] + old_page[512:]
    return old_page[:-512] + new_page[-512:]

def build_image(k, rng):
    lo_cell, hi_cell = 1024 + 400 * (k - 1), 1024 + 400 * k
    lo, hi = lo_cell * 8, hi_cell * 8
    img = bytearray(golden[:hi])  # unaffected prefix/suffix stay real bytes (append-only property)

    slot = slot_for(k)
    other = 4096 - slot
    new_sb = sb_state_bytes(k)
    old_sb = ZERO_PAGE if k == 1 else sb_state_bytes(k - 2)
    img[slot:slot + 4096] = page_variant(old_sb, new_sb, rng.choice(('old', 'new', 'torn', 'zeroed')), rng)
    # the OTHER superblock slot is untouched by commit k -- it must hold the correct,
    # crc-valid gen k-1 state (NOT `golden`'s bytes there, which are the FINAL run's
    # last write to that slot, many generations ahead of k-1)
    img[other:other + 4096] = sb_state_bytes(k - 1)

    page0 = lo - (lo % 4096)
    p = page0
    while p < hi:
        pend = p + 4096
        new_page = golden[p:pend]
        old_page = golden[p:min(lo, pend)] + b'\x00' * max(0, pend - max(p, lo))
        img[p:pend] = page_variant(old_page, new_page, rng.choice(('old', 'new', 'torn', 'zeroed')), rng)
        p += 4096
    return bytes(img)

rng = random.Random(20260906)
invalid = 0
gen_hist = {}
for t in range(TRIALS):
    k = rng.randint(1, NGEN)
    img = build_image(k, rng)
    with open(STORE, 'wb') as f:
        f.write(img)

    bad = None
    try:
        rd = subprocess.run([SEED_BIN, READER_BIN], cwd=T, capture_output=True, text=True, timeout=5)
        reader_fold = rd.stdout.strip().split('\n')[-1] if rd.returncode == 0 else None
        if reader_fold is None:
            bad = 'reader rc=%d' % rd.returncode
    except subprocess.TimeoutExpired:
        bad = 'reader TRAP/timeout'; reader_fold = None

    if bad is None:
        try:
            par = subprocess.run(['python3', ORACLE, '--parse'], cwd=T, capture_output=True, text=True, timeout=5)
            if par.returncode != 0:
                bad = 'parse fail: %s' % par.stderr.strip()[-150:]
            else:
                g_str, fold_str = par.stdout.strip().split()
                g = int(g_str)
                if g not in (k - 1, k):
                    bad = 'gen %d not in {%d,%d}' % (g, k - 1, k)
                else:
                    exp = subprocess.run(['python3', ORACLE, str(g)], cwd=T, capture_output=True, text=True, timeout=5).stdout.strip()
                    if not (reader_fold == fold_str == exp):
                        bad = 'fold mismatch reader=%s parse=%s oracle=%s (g=%d)' % (reader_fold, fold_str, exp, g)
                    else:
                        gen_hist[g] = gen_hist.get(g, 0) + 1
        except subprocess.TimeoutExpired:
            bad = 'parse TRAP/timeout'

    if bad:
        invalid += 1
        print('trial', t, 'k', k, 'INVALID:', bad)

    try: os.remove(STORE)
    except FileNotFoundError: pass

print('picked generations: %d distinct, k-1/k pairs seen %s' % (len(gen_hist), 'ok' if gen_hist else 'none'))
print('scrash_torn: %d trials, %d invalid reopens (bebop store, NGEN=%d)' % (TRIALS, invalid, NGEN))
sys.exit(1 if invalid else 0)
PY
