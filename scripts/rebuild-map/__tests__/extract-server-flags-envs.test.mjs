// scripts/rebuild-map/__tests__/extract-server-flags-envs.test.mjs
//
// Unit tests for the trickiest bit of extract-server-flags-envs.mjs: parsing the Zod
// EnvSchema field count PROGRAMMATICALLY (brace-depth tracking), not via a flat line regex
// — REBUILD-MAP §7 item 4 explicitly calls out that the manual/naive count (80, later found
// live-119) "must die". The regression this guards against: a flat `^\s*NAME: z\.` line
// regex over the WHOLE file would also match keys nested inside z.object({...}) sub-schemas,
// z.array(z.object({...})) items, or a commented-out block — inflating the count. This
// extractor must count ONLY the top-level keys of the named EnvSchema object.
//
// Extended (REBUILD-MAP §7 item 4, second pass): fixtures for spread/renamed Zod shapes,
// per-field optionality/default resolution, added/removed-var drift detection, source
// classification (EnvSchema|raw|both) + shadow, and one intentional LIVE-file regression test
// pinning today's real counts (119 schema / 67 raw / 43 shadow) — everything else in this file
// stays fixture-string-only per the README's stated convention; this one test is a deliberate
// exception, the same role `verify-counts.mjs` plays at the whole-pipeline level but pinned at
// the unit-test tripwire level for this one namespace.
//
// Run: node --test scripts/rebuild-map/__tests__/extract-server-flags-envs.test.mjs

import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  parseEnvSchemaFields,
  parseEnvSchemaFieldDetails,
  parseRawProcessEnvFromFile,
  diffFieldNames,
  classifyVarSources,
  buildVarCensus,
  extract,
  extractVarCensus,
} from '../extract-server-flags-envs.mjs';
import { readRepoFile } from '../lib/common.mjs';

test('counts simple top-level fields', () => {
  const fixture = `
import { z } from 'zod';
const EnvSchema = z.object({
  NODE_ENV: z.enum(['development', 'test', 'production']),
  PORT: z.coerce.number().int().positive().default(8080),
  APP_BASE_URL: z.string().url(),
});
`;
  const fields = parseEnvSchemaFields(fixture);
  assert.deepEqual(fields, ['NODE_ENV', 'PORT', 'APP_BASE_URL']);
});

test('counts a nested-object field itself once, but NOT its inner keys (the naive-regex trap)', () => {
  // `NESTED` is a genuine top-level EnvSchema field (its value happens to be a nested
  // z.object) -- it must be counted once. Its INNER keys (SHOULD_NOT_COUNT_*) belong to
  // that sub-schema, not EnvSchema, and must never inflate the top-level count -- a flat
  // `^\s*NAME: z\.` line-regex over the whole file would wrongly count all 4 keys here.
  const fixture = `
const EnvSchema = z.object({
  TOP_LEVEL_A: z.string(),
  NESTED: z.object({
    SHOULD_NOT_COUNT_1: z.string(),
    SHOULD_NOT_COUNT_2: z.string(),
  }),
  TOP_LEVEL_B: z.string(),
});
`;
  const fields = parseEnvSchemaFields(fixture);
  assert.deepEqual(fields, ['TOP_LEVEL_A', 'NESTED', 'TOP_LEVEL_B']);
  assert.equal(fields.length, 3);
});

test('does NOT count keys inside a z.array(z.object({...})) item schema', () => {
  const fixture = `
const EnvSchema = z.object({
  A: z.string(),
  LIST: z.array(z.object({
    INNER_1: z.string(),
    INNER_2: z.string(),
  })),
  B: z.string(),
});
`;
  const fields = parseEnvSchemaFields(fixture);
  assert.deepEqual(fields, ['A', 'LIST', 'B']);
});

test('handles a multi-line field value (parens spanning lines) without losing depth tracking', () => {
  const fixture = `
const EnvSchema = z.object({
  A: z.string(),
  MULTILINE: z
    .string()
    .optional(),
  B: z.string(),
});
`;
  const fields = parseEnvSchemaFields(fixture);
  assert.deepEqual(fields, ['A', 'MULTILINE', 'B']);
});

test('returns empty array when no EnvSchema declaration is found', () => {
  const fields = parseEnvSchemaFields('const SomethingElse = z.object({ X: z.string() });');
  assert.deepEqual(fields, []);
});

test('field order is preserved (source order), for deterministic downstream ids', () => {
  const fixture = `
const EnvSchema = z.object({
  ZEBRA: z.string(),
  ALPHA: z.string(),
  MIDDLE: z.string(),
});
`;
  const fields = parseEnvSchemaFields(fixture);
  assert.deepEqual(fields, ['ZEBRA', 'ALPHA', 'MIDDLE']);
});

test('raw process.env parser finds every occurrence with line numbers', () => {
  const fixture = `
const a = process.env.FOO;
function f() {
  return process.env.BAR || process.env.BAZ;
}
`;
  const hits = parseRawProcessEnvFromFile(fixture);
  assert.deepEqual(
    hits.map((h) => h.name),
    ['FOO', 'BAR', 'BAZ'],
  );
  assert.equal(hits[0].line, 2);
  assert.equal(hits[1].line, 4);
});

// ---------------------------------------------------------------------------------------
// Spread / renamed Zod shapes (REBUILD-MAP §7 item 4: "robust source-level parsing of the
// Zod object shape")
// ---------------------------------------------------------------------------------------

test('a "renamed" field (value is a reference into ANOTHER schema, not a fresh z.builder) is still counted', () => {
  // Legal Zod: reusing a sub-schema's field under a new top-level key name, e.g.
  // `RENAMED: OtherSchema.shape.ORIGINAL`. The OLD FIELD_KEY_RE required a literal `z` token
  // right after the colon and would have silently dropped this — the fix broadens the match to
  // any non-whitespace value at depth 1.
  const fixture = `
const BaseSchema = z.object({ ORIGINAL: z.string() });
const EnvSchema = z.object({
  A: z.string(),
  RENAMED: BaseSchema.shape.ORIGINAL,
  B: z.string(),
});
`;
  const fields = parseEnvSchemaFields(fixture);
  assert.deepEqual(fields, ['A', 'RENAMED', 'B']);
});

test('a spread composition line (`...OtherSchema.shape`) surfaces as ONE opaque placeholder, not a silent drop', () => {
  const fixture = `
const EnvSchema = z.object({
  A: z.string(),
  ...BaseSchema.shape,
  B: z.string(),
});
`;
  const fields = parseEnvSchemaFields(fixture);
  // 3 entries: A, the spread placeholder, B — NOT 2 (which would mean the spread line was
  // invisible) and not an attempt to guess how many keys the spread injects.
  assert.equal(fields.length, 3);
  assert.equal(fields[0], 'A');
  assert.match(fields[1], /^\.\.\./);
  assert.equal(fields[1], '...BaseSchema.shape');
  assert.equal(fields[2], 'B');
});

test('a spread placeholder is excluded from classifyVarSources (its real key names are unresolvable)', () => {
  const schemaNames = ['A', '...BaseSchema.shape', 'B'];
  const rawNames = ['A'];
  const classified = classifyVarSources(schemaNames, rawNames);
  const names = classified.map((c) => c.name);
  assert.ok(!names.includes('...BaseSchema.shape'));
  assert.deepEqual(names.sort(), ['A', 'B']);
});

// ---------------------------------------------------------------------------------------
// Per-field optionality / default resolution (parseEnvSchemaFieldDetails)
// ---------------------------------------------------------------------------------------

test('parseEnvSchemaFieldDetails: a field with neither .optional() nor .default(...) is required', () => {
  const fixture = `
const EnvSchema = z.object({
  NODE_ENV: z.enum(['development', 'production']),
});
`;
  const [detail] = parseEnvSchemaFieldDetails(fixture);
  assert.equal(detail.name, 'NODE_ENV');
  assert.equal(detail.required, true);
  assert.equal(detail.optional, false);
  assert.equal(detail.default, null);
});

test('parseEnvSchemaFieldDetails: .optional() marks a field not-required, with no default value', () => {
  const fixture = `
const EnvSchema = z.object({
  DEV_AUTH_SECRET: z.string().optional(),
});
`;
  const [detail] = parseEnvSchemaFieldDetails(fixture);
  assert.equal(detail.optional, true);
  assert.equal(detail.required, false);
  assert.equal(detail.default, null);
});

test('parseEnvSchemaFieldDetails: .default(X) marks a field not-required and captures X verbatim', () => {
  const fixture = `
const EnvSchema = z.object({
  PORT: z.coerce.number().int().positive().default(8080),
  LOG_LEVEL: z.enum(['info', 'debug']).default('info'),
});
`;
  const [port, logLevel] = parseEnvSchemaFieldDetails(fixture);
  assert.equal(port.required, false);
  assert.equal(port.default, '8080');
  assert.equal(logLevel.required, false);
  assert.equal(logLevel.default, "'info'");
});

test('parseEnvSchemaFieldDetails: resolves across a multi-line value span (default on a later line)', () => {
  const fixture = `
const EnvSchema = z.object({
  MULTILINE: z
    .string()
    .default('fallback'),
});
`;
  const [detail] = parseEnvSchemaFieldDetails(fixture);
  assert.equal(detail.required, false);
  assert.equal(detail.default, "'fallback'");
});

test('parseEnvSchemaFieldDetails: absolute 1-based file line numbers account for lines before the schema declaration', () => {
  const fixture = `import { z } from 'zod';
// a leading comment
const EnvSchema = z.object({
  FIRST: z.string(),
  SECOND: z.string(),
});
`;
  const details = parseEnvSchemaFieldDetails(fixture);
  assert.equal(details.find((d) => d.name === 'FIRST').line, 4);
  assert.equal(details.find((d) => d.name === 'SECOND').line, 5);
});

test('parseEnvSchemaFieldDetails: a spread entry resolves optional/required/default all to null (unresolvable)', () => {
  const fixture = `
const EnvSchema = z.object({
  ...BaseSchema.shape,
  A: z.string(),
});
`;
  const [spreadEntry] = parseEnvSchemaFieldDetails(fixture);
  assert.equal(spreadEntry.isSpread, true);
  assert.equal(spreadEntry.optional, null);
  assert.equal(spreadEntry.required, null);
  assert.equal(spreadEntry.default, null);
});

// ---------------------------------------------------------------------------------------
// Added/removed detection (diffFieldNames) — REBUILD-MAP §5 UNMAPPED/ORPHAN concept applied
// to one namespace's own re-runs, so schema drift is a diff, not a full manual re-count.
// ---------------------------------------------------------------------------------------

test('diffFieldNames: detects a var added to EnvSchema', () => {
  const before = ['NODE_ENV', 'PORT'];
  const after = ['NODE_ENV', 'PORT', 'NEW_FLAG'];
  const { added, removed } = diffFieldNames(before, after);
  assert.deepEqual(added, ['NEW_FLAG']);
  assert.deepEqual(removed, []);
});

test('diffFieldNames: detects a var removed from EnvSchema', () => {
  const before = ['NODE_ENV', 'PORT', 'DEPRECATED_FLAG'];
  const after = ['NODE_ENV', 'PORT'];
  const { added, removed } = diffFieldNames(before, after);
  assert.deepEqual(added, []);
  assert.deepEqual(removed, ['DEPRECATED_FLAG']);
});

test('diffFieldNames: simultaneous add + remove (a rename-in-place) reports both, order-preserving', () => {
  const before = ['NODE_ENV', 'OLD_NAME', 'PORT'];
  const after = ['NODE_ENV', 'PORT', 'NEW_NAME'];
  const { added, removed } = diffFieldNames(before, after);
  assert.deepEqual(added, ['NEW_NAME']);
  assert.deepEqual(removed, ['OLD_NAME']);
});

// ---------------------------------------------------------------------------------------
// Source classification + shadow (classifyVarSources / buildVarCensus) — REBUILD-MAP §7
// item 4: "Shadow vars (raw read of a var also in EnvSchema) must be classified."
// ---------------------------------------------------------------------------------------

test('classifyVarSources: EnvSchema-only, raw-only, and both (shadow) are classified correctly', () => {
  const schemaNames = ['NODE_ENV', 'APP_BASE_URL'];
  const rawNames = ['APP_BASE_URL', 'METRICS_TOKEN'];
  const rows = classifyVarSources(schemaNames, rawNames);
  const byName = Object.fromEntries(rows.map((r) => [r.name, r]));

  assert.equal(byName.NODE_ENV.source, 'EnvSchema');
  assert.equal(byName.NODE_ENV.shadow, false);

  assert.equal(byName.APP_BASE_URL.source, 'both');
  assert.equal(byName.APP_BASE_URL.shadow, true); // validated AND re-read raw — a duplication

  assert.equal(byName.METRICS_TOKEN.source, 'raw');
  assert.equal(byName.METRICS_TOKEN.shadow, false); // genuinely unvalidated at boot
});

test('buildVarCensus: merges schema details + raw hits into one row per name with file:line', () => {
  const schemaDetails = [
    { name: 'NODE_ENV', line: 4, isSpread: false, optional: false, required: true, default: null },
    { name: 'PORT', line: 5, isSpread: false, optional: false, required: false, default: '8080' },
  ];
  const rawHits = [
    { name: 'PORT', file: 'apps/api/src/server.ts', line: 12 },
    { name: 'METRICS_TOKEN', file: 'apps/api/src/lib/metrics.ts', line: 135 },
  ];
  const rows = buildVarCensus({ schemaDetails, rawHits, schemaFile: 'packages/config/src/index.ts' });
  const byName = Object.fromEntries(rows.map((r) => [r.name, r]));

  assert.equal(byName.NODE_ENV.source, 'EnvSchema');
  assert.equal(byName.NODE_ENV.file, 'packages/config/src/index.ts');
  assert.equal(byName.NODE_ENV.line, 4);

  // PORT is 'both' — the census's representative file:line prefers the SCHEMA location
  // (validated source of truth) over the raw call site, even though a raw hit also exists.
  assert.equal(byName.PORT.source, 'both');
  assert.equal(byName.PORT.shadow, true);
  assert.equal(byName.PORT.file, 'packages/config/src/index.ts');
  assert.equal(byName.PORT.default, '8080');

  assert.equal(byName.METRICS_TOKEN.source, 'raw');
  assert.equal(byName.METRICS_TOKEN.file, 'apps/api/src/lib/metrics.ts');
  assert.equal(byName.METRICS_TOKEN.required, null); // unknown — no Zod validation to inspect
});

// ---------------------------------------------------------------------------------------
// LIVE-file regression pin (intentional exception to the fixture-string-only convention —
// see file header). Pins today's real counts so a future EnvSchema edit that silently changes
// the census shows up as a failing test, not a narrated surprise three months later.
// ---------------------------------------------------------------------------------------

test('REGRESSION: today\'s real packages/config/src/index.ts has exactly 119 top-level EnvSchema fields', () => {
  const content = readRepoFile('packages/config/src/index.ts');
  const fields = parseEnvSchemaFields(content);
  assert.equal(fields.length, 119);
  assert.equal(fields[0], 'NODE_ENV');
  assert.equal(fields.at(-1), 'ACCESS_REQUEST_NOTIFY_MAX_ATTEMPTS');
});

// Pin history: 67 raw / 24 raw-only / 143 unique at authoring; +1 each when the TMA channel
// adapter landed its dark server flag as a raw read (apps/api/src/routes/telegram-webhook.ts
// TMA_ENABLED — the EnvSchema entry is operator-gated, so it counts raw-only until that lands,
// at which point these pins move to both/144-with-43→44).
test('REGRESSION: extract() against the real tree pins 119 schema / 68 raw / 43 shadow / 25 raw-only records', async () => {
  const records = await extract();
  const schema = records.filter((r) => r.source === 'EnvSchema');
  const raw = records.filter((r) => r.source === 'raw');
  const shadow = raw.filter((r) => r.shadow);

  assert.equal(schema.length, 119);
  assert.equal(raw.length, 68);
  assert.equal(shadow.length, 43);
  assert.equal(raw.length - shadow.length, 25);
});

test('REGRESSION: extractVarCensus() against the real tree pins 144 unique vars (76 EnvSchema-only + 25 raw-only + 43 both)', async () => {
  const rows = await extractVarCensus();
  const bySource = { EnvSchema: 0, raw: 0, both: 0 };
  for (const r of rows) bySource[r.source]++;

  assert.equal(rows.length, 144);
  assert.equal(bySource.EnvSchema, 76);
  assert.equal(bySource.raw, 25);
  assert.equal(bySource.both, 43);
});
