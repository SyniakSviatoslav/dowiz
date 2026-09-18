// Act 1 -- ARRIVE. The venue as a place, over the Sea.
//
// The hero is the establishing shot, and it is composed the way the venue's
// own mark is composed: the mark itself, large, on the venue's own paper,
// inside a slowly drawn ring in the venue's accent; the name beneath it in
// the display face, set in capitals and tracked out the way a sign is; a
// hairline rule in the accent with one dot at its centre; then what people
// think of it, whether it is open, how long a delivery takes, what it costs
// to send, and where it is. Every row appears only if its fact exists; a row
// that says "—" is a row that teaches the customer the page is broken.
//
// The full venue page (hours for the week, the map, the reviews, the address
// with directions) is a sheet, reached from the hero and from the Info tab, so
// the menu starts one scroll below the fold rather than three.

import { state, hhmm, todayAt, whenOpens, DAY_NAMES } from '/store/state.js';
import { t, lang } from '/store/i18n.js';
import { $, esc, icon, sheet, stars, loadMapLib } from '/store/ui.js';
import { publishedEta } from '/store/eta.js';

/// How many reviews the venue sheet shows.
const REVIEWS_SHOWN = 8;
/// The venue's own map: a street-level zoom.
const ZOOM_STREET = 16;
const TILES = 'https://tiles.openfreemap.org/styles/liberty';
/// The ring behind the mark: a circle of this radius in a 100-unit box, and a
/// circumference to draw it with. 2π·46 ≈ 289.
const RING_R = 46;
const RING_LEN = Math.round(2 * Math.PI * RING_R);

function openWindowNow(){
  const week = Array.isArray(state.loc?.hours) && state.loc.hours.length === 7 ? state.loc.hours : null;
  if (!week) return null;
  const now = todayAt();
  return (week[now.day] || []).find(w => now.minute >= w.open && now.minute < w.close) || null;
}

/// The one line about being open: "Open until 23:00" / "Closed · opens at 11:00".
export function openLine(){
  const L = state.loc;
  if (!L) return '';
  if (L.status === 'open') {
    const w = openWindowNow();
    return w ? `${t('openUntil')} ${hhmm(w.close)}` : t('openUntil');
  }
  if (L.closedReason === 'paused') return t('pausedNow');
  if (L.nextOpen) return `${t('closedNow')} · ${t('opensAt2')} ${whenOpens(L.nextOpen)}`;
  return t('closedNow');
}

const mapsHref = () => {
  const L = state.loc;
  if (Number.isFinite(L?.lat) && Number.isFinite(L?.lng))
    return `https://www.google.com/maps/search/?api=1&query=${L.lat},${L.lng}`;
  return L?.google?.url || null;
};

/// The town, from the address: the last part that is not the country. It is
/// the eyebrow over the name, the way a sign says where it stands.
function town(){
  const parts = String(state.loc?.address || '').split(',').map(s => s.trim()).filter(Boolean);
  if (parts.length < 2) return '';
  return parts[parts.length - 2];
}

/// The ornaments a rule can carry, by the venue's `stage.motif`. A leaf is
/// two sprigs in the sage tone; a wave is two short crests in the accent.
const MOTIF_SVG = {
  leaf: `<svg class="motif" viewBox="0 0 40 16"><path d="M2 14c6-9 14-11 22-10-3 8-11 12-22 10z"/><path d="M38 14c-6-9-14-11-22-10 3 8 11 12 22 10z"/><path d="M2 14c8-3 14-6 20-9M38 14c-8-3-14-6-20-9" class="vein"/></svg>`,
  wave: `<svg class="motif" viewBox="0 0 40 16"><path d="M1 11c4-6 8-6 12 0s8 6 12 0 8-6 12 0" class="vein"/><path d="M1 6c4-6 8-6 12 0s8 6 12 0 8-6 12 0" class="vein"/></svg>`,
};
/// The rule in the accent with a dot at its centre -- the venue mark's own
/// ornament -- and the venue's motif on either side of the dot when it has one.
export function ruleMarkup(){
  const motif = MOTIF_SVG[state.loc?.stage?.motif] || '';
  return `<div class="rule ${motif ? 'has-motif' : ''}" aria-hidden="true"><i></i>${motif}<b></b>${motif}<i></i></div>`;
}

/// The hero markup. Text nodes that depend on the language carry an id so
/// `refreshHero` can rewrite them without rebuilding the block.
export function heroMarkup(){
  const L = state.loc, g = L.google || null;
  const open = L.status === 'open';
  const eta = publishedEta();
  const where = town();
  return `
  <section class="hero" id="hero">
    ${L.logoUrl ? `<div class="hero-art" aria-hidden="true">
      <svg class="ring" viewBox="0 0 100 100"><circle cx="50" cy="50" r="${RING_R}" pathLength="${RING_LEN}"/></svg>
      <img class="hero-mark" src="${esc(L.logoUrl)}" alt="">
      ${L.stage?.seal ? `<span class="seal">${esc(L.stage.seal)}</span>` : ''}
    </div>` : ''}
    ${where ? `<p class="eyebrow hero-eyebrow">${esc(where)}</p>` : ''}
    <h1 class="hero-name">${esc(L.name)}</h1>
    ${ruleMarkup()}
    <div class="hero-chips">
      <button type="button" class="chip-state ${open ? 'open' : 'shut'}" id="stateChip" data-open-venue>
        <span class="dot"></span><span id="openLine">${esc(openLine())}</span></button>
      ${g && g.rating ? `<button type="button" class="chip-glass" data-open-venue="reviews">
        ${icon('star-filled', 'gold')}<b>${esc(String(g.rating).replace('.', lang === 'en' ? '.' : ','))}</b>
        ${g.reviewCount ? `<span class="muted">(${g.reviewCount})</span>` : ''}</button>` : ''}
      ${eta && open ? `<span class="chip-glass">${icon('clock')}<span>${esc(eta)} <span data-t="etaMin"></span></span></span>` : ''}
      ${L.deliveryFee ? `<span class="chip-glass">${icon('bike')}<span class="money" data-money="${L.deliveryFee}"></span></span>`
                      : `<span class="chip-glass">${icon('bike')}<span data-t="free"></span></span>`}
    </div>
    ${L.address ? `<button type="button" class="hero-addr" data-open-venue>
      ${icon('map-pin')}<span>${esc(L.address)}</span>${icon('chevron-right', 'chev')}</button>` : ''}
  </section>`;
}

export function refreshHero(){
  const el = $('#openLine'); if (el) el.textContent = openLine();
  const chip = $('#stateChip'); if (chip) { chip.classList.toggle('open', state.loc?.status === 'open'); chip.classList.toggle('shut', state.loc?.status !== 'open'); }
}

/// The venue sheet: hours, address, phone, map, reviews. `focus` scrolls to a
/// section once open.
export function openVenue(focus){
  const L = state.loc, g = L.google || null;
  const week = Array.isArray(L.hours) && L.hours.length === 7 ? L.hours : null;
  const names = DAY_NAMES[lang] || DAY_NAMES.en;
  const now = todayAt();
  const href = mapsHref();
  const reviews = (g && Array.isArray(g.reviews) ? g.reviews : []).slice(0, REVIEWS_SHOWN);
  sheet(`
    <div class="vsheet">
      <p class="eyebrow" data-t="about"></p>
      <h2 class="vname">${esc(L.name)}</h2>
      ${g && g.rating ? `<div class="vrate">
        <b>${esc(String(g.rating).replace('.', lang === 'en' ? '.' : ','))}</b>
        <span class="rev-stars" aria-hidden="true">${stars(g.rating)}</span>
        ${g.reviewCount ? `<span class="muted">${g.reviewCount}</span>` : ''}
        ${g.url ? `<a class="vsrc" href="${esc(g.url)}" target="_blank" rel="noopener noreferrer" data-t="fromGoogle"></a>` : ''}
      </div>` : ''}

      ${week ? `<h3 class="vsec-h" id="v-hours" data-t="hoursTitle"></h3>
      <div class="hours">${week.map((wins, i) => `<div class="hours-row${i === now.day ? ' on' : ''}">
        <span>${esc(names[i])}</span>
        <span>${wins.length ? wins.map(w => `${hhmm(w.open)}–${hhmm(w.close)}`).join(', ') : `<span data-t="closedNow"></span>`}</span></div>`).join('')}</div>` : ''}

      ${L.address ? `<h3 class="vsec-h" id="v-where">${esc(L.address)}</h3>` : ''}
      <div class="vrows">
        ${href ? `<a class="vrow" href="${esc(href)}" target="_blank" rel="noopener noreferrer">
          ${icon('map-pin')}<span data-t="directions"></span>${icon('chevron-right', 'chev')}</a>` : ''}
        ${L.phone ? `<a class="vrow" href="tel:${esc(L.phone)}">${icon('phone')}<span>${esc(L.phone)}</span><span class="vrow-act" data-t="callUs"></span></a>` : ''}
        ${Number.isFinite(L.lat) ? `<button class="vrow" type="button" id="mapGo" aria-expanded="false" aria-controls="mapBox">
          ${icon('gps')}<span data-t="onMap"></span>${icon('chevron-down', 'chev')}</button>
          <div class="vmap" id="mapBox" hidden></div>` : ''}
      </div>

      ${reviews.length ? `<h3 class="vsec-h" id="v-reviews"><span data-t="reviewsTitle"></span>
          <span class="vsrc" data-t="fromGoogle"></span></h3>
        <div class="revs">${reviews.map(r => `
          <figure class="rev">
            <figcaption>
              <span class="rev-who" aria-hidden="true">${esc((r.author || '?').trim().charAt(0))}</span>
              <span><b>${esc(r.author || '')}</b>
              <span class="rev-stars" aria-label="${r.rating || 0}/5">${stars(r.rating)}</span></span>
            </figcaption>
            <blockquote>${esc(r.text || '')}</blockquote>
          </figure>`).join('')}</div>` : ''}
    </div>`, { name: 'info' });
  const mg = $('#mapGo'); if (mg) mg.onclick = showMap;
  if (focus) $(`#v-${focus}`)?.scrollIntoView({ block: 'start', behavior: 'smooth' });
}

/// MapLibre, loaded the first time somebody asks to see the map. A third of a
/// megabyte and a tile session are not something a customer reading a menu
/// should pay for.
async function showMap(){
  const box = $('#mapBox'), btn = $('#mapGo');
  if (!box) return;
  const opening = box.hidden;
  box.hidden = !opening;
  btn?.setAttribute('aria-expanded', String(opening));
  if (!opening || box.dataset.drawn) return;
  box.dataset.drawn = '1';
  try {
    const maplibregl = await loadMapLib();
    const map = new maplibregl.Map({
      container: box, style: TILES,
      center: [state.loc.lng, state.loc.lat], zoom: ZOOM_STREET, attributionControl: true,
    });
    new maplibregl.Marker().setLngLat([state.loc.lng, state.loc.lat]).addTo(map);
  } catch (e) {
    box.textContent = `${t('loadFail')} · ${e.message}`;
    box.dataset.drawn = '';
  }
}
