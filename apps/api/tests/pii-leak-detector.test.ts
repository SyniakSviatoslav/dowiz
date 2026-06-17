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

  await t.test('cross-tenant: two locations PII both flagged in same response', () => {
    const html = `<div id="owner_location_a_123">Tenant A</div><div id="owner_location_b_456">Tenant B</div>`;
    const leaks = detectPiiLeak(html);
    assert.equal(leaks.length, 2);
    assert.ok(leaks.some(l => l === 'owner_location_a_123'));
    assert.ok(leaks.some(l => l === 'owner_location_b_456'));
  });

  await t.test('does not flag user_ safe-words even with trailing content', () => {
    const html = `<span class="user_scalable_zoom">ok</span>`;
    const leaks = detectPiiLeak(html);
    assert.equal(leaks.length, 0);
  });

  await t.test('Albanian phone number in HTML is flagged', () => {
    const html = `<p>Call us at +355 69 123 4567 for support</p>`;
    const leaks = detectPiiLeak(html);
    assert.ok(leaks.length >= 1);
    assert.ok(leaks.some(l => l.includes('+355')));
  });
});
