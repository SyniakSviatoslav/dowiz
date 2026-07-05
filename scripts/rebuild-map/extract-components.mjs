#!/usr/bin/env node
// scripts/rebuild-map/extract-components.mjs
//
// Namespace: components
// Mirrors inventory/11-frontend-surface.md §0 C3/C4 extraction:
//   C3: find packages/ui/src -name '*.tsx' | wc -l   (doc: 56)
//   C4: find apps/web/src/components -name '*.tsx' | wc -l   (doc: 11)
// REBUILD-MAP §1 headline: "Components | 67 (56 ui + 11 web; 8 dead-candidates)".

import { walkFiles, idSafe, isMain, printRecords, stableSort } from './lib/common.mjs';

function toRecord(relPath, subtree) {
  const base = relPath.split('/').pop().replace(/\.tsx$/, '');
  const id = `COMP-${subtree}-${idSafe(base)}-${idSafe(relPath)}`;
  return { ns: 'components', id, file: relPath, line: 1 };
}

export async function extract() {
  const ui = walkFiles('packages/ui/src', ['.tsx']).map((f) => toRecord(f, 'ui'));
  const web = walkFiles('apps/web/src/components', ['.tsx']).map((f) => toRecord(f, 'web'));
  return stableSort([...ui, ...web]);
}

if (isMain(import.meta.url)) {
  const records = await extract();
  printRecords(records);
}
