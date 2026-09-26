import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { readCreds, timedGet, getJson, ownerToken, slugOf, key, CREDS } from './net.mjs';

test('credentials: file lines, environment first, a missing file is empty', () => {
  const d = fs.mkdtempSync(path.join(os.tmpdir(), 'evals-net-'));
  const f = path.join(d, 'owner');
  fs.writeFileSync(f, 'export OWNER_EMAIL=a@b\nexport OWNER_PASSWORD=p=q\n# c\nPLAIN=x\n');
  assert.deepEqual(readCreds(f, {}), { OWNER_EMAIL: 'a@b', OWNER_PASSWORD: 'p=q' });
  assert.deepEqual(readCreds(path.join(d, 'none'), { OWNER_EMAIL: 'e', OWNER_PASSWORD: 'w' }), { OWNER_EMAIL: 'e', OWNER_PASSWORD: 'w' });
  assert.deepEqual(readCreds(path.join(d, 'none'), {}), {});
  assert.equal(typeof CREDS, 'string');
});

test('timedGet times the headers and the body with the injected clock', async () => {
  let t = 0;
  const clock = () => (t += 10);
  const f = async (u, o) => {
    assert.equal(o.redirect, 'manual');
    return new Response('hello', { status: 200, headers: { 'cf-cache-status': 'HIT', 'cache-control': 'max-age=60' } });
  };
  const r = await timedGet(f, 'https://h/x', {}, clock);
  assert.deepEqual({ ...r, body: r.body.toString() }, { status: 200, ttfb: 10, total: 20, bytes: 5, cache: 'HIT', cacheControl: 'max-age=60', body: 'hello' });
  const bare = await timedGet(async () => new Response('', { status: 404 }), 'u');
  assert.equal(bare.cache, '');
  assert.equal(bare.cacheControl, '');
});

test('getJson sends the bearer, parses json, and keeps a non-json body as text', async () => {
  const seen = [];
  const f = async (u, o) => { seen.push(o.headers); return new Response(u.endsWith('j') ? '{"a":1}' : 'nope', { status: 200 }); };
  assert.deepEqual(await getJson(f, 'https://h/j', 'T'), { status: 200, json: { a: 1 }, text: '{"a":1}' });
  assert.deepEqual(await getJson(f, 'https://h/t'), { status: 200, json: null, text: 'nope' });
  assert.deepEqual(seen, [{ authorization: 'Bearer T' }, {}]);
});

test('ownerToken: no credentials, a refusal, and a token', async () => {
  assert.deepEqual(await ownerToken(null, 'h', {}), { token: null, why: 'no owner credentials' });
  const c = { OWNER_EMAIL: 'e', OWNER_PASSWORD: 'p' };
  const no = async () => new Response('not json', { status: 401 });
  assert.deepEqual(await ownerToken(no, 'https://h', c), { token: null, why: 'login answered 401' });
  const yes = async (u, o) => {
    assert.equal(u, 'https://h/api/auth/login');
    assert.deepEqual(JSON.parse(o.body), { email: 'e', password: 'p' });
    return new Response('{"access_token":"T"}');
  };
  assert.deepEqual(await ownerToken(yes, 'https://h', c), { token: 'T', why: '' });
});

test('slug and key', () => {
  assert.equal(slugOf('https://sushi-durres.dowiz.org'), 'sushi-durres');
  assert.equal(key('sushi-durres'), 'sushi_durres');
});
