/**
 * Cutover WS router proofs (ADR-0022 REV-S6). S6 upgrades never traverse fastify hooks
 * (`ws` owns the raw 'upgrade' event), so the router wraps that event directly — proven
 * here against a REAL Node ws server and a REAL Rust-stub ws server (no mocked sockets):
 *   - dark shape (upstream null) → listeners untouched, Node ws serves byte-identically
 *   - S6=node flag with the router INSTALLED → passthrough, rust stub sees zero sockets
 *   - S6=rust WITHOUT readiness_ok → refused at read time (machine-gate) → Node
 *   - S6=rust+ready → RAW TCP splice: echo end-to-end through the proxy, trusted
 *     x-dowiz-internal-client-ip + x-correlation-id injected, inbound spoof stripped,
 *     Node ws NEVER also receives the proxied socket
 *   - upstream dead + S6=rust → falls through to Node ws (fail-safe: read/notify plane)
 *   - flip semantics: an ESTABLISHED Node socket keeps streaming from Node after the
 *     flag flips to rust; only the NEXT new upgrade lands on Rust
 */

import { test, describe } from 'node:test';
import assert from 'node:assert/strict';
import http from 'node:http';
import type { AddressInfo } from 'node:net';
import Fastify from 'fastify';
import { WebSocketServer, WebSocket } from 'ws';
import { CutoverFlagsStore } from '../src/lib/cutover/flags.js';
import { installCutoverWsRouter } from '../src/lib/cutover/ws-router.js';

const silentLog = { info: () => {}, warn: () => {} };
const silentStoreLog = { warn: () => {}, error: () => {}, debug: () => {} };

/** Real CutoverFlagsStore over a fake pool (same pattern as cutover-front-door.test.ts). */
async function makeFlags(
  rows: Array<{ surface: string; target: string; readiness_ok: boolean }>,
): Promise<CutoverFlagsStore> {
  const store = new CutoverFlagsStore(
    { query: async () => ({ rows }) } as any,
    { ttlMs: 60_000, log: silentStoreLog },
  );
  await store.refresh();
  return store;
}

/** Fastify app + real Node ws server attached exactly like websocket.ts (no path filter). */
async function makeNodeApp() {
  const app = Fastify({ logger: false });
  await app.listen({ port: 0, host: '127.0.0.1' });
  let connections = 0;
  const wss = new WebSocketServer({ server: app.server });
  wss.on('connection', (ws) => {
    connections += 1;
    ws.on('message', (d) => ws.send(`node:${d}`));
  });
  const port = (app.server.address() as AddressInfo).port;
  return {
    app,
    url: `ws://127.0.0.1:${port}`,
    nodeConnections: () => connections,
    close: async () => {
      for (const c of wss.clients) c.terminate();
      wss.close();
      await app.close();
    },
  };
}

/** Stub RAW rust upstream: its own http server + ws server, capturing handshake headers. */
function makeRustStub(): Promise<{
  url: URL;
  seen: http.IncomingHttpHeaders[];
  connections: () => number;
  close: () => Promise<void>;
}> {
  return new Promise((resolve) => {
    const server = http.createServer((req, res) => {
      res.writeHead(404);
      res.end();
    });
    let connections = 0;
    const seen: http.IncomingHttpHeaders[] = [];
    const wss = new WebSocketServer({ server });
    wss.on('connection', (ws, req) => {
      connections += 1;
      seen.push(req.headers);
      ws.on('message', (d) => ws.send(`rust:${d}`));
    });
    server.listen(0, '127.0.0.1', () => {
      const port = (server.address() as AddressInfo).port;
      resolve({
        url: new URL(`http://127.0.0.1:${port}`),
        seen,
        connections: () => connections,
        close: () =>
          new Promise((r) => {
            for (const c of wss.clients) c.terminate();
            wss.close();
            server.close(() => r());
          }),
      });
    });
  });
}

/** Persistent real ws client: request/response over one live socket. */
function openClient(
  url: string,
  headers?: Record<string, string>,
): Promise<{ ask: (m: string) => Promise<string>; close: () => void }> {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(url, { headers });
    const connectTimer = setTimeout(() => {
      ws.terminate();
      reject(new Error('ws connect timeout'));
    }, 5_000);
    ws.on('error', (err) => {
      clearTimeout(connectTimer);
      reject(err);
    });
    ws.on('open', () => {
      clearTimeout(connectTimer);
      resolve({
        ask: (m: string) =>
          new Promise<string>((res, rej) => {
            const t = setTimeout(() => rej(new Error('ws ask timeout')), 5_000);
            ws.once('message', (d) => {
              clearTimeout(t);
              res(d.toString());
            });
            ws.send(m);
          }),
        close: () => ws.terminate(),
      });
    });
  });
}

/** Connect, exchange exactly one message, disconnect. */
async function wsEcho(url: string, msg: string, headers?: Record<string, string>): Promise<string> {
  const client = await openClient(url, headers);
  try {
    return await client.ask(msg);
  } finally {
    client.close();
  }
}

describe('cutover WS router (S6, ADR-0022 REV-S6)', () => {
  test('dark shape: upstream null → listeners untouched, Node ws serves normally', async () => {
    const node = await makeNodeApp();
    try {
      const before = node.app.server.listeners('upgrade').length;
      installCutoverWsRouter(node.app.server, {
        flags: await makeFlags([{ surface: 'S6', target: 'rust', readiness_ok: true }]),
        upstream: null,
        log: silentLog,
      });
      assert.equal(
        node.app.server.listeners('upgrade').length,
        before,
        'dark shape must not touch the upgrade listeners at all',
      );
      assert.equal(await wsEcho(node.url, 'ping'), 'node:ping');
      assert.equal(node.nodeConnections(), 1);
    } finally {
      await node.close();
    }
  });

  test('S6=node flag with router installed → passthrough; rust stub sees zero sockets', async () => {
    const node = await makeNodeApp();
    const rust = await makeRustStub();
    try {
      installCutoverWsRouter(node.app.server, {
        flags: await makeFlags([{ surface: 'S6', target: 'node', readiness_ok: true }]),
        upstream: rust.url,
        log: silentLog,
      });
      assert.equal(await wsEcho(node.url, 'ping'), 'node:ping');
      assert.equal(node.nodeConnections(), 1);
      assert.equal(rust.connections(), 0, 'node-flagged S6 must never reach the upstream');
    } finally {
      await node.close();
      await rust.close();
    }
  });

  test('S6=rust WITHOUT readiness_ok → refused at read time → Node (machine-gate)', async () => {
    const node = await makeNodeApp();
    const rust = await makeRustStub();
    try {
      installCutoverWsRouter(node.app.server, {
        flags: await makeFlags([{ surface: 'S6', target: 'rust', readiness_ok: false }]),
        upstream: rust.url,
        log: silentLog,
      });
      assert.equal(await wsEcho(node.url, 'ping'), 'node:ping');
      assert.equal(rust.connections(), 0, 'flip without recorded-green DoD must be refused');
    } finally {
      await node.close();
      await rust.close();
    }
  });

  test('S6=rust+ready → raw splice: echo via upstream, trusted headers injected, spoof stripped', async () => {
    const node = await makeNodeApp();
    const rust = await makeRustStub();
    try {
      installCutoverWsRouter(node.app.server, {
        flags: await makeFlags([{ surface: 'S6', target: 'rust', readiness_ok: true }]),
        upstream: rust.url,
        log: silentLog,
      });
      const echoed = await wsEcho(node.url, 'ping', {
        'x-dowiz-internal-client-ip': '6.6.6.6', // spoof attempt — must be stripped
      });
      assert.equal(echoed, 'rust:ping', 'echo must travel end-to-end through the splice');
      assert.equal(rust.connections(), 1);
      assert.equal(node.nodeConnections(), 0, 'Node ws must NOT also receive the proxied socket');
      const h = rust.seen[0]!;
      assert.equal(h['x-dowiz-internal-client-ip'], '127.0.0.1', 'spoof replaced by the real client ip');
      assert.ok(h['x-correlation-id'], 'correlation id travels with the upgrade');
      assert.equal(h['upgrade'], 'websocket', 'WS handshake headers forwarded verbatim');
      assert.ok(h['sec-websocket-key'], 'sec-websocket-* headers forwarded');
    } finally {
      await node.close();
      await rust.close();
    }
  });

  test('upstream dead + S6=rust → falls through to Node ws (fail-safe)', async () => {
    const node = await makeNodeApp();
    try {
      installCutoverWsRouter(node.app.server, {
        flags: await makeFlags([{ surface: 'S6', target: 'rust', readiness_ok: true }]),
        upstream: new URL('http://127.0.0.1:1'), // dead port: connection refused
        log: silentLog,
      });
      assert.equal(await wsEcho(node.url, 'ping'), 'node:ping', 'S6 is read/notify — fail-safe is Node');
      assert.equal(node.nodeConnections(), 1);
    } finally {
      await node.close();
    }
  });

  test('flip semantics: established Node socket stays on Node; only NEW upgrades route to Rust', async () => {
    const rows = [{ surface: 'S6', target: 'node', readiness_ok: true }];
    const store = await makeFlags(rows);
    const node = await makeNodeApp();
    const rust = await makeRustStub();
    let preFlip: Awaited<ReturnType<typeof openClient>> | null = null;
    try {
      installCutoverWsRouter(node.app.server, {
        flags: store,
        upstream: rust.url,
        log: silentLog,
      });
      // Pre-flip: a client connects to Node and STAYS connected.
      preFlip = await openClient(node.url);
      assert.equal(await preFlip.ask('one'), 'node:one');
      // Flip S6 → rust (row mutated + store refreshed, as the TTL poll would).
      rows[0]!.target = 'rust';
      await store.refresh();
      // NEW upgrade lands on Rust…
      assert.equal(await wsEcho(node.url, 'two'), 'rust:two');
      assert.equal(rust.connections(), 1);
      // …while the pre-flip socket keeps streaming from Node until it disconnects.
      assert.equal(await preFlip.ask('three'), 'node:three', 'flip must never hijack an established socket');
    } finally {
      preFlip?.close();
      await node.close();
      await rust.close();
    }
  });
});
