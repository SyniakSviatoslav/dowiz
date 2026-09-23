#!/bin/sh
# G1's proof, the grep half: a gate is triggered before it is trusted.
#
# Runs `consent.sh` against scratch copies of the Worker:
#   1. clean                                   -> must pass (exit 0)
#   2. a bare `whatsapp_text(.., "+355…", ..)` -> must refuse (exit 1): an offer
#      to a number the VENUE chose, with no `Consented` in hand
#   3. the same send inside a body holding a `Consented` -> must pass again
# The type half of G1 is the compiler's: `Consented` has private fields and no
# constructor (`crates/dowiz-hub/src/consent.rs`), and a `campaign::Entry` cannot
# be built without one.
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
SCRATCH=$(mktemp -d)
trap 'rm -rf "$SCRATCH"' EXIT
fail=0

copy() {
  rm -rf "$SCRATCH/r"; mkdir -p "$SCRATCH/r/workers/api"
  cp -r "$REPO/workers/api/src" "$SCRATCH/r/workers/api/src"
}
run() { sh "$HERE/consent.sh" "$SCRATCH/r" >"$SCRATCH/out" 2>&1; echo $?; }

copy
rc=$(run); echo "prove: clean -> rc=$rc (want 0)"
[ "$rc" -eq 0 ] || { cat "$SCRATCH/out"; fail=1; }

copy
cat >> "$SCRATCH/r/workers/api/src/channels.rs" <<'RS'

async fn blast_offer(wa: &WhatsApp) {
    let _ = whatsapp_text(wa, "+355691234567", "20% off tonight").await;
}
RS
rc=$(run); echo "prove: un-consented offer -> rc=$rc (want 1): $(sed -n 2p "$SCRATCH/out")"
[ "$rc" -eq 1 ] || fail=1

copy
cat >> "$SCRATCH/r/workers/api/src/channels.rs" <<'RS'

async fn offer_to(wa: &WhatsApp, c: &dowiz_hub::consent::Consented, to: &str) {
    let _ = whatsapp_text(wa, to, "20% off tonight").await;
}
RS
rc=$(run); echo "prove: the same send holding a Consented -> rc=$rc (want 0)"
[ "$rc" -eq 0 ] || { cat "$SCRATCH/out"; fail=1; }

[ $fail -eq 0 ] && echo "consent.prove: the gate fires in both directions" || echo "consent.prove: FAILED"
exit $fail
