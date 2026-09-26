import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { render, fmt, delta, rowsOf } from './report.mjs';
import { card, table, page, latest, build, dirOf, HEADLINES } from './dashboard.mjs';

const RUN = { suite: 'ci', date: '2026-09-26', commit: 'abc', deployed_version: 'v1', hosts: [], started_at: 0, finished_at: 2000, node: 'v22' };
const doc = {
  _run: RUN,
  'wasm.raw': { value: 1200, unit: 'bytes', rule: 'ratchet', baseline: 1000, status: 'breach', why: '1200 > baseline 1000', note: 'a|b' },
  'wasm.gzip': { value: 500, unit: 'bytes', rule: 'ratchet', baseline: 500, status: 'ok', why: '' },
  'gates.green': { value: 20, unit: 'gates', rule: 'min', limit: 20, baseline: null, status: 'ok', why: '' },
  'cost.plan': { value: null, unit: 'plan', rule: 'trend', baseline: null, status: 'unverified', unverified: 'no token', source: 'dash' },
};

test('fmt and delta', () => {
  assert.equal(fmt(null), '—');
  assert.equal(fmt(undefined), '—');
  assert.equal(fmt(1234567), '1,234,567');
  assert.equal(fmt('x'), 'x');
  assert.equal(delta({ value: 5, baseline: 3 }), '+2');
  assert.equal(delta({ value: 3, baseline: 5 }), '-2');
  assert.equal(delta({ value: 3, baseline: 3 }), '0');
  assert.equal(delta({ value: 3, baseline: null }), '');
  assert.equal(delta({ value: null, baseline: 3 }), '');
  assert.equal(rowsOf(doc).length, 4);
});

test('the markdown lists breaches first, then unverified, then each group', () => {
  const md = render(doc);
  assert.match(md, /^# Evals · ci · 2026-09-26 · abc/);
  assert.match(md, /none \(no network\)/);
  assert.match(md, /4 indicators, 1 breaches, 1 unverified/);
  const b = md.indexOf('## Breaches'), u = md.indexOf('## Unverified'), g = md.indexOf('## wasm');
  assert.ok(b < u && u < g);
  assert.match(md, /`wasm.raw` \| 1,200 bytes \| ratchet \| 1200 > baseline 1000 — a\\\|b/);
  assert.match(md, /`cost.plan` — no token \(source: dash\)/);
  assert.match(md, /\| `gates.green` \| 20 \| gates \| — \|  \| min 20 \| ok \|/);
  assert.match(md, /\*\*breach\*\*/);
});

test('a green run says so, and a run with no meta still renders', () => {
  const md = render({ _run: { ...RUN, hosts: ['https://h'] }, 'a.b': { value: 1, unit: 'u', rule: 'trend', status: 'ok' } });
  assert.match(md, /None — every judged indicator/);
  assert.match(md, /## Unverified\n\nNone\./);
  assert.match(md, /Hosts: https:\/\/h/);
  assert.match(render({}), /# Evals · undefined/);
});

test('dashboard cards colour by status and skip missing ids', () => {
  assert.match(card(doc, 'wasm.raw', 'W'), /card bad/);
  assert.match(card(doc, 'wasm.gzip', 'G'), /card ok/);
  assert.match(card(doc, 'cost.plan', 'P'), /card unv/);
  assert.equal(card(doc, 'nope', 'N'), '');
  assert.ok(HEADLINES.length >= 8);
});

test('dashboard tables open a group with a breach and escape text', () => {
  const t = table({ ...doc, 'x.y': { value: 1, unit: '<b>', rule: 'trend', status: 'ok', note: '&' } });
  assert.match(t, /<details open><summary>wasm/);
  assert.match(t, /<details><summary>x/);
  assert.match(t, /&lt;b&gt;/);
  assert.match(t, /min 20/);
  assert.match(t, /no token/);
});

test('the page embeds the newest run of each suite, and says when there is none', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'evals-dash-'));
  fs.writeFileSync(path.join(dir, '2026-09-25-old-ci.json'), JSON.stringify({ _run: { ...RUN, commit: 'old' } }));
  fs.writeFileSync(path.join(dir, '2026-09-26-abc-ci.json'), JSON.stringify(doc));
  fs.writeFileSync(path.join(dir, '2026-09-26-abc-nightly.json'), JSON.stringify({ _run: { ...RUN, suite: 'nightly' } }));
  fs.writeFileSync(path.join(dir, 'notes.json'), '{}');
  const runs = latest(dir);
  assert.deepEqual(Object.keys(runs).sort(), ['ci', 'nightly']);
  assert.equal(runs.ci._run.commit, 'abc');
  const out = build(dir);
  const html = fs.readFileSync(out, 'utf8');
  assert.match(html, /prefers-color-scheme:dark/);
  assert.match(html, /class="badt">1 breaches/);
  assert.match(html, /0 breaches/);
  assert.match(page({}), /No runs found/);
});

test('a breach with no unit and no note still renders its row', () => {
  const md = render({ _run: RUN, 'a.b': { value: 2, rule: 'zero', status: 'breach', why: '2 != 0' } });
  assert.match(md, /\| `a.b` \| 2  \| zero \| 2 != 0 \|/);
  assert.match(card({ 'a.b': { value: 1, status: 'ok' } }, 'a.b', 'L'), /<div class="u"> · ok/);
});

test('the command lines print the markdown and build the page', async () => {
  const { spawnSync } = await import('node:child_process');
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'evals-cli-'));
  const f = path.join(dir, '2026-09-26-abc-ci.json');
  fs.writeFileSync(f, JSON.stringify(doc));
  const here = path.dirname(new URL(import.meta.url).pathname);
  const r = spawnSync(process.execPath, [path.join(here, 'report.mjs'), f], { encoding: 'utf8' });
  assert.equal(r.status, 0);
  assert.match(r.stdout, /^# Evals · ci/);
  const d = spawnSync(process.execPath, [path.join(here, 'dashboard.mjs'), dir], { encoding: 'utf8' });
  assert.equal(d.status, 0);
  assert.ok(fs.existsSync(path.join(dir, 'latest.html')));
  const bad = spawnSync(process.execPath, [path.join(here, 'run.mjs'), '--suite', 'weekly'], { encoding: 'utf8' });
  assert.equal(bad.status, 2);
  assert.match(bad.stderr, /unknown suite weekly/);
});

test('the dashboard reads the named directory, else the repo one', () => {
  assert.equal(dirOf(['node', 'd.mjs', '/x']), '/x');
  assert.match(dirOf(['node', 'd.mjs']), /docs\/measurements\/evals$/);
});
