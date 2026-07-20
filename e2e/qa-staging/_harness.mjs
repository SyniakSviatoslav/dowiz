// Shared QA harness for staging E2E. Captures console errors, page errors,
// failed network requests, and screenshots for every page under test.
import { chromium } from '@playwright/test';

export const BASE = 'https://dowiz-staging.fly.dev';
export const SHOTS = '/tmp/qa-shots';
export const DEV_SECRET = 'stg-e2e-secret';
export const LOCATION_ID = '28239442-63a1-431e-8cab-2e4ed64ab8e7';

export const MOBILE = {
  name: 'mobile',
  viewport: { width: 390, height: 844 },
  userAgent:
    'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1',
  isMobile: true,
  hasTouch: true,
  deviceScaleFactor: 3,
};
export const DESKTOP = {
  name: 'desktop',
  viewport: { width: 1280, height: 800 },
  userAgent:
    'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36',
  isMobile: false,
  hasTouch: false,
  deviceScaleFactor: 1,
};

// Wake the staging machine (Fly auto-stops). Retry health until 200 or give up.
export async function wakeStaging() {
  for (let i = 0; i < 12; i++) {
    try {
      const r = await fetch(`${BASE}/health`, { signal: AbortSignal.timeout(20000) });
      if (r.status === 200) return true;
    } catch {
      /* cold start */
    }
    await new Promise(res => setTimeout(res, 4000));
  }
  return false;
}

export async function mockAuth(email, role) {
  const r = await fetch(`${BASE}/api/dev/mock-auth`, {
    method: 'POST',
    headers: { 'content-type': 'application/json', 'x-dev-auth-secret': DEV_SECRET },
    body: JSON.stringify({ email, role, locationId: LOCATION_ID }),
    signal: AbortSignal.timeout(20000),
  });
  if (!r.ok) throw new Error(`mock-auth ${role} failed: ${r.status} ${await r.text()}`);
  const j = await r.json();
  return j.access_token;
}

// Returns a context+page wired with collectors. Caller closes via teardown().
export async function makePage(browser, device) {
  const ctx = await browser.newContext({
    viewport: device.viewport,
    userAgent: device.userAgent,
    isMobile: device.isMobile,
    hasTouch: device.hasTouch,
    deviceScaleFactor: device.deviceScaleFactor,
  });
  const page = await ctx.newPage();
  const consoleErrors = [];
  const pageErrors = [];
  const netFailures = [];
  page.on('console', m => {
    if (m.type() === 'error') consoleErrors.push(m.text().slice(0, 300));
  });
  page.on('pageerror', e => pageErrors.push(String(e).slice(0, 300)));
  page.on('response', resp => {
    const s = resp.status();
    if (s >= 400) netFailures.push(`${s} ${resp.request().method()} ${resp.url().slice(0, 140)}`);
  });
  return {
    ctx,
    page,
    consoleErrors,
    pageErrors,
    netFailures,
    async shot(name) {
      const path = `${SHOTS}/${device.name}-${name}.png`;
      try {
        await page.screenshot({ path, fullPage: false });
      } catch {
        /* page may be closed */
      }
      return path;
    },
    async teardown() {
      await ctx.close();
    },
  };
}

// Navigate with cold-start resilience: if the page is 503/blank, wake the
// machine and retry. Fly auto-stops the staging machine so the first hit after
// idle returns 503/000 — this absorbs that without polluting results.
export async function gotoSafe(page, url, { timeout = 45000, settle = 2000 } = {}) {
  for (let attempt = 0; attempt < 3; attempt++) {
    let resp = null;
    try {
      resp = await page.goto(url, { waitUntil: 'domcontentloaded', timeout });
    } catch {
      /* nav timeout/abort */
    }
    const status = resp ? resp.status() : 0;
    if (status >= 200 && status < 400) {
      await page.waitForTimeout(settle);
      const txt = await page.locator('body').innerText().catch(() => '');
      // blank/503 shell → wake + retry
      if (txt.length > 20 && !/service unavailable|503/i.test(txt)) return resp;
    }
    await wakeStaging();
    await page.waitForTimeout(1500);
  }
  return null;
}

export async function setToken(page, token) {
  // Must be on the origin before touching localStorage.
  await page.goto(`${BASE}/`, { waitUntil: 'domcontentloaded' }).catch(() => {});
  await page.evaluate(t => localStorage.setItem('dos_access_token', t), token);
}

// Tiny PASS/FAIL recorder.
export function recorder() {
  const rows = [];
  return {
    add(flow, assertion, status, evidence) {
      rows.push({ flow, assertion, status, evidence });
      const tag = status === 'PASS' ? 'PASS' : status === 'WARN' ? 'WARN' : 'FAIL';
      console.log(`[${tag}] ${flow} :: ${assertion} :: ${evidence || ''}`);
    },
    dump() {
      const pass = rows.filter(r => r.status === 'PASS').length;
      const fail = rows.filter(r => r.status === 'FAIL').length;
      const warn = rows.filter(r => r.status === 'WARN').length;
      console.log(`\n===SUMMARY=== PASS=${pass} FAIL=${fail} WARN=${warn} TOTAL=${rows.length}`);
      console.log('===JSON===');
      console.log(JSON.stringify(rows));
    },
  };
}

export { chromium };
