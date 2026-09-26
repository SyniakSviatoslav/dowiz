#!/usr/bin/env node
// The pure half of tools/learn/all.sh: which lessons still need a video, in what order, how
// long that will take, and the resumable state of a run. No browser, no ffmpeg, no network.
//
//   node tools/learn/all-plan.mjs --out DIR --list  [--only IDS] [--role R] [--lang L] [--retry]
//        the ids to work on, one per line, in recording order (done ones left out)
//   node tools/learn/all-plan.mjs --out DIR --plan  [same filters]   the dry-run table + estimate
//   node tools/learn/all-plan.mjs --out DIR --mark ID STATUS [NOTE]  record done | failed
//   node tools/learn/all-plan.mjs --out DIR --summary                done/total, failed ids
//   node tools/learn/all-plan.mjs --out DIR --capture-hash ID        the hash a capture is keyed by
//
// ORDER (operator, 2026-09-26): read-only lessons first -- courier, guest, owner, waiter -- then
// the ones that write, same role order; inside a group by id. STATE: DIR/.state.json
// { id: { status, at, note } }; a lesson is DONE when its state says so AND its capture hash
// is still the current one (an edited YAML or recorder makes it due again).
import { readFileSync, writeFileSync, existsSync, renameSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
export const REPO = join(HERE, '..', '..');
export const ROLE_ORDER = ['courier', 'guest', 'owner', 'waiter'];
export const LANGS = ['sq', 'en', 'uk'];
export const RECORDER = ['capture.mjs', 'capture-lib.mjs', 'capture-venue.mjs'];
/// Seconds, measured on this box 2026-09-26 (see phase1.txt): per language a browser start and
/// sign-in, per step the dwell; per cut the assembly; per uploaded object one wrangler put.
export const COST = { launchS: 35, stepS: 5.5, assembleS: 25, putS: 9, filesPerCut: 6 };

export function lessons(root = REPO) {
  return JSON.parse(readFileSync(join(root, 'workers/api/public/learn/lessons.json'), 'utf8')).lessons
    .map(l => ({ id: l.id, role: l.role, writes: !!l.writes, steps: l.steps.length }));
}

const natural = (a, b) => a.localeCompare(b, 'en', { numeric: true });
/// Read-only before writing; then role; then id.
export function order(list) {
  const rank = l => (l.writes ? 10 : 0) + (ROLE_ORDER.includes(l.role) ? ROLE_ORDER.indexOf(l.role) : 9);
  return [...list].sort((a, b) => rank(a) - rank(b) || natural(a.id, b.id));
}

export function filter(list, { only = [], role = null } = {}) {
  return list.filter(l => (!only.length || only.includes(l.id)) && (!role || l.role === role));
}

/// sha256 over the lesson's YAML and the recorder's scripts: what a capture is made from.
export function captureHash(yamlFile, here = HERE) {
  const h = createHash('sha256');
  for (const f of [yamlFile, ...RECORDER.map(s => join(here, s))]) h.update(readFileSync(f));
  return h.digest('hex');
}

export function readState(out) {
  try { return JSON.parse(readFileSync(join(out, '.state.json'), 'utf8')); } catch { return {}; }
}
/// Written through a rename, so a kill mid-write leaves the previous state, never half a file.
export function mark(out, id, status, note = '', now = new Date()) {
  const s = readState(out);
  s[id] = { status, at: now.toISOString(), note };
  writeFileSync(join(out, '.state.json.tmp'), JSON.stringify(s, null, 1));
  renameSync(join(out, '.state.json.tmp'), join(out, '.state.json'));
  return s[id];
}

/// Done = marked done, and still recorded from the current YAML and recorder.
export function isDone(out, id, hash, state = readState(out)) {
  const captured = existsSync(join(out, id, '.captured')) ? readFileSync(join(out, id, '.captured'), 'utf8').trim() : '';
  return state[id]?.status === 'done' && captured === hash;
}

/// What is still to do. `retry` keeps only the ones a run marked failed.
export function todo(list, out, { hashOf, retry = false } = {}) {
  const st = readState(out);
  return list.filter(l => !isDone(out, l.id, hashOf(l.id), st) && (!retry || st[l.id]?.status === 'failed'));
}

export function estimateS(l, langs = LANGS) {
  const n = langs.length;
  return Math.round(n * (COST.launchS + l.steps * COST.stepS) + n * COST.assembleS + (n * COST.filesPerCut + 2) * COST.putS);
}

export function planTable(list, langs = LANGS) {
  let total = 0;
  const rows = list.map(l => { const s = estimateS(l, langs); total += s; return `${l.id.padEnd(5)} ${l.role.padEnd(8)} ${l.writes ? 'writes' : 'reads '} ${String(l.steps).padStart(2)} steps x ${langs.join(',')}  ~${Math.ceil(s / 60)} min`; });
  rows.push(`plan: ${list.length} lesson(s) x ${langs.length} language(s), ~${Math.ceil(total / 60)} min (~${(total / 3600).toFixed(1)} h)`);
  return rows;
}

export function summary(all, out) {
  const st = readState(out);
  const done = all.filter(l => st[l.id]?.status === 'done').length;
  const failed = all.filter(l => st[l.id]?.status === 'failed').map(l => `${l.id}${st[l.id].note ? ` (${st[l.id].note})` : ''}`);
  return `progress: ${done}/${all.length} done, ${failed.length} failed${failed.length ? ': ' + failed.join(', ') : ''}`;
}

export function parse(argv) {
  const o = { out: null, cmd: null, only: [], role: null, langs: LANGS, retry: false, rest: [] };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i], v = argv[i + 1];
    if (a === '--out') { o.out = v; i++; }
    else if (a === '--only') { o.only = String(v || '').split(',').filter(Boolean); i++; }
    else if (a === '--role') { o.role = v || null; i++; }
    else if (a === '--lang') { o.langs = String(v || '').split(',').filter(Boolean); i++; }
    else if (a === '--retry') o.retry = true;
    else if (['--list', '--plan', '--mark', '--summary', '--capture-hash'].includes(a)) o.cmd = a.slice(2);
    else o.rest.push(a);
  }
  return o;
}

export function main(argv, { root = REPO, log = console } = {}) {
  const o = parse(argv);
  if (!o.out || !o.cmd) { log.error('usage: all-plan.mjs --out DIR --list|--plan|--mark ID STATUS [NOTE]|--summary|--capture-hash ID'); return 2; }
  const yaml = id => { const l = lessons(root).find(x => x.id === id); return l && join(root, 'docs/learn/lessons', l.role, `${id}.yaml`); };
  if (o.cmd === 'capture-hash') { const f = yaml(o.rest[0]); if (!f) { log.error(`all-plan: no lesson ${o.rest[0]}`); return 1; } log.log(captureHash(f)); return 0; }
  if (o.cmd === 'mark') {
    const [id, status, ...note] = o.rest;
    if (!id || !['done', 'failed'].includes(status)) { log.error('all-plan: --mark ID done|failed [NOTE]'); return 2; }
    mark(o.out, id, status, note.join(' ')); return 0;
  }
  const all = order(filter(lessons(root), o));
  if (o.cmd === 'summary') { log.log(summary(all, o.out)); return 0; }
  const due = todo(all, o.out, { hashOf: id => captureHash(yaml(id)), retry: o.retry });
  if (o.cmd === 'list') { for (const l of due) log.log(l.id); return 0; }
  for (const r of planTable(due, o.langs)) log.log(r);
  return 0;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) process.exit(main(process.argv.slice(2)));
