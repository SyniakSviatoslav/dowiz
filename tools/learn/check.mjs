#!/usr/bin/env node
// The self-review of an assembled cut (tools/learn/check.sh is the entry point).
//
//   node tools/learn/check.mjs DIR LANG [LANG...]     exit = number of failed checks
//
// Every published file is measured with ffprobe, never trusted from the script that wrote it:
// the video's codec, size, rate, bitrate and chapters; every caption track's cue count and
// bounds; chapters.json against the lesson; each loop's size and duration; the poster and the
// contact sheet. One line per check, PASS or FAIL with the measured value.
import { readFileSync, existsSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

export const LANGS = ['sq', 'en', 'uk'];
export const MAX_KBPS = 1400, LOOP_MAX = 409600, TOL_MS = 500;

/// ffprobe as JSON; injectable so the tests run on a fake.
export function probe(file) {
  return JSON.parse(execFileSync('ffprobe', ['-v', 'error', '-print_format', 'json', '-show_format', '-show_streams', '-show_chapters', file], { encoding: 'utf8' }));
}

const ms = t => { const [h, m, s] = t.split(':'); return Math.round((Number(h) * 3600 + Number(m) * 60 + Number(s)) * 1000); };
/// WebVTT -> [{ startMs, endMs, text }]; a malformed timing line is an error.
export function cues(text) {
  if (!text.startsWith('WEBVTT')) throw new Error('not WEBVTT');
  const out = [];
  for (const block of text.split(/\n\n+/).slice(1)) {
    const lines = block.split('\n').filter(Boolean);
    const i = lines.findIndex(l => l.includes('-->'));
    if (i < 0) continue;
    const m = lines[i].match(/^(\d\d:\d\d:\d\d\.\d{3}) --> (\d\d:\d\d:\d\d\.\d{3})$/);
    if (!m) throw new Error(`bad timing line: ${lines[i]}`);
    out.push({ startMs: ms(m[1]), endMs: ms(m[2]), text: lines.slice(i + 1).join(' ') });
  }
  return out;
}

/// All checks for one cut. Answers [{ ok, what, got }].
export function checkCut(dir, lang, p = probe, warn = []) {
  const cut = join(dir, lang), r = [];
  const add = (ok, what, got = '') => r.push({ ok: !!ok, what, got: String(got) });
  const need = f => { const e = existsSync(join(cut, f)); if (!e) add(false, `${lang}/${f} exists`, 'missing'); return e; };
  let lesson, chap;
  try { lesson = JSON.parse(readFileSync(join(dir, 'lesson.json'), 'utf8')); } catch (e) { add(false, 'lesson.json readable', e.message); return r; }
  const n = lesson.steps.length;
  if (need('chapters.json')) {
    chap = JSON.parse(readFileSync(join(cut, 'chapters.json'), 'utf8'));
    add(chap.chapters.length === n, `${lang} chapters.json has one chapter per step`, `${chap.chapters.length}/${n}`);
    add(chap.chapters.every((c, i) => i === 0 || c.startMs >= chap.chapters[i - 1].endMs - 1), `${lang} chapters are in order`, chap.chapters.map(c => c.startMs).join(','));
    // A step whose control was not on screen still shows its caption over the screen it is
    // about (a status chip hidden while there is nothing to show, a menu the venue lacks);
    // it is listed as a warning and in the manifest. More than half absent is no lesson.
    const absent = chap.chapters.filter(c => c.found === false).map(c => c.n);
    add(absent.length * 2 <= n, `${lang} at least half the steps' anchors were on screen`, absent.length ? `absent: step ${absent.join(', ')}` : 'all');
    if (absent.length && absent.length * 2 <= n) warn.push(`WARN ${lang} anchor not on screen: step ${absent.join(', ')}`);
  }
  if (need('video.mp4')) {
    const v = p(join(cut, 'video.mp4'));
    const s = (v.streams || []).find(x => x.codec_type === 'video') || {};
    const dur = Number(v.format?.duration || 0) * 1000;
    const kbps = Number(v.format?.bit_rate || 0) / 1000;
    const [a, b] = String(s.r_frame_rate || '0/1').split('/').map(Number);
    add(s.codec_name === 'h264' && s.profile === 'High', `${lang} video is H.264 High`, `${s.codec_name} ${s.profile}`);
    add(s.width === 720 && s.height === 1280, `${lang} video is 720x1280`, `${s.width}x${s.height}`);
    add(b && a / b === 24, `${lang} video is 24 fps`, s.r_frame_rate);
    add(kbps > 0 && kbps <= MAX_KBPS, `${lang} video <= ${MAX_KBPS} kbps`, kbps.toFixed(0));
    add(!(v.streams || []).some(x => x.codec_type === 'audio'), `${lang} video is silent (no audio stream)`, (v.streams || []).map(x => x.codec_type).join(','));
    add((v.chapters || []).length === n, `${lang} video carries ${n} MP4 chapters`, (v.chapters || []).length);
    if (chap) add(Math.abs(dur - chap.durationMs) <= TOL_MS, `${lang} duration = chapters' end +-${TOL_MS} ms`, `${dur.toFixed(0)} vs ${chap.durationMs}`);
    for (const l of LANGS) if (need(`subs_${l}.vtt`)) {
      let c; try { c = cues(readFileSync(join(cut, `subs_${l}.vtt`), 'utf8')); } catch (e) { add(false, `${lang}/subs_${l}.vtt parses`, e.message); continue; }
      add(c.length === n, `${lang}/subs_${l}.vtt has a cue per step`, `${c.length}/${n}`);
      add(c.every(x => x.endMs > x.startMs && x.endMs <= dur + TOL_MS && x.text.trim()), `${lang}/subs_${l}.vtt cues are inside the video and not empty`, c.map(x => x.endMs).join(','));
    }
  }
  for (let i = 1; i <= n; i++) if (need(`step-${i}.mp4`)) {
    const size = statSync(join(cut, `step-${i}.mp4`)).size;
    const d = Number(p(join(cut, `step-${i}.mp4`)).format?.duration || 0);
    add(size <= LOOP_MAX && d > 0 && d <= 8.5, `${lang}/step-${i}.mp4 <= 400 KB and <= 8 s`, `${size} B ${d.toFixed(2)} s`);
  }
  for (const f of ['poster.jpg', 'contact.jpg']) if (need(f)) {
    const s = (p(join(cut, f)).streams || [])[0] || {};
    add(s.width > 0 && s.height > 0, `${lang}/${f} is an image`, `${s.codec_name} ${s.width}x${s.height}`);
  }
  return r;
}

export function main(argv, log = console, p = probe) {
  const [dir, ...langs] = argv;
  if (!dir || !langs.length || langs.some(l => !LANGS.includes(l))) { log.error('usage: check.mjs DIR LANG [LANG...]   (LANG in sq/en/uk)'); return 2; }
  let fails = 0;
  const warn = [];
  for (const lang of langs) for (const c of checkCut(dir, lang, p, warn)) {
    if (!c.ok) fails++;
    log.log(`${c.ok ? 'PASS' : 'FAIL'} ${c.what}${c.got ? ' :: ' + c.got : ''}`);
  }
  for (const w of warn) log.log(w);
  log.log(`check: ${fails} failed, ${warn.length} warning(s)`);
  return fails;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) process.exit(main(process.argv.slice(2)));
