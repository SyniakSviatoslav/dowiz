/**
 * Cutover WS router (ADR-0022 / REV-S6) — the S6 half the HTTP front-door structurally
 * cannot serve: WS upgrades never traverse fastify hooks because `ws` mounts its own
 * 'upgrade' listener on the raw HTTP server (websocket.ts:192, no `path` filter), so
 * EVERY HTTP Upgrade belongs to S6 by protocol, not by path (matcher.ts
 * isWebSocketUpgrade). This module owns that raw event.
 *
 * INTERCEPTION (wrap-and-replace, NOT prependListener): installed AFTER setupWebSocket
 * has attached the ws listener, it captures the existing 'upgrade' listeners, removes
 * them, and installs ITSELF as the single listener. Passthrough replays
 * (req, socket, head) to the captured originals — byte-identical to today. Chosen over
 * prependListener because Node emits 'upgrade' to EVERY listener: a prepended proxy
 * would splice the socket to Rust while the Node ws server ALSO completes a 101
 * handshake on the same socket — two handshake writers is corruption, not routing.
 * Wrapping makes the ordering deterministic regardless of when either side attaches.
 *
 * DECISION (per NEW upgrade only — never an established socket):
 *   - upstream null → never installed at all (dark shape: listeners untouched).
 *   - flags.targetFor('S6') !== 'rust' (node flag / missing row / readiness_ok=false
 *     machine-gate / unreadable store / break-glass never-started store) → passthrough
 *     to the Node ws server. Zero behavior change.
 *   - 'rust' → RAW TCP splice: connect to the upstream (5s CONNECT budget, NO timeout
 *     once spliced — WS sockets are long-lived by design), replay the request line +
 *     headers (hop-by-hop stripped EXCEPT the WS handshake set: connection / upgrade /
 *     sec-websocket-* travel verbatim), strip inbound x-dowiz-internal-* spoofs and
 *     inject the trusted x-dowiz-internal-client-ip (REV-C6) + a fresh x-correlation-id,
 *     write the head buffer, then pipe both directions. The 101 — or any refusal —
 *     reaches the client verbatim from Rust.
 *   - upstream connect failure/timeout → fall through to the Node ws server. S6 is the
 *     read/notify plane, so fail-safe = the incumbent stack; safe because zero bytes
 *     have touched the client socket before the splice.
 *
 * FLIP SEMANTICS: only NEW upgrades consult the flag. Sockets already handed to the
 * Node ws server keep streaming from Node until they naturally disconnect; their
 * reconnect then lands on Rust. No mass-disconnect at flip time, in either direction.
 *
 * Zero new dependencies: node:net for the splice, node:crypto for the correlation id.
 */

import net from 'node:net';
import { randomUUID } from 'node:crypto';
import type http from 'node:http';
import type { Duplex } from 'node:stream';
import { clientIp } from '../client-ip.js';
import type { CutoverFlagsStore } from './flags.js';

type UpgradeListener = (req: http.IncomingMessage, socket: Duplex, head: Buffer) => void;

export interface CutoverWsRouterOptions {
  /** The SAME store the HTTP front-door polls — one flip authority, one TTL window. */
  flags: Pick<CutoverFlagsStore, 'targetFor'>;
  /** e.g. new URL('http://dowiz-rust-staging.flycast') — null leaves the router uninstalled. */
  upstream: URL | null;
  log: {
    info: (obj: object, msg: string) => void;
    warn: (obj: object, msg: string) => void;
  };
}

const CONNECT_TIMEOUT_MS = 5_000; // CONNECT phase only — cleared the moment the splice starts

/**
 * Hop-by-hop headers stripped from the replay. Deliberately NOT the front-door's full
 * set: `connection` and `upgrade` are hop-by-hop but ARE the WS handshake — they must
 * travel. `host` is rewritten to the upstream; x-dowiz-internal-* is the spoof guard.
 */
const STRIP_HEADERS = new Set([
  'keep-alive',
  'proxy-authenticate',
  'proxy-authorization',
  'te',
  'trailer',
  'transfer-encoding',
  'host',
]);

/** Replay of the client's upgrade request: original line + headers, trust headers injected. */
function buildUpgradeHead(req: http.IncomingMessage, upstream: URL): string {
  const lines = [`${req.method ?? 'GET'} ${req.url ?? '/'} HTTP/1.1`];
  // rawHeaders preserves duplicates + original casing (Sec-WebSocket-Protocol may repeat).
  for (let i = 0; i + 1 < req.rawHeaders.length; i += 2) {
    const name = req.rawHeaders[i]!;
    const lower = name.toLowerCase();
    if (STRIP_HEADERS.has(lower)) continue;
    if (lower.startsWith('x-dowiz-internal-')) continue; // spoof guard (REV-C6)
    if (lower === 'x-correlation-id') continue; // server-authoritative — re-issued below
    lines.push(`${name}: ${req.rawHeaders[i + 1]!}`);
  }
  lines.push(`Host: ${upstream.host}`);
  // Same trust model as the HTTP front-door: Fly-Client-IP is the only trusted source;
  // clientIp() fails safe when it is absent (non-prod → socket address).
  lines.push(
    `X-Dowiz-Internal-Client-Ip: ${clientIp({ headers: req.headers, ip: req.socket?.remoteAddress })}`,
  );
  lines.push(`X-Correlation-Id: ${randomUUID()}`);
  return lines.join('\r\n') + '\r\n\r\n';
}

/**
 * Install the S6 upgrade router on the raw HTTP server. Call AFTER setupWebSocket has
 * attached the Node ws listener — the router captures and wraps whatever is there.
 * With a null upstream this is a no-op (the dark-deploy shape, mirroring the front-door).
 */
export function installCutoverWsRouter(server: http.Server, opts: CutoverWsRouterOptions): void {
  if (!opts.upstream) return; // inert: no listener churn, no decision path, no overhead
  const upstream = opts.upstream;
  const { flags, log } = opts;
  const upstreamPort = Number(upstream.port) || (upstream.protocol === 'https:' ? 443 : 80);

  const originals = server.listeners('upgrade') as UpgradeListener[];
  server.removeAllListeners('upgrade');

  const passThrough: UpgradeListener = (req, socket, head) => {
    if (originals.length === 0) {
      // Mirror Node's default when a server has no 'upgrade' listener: destroy the socket.
      socket.destroy();
      return;
    }
    for (const listener of originals) listener.call(server, req, socket, head);
  };

  const proxyUpgrade: UpgradeListener = (req, socket, head) => {
    let spliced = false;
    let settled = false; // true once this upgrade has a final home (splice, fallthrough, or dead client)

    const upstreamSocket = net.connect({ host: upstream.hostname, port: upstreamPort });
    upstreamSocket.setNoDelay(true);
    upstreamSocket.setTimeout(CONNECT_TIMEOUT_MS);

    // Client bailed while we were still connecting — abort the upstream attempt.
    const onClientEarlyExit = () => {
      if (!spliced) {
        settled = true;
        upstreamSocket.destroy();
      }
    };
    socket.on('error', onClientEarlyExit);
    socket.on('close', onClientEarlyExit);

    const fallThrough = (reason: string) => {
      if (settled) return;
      settled = true;
      socket.removeListener('error', onClientEarlyExit);
      socket.removeListener('close', onClientEarlyExit);
      upstreamSocket.destroy();
      log.warn(
        { reason, upstream: upstream.host },
        '[cutover] S6 rust upstream unreachable — upgrade falls through to Node ws (fail-safe)',
      );
      // Safe: zero bytes have been read from or written to the client socket.
      passThrough(req, socket, head);
    };

    upstreamSocket.on('timeout', () => {
      if (!spliced) fallThrough(`connect timeout after ${CONNECT_TIMEOUT_MS}ms`);
    });
    upstreamSocket.on('error', (err) => {
      if (!spliced) fallThrough(err.message);
      else socket.destroy();
    });
    upstreamSocket.on('close', () => {
      if (spliced) socket.destroy();
    });

    upstreamSocket.on('connect', () => {
      if (settled) return; // client already gone — nothing left to splice
      spliced = true;
      settled = true;
      upstreamSocket.setTimeout(0); // long-lived WS stream: CONNECT budget only, never idle-kill
      socket.removeListener('error', onClientEarlyExit);
      socket.removeListener('close', onClientEarlyExit);
      socket.on('error', () => upstreamSocket.destroy());
      socket.on('close', () => upstreamSocket.destroy());
      if (socket instanceof net.Socket) socket.setNoDelay(true);

      upstreamSocket.write(buildUpgradeHead(req, upstream));
      if (head.length > 0) upstreamSocket.write(head); // client bytes read past the parsed headers
      upstreamSocket.pipe(socket);
      socket.pipe(upstreamSocket);
    });
  };

  server.on('upgrade', (req: http.IncomingMessage, socket: Duplex, head: Buffer) => {
    // The single S6 decision. targetFor already enforces the readiness_ok machine-gate
    // and every fail-safe (missing row / unreadable store / break-glass) answers 'node'.
    if (flags.targetFor('S6') !== 'rust') {
      passThrough(req, socket, head);
      return;
    }
    proxyUpgrade(req, socket, head);
  });

  log.info(
    { upstream: upstream.host, wrappedListeners: originals.length },
    '[cutover] S6 WS router installed — NEW upgrades route by flag; established sockets stay put',
  );
}
