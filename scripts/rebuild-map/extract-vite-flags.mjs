#!/usr/bin/env node
// scripts/rebuild-map/extract-vite-flags.mjs
//
// Namespace: vite-flags
// Mirrors inventory/11-frontend-surface.md §0 C8 / inventory/14 §1a extraction:
//   grep -rhoE 'VITE_[A-Z0-9_]+' apps/web/src packages/ui/src | sort -u | wc -l
// Doc count: 19.

import { walkFiles, readRepoFile, idSafe, isMain, printRecords, stableSort, dedupeById } from './lib/common.mjs';

const VITE_FLAG_RE = /VITE_[A-Z0-9_]+/g;

/** Pure/testable: parse one file's text, return one record per VITE_* occurrence (first-seen line kept by caller dedupe). */
export function parseViteFlagsFromFile(content, relPath) {
  const records = [];
  const lines = content.split('\n');
  for (let i = 0; i < lines.length; i++) {
    const matches = lines[i].match(VITE_FLAG_RE);
    if (!matches) continue;
    for (const name of matches) {
      records.push({ ns: 'vite-flags', id: `FLAG-${idSafe(name)}`, file: relPath, line: i + 1 });
    }
  }
  return records;
}

export async function extract() {
  const files = [
    ...walkFiles('apps/web/src', ['.ts', '.tsx']),
    ...walkFiles('packages/ui/src', ['.ts', '.tsx']),
  ];
  let all = [];
  for (const f of files) {
    all = all.concat(parseViteFlagsFromFile(readRepoFile(f), f));
  }
  // dedupeById keeps the FIRST occurrence per (ns,id) from the input order; sort first so
  // "first occurrence" means "first in stable file/line order", not filesystem-walk order.
  return dedupeById(stableSort(all));
}

if (isMain(import.meta.url)) {
  const records = await extract();
  printRecords(records);
}
