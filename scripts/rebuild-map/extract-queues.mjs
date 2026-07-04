#!/usr/bin/env node
// scripts/rebuild-map/extract-queues.mjs
//
// Namespace: queues
// Mirrors inventory/10-api-realtime-jobs.md §0/§1 queue census. The canonical registry is
// `packages/shared-types/src/queue-names.ts` (`QUEUE_NAMES` const, doc: 28 entries) plus
// "5 local ad-hoc constants" (courier.offer_sweep, order.timeout_sweep,
// acquisition.retention-sweep, delivery-trace.retention-sweep, health-job) declared inline
// at their call sites rather than in the shared registry. Doc total: 33.
//
// This extractor stays generic (no hardcoded name list): it (1) parses every
// `KEY: 'dotted.name'` entry out of queue-names.ts, and (2) greps apps/api/src +
// apps/worker/src for local `const X = 'dotted-or-underscored.name'` declarations whose
// value looks queue-name-shaped (lowercase, contains a `.`), unioning the two sets.
// Known gap: `health-job` (a hyphenated literal with no `.`, per inventory §7 "never
// enqueued anywhere — scaffolding only") doesn't match the dotted-name heuristic and is
// deliberately not special-cased in — expected count is 32 (28 registry + 4 of the 5 local
// consts), a documented 1-row delta from the doc's 33, not a bug to chase.

import { readRepoFile, idSafe, isMain, printRecords, stableSort, dedupeById, walkFiles } from './lib/common.mjs';

const QUEUE_NAMES_FILE = 'packages/shared-types/src/queue-names.ts';
const REGISTRY_ENTRY_RE = /^\s*([A-Z][A-Z0-9_]*)\s*:\s*'([a-z][a-z0-9_.-]*\.[a-z0-9_.-]+)'/;
const LOCAL_CONST_RE = /const\s+[A-Za-z0-9_]+\s*=\s*'([a-z][a-z0-9_.-]*\.[a-z0-9_.-]+)'/;

/** Pure/testable: parse queue-names.ts text -> [{name, line}] (registry entries). */
export function parseQueueRegistry(content) {
  const out = [];
  const lines = content.split('\n');
  for (let i = 0; i < lines.length; i++) {
    const m = REGISTRY_ENTRY_RE.exec(lines[i]);
    if (m) out.push({ name: m[2], line: i + 1 });
  }
  return out;
}

/** Pure/testable: parse one file's text for local queue-name-shaped const declarations. */
export function parseLocalQueueConsts(content) {
  const out = [];
  const lines = content.split('\n');
  for (let i = 0; i < lines.length; i++) {
    const m = LOCAL_CONST_RE.exec(lines[i]);
    if (m) out.push({ name: m[1], line: i + 1 });
  }
  return out;
}

export async function extract() {
  const registry = parseQueueRegistry(readRepoFile(QUEUE_NAMES_FILE)).map(({ name, line }) => ({
    ns: 'queues',
    id: `JOB-${idSafe(name)}`,
    file: QUEUE_NAMES_FILE,
    line,
  }));

  const files = [
    ...walkFiles('apps/api/src', ['.ts']),
    ...walkFiles('apps/worker/src', ['.ts']),
  ].filter((f) => !/\.test\.|\.spec\./.test(f));

  let local = [];
  for (const f of files) {
    const hits = parseLocalQueueConsts(readRepoFile(f));
    for (const h of hits) {
      local.push({ ns: 'queues', id: `JOB-${idSafe(h.name)}`, file: f, line: h.line });
    }
  }

  // Registry entries always win on id collision: dedupe each set independently, then only
  // fold in local names the registry doesn't already have (stableSort alone can't express
  // this priority since it sorts by file path, not source-of-truth precedence).
  const registryDeduped = dedupeById(stableSort(registry));
  const registryIds = new Set(registryDeduped.map((r) => r.id));
  const localDeduped = dedupeById(stableSort(local)).filter((r) => !registryIds.has(r.id));
  return stableSort([...registryDeduped, ...localDeduped]);
}

if (isMain(import.meta.url)) {
  const records = await extract();
  printRecords(records);
}
