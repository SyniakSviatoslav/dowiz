#!/usr/bin/env node
// synthetic-probe — the EXTERNAL synthetic uptime monitor that closes the L1 P0
// (docs/design/bug-catching-net/plan.md). Fly's only health check hits /livez (event-loop
// only, green by design while Postgres is down); the rich /health returns 503 on pg-down but
// NOTHING outside Fly reads it — so a pg-down 503 (customers seeing 500s) pages *no one*.
// This probe polls /health from an external vantage and pages the existing Telegram rail on any
// non-healthy result, converting the loudest blind spot into an alert.
//
// A result is NON-HEALTHY (→ alert + non-zero exit) when ANY of:
//   • HTTP status != 200
//   • body.status not in {ok, healthy}   (so `degraded` pages too — leading indicator)
//   • any sub-check status === 'down'
//   • the request threw / timed out / body wasn't JSON
//
// Usage:
//   node scripts/synthetic-probe.mjs --once            single check; exit code = health (0 ok, 1 down)
//   node scripts/synthetic-probe.mjs --interval 60     loop forever, one check every 60s (scheduler)
//   node scripts/synthetic-probe.mjs --url https://…   override target (default prod /health)
//   node scripts/synthetic-probe.mjs --timeout 10000   per-request timeout ms (default 10000)
//
// Env (paging rail — reuses the EXACT plane-telemetry pattern): TELEGRAM_BOT_TOKEN + PLANE_REPORT_CHAT_ID.
// When unset the probe SKIPS the alert CLEANLY but LOUDLY (silence must never look like success) and
// still exits non-zero on a down result, so a scheduler/exit-code watcher still catches it.
// The health payload is public/recon-safe; the ONLY secret is TELEGRAM_BOT_TOKEN — it lives in the
// api.telegram.org URL and is NEVER logged. Best-effort plane-telemetry `probe` event via spawnSync
// when scripts/plane-telemetry.mjs exists.
//
// Node stdlib only. No new deps. UTC-only timestamps.
import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const DEFAULT_URL = 'https://dowiz.fly.dev/health';
const DEFAULT_TIMEOUT_MS = 10000;
const HEALTHY_BODY_STATUS = new Set(['ok', 'healthy']); // `degraded`/`unhealthy` are NOT healthy
const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url));
const TELEMETRY_SCRIPT = join(SCRIPT_DIR, 'plane-telemetry.mjs');

const nowIso = () => new Date().toISOString();

// ---------------------------------------------------------------------------
// classify — pure decision function (unit-testable without a network)
// Input: { httpStatus, body, error }. Output: { healthy, overall, reasons:[], downChecks:[] }.
// ---------------------------------------------------------------------------
export function classifyHealth({ httpStatus, body, error }) {
  const reasons = [];
  if (error) {
    return { healthy: false, overall: 'unreachable', reasons: [`request_error:${String(error).slice(0, 80)}`], downChecks: [] };
  }
  if (httpStatus !== 200) reasons.push(`http_${httpStatus}`);
  if (!body || typeof body !== 'object') {
    reasons.push('non_json_body');
    return { healthy: false, overall: 'unknown', reasons, downChecks: [] };
  }
  const overall = typeof body.status === 'string' ? body.status : 'unknown';
  if (!HEALTHY_BODY_STATUS.has(overall)) reasons.push(`status_${overall}`);
  const downChecks = [];
  const checks = body.checks && typeof body.checks === 'object' ? body.checks : {};
  for (const [name, c] of Object.entries(checks)) {
    if (c && typeof c === 'object' && c.status === 'down') downChecks.push(name);
  }
  if (downChecks.length) reasons.push(`sub_down:${downChecks.join(',')}`);
  return { healthy: reasons.length === 0, overall, reasons, downChecks };
}

// ---------------------------------------------------------------------------
// probeOnce — one HTTP check against the target. Never throws; failures become non-healthy results.
// ---------------------------------------------------------------------------
export async function probeOnce({ url = DEFAULT_URL, timeoutMs = DEFAULT_TIMEOUT_MS, fetchImpl = fetch } = {}) {
  const ts = nowIso();
  const started = Date.now();
  let httpStatus = 0; let body; let error;
  try {
    const res = await fetchImpl(url, {
      method: 'GET',
      headers: { accept: 'application/json' },
      signal: AbortSignal.timeout(timeoutMs),
    });
    httpStatus = res.status;
    try { body = await res.json(); } catch { body = undefined; }
  } catch (e) {
    error = e?.message || String(e);
  }
  const latencyMs = Date.now() - started;
  const verdict = classifyHealth({ httpStatus, body, error });
  return { ts, url, httpStatus, latencyMs, body, ...verdict };
}

// ---------------------------------------------------------------------------
// structured stdout — ONE line per check (no secret ever printed — token is not part of any field)
// ---------------------------------------------------------------------------
export function formatLine(r) {
  return [
    '[synthetic-probe]',
    `ts=${r.ts}`,
    `url=${r.url}`,
    `http=${r.httpStatus}`,
    `overall=${r.overall}`,
    `healthy=${r.healthy}`,
    `latency_ms=${r.latencyMs}`,
    r.downChecks.length ? `down=${r.downChecks.join(',')}` : 'down=none',
    `reasons=${r.reasons.length ? r.reasons.join('|') : 'none'}`,
  ].join(' ');
}

// ---------------------------------------------------------------------------
// Telegram alert — best-effort, never throws. Skips CLEANLY but LOUDLY when env unset
// (reuses the plane-telemetry cmdSend pattern). Returns { attempted, sent, skipped, reason }.
// ---------------------------------------------------------------------------
export async function sendTelegramAlert(r, { env = process.env, fetchImpl = fetch, timeoutMs = 8000 } = {}) {
  const token = env.TELEGRAM_BOT_TOKEN;
  const chat = env.PLANE_REPORT_CHAT_ID;
  if (!token || !chat) {
    // Silence must never be mistaken for success (H3) — skip loudly.
    console.error('╔════════════════════════════════════════════════════════════════╗');
    console.error('║ [synthetic-probe] TELEGRAM ALERT SKIPPED — env unset            ║');
    console.error(`║ missing: ${!token ? 'TELEGRAM_BOT_TOKEN ' : ''}${!chat ? 'PLANE_REPORT_CHAT_ID' : ''}`.padEnd(65) + '║');
    console.error('║ probe STILL exits non-zero on a down result — a watcher sees it.║');
    console.error('╚════════════════════════════════════════════════════════════════╝');
    return { attempted: false, sent: false, skipped: true, reason: 'env_unset' };
  }
  const text = [
    '🚨 #synthetic-probe #health DOWN',
    `url=${r.url}`,
    `http=${r.httpStatus} overall=${r.overall} latency_ms=${r.latencyMs}`,
    r.downChecks.length ? `sub-checks DOWN: ${r.downChecks.join(', ')}` : null,
    `reasons: ${r.reasons.join(' | ')}`,
    `ts=${r.ts}`,
  ].filter(Boolean).join('\n');
  try {
    // token lives ONLY in the URL path — never logged (H3 / redaction discipline).
    const res = await fetchImpl(`https://api.telegram.org/bot${token}/sendMessage`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ chat_id: chat, text, disable_web_page_preview: true }),
      signal: AbortSignal.timeout(timeoutMs),
    });
    const sent = !!(res && res.ok);
    console.error(`[synthetic-probe] telegram alert: ${sent ? 'sent' : `failed:http_${res && res.status}`}`);
    return { attempted: true, sent, skipped: false };
  } catch (e) {
    console.error(`[synthetic-probe] telegram alert failed (non-fatal): ${e?.message || e}`);
    return { attempted: true, sent: false, skipped: false, reason: 'exception' };
  }
}

// ---------------------------------------------------------------------------
// plane-telemetry — best-effort `probe` event so a check lands in the durable record (advisory).
// spawnSync arg array, shell:false. Absent script / any error → silent no-op (telemetry is never a gate).
// ---------------------------------------------------------------------------
export function emitTelemetry(r, { script = TELEMETRY_SCRIPT } = {}) {
  if (!existsSync(script)) return { emitted: false, reason: 'no_script' };
  try {
    const outcome = r.healthy ? 'pass' : 'fail';
    const detail = `overall=${r.overall} http=${r.httpStatus} latency_ms=${r.latencyMs} reasons=${r.reasons.join(',') || 'none'}`;
    const res = spawnSync(process.execPath, [
      script, 'emit',
      '--kind', 'probe',
      '--outcome', outcome,
      '--emitter', 'remote-probe',
      '--step', 'HEALTH',
      '--target', r.url,
      '--detail', detail,
      '--severity', r.healthy ? 'info' : 'error',
      '--metrics', JSON.stringify({ latency_ms: r.latencyMs, http: r.httpStatus }),
    ], { encoding: 'utf8', shell: false, timeout: 10000 });
    return { emitted: res.status === 0, reason: res.status === 0 ? undefined : `status_${res.status}` };
  } catch (e) {
    return { emitted: false, reason: `exception:${e?.message || e}` };
  }
}

// ---------------------------------------------------------------------------
// runProbe — one full check: probe → log → (alert on down) → telemetry. Returns the probe result.
// ---------------------------------------------------------------------------
export async function runProbe(opts = {}) {
  const {
    url, timeoutMs, fetchImpl, env,
    alertFn = sendTelegramAlert,
    telemetryFn = emitTelemetry,
  } = opts;
  const r = await probeOnce({ url, timeoutMs, fetchImpl });
  console.log(formatLine(r));
  if (!r.healthy) r.alert = await alertFn(r, { env, fetchImpl });
  r.telemetry = telemetryFn(r);
  return r;
}

const sleep = (ms) => new Promise((res) => setTimeout(res, ms));

// ---------------------------------------------------------------------------
// arg parsing (same shape as plane-telemetry)
// ---------------------------------------------------------------------------
function parseArgs(argv) {
  const args = { _: [] };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a.startsWith('--')) {
      const key = a.slice(2);
      const next = argv[i + 1];
      if (next === undefined || (next.startsWith('--') && next.length > 2)) args[key] = true;
      else { args[key] = next; i++; }
    } else args._.push(a);
  }
  return args;
}

const USAGE = `synthetic-probe — external /health synthetic monitor (closes the L1 P0)
usage: node scripts/synthetic-probe.mjs [--once | --interval <seconds>] [--url URL] [--timeout MS]
  --once             single check; exit code = health (0 healthy, 1 non-healthy)
  --interval <sec>   loop forever, one check every <sec> seconds (scheduler / long-running mode)
  --url URL          target (default ${DEFAULT_URL})
  --timeout MS       per-request timeout ms (default ${DEFAULT_TIMEOUT_MS})
env: TELEGRAM_BOT_TOKEN + PLANE_REPORT_CHAT_ID → paging on non-healthy (skips loudly if unset)`;

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help || args._[0] === 'help') { console.log(USAGE); return 0; }
  const url = typeof args.url === 'string' ? args.url : DEFAULT_URL;
  const timeoutMs = args.timeout ? Number(args.timeout) : DEFAULT_TIMEOUT_MS;

  const interval = args.interval !== undefined && args.interval !== true ? Number(args.interval) : undefined;
  if (interval !== undefined) {
    if (!Number.isFinite(interval) || interval <= 0) { console.error('[synthetic-probe] --interval must be a positive number of seconds'); return 2; }
    console.error(`[synthetic-probe] loop mode: probing ${url} every ${interval}s (Ctrl-C to stop)`);
    // eslint-disable-next-line no-constant-condition
    while (true) {
      try { await runProbe({ url, timeoutMs }); } catch (e) { console.error(`[synthetic-probe] probe cycle error (non-fatal): ${e?.message || e}`); }
      await sleep(interval * 1000);
    }
  }

  // default + --once: a single check, exit code carries the health verdict.
  const r = await runProbe({ url, timeoutMs });
  return r.healthy ? 0 : 1;
}

const isMain = process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href;
if (isMain) {
  main().then((code) => { process.exitCode = code ?? 0; })
    .catch((e) => { console.error(`[synthetic-probe] fatal: ${e?.stack || e}`); process.exitCode = 1; });
}
