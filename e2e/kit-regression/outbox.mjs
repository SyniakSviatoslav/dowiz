// Offline WRITE gate — the tap in the basement, and the tap that must be dropped.
//
// WHY A REAL BROWSER AND A REAL SERVER. The thing under test is a queue in
// IndexedDB drained over `fetch` when the network comes back. jsdom has neither
// a real IndexedDB nor a network to take away, and a unit test with a stubbed
// `fetch` cannot tell the difference between "the request was not sent" and
// "the request was sent and the browser was offline" — which is the exact
// distinction `outbox.js` is built around. So: Chromium with
// `context.setOffline()`, and a throwaway HTTP server on loopback that COUNTS
// what actually arrives.
//
// NOTHING LIVE IS TOUCHED. Every other gate in this directory points at
// `https://dubin-sushi.dowiz.org`; this one must not, because half of what it
// proves is a refusal being replayed, and a test that hammers production to
// prove it does not hammer production has missed the point. The server here
// serves `workers/api/public/` from disk (so `/lib/outbox.js` is the real file)
// and answers `/api/` from a script the test sets per case.
//
// THE RED HALF IS IN THE RUN. Each proof runs a NAIVE client against the same
// fake server in the same browser: a client that mints its key at send time and
// retries whatever it gets. That client is the defect — it loses the tap when
// the network is gone, and it hammers a 409 until its budget runs out. The
// numbers printed for it are measured in the same run as the numbers for the
// outbox, so "the defect existed" is not a claim about the past.
//
// ZERO NEW DEPENDENCIES, as `render.mjs` explains: the `playwright` library
// already in the tree, plain assertions, one exit code.
//
//     node e2e/kit-regression/outbox.mjs        # this gate alone

import { chromium } from 'playwright';
import http from 'node:http';
import { readFile, writeFile, mkdir, mkdtemp, rm } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import os from 'node:os';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const PUBLIC = fileURLToPath(new URL('../../workers/api/public/', import.meta.url));

const results = [];
function check(name, ok, detail = '') {
  results.push({ name, ok, detail });
  console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}${ok || !detail ? '' : `\n      ${detail}`}`);
}

const TYPES = {
  '.js': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.html': 'text/html; charset=utf-8',
  '.png': 'image/png',
  '.svg': 'image/svg+xml',
  '.woff2': 'font/woff2',
  '.json': 'application/json; charset=utf-8',
  '.webmanifest': 'application/manifest+json',
};

/// The page under test. It is served by the fake server rather than written
/// into `public/`, because a harness that lives in the served tree is a file
/// the edge would deploy.
const HARNESS = `<!doctype html><meta charset="utf-8"><title>outbox harness</title>
<body><p id="state">booting</p>
<script type="module">
import { createOutbox, MAX_QUEUE } from '/lib/outbox.js';
window.MAX_QUEUE = MAX_QUEUE;
window.log = { sent: [], dropped: [] };
window.ob = createOutbox({
  authorize: () => ({ authorization: 'Bearer courier-test-token' }),
  onSent: (e, payload, status) => window.log.sent.push({ key: e.key, tag: e.tag, status }),
  onDropped: (e, reason, status) => window.log.dropped.push({ key: e.key, tag: e.tag, reason, status }),
});
window.ob.start();

// THE CONTROL. A client with the two defects this module exists to remove: the
// key is minted per ATTEMPT, so every retry is a new order to the server, and
// any answer that is not 2xx is retried until the budget runs out.
window.naive = async (route, body, budget) => {
  let attempts = 0;
  for (let i = 0; i < budget; i++) {
    attempts++;
    try {
      const r = await fetch(route, { method: 'POST', body,
        headers: { 'content-type': 'application/json', 'idempotency-key': crypto.randomUUID() } });
      if (r.ok) break;
    } catch { /* the tap is gone, and nothing remembers it */ }
  }
  return attempts;
};

document.getElementById('state').textContent = 'ready';
window.ready = true;
</script>`;

/// When set, `/courier/**` comes from here instead of the working tree.
let overlay = null;

/// THE COURIER SURFACE AS IT WAS BEFORE THE OUTBOX, pinned to the commit that
/// last shipped it, for the RED half.
///
/// PINNED AND NOT `HEAD`, WHICH IS THE WHOLE POINT. Written against `HEAD` this
/// was correct in the working tree and WRONG the moment the change was
/// committed: `HEAD` would then be the new courier app, the RED overlay would
/// be a second copy of the GREEN one, and the two "the client we had" cases
/// would pass while proving nothing at all. A RED proof that silently becomes
/// its own GREEN is worse than no RED proof, because it still reads as
/// evidence. `5a1b727a` is the commit that last touched these four files
/// before the queue existed.
///
/// `git show` only reads; nothing in this gate writes to the repository. It
/// needs the history, so a CI checkout must not be `fetch-depth: 1`.
const BEFORE_THE_OUTBOX = '5a1b727a';

async function checkoutHead(dir) {
  await mkdir(path.join(dir, 'courier'), { recursive: true });
  for (const f of ['app.js', 'index.html', 'i18n.js', 'courier.css']) {
    const src = execFileSync('git',
      ['show', `${BEFORE_THE_OUTBOX}:workers/api/public/courier/${f}`],
      { cwd: fileURLToPath(new URL('../../', import.meta.url)), maxBuffer: 16 << 20 });
    await writeFile(path.join(dir, 'courier', f), src);
  }
  return dir;
}

/// What the fake `/api/` does next, and everything it has been asked.
const api = {
  seen: [],
  /// (req, n) -> { status, body?, headers? }
  reply: () => ({ status: 200, body: '{"ok":true}' }),
  reset(reply) { this.seen = []; if (reply) this.reply = reply; },
};

function serve() {
  return new Promise(resolve => {
    const srv = http.createServer(async (req, res) => {
      const url = new URL(req.url, 'http://127.0.0.1');
      if (url.pathname === '/harness.html') {
        res.writeHead(200, { 'content-type': TYPES['.html'] });
        res.end(HARNESS);
        return;
      }
      if (url.pathname.startsWith('/api/')) {
        const chunks = [];
        for await (const c of req) chunks.push(c);
        const body = Buffer.concat(chunks).toString('utf8');
        const entry = { path: url.pathname, method: req.method, body,
                        key: req.headers['idempotency-key'] || null,
                        auth: req.headers['authorization'] || null };
        api.seen.push(entry);
        const r = api.reply(entry, api.seen.length);
        res.writeHead(r.status, { 'content-type': 'application/json', ...(r.headers || {}) });
        res.end(r.body ?? '{}');
        return;
      }
      // Anything else is the real tree on disk, and a path that tries to leave
      // it is a 403 rather than a read.
      const rel = path.normalize(url.pathname).replace(/^(\.\.[/\\])+/, '').replace(/^\//, '');
      // A directory is its index, exactly as the Worker's static handler serves
      // `/courier/` — without this the real-surface case fetches a 404 and the
      // failure reads as "the app did not render".
      const named = rel.endsWith('/') || rel === '' ? rel + 'index.html' : rel;
      // THE SURFACE AS IT WAS. When an overlay is set, `/courier/**` is served
      // from the committed version of those files instead of the working tree,
      // so the RED half of the real-surface proof is the code that shipped
      // rather than a description of it.
      const root = (overlay && named.startsWith('courier/')) ? overlay : PUBLIC;
      const file = path.join(root, named);
      if (!file.startsWith(root)) { res.writeHead(403); res.end(); return; }
      try {
        const buf = await readFile(file);
        res.writeHead(200, { 'content-type': TYPES[path.extname(file)] || 'application/octet-stream' });
        res.end(buf);
      } catch { res.writeHead(404); res.end('not found'); }
    });
    srv.listen(0, '127.0.0.1', () => resolve(srv));
  });
}

/// Drain until the queue stops changing. The module schedules its own retries
/// on a backoff measured in seconds; a gate must not sleep through them, so it
/// asks directly and stops when nothing moved.
async function settle(page, rounds = 10) {
  let last = -1;
  for (let i = 0; i < rounds; i++) {
    await page.evaluate(async () => { await window.ob.drain(); });
    const n = await page.evaluate(async () => (await window.ob.pending()).length);
    const sent = await page.evaluate(() => window.log.sent.length + window.log.dropped.length);
    if (n === 0) return n;
    if (n === last && sent === settle._s) return n;
    last = n; settle._s = sent;
  }
  return await page.evaluate(async () => (await window.ob.pending()).length);
}


/// Every file a service worker precaches, and every module each of them
/// statically imports, so "is this import in the shell?" is answered from the
/// tree rather than from memory.
async function shellsAgree(mod) {
  const workers = ['sw.js', 'kit/sw.js'];
  const offenders = [];
  for (const w of workers) {
    const src = await readFile(path.join(PUBLIC, w), 'utf8');
    // The lists are plain arrays of quoted absolute paths; a precached name is
    // any '/...' string literal in the file, which over-collects harmlessly.
    const shell = new Set([...src.matchAll(/'(\/[^']*)'/g)].map(m => m[1]));
    for (const p of shell) {
      if (!p.endsWith('.js')) continue;
      let body;
      try { body = await readFile(path.join(PUBLIC, p.replace(/^\//, '')), 'utf8'); } catch { continue; }
      const imports = [...body.matchAll(/^\s*import\s[^;]*?from\s+'(\/[^']+)'/gm)].map(m => m[1]);
      if (imports.includes(mod) && !shell.has(mod)) offenders.push(`${w}: ${p} imports ${mod}, which is not in its shell`);
    }
  }
  return ['a precached module that imports the outbox precaches the outbox too',
          offenders.length === 0, offenders.join('\n')];
}

export async function run() {
  const srv = await serve();
  const base = `http://127.0.0.1:${srv.address().port}`;
  const browser = await chromium.launch({ args: ['--no-sandbox'] });
  const ctx = await browser.newContext();
  const page = await ctx.newPage();
  const errors = [];
  page.on('pageerror', e => errors.push(String(e)));

  /// Wait until the server has been quiet for a beat. Not decoration: the
  /// module drains ON LOAD, so a fresh page can put the PREVIOUS case's
  /// leftovers on the wire before this case has said a word — which is how the
  /// 403 proof first read `requests_at_server=2` and was right to.
  const quiet = async (ms = 250) => {
    for (;;) {
      const before = api.seen.length;
      await page.waitForTimeout(ms);
      if (api.seen.length === before) return;
    }
  };

  const fresh = async (reply) => {
    // A new page each case, with the queue emptied and the counters zeroed
    // AFTER everything the last one started has finished.
    await page.goto(`${base}/harness.html`);
    await page.waitForFunction(() => window.ready === true);
    await page.evaluate(async () => { await window.ob.forget(); window.log = { sent: [], dropped: [] }; });
    await quiet();
    api.reset(reply);
  };

  try {
    // ── 1. The defect: with no network, a tap is simply gone ─────────────────
    await fresh(() => ({ status: 200, body: '{"ok":true}' }));
    await ctx.setOffline(true);
    const naiveAttempts = await page.evaluate(([b]) =>
      window.naive(b + '/api/courier/orders/o1/pickup', '{}', 3), [base]);
    check('RED — the client we had: three attempts offline, nothing reaches the server and nothing is kept',
      api.seen.length === 0 && naiveAttempts === 3,
      `attempts=${naiveAttempts} requests_at_server=${api.seen.length}`);

    // ── 2. The queue keeps it, with the key minted at tap time ───────────────
    const tap = await page.evaluate(([b]) =>
      window.ob.queue(b + '/api/courier/orders/o1/pickup', { body: '{}', tag: 'pickup:o1' }), [base]);
    const queuedOffline = await page.evaluate(async () => (await window.ob.pending()).length);
    check('GREEN — offline, the tap is queued and answers with its own key',
      tap.ok === true && typeof tap.key === 'string' && tap.key.length > 8
      && queuedOffline === 1 && api.seen.length === 0,
      `tap=${JSON.stringify(tap)} queued=${queuedOffline} requests_at_server=${api.seen.length}`);

    // ── 3. Back online, exactly once, under the key from the tap ─────────────
    await ctx.setOffline(false);
    const left = await settle(page);
    const sent = await page.evaluate(() => window.log.sent);
    check('GREEN — the network returns and the tap lands exactly once',
      api.seen.length === 1 && left === 0 && sent.length === 1,
      `requests_at_server=${api.seen.length} still_queued=${left} sent=${JSON.stringify(sent)}`);
    check('the Idempotency-Key on the wire is the one minted at tap time, not at send time',
      api.seen[0]?.key === tap.key,
      `tapped=${tap.key} on_the_wire=${api.seen[0]?.key}`);
    check('the drain authorises at send time, from the caller',
      api.seen[0]?.auth === 'Bearer courier-test-token', String(api.seen[0]?.auth));

    // ── 3b. A RELOAD IS A DRAIN. Nobody calls `drain()` here ─────────────────
    //
    // A queue that only ran on the `online` EVENT would sit untouched on the
    // phone that was already online when the app was reopened — which is what
    // every courier does after the tunnel: kill the app, open it again.
    await fresh(() => ({ status: 200, body: '{"ok":true}' }));
    await ctx.setOffline(true);
    await page.evaluate(([b]) =>
      window.ob.queue(b + '/api/courier/orders/o1b/pickup', { body: '{}', tag: 'pickup:o1b' }), [base]);
    await ctx.setOffline(false);
    await page.goto(`${base}/harness.html`);
    await page.waitForFunction(() => window.ready === true);
    let landedOnLoad = 0;
    for (let i = 0; i < 20 && !landedOnLoad; i++) { await page.waitForTimeout(100); landedOnLoad = api.seen.length; }
    check('a reload with a tap still queued drains it, with nothing asking it to',
      landedOnLoad === 1 && (await page.evaluate(async () => (await window.ob.pending()).length)) === 0,
      `requests_at_server=${api.seen.length}`);

    // ── 4. Order is the sequence the FSM needs ───────────────────────────────
    await fresh(() => ({ status: 200, body: '{"ok":true}' }));
    await ctx.setOffline(true);
    await page.evaluate(async ([b]) => {
      await window.ob.queue(b + '/api/courier/orders/o2/accept', { body: '{}', tag: 'accept' });
      await window.ob.queue(b + '/api/courier/orders/o2/pickup', { body: '{}', tag: 'pickup' });
      await window.ob.queue(b + '/api/courier/orders/o2/deliver', { body: '{"cash_collected":0}', tag: 'deliver' });
    }, [base]);
    await ctx.setOffline(false);
    await settle(page);
    const order = api.seen.map(s => s.path.split('/').pop());
    check('three taps drain in the order they were tapped',
      order.join(' → ') === 'accept → pickup → deliver', order.join(' → '));

    // ── 5. A transient keeps the key ─────────────────────────────────────────
    await fresh((_e, n) => n === 1
      ? { status: 503, body: '{"error":"edge is having a moment"}' }
      : { status: 200, body: '{"ok":true}' });
    await page.evaluate(([b]) =>
      window.ob.queue(b + '/api/courier/orders/o3/deliver', { body: '{}', tag: 'deliver' }), [base]);
    await settle(page);
    check('a 503 is retried, and the retry carries the SAME key — one tap stays one tap',
      api.seen.length === 2 && api.seen[0].key === api.seen[1].key,
      `requests=${api.seen.length} keys=${api.seen.map(s => s.key).join(' , ')}`);

    // ── 6. THE REFUSAL. The proof this gate exists for ───────────────────────
    //
    // The owner cancelled while the courier was underground. The queued
    // "delivered" is an illegal edge and `courier.rs` answers 409. A client
    // that retries that is an infinite loop against production.
    await fresh(() => ({ status: 409, body: '{"error":"illegal transition DELIVERED"}' }));
    const naive409 = await page.evaluate(([b]) =>
      window.naive(b + '/api/courier/orders/o4/deliver', '{}', 10), [base]);
    const naiveKeys = new Set(api.seen.map(s => s.key)).size;
    check('RED — the client we had hammers a 409 until its budget runs out, under a new key each time',
      api.seen.length === 10 && naive409 === 10 && naiveKeys === 10,
      `attempts=${naive409} requests_at_server=${api.seen.length} distinct_keys=${naiveKeys}`);

    api.reset(() => ({ status: 409, body: '{"error":"illegal transition DELIVERED"}' }));
    await page.evaluate(([b]) =>
      window.ob.queue(b + '/api/courier/orders/o4/deliver', { body: '{}', tag: 'deliver:o4' }), [base]);
    const after409 = await settle(page);
    const dropped = await page.evaluate(() => window.log.dropped);
    check('GREEN — the outbox asks ONCE, drops the tap and says the order changed',
      api.seen.length === 1 && after409 === 0
      && dropped.length === 1 && dropped[0].reason === 'changed' && dropped[0].status === 409,
      `requests_at_server=${api.seen.length} still_queued=${after409} dropped=${JSON.stringify(dropped)}`);

    // ── 7. The ONE 409 that is not a refusal (idempotency.rs rule 4) ─────────
    await fresh(() => ({ status: 409, body: '{"error":"in flight"}', headers: { 'retry-after': '1' } }));
    await page.evaluate(([b]) =>
      window.ob.queue(b + '/api/courier/orders/o5/deliver', { body: '{}', tag: 'deliver:o5' }), [base]);
    await page.evaluate(async () => { await window.ob.drain(); });
    const keptAfterRule4 = await page.evaluate(async () => (await window.ob.pending()).length);
    const droppedRule4 = await page.evaluate(() => window.log.dropped.length);
    check('a 409 WITH Retry-After is the first copy still running — kept, not dropped',
      keptAfterRule4 === 1 && droppedRule4 === 0,
      `still_queued=${keptAfterRule4} dropped=${droppedRule4}`);

    // ── 8. A refusal that is not 409 ─────────────────────────────────────────
    await fresh(() => ({ status: 403, body: '{"error":"not your delivery"}' }));
    await page.evaluate(([b]) =>
      window.ob.queue(b + '/api/courier/orders/o6/deliver', { body: '{}', tag: 'deliver:o6' }), [base]);
    await settle(page);
    const drop403 = await page.evaluate(() => window.log.dropped);
    check('a 403 is an answer too — asked once, dropped, reported as a refusal',
      api.seen.length === 1 && drop403.length === 1 && drop403[0].status === 403 && drop403[0].reason === 'refused',
      `requests_at_server=${api.seen.length} dropped=${JSON.stringify(drop403)}`);

    // ── 9. The bound, and which end gives way ────────────────────────────────
    await fresh(() => ({ status: 200, body: '{"ok":true}' }));
    await ctx.setOffline(true);
    const bound = await page.evaluate(async ([b]) => {
      const max = window.MAX_QUEUE;
      const answers = [];
      for (let i = 0; i < max + 2; i++) {
        answers.push(await window.ob.queue(b + `/api/courier/orders/x${i}/pickup`, { body: '{}', tag: 'p' + i }));
      }
      return { max, held: (await window.ob.pending()).length,
               refused: answers.filter(a => !a.ok).length,
               firstRefusedAt: answers.findIndex(a => !a.ok),
               reason: answers.find(a => !a.ok)?.reason,
               oldestTag: (await window.ob.pending())[0]?.tag };
    }, [base]);
    check('the queue is bounded, and it refuses the NEW tap rather than silently eating the oldest',
      bound.held === bound.max && bound.refused === 2 && bound.firstRefusedAt === bound.max
      && bound.reason === 'full' && bound.oldestTag === 'p0',
      JSON.stringify(bound));
    await ctx.setOffline(false);

    // ── 10. THE SHELL LISTS. Lane P4's defect, guarded rather than remembered ─
    //
    // `public/sw.js` and `public/kit/sw.js` both carry the same warning in
    // their own words: a PRECACHED file that statically imports a module which
    // is NOT precached fails at LINK time offline, and the page comes up blank
    // under a header that promises it works offline. Adding `/lib/outbox.js` is
    // exactly the kind of import that does it. This is a text check, not a
    // browser one, because the answer is in the files.
    check(...await shellsAgree('/lib/outbox.js'));

    // ── 11. THE REAL SURFACE. `/courier/` as it is served, not a harness ─────
    //
    // Everything above proves the module. This proves the WIRING: the courier
    // app as the edge serves it, with the hub replaced by the counter. The tap
    // is a real click on the real "Picked up" button, offline.
    const orders = { onShift: true, courierId: 'c1', shift: { deliveries: 0, cash: 0 }, available: [],
      mine: [{ id: 'ord-basement', status: 'READY', total: 1200, items: 3, payment: 'card',
               address: { line: 'Rruga Taulantia 12' }, contact: { phone: '+355690000000' } }] };
    await fresh(e => e.path.endsWith('/tasks')
      ? { status: 200, body: JSON.stringify(orders) }
      : { status: 200, body: '{"ok":true}' });
    await page.addInitScript(() => {
      localStorage.setItem('dw_c_jwt', 'courier-test-token');
      // The first-run tour would sit over the button this case has to click.
      localStorage.setItem('dw_guide_courier', JSON.stringify({ state: 'done', step: 0 }));
    });
    // RED, on the surface itself: the committed courier app (`git show HEAD:`),
    // the same tap, the same dead network. It keeps nothing.
    const redDir = await mkdtemp(path.join(os.tmpdir(), 'outbox-red-'));
    overlay = await checkoutHead(redDir);
    await page.goto(`${base}/courier/`);
    await page.waitForSelector('#pick', { timeout: 15_000 });
    await ctx.setOffline(true);
    await page.click('#pick');
    await page.waitForTimeout(1500);
    const redPickups = api.seen.filter(r => r.path.endsWith('/pickup')).length;
    const redKept = await page.evaluate(() => new Promise(resolve => {
      // Nothing in that build ever opened this database, so an open here
      // CREATES an empty one — which is the measurement: zero taps kept.
      const r = indexedDB.open('dowiz.outbox');
      r.onsuccess = () => {
        const db = r.result;
        if (!db.objectStoreNames.contains('queue')) { resolve(0); return; }
        const q = db.transaction('queue', 'readonly').objectStore('queue').count();
        q.onsuccess = () => resolve(q.result);
        q.onerror = () => resolve(-1);
      };
      r.onerror = () => resolve(-1);
    }));
    check('RED — the courier app that shipped: the same tap, offline, reaches nothing and is kept nowhere',
      redPickups === 0 && redKept === 0,
      `pickups_at_hub=${redPickups} taps_kept=${redKept}`);
    await ctx.setOffline(false);
    overlay = null;
    await rm(redDir, { recursive: true, force: true }).catch(() => {});

    // GREEN — the same click, on the working tree. The RED build queued
    // nothing, so the database it left behind is the empty one it created.
    await page.goto(`${base}/courier/`);
    await page.waitForSelector('#pick', { timeout: 15_000 });
    api.reset();
    await ctx.setOffline(true);
    await page.click('#pick');
    await page.waitForSelector('#outboxTag:not([hidden])', { timeout: 10_000 });
    const tagText = (await page.textContent('#outboxText'))?.trim();
    const pickupsOffline = api.seen.filter(r => r.path.endsWith('/pickup')).length;
    const stillSaysReady = await page.evaluate(() => !!document.querySelector('.status.st-ready'));
    const ctaGone = await page.evaluate(() => !document.querySelector('#pick'));
    check('offline, a real tap on the courier screen is kept and COUNTED, and nothing reaches the hub',
      pickupsOffline === 0 && !!tagText && ctaGone,
      `pickups_at_hub=${pickupsOffline} hud="${tagText}" cta_replaced=${ctaGone}`);
    check('a queued tap does not look like a landed one — the status chip is still the hub\'s answer',
      stillSaysReady, `st-ready still on screen: ${stillSaysReady}`);

    await ctx.setOffline(false);
    let pickups = [];
    for (let i = 0; i < 40 && !pickups.length; i++) {
      await page.waitForTimeout(100);
      pickups = api.seen.filter(r => r.path.endsWith('/pickup'));
    }
    check('back above ground, the courier surface sends that tap exactly once, with a key',
      pickups.length === 1 && !!pickups[0].key && pickups[0].auth === 'Bearer courier-test-token',
      `pickups=${pickups.length} key=${pickups[0]?.key} auth=${pickups[0]?.auth}`);
    await page.waitForTimeout(400);
    check('and the HUD stops counting once it has landed',
      await page.evaluate(() => document.querySelector('#outboxTag')?.hidden === true),
      `hidden=${await page.evaluate(() => document.querySelector('#outboxTag')?.hidden)}`);

    // Last, so it covers every case above it, including the real surface.
    check('no uncaught error on the page during any of it', errors.length === 0, errors.join('\n'));
  } finally {
    await browser.close();
    srv.close();
  }
  return report();
}

function report() {
  const bad = results.filter(r => !r.ok);
  console.log(bad.length
    ? `\n${bad.length} of ${results.length} failed`
    : `\nall ${results.length} checks pass`);
  return bad.length;
}

// Runnable on its own, and importable by `run.mjs` like every other gate.
if (import.meta.url === `file://${process.argv[1]}`) {
  run().then(n => process.exit(n ? 1 : 0), e => { console.error(e); process.exit(1); });
}
