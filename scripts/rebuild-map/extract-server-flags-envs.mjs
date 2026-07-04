#!/usr/bin/env node
// scripts/rebuild-map/extract-server-flags-envs.mjs
//
// Namespace: server-flags-envs
// REBUILD-MAP §7 item 4: "EnvSchema census (80) was manual — the Phase-0 extractor must
// parse the Zod schema programmatically." This extractor does NOT line-regex-count —
// it walks `packages/config/src/index.ts`, finds the `EnvSchema = z.object({ ... })`
// call, tracks brace depth so only TOP-LEVEL keys of that object are counted (immune to
// nested z.object/z.array/enum option braces), and emits one ENV-<NAME> record per field.
//
// Cross-referenced per inventory/14 §1b / §2:
//   grep -cE '^\s*[A-Z][A-Z0-9_]*:\s*z\.' packages/config/src/index.ts        -> schema fields
//   grep -rhoE 'process\.env\.[A-Z_0-9]+' apps/api/src apps/worker/src \
//     packages/db/src packages/config/src | sort -u                          -> raw reads
// Doc counts: inventory/10 (manual, stale) said 80 schema + 48 raw; inventory/14 (same-day
// reconciliation) already found 119 schema fields live — this extractor should agree with
// the LATTER (119), which is the point: the manual 80 in inventory/10 §0 is the exact
// "must die" count REBUILD-MAP §7 flags.
//
// FINDING (this extractor found it, the doc's own grep command did not): the raw-read grep
// above silently drops apps/api/src/lib/ai-ocr-parser.ts — grep's binary-file heuristic
// misfires on that file (`grep: ...: binary file matches`, no -a), so its 19 process.env.*
// hits (GROQ_*/OPENAI_*/OPENROUTER_*/OPENCODE_ZEN_*/LLM_*/PADDLE_OCR_*/MENU_*) never reach
// the doc's reported 48. This extractor reads files as UTF-8 text directly (no shell grep,
// no binary heuristic) and finds 67 unique raw names — 48 (doc/grep) + 19 (grep-dropped).
// Exactly the class of silent loss REBUILD-MAP §5/§7 exists to make mechanical instead of
// narrated.

import { readRepoFile, idSafe, isMain, printRecords, stableSort, dedupeById, walkFiles } from './lib/common.mjs';

const SCHEMA_FILE = 'packages/config/src/index.ts';
const SCHEMA_DECL_RE = /(?:const|let|var)\s+EnvSchema\s*=\s*z\.object\(\s*\{/;
// `z\b` (not `z\.`) so a value whose builder chain wraps onto the NEXT line
// (`NAME: z\n    .string()`) is still recognized as starting a zod value — the field-key
// line itself only ever needs to show the `z` namespace token, not the first `.method()`.
const FIELD_KEY_RE = /^\s*([A-Z][A-Z0-9_]*)\s*:\s*z\b/;
const PROCESS_ENV_RE = /process\.env\.([A-Z_0-9]+)/g;

/**
 * Pure/testable: given the full text of packages/config/src/index.ts, return the list of
 * top-level EnvSchema field names in source order, via brace-depth tracking (not a flat
 * line regex over the whole file) so nested objects/enums never inflate the count.
 */
export function parseEnvSchemaFields(content) {
  const startMatch = SCHEMA_DECL_RE.exec(content);
  if (!startMatch) return [];
  const bodyStart = startMatch.index + startMatch[0].length; // just after the opening '{'
  const lines = content.slice(bodyStart).split('\n');

  const fields = [];
  let depth = 1; // we're already 1 level deep (inside the object's opening brace)
  for (const line of lines) {
    if (depth === 1) {
      const m = FIELD_KEY_RE.exec(line);
      if (m) fields.push(m[1]);
    }
    for (const ch of line) {
      if (ch === '{' || ch === '(' || ch === '[') depth++;
      else if (ch === '}' || ch === ')' || ch === ']') depth--;
    }
    if (depth <= 0) break; // reached the object's closing brace
  }
  return fields;
}

/** Pure/testable: raw `process.env.X` reads in one file's text -> [{name, line}]. */
export function parseRawProcessEnvFromFile(content) {
  const out = [];
  const lines = content.split('\n');
  for (let i = 0; i < lines.length; i++) {
    let m;
    PROCESS_ENV_RE.lastIndex = 0;
    while ((m = PROCESS_ENV_RE.exec(lines[i]))) {
      out.push({ name: m[1], line: i + 1 });
    }
  }
  return out;
}

export async function extract() {
  const schemaContent = readRepoFile(SCHEMA_FILE);
  const fields = parseEnvSchemaFields(schemaContent);
  const schemaLines = schemaContent.split('\n');
  const schemaRecords = fields.map((name) => {
    const lineIdx = schemaLines.findIndex((l) => new RegExp(`^\\s*${name}\\s*:\\s*z\\.`).test(l));
    return {
      ns: 'server-flags-envs',
      id: `ENV-${idSafe(name)}`,
      file: SCHEMA_FILE,
      line: lineIdx >= 0 ? lineIdx + 1 : 0,
    };
  });

  const rawDirs = ['apps/api/src', 'apps/worker/src', 'packages/db/src', 'packages/config/src'];
  const rawFiles = rawDirs.flatMap((d) => walkFiles(d, ['.ts', '.tsx']));
  let rawRecords = [];
  for (const f of rawFiles) {
    const hits = parseRawProcessEnvFromFile(readRepoFile(f));
    for (const h of hits) {
      rawRecords.push({
        ns: 'server-flags-envs',
        id: `ENV-RAW-${idSafe(h.name)}`,
        file: f,
        line: h.line,
      });
    }
  }
  // first occurrence per raw-name wins (stable-sorted first)
  rawRecords = dedupeById(stableSort(rawRecords));

  return stableSort([...schemaRecords, ...rawRecords]);
}

if (isMain(import.meta.url)) {
  const records = await extract();
  printRecords(records);
}
