#!/usr/bin/env python3
"""transcript_events.py — reconstruct the TRUE tool-outcome stream from a Claude Code
session transcript (JSONL), INCLUDING failures.

WHY THIS EXISTS
  The obvious event source — a PostToolUse hook — is structurally blind: this harness
  fires PostToolUse ONLY on SUCCESSFUL tool calls. A failed Bash (non-zero exit) or a
  failed Edit (error) never reaches the hook, so `run_fail`/`edit_fail` can never be
  observed there. Verified empirically (2026-07-13): `false`, `sh -c 'exit 42'`, and a
  bad Edit produced ZERO PostToolUse invocations.

  The transcript, by contrast, records every tool_use paired with its tool_result and
  an `is_error` flag — the failures the hook drops. A Stop hook receives `transcript_path`
  and can feed this stream to markov_attractor.analyze(). Same alphabet, same analyzer;
  only the source is fixed.

TOKEN MAPPING (identical to loop-detector.sh's alphabet, now failure-aware)
  Edit/Write/MultiEdit  ok / error   -> edit / edit_fail
  Bash error                          -> run_fail
  Bash ok + verify/progress command   -> run_ok
  Bash ok + benign command            -> probe
  Other tools (Read/Grep/Task/...)    -> skipped (not in the loop alphabet)
"""
from __future__ import annotations
import ast
import json
import re
import sys

VERIFY = re.compile(
    r"(test|build|typecheck|tsc|lint|clippy|cargo|vitest|jest|pytest|playwright|"
    r"pnpm (test|build|typecheck|lint)|make|git commit)", re.I)
EDIT_TOOLS = {"Edit", "Write", "MultiEdit"}


def _as_dict(v):
    """tool_use.input may be a dict (raw JSONL) or a stringified dict (defensive)."""
    if isinstance(v, dict):
        return v
    if isinstance(v, str):
        for parse in (json.loads, ast.literal_eval):
            try:
                d = parse(v)
                if isinstance(d, dict):
                    return d
            except Exception:
                pass
    return {}


def _iter_blocks(msg):
    content = msg.get("content") if isinstance(msg, dict) else None
    if isinstance(content, list):
        for c in content:
            if isinstance(c, dict):
                yield c


def events_from_transcript(path):
    """Return the ordered list of state tokens (with failures) for the whole session."""
    uses = {}            # tool_use_id -> (name, input_dict)
    tokens = []
    for ln in open(path, encoding="utf-8", errors="replace"):
        ln = ln.strip()
        if not ln:
            continue
        try:
            d = json.loads(ln)
        except Exception:
            continue
        for c in _iter_blocks(d.get("message", {})):
            t = c.get("type")
            if t == "tool_use":
                uses[c.get("id")] = (c.get("name", ""), _as_dict(c.get("input")))
            elif t == "tool_result":
                name, inp = uses.get(c.get("tool_use_id"), ("", {}))
                is_err = c.get("is_error") in (True, "true", "True")
                tok = _classify(name, inp, is_err)
                if tok:
                    tokens.append(tok)
    return tokens


def _classify(name, inp, is_err):
    if name in EDIT_TOOLS:
        return "edit_fail" if is_err else "edit"
    if name == "Bash":
        if is_err:
            return "run_fail"
        cmd = inp.get("command", "") if isinstance(inp, dict) else ""
        return "run_ok" if VERIFY.search(cmd) else "probe"
    return None  # non-loop tool (Read/Grep/Task/WebSearch/...) — skipped


def main():
    if len(sys.argv) < 2:
        print("usage: transcript_events.py <transcript.jsonl> [tail_n]", file=sys.stderr)
        return 2
    toks = events_from_transcript(sys.argv[1])
    if len(sys.argv) > 2:
        toks = toks[-int(sys.argv[2]):]
    print("\n".join(toks))
    return 0


if __name__ == "__main__":
    sys.exit(main())
