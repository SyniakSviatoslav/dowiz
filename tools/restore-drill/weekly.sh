#!/bin/sh
# THE WEEKLY RESTORE DRILL, FROM THE BOX (W-PITR2; operator 2026-10-07: "weekly drill from the box").
#
# Pulls the NEWEST nightly copy of one venue from the off-site bucket, opens it OFF the Worker
# (seal-open with the platform's secret key, then gunzip), and runs `restore_drill` -- the same
# checked loaders the venue's object uses -- against it and the witness census uploaded beside it.
# Prints `restore-drill: PASS` or `restore-drill: REFUSED (...)` and exits 0 / 1 (2 = could not run).
# READS ONLY: the bucket is listed and read, never written. NOT SCHEDULED by this lane.
#
#   sh tools/restore-drill/weekly.sh <venue> [prefix]        # the bucket (dowiz-offsite by default)
#   sh tools/restore-drill/weekly.sh --local <dir> <venue>     # the same pipeline over files on disk
#
# Inputs: /root/.dowiz_offsite_s3 (S3_KEY, S3_SECRET, CLOUDFLARE_ACCOUNT_ID; never printed),
# BUCKET (default dowiz-offsite), SEAL_SK (default /root/.dowiz_backup_seal.sk), and the two
# binaries, built once:  cd crates/dowiz-hub && cargo build --release --bin restore_drill
#                        cd tools/seal-open && cargo build --release
set -eu
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
DRILL=${RESTORE_DRILL:-$ROOT/crates/dowiz-hub/target/release/restore_drill}
OPEN=${SEAL_OPEN:-$ROOT/tools/seal-open/target/release/seal-open}
SEAL_SK=${SEAL_SK:-/root/.dowiz_backup_seal.sk}
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
fail() { echo "restore-drill: CANNOT RUN: $*" >&2; exit 2; }
[ -x "$DRILL" ] || fail "no $DRILL (cd crates/dowiz-hub && cargo build --release --bin restore_drill)"

# The newest bundle of a listing (stamps sort as text) and the witness with the same stamp.
newest() { grep -E '/[0-9]{8}T[0-9]{6}Z\.json(\.gz)?(\.sealed)?$' | sort | tail -1; }

if [ "${1:-}" = "--local" ]; then
  [ $# -eq 3 ] || fail "usage: weekly.sh --local <dir> <venue>"
  DIR=$2; VENUE=$3
  key=$(cd "$DIR" && ls | sed "s#^#$VENUE/#" | newest) || true
  [ -n "$key" ] || fail "no nightly copy of $VENUE in $DIR"
  cp "$DIR/${key#"$VENUE/"}" "$WORK/"
  wit="${key%%.json*}.witness.json"
  [ -f "$DIR/${wit#"$VENUE/"}" ] && cp "$DIR/${wit#"$VENUE/"}" "$WORK/witness.json"
else
  [ $# -ge 1 ] || fail "usage: weekly.sh <venue> [prefix]"
  VENUE=$1; PREFIX=${2:-}
  [ -r /root/.dowiz_offsite_s3 ] || fail "no /root/.dowiz_offsite_s3"
  set -a; . /root/.dowiz_offsite_s3; set +a
  S3_ENDPOINT=${S3_ENDPOINT:-https://$CLOUDFLARE_ACCOUNT_ID.r2.cloudflarestorage.com}; export S3_ENDPOINT
  BUCKET=${BUCKET:-dowiz-offsite}
  base=${PREFIX:+$PREFIX/}$VENUE/
  key=$(python3 -I "$ROOT/tools/restore-drill/s3.py" list "$BUCKET" "$base" | newest) || true
  [ -n "$key" ] || fail "no nightly copy under s3://$BUCKET/$base"
  python3 -I "$ROOT/tools/restore-drill/s3.py" get "$BUCKET" "$key" "$WORK/$(basename "$key")"
  wit="${key%%.json*}.witness.json"
  python3 -I "$ROOT/tools/restore-drill/s3.py" get "$BUCKET" "$wit" "$WORK/witness.json" 2>/dev/null || rm -f "$WORK/witness.json"
fi

f="$WORK/$(basename "$key")"
echo "restore-drill: $key"
case "$f" in
  *.sealed)
    [ -x "$OPEN" ] || fail "no $OPEN (cd tools/seal-open && cargo build --release)"
    [ -r "$SEAL_SK" ] || fail "no secret key $SEAL_SK"
    "$OPEN" open "$SEAL_SK" "$f" "$WORK/opened" >/dev/null 2>&1 || fail "seal-open refused $key"
    if [ "$(head -c 2 "$WORK/opened" | od -An -tx1 | tr -d ' ')" = "1f8b" ]; then mv "$WORK/opened" "$WORK/b.json.gz"; f=$WORK/b.json.gz; else f=$WORK/opened; fi ;;
esac
case "$f" in *.gz) gunzip -f "$f"; f=${f%.gz} ;; esac
[ -f "$WORK/witness.json" ] || echo "restore-drill: no witness beside $key (the tip is not compared)"
set +e
if [ -f "$WORK/witness.json" ]; then "$DRILL" "$f" "$WORK/witness.json"; else "$DRILL" "$f"; fi
rc=$?
exit $rc
