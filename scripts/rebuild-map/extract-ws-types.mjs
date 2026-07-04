#!/usr/bin/env node
// scripts/rebuild-map/extract-ws-types.mjs
//
// Namespace: ws-types
// Mirrors inventory/10-api-realtime-jobs.md §2/§3 WS census:
//   inbound:  grep -nE "msg\.type ===" apps/api/src/websocket.ts                   (doc: 5)
//   outbound: grep -roE "type:\s*'[a-zA-Z_.]+'" apps/api/src --include='*.ts'      (doc: 24
//             distinct room-targeted frame types, after excluding non-WS unions/tests —
//             this "dumb" extractor deliberately does NOT do that exclusion pass, per the
//             §8b design note "extractors are deliberately dumb" — it emits every distinct
//             `type: '...'` literal it finds under apps/api/src as a candidate row; the
//             gate/reconciliation step is expected to show a DELTA here, not a bug.)
// Control frames (websocket.ts only): auth_success, subscribed, error, client_location,
// client_location_stop (per inventory §3 note: "+5 control types").

import { readRepoFile, idSafe, isMain, printRecords, stableSort, dedupeById, walkFiles } from './lib/common.mjs';

const WS_FILE = 'apps/api/src/websocket.ts';
const INBOUND_RE = /msg\.type\s*===\s*'([^']+)'/;
const OUTBOUND_TYPE_RE = /type:\s*'([a-zA-Z_.]+)'/g;

/** Pure/testable: inbound message types from websocket.ts text -> [{type, line}]. */
export function parseInboundWsTypes(content) {
  const out = [];
  const lines = content.split('\n');
  for (let i = 0; i < lines.length; i++) {
    const m = INBOUND_RE.exec(lines[i]);
    if (m) out.push({ type: m[1], line: i + 1 });
  }
  return out;
}

/** Pure/testable: outbound `type: '...'` literals from one file's text -> [{type, line}]. */
export function parseOutboundWsTypes(content) {
  const out = [];
  const lines = content.split('\n');
  for (let i = 0; i < lines.length; i++) {
    let m;
    OUTBOUND_TYPE_RE.lastIndex = 0;
    while ((m = OUTBOUND_TYPE_RE.exec(lines[i]))) {
      out.push({ type: m[1], line: i + 1 });
    }
  }
  return out;
}

export async function extract() {
  const wsContent = readRepoFile(WS_FILE);

  const inbound = parseInboundWsTypes(wsContent).map(({ type, line }) => ({
    ns: 'ws-types',
    id: `WS-IN-${idSafe(type)}`,
    file: WS_FILE,
    line,
  }));

  const apiFiles = walkFiles('apps/api/src', ['.ts']).filter((f) => !/\.test\.|\.spec\./.test(f));
  let outbound = [];
  for (const f of apiFiles) {
    const hits = parseOutboundWsTypes(readRepoFile(f));
    for (const h of hits) {
      outbound.push({ ns: 'ws-types', id: `WS-OUT-${idSafe(h.type)}`, file: f, line: h.line });
    }
  }
  outbound = dedupeById(stableSort(outbound));

  return stableSort([...dedupeById(stableSort(inbound)), ...outbound]);
}

if (isMain(import.meta.url)) {
  const records = await extract();
  printRecords(records);
}
