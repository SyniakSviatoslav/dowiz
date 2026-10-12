// The invite send's pure parts, in node (W-INVITE).
// `node --test workers/api/public/admin/invite-send-view.test.mjs`
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { railOptions, inviteBody, deliveryLine } from './invite-send-view.js';
import { WORDS } from './invite-send-words.js';

const t = k => WORDS.en[k] ?? k;

test('no rail set up offers no send, and a set-up rail is offered beside sharing it yourself', () => {
  assert.deepEqual(railOptions({ whatsapp: false, telegram: false }), []);
  assert.deepEqual(railOptions(undefined), [], 'an unreadable list offers nothing to send on');
  assert.deepEqual(railOptions({ whatsapp: true }).map(o => o.value), ['', 'whatsapp']);
  assert.deepEqual(railOptions({ whatsapp: true, telegram: true }).map(o => o.value), ['', 'whatsapp', 'telegram']);
});

test('the body names the rail only when one was chosen, and the chat only for telegram', () => {
  assert.deepEqual(inviteBody({ phone: ' +355 69 ', name: ' Ana ', send: '', telegram: '1' }), { phone: '+355 69', name: 'Ana' });
  assert.deepEqual(inviteBody({ phone: '1', name: 'A', send: 'whatsapp', telegram: '9', lang: 'uk' }), { phone: '1', name: 'A', send: 'whatsapp', lang: 'uk' });
  assert.deepEqual(inviteBody({ phone: '1', name: 'A', send: 'telegram', telegram: ' 42 ' }), { phone: '1', name: 'A', send: 'telegram', lang: 'sq', telegram: '42' });
});

test('every delivery state reads as words with a tone, and no delivery reads as nothing', () => {
  assert.equal(deliveryLine(null, t), null);
  assert.equal(deliveryLine({}, t), null);
  assert.deepEqual(deliveryLine({ via: 'whatsapp', state: 'queued' }, t), ['info', 'WhatsApp: queued to send']);
  assert.deepEqual(deliveryLine({ via: 'whatsapp', state: 'sent' }, t), ['ok', 'handed to WhatsApp']);
  assert.deepEqual(deliveryLine({ via: 'telegram', state: 'failed' }, t), ['bad', 'Telegram: not delivered']);
  assert.deepEqual(deliveryLine({ via: 'whatsapp', state: 'parked' }, t)[0], 'warn');
  assert.equal(deliveryLine({ via: 'whatsapp', state: 'noRail' }, t)[1], WORDS.en.invs_noRail);
  for (const s of ['queued', 'retrying', 'parked', 'sent', 'failed', 'unknown', 'noRail', 'noLink', 'error'])
    for (const l of Object.keys(WORDS)) assert.ok(WORDS[l]['invs_' + s], `${l} has words for ${s}`);
});
