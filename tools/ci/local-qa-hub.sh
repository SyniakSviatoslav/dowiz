#!/usr/bin/env bash
# LOCAL qa-durres HUB, FOR THE NIGHTLY ARCHIVED-ORDER CHECK (no Cloudflare account, no repository secrets).
#
#   bash tools/ci/local-qa-hub.sh          # exit 0 GREEN, else the exit code of the failing step
#
# WHAT IT DOES, IN ORDER (each step prints its name and its seconds):
#   1 BUILD    worker-build --release + scripts/strip-wasm.mjs in workers/api (the wrangler.toml [build] command).
#              The repo pins rustc 1.96.1 (rust-toolchain.toml); cargo from ~/.cargo/bin goes through rustup.
#   2 SECRETS  a RANDOM BOOTSTRAP_SECRET (openssl rand) and a random owner password, generated per run. They go
#              into workers/api/.dev.vars (wrangler's local secrets file, deleted on exit) and into a 0600 owner
#              file under the work dir. Nothing is committed, nothing is printed.
#   3 WORKER   `wrangler dev --local` (workerd on this machine: no account, no network to Cloudflare) on
#              127.0.0.1:$QA_HUB_PORT, state in the work dir (fresh each run).
#   4 SEED     tools/ci/qa-hub-seed.mjs: the venue qa-durres, its owner (through POST /api/bootstrap), one dish on sale.
#   5 CHECK    e2e/archive/archived-order.mjs, unchanged, with FLOWS_HOST=http://127.0.0.1:<port> and FLOWS_LOC=qa-durres.
#   6 STOP     the wrangler process group is killed and .dev.vars removed, whatever happened above.
#
# ENVIRONMENT (all optional):
#   QA_HUB_PORT=8787        the local port
#   QA_HUB_WORK=<dir>       work dir (default: a fresh mktemp dir under $RUNNER_TEMP or $TMPDIR)
#   QA_HUB_NO_SLOT=1        do not route the cargo build through bebop-lang/tools/slot.sh (CI sets it: the
#                           runner has no slot cores, and the process-count cap is a box problem, not a CI one)
#   QA_HUB_LOG=<file>       where the archived-order output is also written (the CI summary reads it)
#
# NOT A FLOWS GATE. It rotates the local hub's finished orders (the check is the rotation's point); nothing
# here touches a live venue: every host is 127.0.0.1 and every venue is named qa-durres.
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
API=$ROOT/workers/api
WRANGLER=$ROOT/workers/node_modules/wrangler/wrangler-dist/cli.js
PORT=${QA_HUB_PORT:-8787}
HOST=http://127.0.0.1:$PORT
export PATH="$HOME/.cargo/bin:$PATH"

WORK=${QA_HUB_WORK:-$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/qa-hub.XXXXXX")}
mkdir -p "$WORK"
LOG=${QA_HUB_LOG:-$WORK/archived.log}
T0=$(date +%s)
WPID=
DEV_VARS=$API/.dev.vars

say() { echo "== $* ($(( $(date +%s) - T0 )) s)"; }
# On a runner the job log needs a token to read; an annotation does not. Each failure carries the
# last lines of the log that explains it (%0A is a newline inside an annotation).
annotate() { [ -n "${GITHUB_ACTIONS:-}" ] || return 0; echo "::error title=qa-hub $1::$(tail -25 "$2" 2>/dev/null | sed 's/%/%25/g' | tr '\n' '\001' | sed 's/\x01/%0A/g')"; }
die() { echo "qa-hub: FAIL $*"; exit "${RC:-1}"; }

cleanup() {
  if [ -n "$WPID" ]; then
    # setsid made wrangler a process-group leader: the group takes workerd down with it.
    kill -TERM -- "-$WPID" 2>/dev/null || kill -TERM "$WPID" 2>/dev/null
    wait "$WPID" 2>/dev/null   # a child of this shell: wait blocks until it exits, no polling
    kill -KILL -- "-$WPID" 2>/dev/null
  fi
  if [ -n "${DEV_VARS_MADE:-}" ]; then rm -f "$DEV_VARS"; fi
}
trap cleanup EXIT

# ── tooling ────────────────────────────────────────────────────────────────
for t in node npm openssl curl; do command -v "$t" >/dev/null || { RC=2; die "$t is not on PATH"; }; done
if [ ! -f "$WRANGLER" ]; then
  say "install wrangler (workers/ npm ci)"
  (cd "$ROOT/workers" && npm ci --no-audit --no-fund) || { RC=2; die "npm ci in workers/ rc=$?"; }
fi
if ! command -v worker-build >/dev/null; then
  say "install worker-build 0.8.5"
  cargo install worker-build --version 0.8.5 --locked || { RC=2; die "cargo install worker-build rc=$?"; }
fi

# ── 1 BUILD ────────────────────────────────────────────────────────────────
say "1 build the Worker (worker-build --release, pinned toolchain via rust-toolchain.toml)"
BUILD='cd "$1" && worker-build --release && node scripts/strip-wasm.mjs build/index_bg.wasm --keep build/unstripped'
if [ -n "${QA_HUB_NO_SLOT:-}" ] || [ ! -f "$ROOT/bebop-lang/tools/slot.sh" ]; then
  bash -c "$BUILD" _ "$API" > "$WORK/build.log" 2>&1
else
  # Heavy job on the box: one compute slot (rule: every cargo command goes through slot.sh, in the foreground).
  bash "$ROOT/bebop-lang/tools/slot.sh" qa-hub-build bash -c "$BUILD" _ "$API" > "$WORK/build.log" 2>&1
fi
rc=$?; tail -3 "$WORK/build.log"
[ $rc -eq 0 ] || { RC=$rc; annotate build "$WORK/build.log"; die "build rc=$rc (log $WORK/build.log)"; }

# ── 2 SECRETS ──────────────────────────────────────────────────────────────
say "2 generate per-run secrets"
if [ -e "$DEV_VARS" ]; then RC=2; die "$DEV_VARS already exists: refusing to overwrite a file this script does not own"; fi
BOOTSTRAP_SECRET=$(openssl rand -hex 32)
OWNER_EMAIL=qa-owner@qa-durres.invalid
OWNER_PASSWORD=$(openssl rand -hex 16)
# JWT_SECRET (>= 32 bytes) and AUTH_SIGNING_KEY: without them every login answers 503 "auth misconfigured"
# (the first runner run, a7f7ed30, stopped exactly there).
( umask 077; printf 'BOOTSTRAP_SECRET=%s\nJWT_SECRET=%s\nAUTH_SIGNING_KEY=%s\n' "$BOOTSTRAP_SECRET" "$(openssl rand -hex 32)" "$(openssl rand -hex 32)" > "$DEV_VARS" ) && DEV_VARS_MADE=1 || { RC=2; die "writing $DEV_VARS"; }
chmod 600 "$DEV_VARS"
OWNER_FILE=$WORK/qa_owner
( umask 077
  printf "export OWNER_EMAIL='%s'\nexport OWNER_PASSWORD='%s'\n" "$OWNER_EMAIL" "$OWNER_PASSWORD" > "$OWNER_FILE" )

# ── 3 WORKER ───────────────────────────────────────────────────────────────
say "3 wrangler dev --local on $HOST"
# setsid: a process group of its own, so cleanup can take workerd down with wrangler.
( cd "$API" && exec setsid node --no-warnings "$WRANGLER" dev --local --ip 127.0.0.1 --port "$PORT" \
    --show-interactive-dev-session=false --persist-to "$WORK/state" ) > "$WORK/wrangler.log" 2>&1 &
WPID=$!
# One curl process does the waiting (--retry), so no shell loop is needed. The first start includes wrangler's own build step.
curl -fsS --retry 150 --retry-delay 2 --retry-all-errors --max-time 10 "$HOST/api/version" -o "$WORK/version.json" \
  || { RC=3; tail -20 "$WORK/wrangler.log"; annotate wrangler "$WORK/wrangler.log"; die "wrangler did not answer /api/version (log $WORK/wrangler.log)"; }
echo "   version: $(head -c 200 "$WORK/version.json")"

# ── 4 SEED ─────────────────────────────────────────────────────────────────
say "4 seed qa-durres (bootstrap, owner, one dish on sale)"
QA_HUB_HOST=$HOST BOOTSTRAP_SECRET=$BOOTSTRAP_SECRET OWNER_EMAIL=$OWNER_EMAIL OWNER_PASSWORD=$OWNER_PASSWORD \
  node "$ROOT/tools/ci/qa-hub-seed.mjs" > "$WORK/seed.log" 2>&1; rc=$?
cat "$WORK/seed.log"
[ $rc -eq 0 ] || { RC=$rc; annotate seed "$WORK/seed.log"; annotate wrangler "$WORK/wrangler.log"; die "seed rc=$rc"; }

# ── 5 CHECK ────────────────────────────────────────────────────────────────
say "5 archived-order check against $HOST (FLOWS_LOC=qa-durres)"
( cd "$ROOT" && FLOWS_HOST=$HOST FLOWS_LOC=qa-durres DOWIZ_OWNER_FILE=$OWNER_FILE \
    FLOWS_OUT=$WORK/flows FLOWS_PENDING=$WORK/pending.json \
    node e2e/archive/archived-order.mjs ) 2>&1 | tee "$LOG"
rc=${PIPESTATUS[0]}
echo "archived-order exit=$rc"
[ "$rc" -eq 0 ] || { annotate archived-order "$LOG"; annotate wrangler "$WORK/wrangler.log"; }
say "done"
RC=$rc
exit "$rc"
