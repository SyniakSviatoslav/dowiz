#!/bin/sh
# LANGS — one language set, and every dictionary speaks all of it (lane W-RU).
# The rules live in langs.mjs (read its header); this wrapper is what
# run-all.sh discovers. `sh tools/gates/langs.sh [ROOT]`; langs.prove.sh
# triggers every rule on a scratch copy before this gate is trusted.
exec node "$(dirname "$0")/langs.mjs" ${1:+"$1"}
