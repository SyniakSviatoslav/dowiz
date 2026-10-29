#!/bin/sh
# es-bench's proof: the gate is GREEN on the tree and RED on a deliberately slowed bebop adapter.
# Each case copies crates/es-bench to a scratch dir, rewrites ONE line of src/bebop.rs (the
# adapter -- the file P3 swaps), builds it, and runs the real gate on that binary. A mutation
# that does not apply is a FAILURE of this proof, never a silent pass.
#   A  every point get done twice           -> the `get` row ~2x
#   B  the image serialised twice per build -> the `build` row ~2x
#   C  the verify-on-open crc run twice     -> the `verify` row ~2x
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../.." && pwd)
G="$HERE/es-bench.sh"
SCR=$(mktemp -d "${TMPDIR:-/tmp}/es-bench-prove.XXXXXX")
T="${CARGO_TARGET_DIR:-$ROOT/crates/es-bench/target}-prove"
fail=0

want() { # want <rc> <label> <cmd...>
  w=$1; l=$2; shift 2
  "$@" > "$SCR/out" 2>&1; rc=$?
  grep -E '^GATE|es-bench:' "$SCR/out" | sed 's/^/    /'
  if [ "$rc" -eq "$w" ]; then echo "prove: ok   ($l) rc=$rc"; else echo "prove: FAIL ($l) rc=$rc, wanted $w"; fail=1; fi
}

mutant() { # mutant <old> <new> -> builds $SCR/m and prints the binary path, or fails loudly
  rm -rf "$SCR/m"; mkdir -p "$SCR/m"
  cp -r "$ROOT/crates/es-bench/Cargo.toml" "$ROOT/crates/es-bench/src" "$SCR/m/"
  [ -f "$ROOT/crates/es-bench/Cargo.lock" ] && cp "$ROOT/crates/es-bench/Cargo.lock" "$SCR/m/"
  python3 - "$SCR/m" "$ROOT/crates/bebop-store" "$1" "$2" <<'PY' || return 1
import sys, pathlib
m, store, old, new = sys.argv[1:]
c = pathlib.Path(m, "Cargo.toml"); c.write_text(c.read_text().replace('path = "../bebop-store"', f'path = "{store}"'))
b = pathlib.Path(m, "src", "bebop.rs"); s = b.read_text()
if s.count(old) != 1:
    sys.exit(f"prove: mutation target found {s.count(old)} times in bebop.rs: {old!r}")
b.write_text(s.replace(old, new))
PY
  ( cd "$SCR/m" && CARGO_TARGET_DIR="$T" cargo build --release --quiet ) || { echo "prove: mutant build FAILED"; return 1; }
}

want 0 "the tree" sh "$G"

if mutant 'keys.iter().map(|k| acc_of(&kv.get(k).unwrap().expect("present")))' \
          'keys.iter().map(|k| { std::hint::black_box(kv.get(k).unwrap()); acc_of(&kv.get(k).unwrap().expect("present")) })'; then
  want 1 "A: every get done twice" sh "$G" --bin "$T/release/es-bench"
else fail=1; fi

if mutant 'kv.compacted_bytes_fit(MAX_IMAGE).expect("image fits")' \
          '{ std::hint::black_box(kv.compacted_bytes_fit(MAX_IMAGE)); kv.compacted_bytes_fit(MAX_IMAGE).expect("image fits") }'; then
  want 1 "B: the image built twice" sh "$G" --bin "$T/release/es-bench"
else fail=1; fi

if mutant 'KvIn::open(View::new(bytes)).is_ok()' \
          'std::hint::black_box(KvIn::open(View::new(bytes)).is_ok()) && KvIn::open(View::new(bytes)).is_ok()'; then
  want 1 "C: verify run twice" sh "$G" --bin "$T/release/es-bench"
else fail=1; fi

rm -rf "$SCR"
[ $fail -eq 0 ] && echo "es-bench.prove: the gate fires in both directions" || echo "es-bench.prove: FAILED"
exit $fail
