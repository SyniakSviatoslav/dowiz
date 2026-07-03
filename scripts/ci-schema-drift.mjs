#!/usr/bin/env node
// ci-schema-drift — compare the public-schema column sets of TWO databases and fail if they
// diverge. Built for the P3 root cause of the 2026-07-03 saga: staging had drifted
// (telegram_connect_tokens.owner_id) while prod still had user_id, so "validated on staging"
// was a false proof. This makes drift between the DB you TEST against and the DB you DEPLOY to
// a deterministic, visible CI failure instead of a prod discovery.
//
// By default it fails ONLY on drift affecting tables that pending migrations reference (so cosmetic
// drift on unrelated tables doesn't block a release); pass --all-tables to fail on any drift.
//
// Both connections are READ-ONLY (information_schema only). No hardcoded credentials.
//
// Usage:
//   LEFT_URL='…staging…' RIGHT_URL='…prod…' node scripts/ci-schema-drift.mjs
//   LEFT_URL=… RIGHT_URL=… node scripts/ci-schema-drift.mjs --all-tables
//   node scripts/ci-schema-drift.mjs --json
//
// Env: LEFT_URL, RIGHT_URL (both required), MIGRATIONS_DIR (default packages/db/migrations)
// Exit: 0 = no drift (on scoped tables). 1 = drift. 2 = usage/driver error.
import { readdirSync, readFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { loadPg, redact, classifyPgError } from './_pg-loader.mjs';

const JSON_OUT = process.argv.includes('--json');
const ALL_TABLES = process.argv.includes('--all-tables');
const ROOT = process.cwd();
const MIGRATIONS_DIR = process.env.MIGRATIONS_DIR || join(ROOT, 'packages/db/migrations');
const LEFT_URL = process.env.LEFT_URL;
const RIGHT_URL = process.env.RIGHT_URL;

function die(code, msg) { console.error(`[schema-drift] ${msg}`); process.exit(code); }
if (!LEFT_URL || !RIGHT_URL) die(2, 'FATAL: LEFT_URL and RIGHT_URL are both required (two READ-ONLY connections to compare).');

async function snapshot(Client, url, label) {
  const client = new Client({ connectionString: url, connectionTimeoutMillis: 15000 });
  try { await client.connect(); }
  catch (err) { die(2, `cannot reach ${label} (${classifyPgError(err)}): ${err.message}\n  url: ${redact(url)}`); }
  const rows = (await client.query(
    `select table_name, column_name, data_type from information_schema.columns where table_schema='public'`,
  )).rows;
  await client.end();
  const map = new Map(); // table -> Map(column -> data_type)
  for (const { table_name, column_name, data_type } of rows) {
    if (!map.has(table_name)) map.set(table_name, new Map());
    map.get(table_name).set(column_name, data_type);
  }
  return map;
}

// tables referenced by ANY migration file (scoping set)
function migrationReferencedTables() {
  if (!existsSync(MIGRATIONS_DIR)) return null;
  const IDENT = '[a-z_][a-z0-9_]*';
  const anchors = [
    new RegExp(`\\bON\\s+(?:${IDENT}\\.)?(${IDENT})`, 'gi'),
    new RegExp(`\\bALTER\\s+TABLE\\s+(?:IF\\s+EXISTS\\s+)?(?:${IDENT}\\.)?(${IDENT})`, 'gi'),
    new RegExp(`\\b(?:CREATE\\s+TABLE|FROM|UPDATE|INSERT\\s+INTO)\\s+(?:IF\\s+NOT\\s+EXISTS\\s+)?(?:${IDENT}\\.)?(${IDENT})`, 'gi'),
  ];
  const tables = new Set();
  for (const f of readdirSync(MIGRATIONS_DIR).filter((x) => /\.(ts|js|sql)$/.test(x))) {
    const sql = readFileSync(join(MIGRATIONS_DIR, f), 'utf8');
    for (const re of anchors) { let m; while ((m = re.exec(sql)) !== null) tables.add(m[1].toLowerCase()); }
  }
  return tables;
}

async function main() {
  let Client;
  try { ({ Client } = await loadPg()); } catch (e) { die(2, `FATAL: ${e.message}`); }

  const [left, right] = await Promise.all([
    snapshot(Client, LEFT_URL, 'LEFT'),
    snapshot(Client, RIGHT_URL, 'RIGHT'),
  ]);

  const scope = ALL_TABLES ? null : migrationReferencedTables();
  const inScope = (t) => scope === null ? true : scope.has(t.toLowerCase());

  const drift = []; // { table, column, kind }
  const allTables = new Set([...left.keys(), ...right.keys()]);
  for (const t of allTables) {
    if (!inScope(t)) continue;
    const l = left.get(t), r = right.get(t);
    if (!l) { drift.push({ table: t, column: '*', kind: 'TABLE_MISSING_ON_LEFT' }); continue; }
    if (!r) { drift.push({ table: t, column: '*', kind: 'TABLE_MISSING_ON_RIGHT' }); continue; }
    const cols = new Set([...l.keys(), ...r.keys()]);
    for (const c of cols) {
      if (!l.has(c)) drift.push({ table: t, column: c, kind: 'COLUMN_MISSING_ON_LEFT' });
      else if (!r.has(c)) drift.push({ table: t, column: c, kind: 'COLUMN_MISSING_ON_RIGHT' });
      else if (l.get(c) !== r.get(c)) drift.push({ table: t, column: c, kind: `TYPE_MISMATCH (${l.get(c)} vs ${r.get(c)})` });
    }
  }

  if (JSON_OUT) {
    console.log(JSON.stringify({ ok: drift.length === 0, scoped: !ALL_TABLES, drift }, null, 2));
    process.exit(drift.length === 0 ? 0 : 1);
  }

  console.log('── ci-schema-drift ─────────────────────────────────────');
  console.log(`  LEFT:  ${redact(LEFT_URL)}`);
  console.log(`  RIGHT: ${redact(RIGHT_URL)}`);
  console.log(`  scope: ${ALL_TABLES ? 'ALL tables' : 'tables referenced by migrations'}`);
  if (!drift.length) {
    console.log('────────────────────────────────────────────────────────');
    console.log('[schema-drift] PASS — no drift on scoped tables. "Tested on LEFT" is a valid proof for RIGHT.');
    process.exit(0);
  }
  for (const d of drift) console.log(`    ✗ ${d.table}.${d.column} — ${d.kind}`);
  console.log('────────────────────────────────────────────────────────');
  console.error(`[schema-drift] FAIL — ${drift.length} drift point(s). Validating on LEFT does NOT prove RIGHT.`);
  process.exit(1);
}

main().catch((e) => { console.error(`[schema-drift] UNEXPECTED: ${e?.stack || e}`); process.exit(2); });
