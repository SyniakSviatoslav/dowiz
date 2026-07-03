#!/usr/bin/env node
// ci-connection-preflight — assert every DATABASE_URL_* env var actually CONNECTS
// (with working SSL) before any migrate / deploy step runs. Closes P2/P4/P5 from the
// 2026-07-03 prod-deploy saga:
//   P2 SECRET-STORE FRAGMENTATION — a URL that lives in the wrong store (Fly vs GitHub
//      Actions vs Supabase) fails HERE, in the job that will use it, not on prod.
//   P4 SSL/CONNECTION CONFIG — sslmode omitted (ESSLREQUIRED) or sslmode=require
//      (node-pg → verify-full → self-signed pooler cert rejected) fails HERE, not serially on prod.
//   P5 INFRA-CHANGE OUTAGE — run this with the OPERATIONAL/SESSION runtime URLs BEFORE
//      flipping Supabase "block non-SSL"; if a runtime pool can't connect under SSL it
//      fails the gate instead of taking prod down on boot.
//
// It connects with the *repo's* pg driver (SSL handled purely via the connection-string
// sslmode — same parse the app + node-pg-migrate use), runs `select 1`, and on failure
// names WHICH url and WHETHER it was SSL vs AUTH vs HOST.
//
// No hardcoded credentials — everything from env. Never runs DDL/DML. Read-only `select 1`.
//
// Usage:
//   DATABASE_URL_MIGRATIONS=... DATABASE_URL_OPERATIONAL=... DATABASE_URL_SESSION=... \
//     node scripts/ci-connection-preflight.mjs
//   node scripts/ci-connection-preflight.mjs --require-all   # absent URL = FAIL (default: skip+warn)
//   node scripts/ci-connection-preflight.mjs --json          # machine output
//
// Exit: 0 = every present (or required) URL connected. 1 = at least one failed. 2 = usage/driver error.
import { loadPg, redact, classifyPgError } from './_pg-loader.mjs';

const REQUIRE_ALL = process.argv.includes('--require-all');
const JSON_OUT = process.argv.includes('--json');

// The three pools the system uses (migrate:up reads DATABASE_URL_MIGRATIONS; runtime
// uses OPERATIONAL @6543 pooler + SESSION). Generic DATABASE_URL kept as a fallback name.
const TARGETS = [
  ['DATABASE_URL_MIGRATIONS', 'migrator role — node-pg-migrate up'],
  ['DATABASE_URL_OPERATIONAL', 'runtime app pool (transaction pooler)'],
  ['DATABASE_URL_SESSION', 'runtime session pool'],
  ['DATABASE_URL', 'generic fallback'],
];

const CONNECT_TIMEOUT_MS = Number(process.env.PREFLIGHT_TIMEOUT_MS || 10000);

async function probe(Client, url) {
  const client = new Client({
    connectionString: url,          // pg parses sslmode from the string — matches prod exactly
    connectionTimeoutMillis: CONNECT_TIMEOUT_MS,
    statement_timeout: CONNECT_TIMEOUT_MS,
  });
  try {
    await client.connect();
    const r = await client.query('select 1 as ok');
    if (r?.rows?.[0]?.ok !== 1) throw new Error('select 1 returned unexpected result');
    return { ok: true };
  } catch (err) {
    return { ok: false, kind: classifyPgError(err), detail: err?.message || String(err), code: err?.code || '' };
  } finally {
    try { await client.end(); } catch { /* ignore */ }
  }
}

async function main() {
  let Client;
  try { ({ Client } = await loadPg()); }
  catch (e) { console.error(`[connection-preflight] FATAL: ${e.message}`); process.exit(2); }

  const results = [];
  for (const [name, purpose] of TARGETS) {
    const url = process.env[name];
    if (!url) {
      // DATABASE_URL is only a fallback — never itself required.
      const required = REQUIRE_ALL && name !== 'DATABASE_URL';
      results.push({ name, purpose, status: required ? 'MISSING' : 'skipped', url: '(unset)' });
      continue;
    }
    const r = await probe(Client, url);
    results.push({
      name, purpose, url: redact(url),
      status: r.ok ? 'OK' : 'FAIL',
      kind: r.ok ? undefined : r.kind,
      detail: r.ok ? undefined : r.detail,
      code: r.ok ? undefined : r.code,
    });
  }

  const failed = results.filter((r) => r.status === 'FAIL' || r.status === 'MISSING');

  if (JSON_OUT) {
    console.log(JSON.stringify({ ok: failed.length === 0, results }, null, 2));
    process.exit(failed.length === 0 ? 0 : 1);
  }

  console.log('── ci-connection-preflight ─────────────────────────────');
  for (const r of results) {
    const mark = r.status === 'OK' ? '✓' : r.status === 'skipped' ? '·' : '✗';
    let line = `  ${mark} ${r.name.padEnd(24)} ${r.status.padEnd(8)} ${r.purpose}`;
    if (r.status === 'FAIL') line += `\n      → ${r.kind} failure${r.code ? ` [${r.code}]` : ''}: ${r.detail}\n      → url: ${r.url}`;
    if (r.status === 'MISSING') line += `\n      → required by --require-all but env var is unset`;
    console.log(line);
  }
  console.log('────────────────────────────────────────────────────────');

  if (failed.length) {
    const sslFails = failed.filter((r) => r.kind === 'SSL');
    if (sslFails.length) {
      console.error(`\n[connection-preflight] ${sslFails.length} SSL failure(s). Hint: node-pg treats ` +
        `\`sslmode=require\` as verify-full (rejects the self-signed Supabase pooler cert). Use ` +
        `\`?sslmode=no-verify\` on pooler URLs. A missing sslmode raises ESSLREQUIRED once the DB blocks non-SSL.`);
    }
    console.error(`\n[connection-preflight] FAIL — ${failed.length}/${results.filter(r=>r.status!=='skipped').length} target(s) unreachable. Fix the secret in the store the failing JOB reads (P2), not just Fly.`);
    process.exit(1);
  }
  console.log(`\n[connection-preflight] PASS — all present DB URLs connect with working SSL.`);
  process.exit(0);
}

main().catch((e) => { console.error(`[connection-preflight] UNEXPECTED: ${e?.stack || e}`); process.exit(2); });
