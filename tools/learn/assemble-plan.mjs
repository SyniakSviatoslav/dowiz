#!/usr/bin/env node
// The pure half of tools/learn/assemble.sh: from one capture (lesson.json + <lang>/marks.json)
// it writes everything FFmpeg needs as files, and decides whether anything changed.
//
//   node tools/learn/assemble-plan.mjs DIR LANG    -> DIR/LANG/plan/* and prints shell lines
//
// Written into DIR/LANG/: subs_sq.vtt subs_en.vtt subs_uk.vtt (one cue per step, on THIS cut's
// timings), chapters.json, and plan/ (ffmeta.txt MP4 chapters, filter.txt the burn-in graph,
// title-*.txt the burned step titles, steps.tsv the loop cuts). Prints SKIP when the inputs'
// hash equals the one the last finished assembly left in DIR/LANG/.assembled.
import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';

export const LANGS = ['sq', 'en', 'uk'];
export const W = 720, H = 1280, BAND = 112, SW = 540, SH = 1168;
export const INK = '0x16130f', BONE = '0xf3ede2', MUTED = '0xb9b0a3', HOT = '0xff4d2e';
export const FONT = '/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf';
export const LEAD_MS = 600, LOOP_MAX_MS = 8000;
const HERE = dirname(fileURLToPath(import.meta.url));

/// 83456 -> "00:01:23.456"
export function vttTime(ms) {
  const t = Math.max(0, Math.round(ms));
  const h = Math.floor(t / 3600000), m = Math.floor(t / 60000) % 60, s = Math.floor(t / 1000) % 60;
  return `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}.${String(t % 1000).padStart(3, '0')}`;
}

/// The cut: step n runs from its mark to the next; the video starts LEAD_MS before step 1.
export function timeline(marks) {
  if (!marks.length) throw new Error('no marks: nothing was recorded');
  const t0 = Math.max(0, marks[0].startMs - LEAD_MS);
  const steps = marks.map(m => ({ n: m.n, key: m.key, startMs: m.startMs - t0, endMs: m.endMs - t0, found: m.found }));
  for (const s of steps) if (!(s.endMs > s.startMs)) throw new Error(`step ${s.n}: ends at ${s.endMs} ms, not after its start ${s.startMs}`);
  return { t0, t1: marks[marks.length - 1].endMs, durationMs: marks[marks.length - 1].endMs - t0, steps };
}

/// One WebVTT track: a cue per step with its caption; the title is the cue's first line.
export function vtt(lesson, tl, lang) {
  const cues = tl.steps.map(s => {
    const st = lesson.steps[s.n - 1];
    return `${s.n}\n${vttTime(s.startMs)} --> ${vttTime(s.endMs)}\n${clean(st.title[lang])}\n${clean(st.caption[lang])}`;
  });
  return `WEBVTT\n\n${cues.join('\n\n')}\n`;
}
const clean = t => String(t).replace(/-->/g, '→').replace(/\s*\n\s*/g, ' ').trim();

export function chapters(lesson, tl) {
  return tl.steps.map(s => ({ n: s.n, key: s.key, startMs: s.startMs, endMs: s.endMs, found: s.found,
    title: lesson.steps[s.n - 1].title }));
}

/// MP4 chapter metadata (ffmetadata), titles in the cut's language.
export function ffmeta(lesson, tl, lang) {
  const esc = t => clean(t).replace(/([=;#\\])/g, '\\$1');
  return `;FFMETADATA1\ntitle=${esc(`${lesson.id} ${lesson.title[lang]}`)}\nlanguage=${lang}\n` + tl.steps.map(s =>
    `\n[CHAPTER]\nTIMEBASE=1/1000\nSTART=${s.startMs}\nEND=${s.endMs}\ntitle=${esc(lesson.steps[s.n - 1].title[lang])}\n`).join('');
}

/// A font size that keeps a title inside the band's 640 px.
export const fitSize = (text, max = 34, min = 20) => Math.max(min, Math.min(max, Math.floor(640 / (0.62 * Math.max(1, [...text].length)))));

/// The burn-in graph: trim, 24 fps, the screen scaled into the ink canvas, the lesson title,
/// and per step its title and n/N, each shown only during its own step.
export function filter(lesson, tl, lang, planDir) {
  const n = tl.steps.length;
  const q = p => `'${p.replace(/'/g, "'\\''")}'`;
  const parts = [`[0:v]trim=start=${tl.t0 / 1000}:end=${tl.t1 / 1000},setpts=PTS-STARTPTS,fps=24`,
    `scale=${SW}:${SH}:flags=lanczos,pad=${W}:${H}:${(W - SW) / 2}:${BAND}:color=${INK}`,
    `drawtext=fontfile=${FONT}:textfile=${q(join(planDir, 'title-0.txt'))}:x=40:y=18:fontsize=20:fontcolor=${MUTED}`];
  for (const s of tl.steps) {
    const t = clean(lesson.steps[s.n - 1].title[lang]);
    const en = `enable='between(t,${s.startMs / 1000},${s.endMs / 1000})'`;
    parts.push(`drawtext=fontfile=${FONT}:textfile=${q(join(planDir, `title-${s.n}.txt`))}:x=40:y=56:fontsize=${fitSize(t)}:fontcolor=${BONE}:${en}`);
    parts.push(`drawtext=fontfile=${FONT}:text='${s.n}/${n}':x=w-tw-40:y=22:fontsize=20:fontcolor=${HOT}:${en}`);
  }
  return parts.join(',\n') + ',\nformat=yuv420p[v]\n';
}

/// Loop cuts: from each step's start, at most LOOP_MAX_MS.
export const loops = tl => tl.steps.map(s => [s.n, s.startMs / 1000, Math.min(s.endMs - s.startMs, LOOP_MAX_MS) / 1000]);

/// sha256 over every input of an assembly, the scripts included.
export function inputsHash(paths) {
  const h = createHash('sha256');
  for (const p of paths) { h.update(p.split('/').pop() + '\0'); h.update(readFileSync(p)); h.update('\0'); }
  return h.digest('hex');
}

/// Write the plan for DIR/LANG. Answers { skip, hash, tl, lines } -- `lines` are for the shell.
export function makePlan(dir, lang, { force = false, scripts = [join(HERE, 'assemble.sh'), join(HERE, 'assemble-plan.mjs')] } = {}) {
  if (!LANGS.includes(lang)) throw new Error(`language ${lang} is not one of ${LANGS.join('/')}`);
  const cut = join(dir, lang), planDir = join(cut, 'plan');
  const lesson = JSON.parse(readFileSync(join(dir, 'lesson.json'), 'utf8'));
  const m = JSON.parse(readFileSync(join(cut, 'marks.json'), 'utf8'));
  if (m.id !== lesson.id || m.lang !== lang) throw new Error(`marks.json is ${m.id}/${m.lang}, not ${lesson.id}/${lang}`);
  if (m.marks.length !== lesson.steps.length) throw new Error(`marks.json has ${m.marks.length} steps, the lesson ${lesson.steps.length}`);
  const hash = inputsHash([join(dir, 'lesson.json'), join(cut, 'marks.json'), join(cut, 'raw.webm'), ...scripts.filter(existsSync)]);
  const done = existsSync(join(cut, '.assembled')) ? readFileSync(join(cut, '.assembled'), 'utf8').trim() : '';
  const tl = timeline(m.marks);
  if (!force && done === hash && existsSync(join(cut, 'video.mp4'))) return { skip: true, hash, tl, lines: ['SKIP=1', `HASH=${hash}`] };
  mkdirSync(planDir, { recursive: true });
  for (const l of LANGS) writeFileSync(join(cut, `subs_${l}.vtt`), vtt(lesson, tl, l));
  writeFileSync(join(cut, 'chapters.json'), JSON.stringify({ id: lesson.id, lang, durationMs: tl.durationMs, chapters: chapters(lesson, tl) }, null, 1));
  writeFileSync(join(planDir, 'ffmeta.txt'), ffmeta(lesson, tl, lang));
  writeFileSync(join(planDir, 'title-0.txt'), clean(`${lesson.id} · ${lesson.title[lang]}`));
  for (const s of tl.steps) writeFileSync(join(planDir, `title-${s.n}.txt`), clean(lesson.steps[s.n - 1].title[lang]));
  writeFileSync(join(planDir, 'filter.txt'), filter(lesson, tl, lang, planDir));
  writeFileSync(join(planDir, 'steps.tsv'), loops(tl).map(r => r.join('\t')).join('\n') + '\n');
  const poster = Math.min(tl.steps[0].startMs + 1200, tl.steps[0].endMs - 100) / 1000;
  const sheet = tl.steps.map(s => ((s.startMs + (s.endMs - s.startMs) * 0.6) / 1000).toFixed(3)).join(' ');
  return { skip: false, hash, tl, lines: ['SKIP=0', `HASH=${hash}`, `STEPS=${tl.steps.length}`, `DURATION=${tl.durationMs / 1000}`,
    `POSTER=${poster}`, `SHEET="${sheet}"`, `COLS=${Math.min(tl.steps.length, 5)}`] };
}

export function main(argv, log = console) {
  const force = argv.includes('--force');
  const [dir, lang] = argv.filter(a => a !== '--force');
  if (!dir || !lang) { log.error('usage: assemble-plan.mjs DIR LANG [--force]'); return 2; }
  try { log.log(makePlan(dir, lang, { force }).lines.join('\n')); return 0; }
  catch (e) { log.error(`assemble-plan: ${e.message}`); return 1; }
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) process.exit(main(process.argv.slice(2)));
