// Map screens — Explore 1:2738, Track Live Location 1:7695, Get Direction
// 1:7971. Light set: 1:13153, 1:18120, 1:18396.
//
// The map is MapLibre, already self-hosted at /lib/map/ and already allowed by
// the content policy (`worker-src blob:` for its tile workers, `connect-src`
// for tiles.openfreemap.org). This module loads it the same way the courier app
// does: lazily, once, with the stylesheet, and REJECTING on failure so the
// screen can fall back to the address and the phone number. 932 KB is not
// something to download for a screen that does not show a map.

import { icon, esc } from '/kit/app.js';
import { topBar, ctaBar, plate } from '/kit/parts.js';

const STYLE = {
  light: 'https://tiles.openfreemap.org/styles/bright',
  dark:  'https://tiles.openfreemap.org/styles/dark',
};

// The venue, until a real fix arrives — the same starting point the courier app
// uses, so the two surfaces open on the same place.
const CENTRE = [19.4449964, 41.315347];

const SCREENS = {
  explore: {
    title: null,
    chips: ['Nearest', 'Cuisines', 'Popular', 'Great Offers', 'Open Restaurants'],
    search: 'Search Restaurants',
    sheet: { kind: 'venue', name: 'The Savory Spot',
             sub: 'Italian, American, Chinese, Japanese...', cta: null },
  },
  'track-live': {
    title: 'Track Live Location',
    sheet: { kind: 'track', eta: '07:55 PM - 08:00 PM', partner: 'Charlotte Taylor',
             from: 'The Savory Spot', to: '8502 Preston Rd, Inglewood...',
             item: 'ItaliaCrisp Pizza', itemSub: 'Pizza · 8’ - Small · Qty. : 1' },
  },
  'get-direction': {
    title: 'Get Direction',
    sheet: { kind: 'venue', name: 'Brooklyn Bites',
             sub: '1012 Ocean avanue, New yourk, USA', cta: 'Start' },
  },
};

let libPromise = null;
function loadMapLibrary(){
  if (window.maplibregl) return Promise.resolve();
  if (libPromise) return libPromise;
  libPromise = new Promise((resolve, reject) => {
    const css = document.createElement('link');
    css.rel = 'stylesheet';
    css.href = '/lib/map/maplibre-gl.css';
    document.head.appendChild(css);
    const js = document.createElement('script');
    js.src = '/lib/map/maplibre-gl.js';
    js.async = true;
    js.onload = resolve;
    // Rejecting rather than hanging is the whole point: the screen has to be
    // told, so it can show the address instead of a grey rectangle forever.
    js.onerror = () => { libPromise = null; reject(new Error('map unavailable')); };
    document.head.appendChild(js);
  });
  return libPromise;
}

const sheet = s => {
  if (s.kind === 'track') return `
    <div class="k-map-note">
      <span><span class="k-map-note-k">Estimated Arrival Time</span>
        <span class="k-map-note-v">${esc(s.eta)}</span></span>
    </div>
    <div class="k-sum-rule"></div>
    <div class="k-choice">
      <span class="k-choice-ring">${plate(s.partner)}</span>
      <span class="k-choice-body">
        <span class="k-choice-s">Delivery Partner</span>
        <span class="k-choice-t">${esc(s.partner)}</span>
      </span>
      <button class="k-change" type="button" data-go="chat-detail">CHAT</button>
      <button class="k-change" type="button" data-go="voice-call">CALL</button>
    </div>
    <div class="k-sum-rule"></div>
    <p class="k-choice-s">${icon('pin-17')}${esc(s.from)}</p>
    <p class="k-choice-s">${icon('pin-17')}${esc(s.to)}</p>
    <div class="k-sum-rule"></div>
    <h2 class="k-block-h">Item</h2>
    <div class="k-choice">
      <span class="k-choice-ring">${plate(s.item)}</span>
      <span class="k-choice-body">
        <span class="k-choice-t">${esc(s.item)}</span>
        <span class="k-choice-s">${esc(s.itemSub)}</span>
      </span>
    </div>`;

  return `
    <div class="k-choice">
      <span class="k-choice-ring">${icon('pin-24')}</span>
      <span class="k-choice-body">
        <span class="k-choice-t">${esc(s.name)}</span>
        <span class="k-choice-s">${esc(s.sub)}</span>
      </span>
      ${s.cta ? `<button class="k-change" type="button" id="start">${esc(s.cta)}</button>` : ''}
    </div>`;
};

export function render(params, routeName = 'explore'){
  const s = SCREENS[routeName] || SCREENS.explore;
  return `
  <div class="k-map-screen" data-map="${esc(routeName)}">
    <div class="k-map" id="map" role="img" aria-label="Мапа">
      <p class="k-map-off" id="mapOff" hidden></p>
    </div>

    ${s.title ? topBar(s.title) : `
      <div class="k-map-over">
        <label class="k-field">
          ${icon('search')}
          <input id="q" type="search" placeholder="${esc(s.search)}" aria-label="Пошук">
        </label>
        <div class="k-chips k-map-chips" role="group" aria-label="Фільтри">
          ${s.chips.map((c, i) => `
            <button class="k-chip${i === 0 ? ' on' : ''}" type="button" data-chip="${esc(c)}"
                    aria-pressed="${i === 0}">${esc(c)}</button>`).join('')}
        </div>
      </div>`}

    <div class="k-map-sheet">
      <div class="k-sheet-grab" aria-hidden="true"></div>
      ${sheet(s.sheet)}
      <p class="k-map-said" id="said" role="status"></p>
    </div>
  </div>`;
}

export function bind(root){
  const off = root.querySelector('#mapOff');

  loadMapLibrary().then(() => {
    const dark = matchMedia('(prefers-color-scheme: dark)').matches
      || document.documentElement.getAttribute('data-theme') === 'dark';
    const map = new window.maplibregl.Map({
      container: 'map',
      style: STYLE[dark ? 'dark' : 'light'],
      center: CENTRE,
      zoom: 13,
      attributionControl: { compact: true },
    });
    map.addControl(new window.maplibregl.NavigationControl({ showCompass: false }), 'top-right');
    // MapLibre gives every marker `tabindex="0"` and a button role, because a
    // marker can carry a popup. Ours carries nothing: it marks the address, and
    // activating it does nothing at all. A focusable element that answers no
    // activation is a trap for a keyboard and a 27px miss for a thumb, so the
    // pin is handed back to the picture it is.
    const pin = new window.maplibregl.Marker({ color: '#ea4f16' })
      .setLngLat(CENTRE).addTo(map);
    const el = pin.getElement();
    el.removeAttribute('tabindex');
    el.removeAttribute('role');
    el.setAttribute('aria-hidden', 'true');
  }).catch(() => {
    // Say what happened and leave the address readable underneath.
    off.hidden = false;
    off.textContent = 'Мапа не завантажилась. Адреса нижче лишається дійсною.';
  });

  root.addEventListener('click', e => {
    const chip = e.target.closest('[data-chip]');
    if (chip){
      for (const b of root.querySelectorAll('[data-chip]')){
        const on = b === chip;
        b.setAttribute('aria-pressed', String(on));
        b.classList.toggle('on', on);
      }
      return;
    }
    if (e.target.closest('#start')){
      root.querySelector('#said').textContent =
        'Маршрут ще не підключений — відкрийте адресу у своєму навігаторі.';
    }
  });

}
