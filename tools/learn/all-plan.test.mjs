// node --test tools/learn/all-plan.test.mjs -- the run's order, state, resume and estimate, on a
// scratch repo with three lessons: no browser, no ffmpeg, no network.
import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { tmpdir } from 'node:os';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { lessons, order, filter, captureHash, readState, mark, isDone, todo, estimateS, planTable, summary, parse, main, COST, REPO } from './all-plan.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const L = [
  { id: 'O2', role: 'owner', writes: true, steps: 9 }, { id: 'W1', role: 'waiter', writes: false, steps: 10 },
  { id: 'O10', role: 'owner', writes: false, steps: 4 }, { id: 'O1a', role: 'owner', writes: false, steps: 13 },
  { id: 'C2', role: 'courier', writes: true, steps: 6 }, { id: 'G1', role: 'guest', writes: false, steps: 15 },
  { id: 'C1', role: 'courier', writes: false, steps: 5 }, { id: 'X1', role: 'kitchen', writes: false, steps: 1 },
];
const quiet = { log: () => {}, error: () => {} };
const rec = () => { const lines = []; return { lines, log: l => lines.push(l), error: l => lines.push(l) }; };

/// A scratch repo: lessons.json + three YAMLs.
function root() {
  const r = mkdtempSync(join(tmpdir(), 'allp-'));
  mkdirSync(join(r, 'workers/api/public/learn'), { recursive: true });
  const ls = [{ id: 'C1', role: 'courier', writes: false, steps: [{}, {}] }, { id: 'O2', role: 'owner', writes: true, steps: [{}] },
    { id: 'G1', role: 'guest', steps: [{}] }];
  writeFileSync(join(r, 'workers/api/public/learn/lessons.json'), JSON.stringify({ lessons: ls }));
  for (const l of ls) { mkdirSync(join(r, 'docs/learn/lessons', l.role), { recursive: true }); writeFileSync(join(r, 'docs/learn/lessons', l.role, `${l.id}.yaml`), `id: ${l.id}\n`); }
  return r;
}

test('order: read-only first (courier, guest, owner, waiter, others), then the writing ones; ids naturally', () => {
  assert.deepEqual(order(L).map(l => l.id), ['C1', 'G1', 'O1a', 'O10', 'W1', 'X1', 'C2', 'O2']);
  assert.deepEqual(filter(L, { role: 'owner' }).map(l => l.id), ['O2', 'O10', 'O1a']);
  assert.deepEqual(filter(L, { only: ['W1', 'C1'] }).map(l => l.id), ['W1', 'C1']);
  assert.equal(filter(L).length, L.length);
});

test('lessons: the real catalogue loads with role, writes and step count', () => {
  const all = lessons();
  assert.ok(all.length >= 61);
  assert.ok(all.every(l => typeof l.writes === 'boolean' && l.steps > 0 && l.role));
  const r = root();
  assert.deepEqual(lessons(r).find(l => l.id === 'G1'), { id: 'G1', role: 'guest', writes: false, steps: 1 });
});

test('captureHash: moves with the YAML and with the recorder', () => {
  const r = root(), y = join(r, 'docs/learn/lessons/courier/C1.yaml');
  const h = captureHash(y);
  assert.match(h, /^[0-9a-f]{64}$/);
  writeFileSync(y, 'id: C1\n# edited\n');
  assert.notEqual(captureHash(y), h);
  const fake = mkdtempSync(join(tmpdir(), 'rec-'));
  for (const f of ['capture.mjs', 'capture-lib.mjs', 'capture-venue.mjs']) writeFileSync(join(fake, f), f);
  const a = captureHash(y, fake);
  writeFileSync(join(fake, 'capture.mjs'), 'changed');
  assert.notEqual(captureHash(y, fake), a);
});

test('state: mark writes through a rename; a broken file reads as empty; done needs the current hash', () => {
  const out = mkdtempSync(join(tmpdir(), 'out-'));
  assert.deepEqual(readState(out), {});
  const m = mark(out, 'C1', 'done', '', new Date(0));
  assert.deepEqual(m, { status: 'done', at: '1970-01-01T00:00:00.000Z', note: '' });
  assert.ok(!existsSync(join(out, '.state.json.tmp')));
  assert.equal(isDone(out, 'C1', 'h1'), false);                                  // no .captured
  mkdirSync(join(out, 'C1')); writeFileSync(join(out, 'C1', '.captured'), 'h1\n');
  assert.equal(isDone(out, 'C1', 'h1'), true);
  assert.equal(isDone(out, 'C1', 'h2'), false);                                  // YAML moved on
  mark(out, 'O2', 'failed', 'capture rc=1');
  assert.equal(readState(out).O2.note, 'capture rc=1');
  const hashOf = () => 'h1';
  assert.deepEqual(todo([{ id: 'C1' }, { id: 'O2' }, { id: 'G1' }], out, { hashOf }).map(l => l.id), ['O2', 'G1']);
  assert.deepEqual(todo([{ id: 'C1' }, { id: 'O2' }, { id: 'G1' }], out, { hashOf, retry: true }).map(l => l.id), ['O2']);
  writeFileSync(join(out, '.state.json'), '{broken');
  assert.deepEqual(readState(out), {});
});

test('estimate + planTable + summary: minutes per lesson, a total, done and failed named', () => {
  const l = { id: 'C1', role: 'courier', writes: false, steps: 5 };
  assert.equal(estimateS(l), Math.round(3 * (COST.launchS + 5 * COST.stepS) + 3 * COST.assembleS + (3 * COST.filesPerCut + 2) * COST.putS));
  assert.ok(estimateS(l, ['sq']) < estimateS(l));
  const t = planTable([l, { ...l, id: 'O2', role: 'owner', writes: true }]);
  assert.match(t[0], /^C1 +courier +reads +5 steps x sq,en,uk +~\d+ min$/);
  assert.match(t[1], /writes/);
  assert.match(t[2], /^plan: 2 lesson\(s\) x 3 language\(s\), ~\d+ min \(~\d+\.\d h\)$/);
  const out = mkdtempSync(join(tmpdir(), 'out-'));
  mark(out, 'C1', 'done'); mark(out, 'O2', 'failed', 'check'); mark(out, 'G1', 'failed');
  assert.equal(summary([{ id: 'C1' }, { id: 'O2' }, { id: 'G1' }, { id: 'W1' }], out), 'progress: 1/4 done, 2 failed: O2 (check), G1');
  assert.equal(summary([{ id: 'W1' }], out), 'progress: 0/1 done, 0 failed');
});

test('parse: every flag; bare values are kept for --mark', () => {
  const o = parse(['--out', 'd', '--only', 'C1,O2', '--role', 'owner', '--lang', 'sq', '--retry', '--mark', 'C1', 'done']);
  assert.deepEqual(o, { out: 'd', cmd: 'mark', only: ['C1', 'O2'], role: 'owner', langs: ['sq'], retry: true, rest: ['C1', 'done'] });
  assert.deepEqual(parse(['--only']).only, []);
  assert.equal(parse(['--role']).role, null);
  assert.deepEqual(parse(['--lang']).langs, []);
});

test('main: usage, capture-hash, mark, summary, list (resumes past done), plan', () => {
  const r = root(), out = mkdtempSync(join(tmpdir(), 'out-')), o = { root: r };
  assert.equal(main([], { ...o, log: quiet }), 2);
  const h = rec();
  assert.equal(main(['--out', out, '--capture-hash', 'C1'], { ...o, log: h }), 0);
  assert.match(h.lines[0], /^[0-9a-f]{64}$/);
  assert.equal(main(['--out', out, '--capture-hash', 'Z9'], { ...o, log: quiet }), 1);
  assert.equal(main(['--out', out, '--mark', 'C1', 'maybe'], { ...o, log: quiet }), 2);
  assert.equal(main(['--out', out, '--mark', 'C1', 'done'], { ...o, log: quiet }), 0);
  mkdirSync(join(out, 'C1')); writeFileSync(join(out, 'C1', '.captured'), h.lines[0]);
  assert.equal(main(['--out', out, '--mark', 'O2', 'failed', 'capture', 'rc=1'], { ...o, log: quiet }), 0);
  const l = rec();
  assert.equal(main(['--out', out, '--list'], { ...o, log: l }), 0);
  assert.deepEqual(l.lines, ['G1', 'O2']);
  const rt = rec();
  main(['--out', out, '--list', '--retry'], { ...o, log: rt });
  assert.deepEqual(rt.lines, ['O2']);
  const s = rec();
  main(['--out', out, '--summary'], { ...o, log: s });
  assert.deepEqual(s.lines, ['progress: 1/3 done, 1 failed: O2 (capture rc=1)']);
  const p = rec();
  assert.equal(main(['--out', out, '--plan', '--lang', 'sq'], { ...o, log: p }), 0);
  assert.match(p.lines.at(-1), /^plan: 2 lesson\(s\) x 1 language/);
  assert.equal(readFileSync(join(out, '.state.json'), 'utf8').includes('"O2"'), true);
});

test('all-plan.mjs run as a script: no arguments is the usage and exit 2', () => {
  const r = spawnSync(process.execPath, [join(HERE, 'all-plan.mjs')], { encoding: 'utf8' });
  assert.equal(r.status, 2);
  assert.match(r.stderr, /usage: all-plan\.mjs/);
  assert.ok(REPO.length > 0);
});
