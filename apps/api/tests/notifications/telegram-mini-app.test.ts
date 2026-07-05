import test from 'node:test';
import assert from 'node:assert/strict';
import { buildMiniAppUrl, buildSetChatMenuButtonRequest } from '../../src/notifications/telegram-mini-app.js';

// Guardrail (docs/design/tma-menu-button-wiring/, council-approved 2026-07-04) — RED if:
// - chat_id is ever omittable (B-SEC: an omitted chat_id sets the bot-GLOBAL default menu
//   button for every user of the single shared bot — cross-tenant clobber);
// - the built URL ever points anywhere but the vendor's own /s/:slug;
// - a missing slug silently produces a broken `/s/undefined` or `/s/null` link.

test('buildMiniAppUrl — builds the tenant storefront URL with channel attribution', () => {
  const url = buildMiniAppUrl({ appBaseUrl: 'https://dowiz.fly.dev', slug: 'artepasta' });
  assert.equal(url, 'https://dowiz.fly.dev/s/artepasta?ch=telegram-tma');
});

test('buildMiniAppUrl — strips a trailing slash on appBaseUrl', () => {
  const url = buildMiniAppUrl({ appBaseUrl: 'https://dowiz.fly.dev/', slug: 'artepasta' });
  assert.equal(url, 'https://dowiz.fly.dev/s/artepasta?ch=telegram-tma');
});

test('buildMiniAppUrl — encodes an unsafe slug segment', () => {
  const url = buildMiniAppUrl({ appBaseUrl: 'https://dowiz.fly.dev', slug: 'a b/c' });
  assert.equal(url, 'https://dowiz.fly.dev/s/a%20b%2Fc?ch=telegram-tma');
});

test('buildMiniAppUrl — throws on an empty slug (never link to /s/undefined)', () => {
  assert.throws(() => buildMiniAppUrl({ appBaseUrl: 'https://dowiz.fly.dev', slug: '' }));
});

test('buildSetChatMenuButtonRequest — shape matches Telegram Bot API setChatMenuButton', () => {
  const body = buildSetChatMenuButtonRequest('12345', { appBaseUrl: 'https://dowiz.fly.dev', slug: 'artepasta' });
  assert.deepEqual(body, {
    chat_id: '12345',
    menu_button: {
      type: 'web_app',
      text: 'My Storefront',
      web_app: { url: 'https://dowiz.fly.dev/s/artepasta?ch=telegram-tma' },
    },
  });
});

test('buildSetChatMenuButtonRequest — honors a custom label', () => {
  const body = buildSetChatMenuButtonRequest('12345', { appBaseUrl: 'https://dowiz.fly.dev', slug: 'artepasta', text: 'Order Now' });
  assert.equal(body.menu_button.text, 'Order Now');
});

test('buildSetChatMenuButtonRequest — throws on an empty chatId (never build a bot-global button)', () => {
  assert.throws(() => buildSetChatMenuButtonRequest('', { appBaseUrl: 'https://dowiz.fly.dev', slug: 'artepasta' }));
});
