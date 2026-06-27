import { test } from 'node:test';
import assert from 'node:assert/strict';
import { classify, applyCandidate, type Candidate } from '../src/autoupgrade.js';

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
