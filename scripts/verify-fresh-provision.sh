#!/usr/bin/env bash
#
# verify-fresh-provision.sh
# ─────────────────────────
# From-scratch provisioning smoke test. Proves a BRAND-NEW database can:
#   1. migrate:up      (apply every migration cleanly, no manual steps)
#   2. seed            (load the complete lifecycle fixture)
#   3. boot the API    (which starts pg-boss with migrate:false)
#   4. serve /health   (HTTP 200)
#   5. serve a published storefront's menu with products
#
# This closes the gap left by `verify:migrations`, which only checks migration
# ordering/idempotency and MISSED the orders-comma, dup-policy, MAX(uuid),
# pgboss-not-installed, missing-role, and active/open status bugs.
#
# SAFETY: operates ONLY on the database in the *_TEST env vars below. It will
# refuse to run against any *.supabase.com host. Never point this at prod.
#
# Usage (local):
#   PGHOST=127.0.0.1 PGPORT=5432 MIGRATOR_URL=... APP_URL=... ./scripts/verify-fresh-provision.sh
# Defaults match the local docker/postgres-service setup used in CI.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# ── Config (override via env) ───────────────────────────────────────────────
PGHOST="${PGHOST:-127.0.0.1}"
PGPORT="${PGPORT:-5432}"
DBNAME="${DBNAME:-dowiz_freshcheck}"
MIGRATOR_ROLE="${MIGRATOR_ROLE:-dowiz_migrator}"
MIGRATOR_PW="${MIGRATOR_PW:-migrator_pw}"
APP_ROLE="${APP_ROLE:-dowiz_app}"
APP_PW="${APP_PW:-app_pw}"
REDIS_URL="${REDIS_URL:-redis://127.0.0.1:6379}"
API_PORT="${API_PORT:-3055}"

# Superuser connection used only for DROP/CREATE DATABASE + grants. Defaults to
# the migrator role (a superuser in our setup). In CI override SUPERUSER_URL to
# the postgres-service admin URL. Must target the maintenance 'postgres' db.
SUPERUSER_URL="${SUPERUSER_URL:-postgresql://${MIGRATOR_ROLE}:${MIGRATOR_PW}@${PGHOST}:${PGPORT}/postgres}"
SUPERUSER_DB_URL="${SUPERUSER_URL%/*}/${DBNAME}"

MIGRATOR_URL="postgresql://${MIGRATOR_ROLE}:${MIGRATOR_PW}@${PGHOST}:${PGPORT}/${DBNAME}"
APP_URL="postgresql://${APP_ROLE}:${APP_PW}@${PGHOST}:${PGPORT}/${DBNAME}"

# ── Prod guard ──────────────────────────────────────────────────────────────
case "$PGHOST$SUPERUSER_URL$MIGRATOR_URL$APP_URL" in
  *supabase.com*) echo "REFUSING: target looks like prod (supabase.com)"; exit 1 ;;
esac

PSQL_SU=(psql "$SUPERUSER_URL" -v ON_ERROR_STOP=1 -tAX)

fail() { echo "❌ FRESH-PROVISION FAILED at: $1"; [ -f /tmp/fresh-api.log ] && tail -30 /tmp/fresh-api.log; exit 1; }

echo "── STEP 0: drop & recreate database (truly from scratch) ──"
"${PSQL_SU[@]}" -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname='${DBNAME}' AND pid<>pg_backend_pid();" >/dev/null 2>&1 || true
"${PSQL_SU[@]}" -c "DROP DATABASE IF EXISTS ${DBNAME};" || fail "drop db"
"${PSQL_SU[@]}" -c "CREATE DATABASE ${DBNAME} OWNER ${MIGRATOR_ROLE};" || fail "create db"
psql "$SUPERUSER_DB_URL" -v ON_ERROR_STOP=1 -c \
  "GRANT ALL ON SCHEMA public TO ${APP_ROLE};
   ALTER DEFAULT PRIVILEGES FOR ROLE ${MIGRATOR_ROLE} IN SCHEMA public GRANT ALL ON TABLES TO ${APP_ROLE};
   ALTER DEFAULT PRIVILEGES FOR ROLE ${MIGRATOR_ROLE} IN SCHEMA public GRANT ALL ON SEQUENCES TO ${APP_ROLE};
   ALTER DEFAULT PRIVILEGES FOR ROLE ${MIGRATOR_ROLE} IN SCHEMA public GRANT EXECUTE ON FUNCTIONS TO ${APP_ROLE};" \
  >/dev/null || fail "grant defaults"

# Export the DB/Redis/port env. These are EXPORTED so they take precedence over
# any value in the secrets env-file (Node's --env-file does not override vars
# already present in process.env). The env-file supplies only the app secrets
# (JWT keys, VAPID, OAuth, etc.) that loadEnv() requires — never DB URLs to prod.
export DATABASE_URL_MIGRATIONS="$MIGRATOR_URL"
export DATABASE_URL_SESSION="$MIGRATOR_URL"
export DATABASE_URL_OPERATIONAL="$APP_URL"
export REDIS_URL NODE_ENV=development PORT="$API_PORT"
export APP_BASE_URL="http://127.0.0.1:${API_PORT}"
export DEV_AUTH_SECRET="${DEV_AUTH_SECRET:-local-e2e-secret}"

# Non-prod secrets file for loadEnv() (JWT keys etc.). CI generates this from
# repo secrets; locally it is .env.test. DB URLs in it are IGNORED (overridden
# by the exports above). Must not be the prod .env.
SECRETS_ENV_FILE="${SECRETS_ENV_FILE:-.env.test}"
[ -f "$SECRETS_ENV_FILE" ] || fail "secrets env-file not found: ${SECRETS_ENV_FILE}"
case "$(cat "$SECRETS_ENV_FILE")" in
  *supabase.com*) echo "WARN: ${SECRETS_ENV_FILE} references supabase.com — DB URLs will be overridden by exports, continuing" ;;
esac

echo "── STEP 1: migrate:up (from scratch) ──"
npx node-pg-migrate up -d DATABASE_URL_MIGRATIONS -j ts -m packages/db/migrations \
  --tsx --tsconfig tsconfig.migrations.json > /tmp/fresh-migrate.log 2>&1 \
  || { tail -25 /tmp/fresh-migrate.log; fail "migrate:up"; }
APPLIED=$(grep -c '### MIGRATION' /tmp/fresh-migrate.log || true)
echo "   migrate:up OK — ${APPLIED} migrations applied"

echo "── STEP 2: seed ──"
npx tsx --env-file="$SECRETS_ENV_FILE" packages/db/scripts/seed.ts > /tmp/fresh-seed.log 2>&1 \
  || { tail -25 /tmp/fresh-seed.log; fail "seed"; }
grep -q '✅ Seed completed' /tmp/fresh-seed.log || fail "seed (no success marker)"
echo "   seed OK"

echo "── STEP 3+4: boot API & curl /health ──"
( cd apps/api && npx tsx --env-file="$ROOT/$SECRETS_ENV_FILE" src/server.ts > /tmp/fresh-api.log 2>&1 & echo $! > /tmp/fresh-api.pid )
API_PID="$(cat /tmp/fresh-api.pid)"
trap 'kill "$API_PID" 2>/dev/null || true' EXIT

HEALTH=000
for i in $(seq 1 30); do
  HEALTH=$(curl -s -o /dev/null -w "%{http_code}" "http://127.0.0.1:${API_PORT}/health" || echo 000)
  [ "$HEALTH" = "200" ] && break
  sleep 1
done
[ "$HEALTH" = "200" ] || fail "/health (got HTTP ${HEALTH})"
# pg-boss must NOT have failed (this is the P0-PGBOSS regression guard).
if grep -qiE "pg-boss is not installed|permission denied for schema pgboss" /tmp/fresh-api.log; then
  fail "pg-boss bootstrap regression (not installed / permission denied)"
fi
echo "   /health 200 OK, pg-boss healthy"

echo "── STEP 5: published storefront serves a menu with products ──"
MENU=$(curl -s "http://127.0.0.1:${API_PORT}/public/locations/demo/menu")
PRODUCTS=$(echo "$MENU" | node -e "let s='';process.stdin.on('data',d=>s+=d).on('end',()=>{try{const m=JSON.parse(s);process.stdout.write(String((m.categories||[]).reduce((n,c)=>n+(c.products||[]).length,0)))}catch{process.stdout.write('0')}})")
[ "${PRODUCTS:-0}" -ge 1 ] || fail "/public/locations/demo/menu returned no products (status reconcile regression)"
echo "   published menu serves ${PRODUCTS} products OK"

echo ""
echo "✅ FRESH-PROVISION GREEN: migrate(${APPLIED}) → seed → boot → /health 200 → menu(${PRODUCTS} products)"
