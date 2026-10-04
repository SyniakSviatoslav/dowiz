// BRING GUESTS DIRECT (W-QR): the card in every delivery bag.
//
// A marketplace brings a venue's first order and keeps the guest; this card is
// how the SECOND order comes to the venue's own storefront. The owner sets ONE
// public welcome offer (the same for everyone who scans, once per phone), prints
// an A6 card or a sheet of 8 stickers, and reads what came of it.
//
// THE HUB DRAWS THE QR (`GET /api/owner/bag`, `services/loyalty/bag_routes.rs`,
// the table codes' encoder): this page never builds a code, and loads nothing
// from a CDN (the CSP allows neither). PNG is the same SVG drawn on a canvas.
//
// PRINTING: like the table codes, the sheet is copied into one top-level
// `#bagPrint` block and `bag.css` hides everything else on paper.
//
// ASCII QUOTES ONLY in this file: a typographic quote once took down the
// whole console.

import { $, esc, icon, t, S, api, post, toast, sheet, store, busy, money } from '/admin/core.js';
import { retranslate } from '/admin/i18n.js';
import { btn, field, select, loading, rowDiv } from '/admin/parts.js';
import '/admin/bag-i18n.js';
import { KINDS, formBody, cardLine, campaign, statRows, svgSrc } from '/admin/bag-logic.js';

const STICKERS = 8;
const PNG_PX = 1024;
const q = c => '?location_id=' + encodeURIComponent(store.loc || '') + (c ? '&c=' + encodeURIComponent(c) : '');
const fail = e => toast(String((e && e.message) || e));
const dishName = id => (S.products || []).find(p => p.id === id)?.name || id;
const view = { c: '' };

function ensureCss(){
  if (document.getElementById('bagCss')) return;
  const l = document.createElement('link');
  l.id = 'bagCss'; l.rel = 'stylesheet'; l.href = '/admin/bag.css';
  document.head.appendChild(l);
}

/// One printed piece: the venue's name, the line, the code, the host.
const piece = (d, cls) => `<figure class="bag-piece ${cls}">
  <b class="bag-venue">${esc(S.venue?.name || '')}</b>
  <span class="bag-line">${esc(cardLine(d.offer, t, money, dishName))}</span>
  <img src="${svgSrc(d.svg)}" alt="${esc(d.url)}">
  <small class="bag-host mono">${esc(d.host)}</small></figure>`;

function print(d, kind){
  document.getElementById('bagPrint')?.remove();
  const box = document.createElement('div');
  box.id = 'bagPrint';
  box.innerHTML = kind === 'card' ? piece(d, 'bag-a6') : `<div class="bag-sheet">${Array.from({ length: STICKERS }, () => piece(d, 'bag-sticker')).join('')}</div>`;
  document.body.appendChild(box);
  document.body.classList.add('print-bag');
  const done = () => { document.body.classList.remove('print-bag'); box.remove(); removeEventListener('afterprint', done); };
  addEventListener('afterprint', done);
  window.print();
}

function save(blob, name){
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url; a.download = name;
  document.body.appendChild(a); a.click(); a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

async function svgFile(c){
  // The one-code route, with the owner's token: a plain link could not carry it.
  const r = await fetch('/api/owner/bag/qr.svg' + q(c), { headers: store.t ? { authorization: 'Bearer ' + store.t } : {} });
  if (!r.ok) throw new Error('HTTP ' + r.status);
  return r.text();
}

async function png(c){
  const svg = await svgFile(c);
  const img = new Image();
  await new Promise((ok, no) => { img.onload = ok; img.onerror = no; img.src = svgSrc(svg); });
  const cv = document.createElement('canvas'); cv.width = cv.height = PNG_PX;
  const g = cv.getContext('2d'); g.imageSmoothingEnabled = false; g.drawImage(img, 0, 0, PNG_PX, PNG_PX);
  const blob = await new Promise(ok => cv.toBlob(ok, 'image/png'));
  save(blob, `bag-qr${c ? '-' + c : ''}.png`);
}

function offerForm(d){
  const o = d.offer || { kind: 'off' };
  const kinds = KINDS.filter(k => k !== 'stamps' || d.stamps_on).map(k => ({ value: k, key: 'bag_k_' + k }));
  const dishes = (S.products || []).map(p => ({ value: p.id, label: p.name }));
  return `<section class="group mt-3"><p class="eyebrow" data-t="bag_offer"></p><p class="muted small" data-t="bag_offerHint"></p>
    ${select({ id: 'bag-kind', key: 'bag_kind', value: o.kind, options: kinds, tour: 'bag.kind' })}
    <div class="grid2">${field({ id: 'bag-value', key: 'bag_value', inputmode: 'numeric', value: o.value ?? '', tour: 'bag.value' })}
      ${field({ id: 'bag-min', key: 'bag_min', inputmode: 'numeric', value: o.min ?? '', tour: 'bag.min' })}</div>
    ${select({ id: 'bag-gift', key: 'bag_gift', value: o.product || '', options: dishes, tour: 'bag.gift' })}
    ${field({ id: 'bag-pct', key: 'bag_pct', inputmode: 'numeric', value: d.commission_pct ?? '', hintKey: 'bag_pctHint', tour: 'bag.pct' })}
    <div class="btn-row">${btn({ id: 'bagSave', variant: 'primary', icon: 'check', key: 'save', tour: 'bag.save' })}</div></section>`;
}

function body(d){
  const rows = statRows(d.stats, money).map(([key, v, note]) =>
    rowDiv({ leading: icon('chart-bar'), title: { t: key }, sub: note ? `<span data-t="${note}"></span>` : '', trailing: v == null ? '' : `<b class="mono">${esc(v)}</b>` })).join('');
  return `<p class="warn small bag-warn" data-tour="bag.warn">${icon('alert-triangle')} <span data-t="bag_warn"></span></p>
    ${offerForm(d)}
    <section class="group mt-3"><p class="eyebrow" data-t="bagQrSub"></p>
      ${field({ id: 'bag-c', key: 'bag_campaign', value: view.c, hintKey: 'bag_campaignHint', autocomplete: 'off', tour: 'bag.campaign' })}
      <div class="bag-preview" data-tour="bag.qr">${piece(d, 'bag-a6')}</div>
      <div class="btn-row">${btn({ id: 'bagCard', variant: 'primary', icon: 'download', key: 'bag_card', tour: 'bag.card' })}
        ${btn({ id: 'bagStickers', icon: 'download', key: 'bag_stickers', tour: 'bag.stickers' })}</div>
      <div class="btn-row">${btn({ id: 'bagSvg', variant: 'ghost', icon: 'download', key: 'bag_svg', tour: 'bag.svg' })}
        ${btn({ id: 'bagPng', variant: 'ghost', icon: 'photo', key: 'bag_png', tour: 'bag.png' })}</div></section>
    <section class="group mt-3"><p class="eyebrow" data-t="bag_stats"></p><div class="rows" data-tour="bag.stats">${rows}</div></section>`;
}

function follow(){
  const k = $('#bag-kind')?.value;
  const show = (id, on) => { const el = $(id)?.closest('.ui-field'); if (el) el.hidden = !on; };
  show('#bag-value', k === 'fixed'); show('#bag-min', k === 'fixed'); show('#bag-gift', k === 'gift');
}

function wire(d){
  $('#bag-kind').onchange = follow; follow();
  $('#bagSave').onclick = async () => {
    const b = formBody({ kind: $('#bag-kind').value, value: $('#bag-value').value, min: $('#bag-min').value, product: $('#bag-gift').value, pct: $('#bag-pct').value });
    try { await busy($('#bagSave'), () => post('/owner/bag' + q(), b)); toast(t('saved')); open(); } catch (e) { fail(e); }
  };
  $('#bag-c').onchange = () => { view.c = campaign($('#bag-c').value); open(); };
  $('#bagCard').onclick = () => print(d, 'card');
  $('#bagStickers').onclick = () => print(d, 'stickers');
  $('#bagSvg').onclick = () => svgFile(view.c).then(s => save(new Blob([s], { type: 'image/svg+xml' }), `bag-qr${view.c ? '-' + view.c : ''}.svg`)).catch(fail);
  $('#bagPng').onclick = () => png(view.c).catch(fail);
}

export async function open(){
  ensureCss();
  const head = '<p class="eyebrow" data-t="marketing"></p><h2 data-t="bagQr"></h2><p class="muted small" data-t="bag_hint"></p>';
  sheet(`${head}<div id="bagBody">${loading(2)}</div>`, { name: 'bagQr', keepScroll: true });
  let d;
  try { d = await api('/owner/bag' + q(view.c)); } catch (e) { return fail(e); }
  sheet(`${head}<div id="bagBody">${body(d)}</div>`, { name: 'bagQr', keepScroll: true });
  retranslate($('#sheetIn'));
  wire(d);
}
