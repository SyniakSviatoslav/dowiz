#!/usr/bin/env node
// scripts/rebuild-map/extract-fe-routes.mjs
//
// Namespace: fe-routes
// Mirrors inventory/11-frontend-surface.md §0 C1 extraction:
//   grep -h '<Route ' apps/web/src/main.tsx apps/web/src/routes/*.tsx | wc -l
// Doc count (C1, raw route elements): 40. (REBUILD-MAP §1 reports 27 addressable paths /
// 35 pages — a different unit; this extractor deliberately reproduces C1's route-ELEMENT
// count since that's the one with a machine-exact extraction command. The 27-vs-40
// distinction is itself a reconciliation note, not a bug — see README.)

import { readRepoFile, fileExists, idSafe, isMain, printRecords, stableSort, walkFiles } from './lib/common.mjs';

const ROUTE_ELEMENT_RE = /<Route\s+([^>]*)/;
const PATH_ATTR_RE = /\bpath=(['"])([^'"]*)\1/;
const INDEX_ATTR_RE = /\bindex\b/;

/** Pure/testable: parse one file's text, return one record per `<Route ...` element. */
export function parseFeRoutesFromFile(content, relPath) {
  const records = [];
  const lines = content.split('\n');
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (!/<Route\s/.test(line)) continue;
    const m = ROUTE_ELEMENT_RE.exec(line);
    if (!m) continue;
    const attrs = m[1];
    const pathMatch = PATH_ATTR_RE.exec(attrs);
    let pathFrag;
    if (pathMatch) pathFrag = idSafe(pathMatch[2] || '/');
    else if (INDEX_ATTR_RE.test(attrs)) pathFrag = 'index';
    else pathFrag = `L${i + 1}`;
    const id = `PAGE-${idSafe(relPath)}-${pathFrag}-L${i + 1}`;
    records.push({ ns: 'fe-routes', id, file: relPath, line: i + 1 });
  }
  return records;
}

export async function extract() {
  const candidates = ['apps/web/src/main.tsx', ...walkFiles('apps/web/src/routes', ['.tsx'])];
  let all = [];
  for (const f of candidates) {
    if (!fileExists(f)) continue;
    all = all.concat(parseFeRoutesFromFile(readRepoFile(f), f));
  }
  return stableSort(all);
}

if (isMain(import.meta.url)) {
  const records = await extract();
  printRecords(records);
}
