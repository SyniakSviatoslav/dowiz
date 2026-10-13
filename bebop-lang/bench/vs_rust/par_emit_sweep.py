#!/usr/bin/env python3
"""par_emit_sweep.py (W-R7, R-GRAPH B3, 2026-10-11): parallel emit == serial emit, program by program.

  python3 bench/vs_rust/par_emit_sweep.py --cand <bin> [--base <bin>] [--runs 2] [--w 3]

Every program under selfhost/*.bp selfhost/*/*.bp bench/vs_rust/std_tests bench/parity_constructs and its
neg/ (the sweep memory sweep-122-is-not-enough asks for; 439 files on 2026-10-11) is compiled COLD
(out, .dag, .becache, .use removed first) by:
  base       --base (default ./bebop.bin, the promoted compiler)    -- the control
  cand W=0   BEBOP_CC_W=0                                           -- the serial emit of the candidate
  cand W=w   BEBOP_CC_W=w BEBOP_CC_ALWAYS=1, --runs times           -- workers on EVERY program, even a
             3-fn one (T-4's threshold would send it serial), so the worker path and the abort path
             (every neg/ program) are exercised, and twice because which worker takes which fn races.
Compared per program: exit code, stdout, stderr, md5 of the .bin against base; md5 of the .dag memo image
among the candidate's arms (the image records the compiler's digest, so base's differs by design). Prints one
line per disagreement (BOTH sides) and

  par_emit_sweep: <same>/<programs> identical  base=<md5> cand=<md5> W=<w> runs=<r>  PASS|FAIL

PASS needs every program identical on every column across all four. A program that compiles with NO
binary (rc 0, no file) is a disagreement, never a match.
"""
import glob, hashlib, os, subprocess, sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
os.chdir(ROOT)


def opt(a, k, d):
    return a[a.index(k) + 1] if k in a else d


def md5f(p):
    try:
        return hashlib.md5(open(p, "rb").read()).hexdigest()[:8]
    except OSError:
        return "absent"


def one(seed, b, src, out, env):
    for f in (out, out + ".dag", out + ".becache", out + ".use"):
        try:
            os.remove(f)
        except FileNotFoundError:
            pass
    p = subprocess.run([seed, b, "compile", src, out], env=env, capture_output=True, timeout=600)
    return (p.returncode, p.stdout.decode(errors="replace"), p.stderr.decode(errors="replace"), md5f(out), md5f(out + ".dag"))


def main(a):
    cand = opt(a, "--cand", None)
    base = opt(a, "--base", "./bebop.bin")
    runs = int(opt(a, "--runs", "2"))
    w = opt(a, "--w", "3")
    if not cand or not os.path.exists(cand) or not os.path.exists(base):
        print(f"par_emit_sweep: REFUSED -- --cand {cand} / --base {base} missing"); return 2
    seed = "./seed/build/seed"
    tmp = os.environ.get("BEBOP_TMP", "/tmp/opencode") + "/pesweep"
    os.makedirs(tmp, exist_ok=True)
    progs = sorted(glob.glob("selfhost/*.bp") + glob.glob("selfhost/*/*.bp") + glob.glob("bench/vs_rust/std_tests/*.bp")
                   + glob.glob("bench/parity_constructs/*.bp") + glob.glob("bench/parity_constructs/neg/*.bp"))
    if not progs:
        print("par_emit_sweep: NOT MEASURED -- the corpus scan found 0 programs"); return 2
    env0 = {k: v for k, v in os.environ.items() if not k.startswith("BEBOP_CC_")}
    arms = [("base", base, dict(env0)), ("cand W=0", cand, dict(env0, BEBOP_CC_W="0"))]
    arms += [(f"cand W={w} run{r + 1}", cand, dict(env0, BEBOP_CC_W=w, BEBOP_CC_ALWAYS="1")) for r in range(runs)]
    same = 0
    rcs = {}
    for i, p in enumerate(progs):
        res = [(name, one(seed, b, p, f"{tmp}/o{i}.bin", env)) for name, b, env in arms]
        ref = res[0][1]
        # the .dag image carries the COMPILER's digest, so base and cand differ there by design:
        # the dag column is compared among the candidate's arms only (W=0 is its reference)
        dref = res[1][1][4]
        bad = [(name, r) for name, r in res[1:] if r[:4] != ref[:4] or r[4] != dref]
        if ref[0] == 0 and ref[3] == "absent":
            bad.append(("base", ref))
        rcs[ref[0]] = rcs.get(ref[0], 0) + 1
        if not bad:
            same += 1
            continue
        for name, r in bad:
            cols = [c for c, x, y in zip(("rc", "stdout", "stderr", "bin", "dag"), ref, r) if x != y]
            print(f"  DIFF {p}: {name} differs in {cols}: base=(rc {ref[0]} bin {ref[3]} dag {ref[4]} err {ref[2][-120:]!r})"
                  f" {name}=(rc {r[0]} bin {r[3]} dag {r[4]} err {r[2][-120:]!r})")
    ok = same == len(progs)
    print(f"  base rc histogram: {dict(sorted(rcs.items()))}")
    print(f"par_emit_sweep: {same}/{len(progs)} identical  base={md5f(base)} cand={md5f(cand)} W={w} runs={runs}  {'PASS' if ok else 'FAIL'}")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
