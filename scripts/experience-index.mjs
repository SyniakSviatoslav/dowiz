#!/usr/bin/env node
// experience-index — the reinforced-swarm read-back path (Stage 1, report-only).
// Charter: docs/design/reinforced-swarm/plan.md §3, §5 (Stage 1), §7.
//
// It reads the EXISTING outcome stores (loops/runs/metrics.jsonl + predictions.jsonl,
// and the failure corpus under docs/) and turns them into an ADVISORY next-choice
// recommendation grouped by task-signature. It is the missing "run → outcome →
// weight-the-next-choice" arc, in its minimal honest form.
//
// HARD constraints (load-bearing — see plan §4):
//   • REPORT-ONLY. Writes NOTHING — not gate state, not predictions.jsonl, not
//     routing.jsonl. Pure read + print. Always exits 0.
//   • The router does NOT consume this yet (that is Stage 2). Every recommendation
//     is stamped advisory:true.
//   • The reward/ranking derives ONLY from DETERMINISTIC outcome fields
//     (outcome, fake_green_caught, fail_start→fail_end, per_resolved). It is NEVER
//     the self-graded calibration `confidence` — calibration may only ADVISORILY
//     DISCOUNT an over-confident arm, never be the reward (Execute-Distill-Verify;
//     model-calibration.md §3 "a mirror, never a stick").
//   • Red-line signatures (auth/money/RLS/PII/migrations/bulk) force `escalate`,
//     regardless of any historical win-rate. History never buys a red-line shortcut.
//
// Usage:
//   node scripts/experience-index.mjs digest [--json]
//   node scripts/experience-index.mjs --suggest "<task text>" [--json] [--top-k N]
// Env:
//   EXPERIENCE_INDEX_ROOT   repo-root override (hermetic tests). Default: cwd.
//                           reads <root>/loops/runs/*.jsonl and <root>/docs/**.

import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';

const ROOT = process.env.EXPERIENCE_INDEX_ROOT || process.cwd();
const RUNS_DIR = path.join(ROOT, 'loops', 'runs');
const DOCS_DIR = path.join(ROOT, 'docs');

// ─────────────────────────────────────────────────────────────────────────────
// signature(task) — the deterministic coarse bucket (plan §3.1). Ported from the
// router's word/tag approach (router.ts scoreMatches/words) — coarse ON PURPOSE so
// buckets accumulate enough outcomes to carry weight. Same idea, no TS import.
// ─────────────────────────────────────────────────────────────────────────────

/** CLAUDE.md red-line globs, collapsed to a keyword test. */
const REDLINE_RE = /\b(auth|login|logout|token|jwt|session|money|price|pricing|payment|payments|tax|refund|checkout|rls|pii|gdpr|privacy|migration|migrations|schema|bulk|mass[- ]?edit)\b/i;

// Surface heuristic — first match wins (order = specificity). governance = the
// harness's own dispatch plane (the only in-scope subject, plan scope-fence).
const SURFACES = [
  ['test', /\b(test|tests|e2e|playwright|coverage|spec|specs|assertion|assertions|mutant|mutants)\b/i],
  ['governance', /\b(loop|loops|harness|router|routing|telemetry|reflection|reflections|lesson|lessons|guardrail|plane|calibration|dispatch|swarm|council)\b/i],
  ['infra', /\b(deploy|deployment|migration|migrations|db|database|schema|docker|fly|infra|backup|restore|pool|boot|ci)\b/i],
  ['api', /\b(api|route|routes|endpoint|handler|handlers|zod|server|websocket|ws|contract|rest)\b/i],
  ['ui', /\b(ui|ux|css|component|components|page|pages|storefront|menu|render|contrast|i18n|button|admin|courier|client|palette|theme)\b/i],
];

// Coarse intent vocabulary — a SMALL fixed set (keeps buckets big). Mirrors the
// router's LOOP_WORTHY phrasing without pulling in generic noise.
const INTENT_TAGS = ['fix', 'converge', 'green', 'polish', 'harden', 'qa', 'perf', 'i18n', 'coverage', 'audit', 'refactor', 'build', 'deslop'];

function words(s) {
  return new Set(String(s || '').toLowerCase().match(/[a-z0-9]+/g) ?? []);
}

/**
 * signature(task) → { tags, surface, redline, scope, key }
 * Pure + deterministic. `key` is a short stable hash — a bucket, not an identity.
 */
export function signature(task) {
  const text = String(task || '');
  const w = words(text);
  const redline = REDLINE_RE.test(text);
  let surface = 'general';
  for (const [name, re] of SURFACES) {
    if (re.test(text)) { surface = name; break; }
  }
  const tags = INTENT_TAGS.filter((t) => w.has(t)).sort();
  // scope: red-line or platform surfaces → Class A; else Class B (coarse).
  const scope = redline || surface === 'infra' || surface === 'governance' ? 'A' : 'B';
  const canonical = JSON.stringify({ tags, surface, redline, scope });
  const key = crypto.createHash('sha1').update(canonical).digest('hex').slice(0, 8);
  return { tags, surface, redline, scope, key };
}

// ─────────────────────────────────────────────────────────────────────────────
// Deterministic reward model (plan §3.3, §4 anti-cheat).
// ─────────────────────────────────────────────────────────────────────────────

/**
 * classifyOutcome(line) → 'win' | 'loss' | 'neutral'
 * Reward derives ONLY from deterministic outcome fields. fake_green ⇒ loss even if
 * "green" (anti-cheat: cheap green cannot be gamed into a win). natural_stop is
 * neutral (ambiguous — neither rewarded nor punished; honest for the degenerate
 * autoupgrade rows). confidence NEVER enters here.
 */
export function classifyOutcome(line) {
  const fakeGreen = Number(line.fake_green_caught ?? 0) || 0;
  if (fakeGreen > 0) return 'loss'; // anti-cheat — hard override
  switch (line.outcome) {
    case 'green': return 'win';
    case 'stall':
    case 'abort': return 'loss';
    case 'natural_stop': return 'neutral';
    default: return 'neutral';
  }
}

/** Wilson score interval lower bound (95%, z=1.96). Pure, stdlib. n=0 ⇒ 0. */
export function wilsonLower(wins, n, z = 1.96) {
  if (n <= 0) return 0;
  const phat = wins / n;
  const z2 = z * z;
  const denom = 1 + z2 / n;
  const center = phat + z2 / (2 * n);
  const margin = z * Math.sqrt((phat * (1 - phat) + z2 / (4 * n)) / n);
  return Math.max(0, (center - margin) / denom);
}

/** Beta(1,1) posterior mean — the "smoothed" win-rate (never used for ranking; shown for honesty). */
function betaMean(wins, losses) {
  return (wins + 1) / (wins + losses + 2);
}

const MIN_TRIALS = 5; // below this, an arm is flagged data-starved (never trusted).

/** arm identity: loop×model×effort where those dims exist on the line, else loop. */
function armOf(line) {
  if (line.arm) return String(line.arm);
  const parts = [line.loop ?? 'unknown'];
  if (line.model) parts.push(String(line.model));
  if (line.effort) parts.push(String(line.effort));
  return parts.join('×');
}

// ─────────────────────────────────────────────────────────────────────────────
// Store readers (all tolerant — a missing/garbled store degrades to empty).
// ─────────────────────────────────────────────────────────────────────────────

function readJsonl(file) {
  try {
    return fs.readFileSync(file, 'utf8')
      .split('\n')
      .map((l) => l.trim())
      .filter(Boolean)
      .map((l) => { try { return JSON.parse(l); } catch { return null; } })
      .filter(Boolean);
  } catch { return []; }
}

function readMetrics() { return readJsonl(path.join(RUNS_DIR, 'metrics.jsonl')); }
function readPredictions() { return readJsonl(path.join(RUNS_DIR, 'predictions.jsonl')); }

/** The signature a metrics line falls under: its own `.signature` (Stage 0 stamp)
 *  if present, else derived from the loop id (the loop IS the task bucket today). */
function sigOfMetricLine(line) {
  if (line.signature && typeof line.signature === 'object' && line.signature.key) return line.signature;
  if (typeof line.signature === 'string') return { key: line.signature, tags: [], surface: 'general', redline: false, scope: 'B', derived: true };
  return { ...signature(line.loop || ''), derived: true };
}

// ─────────────────────────────────────────────────────────────────────────────
// Aggregation — group metrics by signature → arm → quality-gated win-rate.
// ─────────────────────────────────────────────────────────────────────────────

function aggregateArms(lines) {
  // sigKey → { sig, arms: Map<arm, stats> }
  const bySig = new Map();
  for (const line of lines) {
    const sig = sigOfMetricLine(line);
    if (!bySig.has(sig.key)) bySig.set(sig.key, { sig, arms: new Map() });
    const bucket = bySig.get(sig.key);
    const arm = armOf(line);
    if (!bucket.arms.has(arm)) {
      bucket.arms.set(arm, { arm, wins: 0, losses: 0, neutral: 0, runs: 0, cost_sum: 0, per_resolved: [], outcomes: {} });
    }
    const s = bucket.arms.get(arm);
    s.runs++;
    s.outcomes[line.outcome] = (s.outcomes[line.outcome] ?? 0) + 1;
    const cls = classifyOutcome(line);
    if (cls === 'win') s.wins++;
    else if (cls === 'loss') s.losses++;
    else s.neutral++;
    if (typeof line.cost_usd === 'number') s.cost_sum += line.cost_usd;
    if (typeof line.per_resolved === 'number') s.per_resolved.push(line.per_resolved);
  }
  return bySig;
}

/** Rank arms within one signature bucket. PRIMARY = Wilson lower bound of the
 *  quality-gated win-rate (a cheap fake-green arm sinks; a tiny-n arm sinks).
 *  SECONDARY tiebreaks only: per_resolved asc, then cost asc. (plan §3.3) */
export function rankArms(armsMap) {
  return [...armsMap.values()].map((s) => {
    const n = s.wins + s.losses; // quality trials only (neutral excluded)
    const wilson = wilsonLower(s.wins, n);
    const meanPerResolved = s.per_resolved.length
      ? s.per_resolved.reduce((a, b) => a + b, 0) / s.per_resolved.length : null;
    return {
      arm: s.arm,
      wins: s.wins, losses: s.losses, neutral: s.neutral, runs: s.runs,
      trials: n,
      win_rate: n ? s.wins / n : null,
      smoothed_win_rate: betaMean(s.wins, s.losses),
      wilson_lower: wilson,
      mean_per_resolved: meanPerResolved,
      mean_cost_usd: s.runs ? s.cost_sum / s.runs : 0,
      insufficient_data: n < MIN_TRIALS,
      outcomes: s.outcomes,
    };
  }).sort((a, b) => {
    if (b.wilson_lower !== a.wilson_lower) return b.wilson_lower - a.wilson_lower;
    const ap = a.mean_per_resolved ?? Infinity, bp = b.mean_per_resolved ?? Infinity;
    if (ap !== bp) return ap - bp;                       // secondary: fewer tokens/resolved
    return a.mean_cost_usd - b.mean_cost_usd;            // tertiary: cheaper (tiebreak ONLY)
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// Calibration gap (advisory discount only — never the reward). plan §3.3(3).
// ─────────────────────────────────────────────────────────────────────────────

function calibrationBySig(predictions) {
  const bySig = new Map();
  for (const p of predictions) {
    const sig = signature(p.target || p.prediction || '');
    if (!bySig.has(sig.key)) bySig.set(sig.key, { sig, total: 0, resolved: 0, hits: 0, conf_sum: 0, conf_n: 0, misses: [] });
    const c = bySig.get(sig.key);
    c.total++;
    if (typeof p.confidence === 'number') { c.conf_sum += p.confidence; c.conf_n++; }
    if (p.gap != null) {
      c.resolved++;
      if (p.gap === 'hit') c.hits++;
      else c.misses.push(p);
    }
  }
  for (const c of bySig.values()) {
    c.hit_rate = c.resolved ? c.hits / c.resolved : null;
    c.mean_confidence = c.conf_n ? c.conf_sum / c.conf_n : null;
    // gap > 0 ⇒ over-confident (advisory: read this arm's confidence DOWN).
    c.calibration_gap = (c.hit_rate != null && c.mean_confidence != null) ? c.mean_confidence - c.hit_rate : null;
  }
  return bySig;
}

// ─────────────────────────────────────────────────────────────────────────────
// Pre-task failure-corpus retrieval (ExpeL / ReasoningBank — plan §3.3(2)).
// Advisory pack: top-k reflections / lessons / regression rows / calibration
// misses by keyword overlap. All read-only, all tolerant of a missing corpus.
// ─────────────────────────────────────────────────────────────────────────────

function safeReadDir(dir) { try { return fs.readdirSync(dir); } catch { return []; } }
function safeRead(file) { try { return fs.readFileSync(file, 'utf8'); } catch { return ''; } }

function firstHeading(md) {
  const m = md.match(/^#+\s*(.+)$/m);
  return m ? m[1].trim() : '';
}

function collectCorpus(predictions) {
  const items = [];
  // reflections (WHY memory) — INBOX + ARCHIVE.
  for (const sub of ['INBOX', 'ARCHIVE']) {
    const d = path.join(DOCS_DIR, 'reflections', sub);
    for (const f of safeReadDir(d)) {
      if (!f.endsWith('.md')) continue;
      const body = safeRead(path.join(d, f));
      items.push({ kind: 'reflection', ref: `docs/reflections/${sub}/${f}`, text: `${f} ${firstHeading(body)}` });
    }
  }
  // lessons — the machine-parsable INDEX rows.
  const idx = safeRead(path.join(DOCS_DIR, 'lessons', 'INDEX.md'));
  for (const line of idx.split('\n')) {
    const m = line.match(/^\|\s*(.+?)\s*\|\s*(docs\/lessons\/.+?\.md)\s*\|/);
    if (m) items.push({ kind: 'lesson', ref: m[2], text: `${m[1]} ${path.basename(m[2], '.md')}` });
  }
  // regression ledger rows — symptom + root cause.
  const led = safeRead(path.join(DOCS_DIR, 'regressions', 'REGRESSION-LEDGER.md'));
  for (const line of led.split('\n')) {
    const cells = line.split('|').map((c) => c.trim());
    // | # | Symptom | Root cause | Guardrail type | Where | Date |
    if (cells.length >= 7 && /^\d+$/.test(cells[1])) {
      items.push({ kind: 'regression', ref: `REGRESSION-LEDGER #${cells[1]}`, text: `${cells[2]} ${cells[3]}` });
    }
  }
  // calibration MISSES (predictions gap!=hit) — "what we mis-predicted here before".
  for (const p of predictions) {
    if (p.gap != null && p.gap !== 'hit') {
      items.push({ kind: 'calibration-miss', ref: `prediction ${p.prediction_id ?? p.target ?? '?'}`, text: `${p.target ?? ''} ${p.prediction ?? ''}` });
    }
  }
  return items;
}

const KIND_PRIORITY = { regression: 3, 'calibration-miss': 2, reflection: 1, lesson: 0 };

/** Rank corpus items by keyword overlap with the task's signature words. */
function retrievePack(task, sig, predictions, k) {
  const qWords = new Set([...words(task), ...sig.tags, sig.surface]);
  const items = collectCorpus(predictions);
  const scored = items.map((it) => {
    const iw = words(it.text);
    let overlap = 0;
    for (const q of qWords) if (q.length > 2 && iw.has(q)) overlap++;
    return { ...it, score: overlap };
  }).filter((it) => it.score > 0)
    .sort((a, b) => (b.score - a.score) || ((KIND_PRIORITY[b.kind] ?? 0) - (KIND_PRIORITY[a.kind] ?? 0)));
  return scored.slice(0, k);
}

// ─────────────────────────────────────────────────────────────────────────────
// Report builders.
// ─────────────────────────────────────────────────────────────────────────────

function buildDigest() {
  const metrics = readMetrics();
  const predictions = readPredictions();
  const bySig = aggregateArms(metrics);
  const calib = calibrationBySig(predictions);
  const signatures = [...bySig.values()].map(({ sig, arms }) => {
    const ranked = rankArms(arms);
    const cal = calib.get(sig.key);
    return {
      key: sig.key,
      sig: { tags: sig.tags, surface: sig.surface, redline: !!sig.redline, scope: sig.scope, derived: !!sig.derived },
      total_runs: ranked.reduce((a, r) => a + r.runs, 0),
      quality_trials: ranked.reduce((a, r) => a + r.trials, 0),
      arms: ranked,
      calibration: cal ? { hit_rate: cal.hit_rate, mean_confidence: cal.mean_confidence, calibration_gap: cal.calibration_gap, resolved: cal.resolved, total: cal.total } : null,
    };
  }).sort((a, b) => b.total_runs - a.total_runs);
  return {
    advisory: true,
    reward_source: 'deterministic-outcome (outcome, fake_green_caught, fail_start→fail_end, per_resolved) — NOT calibration confidence',
    metrics_rows: metrics.length,
    prediction_rows: predictions.length,
    signatures,
  };
}

function buildSuggest(task, k) {
  const sig = signature(task);
  const metrics = readMetrics();
  const predictions = readPredictions();
  const bySig = aggregateArms(metrics);
  const bucket = bySig.get(sig.key);
  const ranked = bucket ? rankArms(bucket.arms) : [];
  const cal = calibrationBySig(predictions).get(sig.key) || null;
  const pack = retrievePack(task, sig, predictions, k);
  // Red-line overrides EVERYTHING (plan §4): history never buys a red-line shortcut.
  const recommendation = sig.redline
    ? { action: 'escalate', reason: 'red-line signature (auth/money/RLS/PII/migrations/bulk) → Triadic Council + human. History is ignored on red-lines.' }
    : ranked.length && !ranked[0].insufficient_data
      ? { action: 'prefer-arm', arm: ranked[0].arm, reason: `highest quality-gated win-rate (Wilson lower bound ${ranked[0].wilson_lower.toFixed(3)} over ${ranked[0].trials} trials)` }
      : { action: 'no-recommendation', reason: `insufficient outcome data for signature ${sig.key} (${ranked.reduce((a, r) => a + r.trials, 0)} quality trials) — falls back to the router's static decision` };
  return {
    advisory: true, // Stage 1: the router does NOT consume this.
    reward_source: 'deterministic-outcome — NOT calibration confidence',
    task,
    signature: { key: sig.key, tags: sig.tags, surface: sig.surface, redline: sig.redline, scope: sig.scope },
    recommendation,
    ranked_arms: ranked,
    calibration: cal ? { hit_rate: cal.hit_rate, mean_confidence: cal.mean_confidence, calibration_gap: cal.calibration_gap, note: cal.calibration_gap != null && cal.calibration_gap > 0 ? 'over-confident here — advisory discount only' : null } : null,
    advisory_pack: pack,
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Text rendering.
// ─────────────────────────────────────────────────────────────────────────────

function pct(x) { return x == null ? 'n/a' : `${(x * 100).toFixed(0)}%`; }

function renderDigest(d) {
  const L = [];
  L.push('experience-index digest  [ADVISORY · report-only · exit 0]');
  L.push(`reward source: ${d.reward_source}`);
  L.push(`data: ${d.metrics_rows} metrics rows · ${d.prediction_rows} prediction rows`);
  if (!d.signatures.length) { L.push('  (no outcome data — nothing to rank)'); return L.join('\n'); }
  for (const s of d.signatures) {
    const derived = s.sig.derived ? ' (sig derived from loop id — no Stage-0 stamp yet)' : '';
    L.push('');
    L.push(`■ signature ${s.key}  surface=${s.sig.surface} scope=${s.sig.scope}${s.sig.redline ? ' REDLINE' : ''}  tags=[${s.sig.tags.join(',')}]${derived}`);
    L.push(`  runs=${s.total_runs}  quality-trials=${s.quality_trials}`);
    for (const a of s.arms) {
      const flag = a.insufficient_data ? '  ⚠ insufficient data' : '';
      const outc = Object.entries(a.outcomes).map(([o, n]) => `${o}:${n}`).join(' ');
      L.push(`    ${a.arm}`);
      L.push(`        win-rate ${pct(a.win_rate)} (W${a.wins}/L${a.losses}/neutral${a.neutral}) · Wilson-LB ${a.wilson_lower.toFixed(3)}${flag}`);
      L.push(`        outcomes[${outc}] · per_resolved ${a.mean_per_resolved ?? 'n/a'} · cost $${a.mean_cost_usd.toFixed(2)}`);
    }
    if (s.calibration && s.calibration.calibration_gap != null) {
      L.push(`  calibration: hit-rate ${pct(s.calibration.hit_rate)} vs mean-conf ${pct(s.calibration.mean_confidence)} · gap ${s.calibration.calibration_gap.toFixed(2)} (advisory discount only)`);
    }
  }
  return L.join('\n');
}

function renderSuggest(s) {
  const L = [];
  L.push('experience-index --suggest  [ADVISORY · router does NOT consume this — Stage 1]');
  L.push(`reward source: ${s.reward_source}`);
  L.push(`task: ${s.task}`);
  L.push(`signature ${s.signature.key}: surface=${s.signature.surface} scope=${s.signature.scope}${s.signature.redline ? ' REDLINE' : ''} tags=[${s.signature.tags.join(',')}]`);
  L.push(`→ recommendation: ${s.recommendation.action}${s.recommendation.arm ? ` = ${s.recommendation.arm}` : ''}`);
  L.push(`  ${s.recommendation.reason}`);
  if (s.ranked_arms.length) {
    L.push('  ranked arms (Wilson-LB primary; per_resolved/cost tiebreak only):');
    for (const a of s.ranked_arms) {
      L.push(`    - ${a.arm}: Wilson-LB ${a.wilson_lower.toFixed(3)} (W${a.wins}/L${a.losses})${a.insufficient_data ? ' ⚠ insufficient' : ''}`);
    }
  } else {
    L.push('  (no arm history for this signature)');
  }
  if (s.calibration && s.calibration.calibration_gap != null) {
    L.push(`  calibration gap ${s.calibration.calibration_gap.toFixed(2)}${s.calibration.note ? ` — ${s.calibration.note}` : ''}`);
  }
  L.push(`  advisory pack (top ${s.advisory_pack.length} — what bit us here before):`);
  if (!s.advisory_pack.length) L.push('    (empty — no matching corpus items)');
  for (const it of s.advisory_pack) L.push(`    [${it.kind}] ${it.ref}  (overlap ${it.score})`);
  return L.join('\n');
}

// ─────────────────────────────────────────────────────────────────────────────
// CLI — always exits 0 (advisory; never blocks). No writes anywhere.
// ─────────────────────────────────────────────────────────────────────────────

function parseArgs(argv) {
  const a = { json: false, suggest: null, digest: false, topK: 5 };
  for (let i = 0; i < argv.length; i++) {
    const t = argv[i];
    if (t === '--json') a.json = true;
    else if (t === '--suggest') a.suggest = argv[++i] ?? '';
    else if (t === '--top-k') a.topK = Math.max(1, parseInt(argv[++i] ?? '5', 10) || 5);
    else if (t === 'digest') a.digest = true;
    else if (!t.startsWith('-') && a.suggest === null && !a.digest) a.digest = true; // bare word ⇒ digest
  }
  return a;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  try {
    if (args.suggest !== null) {
      const out = buildSuggest(args.suggest, args.topK);
      console.log(args.json ? JSON.stringify(out, null, 2) : renderSuggest(out));
    } else {
      const out = buildDigest();
      console.log(args.json ? JSON.stringify(out, null, 2) : renderDigest(out));
    }
  } catch (e) {
    // Advisory: a read/parse failure must never break the caller.
    console.error(`[experience-index] advisory read failed (ignored): ${e?.message ?? e}`);
  }
  process.exit(0); // HARD: always exit 0.
}

if (import.meta.url === `file://${process.argv[1]}`) main();
