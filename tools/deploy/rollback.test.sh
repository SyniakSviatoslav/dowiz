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
  ROLLBACK_PIN_MARK="$T/kv-pinned"
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

echo "rollback.test: $pass passed, $bad failed"
[ $bad = 0 ]
