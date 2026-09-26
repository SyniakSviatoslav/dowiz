import test from 'node:test';
import assert from 'node:assert/strict';
import { parseRgb, luminance, contrast, pageProbe, visit, collect, defaultLaunch, CSP_HOOK, SURFACES, WIDTHS } from './ux.mjs';

const byId = xs => Object.fromEntries(xs.map(x => [x.id, x]));

test('colours parse from rgb and rgba; anything else is null', () => {
  assert.deepEqual(parseRgb('rgb(1, 2, 3)'), [1, 2, 3, 1]);
  assert.deepEqual(parseRgb('rgba(1, 2, 3, 0.5)'), [1, 2, 3, 0.5]);
  assert.equal(parseRgb('transparent'), null);
});

test('WCAG contrast: black on white is 21, a colour on itself is 1', () => {
  assert.equal(Math.round(contrast([0, 0, 0], [255, 255, 255])), 21);
  assert.equal(contrast([10, 10, 10], [10, 10, 10]), 1);
  assert.equal(luminance([255, 255, 255]), 1);
  assert.equal(luminance([0, 0, 0]), 0);
});

/** A stub page: elements with styles, parents and own text. */
function stubDom(els) {
  const styles = new Map();
  const mk = ({ text = 'hi', color = 'rgb(0, 0, 0)', bg = 'rgba(0, 0, 0, 0)', w = 10, parent = null, fontSize = '16px', fontWeight = '400', visibility = 'visible', display = 'block', opacity = '1' }) => {
    const el = { parentElement: parent, childNodes: text === null ? [] : [{ nodeType: 3, textContent: text }], getBoundingClientRect: () => ({ width: w, height: w }) };
    styles.set(el, { color, backgroundColor: bg, fontSize, fontWeight, visibility, display, opacity });
    return el;
  };
  const body = mk({ text: null, bg: 'rgb(255, 255, 255)' });
  const list = els.map(e => mk({ parent: body, ...e }));
  return { list, styles };
}

test('the in-page probe counts failing text against the first opaque background', () => {
  const { list, styles } = stubDom([
    {}, // black on white: passes
    { color: 'rgb(200, 200, 200)' }, // pale grey on white: fails
    { color: 'rgb(140, 140, 140)', fontSize: '30px' }, // large text, ratio ~3: passes at 3
    { color: 'rgb(140, 140, 140)', fontSize: '19px', fontWeight: '700' }, // large bold: judged at 3
    { text: '   ' }, { w: 0 }, { visibility: 'hidden' }, { display: 'none' }, { opacity: '0' }, // skipped
    { color: 'transparent' }, // unparseable colour: skipped
  ]);
  const orphan = { parentElement: null, childNodes: [{ nodeType: 3, textContent: 'x' }], getBoundingClientRect: () => ({ width: 1, height: 1 }) };
  styles.set(orphan, { color: 'rgb(250, 250, 250)', backgroundColor: 'rgba(0,0,0,0)', fontSize: '12px', fontWeight: '400', visibility: 'visible', display: 'block', opacity: '1' });
  const saved = { document: globalThis.document, getComputedStyle: globalThis.getComputedStyle, window: globalThis.window };
  globalThis.document = { querySelectorAll: () => [...list, orphan] };
  globalThis.getComputedStyle = el => styles.get(el);
  globalThis.window = {};
  try {
    const r = pageProbe();
    assert.deepEqual(r, { fails: 2, checked: 5, fcp: null, csp: 0, cspDirectives: '' });
    globalThis.window = { __csp: ['style-src', 'style-src', 'img-src'] };
    const orig = performance.getEntriesByType;
    performance.getEntriesByType = () => [{ name: 'first-contentful-paint', startTime: 812.4 }];
    try { assert.equal(pageProbe().fcp, 812); assert.equal(pageProbe().csp, 3); assert.equal(pageProbe().cspDirectives, 'style-src img-src'); } finally { performance.getEntriesByType = orig; }
  } finally {
    Object.assign(globalThis, saved);
  }
});

/** A fake Playwright: every page emits what `script` says, then evaluates to `probe`. */
function fakeBrowser(script) {
  const log = { contexts: 0, closed: 0, urls: [], init: [] };
  const browser = {
    async newContext(o) {
      log.contexts += 1;
      assert.equal(o.serviceWorkers, 'block');
      return {
        async newPage() {
          const on = {};
          return {
            addInitScript: async s => log.init.push(s),
            on: (ev, fn) => { on[ev] = fn; },
            async goto(url) {
              log.urls.push(url);
              for (const m of script.console || []) on.console({ type: () => m[0], text: () => m[1] });
              for (let i = 0; i < (script.pageErrors || 0); i++) on.pageerror(new Error('x'));
              for (const s of script.sizes || []) await on.requestfinished({ sizes: async () => { if (!s) throw new Error('gone'); return s; } });
            },
            waitForTimeout: async () => {},
            evaluate: async fn => { assert.equal(fn, pageProbe); return script.probe; },
          };
        },
        close: async () => { log.closed += 1; },
      };
    },
    close: async () => { log.browserClosed = true; },
  };
  return { browser, log };
}

test('visit counts console errors, page errors, requests and wire bytes', async () => {
  const { browser, log } = fakeBrowser({
    console: [['error', 'Refused to load'], ['warning', 'w'], ['error', 'second']], pageErrors: 1,
    sizes: [{ responseBodySize: 100, responseHeadersSize: 20 }, null], probe: { fails: 1, checked: 9, fcp: 700, csp: 2, cspDirectives: 'script-src' },
  });
  const r = await visit(browser, 'https://h/', 360);
  assert.deepEqual(r, { console: 2, pageErrors: 1, requests: 2, bytes: 120, first: 'Refused to load', fails: 1, checked: 9, fcp: 700, csp: 2, cspDirectives: 'script-src' });
  assert.deepEqual(log.init, [CSP_HOOK]);
  assert.equal(log.closed, 1);
});

test('the ux collector: every surface at both widths, and the browser closed even on failure', async () => {
  const { browser, log } = fakeBrowser({ console: [['error', 'e']], probe: { fails: 3, checked: 40, fcp: 500, csp: 0 } });
  const r = byId(await collect({ host: 'https://h', launch: async () => browser }));
  assert.equal(log.urls.length, Object.keys(SURFACES).length * WIDTHS.length);
  assert.equal(r['ux.store.fcp_ms'].value, 500);
  assert.equal(r['ux.admin.console_errors'].value, 2);
  assert.equal(r['ux.admin.console_errors'].note, 'e');
  assert.equal(r['ux.courier.contrast_failures'].value, 3);
  assert.equal(r['ux.courier.csp_violations'].note, undefined);
  assert.match(r['ux.courier.contrast_failures'].note, /approximate \(not axe\): of 40/);
  assert.ok(log.browserClosed);
  const quiet = fakeBrowser({ probe: { fails: 0, checked: 1, fcp: 1, csp: 0 } });
  const q = byId(await collect({ host: 'https://h', launch: async () => quiet.browser }));
  assert.equal(q['ux.room.console_errors'].note, undefined);
  const bad = fakeBrowser({ probe: null });
  bad.browser.newContext = async () => { throw new Error('no chromium'); };
  await assert.rejects(collect({ host: 'https://h', launch: async () => bad.browser }), /no chromium/);
  assert.ok(bad.log.browserClosed);
});

test('the default launcher starts a real headless Chromium (skipped where playwright is absent)', async t => {
  let b;
  try { b = await defaultLaunch(); } catch (e) { t.skip(`no playwright here: ${e.message.slice(0, 80)}`); return; }
  assert.equal(typeof b.newContext, 'function');
  await b.close();
});

test('an error seen only at the second width still names itself', async () => {
  let n = 0;
  const { browser } = fakeBrowser({ probe: { fails: 0, checked: 1, fcp: 1, csp: 0 } });
  const orig = browser.newContext;
  browser.newContext = async o => {
    const c = await orig(o);
    const np = c.newPage;
    c.newPage = async () => { const p = await np(); const g = p.goto; const on = p.on; const hs = {};
      p.on = (ev, fn) => { hs[ev] = fn; on(ev, fn); };
      p.goto = async u => { await g(u); if (n++ % 2 === 1) hs.console({ type: () => 'error', text: () => 'wide only' }); };
      return p; };
    return c;
  };
  const r = byId(await collect({ host: 'https://h', launch: async () => browser, settleMs: 0 }));
  assert.equal(r['ux.store.console_errors'].note, 'wide only');
});

test('a real headless Chromium over a local page: paint, requests, CSP and contrast (skipped without playwright)', async t => {
  try { (await defaultLaunch()).close(); } catch (e) { t.skip(`no playwright here: ${e.message.slice(0, 80)}`); return; }
  const http = await import('node:http');
  const page = '<!doctype html><meta http-equiv="Content-Security-Policy" content="img-src \'none\'">'
    + '<body style="background:#fff"><p style="color:#000">readable</p><p style="color:#eee">faint</p><img src="/x.png"></body>';
  const srv = http.createServer((q, s) => { s.writeHead(200, { 'content-type': 'text/html' }); s.end(page); });
  await new Promise(r => srv.listen(0, '127.0.0.1', r));
  try {
    const r = byId(await collect({ host: `http://127.0.0.1:${srv.address().port}`, settleMs: 300 }));
    assert.equal(r['ux.store.contrast_failures'].value, 1);
    assert.equal(r['ux.store.csp_violations'].value, 2, 'the blocked image, once per width');
    assert.equal(r['ux.store.csp_violations'].note, 'directives: img-src');
    assert.ok(r['ux.store.boot_requests'].value >= 1);
    assert.ok(r['ux.store.fcp_ms'].value > 0);
  } finally { srv.close(); }
});
