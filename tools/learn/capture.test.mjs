// node --test tools/learn/capture.test.mjs -- the recorder's rules, without a browser or a venue.
import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, mkdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { parseArgs, checkHost, lessonFile, loadLesson, guard, localAsset, typeOf, redact, plan, mark, closeMarks, stepFound, sourceSha,
  DEFAULT_HOST, APPS, REPO } from './capture-lib.mjs';

test('parseArgs: defaults to the three languages, the operator venue and the local UI', () => {
  const o = parseArgs(['W3', '--out', '/x']);
  assert.deepEqual([o.id, o.langs, o.host, o.hostGiven, o.ui, o.out, o.desktop, o.dryRun], ['W3', ['sq', 'en', 'uk'], DEFAULT_HOST, false, 'local', '/x', false, false]);
  const p = parseArgs(['--lesson', 'f.yaml', '--out', 'o', '--lang', 'uk', '--ui', 'live', '--host', 'https://a.b/', '--desktop', '--dry-run']);
  assert.deepEqual([p.lesson, p.langs, p.ui, p.host, p.hostGiven, p.desktop, p.dryRun], ['f.yaml', ['uk'], 'live', 'https://a.b', true, true, true]);
  assert.equal(parseArgs(['W3', '--out', 'o', '--allow-writes']).allowWrites, true);
  assert.equal(o.allowWrites, false);
});

test('parseArgs: every bad input is named', () => {
  assert.match(parseArgs([]).error, /usage/);
  assert.match(parseArgs(['W3']).error, /--out/);
  assert.match(parseArgs(['W3', '--out', 'o', '--ui', 'x']).error, /--ui/);
  assert.match(parseArgs(['W3', '--out', 'o', '--lang', 'de']).error, /de/);
  assert.match(parseArgs(['W3', '--out', 'o', '--lang', '']).error, /nothing/);
  assert.match(parseArgs(['W3', '--out', 'o', '--bogus']).error, /unknown flag --bogus/);
  assert.match(parseArgs(['W3', 'W4', '--out', 'o']).error, /unexpected argument W4/);
});

test('checkHost: dubin-sushi passes; another host is refused unless --host named it', () => {
  assert.equal(checkHost(DEFAULT_HOST, false), null);
  assert.match(checkHost('https://sushi-durres.dowiz.org', false), /refusing .* unless --host/);
  assert.equal(checkHost('https://sushi-durres.dowiz.org', true), null);
  assert.equal(checkHost('http://127.0.0.1:8787', true), null);
  assert.match(checkHost('http://example.org', true), /https only/);
  assert.match(checkHost('nope', true), /not a URL/);
  assert.match(checkHost('https://a.b/room/', true), /origin/);
});

test('lessonFile + loadLesson: a real lesson loads with the build rules; a broken one is refused by name', () => {
  const f = lessonFile('W3');
  assert.ok(f.endsWith('docs/learn/lessons/waiter/W3.yaml'));
  const { lesson } = loadLesson(f);
  assert.equal(lesson.id, 'W3'); assert.equal(lesson.role, 'waiter');
  assert.equal(lesson.steps[0].action.selector, '[data-tour="room.open"]');
  assert.equal(lessonFile('Z99'), null);
  const d = mkdtempSync(join(tmpdir(), 'cap-')); mkdirSync(join(d, 'waiter'));
  writeFileSync(join(d, 'waiter', 'W3.yaml'), readFileSync(f, 'utf8').replace('do: click', 'do: dance'));
  assert.match(loadLesson(join(d, 'waiter', 'W3.yaml'), d).errors.join(), /dance/);
  writeFileSync(join(d, 'waiter', 'W9.yaml'), 'id: [unclosed');
  assert.ok(loadLesson(join(d, 'waiter', 'W9.yaml'), d).errors.length);
});

test('guard: nothing writes by default; with --allow-writes only a writing step does; reads and sign-in pass', () => {
  const ro = { writes: false }, rw = { writes: true };
  assert.equal(guard(rw, 'POST', 'https://h/api/owner/campaigns/1/send'), 'block');
  assert.equal(guard(ro, 'POST', 'https://h/api/x', true), 'block');
  assert.equal(guard(ro, 'POST', 'https://h/api/owner/orders/1/action'), 'block');
  assert.equal(guard(ro, 'DELETE', 'https://h/api/x'), 'block');
  assert.equal(guard(null, 'PUT', 'https://h/api/x'), 'block');
  assert.equal(guard(rw, 'POST', 'https://h/api/public/locations/x/orders', true), 'record');
  assert.equal(guard(ro, 'GET', 'https://h/api/staff/room'), 'allow');
  assert.equal(guard(ro, 'POST', 'https://h/api/auth/refresh'), 'allow');
  assert.equal(guard(ro, 'POST', 'https://h/api/staff/login'), 'allow');
  assert.equal(guard(ro, 'POST', 'https://h/room/app.js'), 'allow');
  assert.equal(guard(ro, 'POST', 'not a url'), 'allow');
});

test('localAsset: public files from the tree, never the API, never outside public/', () => {
  const pub = join(REPO, 'workers/api/public');
  assert.equal(localAsset('/room/', pub), join(pub, 'room/index.html'));
  assert.equal(localAsset('/room/app.js?v=2', pub), join(pub, 'room/app.js'));
  assert.equal(localAsset('/', pub), join(pub, 'store/index.html'));
  assert.equal(localAsset('/api/staff/room', pub), null);
  assert.equal(localAsset('/../../etc/passwd', pub), null);
  assert.equal(localAsset('/%2e%2e/%2e%2e/etc/passwd', pub), null);
  assert.equal(localAsset('/room/nope.js', pub), null);
  assert.equal(localAsset('/room', pub), null);          // a directory is not a file
});

test('typeOf: known types and a safe default', () => {
  assert.match(typeOf('a.js'), /javascript/); assert.match(typeOf('a.HTML'), /text\/html/);
  assert.equal(typeOf('a.bin'), 'application/octet-stream');
});

test('redact: no token survives a log line', () => {
  const s = redact('Bearer abc.def eyJhbGciOi.eyJzdWIi.sig {"password":"hunter2","jwt":"x"}');
  assert.doesNotMatch(s, /abc\.def|eyJ|hunter2|"x"/);
  assert.match(s, /<token>.*<jwt>.*<redacted>/);
});

test('plan: pending steps are skipped with the reason; the rest drive their action', () => {
  const lesson = { steps: [
    { n: 1, key: '1', anchor: 'a.b', writes: false, pending: false, action: { do: 'type', selector: '[data-tour="a.b"]', value: 'v' } },
    { n: 2, key: '2', anchor: 'c.d', writes: true, pending: true, action: { do: 'click', selector: '[data-tour="c.d"]' } }] };
  const p = plan(lesson);
  assert.deepEqual([p[0].do, p[0].value, p[0].why], ['type', 'v', null]);
  assert.deepEqual([p[1].do, p[1].writes, p[1].value], ['skip', true, null]);
  assert.match(p[1].why, /pending/);
});

test('mark + closeMarks: each step ends where the next begins, never before it starts', () => {
  const m = [mark(1, '1', 100.4, true), mark(2, '2', -5, false, ['POST /api/x'])];
  assert.deepEqual([m[0].startMs, m[1].startMs, m[1].blocked], [100, 0, ['POST /api/x']]);
  const c = closeMarks([mark(1, '1', 100, true), mark(2, '2', 900, true)], 2000);
  assert.deepEqual(c.map(x => x.endMs), [900, 2000]);
  assert.equal(closeMarks([mark(1, '1', 500, true)], 400)[0].endMs, 501);
});

test('APPS: every recordable role names its app, language key and ready selector', () => {
  for (const r of ['owner', 'waiter', 'courier', 'guest']) assert.ok(APPS[r].path && APPS[r].langKey && APPS[r].ready);
});

test('stepFound: a card-only step has nothing to find; a pending one never is; an anchored one when its element was', () => {
  assert.equal(stepFound({ do: 'wait', selector: null }, null), true);
  assert.equal(stepFound({ do: 'skip', selector: '[data-tour="a.b"]' }, {}), false);
  assert.equal(stepFound({ do: 'click', selector: '[data-tour="a.b"]' }, null), false);
  assert.equal(stepFound({ do: 'click', selector: '[data-tour="a.b"]' }, {}), true);
});

test('sourceSha: the YAML bytes, so any edit moves it', () => {
  const d = mkdtempSync(join(tmpdir(), 'sha-'));
  writeFileSync(join(d, 'a.yaml'), 'id: A1\n');
  const h = sourceSha(join(d, 'a.yaml'));
  assert.match(h, /^[0-9a-f]{64}$/);
  writeFileSync(join(d, 'a.yaml'), 'id: A2\n');
  assert.notEqual(sourceSha(join(d, 'a.yaml')), h);
});

test('parseArgs + loadLesson: a bare --host is an empty origin (refused by checkHost); a file outside the root keeps its path', () => {
  const o = parseArgs(['W3', '--out', 'o', '--host']);
  assert.equal(o.host, ''); assert.equal(o.hostGiven, true);
  assert.match(checkHost(o.host, o.hostGiven), /not a URL/);
  const { lesson } = loadLesson(lessonFile('W3'), '/nowhere');
  assert.equal(lesson.id, 'W3');
});
