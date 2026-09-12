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
# B1 steps 2-3 (this session): scrash_small.bp (the writer this harness actually runs --
# see below) now commits through st_commit_sync (msync the appended range, THEN toggle
# the superblock, THEN msync the superblock page -- store.bp:230), and st_open/
# st_reopen_verify (store.bp:103,131) re-verify the picked superblock's payload arena on
# every reopen, self-healing a torn PAYLOAD page behind a valid-crc superblock (the B1
# gap REPORT-g5b.md documented against the OLDER plain-st_commit writer). The remaining
# 10/50 invalid reopens measured this session (torn.log) were NOT that gap: root-caused
# to two harness-model bugs in build_image below (a torn straddling page could wipe an
# EARLIER, already-durable generation's own root even though the writer never touched
# those bytes again; a superblock 512 B sector tear can coincidentally reconstruct a
# fully valid gen k or gen k-2 header, which the payload loop didn't honour) and one
# oracle bug (bench/oracles/scrash.py --parse has no st_reopen_verify-equivalent
# fallback to the other superblock on a payload-verify failure) -- all three fixed this
# session, see the inline comments at each fix and docs/exp.journal.
# scrash.bp itself (the separate G5 SIGKILL gate) is untouched -- kill -9 has no torn-page
# model, so it has no ordering gap to close; out of scope here (scrash_small.bp:8-9).
# sys_fsync / st_compact directory-fsync remain OUT of scope (deferred to after the A4
# fuzz window per this task's own instructions) -- msync (not fsync) is what
# st_commit_sync uses, sufficient for the in-process/page-cache torn-write model this
# harness proves (a real device flush is unreachable under proot/f2fs nobarrier anyway).
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
( cd "$T" && rm -f scrash.store && "$REPO_ROOT/seed/build/seed" "$T/scrash_small.bin" w > "$T/scrash_commits.log" )
[ -s "$T/scrash.store" ] || { echo "WRITERFAIL: no scrash.store produced"; exit 1; }
# audit: scrash_small.bp now prints "lo hi sbidx" per commit (B1 step 1) -- sanity-check the
# printed range against this harness's own closed-form (build_image below derives the same
# [lo,hi) from the deterministic 400-cells/commit layout independently of this log).
python3 - "$T" "$NGEN" <<'PY'
import sys
T, NGEN = sys.argv[1], int(sys.argv[2])
lines = [l.split() for l in open(T + '/scrash_commits.log') if l.strip()][:NGEN]
bad = 0
for k, (lo, hi, sb) in enumerate(lines, 1):
    lo, hi, sb = int(lo), int(hi), int(sb)
    # +21 cells of PartTab per preceding commit (B5 step 1), on BOTH ends: the low end is
    # page-aligned down, so omitting it puts the expected start a whole 4096-byte page early.
    exp_lo_cell, exp_hi_cell = 1024 + 400 * (k - 1), 1024 + 400 * k
    exp_lo = (exp_lo_cell * 8) - (exp_lo_cell * 8) % 4096
    exp_hi = exp_hi_cell * 8
    exp_sb = 512 if k % 2 == 1 else 0  # sbidx is a CELL index (0/512)
    if (lo, hi, sb) != (exp_lo, exp_hi, exp_sb):
        bad += 1
        if bad <= 3: print('sync-range mismatch commit', k, (lo, hi, sb), 'expected', (exp_lo, exp_hi, exp_sb))
print('sync-range audit: %d/%d commits logged, %d mismatches' % (len(lines), NGEN, bad))
if bad or len(lines) != NGEN: sys.exit(1)
PY
[ $? -eq 0 ] || { echo "SYNCRANGE MISMATCH: harness model diverges from the real writer"; exit 1; }

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
    sb_choice = rng.choice(('old', 'new', 'torn', 'zeroed'))
    sb_sub = rng.choice(('first', 'last')) if sb_choice == 'torn' else None
    if sb_choice == 'torn':
        sb_page = (new_sb[:512] + old_sb[512:]) if sb_sub == 'first' else (old_sb[:-512] + new_sb[-512:])
    else:
        sb_page = page_variant(old_sb, new_sb, sb_choice, rng)
    img[slot:slot + 4096] = sb_page
    # Commit k is DURABLE (a reader will pick a crc-valid gen k) iff its superblock write
    # landed as 'new' in the bytes that matter for validity -- the crc-checked region (cells
    # 0..14 + the crc word = bytes 0..127) sits entirely inside the FIRST 512 B sector, so a
    # sector-granular tear that keeps the first sector from `new` (sb_sub == 'first') is
    # indistinguishable from a whole 'new' superblock write. Harness fix (b), root-caused this
    # session (bench/oracles/scrash.py trial repro): without this, 'torn'+'first' silently
    # produced a fully valid gen-k superblock while the payload loop below still tore gen k's
    # OWN synced range independently -- a state st_commit_sync's ordering (msync payload, THEN
    # toggle the superblock) makes physically impossible, since the superblock write cannot
    # observably durable-land before the payload msync it's ordered after.
    sb_lands_new = sb_choice == 'new' or (sb_choice == 'torn' and sb_sub == 'first')
    # the OTHER superblock slot is untouched by commit k -- it must hold the correct,
    # crc-valid gen k-1 state (NOT `golden`'s bytes there, which are the FINAL run's
    # last write to that slot, many generations ahead of k-1)
    img[other:other + 4096] = sb_state_bytes(k - 1)

    page0 = lo - (lo % 4096)
    p = page0
    while p < hi:
        pend = p + 4096
        # write_start: the first byte in THIS page actually appended by commit k. On the
        # first (straddling) page write_start > p -- bytes [p, write_start) belong to an
        # EARLIER commit's already-durable data (append-only: never rewritten, only ever
        # re-synced as part of this page's dirty range) and must never be torn/zeroed by
        # commit k's own crash window. Harness fix (b), root-caused this session: the old
        # code zero/old-filled that whole prefix as if commit k had never written it, so a
        # 'zeroed'/'torn' draw on this page could wipe an EARLIER, already-durable
        # generation's own root/objects even when commit k's superblock never landed --
        # "tearing a page the writer never wrote between S_{k-1} and S_k" (blueprint section
        # 8 risk table / task classification (b)).
        write_start = max(p, lo)
        new_seg = golden[write_start:pend]
        old_seg = b'\x00' * (pend - write_start)  # unwritten-as-of-k-1 reads zero (sparse ftruncate)
        if sb_lands_new:
            # st_commit_sync ordering: msync(payload range) happens-before the superblock
            # write; if the superblock is observably durable ('new'), every page inside the
            # range it synced (this loop, page-aligned exactly like store.bp:230) is durable
            # too -- never torn/old/zeroed once the later, ordered write is durable.
            seg = new_seg
        else:
            seg = page_variant(old_seg, new_seg, rng.choice(('old', 'new', 'torn', 'zeroed')), rng)
        img[write_start:pend] = seg
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
