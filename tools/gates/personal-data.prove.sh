#!/bin/sh
# P1's proof: the gate is triggered before it is trusted.
#
# Runs `personal-data.sh` against scratch copies of the Worker:
#   1. clean                                              -> 0 (exit 0)
#   2. `pub const IMAGE_X: &str = "x";` in lib.rs          -> RED naming "x"
#   3. the same line removed again                         -> GREEN
#   4. the same line inside a comment                      -> GREEN (comments stripped)
#   5. a new https host in shipping code                   -> RED naming it
#   6. the same host inside a #[cfg(test)] module          -> GREEN (tests stripped)
#   7. a new `dw_*` key written to localStorage            -> RED naming it
#   8. a host added to the CSP in public/_headers          -> RED naming it
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
SCRATCH=$(mktemp -d)
trap 'rm -rf "$SCRATCH"' EXIT
fail=0
R="$SCRATCH/r"

copy() {
  rm -rf "$R"; mkdir -p "$R/workers/api"
  cp -r "$REPO/workers/api/src" "$R/workers/api/src"
  (cd "$REPO/workers/api" && find public \( -name '*.js' -o -name '*.html' -o -name '_headers' \) -print) |
    while read -r f; do mkdir -p "$R/workers/api/$(dirname "$f")"; cp "$REPO/workers/api/$f" "$R/workers/api/$f"; done
}
run() { sh "$HERE/personal-data.sh" "$R" >"$SCRATCH/out" 2>&1; echo $?; }
expect() { # label want-rc [must-contain]
  if [ "$rc" -ne "$2" ]; then echo "prove: $1 -> rc=$rc (want $2)"; cat "$SCRATCH/out"; fail=1; return; fi
  if [ -n "${3:-}" ] && ! grep -q "$3" "$SCRATCH/out"; then echo "prove: $1 -> output does not name $3"; cat "$SCRATCH/out"; fail=1; return; fi
  echo "prove: $1 -> rc=$rc (want $2)${3:+ naming $3}: $(grep -m1 "$3\|personal-data [0-9]" "$SCRATCH/out" | sed 's/^ *//')"
}

copy
rc=$(run); expect "clean" 0 "personal-data 0"

printf '\npub const IMAGE_X: &str = "x";\n' >> "$R/workers/api/src/lib.rs"
rc=$(run); expect "IMAGE_X added" 1 '"x"'

copy
rc=$(run); expect "IMAGE_X removed" 0 "personal-data 0"

printf '\n// pub const IMAGE_Y: &str = "y";\n/* pub const K_Z: &str = "z"; */\n' >> "$R/workers/api/src/lib.rs"
rc=$(run); expect "the same constants inside comments" 0 "personal-data 0"

copy
printf '\npub fn leak() -> &'"'"'static str { "https://collector.example.net/in" }\n' >> "$R/workers/api/src/lib.rs"
rc=$(run); expect "a new host in shipping code" 1 "collector.example.net"

copy
printf '\n#[cfg(test)]\nmod t_leak { #[test] fn h() { let _ = "https://collector.example.net/in"; } }\n' >> "$R/workers/api/src/lib.rs"
rc=$(run); expect "the same host in a test module" 0 "personal-data 0"

copy
printf "\ntry { localStorage.setItem('dw_secret_note', 'x'); } catch {}\n" >> "$R/workers/api/public/store/storage.js"
rc=$(run); expect "a new browser key" 1 "dw_secret_note"

copy
sed -i 's#connect-src #connect-src https://beacon.example.org #' "$R/workers/api/public/_headers"
rc=$(run); expect "a host added to the CSP" 1 "beacon.example.org"

[ $fail -eq 0 ] && echo "personal-data.prove: the gate fires in every direction" || echo "personal-data.prove: FAILED"
exit $fail
