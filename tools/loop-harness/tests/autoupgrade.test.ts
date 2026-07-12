import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {
  classify, applyCandidate, evaluateLenses, recordKeptUpgrades,
  type Candidate, type ApplyOutcome,
} from '../src/autoupgrade.js';
import * as au from '../src/autoupgrade.js';
import { readProvenUpgrades } from '../src/proven-upgrades.js';
import { readMetrics } from '../src/storage.js';

const cand = (over: Partial<Candidate>): Candidate => ({
  id: 'c', pattern: '', source: 't', area: '', evidence: '', expected_speedup: '',
  blast_radius: 'low', reversible: true, action: '', ...over,
});

// FIRM BOUNDARY — every one of these MUST be Class B (never autonomously mutated).
const BOUNDARY: Array<[string, string]> = [
  ['auth', 'swap the login/session JWT verifier'],
  ['rls', 'change a tenant-isolation RLS policy'],
  ['secrets', 'rotate the pii-cipher secret key'],
  ['payments', 'adjust the cash settlement money math'],
  ['pii', 'tweak the PII anonymizer / gdpr erasure'],
  ['schema', 'add a column via a schema migration'],
  ['architecture', 'move from monolith to microservices on k8s'],
  ['major-dep', 'major breaking upgrade of the queue runtime'],
];

for (const [label, action] of BOUNDARY) {
  test(`classify — firm boundary '${label}' → Class B (never auto)`, () => {
    const r = classify(cand({ area: label, action, blast_radius: 'low', reversible: true }));
    assert.equal(r.class, 'B', `${label} must be B; got ${r.class} (${r.reason})`);
  });
}

test('classify — even a reversible low-blast PII change is Class B', () => {
  // the dangerous case: passes the cheap A heuristics but is firm-boundary
  const r = classify(cand({ area: 'dev-loop', pattern: 'redact PII before the LLM call', action: 'edit anonymizer', reversible: true, blast_radius: 'low' }));
  assert.equal(r.class, 'B');
});

test('classify — perf/dev-loop changes are Class A', () => {
  assert.equal(classify(cand({ area: 'dev-loop mcp config', action: "claude mcp remove 'x'" })).class, 'A');
  assert.equal(classify(cand({ area: 'perf db', pattern: 'covering index for slow query', action: 'CREATE INDEX CONCURRENTLY' })).class, 'A');
  assert.equal(classify(cand({ area: 'test-setup', action: 'reuse testcontainer' })).class, 'A');
});

test('classify — regression: "loaded every session" prose does NOT false-trip Class B', () => {
  // the canonical Class-A example (prune ghost MCP) must stay A despite the word
  // "session" appearing innocuously in its description.
  const r = classify(cand({
    area: 'dev-loop mcp config token-bloat',
    pattern: "unused MCP server 'claude_ai_Notion' loaded every session",
    action: "claude mcp remove 'claude_ai_Notion'",
  }));
  assert.equal(r.class, 'A', `ghost-MCP prune must be A; got ${r.class} (${r.reason})`);
});

test('classify — fail-safe: not reversible → B, high blast → B', () => {
  assert.equal(classify(cand({ area: 'dev-loop', reversible: false })).class, 'B');
  assert.equal(classify(cand({ area: 'dev-loop', blast_radius: 'high' })).class, 'B');
});

test('applyCandidate — auto-apply is hard-disabled (throws)', () => {
  assert.throws(() => applyCandidate(cand({})), /DISABLED|report-only/);
});

// ─── STORM-insight B — decorrelated lenses (security · reversibility · perf) ───

test('lenses — Class A iff ALL three lenses pass', () => {
  const c = cand({ area: 'dev-loop perf config tune', action: 'set ttl = 8 (git-revert)' });
  const lenses = evaluateLenses(c);
  assert.deepEqual(lenses.map((l) => l.lens), ['security', 'reversibility', 'perf']);
  assert.ok(lenses.every((l) => l.pass), 'all pass');
  const r = classify(c);
  assert.equal(r.class, 'A');
  assert.equal(r.lenses.length, 3);
});

test('lenses — a single failing lens demotes to B (security fail)', () => {
  const c = cand({ area: 'rls tenant', action: 'change a tenant-isolation RLS policy' });
  const r = classify(c);
  assert.equal(r.class, 'B');
  const sec = r.lenses.find((l) => l.lens === 'security')!;
  assert.equal(sec.pass, false, 'security lens fails');
  assert.equal(r.lenses.find((l) => l.lens === 'perf')!.pass, true, 'lenses are decorrelated — perf can still pass');
});

test('lenses — reversibility lens fails on irreversible AND on major/breaking dep', () => {
  assert.equal(evaluateLenses(cand({ reversible: false })).find((l) => l.lens === 'reversibility')!.pass, false);
  assert.equal(
    evaluateLenses(cand({ action: 'major breaking upgrade of the runtime' })).find((l) => l.lens === 'reversibility')!.pass,
    false,
  );
});

// ─── EvoMap-insight A — proven-upgrade registry, lens-gated ───

const tmp = () => fs.mkdtempSync(path.join(os.tmpdir(), 'au-'));

test('recordKeptUpgrades — a KEPT Class-A candidate is recorded with metric + revert', () => {
  const dir = tmp();
  // synthetic repo-perf candidate (auto-apply stays OFF; we simulate the KEEP path)
  const c = cand({
    id: 'repo-perf:tune:cacheTtl:300', area: 'perf config tune dev-loop',
    action: 'set config.ts cacheTtl = 300 (mechanical; git-revert)', reversible: true, blast_radius: 'low',
    perf: { repoDir: dir, paths: ['config.ts'] } as Candidate['perf'],
  });
  assert.equal(classify(c).class, 'A', 'precondition: candidate is Class A');
  const outcomes: ApplyOutcome[] = [{ id: c.id, decision: 'kept', reason: 'KEPT', speedup_pct: 12, before: 1000, after: 880 }];
  const genes = recordKeptUpgrades(dir, [c], outcomes, 'T0');
  assert.equal(genes.length, 1);
  const g = readProvenUpgrades(dir)[0]!;
  assert.equal(g.id, 'repo-perf:tune:cacheTtl:300');
  assert.equal(g.speedup_pct, 12);
  assert.equal(g.metric_before, 1000);
  assert.equal(g.metric_after, 880);
  assert.equal(g.revert, 'git checkout -- config.ts', 'revert record persisted');
  assert.match(g.provenance, /lenses pass/);
});

test('recordKeptUpgrades — a lens-failing (firm-boundary) candidate is NOT recorded even if marked kept', () => {
  const dir = tmp();
  // defense-in-depth: a firm-boundary candidate that somehow carries a 'kept'
  // outcome must never become a proven gene (security lens fails).
  const c = cand({ id: 'staged-security:rls', area: 'security rls schema migration', reversible: false, blast_radius: 'high' });
  assert.equal(classify(c).class, 'B', 'classify demotes it to B');
  const outcomes: ApplyOutcome[] = [{ id: c.id, decision: 'kept', reason: 'spoofed kept', speedup_pct: 99, before: 100, after: 1 }];
  const genes = recordKeptUpgrades(dir, [c], outcomes, 'T0');
  assert.equal(genes.length, 0, 'lens-failing change not recorded');
  assert.deepEqual(readProvenUpgrades(dir), [], 'registry stays empty');
});

// ─── Telemetry truthfulness — the zero-cost-row regression ───
// (reflection: 2026-07-02-advisory-arm-revival — "autoupgrade session-telemetry
// collector still emits zero-cost rows"). Root: autoupgrade's private finalize
// path (a) never handed buildRecord a session usage source, (b) resolved t_end
// eagerly at process start so wall_s was frozen at 0, and (c) hardcoded literal
// zeros into its metrics.jsonl line instead of reading record.telemetry the way
// cli.ts finalize does. A NON-EMPTY usage source MUST yield non-zero tokens,
// cost and wall_s in both the run record and the persisted metrics row —
// a zero row on a non-empty source is the regression and must FAIL here.

test('runAutoupgrade + persist — non-empty session source → non-zero tokens/cost/wall_s (zero row = FAIL)', async (t) => {
  // Hermetic: break PATH so child spawns (codeburn npx / git) fail fast — both
  // collectors tolerate that; the run then measures ONLY the session fixture.
  const oldPath = process.env.PATH;
  process.env.PATH = '/nonexistent-au-telemetry-test';
  t.after(() => { process.env.PATH = oldPath; });

  const repo = tmp();
  const base = tmp();
  const tStart = new Date(Date.now() - 5000).toISOString(); // run "began" 5s ago
  const mid = new Date(Date.now() - 2000).toISOString();    // usage inside the window
  const session = path.join(tmp(), 'session.jsonl');
  fs.writeFileSync(session, [
    JSON.stringify({ type: 'assistant', timestamp: mid, message: { model: 'claude-sonnet-4-6', usage: { input_tokens: 1200, output_tokens: 3400 }, content: [] } }),
    JSON.stringify({ type: 'assistant', timestamp: mid, message: { model: 'claude-sonnet-4-6', usage: { input_tokens: 800, output_tokens: 600 }, content: [] } }),
  ].join('\n') + '\n');

  const record = await au.runAutoupgrade({ repoDir: repo, baseDir: base, tStart, apply: false, session });
  assert.equal(record.telemetry.tokens_in, 2000, 'collector must see the session usage (tokens_in)');
  assert.equal(record.telemetry.tokens_out, 4000, 'collector must see the session usage (tokens_out)');
  assert.ok(record.telemetry.cost_usd > 0, 'cost must be derived from measured usage');
  assert.ok(
    Number.isFinite(record.wall_s) && record.wall_s >= 5,
    `wall_s must reflect the run duration (t_end resolved AFTER the work, not at process start); got ${record.wall_s}`,
  );

  // The metrics row must carry the MEASURED telemetry — never hardcoded zeros.
  const line = au.persistAutoupgradeRun(base, record);
  const rows = readMetrics(base, 'autoupgrade');
  assert.equal(rows.length, 1);
  const row = rows[0]!;
  assert.deepEqual(
    { tokens_in: row.tokens_in, tokens_out: row.tokens_out },
    { tokens_in: 2000, tokens_out: 4000 },
    'zero-cost metrics row from a non-empty session source — the regression this test pins',
  );
  assert.ok(row.cost_usd > 0 && row.wall_s >= 5, `metrics row must carry real cost/wall_s; got cost=${row.cost_usd} wall_s=${row.wall_s}`);
  assert.equal(row.cost_usd, record.telemetry.cost_usd, 'metrics row is derived from record.telemetry');
  assert.equal(line.tokens_in, row.tokens_in, 'returned line matches the persisted row');
});

test('runAutoupgrade — no usage source → tokens honestly 0 but wall_s still real', async (t) => {
  const oldPath = process.env.PATH;
  process.env.PATH = '/nonexistent-au-telemetry-test';
  t.after(() => { process.env.PATH = oldPath; });
  const record = await au.runAutoupgrade({
    repoDir: tmp(), baseDir: tmp(), tStart: new Date(Date.now() - 3000).toISOString(), apply: false,
  });
  assert.equal(record.telemetry.tokens_in, 0, 'pure-script run with no source: zero tokens is truthful');
  assert.ok(Number.isFinite(record.wall_s) && record.wall_s >= 3, `wall_s must never freeze at 0; got ${record.wall_s}`);
});

// ─── Run-#5 firm boundary — staged security migration MUST stay Class B ───

test('run-#5 — a staged security migration candidate is Class B (firm boundary holds)', () => {
  // shape mirrors mapStagedSecurity() output for docs/security/*migration* files.
  const c = cand({
    id: 'staged-security:SECURITY-DEFINER-search-path.migration',
    pattern: 'staged security/data change docs/security/SECURITY-DEFINER-search-path.migration',
    source: 'docs/security scan', area: 'security rls schema migration',
    evidence: 'pre-authored migration awaiting DB-owner apply', reversible: false, blast_radius: 'high',
    action: 'queue for human/DB-owner — NEVER autonomously applied',
  });
  const r = classify(c);
  assert.equal(r.class, 'B', 'staged security migration must be Class B (run-#5 behavior)');
  assert.equal(r.lenses.find((l) => l.lens === 'security')!.pass, false);
});
