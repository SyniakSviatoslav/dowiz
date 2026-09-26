// SUITE nightly · ORDER: what one delivered order costs, from e2e/evals/traffic.mjs's JSON.
//
// THIS COLLECTOR NEVER PLACES AN ORDER. traffic.mjs places a real order and walks it to
// DELIVERED, and it has no clean-up path: a walk that stops half way (the courier cannot sign
// in, a button moved) leaves an order past PENDING that nothing can end
// (dowiz-order-fsm-has-no-exit). So the order suite reads a traffic JSON that a person produced
// on purpose — `EVALS_TRAFFIC_JSON=/path/traffic-….json` — and otherwise says UNVERIFIED.
import fs from 'node:fs';
import { ind, unverified } from '../rules.mjs';

export const WHY_UNRUN = 'traffic.mjs places a real order and has no clean-up for a walk that stops half way; '
  + 'run it by hand against sushi-durres and pass EVALS_TRAFFIC_JSON';

export const UNITS = { api_requests: 'requests', api_bytes: 'bytes', p95_ms: 'ms', cells_per_order: 'cells' };

/** The four budgets and the first paint, as indicators. */
export function fromReport(r, file) {
  const src = `e2e/evals/traffic.mjs report ${file} (order ${String(r.at).slice(0, 10)})`;
  return [
    ind('order.api_requests', r.requests?.api ?? null, 'requests', 'ratchet', src, { note: JSON.stringify(r.requests?.byWho || {}) }),
    ind('order.requests_total', r.requests?.total ?? null, 'requests', 'ratchet', src),
    ind('order.api_bytes', r.bytes?.api ?? null, 'bytes', 'ratchet', src),
    ind('order.p50_ms', r.latency?.p50 ?? null, 'ms', 'plus25', src),
    ind('order.p95_ms', r.latency?.p95 ?? null, 'ms', 'plus25', src),
    ind('order.cells_per_order', r.image?.cellsPerOrder ?? null, 'cells', 'ratchet', src),
    ind('order.store_fcp_ms', r.firstPaint?.fcp ?? null, 'ms', 'plus25', src),
    ind('order.placed_to_delivered_ms', r.lifecycleMs?.delivered ?? null, 'ms', 'trend', src),
  ];
}

export async function collect(ctx) {
  const file = ctx.env?.EVALS_TRAFFIC_JSON;
  if (!file || !fs.existsSync(file)) {
    const why = file ? `EVALS_TRAFFIC_JSON=${file} does not exist` : WHY_UNRUN;
    return Object.entries(UNITS).map(([k, unit]) =>
      unverified(`order.${k}`, unit, unit === 'ms' ? 'plus25' : 'ratchet', 'e2e/evals/traffic.mjs', why));
  }
  return fromReport(JSON.parse(fs.readFileSync(file, 'utf8')), file);
}
