#!/bin/sh
# x86-check -- COMPILE THE x86_64-ONLY CODE ON A BOX THAT IS NOT x86_64.
#
# 2026-10-06..08: CI on main was red for two days because the AVX2 path in
# crates/dowiz-core/src/inference/simd_i8.rs (behind #[cfg(target_arch = "x86_64")]) named a
# const MAX_K that never existed. The dev box is aarch64: cfg removes that code before name
# resolution, so every local `cargo test` and run-all was green over it, and only the x86_64
# runner ever compiled it. Fixed in f773ad35. This gate makes the box compile that code too:
# `cargo check --tests --target x86_64-unknown-linux-gnu` with the pinned toolchain, for every
# crate whose sources mention x86_64 (found, never listed by hand -- a new crate is covered the
# day it gains an x86 path).
#
#   sh tools/gates/x86-check.sh                  # every crate found under the repo
#   sh tools/gates/x86-check.sh <crate-dir>...   # just these (the proof uses this)
#
# Exit: 0 GREEN, 1 RED (a crate does not compile for x86_64; its errors are printed), 2 REFUSED
# (no x86_64 std for the pinned toolchain, no cargo, or nothing found to check). A gate that
# cannot measure never passes (memory: gates-that-count-skips-as-passes).
#
# On an x86_64 host this is what `cargo test` already compiles, so the gate says so and exits 0;
# X86_CHECK_FORCE=1 checks anyway (the proof sets it, so the alarm is heard on the CI runner too).
# X86_CHECK_TOOLCHAIN=<dir> overrides the toolchain (the proof points it at a fake one).
# X86_CHECK_TARGET=<dir> overrides the shared target dir (default target/x86-check, gitignored).
#
# By default it does NOT run the x86 tests: a compile catches the class that broke CI (a name, a
# type, a missing import). A wrong answer needs the code to run: X86_CHECK_RUN=<test filter> also
# links each crate's lib tests and runs those matching the filter under `qemu-x86_64 -cpu max`
# (AVX2/SHA detected). Measured 2026-10-08: dowiz-core link ~150 s, simd_i8's 6 tests 3.5 s; the
# whole suite is ~15+ min, so pass a filter. A filter that runs nothing anywhere is REFUSED.
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
TRIPLE=x86_64-unknown-linux-gnu
PAT='target_arch *= *"x86_64"|is_x86_feature_detected|arch::x86_64'

host=$(uname -m)
if [ "$host" = x86_64 ] && [ -z "${X86_CHECK_FORCE:-}" ]; then
  echo "x86-check: native x86_64, covered by cargo test"
  exit 0
fi

# The toolchain: the one rust-toolchain.toml pins. The distro rustc on PATH is NOT it (memory:
# wasm32-needs-upstream-toolchain), so off x86 the rustup directory is named explicitly.
pin=$(sed -n 's/^channel *= *"\(.*\)"/\1/p' "$REPO/rust-toolchain.toml")
if [ -n "${X86_CHECK_TOOLCHAIN:-}" ]; then
  TC=$X86_CHECK_TOOLCHAIN
elif [ "$host" = x86_64 ]; then
  TC=$(rustc --print sysroot 2>/dev/null) || TC=/nonexistent
else
  TC="${RUSTUP_HOME:-$HOME/.rustup}/toolchains/$pin-$host-unknown-linux-gnu"
fi
if [ ! -x "$TC/bin/cargo" ]; then
  echo "x86-check: REFUSED -- no cargo at $TC/bin/cargo (pinned toolchain $pin not installed?)"
  echo "  install: rustup toolchain install $pin --profile minimal && rustup target add $TRIPLE --toolchain $pin"
  exit 2
fi
if ! ls "$TC/lib/rustlib/$TRIPLE/lib/"libstd-*.rlib >/dev/null 2>&1; then
  echo "x86-check: REFUSED -- no x86_64 std in $TC/lib/rustlib/$TRIPLE/lib"
  echo "  install: rustup target add $TRIPLE --toolchain $pin"
  exit 2
fi

# The crates: each file that mentions x86_64, mapped to the nearest Cargo.toml above it.
crate_of() { # crate_of <file> -> the directory holding its Cargo.toml
  d=$(dirname "$1")
  while [ "$d" != / ] && [ "$d" != . ] && [ ! -f "$d/Cargo.toml" ]; do d=$(dirname "$d"); done
  [ -f "$d/Cargo.toml" ] && echo "$d"
}
if [ $# -gt 0 ]; then
  crates=$*
else
  # Hidden directories are pruned: .claude/worktrees holds stale agent copies of the whole repo,
  # and a sweep of them reported a fixed bug twelve times over (2026-10-08).
  crates=$(find "$REPO" \( -name target -o -name node_modules -o -name '.?*' -o -name vendor \) -prune \
             -o -name '*.rs' -print | xargs grep -lE "$PAT" 2>/dev/null \
           | while read -r f; do crate_of "$f"; done | sort -u)
fi
[ -n "$crates" ] || { echo "x86-check: REFUSED -- no crate with x86_64-gated code found (pattern: $PAT)"; exit 2; }

TD=${X86_CHECK_TARGET:-$REPO/target/x86-check}
if [ -n "${X86_CHECK_RUN:-}" ] && [ "$host" != x86_64 ]; then
  for t in qemu-x86_64 x86_64-linux-gnu-gcc; do command -v $t >/dev/null || {
    echo "x86-check: REFUSED -- X86_CHECK_RUN needs $t (apt install qemu-user gcc-x86-64-linux-gnu)"; exit 2; }; done
  export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc QEMU_LD_PREFIX=/usr/x86_64-linux-gnu \
         CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER="qemu-x86_64 -cpu max"
fi
# --color never: CI exports CARGO_TERM_COLOR=always, and coloured lines never match the error grep
# below (the proof's defect case printed a RED with no error lines on the runner, 2026-10-09).
LOG=$(mktemp)
trap 'rm -f "$LOG"' EXIT
n=0; red=0; reds=""; ran=0
for c in $crates; do
  n=$((n + 1))
  name=${c#"$REPO"/}
  files=$(grep -rlE "$PAT" "$c/src" 2>/dev/null | wc -l)
  t0=$(date +%s)
  (cd "$c" && CARGO_TARGET_DIR="$TD" PATH="$TC/bin:$PATH" \
     cargo check --color never --tests --target "$TRIPLE" --message-format short) > "$LOG" 2>&1
  rc=$?
  dt=$(( $(date +%s) - t0 ))
  if [ "$rc" -eq 0 ]; then
    echo "  ok   $name ($files x86 file(s), ${dt}s)"
    [ -n "${X86_CHECK_RUN:-}" ] || continue
    (cd "$c" && CARGO_TARGET_DIR="$TD" PATH="$TC/bin:$PATH" \
       cargo test --color never --lib --target "$TRIPLE" -- "$X86_CHECK_RUN") > "$LOG" 2>&1
    rc=$?; res=$(grep 'test result:' "$LOG" | tail -1)
    k=$(printf '%s' "$res" | sed -n 's/.* \([0-9]*\) passed.*/\1/p'); ran=$((ran + ${k:-0}))
    echo "  run  $name '$X86_CHECK_RUN' rc=$rc: ${res:-no test result line}"
    [ "$rc" -eq 0 ] || { red=$((red + 1)); reds="$reds $name(tests)"; grep -E '^test .* FAILED|panicked' "$LOG" | head -10; }
  else
    red=$((red + 1)); reds="$reds $name"
    echo "  RED  $name rc=$rc (${dt}s):"
    grep -E ': error|^error' "$LOG" | head -20 | sed 's/^/       /'
  fi
done
[ "$red" -eq 0 ] && [ -n "${X86_CHECK_RUN:-}" ] && [ "$ran" -eq 0 ] && {
  echo "x86-check: REFUSED -- X86_CHECK_RUN='$X86_CHECK_RUN' ran 0 tests in $n crate(s)"; exit 2; }
if [ "$red" -eq 0 ]; then
  echo "x86-check: GREEN -- $n crate(s) compile for $TRIPLE with $pin:$(echo $crates | sed "s#$REPO/##g; s/^/ /")"
  exit 0
fi
echo "x86-check: RED -- $red of $n crate(s) do not compile for $TRIPLE:$reds"
exit 1
