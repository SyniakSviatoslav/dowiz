// Permission prompts — What is Your Location? 1:2612, Search Manual Location
// Access 1:2645, Enable Notification Access 1:2700 (light 1:13024, 1:13058,
// 1:13114).
//
// One shape: a 118px round holder with a 56px glyph at 104,218, a 24/600 ask,
// a 12/400 line, and a 327x48 button with a quieter way out under it.
//
// These screens ASK THE BROWSER, they do not pretend to. A prompt that says
// "Allow" and then does nothing is the kind of screen that makes people stop
// trusting the next one.

import { icon, esc } from '/kit/app.js';

const ASKS = {
  location: {
    icon: 'pin-24', head: 'What is Your Location?',
    body: 'We need your location to show nearby restaurants and give an honest delivery time.',
    cta: 'Allow Location Access', skip: 'Enter Location Manually', skipTo: 'manual-location',
    kind: 'geo',
  },
  'manual-location': {
    icon: 'search', head: 'Search Your Location',
    body: 'Type an address and we will use it for this order.',
    cta: 'Use This Address', skip: 'Use My Location', skipTo: 'location',
    kind: 'manual',
  },
  'notification-access': {
    icon: 'notification-bing', head: 'Enable Notification Access',
    body: 'Enable notifications to receive real-time updates about your order.',
    cta: 'Allow Notification', skip: 'Not Now', skipTo: 'home',
    kind: 'notify',
  },
};

export function render(params, routeName = 'location'){
  const a = ASKS[routeName] || ASKS.location;
  return `
  <div class="k-ask" data-ask="${esc(routeName)}">
    <span class="k-ask-ring">${icon(a.icon)}</span>
    <h1>${esc(a.head)}</h1>
    <p>${esc(a.body)}</p>
    ${a.kind === 'manual' ? `
      <div class="k-in">
        <label class="k-field k-ask-field">
          ${icon('search')}
          <input id="addr" type="text" placeholder="Вулиця, будинок" aria-label="Адреса">
        </label>
      </div>` : ''}
    <p class="k-ask-said" id="said" role="status"></p>
    <div class="k-ask-acts">
      <button class="k-submit" type="button" id="allow">${esc(a.cta)}</button>
      <button class="k-ask-skip" type="button" data-go="${esc(a.skipTo)}">${esc(a.skip)}</button>
    </div>
  </div>`;
}

export function bind(root){
  const host = root.querySelector('[data-ask]');
  const a = ASKS[host.dataset.ask] || ASKS.location;
  const said = root.querySelector('#said');

  root.querySelector('#allow')?.addEventListener('click', async () => {
    if (a.kind === 'geo') return askGeo(said);
    if (a.kind === 'notify') return askNotify(said);
    const addr = root.querySelector('#addr')?.value.trim();
    if (!addr) { said.textContent = 'Введіть адресу'; return; }
    try { sessionStorage.setItem('dw_kit_addr', addr); } catch { /* private window */ }
    location.hash = '#/home';
  });
}

function askGeo(said){
  if (!navigator.geolocation){
    said.textContent = 'Цей браузер не вміє визначати місце — введіть адресу вручну.';
    return;
  }
  said.textContent = 'Питаємо браузер…';
  navigator.geolocation.getCurrentPosition(
    pos => {
      // Coarse on purpose: a delivery radius does not need six decimal places,
      // and an order confirmation carries a name and a phone beside it.
      const lat = pos.coords.latitude.toFixed(3), lon = pos.coords.longitude.toFixed(3);
      try { sessionStorage.setItem('dw_kit_geo', `${lat},${lon}`); } catch { /* private */ }
      location.hash = '#/home';
    },
    err => { said.textContent = `Не вийшло: ${err.message}. Введіть адресу вручну.`; },
    { enableHighAccuracy: false, timeout: 10000, maximumAge: 300000 });
}

async function askNotify(said){
  if (!('Notification' in window)){
    said.textContent = 'Цей браузер не показує сповіщень.';
    return;
  }
  try {
    const answer = await Notification.requestPermission();
    said.textContent = answer === 'granted' ? 'Дозволено.' : 'Відхилено — можна ввімкнути пізніше.';
    if (answer === 'granted') location.hash = '#/home';
  } catch (e){
    said.textContent = `Не вийшло: ${e.message}`;
  }
}
