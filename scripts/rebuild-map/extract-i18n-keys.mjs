#!/usr/bin/env node
// scripts/rebuild-map/extract-i18n-keys.mjs
//
// Namespace: i18n-keys
// Mirrors inventory/11-frontend-surface.md §0 C6 extraction:
//   grep -cE "^  '[^']+':" packages/ui/src/lib/i18n-catalog.ts
// Doc count: 1,445 (× 3 locales sq/en/uk). Implemented as a brace-depth-aware top-level-key
// walk (same technique as the env extractor) so a multi-line nested value never double-counts.

import { readRepoFile, idSafe, isMain, printRecords, stableSort } from './lib/common.mjs';

const CATALOG_FILE = 'packages/ui/src/lib/i18n-catalog.ts';
const CATALOG_DECL_RE = /catalog\s*:\s*Record<string,\s*CatalogEntry>\s*=\s*\{/;
const KEY_RE = /^\s*'([^']+)'\s*:/;

/** Pure/testable: given the catalog file's text, return [{key, line}] for top-level entries. */
export function parseI18nKeys(content) {
  const startMatch = CATALOG_DECL_RE.exec(content);
  if (!startMatch) return [];
  const before = content.slice(0, startMatch.index);
  const startLine = before.split('\n').length; // 1-based line of the declaration
  const bodyStart = startMatch.index + startMatch[0].length;
  const lines = content.slice(bodyStart).split('\n');

  const out = [];
  let depth = 1;
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (depth === 1) {
      const m = KEY_RE.exec(line);
      if (m) out.push({ key: m[1], line: startLine + i });
    }
    for (const ch of line) {
      if (ch === '{' || ch === '(' || ch === '[') depth++;
      else if (ch === '}' || ch === ')' || ch === ']') depth--;
    }
    if (depth <= 0) break;
  }
  return out;
}

export async function extract() {
  const content = readRepoFile(CATALOG_FILE);
  const entries = parseI18nKeys(content);
  return stableSort(
    entries.map(({ key, line }) => ({
      ns: 'i18n-keys',
      id: `KEY-${idSafe(key)}`,
      file: CATALOG_FILE,
      line,
    })),
  );
}

if (isMain(import.meta.url)) {
  const records = await extract();
  printRecords(records);
}
