import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { tree } from './fixture.mjs';
import { stripComments, importsOf, htmlEntries, resolveSpec, bootGraph, weigh, walk } from './graph.mjs';
import { collect, shellList, shellMissing, unreferenced, pollValues, routeTable } from './static.mjs';

const byId = xs => Object.fromEntries(xs.map(x => [x.id, x]));

test('comments are not edges; static and dynamic imports are told apart', () => {
  assert.equal(stripComments("a // b\n/* c */d 'http://x'"), "a \nd 'http://x'");
  const i = importsOf("import a from './a.js';\nexport { b } from \"./b.js\";\nimport './c.js';\n// import './no.js'\nimport('./d.js');");
  assert.deepEqual(i, { stat: ['./a.js', './b.js', './c.js'], dyn: ['./d.js'] });
});

test('html entries: stylesheet, preload, modulepreload and scripts; not icons, not comments', () => {
  const h = '<link rel=preload href="/p.woff"><link rel="icon" href="/i.png"><link rel="stylesheet" href="/s.css"><link rel="stylesheet"><!-- <script src="/c.js"></script> --><script src="/a.js"></script><script>inline</script>';
  assert.deepEqual(htmlEntries(h), ['/p.woff', '/s.css', '/a.js']);
});

test('specifiers resolve against the served root or the importing file; urls are null', () => {
  assert.equal(resolveSpec('/pub', '/pub/a/b.js', '/x.js?v=1#h'), '/pub/x.js');
  assert.equal(resolveSpec('/pub', '/pub/a/b.js', '../y.js'), '/pub/y.js');
  for (const u of ['https://a/x.js', 'data:text/js,', '//cdn/x.js']) assert.equal(resolveSpec('/pub', '/pub/a.js', u), null);
});

test('a boot graph follows static imports, lists dynamic ones apart, and counts the missing', () => {
  const pub = path.join(tree(), 'workers/api/public');
  const g = bootGraph(pub, 'store/index.html');
  const rel = xs => xs.map(f => path.relative(pub, f));
  assert.deepEqual(rel(g.files), ['lib/ui.js', 'store/app.js', 'store/i18n.js', 'store/index.html', 'store/s.css', 'store/storage.js']);
  assert.deepEqual(rel(g.missing), ['lib/missing.js']);
  assert.deepEqual(rel(g.dynamic), ['store/later.js']);
  const w = weigh([path.join(pub, 'store/s.css')]);
  assert.equal(w.raw, 6);
  assert.ok(w.gzip > 0 && w.br > 0);
  assert.equal(walk(pub).length >= 20, true);
});

test('shell lists, what each shell misses, and dead files', () => {
  const root = tree();
  const pub = path.join(root, 'workers/api/public');
  assert.deepEqual(shellList(fs.readFileSync(path.join(pub, 'sw.js'), 'utf8')), ['/', '/store/app.js', '/admin/']);
  assert.deepEqual(shellMissing(pub, 'sw.js'), ['/admin/app.js', '/admin/core.js', '/lib/ui.js', '/store/i18n.js', '/store/s.css', '/store/storage.js']);
  assert.deepEqual(shellMissing(pub, 'room/sw.js'), ['/room/app.js']);
  assert.deepEqual(shellMissing(pub, 'courier/sw.js'), []);
  const dead = unreferenced(pub, path.join(root, 'workers/api/src'), []).map(f => path.relative(pub, f.path)).sort();
  assert.ok(dead.includes('orphan/dead.bin'));
  assert.ok(!dead.includes('named/used.txt'), 'named in lib.rs');
  assert.ok(!dead.includes('_headers') && !dead.includes('store/index.html'));
  assert.ok(!unreferenced(pub, path.join(root, 'workers/api/src'), [[path.join(pub, 'orphan/dead.bin')]]).some(f => f.path.endsWith('dead.bin')));
});

test('poll values read the constants; a missing one is null, not zero', () => {
  const root = tree({ 'workers/api/public/room/app.js': 'no constant here' });
  const v = Object.fromEntries(pollValues(path.join(root, 'workers/api/public')).map(([id, , n]) => [id, n]));
  assert.deepEqual(v, { 'poll.console_ms': 15000, 'poll.console_idle_ms': 60000, 'poll.room_ms': null, 'poll.track_ms': 12000,
    'poll.track_slow_ms': 60000, 'poll.courier_shift_ms': 12000, 'poll.courier_off_ms': 60000 });
  assert.deepEqual(routeTable(path.join(root, 'workers/api/src/lib.rs')), ['/api/a', '/api/b/:id']);
});

test('the static collector turns all of it into indicators', async () => {
  const r = byId(await collect({ root: tree() }));
  assert.equal(r['surfaces.store.files'].value, 6);
  assert.equal(r['surfaces.store.missing'].value, 1);
  assert.equal(r['surfaces.store.missing'].note, 'lib/missing.js');
  assert.equal(r['surfaces.admin.missing'].note, undefined);
  assert.equal(r['served.unreferenced_bytes'].rule, 'ratchet');
  assert.match(r['served.unreferenced_bytes'].note, /orphan/);
  assert.equal(r['ux.shell_missing'].value, 7);
  assert.match(r['ux.shell_missing'].note, /room\/sw.js: \/room\/app.js/);
  assert.equal(r['poll.room_ms'].value, 5000);
  assert.equal(r['ux.console_tab_bytes'].note, '2 lazy chunks');
  assert.equal(r['routes.count'].value, 2);
  const clean = byId(await collect({ root: tree({ 'workers/api/public/sw.js': 'const SHELL = [];', 'workers/api/public/room/sw.js': 'const SHELL = [];', 'workers/api/public/courier/sw.js': 'const SHELL = [];' }) }));
  assert.equal(clean['ux.shell_missing'].value, 0);
  assert.equal(clean['ux.shell_missing'].note, undefined);
});

test('a link with no rel is not an entry, and a file imported twice is walked once', () => {
  assert.deepEqual(htmlEntries('<link href="/x.css">'), []);
  const root = tree({
    'workers/api/public/d/index.html': '<script src="/d/a.js"></script><script src="/d/b.js"></script>',
    'workers/api/public/d/a.js': "import './c.js';", 'workers/api/public/d/b.js': "import './c.js';", 'workers/api/public/d/c.js': '',
    'workers/api/public/orphan/dead.bin': null,
  });
  const pub = path.join(root, 'workers/api/public');
  assert.equal(bootGraph(pub, 'd/index.html').files.length, 4);
  assert.ok(!fs.existsSync(path.join(pub, 'orphan/dead.bin')), 'a null body writes nothing');
});
