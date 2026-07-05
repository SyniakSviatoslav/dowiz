/**
 * Cutover front-door (ADR-0022 §1/§3/§4) — Node stays the single public ingress; this
 * hook resolves each request's owning surface via the council-proven template matcher
 * (REV-C1/C2), consults the Postgres-backed flag store (REV-C3), and either falls
 * through to the existing Node handler (default — byte-identical behavior) or streams
 * the request to the internal-only Rust app over private networking.
 *
 * DECISION ORDER (every request):
 *   1. CUTOVER_FORCE_ALL_NODE break-glass → pass-through (no DB, no matcher).
 *   2. No upstream configured → pass-through (the dark deploy: harness fully inert).
 *   3. Surface match (WS upgrades never traverse fastify hooks — `ws` attaches its own
 *      'upgrade' listener on the raw server, so S6 routing lives at that listener when
 *      S6 cuts over, NOT here; the matcher's WS branch is kept for that future site).
 *   4. UNMAPPED / INFRA_NEVER_FLIPS / unmatched → Node (fail-closed; static assets and
 *      the SPA fallback are deliberately not in the 236-route map).
 *   5. Flag says node, or rust-without-readiness (read-time machine-gate) → Node.
 *   6. Upstream health-gated (REV-C5): unhealthy + non-money → serve Node NOW and fire
 *      the GLOBAL auto-degrade (one consensus UPDATE, debounced); unhealthy + money →
 *      truthful 503 (never silently reroute a money surface — human go/no-go).
 *   7. Forward: stream request → Rust, stream response → client.
 *
 * FORWARDING (REV-C6/C9/C11):
 *   - Real client IP travels in `x-dowiz-internal-client-ip` — set ONLY here, inbound
 *     copies stripped first (spoof-proof: Rust additionally accepts it only from the
 *     private-network source). XFF trust stays dead.
 *   - Per-surface timeout ≥ the surface's real server-side budget (Rust budget 30s →
 *     default 35s; S1 read-only 15s). A timeout on a bodied method answers with a
 *     TRUTHFUL retry-safe envelope ("may or may not have been applied"), never a bare
 *     failure that goads a duplicate submit.
 *   - Pre-response connection errors on bodyless methods (GET/HEAD/OPTIONS) fall
 *     through to Node (zero bytes consumed, zero bytes sent — safe); bodied methods
 *     never fall through (their body stream may be partially consumed).
 *
 * Zero new dependencies: `node:http`/`node:https` for the hop (the runtime's own client).
 */

import http from 'node:http';
import https from 'node:https';
import type { FastifyInstance, FastifyReply, FastifyRequest } from 'fastify';
import type { Pool } from 'pg';
import { buildErrorEnvelope } from '../api-error.js';
import { clientIp } from '../client-ip.js';
import { matchSurfaceForRequest, type SurfaceId } from './matcher.js';
import { ROUTE_TEMPLATES } from './route-templates.generated.js';
import { CutoverFlagsStore, NO_AUTO_DEGRADE } from './flags.js';

export interface CutoverFrontDoorOptions {
  pool: Pool;
  /** e.g. http://dowiz-rust-staging.flycast — unset/empty leaves the harness inert. */
  rustUpstream: string | undefined;
  /** 'true' = break-glass: every request is Node, flag store never consulted. */
  forceAllNode: boolean;
  flagsTtlMs: number;
  healthIntervalMs: number;
}

/** REV-C11: front-door budget must be ≥ the surface's real server-side budget. */
const SURFACE_TIMEOUT_MS: Readonly<Partial<Record<SurfaceId, number>>> = { S1: 15_000 };
const DEFAULT_TIMEOUT_MS = 35_000; // Rust server budget is 30s — stay strictly above it

const HOP_BY_HOP = new Set([
  'connection',
  'keep-alive',
  'proxy-authenticate',
  'proxy-authorization',
  'te',
  'trailer',
  'transfer-encoding',
  'upgrade',
]);

const HEALTH_TRIP_AFTER = 3; // consecutive failures before the breaker trips (hysteresis)
const HEALTH_RECOVER_AFTER = 2; // consecutive successes before forwarding resumes
const UNMAPPED_API_LOG_INTERVAL_MS = 10_000;

/**
 * Upstream health, sampled out-of-band so the request path only reads a boolean.
 * Direction invariant: a trip can trigger auto-degrade (toward Node); a recovery only
 * re-enables forwarding for surfaces still flagged rust — it NEVER re-flips a flag.
 */
export class UpstreamHealth {
  private consecutiveFails = 0;
  private consecutiveOks = 0;
  private tripped = false;
  private timer: NodeJS.Timeout | null = null;

  constructor(
    private readonly healthUrl: URL,
    private readonly intervalMs: number,
    private readonly onTrip: () => void,
    private readonly log: { warn: (o: object, m: string) => void },
  ) {}

  get healthy(): boolean {
    return !this.tripped;
  }

  start(): void {
    if (this.timer) return;
    this.timer = setInterval(() => this.probe(), this.intervalMs);
    this.timer.unref();
    this.probe();
  }

  stop(): void {
    if (this.timer) {
      clearInterval(this.timer);
      this.timer = null;
    }
  }

  /** Exposed for tests — one probe cycle. */
  probe(): void {
    const mod = this.healthUrl.protocol === 'https:' ? https : http;
    const req = mod.get(this.healthUrl, { timeout: 2_000 }, (res) => {
      res.resume(); // drain
      if (res.statusCode !== undefined && res.statusCode >= 200 && res.statusCode < 300) {
        this.recordOk();
      } else {
        this.recordFail(`status ${res.statusCode}`);
      }
    });
    req.on('timeout', () => req.destroy(new Error('health probe timeout')));
    req.on('error', (err) => this.recordFail(err.message));
  }

  private recordOk(): void {
    this.consecutiveFails = 0;
    if (this.tripped && ++this.consecutiveOks >= HEALTH_RECOVER_AFTER) {
      this.tripped = false;
      this.consecutiveOks = 0;
      this.log.warn({}, '[cutover] rust upstream health RECOVERED — forwarding re-enabled (flags unchanged)');
    }
  }

  private recordFail(reason: string): void {
    this.consecutiveOks = 0;
    if (!this.tripped && ++this.consecutiveFails >= HEALTH_TRIP_AFTER) {
      this.tripped = true;
      this.consecutiveFails = 0;
      this.log.warn({ reason }, '[cutover] rust upstream health TRIPPED — degrading');
      this.onTrip();
    }
  }
}

function surfaceTimeoutMs(surface: SurfaceId): number {
  return SURFACE_TIMEOUT_MS[surface] ?? DEFAULT_TIMEOUT_MS;
}

function isBodyless(method: string): boolean {
  return method === 'GET' || method === 'HEAD' || method === 'OPTIONS';
}

function retrySafeEnvelope(request: FastifyRequest, code: string): object {
  const message = isBodyless(request.method)
    ? 'Service temporarily unavailable — safe to retry.'
    : 'The upstream did not confirm this request. It may or may not have been applied — ' +
      'check the current state before retrying.';
  return buildErrorEnvelope(503, code, message, String(request.id), { retryAfterMs: 1_000 });
}

/**
 * Stream the request to the Rust upstream. Resolves 'done' when the response has been
 * fully relayed, 'fallthrough' when the caller should let Node handle the request.
 */
function forwardToRust(
  request: FastifyRequest,
  reply: FastifyReply,
  upstream: URL,
  surface: SurfaceId,
  log: { warn: (o: object, m: string) => void },
): Promise<'done' | 'fallthrough'> {
  return new Promise((resolve) => {
    const mod = upstream.protocol === 'https:' ? https : http;
    const headers: http.OutgoingHttpHeaders = {};
    for (const [name, value] of Object.entries(request.raw.headers)) {
      const lower = name.toLowerCase();
      if (HOP_BY_HOP.has(lower)) continue;
      if (lower.startsWith('x-dowiz-internal-')) continue; // spoof guard (REV-C6)
      if (lower === 'host') continue;
      if (value !== undefined) headers[lower] = value;
    }
    headers['host'] = upstream.host;
    headers['x-forwarded-host'] = request.hostname;
    headers['x-dowiz-internal-client-ip'] = clientIp(request); // REV-C6 trusted internal header
    headers['x-correlation-id'] = String(request.id);

    let settled = false;
    const settle = (outcome: 'done' | 'fallthrough') => {
      if (!settled) {
        settled = true;
        resolve(outcome);
      }
    };

    const bodyless = isBodyless(request.method);
    const timeoutMs = surfaceTimeoutMs(surface);

    const upstreamReq = mod.request(
      {
        protocol: upstream.protocol,
        hostname: upstream.hostname,
        port: upstream.port || (upstream.protocol === 'https:' ? 443 : 80),
        path: request.raw.url,
        method: request.method,
        headers,
      },
      (res) => {
        // Response is flowing — from here on, this request belongs to Rust.
        reply.hijack();
        const outHeaders: http.OutgoingHttpHeaders = {};
        for (const [name, value] of Object.entries(res.headers)) {
          if (HOP_BY_HOP.has(name.toLowerCase())) continue;
          if (value !== undefined) outHeaders[name] = value;
        }
        // Deterministic served-by oracle for parity E2E + ops (absence ⇒ Node).
        outHeaders['x-dowiz-cutover'] = `rust:${surface}`;
        reply.raw.writeHead(res.statusCode ?? 502, outHeaders);
        res.pipe(reply.raw);
        res.on('error', () => reply.raw.destroy());
        reply.raw.on('close', () => settle('done'));
        reply.raw.on('finish', () => settle('done'));
      },
    );

    const hardTimer = setTimeout(() => {
      upstreamReq.destroy(new Error(`cutover forward timeout after ${timeoutMs}ms`));
    }, timeoutMs);
    hardTimer.unref();

    upstreamReq.on('error', (err) => {
      clearTimeout(hardTimer);
      if (reply.raw.headersSent) {
        // Mid-stream failure — nothing honest left to send; cut the connection.
        reply.raw.destroy();
        settle('done');
        return;
      }
      log.warn(
        { surface, method: request.method, url: request.url, err: err.message },
        '[cutover] rust upstream error before response',
      );
      if (bodyless) {
        // Zero bytes consumed or sent — Node can serve this request as if the
        // forward never happened (per-request fail-safe; the health breaker
        // handles the systemic case).
        settle('fallthrough');
        return;
      }
      void reply
        .status(503)
        .header('retry-after', '1')
        .send(retrySafeEnvelope(request, 'CUTOVER_UPSTREAM_UNAVAILABLE'));
      settle('done');
    });

    upstreamReq.on('close', () => clearTimeout(hardTimer));

    if (bodyless) {
      upstreamReq.end();
    } else {
      request.raw.pipe(upstreamReq);
      request.raw.on('error', () => upstreamReq.destroy());
    }
    // Client gave up — stop the upstream work too.
    reply.raw.on('close', () => {
      if (!reply.raw.writableEnded) upstreamReq.destroy();
    });
  });
}

/**
 * Registers the front-door onRequest hook + background loops. Call AFTER the global
 * rate-limit plugin and the auth-prefix 401 gate are registered (ADR-0022: rate-limit
 * and cheap auth screening apply BEFORE any forward), and after the subdomain-rewrite
 * hook (the matcher and Rust must both see the REWRITTEN url).
 */
export function registerCutoverFrontDoor(
  fastify: FastifyInstance,
  opts: CutoverFrontDoorOptions,
): { flags: CutoverFlagsStore; health: UpstreamHealth | null } {
  const log = fastify.log;
  const flags = new CutoverFlagsStore(opts.pool, {
    ttlMs: opts.flagsTtlMs,
    log: {
      warn: (o, m) => log.warn(o, m),
      error: (o, m) => log.error(o, m),
      debug: (o, m) => log.debug(o, m),
    },
  });

  if (opts.forceAllNode || !opts.rustUpstream) {
    if (opts.forceAllNode) {
      log.warn({}, '[cutover] CUTOVER_FORCE_ALL_NODE break-glass active — harness bypassed, all traffic Node');
    }
    // Inert: no hook, no poll, no probes. The dark-deploy shape (and the break-glass
    // shape when the flag store itself is impaired — ADR-0022 §4).
    return { flags, health: null };
  }

  const upstream = new URL(opts.rustUpstream);
  const health = new UpstreamHealth(
    new URL('/healthz', upstream),
    opts.healthIntervalMs,
    () => {
      // Breaker tripped: degrade every rust-flagged, non-money surface (global action).
      for (const [surface, entry] of flags.snapshot()) {
        if (entry.target === 'rust' && !NO_AUTO_DEGRADE.has(surface)) {
          void flags.autoDegrade(surface as SurfaceId, 'upstream-health-breaker-tripped');
        }
      }
    },
    { warn: (o, m) => log.warn(o, m) },
  );

  flags.start();
  health.start();
  let lastUnmappedApiLogAt = 0;

  fastify.addHook('onRequest', async (request, reply) => {
    const match = matchSurfaceForRequest(request.method, request.raw.url ?? request.url, request.headers, ROUTE_TEMPLATES);
    if (!match.matched) {
      // Fail-closed to Node. Static assets/SPA fallback are unmapped BY DESIGN — only
      // an unmapped /api/* path is a census anomaly worth surfacing (sampled).
      const path = (request.raw.url ?? request.url).split('?')[0] ?? '';
      if (path.startsWith('/api/') && Date.now() - lastUnmappedApiLogAt > UNMAPPED_API_LOG_INTERVAL_MS) {
        lastUnmappedApiLogAt = Date.now();
        log.warn({ method: request.method, path, reason: match.reason }, '[cutover] unmapped /api route → Node');
      }
      return;
    }
    const surface = match.surface as SurfaceId;
    if (surface === 'UNMAPPED' || surface === 'INFRA_NEVER_FLIPS' || surface === 'S6') return;
    if (flags.targetFor(surface) !== 'rust') return;

    if (!health.healthy) {
      if (NO_AUTO_DEGRADE.has(surface)) {
        // Money/irreversible: never silently reroute (REV-C5) — truthful 503 (REV-C9).
        return reply
          .status(503)
          .header('retry-after', '1')
          .send(retrySafeEnvelope(request, 'CUTOVER_UPSTREAM_UNAVAILABLE'));
      }
      void flags.autoDegrade(surface, 'upstream-unhealthy-at-request');
      return; // serve from Node now — the global degrade converges all instances
    }

    const outcome = await forwardToRust(request, reply, upstream, surface, {
      warn: (o, m) => log.warn(o, m),
    });
    if (outcome === 'fallthrough') return;
    // 'done': reply hijacked or error response sent — fastify stops the lifecycle.
  });

  fastify.addHook('onClose', async () => {
    flags.stop();
    health.stop();
  });

  return { flags, health };
}
