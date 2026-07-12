import test from 'node:test';
import assert from 'node:assert/strict';
import {
  isBot,
  escapeHtml,
  buildTenantMeta,
  injectTenantMeta,
  serveSpaShell,
} from '../src/lib/spa-shell.js';

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

  // Every BOT_UA alternation must be exercised — an un-tested pattern can silently
  // rot (regex edit) and route a real crawler to the SPA, killing its SEO unfurl.
  await t.test('remaining BOT_UA patterns → bot (SSR)', () => {
    for (const ua of [
      'Mozilla/5.0 (compatible; AhrefsBot/7.0; +http://ahrefs.com/robot/)',
      'Mozilla/5.0 (compatible; SemrushBot/7~bl; +http://www.semrush.com/bot.html)',
      'Mozilla/5.0 (compatible; PetalBot;+https://webmaster.petalsearch.com/site/petalbot)',
      'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_5) AppleWebKit/600 (Applebot/0.1)',
      'Mozilla/5.0 (compatible; DuckDuckBot/1.1; +http://duckduckgo.com/duckduckbot.html)',
      'Mozilla/5.0 (compatible; Baiduspider/2.0; +http://www.baidu.com/search/spider.html)',
      'Mozilla/5.0 (compatible; YandexBot/3.0; +http://yandex.com/bots)',
      'Mozilla/5.0 (compatible; Discordbot/2.0; +https://discordapp.com)',
      'Slackbot-LinkExpanding 1.0 (+https://api.slack.com/robots)',
      'Pinterest/0.2 (+http://www.pinterest.com/bot.html)',
      'Quora Link Preview/1.0',
      'Embedly/0.2',
      'Mozilla/5.0 (compatible; Mediapartners-Google/2.1; +http://www.google.com/bot.html)',
      'Mozilla/5.0 (compatible; Yahoo! Slurp; http://help.yahoo.com/help/us/ysearch/slurp)',
      'Mozilla/5.0 (compatible; spider)',
      'Mozilla/5.0 (compatible; crawler)',
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

// escapeHtml is the only XSS barrier for the hand-templated SPA shell meta block:
// tenant name/address/logoUrl come from owner-controlled DB rows and are interpolated
// into <title>/attribute contexts. A regression here = stored XSS on every storefront.
test('escapeHtml — neutralises all five HTML-significant chars', () => {
  // '&' must be replaced first or it double-escapes the entity-prefixes below.
  assert.equal(escapeHtml(`&<>"'`), '&amp;&lt;&gt;&quot;&#39;');
  assert.equal(escapeHtml('plain text'), 'plain text');
});

test('buildTenantMeta — escapes injected name/address (no raw markup escapes the tag)', () => {
  const meta = buildTenantMeta({
    name: '<script>alert(1)</script>',
    address: '" onerror="x',
    logoUrl: 'https://cdn.example.com/l.png?a=1&b=2',
    slug: 'evil',
  });
  // The script payload survives only in escaped form — never as an executable element.
  assert.ok(!meta.includes('<script>alert(1)</script>'), 'raw <script> must not appear');
  assert.ok(meta.includes('&lt;script&gt;alert(1)&lt;/script&gt;'), 'name must be entity-escaped');
  // The address quote must not be able to break out of the content="..." attribute.
  assert.ok(!meta.includes('content="" onerror="x'), 'attribute breakout must not appear');
  assert.ok(meta.includes('&quot; onerror=&quot;x'), 'address quotes must be entity-escaped');
  // logoUrl present → og:image emitted, with & escaped.
  assert.ok(meta.includes('property="og:image" content="https://cdn.example.com/l.png?a=1&amp;b=2"'));
});

test('buildTenantMeta — omits og:image when no logoUrl', () => {
  const meta = buildTenantMeta({ name: 'Cafe', address: null, logoUrl: null, slug: 'cafe' });
  assert.ok(!meta.includes('og:image'), 'og:image must be absent without a logo');
  assert.ok(meta.includes('<title>Cafe — Order Online | Dowiz</title>'));
});

test('injectTenantMeta — replaces the static title marker, else inserts before </head>', () => {
  const replaced = injectTenantMeta('<head><title>Dowiz</title></head>', '<title>X</title>');
  assert.equal(replaced, '<head><title>X</title></head>');
  // Fallback path: marker absent → meta inserted before </head>, SPA preserved.
  const inserted = injectTenantMeta('<head><meta charset="utf-8"></head>', '<title>X</title>');
  assert.ok(inserted.includes('<title>X</title>'), 'meta must be injected');
  assert.ok(inserted.includes('</head>'), '</head> must be preserved');
  assert.ok(inserted.indexOf('<title>X</title>') < inserted.indexOf('</head>'), 'meta must precede </head>');
});

// serveSpaShell is the exported handler the /s/:slug route invokes. The shell HTML
// file is absent in unit context, so the tenant-found inject path throws and falls
// back to sendFile — exactly the production safety net we assert here. The CSP and
// frame-ancestors headers are set BEFORE that branch, so they are asserted directly.
function makeReply() {
  const headers: Record<string, string> = {};
  const calls = { sendFile: '', sentHtml: '', type: '', removedXFO: false };
  const reply: any = {
    header(k: string, v: string) { headers[k] = v; return reply; },
    type(t: string) { calls.type = t; return { send(h: string) { calls.sentHtml = h; return h; } }; },
    sendFile(f: string) { calls.sendFile = f; return f; },
    raw: { removeHeader(n: string) { if (n === 'X-Frame-Options') calls.removedXFO = true; } },
  };
  return { reply, headers, calls };
}
const makeDb = (rows: any[], err?: Error) => ({
  async query() { if (err) throw err; return { rows }; },
});

test('serveSpaShell — sets SPA CSP + no-store, default frame-ancestors self', async () => {
  const { reply, headers, calls } = makeReply();
  await serveSpaShell(reply, makeDb([]), 'no-such-slug');
  assert.match(headers['Content-Security-Policy'], /frame-ancestors 'self'$/);
  assert.ok(headers['Content-Security-Policy'].includes("script-src 'self' 'unsafe-inline' 'unsafe-eval'"));
  assert.equal(headers['Cache-Control'], 'no-cache, no-store, must-revalidate');
  assert.equal(calls.removedXFO, false, 'X-Frame-Options must stay for self-only embed');
  // No tenant row → straight to the static shell, deterministically.
  assert.equal(calls.sendFile, 'index.html');
});

test('serveSpaShell — DB error is caught and falls back to the static shell', async () => {
  const { reply, headers, calls } = makeReply();
  await serveSpaShell(reply, makeDb([], new Error('connection timeout')), 'boom');
  assert.equal(calls.sendFile, 'index.html');
  assert.match(headers['Content-Security-Policy'], /frame-ancestors 'self'$/);
});

test('serveSpaShell — theme frame_ancestors widens CSP and drops X-Frame-Options', async () => {
  const { reply, headers, calls } = makeReply();
  await serveSpaShell(
    reply,
    makeDb([{ name: 'Cafe', address: null, frame_ancestors: ['https://a.com', 'https://b.com'], logo_url: null }]),
    'cafe',
  );
  assert.ok(
    headers['Content-Security-Policy'].endsWith('frame-ancestors https://a.com https://b.com'),
    `expected widened frame-ancestors, got: ${headers['Content-Security-Policy']}`,
  );
  assert.equal(calls.removedXFO, true, 'embed permitted → X-Frame-Options must be removed');
  // Shell HTML is absent in unit context → inject path throws → static-shell fallback.
  assert.equal(calls.sendFile, 'index.html');
});
