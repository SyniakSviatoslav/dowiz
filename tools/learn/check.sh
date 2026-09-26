#!/bin/sh
# Validate assembled lesson cuts with ffprobe; a cut that fails is not published.
#   sh tools/learn/check.sh DIR [LANG...]     (default: every language DIR has a cut for)
# Exit code = the number of failed checks. The checks live in check.mjs (tested by check.test.mjs).
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
DIR=${1:?usage: check.sh DIR [LANG...]}
shift
LANGS="$*"
if [ -z "$LANGS" ]; then for l in sq en uk; do [ -d "$DIR/$l" ] && LANGS="$LANGS $l"; done; fi
[ -n "$LANGS" ] || { echo "check: no cut under $DIR" >&2; exit 2; }
node "$HERE/check.mjs" "$DIR" $LANGS
