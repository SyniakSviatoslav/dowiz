#!/bin/sh
# SPDX-License-Identifier: MIT OR Apache-2.0
# cell-usize's proof: the real gate over a scratch copy of the three crates' sources.
#   1. clean                                     -> rc 0
#   2. a `get(..) as usize` in einfile-kv          -> rc 1, names the file
#   3. a clamp `.max(0) as usize` in einfile-log   -> rc 1
#   4. a `cell_at(..) as usize` in einfile-image   -> rc 1
#   5. the same line in a test file              -> rc 0: tests name offsets
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
S=$(mktemp -d "${TMPDIR:-/tmp}/cell-usize.XXXXXX")
trap 'rm -rf "$S"' EXIT
fresh() {
  rm -rf "$S/crates"; mkdir -p "$S/crates"
  for c in einfile-image einfile-log einfile-kv; do mkdir -p "$S/crates/$c"; cp -R "$REPO/crates/$c/src" "$S/crates/$c/src"; done
}
put() { { printf '%s\n' "$2"; cat "$S/crates/$1"; } > "$S/x" && mv "$S/x" "$S/crates/$1"; }
gate() { CELL_USIZE_ROOT="$S" sh "$HERE/cell-usize.sh" > "$S/out" 2>&1; }
fail() { echo "cell-usize.prove: FAILED $1"; cat "$S/out"; exit 1; }
fresh; gate || fail "clean is not rc 0"
fresh; put einfile-kv/src/limits.rs 'fn a(st: &S) -> usize { st.get(0, 1) as usize }'; gate; [ $? = 1 ] && grep -q limits.rs "$S/out" || fail "get as usize passed"
fresh; put einfile-log/src/verify.rs 'fn a(c: i64) -> usize { c.max(0) as usize }'; gate; [ $? = 1 ] || fail "a clamp passed"
fresh; put einfile-image/src/crc.rs 'fn a<C: Cells>(c: &C) -> usize { c.cell_at(3) as usize }'; gate; [ $? = 1 ] || fail "cell_at as usize passed"
fresh; put einfile-kv/src/limits/tests.rs 'fn a(st: &S) -> usize { st.get(0, 1) as usize }'; gate || fail "a test file was judged"
echo "cell-usize.prove: GREEN 5 of 5"
