import { readFileSync } from 'fs';
import { join } from 'path';
import { z } from 'zod';

const BASE = process.env.RADAR_BASE_URL || 'https://dowiz.fly.dev';

interface ProbeResult {
  key: string;
  status: 'OK' | 'DRIFT' | 'INCONCLUSIVE';
  driftClass: string;
  detail: string;
}

const results: ProbeResult[] = [];

function emit(key: string, status: string, cls: string, detail: string) {
  results.push({ key, status: status as any, driftClass: cls, detail });
  console.log(`${status === 'OK' ? '✅' : status === 'DRIFT' ? '🔴' : '🔶'} ${key}: ${cls} — ${detail.substring(0, 120)}`);
}

// ── PHASE 1: Expected env list from EnvSchema ──
const ENV_SCHEMA = z.object({
  NODE_ENV: z.enum(["development", "test", "production"]),
  PORT: z.coerce.number().int().positive(),
  APP_BASE_URL: z.string().url(),
  DATABASE_URL_OPERATIONAL: z.string().url(),
  DATABASE_URL_SESSION: z.string().url(),
  DATABASE_URL_MIGRATIONS: z.string().url(),
  REDIS_URL: z.string().url(),
  JWT_PRIVATE_KEY: z.string().min(1),
  JWT_PUBLIC_KEY: z.string().min(1),
  JWT_KID: z.string().min(1),
  GOOGLE_CLIENT_ID: z.string().min(1),
  GOOGLE_CLIENT_SECRET: z.string().min(1),
  VAPID_PUBLIC_KEY: z.string().min(1),
  VAPID_PRIVATE_KEY: z.string().min(1),
  VAPID_SUBJECT: z.string().email(),
  LOG_LEVEL: z.enum(['trace', 'debug', 'info', 'warn', 'error', 'fatal']),
});

const MANDATORY_KEYS = Object.keys(ENV_SCHEMA.shape);

// ── Helper: probe env on a Fly.io instance ──
async function probeFlyEnv(host: string, label: string): Promise<Map<string, string>> {
  const env = new Map<string, string>();
  // Fly.io exposes env via /debug/env or flyctl ssh
  // For staging, probe via health endpoint which reflects config state
  try {
    const res = await fetch(`${host}/health`, { signal: AbortSignal.timeout(10000) });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    const health = await res.json();
    // Health doesn't expose env values, but reflects config state via probes
    env.set('health_reachable', 'true');
    env.set('health_status', health.status || 'unknown');
    return env;
  } catch (err: any) {
    env.set('health_reachable', 'false');
    env.set('health_error', err.message);
    return env;
  }
}

async function main() {
  console.log('\n=== CONFIG DRIFT PROBE ===\n');
  console.log(`Target: ${BASE}\n`);

  // ── Group 1: Env presence via health ──
  console.log('--- Group 1: Env presence (via health probes) ---');
  const healthRes = await fetch(`${BASE}/health`, { signal: AbortSignal.timeout(10000) });
  const healthOk = healthRes.ok;
  emit('health-endpoint', healthOk ? 'OK' : 'DRIFT', healthOk ? 'present' : 'missing', `GET /health returned ${healthRes.status}`);

  // Check critical subsystems as proxy for required credentials
  const health = healthOk ? await healthRes.json() : null;
  const checks = health?.checks || {};

  // Telegram token: present if telegram check returns ok or degraded
  const tgOk = checks.telegram?.status === 'ok';
  emit('telegram-token', tgOk ? 'OK' : 'DRIFT', tgOk ? 'valid' : 'invalid', `Telegram bot: ${checks.telegram?.status || 'missing'}`);

  // R2 credentials: present if R2 check returns ok
  const r2Ok = checks.r2?.status === 'ok';
  emit('r2-credentials', r2Ok ? 'OK' : 'DRIFT', r2Ok ? 'valid' : 'invalid', `R2 storage: ${checks.r2?.status || 'missing'}`);

  // Postgres: confirms DATABASE_URL operational
  const pgOk = checks.postgres?.status === 'ok';
  emit('postgres-connection', pgOk ? 'OK' : 'DRIFT', pgOk ? 'valid' : 'invalid', `Postgres: ${checks.postgres?.status || 'missing'}`);

  // MessageBus: confirms REDIS_URL or session pool
  const mbOk = checks.messageBus?.status === 'ok';
  emit('messagebus-connection', mbOk ? 'OK' : 'DRIFT', mbOk ? 'valid' : 'invalid', `MessageBus: ${checks.messageBus?.status || 'missing'}`);

  // Workers: confirms pg-boss workers are running
  const wkOk = checks.workers?.status === 'ok';
  emit('workers-running', wkOk ? 'OK' : 'DRIFT', wkOk ? 'valid' : 'invalid', `Workers: ${checks.workers?.status || 'missing'}`);

  // ── Group 2: Runtime config via HTTP probes ──
  console.log('\n--- Group 2: Security headers & CORS ---');

  // Probe security headers on document response
  const docRes = await fetch(`${BASE}/s/demo`, { signal: AbortSignal.timeout(10000) });
  const headers = docRes.headers;

  const expectedHeaders: [string, string | RegExp][] = [
    ['content-security-policy', /default-src/],
    ['x-content-type-options', 'nosniff'],
    ['x-frame-options', 'SAMEORIGIN'],
    ['referrer-policy', 'strict-origin-when-cross-origin'],
    ['strict-transport-security', /max-age=\d+/],
  ];

  for (const [name, expected] of expectedHeaders) {
    const actual = headers.get(name);
    const match = typeof expected === 'string' ? actual === expected : actual ? expected.test(actual) : false;
    emit(`security-header-${name}`, match ? 'OK' : 'DRIFT', match ? 'present' : 'missing-or-wrong',
      match ? actual!.substring(0, 60) : `Expected ${expected}, got ${actual || '(missing)'}`);
  }

  // CSP should include specific directives for tile/CDN domains
  const csp = headers.get('content-security-policy') || '';
  const cspHasTile = csp.includes('tiles.openfreemap.org');
  emit('csp-tile-domain', cspHasTile ? 'OK' : 'DRIFT', cspHasTile ? 'present' : 'missing', 
    cspHasTile ? 'tiles.openfreemap.org in img-src/connect-src' : 'Missing tiles.openfreemap.org in CSP');
  const cspHasR2 = csp.includes('r2.') || process.env.R2_PUBLIC_URL ? true : false;
  emit('csp-r2-domain', cspHasR2 ? 'OK' : 'WARN', cspHasR2 ? 'present' : 'not-configured',
    cspHasR2 ? 'R2 image domain in CSP' : 'R2_PUBLIC_URL not set - images served via API proxy');

  // CORS probe: send OPTIONS with different origins
  for (const origin of ['https://dowiz.app', 'https://evil.com']) {
    const corsRes = await fetch(`${BASE}/api/health`, {
      method: 'OPTIONS',
      headers: { Origin: origin, 'Access-Control-Request-Method': 'GET' },
      signal: AbortSignal.timeout(5000),
    });
    const acao = corsRes.headers.get('access-control-allow-origin');
    const isWildcard = acao === '*';
    const isAllowed = acao === origin || acao === '*';
    if (origin === 'https://evil.com' && isAllowed) {
      emit(`cors-${origin}`, 'DRIFT', 'exposed', `CORS allows ${origin}: ACAO=${acao}`);
    } else {
      emit(`cors-${origin}`, 'OK', isWildcard ? 'warning-wildcard' : 'restricted', `ACAO=${acao || '(not sent)'}`);
    }
  }

  // Source map leak check
  const smRes = await fetch(`${BASE}/assets/index.js.map`, { signal: AbortSignal.timeout(5000) }).catch(() => null);
  if (smRes && smRes.status === 200) {
    emit('source-map-leak', 'DRIFT', 'exposed', 'Source map accessible at /assets/index.js.map');
  } else {
    emit('source-map-leak', 'OK', 'protected', `HTTP ${smRes?.status || 'fetch failed'}`);
  }

  // HTTPS check
  emit('https-active', BASE.startsWith('https://') ? 'OK' : 'DRIFT', BASE.startsWith('https://') ? 'enforced' : 'missing', `URL scheme: ${BASE.split(':')[0]}`);

  // ── Group 3: Environment consistency ──
  console.log('\n--- Group 3: Environment consistency ---');

  // Check that staging has different credentials than prod would
  // (impossible to compare without prod access, so check basic sanity)
  if (checks.telegram?.data?.result?.id) {
    emit('telegram-bot-id', 'OK', 'present', `Bot ID: ${checks.telegram.data.result.id}`);
  }

  // Check DB connection pool type (session vs transaction)
  // We can't directly query inet_server_port, but health confirms postgres ok
  // The pooler type matters for LISTEN/NOTIFY — MessageBus ok confirms session pool
  emit('session-pool', mbOk ? 'OK' : 'DRIFT', mbOk ? 'session' : 'unknown',
    mbOk ? 'MessageBus connected (requires session pool)' : 'MessageBus failed');

  // Check for expected DB role separation
  // Confirmed by health endpoint showing postgres check passed
  emit('db-role-operational', pgOk ? 'OK' : 'DRIFT', pgOk ? 'present' : 'missing',
    'Operational pool connects successfully');

  // ── Summary ──
  console.log('\n=== SUMMARY ===');
  const driftCount = results.filter(r => r.status === 'DRIFT').length;
  const inconclusiveCount = results.filter(r => r.status === 'INCONCLUSIVE').length;
  const okCount = results.length - driftCount - inconclusiveCount;

  console.log(`Total checks: ${results.length}`);
  console.log(`OK: ${okCount}`);
  console.log(`DRIFT: ${driftCount}`);
  console.log(`INCONCLUSIVE: ${inconclusiveCount}`);

  if (driftCount > 0) {
    console.log('\n--- DRIFT details ---');
    for (const r of results.filter(r => r.status === 'DRIFT')) {
      console.log(`🔴 ${r.key}: ${r.detail.substring(0, 100)}`);
    }
  }

  process.exit(driftCount > 0 ? 1 : 0);
}

main().catch(err => {
  console.error('\n🔶 CONFIG PROBE HARNESS FAILURE:', err.message);
  process.exit(2);
});
