#!/usr/bin/env node
// scripts/rebuild-map/extract-routes.mjs
//
// Namespace: routes
// Mirrors the exact fastify-registration grep from inventory/10-api-realtime-jobs.md §0
// (run from apps/api):
//   grep -rnE "^\s*(fastify|app|server|f|instance)\.(get|post|put|patch|delete|all|head|options|route)\("
//     src --include="*.ts" | grep -vE "\.test\.|\.spec\."
// Doc count: 236.

import { walkFiles, readRepoFile, idSafe, isMain, printRecords, stableSort } from './lib/common.mjs';

// Base match = the doc's exact grep pattern (call must START the line, modulo leading whitespace).
// Deliberately does NOT require the path argument to be on the same line — many registrations
// wrap the literal onto the next line (`fastify.post(\n  '/x',`), and the doc's grep counts the
// call line regardless of where the literal lands.
const ROUTE_CALL_RE =
  /^\s*(?:fastify|app|server|f|instance)\.(get|post|put|patch|delete|all|head|options|route)\s*\(/;
// Best-effort literal-path sniff: only used to make the ID readable; falls back to file:line.
const INLINE_PATH_RE = /\(\s*(['"`])([^'"`]*)\1/;

/** Pure/testable: parse one file's text, return route records (line-scoped). */
export function parseRoutesFromFile(content, relPath) {
  const records = [];
  const lines = content.split('\n');
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const m = ROUTE_CALL_RE.exec(line);
    if (!m) continue;
    const method = m[1].toUpperCase();
    const pathMatch = INLINE_PATH_RE.exec(line);
    const literalPath = pathMatch ? pathMatch[2] : '';
    const pathFrag = literalPath ? idSafe(literalPath) : `L${i + 1}`;
    const id = `ROUTE-${method}-${idSafe(relPath)}-${pathFrag}`;
    records.push({ ns: 'routes', id, file: relPath, line: i + 1 });
  }
  return records;
}

export async function extract() {
  const files = walkFiles('apps/api/src', ['.ts']).filter(
    (f) => !/\.test\.|\.spec\./.test(f),
  );
  let all = [];
  for (const f of files) {
    const content = readRepoFile(f);
    all = all.concat(parseRoutesFromFile(content, f));
  }
  return stableSort(all);
}

if (isMain(import.meta.url)) {
  const records = await extract();
  printRecords(records);
}
