// Run from the repo root: node marketing/promo-30s/capture/capture-posts.mjs <out-dir>
// Shot 8 re-capture: a staged draft post (the API is intercepted), open it, Publish, the toast.
import { chromium } from 'playwright';
import fs from 'node:fs';
const OUT = process.argv[2];
const env = Object.fromEntries(fs.readFileSync('/root/.dowiz_owner', 'utf8').split('\n').filter(l => l.startsWith('export ')).map(l => l.slice(7).split('=').map(s => s.trim())));
const H = 'https://sushi-durres.dowiz.org';
const VP = { width: 540, height: 1170 };
const b = await chromium.launch({ args: ['--no-sandbox', '--disable-dev-shm-usage'] });
const ctx = await b.newContext({ viewport: VP, deviceScaleFactor: 2, isMobile: true, hasTouch: true, colorScheme: 'dark', locale: 'en-US', recordVideo: { dir: OUT, size: VP } });
await ctx.addInitScript(() => { try { localStorage.setItem('dw_admin_lang', 'en'); } catch {} });
const DRAFT = { id: 'p_promo1', state: 'draft', about: 'new dish · Philadelphia Premium', text: 'New on the menu: Philadelphia Premium — salmon, cream cheese, cucumber, on warm sushi rice. Tonight in Durrës, delivered while it is still cold. Order in the app.' };
let published = false;
await ctx.route('**/api/owner/posts', r => r.fulfill({ json: { enabled: true, channel: '@dubinsushi', posts: published ? [{ ...DRAFT, state: 'published' }] : [DRAFT] } }));
await ctx.route('**/api/owner/posts/*/approve', async r => { published = true; await new Promise(res => setTimeout(res, 700)); r.fulfill({ json: { ok: true, state: 'published', channels: ['telegram', 'instagram'] } }); });
const p = await ctx.newPage();
try {
  await p.goto(`${H}/admin/`, { waitUntil: 'networkidle', timeout: 90_000 });
  await p.fill('#e', env.OWNER_EMAIL); await p.fill('#p', env.OWNER_PASSWORD); await p.click('#go');
  await p.waitForSelector('#nav:not([hidden])', { timeout: 30_000 }); await p.waitForTimeout(800);
  await p.click('#nav .tab[data-tab="more"]'); await p.waitForSelector('[data-open="posts"]'); await p.waitForTimeout(400);
  await p.click('[data-open="posts"]'); await p.waitForSelector('[data-post]', { timeout: 15_000 }); await p.waitForTimeout(1600);
  console.log('T draft-list', Date.now());
  await p.click('[data-post]'); await p.waitForSelector('#postYes', { timeout: 10_000 }); await p.waitForTimeout(1800);
  console.log('T open', Date.now());
  await p.click('#postYes'); await p.waitForTimeout(2600);
  console.log('T published', Date.now());
} catch (e) { console.log('THREW', String(e.message).split('\n')[0].slice(0, 160)); }
const v = p.video(); await ctx.close(); const path = await v.path(); fs.renameSync(path, `${OUT}/admin-posts-approve.webm`); console.log('clip admin-posts-approve');
await b.close();
