#!/usr/bin/env node
// scripts/rebuild-map/extract-all.mjs
//
// Driver: runs every namespace extractor against the CURRENT (Node/React) tree and writes
// scripts/rebuild-map/out/inventory-current.jsonl — one stable-sorted JSON line per record
// ({ns, id, file, line}), matching REBUILD-MAP.md §5 "extract-old.<ns>" half of the gate.
//
// Usage: node scripts/rebuild-map/extract-all.mjs [--stdout]
//   (no flag)  writes out/inventory-current.jsonl, prints a per-namespace count summary
//   --stdout   also streams every record to stdout (useful for `repowise distill`)

import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { REPO_ROOT, stableSort } from './lib/common.mjs';

import { extract as extractRoutes } from './extract-routes.mjs';
import { extract as extractFeRoutes } from './extract-fe-routes.mjs';
import { extract as extractComponents } from './extract-components.mjs';
import { extract as extractViteFlags } from './extract-vite-flags.mjs';
import { extract as extractServerFlagsEnvs } from './extract-server-flags-envs.mjs';
import { extract as extractI18nKeys } from './extract-i18n-keys.mjs';
import { extract as extractWsTypes } from './extract-ws-types.mjs';
import { extract as extractQueues } from './extract-queues.mjs';
import { extract as extractErrorCodes } from './extract-error-codes.mjs';
import { extract as extractTables } from './extract-tables.mjs';
import { extract as extractScriptsGates } from './extract-scripts-gates.mjs';

const EXTRACTORS = [
  ['routes', extractRoutes],
  ['fe-routes', extractFeRoutes],
  ['components', extractComponents],
  ['vite-flags', extractViteFlags],
  ['server-flags-envs', extractServerFlagsEnvs],
  ['i18n-keys', extractI18nKeys],
  ['ws-types', extractWsTypes],
  ['queues', extractQueues],
  ['error-codes', extractErrorCodes],
  ['tables', extractTables],
  ['scripts-gates', extractScriptsGates],
];

async function main() {
  const stdoutMode = process.argv.includes('--stdout');
  const counts = {};
  let all = [];

  for (const [ns, fn] of EXTRACTORS) {
    const records = await fn();
    counts[ns] = records.length;
    all = all.concat(records);
  }

  all = stableSort(all);
  const outDir = join(REPO_ROOT, 'scripts/rebuild-map/out');
  mkdirSync(outDir, { recursive: true });
  const outFile = join(outDir, 'inventory-current.jsonl');
  const body = all.map((r) => JSON.stringify({ ns: r.ns, id: r.id, file: r.file, line: r.line || 0 })).join('\n') + '\n';
  writeFileSync(outFile, body, 'utf8');

  if (stdoutMode) process.stdout.write(body);

  console.error(`Wrote ${all.length} records to scripts/rebuild-map/out/inventory-current.jsonl`);
  console.error('Per-namespace counts:');
  for (const [ns] of EXTRACTORS) {
    console.error(`  ${ns.padEnd(20)} ${counts[ns]}`);
  }
}

main();
