#!/usr/bin/env node
/**
 * deliveryos-rls-tenant-isolation checker
 * Scans migration files for RLS & tenant-isolation contract violations.
 * Exit code != 0 when violations found.
 *
 * Checks:
 * 1. New CREATE TABLE without ALTER TABLE ... FORCE ROW LEVEL SECURITY or whitelist comment
 * 2. SQL string concatenation (unescaped user input)
 * 3. New table not in whitelist but missing RLS
 */
import { readFileSync } from 'fs';
import { resolve, join } from 'path';
import { glob } from 'node:fs/promises';

const REPO_ROOT = resolve(new URL('.', import.meta.url).pathname, '../../..', '..', '..');
const TARGET_PATH = process.argv[2] ? resolve(process.argv[2]) : null;

const WHITELIST = [
  'exchange_rates',
  'analytics_events',    // non-tenant: anonymous telemetry, see migration comment
  'analytics_abuse_log', // non-tenant: abuse detection, see migration comment
  'analytics_cwv',       // non-tenant: CWV metrics, see migration comment
  'users',
  'ops_worker_heartbeat',
  'auth_refresh_tokens',
  'pgboss', // pg-boss internal tables
];

function shouldExclude(filePath) {
  const exclude = ['node_modules', 'dist', '.git', 'graphify-out'];
  return exclude.some(e => filePath.includes(e));
}

/**
 * Extract all table names referenced in a migration file
 */
function extractCreateTables(content) {
  const tables = [];
  const regex = /CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?(?:public\.)?(\w+)/gi;
  let match;
  while ((match = regex.exec(content)) !== null) {
    tables.push(match[1]);
  }
  return tables;
}

/**
 * Extract all ALTER TABLE ... FORCE ROW LEVEL SECURITY targets
 */
function extractForceRls(content) {
  const tables = [];
  const regex = /ALTER\s+TABLE\s+(?:public\.)?(\w+)\s+FORCE\s+ROW\s+LEVEL\s+SECURITY/gi;
  let match;
  while ((match = regex.exec(content)) !== null) {
    tables.push(match[1]);
  }
  return tables;
}

/**
 * Extract all ALTER TABLE ... NO FORCE ROW LEVEL SECURITY targets
 */
function extractNoForceRls(content) {
  const tables = [];
  const regex = /ALTER\s+TABLE\s+(?:public\.)?(\w+)\s+NO\s+FORCE\s+ROW\s+LEVEL\s+SECURITY/gi;
  let match;
  while ((match = regex.exec(content)) !== null) {
    tables.push(match[1]);
  }
  return tables;
}

/**
 * Check for SQL string concatenation (unparameterized queries)
 */
function hasSqlConcatenation(content) {
  const lines = content.split('\n');
  const violations = [];
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    // Skip comments and simple assignments
    if (line.trim().startsWith('--') || line.trim().startsWith('//')) continue;
    if (line.trim().startsWith('*')) continue;

    // Pattern: `...${variable}...` used in SQL strings
    // This catches template literals with vars inside SQL strings
    const sqlContext = line.match(/`.*(SELECT|INSERT|UPDATE|DELETE|CREATE|ALTER|DROP).*\$\{/i);
    if (sqlContext) {
      violations.push({ line: i + 1, snippet: line.trim() });
    }
  }
  return violations;
}

/**
 * Check if a line has a whitelist exemption comment
 */
function hasWhitelistComment(lines, createTableLine) {
  // Check the line itself and the 3 lines above for comments mentioning 'whitelist' or 'exempt' or 'no-rls'
  const start = Math.max(0, createTableLine - 4);
  const end = Math.min(lines.length, createTableLine);
  for (let i = start; i < end; i++) {
    if (lines[i].match(/whitelist|exempt|no.?rls|non-?tenant/i)) return true;
  }
  return false;
}

async function scan() {
  const violations = [];
  const warnings = [];

  // Determine scan scope: specific file or full repo
  const scanFiles = TARGET_PATH
    ? [TARGET_PATH]
    : [];

  if (TARGET_PATH) {
    // Specific file mode — use as-is
  } else {
    // Full scan mode — collect all migration files
    for await (const file of glob(join(REPO_ROOT, 'packages/db/migrations/**/*.ts'), { nodir: true })) {
      scanFiles.push(file);
    }
  }

  for (const file of scanFiles) {
    const relPath = TARGET_PATH ? file : file.replace(REPO_ROOT, '').replace(/\\/g, '/');
    if (shouldExclude(relPath)) continue;

    const content = readFileSync(file, 'utf-8');
    const lines = content.split('\n');

    // Check 1: New tables without RLS
    const createdTables = extractCreateTables(content);
    const forceRlsTables = extractForceRls(content);
    const noForceRlsTables = extractNoForceRls(content);

    for (const table of createdTables) {
      if (WHITELIST.includes(table)) continue;
      if (forceRlsTables.includes(table)) continue;
      // Check if it's explicitly no-force-RLS (revert migration)
      if (noForceRlsTables.includes(table)) continue;

      // Find the line where this table was created
      const createLine = lines.findIndex(l => l.includes(`CREATE TABLE`) && l.includes(table));
      if (createLine >= 0 && hasWhitelistComment(lines, createLine)) continue;

      violations.push({
        file: relPath,
        rule: 'MISSING_RLS_FORCE',
        detail: `Table "${table}" is created but missing ALTER TABLE ... FORCE ROW LEVEL SECURITY`,
      });
    }

    // Check 2: SQL string concatenation in pgm.sql calls
    const concatViolations = hasSqlConcatenation(content);
    for (const v of concatViolations) {
      violations.push({
        file: relPath,
        line: v.line,
        rule: 'SQL_CONCATENATION',
        detail: 'SQL string concatenation detected — use parameterized queries ($1, $2) instead',
        snippet: v.snippet,
      });
    }
  }

  // Check 3: Scan route files for raw SQL without parameterized queries (full scan only)
  if (!TARGET_PATH) {
  for await (const file of glob(join(REPO_ROOT, 'apps/api/src/routes/**/*.ts'), { nodir: true })) {
    const relPath = file.replace(REPO_ROOT, '').replace(/\\/g, '/');
    if (shouldExclude(relPath)) continue;

    const content = readFileSync(file, 'utf-8');
    const lines = content.split('\n');

    // Look for `db.query(...` with string concatenation or template literals inside SQL
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      // Skip comments
      if (line.trim().startsWith('//') || line.trim().startsWith('/*')) continue;
      if (line.match(/db\.query\(`.*\$\{/)) {
        violations.push({
          file: relPath,
          line: i + 1,
          rule: 'SQL_CONCATENATION',
          detail: 'db.query() with template literal containing interpolation — use parameterized $N instead',
          snippet: line.trim(),
        });
      }
    }
  }
  }

  const result = {
    passed: violations.length === 0,
    violations,
    warnings,
    summary: {
      total: violations.length,
      byRule: {},
    },
  };

  for (const v of violations) {
    result.summary.byRule[v.rule] = (result.summary.byRule[v.rule] || 0) + 1;
  }

  console.log(JSON.stringify(result, null, 2));

  if (violations.length > 0) {
    process.exit(1);
  }
}

scan().catch(err => {
  console.error(JSON.stringify({ passed: false, error: err.message }));
  process.exit(1);
});
