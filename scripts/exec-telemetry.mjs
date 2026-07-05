#!/usr/bin/env node
// exec-telemetry — general harness-wide append-only exec-history emitter (SYSTEMS-MAP.md
// backlog item 3). Distinct from scripts/plane-telemetry.mjs (that one is plane-maintainer-only,
// with Telegram/orphan-branch publish). This one is a plain local log any layer of the harness
// (loop run, agent call, gate pass/fail, guardrail check, ...) can append an event to, read by
// scripts/telemetry-analyze.mjs for a bottleneck/pattern report. Advisory only — never a gate.
//
// Subcommands:
//   emit   --layer L --action-kind K --name N --outcome O --duration-ms N [--tokens N] [--meta JSON]
//          → appends one schema-v1 event to loops/runs/exec-events-YYYY-MM.jsonl
//   query  [--layer L] [--action-kind K] [--outcome O] [--since 24h] [--json]
//          → filtered read of the current+previous month's local log (advisory, read-only)
//
// Env: EXEC_TELEMETRY_ROOT (repo root override, for tests).
// Node stdlib only. No new deps. UTC-only timestamps.
import { appendFileSync, existsSync, mkdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

export const SCHEMA_VERSION = 1;
const ROOT = () => process.env.EXEC_TELEMETRY_ROOT || process.cwd();
const RUNS_DIR = (root = ROOT()) => join(root, 'loops', 'runs');
const NAME_CAP = 160;
const META_CAP = 500; // serialized-JSON char cap — meta is a small tag bag, not a payload store

export const ACTION_KINDS = ['loop-run', 'agent-call', 'gate-pass', 'gate-fail', 'guardrail-check', 'skill-run', 'council-review', 'other'];
export const OUTCOMES = ['pass', 'fail', 'error', 'skipped', 'deferred'];
const LAYER_RE = /^[a-z][a-z0-9-]{1,39}$/; // kebab-case, e.g. "loop-registry", "ssg", "skill-evolution"

const nowIso = () => new Date().toISOString();
const monthOf = (iso) => iso.slice(0, 7);
const prevMonthOf = (iso) => {
  const d = new Date(`${iso.slice(0, 7)}-15T00:00:00Z`);
  d.setUTCMonth(d.getUTCMonth() - 1);
  return d.toISOString().slice(0, 7);
};
const eventsFileName = (month) => `exec-events-${month}.jsonl`;
export const eventsPath = (month, root = ROOT()) => join(RUNS_DIR(root), eventsFileName(month));

function ensureRunsDir(root) { mkdirSync(RUNS_DIR(root), { recursive: true }); }

function readJsonl(path) {
  if (!existsSync(path)) return [];
  const out = [];
  for (const line of readFileSync(path, 'utf8').split('\n')) {
    const t = line.trim();
    if (!t) continue;
    try { out.push(JSON.parse(t)); } catch { /* crashed trailing line — skip (append-atomicity) */ }
  }
  return out;
}

/** Build + validate one event. Throws a descriptive Error on invalid input — caller decides fail-mode. */
export function buildEvent({ layer, actionKind, name, outcome, durationMs, tokens, meta }) {
  const layerStr = String(layer ?? '');
  if (!LAYER_RE.test(layerStr)) throw new Error(`layer must match ${LAYER_RE} (kebab-case, e.g. "loop-registry"), got ${JSON.stringify(layerStr)}`);
  if (!ACTION_KINDS.includes(actionKind)) throw new Error(`action_kind must be one of ${ACTION_KINDS.join('|')}`);
  if (!OUTCOMES.includes(outcome)) throw new Error(`outcome must be one of ${OUTCOMES.join('|')}`);
  const dur = Number(durationMs);
  if (!Number.isFinite(dur) || dur < 0) throw new Error('duration_ms must be a non-negative number');
  let tokensNum;
  if (tokens !== undefined && tokens !== null && tokens !== '') {
    tokensNum = Number(tokens);
    if (!Number.isFinite(tokensNum) || tokensNum < 0) throw new Error('tokens must be a non-negative number when provided');
  }
  let metaObj;
  if (meta !== undefined && meta !== null && meta !== '') {
    metaObj = typeof meta === 'string' ? JSON.parse(meta) : meta;
    if (typeof metaObj !== 'object' || Array.isArray(metaObj)) throw new Error('meta must be a JSON object');
    const serialized = JSON.stringify(metaObj);
    if (serialized.length > META_CAP) throw new Error(`meta serialized length ${serialized.length} exceeds cap ${META_CAP} — meta is a small tag bag, not a payload store`);
  }
  return {
    schema_version: SCHEMA_VERSION,
    ts: nowIso(),
    layer: layerStr,
    action_kind: actionKind,
    name: String(name ?? '').slice(0, NAME_CAP),
    duration_ms: Math.round(dur),
    ...(tokensNum !== undefined ? { tokens: tokensNum } : {}),
    outcome,
    ...(metaObj !== undefined ? { meta: metaObj } : {}),
  };
}

/** Append one event (already built by buildEvent) to the current month's log. */
export function appendEvent(event, root = ROOT()) {
  ensureRunsDir(root);
  appendFileSync(eventsPath(monthOf(event.ts), root), `${JSON.stringify(event)}\n`);
  return event;
}

/** Read current+previous month's local events (advisory, read-only). */
export function readEvents(root = ROOT()) {
  const iso = nowIso();
  const rows = [monthOf(iso), prevMonthOf(iso)].flatMap((m) => readJsonl(eventsPath(m, root)));
  return rows.filter((r) => r?.schema_version === SCHEMA_VERSION);
}

export function parseSince(v) {
  if (!v || v === true) return null;
  const m = /^(\d+)([hd])$/.exec(String(v));
  if (m) return new Date(Date.now() - Number(m[1]) * (m[2] === 'h' ? 3600e3 : 86400e3)).toISOString();
  return String(v);
}

// ---------------------------------------------------------------------------
// CLI
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

function fail(msg, code = 2) {
  console.error(`[exec-telemetry] error: ${msg}`);
  process.exitCode = code;
  return code;
}

function cmdEmit(args) {
  let event;
  try {
    event = buildEvent({
      layer: args.layer, actionKind: args['action-kind'], name: args.name, outcome: args.outcome,
      durationMs: args['duration-ms'], tokens: args.tokens, meta: args.meta,
    });
  } catch (e) { return fail(e.message); }
  appendEvent(event);
  console.log(`ts=${event.ts} layer=${event.layer} action_kind=${event.action_kind} outcome=${event.outcome}`);
  return 0;
}

function cmdQuery(args) {
  let events = readEvents();
  const since = parseSince(args.since);
  if (since) events = events.filter((e) => e.ts >= since);
  if (args.layer) events = events.filter((e) => e.layer === String(args.layer));
  if (args['action-kind']) events = events.filter((e) => e.action_kind === String(args['action-kind']));
  if (args.outcome) events = events.filter((e) => e.outcome === String(args.outcome));
  events = [...events].sort((a, b) => String(a.ts).localeCompare(String(b.ts)));
  if (args.json) {
    console.log(JSON.stringify({ schema_version: SCHEMA_VERSION, count: events.length, advisory: true, events }, null, 2));
    return 0;
  }
  console.log(`== exec-telemetry query (advisory) == matched=${events.length}`);
  for (const e of events) {
    console.log(`${e.ts} ${e.layer} ${e.action_kind}/${e.outcome} ${e.duration_ms}ms ${e.name}`);
  }
  return 0;
}

async function main() {
  const [sub, ...rest] = process.argv.slice(2);
  const args = parseArgs(rest);
  if (sub === 'emit') return cmdEmit(args);
  if (sub === 'query') return cmdQuery(args);
  console.error('[exec-telemetry] usage: exec-telemetry.mjs <emit|query> [...args]');
  process.exitCode = 2;
  return 2;
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main();
}
