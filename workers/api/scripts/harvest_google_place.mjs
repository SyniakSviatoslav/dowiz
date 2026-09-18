// The venue's own Google listing, read once with a real browser.
//
// WHY A BROWSER AND NOT THE PLACES API. The API is the supported road and it
// wants a billing key; the operator chose to go without one (2026-09-18). The
// cost of that choice is written here rather than discovered later:
//
//   * This reads Google's RENDERED PAGE. Five runs of the same URL during
//     development produced THREE different DOM variants -- the overview pane
//     sometimes carries reviews and sometimes does not, and the tab strip is
//     sometimes absent entirely. Anything read this way is best-effort.
//   * It is a ONE-OFF HARVEST into the venue's own record, never a proxy on the
//     request path. A storefront that cannot draw until Google answers is a
//     storefront that goes down when Google rate-limits it.
//   * Google's terms govern what may be stored and shown. Displaying this
//     material carries an attribution obligation, which the storefront honours
//     by naming Google as the source and linking to the listing.
//
// WHAT IS RELIABLE AND WHAT IS NOT. Address, coordinates, opening hours, the
// rating and its count come off the place page every time, through handles that
// are NOT localised (`data-item-id`, the URL's own `@lat,lng`) -- the first
// version read English aria-labels and got nothing, because Google served the
// page in Ukrainian whatever `hl` asked for. The individual review texts need
// the `#lrd=` deep link with the listing's CID; without the right CID the page
// loads with an empty h1 and no cards at all.
//
//   node scripts/harvest_google_place.mjs <maps url> --out=place.json
//
// ABSENT IS ABSENT. A field the page does not show is omitted, never guessed:
// this listing publishes no telephone number, and a harvest that invented one
// would send customers to a stranger.
import { chromium } from 'playwright';
import { writeFileSync } from 'node:fs';

const args = process.argv.slice(2);
const URL_IN = args.find(a => a.startsWith('http'));
const OUT = (args.find(a => a.startsWith('--out=')) || '--out=place.json').slice(6);
const MAX = Number((args.find(a => a.startsWith('--reviews=')) || '--reviews=24').slice(10));
if (!URL_IN) {
  console.error('usage: harvest_google_place.mjs <maps url> [--out=place.json] [--reviews=24]');
  process.exit(2);
}

const UA = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 ' +
           '(KHTML, like Gecko) Chrome/124.0 Safari/537.36';

const browser = await chromium.launch({ args: ['--no-sandbox', '--disable-dev-shm-usage'] });
const ctx = await browser.newContext({ viewport: { width: 1280, height: 1600 }, userAgent: UA });
const page = await ctx.newPage();

/** The consent wall stands between Europe and every Google page. */
const consent = async p => {
  for (const label of ['Accept all', 'Прийняти все', 'Pranoji të gjitha', 'Alle akzeptieren']) {
    const b = p.getByRole('button', { name: label }).first();
    if (await b.isVisible().catch(() => false)) { await b.click().catch(() => {}); return; }
  }
};

const place = { source: 'google-maps', harvestedAt: new Date().toISOString(), url: URL_IN };

// ── The place page ─────────────────────────────────────────────────────────
await page.goto(URL_IN, { waitUntil: 'domcontentloaded', timeout: 90_000 });
await consent(page);
await page.waitForTimeout(4000);

place.name = clean(await page.locator('h1').first().textContent().catch(() => null));

place.rating = await page.evaluate(() => {
  const el = document.querySelector('div.F7nice span[aria-hidden="true"]');
  const n = el && Number(el.textContent.replace(',', '.'));
  return Number.isFinite(n) ? n : null;
});
// The count is on the button that opens the review chart; its LABEL is
// localised, its digits are not.
place.reviewCount = await page.evaluate(() => {
  const b = [...document.querySelectorAll('button')]
    .find(x => /reviewChart\.more/.test(x.getAttribute('jsaction') || ''));
  const m = b && /([\d\s .,]+)\s*$/.exec(b.textContent.trim());
  return m ? Number(m[1].replace(/[^\d]/g, '')) || null : null;
});

// ── Opening hours ──────────────────────────────────────────────────────────
// Collapsed, the table shows today alone. The disclosure is found by its
// jsaction, which names the pane rather than the language.
const oh = page.locator('div[jsaction*="pane.openhours"][jsaction*="dropdown"]').first();
if (await oh.count()) { await oh.click().catch(() => {}); await page.waitForTimeout(1600); }
place.hours = await page.evaluate(() => {
  const out = [];
  for (const r of document.querySelectorAll('table tr')) {
    const c = [...r.querySelectorAll('td,th')].map(x => x.textContent.trim()).filter(Boolean);
    // A day and a span of time. Anything else in a table on this page is not
    // opening hours -- the rating histogram is a table too.
    if (c.length >= 2 && /\d/.test(c[1])) out.push({ day: c[0], hours: c[1] });
  }
  return out.length ? out.slice(0, 7) : null;
});

const resolved = page.url();
// The coordinates are in the URL once the map settles, which is later than the
// first paint: the first version read them immediately and always got nothing.
// Read after the hours disclosure below has had its turn, so the map has had
// several more seconds.
// Two patterns, because only one of them is there on any given load: the map
// centre `@lat,lng` and the place's own `!8m2!3d<lat>!4d<lng>`. The first run
// that lost the coordinates had the second pattern and not the first.
const at = /@(-?\d+\.\d+),(-?\d+\.\d+)/.exec(resolved)
  || /!3d(-?\d+\.\d+)!4d(-?\d+\.\d+)/.exec(resolved);
if (at) { place.lat = Number(at[1]); place.lng = Number(at[2]); }

// `data-item-id` is Google's own key for each fact row and is not localised --
// unlike the aria-label, which is whatever language the page came back in.
// GOOGLE'S OWN ICON CHARACTER TRAVELS WITH THE TEXT. Each fact row begins with
// a ligature from Google's icon font -- U+E0C8 for the address pin -- which is
// in the Private Use Area, renders as a box in any other font, and went
// straight into the venue's stored address and onto its storefront. Stripped
// here, where it is obviously foreign, rather than in three renderers.
const clean = t => (t || '')
  .replace(/[\u{E000}-\u{F8FF}\u{F0000}-\u{FFFFD}]/gu, '')
  .replace(/\s+/g, ' ')
  .trim() || null;

const item = async id => {
  const el = page.locator(`[data-item-id="${id}"], [data-item-id^="${id}:"]`).first();
  return (await el.count()) ? clean(await el.textContent().catch(() => null)) : null;
};
place.address = await item('address');
place.website = await item('authority');
place.plusCode = await item('oloc');
place.phone = await item('phone');

// The listing's CID, which the reviews deep link needs and nothing else gives.
const cid = /!1s(0x[0-9a-f]+):(0x[0-9a-f]+)/.exec(resolved);
if (cid) {
  place.featureId = `${cid[1]}:${cid[2]}`;
  place.cid = BigInt(cid[2]).toString();
}


// ── Reviews ────────────────────────────────────────────────────────────────
// THE DEEP LINK, NOT THE TAB. The tab strip is absent in some variants and the
// overview pane carries reviews in others; `?cid=<decimal>#lrd=<featureId>`
// opens the reviews dialog in every variant seen. A WRONG cid does not error --
// it loads a page with an empty h1 and no cards, which is why the h1 is checked.
place.reviews = [];
if (place.cid && place.featureId) {
  const rp = await ctx.newPage();
  await rp.goto(`https://www.google.com/maps?cid=${place.cid}#lrd=${place.featureId},1,,,`,
    { waitUntil: 'domcontentloaded', timeout: 90_000 });
  await consent(rp);
  await rp.waitForTimeout(6000);
  const named = await rp.evaluate(() => (document.querySelector('h1') || {}).textContent || '');
  if (!named.trim()) console.log('reviews: the dialog loaded nameless — the cid is probably wrong');

  // THE WHEEL SCROLLS THE PAGE, NOT THE LIST. The reviews sit in a virtualised
  // box inside the pane, and wheeling over `div[role=main]` moved nothing: the
  // dialog held three reviews of seventy-six and stayed there through twelve
  // turns of the wheel. The box is found by measurement -- the tallest element
  // whose content overflows it -- and driven by its own scrollTop.
  let seen = 0;
  for (let i = 0; i < 40; i++) {
    const n = await rp.evaluate(() => new Set(
      [...document.querySelectorAll('[data-review-id]')]
        .map(e => e.getAttribute('data-review-id'))).size);
    if (n >= MAX) break;
    if (n === seen && i > 6) break;          // the list has stopped growing
    seen = n;
    await rp.evaluate(() => {
      // THE BOX IS THE ONE THAT OVERFLOWS MOST, not the one that is tallest.
      // Sorting by clientHeight picked a 1600px container that does not scroll
      // at all and left the 1860px-overflowing review list untouched.
      const box = [...document.querySelectorAll('div')]
        .filter(d => d.scrollHeight > d.clientHeight + 200 && d.clientHeight > 300)
        .sort((a, b) => (b.scrollHeight - b.clientHeight) - (a.scrollHeight - a.clientHeight))[0];
      // A STEP, NOT A JUMP. The list is virtualised: sending scrollTop to the
      // bottom asks for rows that have not been fetched, and the box comes back
      // to where it was.
      if (box) box.scrollTop = Math.min(box.scrollHeight, box.scrollTop + box.clientHeight * 0.9);
      else window.scrollTo(0, document.body.scrollHeight);
    });
    await rp.waitForTimeout(1200);
  }
  // A truncated review is not a review. The expander is one jsaction.
  for (const b of await rp.locator('button[jsaction*="review.expandReview"]').all()) {
    await b.click().catch(() => {});
  }
  await rp.waitForTimeout(600);

  place.reviews = await rp.evaluate(max => {
    const out = [];
    for (const card of document.querySelectorAll('[data-review-id]')) {
      const id = card.getAttribute('data-review-id');
      if (!id || out.some(r => r.id === id)) continue;
      const txt = card.innerText || '';
      const stars = card.querySelector('span[role="img"][aria-label]');
      const rating = stars ? Number((/(\d)/.exec(stars.getAttribute('aria-label')) || [])[1]) || null : null;
      // The reviewer link is a SIBLING button in some variants, so the first
      // line of the card is the fallback: that line is the name in every
      // variant seen, and an empty author made three of three reviews
      // anonymous.
      const author = ((card.querySelector('[jsaction*="reviewerLink"]')?.innerText || '')
        .split('\n')[0].trim()) || (txt.split('\n').map(x => x.trim()).find(Boolean) || null);
      // The body is the longest line that is not the author, the badge line or
      // the "N months ago" stamp: those are short and the review is not.
      const body = txt.split('\n').map(s => s.trim())
        .filter(s => s.length > 40 && s !== author)
        .sort((a, b2) => b2.length - a.length)[0] || null;
      if (!author && !body) continue;
      out.push({ id, author, rating, text: body });
      if (out.length >= max) break;
    }
    return out;
  }, MAX);
  await rp.close();
}


// ── The schedule, in the shape the kernel already reads ────────────────────
//
// `dowiz_hub::hours` wants seven arrays of minute windows, MONDAY FIRST. Google
// prints day names in whatever language it decided to serve, starting on
// today. The mapping happens HERE, where the source language is in front of us,
// and not in the Worker: a Worker guessing at weekday names in an unknown
// locale would open the venue on the wrong day.
//
// A day that cannot be identified is LEFT OUT rather than guessed, and a
// schedule missing days is not written at all -- `hours` absent means "open
// whenever", which is what every venue already had; `hours` wrong means a
// customer at a locked door.
const DAYS = [
  ['monday', 'понеділок', 'e hënë', 'montag', 'lunedì'],
  ['tuesday', 'вівторок', 'e martë', 'dienstag', 'martedì'],
  ['wednesday', 'середа', 'e mërkurë', 'mittwoch', 'mercoledì'],
  ['thursday', 'четвер', 'e enjte', 'donnerstag', 'giovedì'],
  ['friday', 'пʼятниця', "п'ятниця", 'e premte', 'freitag', 'venerdì'],
  ['saturday', 'субота', 'e shtunë', 'samstag', 'sabato'],
  ['sunday', 'неділя', 'e diel', 'sonntag', 'domenica'],
];
const CLOSED = ['closed', 'зачинено', 'закрито', 'mbyllur', 'geschlossen'];
const ALLDAY = ['24 hours', 'open 24', 'цілодобово', '24 години'];

const dayIndex = label => {
  const l = (label || '').toLowerCase().trim();
  return DAYS.findIndex(names => names.some(n => l.startsWith(n) || l === n));
};
const minutes = hhmm => {
  const m = /(\d{1,2})[:.](\d{2})/.exec(hhmm);
  return m ? Number(m[1]) * 60 + Number(m[2]) : null;
};

if (place.hours) {
  const week = [[], [], [], [], [], [], []];
  let known = 0;
  for (const row of place.hours) {
    const i = dayIndex(row.day);
    if (i < 0) continue;
    const text = (row.hours || '').toLowerCase();
    known++;
    if (CLOSED.some(c => text.includes(c))) continue;            // shut: no windows
    if (ALLDAY.some(c => text.includes(c))) { week[i].push({ open: 0, close: 1440 }); continue; }
    // One row can hold two services: "11:00–15:00, 18:00–23:00".
    for (const span of text.split(',')) {
      const parts = span.split(/[–—-]/).map(x => x.trim()).filter(Boolean);
      if (parts.length < 2) continue;
      const open = minutes(parts[0]);
      let close = minutes(parts[1]);
      if (open === null || close === null || open === close) continue;
      // Past midnight: the kernel's Window handles a close BEFORE an open, and
      // 00:00 as a close means the end of the day, not the start of it.
      if (close === 0) close = 1440;
      week[i].push({ open, close });
    }
  }
  if (known === 7) place.hoursMinutes = week;
  else console.log(`hours: only ${known} of 7 days were recognised — the schedule is NOT normalised`);
}

await browser.close();

for (const k of Object.keys(place)) {
  if (place[k] === null || place[k] === '' || (Array.isArray(place[k]) && !place[k].length)) delete place[k];
}
writeFileSync(OUT, JSON.stringify(place, null, 1));

const say = (k, v) => `${k} ${v || 'MISSING'}`;
console.log(`place: ${place.name || '(no name)'}`);
console.log('  ' + [
  say('address', place.address && 'yes'),
  say('phone', place.phone && 'yes'),
  say('coords', place.lat && `${place.lat},${place.lng}`),
].join('  '));
console.log('  ' + [
  say('hours', place.hours && place.hours.length + ' days'),
  say('rating', place.rating && `${place.rating} (${place.reviewCount ?? '?'})`),
  say('reviews', place.reviews && place.reviews.length),
].join('  '));
console.log(`→ ${OUT}`);
