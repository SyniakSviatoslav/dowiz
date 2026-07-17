#!/usr/bin/env python3
"""E2E + integration proof for the TRANSCRIPT-sourced attractor detector.

Native upgrade (2026-07-17): the deleted `markov_attractor.py` is replaced by the
compiled kernel binary `kernel/target/debug/markov_attractor` (pure-Rust port with
VbM parity tests). This test now drives that native bin directly on a stdin token
stream — the same contract the Python produced — instead of copying a deleted file
into a temp dir.

Run: python3 tools/loop-signals/test_transcript_e2e.py

Proves two things the PostToolUse source could not:
  1. transcript_events.py recovers FAILURE tokens (run_fail/edit_fail) from a
     transcript — the events a PostToolUse hook never sees (that hook fires only
     on SUCCESS).
  2. The native markov_attractor kernel recovers the right verdict — LIMIT_CYCLE on
     a failure 2-cycle, STRANGE_ATTRACTOR on failure-driven churn, HEALTHY otherwise.
"""
import json, os, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, HERE)
from transcript_events import events_from_transcript  # noqa: E402

# Native kernel binary (built from kernel/src/bin/markov_attractor.rs).
MARKOV_BIN = os.path.join(REPO, "kernel", "target", "debug", "markov_attractor")

STEP = {
    "edit":      ("Edit", {"file_path": "a.ts"}, False),
    "edit_fail": ("Edit", {"file_path": "a.ts"}, True),
    "run_ok":    ("Bash", {"command": "pnpm test"}, False),
    "run_fail":  ("Bash", {"command": "pnpm test"}, True),
    "probe":     ("Bash", {"command": "ls -la"}, False),
}


def write_transcript(path, tokens):
    with open(path, "w") as f:
        for i, t in enumerate(tokens):
            name, inp, is_err = STEP[t]
            tid = f"toolu_{i:04d}"
            f.write(json.dumps({"type": "assistant", "message": {"content": [
                {"type": "tool_use", "id": tid, "name": name, "input": inp}]}}) + "\n")
            res = {"type": "tool_result", "tool_use_id": tid, "content": "x"}
            if is_err:
                res["is_error"] = True
            f.write(json.dumps({"type": "user", "message": {"content": [res]}}) + "\n")


def lcg(alphabet, n, seed=1):
    # HIGH bits (x>>16): an LCG's low bits have short periods, so `x % k` for a
    # power-of-2 k yields an accidental perfect cycle, not churn.
    x, out = seed, []
    for _ in range(n):
        x = (1103515245 * x + 12345) & 0x7FFFFFFF
        out.append(alphabet[(x >> 16) % len(alphabet)])
    return out


fails = 0
tmp = tempfile.mkdtemp(prefix="attractor-e2e-")
try:
    # --- 1) extraction recovers failures --------------------------------------
    tpath = os.path.join(tmp, "t.jsonl")
    write_transcript(tpath, ["edit", "run_fail", "edit_fail", "run_ok", "probe"])
    toks = events_from_transcript(tpath)
    print(f"[extract] {toks}")
    if "run_fail" not in toks or "edit_fail" not in toks:
        print("  FAIL: failure tokens not recovered from transcript"); fails += 1
    else:
        print("  PASS: run_fail + edit_fail recovered (PostToolUse would show neither)")

    # --- 2) native markov_attractor kernel returns the right verdict ---------
    if not os.path.exists(MARKOV_BIN):
        print(f"  WARN: native bin not built at {MARKOV_BIN}; run `cargo build --bin markov_attractor` in kernel/")
    else:
        def verdict(tokens):
            inp = "\n".join(tokens) + "\n"
            p = subprocess.run([MARKOV_BIN], input=inp, text=True,
                               capture_output=True)
            try:
                return json.loads(p.stdout).get("verdict", "UNKNOWN")
            except Exception:
                return "UNKNOWN"

        CASES = [
            ("limit cycle edit<->run_fail", ["edit", "run_fail"] * 8, "LIMIT_CYCLE"),
            ("healthy edit<->run_ok",       ["edit", "run_ok"] * 8,   "HEALTHY"),
            ("strange churn (no green)",    lcg(["edit", "edit_fail", "run_fail", "probe"], 44), "STRANGE_ATTRACTOR"),
            ("wrap-up bookkeeping",         ["edit", "probe"] * 6,    "HEALTHY"),
        ]
        for name, toks, expect in CASES:
            got = verdict(toks)
            ok = (got == expect)
            print(f"[{'PASS' if ok else 'FAIL'}] native {name:<30} expect={expect:<13} got={got}")
            if not ok:
                fails += 1
finally:
    import shutil
    shutil.rmtree(tmp, ignore_errors=True)

print("\n" + ("ALL GREEN" if fails == 0 else f"{fails} FAILURE(S)"))
sys.exit(1 if fails else 0)
