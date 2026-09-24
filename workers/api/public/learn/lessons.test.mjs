// The lesson build: tools/learn/yaml-lite.mjs (the YAML subset the lessons are
// written in) and tools/learn/build-lessons.mjs (YAML -> this folder's
// lessons.json). It lives HERE, beside the file it proves, because CI runs
// `node --test` over workers/api/public/**.test.mjs.
// `node --test workers/api/public/learn/lessons.test.mjs`
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, mkdtempSync, mkdirSync, rmSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { spawnSync } from 'node:child_process';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { parseYaml, YamlError } from '../../../../tools/learn/yaml-lite.mjs';
import { bool, normalize, build, sources, main, OUT, SRC, ROLES } from '../../../../tools/learn/build-lessons.mjs';

const REPO = fileURLToPath(new URL('../../../../', import.meta.url));
const BUILD = join(REPO, 'tools/learn/build-lessons.mjs');

// ── yaml-lite ────────────────────────────────────────────────────────────────
test('yaml: mappings, nesting, sequences of mappings and of scalars, every scalar a string', () => {
  const doc = parseYaml(`# a comment
id: C2
order: 2

title:
  sq: "Merrni një porosi"
  en: 'It''s yours'
covers: [waiting, pickList, offer]
none: []
empty:
note: plain words # a trailing comment
steps:
  - key: 1
    action:
      do: click
  - just a scalar
  -
    nested: yes
  -
list:
- a
- "b: c"
`);
  assert.deepEqual(doc, {
    id: 'C2', order: '2', title: { sq: 'Merrni një porosi', en: "It's yours" }, covers: ['waiting', 'pickList', 'offer'],
    none: [], empty: null, note: 'plain words',
    steps: [{ key: '1', action: { do: 'click' } }, 'just a scalar', { nested: 'yes' }, null], list: ['a', 'b: c'] });
  assert.equal(parseYaml(''), null);
  assert.equal(parseYaml('# only\n\n'), null);
  assert.deepEqual(parseYaml('a: "x\\u00eb\\n"'), { a: 'xë\n' });
});

test('yaml: every refusal names its line', () => {
  const bad = {
    'a: 1\n\tb: 2': /line 2: tab in indentation/,
    'a: 1\na: 2': /line 2: duplicate key a/,
    'a: "open': /line 1: bad double-quoted string/,
    'a: "1" "2"': /line 1: bad double-quoted string/,
    "a: 'it's'": /line 1: bad single-quoted string/,
    'a: [b, c': /line 1: unclosed flow sequence/,
    'a: [b, "c"]': /line 1: a flow sequence holds plain scalars only/,
    'a: {b: c}': /line 1: flow mappings are not supported/,
    'a: 1\njust text': /line 2: expected "key: value", got just text/,
    'a: 1\n    b: 2': /line 2: unexpected indentation/,
    '  a: 1\nb: 2': /line 2: unexpected content b: 2/,
  };
  for (const [src, re] of Object.entries(bad)) {
    assert.throws(() => parseYaml(src), e => e instanceof YamlError && re.test(e.message), JSON.stringify(src));
  }
});

test('yaml: reads every lesson file the same as js-yaml does (when this box has it)', async t => {
  let yaml;
  try { yaml = createRequire(import.meta.url)('/usr/share/nodejs/js-yaml'); } catch { return t.skip('js-yaml not installed here'); }
  const files = sources(REPO);
  assert.ok(files.length > 40, `only ${files.length} lesson files`);
  for (const [file, text] of files) {
    const a = [], b = [];
    assert.deepEqual(normalize(parseYaml(text), file, a), normalize(yaml.load(text), file, b), file);
    assert.deepEqual(a, b, file);
  }
});

// ── normalize ────────────────────────────────────────────────────────────────
const tri = s => ({ sq: `${s} sq`, en: `${s} en`, uk: `${s} uk` });
const good = () => ({ id: 'C9', role: 'courier', module: 'm', order: '3', covers: ['x'], title: tri('t'), goal: tri('g'),
  steps: [
    { key: 'hi', anchor: 'none', writes: 'no', action: { do: 'wait' }, title: tri('a'), caption: tri('b') },
    { anchor: 'run.eta', pending: 'no', writes: 'yes', action: { do: 'type', selector: '[data-tour="run.eta"]', value: 'v' }, title: tri('c'), caption: tri('d') },
    { anchor: 'more.tile.venue', pending: true, writes: false, action: { do: 'click', selector: '[data-tour="more.tile.venue"]' }, title: tri('e'), caption: tri('f') },
  ] });
const F = 'docs/learn/lessons/courier/C9.yaml';

test('normalize: a good lesson -> the shape learn.js reads', () => {
  const errs = [];
  const l = normalize(good(), F, errs);
  assert.deepEqual(errs, []);
  assert.equal(l.app, 'courier'); assert.equal(l.order, 3); assert.equal(l.writes, true);
  assert.deepEqual(l.steps.map(s => [s.n, s.key, s.anchor, s.at, s.pending, s.writes]), [
    [1, 'hi', null, null, false, false], [2, '2', 'run.eta', '[data-tour="run.eta"]', false, true],
    [3, '3', 'more.tile.venue', '[data-tour="more.tile.venue"]', true, false]]);
  assert.deepEqual(l.steps[1].action, { do: 'type', selector: '[data-tour="run.eta"]', value: 'v' });
  assert.deepEqual(l.steps[0].action, { do: 'wait', selector: null });
  const noCovers = good(); delete noCovers.covers;
  assert.deepEqual(normalize(noCovers, F, []).covers, []);
});

test('normalize: each broken field is refused by name, and nothing half-built comes back', () => {
  const cases = [
    [d => null, /not a mapping/],
    [d => ['x'], /not a mapping/],
    [d => ({ ...d, id: 'C8' }), /id "C8" is not the file name C9/],
    [d => ({ ...d, role: 'waiter' }), /role "waiter" is not the folder courier/],
    [d => ({ ...d, module: '' }), /module: missing/],
    [d => ({ ...d, order: 'two' }), /order: not a whole number/],
    [d => ({ ...d, covers: 'x' }), /covers: not a list/],
    [d => ({ ...d, title: { sq: 'a', en: 'b' } }), /title\.uk: missing/],
    [d => ({ ...d, goal: null }), /goal\.sq: missing/],
    [d => ({ ...d, steps: [] }), /steps: none/],
    [d => ({ ...d, steps: ['x'] }), /steps\[1\]: not a mapping/],
    [d => { d.steps[1].key = 'hi'; return d; }, /steps\[2\]: key hi twice/],
    [d => { d.steps[1].anchor = 'Run.eta'; d.steps[1].action.selector = '[data-tour="Run.eta"]'; return d; }, /anchor Run\.eta is not <module>\.<control>/],
    [d => { d.steps[1].pending = 'maybe'; return d; }, /steps\[2\]\.pending: expected yes or no, got "maybe"/],
    [d => { d.steps[1].action.do = 'drag'; return d; }, /action\.do: "drag" is not one of click\/type\/wait/],
    [d => { d.steps[1].action.selector = '#eta'; return d; }, /action\.selector: "#eta" should be/],
    [d => { d.steps[0].action.do = 'click'; return d; }, /a step with no anchor can only wait/],
    [d => { delete d.steps[1].action.value; return d; }, /action\.value: a type action needs one/],
    [d => { delete d.steps[2].action; return d; }, /steps\[3\]\.action\.do/],
    [d => { d.steps[2].caption = { sq: ' ', en: 'x', uk: 'y' }; return d; }, /steps\[3\]\.caption\.sq: missing/],
  ];
  for (const [mut, re] of cases) {
    const errs = [];
    assert.equal(normalize(mut(good()), F, errs), null, re.source);
    assert.ok(errs.some(e => re.test(e)), `${re.source} not in ${JSON.stringify(errs)}`);
  }
  const errs = [];
  normalize({ ...good(), id: 'c9' }, 'docs/learn/lessons/courier/c9.yaml', errs);
  assert.ok(errs.some(e => /id c9 is not <Letter><number>\[a-z\]/.test(e)));
});

test('bool: yes/no/true/false only', () => {
  const errs = [];
  assert.deepEqual(['yes', 'true', true, 'no', 'false', false].map(v => bool(v, 'w', errs)), [true, true, true, false, false, false]);
  assert.deepEqual(errs, []);
  assert.equal(bool('1', 'w', errs), false);
  assert.deepEqual(errs, ['w: expected yes or no, got "1"']);
});

// ── build / main ─────────────────────────────────────────────────────────────
test('build: the committed lessons.json is exactly what the YAML builds (the gate runs the same check)', () => {
  const { text, errors, lessons } = build(sources(REPO));
  assert.deepEqual(errors, []);
  assert.equal(readFileSync(join(REPO, OUT), 'utf8'), text, 'stale: node tools/learn/build-lessons.mjs');
  const j = JSON.parse(text);
  assert.equal(j.lessons.length, lessons.length);
  for (const r of ROLES) assert.equal(j.counts[r], lessons.filter(l => l.role === r).length, r);
  assert.deepEqual(j.langs, ['sq', 'en', 'uk']);
  assert.match(j.built_from, /^[0-9a-f]{16}$/);
  const ids = j.lessons.map(l => l.id);
  assert.equal(new Set(ids).size, ids.length);
});

test('build: a parse error, an invalid lesson and a duplicate id are each reported; valid ones still sort by role then order', () => {
  const y = (id, role, order) => `id: ${id}\nrole: ${role}\nmodule: m\norder: ${order}\ntitle:\n  sq: a\n  en: a\n  uk: a\ngoal:\n  sq: a\n  en: a\n  uk: a\nsteps:\n  - anchor: none\n    writes: no\n    action:\n      do: wait\n    title:\n      sq: a\n      en: a\n      uk: a\n    caption:\n      sq: a\n      en: a\n      uk: a\n`;
  const r = build([
    ['x/courier/C2.yaml', y('C2', 'courier', 2)],
    ['x/courier/C1.yaml', y('C1', 'courier', 1)],
    ['x/owner/O1.yaml', y('O1', 'owner', 9)],
    ['x/owner/O2.yaml', 'a: "bad'],
    ['x/owner/O3.yaml', y('O3', 'owner', 'x')],
    ['y/owner/O1.yaml', y('O1', 'owner', 1)],
    ['x/owner/O5.yaml', y('O5', 'owner', 1)],
  ]);
  assert.deepEqual(r.lessons.map(l => `${l.id}:${l.order}`), ['O1:1', 'O5:1', 'O1:9', 'C1:1', 'C2:2'], 'a tie on order falls back to the id');
  assert.ok(r.errors.some(e => e.startsWith('x/owner/O2.yaml: line 1')));
  assert.ok(r.errors.some(e => /O3\.yaml: order/.test(e)));
  assert.ok(r.errors.includes('O1: two lessons with this id'));
});

function scratch(){
  const root = mkdtempSync(join(tmpdir(), 'learn-build-'));
  mkdirSync(join(root, SRC, 'courier'), { recursive: true });
  writeFileSync(join(root, SRC, 'courier', 'C1.yaml'), readFileSync(join(REPO, SRC, 'courier', 'C1.yaml')));
  return root;
}
const quiet = () => { const out = { log: [], error: [] }; return { out, log: { log: m => out.log.push(m), error: m => out.error.push(m) } }; };

test('main: writes, then --check passes; a YAML edit makes --check refuse; an invalid YAML refuses both ways', () => {
  const root = scratch();
  try {
    let q = quiet();
    assert.equal(main(['--check'], root, q.log), 1, 'no lessons.json yet: stale');
    assert.match(q.out.error[0], /is stale/);
    q = quiet();
    assert.equal(main([], root, q.log), 0);
    assert.match(q.out.log[0], /wrote 1 lessons/);
    assert.ok(existsSync(join(root, OUT)));
    q = quiet();
    assert.equal(main(['--check', '--root', root], REPO, q.log), 0, '--root wins over the default');
    assert.match(q.out.log[0], /1 lessons, .* up to date/);
    const f = join(root, SRC, 'courier', 'C1.yaml');
    writeFileSync(f, readFileSync(f, 'utf8').replace('"Turni i parë"', '"Turni i parë!"'));
    assert.equal(main(['--check'], root, quiet().log), 1);
    writeFileSync(f, readFileSync(f, 'utf8').replace('module: shift\n', ''));
    q = quiet();
    assert.equal(main([], root, q.log), 1);
    assert.deepEqual(q.out.error, ['build-lessons: docs/learn/lessons/courier/C1.yaml: module: missing', 'build-lessons: 1 error(s)']);
  } finally { rmSync(root, { recursive: true, force: true }); }
});

test('the CLI: `--check` on the repo exits 0, on a stale scratch copy exits 1', () => {
  const ok = spawnSync(process.execPath, [BUILD, '--check'], { encoding: 'utf8' });
  assert.equal(ok.status, 0, ok.stderr);
  const root = scratch();
  try {
    const bad = spawnSync(process.execPath, [BUILD, '--check', '--root', root], { encoding: 'utf8' });
    assert.equal(bad.status, 1);
    assert.match(bad.stderr, /is stale/);
  } finally { rmSync(root, { recursive: true, force: true }); }
});
