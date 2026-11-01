#!/bin/sh
# SPDX-License-Identifier: MIT OR Apache-2.0
# atk3's proof (W-MG-HOST): one rule of the host broken per case, on a COPY; the gate must go red and
# name the row. The real tree is never touched. No compiler. Exit 0 = every kill fired.
set -u
REPO="$(cd "$(dirname "$0")/../.." && pwd)"
S="$(mktemp -d "${TMPDIR:-/tmp}/atk3prove.XXXXXX")" || exit 2
trap 'rm -rf "$S"' EXIT
fresh() {
  rm -rf "$S/r"; mkdir -p "$S/r/tools/gates" "$S/r/tools/verify" "$S/r/examples" "$S/r/crates/bebop-wasm-reader/src"
  cp "$REPO"/tools/gates/atk3* "$S/r/tools/gates/"; cp "$REPO/tools/verify/bebop-verify.mjs" "$REPO/tools/verify/bebop-verify.py" "$S/r/tools/verify/"
  cp "$REPO/examples/read.mjs" "$S/r/examples/"; cp "$REPO/crates/bebop-wasm-reader/src/status.rs" "$S/r/crates/bebop-wasm-reader/src/"
}
fail=0
kill1() { # kill1 <what> <file> <sed expr> <the row the gate must name> [the rc, default 1]
  want=${5:-1}
  fresh
  cp "$S/r/$2" "$S/before"; sed -i "$3" "$S/r/$2"
  if cmp -s "$S/before" "$S/r/$2"; then echo "prove: FAILED $1: the mutation changed nothing"; fail=1; return; fi
  sh "$S/r/tools/gates/atk3.sh" > "$S/out" 2>&1; rc=$?
  if [ "$rc" = "$want" ] && grep -q "$4" "$S/out"; then echo "prove: $1 -> rc=$rc: $(grep -m1 "$4" "$S/out" | cut -c1-110)"
  else echo "prove: FAILED $1 (rc=$rc, want $want naming '$4')"; tail -3 "$S/out"; fail=1; fi
}
fresh; sh "$S/r/tools/gates/atk3.sh" > "$S/out" 2>&1; rc=$?; echo "prove: unmutated copy -> rc=$rc: $(tail -1 "$S/out")"; [ "$rc" = 0 ] || fail=1
R=examples/read.mjs; M=tools/verify/bebop-verify.mjs
kill1 "the pin is not asked" $R 's/if (got !== PINNED \&\& !own)/if (got === "" \&\& !own)/' '^HOLE  read_forged_green'
kill1 "the pin drifts from the reader" $R 's/^const PINNED = "[0-9a-f]\{4\}/const PINNED = "dead/' 'RED read.mjs pins'
kill1 "imports are not looked at" $R 's/if (id === 2 \&\&/if (id === 99 \&\&/' '^HOLE  read_import_nostart'
kill1 "a start section is not looked at" $R 's/if (id === 8) return/if (id === 99) return/' '^HOLE  read_start_loop'
kill1 "the page bound is past the limit" $R 's/MAX_PAGES = 16384/MAX_PAGES = 65536/' '^HOLE  read_mem_over'
kill1 "a memory may have no maximum" $R 's/n !== 1 || w\[at.i++\] !== 1/n !== 1 || w[at.i++] > 1/' '^HOLE  read_mem_no_max'
kill1 "DOC.SPEC is not held to 1024" $R 's/cell(doc + 8) !== 1024n/cell(doc + 8) === -1n/' '^HOLE  read_spec_moved'
kill1 "a printed line is not filtered" $R 's/const clean = (s) => String(s).replace(/const clean = (s) => String(s) || String(s).replace(/' '^HOLE  path_read_mjs'
kill1 "the clock is gone" $R 's/}, limit);/}, 1e7);/' '^HOLE  read_abi_loop'
kill1 "the catch rethrows" $R 's/} catch (e) { stop(HOST_FAILED/} catch (e) { throw e; stop(HOST_FAILED/' '^HOLE  path_read_mjs'
kill1 "num does not bound" $R 's/if (v < 0n || v > BigInt(max)) stop(/if (v < 0n \&\& v > BigInt(max)) stop(/' '^HOLE  read_ref_huge'
kill1 "the flag marks stdout no more" $R 's/const out = (s) => console.log((own ? MARK : "") + clean(s))/const out = (s) => console.log(clean(s))/' '^HOLE  read_forged_flag'
kill1 "the exit code under the flag is 0" $R 's/process.exit(own ? UNPINNED : 0)/process.exit(0)/' '^HOLE  read_abi_status\|^HOLE  read_forged_flag'
kill1 "a word drifts from status.rs" crates/bebop-wasm-reader/src/status.rs 's/BAD_CRC: an object/BAD_CRC: one object/' 'RED read.mjs WORDS'
kill1 "the host passes null for an empty file" $M 's/const room = Math.max(image.length, 1);/const room = image.length;/' '^HOLE  mjs_file_empty'
kill1 "the verifier has no catch" $M 's/^process.on("uncaughtException", failed);$//;s/^process.on("unhandledRejection", failed);$//' '^HOLE  path_wasm_mjs'
kill1 "the harness loses rows" tools/gates/atk3_bootstrap.py '/row("agree" if good else "HOLE", "read_abi_status"/d' 'REFUSED 28 rows' 2
if [ "$fail" = 0 ]; then echo "prove: GREEN"; else echo "prove: FAILED"; fi
exit "$fail"
