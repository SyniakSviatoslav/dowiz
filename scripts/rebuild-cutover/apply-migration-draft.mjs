#!/usr/bin/env node
// Apply the SQL body of an operator-gated node-pg-migrate DRAFT to a database, WITHOUT
// placing the file into packages/db/migrations/ (which is red-line hard-gated). Extracts the
// `pgm.sql(`...`)` block from the draft's `up()` and executes it. Every draft here is idempotent
// (CREATE OR REPLACE FUNCTION / IF NOT EXISTS / DROP TRIGGER IF EXISTS), so re-running is safe.
//
//   node scripts/rebuild-cutover/apply-migration-draft.mjs <draft.ts> <postgres-url>
//
// Used to place 085/086/087/088 on the staging DB during the cutover (2026-07-05) and by the
// rust-live-pg CI job. It does NOT record a pgmigrations row — formal placement into
// packages/db/migrations/ (operator, red-line) owns the version ledger. This is a fixture/probe
// applier, not a substitute for the migration runner.

import fs from 'node:fs';
import { execFileSync } from 'node:child_process';

const [draftPath, dbUrl] = process.argv.slice(2);
if (!draftPath || !dbUrl) {
  console.error('usage: apply-migration-draft.mjs <draft.ts> <postgres-url>');
  process.exit(2);
}

const src = fs.readFileSync(draftPath, 'utf8');
// Grab the first pgm.sql(`…`) template literal in up(). Drafts here each have exactly one.
const m = src.match(/pgm\.sql\(`([\s\S]*?)`\)\s*;/);
if (!m) {
  console.error(`no pgm.sql(\`...\`) block found in ${draftPath}`);
  process.exit(1);
}
const sql = m[1];

try {
  // psql reads the SQL on stdin; ON_ERROR_STOP makes any statement failure a non-zero exit.
  execFileSync('psql', [dbUrl, '-v', 'ON_ERROR_STOP=1', '-f', '-'], {
    input: sql,
    stdio: ['pipe', 'inherit', 'inherit'],
  });
  console.log(`applied ${draftPath}`);
} catch {
  console.error(`FAILED applying ${draftPath}`);
  process.exit(1);
}
