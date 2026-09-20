// dowiz — the main hub.
//
// The one screen from which a client's hub comes into existence. It is
// deliberately small: sign in, see the hubs, make one. Everything a venue does
// after that happens on the venue's own host.

const API = '/api';
const $ = s => document.querySelector(s);
const esc = s => String(s ?? '').replace(/[&<>"']/g, c =>
  ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));

// The token lives in memory only. This screen mints hubs on the platform's own
// domain, so a token left in localStorage is a key to that sitting in a browser
// that may be shared; a reload asking for the password again is the cheapest
// possible mitigation and costs an administrator five seconds.
let TOKEN = null;

async function api(path, opts = {}) {
  const r = await fetch(API + path, {
    ...opts,
    headers: {
      'content-type': 'application/json',
      ...(opts.headers || {}),
      ...(TOKEN ? { authorization: 'Bearer ' + TOKEN } : {}),
    },
  });
  const text = await r.text();
  let body = null;
  try { body = text ? JSON.parse(text) : null; } catch { /* not json */ }
  if (!r.ok) {
    throw new Error((body && (body.error || body.message)) || text || ('HTTP ' + r.status));
  }
  return body;
}

function say(el, msg, kind) {
  el.className = 'said ' + (kind || '');
  el.textContent = msg;
  el.classList.remove('hidden');
}

// ── sign in ─────────────────────────────────────────────────────────────────
async function signIn() {
  const btn = $('#go'), said = $('#signinSaid');
  const email = $('#email').value.trim(), password = $('#pw').value;
  if (!email || !password) { say(said, 'Потрібні пошта і пароль.', 'bad'); return; }
  btn.disabled = true;
  try {
    const d = await api('/auth/login', {
      method: 'POST',
      body: JSON.stringify({ email, password }),
    });
    TOKEN = d.access_token;
    // THE ROLE IS PROVED BY THE PLATFORM, NOT ASSUMED BY THIS PAGE. A perfectly
    // valid owner token signs in here and is not a platform administrator; the
    // hub list is what says so, and it answers 404 rather than telling them
    // this door exists.
    await loadHubs();
    $('#signin').classList.add('hidden');
    $('#hubs').classList.remove('hidden');
    $('#wait').classList.remove('hidden');
    $('#create').classList.remove('hidden');
    loadWaitlist().catch(e => { $('#waitList').innerHTML = `<p class="hint">${esc(String(e.message || e))}</p>`; });
    $('#who').textContent = email;
    said.classList.add('hidden');
  } catch (e) {
    const m = String(e.message || e);
    say(said, /not found|404/i.test(m)
      ? 'Цей обліковий запис не адміністратор платформи.'
      : m, 'bad');
  } finally {
    btn.disabled = false;
  }
}

// ── the hubs ────────────────────────────────────────────────────────────────
async function loadHubs() {
  const d = await api('/platform/hubs');
  if (d.platform) $('#pHost').textContent = d.platform;
  const list = $('#hubList');
  if (!d.hubs || !d.hubs.length) {
    list.innerHTML = '<p class="hint">Жодного хабу ще немає.</p>';
    return d;
  }
  list.innerHTML = d.hubs.map(h => `
    <div class="hub">
      <span class="grow">
        <b>${esc(h.name)}</b>
        <div><a href="${esc(h.url)}" target="_blank" rel="noopener">${esc(h.host)}</a></div>
        <div class="hint">
          <a href="${esc(h.console)}" target="_blank" rel="noopener">консоль</a> ·
          <a href="${esc(h.courier)}" target="_blank" rel="noopener">кур'єр</a>
        </div>
      </span>
      <span class="tag">${esc(h.status)}</span>
    </div>`).join('');
  return d;
}

// ── the waiting list ────────────────────────────────────────────────────────
// What the landing page collected. The mail is the bell; this is the record,
// and it is here so a mail that never arrived costs nobody a lead.
async function loadWaitlist() {
  const d = await api('/platform/waitlist');
  const list = $('#waitList');
  if (!d.rows || !d.rows.length) {
    list.innerHTML = '<p class="hint">Поки порожньо.</p>';
    return d;
  }
  const when = ms => new Date(ms).toLocaleString('uk-UA', { dateStyle: 'short', timeStyle: 'short' });
  list.innerHTML = d.rows.map(r => `
    <div class="hub">
      <span class="grow">
        <b>${esc(r.email)}</b>
        <div>${esc(r.venue || '—')}</div>
        <div class="hint">${esc(when(r.updated_ms))} · ${esc(r.lang)} · ${esc(r.source || '')}${r.notified_ms ? '' : ' · лист не надіслано'}</div>
      </span>
      <span class="tag">${r.notified_ms ? 'mailed' : 'stored'}</span>
    </div>`).join('');
  return d;
}

// ── making one ──────────────────────────────────────────────────────────────
async function createHub() {
  const btn = $('#make'), said = $('#makeSaid');
  const slug = $('#slug').value.trim().toLowerCase();
  const name = $('#name').value.trim();
  if (!slug || !name) { say(said, 'Потрібні slug і назва.', 'bad'); return; }

  const body = { slug, name, phone: $('#phone').value.trim() };
  const oemail = $('#oemail').value.trim(), opw = $('#opw').value;
  if (oemail || opw) {
    if (!oemail || !opw) { say(said, 'Власнику потрібні і пошта, і пароль.', 'bad'); return; }
    body.owner = { email: oemail, password: opw };
  }

  btn.disabled = true;
  try {
    const d = await api('/platform/hubs', { method: 'POST', body: JSON.stringify(body) });
    // The links are the answer. An administrator who has just made a hub wants
    // to open it, and retyping a hostname they were only shown is how a typo
    // becomes "the hub does not work".
    say(said,
      `Хаб «${d.name}» створено.\n` +
      `Вітрина:  ${d.url}\n` +
      `Консоль:  ${d.console}\n` +
      `Кур'єр:   ${d.courier}\n` +
      (d.ownerEmail ? `Власник:  ${d.ownerEmail}\n` : 'Власника не створено.\n') +
      'Хаб відкривається ЗАКРИТИМ — власник відкриє його, коли буде меню.',
      'good');
    ['slug', 'name', 'phone', 'oemail', 'opw'].forEach(id => { $('#' + id).value = ''; });
    await loadHubs();
  } catch (e) {
    say(said, String(e.message || e), 'bad');
  } finally {
    btn.disabled = false;
  }
}

$('#go').onclick = signIn;
$('#make').onclick = createHub;
// Enter submits the form it is in, because a sign-in that needs the mouse is
// not one anybody uses twice.
['email', 'pw'].forEach(id => {
  $('#' + id).onkeydown = e => { if (e.key === 'Enter') { e.preventDefault(); signIn(); } };
});
