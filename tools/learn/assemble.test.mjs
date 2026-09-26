// node --test tools/learn/assemble.test.mjs -- the assembly plan and the ffprobe checks, on a
// fixture capture (tools/learn/fixtures/) and a fake probe; one real ffmpeg run proves the
// shell end to end on a synthetic two-step recording (skipped, loudly, without ffmpeg).
import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, cpSync, readFileSync, writeFileSync, existsSync, statSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { tmpdir } from 'node:os';
import { execFileSync, spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { vttTime, timeline, vtt, chapters, ffmeta, fitSize, filter, loops, makePlan, main as planMain, LEAD_MS } from './assemble-plan.mjs';
import { cues, checkCut, main as checkMain } from './check.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const FIX = join(HERE, 'fixtures', 'capture');
const lesson = JSON.parse(readFileSync(join(FIX, 'lesson.json'), 'utf8'));
const marks = JSON.parse(readFileSync(join(FIX, 'sq', 'marks.json'), 'utf8')).marks;
const quiet = { log: () => {}, error: () => {} };
const scratch = () => { const d = mkdtempSync(join(tmpdir(), 'asm-')); cpSync(FIX, d, { recursive: true }); return d; };

test('vttTime', () => {
  assert.equal(vttTime(0), '00:00:00.000');
  assert.equal(vttTime(3723456), '01:02:03.456');
  assert.equal(vttTime(-5), '00:00:00.000');
});

test('timeline: starts LEAD_MS before step 1; a step that does not move forward is refused', () => {
  const tl = timeline(marks);
  assert.equal(tl.t0, marks[0].startMs - LEAD_MS);
  assert.equal(tl.steps[0].startMs, LEAD_MS);
  assert.equal(tl.durationMs, marks.at(-1).endMs - tl.t0);
  assert.equal(timeline([{ n: 1, key: '1', startMs: 100, endMs: 900 }]).t0, 0);
  assert.throws(() => timeline([]), /no marks/);
  assert.throws(() => timeline([{ n: 1, startMs: 900, endMs: 900 }]), /step 1/);
});

test('vtt + chapters + ffmeta: one cue and one chapter per step, in every language', () => {
  const tl = timeline(marks);
  for (const l of ['sq', 'en', 'uk']) {
    const c = cues(vtt(lesson, tl, l));
    assert.equal(c.length, lesson.steps.length);
    assert.match(c[0].text, new RegExp(lesson.steps[0].caption[l].slice(0, 12)));
  }
  assert.equal(cues(vtt({ ...lesson, steps: lesson.steps.map(s => ({ ...s, caption: { ...s.caption, en: 'a --> b\nc' } })) }, tl, 'en'))[0].text.includes('-->'), false);
  const ch = chapters(lesson, tl);
  assert.deepEqual(ch.map(c => c.n), lesson.steps.map(s => s.n));
  const m = ffmeta(lesson, tl, 'uk');
  assert.equal((m.match(/\[CHAPTER\]/g) || []).length, lesson.steps.length);
  assert.match(ffmeta({ ...lesson, title: { ...lesson.title, en: 'a=b;c' } }, tl, 'en'), /a\\=b\\;c/);
});

test('fitSize: long titles shrink, never below the floor', () => {
  assert.equal(fitSize('Open'), 34);
  assert.ok(fitSize('x'.repeat(40)) < 34);
  assert.equal(fitSize('x'.repeat(400)), 20);
});

test('filter: every step titled only during its own step, the screen padded into 720x1280', () => {
  const tl = timeline(marks);
  const f = filter(lesson, tl, 'sq', "/p/it's");
  assert.match(f, /pad=720:1280:90:112/);
  assert.equal((f.match(/enable='between/g) || []).length, lesson.steps.length * 2);
  assert.match(f, /it'\\''s/);
  assert.match(f, /format=yuv420p\[v\]/);
  assert.deepEqual(loops({ steps: [{ n: 1, startMs: 0, endMs: 20000 }] }), [[1, 0, 8]]);
});

test('makePlan: writes the tracks, then SKIPs on the same inputs; refuses a mismatched capture', () => {
  const d = scratch();
  const p = makePlan(d, 'sq', { scripts: [] });
  assert.equal(p.skip, false);
  for (const f of ['subs_sq.vtt', 'subs_en.vtt', 'subs_uk.vtt', 'chapters.json', 'plan/filter.txt', 'plan/ffmeta.txt', 'plan/steps.tsv', 'plan/title-1.txt'])
    assert.ok(existsSync(join(d, 'sq', f)), f);
  writeFileSync(join(d, 'sq', 'video.mp4'), 'x'); writeFileSync(join(d, 'sq', '.assembled'), p.hash);
  assert.equal(makePlan(d, 'sq', { scripts: [] }).skip, true);
  assert.equal(makePlan(d, 'sq', { scripts: [], force: true }).skip, false);
  writeFileSync(join(d, 'sq', 'raw.webm'), 'changed');
  assert.equal(makePlan(d, 'sq', { scripts: [] }).skip, false);
  assert.throws(() => makePlan(d, 'de'), /language de/);
  assert.throws(() => makePlan(d, 'en'), /ENOENT|marks/);
  const m = JSON.parse(readFileSync(join(d, 'sq', 'marks.json'), 'utf8'));
  writeFileSync(join(d, 'sq', 'marks.json'), JSON.stringify({ ...m, lang: 'uk' }));
  assert.throws(() => makePlan(d, 'sq', { scripts: [] }), /not W3\/sq/);
  writeFileSync(join(d, 'sq', 'marks.json'), JSON.stringify({ ...m, marks: m.marks.slice(1) }));
  assert.throws(() => makePlan(d, 'sq', { scripts: [] }), /1 steps, the lesson 2/);
  assert.equal(planMain([], quiet), 2);
  assert.equal(planMain([d, 'sq'], quiet), 1);
});

test('cues: a malformed timing line is an error, not a skipped cue', () => {
  assert.throws(() => cues('nope'), /not WEBVTT/);
  assert.throws(() => cues('WEBVTT\n\n1\n00:00:01 --> 00:00:02\nx\n'), /bad timing/);
  assert.deepEqual(cues('WEBVTT\n\nNOTE hi\n\n1\n00:00:01.000 --> 00:00:02.500\na\nb\n'), [{ startMs: 1000, endMs: 2500, text: 'a b' }]);
});

/// A fake ffprobe answering what a good cut measures; `over` overrides per file name.
const fake = (over = {}) => f => {
  const name = f.split('/').pop();
  if (over[name]) return over[name];
  if (name === 'video.mp4') return { streams: [{ codec_type: 'video', codec_name: 'h264', profile: 'High', width: 720, height: 1280, r_frame_rate: '24/1' }],
    format: { duration: String(timeline(marks).durationMs / 1000), bit_rate: '900000' }, chapters: lesson.steps.map(() => ({})) };
  if (name.startsWith('step-')) return { format: { duration: '4.0' } };
  return { streams: [{ codec_name: 'mjpeg', width: 720, height: 1280 }] };
};
function assembled() {
  const d = scratch();
  makePlan(d, 'sq', { scripts: [] });
  for (const f of ['video.mp4', 'step-1.mp4', 'step-2.mp4', 'poster.jpg', 'contact.jpg']) writeFileSync(join(d, 'sq', f), 'x');
  return d;
}

test('checkCut: a good cut passes every check', () => {
  const r = checkCut(assembled(), 'sq', fake());
  assert.deepEqual(r.filter(c => !c.ok), []);
  assert.ok(r.length >= 18);
});

test('checkCut: each defect is caught by name', () => {
  const d = assembled();
  const bad = { 'video.mp4': { streams: [{ codec_type: 'video', codec_name: 'vp8', width: 390, height: 844, r_frame_rate: '30/1' }, { codec_type: 'audio' }],
    format: { duration: '1', bit_rate: '2000000' }, chapters: [] }, 'step-1.mp4': { format: { duration: '12' } }, 'poster.jpg': { streams: [] } };
  const fails = checkCut(d, 'sq', fake(bad)).filter(c => !c.ok).map(c => c.what).join('\n');
  for (const w of ['H.264', '720x1280', '24 fps', '1400 kbps', 'silent', 'MP4 chapters', 'duration', 'cues are inside', 'step-1.mp4', 'poster.jpg'])
    assert.match(fails, new RegExp(w.replace('.', '\\.')));
  const ch = JSON.parse(readFileSync(join(d, 'sq', 'chapters.json'), 'utf8'));
  ch.chapters[1].found = false; writeFileSync(join(d, 'sq', 'chapters.json'), JSON.stringify(ch));
  const warn = [];
  assert.deepEqual(checkCut(d, 'sq', fake(), warn).filter(c => !c.ok), []);          // one of two absent: a warning
  assert.deepEqual(warn, ['WARN sq anchor not on screen: step 2']);
  ch.chapters[0].found = false; writeFileSync(join(d, 'sq', 'chapters.json'), JSON.stringify(ch));
  assert.match(checkCut(d, 'sq', fake()).filter(c => !c.ok).map(c => c.got).join(), /absent: step 1, 2/);
  writeFileSync(join(d, 'sq', 'subs_en.vtt'), 'garbage');
  assert.match(checkCut(d, 'sq', fake()).filter(c => !c.ok).map(c => c.what).join(), /subs_en\.vtt parses/);
});

test('checkCut + main: missing files and unusable input', () => {
  const d = scratch();
  assert.match(checkCut(d, 'sq', fake()).map(c => c.what + c.got).join(), /chapters\.json exists.*missing/);
  assert.match(checkCut('/nonexistent', 'sq', fake())[0].what, /lesson\.json/);
  assert.equal(checkMain([], quiet), 2);
  assert.equal(checkMain([d, 'de'], quiet), 2);
  assert.equal(checkMain([assembled(), 'sq'], quiet, fake()), 0);
});

const hasFF = spawnSync('ffmpeg', ['-version']).status === 0;
test('assemble.sh + check.sh end to end on a synthetic recording; the re-run does nothing', { skip: hasFF ? false : 'ffmpeg not installed' }, () => {
  const d = scratch();
  execFileSync('ffmpeg', ['-nostdin', '-loglevel', 'error', '-y', '-f', 'lavfi', '-i', 'testsrc=size=390x844:rate=25:duration=12', '-c:v', 'libvpx', '-b:v', '300k', join(d, 'sq', 'raw.webm')]);
  const out = execFileSync('sh', [join(HERE, 'assemble.sh'), d, 'sq'], { encoding: 'utf8' });
  assert.match(out, /video\.mp4 \(\d+ bytes/);
  assert.match(execFileSync('sh', [join(HERE, 'assemble.sh'), d, 'sq'], { encoding: 'utf8' }), /unchanged/);
  const r = spawnSync('sh', [join(HERE, 'check.sh'), d], { encoding: 'utf8' });
  assert.equal(r.status, 0, r.stdout);
  assert.ok(statSync(join(d, 'sq', 'contact.jpg')).size > 0);
  const bad = spawnSync('sh', [join(HERE, 'assemble.sh'), d, 'de'], { encoding: 'utf8' });
  assert.notEqual(bad.status, 0);
});

test('checkCut + main: a probe that answers nothing fails every measured check; main prints FAIL, the value and the warnings', () => {
  const d = assembled();
  const bad = checkCut(d, 'sq', () => ({})).filter(c => !c.ok).map(c => c.what).join('\n');
  for (const w of ['H.264', '720x1280', '24 fps', 'kbps', 'MP4 chapters', 'duration', 'step-1.mp4', 'poster.jpg is an image', 'contact.jpg is an image'])
    assert.match(bad, new RegExp(w.replace('.', '\\.')));
  const ch = JSON.parse(readFileSync(join(d, 'sq', 'chapters.json'), 'utf8'));
  ch.chapters[1].found = false; writeFileSync(join(d, 'sq', 'chapters.json'), JSON.stringify(ch));
  const lines = [];
  const n = checkMain([d, 'sq'], { log: l => lines.push(l), error: l => lines.push(l) }, f => f.endsWith('video.mp4') ? {} : fake()(f));
  assert.ok(n > 0);
  assert.match(lines.join('\n'), /FAIL sq video is H\.264 High :: undefined undefined/);
  assert.match(lines.join('\n'), /WARN sq anchor not on screen: step 2\ncheck: \d+ failed, 1 warning/);
});
