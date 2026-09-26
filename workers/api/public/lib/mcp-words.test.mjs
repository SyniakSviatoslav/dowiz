// lib/mcp-words.js: the MCP panel's words, the same keys in every language.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mcpWords, LANGS } from './mcp-words.js';

test('every language has every word, filled, and no typographic quote', () => {
  assert.deepEqual(LANGS, ['sq', 'en', 'uk']);
  const keys = Object.keys(mcpWords('en', 'owner')).sort();
  for (const l of LANGS) {
    const w = mcpWords(l, 'staff');
    assert.deepEqual(Object.keys(w).sort(), keys, l);
    for (const [k, v] of Object.entries(w)) {
      const s = typeof v === 'string' ? v : JSON.stringify(v);
      assert.ok(s.length > 0, `${l}.${k}`);
      assert.ok(!/[‘’“”]/.test(s), `${l}.${k} has a typographic quote`);
    }
    assert.deepEqual(Object.keys(w.roleNames), ['owner', 'waiter', 'kitchen', 'counter-manager', 'courier']);
    assert.ok(w.placeholderNote.includes('{k}'), l);
  }
});

test('the hint is the one for whose panel it is; an unknown language is English', () => {
  assert.equal(mcpWords('uk', 'owner').hint, mcpWords('uk', 'owner').hintOwner);
  assert.equal(mcpWords('sq', 'courier').hint, mcpWords('sq', 'courier').hintCourier);
  assert.equal(mcpWords('en', 'staff').hint, mcpWords('en', 'staff').hintStaff);
  assert.equal(mcpWords('en', 'nobody').hint, mcpWords('en').hintStaff);
  assert.equal(mcpWords('de', 'owner').title, 'AI agent (MCP)');
});
