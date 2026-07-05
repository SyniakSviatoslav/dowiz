#!/usr/bin/env node
// scripts/rebuild-map/seed-traceability.mjs
//
// REBUILD-MAP.md §3 Phase-0 item 1 / §4: seed docs/design/rebuild-plan/traceability.csv from
// the extractor output. Schema (REBUILD-MAP §4):
//   id, namespace, artifact(file:line), behavior(blank-ok), target(blank=UNMAPPED),
//   proof(blank), phase(blank), redline(0/1), status(=MAPPED), disposition(blank)
//
// Redline heuristic (task brief): mark 1 when the id/file matches
// money|auth|rls|gdpr|payment|courier-dispatch|ws (case-insensitive). This is intentionally
// imperfect — false positives/negatives are expected; the lead refines redline rows during
// council packet drafting (REBUILD-MAP §3 Phase-0 item 5).
//
// Deterministic: reads out/inventory-current.jsonl (already stable-sorted by extract-all.mjs)
// and writes rows in the same order — re-running with no source changes diffs clean.
//
// Usage: node scripts/rebuild-map/seed-traceability.mjs
//   (run extract-all.mjs first so out/inventory-current.jsonl is current)

import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { REPO_ROOT } from './lib/common.mjs';

const IN_FILE = join(REPO_ROOT, 'scripts/rebuild-map/out/inventory-current.jsonl');
const OUT_FILE = join(REPO_ROOT, 'docs/design/rebuild-plan/traceability.csv');

const REDLINE_RE = /money|auth|rls|gdpr|payment|courier-?dispatch|dispatch|\bws\b|websocket/i;
// ws-types is structurally a redline namespace regardless of content match (ADR-0013 tri-state authz).
const REDLINE_NAMESPACES = new Set(['ws-types']);

const CSV_HEADER = ['id', 'namespace', 'artifact', 'behavior', 'target', 'proof', 'phase', 'redline', 'status', 'disposition'];

function csvEscape(value) {
  const s = String(value ?? '');
  if (/[",\n]/.test(s)) return `"${s.replace(/"/g, '""')}"`;
  return s;
}

function isRedline(record) {
  if (REDLINE_NAMESPACES.has(record.ns)) return true;
  const haystack = `${record.id} ${record.file}`;
  return REDLINE_RE.test(haystack);
}

function main() {
  if (!existsSync(IN_FILE)) {
    console.error(`Missing ${IN_FILE} — run: node scripts/rebuild-map/extract-all.mjs first`);
    process.exit(1);
  }
  const lines = readFileSync(IN_FILE, 'utf8').trim().split('\n').filter(Boolean);
  const records = lines.map((l) => JSON.parse(l));

  const rows = records.map((r) => {
    const artifact = `${r.file}:${r.line || 0}`;
    return [
      r.id,
      r.ns,
      artifact,
      '', // behavior — blank-ok at seed
      '', // target — blank == UNMAPPED
      '', // proof — blank at seed
      '', // phase — blank at seed
      isRedline(r) ? '1' : '0',
      'MAPPED', // status — every seeded row starts MAPPED (it has a matrix row now)
      '', // disposition — blank at seed, lead assigns PORT/CARRY-VERBATIM/FIX-IN-PORT/RETIRE/KEEP
    ];
  });

  const body = [CSV_HEADER, ...rows].map((row) => row.map(csvEscape).join(',')).join('\n') + '\n';
  writeFileSync(OUT_FILE, body, 'utf8');

  const redlineCount = rows.filter((r) => r[7] === '1').length;
  console.error(`Wrote ${rows.length} rows to docs/design/rebuild-plan/traceability.csv (${redlineCount} redline).`);
}

main();
