#!/usr/bin/env node
// scripts/rebuild-map/extract-error-codes.mjs
//
// Namespace: error-codes
// Mirrors inventory/14-crosscutting-proofnet.md §3 extraction:
//   grep -rn "\.sendError(" apps/api/src | wc -l                                    -> 311 sites
//   grep -rhoE "\.sendError\([0-9]+,\s*'[A-Za-z0-9_./-]+'" apps/api/src \
//     | sed -E "s/.*'(.*)'/\1/" | sort -u | wc -l                                   -> 68 unique codes
// One record per UNIQUE code (id = ERR-<CODE>), file:line = first occurrence.

import { readRepoFile, idSafe, isMain, printRecords, stableSort, dedupeById, walkFiles } from './lib/common.mjs';

const SEND_ERROR_RE = /\.sendError\(\s*[0-9]+\s*,\s*'([A-Za-z0-9_./-]+)'/;

/** Pure/testable: parse one file's text -> [{code, line}] for every .sendError(status, 'CODE') site. */
export function parseSendErrorSites(content) {
  const out = [];
  const lines = content.split('\n');
  for (let i = 0; i < lines.length; i++) {
    const m = SEND_ERROR_RE.exec(lines[i]);
    if (m) out.push({ code: m[1], line: i + 1 });
  }
  return out;
}

export async function extract() {
  const files = walkFiles('apps/api/src', ['.ts']);
  let all = [];
  for (const f of files) {
    const hits = parseSendErrorSites(readRepoFile(f));
    for (const h of hits) {
      all.push({ ns: 'error-codes', id: `ERR-${idSafe(h.code)}`, file: f, line: h.line });
    }
  }
  // one record per unique code (first occurrence)
  return dedupeById(stableSort(all));
}

if (isMain(import.meta.url)) {
  const records = await extract();
  printRecords(records);
}
