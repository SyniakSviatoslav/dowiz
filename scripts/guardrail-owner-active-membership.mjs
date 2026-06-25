#!/usr/bin/env node
// Guardrail (ADR-0004 P-d): every owner-authorization / location-resolution query that filters
// `role = 'owner'` MUST also filter `status = 'active'` — otherwise a removed/downgraded owner
// holding a still-valid (≤24h) access token keeps tenant access. A new owner-membership query
// that forgets the status predicate re-opens the insider-removal window. This is the regression
// backstop the council required (red before the P-d fix, green after).
//
// Run: node scripts/guardrail-owner-active-membership.mjs
// Escape hatch for a genuine non-auth query (e.g. an audit/GDPR listing that intentionally
// includes removed owners): add `guardrail-exempt: <reason>` on the same line.
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = 'apps/api/src';
const violations = [];

function walk(dir) {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    const s = statSync(p);
    if (s.isDirectory()) walk(p);
    else if (p.endsWith('.ts')) scan(p);
  }
}

function scan(file) {
  const lines = readFileSync(file, 'utf8').split('\n');
  lines.forEach((line, i) => {
    if (!/role\s*=\s*'owner'/.test(line)) return;
    if (!/memberships/i.test(line)) return;            // only membership queries
    if (/status\s*=\s*'active'/.test(line)) return;     // compliant
    if (/guardrail-exempt:/.test(line)) return;         // explicitly waived
    violations.push(`${file}:${i + 1}: owner membership query without status='active' → ${line.trim().slice(0, 120)}`);
  });
}

walk(ROOT);

if (violations.length) {
  console.error(`✗ guardrail-owner-active-membership: ${violations.length} owner membership query(ies) missing status='active' (ADR-0004 P-d):`);
  for (const v of violations) console.error('  ' + v);
  console.error("\nAdd \"AND status = 'active'\" to the query, or annotate with `guardrail-exempt: <reason>` if it is a deliberate non-auth listing.");
  process.exit(1);
}
console.log("✓ guardrail-owner-active-membership: all owner membership queries filter status='active' (ADR-0004 P-d).");
