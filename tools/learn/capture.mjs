#!/usr/bin/env node
// Record one lesson as a silent phone video, per language, with per-step marks.
//
//   node tools/learn/capture.mjs W3 --out DIR [--lang sq,en,uk] [--ui local|live] [--host URL] [--allow-writes]
//
// Reads docs/learn/lessons/<role>/<id>.yaml (or --lesson FILE), signs in on the venue with
// the role's credentials from /root/.dowiz_owner (read here, never printed), drives each
// step's action on its data-tour anchor and writes, per language:
//   DIR/<lang>/raw.webm     Playwright's recording (VP8) at the viewport's size (390x844)
//   DIR/<lang>/marks.json   { step, startMs, endMs, found, blocked } per step
//   DIR/lesson.json         the normalized lesson the marks refer to (assemble.sh reads it)
//
// SAFETY. Recordings run on the REAL venue (operator, 2026-09-24), so: any host but
// dubin-sushi is refused unless --host names it; by default NO step can write (every
// mutating /api/ call is aborted and listed in its mark). Only with --allow-writes do a
// `writes: yes` step's calls go out; then the venue's open sittings and orders are compared
// before and after -- anything new is closed (rejected / reported) and the exit code is the
// count still open. `--ui local` (the default while the anchors are undeployed) answers the
// app's static files from this working tree; the API is always the venue's.
// ONE browser at a time, single-process flags: the box dies above 32 processes.
import { chromium } from 'playwright';
import { mkdirSync, writeFileSync, readFileSync, renameSync, readdirSync, rmSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { parseArgs, checkHost, lessonFile, loadLesson, APPS, PHONE, DESKTOP, LOCATION, REPO,
  guard, localAsset, typeOf, redact, plan, mark, closeMarks, stepFound, sourceSha } from './capture-lib.mjs';
import { snapshot, diff, closeNew, signIn, seed } from './capture-venue.mjs';

const opt = parseArgs(process.argv.slice(2));
if (opt.error) { console.error(`capture: ${opt.error}`); process.exit(2); }
const refused = checkHost(opt.host, opt.hostGiven);
if (refused) { console.error(`capture: ${refused}`); process.exit(2); }
const file = opt.lesson ? resolve(opt.lesson) : lessonFile(opt.id);
if (!file) { console.error(`capture: no lesson ${opt.id} under docs/learn/lessons/`); process.exit(2); }
const { lesson, errors } = loadLesson(file);
if (errors) { for (const e of errors) console.error(`capture: ${e}`); process.exit(2); }
const app = APPS[lesson.role];
const steps = plan(lesson);
const view = opt.desktop ? DESKTOP : PHONE;
const OUT = resolve(opt.out);
mkdirSync(OUT, { recursive: true });
writeFileSync(join(OUT, 'lesson.json'), JSON.stringify(lesson, null, 1));
writeFileSync(join(OUT, 'source.sha256'), sourceSha(file) + '\n');   // what the video was made from (tools/gates/learn.sh item 5)
for (const s of steps) console.log(`plan ${lesson.id}.${s.n} ${s.do.padEnd(5)} ${s.anchor || '(card)'}${s.writes ? ' WRITES' : ''}${s.why ? ' -- ' + s.why : ''}`);
if (opt.dryRun) process.exit(0);

const procs = () => readdirSync('/proc').filter(n => /^\d+$/.test(n)).length;
if (procs() > 24) { console.error(`capture: ${procs()} processes > 24, refusing to launch a browser`); process.exit(3); }
const creds = Object.fromEntries(readFileSync('/root/.dowiz_owner', 'utf8').split('\n')
  .filter(l => l.startsWith('export ')).map(l => { const s = l.slice(7); const i = s.indexOf('=');
    return [s.slice(0, i), s.slice(i + 1).replace(/^['"]|['"]$/g, '')]; }));
const PUBLIC = join(REPO, 'workers/api/public');
const loc = LOCATION[opt.host] || null;

/// One route handler: the write guard for the API, local files for the UI.
async function routes(ctx, cur) {
  await ctx.route('**/*', async route => {
    const r = route.request(), u = new URL(r.url());
    if (u.origin === opt.host && u.pathname.startsWith('/api/')) {
      const g = guard(cur.step, r.method(), r.url(), opt.allowWrites);
      if (g === 'block') { cur.blocked.push(`${r.method()} ${u.pathname}`); return route.abort('blockedbyclient'); }
      if (g === 'record') cur.wrote.push(`${r.method()} ${u.pathname}`);
      return route.continue();
    }
    const f = opt.ui === 'local' && u.origin === opt.host ? localAsset(u.pathname, PUBLIC) : null;
    if (!f) return route.continue();
    const live = await route.fetch().catch(() => null);
    const headers = { ...(live ? live.headers() : {}), 'content-type': typeOf(f), 'cache-control': 'no-store' };
    delete headers['content-length']; delete headers['content-encoding']; delete headers.etag;
    return route.fulfill({ status: 200, headers, body: readFileSync(f) });
  });
}

/// A tap ring and a step highlight drawn in the page (a headless capture has no pointer).
const overlay = () => {
  const mk = () => {
    if (document.getElementById('__lr')) return;
    const r = document.createElement('div'); r.id = '__lr';
    Object.assign(r.style, { position: 'fixed', zIndex: 2147483647, pointerEvents: 'none', border: '3px solid #ff4d2e',
      borderRadius: '12px', boxShadow: '0 0 0 4px rgba(255,77,46,.25)', transition: 'all .25s ease', display: 'none' });
    document.documentElement.appendChild(r);
  };
  window.__ring = (x, y, w, h) => { mk(); const r = document.getElementById('__lr');
    Object.assign(r.style, { display: 'block', left: `${x - 4}px`, top: `${y - 4}px`, width: `${w + 8}px`, height: `${h + 8}px` }); };
  window.__unring = () => { const r = document.getElementById('__lr'); if (r) r.style.display = 'none'; };
};

const dwell = s => Math.min(6500, Math.max(2800, 45 * Math.max(...Object.values(s.caption).map(c => c.length))));

const ARGS = ['--no-sandbox', '--disable-dev-shm-usage', '--no-zygote', '--single-process',
  '--renderer-process-limit=1', '--disable-gpu', '--disable-3d-apis', '--disable-extensions'];
let browser = null;   // one per language: with --single-process a browser holds one context
let open = 0;
try {
  const who = await signIn(opt.host, lesson.role, creds);
  if (!who.token) throw new Error(`sign-in for ${lesson.role} answered ${who.status}`);
  // A read-only lesson cannot write (the guard aborts every mutating call), so it needs no
  // before/after proof. A writing one is compared EVEN with writes blocked -- the proof that
  // nothing was left open is measured, not inferred from the guard -- and what it left is closed.
  const before = lesson.writes ? await snapshot(opt.host, loc, who) : null;
  for (const lang of opt.langs) {
    const dir = join(OUT, lang); rmSync(dir, { recursive: true, force: true }); mkdirSync(dir, { recursive: true });
    const cur = { step: null, blocked: [], wrote: [] };
    const base = { viewport: view, deviceScaleFactor: 2, isMobile: !opt.desktop, hasTouch: !opt.desktop, serviceWorkers: 'block', locale: lang };
    // Signed in once by the API, the app's storage seeded: no credential is typed on camera.
    // One context only: with --single-process a second context's page dies.
    const store = seed(lesson.role, who);
    browser = await chromium.launch({ args: ARGS });
    const ctx = await browser.newContext({ ...base, recordVideo: { dir, size: { width: view.width, height: view.height } } });
    await routes(ctx, cur);
    await ctx.addInitScript(([k, v, st]) => {
      try { localStorage.setItem(k, v); for (const [a, b] of Object.entries(st.local)) localStorage.setItem(a, b);
        for (const [a, b] of Object.entries(st.session)) sessionStorage.setItem(a, b); } catch {}
    }, [app.langKey, lang, store]);
    await ctx.addInitScript(overlay);
    const p = await ctx.newPage();
    const t0 = Date.now();
    const errs = []; p.on('pageerror', e => errs.push(redact(e.message).slice(0, 200)));
    await p.goto(opt.host + app.path, { waitUntil: 'domcontentloaded', timeout: 90000 });
    await p.waitForSelector(app.ready, { timeout: 40000 }).catch(() => errs.push(`ready selector ${app.ready} never appeared`));
    await p.waitForTimeout(2500);
    if (lesson.role === 'guest') for (let i = 0; i < 4; i++) {   // the storefront's first-visit sheets (install hint)
      const open = await p.evaluate(() => document.getElementById('sheet')?.dataset.name || '').catch(() => '');
      if (!open) break;
      const later = await p.$('#insLater');
      if (later) await later.click().catch(() => {}); else await p.evaluate(() => document.getElementById('scrim')?.click());
      await p.waitForTimeout(600);
    }
    const marks = [];
    for (const s of lesson.steps) {
      const st = steps[s.n - 1];
      cur.step = s; cur.blocked = []; cur.wrote = [];
      const start = Date.now() - t0;
      let el = null;
      if (st.do !== 'skip' && st.selector) {
        el = await p.waitForSelector(st.selector, { state: 'visible', timeout: 6000 }).catch(() => null);
        if (el) {
          await el.scrollIntoViewIfNeeded().catch(() => {});
          const b = await el.boundingBox();
          if (b) await p.evaluate(([x, y, w, h]) => window.__ring(x, y, w, h), [b.x, b.y, b.width, b.height]);
          await p.waitForTimeout(900);
          if (st.do === 'click') {
            // A covered control (a sheet's scrim, a sticky bar) times Playwright's click out; the
            // DOM click is the fallback, and the fallback is listed so the frame can be checked.
            await el.click({ timeout: 4000 }).catch(async e => {
              errs.push(`step ${s.n} click fell back to a DOM click: ${e.message.split('\n')[0]}`);
              await el.evaluate(x => x.click()).catch(e2 => errs.push(`step ${s.n} DOM click: ${e2.message.split('\n')[0]}`));
            });
            await p.evaluate(() => window.__unring && window.__unring()).catch(() => {});   // the screen moves on; the ring must not stay behind
          }
          if (st.do === 'type') { await el.fill(''); await el.type(st.value, { delay: 70 }).catch(e => errs.push(`step ${s.n} type: ${e.message.split('\n')[0]}`)); }
        }
      }
      const found = stepFound(st, el);
      await p.waitForTimeout(dwell(s));
      await p.evaluate(() => window.__unring && window.__unring()).catch(() => {});
      marks.push({ ...mark(s.n, s.key, start, found, [...cur.blocked]), wrote: [...cur.wrote], anchor: s.anchor, skipped: st.do === 'skip' });
      console.log(`${lang} step ${s.n} ${s.anchor || '(card)'} ${st.do === 'skip' ? 'SKIPPED (pending)' : found ? 'found' : 'ABSENT'}${cur.blocked.length ? ' blocked: ' + cur.blocked.join(', ') : ''}${cur.wrote.length ? ' wrote: ' + cur.wrote.join(', ') : ''}`);
    }
    await p.waitForTimeout(1200);
    const end = Date.now() - t0;
    const video = p.video();
    await ctx.close();
    renameSync(await video.path(), join(dir, 'raw.webm'));
    await browser.close(); browser = null;
    writeFileSync(join(dir, 'marks.json'), JSON.stringify({ id: lesson.id, lang, host: opt.host, ui: opt.ui, viewport: view,
      recordedAt: new Date().toISOString(), marks: closeMarks(marks, end), errors: errs }, null, 1));
    console.log(`${lang}: ${marks.filter(m => m.found).length}/${marks.length} anchors found, ${errs.length} page errors -> ${dir}`);
  }
  if (before) {
    const fresh = diff(before, await snapshot(opt.host, loc, who));
    open = await closeNew(opt.host, loc, who, fresh, console.log);
    console.log(`artefacts: ${fresh.orders.length} new order(s), ${fresh.sittings.length} new sitting(s); ${open} still open`);
  } else console.log('artefacts: none possible -- writes not allowed, every mutating /api/ call was aborted (listed per step above)');
} catch (e) {
  console.error(`capture: ${redact(e.message)}`); open = open || 1;
} finally {
  if (browser) await browser.close().catch(() => {});
}
process.exit(open);
