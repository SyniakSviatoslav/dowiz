#!/usr/bin/env python3
"""token_ledger.py (operator token-economy priority 2026-09-07): EXACT token accounting of a Claude Code session
from its transcript jsonl (default: newest in ~/.claude/projects/-root). Prints per-turn context, alert-turn cost,
tool-result bulk, Agent prompt bulk -- the numbers the mechanical rules are judged by."""
import json, glob, os, sys
S = sys.argv[1] if len(sys.argv) > 1 else sorted(glob.glob(os.path.expanduser('~/.claude/projects/-root/*.jsonl')), key=os.path.getmtime)[-1]
inp = cr = cc = out = turns = 0; ctx = []; alert = 0; alert_ctx = 0; last_alert = False
tool_chars = 0; n_res = 0; big = []; ap = []; sends = []
for line in open(S):
    try: j = json.loads(line)
    except Exception: continue
    m = j.get('message') or {}
    if j.get('type') == 'user':
        s = json.dumps(m.get('content')); last_alert = ('BOX ALERT' in s) or ('Monitor event' in s); alert += last_alert
    if not isinstance(m, dict): continue
    u = m.get('usage')
    if u:
        c = u.get('input_tokens', 0) + u.get('cache_read_input_tokens', 0) + u.get('cache_creation_input_tokens', 0)
        inp += u.get('input_tokens', 0); cr += u.get('cache_read_input_tokens', 0); cc += u.get('cache_creation_input_tokens', 0)
        out += u.get('output_tokens', 0); turns += 1; ctx.append(c); alert_ctx += c if last_alert else 0
    if isinstance(m.get('content'), list):
        for c in m['content']:
            if not isinstance(c, dict): continue
            if c.get('type') == 'tool_result':
                t = c.get('content'); s = t if isinstance(t, str) else json.dumps(t); tool_chars += len(s); n_res += 1; big.append(len(s))
            if c.get('type') == 'tool_use':
                i = c.get('input', {})
                if c.get('name') == 'Agent': ap.append(len(i.get('prompt', '')))
                if c.get('name') == 'SendMessage': sends.append(len(i.get('message', '')))
big.sort(reverse=True)
print(f"session {os.path.basename(S)}")
print(f"turns={turns} uncached_input={inp} cache_read={cr} cache_create={cc} output={out}")
print(f"context/turn: last50 avg={sum(ctx[-50:])//max(1,len(ctx[-50:]))} max={max(ctx) if ctx else 0}")
print(f"monitor/alert turns={alert} context_tokens_spent_on_them={alert_ctx}")
print(f"tool_results={n_res} ~tokens={tool_chars//4}  (>2k chars: n={sum(1 for b in big if b>2000)} ~tokens={sum(b for b in big if b>2000)//4} top5={big[:5]})")
print(f"Agent launches={len(ap)} prompt ~tokens={sum(ap)//4} avg={sum(ap)//max(1,len(ap))//4}; resumes={len(sends)} ~tokens={sum(sends)//4}")
