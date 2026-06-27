import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { collectSessionTelemetry, collectGitMem, collectWorkflowTelemetry, mergeTelemetry } from '../src/collect.js';

// A minimal session JSONL fixture in the real Claude Code shape.
function fixture(): string {
  const lines = [
    // inside window — opus assistant turn with usage + a tool_use + an Agent
    { type: 'assistant', timestamp: '2026-06-27T10:00:30Z', message: { model: 'claude-opus-4-8', usage: { input_tokens: 1000, output_tokens: 200, cache_read_input_tokens: 5000, cache_creation_input_tokens: 0 }, content: [
      { type: 'text', text: 'hi' },
      { type: 'tool_use', name: 'mcp__playwright-test__browser_navigate', input: {} },
      { type: 'tool_use', name: 'Agent', input: { subagent_type: 'Explore' } },
    ] } },
    // inside window — haiku turn
    { type: 'assistant', timestamp: '2026-06-27T10:00:45Z', message: { model: 'claude-haiku-4-5', usage: { input_tokens: 500, output_tokens: 50, cache_read_input_tokens: 0 }, content: [] } },
    // OUTSIDE window (too late) — must be excluded
    { type: 'assistant', timestamp: '2026-06-27T11:00:00Z', message: { model: 'claude-opus-4-8', usage: { input_tokens: 9999, output_tokens: 9999 }, content: [] } },
    // a user line — ignored
    { type: 'user', timestamp: '2026-06-27T10:00:31Z', message: { role: 'user', content: 'x' } },
  ];
  const f = path.join(fs.mkdtempSync(path.join(os.tmpdir(), 'sess-')), 's.jsonl');
  fs.writeFileSync(f, lines.map((l) => JSON.stringify(l)).join('\n') + '\n');
  return f;
}

test('collectSessionTelemetry — sums tokens by model within the time window only', () => {
  const t = collectSessionTelemetry(fixture(), '2026-06-27T10:00:00Z', '2026-06-27T10:01:00Z');
  // opus 1000+200+5000 + haiku 500+50 ; the 11:00 opus line excluded
  assert.equal(t.tokens.in, 1500);
  assert.equal(t.tokens.out, 250);
  assert.equal(t.tokens.cache_read, 5000);
  // tokensByModel = COMPUTE tokens (in+out) for eco — cache-read excluded
  assert.equal(t.tokensByModel['claude-opus-4-8'], 1200);
  assert.equal(t.tokensByModel['claude-haiku-4-5'], 550);
  // display by_model keeps the full total (incl. cache)
  assert.equal(t.tokens.by_model!['claude-opus-4-8'], 6200);
  // haiku display total = 500+50 (no cache) — guards against a dropped by_model entry
  assert.equal(t.tokens.by_model!['claude-haiku-4-5'], 550);
});

test('collectSessionTelemetry — cost is the exact rounded sum', () => {
  const t = collectSessionTelemetry(fixture(), '2026-06-27T10:00:00Z', '2026-06-27T10:01:00Z');
  // opus = (1000/1e6)*15 + (200/1e6)*75 + (5000/1e6)*1.5 = 0.0375
  // haiku = (500/1e6)*0.8 + (50/1e6)*4 = 0.0006 ; total 0.0381 → rounded(2dp) 0.04
  assert.equal(t.tokens.cost_usd, 0.04);
});

test('collectSessionTelemetry — extracts MCP skills + agents from tool_use blocks', () => {
  const t = collectSessionTelemetry(fixture(), '2026-06-27T10:00:00Z', '2026-06-27T10:01:00Z');
  assert.equal(t.skills_used['playwright-test'], 1);
  assert.equal(t.agents['Explore'], 1);
});

test('collectSessionTelemetry — missing file returns empty (no throw)', () => {
  const t = collectSessionTelemetry('/no/such/file.jsonl', 'a', 'b');
  assert.equal(t.tokens.in, 0);
});

// A workflow transcript dir fixture: agent-*.jsonl in the subagent message shape
// ({message:{usage,model,content}}) — no timestamp window (whole transcript counts).
function wfFixture(): string {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'wf-'));
  const sub = path.join(dir, 'wf_x'); fs.mkdirSync(sub); // nested → walk must recurse
  const a = [
    { message: { model: 'claude-sonnet-4-6', usage: { input_tokens: 100, output_tokens: 2000, cache_read_input_tokens: 30000, cache_creation_input_tokens: 8000 }, content: [{ type: 'tool_use', name: 'Read', input: {} }] } },
  ];
  const b = [
    { message: { model: 'claude-sonnet-4-6', usage: { input_tokens: 50, output_tokens: 1000, cache_read_input_tokens: 10000, cache_creation_input_tokens: 0 }, content: [] } },
    { message: { model: 'claude-opus-4-8', usage: { input_tokens: 200, output_tokens: 500, cache_read_input_tokens: 0, cache_creation_input_tokens: 1000 }, content: [] } },
  ];
  fs.writeFileSync(path.join(sub, 'agent-a1.jsonl'), a.map((l) => JSON.stringify(l)).join('\n') + '\n');
  fs.writeFileSync(path.join(sub, 'agent-a2.jsonl'), b.map((l) => JSON.stringify(l)).join('\n') + '\n');
  fs.writeFileSync(path.join(sub, 'agent-a1.meta.json'), '{}'); // non-jsonl → ignored
  return dir;
}

test('collectWorkflowTelemetry — aggregates subagent transcripts (recursive) incl. cache-write', () => {
  const t = collectWorkflowTelemetry(wfFixture());
  assert.equal(t.tokens.in, 350);   // 100+50+200
  assert.equal(t.tokens.out, 3500); // 2000+1000+500
  assert.equal(t.tokens.cache_read, 40000);  // 30000+10000
  assert.equal(t.tokens.cache_write, 9000);  // 8000+1000
  // compute (in+out) per model for eco — cache excluded
  assert.equal(t.tokensByModel['claude-sonnet-4-6'], 3150); // 100+2000+50+1000
  assert.equal(t.tokensByModel['claude-opus-4-8'], 700);    // 200+500
  assert.ok(t.tokens.cost_usd! > 0);
  assert.equal(t.skills_used['Read'] ?? 0, 0); // Read is not an MCP/Skill → not counted
});

test('collectWorkflowTelemetry — missing dir returns empty (no throw)', () => {
  assert.equal(collectWorkflowTelemetry('/no/such/dir').tokens.in, 0);
});

test('mergeTelemetry — sums session + workflow blocks (tokens, cache, by-model, cost)', () => {
  const sess = collectSessionTelemetry(fixture(), '2026-06-27T10:00:00Z', '2026-06-27T10:01:00Z');
  const wf = collectWorkflowTelemetry(wfFixture());
  const m = mergeTelemetry(sess, wf);
  assert.equal(m.tokens.in, sess.tokens.in! + wf.tokens.in!);
  assert.equal(m.tokens.out, sess.tokens.out! + wf.tokens.out!);
  assert.equal(m.tokens.cache_write, wf.tokens.cache_write); // session fixture has 0
  assert.equal(m.tokensByModel['claude-sonnet-4-6'], 3150);
  assert.equal(m.tokensByModel['claude-opus-4-8'], sess.tokensByModel['claude-opus-4-8']! + 700);
  assert.ok(m.tokens.cost_usd! >= sess.tokens.cost_usd! + wf.tokens.cost_usd! - 0.01);
});

test('mergeTelemetry — tolerates nulls', () => {
  const m = mergeTelemetry(null, undefined);
  assert.equal(m.tokens.in, 0);
});

test('collectGitMem — returns a branch + a non-negative RSS sample', () => {
  const g = collectGitMem(process.cwd());
  assert.equal(typeof g.branch, 'string');
  assert.ok(g.branch.length > 0);
  assert.ok(g.rss_peak_mb >= 0);
  assert.equal(g.commits, 0); // no sinceRef → 0
});
