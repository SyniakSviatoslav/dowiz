#!/usr/bin/env node
// scripts/rebuild-map/map-coverage.mjs
//
// Coverage gate v0 (REBUILD-MAP.md §5 / inventory/14 §8b). Compares
// scripts/rebuild-map/out/inventory-current.jsonl (extract-old: current Node/React tree)
// against docs/design/rebuild-plan/traceability.csv (the matrix), and reports:
//   UNMAPPED — in inventory, no matrix row              (census rot / matrix drift)
//   ORPHAN   — matrix row whose artifact no longer exists at that id (stale row)
//   UNBUILT  — matrix rows whose target is set (past MAPPED, i.e. someone claimed progress)
//              but no Rust/Astro side exists yet. Phase 0 has no Rust tree to check against,
//              so this lane is STUBBED here (see the UNBUILT section below) — it becomes
//              real once `extract-new.<ns>` scripts exist for the Rust/Astro side (§5).
//
// Usage:
//   node scripts/rebuild-map/map-coverage.mjs           # report only, exit 0
//   node scripts/rebuild-map/map-coverage.mjs --strict   # exit 1 if UNMAPPED > 0
//
// Run extract-all.mjs + seed-traceability.mjs first (or just `pnpm rebuild:map`, see README).

import { readFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { REPO_ROOT } from './lib/common.mjs';

const INVENTORY_FILE = join(REPO_ROOT, 'scripts/rebuild-map/out/inventory-current.jsonl');
const TRACEABILITY_FILE = join(REPO_ROOT, 'docs/design/rebuild-plan/traceability.csv');

function parseCsv(text) {
  // Minimal CSV parser sufficient for this file's shape (fields may be quoted with "" escapes).
  const rows = [];
  const lines = text.split('\n').filter((l) => l.length > 0);
  for (const line of lines) {
    const fields = [];
    let cur = '';
    let inQuotes = false;
    for (let i = 0; i < line.length; i++) {
      const ch = line[i];
      if (inQuotes) {
        if (ch === '"') {
          if (line[i + 1] === '"') {
            cur += '"';
            i++;
          } else {
            inQuotes = false;
          }
        } else {
          cur += ch;
        }
      } else if (ch === '"') {
        inQuotes = true;
      } else if (ch === ',') {
        fields.push(cur);
        cur = '';
      } else {
        cur += ch;
      }
    }
    fields.push(cur);
    rows.push(fields);
  }
  return rows;
}

function loadInventory() {
  if (!existsSync(INVENTORY_FILE)) {
    console.error(`Missing ${INVENTORY_FILE} — run: node scripts/rebuild-map/extract-all.mjs first`);
    process.exit(1);
  }
  const lines = readFileSync(INVENTORY_FILE, 'utf8').trim().split('\n').filter(Boolean);
  return lines.map((l) => JSON.parse(l));
}

function loadTraceability() {
  if (!existsSync(TRACEABILITY_FILE)) {
    console.error(`Missing ${TRACEABILITY_FILE} — run: node scripts/rebuild-map/seed-traceability.mjs first`);
    process.exit(1);
  }
  const raw = readFileSync(TRACEABILITY_FILE, 'utf8');
  const rows = parseCsv(raw);
  const [header, ...body] = rows;
  return body.map((fields) => Object.fromEntries(header.map((h, i) => [h, fields[i] ?? ''])));
}

function main() {
  const strict = process.argv.includes('--strict');
  const inventory = loadInventory();
  const matrix = loadTraceability();

  const matrixIds = new Set(matrix.map((r) => r.id));
  const inventoryIds = new Set(inventory.map((r) => r.id));

  const unmapped = inventory.filter((r) => !matrixIds.has(r.id));
  const orphan = matrix.filter((r) => !inventoryIds.has(r.id));
  // UNBUILT stub: rows whose `target` is non-blank (claims a rebuild_target) but whose
  // `status` hasn't reached BUILT/PROVEN/CUTOVER — meaningful only once extract-new.<ns>
  // (the Rust/Astro side) exists. At Phase 0 there is no Rust tree, so this is always
  // empty by construction; documented, not silently skipped.
  const unbuilt = matrix.filter((r) => r.target && r.target.trim() && r.status === 'MAPPED');

  const byNs = {};
  for (const r of inventory) {
    byNs[r.ns] = byNs[r.ns] || { total: 0, unmapped: 0 };
    byNs[r.ns].total++;
  }
  for (const r of unmapped) {
    byNs[r.ns].unmapped++;
  }

  console.log('Map-coverage gate v0 (REBUILD-MAP.md §5)\n');
  console.log(`Inventory (current tree): ${inventory.length} records`);
  console.log(`Traceability matrix:      ${matrix.length} rows\n`);

  console.log('Per-namespace:');
  console.log(['namespace'.padEnd(20), 'total'.padStart(6), 'unmapped'.padStart(9)].join(' | '));
  console.log('-'.repeat(42));
  for (const ns of Object.keys(byNs).sort()) {
    console.log([ns.padEnd(20), String(byNs[ns].total).padStart(6), String(byNs[ns].unmapped).padStart(9)].join(' | '));
  }

  console.log(`\nUNMAPPED: ${unmapped.length} (in inventory, no matrix row)`);
  console.log(`ORPHAN:   ${orphan.length} (matrix row, artifact not in current inventory)`);
  console.log(
    `UNBUILT:  ${unbuilt.length} (STUBBED — needs extract-new.<ns> against the Rust/Astro tree, ` +
      'which does not exist yet at Phase 0; interface documented below)',
  );

  console.log(
    '\nUNBUILT interface (for the Phase A implementer): a Rust-side extract-new.<ns> pair per\n' +
      'namespace (§5/§8b design table) should emit the same {ns,id,file,line} shape from\n' +
      'crates/api + apps/astro; UNBUILT = matrix rows with phase <= current AND status >= BUILT\n' +
      'whose id is absent from that new-side set (REBUILD-MAP §8b invariant 2).',
  );

  if (strict && unmapped.length > 0) {
    console.error(`\n--strict: failing, UNMAPPED=${unmapped.length} > 0`);
    process.exit(1);
  }
  process.exit(0);
}

main();
