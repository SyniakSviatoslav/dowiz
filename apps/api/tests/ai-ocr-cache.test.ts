import { test } from 'node:test';
import assert from 'node:assert/strict';
import { llmCacheKeyOf, llmCacheGet, llmCacheSet, LLM_CACHE_CAP } from '../src/lib/ai-ocr-parser.js';

// Guardrail (token economy, ADR-0012) — the menu-parser LLM response cache. An identical parse
// (same provider+model+prompt, where the prompt embeds the redacted source) must be served from cache
// so it costs ZERO LLM tokens. Proves: key determinism, hit-after-set / miss, and bounded FIFO eviction.

test('cache key is deterministic and input-sensitive', () => {
  const a = llmCacheKeyOf('openrouter', 'm1', 'PROMPT-X');
  const b = llmCacheKeyOf('openrouter', 'm1', 'PROMPT-X');
  assert.equal(a, b, 'same inputs → same key (a hit is possible)');
  assert.notEqual(a, llmCacheKeyOf('openrouter', 'm1', 'PROMPT-Y'), 'different prompt → different key (no stale serve)');
  assert.notEqual(a, llmCacheKeyOf('zen', 'm1', 'PROMPT-X'), 'different provider → different key');
  assert.notEqual(a, llmCacheKeyOf('openrouter', 'm2', 'PROMPT-X'), 'different model → different key');
});

test('set then get is a hit; unknown key is a miss', () => {
  const k = llmCacheKeyOf('groq', 'mtest', 'unique-prompt-' + 'hit'.repeat(3));
  assert.equal(llmCacheGet(k), undefined, 'cold key misses (would call the LLM)');
  llmCacheSet(k, '{"ok":true}');
  assert.equal(llmCacheGet(k), '{"ok":true}', 'warm key hits → LLM call skipped, 0 tokens');
});

test('cache is bounded — oldest entries evict at the cap (no unbounded growth)', () => {
  const first = llmCacheKeyOf('groq', 'cap', 'evict-first');
  llmCacheSet(first, 'first');
  // Fill well past the cap with distinct keys.
  for (let i = 0; i < LLM_CACHE_CAP + 5; i++) llmCacheSet(llmCacheKeyOf('groq', 'cap', 'k' + i), 'v' + i);
  assert.equal(llmCacheGet(first), undefined, 'the oldest entry was FIFO-evicted past the cap');
});
