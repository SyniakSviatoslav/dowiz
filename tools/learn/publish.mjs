#!/usr/bin/env node
// Publish checked cuts: into the STAGING tree and, with --r2, into the R2 bucket `dowiz-learn`
// that the gated route (workers/api/src/learn.rs) serves.
//
// The staging tree is NOT under workers/api/public: everything there is a static asset, served
// to anyone without the Worker, and the videos are gated (operator, 2026-09-24). It lives at
// $LEARN_STAGE or ~/.cache/dowiz-learn/media. Only the manifest (URLs, durations, hashes, the
// YAML each video was made from -- no media) is kept in the repo, at MANIFEST, for
// tools/gates/learn.sh item 5 and as the record of what R2 holds.
//
//   node tools/learn/publish.mjs OUT_ROOT [ID...]      copy checked cuts + rewrite the manifest
//   node tools/learn/publish.mjs --manifest-only       rewrite the manifest from what is published
//   node tools/learn/publish.mjs --r2 [--dry-run]      upload the published tree to R2 (publish.sh
//                                                      sources the deploy token first)
//
// OUT_ROOT holds capture/assemble outputs (<ID>/lesson.json, <ID>/source.sha256, <ID>/<lang>/...).
// A cut whose checks fail is NOT published and is named (HELD). The manifest is rebuilt from the
// published tree only, so it can never name a file that is not there; each lesson carries the
// sha256 of the YAML its video was made from (`source`), which tools/gates/learn.sh compares.
//
// R2. The route serves `learn/<ID>/<lang>/<file>` and `learn/manifest.json` (URLs rewritten to
// /api/learn/media/...). The first thing --r2 does is put one probe object: a token without R2
// write fails THERE, loudly, with exit 4 -- never after half an upload, never silently. Unchanged
// objects are skipped by a ledger of what was last uploaded (~/.cache/dowiz-learn/r2-ledger.json).
import { readFileSync, writeFileSync, existsSync, mkdirSync, readdirSync, statSync, copyFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { homedir, tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';
import { checkCut, probe as ffprobe } from './check.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
export const REPO = join(HERE, '..', '..');
export const MANIFEST = 'workers/api/public/learn/media/manifest.json';
export const STAGE = process.env.LEARN_STAGE || join(homedir(), '.cache', 'dowiz-learn', 'media');
export const LANGS = ['sq', 'en', 'uk'];
export const FILES = ['video.mp4', 'poster.jpg', 'subs_sq.vtt', 'subs_en.vtt', 'subs_uk.vtt', 'chapters.json'];
export const BUCKET = 'dowiz-learn';
export const R2_PREFIX = 'learn/';
export const LEDGER = join(homedir(), '.cache', 'dowiz-learn', 'r2-ledger.json');
export const REFUSED = 4;

export const sha = f => createHash('sha256').update(readFileSync(f)).digest('hex');
const same = (a, b) => existsSync(b) && statSync(a).size === statSync(b).size && sha(a) === sha(b);

/// Copy one cut; answers how many files were written (0 = it was already there).
export function publishCut(out, id, lang, media) {
  const dst = join(media, id, lang);
  mkdirSync(dst, { recursive: true });
  let wrote = 0;
  for (const f of FILES) {
    const a = join(out, id, lang, f), b = join(dst, f);
    if (!existsSync(a)) throw new Error(`${id}/${lang}/${f} missing: assemble it first`);
    if (!same(a, b)) { copyFileSync(a, b); wrote++; }
  }
  for (const f of ['lesson.json', 'source.sha256']) {
    const a = join(out, id, f), b = join(media, id, f);
    if (existsSync(a) && !same(a, b)) { copyFileSync(a, b); wrote++; }
  }
  return wrote;
}

/// The manifest, from the published tree only. Keys are sorted so a rebuild with nothing
/// new is byte-identical.
export function manifest(media) {
  const lessons = {};
  const ids = existsSync(media) ? readdirSync(media).filter(d => statSync(join(media, d)).isDirectory()).sort() : [];
  for (const id of ids) {
    const lf = join(media, id, 'lesson.json'), sf = join(media, id, 'source.sha256');
    const lesson = existsSync(lf) ? JSON.parse(readFileSync(lf, 'utf8')) : null;
    const cuts = {};
    for (const lang of LANGS) {
      const dir = join(media, id, lang);
      if (!FILES.every(f => existsSync(join(dir, f)))) continue;
      const ch = JSON.parse(readFileSync(join(dir, 'chapters.json'), 'utf8'));
      const url = f => `/api/learn/media/${id}/${lang}/${f}`;
      cuts[lang] = { durationMs: ch.durationMs, steps: ch.chapters.length, absent: ch.chapters.filter(c => c.found === false).map(c => c.n), sha256: sha(join(dir, 'video.mp4')).slice(0, 16),
        bytes: statSync(join(dir, 'video.mp4')).size, video: url('video.mp4'), poster: url('poster.jpg'), chapters: url('chapters.json'),
        subs: Object.fromEntries(LANGS.map(l => [l, url(`subs_${l}.vtt`)])) };
    }
    if (Object.keys(cuts).length) lessons[id] = { role: lesson?.role ?? null, module: lesson?.module ?? null,
      source: existsSync(sf) ? readFileSync(sf, 'utf8').trim() : null, cuts };
  }
  return { version: 1, lessons };
}


const TYPES = { mp4: 'video/mp4', vtt: 'text/vtt; charset=utf-8', jpg: 'image/jpeg', json: 'application/json', txt: 'text/plain' };
export const typeOf = f => TYPES[f.split('.').pop()] || 'application/octet-stream';

/// Every object the bucket should hold: [key, local file], manifest last.
export function objects(media) {
  const out = [];
  for (const [id, l] of Object.entries(manifest(media).lessons))
    for (const lang of Object.keys(l.cuts)) for (const f of FILES) out.push([`${R2_PREFIX}${id}/${lang}/${f}`, join(media, id, lang, f)]);
  return out;
}

/// One `wrangler r2 object put`. Answers { ok, out } -- `out` is stderr+stdout, tokens redacted.
/// $LEARN_WRANGLER (a wrangler 4 bin/wrangler.js) skips npx's resolution: 5.3 s a put instead of
/// 11 s, measured 2026-09-26 -- over ~1 100 objects that is the difference of 1.7 hours.
export function wranglerPut(key, file, cwd = join(REPO, 'workers/api'), spawn = spawnSync, bin = process.env.LEARN_WRANGLER) {
  const args = ['r2', 'object', 'put', `${BUCKET}/${key}`, '--file', file, '--content-type', typeOf(file), '--remote'];
  const r = bin ? spawn(process.execPath, [bin, ...args], { cwd, encoding: 'utf8', timeout: 180000 })
    : spawn('npx', ['--yes', 'wrangler@4', ...args], { cwd, encoding: 'utf8', timeout: 180000 });
  const out = `${r.stdout || ''}${r.stderr || ''}${r.error ? r.error.message : ''}`.replace(/[A-Za-z0-9_-]{32,}/g, '<redacted>');
  return { ok: r.status === 0, out };
}

/// Does this failure say the TOKEN may not write R2 (as opposed to a flaky network)?
export const authError = out => /Authentication error|code: 10000|not authorized|Unauthorized|403|10042|permission/i.test(out);

/// Upload what changed. `put(key, file)` is injectable for the tests. Answers the exit code:
/// 0 done, 1 some object failed, REFUSED (4) the token cannot write R2 -- nothing uploaded.
export function toR2(media, { put = wranglerPut, ledgerFile = LEDGER, dryRun = false, log = console, scratch = tmpdir() } = {}) {
  const want = objects(media);
  const m = join(scratch, `learn-manifest-${process.pid}.json`);
  writeFileSync(m, JSON.stringify(manifest(media), null, 1) + '\n');
  want.push([`${R2_PREFIX}manifest.json`, m]);
  let ledger = {};
  try { ledger = JSON.parse(readFileSync(ledgerFile, 'utf8')); } catch { ledger = {}; }
  const todo = want.filter(([k, f]) => ledger[k] !== sha(f));
  log.log(`r2: ${want.length} object(s), ${todo.length} changed since the last upload`);
  if (dryRun) { for (const [k] of todo) log.log(`r2: would put ${k}`); return 0; }
  const probeFile = join(scratch, `learn-probe-${process.pid}.txt`);
  writeFileSync(probeFile, `probe ${new Date().toISOString()}\n`);
  const p = put(`${R2_PREFIX}.probe.txt`, probeFile);
  if (!p.ok) {
    const why = authError(p.out) ? 'the token cannot write R2 objects' : 'the probe upload failed';
    log.error(`r2: REFUSED -- ${why}; NOTHING was uploaded. wrangler said:\n${p.out.trim().split('\n').slice(-6).join('\n')}`);
    log.error('r2: the deploy token needs "Workers R2 Storage: Edit" on this account (operator action); then re-run publish.sh --r2.');
    return REFUSED;
  }
  let failed = 0;
  mkdirSync(dirname(ledgerFile), { recursive: true });
  for (const [k, f] of todo) {
    const r = put(k, f);
    if (!r.ok) { failed++; log.error(`r2: FAILED ${k}: ${r.out.trim().split('\n').pop()}`); continue; }
    ledger[k] = sha(f);
    writeFileSync(ledgerFile, JSON.stringify(ledger, null, 1));   // per object: a killed run resumes
    log.log(`r2: put ${k}`);
  }
  log.log(`r2: ${todo.length - failed} uploaded, ${failed} failed, ${want.length - todo.length} unchanged`);
  return failed ? 1 : 0;
}

export function main(argv, { repo = REPO, media = STAGE, log = console, probe = ffprobe, r2 = {} } = {}) {
  if (argv.includes('--r2')) return toR2(media, { ...r2, dryRun: argv.includes('--dry-run'), log });
  const only = argv.includes('--manifest-only');
  const [out, ...ids] = argv.filter(a => a !== '--manifest-only');
  if (!only && !out) { log.error('usage: publish.mjs OUT_ROOT [ID...] | --manifest-only | --r2 [--dry-run]'); return 2; }
  let failed = 0;
  if (!only) {
    const all = ids.length ? ids : readdirSync(out).filter(d => existsSync(join(out, d, 'lesson.json'))).sort();
    for (const id of all) for (const lang of LANGS) {
      if (!existsSync(join(out, id, lang, 'video.mp4'))) continue;
      const bad = checkCut(join(out, id), lang, probe).filter(c => !c.ok);
      if (bad.length) { failed++; log.log(`HELD  ${id}/${lang}: ${bad.map(c => c.what + (c.got ? ` (${c.got})` : '')).join('; ')}`); continue; }
      const n = publishCut(out, id, lang, media);
      log.log(`${n ? 'PUB ' : 'SAME'}  ${id}/${lang}${n ? ` (${n} file(s))` : ''}`);
    }
  }
  const m = manifest(media), mf = join(repo, MANIFEST);
  mkdirSync(dirname(mf), { recursive: true });
  writeFileSync(mf, JSON.stringify(m, null, 1) + '\n');
  const cuts = Object.values(m.lessons).reduce((n, l) => n + Object.keys(l.cuts).length, 0);
  log.log(`publish: manifest names ${Object.keys(m.lessons).length} lesson(s), ${cuts} cut(s); ${failed} cut(s) held back`);
  return failed ? 1 : 0;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) process.exit(main(process.argv.slice(2)));
