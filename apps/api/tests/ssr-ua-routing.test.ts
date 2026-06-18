import test from 'node:test';
import assert from 'node:assert/strict';
import { BOT_UA } from '../src/routes/public/ssr.js';

// /s/:slug serves SSR (JSON-LD/OG) to crawlers + social scrapers, and the SPA
// to real visitors. The regex MUST NOT match normal browsers, or every human
// gets the SSR page instead of the SPA storefront.
test('SSR/SPA user-agent routing', async (t) => {
  await t.test('crawlers + social scrapers → SSR', () => {
    for (const ua of [
      'Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)',
      'facebookexternalhit/1.1 (+http://www.facebook.com/externalhit_uatext.php)',
      'Twitterbot/1.0',
      'Mozilla/5.0 (compatible; bingbot/2.0; +http://www.bing.com/bingbot.htm)',
      'WhatsApp/2.23',
      'TelegramBot (like TwitterBot)',
      'LinkedInBot/1.0',
    ]) {
      assert.equal(BOT_UA.test(ua), true, `expected BOT: ${ua}`);
    }
  });

  await t.test('real browsers → SPA (must NOT match)', () => {
    for (const ua of [
      'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1',
      'Mozilla/5.0 (Linux; Android 13) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120 Mobile Safari/537.36',
      'Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0',
      'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120 Safari/537.36',
      '', // missing UA → treat as human (gets SPA)
    ]) {
      assert.equal(BOT_UA.test(ua), false, `expected HUMAN: ${ua}`);
    }
  });
});
