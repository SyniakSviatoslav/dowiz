#!/bin/sh
# G1 — AN UN-CONSENTED SEND IS UNREPRESENTABLE, NOT DISCOURAGED.
#
# BLUEPRINT-CRM-CONSENT-LOYALTY-2026-09-22 §5 G1. Albania's Law 124/2024
# requires prior explicit consent before a marketing message, and GDPR Art.
# 7(1) requires the controller to be able to DEMONSTRATE it. A rule that lives
# in a document is a rule that gets bypassed at 23:00 on a Friday, so this one
# lives in two places that cannot be talked out of it:
#
#   * THE TYPE. `dowiz_hub::consent::Consented` has private fields and no
#     constructor; the only thing that produces one is `consent::state(...)`
#     returning `Some`. A function that takes `&Consented` therefore CANNOT be
#     called for somebody who never said yes. That half is the compiler's, and
#     `consent.prove.sh` shows it refusing.
#   * THIS GREP. The type cannot help a send that takes a bare `&str`, and all
#     four of this tree's send sites do. So: every call to `whatsapp_text`,
#     `instagram_text` or `notify::telegram` must address either a recipient
#     the VENUE owns, or a thread the CUSTOMER opened, or it must sit in a body
#     that holds a `Consented`.
#
# WHAT IS ALLOWED WITHOUT CONSENT, and it is the blueprint's sentence §3.2:
# anything that is (a) about an order this person placed, sent to the contact
# they gave for it, or (b) a reply inside a conversation this person opened,
# within the channel's own window. Everything else is marketing.
#
#   &wa.to        the venue's own WhatsApp number, from its settings
#   chat.trim()   the venue's own Telegram chat, from its settings
#   &e.to         an outbox Entry, whose id IS an order's id plus the kind
#   &peer         the peer of a thread that reached the inbox webhook
#
# HOW THIS COUNTS. Brace-matched function bodies and the ACTUAL second
# argument of each call, not lines: a file-level grep would pass a file that
# happens to mention consent somewhere else, which is exactly the shape of
# reassurance this repo has been burned by. Python because `sh` cannot match
# braces, and it is already required by `one-venue.sh` beside it.
#
# `sh tools/gates/consent.sh [ROOT]` — ROOT defaults to the repo and is there
# so `consent.prove.sh` can run this same code against a scratch copy holding
# a deliberate defect. Exit 1 when the count rises above the baseline.
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT="${1:-$(cd "$HERE/../.." && pwd)}"
BASELINE="$HERE/consent.baseline"

found=$(ROOT="$ROOT" python3 - <<'PY'
import os, re, glob

ROOT = os.environ['ROOT']

# The doors a message leaves this product through, to a PERSON.
# `instagram_publish` is not here: it posts to the venue's own feed.
SEND = re.compile(r'\b(whatsapp_text|instagram_text|telegram)\s*\(')

# Recipients that are the venue's own, or a thread the customer opened.
ALLOWED = {'&wa.to', 'chat.trim()', '&e.to', '&peer', 'peer', '&peer.clone()'}

def args_of(src, open_paren):
    """The top-level arguments of one call, as written."""
    depth, i, parts, start = 0, open_paren, [], open_paren + 1
    while i < len(src):
        c = src[i]
        if c in '([{':
            depth += 1
        elif c in ')]}':
            depth -= 1
            if depth == 0:
                parts.append(src[start:i])
                return parts
        elif c == ',' and depth == 1:
            parts.append(src[start:i])
            start = i + 1
        i += 1
    return parts

hits = []
for f in sorted(glob.glob(os.path.join(ROOT, 'workers/api/src/**/*.rs'), recursive=True)):
    src = open(f, encoding='utf-8').read()
    for m in re.finditer(r'\bfn (\w+)\s*\(', src):
        b = src.find('{', m.end())
        if b < 0:
            continue
        depth, i = 1, b + 1
        while i < len(src) and depth > 0:
            if src[i] == '{':
                depth += 1
            elif src[i] == '}':
                depth -= 1
            i += 1
        body = src[b:i]
        # A body that HOLDS a `Consented` has the proof in its hand. Nothing
        # else can produce one, so naming the type is not a claim.
        if 'Consented' in body:
            continue
        for s in SEND.finditer(body):
            # The declaration of a send is not a send.
            if body[:s.start()].rstrip().endswith('fn'):
                continue
            args = args_of(body, s.end() - 1)
            if len(args) < 2:
                continue
            to = args[1].strip()
            if to in ALLOWED:
                continue
            hits.append(f"{os.path.relpath(f, ROOT)}::{m.group(1)}  ->  {s.group(1)}(.., {to}, ..)")

print(len(hits))
for h in hits:
    print('  ' + h)
PY
)
n=$(printf '%s\n' "$found" | head -1)
names=$(printf '%s\n' "$found" | tail -n +2)

if [ ! -f "$BASELINE" ]; then
  printf 'sends=%s\n' "$n" > "$BASELINE"
  echo "consent: baseline written at $n"
  exit 0
fi
b=$(awk -F= '/^sends=/{print $2}' "$BASELINE")

if [ "$n" -gt "$b" ]; then
  echo "consent: REFUSED — $n send(s) address a person with no proof they agreed (baseline $b):"
  printf '%s\n' "$names"
  echo "consent: fold the venue's consent log with dowiz_hub::consent::state and hold the"
  echo "consent: Consented it returns, or address the venue's own contact. Law 124/2024"
  echo "consent: requires prior explicit consent, and Art. 7(1) requires you to prove it."
  exit 1
fi
if [ "$n" -lt "$b" ]; then
  echo "consent: $n (baseline $b) — the ratchet has fallen. Lower it to sends=$n in this commit."
  exit 1
fi
echo "consent: $n un-consented send(s) (baseline $b)"
