#!/usr/bin/env node
// docs/learn/lessons/<role>/<id>.yaml  ->  workers/api/public/learn/lessons.json
//
// The YAML is the one source of every lesson: the in-app tour (/lib/learn.js)
// loads the JSON this writes, and the recorder reads the same YAML for its
// actions and captions. The JSON is committed so the Worker serves it as a
// static asset; `--check` refuses a JSON that no longer matches its YAML
// (tools/gates/learn.sh runs it), so the two cannot drift.
//
//   node tools/learn/build-lessons.mjs            write the JSON
//   node tools/learn/build-lessons.mjs --check    exit 1 when stale or invalid
//   ... --root DIR                                 read and write under DIR instead of the repo
//
// Every refusal names the file and the field. Nothing is defaulted silently:
// a missing language, an unknown action, a selector that does not match its
// anchor are errors, because each would show up later as a blank card or a
// recorder clicking nothing.
import { readFileSync, writeFileSync, readdirSync, existsSync, mkdirSync } from 'node:fs';
import { join, dirname, relative } from 'node:path';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import { parseYaml } from './yaml-lite.mjs';

export const ROLES = ['owner', 'waiter', 'courier', 'guest'];
export const ROLE_APP = { owner: 'owner', waiter: 'room', courier: 'courier', guest: 'store' };
export const LANGS = ['sq', 'en', 'uk'];
export const ACTIONS = ['click', 'type', 'wait'];
const ANCHOR = /^[a-z][A-Za-z]*(?:\.[A-Za-z]+)+$/;
const ID = /^[A-Z][0-9]+[a-z]?$/;
const REPO = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
export const SRC = 'docs/learn/lessons';
export const OUT = 'workers/api/public/learn/lessons.json';

/// yes/no/true/false -> boolean; anything else is an error.
export function bool(v, where, errs){
  if (v === true || v === 'yes' || v === 'true') return true;
  if (v === false || v === 'no' || v === 'false') return false;
  errs.push(`${where}: expected yes or no, got ${JSON.stringify(v)}`);
  return false;
}

function langs(v, where, errs){
  const out = {};
  for (const l of LANGS) {
    const s = v && typeof v === 'object' ? v[l] : undefined;
    if (typeof s !== 'string' || !s.trim()) errs.push(`${where}.${l}: missing`);
    else out[l] = s.trim();
  }
  return out;
}

/// One parsed YAML document -> one lesson, or errors. `file` is repo-relative.
export function normalize(doc, file, errs){
  const [role, name] = file.split('/').slice(-2);
  const id = name.replace(/\.yaml$/, '');
  const e0 = errs.length;
  const at = f => `${file}: ${f}`;
  if (!doc || typeof doc !== 'object' || Array.isArray(doc)) { errs.push(at('not a mapping')); return null; }
  if (doc.id !== id) errs.push(at(`id ${JSON.stringify(doc.id)} is not the file name ${id}`));
  if (!ID.test(id)) errs.push(at(`id ${id} is not <Letter><number>[a-z]`));
  if (doc.role !== role || !ROLES.includes(role)) errs.push(at(`role ${JSON.stringify(doc.role)} is not the folder ${role}`));
  if (typeof doc.module !== 'string' || !doc.module) errs.push(at('module: missing'));
  const order = Number(doc.order);
  if (!Number.isInteger(order) || order < 0) errs.push(at(`order: not a whole number (${JSON.stringify(doc.order)})`));
  const covers = doc.covers ?? [];
  if (!Array.isArray(covers) || covers.some(c => typeof c !== 'string' || !c)) errs.push(at('covers: not a list of names'));
  const title = langs(doc.title, at('title'), errs);
  const goal = langs(doc.goal, at('goal'), errs);
  if (!Array.isArray(doc.steps) || !doc.steps.length) { errs.push(at('steps: none')); return null; }
  const keys = new Set();
  const steps = doc.steps.map((s, i) => {
    const w = at(`steps[${i + 1}]`);
    if (!s || typeof s !== 'object') { errs.push(`${w}: not a mapping`); return null; }
    const key = String(s.key ?? i + 1);
    if (keys.has(key)) errs.push(`${w}: key ${key} twice`);
    keys.add(key);
    const anchor = s.anchor === 'none' || s.anchor == null ? null : String(s.anchor);
    if (anchor && !ANCHOR.test(anchor)) errs.push(`${w}: anchor ${anchor} is not <module>.<control>`);
    const pending = anchor ? bool(s.pending, `${w}.pending`, errs) : false;
    const writes = bool(s.writes, `${w}.writes`, errs);
    const a = s.action && typeof s.action === 'object' ? s.action : {};
    if (!ACTIONS.includes(a.do)) errs.push(`${w}.action.do: ${JSON.stringify(a.do)} is not one of ${ACTIONS.join('/')}`);
    const sel = anchor ? `[data-tour="${anchor}"]` : null;
    if ((a.selector ?? null) !== sel) errs.push(`${w}.action.selector: ${JSON.stringify(a.selector)} should be ${JSON.stringify(sel)}`);
    if (!anchor && a.do !== 'wait') errs.push(`${w}: a step with no anchor can only wait`);
    if (a.do === 'type' && (typeof a.value !== 'string' || !a.value)) errs.push(`${w}.action.value: a type action needs one`);
    const action = { do: a.do, selector: sel };
    if (typeof a.value === 'string') action.value = a.value;
    return { n: i + 1, key, anchor, at: sel, pending, writes, action,
      title: langs(s.title, `${w}.title`, errs), caption: langs(s.caption, `${w}.caption`, errs) };
  });
  if (errs.length > e0) return null;
  return { id, role, app: ROLE_APP[role], module: doc.module, order, covers, writes: steps.some(s => s.writes),
    title, goal, steps };
}

/// Every lesson file under `root`, sorted, as [relative path, text].
export function sources(root = REPO){
  const out = [];
  for (const role of ROLES) {
    const dir = join(root, SRC, role);
    if (!existsSync(dir)) continue;
    for (const f of readdirSync(dir).filter(f => f.endsWith('.yaml')).sort()) {
      const p = join(dir, f);
      out.push([relative(root, p).split('\\').join('/'), readFileSync(p, 'utf8')]);
    }
  }
  return out;
}

/// Build the JSON text from the sources. Returns { text, errors, lessons }.
export function build(files){
  const errs = [];
  const hash = createHash('sha256');
  const lessons = [];
  for (const [file, text] of files) {
    hash.update(file + '\0' + text + '\0');
    let doc;
    try { doc = parseYaml(text); } catch (e) { errs.push(`${file}: ${e.message}`); continue; }
    const l = normalize(doc, file, errs);
    if (l) lessons.push(l);
  }
  const seen = new Set();
  for (const l of lessons) { if (seen.has(l.id)) errs.push(`${l.id}: two lessons with this id`); seen.add(l.id); }
  lessons.sort((a, b) => ROLES.indexOf(a.role) - ROLES.indexOf(b.role) || a.order - b.order || a.id.localeCompare(b.id));
  const counts = Object.fromEntries(ROLES.map(r => [r, lessons.filter(l => l.role === r).length]));
  const head = { version: 1, built_from: hash.digest('hex').slice(0, 16), counts, langs: LANGS };
  const text = `${JSON.stringify(head).slice(0, -1)},"lessons":[\n${lessons.map(l => JSON.stringify(l)).join(',\n')}\n]}\n`;
  return { text, errors: errs, lessons };
}

/// CLI. Returns the exit code (so the test can drive it without a process).
export function main(argv = process.argv.slice(2), root = REPO, log = console){
  const check = argv.includes('--check');
  const r = argv.indexOf('--root');
  if (r >= 0 && argv[r + 1]) root = argv[r + 1];   // the gate's proof runs on a scratch copy
  const { text, errors, lessons } = build(sources(root));
  if (errors.length) { for (const e of errors) log.error(`build-lessons: ${e}`); log.error(`build-lessons: ${errors.length} error(s)`); return 1; }
  const out = join(root, OUT);
  if (check) {
    const have = existsSync(out) ? readFileSync(out, 'utf8') : '';
    if (have !== text) { log.error(`build-lessons: ${OUT} is stale -- run node tools/learn/build-lessons.mjs`); return 1; }
    log.log(`build-lessons: ${lessons.length} lessons, ${OUT} up to date`);
    return 0;
  }
  mkdirSync(dirname(out), { recursive: true });
  writeFileSync(out, text);
  log.log(`build-lessons: wrote ${lessons.length} lessons to ${OUT}`);
  return 0;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) process.exit(main());
