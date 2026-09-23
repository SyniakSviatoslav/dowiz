#!/bin/sh
# G1 — THE ORDER KINDS ARE ONE SET, IN RUST AND IN THE BROWSER.
#
# `EventKind::is_order` (crates/dowiz-hub/src/lib.rs) decides which log records
# fold into an order. The browser's replica (workers/api/public/lib/replica.js)
# keeps a HAND COPY of that set as bytes, `ORDER_KINDS`, because the socket
# carries a kind byte and the replica must fold exactly what the server folds.
# `vocab.sh` cannot see it: `gen-vocab` reads `dowiz_core`, and `EventKind` is
# `dowiz_hub`'s. So a kind added to one side only is invisible until a waiter's
# amendment silently fails to appear on a console (BLUEPRINT-POS-THE-ROOM §3.3,
# §6 G1) — or an audit record is folded into a kitchen's queue.
#
# WHAT IT COMPARES. The variant names listed inside `is_order`, mapped to their
# discriminants in the enum, against the numbers inside `new Set([...])` on the
# `ORDER_KINDS` line of every browser file that declares one.
#
# PROVED BOTH WAYS: `sh tools/gates/event-kinds.sh --prove` runs it against a
# copy with a kind added to one side only (must be RED) and to both (GREEN).
set -eu
cd "$(dirname "$0")/../.."
RS=${EVENT_KINDS_RS:-crates/dowiz-hub/src/lib.rs}
JS_DIR=${EVENT_KINDS_JS:-workers/api/public}

if [ "${1:-}" = "--prove" ]; then
  tmp=$(mktemp -d)
  trap 'rm -rf "$tmp"' EXIT
  mkdir -p "$tmp/js"
  cp "$RS" "$tmp/lib.rs"
  grep -rl 'const ORDER_KINDS' "$JS_DIR" | head -1 | xargs -I{} cp {} "$tmp/js/replica.js"
  run() { EVENT_KINDS_RS="$tmp/lib.rs" EVENT_KINDS_JS="$tmp/js" sh "$0" >/dev/null 2>&1; }
  run || { echo "event-kinds --prove: the tree itself is RED; prove from a green tree"; exit 1; }
  # A kind the browser folds and the server does not.
  sed -i 's/const ORDER_KINDS = new Set(\[/const ORDER_KINDS = new Set([42, /' "$tmp/js/replica.js"
  if run; then echo "event-kinds --prove: FAILED — a browser-only kind went unnoticed"; exit 1; fi
  echo "event-kinds --prove: a kind on one side only is RED"
  cp "$RS" "$tmp/lib.rs"
  grep -rl 'const ORDER_KINDS' "$JS_DIR" | head -1 | xargs -I{} cp {} "$tmp/js/replica.js"
  # A kind the server folds and the browser does not: drop the last number.
  sed -i -E 's/(const ORDER_KINDS = new Set\(\[[^]]*), [0-9]+\]/\1]/' "$tmp/js/replica.js"
  if run; then echo "event-kinds --prove: FAILED — a server-only kind went unnoticed"; exit 1; fi
  echo "event-kinds --prove: a kind the browser forgot is RED"
  cp "$RS" "$tmp/lib.rs"
  grep -rl 'const ORDER_KINDS' "$JS_DIR" | head -1 | xargs -I{} cp {} "$tmp/js/replica.js"
  run || { echo "event-kinds --prove: FAILED — the restored copy is not green"; exit 1; }
  echo "event-kinds --prove: both sides equal is GREEN"
  exit 0
fi

rust=$(python3 - "$RS" <<'PY'
import re, sys
src = open(sys.argv[1], encoding='utf-8').read()
enum = re.search(r'pub enum EventKind \{(.*?)\n\}', src, re.S).group(1)
num = {m.group(1): int(m.group(2)) for m in re.finditer(r'^\s*(\w+)\s*=\s*(\d+),', enum, re.M)}
body = re.search(r'pub fn is_order\(self\) -> bool \{(.*?)\n    \}', src, re.S).group(1)
names = re.findall(r'EventKind::(\w+)', body)
if not names:
    print('PARSE'); sys.exit(0)
print(' '.join(str(n) for n in sorted(num[x] for x in names)))
PY
)
[ "$rust" != "PARSE" ] && [ -n "$rust" ] || { echo "event-kinds: could not read is_order from $RS — the parse broke, not the rule"; exit 1; }

files=$(grep -rl 'const ORDER_KINDS' "$JS_DIR" || true)
[ -n "$files" ] || { echo "event-kinds: no ORDER_KINDS copy under $JS_DIR — the parse broke, not the rule"; exit 1; }

rc=0
for f in $files; do
  js=$(grep -o 'const ORDER_KINDS = new Set(\[[^]]*\])' "$f" | grep -o '[0-9][0-9]*' | sort -n | tr '\n' ' ' | sed 's/ $//')
  if [ "$js" != "$rust" ]; then
    echo "event-kinds: REFUSED — $f folds kinds [$js], EventKind::is_order is [$rust]"
    rc=1
  fi
done
[ $rc -eq 0 ] && echo "event-kinds: is_order is [$rust] and every browser copy agrees ($(echo "$files" | wc -l | tr -d ' ') file(s))"
exit $rc
