#!/usr/bin/env node
// Guardrail — every SECURITY DEFINER Postgres function created in a migration MUST pin its
// `search_path` in the same CREATE … <body-start> span (pg-privilege-hardening, ITEM 1 / §9).
//
// WHY: a SECURITY DEFINER function with a mutable search_path is an RLS-bypass / privilege-escalation
// vector — a caller who can create an object in an earlier-resolved schema (notably an implicit
// `pg_temp`) can shadow a relation/function the definer touches. The runtime fix (MIG-ITEM1) re-pins
// the historical offenders to `pg_catalog, public, pg_temp`; THIS gate stops a NEW migration from
// re-introducing an unpinned definer. Deterministic, text-level, no DB needed → runs in CI/pre-commit.
//
// MECHANISM (text-level, not AST — catches multi-line template literals + `const FN = \`…\`` forms):
//   1. Read every packages/db/migrations/**/*.ts, concatenate each file's text.
//   2. Extract each `CREATE (OR REPLACE)? FUNCTION <name>(…)` header up to its body delimiter
//      (`AS $tag$` / `AS $$`). That header span is where SECURITY DEFINER and SET search_path live.
//   3. For every header that contains `SECURITY DEFINER`, require `SET search_path` in the same span.
//   4. History is immutable → a frozen ALLOWLIST (scripts/definer-baseline.json) exempts the
//      pre-existing offender occurrences (their historical CREATEs + the *_PRIOR rollback bodies).
//      Any offender NOT in the baseline → exit(1). New migrations cannot launder a new offender
//      through the baseline: the baseline is keyed by {file, fn} and a new file is never present.
//
// Run: node scripts/guardrail-definer-search-path.mjs        (CI / pre-commit)
//      node scripts/guardrail-definer-search-path.mjs --report  (list current offenders, seed baseline)
import { readFileSync, readdirSync, existsSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = process.cwd();
// MIG_DIR / BASELINE_PATH are overridable for a self-contained red→green test (the real migrations/
// dir is protect-paths-blocked, so the proof fixture lives outside it). Defaults are the live paths.
const MIG_DIR = process.env.DEFINER_MIG_DIR || join(ROOT, 'packages', 'db', 'migrations');
const BASELINE_PATH = process.env.DEFINER_BASELINE || join(ROOT, 'scripts', 'definer-baseline.json');

const REPORT = process.argv.includes('--report');

/** Walk a dir for *.ts migration files (flat — migrations/ is flat). */
function migrationFiles() {
  if (!existsSync(MIG_DIR)) return [];
  return readdirSync(MIG_DIR)
    .filter((f) => f.endsWith('.ts'))
    .sort()
    .map((f) => join(MIG_DIR, f));
}

/**
 * Extract every CREATE FUNCTION header span and its definer/search_path flags from one file's text.
 * Returns [{ fn, securityDefiner, hasSearchPath }].
 */
function scanFunctions(text) {
  const out = [];
  // Match the start of each function definition. `[\s\S]` so the (…) arg list can span lines.
  const headRe = /CREATE\s+(?:OR\s+REPLACE\s+)?FUNCTION\s+([a-zA-Z0-9_."]+)\s*\(/gi;
  let m;
  while ((m = headRe.exec(text)) !== null) {
    const fn = m[1].replace(/"/g, '');
    const start = m.index;
    // Body delimiter: `AS $tag$` / `AS $$` (plpgsql/sql) or `AS '…'`. Take the first after the header.
    const rest = text.slice(start);
    const bodyM = rest.match(/\bAS\s+(\$[a-zA-Z0-9_]*\$|')/i);
    const span = bodyM ? rest.slice(0, bodyM.index) : rest.slice(0, 2000); // header-only span
    const securityDefiner = /SECURITY\s+DEFINER/i.test(span);
    const hasSearchPath = /SET\s+search_path/i.test(span);
    out.push({ fn, securityDefiner, hasSearchPath });
  }
  return out;
}

/** Current offenders: DEFINER functions whose header span lacks SET search_path. Keyed {file, fn}. */
function currentOffenders() {
  const offenders = [];
  for (const path of migrationFiles()) {
    const file = path.slice(ROOT.length + 1).replace(/\\/g, '/');
    const text = readFileSync(path, 'utf8');
    for (const f of scanFunctions(text)) {
      if (f.securityDefiner && !f.hasSearchPath) offenders.push({ file, fn: f.fn });
    }
  }
  return offenders;
}

const key = (o) => `${o.file}::${o.fn}`;

function loadBaseline() {
  if (!existsSync(BASELINE_PATH)) return new Set();
  const json = JSON.parse(readFileSync(BASELINE_PATH, 'utf8'));
  return new Set((json.allow || []).map((o) => key(o)));
}

// ─────────────────────────────────────────────────────────────────────────────
const offenders = currentOffenders();

if (REPORT) {
  // Emit a baseline-shaped JSON of every current offender (one row per occurrence). Manual review,
  // then freeze into scripts/definer-baseline.json. Dedup by key (a function re-CREATEd in the same
  // file across up/down is one logical offender per file).
  const seen = new Set();
  const allow = [];
  for (const o of offenders) {
    if (seen.has(key(o))) continue;
    seen.add(key(o));
    allow.push({ file: o.file, fn: o.fn });
  }
  process.stdout.write(JSON.stringify({ allow }, null, 2) + '\n');
  process.exit(0);
}

const baseline = loadBaseline();
const unlisted = offenders.filter((o) => !baseline.has(key(o)));

if (unlisted.length > 0) {
  console.error(
    '❌ SECURITY DEFINER function without a pinned `search_path` (not in scripts/definer-baseline.json):',
  );
  for (const o of unlisted) console.error(`   ${o.file} :: ${o.fn}()`);
  console.error(
    '\nFix: add `SET search_path = pg_catalog, public, pg_temp` to the CREATE FUNCTION header.\n' +
      'A mutable search_path on a DEFINER fn is an RLS-bypass vector. The baseline is frozen for\n' +
      'pre-existing history only — new migrations must pin the path, never extend the baseline.',
  );
  process.exit(1);
}

console.log(
  `✅ definer-search-path: ${offenders.length} historical offender occurrence(s) baseline-exempt, ` +
    `0 unlisted. No new unpinned SECURITY DEFINER functions.`,
);
process.exit(0);
