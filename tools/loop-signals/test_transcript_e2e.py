#!/usr/bin/env python3
"""E2E + integration proof for the TRANSCRIPT-sourced attractor detector.

Run: python3 tools/loop-signals/test_transcript_e2e.py

Proves two things the PostToolUse source could not:
  1. transcript_events.py recovers FAILURE tokens (run_fail/edit_fail) from a
     transcript — the events a PostToolUse hook never sees (this harness fires
     PostToolUse only on SUCCESS).
  2. The wired loop-detector.sh, given a `transcript_path`, reads that stream and
     emits the advisory: a trap on a failure-driven loop, silent on healthy work.
"""
import json, os, shutil, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, HERE)
from transcript_events import events_from_transcript  # noqa: E402

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
    x, out = seed, []
    for _ in range(n):
        x = (1103515245 * x + 12345) & 0x7FFFFFFF
        out.append(alphabet[x % len(alphabet)])
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

    # --- 2) wired hook reads transcript_path and fires ------------------------
    os.makedirs(os.path.join(tmp, "tools/loop-signals"), exist_ok=True)
    os.makedirs(os.path.join(tmp, ".claude/hooks"), exist_ok=True)
    for f in ("markov_attractor.py", "transcript_events.py"):
        shutil.copy(os.path.join(HERE, f), os.path.join(tmp, "tools/loop-signals", f))
    shutil.copy(os.path.join(REPO, ".claude/hooks/loop-detector.sh"),
                os.path.join(tmp, ".claude/hooks/loop-detector.sh"))
    subprocess.run(["git", "init", "-q"], cwd=tmp, check=True)

    def fire(tokens):
        tp = os.path.join(tmp, "run.jsonl")
        write_transcript(tp, tokens)
        shutil.rmtree(os.path.join(tmp, ".claude/.loop-state"), ignore_errors=True)
        inp = {"transcript_path": tp, "tool_name": "Bash",
               "tool_input": {"command": "ls"}, "tool_response": {"stdout": "ok", "stderr": ""}}
        p = subprocess.run(["bash", os.path.join(tmp, ".claude/hooks/loop-detector.sh")],
                           input=json.dumps(inp), text=True, capture_output=True, cwd=tmp)
        try:
            ctx = json.loads(p.stdout).get("hookSpecificOutput", {}).get("additionalContext", "")
        except Exception:
            ctx = ""
        for v in ("LIMIT_CYCLE", "STRANGE_ATTRACTOR"):
            if v in ctx:
                return v
        return "SILENT"

    CASES = [
        ("limit cycle edit<->run_fail", ["edit", "run_fail"] * 8, "LIMIT_CYCLE"),
        ("healthy edit<->run_ok",        ["edit", "run_ok"] * 8,   "SILENT"),
        ("failure churn fires a trap",   lcg(["edit", "edit_fail", "run_fail", "probe"], 36), "TRAP"),
        ("wrap-up bookkeeping",          ["edit", "probe"] * 6,    "SILENT"),
    ]
    for name, toks, expect in CASES:
        got = fire(toks)
        ok = (got in ("LIMIT_CYCLE", "STRANGE_ATTRACTOR")) if expect == "TRAP" else (got == expect)
        print(f"[{'PASS' if ok else 'FAIL'}] hook {name:<30} expect={expect:<13} got={got}")
        if not ok:
            fails += 1
finally:
    shutil.rmtree(tmp, ignore_errors=True)

print("\n" + ("ALL GREEN" if fails == 0 else f"{fails} FAILURE(S)"))
sys.exit(1 if fails else 0)
