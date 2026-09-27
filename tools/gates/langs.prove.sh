#!/bin/sh
# langs' proof: a gate is triggered before it is trusted.
#
# Runs `langs.sh` against scratch copies of the trees it reads:
#   1. clean                                                   -> must pass
#   2. the Rust LANGS reordered (set)                          -> must refuse
#   3. a hardcoded three-language list in code (literal)       -> must refuse
#   4. one Russian word deleted from a dictionary (keys)       -> must refuse
#   5. a match arm's "ru" line deleted (parity)                -> must refuse
#   6. a per-language sibling removed, notice/ru.rs (files)    -> must refuse
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
SCRATCH=$(mktemp -d)
trap 'rm -rf "$SCRATCH"' EXIT
fail=0
R="$SCRATCH/r"

copy() {
  rm -rf "$R"
  for d in workers/api/public workers/api/src crates/dowiz-hub/src tools/learn tools/evals docs/privacy; do
    mkdir -p "$R/$(dirname "$d")"
    cp -r "$REPO/$d" "$R/$d"
  done
  rm -rf "$R/workers/api/public/kit"
}
want() { # want <rc> <label>
  sh "$HERE/langs.sh" "$R" >"$SCRATCH/out" 2>&1; rc=$?
  echo "prove: $2 -> rc=$rc (want $1): $(grep -m1 '^  [a-z]' "$SCRATCH/out" || tail -1 "$SCRATCH/out")"
  [ "$rc" -eq "$1" ] || { tail -20 "$SCRATCH/out"; fail=1; }
}
edit() { # edit <file> <old> <new>: exactly one replacement, or the proof fails
  python3 - "$1" "$2" "$3" <<'PY' || { echo "prove: could not edit $1"; fail=1; }
import sys
p, old, new = sys.argv[1:4]
s = open(p, encoding='utf-8').read()
assert s.count(old) == 1, f'{old!r} occurs {s.count(old)} times in {p}'
open(p, 'w', encoding='utf-8').write(s.replace(old, new))
PY
}

copy
want 0 "clean"

copy
edit "$R/crates/dowiz-hub/src/lang.rs" '["sq", "en", "uk", "ru"]' '["sq", "en", "ru", "uk"]'
want 1 "Rust LANGS reordered"

copy
edit "$R/workers/api/public/admin/stock.js" "import '/admin/ingredients-i18n.js';" "import '/admin/ingredients-i18n.js';
export const THREE = ['sq', 'en', 'uk'];"
want 1 "a hardcoded ['sq', 'en', 'uk'] in admin/stock.js"

copy
edit "$R/workers/api/public/admin/access-i18n.js" "acc_changePw: 'Сменить пароль', " ""
want 1 "ru acc_changePw deleted from admin/access-i18n.js"

copy
edit "$R/workers/api/src/services/campaigns/send.rs" '        "ru" => "Ответьте STOP, чтобы больше не получать эти сообщения.",
' ''
want 1 'the "ru" arm deleted from campaigns/send.rs'

copy
rm "$R/workers/api/src/privacy/notice/ru.rs"
want 1 "privacy/notice/ru.rs removed"

[ "$fail" -eq 0 ] && echo "langs.prove: every rule fired" || echo "langs.prove: FAILED"
exit "$fail"
