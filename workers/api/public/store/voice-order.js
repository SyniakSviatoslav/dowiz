// Ordering by voice: the browser hears, the menu decides, the customer confirms.
//
// RECOGNITION IS THE BROWSER'S (lib/voice.js): the Web Speech API is on the
// phone already, in the customer's language, and no audio leaves it. What is
// heard is matched against the VENUE'S OWN MENU, here, on the words the menu
// is in right now -- "два Philadelphia", "dy Hot Ebi", "one salmon bowl".
// Nothing is added on a guess: the best match is shown as a card with its
// photograph, quantity and price, read back aloud, and goes into the basket
// only when the customer says so with a tap. Two more candidates sit under
// it for the case where the best guess was the wrong roll.
//
// A number is a word or a digit; a dish is the longest run of its name the
// transcript contains. A word the transcript has that no dish has costs
// nothing -- people say "please".

import { state, normalise, addLine, moneyEl } from '/store/state.js';
import { t, lang } from '/store/i18n.js';
import { $, $$, esc, icon, sheet, closeSheet, toast, fallbackArt, paintFallbacks } from '/store/ui.js';
import { seaEvent } from '/store/sea.js';

/// How many candidates the card offers, and the lowest score shown at all.
const CANDIDATES = 3;
const SCORE_MIN = 0.34;
/// A quantity said aloud is between these.
const QTY_MIN = 1;
const QTY_MAX = 20;
/// A word this short is a particle, not a dish name, and does not count.
const WORD_MIN = 2;
/// The Sea's pulse when a voice order lands.
const SEA_PULSE = 30;

let rec = null;

const words = s => normalise(s).replace(/[^\p{L}\p{N}\s]/gu, ' ').split(/\s+/).filter(w => w.length >= WORD_MIN);

/// The quantity in the transcript, from a digit or a number word, else one.
function quantity(tokens){
  const table = t('qtyWords') || {};
  for (const w of tokens) {
    const n = /^\d{1,2}$/.test(w) ? Number(w) : table[w];
    if (Number.isFinite(n) && n >= QTY_MIN && n <= QTY_MAX) return n;
  }
  return QTY_MIN;
}

/// Score a dish against the transcript: the share of the dish's name words
/// the transcript contains, weighted toward longer names when tied.
function score(p, tokens){
  const name = words(p.name);
  if (!name.length) return 0;
  const hits = name.filter(w => tokens.some(tk => tk === w || (w.length > 3 && tk.startsWith(w.slice(0, -1))) || (tk.length > 3 && w.startsWith(tk.slice(0, -1))))).length;
  if (!hits) return 0;
  return hits / name.length + hits * 0.01;
}

function candidates(transcript){
  const tokens = words(transcript);
  const list = [];
  for (const p of state.products.values()) {
    if (!p.available) continue;
    const s = score(p, tokens);
    if (s >= SCORE_MIN) list.push([s, p]);
  }
  list.sort((a, b) => b[0] - a[0]);
  return { qty: quantity(tokens), picks: list.slice(0, CANDIDATES).map(([, p]) => p) };
}

function say(text){ import('/lib/voice.js').then(v => v.speak(text, v.tagFor(lang))).catch(() => {}); }

function card(p, qty, main){
  return `<button type="button" class="vpick ${main ? 'main' : ''}" data-pick="${esc(p.id)}">
    <span class="vpick-img">${p.imageUrl ? `<img src="${esc(p.imageUrl)}" alt="" data-fb="${esc(p.name)}">` : fallbackArt(p.name)}</span>
    <span class="vpick-t"><b>${main ? `${qty} × ` : ''}${esc(p.name)}</b><small>${esc(p.description || '')}</small></span>
    ${moneyEl(p.price * (main ? qty : 1))}
  </button>`;
}

function showResult(transcript){
  const { qty, picks } = candidates(transcript);
  const host = $('#voiceOut'); if (!host) return;
  if (!picks.length) {
    host.innerHTML = `<p class="voice-heard">${icon('microphone')}<span>${esc(transcript)}</span></p><p class="err">${esc(t('noMatch'))}</p>`;
    say(t('noMatch'));
    return;
  }
  host.innerHTML = `<p class="voice-heard">${icon('microphone')}<span>${esc(transcript)}</span></p>
    <p class="eyebrow" data-t="heard"></p>
    ${card(picks[0], qty, true)}
    <div class="vpick-row"><button type="button" class="btn" id="voiceAdd">${icon('plus')}<span data-t="add"></span></button>
      <button type="button" class="btn btn-ghost" id="voiceAgain">${icon('microphone')}<span data-t="notThis"></span></button></div>
    ${picks.length > 1 ? `<div class="vpick-alt">${picks.slice(1).map(p => card(p, 1, false)).join('')}</div>` : ''}`;
  paintFallbacks(host);
  for (const el of $$('[data-t]', host)) el.textContent = t(el.dataset.t);
  say(`${qty} ${picks[0].name}`);
  const add = (p, n, el) => {
    addLine(p.id, [], n);
    const r = el?.getBoundingClientRect?.();
    seaEvent('order_created', SEA_PULSE, r ? { x: r.left + r.width / 2, y: r.top + r.height / 2 } : undefined);
    toast(`${t('addedByVoice')}: ${p.name} · ${n}`);
    say(`${t('addedByVoice')}: ${p.name}`);
    dispatchEvent(new Event('dw:cart'));
    closeSheet();
  };
  $('#voiceAdd').onclick = e => add(picks[0], qty, e.currentTarget);
  $('#voiceAgain').onclick = listen;
  for (const b of $$('.vpick:not(.main)', host)) b.onclick = () => add(state.products.get(b.dataset.pick), qty, b);
}

/// Open the voice sheet and start listening. Silent where unsupported.
export async function openVoice(){
  let voice;
  try { voice = await import('/lib/voice.js'); } catch { return toast(t('noVoice')); }
  if (!voice.supported()) return toast(t('noVoice'));
  sheet(`<div class="vsheet-voice">
    <p class="eyebrow" data-t="voice"></p>
    <h2 id="voiceState" data-t="listening"></h2>
    <p class="muted small" data-t="sayLike"></p>
    <div class="voice-wave" id="voiceWave" aria-hidden="true"><i></i><i></i><i></i><i></i><i></i></div>
    <div id="voiceOut"></div>
  </div>`, { name: 'voice' });
  listen();
}

async function listen(){
  const voice = await import('/lib/voice.js');
  const stateEl = $('#voiceState'), wave = $('#voiceWave'), out = $('#voiceOut');
  if (!stateEl) return;
  stateEl.textContent = t('listening'); wave?.classList.add('on'); if (out) out.innerHTML = '';
  try { rec?.abort?.(); } catch {}
  rec = voice.create({
    lang: voice.tagFor(lang),
    onResult: ({ transcript, isFinal }) => {
      if (!isFinal) { if (out) out.innerHTML = `<p class="voice-heard interim">${icon('microphone')}<span>${esc(transcript)}</span></p>`; return; }
      wave?.classList.remove('on'); stateEl.textContent = t('heard');
      showResult(transcript);
    },
    onError: why => { wave?.classList.remove('on'); stateEl.textContent = why === 'microphone-denied' ? t('micDenied') : t('noVoice'); },
    onEnd: () => { wave?.classList.remove('on'); if (stateEl.textContent === t('listening')) stateEl.textContent = t('heard'); },
  });
  try { rec.start(); } catch { stateEl.textContent = t('noVoice'); }
}
