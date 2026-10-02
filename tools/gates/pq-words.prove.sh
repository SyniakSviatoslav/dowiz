#!/bin/sh
# PQ-0's proof: a gate is triggered before it is trusted. Runs `pq-words.sh`
# against scratch copies of the surfaces it reads (platform/, the two other
# public files that mention PQ, README.md, SECURITY.md):
#   1. clean                                        -> must pass (exit 0): the
#      negated "makes no claim of post-quantum encryption" in README/SECURITY,
#      the `_headers` note about Cloudflare's own TLS and pass.js's "a
#      post-quantum signature is NOT used here" are all allowed — THE GREEN
#      CASE IS HALF THE PROOF
#   2..7. one planted live-PQ phrase per language and per clause -> must refuse
#   8. the bare label "post-quantum encryption" in index.html -> must refuse
#   9. the same label in README.md's negation                 -> must pass
#  10. the PQ section deleted from one language                -> must refuse
# Exit 0 only when every case answers as it should.
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
SCRATCH=$(mktemp -d)
trap 'rm -rf "$SCRATCH"' EXIT
fail=0
R="$SCRATCH/r"
WORDS="$R/workers/api/public/platform/landing-words.js"

copy() {
  rm -rf "$R"; mkdir -p "$R/workers/api/public/kit/screens"
  cp -r "$REPO/workers/api/public/platform" "$R/workers/api/public/platform"
  cp "$REPO/workers/api/public/_headers" "$R/workers/api/public/_headers"
  cp "$REPO/workers/api/public/kit/screens/pass.js" "$R/workers/api/public/kit/screens/pass.js"
  cp "$REPO/README.md" "$REPO/SECURITY.md" "$R/"
}
run() { # want label
  sh "$HERE/pq-words.sh" "$R" >"$SCRATCH/out" 2>&1; rc=$?
  [ $rc -eq "$1" ] || { echo "prove: $2 -> rc=$rc (want $1):"; cat "$SCRATCH/out"; fail=1; }
  echo "prove: $2 -> rc=$rc (want $1): $(grep -m1 'REFUSED\|0 hits' "$SCRATCH/out")"
}
plant() { # file line
  printf '%s\n' "$2" >> "$1"
}

copy
run 0 "clean tree"

copy; plant "$WORDS" "    x1: 'ML-DSA-65 on every packet.',"
run 1 "en: 'every packet'"

copy; plant "$WORDS" "    x1: 'ML-DSA-65 на кожному пакеті.',"
run 1 "uk: 'на кожному пакеті'"

copy; plant "$WORDS" "    x1: 'каждый пакет подписан ML-DSA-65.',"
run 1 "ru: 'каждый пакет'"

copy; plant "$WORDS" "    x1: 'çdo paketë nënshkruhet.',"
run 1 "sq: 'çdo paketë'"

copy; plant "$R/SECURITY.md" "- there is no classical-only fallback."
run 1 "SECURITY.md: 'no classical-only fallback'"

copy; plant "$R/README.md" "- X25519 + ML-KEM-768 on the channel, AES-256-GCM at rest."
run 1 "README.md: 'on the channel' + 'at rest'"

copy; plant "$R/workers/api/public/platform/index.html" "<p>Post-quantum encryption.</p>"
run 1 "index.html: the bare label 'post-quantum encryption'"

copy; plant "$R/README.md" "- dowiz makes no claim of post-quantum encryption (again)."
run 0 "README.md: the same label inside a negation"

copy
python3 - "$WORDS" <<'PY'
import sys, re
p = sys.argv[1]; s = open(p, encoding='utf-8').read()
# Drop every ML-KEM-768 mention from the English block only.
i = s.index('\n  en: {'); j = s.index('\n  sq: {')
s = s[:i] + s[i:j].replace('ML-KEM-768', 'ML-KEM') + s[j:]
open(p, 'w', encoding='utf-8').write(s)
PY
run 1 "landing-words.js: the PQ section deleted from one language"

[ $fail -eq 0 ] && echo "pq-words.prove: the gate fires in both directions" || echo "pq-words.prove: FAILED"
exit $fail
