#!/usr/bin/env bash
# deploy.sh's refusals and its happy path, against a throwaway git repo and stubs for every
# external: wrangler (on PATH), slot.sh, cargo, the verifier and the CRUD probe. No network,
# no Cloudflare, no compile.     bash tools/deploy/test.sh
# Each refusal is checked for its exit code, its named FAIL line, AND that wrangler was never
# asked to upload; each has a positive twin. The live-commit mismatch itself is proved in
# verify.test.mjs (a stub Worker in-process); here, that a verifier FAIL stops the run with
# the rollback command printed and no rollback run.
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
T=$(mktemp -d "${TMPDIR:-/tmp}/deploy-test.XXXXXX")
trap 'rm -rf "$T"' EXIT
pass=0; bad=0
ok()  { pass=$((pass + 1)); echo "ok   $1"; }
no()  { bad=$((bad + 1)); echo "FAIL $1"; sed 's/^/     | /' "$T/out" | tail -8; }

mkdir -p "$T/bin" "$T/repo"
cat > "$T/bin/wrangler" <<'EOF'
#!/bin/sh
echo "$* token=${CLOUDFLARE_API_TOKEN:+set}" >> "$STUB_CALLS"
case "$1 $2" in
  "deployments status") [ -n "${NO_PREV:-}" ] && { echo "error: nope"; exit 1; }
    if [ -n "${STATUS_TIMEOUTS:-}" ] && [ "$(grep -c '^deployments status' "$STUB_CALLS")" -le "$STATUS_TIMEOUTS" ]; then
      echo "X [ERROR] The request to Cloudflare's API timed out."; exit 1; fi
    echo '{"versions":[{"version_id":"0ld00000-0000-4000-8000-000000000000","percentage":100}]}' ;;
  deploy*) mkdir -p build
    if [ -n "${FETCH_FAILS:-}" ] && [ "$(grep -c '^deploy ' "$STUB_CALLS")" -le "$FETCH_FAILS" ]; then echo "X [ERROR] fetch failed"; exit 1; fi
    [ -n "${UPLOAD_ERR:-}" ] && { echo "X [ERROR] Authentication error [code: 10000]"; exit 1; }
    if [ -n "${WASM_NO_COMMIT:-}" ]; then echo x > build/index_bg.wasm; else printf 'wasm\0%s\0' "$DOWIZ_COMMIT" > build/index_bg.wasm; fi
    case "$*" in *--dry-run*) echo "--dry-run: exiting now." ;; *) echo "Current Version ID: 4e400000-0000-4000-8000-000000000000" ;; esac ;;
esac
EOF
printf '#!/bin/sh\nshift\nexec "$@"\n' > "$T/bin/slot"
printf '#!/bin/sh\necho "test result: ok. stub"\nexit ${CARGO_RC:-0}\n' > "$T/bin/cargo"
printf '#!/bin/sh\necho "verify args: $*"\nexit ${VERIFY_RC:-0}\n' > "$T/bin/verify"
printf '#!/bin/sh\necho "PASS stub probe host=$HOST"\nexit ${PROBE_RC:-0}\n' > "$T/bin/probe"
chmod +x "$T/bin/"*
printf '# a comment \342\200\224 like the real file\nexport CLOUDFLARE_API_TOKEN=stub-secret-9f3k\n' > "$T/token"

g() { git -C "$T/repo" -c user.name=t -c user.email=t@t "$@" > /dev/null 2>&1; }
g init -b main
mkdir -p "$T/repo/tools/gates" "$T/repo/workers/api/public/admin"
printf '#!/bin/sh\necho "run-all: stub"\nexit ${GATES_RC:-0}\n' > "$T/repo/tools/gates/run-all.sh"
for f in app core i18n; do echo "export const $f = 1;" > "$T/repo/workers/api/public/admin/$f.js"; done
ln -s admin/app.js "$T/repo/workers/api/public/link.js"
g add -A; g commit -m one
SHA=$(git -C "$T/repo" rev-parse HEAD)

export PATH="$T/bin:$PATH" STUB_CALLS="$T/calls" DEPLOY_REPO="$T/repo" DEPLOY_WORK="$T/work" \
  DEPLOY_SLOT="$T/bin/slot" DEPLOY_WRANGLER=wrangler DEPLOY_TOKEN_FILE="$T/token" DEPLOY_CARGO="$T/bin/cargo" \
  DEPLOY_VERIFY="$T/bin/verify" DEPLOY_PROBE="$T/bin/probe" DEPLOY_HOST=https://qa-stub.invalid
run() { : > "$T/calls"; bash "$HERE/deploy.sh" "$@" > "$T/out" 2>&1; RC=$?; }
uploads() { grep -c '^deploy token=' "$T/calls"; }
has() { grep -q -- "$1" "$T/out"; }

# ── refusals, each with no upload ──────────────────────────────────────────
g checkout --detach
run; [ $RC = 10 ] && has 'FAIL not-main: HEAD is detached' && [ "$(uploads)" = 0 ] && ok "detached HEAD refused (10), no upload" || no "detached HEAD"
g checkout -b feature
run; [ $RC = 10 ] && has 'FAIL not-main: HEAD is refs/heads/feature' && [ "$(uploads)" = 0 ] && ok "branch feature refused (10), no upload" || no "non-main branch"
g checkout main

echo "// uncommitted" >> "$T/repo/workers/api/public/admin/core.js"
run; [ $RC = 11 ] && has 'FAIL dirty-tree: tracked files differ' && has 'admin/core.js' && [ "$(uploads)" = 0 ] && ok "dirty tracked file refused (11), names the file" || no "dirty tree"
# staged, then the working file put back to HEAD's bytes: only the index differs now
g add workers/api/public/admin/core.js; git -C "$T/repo" show HEAD:workers/api/public/admin/core.js > "$T/repo/workers/api/public/admin/core.js"
run; [ $RC = 11 ] && has 'FAIL dirty-tree: the index differs' && [ "$(uploads)" = 0 ] && ok "staged-only change refused (11)" || no "staged change"
g restore --staged workers/api/public/admin/core.js; g restore workers/api/public/admin/core.js

GATES_RC=1 run; [ $RC = 20 ] && has 'FAIL gates:' && [ ! -s "$T/calls" ] && ok "run-all red refused (20), wrangler never called" || no "gates fail"
CARGO_RC=101 run; [ $RC = 21 ] && has 'FAIL cargo-test:' && [ ! -s "$T/calls" ] && ok "cargo test red refused (21), wrangler never called" || no "cargo fail"
NO_PREV=1 run; [ $RC = 30 ] && has 'FAIL previous:' && [ "$(uploads)" = 0 ] && ok "no readable previous version refused (30), no upload" || no "no previous"

# ── positive twin: untracked junk does not block and is not shipped; the token is never printed ──
echo "junk" > "$T/repo/workers/api/public/untracked.js"
run
if [ $RC = 0 ] && has "deploy: OK $SHA" && [ "$(uploads)" = 1 ] && grep -q '^deploy token=set' "$T/calls" \
   && grep -q "^deployments status --json token=set" "$T/calls" && [ ! -e "$T/work/src/workers/api/public/untracked.js" ] \
   && [ "$(readlink "$T/work/src/workers/api/public/link.js")" = admin/app.js ] && has "verify args: --host https://qa-stub.invalid --commit $SHA" \
   && ! grep -rq 'stub-secret-9f3k' "$T/out" "$T/work/logs"; then ok "clean main deploys once (0), copy == HEAD, token never printed"; else no "happy path"; fi

# ── a commit whose gates + cargo test passed is not re-gated; a failed one is; DEPLOY_REGATE forces ──
rm -rf "$T/work/passed"; GATES_RC=1 run; [ $RC = 20 ] && [ ! -e "$T/work/passed/$SHA" ] || no "red gates leave no pass mark"
CARGO_RC=1 run; [ $RC = 21 ] && [ ! -e "$T/work/passed/$SHA" ] || no "red cargo test leaves no pass mark"
UPLOAD_ERR=1 run; [ $RC = 31 ] && [ -e "$T/work/passed/$SHA" ] && has 'run-all: stub' || no "green gates leave a pass mark"
GATES_RC=1 run; [ $RC = 0 ] && has 'already passed on' && ! has 'run-all: stub' && ! has 'test result: ok. stub' \
  && ok "same commit after an upload failure: gates + cargo test skipped (0)" || no "pass mark skip"
GATES_RC=1 DEPLOY_REGATE=1 run; [ $RC = 20 ] && ok "DEPLOY_REGATE=1 re-runs the gates (red -> 20)" || no "regate"
rm -rf "$T/work/passed"

# ── upload: the box's connect timeout ("fetch failed") is retried, up to 3 tries; any other error is not ──
FETCH_FAILS=2 run; [ $RC = 0 ] && [ "$(uploads)" = 3 ] && has 'upload try 2: fetch failed' && has "deploy: OK $SHA" \
  && ok "fetch failed twice, third upload lands (0), 3 uploads" || no "fetch-failed retry"
FETCH_FAILS=3 run; [ $RC = 31 ] && [ "$(uploads)" = 3 ] && has 'FAIL upload: wrangler rc=1' \
  && ok "fetch failed three times -> gives up (31) after 3 uploads" || no "fetch-failed give up"
UPLOAD_ERR=1 run; [ $RC = 31 ] && [ "$(uploads)" = 1 ] && ! has 'retrying' \
  && ok "any other upload error fails at once (31), no retry" || no "no retry on other errors"

STATUS_TIMEOUTS=2 run; [ $RC = 0 ] && [ "$(grep -c '^deployments status' "$T/calls")" = 3 ] && has 'previous try 2: Cloudflare API timed out' \
  && ok "deployments status timed out twice, third read lands (0)" || no "status timeout retry"
STATUS_TIMEOUTS=3 run; [ $RC = 30 ] && [ "$(uploads)" = 0 ] && ok "status timed out three times -> refused (30), no upload" || no "status timeout give up"

# ── after the upload: a verifier or probe FAIL stops with the rollback command, and runs none ──
VERIFY_RC=1 run; [ $RC = 40 ] && has 'FAIL live-is-not-head' && has 'rollback 0ld00000-0000-4000-8000-000000000000' \
  && ! grep -q rollback "$T/calls" && ok "live != HEAD fails (40), prints rollback to the previous id, runs none" || no "verify fail"
PROBE_RC=1 run; [ $RC = 41 ] && has 'FAIL probe-crud' && has 'rollback 0ld00000' && ! grep -q rollback "$T/calls" && ok "probe_crud red fails (41) with the rollback command" || no "probe fail"
WASM_NO_COMMIT=1 run; [ $RC = 32 ] && has 'FAIL wasm-commit' && ok "a wasm without the commit fails (32)" || no "wasm lacks commit"

# ── the copy: unchanged files keep their mtime, extras go, tampering is repaired ──
F=$T/work/src/workers/api/public/admin/app.js
touch -d '2020-01-01' "$F"; echo stale > "$T/work/src/workers/api/public/stale.js"
echo "tampered" > "$T/work/src/workers/api/public/admin/i18n.js"
echo "export const core = 2;" > "$T/repo/workers/api/public/admin/core.js"; g commit -am two
run
[ $RC = 0 ] && [ "$(date -r "$F" +%Y)" = 2020 ] && [ ! -e "$T/work/src/workers/api/public/stale.js" ] \
  && grep -q 'core = 2' "$T/work/src/workers/api/public/admin/core.js" && grep -q 'i18n = 1' "$T/work/src/workers/api/public/admin/i18n.js" \
  && has 'wrote 1, removed 1 extra, repaired 1' && ok "sync rewrites only the changed file, deletes extras, repairs tampering" || no "sync"

# ── dry run: builds and checks the wasm, never reads the token, never asks for the previous version ──
run --dry-run; [ $RC = 0 ] && has 'DRY-RUN OK' && [ "$(cat "$T/calls")" = "deploy --dry-run --outdir $T/work/dry token=" ] && ok "--dry-run: one no-upload build, no token" || no "dry run"
run --verify; [ $RC = 0 ] && [ ! -s "$T/calls" ] && has 'deploy: OK' && ok "--verify: checks live only, no wrangler" || no "verify mode"

echo "deploy.test: $pass passed, $bad failed"
[ $bad = 0 ]
