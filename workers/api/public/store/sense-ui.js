// The dish's taste, texture and aroma on the storefront (W-SENSE rows 2, 4, 5): the words, the
// filter chips, the session's mood and the venue's moment. Markup is store/sense-view.js (pure);
// the rules are store/sense.js (pure). This file only holds the page's state and wires it.
//
// THE MOOD NEVER LEAVES THIS PAGE: it is a module variable, not storage, and no request carries
// it (tools/gates/no-tracking.mjs rule 6). THE MOMENT IS THE VENUE'S: the context route takes no
// query, so nothing about the guest's place can be sent (services/venue/context.rs).
//
// ASCII QUOTES ONLY in this file.

import { T, LANGS, retranslate } from '/store/i18n.js';
import { $, $$ } from '/store/ui.js';
import { API, SLUG } from '/store/state.js';
import { SENSE_WORDS } from '/lib/sense-words.js';
import { senseOf, cardAttr, filterChips, passes } from '/store/sense.js';
import { cardSense, sheetSense, filterRow, moodRow } from '/store/sense-view.js';

for (const l of LANGS) Object.assign(T[l], SENSE_WORDS[l]);

// ── the page's state ────────────────────────────────────────────────────────
const active = new Set();
let mood = null;
let moment = null;
export const currentMood = () => mood;
export const momentKeys = () => moment;

/// The venue's moment (band, kind of day, weather), once per page. Silent on failure: no context.
export async function loadMoment(){
  if (moment) return moment;
  try {
    const r = await fetch(`${API}/public/locations/${encodeURIComponent(SLUG)}/context`);
    if (r.ok) moment = (await r.json())?.keys || null;
  } catch { moment = null; }
  return moment;
}

// ── cards ───────────────────────────────────────────────────────────────────
/// What a grid card carries: the data a filter reads and the compact line.
export const cardData = p => cardAttr(senseOf(p));
export const cardLine = p => cardSense(p);
export const sheetLine = (p, guest) => sheetSense(p, guest);

// ── filters ─────────────────────────────────────────────────────────────────
export const senseFilters = products => filterRow(filterChips(products), active);
/// Does the guest's taste filter hide this card?
export const senseHides = el => !passes(el.dataset.sense || '', active);
/// One delegated listener on the row; `onChange` refilters the menu.
export function wireSenseFilters(onChange){
  const row = $('#sxFilters'); if (!row) return;
  row.onclick = e => {
    const b = e.target.closest('[data-sxf]'); if (!b) return;
    const id = b.dataset.sxf;
    if (active.has(id)) active.delete(id); else active.add(id);
    b.classList.toggle('on', active.has(id)); b.setAttribute('aria-pressed', String(active.has(id)));
    onChange?.();
  };
}
export const clearSenseFilters = () => { active.clear(); for (const b of $$('[data-sxf]')) { b.classList.remove('on'); b.setAttribute('aria-pressed', 'false'); } };

// ── mood ────────────────────────────────────────────────────────────────────
export const moodMarkup = () => moodRow(mood);
export function wireMood(host, onChange){
  if (!host) return;
  for (const b of $$('[data-mood]', host)) b.onclick = () => {
    mood = mood === b.dataset.mood ? null : b.dataset.mood;
    onChange?.();
  };
  retranslate(host);
}
