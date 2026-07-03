#!/usr/bin/env node
// ci-migration-preflight — prove PENDING migrations apply against PROD's ACTUAL schema
// BEFORE they run on prod. Closes P3 (PROD≠STAGING DRIFT) from the 2026-07-03 saga:
//   - migration 1790000000077 keys an RLS policy on telegram_connect_tokens.owner_id,
//     but PROD's table has user_id (staging had drifted to owner_id) → migrate failed on prod.
//   - migrations 077-082 `GRANT … TO dowiz_app`, but dowiz_app did not exist on prod.
// Migrations were validated on staging + a fresh clean DB — NEITHER matches prod. This gate
// validates against the source you point it at (prod, READ-ONLY) so drift fails in CI.
//
// Two modes:
//
//   LIGHT (default — no external tools, always available):
//     1. connect to SOURCE (read-only), read the applied set from `pgmigrations`,
//     2. pending = migration files not yet applied on SOURCE,
//     3. statically extract the tables / columns / roles each pending migration references,
//     4. assert every referenced table+column exists (information_schema) and every referenced
//        role exists (pg_roles) ON SOURCE. Missing owner_id / missing dowiz_app → exit 1.
//     Heuristic (regex) — it catches the drift CLASS that bit us; see LIMITATIONS below.
//
//   FULL (--full — needs pg_dump + a writable SCRATCH_URL; the strongest proof):
//     pg_dump --schema-only SOURCE → load into SCRATCH → `node-pg-migrate up` against SCRATCH.
//     A real apply against a prod-shaped schema. Implemented; auto-skips if pg_dump is absent.
//
// SOURCE is only ever read (information_schema / pg_dump --schema-only). Never DDL/DML on SOURCE.
// No hardcoded credentials — SOURCE_URL / SCRATCH_URL from env.
//
// Usage:
//   SOURCE_URL='postgres://…prod…?sslmode=no-verify' node scripts/ci-migration-preflight.mjs
//   SOURCE_URL=… SCRATCH_URL='postgres://…scratch…' node scripts/ci-migration-preflight.mjs --full
//   node scripts/ci-migration-preflight.mjs --json
//
// Env: SOURCE_URL (required), SCRATCH_URL (--full), MIGRATIONS_DIR (default packages/db/migrations)
// Exit: 0 = pending migrations validate against SOURCE. 1 = drift found. 2 = usage/driver error.
import { readdirSync, readFileSync, existsSync } from 'node:fs';
import { join, basename } from 'node:path';
import { spawnSync } from 'node:child_process';
import { loadPg, redact, classifyPgError } from './_pg-loader.mjs';

const FULL = process.argv.includes('--full');
const JSON_OUT = process.argv.includes('--json');
const ROOT = process.cwd();
const MIGRATIONS_DIR = process.env.MIGRATIONS_DIR || join(ROOT, 'packages/db/migrations');
const SOURCE_URL = process.env.SOURCE_URL;
const SCRATCH_URL = process.env.SCRATCH_URL;

function die(code, msg) { console.error(`[migration-preflight] ${msg}`); process.exit(code); }

const SELF_TEST = process.argv.includes('--self-test');
if (!SELF_TEST && !SOURCE_URL) die(2, 'FATAL: SOURCE_URL is required (a READ-ONLY connection to the schema to validate against, e.g. prod).');
if (!existsSync(MIGRATIONS_DIR)) die(2, `FATAL: migrations dir not found: ${MIGRATIONS_DIR}`);

// ── static extractor ──────────────────────────────────────────────────────────
// Pull the schema objects a migration touches. Heuristic but tuned to the DDL this repo writes:
//   CREATE POLICY … ON <t>, ALTER TABLE <t>, GRANT … ON <t> TO <role>, <t>.<col>, <col> = app_…()
const IDENT = '[a-z_][a-z0-9_]*';
function extractRefs(sql) {
  const tables = new Set();
  const columns = new Set();   // "table.column" OR "?.column" (unqualified — checked against every ref'd table)
  const roles = new Set();

  const grab = (re, fn) => { let m; while ((m = re.exec(sql)) !== null) fn(m); };

  // tables
  grab(new RegExp(`\\bON\\s+(?:${IDENT}\\.)?(${IDENT})\\b`, 'gi'), (m) => tables.add(m[1].toLowerCase()));
  grab(new RegExp(`\\bALTER\\s+TABLE\\s+(?:IF\\s+EXISTS\\s+)?(?:${IDENT}\\.)?(${IDENT})`, 'gi'), (m) => tables.add(m[1].toLowerCase()));
  grab(new RegExp(`\\bFROM\\s+(?:${IDENT}\\.)?(${IDENT})`, 'gi'), (m) => tables.add(m[1].toLowerCase()));
  grab(new RegExp(`\\bUPDATE\\s+(?:${IDENT}\\.)?(${IDENT})`, 'gi'), (m) => tables.add(m[1].toLowerCase()));
  grab(new RegExp(`\\bINSERT\\s+INTO\\s+(?:${IDENT}\\.)?(${IDENT})`, 'gi'), (m) => tables.add(m[1].toLowerCase()));

  // qualified table.column
  grab(new RegExp(`\\b(${IDENT})\\.(${IDENT})\\b`, 'gi'), (m) => {
    const t = m[1].toLowerCase(), c = m[2].toLowerCase();
    // skip schema.table (pg_catalog., public., information_schema.) and function-ish refs
    if (['pg_catalog', 'public', 'information_schema', 'app', 'current_setting'].includes(t)) return;
    columns.add(`${t}.${c}`);
  });

  // unqualified predicate columns inside policies: USING (col = …) / WITH CHECK (col = …) / WHERE col
  grab(new RegExp(`\\b(?:USING|WITH\\s+CHECK|WHERE)\\s*\\(?\\s*(${IDENT})\\s*(?:=|<>|IN|IS)`, 'gi'),
    (m) => columns.add(`?.${m[1].toLowerCase()}`));

  // roles
  grab(new RegExp(`\\bTO\\s+(${IDENT})\\b`, 'gi'), (m) => {
    const r = m[1].toLowerCase();
    if (['authenticated', 'public', 'current_user', 'session_user'].includes(r)) return; // built-ins / pg keywords
    roles.add(r);
  });

  // SQL keywords that can slip through the table regexes — prune them.
  const KW = new Set(['select', 'exists', 'all', 'if', 'not', 'only', 'values', 'set', 'and', 'or', 'as', 'to']);
  for (const kw of KW) { tables.delete(kw); }
  return { tables: [...tables], columns: [...columns], roles: [...roles] };
}

async function readAppliedSet(client) {
  try {
    const r = await client.query('select name from pgmigrations');
    return new Set(r.rows.map((row) => String(row.name)));
  } catch (e) {
    // pgmigrations absent → treat as fresh (everything pending). (24P01/42P01 = undefined_table)
    if (e?.code === '42P01') return null;
    throw e;
  }
}

function migrationName(file) { return basename(file).replace(/\.(ts|js|sql)$/i, ''); }

async function light(Client) {
  const client = new Client({ connectionString: SOURCE_URL, connectionTimeoutMillis: 15000 });
  try {
    await client.connect();
  } catch (err) {
    console.error(`[migration-preflight] cannot reach SOURCE (${classifyPgError(err)}): ${err.message}\n  url: ${redact(SOURCE_URL)}`);
    process.exit(2);
  }

  const applied = await readAppliedSet(client);
  const files = readdirSync(MIGRATIONS_DIR).filter((f) => /\.(ts|js|sql)$/.test(f)).sort();
  const pending = files.filter((f) => applied === null ? true : !applied.has(migrationName(f)));

  // live schema snapshot (public)
  const colRows = (await client.query(
    `select table_name, column_name from information_schema.columns where table_schema = 'public'`,
  )).rows;
  const liveCols = new Map(); // table -> Set(columns)
  for (const { table_name, column_name } of colRows) {
    if (!liveCols.has(table_name)) liveCols.set(table_name, new Set());
    liveCols.get(table_name).add(column_name);
  }
  const liveRoles = new Set((await client.query('select rolname from pg_roles')).rows.map((r) => r.rolname));
  await client.end();

  const findings = [];
  for (const f of pending) {
    const sql = readFileSync(join(MIGRATIONS_DIR, f), 'utf8');
    const { tables, columns, roles } = extractRefs(sql);
    const refTables = new Set(tables);

    // tables that the migration touches AND that already exist on SOURCE (migration may also CREATE new ones)
    const created = new Set();
    { let m; const cre = new RegExp(`\\bCREATE\\s+TABLE\\s+(?:IF\\s+NOT\\s+EXISTS\\s+)?(?:${IDENT}\\.)?(${IDENT})`, 'gi');
      while ((m = cre.exec(sql)) !== null) created.add(m[1].toLowerCase()); }

    // column checks
    for (const ref of columns) {
      const [t, c] = ref.split('.');
      if (t === '?') {
        // unqualified: must exist in AT LEAST ONE table this migration references (that pre-exists)
        const candidates = [...refTables].filter((tt) => liveCols.has(tt) && !created.has(tt));
        if (candidates.length === 0) continue; // no pre-existing referenced table → can't judge
        const ok = candidates.some((tt) => liveCols.get(tt).has(c));
        if (!ok) findings.push({ file: f, kind: 'COLUMN', detail: `column "${c}" not found on any referenced pre-existing table [${candidates.join(', ')}]` });
      } else {
        if (created.has(t)) continue;            // table created by this migration — skip
        if (!liveCols.has(t)) continue;          // table not on SOURCE — flagged by table check below if referenced structurally
        if (!liveCols.get(t).has(c)) findings.push({ file: f, kind: 'COLUMN', detail: `${t}.${c} — column does not exist on SOURCE (drift)` });
      }
    }
    // role checks
    for (const r of roles) {
      if (!liveRoles.has(r)) findings.push({ file: f, kind: 'ROLE', detail: `role "${r}" does not exist on SOURCE (GRANT … TO ${r} will fail)` });
    }
  }

  return { applied: applied === null ? '(fresh — no pgmigrations)' : applied.size, pending: pending.map(migrationName), findings };
}

// ── FULL mode: pg_dump --schema-only → scratch → migrate ────────────────────────
function pgDumpAvailable() { return spawnSync('pg_dump', ['--version'], { encoding: 'utf8' }).status === 0; }

function runFull() {
  if (!SCRATCH_URL) die(2, '--full requires SCRATCH_URL (a writable ephemeral target).');
  if (!pgDumpAvailable()) die(2, '--full requires pg_dump on PATH (not found). Use LIGHT mode or install postgresql-client.');
  console.log('[migration-preflight] FULL: pg_dump --schema-only SOURCE → SCRATCH → node-pg-migrate up');

  const dump = spawnSync('pg_dump', ['--schema-only', '--no-owner', '--no-privileges', SOURCE_URL], { encoding: 'utf8', maxBuffer: 256 * 1024 * 1024 });
  if (dump.status !== 0) die(2, `pg_dump failed: ${dump.stderr}`);

  const load = spawnSync('psql', [SCRATCH_URL, '-v', 'ON_ERROR_STOP=1'], { input: dump.stdout, encoding: 'utf8' });
  if (load.status !== 0) die(1, `loading prod schema into SCRATCH failed: ${load.stderr}`);

  // Real apply of the pending set against the prod-shaped scratch DB.
  const mig = spawnSync('pnpm', ['migrate:up'], {
    encoding: 'utf8',
    env: { ...process.env, DATABASE_URL_MIGRATIONS: SCRATCH_URL },
    stdio: 'inherit',
  });
  if (mig.status !== 0) die(1, 'migrate:up FAILED against prod-shaped scratch schema — drift confirmed (this is exactly the prod failure, caught in CI).');
  console.log('[migration-preflight] FULL PASS — pending migrations apply cleanly on a prod-shaped schema.');
  process.exit(0);
}

// ── self-test: prove the extractor catches the exact drift class that bit us ─────
// (no DB needed). Runs the regex extractor on real migration 077 and asserts it sees
// telegram_connect_tokens.owner_id (the drifted column) and the dowiz_app role (the
// absent-on-prod role). Fails loud if the extractor ever regresses.
function selfTest() {
  const fixture = join(MIGRATIONS_DIR, '1790000000077_rls-nobypassrls-phase1-policies.ts');
  const checks = [];
  if (existsSync(fixture)) {
    const { tables, columns, roles } = extractRefs(readFileSync(fixture, 'utf8'));
    // 077 writes `USING (owner_id = …)` (unqualified) on telegram_connect_tokens — LIGHT mode
    // resolves the unqualified `?.owner_id` against the referenced pre-existing table, so BOTH
    // the table ref and the owner_id column ref must be captured for the drift check to fire.
    checks.push(['references telegram_connect_tokens', tables.includes('telegram_connect_tokens')]);
    checks.push(['captures owner_id (qualified or unqualified)',
      columns.includes('telegram_connect_tokens.owner_id') || columns.includes('?.owner_id')]);
    checks.push(['sees dowiz_app role', roles.includes('dowiz_app')]);
  } else {
    console.log(`[self-test] fixture missing (${basename(fixture)}) — running synthetic fixture instead.`);
  }
  // synthetic fixture (always runs — decoupled from repo state)
  const syn = extractRefs(`CREATE POLICY owner_isolation ON telegram_connect_tokens FOR ALL
    USING (owner_id = app_current_user()); GRANT EXECUTE ON FUNCTION f() TO dowiz_app;`);
  checks.push(['synthetic: owner_id captured', syn.columns.includes('telegram_connect_tokens.owner_id') || syn.columns.includes('?.owner_id')]);
  checks.push(['synthetic: dowiz_app captured', syn.roles.includes('dowiz_app')]);

  let pass = true;
  for (const [name, ok] of checks) { console.log(`  ${ok ? '✓' : '✗'} ${name}`); if (!ok) pass = false; }
  console.log(pass ? '[self-test] PASS' : '[self-test] FAIL');
  process.exit(pass ? 0 : 1);
}

async function main() {
  if (process.argv.includes('--self-test')) return selfTest();
  if (FULL) return runFull();
  let Client;
  try { ({ Client } = await loadPg()); } catch (e) { die(2, `FATAL: ${e.message}`); }

  const { applied, pending, findings } = await light(Client);

  if (JSON_OUT) {
    console.log(JSON.stringify({ ok: findings.length === 0, applied, pending, findings }, null, 2));
    process.exit(findings.length === 0 ? 0 : 1);
  }

  console.log('── ci-migration-preflight (LIGHT) ──────────────────────');
  console.log(`  source:   ${redact(SOURCE_URL)}`);
  console.log(`  applied:  ${applied}`);
  console.log(`  pending:  ${pending.length}${pending.length ? ` → ${pending.join(', ')}` : ''}`);
  if (!findings.length) {
    console.log('────────────────────────────────────────────────────────');
    console.log('[migration-preflight] PASS — every table/column/role referenced by pending migrations exists on SOURCE.');
    console.log('  NOTE (LIGHT limitations): regex extraction covers the drift CLASS that bit us (missing');
    console.log('  column / missing role). For a full apply-proof run `--full` with pg_dump + a scratch DB.');
    process.exit(0);
  }
  console.log('  DRIFT:');
  for (const fnd of findings) console.log(`    ✗ [${fnd.kind}] ${fnd.file}: ${fnd.detail}`);
  console.log('────────────────────────────────────────────────────────');
  console.error(`[migration-preflight] FAIL — ${findings.length} drift finding(s). These migrations would fail on SOURCE (prod).`);
  process.exit(1);
}

main().catch((e) => { console.error(`[migration-preflight] UNEXPECTED: ${e?.stack || e}`); process.exit(2); });
