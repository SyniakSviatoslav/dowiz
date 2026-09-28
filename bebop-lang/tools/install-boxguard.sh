#!/bin/bash
# One-time install of the box guard (2026-09-28, after the 11th session death). Run by the OPERATOR:
#   ! bash /root/dowiz/bebop-lang/tools/install-boxguard.sh
# 1. ~/.claude/settings.json: PreToolUse hook no-poll.sh on Bash|Monitor (refuses polling loops,
#    tail -f and Monitor watchers -- they held ~20 of the 34 processes at death #11), and a
#    SessionStart hook that starts tools/procguard.sh (safety valve + flight recorder).
# 2. Starts procguard now.
# The previous settings.json is kept as settings.json.bak-boxguard.
set -eu
S=/root/.claude/settings.json
cp "$S" "$S.bak-boxguard"
python3 - "$S" <<'EOF'
import json, sys
p = sys.argv[1]; s = json.load(open(p)); h = s.setdefault('hooks', {})
def add(ev, entry):
    lst = h.setdefault(ev, [])
    if not any(e.get('hooks') == entry['hooks'] for e in lst): lst.append(entry)
add('PreToolUse', {"matcher": "Bash|Monitor", "hooks": [{"type": "command", "command": "/root/.claude/hooks/no-poll.sh"}]})
add('SessionStart', {"matcher": "startup|resume|compact|clear", "hooks": [{"type": "command",
    "command": "setsid /root/dowiz/bebop-lang/tools/procguard.sh </dev/null >/dev/null 2>&1 & exit 0"}]})
json.dump(s, open(p, 'w'), indent=2)
print("hooks now:", json.dumps(h, indent=1))
EOF
chmod +x /root/.claude/hooks/no-poll.sh /root/dowiz/bebop-lang/tools/procguard.sh
setsid /root/dowiz/bebop-lang/tools/procguard.sh </dev/null >/dev/null 2>&1 &
sleep 1
/root/dowiz/bebop-lang/tools/procguard.sh --status 3
