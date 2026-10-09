#!/bin/sh
# x86-check's proof: the exact defect of 2026-10-06 put back, in a scratch copy of dowiz-core.
#   1. clean copy                                                  -> rc 0
#   2. an undefined const inside a #[cfg(target_arch = "x86_64")] fn -> rc 1, naming simd_i8.rs
#      (the box's own `cargo check` stays green over it: that is the blind spot being closed)
#   3. a toolchain with no x86_64 std                              -> rc 2 REFUSED, never a pass
# Compiles Rust: run-all runs it only with --cargo. X86_CHECK_FORCE=1, so on the x86_64 CI runner
# the alarm is still heard (the gate's native shortcut would otherwise answer 0 to everything).
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
G="$HERE/x86-check.sh"
SCRATCH=$(mktemp -d)
trap 'rm -rf "$SCRATCH"' EXIT
fail=0
C="$SCRATCH/r/crates/dowiz-core"
copy() { # dowiz-core plus the one path dependency its tests name (../../tools/eqc-rs)
  rm -rf "$SCRATCH/r"; mkdir -p "$SCRATCH/r/crates" "$SCRATCH/r/tools"
  mkdir "$C"; cp -r "$REPO/crates/dowiz-core/Cargo.toml" "$REPO/crates/dowiz-core/Cargo.lock" \
    "$REPO/crates/dowiz-core/src" "$REPO/crates/dowiz-core/tests" "$C/"
  ln -s "$REPO/tools/eqc-rs" "$SCRATCH/r/tools/eqc-rs"
}
mutate() { # mutate <file> <old> <new>: replace one literal; refuse when it is not there
  python3 - "$C/$1" "$2" "$3" <<'PY' || { echo "prove: mutation not applied to $1"; fail=1; }
import sys
p, old, new = sys.argv[1:]
s = open(p).read()
if old not in s: sys.exit(1)
open(p, 'w').write(s.replace(old, new, 1))
PY
}
want() { # want <rc> <label> <must-mention> [env...]
  exp=$1; label=$2; needle=$3; shift 3
  env X86_CHECK_FORCE=1 X86_CHECK_TARGET="${X86_CHECK_TARGET:-$REPO/target/x86-check}" "$@" \
    sh "$G" "$C" > "$SCRATCH/out" 2>&1; rc=$?
  echo "prove: $label -> rc=$rc (want $exp): $(tail -1 "$SCRATCH/out")"
  [ "$rc" -eq "$exp" ] || { tail -8 "$SCRATCH/out"; fail=1; }
  grep -q -- "$needle" "$SCRATCH/out" || { echo "prove: $label: output does not mention '$needle'"; fail=1; }
}
copy; want 0 "clean" "GREEN"
copy; mutate src/inference/simd_i8.rs 'unsafe fn dot_i8_avx2(a: *const i8, w: *const i8, k: usize) -> i32 {
    use core::arch::x86_64::*;' 'unsafe fn dot_i8_avx2(a: *const i8, w: *const i8, k: usize) -> i32 {
    use core::arch::x86_64::*;
    let _ = XCHECK_NEVER_DEFINED;'
want 1 "undefined const in the x86_64-only AVX2 fn" "simd_i8.rs"
mkdir -p "$SCRATCH/fake/bin" "$SCRATCH/fake/lib/rustlib"
printf '#!/bin/sh\nexit 0\n' > "$SCRATCH/fake/bin/cargo"; chmod +x "$SCRATCH/fake/bin/cargo"
copy; want 2 "a toolchain without the x86_64 std" "REFUSED" X86_CHECK_TOOLCHAIN="$SCRATCH/fake"
[ $fail -eq 0 ] && echo "x86-check.prove: GREEN -- 3 of 3 cases (clean, x86-only defect, no x86 std)" \
                || echo "x86-check.prove: RED"
exit $fail
