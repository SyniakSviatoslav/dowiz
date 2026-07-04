// scripts/rebuild-map/__tests__/extract-routes.test.mjs
//
// Unit tests for the trickiest bit of extract-routes.mjs: the fastify-registration regex
// must match the doc's exact grep semantics (inventory/10 §0) INCLUDING multi-line calls
// where the path literal is on a following line — a real, common pattern in this codebase
// (e.g. apps/api/src/routes/owner/menu-availability.ts) that a naive "quote right after the
// paren" regex silently drops (this was a real red->green bug found while building this
// extractor — see git history / the regex comment in extract-routes.mjs).
//
// Run: node --test scripts/rebuild-map/__tests__/extract-routes.test.mjs

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { parseRoutesFromFile } from '../extract-routes.mjs';

test('matches a single-line literal-path route call', () => {
  const fixture = `
export default async function routes(fastify) {
  fastify.get('/api/owner/products', async (req, reply) => {});
}
`;
  const records = parseRoutesFromFile(fixture, 'fake/products.ts');
  assert.equal(records.length, 1);
  assert.equal(records[0].ns, 'routes');
  assert.equal(records[0].line, 3);
  assert.match(records[0].id, /^ROUTE-GET-/);
});

test('matches a multi-line call where the path literal is on the NEXT line (the red->green case)', () => {
  const fixture = `
export default async function routes(server) {
  server.patch(
    '/api/owner/menu-availability/:id',
    { schema },
    async (req, reply) => {},
  );
}
`;
  const records = parseRoutesFromFile(fixture, 'fake/menu-availability.ts');
  assert.equal(records.length, 1);
  assert.equal(records[0].line, 3); // the call-opening line, matching the doc's grep line count
  assert.match(records[0].id, /^ROUTE-PATCH-/);
});

test('every recognized method verb is matched (get/post/put/patch/delete/all/head/options/route)', () => {
  const methods = ['get', 'post', 'put', 'patch', 'delete', 'all', 'head', 'options', 'route'];
  const fixture = methods.map((m) => `  fastify.${m}('/x', async () => {});`).join('\n');
  const records = parseRoutesFromFile(fixture, 'fake/all-methods.ts');
  assert.equal(records.length, methods.length);
});

test('does NOT match a call that is not anchored at line-start (per the doc grep semantics)', () => {
  const fixture = `  const x = 1; fastify.get('/inline-after-code', () => {});`;
  const records = parseRoutesFromFile(fixture, 'fake/inline.ts');
  assert.equal(records.length, 0);
});

test('does NOT match a commented-out route registration', () => {
  const fixture = `  // fastify.get('/dead', () => {});`;
  const records = parseRoutesFromFile(fixture, 'fake/commented.ts');
  assert.equal(records.length, 0);
});

test('two routes with the same method+path in different files get distinct, stable ids', () => {
  const fixture = `  fastify.get('/api/health', async () => {});`;
  const a = parseRoutesFromFile(fixture, 'fake/one.ts');
  const b = parseRoutesFromFile(fixture, 'fake/two.ts');
  assert.notEqual(a[0].id, b[0].id);
});

test('records are line-scoped with a stable {ns,id,file,line} shape', () => {
  const fixture = `fastify.post('/api/orders', async () => {});`;
  const [record] = parseRoutesFromFile(fixture, 'fake/orders.ts');
  assert.deepEqual(Object.keys(record).sort(), ['file', 'id', 'line', 'ns'].sort());
  assert.equal(record.ns, 'routes');
  assert.equal(record.file, 'fake/orders.ts');
});
