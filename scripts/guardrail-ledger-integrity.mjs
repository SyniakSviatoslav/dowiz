#!/usr/bin/env node
// Guardrail — REGRESSION-LEDGER row numbers must be unique (the "ledger #N" cross-reference
// scheme used by lessons, memory, and commit messages breaks on duplicates).
//
// Regression (meta-loop P3, 2026-07-02): rows #7, #9, #10, #11 each appeared TWICE (distinct
// bugs sharing a number) and #27 is missing — the ledger is appended per-fix by the main agent
// with no curation step, so numbering drifted for weeks unnoticed. Duplicates were
// disambiguated with a letter suffix (7b, 9b, 10b, 11b); #27 stays a documented gap. This
// asserts uniqueness so drift can never silently return.
//
// Run: node scripts/guardrail-ledger-integrity.mjs [path]   (path override enables red-proofs
// against historical versions, e.g. a git-show temp file)
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const PATH = process.argv[2] || join(process.cwd(), 'docs/regressions/REGRESSION-LEDGER.md');
const src = readFileSync(PATH, 'utf8');

const nums = [];
for (const line of src.split('\n')) {
  const m = line.match(/^\|\s*(\d+[a-z]?)\s*\|/);
  if (m) nums.push(m[1]);
}
if (nums.length === 0) {
  console.error('✗ guardrail-ledger-integrity: parsed 0 ledger rows — table format changed?');
  process.exit(1);
}

const seen = new Map();
const dups = [];
for (const n of nums) {
  seen.set(n, (seen.get(n) || 0) + 1);
  if (seen.get(n) === 2) dups.push(n);
}
if (dups.length) {
  console.error(`✗ guardrail-ledger-integrity: duplicate row number(s): ${dups.join(', ')} — every row needs a unique # (suffix with a letter if two fixes truly share a number).`);
  process.exit(1);
}

const KNOWN_GAPS = new Set([27]); // historical, documented — do not re-flag
const plain = nums.filter((n) => /^\d+$/.test(n)).map(Number).sort((a, b) => a - b);
const gaps = [];
for (let i = plain[0]; i <= plain[plain.length - 1]; i++) {
  if (!plain.includes(i) && !KNOWN_GAPS.has(i)) gaps.push(i);
}
if (gaps.length) console.log(`  note: unassigned number(s) (not failing, but don't reuse): ${gaps.join(', ')}`);

console.log(`✓ guardrail-ledger-integrity: ${nums.length} rows, all numbers unique (max #${plain[plain.length - 1]}).`);
