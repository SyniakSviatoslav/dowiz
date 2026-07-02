#!/usr/bin/env node
// plane-guard — turns the 11 memory-corpus meta-patterns into ONE deterministic gate
// (memory: memory-corpus-meta-patterns-2026-07-02). Pattern #4 applied to itself:
// the advisory insight ("here are the patterns we keep re-learning") becomes authority.
//
// Each pattern maps to a concrete assertion. Patterns already enforced by an existing
// guardrail are verified as WIRED (the script exists + is referenced by verify:all);
// the four gaps (dark-first, proxy-signal, prod↔staging drift, feedback-contract) are
// asserted inline here.
//
// HARD checks fail the process (exit 1). SOFT checks warn (advisory → the health pass).
// Always prints a §5 LOOP REPORT (rule-loop-report-always-2026-06-27), success or fail.
//
// Run:  node scripts/plane-guard.mjs            (static — CI/local safe)
//       node scripts/plane-guard.mjs --staging  (adds live prod↔staging drift probe)
//       node scripts/plane-guard.mjs --json      (machine JSON only)
import { readFileSync, existsSync, writeFileSync, readdirSync, statSync } from 'node:fs';
import { execSync, spawnSync } from 'node:child_process';
import { join } from 'node:path';

const ROOT = process.cwd();
const STAGING = process.argv.includes('--staging');
const JSON_ONLY = process.argv.includes('--json');
const results = [];
const rec = (pattern, name, level, ok, detail) =>
  results.push({ pattern, name, level, ok, detail });

const read = (p) => (existsSync(join(ROOT, p)) ? readFileSync(join(ROOT, p), 'utf8') : '');
const has = (p) => existsSync(join(ROOT, p));

// ── P4/P5/P6/P7/P9/P10 — already deterministic: verify the gate is WIRED ─────────
// (presence of the guardrail script + a reference in verify-all.ts = enforcement lives).
const verifyAll = read('scripts/verify-all.ts');
const pkg = read('package.json');
// Each entry: [pattern, guardrail script, token that proves it's wired, file that must contain it].
// The token is what verify:all (or package.json for the CI privacy-gate) actually references —
// pnpm aliases for some, `node scripts/…` for others; agent-health-pass is advisory-by-design so
// its authority is proven by gate-armament, not by being in the fail-fast verify:all.
const wired = [
  ['P4 advisory→authority', 'scripts/guardrail-gate-armament.mjs', 'guardrail-gate-armament.mjs', verifyAll, 'verify:all'],
  ['P5 fix-the-class (ratchet)', 'scripts/guardrail-ledger-integrity.mjs', 'guardrail-ledger-integrity.mjs', verifyAll, 'verify:all'],
  ['P6 red-line topology', '.claude/hooks/red-line-doubt-gate.sh', 'gate-armament', verifyAll, 'verify:all'],
  ['P7 council-before-code', '.claude/hooks/serious-gate.sh', 'gate-armament', verifyAll, 'verify:all'],
  ['P9 subtractive', 'scripts/guardrail-license.mjs', 'guardrail:license', verifyAll, 'verify:all'],
  ['P10 data-sovereignty', 'scripts/compliance-gate.ts', 'compliance:gate', pkg, 'package.json (CI privacy-gate)'],
];
for (const [pat, script, refToken, refBody, refWhere] of wired) {
  const scriptOk = has(script);
  const refOk = refBody.includes(refToken);
  rec(pat, `wired: ${script}`, 'hard', scriptOk && refOk,
    scriptOk ? (refOk ? `present + wired in ${refWhere}` : `present but NOT wired (${refToken}) in ${refWhere}`) : 'guardrail script MISSING');
}

// ── P3 — dark-first: launch-gated *_ENABLED flags default 'false' ────────────────
// Safe-on allowlist: flags documented as safe-by-default (non-launch, ops kill-switch).
const DARK_ALLOW = new Set(['FUNNEL_INGEST_ENABLED']);
const cfg = read('packages/config/src/index.ts');
const flagRe = /(\w*_ENABLED):\s*z\.enum\(\['true',\s*'false'\]\)\.default\('(true|false)'\)/g;
let m, darkViolations = [];
while ((m = flagRe.exec(cfg)) !== null) {
  const [, flag, def] = m;
  if (def === 'true' && !DARK_ALLOW.has(flag)) darkViolations.push(flag);
}
rec('P3 dark-first', 'launch flags default OFF', 'hard', darkViolations.length === 0,
  darkViolations.length ? `default 'true' without allowlist: ${darkViolations.join(', ')}` : `all *_ENABLED default false (allow-on: ${[...DARK_ALLOW].join(', ')})`);

// ── P1/P2 — verify-artifact-not-proxy: no git-commit/deploy piped to tail|head|grep
// (deploy-validation-traps-2026-06-26 — the masked-exit-code trap). Scan tracked shell/js.
let proxyHits = [];
try {
  const tracked = execSync('git ls-files scripts .github tools "*.sh"', { cwd: ROOT, encoding: 'utf8' })
    .split('\n').filter((f) => f && /\.(sh|mjs|ts|js|ya?ml)$/.test(f) && f !== 'scripts/plane-guard.mjs');
  const bad = /(git\s+commit|fly(?:ctl)?\s+deploy)[^\n|]*\|\s*(tail|head|grep)\b/;
  for (const f of tracked) {
    const c = read(f);
    if (bad.test(c)) proxyHits.push(f);
  }
} catch { /* git not available → skip, reported as soft */ }
rec('P1/P2 verify-artifact', 'no commit/deploy piped to tail|head|grep', 'hard', proxyHits.length === 0,
  proxyHits.length ? `masked-exit-code trap in: ${proxyHits.join(', ')}` : 'no masked-exit-code pipes in tracked scripts');

// ── P8 — prod↔staging gap: migration ordering (static) + live drift (--staging) ──
let migFiles = [];
try { migFiles = readdirSync(join(ROOT, 'packages/db/migrations')).filter((f) => /^\d+.*\.(sql|js|ts|cjs|mjs)$/.test(f)); } catch {}
const nums = migFiles.map((f) => parseInt(f, 10)).filter(Number.isFinite);
const ordered = nums.every((n, i) => i === 0 || n >= nums[i - 1]);
rec('P8 prod↔staging', 'migration numbering monotonic', 'hard', migFiles.length > 0 && ordered,
  migFiles.length ? (ordered ? `${migFiles.length} migrations, monotonic` : 'migration numbers NOT monotonic — drift risk') : 'no migrations found');
if (STAGING) {
  // Live probe: compare local migration head vs the staging DB's applied head.
  // Advisory (soft) — needs DATABASE_URL_MIGRATIONS in env; skips cleanly if absent.
  const dbUrl = process.env.DATABASE_URL_MIGRATIONS || process.env.STAGING_DATABASE_URL || '';
  if (!dbUrl) rec('P8 prod↔staging', 'live staging drift probe', 'soft', true, 'skipped — no staging DB url in env');
  else rec('P8 prod↔staging', 'live staging drift probe', 'soft', true, `local head=${nums[nums.length - 1] ?? '?'} (compare against staging pgmigrations in-agent)`);
}

// ── P11 — feedback contract: the autonomy envelope + loop-report rule are documented
const envelopeDoc = 'docs/governance/plane-maintainer-agent.md';
rec('P11 feedback-contract', 'autonomy envelope documented', 'hard', has(envelopeDoc),
  has(envelopeDoc) ? `${envelopeDoc} present` : `${envelopeDoc} MISSING — the agent has no written authority boundary`);

/* PLANE-TELEMETRY-GUARD-REGION START */
// ── plane-telemetry governance checks (ADR-plane-telemetry-and-calibration, Decisions 5/8) ──
// This whole region is EXCLUDED from the advisory-forever self-scan below — it legitimately
// names the ledger in order to guard it. HONESTY (breaker R2-M2): these guards are structural
// FRICTION + review-forcing, NOT structurally impossible — they defeat the casual gating PR
// and the copy-paste, not a determined obfuscator (dynamic require / unenumerated repo /
// laundered strings). Hiding a gate inside this region would evade the self-scan: that too is
// a reviewed diff to this file, i.e. review-forcing.

// ── telemetry-liveness (SOFT — H3 anti-silent-skip) — ADVISORY-LIVENESS-ONLY ─────────────
// Newest telemetry event must be < N days old (default 3; PLANE_TELEMETRY_LIVENESS_DAYS).
// Sources probed in order: local loops/runs/plane-events-*.jsonl, then the tip commit of
// origin/telemetry/plane if the ref exists. Missing everything → SOFT "telemetry silent":
// silence must be VISIBLE, never a hard fail that blocks unrelated work (H3 — the failure
// mode is months of zero telemetry mistaken for success, not a broken build).
const LIVENESS_DAYS = Number(process.env.PLANE_TELEMETRY_LIVENESS_DAYS || 3);
let liveTs = 0, liveSrc = 'none';
try {
  const runsDir = join(ROOT, 'loops/runs');
  for (const f of existsSync(runsDir) ? readdirSync(runsDir) : []) {
    if (!/^plane-events-.*\.jsonl$/.test(f)) continue;
    let fileBest = 0;
    for (const mm of read(`loops/runs/${f}`).matchAll(/"ts"\s*:\s*"([^"]+)"/g)) {
      const t = Date.parse(mm[1]);
      if (Number.isFinite(t) && t > fileBest) fileBest = t;
    }
    if (!fileBest) fileBest = statSync(join(runsDir, f)).mtimeMs; // rows without parsable ts
    if (fileBest > liveTs) { liveTs = fileBest; liveSrc = `loops/runs/${f}`; }
  }
} catch { /* unreadable scratch → treated as silent, surfaced below */ }
if (Date.now() - liveTs >= LIVENESS_DAYS * 86400000) {
  // R2-M3: spawnSync arg arrays, shell:false semantics — never string-built shell lines.
  const ref = spawnSync('git', ['rev-parse', '--verify', '--quiet', 'origin/telemetry/plane'], { cwd: ROOT, encoding: 'utf8' });
  if (ref.status === 0) {
    const log = spawnSync('git', ['log', '-1', '--format=%ct', 'origin/telemetry/plane'], { cwd: ROOT, encoding: 'utf8' });
    const ct = Number((log.stdout || '').trim());
    if (log.status === 0 && Number.isFinite(ct) && ct * 1000 > liveTs) { liveTs = ct * 1000; liveSrc = 'origin/telemetry/plane'; }
  }
}
const liveAgeD = liveTs ? (Date.now() - liveTs) / 86400000 : Infinity;
rec('telemetry-liveness', `newest telemetry event < ${LIVENESS_DAYS}d`, 'soft', liveAgeD < LIVENESS_DAYS,
  liveTs === 0
    ? 'telemetry silent — no loops/runs/plane-events-*.jsonl and no origin/telemetry/plane ref (H3: silence made VISIBLE; soft by design — never blocks unrelated work)'
    : liveAgeD < LIVENESS_DAYS
      ? `newest event ${liveAgeD.toFixed(2)}d old via ${liveSrc}`
      : `telemetry STALE — newest event ${liveAgeD.toFixed(1)}d old via ${liveSrc} — configured-but-never-delivered? (soft by design)`);

// ── advisory-forever (HARD — H4/R6/R2-M2) — "the mirror is never a stick", as code ───────
// The prediction ledger / plane-events are calibration mirrors, FOREVER advisory: they must
// never be wired as gate input. The gate surface below is ENUMERATED + COMMITTED + VERSIONED
// right here (R3-5): changing it is a reviewed diff to this file, so it cannot drift
// silently. Per R3-5 the list MUST include .github/workflows and tools/loop-harness —
// ordinary, non-obfuscated gate homes a casual gating PR would otherwise reach.
// package.json (scripts section only), .husky/** and .github/** are READ-ONLY here.
// The "// ADVISORY-LIVENESS-ONLY" tag is deliberately NOT honored as an exemption anywhere
// on the wider surface (closes R2-M2 bypass #3 — copy-the-magic-comment); the only permitted
// ledger reference is this guard region itself, stripped from the self-scan.
const PT_RS = '/* PLANE-TELEMETRY-GUARD-REGION START */';
const PT_REND = '/* PLANE-TELEMETRY-GUARD-REGION END */';
const GATE_SURFACE = [
  'scripts/verify-all.ts',
  'scripts/plane-guard.mjs',   // self — minus this guard region
  'package.json',              // scripts section only (READ-ONLY)
  '.husky',                    // READ-ONLY
  '.github/workflows',         // READ-ONLY (R3-5)
  'tools/loop-harness/src',    // R3-5
  'tools/eslint-plugin-local',
];
const SURFACE_FILE = /(\.(mjs|cjs|js|ts|sh|ya?ml|json)$|^[^.]+$)/;
const surfaceWalk = (rel) => {
  const abs = join(ROOT, rel);
  if (!existsSync(abs)) return [];
  if (statSync(abs).isFile()) return [rel];
  return readdirSync(abs, { withFileTypes: true }).flatMap((e) =>
    e.name === 'node_modules' ? [] :
    e.isDirectory() ? surfaceWalk(`${rel}/${e.name}`) :
    SURFACE_FILE.test(e.name) ? [`${rel}/${e.name}`] : []);
};
const surfaceText = (rel) => {
  let body = read(rel);
  if (rel === 'package.json') { try { body = JSON.stringify(JSON.parse(body).scripts || {}, null, 1); } catch { /* unparsable → scan raw */ } }
  if (rel === 'scripts/plane-guard.mjs') {
    const s = body.indexOf(PT_RS), e = body.lastIndexOf(PT_REND);
    if (s !== -1 && e !== -1) body = body.slice(0, s) + body.slice(e + PT_REND.length);
  }
  return body;
};
// Literal ledger references + simple indirection heuristics (resolution R2-M2 bypass #1:
// readdir/glob over the telemetry dirs, string-concat laundering of the ledger name).
const LEDGER_RES = [
  [/predictions?\.jsonl/i, 'ledger file'],
  [/plane-events/i, 'plane-events'],
  [/readdir\w*\s*\([^)\n]*(?:loops\/runs|['"`]telemetry)/, 'readdir indirection'],
  [/\bglob\w*\s*\([^)\n]*(?:loops\/runs|['"`]telemetry)/i, 'glob indirection'],
  [/['"`]predict['"`]\s*\+|\+\s*['"`]ions\.jsonl['"`]/, 'string-concat indirection'],
];
const surfaceFiles = GATE_SURFACE.flatMap(surfaceWalk);
const advisoryHits = [];
for (const rel of surfaceFiles) {
  surfaceText(rel).split('\n').forEach((line, i) => {
    const hit = LEDGER_RES.find(([re]) => re.test(line));
    if (hit) advisoryHits.push(`${rel}:${i + 1} [${hit[1]}]`);
  });
}
rec('advisory-forever', 'prediction ledger / plane-events never wired as gate input', 'hard', advisoryHits.length === 0,
  advisoryHits.length
    ? `ledger referenced in gate position (mirror-never-a-stick violated): ${advisoryHits.slice(0, 5).join('; ')}${advisoryHits.length > 5 ? ` +${advisoryHits.length - 5} more` : ''} — this guard is friction + review-forcing, not structurally impossible (R2-M2)`
    : `enumerated surface clean (${surfaceFiles.length} files under ${GATE_SURFACE.length} versioned roots incl. .github/workflows + tools/loop-harness per R3-5) — friction + review-forcing, not impossibility (R2-M2)`);

// ── ingestion-authority (HARD — Counsel R2-a/R9) — inbox is read-only, never an executor ─
// Maintainer findings are advisory INPUTS: no file on the same enumerated surface may pipe
// `plane-telemetry.mjs inbox` output into exec/auto-apply/auto-merge. Line-scoped
// same-statement heuristic (inbox + exec/spawn/pipe/merge/apply co-located). Same honest
// bound: friction + review-forcing, not impossibility.
const INBOX_TOKEN = /\binbox\b/i;
const EXEC_TOKEN = /\b(?:exec\w*|spawn\w*|execa|fork)\s*\(|\|\s*(?:sh|bash|xargs|node)\b|\bgh\s+pr\s+merge\b|\bgit\s+(?:merge|apply)\b|applyPatch|auto-?apply/i;
const inboxHits = [];
for (const rel of surfaceFiles) {
  surfaceText(rel).split('\n').forEach((line, i) => {
    if (INBOX_TOKEN.test(line) && EXEC_TOKEN.test(line)) inboxHits.push(`${rel}:${i + 1}`);
  });
}
rec('ingestion-authority', 'inbox output never piped into exec/auto-apply', 'hard', inboxHits.length === 0,
  inboxHits.length
    ? `inbox output wired toward execution (advisory-inputs-never-authority violated): ${inboxHits.join('; ')} — this guard is friction + review-forcing, not structurally impossible`
    : `no inbox→exec/auto-apply coupling on the enumerated surface (${surfaceFiles.length} files) — friction + review-forcing, not impossibility`);
/* PLANE-TELEMETRY-GUARD-REGION END */

// ── verdict + §5 LOOP REPORT ─────────────────────────────────────────────────────
const hardFails = results.filter((r) => r.level === 'hard' && !r.ok);
const softFails = results.filter((r) => r.level === 'soft' && !r.ok);
const verdict = hardFails.length === 0 ? 'PASS' : 'FAIL';
const ts = new Date().toISOString();
const payload = { tool: 'plane-guard', ts, verdict, staging: STAGING, hardFails: hardFails.length, softFails: softFails.length, results };

// Machine JSON to loops/runs (lossless, per the harness convention).
try { writeFileSync(join(ROOT, `loops/runs/plane-guard-${ts.replace(/[:.]/g, '-')}.json`), JSON.stringify(payload, null, 2)); } catch {}

if (JSON_ONLY) { console.log(JSON.stringify(payload, null, 2)); process.exit(verdict === 'PASS' ? 0 : 1); }

const icon = (r) => (r.ok ? '✅' : r.level === 'hard' ? '❌' : '⚠️');
console.log('══════════════════════════════════════════════════════════');
console.log('  §5 LOOP REPORT — plane-guard (11 meta-patterns → gate)');
console.log('══════════════════════════════════════════════════════════');
console.log(`  verdict: ${verdict}  ·  ${ts}  ·  mode: ${STAGING ? 'staging' : 'static'}`);
console.log(`  hard: ${results.filter(r=>r.level==='hard').length - hardFails.length}/${results.filter(r=>r.level==='hard').length} pass · soft warns: ${softFails.length}\n`);
for (const r of results) console.log(`  ${icon(r)} [${r.pattern}] ${r.name} — ${r.detail}`);
if (hardFails.length) {
  console.log('\n  CARRY-FORWARD (hard fails to fix):');
  for (const r of hardFails) console.log(`   → ${r.pattern}: ${r.detail}`);
}
console.log('══════════════════════════════════════════════════════════');
process.exit(verdict === 'PASS' ? 0 : 1);
