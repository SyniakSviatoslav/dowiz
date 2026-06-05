import test from 'node:test';
import assert from 'node:assert/strict';
import { detectPiiLeak } from '../src/lib/pii-leak-detector.js';

test('PII Leak Detector', async (t) => {
  await t.test('flags owner_id and customer_id', () => {
    const html = `<html><div id="owner_12345">Hello</div><span data-id="customer_abc"></span></html>`;
    const leaks = detectPiiLeak(html);
    assert.equal(leaks.length, 2);
    assert.equal(leaks[0], 'owner_12345');
    assert.equal(leaks[1], 'customer_abc');
  });

  await t.test('ignores safe words', () => {
    const html = `<html><div style="user-select: none;">Hello</div><span class="user-agent-check"></span></html>`;
    const leaks = detectPiiLeak(html);
    assert.equal(leaks.length, 0);
  });

  await t.test('flags user_id if not safe word', () => {
    const html = `<html><div data-u="user_999">Hello</div></html>`;
    const leaks = detectPiiLeak(html);
    assert.equal(leaks.length, 1);
    assert.equal(leaks[0], 'user_999');
  });
});
