import { test, expect, type Page } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import { setTimeout as delay } from 'node:timers/promises';

/**
 * Tier-2 storefront QA gate (docs/design/TIER-2-QUALITY-BARS-2026-07-12.md §2).
 *
 * Self-contained: boots the canonical Rust `dowiz-server` (serving the already-built
 * `web/dist` SPA) on :3000, then exercises BOTH surfaces the gate protects:
 *   1. the SPA renders with no console errors (§1 item 11), and
 *   2. the real order contract through the API (§1 item 3 integer money / no tween,
 *      §2 item 5 `?ch=` → channel attribution, plus the kernel 409 RED).
 *
 * Run:
 *   VITE_BASE_URL=http://localhost:3000 \
 *     npx playwright test e2e/tests/tier2-storefront-contract.spec.ts --project=desktop
 *
 * (VITE_BASE_URL suppresses the legacy Node webServer in playwright.config.ts so this
 * spec can own the port with the Rust server.)
 */

const DB = `/tmp/dowiz_tier2_gate_${process.pid}.sqlite`;
let server: ChildProcess | undefined;

async function waitForServer(timeoutMs = 30000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const r = await fetch('http://localhost:3000/api/healthz');
      if (r.ok) return;
    } catch (error) {
      // best-effort poll; server may not be listening yet
      void error;
    }
    await delay(250);
  }
  throw new Error('dowiz-server did not come up on :3000');
}

// Free port 3000 robustly: find listeners via lsof, kill them, but never match
// this own process. fuser can self-match pkill patterns, so we scope by PID list.
async function freePort3000() {
  try {
    const { execSync } = require('node:child_process');
    const pids = execSync('lsof -t -iTCP:3000 -sTCP:LISTEN 2>/dev/null || true')
      .toString()
      .trim()
      .split('\n')
      .filter(Boolean);
    for (const pid of pids) {
      const n = Number(pid);
      if (n && n !== process.pid) process.kill(n, 'SIGKILL');
    }
  } catch (error) {
    // best-effort: freeing the port is not fatal if lsof is absent
    void error;
  }
  await delay(500);
}

test.beforeAll(async () => {
  await freePort3000();
  try {
    require('node:fs').rmSync(DB, { force: true });
  } catch (error) {
    // best-effort cleanup; a fresh DB is created on boot regardless
    void error;
  }
  server = spawn('server/target/debug/dowiz-server', [], {
    cwd: process.cwd(),
    env: { ...process.env, DOWIZ_DB: DB, DOWIZ_DIST: 'web/dist' },
    stdio: 'ignore',
  });
  await waitForServer();
});

test.afterAll(() => {
  server?.kill('SIGKILL');
  try {
    require('node:fs').rmSync(DB, { force: true });
  } catch (error) {
    // best-effort cleanup of the test sqlite file
    void error;
  }
});

test('storefront SPA renders with no console errors', async ({ page }: { page: Page }) => {
  const errors: string[] = [];
  page.on('console', (m) => {
    if (m.type() === 'error') errors.push(m.text());
  });
  page.on('pageerror', (e) => errors.push(e.message));

  await page.goto('/', { waitUntil: 'networkidle' });

  // §1 item 11: no console.error on happy path.
  expect(errors, `console errors: ${errors.join(' | ')}`).toHaveLength(0);

  // The canonical storefront menu renders (built web/dist shows these demo items).
  // `exact: true` avoids the modifier labels ("Double pepperoni") matching loosely.
  await expect(page.getByText('Margherita', { exact: true })).toBeVisible();
  await expect(page.getByText('Pepperoni', { exact: true })).toBeVisible();
});

test('order contract: integer money, PENDING, persisted, 409 on illegal transition', async ({
  request,
}) => {
  // Place an order through the real API (server-authoritative integer money).
  const created = await request.post('/api/orders', {
    data: {
      location_id: 'v-t2',
      channel: 'tiktok',
      items: [{ product_id: 'p1', quantity: 2, unit_price: 900 }],
      cash_pay_with: '5000',
    },
  });
  expect(created.status()).toBe(201);
  const body = await created.json();

  // §1 item 3: order total is integer minor units, never a float; status PENDING.
  expect(body.status).toBe('PENDING');
  expect(Number.isInteger(body.subtotal)).toBe(true);
  expect(Number.isInteger(body.total)).toBe(true);
  expect(body.subtotal).toBe(1800); // 2 × 900, exact integer math
  expect(body.total).toBe(1800);
  const orderId = body.id;

  // Reload via channel ledger: the order is persisted and attributed to its channel.
  const channel = await request.get('/api/orders/channel');
  const chBody = await channel.json();
  const tiktok = chBody.orders_by_channel.find((c: { channel: string }) => c.channel === 'tiktok');
  expect(tiktok?.count).toBeGreaterThanOrEqual(1);

  // RED: an illegal transition (PENDING -> DELIVERED) must be rejected with 409,
  // proving the kernel decide/fold Law is the single source of truth.
  const illegal = await request.post(`/api/orders/${orderId}/event`, {
    data: { next_status: 'DELIVERED' },
  });
  expect(illegal.status()).toBe(409);

  // A legal first transition is accepted.
  const legal = await request.post(`/api/orders/${orderId}/event`, {
    data: { next_status: 'CONFIRMED' },
  });
  expect(legal.status()).toBe(200);
  expect((await legal.json()).status).toBe('CONFIRMED');
});

test('Tier-3 plumbing: claimed venue + ?ch= order attributes to that venue', async ({
  request,
}) => {
  const venue = 'v-tokyo';
  const claim = await request.post(`/api/venues/${venue}/claim`, {
    data: { owner_id: 'o-t2', name: 'Tokyo' },
  });
  expect(claim.status()).toBe(200);
  expect((await claim.json()).claimed).toBe(true);

  // Unknown venue is 404 (RED).
  const missing = await request.get('/api/venues/does-not-exist');
  expect(missing.status()).toBe(404);

  // An order stamped with the venue id as its channel is attributed to it.
  const order = await request.post('/api/orders', {
    data: {
      location_id: venue,
      channel: venue,
      items: [{ product_id: 'p1', quantity: 1, unit_price: 900 }],
      cash_pay_with: '1000',
    },
  });
  expect(order.status()).toBe(201);

  const channel = await request.get('/api/orders/channel');
  const chBody = await channel.json();
  const row = chBody.orders_by_channel.find((c: { channel: string }) => c.channel === venue);
  expect(row?.count).toBeGreaterThanOrEqual(1);
});
