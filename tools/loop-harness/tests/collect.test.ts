import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { collectSessionTelemetry, collectGitMem } from '../src/collect.js';

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
});

test('collectSessionTelemetry — cost is computed and non-zero', () => {
  const t = collectSessionTelemetry(fixture(), '2026-06-27T10:00:00Z', '2026-06-27T10:01:00Z');
  assert.ok(t.tokens.cost_usd! > 0);
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

test('collectGitMem — returns a branch + a non-negative RSS sample', () => {
  const g = collectGitMem(process.cwd());
  assert.equal(typeof g.branch, 'string');
  assert.ok(g.branch.length > 0);
  assert.ok(g.rss_peak_mb >= 0);
  assert.equal(g.commits, 0); // no sinceRef → 0
});
