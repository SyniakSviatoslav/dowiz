#!/usr/bin/env node
// report-demos-to-telegram — fetch the LIVE demo-storefront data from staging and post a structured
// report to a Telegram channel/topic. Callable + repeatable (just re-runs the fetch + send).
//
// Usage:
//   node scripts/report-demos-to-telegram.mjs [--chat <id>] [--topic <threadId>] [--base <url>] [--dry]
//
// Defaults target the plane-reporting channel topic https://t.me/c/3901655568/13
//   (API chat_id -1003901655568, message_thread_id 13).
// Requires TELEGRAM_BOT_TOKEN in env or ./.env. The bot MUST be an admin/member of the target channel.
// Public business data only (the demo storefronts are already public at /s/<slug>).

import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const arg = (k, d) => {
  const i = process.argv.indexOf(`--${k}`);
  return i >= 0 && process.argv[i + 1] && !process.argv[i + 1].startsWith('--') ? process.argv[i + 1] : d;
};
const has = (k) => process.argv.includes(`--${k}`);

function loadToken() {
  if (process.env.TELEGRAM_BOT_TOKEN) return process.env.TELEGRAM_BOT_TOKEN;
  try {
    const txt = readFileSync(resolve('.env'), 'utf8');
    const m = txt.match(/^\s*TELEGRAM_BOT_TOKEN\s*=\s*(.+)$/m);
    if (m) return m[1].trim().replace(/^["']|["']$/g, '');
  } catch { /* no .env */ }
  return null;
}

const TOKEN = loadToken();
const CHAT = arg('chat', '-1003901655568');
const TOPIC = arg('topic', '13');
const BASE = arg('base', 'https://dowiz-staging.fly.dev');
const DRY = has('dry');
const API = `https://api.telegram.org/bot${TOKEN}`;

// The prebuilt demo storefronts (slug → cuisine label). Data itself is fetched live below.
const DEMOS = [
  ['demo', 'sushi'],
  ['apollonia', 'mediterranean'],
  ['aragosta', 'seafood'],
  ['artepasta', 'italian'],
  ['casa-mia', 'italian'],
  ['dyrrah-mare', 'seafood'],
  ['eljos-pizza', 'pizzeria'],
  ['idua', 'traditional'],
  ['lamuse', 'mediterranean'],
  ['liriada', 'traditional'],
  ['otantik', 'turkish'],
  ['ventus', 'seafood'],
];

const esc = (s) => String(s ?? '').replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function tg(method, body, tries = 4) {
  for (let a = 0; a < tries; a++) {
    const r = await fetch(`${API}/${method}`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(body),
    });
    const j = await r.json();
    if (j.ok) return j.result;
    if (j.error_code === 429 && j.parameters?.retry_after) {
      await sleep((j.parameters.retry_after + 1) * 1000);
      continue;
    }
    throw new Error(`${method}: ${j.error_code} ${j.description}`);
  }
  throw new Error(`${method}: exhausted retries`);
}

async function fetchVenue(slug) {
  try {
    const r = await fetch(`${BASE}/public/locations/${slug}/info`);
    if (!r.ok) return null;
    return await r.json();
  } catch { return null; }
}

// The cold-outreach pitch (Albanian), tailored per venue. Expert-opinion style: empathy first
// (their rating), value = keep 100% profit + data autonomy (no commission), low-pressure ("is this
// useful?" not "buy"), the demo link does the talking, positioned as a developer/partner who adapts
// it. No emojis; one phone screen. Contact stays a single compact sign-off line.
function offer(v) {
  const url = `${BASE}/s/${v.slug}`;
  const ratingClause = v.googleRating
    ? ` — rating-u ${Number(v.googleRating).toFixed(1)} është vërtet mbresëlënës`
    : '';
  return [
    'Tungjatjeta!',
    '',
    `Jam Syniak Sviatoslav, klient i rregullt, dhe gjithmonë e kam vlerësuar cilësinë te ${v.name}${ratingClause}.`,
    '',
    'Si zhvillues softueri, po ndërtoj një sistem porosish që u mundëson restoranteve të mbajnë 100% të fitimit dhe kontrollin e plotë mbi të dhënat e klientëve — pa komisione. Pa asnjë detyrim, e ktheva menunë tuaj në një demo që ta provoni vetë:',
    url,
    '',
    "Nuk po ju shkruaj për t'ju shitur gjë, por thjesht për të marrë mendimin tuaj si profesionist: a mendoni se një sistem i tillë do t'ju vlente? E ndërtoj vetë, ndaj mund ta përshtas plotësisht për nevojat tuaja dhe t'ju mbështes nga dita e parë.",
    '',
    'Gjithë të mirat,',
    'Syniak Sviatoslav',
    '@der_delulu · syniaksviatoslav@proton.me',
  ].join('\n');
}

function venueMessage(v, cuisine, i) {
  const pin = v.lat && v.lng ? `${v.lat}, ${v.lng}` : '—';
  const rating = v.googleRating
    ? `${Number(v.googleRating).toFixed(1)}${v.googleReviewCount ? ` (${v.googleReviewCount} vlerësime)` : ''}`
    : '—';
  const maps = v.googleMapsUrl ? ` · <a href="${esc(v.googleMapsUrl)}">Maps</a>` : '';
  const data = [
    `<b>${i}. ${esc(v.name)}</b> — ${esc(cuisine)}`,
    `Tel: ${esc(v.phone || '—')}`,
    `Pin: <code>${pin}</code>${maps}`,
    `Adresa: ${esc(v.address || '—')}`,
    `Vlerësimi Google: ${rating}`,
    `Storefront: ${BASE}/s/${v.slug}`,
  ].join('\n');
  return `${data}\n\nOferta (shqip) — gati për dërgim:\n<pre>${esc(offer(v))}</pre>`;
}

(async () => {
  if (!TOKEN) { console.error('FATAL: TELEGRAM_BOT_TOKEN not found (env or ./.env)'); process.exit(1); }

  const me = await tg('getMe', {});
  console.error(`[bot] @${me.username} (id ${me.id})`);

  let chat;
  try {
    chat = await tg('getChat', { chat_id: CHAT });
    console.error(`[chat] "${chat.title || chat.id}" type=${chat.type} forum=${chat.is_forum ?? false}`);
  } catch (e) {
    console.error(`FATAL preflight (getChat ${CHAT}): ${e.message}`);
    console.error(`→ add @${me.username} to the channel as an admin (post rights), then re-run.`);
    process.exit(2);
  }

  const venues = [];
  for (const [slug, cuisine] of DEMOS) {
    const v = await fetchVenue(slug);
    if (v && v.name) venues.push([v, cuisine]);
    else console.error(`[skip] ${slug} — not live (404) or no data`);
  }

  const now = new Date().toISOString().replace('T', 'T').slice(0, 16) + 'Z';
  const header =
    `<b>DOWIZ — Demot e gatshme + oferta për secilin</b>\n` +
    `${venues.length} vende · Durrës, Shqipëri · staging · ${now}\n` +
    `Shërbim porosie e dorëzimi PA KOMISION · të dhënat mbeten tuajat · mbështetje e plotë teknike nga dita e parë`;

  const messages = [header, ...venues.map(([v, c], i) => venueMessage(v, c, i + 1))];

  if (DRY) {
    console.error('--- DRY RUN (not sending) ---');
    messages.forEach((m, i) => console.error(`\n===== message ${i + 1}/${messages.length} (${m.length} chars) =====\n${m}`));
    console.log(JSON.stringify({ ok: true, dry: true, venues: venues.length, messages: messages.length }));
    return;
  }

  for (let i = 0; i < messages.length; i++) {
    await tg('sendMessage', {
      chat_id: CHAT,
      message_thread_id: Number(TOPIC),
      text: messages[i],
      parse_mode: 'HTML',
      // Rich link unfurl is now ON (per-venue OG card at /og/:slug.png). Telegram previews the first
      // URL in the message → the storefront card. Header message has no link, so it stays clean.
      disable_web_page_preview: i === 0,
    });
    console.error(`[sent] ${i + 1}/${messages.length} (${messages[i].length} chars) → topic ${TOPIC}`);
    if (i < messages.length - 1) await sleep(700); // gentle pacing to avoid group rate-limit
  }

  console.log(JSON.stringify({
    ok: true, bot: me.username, chat: chat.title || CHAT, topic: TOPIC,
    venues: venues.length, messages: messages.length,
  }));
})().catch((e) => { console.error('FATAL:', e.message); process.exit(1); });
