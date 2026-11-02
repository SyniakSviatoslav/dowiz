#!/bin/sh
# SPDX-License-Identifier: MIT OR Apache-2.0
# reader-repro's proof: each way the pin can stop being its source, broken on a COPY; the gate must
# fire and name it. The real tree is never touched. Behind --cargo; on the box under slot.sh, -j1.
#
# FIVE COMPILES, NOT TWELVE (the box SIGKILLed the twelve twice). The gate runs real and whole in every
# case: only CARGO is a shim, the seam build.sh already has. The shim keys a module on the sha of all
# the compiler reads (crates, .cargo, the toolchain file, build.sh, the wrapper; the paths relative, so
# the directory is not in the key). `store` always compiles, `use` replays a module of the SAME key.
#   unmutated copy, build.sh without its wrapper: `store`, two compiles each. These two ARE the claim
#     that a compile depends, or does not, on the directory; a replay could not show either.
#   the three pin mutants: `use`, no compile. They touch only files the compiler never reads, and the
#     key proves it: a mutant that reached a compile input would miss and compile.
#   the moved source: `use`, one compile, because its point is that the pin no longer matches.
# The total is checked at the end, so a key that stopped working is a FAILED and not a quiet slowdown.
#
# Exit: 0 every kill fired, 1 FAILED, 3 NO VERDICT (the box killed a case; never a pass).
set -u
REPO="$(cd "$(dirname "$0")/../.." && pwd)"
S="$(mktemp -d "${TMPDIR:-/tmp}/rrprove.XXXXXX")" || exit 2
trap 'rm -rf "$S"' EXIT
fail=0; noverdict=0
RR_REAL="${CARGO:-$HOME/.cargo/bin/cargo}"; [ -x "$RR_REAL" ] || RR_REAL=cargo
RR_M="$S/memo"; mkdir -p "$RR_M"; : > "$RR_M/compiles"; export RR_REAL RR_M
cat > "$S/cargo-memo" <<'SHIM'
#!/bin/sh
# cwd is the checkout's crates/bebop-wasm-reader; build.sh always passes --target-dir
t=; p=; for a in "$@"; do [ "$p" = --target-dir ] && t=$a; p=$a; done
[ -n "$t" ] || { echo "error: cargo-memo: no --target-dir" >&2; exit 2; }
w="$t/wasm32-unknown-unknown/release/bebop_wasm_reader.wasm"
key=$(cd ../.. && find crates .cargo rust-toolchain.toml tools/verify/build.sh tools/verify/rustc-stable-id.sh -type f -not -path '*/target/*' \
  | LC_ALL=C sort | xargs sha256sum | sha256sum | cut -c1-64)
if [ "$RR_MEMO" = use ] && [ -s "$RR_M/$key" ]; then mkdir -p "$(dirname "$w")" && cp "$RR_M/$key" "$w"; exit $?; fi
echo "$key" >> "$RR_M/compiles"
"$RR_REAL" "$@" || exit $?
cp "$w" "$RR_M/$key"
SHIM
chmod +x "$S/cargo-memo"
fresh() {
  rm -rf "$S/r"; mkdir -p "$S/r/crates" "$S/r/tools/gates" "$S/r/examples"
  for c in $(sed -n 's/^name = "\(.*\)"$/\1/p' "$REPO/crates/bebop-wasm-reader/Cargo.lock"); do
    tar -C "$REPO/crates" --exclude=target -cf - "$c" | tar -C "$S/r/crates" -xf -
  done
  cp -a "$REPO/tools/verify" "$S/r/tools/"; cp "$REPO"/tools/gates/reader-repro.sh "$REPO"/tools/gates/atk3_pin.py "$S/r/tools/gates/"
  cp "$REPO/examples/read.mjs" "$S/r/examples/"; cp "$REPO/rust-toolchain.toml" "$S/r/"; cp -a "$REPO/.cargo" "$S/r/"
}
gate() { # gate <what> <rc wanted> <text the last line must name> [env...]
  w=$1; r=$2; t=$3; shift 3
  env "$@" sh "$S/r/tools/gates/reader-repro.sh" > "$S/out" 2>&1; rc=$?
  if [ "$rc" = "$r" ] && tail -1 "$S/out" | grep -q "$t"; then echo "prove: $w -> rc=$rc: $(tail -1 "$S/out" | cut -c1-120)"
  elif [ "$rc" = 3 ] || [ "$rc" -ge 128 ]; then echo "prove: NO VERDICT $w (rc=$rc), the box and not the gate"; tail -3 "$S/out"; noverdict=1
  else echo "prove: FAILED $w (rc=$rc, want $r naming '$t')"; tail -3 "$S/out"; fail=1; fi
}
mut() { # mut <what> <file> <sed expr> <text the gate must name> <store|use>
  fresh; cp "$S/r/$2" "$S/before"; sed -i "$3" "$S/r/$2"
  if cmp -s "$S/before" "$S/r/$2"; then echo "prove: FAILED $1: the mutation changed nothing"; fail=1; return; fi
  gate "$1" 1 "$4" CARGO="$S/cargo-memo" RR_MEMO="$5"
}
fresh; gate "unmutated copy" 0 'GREEN' CARGO="$S/cargo-memo" RR_MEMO=store
M=tools/verify/bebop-verify.mjs
mut "read.mjs pins another reader" examples/read.mjs 's/^const PINNED = "[0-9a-f]\{4\}/const PINNED = "dead/' 'RED read.mjs PINNED' use
mut "WASM_SHA256 is not the source" $M 's/^const WASM_SHA256 = "[0-9a-f]\{4\}/const WASM_SHA256 = "dead/' 'RED WASM_SHA256' use
mut "the base64 is another module" $M 's/^const WASM_B64 = "AGFzbQ/const WASM_B64 = "AGFzbA/' 'RED the embedded base64' use
mut "the source moved under the pin" crates/bebop-wasm-reader/src/status.rs 's/BAD_CRC: an object/BAD_CRC: one object/' 'RED WASM_SHA256' use
mut "build.sh forgets the wrapper" tools/verify/build.sh 's/ RUSTC_WRAPPER="$V\/rustc-stable-id.sh"//' 'depend on the directory' store
# No compiler from here on. These fakes answer the gate's two questions; PATH stays whole, because
# `env PATH=/nonexistent sh` is rc=127 from env itself and would prove nothing about the gate.
fresh; printf '#!/bin/sh\necho "error: could not compile x"\nexit 101\n' > "$S/nocargo"; chmod +x "$S/nocargo"
gate "a build that dies is no verdict" 3 'NO VERDICT' CARGO="$S/nocargo"
printf '#!/bin/sh\necho "rustc 1.75.0 (distro)"\n' > "$S/oldrustc"; chmod +x "$S/oldrustc"
gate "the distro rustc is refused" 2 'REFUSED rustc here is' RUSTC="$S/oldrustc"
v=$(sed -n 's/^channel = "\(.*\)"$/\1/p' "$REPO/rust-toolchain.toml")
printf '#!/bin/sh\n[ "$1" = --version ] && echo "rustc %s (fake)" || echo /nonexistent\n' "$v" > "$S/nostd"; chmod +x "$S/nostd"
gate "a toolchain with no wasm32 std is refused" 2 'no wasm32-unknown-unknown std' RUSTC="$S/nostd"
n=$(wc -l < "$RR_M/compiles" | tr -d ' ')
if [ $noverdict = 1 ]; then echo "prove: NO VERDICT, not a pass ($n compiles before the kill)"; [ $fail = 0 ] && exit 3; fi
if [ $fail = 0 ] && [ "$n" != 5 ]; then echo "prove: FAILED $n compiles, want 5: the shim's key no longer holds"; fail=1; fi
[ $fail = 0 ] && echo "prove: every kill fired ($n compiles)" || echo "prove: FAILED"
exit $fail
