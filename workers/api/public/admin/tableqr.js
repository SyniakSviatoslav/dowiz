// TABLE QR CODES (A9): one printable code per table on the floor plan.
//
// The Worker draws every code (`GET /api/owner/tables/qr`,
// services/orders/room/table_qr.rs): the link inside it carries a signature
// only the hub can make, so this page never builds a URL or a QR itself. It
// shows them, prints them, and downloads one as an SVG file
// (`GET /api/owner/tables/:zone/:n/qr.svg`).
//
// PRINTING: the codes are copied into one top-level `#qrPrint` block and the
// page's own stylesheet (`tableqr.css`, loaded once -- the CSP allows no inline
// style) hides everything else on paper. A sheet inside a scrolling modal
// prints clipped; a top-level block does not.
//
// ASCII QUOTES ONLY in this file: a typographic quote once took down the
// whole console.

import { $, $$, esc, icon, t, api, store, toast, sheet } from '/admin/core.js';
import { T, LANGS } from '/admin/i18n.js';
import { btn, empty } from '/admin/parts.js';

const WORDS = {
  sq: { qrHint: 'Nje kod per cdo tavoline te planit. Klienti e skanon, porosit, dhe kamerieri e konfirmon porosine.', qrPrint: 'Printo kodet', qrNone: 'Plani i salles nuk ka ende tavolina. Vizatojeni te Rezervimet.', qrTable: 'Tavolina', qrDownload: 'Shkarko', qrScan: 'Skanoni per te porositur' },
  en: { qrHint: 'One code per table on the plan. A guest scans it, orders, and a waiter confirms the round.', qrPrint: 'Print the codes', qrNone: 'The floor plan has no tables yet. Draw it under Bookings.', qrTable: 'Table', qrDownload: 'Download', qrScan: 'Scan to order' },
  uk: { qrHint: 'Один код на кожен стіл плану. Гість сканує, замовляє, а офіціант підтверджує замовлення.', qrPrint: 'Надрукувати коди', qrNone: 'У плані зали ще немає столів. Намалюйте його в Бронюваннях.', qrTable: 'Стіл', qrDownload: 'Завантажити', qrScan: 'Скануйте, щоб замовити' },
};
for (const l of LANGS) Object.assign(T[l], WORDS[l]);

const fail = e => toast(String(e.message || e));

/// A code as an image the CSP allows (`img-src data:`). encodeURIComponent
/// turns every `#` into `%23` -- a raw `#` in a data URL ends it.
export const svgSrc = svg => 'data:image/svg+xml,' + encodeURIComponent(svg || '');

/// One table's card: the code, the table's name, the words under it.
export const card = x => `<figure class="qr-card">
  <img src="${svgSrc(x.svg)}" alt="${esc(t('qrTable'))} ${esc(x.zone_name || x.zone)} ${esc(x.n)}">
  <figcaption><b>${esc(t('qrTable'))} ${esc(x.n)}</b><small>${esc(x.zone_name || x.zone)} · ${esc(t('qrScan'))}</small></figcaption>
  ${btn({ variant: 'ghost', cls: 'qr-dl', icon: 'download', label: t('qrDownload'), data: { zone: x.zone, n: x.n }, tour: 'tableqr.download' })}
</figure>`;

function ensureCss(){
  if (document.getElementById('qrCss')) return;
  const l = document.createElement('link');
  l.id = 'qrCss'; l.rel = 'stylesheet'; l.href = '/admin/tableqr.css';
  document.head.appendChild(l);
}

function print(list){
  document.getElementById('qrPrint')?.remove();
  const box = document.createElement('div');
  box.id = 'qrPrint';
  box.innerHTML = `<div class="qr-grid">${list.map(card).join('')}</div>`;
  document.body.appendChild(box);
  document.body.classList.add('print-qr');
  const done = () => { document.body.classList.remove('print-qr'); box.remove(); removeEventListener('afterprint', done); };
  addEventListener('afterprint', done);
  window.print();
}

async function download(zone, n){
  // The one-code route, with the owner's token: a plain link could not carry it.
  const r = await fetch(`/api/owner/tables/${encodeURIComponent(zone)}/${encodeURIComponent(n)}/qr.svg`,
    { headers: store.t ? { authorization: 'Bearer ' + store.t } : {} });
  if (!r.ok) throw new Error('HTTP ' + r.status);
  const url = URL.createObjectURL(await r.blob());
  const a = document.createElement('a');
  a.href = url; a.download = `table-${zone}-${n}.svg`;
  document.body.appendChild(a); a.click(); a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

export async function open(){
  ensureCss();
  let d;
  try { d = await api('/owner/tables/qr'); } catch (e) { return fail(e); }
  const list = d.tables || [];
  sheet(`<p class="eyebrow" data-t="settings"></p><h2 data-t="tableQr"></h2><p class="muted small" data-t="qrHint"></p>
    ${list.length ? `<div class="btn-row">${btn({ id: 'qrPrintGo', variant: 'primary', icon: 'download', key: 'qrPrint', tour: 'tableqr.print' })}</div>
    <p class="hint mono">${esc(d.host || '')}</p>
    <div class="qr-grid" data-tour="tableqr.grid">${list.map(card).join('')}</div>` : empty('receipt', { key: 'qrNone' })}`,
    { name: 'tableqr', keepScroll: true });
  const go = $('#qrPrintGo');
  if (go) go.onclick = () => print(list);
  for (const b of $$('.qr-dl', $('#sheetIn'))) b.onclick = () => download(b.dataset.zone, b.dataset.n).catch(fail);
}
