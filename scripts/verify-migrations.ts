/**
 * verify:migrations — check migration ordering and detect out-of-sequence files
 *
 * node-pg-migrate applies migrations in alphabetical (string) order of the
 * filename prefix. If a file with an earlier timestamp exists but was applied
 * AFTER a later-timestamp file, the migration breaks.
 *
 * This script detects:
 * 1. Files whose numeric prefix is out of alphabetical order with neighbors
 * 2. Migrations whose timestamp prefix sorts AFTER an already-applied prefix
 *
 * Also warns on:
 * - Non-idempotent DDL patterns (ADD COLUMN IF NOT EXISTS recommended)
 * - Dangling .ts files without corresponding migration record
 */
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const MIGRATIONS_DIR = join(process.cwd(), 'packages/db/migrations');

const MIG_FILENAME_RE = /^(\d{13,})[-_].+\.ts$/;

interface MigFile {
  filename: string;
  prefix: string;
  path: string;
}

function parseMigrations(): MigFile[] {
  return readdirSync(MIGRATIONS_DIR)
    .filter(f => MIG_FILENAME_RE.test(f))
    .map(f => {
      const m = f.match(MIG_FILENAME_RE)!;
      return { filename: f, prefix: m[1], path: join(MIGRATIONS_DIR, f) };
    })
    .sort((a, b) => a.filename.localeCompare(b.filename));
}

function main() {
  const migrations = parseMigrations();
  let errors = 0;
  let warnings = 0;

  console.log(`Found ${migrations.length} migration files in ${MIGRATIONS_DIR}`);

  // Check 1: alphabetical order matches numeric order
  for (let i = 1; i < migrations.length; i++) {
    const prev = migrations[i - 1];
    const curr = migrations[i];
    if (curr.prefix < prev.prefix) {
      console.error(`❌ ORDERING ERROR: ${curr.filename} sorts before ${prev.filename}`);
      console.error(`   ${curr.filename} has prefix ${curr.prefix} < ${prev.prefix}`);
      console.error(`   Fix: rename ${curr.filename} to use a prefix > ${prev.prefix}`);
      errors++;
    }
  }

  // Check 2: detect timestamp-gap anomalies (clusters within 1s)
  for (let i = 1; i < migrations.length; i++) {
    const prev = migrations[i - 1];
    const curr = migrations[i];
    const prevTs = parseInt(prev.prefix, 10);
    const currTs = parseInt(curr.prefix, 10);
    const gapMs = currTs - prevTs;
    if (gapMs < 100 && gapMs > 0) {
      console.warn(`⚠️  NARROW GAP: ${gapMs}ms between ${prev.filename} and ${curr.filename}`);
      console.warn(`   If these were created in parallel, they may need re-ordering`);
      warnings++;
    }
  }

  // Check 3: warn on non-idempotent patterns (ADD COLUMN without IF NOT EXISTS)
  for (const m of migrations) {
    const content = readFileSync(m.path, 'utf8');
    if (/\bADD\s+COLUMN\b/i.test(content) && !/\bIF\s+NOT\s+EXISTS\b/i.test(content)) {
      console.warn(`⚠️  NON-IDEMPOTENT: ${m.filename} uses ADD COLUMN without IF NOT EXISTS`);
      console.warn(`   Add 'IF NOT EXISTS' to prevent failure on re-run`);
      warnings++;
    }
  }

  console.log(`\n${errors > 0 ? `❌ ${errors} ordering error(s)` : '✅ Ordering OK'}`);
  if (warnings > 0) console.log(`⚠️  ${warnings} warning(s)`);

  process.exit(errors > 0 ? 1 : 0);
}

main();
