#!/usr/bin/env bash
# W-MG-ATK4, ATTACK 3: rollback.sh's floor when its state files are damaged. Stubs only: no network, no Cloudflare.
#   bash tools/deploy/atk4-rollback.test.sh     (exit = holes open; every HOLE line is a test that must pass one day)
# The marks live in /root/.cache/dowiz-deploy on ONE box, the directory the disk-full recipe reaps.
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
T=$(mktemp -d "${TMPDIR:-/tmp}/atk4-rollback.XXXXXX"); trap 'rm -rf "$T"' EXIT
pass=0; bad=0
ok() { pass=$((pass + 1)); echo "ok   $1"; }
no() { bad=$((bad + 1)); echo "HOLE $1"; sed 's/^/     | /' "$T/out" | tail -3; }
mkdir -p "$T/bin" "$T/repo/workers/api"
printf '#!/bin/sh\no=; while [ $# -gt 0 ]; do [ "$1" = -o ] && { o=$2; shift; }; shift; done\necho "{\\"failed\\": 0}" > "$o"; printf 200\n' > "$T/bin/curl"
printf '#!/bin/sh\necho "wrangler $*" >> "$STUB_CALLS"\n' > "$T/bin/wrangler"
chmod +x "$T/bin/"*; : > "$T/token"
export STUB_CALLS="$T/calls" ROLLBACK_REPO="$T/repo" ROLLBACK_CURL="$T/bin/curl" ROLLBACK_WRANGLER="$T/bin/wrangler" ROLLBACK_TOKEN_FILE="$T/token" \
  ROLLBACK_ADMIN_TOKEN=tok ROLLBACK_PLATFORM=https://platform.invalid ROLLBACK_PIN_MARK="$T/pin" ROLLBACK_FLIP_MARK="$T/flip" \
  ROLLBACK_FLOOR_MARK="$T/floor" DEPLOY_VERSIONS="$T/versions.log"
run() { : > "$T/calls"; bash "$HERE/rollback.sh" "$@" > "$T/out" 2>&1; RC=$?; }
switched() { grep -c '^wrangler rollback' "$T/calls"; }
state() { # a healthy history: prod (never recorded), the floor, the flip
  echo 'x sha flo0r sha256_venues= declared=1' > "$T/versions.log"; echo 'x sha f1ip sha256_venues=qa-durres declared=1' >> "$T/versions.log"
  echo 'flo0r x sha' > "$T/floor"; echo 'qa-durres flipped at x by f1ip (sha)' > "$T/flip"
  echo 'SHA256_VENUES = "qa-durres"' > "$T/repo/workers/api/wrangler.toml"
}
state; run pr0d; [ $RC = 53 ] && [ "$(switched)" = 0 ] && ok "control: prod refused past a flip (53)" || no "control"
state; run flo0r; [ $RC = 0 ] && [ "$(switched)" = 1 ] && ok "control: the floor switches (0)" || no "floor"
state; rm "$T/versions.log"; run flo0r; [ $RC = 53 ] && ok "held: versions.log missing refuses (53) even the floor" || no "versions missing"
state; sed -i 1d "$T/versions.log"; run f1ip; [ $RC = 53 ] && ok "held: versions.log truncated above the floor line refuses (53)" || no "versions truncated"
state; : > "$T/floor"; run flo0r; [ $RC = 53 ] && ok "held: an empty floor mark refuses (53)" || no "empty floor"
state; rm "$T/floor"; run flo0r; [ $RC = 53 ] && ok "held: a missing floor mark refuses (53)" || no "missing floor"
state; : > "$T/flip"; run pr0d; [ $RC = 53 ] && ok "held: an empty flip mark still counts as a flip (53)" || no "empty flip"
state; ROLLBACK_PAST_FLOOR=1 run pr0d; [ $RC = 0 ] && grep -q 'PAST THE FLOOR' "$T/out" && ok "held: ROLLBACK_PAST_FLOOR=1 switches and says so" || no "past floor"
# THE FAIL-OPEN: no flip mark means "no flip happened", whatever versions.log and the tree say.
state; rm "$T/flip"; run pr0d; [ $RC = 53 ] && [ "$(switched)" = 0 ] && ok "a lost flip mark: versions.log still names the flip, refused" || no "flip mark lost, versions.log says sha256_venues=qa-durres: prod switched rc=$RC"
state; rm "$T/flip"; echo 'SHA256_VENUES = ""' > "$T/repo/workers/api/wrangler.toml"; run pr0d; [ $RC = 53 ] && [ "$(switched)" = 0 ] && ok "mark lost and the var back to empty: versions.log alone refuses" || no "versions.log witness: prod switched rc=$RC"
state; rm "$T/flip" "$T/floor" "$T/versions.log"; run pr0d; [ $RC = 53 ] && [ "$(switched)" = 0 ] && ok "the whole cache reaped: refused while wrangler.toml names a venue" || no "all marks lost, wrangler.toml flipped: prod switched rc=$RC"
echo "atk4-rollback: $pass ok, $bad HOLE"; [ $bad = 0 ]
