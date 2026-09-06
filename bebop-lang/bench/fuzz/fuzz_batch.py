#!/usr/bin/env python3
"""fuzz_batch.py START N — one shard of the T39 fuzzer in ONE python process.

fuzz.sh used to start three python interpreters per seed (gen.py, shrink.py --classify,
bpref.py); under proot an interpreter start costs 0.2-0.5 s (every import stat is ~1 ms),
which was ~70 % of a seed. Here gen.py runs in-process, bpref runs in a FORKED child
(the same code and exit codes, a real 40 s timeout via waitpid, no interpreter start) and
seed compile/run stay subprocesses (0.1 s each). Output: the same `CAT seed` / `STRAY seed
name` lines as fuzz.sh's one(); the same repro files under $REPROS. env: BEBOP_BIN, REPROS,
TMP (the per-run scratch dir, from fuzz.sh), RUN_T."""
import os
import select
import shutil
import signal
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..', 'tools'))
import gen      # noqa: E402
import shrink   # noqa: E402
import bpref    # noqa: E402

ORACLE_T = 40.0  # the D11-D oracle budget, as in shrink.oracle


def forked_bpref(bp, wd):
    """(rc, stdout, stderr) of tools/bpref.py on bp, computed by a forked child running
    bpref.main() with fds 1/2 on pipes; rc 124 on timeout (child killed)."""
    sys.stdout.flush(); sys.stderr.flush()  # or the child flushes the parent's buffered lines too
    r1, w1 = os.pipe(); r2, w2 = os.pipe()
    pid = os.fork()
    if pid == 0:  # child: become `python3 tools/bpref.py bp` without the interpreter start
        os.close(r1); os.close(r2); os.dup2(w1, 1); os.dup2(w2, 2); os.close(w1); os.close(w2)
        os.chdir(wd); sys.argv = ['bpref.py', bp]
        code = 0
        try:
            bpref.main()
        except SystemExit as ex:
            code = ex.code if isinstance(ex.code, int) else (0 if ex.code is None else 1)
        except BaseException as ex:  # noqa: mirrors bpref's own error path
            sys.stderr.write('bpref error: %s: %s\n' % (type(ex).__name__, ex)); code = 2
        sys.stdout.flush(); sys.stderr.flush()
        os._exit(code & 255)
    os.close(w1); os.close(w2)
    bufs = {r1: [], r2: []}; open_fds = [r1, r2]; deadline = time.time() + ORACLE_T; killed = False
    while open_fds:
        left = deadline - time.time()
        if left <= 0:
            os.kill(pid, signal.SIGKILL); killed = True; break
        ready, _, _ = select.select(open_fds, [], [], left)
        for fd in ready:
            chunk = os.read(fd, 65536)
            if chunk: bufs[fd].append(chunk)
            else: open_fds.remove(fd)
    _, status = os.waitpid(pid, 0)
    for fd in (r1, r2): os.close(fd)
    if killed: return 124, '', ''
    rc = os.WEXITSTATUS(status) if os.WIFEXITED(status) else -os.WTERMSIG(status)
    return rc, b''.join(bufs[r1]).decode(errors='replace'), b''.join(bufs[r2]).decode(errors='replace')


shrink.oracle = forked_bpref


def one(s, tmp, repros):
    d = os.path.join(tmp, str(s)); os.makedirs(d, exist_ok=True)
    bp = os.path.join(d, 'p.bp')
    try:
        src = gen.Gen(s).program(); open(bp, 'w').write(src)
    except Exception:
        print('GENFAIL %d' % s); return
    cat, got, exp = shrink.classify(bp=bp)
    if cat not in ('OK', 'BPREF-DEPTH'):
        with open(os.path.join(repros, '%s-%d.bp' % (cat, s)), 'w') as f:
            f.write('// %s seed=%d expected=%s got=%s\n' % (cat, s, exp, got)); f.write(src)
    print('%s %d' % (cat or 'HARNESS-ERROR', s))
    for name in sorted(os.listdir(d)):  # anything else = a stray file written by a run
        if name not in ('p.bp', 'p.bin', 'p.bin.becache', 'gerr'): print('STRAY %d %s' % (s, name))
    shutil.rmtree(d, ignore_errors=True)


def main():
    # item 1 (process-count gate, retro D13): refuse above the cap when run directly (fuzz.sh
    # already gates before spawning shards; this covers a direct/daemon invocation too).
    root = os.path.join(HERE, '..', '..')
    if not os.environ.get('REAP_GATED') and subprocess.run(['tools/reap.sh', '--check', os.environ.get('PROC_CAP', '30')], cwd=root).returncode != 0:
        sys.exit(97)
    start, n = int(sys.argv[1]), int(sys.argv[2])
    tmp = os.environ.get('TMP') or os.path.join(os.environ.get('BEBOP_TMP', '/tmp/opencode/agentB-fuzz'), 'fuzz.%d' % os.getpid())
    repros = os.environ.get('REPROS', os.path.join(HERE, 'repros'))
    os.makedirs(tmp, exist_ok=True); os.makedirs(repros, exist_ok=True)
    for s in range(start, start + n):
        one(s, tmp, repros); sys.stdout.flush()


if __name__ == '__main__':
    main()
