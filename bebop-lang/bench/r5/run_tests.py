#!/usr/bin/env python3
import subprocess, os, sys, time

COV = "/root/dowiz/bebop-lang/bench/r5/cov"
BIN = os.path.join(COV, "bebopc")
SAMPLES = "/root/dowiz/bebop-lang/samples"
SELFHOST = "/root/dowiz/bebop-lang/selfhost"

# no-arg self-test commands (from main.c dispatch)
noarg = [
    "version", "size", "glyphs", "compute", "pool", "scale", "memristor", "pt",
    "adc", "arena", "calyx", "qtt", "ntt", "ntt32", "hyper", "mem", "lmem",
    "hydra", "verifier", "verify", "vsa", "codegen", "native", "money", "fft",
    "event", "modular", "sort", "token", "checksum", "hex", "trig", "rng",
    "stats", "pid", "markov", "noether", "spectral", "autonomic", "metatest",
    "atomic", "conv", "proof", "nat", "str", "x86_64", "power", "telemetry",
    "fmttest", "comptime", "contract", "termination", "universe", "array",
    "vir", "pac", "effect", "jittable", "supervise", "session", "syscall",
    "typereflect", "atomicjit", "smt", "math", "graph", "chain", "tensor",
    "oracle", "mesh", "gt", "pq", "bench", "zlib", "sha256", "x25519", "tls",
    "aes_gcm",
]

# commands needing args
arg_cmds = [
    ["glyph", "fn"],
    ["glyph", "struct"],
    ["tokens", f"{SAMPLES}/hello.bp"],
    ["parse", f"{SAMPLES}/hello.bp"],
    ["fmt", f"{SAMPLES}/hello.bp"],
    ["expr", "1 + 2 * 3"],
    ["expr", "let y = 2 in y * y"],
    ["expr", "if (1 == 1) then 10 else 20"],
    ["expr", r"(\x:i64. x + 1)(41)"],
    ["compile", f"{SAMPLES}/hello.bp"],
    ["jit", "1 + 2 * 3"],
    ["check", f"{SAMPLES}/hello.bp"],
    ["strict", f"{SELFHOST}/std/dp.bp"],
    ["check", f"{SAMPLES}/theorem-sample.bp"],
    ["check", f"{SAMPLES}/theorem-false.bp"],
    ["morse", "ntt"],
    ["unmorse", "... --- ..."],
    ["unmorse", "quantum hypervector"],
    ["run", f"{SAMPLES}/hello.bp", "main", "0"],
]

results = []
def run(args, timeout=90):
    t0 = time.time()
    try:
        p = subprocess.run([BIN] + args, cwd=COV, capture_output=True, text=True,
                           timeout=timeout)
        return p.returncode, p.stdout, p.stderr, time.time()-t0
    except subprocess.TimeoutExpired:
        return "TIMEOUT", "", "", time.time()-t0
    except Exception as e:
        return f"ERR:{e}", "", "", time.time()-t0

log = []
for c in noarg:
    rc, out, err, dt = run([c])
    tail = (out or err or "").strip().splitlines()[-1:] or [""]
    status = "PASS" if rc == 0 else f"RC={rc}"
    log.append((c, status, dt, tail[-1][:80]))

for a in arg_cmds:
    rc, out, err, dt = run(a)
    tail = (out or err or "").strip().splitlines()[-1:] or [""]
    status = "PASS" if rc == 0 else f"RC={rc}"
    log.append((a[0] + " " + " ".join(a[1:2]), status, dt, tail[-1][:80]))

with open(os.path.join(COV, "run_log.txt"), "w") as f:
    for name, status, dt, tail in log:
        f.write(f"{name:28s} {status:12s} {dt:6.1f}s  {tail}\n")

print(f"TOTAL {len(log)} commands")
fails = [x for x in log if x[1] != "PASS"]
print(f"PASS {len(log)-len(fails)}  FAIL/OTHER {len(fails)}")
for name, status, dt, tail in fails:
    print(f"  {name:28s} {status:12s} {tail}")
