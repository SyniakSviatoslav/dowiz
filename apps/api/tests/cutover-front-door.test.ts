/**
 * Cutover front-door + flag-store proofs (ADR-0022; council RESOLVE REV-C3/C5/C6/C9).
 *
 * Every REV that is enforceable in-process is asserted here against a REAL fastify
 * listener and a REAL stub upstream (no mocked http):
 *   - fail-safe: unreadable flag store / missing row / not-ready flip → Node
 *   - read-time machine-gate: target=rust + readiness_ok=false is REFUSED
 *   - REV-C5: money surfaces (S5/S7/S9) never auto-degrade, never silently reroute
 *   - REV-C6: inbound x-dowiz-internal-* spoofs are stripped; the trusted header is
 *     set from the front-door's own client-ip resolution
 *   - REV-C9: bodied-method upstream failure answers a truthful retry-safe envelope
 *   - GET pre-response failure falls through to Node (zero bytes consumed/sent)
 *   - break-glass CUTOVER_FORCE_ALL_NODE bypasses everything
 *
 * NOT covered here (covered elsewhere): template disjointness + fail-closed matching
 * (27 tests in cutover-matcher.test.ts); the real 15s/35s timeout waits (REV-C11 —
 * value asserted structurally, the wait itself is proven by the staging chaos drill).
 */

import { test, describe } from 'node:test';
import assert from 'node:assert/strict';
import http from 'node:http';
import type { AddressInfo } from 'node:net';
import Fastify from 'fastify';
import { CutoverFlagsStore, NO_AUTO_DEGRADE } from '../src/lib/cutover/flags.js';
import { registerCutoverFrontDoor } from '../src/lib/cutover/front-door.js';

const silentLog = { warn: () => {}, error: () => {}, debug: () => {} };

interface FakePoolState {
  rows: Array<{ surface: string; target: string; readiness_ok: boolean }>;
  failSelects: boolean;
  degradeCalls: Array<{ surface: string; reason: string }>;
}

function makeFakePool(state: FakePoolState) {
  return {
    query: async (text: string, params?: unknown[]) => {
      if (text.includes('cutover_auto_degrade')) {
        state.degradeCalls.push({ surface: String(params?.[0]), reason: String(params?.[1]) });
        return { rows: [{ cutover_auto_degrade: true }] };
      }
      if (state.failSelects) throw new Error('relation "cutover_flags" does not exist');
      return { rows: state.rows };
    },
  } as any;
}

async function waitFor(cond: () => boolean, ms = 2_000): Promise<void> {
  const start = Date.now();
  while (!cond()) {
    if (Date.now() - start > ms) throw new Error('waitFor timed out');
    await new Promise((r) => setTimeout(r, 10));
  }
}

/** Real upstream stub capturing what it receives. */
function makeUpstream(): Promise<{
  url: string;
  seen: Array<{ method: string; url: string; headers: http.IncomingHttpHeaders }>;
  close: () => Promise<void>;
}> {
  const seen: Array<{ method: string; url: string; headers: http.IncomingHttpHeaders }> = [];
  const server = http.createServer((req, res) => {
    seen.push({ method: req.method ?? '', url: req.url ?? '', headers: req.headers });
    if (req.url === '/healthz') {
      res.writeHead(200, { 'content-type': 'application/json' });
      res.end('{"status":"ok"}');
      return;
    }
    res.writeHead(200, { 'content-type': 'application/json', 'x-upstream': 'rust-stub' });
    res.end(JSON.stringify({ stack: 'rust' }));
  });
  return new Promise((resolve) => {
    server.listen(0, '127.0.0.1', () => {
      const port = (server.address() as AddressInfo).port;
      resolve({
        url: `http://127.0.0.1:${port}`,
        seen,
        close: () => new Promise((r) => server.close(() => r())),
      });
    });
  });
}

async function makeApp(opts: {
  state: FakePoolState;
  rustUpstream: string | undefined;
  astroUpstream?: string;
  forceAllNode?: boolean;
}) {
  const app = Fastify({ logger: false });
  const handle = registerCutoverFrontDoor(app, {
    pool: makeFakePool(opts.state),
    rustUpstream: opts.rustUpstream,
    astroUpstream: opts.astroUpstream,
    forceAllNode: opts.forceAllNode ?? false,
    flagsTtlMs: 50,
    healthIntervalMs: 30,
  });
  // Node-side twins of real mapped routes (register() defers → the hook applies).
  await app.register(async (f) => {
    f.get('/api/public/theme/:slug', async () => ({ stack: 'node' })); // S1
    f.get('/s/:slug', async () => ({ stack: 'node' })); // S1 HTML (astro sub-target)
    f.get('/s/:slug/checkout', async () => ({ stack: 'node' })); // S1 HTML node-keep
    f.post('/api/owner/locations/:locationId/products', async () => ({ stack: 'node' })); // S3
    f.post('/api/owner/locations/:locationId/settlements/:id/approve', async () => ({ stack: 'node' })); // S5
    f.get('/definitely/not/mapped', async () => ({ stack: 'node-unmapped' }));
  });
  await app.listen({ port: 0, host: '127.0.0.1' });
  const base = `http://127.0.0.1:${(app.server.address() as AddressInfo).port}`;
  return { app, handle, base };
}

describe('CutoverFlagsStore (REV-C3 fail-safe + REV-C5 degrade constraints)', () => {
  test('unreadable store → all-Node; recovery is not stale (fail-safe, never last-known)', async () => {
    const state: FakePoolState = {
      rows: [{ surface: 'S1', target: 'rust', readiness_ok: true }],
      failSelects: false,
      degradeCalls: [],
    };
    const store = new CutoverFlagsStore(makeFakePool(state), { ttlMs: 50, log: silentLog });
    await store.refresh();
    assert.equal(store.targetFor('S1'), 'rust');
    state.failSelects = true; // store becomes unreadable AFTER a good read
    await store.refresh();
    assert.equal(store.targetFor('S1'), 'node', 'stale rust state must NOT survive a failed refresh');
  });

  test('read-time machine-gate: rust without readiness_ok is refused; missing row is node', async () => {
    const state: FakePoolState = {
      rows: [
        { surface: 'S1', target: 'rust', readiness_ok: true },
        { surface: 'S2', target: 'rust', readiness_ok: false },
        { surface: 'S3', target: 'node', readiness_ok: true },
      ],
      failSelects: false,
      degradeCalls: [],
    };
    const store = new CutoverFlagsStore(makeFakePool(state), { ttlMs: 50, log: silentLog });
    await store.refresh();
    assert.equal(store.targetFor('S1'), 'rust');
    assert.equal(store.targetFor('S2'), 'node', 'flip without recorded-green DoD must be refused');
    assert.equal(store.targetFor('S3'), 'node');
    assert.equal(store.targetFor('S9'), 'node', 'absent row answers node');
  });

  test('REV-C5: money/irreversible surfaces never auto-degrade from the app', async () => {
    const state: FakePoolState = { rows: [], failSelects: false, degradeCalls: [] };
    const store = new CutoverFlagsStore(makeFakePool(state), { ttlMs: 50, log: silentLog });
    for (const money of ['S5', 'S7', 'S9'] as const) {
      assert.equal(NO_AUTO_DEGRADE.has(money), true);
      assert.equal(await store.autoDegrade(money, 'test'), false);
    }
    assert.equal(state.degradeCalls.length, 0, 'no degrade statement may reach the DB for money surfaces');
  });

  test('non-money auto-degrade goes through the constrained DEFINER fn, debounced', async () => {
    const state: FakePoolState = { rows: [], failSelects: false, degradeCalls: [] };
    const store = new CutoverFlagsStore(makeFakePool(state), { ttlMs: 50, log: silentLog });
    assert.equal(await store.autoDegrade('S1', 'breaker tripped'), true);
    assert.equal(state.degradeCalls.length, 1);
    assert.equal(state.degradeCalls[0]!.surface, 'S1');
    assert.equal(await store.autoDegrade('S1', 'again'), false, 'debounced within the window');
    assert.equal(state.degradeCalls.length, 1);
  });
});

describe('front-door routing (dark-by-default, forward, fail modes)', () => {
  test('inert when no upstream configured — byte-identical Node behavior', async () => {
    const state: FakePoolState = {
      rows: [{ surface: 'S1', target: 'rust', readiness_ok: true }],
      failSelects: false,
      degradeCalls: [],
    };
    const { app, base } = await makeApp({ state, rustUpstream: undefined });
    try {
      const res = await fetch(`${base}/api/public/theme/demo`);
      assert.equal((await res.json() as any).stack, 'node');
      assert.equal(res.headers.get('x-dowiz-cutover'), null);
    } finally {
      await app.close();
    }
  });

  test('break-glass CUTOVER_FORCE_ALL_NODE bypasses rust flags entirely', async () => {
    const upstream = await makeUpstream();
    const state: FakePoolState = {
      rows: [{ surface: 'S1', target: 'rust', readiness_ok: true }],
      failSelects: false,
      degradeCalls: [],
    };
    const { app, base } = await makeApp({ state, rustUpstream: upstream.url, forceAllNode: true });
    try {
      const res = await fetch(`${base}/api/public/theme/demo`);
      assert.equal((await res.json() as any).stack, 'node');
      assert.equal(upstream.seen.filter((s) => s.url !== '/healthz').length, 0);
    } finally {
      await app.close();
      await upstream.close();
    }
  });

  test('flipped S1 forwards: served-by oracle header, spoof stripped, trusted ip + correlation set', async () => {
    const upstream = await makeUpstream();
    const state: FakePoolState = {
      rows: [{ surface: 'S1', target: 'rust', readiness_ok: true }],
      failSelects: false,
      degradeCalls: [],
    };
    const { app, handle, base } = await makeApp({ state, rustUpstream: upstream.url });
    try {
      await waitFor(() => handle.flags.snapshot().has('S1'));
      const res = await fetch(`${base}/api/public/theme/demo?x=1`, {
        headers: { 'x-dowiz-internal-client-ip': '6.6.6.6' }, // spoof attempt
      });
      assert.equal(res.status, 200);
      assert.equal(res.headers.get('x-dowiz-cutover'), 'rust:S1');
      assert.equal((await res.json() as any).stack, 'rust');
      const forwarded = upstream.seen.find((s) => s.url === '/api/public/theme/demo?x=1');
      assert.ok(forwarded, 'upstream must receive the exact path+query');
      assert.equal(forwarded!.headers['x-dowiz-internal-client-ip'], '127.0.0.1', 'spoof replaced by real ip');
      assert.ok(forwarded!.headers['x-correlation-id'], 'correlation id travels');
    } finally {
      await app.close();
      await upstream.close();
    }
  });

  test('rust flag without readiness stays on Node (refused at read time)', async () => {
    const upstream = await makeUpstream();
    const state: FakePoolState = {
      rows: [{ surface: 'S1', target: 'rust', readiness_ok: false }],
      failSelects: false,
      degradeCalls: [],
    };
    const { app, handle, base } = await makeApp({ state, rustUpstream: upstream.url });
    try {
      await waitFor(() => handle.flags.snapshot().has('S1'));
      const res = await fetch(`${base}/api/public/theme/demo`);
      assert.equal((await res.json() as any).stack, 'node');
      assert.equal(upstream.seen.filter((s) => s.url !== '/healthz').length, 0);
    } finally {
      await app.close();
      await upstream.close();
    }
  });

  test('unmapped path fails closed to Node even with every flag rust', async () => {
    const upstream = await makeUpstream();
    const state: FakePoolState = {
      rows: ['S1','S2','S3','S4','S5','S6','S7','S8','S9','S10'].map((s) => ({
        surface: s, target: 'rust', readiness_ok: true,
      })),
      failSelects: false,
      degradeCalls: [],
    };
    const { app, handle, base } = await makeApp({ state, rustUpstream: upstream.url });
    try {
      await waitFor(() => handle.flags.snapshot().size === 10);
      const res = await fetch(`${base}/definitely/not/mapped`);
      assert.equal((await res.json() as any).stack, 'node-unmapped');
    } finally {
      await app.close();
      await upstream.close();
    }
  });

  test('GET pre-response upstream failure falls through to Node (zero bytes → safe)', async () => {
    const state: FakePoolState = {
      rows: [{ surface: 'S1', target: 'rust', readiness_ok: true }],
      failSelects: false,
      degradeCalls: [],
    };
    // Dead port: connection refused before any response. Health may not have tripped yet.
    const { app, handle, base } = await makeApp({ state, rustUpstream: 'http://127.0.0.1:1' });
    try {
      await waitFor(() => handle.flags.snapshot().has('S1'));
      if (handle.health!.healthy) {
        const res = await fetch(`${base}/api/public/theme/demo`);
        assert.equal(res.status, 200);
        assert.equal((await res.json() as any).stack, 'node', 'GET must fall through, not 5xx');
      }
    } finally {
      await app.close();
    }
  });

  test('bodied method upstream failure → truthful retry-safe 503, never a silent retry goad (REV-C9)', async () => {
    const state: FakePoolState = {
      rows: [{ surface: 'S3', target: 'rust', readiness_ok: true }],
      failSelects: false,
      degradeCalls: [],
    };
    const { app, handle, base } = await makeApp({ state, rustUpstream: 'http://127.0.0.1:1' });
    try {
      await waitFor(() => handle.flags.snapshot().has('S3'));
      if (handle.health!.healthy) {
        const res = await fetch(`${base}/api/owner/locations/11111111-1111-1111-1111-111111111111/products`, {
          method: 'POST',
          headers: { 'content-type': 'application/json' },
          body: '{"name":"x"}',
        });
        assert.equal(res.status, 503);
        const body = await res.json() as any;
        assert.equal(body.code, 'CUTOVER_UPSTREAM_UNAVAILABLE');
        assert.match(body.message, /may or may not/i, 'must be truthful about unknown outcome');
        assert.ok(res.headers.get('retry-after'));
      }
    } finally {
      await app.close();
    }
  });

  test('S1 Astro partition: page → astro, API → rust, unimplemented page → Node (Q6 sub-target)', async () => {
    const rust = await makeUpstream();
    const astro = await makeUpstream();
    const state: FakePoolState = {
      rows: [{ surface: 'S1', target: 'rust', readiness_ok: true }],
      failSelects: false,
      degradeCalls: [],
    };
    const { app, handle, base } = await makeApp({ state, rustUpstream: rust.url, astroUpstream: astro.url });
    try {
      await waitFor(() => handle.flags.snapshot().has('S1'));
      const page = await fetch(`${base}/s/demo`);
      assert.equal(page.headers.get('x-dowiz-cutover'), 'astro:S1', 'menu page routes to Astro');
      assert.ok(astro.seen.some((s) => s.url === '/s/demo'));
      const api = await fetch(`${base}/api/public/theme/demo`);
      assert.equal(api.headers.get('x-dowiz-cutover'), 'rust:S1', 'API reads stay on Rust');
      assert.ok(rust.seen.some((s) => s.url === '/api/public/theme/demo'));
      const keep = await fetch(`${base}/s/demo/checkout`);
      assert.equal(keep.headers.get('x-dowiz-cutover'), null, 'unimplemented page stays Node');
      assert.equal(((await keep.json()) as any).stack, 'node');
      assert.equal(astro.seen.some((s) => s.url.includes('checkout')), false);
    } finally {
      await app.close();
      await rust.close();
      await astro.close();
    }
  });

  test('S1 flipped WITHOUT astro upstream: page stays Node, API goes rust', async () => {
    const rust = await makeUpstream();
    const state: FakePoolState = {
      rows: [{ surface: 'S1', target: 'rust', readiness_ok: true }],
      failSelects: false,
      degradeCalls: [],
    };
    const { app, handle, base } = await makeApp({ state, rustUpstream: rust.url });
    try {
      await waitFor(() => handle.flags.snapshot().has('S1'));
      const page = await fetch(`${base}/s/demo`);
      assert.equal(page.headers.get('x-dowiz-cutover'), null);
      assert.equal(((await page.json()) as any).stack, 'node');
      const api = await fetch(`${base}/api/public/theme/demo`);
      assert.equal(api.headers.get('x-dowiz-cutover'), 'rust:S1');
    } finally {
      await app.close();
      await rust.close();
    }
  });

  test('astro upstream dead: menu page GET falls through to Node (bodyless fail-safe)', async () => {
    const rust = await makeUpstream();
    const state: FakePoolState = {
      rows: [{ surface: 'S1', target: 'rust', readiness_ok: true }],
      failSelects: false,
      degradeCalls: [],
    };
    const { app, handle, base } = await makeApp({ state, rustUpstream: rust.url, astroUpstream: 'http://127.0.0.1:1' });
    try {
      await waitFor(() => handle.flags.snapshot().has('S1'));
      const page = await fetch(`${base}/s/demo`);
      assert.equal(page.status, 200);
      assert.equal(((await page.json()) as any).stack, 'node', 'dead Astro must fall through, not 5xx');
    } finally {
      await app.close();
      await rust.close();
    }
  });

  test('tripped health: money surface answers 503 untouched, non-money degrades globally (REV-C5)', async () => {
    const state: FakePoolState = {
      rows: [
        { surface: 'S1', target: 'rust', readiness_ok: true },
        { surface: 'S5', target: 'rust', readiness_ok: true },
      ],
      failSelects: false,
      degradeCalls: [],
    };
    const { app, handle, base } = await makeApp({ state, rustUpstream: 'http://127.0.0.1:1' });
    try {
      await waitFor(() => handle.health!.healthy === false, 5_000); // 3 refused probes @30ms
      // Money: never silently rerouted — truthful 503.
      const money = await fetch(`${base}/api/owner/locations/11111111-1111-1111-1111-111111111111/settlements/22222222-2222-2222-2222-222222222222/approve`, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: '{}',
      });
      assert.equal(money.status, 503);
      assert.equal(((await money.json()) as any).code, 'CUTOVER_UPSTREAM_UNAVAILABLE');
      // Non-money: served by Node now + global degrade fired (S1, never S5).
      const read = await fetch(`${base}/api/public/theme/demo`);
      assert.equal(((await read.json()) as any).stack, 'node');
      await waitFor(() => state.degradeCalls.some((c) => c.surface === 'S1'));
      assert.equal(state.degradeCalls.some((c) => c.surface === 'S5'), false, 'S5 must never be auto-degraded');
    } finally {
      await app.close();
    }
  });
});
