import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { classify, applyCandidate, buildHooks, evaluateClassA, type Candidate } from '../src/autoupgrade.js';
import type { RepoPerfSpec } from '../src/repo-apply.js';

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

// ─── §0 trusted-source gate: buildHooks must NEVER hand the oracle a candidate
//      derived from an untrusted (web/LLM) source, even if it is shaped repo-perf. ───

// fields are never executed on the untrusted path (isTrustedSource returns before
// makeRepoHooks), so a structural stub is sufficient to satisfy the type.
const perfStub = { repoDir: '/dev/null/x', paths: [] } as unknown as RepoPerfSpec;

test('buildHooks — untrusted source returns skip (no oracle hooks for web/LLM-derived patch)', () => {
  const r = buildHooks(cand({ id: 'repo-perf:x', source: 'web research / LLM patch', perf: perfStub }));
  assert.ok('skip' in r, 'untrusted repo-perf must NOT yield runnable oracle hooks');
  assert.match((r as { skip: string }).skip, /untrusted source/);
});

test('buildHooks — control: a non-repo-perf candidate skips for its own reason, not the untrusted gate', () => {
  // proves the untrusted-source skip is specific to repo-perf, not a blanket skip
  const r = buildHooks(cand({ id: 'ghost-mcp:claude_ai_Notion', source: 'codeburn optimize' }));
  assert.ok('skip' in r);
  assert.doesNotMatch((r as { skip: string }).skip, /untrusted source/);
  assert.match((r as { skip: string }).skip, /account-managed MCP/);
});

// ─── §4 credential isolation: the apply path must refuse when prod secrets are in
//      the environment (a compromised step then has nothing to exfiltrate). ───

const classACand = () => cand({ id: 'ghost-mcp:x', area: 'dev-loop mcp config' });

test('evaluateClassA — prod credential in env blocks apply (§4 containment)', async () => {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), 'autoupg-'));
  // synthetic (non-real) secret-shaped var — never a live credential
  const out = await evaluateClassA([classACand()], true, base, { DATABASE_URL: 'postgres://synthetic/test' });
  assert.equal(out.length, 1);
  assert.equal(out[0]!.decision, 'skipped');
  assert.match(out[0]!.reason, /CONTAINMENT.*DATABASE_URL/);
});

test('evaluateClassA — isolated env: containment does NOT fire, candidate proceeds to its own skip (control)', async () => {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), 'autoupg-'));
  const out = await evaluateClassA([classACand()], true, base, {}); // no secret-shaped vars
  assert.equal(out[0]!.decision, 'skipped');
  assert.doesNotMatch(out[0]!.reason, /CONTAINMENT/);
  assert.match(out[0]!.reason, /account-managed MCP/); // reached per-candidate buildHooks, not the blanket gate
});

// known-bug (ESCALATE — red-line auth boundary): AREA_BOUNDARY is `\bauth\b`, which
// does NOT match the compound tags 'authentication'/'authorization'/'authz'. A producer
// emitting a compound auth tag would BYPASS the firm boundary → Class A (auto-eligible).
// This guardrail is RED until the regex is widened. The fix touches the auth-classification
// red-line, so it is escalated, NOT self-applied here. Do not weaken/remove this assertion.
for (const area of ['authentication', 'authorization', 'authz']) {
  test(`classify — compound auth area '${area}' must still be Class B (firm boundary, no TEXT_BOUNDARY assist)`, () => {
    // action/pattern are boundary-free so ONLY the area tag is under test
    const r = classify(cand({ area, action: 'edit a request handler', pattern: 'refactor', reversible: true, blast_radius: 'low' }));
    assert.equal(r.class, 'B', `${area} must be B (firm boundary); got ${r.class} (${r.reason})`);
  });
}
