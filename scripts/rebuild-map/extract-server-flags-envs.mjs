#!/usr/bin/env node
// scripts/rebuild-map/extract-server-flags-envs.mjs
//
// Namespace: server-flags-envs
// REBUILD-MAP §7 item 4: "EnvSchema census (80) was manual — the Phase-0 extractor must
// parse the Zod schema programmatically." This extractor does NOT line-regex-count over the
// whole file — it walks `packages/config/src/index.ts`, finds the `EnvSchema = z.object({ ... })`
// call, and tracks bracket/paren/brace depth so only TOP-LEVEL entries of that object are
// counted (immune to nested z.object/z.array/enum option braces). Per top-level entry it also
// derives optionality and a literal `.default(...)` value from that entry's own source SPAN
// (its key line through the line before the next top-level entry) — again by regex over that
// span's text, never by importing/evaluating Zod or a TS parser.
//
// PARSING APPROACH (source-level, zero-dep — no ts-morph/typescript/@babel/* import):
//   1. Locate the `const EnvSchema = z.object({` declaration (SCHEMA_DECL_RE).
//   2. Walk the object body line-by-line, tracking `{ ( [` vs `} ) ]` depth. Only lines seen at
//      depth === 1 (direct children of the outer object) are candidate top-level entries.
//   3. A depth-1 line is one of:
//        a. `NAME: <value>`  (FIELD_KEY_RE) -> a real top-level field, name = NAME
//        b. `...someExpr`    (SPREAD_RE)    -> a spread-composed slice, name = `...someExpr`
//        c. anything else (blank line, a lone `});`, a comment)  -> ignored
//   4. Each entry's "span" (see `parseEnvSchemaFieldSpans`) = its own line through the line
//      immediately before the NEXT depth-1 entry (or the schema's closing brace for the last
//      entry). `.optional()` / `.default(X)` are regexed over that span's JOINED text — this is
//      what lets a value that wraps onto several lines (`FOO: z\n  .string()\n  .optional(),`)
//      still resolve correctly without a real parser.
//
// FAILURE MODES (by design — a zero-dep grep-level parser, not a Zod-aware/typed AST parser):
//   - FIELD_KEY_RE used to require the value to start with a literal `z` token (`NAME: z...`)
//     so a builder chain wrapping onto the next line was still recognized. That missed a
//     "renamed" field whose value is a bare REFERENCE into another schema instead of a fresh
//     `z.` builder call, e.g. `RENAMED: OtherSchema.shape.ORIGINAL` (legal Zod — reusing a
//     sub-schema's field under a new top-level key name). FIELD_KEY_RE is now `NAME: <anything
//     non-whitespace>` at depth 1 — strictly more permissive, so every prior match still
//     matches (pinned by the existing fixture tests) and renamed/reference-valued fields are
//     now also counted, with `optional`/`default` left unset for that entry (their real Zod
//     shape lives in the OTHER schema, out of this file's reach).
//   - SPREAD (`...OtherSchema.shape`) composition is fundamentally unresolvable at this parsing
//     level: how many keys it injects (and their names) lives in ANOTHER file/type, which a
//     zero-dep source-text parser cannot chase without becoming a type-checker. Rather than
//     silently dropping however many fields a spread contributes (the truly dangerous failure —
//     an invisible undercount), this extractor emits ONE opaque placeholder entry per spread
//     line (`...<expr>`, `isSpread: true`, `ENV-SPREAD-<expr>` id) so its presence is visible in
//     the count and census, with `optional`/`required`/`default` left `null` (unresolvable) — a
//     VISIBLE under-count signal, not a silent one. `packages/config/src/index.ts` has zero
//     spread usage today (`grep -c '\.\.\.' packages/config/src/index.ts` = 0), so this path is
//     currently inert; the day someone composes EnvSchema via spread, the census gains one
//     `ENV-SPREAD-*` row flagging "re-derive this site by hand" instead of quietly losing count.
//   - `.default\(([^)]*)\)` captures the default's argument text with a naive "up to the first
//     `)`" regex — correct for every literal default in the real schema today (strings/numbers/
//     CSV strings; verified: every `.default(...)` argument in packages/config/src/index.ts is a
//     literal, no nested calls), but would truncate a default computed by a nested call, e.g.
//     `.default(computeSomething())` -> captures `computeSomething(` only. No such pattern
//     exists in the current schema; documented, not a crash risk (worst case: a
//     truncated-but-harmless string is recorded as the default).
//   - Optional-vs-required heuristic: a field is NOT required when its span contains
//     `.optional()` OR `.default(...)` (a default makes the key absent-safe even though Zod
//     itself still calls that key "present" internally) — matches the inventory doc's own
//     `required?` column semantics (10 §5 "Full table": `PORT | no | 8080`, `NODE_ENV | yes |—`).
//   - Comment text containing the literal substrings `.default(`/`.optional()` would falsely
//     attribute to whichever field's span it falls inside; verified today there are none
//     (`grep -c '\.default(\|\.optional()' packages/config/src/index.ts` == the sum of real
//     `: z.` field matches, i.e. only real field lines carry these tokens) — a latent risk if a
//     future comment ever quotes Zod method calls verbatim, not something this parser guards
//     against structurally.
//
// Cross-referenced per inventory/14 §1b / §2:
//   grep -cE '^\s*[A-Z][A-Z0-9_]*:\s*z\.' packages/config/src/index.ts        -> schema fields
//   grep -rhoE 'process\.env\.[A-Z_0-9]+' apps/api/src apps/worker/src \
//     packages/db/src packages/config/src | sort -u                          -> raw reads
// Doc counts: inventory/10 (manual, stale) said 80 schema + 48 raw; inventory/14 (same-day
// reconciliation) already found 119 schema fields live — this extractor agrees with the LATTER
// (119), which is the point: the manual 80 in inventory/10 §0 is the exact "must die" count
// REBUILD-MAP §7 flags (10 §5 lines 2635/2701: "no single grep -c line matched cleanly due to
// multi-line/comment formatting... worth a script-based exact count").
//
// FINDING (this extractor found it, the doc's own grep command did not): the raw-read grep
// above silently drops apps/api/src/lib/ai-ocr-parser.ts — grep's binary-file heuristic
// misfires on that file (`grep: ...: binary file matches`, no -a), so its 19 process.env.*
// hits (GROQ_*/OPENAI_*/OPENROUTER_*/OPENCODE_ZEN_*/LLM_*/PADDLE_OCR_*/MENU_*) never reach
// the doc's reported 48. This extractor reads files as UTF-8 text directly (no shell grep,
// no binary heuristic) and finds 67 unique raw names — 48 (doc/grep) + 19 (grep-dropped).
// Exactly the class of silent loss REBUILD-MAP §5/§7 exists to make mechanical instead of
// narrated.
//
// SHADOW CLASSIFICATION (per REBUILD-MAP §7 item 4 "shadow vars ... must be classified"): every
// raw name is tagged `shadow: true` when it ALSO appears as a top-level EnvSchema key
// (validated at boot, but re-read raw at some call site instead of threading `env.X` — a
// duplication, not a true gap) vs `shadow: false` for a raw name with NO EnvSchema entry at all
// (genuinely unvalidated at boot — the inventory doc's original "shadow" definition, 10 §5 line
// 2288: "bypassing the schema"). `buildVarCensus()` / `extractVarCensus()` give the merged
// one-row-per-name view (`source: 'EnvSchema' | 'raw' | 'both'`) the task brief asks for; run
// this file with `--census` to print it.

import { readRepoFile, idSafe, isMain, stableSort, dedupeById, walkFiles, printRecords } from './lib/common.mjs';

const SCHEMA_FILE = 'packages/config/src/index.ts';
const SCHEMA_DECL_RE = /(?:const|let|var)\s+EnvSchema\s*=\s*z\.object\(\s*\{/;
// Broadened from requiring a literal `z` token after the colon (see FAILURE MODES above): any
// depth-1 `NAME: <value>` line is a real top-level field, whether its value is a fresh
// `z.something()` builder or a bare reference/rename into another schema's shape.
const FIELD_KEY_RE = /^\s*([A-Z][A-Z0-9_]*)\s*:\s*\S/;
const SPREAD_RE = /^\s*\.\.\.\s*([A-Za-z_][A-Za-z0-9_.]*)/;
const PROCESS_ENV_RE = /process\.env\.([A-Z_0-9]+)/g;
const OPTIONAL_CALL_RE = /\.optional\(\s*\)/;
const DEFAULT_CALL_RE = /\.default\(([^)]*)\)/;

/**
 * Pure/testable: given the full text of packages/config/src/index.ts, return every top-level
 * entry of the named EnvSchema object in source order as `{ name, lineIdx, isSpread, raw }`
 * (`lineIdx` is 0-based, relative to the object body's first line — see
 * `parseEnvSchemaFieldDetails` for absolute-file-line conversion). `raw` is the entry's own
 * source span (its key line through the line before the next top-level entry), the text
 * `.optional()`/`.default(...)` are regexed against.
 */
export function parseEnvSchemaFieldSpans(content) {
  const startMatch = SCHEMA_DECL_RE.exec(content);
  if (!startMatch) return [];
  const bodyStart = startMatch.index + startMatch[0].length; // just after the opening '{'
  const lines = content.slice(bodyStart).split('\n');

  const entries = [];
  let depth = 1; // we're already 1 level deep (inside the object's opening brace)
  let closeLineIdx = lines.length - 1;
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (depth === 1) {
      const m = FIELD_KEY_RE.exec(line);
      if (m) {
        entries.push({ name: m[1], lineIdx: i, isSpread: false });
      } else {
        const s = SPREAD_RE.exec(line);
        if (s) entries.push({ name: `...${s[1]}`, lineIdx: i, isSpread: true });
      }
    }
    for (const ch of line) {
      if (ch === '{' || ch === '(' || ch === '[') depth++;
      else if (ch === '}' || ch === ')' || ch === ']') depth--;
    }
    if (depth <= 0) {
      closeLineIdx = i;
      break; // reached the object's closing brace
    }
  }

  return entries.map((e, idx) => {
    const endLine = idx + 1 < entries.length ? entries[idx + 1].lineIdx : closeLineIdx + 1;
    return { ...e, raw: lines.slice(e.lineIdx, endLine).join('\n') };
  });
}

/**
 * Pure/testable: given the full text of packages/config/src/index.ts, return the list of
 * top-level EnvSchema field names in source order, via brace-depth tracking (not a flat line
 * regex over the whole file) so nested objects/enums never inflate the count. A spread
 * composition line (`...OtherSchema.shape`) surfaces as one opaque `...expr` entry — see the
 * SPREAD failure mode in the file header.
 */
export function parseEnvSchemaFields(content) {
  return parseEnvSchemaFieldSpans(content).map((e) => e.name);
}

/**
 * Pure/testable: like `parseEnvSchemaFields` but with optionality/default resolved per field,
 * plus an ABSOLUTE 1-based file line number (the convention every other extractor in this pack
 * uses). Spread placeholder entries get `optional: null, required: null, default: null`
 * (unresolvable — see FAILURE MODES).
 */
export function parseEnvSchemaFieldDetails(content) {
  const startMatch = SCHEMA_DECL_RE.exec(content);
  if (!startMatch) return [];
  const bodyStart = startMatch.index + startMatch[0].length;
  const bodyStartLine = content.slice(0, bodyStart).split('\n').length; // 1-based line of the '{' line

  return parseEnvSchemaFieldSpans(content).map((e) => {
    if (e.isSpread) {
      return { name: e.name, line: bodyStartLine + e.lineIdx, isSpread: true, optional: null, required: null, default: null };
    }
    const optional = OPTIONAL_CALL_RE.test(e.raw);
    const defaultMatch = DEFAULT_CALL_RE.exec(e.raw);
    const hasDefault = defaultMatch !== null;
    return {
      name: e.name,
      line: bodyStartLine + e.lineIdx,
      isSpread: false,
      optional,
      required: !(optional || hasDefault),
      default: hasDefault ? defaultMatch[1].trim() : null,
    };
  });
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

/**
 * Pure/testable: given a schema field-name array from BEFORE and AFTER a source change, return
 * which names were added/removed — the map-coverage gate's UNMAPPED/ORPHAN concept (§5) applied
 * to this one namespace's own re-runs, so drift (a var added to or removed from EnvSchema) is a
 * diff against the last count, not a full manual re-read of the source.
 */
export function diffFieldNames(beforeNames, afterNames) {
  const beforeSet = new Set(beforeNames);
  const afterSet = new Set(afterNames);
  return {
    added: afterNames.filter((n) => !beforeSet.has(n)),
    removed: beforeNames.filter((n) => !afterSet.has(n)),
  };
}

/**
 * Pure/testable: classify every name seen in EITHER the schema field list OR the raw
 * process.env name list as `source: 'EnvSchema' | 'raw' | 'both'`. `shadow` is true only for
 * `'both'` — a raw read that duplicates a validated schema key (see SHADOW CLASSIFICATION in
 * the file header). Spread placeholder names (`...expr`) are excluded from the schema side —
 * their real key name(s) are unresolvable here, so they can never be matched against raw reads.
 */
export function classifyVarSources(schemaNames, rawNames) {
  const schemaSet = new Set(schemaNames.filter((n) => !n.startsWith('...')));
  const rawSet = new Set(rawNames);
  const allNames = Array.from(new Set([...schemaSet, ...rawSet])).sort();
  return allNames.map((name) => {
    const inSchema = schemaSet.has(name);
    const inRaw = rawSet.has(name);
    return {
      name,
      source: inSchema && inRaw ? 'both' : inSchema ? 'EnvSchema' : 'raw',
      shadow: inSchema && inRaw,
    };
  });
}

/**
 * Merged per-var census row: name, source, required/default, and a representative file:line
 * (schema location wins when present, else the first raw occurrence) — the exact shape
 * REBUILD-MAP §7 item 4 asks for ("name, source (EnvSchema|raw|both), optional/required,
 * default, file:line"). Not part of the {ns,id,file,line} extract-all.mjs pipeline (that stays
 * schema-record + raw-record granularity for backward compatibility with map-coverage.mjs /
 * seed-traceability.mjs / verify-counts.mjs) — call this directly, or run this file with
 * `--census`, for the enriched report.
 */
export function buildVarCensus({ schemaDetails, rawHits, schemaFile }) {
  const detailsByName = new Map(schemaDetails.filter((d) => !d.isSpread).map((d) => [d.name, d]));
  const rawByName = new Map();
  for (const h of rawHits) {
    if (!rawByName.has(h.name)) rawByName.set(h.name, h); // first occurrence wins
  }
  const schemaNames = schemaDetails.map((d) => d.name);
  const rawNames = [...rawByName.keys()];
  const classified = classifyVarSources(schemaNames, rawNames);

  return classified.map(({ name, source, shadow }) => {
    const detail = detailsByName.get(name);
    const rawHit = rawByName.get(name);
    return {
      name,
      source,
      shadow,
      required: detail ? detail.required : null,
      default: detail ? detail.default : null,
      file: detail ? schemaFile : rawHit.file,
      line: detail ? detail.line : rawHit.line,
    };
  });
}

/** Shared raw-scan: walk the 4 raw-env dirs, dedupe (ENV-RAW-<NAME>, first occurrence wins). */
function scanRawEnvReads() {
  const rawDirs = ['apps/api/src', 'apps/worker/src', 'packages/db/src', 'packages/config/src'];
  const rawFiles = rawDirs.flatMap((d) => walkFiles(d, ['.ts', '.tsx']));
  const hits = [];
  for (const f of rawFiles) {
    for (const h of parseRawProcessEnvFromFile(readRepoFile(f))) {
      hits.push({ ns: 'server-flags-envs', id: `ENV-RAW-${idSafe(h.name)}`, name: h.name, file: f, line: h.line });
    }
  }
  return dedupeById(stableSort(hits));
}

export async function extract() {
  const schemaContent = readRepoFile(SCHEMA_FILE);
  const details = parseEnvSchemaFieldDetails(schemaContent);
  const schemaNameSet = new Set(details.filter((d) => !d.isSpread).map((d) => d.name));

  const schemaRecords = details.map((d) => ({
    ns: 'server-flags-envs',
    id: d.isSpread ? `ENV-SPREAD-${idSafe(d.name)}` : `ENV-${idSafe(d.name)}`,
    file: SCHEMA_FILE,
    line: d.line,
    source: 'EnvSchema',
    optional: d.optional,
    required: d.required,
    default: d.default,
  }));

  const rawHits = scanRawEnvReads();
  const rawRecords = rawHits.map((h) => ({
    ns: h.ns,
    id: h.id,
    file: h.file,
    line: h.line,
    source: 'raw',
    shadow: schemaNameSet.has(h.name),
  }));

  return stableSort([...schemaRecords, ...rawRecords]);
}

/** Merged per-var census against the real tree — used by `--census` and by ad hoc reporting. */
export async function extractVarCensus() {
  const schemaContent = readRepoFile(SCHEMA_FILE);
  const schemaDetails = parseEnvSchemaFieldDetails(schemaContent);
  const rawHits = scanRawEnvReads().map((h) => ({ name: h.name, file: h.file, line: h.line }));
  return buildVarCensus({ schemaDetails, rawHits, schemaFile: SCHEMA_FILE });
}

if (isMain(import.meta.url)) {
  if (process.argv.includes('--census')) {
    const rows = await extractVarCensus();
    for (const r of rows) process.stdout.write(JSON.stringify(r) + '\n');
  } else {
    const records = await extract();
    printRecords(records);
  }
}
