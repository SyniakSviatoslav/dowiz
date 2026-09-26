// The Telegram screen (W-TG T5): connect the venue's bot, link groups with a
// one-time code, and choose per group what arrives -- the matrix. Drawing is
// telegram-view.js (pure, tested); this file is the fetches and the taps.
//
// Every change is saved on the tap (POST /api/owner/telegram/group), so there
// is no Save button to forget. The venue travels as ?location_id=, because the
// hub refuses unknown body fields.
//
// ASCII QUOTES ONLY in this file.

import { $, t, api, post, toast, sheet, store, clock, day, busy, confirm, retranslate } from '/admin/core.js';
import { T, LANGS } from '/admin/i18n.js';
import { WORDS } from '/admin/telegram-words.js';
import { page, wizard, nextMode, minutes } from '/admin/telegram-view.js';

// Merged, never replaced: a language with no row of its own reads English.
for (const l of LANGS) for (const [key, v] of Object.entries({ ...WORDS.en, ...(WORDS[l] || {}) })) if (T[l] && !(key in T[l])) T[l][key] = v;

const q = () => '?location_id=' + encodeURIComponent(store.loc || '');
const fail = e => toast(String((e && e.message) || e));
const when = ms => (ms ? `${day(ms)} ${clock(ms)}` : '');
let D = null, pending = null, timer = null;

function css(){
  if (document.getElementById('tgCss')) return;
  const l = document.createElement('link');
  l.id = 'tgCss'; l.rel = 'stylesheet'; l.href = '/admin/telegram.css';
  document.head.appendChild(l);
}

async function load(){
  D = await api('/owner/telegram' + q());
  // A code minted before this screen opened: rebuild the command and the link.
  if (D.pending && D.bot && (!pending || pending.code !== D.pending.code)) {
    const u = D.bot.username;
    pending = { ...D.pending, command: `/link@${u} ${D.pending.code}`, deepLink: `https://t.me/${u}?startgroup=${D.pending.code}` };
  }
  return D;
}

function draw(){
  sheet(page(D, pending, Date.now(), when), { name: 'telegram', keepScroll: true });
  retranslate();
  bind();
}

export async function open(){
  css();
  try { await load(); } catch (e) { return fail(e); }
  draw();
}

const group = id => (D.groups || []).find(g => g.id === id);

async function patch(id, p){
  const r = await post('/owner/telegram/group' + q(), { id, patch: p });
  const i = D.groups.findIndex(g => g.id === id);
  if (r.group && i >= 0) D.groups[i] = { ...D.groups[i], ...r.group, linked: D.groups[i].linked };
  D.legacy = false;
  return r;
}

function bind(){
  const root = $('#sheetIn');
  $('#tgConnect').onclick = async () => {
    try {
      const r = await busy($('#tgConnect'), () => post('/owner/telegram/connect' + q(), { token: $('#tgToken').value.trim() }));
      toast(`@${r.bot} · ${t('tg_connected')}`);
      await load(); draw();
    } catch (e) { fail(e); }
  };
  const add = $('#tgAdd');
  if (add) add.onclick = async () => {
    try { pending = await busy(add, () => post('/owner/telegram/link' + q(), {})); watch(); draw(); } catch (e) { fail(e); }
  };
  const copy = $('#tgCopy');
  if (copy) copy.onclick = () => navigator.clipboard?.writeText($('#tgCmd').textContent).then(() => toast(t('tg_copied')), fail);
  root.onclick = e => tap(e.target).catch(fail);
  root.onchange = e => change(e.target).catch(fail);
  if (pending) watch();
}

/// Taps: a matrix cell, a language, a customer-data level, quiet hours on/off,
/// test, pause, unlink.
async function tap(el){
  const b = el.closest('button');
  if (!b || b.disabled) return;
  const d = b.dataset;
  const g = el.closest('[data-group]')?.dataset.group;
  if (d.cell) {
    const [id, ev] = d.cell.split('/');
    const mode = nextMode(d.mode, d.scheduled === '1');
    await patch(id, { subs: { [ev]: mode } });
    return draw();
  }
  if (d.glang && g) { await patch(g, { lang: d.glang }); return draw(); }
  if (d.gpii && g) { await patch(g, { pii: d.gpii }); return draw(); }
  if (d.gquiet) {
    const on = !group(d.gquiet).quiet;
    const box = el.closest('[data-group]');
    const from = minutes($('[data-gqf]', box).value), to = minutes($('[data-gqt]', box).value);
    await patch(d.gquiet, { quiet: on && from != null && to != null && from !== to ? { from, to } : null });
    return draw();
  }
  if (d.gtest) {
    const r = await busy(b, () => post('/owner/telegram/test' + q(), { id: d.gtest }));
    return toast(r.ok ? t('tg_sentOk') : r.error);
  }
  if (d.gmute) { await patch(d.gmute, { muted: group(d.gmute).state !== 'muted' }); return draw(); }
  if (d.gunlink) {
    if (!(await confirm(t('tg_unlinkQ'), t('tg_unlinkBody'), { danger: true }))) return draw();
    await post('/owner/telegram/unlink' + q(), { id: d.gunlink });
    await load(); return draw();
  }
}

/// Time inputs: the quiet window (only while it is on) and the summary time.
async function change(el){
  const d = el.dataset;
  const box = el.closest('[data-group]');
  if (d.gdig) {
    const m = minutes(el.value);
    if (m != null) await patch(d.gdig, { digest_at: m });
    return;
  }
  const id = d.gqf || d.gqt;
  if (id && group(id).quiet) {
    const from = minutes($('[data-gqf]', box).value), to = minutes($('[data-gqt]', box).value);
    if (from != null && to != null && from !== to) await patch(id, { quiet: { from, to } });
  }
}

/// While a code is pending: count down, and look for the new group every 3 s.
function watch(){
  clearTimeout(timer);
  const before = (D.groups || []).length;
  const tick = async () => {
    if (!pending || !document.getElementById('tgWiz')) return;
    const left = pending.exp_ms - Date.now();
    if (left <= 0) { $('#tgWiz').innerHTML = wizard(pending, 0); retranslate(); pending = null; return; }
    const out = $('#tgLeft');
    if (out) out.textContent = `${Math.floor(left / 60000)}:${String(Math.floor(left / 1000) % 60).padStart(2, '0')}`;
    try {
      await load();
      if ((D.groups || []).length > before || !D.pending) {
        const g = D.groups[D.groups.length - 1];
        pending = null;
        toast(`${t('tg_linked')} ${g ? g.title || g.id : ''}`);
        return draw();
      }
    } catch { /* the next tick asks again */ }
    timer = setTimeout(tick, 3000);
  };
  timer = setTimeout(tick, 3000);
}

