// node --test tools/learn/publish.test.mjs -- the tree copy, the manifest, and the R2 upload
// (probe first, loud refusal, ledger skip) against a fake `put` and a fake spawn: no network.
import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, existsSync, cpSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';
import { publishCut, manifest, objects, toR2, wranglerPut, authError, typeOf, main, MANIFEST, STAGE, FILES, REFUSED, sha } from './publish.mjs';
import { makePlan, timeline } from './assemble-plan.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const FIX = join(HERE, 'fixtures', 'capture');
const lesson = JSON.parse(readFileSync(join(FIX, 'lesson.json'), 'utf8'));
const marks = JSON.parse(readFileSync(join(FIX, 'sq', 'marks.json'), 'utf8')).marks;
const quiet = { log: () => {}, error: () => {} };
const rec = () => { const lines = []; return { lines, log: l => lines.push(l), error: l => lines.push(l) }; };

/// An OUT root with one assembled cut (sq) of the fixture lesson; the media files are stubs.
function outRoot() {
  const out = mkdtempSync(join(tmpdir(), 'pub-'));
  const d = join(out, lesson.id);
  cpSync(FIX, d, { recursive: true });
  writeFileSync(join(d, 'source.sha256'), 'ab'.repeat(32) + '\n');
  makePlan(d, 'sq', { scripts: [] });
  for (const f of ['video.mp4', 'step-1.mp4', 'step-2.mp4', 'poster.jpg', 'contact.jpg']) writeFileSync(join(d, 'sq', f), `x-${f}`);
  return out;
}
const good = f => {
  const name = f.split('/').pop();
  if (name === 'video.mp4') return { streams: [{ codec_type: 'video', codec_name: 'h264', profile: 'High', width: 720, height: 1280, r_frame_rate: '24/1' }],
    format: { duration: String(timeline(marks).durationMs / 1000), bit_rate: '900000' }, chapters: lesson.steps.map(() => ({})) };
  if (name.startsWith('step-')) return { format: { duration: '4.0' } };
  return { streams: [{ codec_name: 'mjpeg', width: 720, height: 1280 }] };
};
const repo = () => mkdtempSync(join(tmpdir(), 'repo-'));

test('publishCut: copies a cut once; the second time writes nothing; a missing file is named', () => {
  const out = outRoot(), media = join(repo(), 'm');
  assert.equal(publishCut(out, lesson.id, 'sq', media), FILES.length + 2);
  assert.equal(publishCut(out, lesson.id, 'sq', media), 0);
  assert.throws(() => publishCut(out, lesson.id, 'en', media), /en\/video\.mp4 missing/);
});

test('manifest: only complete cuts, the source hash carried, every URL the gated route\'s', () => {
  const out = outRoot(), media = join(repo(), 'm');
  assert.deepEqual(manifest(media), { version: 1, lessons: {} });
  publishCut(out, lesson.id, 'sq', media);
  mkdirSync(join(media, 'Z9', 'sq'), { recursive: true });                      // an incomplete cut
  const m = manifest(media);
  assert.deepEqual(Object.keys(m.lessons), [lesson.id]);
  const l = m.lessons[lesson.id];
  assert.equal(l.source, 'ab'.repeat(32));
  assert.equal(l.role, lesson.role);
  assert.equal(l.cuts.sq.video, `/api/learn/media/${lesson.id}/sq/video.mp4`);
  assert.equal(l.cuts.sq.steps, lesson.steps.length);
  assert.equal(l.cuts.sq.subs.uk, `/api/learn/media/${lesson.id}/sq/subs_uk.vtt`);
  assert.doesNotMatch(JSON.stringify(m), /"\/learn\/media/);
  mkdirSync(join(media, 'Y1', 'sq'), { recursive: true });                       // no lesson.json / source: nulls, not a crash
  for (const f of FILES) cpSync(join(media, lesson.id, 'sq', f), join(media, 'Y1', 'sq', f));
  assert.deepEqual([manifest(media).lessons.Y1.role, manifest(media).lessons.Y1.source], [null, null]);
});

test('objects + typeOf: every file of every cut under learn/, typed', () => {
  const out = outRoot(), media = join(repo(), 'm');
  publishCut(out, lesson.id, 'sq', media);
  const o = objects(media);
  assert.equal(o.length, FILES.length);
  assert.equal(o[0][0], `learn/${lesson.id}/sq/video.mp4`);
  assert.deepEqual(['a.mp4', 'a.vtt', 'a.jpg', 'a.json', 'a.txt', 'a.bin'].map(typeOf),
    ['video/mp4', 'text/vtt; charset=utf-8', 'image/jpeg', 'application/json', 'text/plain', 'application/octet-stream']);
});

test('toR2: the probe goes first; a token without R2 write is REFUSED loudly and nothing is uploaded', () => {
  const out = outRoot(), media = join(repo(), 'm'), s = repo();
  publishCut(out, lesson.id, 'sq', media);
  const puts = [];
  const r = rec();
  const code = toR2(media, { put: k => { puts.push(k); return { ok: false, out: 'X [ERROR] Authentication error [code: 10000]' }; }, ledgerFile: join(s, 'l.json'), log: r, scratch: s });
  assert.equal(code, REFUSED);
  assert.deepEqual(puts, ['learn/.probe.txt']);
  assert.match(r.lines.join('\n'), /REFUSED -- the token cannot write R2 objects; NOTHING was uploaded/);
  assert.ok(!existsSync(join(s, 'l.json')));
  const n = rec();
  assert.equal(toR2(media, { put: () => ({ ok: false, out: 'ENOTFOUND' }), ledgerFile: join(s, 'l.json'), log: n, scratch: s }), REFUSED);
  assert.match(n.lines.join('\n'), /the probe upload failed/);
});

test('toR2: uploads what changed, manifest last, ledger skips the rest; a failed object is counted', () => {
  const out = outRoot(), media = join(repo(), 'm'), s = repo(), ledgerFile = join(s, 'c', 'l.json');
  publishCut(out, lesson.id, 'sq', media);
  const puts = [];
  const put = (k, f) => { puts.push([k, readFileSync(f, 'utf8')]); return { ok: true, out: '' }; };
  assert.equal(toR2(media, { put, ledgerFile, log: quiet, scratch: s }), 0);
  assert.equal(puts.length, 1 + FILES.length + 1);
  assert.equal(puts.at(-1)[0], 'learn/manifest.json');
  assert.match(puts.at(-1)[1], /\/api\/learn\/media\//);
  puts.length = 0;
  assert.equal(toR2(media, { put, ledgerFile, log: quiet, scratch: s }), 0);
  assert.deepEqual(puts.map(p => p[0]), ['learn/.probe.txt']);                   // nothing changed
  writeFileSync(join(media, lesson.id, 'sq', 'poster.jpg'), 'new');
  const failing = (k) => ({ ok: k === 'learn/.probe.txt' || k.endsWith('manifest.json'), out: 'boom\n500' });
  const r = rec();
  assert.equal(toR2(media, { put: failing, ledgerFile, log: r, scratch: s }), 1);
  assert.match(r.lines.join('\n'), /FAILED learn\/.*poster\.jpg: 500/);
  const d = rec();
  assert.equal(toR2(media, { put: () => { throw new Error('no put in a dry run'); }, ledgerFile: join(s, 'none.json'), log: d, dryRun: true, scratch: s }), 0);
  assert.match(d.lines.join('\n'), /would put learn\/manifest\.json/);
});

test('wranglerPut: the exact command, the exit code, tokens redacted; authError reads wrangler', () => {
  let seen;
  const spawn = (cmd, args, o) => { seen = { cmd, args, cwd: o.cwd }; return { status: 1, stdout: 'x', stderr: `token ${'a'.repeat(40)} Authentication error [code: 10000]` }; };
  const r = wranglerPut('learn/W3/sq/video.mp4', '/f/video.mp4', '/cwd', spawn, '');
  assert.equal(r.ok, false);
  assert.doesNotMatch(r.out, /a{40}/);
  assert.deepEqual([seen.cmd, seen.cwd], ['npx', '/cwd']);
  assert.deepEqual(seen.args.slice(0, 6), ['--yes', 'wrangler@4', 'r2', 'object', 'put', 'dowiz-learn/learn/W3/sq/video.mp4']);
  assert.ok(seen.args.includes('--remote') && seen.args.includes('video/mp4'));
  assert.equal(wranglerPut('k', 'f.vtt', '/c', () => ({ status: 0 })).ok, true);
  wranglerPut('k', 'f.jpg', '/c', spawn, '/w/bin/wrangler.js');                   // the direct binary, no npx
  assert.deepEqual([seen.cmd, seen.args.slice(0, 5)], [process.execPath, ['/w/bin/wrangler.js', 'r2', 'object', 'put', 'dowiz-learn/k']]);
  assert.match(wranglerPut('k', 'f', '/c', () => ({ status: null, error: new Error('spawn npx ENOENT') })).out, /ENOENT/);
  assert.equal(authError('Authentication error [code: 10000]'), true);
  assert.equal(authError('getaddrinfo ENOTFOUND'), false);
});

test('main: usage, publish + HELD into the staging tree, the manifest into the repo, manifest-only, --r2 routed', () => {
  const r0 = repo(), st = join(repo(), 'stage'), o = { repo: r0, media: st };
  assert.equal(main([], { ...o, log: quiet }), 2);
  const out = outRoot();
  const r = rec();
  assert.equal(main([out], { ...o, log: r, probe: good }), 0);
  assert.match(r.lines.join('\n'), /PUB   W3\/sq[\s\S]*1 lesson\(s\), 1 cut\(s\); 0 cut\(s\) held/);
  assert.ok(existsSync(join(st, 'W3', 'sq', 'video.mp4')));
  assert.ok(!existsSync(join(r0, 'workers/api/public/learn/media/W3')));          // no media under public/
  const r2 = rec();
  assert.equal(main([out, lesson.id], { ...o, log: r2, probe: good }), 0);
  assert.match(r2.lines[0], /SAME  W3\/sq/);
  const bad = f => ({ ...good(f), streams: [{ codec_type: 'video', codec_name: 'vp8' }] });
  const r3 = rec();
  assert.equal(main([out], { ...o, log: r3, probe: bad }), 1);
  assert.match(r3.lines[0], /HELD  W3\/sq: .*H\.264/);
  assert.equal(main(['--manifest-only'], { ...o, log: quiet }), 0);
  assert.ok(JSON.parse(readFileSync(join(r0, MANIFEST), 'utf8')).lessons.W3);
  const s = repo();
  assert.equal(main(['--r2', '--dry-run'], { ...o, log: quiet, r2: { ledgerFile: join(s, 'l.json'), scratch: s } }), 0);
  assert.equal(sha(join(r0, MANIFEST)).length, 64);
  assert.match(STAGE, /dowiz-learn/);
});

test('publish.mjs run as a script: no arguments is the usage and exit 2', async () => {
  const { spawnSync } = await import('node:child_process');
  const r = spawnSync(process.execPath, [join(HERE, 'publish.mjs')], { encoding: 'utf8', env: { ...process.env, LEARN_STAGE: '/nonexistent-stage' } });
  assert.equal(r.status, 2);
  assert.match(r.stderr, /usage: publish\.mjs/);
});

test('manifest: a step whose anchor was not on screen is listed as absent', () => {
  const out = outRoot(), media = join(repo(), 'm');
  publishCut(out, lesson.id, 'sq', media);
  const f = join(media, lesson.id, 'sq', 'chapters.json');
  const ch = JSON.parse(readFileSync(f, 'utf8')); ch.chapters[1].found = false; writeFileSync(f, JSON.stringify(ch));
  assert.deepEqual(manifest(media).lessons[lesson.id].cuts.sq.absent, [2]);
});
