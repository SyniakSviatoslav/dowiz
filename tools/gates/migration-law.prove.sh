#!/bin/sh
# S7c's proof: the gate is triggered before it is trusted. Runs `migration-law.sh` against scratch
# copies -- clean (must pass), a stock kind that gained a field with no law registered (must
# refuse), a block schema string edited (must refuse), a register naming a test nobody wrote
# (must refuse), an ORDER-LOG kind's writer gaining a field, the kernel snapshot gaining a key, a ninth
# kind byte and a writer the gate cannot place in a fn (each must refuse), and the field change done RIGHT -- a new law test plus a v+1 line (must pass).
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
SCRATCH=$(mktemp -d)
trap 'rm -rf "$SCRATCH"' EXIT
fail=0

copy() {
  rm -rf "$SCRATCH/r"; mkdir -p "$SCRATCH/r/crates/dowiz-hub" "$SCRATCH/r/workers/api" "$SCRATCH/r/tools/gates"
  cp -r "$REPO/crates/dowiz-hub/src" "$SCRATCH/r/crates/dowiz-hub/src"
  cp -r "$REPO/workers/api/src" "$SCRATCH/r/workers/api/src"
  mkdir -p "$SCRATCH/r/crates/dowiz-core" && cp -r "$REPO/crates/dowiz-core/src" "$SCRATCH/r/crates/dowiz-core/src"
  cp "$HERE/migration-law.registry" "$SCRATCH/r/tools/gates/"
}
edit() { # file old new
  python3 - "$1" "$2" "$3" <<'PY'
import sys
p, old, new = sys.argv[1:]
s = open(p, encoding='utf-8').read()
assert old in s, f'{old!r} moved in {p}; update the proof'
open(p, 'w', encoding='utf-8').write(s.replace(old, new, 1))
PY
}
run() { # label want
  sh "$HERE/migration-law.sh" "$SCRATCH/r" >"$SCRATCH/out" 2>&1; rc=$?
  [ $rc -eq "$2" ] || { echo "prove: $1 -> rc=$rc, want $2:"; cat "$SCRATCH/out"; fail=1; }
  echo "prove: $1 -> rc=$rc (want $2): $(grep -m1 REFUSED "$SCRATCH/out" || tail -1 "$SCRATCH/out")"
}

copy; run "clean" 0

copy
edit "$SCRATCH/r/crates/dowiz-hub/src/stock/codec.rs" \
  'r#"{{"k":"received","item":"{}","qty":{qty}}}"#' 'r#"{{"k":"received","item":"{}","qty":{qty},"lot":""}}"#'
run "received gained a field, no law" 1

copy
edit "$SCRATCH/r/crates/dowiz-hub/src/block/schema.rs" 'qty:i64:0,gen:i64:7' 'qty:i64:0,gen:i64:7,lot:i64:0'
run "a block schema string changed, no law" 1

copy
edit "$SCRATCH/r/tools/gates/migration-law.registry" 'stock::migration_tests::wasted_gained_by_and_the_laws_hold' 'stock::migration_tests::nobody_wrote_this'
run "a register naming a test nobody wrote" 1

copy
edit "$SCRATCH/r/tools/gates/migration-law.registry" 'stock::migration_tests::stocktake_gained_by_and_the_laws_hold' 'stock::migration_tests::every_kind_round_trips'
run "two versions reusing one law test" 1

# Done right: the field, a law test for it, and the v2 line.
copy
edit "$SCRATCH/r/crates/dowiz-hub/src/stock/codec.rs" \
  'r#"{{"k":"received","item":"{}","qty":{qty}}}"#' 'r#"{{"k":"received","item":"{}","qty":{qty},"lot":""}}"#'
printf '\n#[test]\nfn received_gained_lot_and_the_laws_hold() {}\n' >> "$SCRATCH/r/crates/dowiz-hub/src/stock/migration_tests.rs"
printf 'stock.received 2 item,qty,lot stock::migration_tests::received_gained_lot_and_the_laws_hold\n' >> "$SCRATCH/r/tools/gates/migration-law.registry"
run "the field shipped with its law and a v2 line" 0

# ── THE ORDER LOG (W-P0-LOG) ──
copy
edit "$SCRATCH/r/crates/dowiz-hub/src/shred.rs" '"at":{now_ms},"reason":"shred"}}' '"at":{now_ms},"reason":"shred","lot":0}}'
run "log.forgotten's writer gained a field, no law" 1

copy
edit "$SCRATCH/r/workers/api/src/command/kitchen_ack.rs" 'hub.append(dowiz_hub::EventKind::Noted' 'let _ = json!({"plate": 1}); hub.append(dowiz_hub::EventKind::Noted'
run "a Noted writer wrote a new key, no law" 1

copy
edit "$SCRATCH/r/crates/dowiz-core/src/json_api.rs" '(String::from("cash_pay_with"), Value::from(o.cash_pay_with.clone())),' '(String::from("cash_pay_with"), Value::from(o.cash_pay_with.clone())), (String::from("eta_ms"), Value::from(0)),'
run "the kernel snapshot gained a key, no law" 1

copy
edit "$SCRATCH/r/crates/dowiz-hub/src/event.rs" '8 => Some(EventKind::Forgotten),' '8 => Some(EventKind::Forgotten),
            9 => Some(EventKind::Linked),'
run "a ninth kind byte, no law" 1

copy
printf '\npub static Y: fn() -> Result<Option<(EventKind, i32)>, ()> = || Ok(Some((EventKind::Paid, 1)));\n' >> "$SCRATCH/r/crates/dowiz-hub/src/rotate.rs"
run "a Paid writer the gate cannot place in a fn" 1

# Done right: the log field, its law test, and the v2 line.
copy
edit "$SCRATCH/r/crates/dowiz-hub/src/shred.rs" '"at":{now_ms},"reason":"shred"}}' '"at":{now_ms},"reason":"shred","lot":0}}'
printf '\n#[test]\nfn forgotten_gained_lot_and_the_laws_hold() {}\n' >> "$SCRATCH/r/crates/dowiz-hub/src/event/migration_tests.rs"
fp=$(sed -n 's/^log\.forgotten *1 \([^ ]*\) .*/\1/p' "$SCRATCH/r/tools/gates/migration-law.registry" | tr ',' '\n' | { cat; echo lot; } | sort | paste -sd, -)
printf 'log.forgotten 2 %s event::migration_tests::forgotten_gained_lot_and_the_laws_hold\n' "$fp" >> "$SCRATCH/r/tools/gates/migration-law.registry"
run "the log field shipped with its law and a v2 line" 0

[ $fail -eq 0 ] && echo "migration-law.prove: the gate fires in both directions" || echo "migration-law.prove: FAILED"
exit $fail
