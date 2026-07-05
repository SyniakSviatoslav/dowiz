import crypto from 'node:crypto';

// Zero-dependency Prometheus text-format metrics (deliberate: no prom-client —
// the exposition format is trivial and the app needs ~10 series, not a framework).
//
// What this answers that logs cannot: is the operational pool saturating BEFORE
// it wedges (the "menu blinks empty" incident class), how deep is the pg-boss
// backlog, how many live WS connections, request rate/latency/error split.
//
// Scrape surface: GET /metrics, dark by default — 404 unless METRICS_TOKEN is set,
// and then only with a constant-time-matched bearer. Fly-internal scrapers pass
// the token; the endpoint never appears in public route maps.

const REQ_BUCKETS_S = [0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10];

interface RouteStats {
  buckets: number[]; // cumulative counts per REQ_BUCKETS_S slot
  sum: number;
  count: number;
  byStatus: Map<number, number>;
}

const routes = new Map<string, RouteStats>(); // "METHOD route-pattern" → stats
let wsMessagesOut = 0;

export function recordHttp(method: string, routePattern: string, status: number, seconds: number): void {
  const key = `${method} ${routePattern}`;
  let s = routes.get(key);
  if (!s) {
    s = { buckets: new Array(REQ_BUCKETS_S.length).fill(0), sum: 0, count: 0, byStatus: new Map() };
    routes.set(key, s);
  }
  s.sum += seconds;
  s.count += 1;
  s.byStatus.set(status, (s.byStatus.get(status) ?? 0) + 1);
  for (let i = 0; i < REQ_BUCKETS_S.length; i++) {
    if (seconds <= REQ_BUCKETS_S[i]!) s.buckets[i] = (s.buckets[i] ?? 0) + 1;
  }
}

export function recordWsMessageOut(): void {
  wsMessagesOut += 1;
}

/** Escape a label value per the Prometheus text format. */
function esc(v: string): string {
  return v.replace(/\\/g, '\\\\').replace(/"/g, '\\"').replace(/\n/g, '\\n');
}

export interface GaugeSource {
  /** name (must be a valid prom metric name) → current value; called per scrape. */
  [metric: string]: () => number | Promise<number>;
}

export async function renderMetrics(gauges: GaugeSource = {}): Promise<string> {
  const out: string[] = [];

  out.push('# TYPE http_requests_total counter');
  for (const [key, s] of routes) {
    const [method, route] = [key.slice(0, key.indexOf(' ')), key.slice(key.indexOf(' ') + 1)];
    for (const [status, n] of s.byStatus) {
      out.push(`http_requests_total{method="${esc(method!)}",route="${esc(route)}",status="${status}"} ${n}`);
    }
  }

  out.push('# TYPE http_request_duration_seconds histogram');
  for (const [key, s] of routes) {
    const [method, route] = [key.slice(0, key.indexOf(' ')), key.slice(key.indexOf(' ') + 1)];
    const labels = `method="${esc(method!)}",route="${esc(route)}"`;
    for (let i = 0; i < REQ_BUCKETS_S.length; i++) {
      out.push(`http_request_duration_seconds_bucket{${labels},le="${REQ_BUCKETS_S[i]}"} ${s.buckets[i]}`);
    }
    out.push(`http_request_duration_seconds_bucket{${labels},le="+Inf"} ${s.count}`);
    out.push(`http_request_duration_seconds_sum{${labels}} ${s.sum}`);
    out.push(`http_request_duration_seconds_count{${labels}} ${s.count}`);
  }

  out.push('# TYPE ws_messages_out_total counter');
  out.push(`ws_messages_out_total ${wsMessagesOut}`);

  for (const [name, fn] of Object.entries(gauges)) {
    try {
      const v = await fn();
      if (Number.isFinite(v)) {
        out.push(`# TYPE ${name} gauge`);
        out.push(`${name} ${v}`);
      }
    } catch {
      // a broken gauge must never break the scrape — the rest still exports
    }
  }

  const mem = process.memoryUsage();
  out.push('# TYPE process_resident_memory_bytes gauge');
  out.push(`process_resident_memory_bytes ${mem.rss}`);
  out.push('# TYPE process_heap_used_bytes gauge');
  out.push(`process_heap_used_bytes ${mem.heapUsed}`);
  out.push('# TYPE process_uptime_seconds gauge');
  out.push(`process_uptime_seconds ${Math.round(process.uptime())}`);

  return out.join('\n') + '\n';
}

/** test-only: reset counters between test cases */
export function resetMetrics(): void {
  routes.clear();
  wsMessagesOut = 0;
}

function tokenMatches(header: unknown, expected: string): boolean {
  if (typeof header !== 'string') return false;
  const provided = header.startsWith('Bearer ') ? header.slice(7) : header;
  const a = Buffer.from(provided);
  const b = Buffer.from(expected);
  if (a.length !== b.length) return false;
  return crypto.timingSafeEqual(a, b);
}

/**
 * Registers the onResponse recorder + the token-gated GET /metrics route.
 * `gauges` are read live per scrape (pool saturation, WS connections, queue depth).
 */
export function registerMetrics(fastify: any, gauges: GaugeSource = {}): void {
  fastify.addHook('onResponse', (request: any, reply: any, done: () => void) => {
    // routeOptions.url is the bounded route PATTERN (/api/orders/:id), never the
    // raw URL — keeps label cardinality finite and keeps IDs/tokens out of metrics.
    const route = request.routeOptions?.url ?? 'unmatched';
    if (route !== '/metrics') {
      recordHttp(request.method, route, reply.statusCode, (reply.elapsedTime ?? 0) / 1000);
    }
    done();
  });

  fastify.get('/metrics', async (request: any, reply: any) => {
    const token = process.env.METRICS_TOKEN;
    if (!token) return reply.status(404).send({ error: 'Not found' }); // dark by default
    if (!tokenMatches(request.headers.authorization, token)) {
      return reply.status(401).send({ error: 'Unauthorized' });
    }
    reply.header('content-type', 'text/plain; version=0.0.4; charset=utf-8');
    return renderMetrics(gauges);
  });
}
