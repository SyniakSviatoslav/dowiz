import test from 'node:test';
import assert from 'node:assert/strict';
import { isBot } from '../src/lib/spa-shell.js';

// /s/:slug serves the SPA to humans and the SSR menu (JSON-LD/OG) to crawlers.
// isBot MUST NOT classify real browsers as bots, or every customer gets the SSR
// menu instead of the interactive SPA storefront.
test('isBot — SPA/SSR routing classifier', async (t) => {
  await t.test('crawlers + social scrapers → bot (SSR)', () => {
    for (const ua of [
      'Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)',
      'facebookexternalhit/1.1 (+http://www.facebook.com/externalhit_uatext.php)',
      'Twitterbot/1.0',
      'Mozilla/5.0 (compatible; bingbot/2.0; +http://www.bing.com/bingbot.htm)',
      'WhatsApp/2.23', 'TelegramBot (like TwitterBot)', 'LinkedInBot/1.0',
    ]) assert.equal(isBot(ua), true, `expected bot: ${ua}`);
  });

  await t.test('real browsers → human (SPA)', () => {
    for (const ua of [
      'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1',
      'Mozilla/5.0 (Linux; Android 13) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120 Mobile Safari/537.36',
      'Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0',
      'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120 Safari/537.36',
    ]) assert.equal(isBot(ua), false, `expected human: ${ua}`);
    assert.equal(isBot(''), false);
    assert.equal(isBot(undefined), false);
  });
});
