#!/bin/sh
# PQ-0 — THE PUBLIC WORDS ABOUT POST-QUANTUM SAY ONLY WHAT IS LIVE.
#
# THE DEFECT, measured live on 2026-10-02 (docs/research/2026-10-02-system-
# integration-check.md §3): dowiz.org said, in four languages, "X25519 +
# ML-KEM-768 on the channel, ML-DSA-65 on every packet, AES-256-GCM at rest ...
# there is no classical-only fallback". Every clause was false. Tokens are
# HMAC-SHA256 (`workers/api/src/auth.rs:34`), transport is Cloudflare TLS,
# ML-DSA-65 has no caller in the Worker (`grep -rl "dsa::" workers/api/src` ->
# nothing), the hybrid backup seal is wired but OFF until `BACKUP_SEAL_PK` is
# set (`workers/api/src/cloud/seal.rs:40-63`, `cloud.rs:434`), and AES-GCM at
# rest (`crates/dowiz-hub/src/shred.rs`) sits behind the hub's default-off
# `shred` feature, which the Worker never enables. README.md and SECURITY.md
# said so precisely; the landing contradicted them for days.
#
# THE RULE (operator, 2026-10-02): post-quantum stays PUBLIC, and the public
# text states exactly what is true. What is true today: the two FIPS
# primitives exist, pass the NIST ACVP vectors in CI (`.github/workflows/
# ci.yml:50` runs `cargo test` in dowiz-core; `pq/kem/acvp_tests.rs`,
# `pq/dsa/dsa_acvp_tests.rs`), and are not on any live path. So this gate
# refuses two shapes of sentence:
#
#   1. LIVE-PQ CLAIMS, on any surface a visitor or a reader can see
#      (workers/api/public/**, README.md, SECURITY.md): "every packet" signed,
#      ML-KEM "on the channel"/"channels negotiate", "no classical-only
#      fallback", AES-256-GCM "at rest"/"локально", "encrypted at rest" — in
#      uk, ru, en and sq.
#   2. THE BARE LABEL "post-quantum encryption" / "post-quantum (delivery)
#      protocol" and its translations on the PUBLIC surfaces only
#      (workers/api/public/**). README and SECURITY use the phrase in a
#      negation ("makes no claim of post-quantum encryption") and keep it.
#      The public word is "post-quantum-ready", or the named primitive.
#
# And one shape of DELETION: landing-words.js must still name ML-KEM-768 in
# all four languages. Retracting a false claim by deleting the section would
# satisfy a grep and defy the operator; say what is true instead.
#
# WHAT IT DOES NOT CHECK: whether a sentence that passes is the WHOLE truth, or
# the translation faithful. When F16 lands (key installed, one sealed copy
# opened) the second sentence of p3p changes; this file does not need to.
#
# `sh tools/gates/pq-words.sh [ROOT]` — ROOT defaults to the repo and is there
# so `pq-words.prove.sh` can run this same code against a scratch copy holding
# a planted phrase. Exit 1 on any hit; no baseline, the tolerance is zero.
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT="${1:-$(cd "$HERE/../.." && pwd)}"
cd "$ROOT"

PUBLIC=workers/api/public
WORDS=$PUBLIC/platform/landing-words.js
[ -f "$WORDS" ] || { echo "pq-words: $WORDS is missing — the landing moved, not the words; update this gate"; exit 1; }

# 1. Live-PQ claims. One alternation per clause, every language on one line so
#    a reader can see the four translations side by side. BYTE-SAFE on purpose:
#    the box runs grep in the C locale, where `[а-я]*` is a range over bytes and
#    matches nothing Cyrillic (pq-words.prove.sh caught exactly that), so word
#    tails are `[^ ]*`, never a Cyrillic class.
LIVE='(every|each) packet|кожн[^ ]* пакет|кожен пакет|кажд[^ ]* пакет|çdo paket'
LIVE="$LIVE|classical[- ]only|класичн[^ ]* [^ ]*запасн|классическ[^ ]* [^ ]*запасн|rezervë klasike"
LIVE="$LIVE|ML-KEM-768 (on|in) the channel|ML-KEM-768 (у|в) канал|ML-KEM-768 në kanal|channels? negotiate|канал (узгоджується|согласуется)|kanali negociohet"
LIVE="$LIVE|AES-256-GCM (at rest|локально|lokalisht)|encrypted at rest|зашифрован[^ ]* локально|kriptohen lokalisht"

# 2. The bare label, public surfaces only.
LABEL='post-quantum (encryption|delivery protocol|protocol)|постквантов(е|ий) (шифрування|протокол)|постквантов(ое|ый) (шифрование|протокол)|kriptim post-kuantik|lokal, post-kuantik'

hits=0
report() { # pattern, files...
  pat=$1; shift
  out=$(grep -rInE -i -e "$pat" "$@" 2>/dev/null || true)
  [ -z "$out" ] && return 0
  printf '%s\n' "$out" | sed 's/^/  /'
  hits=$((hits + $(printf '%s\n' "$out" | wc -l | tr -d ' ')))
}

echo "pq-words: live-PQ claims (public + README + SECURITY):"
report "$LIVE" "$PUBLIC" README.md SECURITY.md
echo "pq-words: bare 'post-quantum encryption/protocol' label (public only):"
report "$LABEL" "$PUBLIC"

# 3. The section is still there, in EVERY language block (`  xx: {` ... next
#    block). Counting lines was not enough: three languages' two lines each
#    out-count four languages (pq-words.prove.sh, case 10).
langs=$(grep -cE '^  [a-z][a-z]: \{' "$WORDS" || true)
silent=$(awk '
  /^  [a-z][a-z]: \{/ { if (lang != "" && !seen) print lang; lang = $1; seen = 0; next }
  /ML-KEM-768/        { seen = 1 }
  END                 { if (lang != "" && !seen) print lang }' "$WORDS" | tr -d ':' | tr '\n' ' ')
if [ "$langs" -lt 4 ] || [ -n "$silent" ]; then
  echo "  $WORDS: $langs language block(s); ML-KEM-768 missing from: ${silent:-none}"
  echo "pq-words: REFUSED — post-quantum must stay PUBLIC (operator 2026-10-02): say what is true, do not delete it."
  exit 1
fi

if [ "$hits" -gt 0 ]; then
  echo "pq-words: REFUSED — $hits line(s) claim post-quantum protection that is not live."
  echo "pq-words: the true sentence is in docs/research/2026-10-02-system-integration-check.md §3.3;"
  echo "pq-words: name the primitive and the NIST ACVP vectors, say what the live path uses (HMAC-SHA256, TLS),"
  echo "pq-words: and call the seal 'built and switched off' until BACKUP_SEAL_PK is installed (F16)."
  exit 1
fi
files=$(find "$PUBLIC" -type f | wc -l | tr -d ' ')
echo "pq-words: 0 hits in $files public files + README.md + SECURITY.md; ML-KEM-768 named in $langs languages"
