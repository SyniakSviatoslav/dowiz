// verify.mjs against a stub Worker in THIS process (no second process, nothing to kill):
// the right commit + identical assets pass; a different commit, a changed asset, a missing
// route and a closed port each FAIL by name.   node tools/deploy/verify.test.mjs
import assert from 'node:assert/strict';
import fs from 'node:fs';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import { verify } from './verify.mjs';

const HEAD = 'c0262881' + 'ab'.repeat(16);
const OTHER = 'deadbeef' + 'cd'.repeat(16);
const src = fs.mkdtempSync(path.join(os.tmpdir(), 'verify-test-'));
const files = ['admin/app.js', 'admin/core.js', 'admin/i18n.js'];
for (const f of files) {
  fs.mkdirSync(path.join(src, 'workers/api/public/admin'), { recursive: true });
  fs.writeFileSync(path.join(src, 'workers/api/public', f), `// ${f}\nexport const x = 1;\n`);
}

// What the stub serves; each case edits it.
let live;
const reset = () => { live = { commit: HEAD, version: 200, assets: Object.fromEntries(files.map(f => [f, fs.readFileSync(path.join(src, 'workers/api/public', f))])) }; };
const server = http.createServer((req, res) => {
  const p = new URL(req.url, 'http://x').pathname;
  if (p === '/api/version') {
    res.writeHead(live.version, { 'content-type': 'application/json' });
    return res.end(live.version === 200 ? JSON.stringify({ commit: live.commit, built_at: '2026-09-30T12:00:00Z' }) : 'no route');
  }
  const a = live.assets[p.slice(1)];
  if (a) { res.writeHead(200); return res.end(a); }
  res.writeHead(404); res.end('nf');
});
await new Promise(r => server.listen(0, '127.0.0.1', r));
const host = `http://127.0.0.1:${server.address().port}`;
const run = () => verify({ host, commit: HEAD, src, files });
const failed = res => res.lines.filter(l => l.startsWith('FAIL')).map(l => l.split(' :: ')[0].slice(5));

let n = 0;
const test = async (name, fn) => { reset(); await fn(); n++; console.log(`ok ${name}`); };

await test('the right commit and identical assets pass', async () => {
  const r = await run();
  assert.equal(r.ok, true, r.lines.join('\n'));
  assert.equal(r.lines.length, 4);
});
await test('a different live commit FAILs loudly, naming both', async () => {
  live.commit = OTHER;
  const r = await run();
  assert.equal(r.ok, false);
  assert.deepEqual(failed(r), ['version-commit']);
  assert.match(r.lines[0], new RegExp(`live=${OTHER} head=${HEAD}`));
});
await test('an unknown build (hand deploy) FAILs', async () => {
  live.commit = 'unknown';
  assert.deepEqual(failed(await run()), ['version-commit']);
});
await test('one changed admin byte FAILs that asset only', async () => {
  live.assets['admin/core.js'] = Buffer.concat([live.assets['admin/core.js'], Buffer.from(' ')]);
  assert.deepEqual(failed(await run()), ['asset admin/core.js']);
});
await test('a Worker without /api/version FAILs as version-route', async () => {
  live.version = 404;
  assert.deepEqual(failed(await run()), ['version-route']);
});
await test('an asset missing from the HEAD copy FAILs, not skips', async () => {
  const r = await verify({ host, commit: HEAD, src, files: [...files, 'admin/nope.js'] });
  assert.deepEqual(failed(r), ['asset admin/nope.js']);
});
server.close();
await test('an unreachable host FAILs every check', async () => {
  const r = await verify({ host: 'http://127.0.0.1:1', commit: HEAD, src, files });
  assert.equal(failed(r).length, 4);
});
fs.rmSync(src, { recursive: true, force: true });
console.log(`verify.test: ${n} passed`);
