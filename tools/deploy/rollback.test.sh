#!/usr/bin/env bash
# rollback.sh against stubs for curl and wrangler: no network, no Cloudflare.   bash tools/deploy/rollback.test.sh
# The rule under test: the Worker is switched ONLY after every venue answered the compaction, and the
# compaction comes FIRST. Each refusal is checked for its exit code, its FAIL line and that wrangler was never
# called; each has a positive twin.
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
T=$(mktemp -d "${TMPDIR:-/tmp}/rollback-test.XXXXXX")
trap 'rm -rf "$T"' EXIT
pass=0; bad=0
ok() { pass=$((pass + 1)); echo "ok   $1"; }
no() { bad=$((bad + 1)); echo "FAIL $1"; sed 's/^/     | /' "$T/out" | tail -8; }

mkdir -p "$T/bin" "$T/repo/workers/api"
cat > "$T/bin/curl" <<'EOF'
#!/bin/sh
# -o FILE -w FMT ... URL (last). Logs "curl <url> auth=<bearer>" and answers per env.
out=; auth=; url=
while [ $# -gt 0 ]; do
  case "$1" in -o) out=$2; shift ;; -H) case "$2" in authorization:*) auth=${2#authorization: Bearer };; esac; shift ;;
    -w|-X|--data-binary) shift ;; -*) ;; *) url=$1 ;; esac; shift
done
echo "curl $url auth=$auth" >> "$STUB_CALLS"
case "$url" in
  */api/auth/login) echo '{"access_token":"tok-admin"}' > "$out"; printf '%s' "${LOGIN_CODE:-200}" ;;
  */api/platform/compact*) echo "{\"failed\": ${COMPACT_FAILED:-0}, \"venues\": []}" > "$out"; printf '%s' "${COMPACT_CODE:-200}" ;;
  *) echo '{}' > "$out"; printf 404 ;;
esac
EOF
printf '#!/bin/sh\necho "wrangler $* token=${CLOUDFLARE_API_TOKEN:+set}" >> "$STUB_CALLS"\nexit ${WRANGLER_RC:-0}\n' > "$T/bin/wrangler"
chmod +x "$T/bin/"*
printf 'export CLOUDFLARE_API_TOKEN=stub-secret-77q\n' > "$T/token"
printf 'export ADMIN_EMAIL=admin@x.test\nexport ADMIN_PASSWORD=pw-secret-31z\n' > "$T/admin"

export STUB_CALLS="$T/calls" ROLLBACK_REPO="$T/repo" ROLLBACK_CURL="$T/bin/curl" ROLLBACK_WRANGLER="$T/bin/wrangler" \
  ROLLBACK_TOKEN_FILE="$T/token" ROLLBACK_ADMIN_FILE="$T/admin" ROLLBACK_PLATFORM=https://platform.invalid \
  ROLLBACK_PIN_MARK="$T/kv-pinned" ROLLBACK_FLIP_MARK="$T/sha256-flipped" ROLLBACK_FLOOR_MARK="$T/sha256-floor" \
  DEPLOY_VERSIONS="$T/versions.log"
run() { : > "$T/calls"; bash "$HERE/rollback.sh" "$@" > "$T/out" 2>&1; RC=$?; }
has() { grep -q -- "$1" "$T/out"; }
switched() { grep -c '^wrangler rollback' "$T/calls"; }

run; [ $RC = 2 ] && [ "$(switched)" = 0 ] && ok "no version id: usage (2), no switch" || no "usage"

# ── the happy path: login, compaction, THEN the switch; no secret printed ──
run 0ld00000-0000 "deploy.sh: abc failed"
if [ $RC = 0 ] && [ "$(switched)" = 1 ] && [ "$(sed -n 2p "$T/calls")" = "curl https://platform.invalid/api/platform/compact auth=tok-admin" ] \
   && [ "$(sed -n 3p "$T/calls")" = "wrangler rollback 0ld00000-0000 -m deploy.sh: abc failed token=set" ] \
   && ! grep -q 'pw-secret-31z\|stub-secret-77q' "$T/out"; then ok "compact (as admin) BEFORE the switch, then switched (0), no secret printed"
else no "happy path"; fi

# ── refusals: no switch ──
COMPACT_CODE=502 COMPACT_FAILED=1 run 0ld0; [ $RC = 50 ] && has 'FAIL compact' && has 'NOT switched' && [ "$(switched)" = 0 ] \
  && ok "a venue failed to compact: 502 -> refused (50), no switch" || no "compact 502"
COMPACT_CODE=404 run 0ld0; [ $RC = 50 ] && [ "$(switched)" = 0 ] && ok "the route is missing (404): refused (50), no switch" || no "compact 404"
LOGIN_CODE=401 run 0ld0; [ $RC = 51 ] && has 'FAIL admin' && [ "$(switched)" = 0 ] && ok "administrator login refused: (51), no switch" || no "login"
ROLLBACK_ADMIN_FILE=$T/none run 0ld0; [ $RC = 51 ] && [ "$(switched)" = 0 ] && ok "no admin credentials: (51), no switch" || no "no admin file"
ROLLBACK_ADMIN_FILE=$T/none ROLLBACK_ADMIN_TOKEN=tok-env run 0ld0; [ $RC = 0 ] && grep -q 'compact auth=tok-env' "$T/calls" \
  && ok "twin: a token from the environment is used as is (0)" || no "env token"
WRANGLER_RC=1 run 0ld0; [ $RC = 52 ] && has 'FAIL switch' && ok "wrangler rollback failed: (52)" || no "wrangler fail"

# ── the explicit skip, loud ──
ROLLBACK_SKIP_COMPACT=1 run 0ld0; [ $RC = 0 ] && has 'SKIPPED by ROLLBACK_SKIP_COMPACT=1' && ! grep -q compact "$T/calls" && [ "$(switched)" = 1 ] \
  && ok "ROLLBACK_SKIP_COMPACT=1: no compaction, says so, switches (0)" || no "skip"

# ── the pin mark: written by a compacting rollback, never by a refused one; deploy.sh lifts it ──
rm -f "$T/kv-pinned"; COMPACT_CODE=502 COMPACT_FAILED=1 run 0ld0; [ ! -e "$T/kv-pinned" ] && ok "refused compaction leaves no pin mark" || no "mark on 502"
run 0ld0; [ $RC = 0 ] && [ -s "$T/kv-pinned" ] && ok "a compacting rollback writes the pin mark" || no "mark written"

# ── unpin: the lift, and nothing else ──
run --unpin; [ $RC = 0 ] && grep -q 'compact?pin=0 auth=tok-admin' "$T/calls" && [ "$(switched)" = 0 ] && [ ! -e "$T/kv-pinned" ] \
  && ok "--unpin: POST compact?pin=0, no switch (0), pin mark removed" || no "unpin"

# ── the sha256 floor (W-MG-MIG): once a venue writes sha256 ids, a target that cannot read them is refused ──
# deploy.sh's records: the floor is the first build that reads sha256 (f100), then a flip (f200), then one more (f300).
printf '2026-11-01T00:00:00Z sha0 0ld00000-0000 sha256_venues=\n2026-11-02T00:00:00Z sha1 f100 sha256_venues=\n2026-11-03T00:00:00Z sha2 f200 sha256_venues=qa-durres\n2026-11-04T00:00:00Z sha3 f300 sha256_venues=*\n' > "$T/versions.log"
echo "f100 2026-11-02T00:00:00Z sha1" > "$T/sha256-floor"
rm -f "$T/sha256-flipped"
run 0ld00000-0000; [ $RC = 0 ] && [ "$(switched)" = 1 ] && ! has 'sha256 floor' \
  && ok "no venue flipped: the floor does not apply, an old target switches (0)" || no "floor off before a flip"
echo "qa-durres flipped at 2026-11-03T00:00:00Z by f200 (sha2)" > "$T/sha256-flipped"
run 0ld00000-0000; [ $RC = 53 ] && has 'FAIL floor' && has 'QUARANTINE every order' && [ ! -s "$T/calls" ] \
  && ok "flipped + target before the floor: refused (53) before compact or switch, says what it would do" || no "floor refusal"
run unknown-0000; [ $RC = 53 ] && [ ! -s "$T/calls" ] && ok "flipped + a target deploy.sh never recorded: refused (53)" || no "floor unknown target"
run f100; [ $RC = 0 ] && [ "$(switched)" = 1 ] && has 'f100 reads sha256 records' && ok "twin: the floor itself switches (0)" || no "floor target ok"
run f300 m; [ $RC = 0 ] && [ "$(switched)" = 1 ] && ok "twin: a version after the floor switches (0)" || no "post-floor target ok"
ROLLBACK_PAST_FLOOR=1 run 0ld00000-0000; [ $RC = 0 ] && has 'PAST THE FLOOR by ROLLBACK_PAST_FLOOR=1' && [ "$(switched)" = 1 ] \
  && ok "ROLLBACK_PAST_FLOOR=1: switches anyway (0) and names the consequence" || no "past floor override"
mv "$T/sha256-floor" "$T/floor.away"; run f300; [ $RC = 53 ] && ok "flipped but no floor record: refused (53), the target cannot be proved to read sha256" || no "no floor mark"
mv "$T/floor.away" "$T/sha256-floor"
run --unpin; [ $RC = 0 ] && ok "--unpin is not a rollback: the floor is not asked (0)" || no "unpin under flip"

echo "rollback.test: $pass passed, $bad failed"
[ $bad = 0 ]
