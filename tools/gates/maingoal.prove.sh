#!/bin/sh
# SPDX-License-Identifier: MIT OR Apache-2.0
# maingoal's proof: the REAL tools/gates/maingoal.sh runs from a SCRATCH ROOT (the gate, the two
# verifiers, read.mjs, the format text, a link to crates/ -- it resolves its root from its own path),
# with a scratch copy of the prebuilt binaries:
#   1. clean                                                      -> rc 0, GREEN, and the SKIP is named
#   2. the stamp names other sources                              -> rc 2, STALE
#   3. bebop-verify.py forgets the hub's kind byte                -> rc 1, python's counters DISAGREE
#   4. doc_verify is a liar that prints the wanted answer         -> rc 1, the kills fail
#   5. the forger changes nothing                                 -> rc 1, the kills fail
#   6. read.mjs runs a reader that fails its digest               -> rc 1, kill wasm-byte fails
#   7. one reader's status words drift                            -> rc 1, kill rec-bit fails
#   8. python's `cut` is off by one                               -> rc 1, "cut: .*DISAGREE"
#   9. python believes the table (`keys_disagree` stuck at 0)      -> rc 1, kill zero-revoked fails
#  10. python reads a tombstone by its erased nonce again          -> rc 1, the legacy tombstone DISAGREES
#  11. python does not walk the seals' tips                        -> rc 1, kill unsealed-tail fails
# With no binaries the gate refuses (rc 2), and so does this proof, naming the same command.
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
SRC="${MAINGOAL_BIN:-$REPO/crates/dowiz-hub/target/maingoal}"
# run-all runs a proof BEFORE its gate: on GitHub nothing is prebuilt, so the build happens here
if [ -n "${GITHUB_ACTIONS:-}" ]; then sh "$HERE/maingoal.sh" --build > /dev/null 2>&1 || { echo "maingoal.prove: RED maingoal.sh --build failed"; exit 1; }; fi
if [ ! -f "$SRC/stamp" ]; then echo "maingoal.prove: REFUSED no prebuilt binaries in $SRC: bash bebop-lang/tools/slot.sh maingoal sh tools/gates/maingoal.sh --build"; exit 2; fi
S=$(mktemp -d "${TMPDIR:-/tmp}/maingoal-prove.XXXXXX")
trap 'rm -rf "${S:?}"' EXIT
fail=0
fresh() {
  rm -rf "${S:?}/r" "${S:?}/bin"
  mkdir -p "$S/r/tools/gates" "$S/r/tools/verify" "$S/r/examples" "$S/r/docs" "$S/bin"
  cp "$HERE/maingoal.sh" "$HERE/maingoal.forge.py" "$S/r/tools/gates/"
  cp "$REPO/tools/verify/bebop-verify.py" "$REPO/tools/verify/bebop-verify.mjs" "$S/r/tools/verify/"
  cp "$REPO/examples/read.mjs" "$S/r/examples/"; cp "$REPO/docs/FORMAT-BEBOP-IMAGE-v2.md" "$S/r/docs/"
  ln -s "$REPO/crates" "$S/r/crates"
  cp "$SRC/doc_verify" "$SRC/anchor" "$SRC/maingoal_device" "$SRC/stamp" "$S/bin/"
}
gate() { MAINGOAL_BIN="$S/bin" sh "$S/r/tools/gates/maingoal.sh" "$@"; }
want() { # want <rc> <label> <grep>
  gate > "$S/out" 2>&1; rc=$?
  hit=$(grep -m1 -- "$3" "$S/out" | cut -c1-150)
  echo "maingoal.prove: $2 -> rc=$rc (want $1): ${hit:-$(tail -1 "$S/out" | cut -c1-150)}"
  [ "$rc" -eq "$1" ] && [ -n "$hit" ] || { tail -5 "$S/out" | cut -c1-200; fail=1; }
}
PYV="$S/r/tools/verify/bebop-verify.py"
fresh; want 0 "clean scratch" "GREEN (3 of 3 scenarios held.*1 SKIPPED not passed: two offline devices merge"
fresh; sed -i 's/^src=[0-9a-f]\{8\}/src=00000000/' "$S/bin/stamp"; want 2 "stamp names other sources" "STALE"
fresh; sed -i 's/^    return b\[1:\] if b\[:1\] in (b"D", b"K") else b$/    return b/' "$PYV"; want 1 "python forgets the kind byte" "hub.sealed: records: doc_verify .* python .* DISAGREE"
fresh; cat > "$S/bin/doc_verify" <<'L'
#!/bin/sh
echo "doc records=4 chained=4 legacy=0 redacted=0 broken=0 seals_ok=4 seals_bad=0 forks=0 keys=1 admitted=1 anchors=2 anchors_ok=1 anchors_bad=0 anchors_unchecked=1 anchors_cut=1 segments=1 cut=9 cut_redacted=2 declared=2 keys_disagree=0 revokes_unsettled=0 tombs_foreign=0 unsealed=0"
echo HOLDS
L
chmod +x "$S/bin/doc_verify"; gate --stamp > /dev/null; want 1 "doc_verify is a liar" "KILL rec-bit FAILED"
fresh; printf 'import sys, shutil\nshutil.copy(sys.argv[2], sys.argv[3])\n' > "$S/r/tools/gates/maingoal.forge.py"; want 1 "the forger changes nothing" "KILL rec-bit FAILED"
fresh; sed -i 's/if (got !== claimed)/if (false)/' "$S/r/examples/read.mjs"; want 1 "read.mjs ignores the digest" "KILL wasm-byte FAILED"
fresh; sed -i 's/10: "BAD_CRC: an object the read reached fails its crc (root/10: "BAD_CRC: an object fails its crc (root/' "$PYV"; want 1 "python's words drift" "KILL rec-bit FAILED"
fresh; sed -i 's/cut=sum(s\["last"\] - s\["first"\] + 1 for s in segs),/cut=sum(s["last"] - s["first"] + 2 for s in segs),/' "$PYV"; want 1 "python's cut is off by one" "cut: doc_verify .* python .* DISAGREE"
fresh; sed -i 's/keys_disagree=keys_disagree(keys, revokes)/keys_disagree=0/' "$PYV"; want 1 "python believes the table" "KILL zero-revoked FAILED"
fresh; sed -i 's/^    return bool(b) and b\[0\] \& REDACTED_BIT != 0$/    return False/' "$PYV"; want 1 "python misses the legacy tombstone" "legacy-forget.sealed: redacted: doc_verify .* python .* DISAGREE"
fresh; sed -i 's/unsealed=unsealed(recs, tips, segs)/unsealed=0/' "$PYV"; want 1 "python walks no tip" "KILL unsealed-tail FAILED"
[ "$fail" = 0 ] && echo "maingoal.prove: GREEN (11 cases: one pass, one refusal, nine named reds)" || echo "maingoal.prove: RED"
exit $fail
