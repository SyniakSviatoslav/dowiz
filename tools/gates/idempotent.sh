#!/bin/sh
# F34 — EVERY ROUTE A PHONE CAN REPLAY IS IDEMPOTENT ON THE SERVER.
#
# THE TWO HALVES OF ONE FEATURE, AND THEY LIVE IN DIFFERENT LANGUAGES.
# `workers/api/public/lib/outbox.js` keeps a courier's tap when the network is
# gone and replays it when the signal returns. That replay is SAFE without any
# server support -- a repeated transition is an illegal edge and the FSM refuses
# it -- but it is not TRUTHFUL: a courier whose first call landed and whose
# response was lost is told the order changed while they were away, and on
# `deliver` the cash shortfall goes with it.
#
# So a queueable route needs `idempotency::guard`, and the two halves are
# written by different people at different times in different files. This is
# what keeps them from drifting apart: every route the client can queue must
# name a guard on the server.
#
# HOW IT PAIRS THEM. The client's queueable routes are the `tapped(...)` calls
# in the courier app, which carry a path like `/courier/orders/${id}/pickup`;
# the last segment is the verb. The server's guards are the `"courier.<verb>"`
# route constants passed to `idempotency::guard`. A verb on one side with no
# partner on the other is the finding.
#
# WHAT IT CANNOT SEE, said plainly: a route queued by some future surface that
# does not use `tapped`, and a guard whose constant does not start `courier.`.
# It is a pairing check for the one queue that exists, not a theory of every
# retry. `owner.order_action` is guarded too and is deliberately NOT counted
# here -- no client queues it yet, and a gate that counts what nothing does
# would be measuring an intention.
set -eu
cd "$(dirname "$0")/../.."
APP=workers/api/public/courier/app.js
SRV=workers/api/src/courier.rs

[ -f "$APP" ] || { echo "idempotent: $APP is gone; this gate no longer knows what it guards"; exit 1; }

# The verb at the end of every path handed to `tapped(`.
queued=$(grep -o "tapped(\`[^\`]*\`" "$APP" | sed 's|.*/||; s|`$||' | sort -u)
[ -n "$queued" ] || { echo "idempotent: no tapped() routes found — the parse broke, not the feature"; exit 1; }

# The verbs the server guards. READ FROM INSIDE THE `guard(` CALL, not from
# anywhere the string appears: `courier.claim` is a `loud!` place name, and
# counting it would have this gate report a guard that does not exist.
guarded=$(grep -A9 'idempotency::guard(' "$SRV" \
  | grep -o '"courier\.[a-z_]*"' | tr -d '"' | sed 's/^courier\.//' | sort -u)

missing=""
for v in $queued; do
  printf '%s\n' "$guarded" | grep -qx "$v" || missing="$missing  $v: queued by $APP, no idempotency::guard in $SRV\n"
done
# AND THE OTHER DIRECTION, because a guard for a route nothing queues is a
# claim nobody checks; it is reported, not refused.
for v in $guarded; do
  printf '%s\n' "$queued" | grep -qx "$v" || echo "idempotent: note — $SRV guards '$v', which no client queues"
done

n=$(printf "$missing" | grep -c . || true)
echo "idempotent: $(printf '%s\n' "$queued" | wc -l | tr -d ' ') queueable route(s), $n without a server guard"
if [ "$n" -gt 0 ]; then
  echo "idempotent: REFUSED — a phone can replay these and the server cannot tell it apart from a new tap."
  printf "$missing"
  exit 1
fi
exit 0
