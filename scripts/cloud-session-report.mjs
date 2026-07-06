#!/usr/bin/env node
// ─────────────────────────────────────────────────────────────────────────────
//  cloud-session-report.mjs — MANDATORY per-cloud-session Telegram report.
//
//  Every cloud routine run MUST call this as its final action (success, blocker,
//  OR error). It sends one summary to the ops Telegram channel:
//    run name · status · details · EXECUTION TIME · TOKEN USAGE.
//
//  Token usage + execution time are parsed from THIS session's own transcript
//  (~/.claude/projects/<slug>/<session>.jsonl) — the same `message.usage` fields
//  the context-budget-guard / audit-token-router already read:
//    output   = Σ output_tokens                (tokens this session generated)
//    new-in   = Σ (input_tokens + cache_creation_input_tokens)  (fresh input paid for)
//    cache    = Σ cache_read_input_tokens       (residency — the quadratic cost)
//    peak-ctx = max(input + cache_read + cache_creation)  (largest live prefix)
//  Execution time = last−first transcript timestamp (fallback: now − --started).
//
//  Env (reused from plane-telemetry): TELEGRAM_BOT_TOKEN + PLANE_REPORT_CHAT_ID.
//  Skips LOUDLY (non-zero) if unset so a misconfigured routine is visible, never
//  silently unreported. Outbound text is secret-scanned + redacted fail-safe.
//
//  Usage:
//    node scripts/cloud-session-report.mjs --run <name> --status <ok|blocked|error>
//         --details "<summary>" [--started <unixSeconds>] [--transcript <path>] [--dry-run]
//    node scripts/cloud-session-report.mjs --self-test
// ─────────────────────────────────────────────────────────────────────────────
import { readFileSync, readdirSync, existsSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { homedir } from 'node:os';

// ── args ──
function arg(name, def = null) {
  const i = process.argv.indexOf(`--${name}`);
  return i >= 0 && i + 1 < process.argv.length ? process.argv[i + 1] : def;
}
const has = (name) => process.argv.includes(`--${name}`);

// ── secret scan (fail-safe: REDACT before sending; a report must never leak) ──
const SECRET_RES = [
  /\b\d{6,}:[A-Za-z0-9_-]{30,}\b/g, // telegram bot token
  /\bxox[baprs]-[A-Za-z0-9-]{10,}\b/g, // slack
  /\b(?:sk|pk|rk)-[A-Za-z0-9]{20,}\b/g, // openai/stripe-ish
  /\bAKIA[0-9A-Z]{16}\b/g, // aws access key id
  /\bgh[pousr]_[A-Za-z0-9]{30,}\b/g, // github token
  /-----BEGIN [A-Z ]*PRIVATE KEY-----/g, // pem
  /\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\b/g, // jwt
  /(postgres(?:ql)?|mysql|redis|amqp):\/\/[^\s]+/g, // db/broker URLs w/ creds
];
export function redactSecrets(text) {
  let out = String(text);
  let hit = 0;
  for (const re of SECRET_RES) out = out.replace(re, () => { hit++; return '[REDACTED]'; });
  return { text: out, redactions: hit };
}

// ── token/time parse from a transcript JSONL ──
export function parseTranscript(text) {
  let output = 0, newIn = 0, cache = 0, peak = 0, turns = 0;
  let firstTs = null, lastTs = null;
  for (const line of text.split('\n')) {
    if (!line || (line.indexOf('"usage"') < 0 && line.indexOf('"timestamp"') < 0)) continue;
    let obj;
    try { obj = JSON.parse(line); } catch { continue; }
    const ts = obj.timestamp;
    if (ts) {
      const t = Date.parse(ts);
      if (!Number.isNaN(t)) {
        if (firstTs === null || t < firstTs) firstTs = t;
        if (lastTs === null || t > lastTs) lastTs = t;
      }
    }
    const u = obj.message && obj.message.usage;
    if (u) {
      turns++;
      output += u.output_tokens || 0;
      newIn += (u.input_tokens || 0) + (u.cache_creation_input_tokens || 0);
      cache += u.cache_read_input_tokens || 0;
      const ctx = (u.input_tokens || 0) + (u.cache_read_input_tokens || 0) + (u.cache_creation_input_tokens || 0);
      if (ctx > peak) peak = ctx;
    }
  }
  return { output, newIn, cache, peak, turns, firstTs, lastTs };
}

function newestTranscript() {
  const base = join(homedir(), '.claude', 'projects');
  if (!existsSync(base)) return null;
  let best = null, bestMtime = -1;
  for (const proj of readdirSync(base)) {
    const dir = join(base, proj);
    let st;
    try { st = statSync(dir); } catch { continue; }
    if (!st.isDirectory()) continue;
    for (const f of readdirSync(dir)) {
      if (!f.endsWith('.jsonl')) continue;
      const p = join(dir, f);
      const m = statSync(p).mtimeMs;
      if (m > bestMtime) { bestMtime = m; best = p; }
    }
  }
  return best;
}

// ── formatting ──
export function fmtTokens(n) {
  if (n == null) return 'n/a';
  if (n >= 1e9) return (n / 1e9).toFixed(2) + 'B';
  if (n >= 1e6) return (n / 1e6).toFixed(1) + 'M';
  if (n >= 1e3) return (n / 1e3).toFixed(1) + 'K';
  return String(n);
}
export function fmtDuration(ms) {
  if (ms == null || ms < 0) return 'n/a';
  const s = Math.round(ms / 1000);
  const h = Math.floor(s / 3600), m = Math.floor((s % 3600) / 60), sec = s % 60;
  return (h ? `${h}h ` : '') + (h || m ? `${m}m ` : '') + `${sec}s`;
}

const STATUS_ICON = { ok: '✅', green: '✅', done: '✅', blocked: '⛔', blocker: '⛔', error: '❌', fail: '❌', noop: '➖', skipped: '➖' };

export function buildMessage({ run, status, details, durMs, tok, transcriptFound }) {
  const icon = STATUS_ICON[String(status).toLowerCase()] || 'ℹ️';
  const tokLine = transcriptFound
    ? `🎟 out ${fmtTokens(tok.output)} · new-in ${fmtTokens(tok.newIn)} · cache ${fmtTokens(tok.cache)} · peak ${fmtTokens(tok.peak)} · ${tok.turns} turns`
    : '🎟 token usage: n/a (transcript not found)';
  const { text: safeDetails, redactions } = redactSecrets(details || '(no details provided)');
  const head = `🤖 Cloud session · ${run}\n${icon} ${status}\n⏱ ${fmtDuration(durMs)}   ·   ${tokLine}`;
  const body = safeDetails.length > 1200 ? safeDetails.slice(0, 1200) + '…' : safeDetails;
  const foot = redactions ? `\n⚠ ${redactions} secret-like value(s) redacted from this report.` : '';
  return `${head}\n—\n${body}${foot}`;
}

async function tgSend(text) {
  const token = process.env.TELEGRAM_BOT_TOKEN;
  const chat = process.env.PLANE_REPORT_CHAT_ID;
  if (!token || !chat) {
    console.error('✗ cloud-session-report: TELEGRAM SKIPPED — env unset (' +
      `${!token ? 'TELEGRAM_BOT_TOKEN ' : ''}${!chat ? 'PLANE_REPORT_CHAT_ID' : ''}`.trim() + ').');
    console.error('  The report was NOT sent. This is a loud skip, not a silent one — set the env in the routine.');
    return false;
  }
  const res = await fetch(`https://api.telegram.org/bot${token}/sendMessage`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ chat_id: chat, text, disable_web_page_preview: true }),
  });
  if (!res.ok) {
    console.error(`✗ cloud-session-report: Telegram API ${res.status} — ${(await res.text()).slice(0, 200)}`);
    return false;
  }
  return true;
}

// ── self-test (hermetic — no network, no fs) ──
function selfTest() {
  let fail = 0;
  const ok = (n, c) => { console.log(`  ${c ? '✓' : '✗'} ${n}`); if (!c) fail++; };
  console.log('cloud-session-report self-test:');

  const fixture = [
    JSON.stringify({ timestamp: '2026-07-06T18:00:00.000Z', message: { usage: { input_tokens: 100, cache_read_input_tokens: 5000, cache_creation_input_tokens: 200, output_tokens: 300 } } }),
    JSON.stringify({ timestamp: '2026-07-06T18:05:00.000Z', message: { usage: { input_tokens: 50, cache_read_input_tokens: 9000, cache_creation_input_tokens: 0, output_tokens: 700 } } }),
    'garbage-not-json',
    JSON.stringify({ timestamp: '2026-07-06T18:34:12.000Z', message: { usage: { input_tokens: 10, cache_read_input_tokens: 12000, cache_creation_input_tokens: 0, output_tokens: 128 } } }),
  ].join('\n');
  const t = parseTranscript(fixture);
  ok('sums output tokens (300+700+128=1128)', t.output === 1128);
  ok('sums new-input (input+cache_creation)', t.newIn === 100 + 200 + 50 + 0 + 10 + 0);
  ok('sums cache-read (5000+9000+12000)', t.cache === 26000);
  ok('peak = max ctx (10+12000)', t.peak === 12010);
  ok('counts 3 usage turns (garbage skipped)', t.turns === 3);
  ok('duration first→last = 34m12s', fmtDuration(t.lastTs - t.firstTs) === '34m 12s');

  ok('fmtTokens B/M/K', fmtTokens(3.3e9) === '3.30B' && fmtTokens(2.1e6) === '2.1M' && fmtTokens(4530) === '4.5K' && fmtTokens(42) === '42');
  ok('fmtDuration h/m/s', fmtDuration(3661000) === '1h 1m 1s' && fmtDuration(5000) === '5s');

  const planted = 'ran ok, token was 123456789:ABCdefGHIjklMNOpqrstUVwxyz0123456789';
  const red = redactSecrets(planted);
  ok('secret-scan redacts a telegram token', red.redactions === 1 && !red.text.includes('ABCdefGHIjkl'));

  const msg = buildMessage({ run: 'x', status: 'ok', details: planted, durMs: t.lastTs - t.firstTs, tok: t, transcriptFound: true });
  ok('message carries status/time/tokens and redacts secrets', /✅ ok/.test(msg) && /34m 12s/.test(msg) && /out 1\.1K/.test(msg) && !msg.includes('ABCdefGHIjkl') && /redacted/.test(msg));
  ok('message handles missing transcript', /n\/a/.test(buildMessage({ run: 'x', status: 'error', details: 'crashed', durMs: null, tok: {}, transcriptFound: false })));

  if (fail) { console.error(`✗ cloud-session-report self-test: ${fail} failed.`); process.exit(1); }
  console.log('✓ cloud-session-report self-test: token/time parse + format + secret-scan + message all pass.');
}

// ── main ──
if (has('self-test')) {
  selfTest();
} else {
  const run = arg('run', 'unknown-routine');
  const status = arg('status', 'unknown');
  const details = arg('details', '');
  const started = arg('started');
  const tPath = arg('transcript') || newestTranscript();
  let tok = { output: 0, newIn: 0, cache: 0, peak: 0, turns: 0, firstTs: null, lastTs: null };
  let transcriptFound = false;
  if (tPath && existsSync(tPath)) {
    try { tok = parseTranscript(readFileSync(tPath, 'utf8')); transcriptFound = tok.turns > 0; } catch { /* degrade */ }
  }
  let durMs = null;
  if (transcriptFound && tok.firstTs != null && tok.lastTs != null) durMs = tok.lastTs - tok.firstTs;
  else if (started && !Number.isNaN(Number(started))) durMs = Date.now() - Number(started) * 1000;

  const message = buildMessage({ run, status, details, durMs, tok, transcriptFound });
  console.log('── report ──\n' + message + '\n────────────');
  if (has('dry-run')) { console.log('(--dry-run: not sent)'); process.exit(0); }
  const sent = await tgSend(message);
  process.exit(sent ? 0 : 1);
}
