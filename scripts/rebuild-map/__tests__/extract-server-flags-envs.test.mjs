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
// Run: node --test scripts/rebuild-map/__tests__/extract-server-flags-envs.test.mjs

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { parseEnvSchemaFields, parseRawProcessEnvFromFile } from '../extract-server-flags-envs.mjs';

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
