// Cold-start gate — the courier who kills the app underground and reopens it there.
//
// THE DEFECT (roadmap D2). The courier app had no service worker of its own:
// `/sw.js` is the storefront's and is registered only by the storefront. The
// outbox kept a courier's tap in IndexedDB through a dead network, but a courier
// who CLOSED the app in a basement and opened it again there got the browser's
// own error page — no address, no phone number, no "delivered" button, not even
// the words "no connection". The queue was safe and nothing could run to show it.
//
// WHAT THIS PROVES, both halves in the same run:
//   RED   — the courier app as it was at `BEFORE` (served from `git show`),
//           reopened with the server GONE: the run's address is not on screen.
//   GREEN — the working tree: the shell comes from `/courier/sw.js`, the last
//           answer from the hub is shown, and it is SAID to be the last answer.
//
// THE NETWORK IS REALLY GONE, not emulated. `context.setOffline()` does not
// reliably cover a service worker's own fetches, so a gate built on it can pass
// because the "offline" worker quietly reached the server. Here the server is
// closed and its sockets destroyed; the reopen is answered by the cache or by
// nothing. The same port is reused, so the origin — and with it the worker's
// registration — is the one the first visit installed.
//
// Nothing live is touched: the server serves `workers/api/public/` from disk
// and answers `/api/` itself.
//
//     node e2e/kit-regression/courier-cold.mjs

import { chromium } from 'playwright';
import http from 'node:http';
import { readFile, writeFile, mkdir, mkdtemp, rm } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import os from 'node:os';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const PUBLIC = fileURLToPath(new URL('../../workers/api/public/', import.meta.url));
const REPO = fileURLToPath(new URL('../../', import.meta.url));
/// The last commit before the courier had a service worker of its own.
const BEFORE = '827f7902';

const TYPES = {
  '.js': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.html': 'text/html; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
};

const ADDRESS = 'Rruga Taulantia 12, Durrës';
const TASKS = {
  onShift: true,
  courierId: 'c-1',
  shift: { deliveries: 2, cash: 0 },
  available: [],
  mine: [{
    id: 'ord-cold-1', status: 'IN_DELIVERY', payment: 'card', total: 1500,
    address: { line: ADDRESS }, contact: { phone: '+355690000000' },
  }],
};

const results = [];
function check(name, ok, detail = '') {
  results.push({ name, ok, detail });
  console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}${ok || !detail ? '' : `\n      ${detail}`}`);
}

function serve(root, port = 0) {
  const sockets = new Set();
  const srv = http.createServer(async (req, res) => {
    const url = new URL(req.url, 'http://127.0.0.1');
    if (url.pathname.startsWith('/api/')) {
      const body = url.pathname === '/api/courier/tasks' ? JSON.stringify(TASKS) : '{}';
      res.writeHead(200, { 'content-type': 'application/json' });
      res.end(body);
      return;
    }
    const rel = path.normalize(url.pathname).replace(/^(\.\.[/\\])+/, '').replace(/^\//, '');
    const named = rel.endsWith('/') || rel === '' ? rel + 'index.html' : rel;
    const base = named.startsWith('courier/') && root ? root : PUBLIC;
    const file = path.join(base, named);
    if (!file.startsWith(base)) { res.writeHead(403); res.end(); return; }
    try {
      const buf = await readFile(file);
      res.writeHead(200, { 'content-type': TYPES[path.extname(file)] || 'application/octet-stream' });
      res.end(buf);
    } catch { res.writeHead(404); res.end('not found'); }
  });
  srv.on('connection', s => { sockets.add(s); s.on('close', () => sockets.delete(s)); });
  return new Promise(resolve => srv.listen(port, '127.0.0.1', () => resolve({
    srv,
    port: srv.address().port,
    kill: () => new Promise(r => { for (const s of sockets) s.destroy(); srv.close(() => r()); }),
  })));
}

/// The courier directory as it was at `BEFORE`, for the RED half.
async function before() {
  const dir = await mkdtemp(path.join(os.tmpdir(), 'courier-before-'));
  await mkdir(path.join(dir, 'courier'), { recursive: true });
  const files = execFileSync('git', ['ls-tree', '--name-only', `${BEFORE}:workers/api/public/courier/`],
    { cwd: REPO }).toString().trim().split('\n');
  for (const f of files) {
    const src = execFileSync('git', ['show', `${BEFORE}:workers/api/public/courier/${f}`],
      { cwd: REPO, maxBuffer: 16 << 20 });
    await writeFile(path.join(dir, 'courier', f), src);
  }
  return dir;
}

/// Visit online, go underground, reopen. Returns what the reopened page shows.
async function coldStart(browser, root) {
  const first = await serve(root);
  const base = `http://127.0.0.1:${first.port}`;
  const ctx = await browser.newContext();
  let page = await ctx.newPage();
  await page.goto(`${base}/courier/`);
  await page.evaluate(() => localStorage.setItem('dw_c_jwt', 'courier-test-token'));
  await page.reload();
  await page.waitForFunction(a => document.body.innerText.includes(a), ADDRESS, { timeout: 15000 });
  // Give a worker, if the page registers one, the chance to install and take
  // control — bounded, and the answer is recorded rather than assumed.
  const controlled = await page.evaluate(async () => {
    if (!('serviceWorker' in navigator)) return false;
    const reg = await Promise.race([
      navigator.serviceWorker.ready,
      new Promise(r => setTimeout(() => r(null), 5000)),
    ]);
    return !!reg;
  });
  await page.close();
  await first.kill();

  page = await ctx.newPage();
  let navError = null;
  try { await page.goto(`${base}/courier/`, { timeout: 10000 }); }
  catch (e) { navError = String(e.message || e).split('\n')[0]; }
  await page.waitForTimeout(2500);
  const text = await page.evaluate(() => document.body ? document.body.innerText : '').catch(() => '');
  await ctx.close();
  return { controlled, navError, text };
}

export async function run() {
  const browser = await chromium.launch({ args: ['--no-sandbox'] });
  const dir = await before();
  try {
    const red = await coldStart(browser, dir);
    check('RED — the app as it was: reopened underground, the run\'s address is not on screen',
      !red.text.includes(ADDRESS),
      `nav=${red.navError} text=${JSON.stringify(red.text.slice(0, 120))}`);

    const green = await coldStart(browser, null);
    check('GREEN — a service worker controls /courier/ after the first visit', green.controlled);
    check('GREEN — reopened underground, the page opens (no navigation error)',
      green.navError === null, `nav=${green.navError}`);
    check('GREEN — the address of the run in hand is on screen',
      green.text.includes(ADDRESS), `text=${JSON.stringify(green.text.slice(0, 200))}`);
    check('GREEN — and it is SAID to be the last answer, not presented as live',
      /as of|të dhënat e|дані на/.test(green.text), `text=${JSON.stringify(green.text.slice(0, 200))}`);
  } finally {
    await browser.close();
    await rm(dir, { recursive: true, force: true });
  }
  const failed = results.filter(r => !r.ok).length;
  console.log(`courier-cold: ${results.length - failed}/${results.length} passed`);
  return failed;
}

if (import.meta.url === `file://${process.argv[1]}`) {
  run().then(n => process.exit(n ? 1 : 0), e => { console.error(e); process.exit(2); });
}
