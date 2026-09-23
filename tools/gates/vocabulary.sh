#!/bin/sh
# F4 — THE CLIENTS SPEAK THE KERNEL'S VOCABULARY.
#
# THE DEFECT, caught before it shipped rather than after. `OrderStatus` in
# `crates/dowiz-core/src/order_machine.rs` has twelve members. Every browser
# surface carries a HAND COPY of that list -- three languages in
# `admin/i18n.js`, a colour per status in three stylesheets -- and two members
# were missing from all of them: `REFUNDING` and `COMPENSATED_REFUND`.
#
# The FSM has reached both since `allowed_next` was written. Nothing emits them
# yet (there is no refund route), so the day one lands, three consoles would
# have shown an Albanian restaurant owner the raw word COMPENSATED_REFUND, in
# an unstyled cell, with no colour rule to draw it.
#
# This is the whole cost of "shared types that stop at the WASM boundary": the
# kernel is the single authority for the server and the clients re-type it. The
# blueprint's answer is to GENERATE the vocabulary
# (`BLUEPRINT-ARCHITECTURE-EVOLUTION-2026-09-22.md` P4). This gate is the cheap
# half of it: until the generator exists, a member added to the FSM and not to
# the clients is refused here.
#
# WHAT IT DOES NOT CHECK: whether a translation is correct, or whether a colour
# is a good one. It checks that every status the kernel can produce has a word
# and a colour on every surface that draws one.
set -eu
cd "$(dirname "$0")/../.."

FSM=crates/dowiz-core/src/order_machine.rs
statuses=$(grep -oE 'Self::[A-Za-z]+ => "[A-Z_]+"' "$FSM" | sed 's/.*"\(.*\)"/\1/' | sort -u)
n=$(printf '%s\n' "$statuses" | wc -l | tr -d ' ')
[ "$n" -ge 10 ] || { echo "vocabulary: only $n statuses parsed from $FSM — the parse broke, not the vocabulary"; exit 1; }

missing=0
for s in $statuses; do
  for f in workers/api/public/admin/i18n.js; do
    # Three languages in one file: the word must appear as many times as there
    # are `st:{` blocks, or one language is quietly short.
    langs=$(grep -c 'st:{' "$f")
    have=$(grep -oE "\b$s:" "$f" | wc -l | tr -d ' ')
    if [ "$have" -lt "$langs" ]; then
      echo "vocabulary: $s has $have of $langs translations in $f"
      missing=$((missing + 1))
    fi
  done
  for f in workers/api/public/admin/admin.css \
           workers/api/public/platform/platform.css \
           workers/api/public/courier/courier.css \
           workers/api/public/room/room.css; do
    grep -q -- "--st-$s:" "$f" || { echo "vocabulary: $s has no colour in $f"; missing=$((missing + 1)); }
  done
done

if [ "$missing" -gt 0 ]; then
  echo "vocabulary: REFUSED — $missing gap(s) between the kernel's OrderStatus and the surfaces that draw it."
  echo "vocabulary: add the word and the colour, or generate them (blueprint P4)."
  exit 1
fi
echo "vocabulary: $n kernel statuses, all present in 3 languages and 3 stylesheets"
