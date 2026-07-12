#!/usr/bin/env node
// plane-telemetry — the plane's SINGLE telemetry egress choke-point (ADR-plane-telemetry-and-calibration).
// One schema owner, one redaction boundary, one Telegram sender, one durable-store writer.
// Every emitter (plane-report, plane-guard, the maintainer agent's loop steps, RemoteTrigger probes)
// calls this CLI; nothing else composes telemetry egress.
//
// Subcommands:
//   emit     --run-id R --kind K --outcome O [--target T] [--detail D] [--emitter E] [--step S]
//            [--note N] [--metrics JSON] [--refs JSON] [--seq N]
//            [--duration-ms N | --start-ts <epoch-ms|ISO>] [--severity info|warn|error]
//            [--host cloud|local] [--parent-run-id ID] [--step-index N]
//            → appends schema-v1 event to local scratch loops/runs/plane-events-YYYY-MM.jsonl
//              (field-scoped redaction incl. `step`, fail-closed → redactor_error stub on throw).
//              Richer fields are OPTIONAL + additive — emitted only when present, so a v1 reader
//              that ignores unknown keys is unaffected and old rows without them still parse.
//              severity/host are ENUMS (host never a secret); duration_ms/step_index numeric;
//              parent_run_id structural. ADVISORY tags only — telemetry is NEVER a gate.
//   predict  --run-id R --target T --prediction P --confidence 0..1 --method "primary:…|fallback:…"
//   resolve  --prediction-id ID --actual A --gap hit|miss|partial
//            (refuses out-of-order backdating: ts_predicted must be strictly earlier than the
//             run's first emitted event — M1 commit-reveal friction)
//   send     --run-id R  → ONE Telegram summary per run; skips CLEANLY but LOUDLY when env unset;
//            sendDocument current-run re-redacted slice on fail/overflow; chunk-at-3800 fallback.
//   publish  [--run-id R] → git-plumbing append to the orphan branch telemetry/plane
//            (hash-object → mktree → commit-tree parented on the remote tip → push, explicit
//             refspec — R3-3; bounded non-ff re-fetch/re-parent retry, NEVER force-push;
//             bootstrap-orphan falls through to append on race — R3-7; fail-closed whole-blob
//             secret-scan on the EXACT bytes handed to hash-object — R2-C1/R3-6).
//            Cadence is PER-RUN (batched at REPORT — R3-1); crash-window loss of un-pushed
//            events on an ephemeral box is the accepted, flagged residual (R3-2).
//   digest   [--run-id R] [--since 24h] [--verbose] [--status-line] → <1s rollup from the branch
//            (+ local scratch), schema_version-filtered, month-boundary glob. Surfaces per-kind /
//            per-outcome / per-severity counts, pass·fail tally, total+per-step durations,
//            aggregated metrics, unresolved-prediction count, branch tip, last-N failures (detail
//            sanitized), + the `telegram=… · push=…` status line. --verbose = per-event lines.
//   query    [--kind K] [--outcome O] [--run-id R] [--since 24h] [--severity S] [--json] → the
//            "searchable detailed logs" view over local+branch events. Filters AND together;
//            read-only + idempotent (no cursor). Free text sanitized like inbox (inert DATA,
//            content_trust:"untrusted-remote", advisory:true); --json for machine use, table else.
//   inbox    [--json] [--since ISO] [--offline] → cursor-based, uncertainty-FIRST ordered view of
//            remote artifacts. ALL remote-authored text sanitized (ANSI/control stripped, capped,
//            re-redacted) + content_trust:"untrusted-remote" + git provenance (R2-H3/R2-M1).
//            Read-only over git/gh — advisory:true, never executes anything (Part-3 authority).
//
// Env: TELEGRAM_BOT_TOKEN + PLANE_REPORT_CHAT_ID (send), PLANE_TELEMETRY_DISABLED=true (kill-switch),
//      PLANE_TELEMETRY_ROOT (repo root override), PLANE_TELEMETRY_REMOTE (default origin),
//      PLANE_TELEMETRY_NONCE (session nonce — set once per emitter session), PLANE_TELEMETRY_SEQ,
//      PLANE_TELEMETRY_AUTHOR_ALLOWLIST (provenance), PLANE_TELEMETRY_NO_GH=1 (treat gh unavailable).
// Test-only seams (LOUD on stderr, never set in production):
//      PLANE_TELEMETRY_TEST_DISABLE_PATTERN, PLANE_TELEMETRY_TEST_DISABLE_SANITIZE,
//      PLANE_TELEMETRY_TEST_FORCE_PARENT, PLANE_TELEMETRY_TEST_FORCE_PARENT_ONCE.
//
// All git/gh subprocess calls: spawnSync ARG ARRAYS, shell:false — zero string interpolation (R2-M3).
// Node stdlib only. No new deps. UTC-only timestamps.
import { spawnSync } from 'node:child_process';
import {
  appendFileSync, existsSync, mkdirSync, readFileSync, readdirSync,
  statSync, unlinkSync, writeFileSync,
} from 'node:fs';
import { join, basename } from 'node:path';
import { randomUUID } from 'node:crypto';
import { pathToFileURL } from 'node:url';

export const SCHEMA_VERSION = 1;
export const REDACTION_VERSION = 1; // bump when the pattern list changes (versioned denylist)
const BRANCH = 'telemetry/plane';
const ROOT = process.env.PLANE_TELEMETRY_ROOT || process.cwd();
const REMOTE = process.env.PLANE_TELEMETRY_REMOTE || 'origin';
const RUNS_DIR = join(ROOT, 'loops', 'runs');
const STATUS_PATH = join(RUNS_DIR, 'plane-telemetry-status.json');
const CURSOR_PATH = join(RUNS_DIR, 'inbox-cursor.json');
const LOCK_PATH = join(RUNS_DIR, 'inbox.lock');
const REMOTE_REF = `refs/remotes/${REMOTE}/${BRANCH}`;
const DETAIL_CAP = 280;
const PUSH_RETRIES = 3;

const KINDS = ['run', 'probe', 'sense', 'diagnose', 'heal', 'scout', 'report', 'escalation', 'fail', 'redactor_error'];
const OUTCOMES = ['pass', 'fail', 'fixed', 'deferred', 'escalated', 'skipped', 'natural_stop', 'error'];
const SEVERITIES = ['info', 'warn', 'error']; // advisory tag ONLY — never a gate
const HOSTS = ['cloud', 'local']; // enum ONLY — never a secret (from PLANE_TELEMETRY_HOST or --host)
const EMITTERS_HINT = 'plane-report|plane-guard|maintainer-agent|remote-probe|cli';
const REFS_KEYS = ['commit', 'pr', 'ledger', 'issue'];

// ---------------------------------------------------------------------------
// Redaction — Layer 2 of the two-layer egress defense (Layer 1 = allowlist schema).
// FIELD-SCOPED: applied to free-text fields ONLY (detail/note/target + prediction strings).
// Structural fields (run_id, ts, event_id, seq, tags, refs, kind, …) are NEVER scanned (H2).
// `pii:true` classes are write-time-only: excluded from the whole-blob push scan so a random
// all-digit UUID/timestamp can never false-abort a push (H2 lesson applied to the blob path).
// ---------------------------------------------------------------------------
const PATTERNS = [
  { name: 'fly_token', re: /FlyV1[\s_]\S+/g },
  { name: 'fly_macaroon', re: /\bfm[12]_[A-Za-z0-9+/=_-]{4,}/g },
  { name: 'fly_org_token', re: /\bfo1_[A-Za-z0-9_-]{4,}/g },
  { name: 'supabase_pat', re: /\bsbp_[A-Za-z0-9]{8,}/g },
  { name: 'supabase_secret', re: /\bsb_secret_[A-Za-z0-9_-]{4,}/g },
  { name: 'supabase_publishable', re: /\bsb_publishable_[A-Za-z0-9_-]{4,}/g },
  { name: 'jwt', re: /\beyJ[A-Za-z0-9_-]{5,}\.[A-Za-z0-9_-]{5,}\.[A-Za-z0-9_-]{5,}/g },
  { name: 'credentialed_url', re: /\b[a-z][a-z0-9+.-]*:\/\/[^\s/@]+:[^\s/@]+@\S*/gi },
  { name: 'dsn', re: /\b(?:postgres(?:ql)?|redis|mysql|amqp):\/\/\S+/gi },
  { name: 'telegram_bot_token', re: /\b\d{6,}:[A-Za-z0-9_-]{30,}\b/g },
  { name: 'aws_key', re: /\bAKIA[0-9A-Z]{16}\b/g },
  { name: 'github_token', re: /\b(?:ghp_[A-Za-z0-9]{36}|gho_[A-Za-z0-9]{36}|github_pat_[A-Za-z0-9_]{22,})\b/g },
  { name: 'openai_key', re: /\bsk-[A-Za-z0-9_-]{20,}\b/g },
  { name: 'slack_token', re: /\bxox[baprs]-[A-Za-z0-9-]{10,}/g },
  // The load-bearing rule (H1/R2-M4): case-INSENSITIVE secret-named KEY=VALUE / KEY: VALUE,
  // value captured to end-of-line (space-bearing values fully redacted). `\b[\w]*keyword`
  // (leading part optional) so bare lowercase `token=`/`password:` match too.
  {
    name: 'key_value',
    re: /\b([A-Za-z0-9_]*(?:secret|token|key|password|passwd|pwd|dsn|credential)[A-Za-z0-9_]*)[ \t]*[=:][ \t]*(.+)$/gim,
    replace: (_m, name) => `${name}=[REDACTED:key_value]`,
    valueGroup: 2,
  },
  { name: 'email', re: /[\w.+-]+@[\w-]+\.[\w.-]+/g, pii: true },
  // ≥9 digits, guarded against ISO dates / ids (no match directly after digit, T, :, ., -, +)
  { name: 'phone', re: /(?<![\dT:.+-])\+?\d(?:[ ()-]?\d){8,}/g, pii: true },
];

function activePatterns() {
  const disabled = (process.env.PLANE_TELEMETRY_TEST_DISABLE_PATTERN || '').split(',').filter(Boolean);
  if (disabled.length) {
    console.error(`[plane-telemetry] ⚠️ TEST HOOK ACTIVE — redaction pattern(s) DISABLED: ${disabled.join(',')} (never set in production)`);
    return PATTERNS.filter((p) => !disabled.includes(p.name));
  }
  return PATTERNS;
}

/** Redact free text. Returns { text, hits:[class,…] }. Throws only on non-coercible input. */
export function redactFreeText(input) {
  let text = String(input ?? '');
  const hits = [];
  for (const p of activePatterns()) {
    const re = new RegExp(p.re.source, p.re.flags);
    if (re.test(text)) {
      hits.push(p.name);
      const re2 = new RegExp(p.re.source, p.re.flags);
      text = text.replace(re2, p.replace ?? `[REDACTED:${p.name}]`);
    }
  }
  return { text, hits };
}

/**
 * Fail-closed secret-scan for the publish path (R2-C1/R3-6): scans the EXACT serialized bytes
 * that will be handed to `git hash-object` — the WHOLE blob, so older lines re-scan under the
 * CURRENT pattern list. Secret classes only (pii classes are field-scoped write-time concerns;
 * scanning them against structural JSON would false-fire on uuids/timestamps).
 * `[REDACTED:…]` placeholders are not hits. Returns [{class, sample}] — empty means clean.
 */
export function scanForSecrets(blob) {
  const text = String(blob ?? '');
  const findings = [];
  for (const p of activePatterns()) {
    if (p.pii) continue;
    const re = new RegExp(p.re.source, p.re.flags);
    let m;
    while ((m = re.exec(text)) !== null) {
      const value = p.valueGroup ? m[p.valueGroup] : m[0];
      if (typeof value === 'string' && value.trimStart().startsWith('[REDACTED:')) continue;
      findings.push({ class: p.name, sample: `[${p.name} match len=${String(value ?? m[0]).length}]` });
      if (re.lastIndex === m.index) re.lastIndex++;
    }
  }
  return findings;
}

/**
 * Sanitize remote-authored text before any terminal/--json surface (R2-H3): strip ANSI/OSC and
 * all control chars, flatten newlines, cap length, then re-redact (defense in depth). The result
 * is inert quoted DATA — JSON-escaping happens at serialization. Never instructions.
 */
export function sanitizeRemote(input, cap = 400) {
  if (process.env.PLANE_TELEMETRY_TEST_DISABLE_SANITIZE === '1') {
    console.error('[plane-telemetry] ⚠️ TEST HOOK ACTIVE — sanitizeRemote DISABLED (never set in production)');
    return String(input ?? '');
  }
  let s = String(input ?? '');
  // Control characters in these regexes are the POINT of this sanitizer (R2-H3): it must match
  // and strip raw ESC/BEL/C0 bytes from remote-authored text before any terminal surface.
  /* eslint-disable no-control-regex */
  s = s.replace(/\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)?/g, ''); // OSC sequences
  s = s.replace(/\x1b\[[0-9;?]*[ -/]*[@-~]/g, ''); // CSI sequences
  s = s.replace(/\x1b./g, ''); // any other escape
  s = s.replace(/[\r\n]+/g, ' ');
  s = s.replace(/[\x00-\x1f\x7f]/g, ' '); // remaining C0 controls + DEL
  /* eslint-enable no-control-regex */
  s = redactFreeText(s).text;
  if (s.length > cap) s = `${s.slice(0, cap)}…[capped]`;
  return s;
}

/** Read-side dedup: drops ONLY exact re-sends (same event_id). Distinct events always survive (R2-H2). */
export function dedupeEvents(events) {
  const seen = new Set();
  const out = [];
  for (const e of events) {
    const id = e?.event_id ?? JSON.stringify(e);
    if (seen.has(id)) continue;
    seen.add(id);
    out.push(e);
  }
  return out;
}

/** Stable sort/report order: (ts, run_id, seq) with nonce tiebreak (L2/R2-H2). */
export function sortEvents(events) {
  return [...events].sort((a, b) =>
    String(a.ts).localeCompare(String(b.ts)) ||
    String(a.run_id).localeCompare(String(b.run_id)) ||
    (a.seq ?? 0) - (b.seq ?? 0) ||
    String(a.nonce).localeCompare(String(b.nonce)));
}

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------
const nowIso = () => new Date().toISOString();
const monthOf = (iso) => iso.slice(0, 7);
const prevMonthOf = (iso) => {
  const d = new Date(`${iso.slice(0, 7)}-15T00:00:00Z`);
  d.setUTCMonth(d.getUTCMonth() - 1);
  return d.toISOString().slice(0, 7);
};
const eventsFileName = (month) => `plane-events-${month}.jsonl`;
export const deriveRunId = (iso = nowIso()) => `plane-${iso.slice(0, 16).replace(/:/g, '-')}-00Z`;

function ensureRunsDir() { mkdirSync(RUNS_DIR, { recursive: true }); }

/** Per-step timing (additive). --duration-ms wins; else derive from --start-ts (epoch-ms or ISO).
 *  Returns a non-negative integer ms, or undefined when absent/uncomputable (field then omitted). */
function computeDurationMs(args) {
  if (args['duration-ms'] !== undefined) {
    const d = Number(args['duration-ms']);
    return Number.isFinite(d) && d >= 0 ? Math.round(d) : undefined;
  }
  if (args['start-ts'] !== undefined) {
    const raw = String(args['start-ts']);
    const startMs = /^\d+$/.test(raw) ? Number(raw) : Date.parse(raw);
    if (Number.isFinite(startMs)) {
      const d = Date.now() - startMs;
      if (d >= 0 && d < 86400000 * 366) return Math.round(d); // guard clock-skew / absurd spans
    }
  }
  return undefined;
}

function appendJsonl(path, obj) {
  ensureRunsDir();
  appendFileSync(path, `${JSON.stringify(obj)}\n`);
}

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

function parseJsonlText(text) {
  const out = [];
  for (const line of String(text ?? '').split('\n')) {
    const t = line.trim();
    if (!t) continue;
    try { out.push(JSON.parse(t)); } catch { /* skip malformed */ }
  }
  return out;
}

function localEventFiles(iso = nowIso()) {
  return [monthOf(iso), prevMonthOf(iso)].map((m) => join(RUNS_DIR, eventsFileName(m)));
}

function readLocalEvents() {
  return localEventFiles().flatMap((p) => readJsonl(p));
}

function filterSchema(rows) {
  const ok = [];
  for (const r of rows) {
    if (r?.schema_version === SCHEMA_VERSION) ok.push(r);
    else console.error(`[plane-telemetry] warn: skipping row with unknown schema_version=${r?.schema_version}`);
  }
  return ok;
}

function readStatus() {
  try { return JSON.parse(readFileSync(STATUS_PATH, 'utf8')); } catch { return { schema_version: 1 }; }
}

function writeStatus(patch) {
  ensureRunsDir();
  const cur = readStatus();
  writeFileSync(STATUS_PATH, JSON.stringify({ ...cur, ...patch, schema_version: 1 }, null, 2));
}

function statusLine() {
  const s = readStatus();
  return `telegram=${s.telegram?.status ?? 'none'} · push=${s.push?.status ?? 'none'}`;
}

// All subprocess calls: arg arrays, shell:false — zero string interpolation (R2-M3).
function run(cmd, args, opts = {}) {
  const r = spawnSync(cmd, args, { cwd: ROOT, encoding: 'utf8', shell: false, ...opts });
  if (r.error) return { status: 127, stdout: '', stderr: String(r.error.message ?? r.error) };
  return { status: r.status ?? 1, stdout: r.stdout ?? '', stderr: r.stderr ?? '' };
}

const GIT_ID_ENV = {
  GIT_AUTHOR_NAME: process.env.GIT_AUTHOR_NAME || 'plane-telemetry',
  GIT_AUTHOR_EMAIL: process.env.GIT_AUTHOR_EMAIL || 'plane-telemetry@local',
  GIT_COMMITTER_NAME: process.env.GIT_COMMITTER_NAME || 'plane-telemetry',
  GIT_COMMITTER_EMAIL: process.env.GIT_COMMITTER_EMAIL || 'plane-telemetry@local',
};

function git(args, opts = {}) {
  return run('git', args, { env: { ...process.env, ...GIT_ID_ENV }, ...opts });
}

/** Fetch the telemetry branch with an EXPLICIT refspec — works on single-branch/shallow checkouts (R3-3). */
function fetchBranch() {
  return git(['fetch', REMOTE, `+refs/heads/${BRANCH}:${REMOTE_REF}`]).status === 0;
}

function remoteTip() {
  const r = git(['rev-parse', '--verify', '--quiet', REMOTE_REF]);
  return r.status === 0 ? r.stdout.trim() : null;
}

function showBranchFile(tip, name) {
  const r = git(['show', `${tip}:telemetry/${name}`]);
  return r.status === 0 ? r.stdout : null;
}

// ---------------------------------------------------------------------------
// Arg parsing
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
  console.error(`[plane-telemetry] error: ${msg}`);
  process.exitCode = code;
  return code;
}

// ---------------------------------------------------------------------------
// emit
// ---------------------------------------------------------------------------
function writeRedactorErrorStub(runId, reason) {
  // Fail-CLOSED (M5): payload dropped; only an ids-and-nothing-else stub survives.
  const stub = {
    schema_version: SCHEMA_VERSION,
    event_id: randomUUID(),
    run_id: runId || deriveRunId(),
    nonce: process.env.PLANE_TELEMETRY_NONCE || randomUUID(),
    seq: 0,
    ts: nowIso(),
    emitter: 'plane-telemetry',
    kind: 'redactor_error',
    step: 'REDACT',
    outcome: 'error',
    target: 'redactor',
    detail: `<none — redaction failed: ${String(reason).slice(0, 80).replace(/[^\x20-\x7e]/g, '?')}>`,
    tags: ['#plane', '#fail'],
  };
  appendJsonl(join(RUNS_DIR, eventsFileName(monthOf(stub.ts))), stub);
  return stub;
}

function cmdEmit(args) {
  const kind = String(args.kind || '');
  const outcome = String(args.outcome || '');
  if (!KINDS.includes(kind)) return fail(`--kind must be one of ${KINDS.join('|')}`);
  if (!OUTCOMES.includes(outcome)) return fail(`--outcome must be one of ${OUTCOMES.join('|')}`);
  const ts = nowIso();
  const runId = String(args['run-id'] || deriveRunId(ts));
  let detail; let note; let target; let step;
  try {
    detail = redactFreeText(String(args.detail ?? '')).text.slice(0, DETAIL_CAP);
    note = args.note !== undefined ? redactFreeText(String(args.note)).text.slice(0, DETAIL_CAP) : undefined;
    target = redactFreeText(String(args.target ?? '')).text.slice(0, 120);
    // `step` is caller-controlled free text → field-scoped redaction like detail/note/target.
    step = redactFreeText(String(args.step || kind.toUpperCase())).text.slice(0, 120);
  } catch (e) {
    writeRedactorErrorStub(runId, e.message);
    console.error('[plane-telemetry] redactor threw — payload DROPPED, redactor_error stub written (fail-closed)');
    return 0;
  }
  // Richer OPTIONAL fields (additive — schema stays v1; emitted only when present so a v1
  // reader that ignores unknown keys is unaffected, and old rows without them still parse).
  let severity;
  if (args.severity !== undefined) {
    severity = String(args.severity);
    if (!SEVERITIES.includes(severity)) return fail(`--severity must be one of ${SEVERITIES.join('|')} (advisory tag, not a gate)`);
  }
  const rawHost = args.host !== undefined ? String(args.host) : process.env.PLANE_TELEMETRY_HOST;
  let host;
  if (rawHost !== undefined && rawHost !== '') {
    if (HOSTS.includes(rawHost)) host = rawHost;
    else console.error(`[plane-telemetry] warn: ignoring host="${rawHost}" (must be ${HOSTS.join('|')}; never a secret)`);
  }
  const durationMs = computeDurationMs(args);
  const parentRunId = args['parent-run-id'] !== undefined ? String(args['parent-run-id']).slice(0, 120) : undefined; // structural id
  let stepIndex;
  if (args['step-index'] !== undefined) {
    const n = Number(args['step-index']);
    if (Number.isInteger(n) && n >= 0) stepIndex = n;
    else return fail('--step-index must be a non-negative integer');
  }
  const localEvents = readLocalEvents();
  const seq = args.seq !== undefined ? Number(args.seq)
    : process.env.PLANE_TELEMETRY_SEQ !== undefined ? Number(process.env.PLANE_TELEMETRY_SEQ)
      : localEvents.filter((e) => e.run_id === runId).length;
  let metrics; let refs;
  try { if (args.metrics) metrics = JSON.parse(String(args.metrics)); } catch { return fail('--metrics must be JSON'); }
  try {
    if (args.refs) {
      const raw = JSON.parse(String(args.refs));
      refs = {};
      for (const k of REFS_KEYS) if (raw[k] !== undefined) refs[k] = raw[k]; // scalar-id allowlist only
    }
  } catch { return fail('--refs must be JSON'); }
  const event = {
    schema_version: SCHEMA_VERSION,
    event_id: randomUUID(), // globally unique by construction (R2-H2)
    run_id: runId,
    nonce: process.env.PLANE_TELEMETRY_NONCE || randomUUID(), // per-process/session nonce
    seq,
    ts,
    emitter: String(args.emitter || 'cli'),
    kind,
    step,
    outcome,
    ...(severity !== undefined ? { severity } : {}),
    ...(host !== undefined ? { host } : {}),
    ...(durationMs !== undefined ? { duration_ms: durationMs } : {}),
    ...(parentRunId !== undefined ? { parent_run_id: parentRunId } : {}),
    ...(stepIndex !== undefined ? { step_index: stepIndex } : {}),
    target,
    detail,
    ...(note !== undefined ? { note } : {}),
    tags: ['#plane', `#${kind}`, `#${outcome}`],
    ...(metrics ? { metrics } : {}),
    ...(refs && Object.keys(refs).length ? { refs } : {}),
  };
  appendJsonl(join(RUNS_DIR, eventsFileName(monthOf(ts))), event);
  console.log(`event_id=${event.event_id}`);
  return 0;
}

// ---------------------------------------------------------------------------
// predict / resolve
// ---------------------------------------------------------------------------
const PREDICTIONS_PATH = () => join(RUNS_DIR, 'predictions.jsonl');

function latestPredictions(rows) {
  const byId = new Map();
  for (const p of rows) {
    const prev = byId.get(p.prediction_id);
    // last-write-wins by resolved/predict_seq, NOT skewable ts_actual (L2)
    if (!prev || (p.resolved && !prev.resolved) || (p.predict_seq ?? 0) >= (prev.predict_seq ?? 0)) byId.set(p.prediction_id, p);
  }
  return [...byId.values()];
}

function cmdPredict(args) {
  const ts = nowIso();
  const runId = String(args['run-id'] || deriveRunId(ts));
  const confidence = Number(args.confidence);
  if (!(confidence >= 0 && confidence <= 1)) return fail('--confidence must be in [0,1]');
  if (!args.target || !args.prediction) return fail('predict requires --target and --prediction');
  let target; let prediction; let method;
  try {
    target = redactFreeText(String(args.target)).text.slice(0, 120);
    prediction = redactFreeText(String(args.prediction)).text.slice(0, DETAIL_CAP);
    method = redactFreeText(String(args.method ?? '')).text.slice(0, DETAIL_CAP);
  } catch (e) {
    writeRedactorErrorStub(runId, e.message);
    console.error('[plane-telemetry] redactor threw — prediction DROPPED (fail-closed)');
    return 0;
  }
  const rows = readJsonl(PREDICTIONS_PATH());
  const predictSeq = new Set(rows.filter((p) => p.run_id === runId).map((p) => p.prediction_id)).size + 1;
  const record = {
    schema_version: SCHEMA_VERSION,
    prediction_id: randomUUID().replace(/-/g, '').slice(0, 12),
    run_id: runId,
    predict_seq: predictSeq,
    ts_predicted: ts,
    target,
    prediction,
    confidence,
    method,
    ts_actual: null,
    actual: null,
    gap: null,
    resolved: false,
  };
  appendJsonl(PREDICTIONS_PATH(), record);
  console.log(`prediction_id=${record.prediction_id}`);
  return 0;
}

function cmdResolve(args) {
  const id = String(args['prediction-id'] || '');
  const gap = String(args.gap || '');
  if (!id) return fail('resolve requires --prediction-id');
  if (!['hit', 'miss', 'partial'].includes(gap)) return fail('--gap must be hit|miss|partial');
  const rows = readJsonl(PREDICTIONS_PATH());
  const pred = latestPredictions(rows).find((p) => p.prediction_id === id);
  if (!pred) return fail(`prediction ${id} not found`, 1);
  // M1 commit-reveal ordering friction: the prediction must have been recorded STRICTLY EARLIER
  // than the run's first emitted outcome event — you cannot resolve a post-hoc "prediction".
  const runEvents = readLocalEvents().filter((e) => e.run_id === pred.run_id && e.kind !== 'redactor_error');
  if (runEvents.length) {
    const firstTs = runEvents.map((e) => e.ts).sort()[0];
    if (String(pred.ts_predicted) >= String(firstTs)) {
      return fail(`REFUSED out-of-order resolve: prediction ${id} ts_predicted=${pred.ts_predicted} is not strictly earlier than the run's first event ts=${firstTs} (M1 backdating friction)`, 1);
    }
  }
  let actual;
  try { actual = redactFreeText(String(args.actual ?? '')).text.slice(0, DETAIL_CAP); } catch (e) {
    writeRedactorErrorStub(pred.run_id, e.message);
    console.error('[plane-telemetry] redactor threw — resolve DROPPED (fail-closed)');
    return 0;
  }
  appendJsonl(PREDICTIONS_PATH(), { ...pred, ts_actual: nowIso(), actual, gap, resolved: true });
  console.log(`resolved=${id} gap=${gap}`);
  if (gap !== 'hit') console.error(`[plane-telemetry] gap=${gap} → write a WHY reflection to docs/reflections/INBOX/ (advisory — calibration, not a score)`);
  return 0;
}

// ---------------------------------------------------------------------------
// send (Telegram — best-effort, never throws, never non-zero)
// ---------------------------------------------------------------------------
function composeSummary(runId, events, predictions) {
  const runEvents = sortEvents(events.filter((e) => e.run_id === runId));
  const count = (fn) => runEvents.filter(fn).length;
  const fails = runEvents.filter((e) => e.outcome === 'fail' || e.outcome === 'error');
  const verdict = fails.length ? 'FAIL' : 'PASS';
  const preds = latestPredictions(predictions).filter((p) => p.run_id === runId);
  const hit = preds.filter((p) => p.gap === 'hit').length;
  const miss = preds.filter((p) => p.gap === 'miss').length;
  const topFail = fails[0];
  const lines = [
    `#plane #run ${verdict === 'PASS' ? '#pass' : '#fail'}  schema=${SCHEMA_VERSION}`,
    `run_id=${runId}`, // structural — verbatim, never through the redactor (H2 bridge)
    `verdict=${verdict}`,
    `events=${runEvents.length} healed=${count((e) => e.kind === 'heal' && (e.outcome === 'fixed' || e.outcome === 'pass'))} escalated=${count((e) => e.kind === 'escalation')} failed=${fails.length}`,
    `calib: predicted=${preds.length} hit=${hit} miss=${miss} (reliability, not a score)`,
    ...(topFail ? [`top_fail=${redactFreeText(`${topFail.target}: ${topFail.detail}`).text.slice(0, 200)}`] : []),
    statusLine(),
  ];
  return { text: lines.join('\n'), verdict, runEvents };
}

// Test-only seam (LOUD naming, matches the PLANE_TELEMETRY_TEST_* convention above): lets tests
// point at a local loopback stub instead of the real Telegram API, so send-path failure handling
// is provable without live network egress. Never set in production.
const TG_API_BASE = process.env.PLANE_TELEMETRY_TEST_TG_BASE_URL || 'https://api.telegram.org';

async function tgApi(token, method, body, isForm = false) {
  const res = await fetch(`${TG_API_BASE}/bot${token}/${method}`, {
    method: 'POST',
    ...(isForm ? { body } : { headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) }),
    signal: AbortSignal.timeout(8000),
  });
  return res.ok;
}

// Send outcomes must survive the ephemeral box: record them as EVENTS (published with the run),
// not only in the box-local status file. Gap found on the first cloud run (2026-07-02): the
// telegram skip was invisible from the durable record.
function recordSendOutcome(runId, statusStr) {
  const ts = nowIso();
  let detail;
  try { detail = redactFreeText(`telegram=${statusStr}`).text.slice(0, DETAIL_CAP); } catch { detail = 'telegram=[redactor_error]'; }
  const outcome = statusStr.startsWith('sent') ? 'pass' : statusStr.startsWith('skipped') ? 'skipped' : 'error';
  appendJsonl(join(RUNS_DIR, eventsFileName(monthOf(ts))), {
    schema_version: SCHEMA_VERSION, event_id: randomUUID(), run_id: runId,
    nonce: process.env.PLANE_TELEMETRY_NONCE || randomUUID(),
    seq: readLocalEvents().filter((e) => e.run_id === runId).length,
    ts, emitter: 'cli', kind: 'report', step: 'SEND', outcome,
    target: 'telegram', detail, tags: ['#plane', '#report', `#${outcome}`],
  });
}

async function cmdSend(args) {
  const runId = String(args['run-id'] || deriveRunId());
  const token = process.env.TELEGRAM_BOT_TOKEN;
  const chat = process.env.PLANE_REPORT_CHAT_ID;
  if (!token || !chat) {
    // Skip CLEANLY but LOUDLY (H3 — silence must never be mistaken for success).
    console.error('╔════════════════════════════════════════════════════════════════╗');
    console.error('║ [plane-telemetry] TELEGRAM SKIPPED — env unset                  ║');
    console.error(`║ missing: ${!token ? 'TELEGRAM_BOT_TOKEN ' : ''}${!chat ? 'PLANE_REPORT_CHAT_ID' : ''}`.padEnd(65) + '║');
    console.error('║ events are still written locally + publishable to the branch.  ║');
    console.error('╚════════════════════════════════════════════════════════════════╝');
    writeStatus({ telegram: { status: 'skipped:env_unset', ts: nowIso(), run_id: runId } });
    recordSendOutcome(runId, 'skipped:env_unset');
    return 0;
  }
  const events = filterSchema(dedupeEvents(readLocalEvents()));
  const predictions = readJsonl(PREDICTIONS_PATH());
  const { text, verdict, runEvents } = composeSummary(runId, events, predictions);
  try {
    let status = 'sent';
    const overflow = text.length > 3800 || runEvents.length > 40;
    const sentSummary = text.length <= 4096 ? await tgApi(token, 'sendMessage', { chat_id: chat, text, disable_web_page_preview: true }) : false;
    if (!sentSummary || verdict === 'FAIL' || overflow) {
      // sendDocument: ONLY the current-run slice, RE-REDACTED at attach time (M4).
      const slice = runEvents.map((e) => JSON.stringify({
        ...e,
        detail: redactFreeText(String(e.detail ?? '')).text,
        ...(e.note !== undefined ? { note: redactFreeText(String(e.note)).text } : {}),
      })).join('\n');
      const fd = new FormData();
      fd.append('chat_id', chat);
      fd.append('caption', `#plane run_id=${runId} — current-run slice (re-redacted at attach)`);
      fd.append('document', new Blob([slice], { type: 'application/json' }), `plane-run-${runId}.jsonl`);
      const docOk = await tgApi(token, 'sendDocument', fd, true).catch(() => false);
      if (!docOk && !sentSummary) {
        // chunk-at-3800 fallback only if the document send failed. H3: a chunk send can fail the
        // same way the summary/document did (network down, bad token/chat) — status must reflect
        // whether any chunk actually landed, not just that the loop ran (previously hardcoded
        // 'sent:chunked' even when every chunk failed, reporting false success — 2026-07-12).
        let anyChunkOk = false;
        for (let i = 0, n = Math.ceil(text.length / 3800); i < n; i++) {
          const ok = await tgApi(token, 'sendMessage', { chat_id: chat, text: `(${i + 1}/${n}) ${text.slice(i * 3800, (i + 1) * 3800)}`, disable_web_page_preview: true }).catch(() => false);
          if (ok) anyChunkOk = true;
        }
        status = anyChunkOk ? 'sent:chunked' : 'failed:unreachable';
      } else if (!docOk) status = 'sent:doc_failed';
    }
    writeStatus({ telegram: { status, ts: nowIso(), run_id: runId } });
    recordSendOutcome(runId, status);
    console.error(`[plane-telemetry] telegram: ${status}`);
  } catch (e) {
    const failStatus = `failed:${String(e.message).slice(0, 60)}`;
    writeStatus({ telegram: { status: failStatus, ts: nowIso(), run_id: runId } });
    recordSendOutcome(runId, failStatus);
    console.error(`[plane-telemetry] telegram failed (non-fatal): ${e.message}`);
  }
  return 0; // a dead Telegram can never fail a run
}

// ---------------------------------------------------------------------------
// publish — git plumbing → orphan branch telemetry/plane (R2-C1)
// ---------------------------------------------------------------------------
function collectPublishFiles() {
  const files = [];
  for (const p of localEventFiles()) if (existsSync(p)) files.push({ name: basename(p), local: readFileSync(p, 'utf8') });
  if (existsSync(PREDICTIONS_PATH())) files.push({ name: 'predictions.jsonl', local: readFileSync(PREDICTIONS_PATH(), 'utf8') });
  return files;
}

function lineKey(line) {
  try {
    const o = JSON.parse(line);
    if (o.event_id) return `e:${o.event_id}`;
  } catch { /* fall through to exact-line key */ }
  return `l:${line}`;
}

function mergeContent(existing, local) {
  const seen = new Set();
  const out = [];
  const add = (line) => {
    const t = line.trimEnd();
    if (!t.trim()) return;
    const k = lineKey(t);
    if (seen.has(k)) return;
    seen.add(k);
    out.push(t);
  };
  for (const l of String(existing ?? '').split('\n')) add(l);
  for (const l of String(local ?? '').split('\n')) add(l);
  return out.length ? `${out.join('\n')}\n` : '';
}

function cmdPublish(args) {
  const runId = String(args['run-id'] || '');
  const files = collectPublishFiles();
  if (!files.length) {
    console.error('[plane-telemetry] publish: nothing to publish (no local scratch)');
    writeStatus({ push: { status: 'ok:noop', ts: nowIso(), run_id: runId } });
    return 0;
  }
  fetchBranch(); // best-effort; absent branch → bootstrap
  const forceParent = process.env.PLANE_TELEMETRY_TEST_FORCE_PARENT;
  const forceParentOnce = process.env.PLANE_TELEMETRY_TEST_FORCE_PARENT_ONCE;
  if (forceParent || forceParentOnce) console.error('[plane-telemetry] ⚠️ TEST HOOK ACTIVE — forced parent (never set in production)');

  for (let attempt = 0; attempt <= PUSH_RETRIES; attempt++) {
    let tip = remoteTip();
    if (forceParent) tip = forceParent === 'orphan' ? null : forceParent;
    else if (forceParentOnce && attempt === 0) tip = forceParentOnce === 'orphan' ? null : forceParentOnce;

    // Build the exact blob contents (branch content ∪ unseen local lines — append-only union).
    const blobs = [];
    let newLines = 0;
    for (const f of files) {
      const existing = tip ? (showBranchFile(tip, f.name) ?? '') : '';
      const content = mergeContent(existing, f.local);
      if (content !== existing) newLines++;
      blobs.push({ name: f.name, content });
    }
    if (tip && newLines === 0) {
      console.error('[plane-telemetry] publish: branch already has every local line (noop)');
      writeStatus({ push: { status: 'ok:noop', ts: nowIso(), run_id: runId } });
      return 0;
    }

    // Fail-CLOSED whole-blob secret-scan on the EXACT bytes handed to hash-object (R2-C1/R3-6).
    for (const b of blobs) {
      const findings = scanForSecrets(b.content);
      if (findings.length) {
        writeRedactorErrorStub(runId, `push-scan hit in ${b.name}: ${findings.map((f) => f.class).join(',')}`);
        writeStatus({ push: { status: 'failed:secret_scan', ts: nowIso(), run_id: runId } });
        console.error(`[plane-telemetry] PUSH ABORTED (fail-closed): secret-scan hit in ${b.name} → ${findings.map((f) => f.class).join(', ')}`);
        return 1;
      }
    }

    // Plumbing: hash-object → mktree(telemetry/) → mktree(root) → commit-tree → push. No working
    // tree, no husky hook — the in-emitter scan above IS the guardrail (not --no-verify evasion).
    const entries = [];
    let plumbingFailed = false;
    for (const b of blobs) {
      const h = git(['hash-object', '-w', '--stdin'], { input: b.content }); // scan-bytes ≡ committed-bytes
      if (h.status !== 0) { plumbingFailed = true; break; }
      entries.push(`100644 blob ${h.stdout.trim()}\t${b.name}`);
    }
    let commitSha = null;
    if (!plumbingFailed) {
      const inner = git(['mktree'], { input: `${entries.join('\n')}\n` });
      const outer = inner.status === 0 ? git(['mktree'], { input: `040000 tree ${inner.stdout.trim()}\ttelemetry\n` }) : inner;
      if (outer.status === 0) {
        const msg = `plane-telemetry: publish${runId ? ` run_id=${runId}` : ''} (${files.length} file(s))`;
        const c = git(['commit-tree', outer.stdout.trim(), ...(tip ? ['-p', tip] : []), '-m', msg]);
        if (c.status === 0) commitSha = c.stdout.trim();
      }
    }
    if (!commitSha) {
      writeStatus({ push: { status: 'failed:plumbing', ts: nowIso(), run_id: runId } });
      console.error('[plane-telemetry] publish failed: git plumbing error');
      return 1;
    }

    const push = git(['push', REMOTE, `${commitSha}:refs/heads/${BRANCH}`]); // explicit refspec, NEVER force
    if (push.status === 0) {
      git(['update-ref', REMOTE_REF, commitSha]);
      writeStatus({ push: { status: 'ok', ts: nowIso(), run_id: runId, commit: commitSha.slice(0, 12) } });
      console.error(`[plane-telemetry] publish: pushed ${commitSha.slice(0, 12)} → ${REMOTE}/${BRANCH}`);
      return 0;
    }
    const nonFf = /non-fast-forward|fetch first|rejected|cannot lock ref|failed to push/i.test(push.stderr);
    if (!nonFf) {
      writeStatus({ push: { status: 'failed:push_error', ts: nowIso(), run_id: runId } });
      console.error(`[plane-telemetry] publish failed (push error): ${push.stderr.trim().slice(0, 200)}`);
      appendJsonl(join(RUNS_DIR, eventsFileName(monthOf(nowIso()))), failEvent(runId, 'push failed: transport/permission error'));
      return 1;
    }
    // Non-ff (concurrent writer / bootstrap race — R3-7): re-fetch, re-parent, retry. The
    // bootstrap path falls through into the append path here by construction.
    console.error(`[plane-telemetry] publish: non-fast-forward on attempt ${attempt + 1}/${PUSH_RETRIES + 1} — re-fetching + re-parenting`);
    fetchBranch();
  }
  // Exhaustion: keep the record in local scratch, flag LOUDLY, exit non-zero — never force-push.
  // (Ephemeral-box residual R3-2: accepted + flagged, visible in digest as push=failed:non_ff.)
  appendJsonl(join(RUNS_DIR, eventsFileName(monthOf(nowIso()))), failEvent(runId, `push failed: non_ff after ${PUSH_RETRIES + 1} attempts`));
  writeStatus({ push: { status: 'failed:non_ff', ts: nowIso(), run_id: runId } });
  console.error('[plane-telemetry] PUSH FAILED (non_ff, retries exhausted) — record kept in local scratch, flagged in digest');
  return 1;
}

function failEvent(runId, detail) {
  return {
    schema_version: SCHEMA_VERSION,
    event_id: randomUUID(),
    run_id: runId || deriveRunId(),
    nonce: process.env.PLANE_TELEMETRY_NONCE || randomUUID(),
    seq: 0,
    ts: nowIso(),
    emitter: 'plane-telemetry',
    kind: 'fail',
    step: 'PUBLISH',
    outcome: 'error',
    target: 'telemetry/plane push',
    detail,
    tags: ['#plane', '#fail'],
  };
}

// ---------------------------------------------------------------------------
// digest
// ---------------------------------------------------------------------------
function readBranchRows() {
  const tip = remoteTip();
  if (!tip) return { events: [], predictions: [], source: null };
  const iso = nowIso();
  const events = [];
  for (const m of [monthOf(iso), prevMonthOf(iso)]) { // month-boundary glob (L2)
    const text = showBranchFile(tip, eventsFileName(m));
    if (text) events.push(...parseJsonlText(text));
  }
  const predictions = parseJsonlText(showBranchFile(tip, 'predictions.jsonl') ?? '');
  return { events, predictions, source: 'branch' };
}

function parseSince(v) {
  if (!v || v === true) return null;
  const m = /^(\d+)([hd])$/.exec(String(v));
  if (m) return new Date(Date.now() - Number(m[1]) * (m[2] === 'h' ? 3600e3 : 86400e3)).toISOString();
  return String(v);
}

function cmdDigest(args) {
  if (args['status-line']) { console.log(statusLine()); return 0; }
  fetchBranch(); // best-effort; explicit refspec (R3-3)
  const branch = readBranchRows();
  const local = { events: readLocalEvents(), predictions: readJsonl(PREDICTIONS_PATH()) };
  const source = branch.source ? 'branch+local' : 'local';
  let events = filterSchema(dedupeEvents([...branch.events, ...local.events]));
  let predictions = latestPredictions([...branch.predictions, ...local.predictions].filter((p) => p?.schema_version === SCHEMA_VERSION));
  const since = parseSince(args.since);
  if (since) events = events.filter((e) => e.ts >= since);
  if (args['run-id']) {
    events = events.filter((e) => e.run_id === args['run-id']);
    predictions = predictions.filter((p) => p.run_id === args['run-id']);
  }
  events = sortEvents(events);
  const tip = remoteTip();
  const by = (key) => {
    const m = {};
    for (const e of events) { const v = e[key]; if (v === undefined || v === null) continue; m[v] = (m[v] ?? 0) + 1; }
    return Object.entries(m).map(([k, v]) => `${k}=${v}`).join(' ') || '(none)';
  };
  const fails = events.filter((e) => e.outcome === 'fail' || e.outcome === 'error');
  const passish = events.filter((e) => e.outcome === 'pass' || e.outcome === 'fixed').length;
  const resolved = predictions.filter((p) => p.resolved);
  const unresolved = predictions.filter((p) => !p.resolved).length;
  // per-step timing rollup (total + per-step; ignores untimed events)
  let totalDur = 0; let timed = 0; const stepDur = {};
  for (const e of events) {
    if (!Number.isFinite(e.duration_ms)) continue;
    totalDur += e.duration_ms; timed++;
    const k = e.step || '?';
    (stepDur[k] ??= { sum: 0, n: 0 });
    stepDur[k].sum += e.duration_ms; stepDur[k].n++;
  }
  const perStep = Object.entries(stepDur).map(([k, v]) => `${k}=${v.sum}ms(n=${v.n})`).join(' ') || '(none timed)';
  // metrics aggregation — numeric values summed by key across events (counts/tokens/cost surfaced)
  const metricAgg = {};
  for (const e of events) {
    if (!e.metrics || typeof e.metrics !== 'object') continue;
    for (const [k, v] of Object.entries(e.metrics)) if (typeof v === 'number' && Number.isFinite(v)) metricAgg[k] = (metricAgg[k] ?? 0) + v;
  }
  const metricsLine = Object.entries(metricAgg).map(([k, v]) => `${k}=${v}`).join(' ') || '(none)';
  // last-N failures with detail (sanitized — branch rows are remote-authored, treat as DATA)
  const lastFails = fails.slice(-5).map((e) => ` - [${e.run_id}] ${sanitizeRemote(String(e.target ?? ''), 80)}: ${sanitizeRemote(String(e.detail ?? ''), 160)}`);
  const out = [
    `# plane-telemetry digest (source=${source}, schema=${SCHEMA_VERSION})`,
    `events=${events.length} runs=${new Set(events.map((e) => e.run_id)).size} branch_tip=${tip ? tip.slice(0, 12) : 'none'}`,
    `tally: pass=${passish} fail=${fails.length} other=${events.length - passish - fails.length}`,
    `by kind: ${by('kind')}`,
    `by outcome: ${by('outcome')}`,
    `by severity: ${by('severity')}`,
    `durations: total=${totalDur}ms timed=${timed} | per-step: ${perStep}`,
    `metrics: ${metricsLine}`,
    `calib: predicted=${predictions.length} resolved=${resolved.length} unresolved=${unresolved} hit=${resolved.filter((p) => p.gap === 'hit').length} miss=${resolved.filter((p) => p.gap === 'miss').length} partial=${resolved.filter((p) => p.gap === 'partial').length} (reliability, not a score)`,
    lastFails.length ? `last fails (${fails.length}, showing ${lastFails.length}):\n${lastFails.join('\n')}` : 'last fails: (none)',
    statusLine(),
  ];
  if (args.verbose) {
    out.push('-- per-event (verbose, sanitized) --');
    for (const e of events) {
      const dur = Number.isFinite(e.duration_ms) ? ` ${e.duration_ms}ms` : '';
      const sev = e.severity ? ` [${e.severity}]` : '';
      out.push(` ${e.ts} ${e.run_id} ${e.kind}/${e.outcome}${sev}${dur} ${sanitizeRemote(String(e.step ?? ''), 40)}: ${sanitizeRemote(String(e.detail ?? ''), 160)}`);
    }
  }
  console.log(out.join('\n'));
  return 0;
}

// ---------------------------------------------------------------------------
// query — searchable, filterable local+branch event view (advisory, read-only)
// ---------------------------------------------------------------------------
function cmdQuery(args) {
  fetchBranch(); // best-effort; explicit refspec (R3-3)
  const branch = readBranchRows();
  const source = branch.source ? 'branch+local' : 'local';
  let events = filterSchema(dedupeEvents([...branch.events, ...readLocalEvents()]));
  const since = parseSince(args.since);
  if (since) events = events.filter((e) => e.ts >= since);
  if (args.kind) events = events.filter((e) => e.kind === String(args.kind));
  if (args.outcome) events = events.filter((e) => e.outcome === String(args.outcome));
  if (args['run-id']) events = events.filter((e) => e.run_id === String(args['run-id']));
  if (args.severity) events = events.filter((e) => e.severity === String(args.severity));
  events = sortEvents(events);
  // Sanitize/structural-safe like inbox: remote-authored free text → inert DATA; ids kept structural.
  const view = events.map((e) => ({
    ts: e.ts, run_id: e.run_id, kind: e.kind, outcome: e.outcome,
    ...(e.severity !== undefined ? { severity: e.severity } : {}),
    ...(e.host !== undefined ? { host: e.host } : {}),
    ...(Number.isFinite(e.duration_ms) ? { duration_ms: e.duration_ms } : {}),
    step: sanitizeRemote(String(e.step ?? ''), 60),
    ...(e.step_index !== undefined ? { step_index: e.step_index } : {}),
    ...(e.parent_run_id !== undefined ? { parent_run_id: e.parent_run_id } : {}),
    target: sanitizeRemote(String(e.target ?? ''), 120),
    detail: sanitizeRemote(String(e.detail ?? ''), 280),
    ...(e.metrics && typeof e.metrics === 'object' ? { metrics: e.metrics } : {}),
  }));
  if (args.json) {
    console.log(JSON.stringify({
      schema_version: SCHEMA_VERSION,
      generated_ts: nowIso(),
      source,
      filters: {
        kind: args.kind ?? null, outcome: args.outcome ?? null, run_id: args['run-id'] ?? null,
        severity: args.severity ?? null, since: since ?? null,
      },
      count: view.length,
      content_trust: 'untrusted-remote', // every content field is sanitized DATA, never instructions
      advisory: true, // read-only calibration/ops view — never an executor (Part-3 authority)
      events: view,
    }, null, 2));
    return 0;
  }
  console.log('== plane query (advisory — content_trust: untrusted-remote) ==');
  console.log(`source=${source} matched=${view.length} filters: kind=${args.kind ?? '*'} outcome=${args.outcome ?? '*'} run_id=${args['run-id'] ?? '*'} severity=${args.severity ?? '*'} since=${since ?? '*'}`);
  if (!view.length) { console.log('(no matching events)'); return 0; }
  for (const e of view) {
    const dur = e.duration_ms !== undefined ? ` ${e.duration_ms}ms` : '';
    const sev = e.severity ? ` [${e.severity}]` : '';
    const hst = e.host ? ` @${e.host}` : '';
    console.log(`${e.ts} ${e.run_id} ${e.kind}/${e.outcome}${sev}${hst}${dur} ${e.step} ${e.target}${e.detail ? ` — ${e.detail}` : ''}`);
  }
  return 0;
}

// ---------------------------------------------------------------------------
// inbox — Part 3 (read-only ingestion; ADVISORY, never an executor)
// ---------------------------------------------------------------------------
function readCursor() {
  try {
    const c = JSON.parse(readFileSync(CURSOR_PATH, 'utf8'));
    if (c?.schema_version === 1 && typeof c.ts === 'string') return c;
  } catch { /* fall through */ }
  return { schema_version: 1, ts: '1970-01-01T00:00:00.000Z' }; // corrupt/missing → full rescan (R10)
}

function branchProvenance() {
  const r = git(['log', '-1', '--format=%H%x1f%an', REMOTE_REF]);
  if (r.status !== 0) return { sha: null, author: 'unknown', branch: BRANCH, status: 'unexpected' };
  const [sha, author] = r.stdout.trim().split('\x1f');
  const allow = (process.env.PLANE_TELEMETRY_AUTHOR_ALLOWLIST || 'plane-telemetry,maintainer-bot,dowiz-maintainer,SyniakSviatoslav')
    .split(',').map((s) => s.trim()).filter(Boolean);
  return { sha: sha?.slice(0, 12) ?? null, author: sanitizeRemote(author ?? 'unknown', 60), branch: BRANCH, status: allow.includes(author) ? 'expected' : 'unexpected' };
}

function ghAvailable() {
  if (process.env.PLANE_TELEMETRY_NO_GH === '1') return false;
  return run('gh', ['auth', 'status']).status === 0;
}

function ghList(kind) {
  const r = run('gh', [kind, 'list', '--label', 'plane-guard', '--state', 'open',
    '--json', kind === 'pr' ? 'number,title,url,author' : 'number,title,url,author']);
  if (r.status !== 0) return null;
  try { return JSON.parse(r.stdout); } catch { return null; }
}

function cmdInbox(args) {
  ensureRunsDir();
  // advisory lockfile (R2-L1 — best-effort serialization; harmless if raced)
  let locked = false;
  try {
    if (existsSync(LOCK_PATH) && Date.now() - statSync(LOCK_PATH).mtimeMs < 60_000) {
      console.error('[plane-telemetry] inbox: another inbox appears to be running (advisory lock) — continuing read-only');
    } else { writeFileSync(LOCK_PATH, String(process.pid)); locked = true; }
  } catch { /* advisory only */ }
  try {
    const offline = Boolean(args.offline);
    const online = offline ? false : fetchBranch();
    if (!offline && !online) console.error('[plane-telemetry] inbox: fetch failed — degrading to git-only view of already-pulled objects');
    const branch = readBranchRows();
    const prov = branch.source ? branchProvenance() : null;
    const cursor = readCursor();
    const sinceTs = parseSince(args.since) ?? cursor.ts;

    const events = filterSchema(dedupeEvents(branch.events));
    const predictions = latestPredictions(branch.predictions.filter((p) => p?.schema_version === SCHEMA_VERSION));
    const items = [];
    const mk = (kind, extra) => items.push({ kind, ...extra, ...(prov ? { provenance: prov } : {}) });

    // Uncertainty-FIRST stable ordering contract (Counsel R2-c) — this array order IS the contract:
    // unresolved predictions → misses → reflections → hard fails → escalations → PRs → issues.
    for (const p of predictions.filter((p) => !p.resolved)) {
      mk('prediction_unresolved', {
        source: 'predictions', prediction_id: p.prediction_id, run_id: p.run_id,
        target: sanitizeRemote(p.target), prediction: sanitizeRemote(p.prediction), confidence: p.confidence,
      });
    }
    for (const p of predictions.filter((p) => p.resolved && p.gap !== 'hit' && String(p.ts_actual) > sinceTs)) {
      mk('prediction_miss', {
        source: 'predictions', prediction_id: p.prediction_id, run_id: p.run_id, gap: p.gap,
        target: sanitizeRemote(p.target), prediction: sanitizeRemote(p.prediction), actual: sanitizeRemote(String(p.actual ?? '')), confidence: p.confidence,
      });
    }
    const reflDir = join(ROOT, 'docs', 'reflections', 'INBOX');
    if (existsSync(reflDir)) {
      for (const f of readdirSync(reflDir).filter((f) => f.endsWith('.md')).sort()) {
        const p = join(reflDir, f);
        try {
          if (statSync(p).mtime.toISOString() <= sinceTs) continue;
          const excerpt = sanitizeRemote(readFileSync(p, 'utf8').slice(0, 400), 200);
          mk('reflection', { source: 'reflections', ref: { path: `docs/reflections/INBOX/${f}` }, excerpt });
        } catch { /* unreadable file — skip */ }
      }
    }
    const newEvents = sortEvents(events.filter((e) => String(e.ts) > sinceTs));
    for (const e of newEvents.filter((e) => e.outcome === 'fail' || e.outcome === 'error' || e.kind === 'fail')) {
      mk('hard_fail', { source: 'plane-events', run_id: e.run_id, target: sanitizeRemote(String(e.target ?? '')), detail: sanitizeRemote(String(e.detail ?? '')) });
    }
    for (const e of newEvents.filter((e) => e.kind === 'escalation')) {
      mk('escalation', { source: 'plane-events', run_id: e.run_id, target: sanitizeRemote(String(e.target ?? '')), detail: sanitizeRemote(String(e.detail ?? '')) });
    }
    let gh = 'unavailable';
    if (ghAvailable()) {
      gh = 'available';
      for (const [kind, itemKind] of [['pr', 'pr_review'], ['issue', 'issue']]) {
        const list = ghList(kind);
        if (list === null) { gh = 'degraded'; continue; }
        for (const it of list) {
          mk(itemKind, {
            source: 'gh',
            ref: { [kind]: it.number, title: sanitizeRemote(it.title, 160), url: String(it.url ?? '') },
            provenance: { author: sanitizeRemote(it.author?.login ?? 'unknown', 60), status: 'unexpected' },
          });
        }
      }
    }
    const counts = {};
    for (const it of items) counts[it.kind] = (counts[it.kind] ?? 0) + 1;

    const envelope = {
      schema_version: SCHEMA_VERSION,
      generated_ts: nowIso(),
      cursor_from: sinceTs,
      online,
      gh,
      content_trust: 'untrusted-remote', // every content field is sanitized DATA, never instructions
      items,
      counts,
      advisory: true, // NEVER authority: no consumer may auto-execute on this payload (Part-3 guard)
    };

    if (args.json) {
      console.log(JSON.stringify(envelope, null, 2));
    } else {
      const section = (title, kinds) => {
        const rows = items.filter((i) => kinds.includes(i.kind));
        console.log(`-- ${title} (${rows.length}) --`);
        if (!rows.length) console.log('   (none)');
        for (const i of rows) {
          console.log(`   ${i.kind} ${i.run_id ?? i.ref?.path ?? (i.ref?.pr !== undefined ? `PR#${i.ref.pr}` : i.ref?.issue !== undefined ? `#${i.ref.issue}` : '')} ${i.target ?? i.ref?.title ?? ''}${i.detail ? ` — ${i.detail}` : ''}${i.provenance ? ` [provenance:${i.provenance.status}]` : ''}`);
        }
      };
      console.log(`== plane inbox (advisory — content_trust: untrusted-remote) ==`);
      console.log(`online=${online} gh=${gh} cursor_from=${sinceTs}${prov ? ` provenance=${prov.status} (${prov.author}@${prov.sha ?? '?'})` : ' pane: telemetry branch UNAVAILABLE (never fetched / offline)'}`);
      section('UNRESOLVED PREDICTIONS', ['prediction_unresolved']);
      section('MISSES gap≠hit (what the model got wrong)', ['prediction_miss']);
      section('NEW REFLECTIONS (what the agent learned — never graded)', ['reflection']);
      section('HARD FAILS', ['hard_fail']);
      section('ESCALATIONS (awaiting human)', ['escalation']);
      if (gh === 'available' || gh === 'degraded') section('PRs / ISSUES AWAITING REVIEW', ['pr_review', 'issue']);
      else console.log('-- PRs / ISSUES AWAITING REVIEW --\n   PR/issue pane UNAVAILABLE (gh missing/unauthed)'); // R2-L2: explicit, never silently empty
      if (!items.length) console.log('-- ok/quiet — nothing new since cursor --');
    }

    // cursor: per-box optimization, never the record
    const newestTs = newEvents.length ? newEvents[newEvents.length - 1].ts : sinceTs;
    try {
      writeFileSync(CURSOR_PATH, JSON.stringify({
        schema_version: 1, ts: newestTs,
        last_event_id: newEvents.length ? newEvents[newEvents.length - 1].event_id : cursor.last_event_id ?? null,
      }, null, 2));
    } catch { /* cursor loss costs one rescan, never data */ }
    return 0;
  } finally {
    if (locked) { try { unlinkSync(LOCK_PATH); } catch { /* already gone */ } }
  }
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------
const USAGE = `plane-telemetry — single telemetry egress choke-point (schema v${SCHEMA_VERSION}, redaction v${REDACTION_VERSION})
usage: node scripts/plane-telemetry.mjs <emit|predict|resolve|send|publish|digest|query|inbox> [--flags]
       emitters: ${EMITTERS_HINT}; kinds: ${KINDS.join('|')}; outcomes: ${OUTCOMES.join('|')}
       emit richer (all OPTIONAL, additive): --duration-ms N | --start-ts <epoch-ms|ISO>,
         --severity ${SEVERITIES.join('|')} (advisory tag, NOT a gate), --host ${HOSTS.join('|')} (or PLANE_TELEMETRY_HOST),
         --parent-run-id ID --step-index N (nesting), --metrics JSON, --refs JSON
       digest [--run-id R] [--since 24h] [--verbose] [--status-line] → rollup (kinds/outcomes/severity,
         pass·fail tally, total+per-step durations, aggregated metrics, unresolved-prediction count,
         branch tip, last-N failures)
       query [--kind K] [--outcome O] [--run-id R] [--since 24h] [--severity S] [--json]
         → searchable local+branch events; sanitized DATA (content_trust: untrusted-remote), advisory
kill-switch: PLANE_TELEMETRY_DISABLED=true → no-op exit 0`;

async function main() {
  const [cmd, ...rest] = process.argv.slice(2);
  if (process.env.PLANE_TELEMETRY_DISABLED === 'true') {
    console.error('[plane-telemetry] disabled via PLANE_TELEMETRY_DISABLED — no-op');
    return 0;
  }
  const args = parseArgs(rest);
  switch (cmd) {
    case 'emit': return cmdEmit(args);
    case 'predict': return cmdPredict(args);
    case 'resolve': return cmdResolve(args);
    case 'send': return await cmdSend(args);
    case 'publish': return cmdPublish(args);
    case 'digest': return cmdDigest(args);
    case 'query': return cmdQuery(args);
    case 'inbox': return cmdInbox(args);
    case undefined:
    case 'help':
    case '--help':
      console.log(USAGE);
      return cmd === undefined ? 2 : 0;
    default:
      return fail(`unknown subcommand "${cmd}"\n${USAGE}`);
  }
}

const isMain = process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href;
if (isMain) {
  main().then((code) => { process.exitCode = code ?? process.exitCode ?? 0; })
    .catch((e) => { console.error(`[plane-telemetry] fatal: ${e.stack || e}`); process.exitCode = 1; });
}
