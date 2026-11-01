#!/usr/bin/env bash
# W-MG-ATK4, ATTACK 2/3: what deploy.sh records of a flip. A throwaway git repo and stubs for every external (the
# harness of tools/deploy/test.sh): no network, no Cloudflare, no compile.  bash tools/deploy/atk4-deploy.test.sh
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
T=$(mktemp -d "${TMPDIR:-/tmp}/atk4-deploy.XXXXXX"); trap 'rm -rf "$T"' EXIT
pass=0; bad=0
ok() { pass=$((pass + 1)); echo "ok   $1"; }
no() { bad=$((bad + 1)); echo "HOLE $1"; sed 's/^/     | /' "$T/out" | tail -3; }
mkdir -p "$T/bin" "$T/repo/tools/gates" "$T/repo/workers/api/public/admin"
cat > "$T/bin/wrangler" <<'EOF'
#!/bin/sh
case "$1 $2" in
  "deployments status") echo '{"versions":[{"version_id":"pr0d0000-0000-4000-8000-000000000000","percentage":100}]}' ;;
  deploy*) mkdir -p build
    if [ -n "${WASM_NO_COMMIT:-}" ]; then echo x > build/index_bg.wasm; else printf 'wasm\0%s\0' "$DOWIZ_COMMIT" > build/index_bg.wasm; fi
    n=$(cat "$STUB_N"); echo $((n + 1)) > "$STUB_N"; echo "Current Version ID: 4e40000$n-0000-4000-8000-000000000000" ;;
  *) echo "wrangler $*" >> "$STUB_N.calls" ;;
esac
EOF
printf '#!/bin/sh\nshift\nexec "$@"\n' > "$T/bin/slot"
printf '#!/bin/sh\necho "test result: ok. stub"\n' > "$T/bin/cargo"
printf '#!/bin/sh\nexit ${VERIFY_RC:-0}\n' > "$T/bin/verify"
printf '#!/bin/sh\necho PASS\n' > "$T/bin/probe"
printf '#!/bin/sh\no=; while [ $# -gt 0 ]; do [ "$1" = -o ] && { o=$2; shift; }; shift; done\necho "{\\"failed\\": 0}" > "$o"; printf 200\n' > "$T/bin/curl"
chmod +x "$T/bin/"*; : > "$T/token"
printf '#!/bin/sh\necho "run-all: stub"\n' > "$T/repo/tools/gates/run-all.sh"; printf '#!/bin/sh\necho "flows: stub"\n' > "$T/repo/tools/gates/flows.sh"
for f in app core i18n; do echo "export const $f = 1;" > "$T/repo/workers/api/public/admin/$f.js"; done
g() { git -C "$T/repo" -c user.name=t -c user.email=t@t "$@" > /dev/null 2>&1; }
g init -b main
export ROLLBACK_PIN_MARK="$T/none" ROLLBACK_FLOOR_MARK="$T/floor" ROLLBACK_FLIP_MARK="$T/flip" DEPLOY_VERSIONS="$T/versions.log" PATH="$T/bin:$PATH" \
  STUB_N="$T/n" DEPLOY_REPO="$T/repo" DEPLOY_WORK="$T/work" DEPLOY_SLOT="$T/bin/slot" DEPLOY_WRANGLER=wrangler DEPLOY_TOKEN_FILE="$T/token" DEPLOY_CARGO="$T/bin/cargo" \
  DEPLOY_VERIFY="$T/bin/verify" DEPLOY_PROBE="$T/bin/probe" DEPLOY_HOST=https://qa-stub.invalid FLOWS_SETTLE_S=0 DEPLOY_FLOWS=0 \
  ROLLBACK_CURL="$T/bin/curl" ROLLBACK_WRANGLER="$T/bin/wrangler" ROLLBACK_TOKEN_FILE="$T/token" ROLLBACK_ADMIN_TOKEN=tok ROLLBACK_REPO="$T/repo" ROLLBACK_PLATFORM=https://platform.invalid
echo 0 > "$T/n"
commit() { printf '%s\n' "$1" > "$T/repo/workers/api/wrangler.toml"; g add -A; g commit -m "c$(date +%N)"; }
deploy() { bash "$HERE/deploy.sh" > "$T/out" 2>&1; RC=$?; }
fresh() { rm -f "$T/floor" "$T/flip" "$T/versions.log"; }
last() { tail -1 "$T/versions.log" | cut -d' ' -f3; }

fresh; commit 'SHA256_VENUES = ""'; deploy; [ $RC = 0 ] && [ -s "$T/floor" ] && [ ! -e "$T/flip" ] && ok "control: the expand release records a floor and no flip" || no "control expand"
commit 'SHA256_VENUES = "qa-durres"'; deploy; [ -s "$T/flip" ] && ok "control: a flip is marked" || no "control flip"
fresh; commit 'SHA256_VENUES = ""'; VERIFY_RC=1 deploy; [ $RC = 40 ] && [ -s "$T/floor" ] && ok "held: a floor recorded by a deploy that failed verification (the reader is in that build)" || no "failed-verify floor rc=$RC"
# Valid TOML the sed does not read: the flip goes live unrecorded and rollback.sh applies no floor.
fresh; commit 'SHA256_VENUES = ""'; deploy; commit "SHA256_VENUES = '*'"; deploy; [ -s "$T/flip" ] && ok "a single-quoted flip is marked" || no "SHA256_VENUES = '*' (valid TOML): a live flip, no mark"
fresh; commit 'SHA256_VENUES = ""'; deploy; commit '  SHA256_VENUES = "*"'; deploy; [ -s "$T/flip" ] && ok "an indented flip is marked" || no 'indented SHA256_VENUES (valid TOML): a live flip, no mark'
# The version is already live when the wasm check fails, and the record comes after that check.
fresh; commit 'SHA256_VENUES = ""'; deploy; commit 'SHA256_VENUES = "*"'; WASM_NO_COMMIT=1 deploy; [ $RC = 32 ] && [ -s "$T/flip" ] && ok "exit 32 after an upload still marks the flip" || no "exit 32: the flip was uploaded and never recorded (rc=$RC)"
# A commit that drops the var, past a flip: a forward deploy of a build the floor exists to keep out, and then a legal target.
fresh; commit 'SHA256_VENUES = ""'; deploy; commit 'SHA256_VENUES = "qa-durres"'; deploy
commit '# reverted: this build predates the reader'; deploy; D=$RC; [ $D != 0 ] && ok "a build with no reader is refused past a flip" || no "deploy.sh shipped a build that does not declare the var, past a flip (rc=$D)"
# A line recorded past the floor by a build with no reader (a version from before this rule): not a target.
V=$(last); echo "x sha n0reader sha256_venues= declared=0" >> "$T/versions.log"
bash "$HERE/rollback.sh" n0reader > "$T/out" 2>&1; R=$?; [ $R = 53 ] && ok "rollback.sh refuses a post-floor line with no reader (53)" || no "rollback.sh accepts a post-floor line with no reader rc=$R"
bash "$HERE/rollback.sh" "$V" > "$T/out" 2>&1; R=$?; [ $R = 0 ] && ok "twin: the flip's own version switches (0)" || no "twin rc=$R"
# Every mark reaped and the var back to "": the history still says a flip happened.
fresh; commit 'SHA256_VENUES = ""'; commit 'SHA256_VENUES = "qa-durres"'; commit 'SHA256_VENUES = ""'
bash "$HERE/rollback.sh" pr0d > "$T/out" 2>&1; R=$?; [ $R = 53 ] && grep -q 'set SHA256_VENUES' "$T/out" && ok "marks and var gone: a commit that set a list refuses (53)" || no "the git witness rc=$R"
# W-MG-TIDY: deploy's exit 14 reads the witnesses rollback.sh reads. Marks reaped, the tree without the var, and history says a flip.
fresh; commit 'SHA256_VENUES = ""'; commit 'SHA256_VENUES = "qa-durres"'; commit '# reverted: this build predates the reader'; deploy
[ $RC = 14 ] && grep -q 'set SHA256_VENUES' "$T/out" && ok "marks reaped: history alone refuses a build with no reader (14)" || no "deploy.sh shipped no reader past a flip only git remembers rc=$RC"
fresh; rm -rf "$T/repo/.git"; g init -b main; commit "# never declared"; deploy; [ $RC = 0 ] && ok "twin: no witness at all, a build without the var deploys" || no "twin no witness rc=$RC"
# The history witness reads what TOML reads: an inline table and a quoted key set a list too; a comment does not.
for form in 'vars = { SHA256_VENUES = "*" }' "vars = { SHA256_VENUES = 'qa-durres' }" '"SHA256_VENUES" = "*"'; do
  fresh; rm -rf "$T/repo/.git"; g init -b main; commit "$form"; commit 'SHA256_VENUES = ""'
  bash "$HERE/rollback.sh" pr0d > "$T/out" 2>&1; R=$?; [ $R = 53 ] && ok "git witness reads: $form (53)" || no "git witness missed $form rc=$R"
done
for form in '# SHA256_VENUES = "qa-durres"' 'vars = { SHA256_VENUES = "" }'; do
  fresh; rm -rf "$T/repo/.git"; g init -b main; commit "$form"; commit 'SHA256_VENUES = ""'
  bash "$HERE/rollback.sh" pr0d > "$T/out" 2>&1; R=$?; [ $R = 0 ] && ok "twin: no flip in: $form (0)" || no "git witness false alarm on $form rc=$R"
done
echo "atk4-deploy: $pass ok, $bad HOLE"; [ $bad = 0 ]
