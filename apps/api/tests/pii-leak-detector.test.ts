import test from 'node:test';
import assert from 'node:assert/strict';
import { detectPiiLeak } from '../src/lib/pii-leak-detector.js';

// detectPiiLeak returns structured PiiLeakResult[] ({source,pattern,value,context});
// assert against the .value field (the matched token).
test('PII Leak Detector', async (t) => {
  await t.test('flags owner_id and customer_id', () => {
    const html = `<html><div id="owner_12345">Hello</div><span data-id="customer_abc"></span></html>`;
    const leaks = detectPiiLeak(html);
    assert.equal(leaks.length, 2);
    assert.equal(leaks[0].value, 'owner_12345');
    assert.equal(leaks[1].value, 'customer_abc');
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
    assert.equal(leaks[0].value, 'user_999');
  });

  // [CRITICAL] Module-level /g PII_VALUE_PATTERNS share lastIndex across calls.
  // A second call on a PII-matching string must NOT silently miss the leading
  // match because exec() resumed mid-string. Both calls must be identical.
  await t.test('resets regex lastIndex between calls (no cross-call drift)', () => {
    const html = `<span>foo@bar.com</span>`;
    const first = detectPiiLeak(html);
    const second = detectPiiLeak(html);
    assert.equal(first.length, 1);
    assert.equal(second.length, first.length);
    assert.equal(second[0].source, 'string_value');
    assert.equal(second[0].value, 'foo@bar.com');
  });

  // [HIGH] JSON scan path: dangerous-key scanner + JSON value scanner + whole-string scan.
  await t.test('flags dangerous JSON keys and masks their values', () => {
    const json = JSON.stringify({ email: 'foo@bar.com', password_hash: 'abc12345678' });
    const leaks = detectPiiLeak(json);
    assert.equal(leaks.length, 4);
    const keyLeaks = leaks.filter((l) => l.source === 'json_key');
    assert.equal(keyLeaks.length, 2);
    // password_hash value is masked, never emitted raw
    const pw = keyLeaks.find((l) => l.value.startsWith('password_hash:'));
    assert.ok(pw, 'password_hash key flagged');
    assert.equal(pw!.value, 'password_hash: abc1...5678');
    assert.ok(!pw!.value.includes('abc12345678'));
    // value scanner catches the email inside the JSON string
    assert.ok(leaks.some((l) => l.source === 'json_value' && l.value === 'email: foo@bar.com'));
  });

  // [HIGH] val.length > 4 threshold boundary for the dangerous-key scanner.
  await t.test('respects the val.length > 4 boundary on dangerous keys', () => {
    assert.equal(detectPiiLeak(JSON.stringify({ email: 'ab' })).length, 0); // 2 chars, skipped
    assert.equal(detectPiiLeak(JSON.stringify({ email: 'abcd' })).length, 0); // 4 chars, skipped
    const over = detectPiiLeak(JSON.stringify({ email: 'abcde' })); // 5 chars, caught
    assert.equal(over.length, 1);
    assert.equal(over[0].source, 'json_key');
  });

  // [HIGH] PII_VALUE_PATTERNS (phone / email / address) must actually match.
  await t.test('matches Albanian phone, email and address value patterns', () => {
    const phone = detectPiiLeak(`<span>+355 69 123 4567</span>`);
    assert.equal(phone.length, 1);
    assert.equal(phone[0].source, 'string_value');
    assert.equal(phone[0].value, '+355 69 123 4567');

    const email = detectPiiLeak(`<span>foo@bar.com</span>`);
    assert.equal(email.length, 1);
    assert.equal(email[0].value, 'foo@bar.com');

    const addr = detectPiiLeak(`<span>Rruga Myslym</span>`);
    assert.equal(addr.length, 1);
    assert.equal(addr[0].value, 'Rruga Myslym');
  });
});
