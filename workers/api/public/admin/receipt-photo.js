// AN INVOICE INTO STOCK (W-OCR, research 2026-10-03 rows P10 and P11's fallback):
// the Stock screen's tools row opens this sheet. Two ways in, one table out:
//
//   From a photo   camera or file -> tesseract.js (`sqi+eng`, LSTM) IN THE
//                  BROWSER, vendored under /lib/ocr and loaded only on this
//                  tap (about 5 MB over the wire, once; the browser caches the
//                  language data) -> receipt-photo-logic.js parses the lines.
//   E-invoice file the UBL 2.1 XML the supplier sent (or a PDF that embeds
//                  it) -> einvoice-logic.js, exact, no OCR.
//
// Every line is matched to a supply -- by what this supplier's invoices were
// confirmed to say before (`supplierAliases`, aliases.rs), else by a shared
// word -- and the OWNER confirms each one. Confirm writes each line through the
// receipt door that already exists, `POST /api/owner/stock/received` (the
// invoice's total, the supplier, the invoice number), then remembers the
// matches on the supplier's card: `POST /api/owner/stock/alias`.
//
// NOTHING LEAVES THE PHONE but those two requests: the photo and the file are
// read on the device. "Read with my AI" is NOT offered: the venue's own AI
// (W-AI) declares no vision model, and no platform key is added for it.

import { $, $$, esc, t, api, post, sheet, closeSheet, busy, toast } from '/admin/core.js';
import { btn, field, select, input, pill, rowDiv, empty, loading } from '/admin/parts.js';
import * as R from '/admin/receipt-photo-logic.js';
import * as E from '/admin/einvoice-logic.js';
// W-STORE (P12) hook: a delivery goes to the storage chosen under Storages -> "Deliveries go to".
import { withStore } from './stock-storages-logic.js';

const fail = e => toast(t(String(e.message || e)));
/// wasm-feature-detect's SIMD probe (a module using v128); no SIMD -> the plain core.
const SIMD = [0, 97, 115, 109, 1, 0, 0, 0, 1, 5, 1, 96, 0, 1, 123, 3, 2, 1, 0, 10, 10, 1, 8, 0, 65, 0, 253, 15, 253, 98, 11];
let reader = null;

/// The OCR worker, made once per page on the first photo.
function ocrWorker(say){
  reader ??= (async () => {
    const T = await import('/lib/ocr/tesseract.esm.min.js');
    const create = T.createWorker || T.default?.createWorker;
    const simd = WebAssembly.validate(new Uint8Array(SIMD));
    const w = await create(['sqi', 'eng'], 1, { workerPath: '/lib/ocr/worker.min.js', corePath: `/lib/ocr/tesseract-core-${simd ? 'simd-' : ''}lstm.js`,
      langPath: '/lib/ocr', gzip: true, workerBlobURL: false, logger: m => say(m) });
    await w.setParameters({ preserve_interword_spaces: '1', tessedit_pageseg_mode: '6' });
    return w;
  })().catch(e => { reader = null; throw e; });
  return reader;
}

/// A PDF's embedded e-invoice: the raw bytes first, then every Flate stream inflated.
async function pdfXml(buf){
  const latin1 = b => Array.from(b, c => String.fromCharCode(c)).join('');
  const all = new Uint8Array(buf), s = latin1(all);
  let x = E.xmlInPdf(s);
  if (x || typeof DecompressionStream !== 'function') return x;
  for (const m of s.matchAll(/stream\r?\n/g)) {
    const end = s.indexOf('endstream', m.index);
    if (end < 0) continue;
    try {
      const out = await new Response(new Blob([all.slice(m.index + m[0].length, end)]).stream().pipeThrough(new DecompressionStream('deflate'))).arrayBuffer();
      x = E.xmlInPdf(latin1(new Uint8Array(out)));
      if (x) return x;
    } catch { /* not a Flate stream */ }
  }
  return '';
}

export async function open(){
  sheet(`<p class="eyebrow" data-t="inv_title"></p><h2 data-t="rp_title"></h2><p class="sheet-hint" data-t="rp_hint"></p>
    <div id="rpBody">${loading(2)}</div>`, { name: 'receiptPhoto' });
  let stock;
  try { stock = await api('/owner/stock'); } catch (e) { $('#rpBody').innerHTML = empty('alert-triangle', { key: 'loadFail', body: String(e.message || e), alert: true }); return; }
  const cards = stock.supplierCards || [], aliases = stock.supplierAliases || {}, supplies = stock.supplies || [];
  $('#rpBody').innerHTML = `
    ${select({ id: 'rpSup', key: 'rp_supplier', value: '', tour: 'receipt.supplier', options: [{ value: '', key: 'rp_pickSupplier' }, ...cards.map(c => ({ value: c.id, label: c.name }))] })}
    ${cards.length ? '' : `<p class="hint" data-t="rp_noCards"></p>${btn({ variant: 'ghost', icon: 'building', key: 'ol_makeCard', data: { stockx: 'suppliers' } })}`}
    ${field({ id: 'rpDoc', key: 'inv_doc', autocomplete: 'off' })}
    <div class="btn-row">${btn({ id: 'rpPhoto', variant: 'primary', icon: 'camera-plus', key: 'rp_photo', tour: 'receipt.photo' })}${btn({ id: 'rpFile', variant: 'ghost', icon: 'upload', key: 'rp_file', tour: 'receipt.einvoice' })}</div>
    ${input({ id: 'rpPhotoIn', type: 'file', accept: 'image/*', hidden: true })}${input({ id: 'rpFileIn', type: 'file', accept: '.xml,.pdf,application/xml,text/xml,application/pdf', hidden: true })}
    <p class="hint" id="rpStatus" role="status"></p>
    <div id="rpLines"></div>
    <div class="btn-row" id="rpGoRow" hidden>${btn({ id: 'rpGo', variant: 'primary', icon: 'check', key: 'rp_confirm', tour: 'receipt.confirm' })}</div>`;
  $('#rpPhotoIn').setAttribute('capture', 'environment');
  let lines = [], nipt = '';
  const status = k => { $('#rpStatus').textContent = k ? t(k) : ''; };
  const known = () => aliases[$('#rpSup').value]?.lines || {};
  const supOf = id => supplies.find(s => s.id === id);

  const draw = () => {
    if (!lines.length) { $('#rpLines').innerHTML = empty('file', { key: 'rp_noLines' }); $('#rpGoRow').hidden = true; return; }
    const opts = [{ value: '', key: 'rp_skip' }, ...supplies.slice().sort((a, b) => String(a.name).localeCompare(String(b.name))).map(s => ({ value: s.id, label: `${s.name} (${s.unit})` }))];
    $('#rpLines').innerHTML = `<div class="rows" role="list">${lines.map((l, i) => {
      const m = R.match(l, known(), supplies);
      l.item = m.item;
      const q = l.qty ? `${String(l.qty.v / 10 ** l.qty.d).replace('.', ',')} ${l.unit}` : '?';
      const facts = [q, l.unit_price != null ? `× ${l.unit_price}` : '', `= ${l.total} L`].filter(Boolean).join(' ');
      const tags = [m.by ? pill(m.by === 'alias' ? 'ok' : 'info', { key: m.by === 'alias' ? 'rp_byAlias' : 'rp_byName' }) : '',
        ...l.flags.map(f => pill(f === 'mismatch' ? 'bad' : 'warn', { key: 'rp_f_' + f }))].join(' ');
      return rowDiv({ title: l.text, sub: `<span class="mono">${esc(facts)}</span> ${tags}`,
        trailing: `${select({ ariaLabel: l.text, value: m.item, options: opts, data: { rpi: String(i) }, tour: i ? undefined : 'receipt.match' })}` });
    }).join('')}</div>`;
    $('#rpGoRow').hidden = false;
    for (const s of $$('[data-rpi]', $('#rpLines'))) s.onchange = () => { lines[Number(s.dataset.rpi)].item = s.value; };
  };
  $('#rpSup').onchange = () => draw();

  const take = (parsed, ownNipt) => {
    lines = parsed.lines; nipt = ownNipt || parsed.nipt || '';
    if (parsed.doc && !$('#rpDoc').value) $('#rpDoc').value = parsed.doc;
    if (!$('#rpSup').value) { const c = E.cardFor({ nipt, supplier: parsed.supplier }, cards, aliases); if (c.id) $('#rpSup').value = c.id; }
    status(lines.length ? 'rp_check' : ''); draw();
  };

  $('#rpPhoto').onclick = () => $('#rpPhotoIn').click();
  $('#rpFile').onclick = () => $('#rpFileIn').click();
  $('#rpPhotoIn').onchange = async e => {
    const f = e.target.files?.[0]; if (!f) return;
    try {
      await busy($('#rpPhoto'), async () => {
        status('rp_loading');
        const w = await ocrWorker(m => { if (m.status === 'recognizing text') $('#rpStatus').textContent = `${t('rp_reading')} ${Math.round((m.progress || 0) * 100)}%`; });
        const { data } = await w.recognize(f);
        take(R.parseText(data.text));
      });
    } catch (err) { status(''); fail(err); } finally { e.target.value = ''; }
  };
  $('#rpFileIn').onchange = async e => {
    const f = e.target.files?.[0]; if (!f) return;
    try {
      const buf = await f.arrayBuffer();
      const head = new TextDecoder().decode(buf.slice(0, 5));
      const src = head === '%PDF-' ? await pdfXml(buf) : new TextDecoder('utf-8').decode(buf);
      if (!src) throw new Error('ei_pdfNoXml');
      const inv = E.parseInvoice(src);
      take(inv, inv.nipt);
    } catch (err) { fail(err); } finally { e.target.value = ''; }
  };

  $('#rpGo').onclick = async () => {
    const card = cards.find(c => c.id === $('#rpSup').value);
    const doc = $('#rpDoc').value.trim();
    const chosen = lines.filter(l => l.item);
    if (!chosen.length) return toast(t('rp_pickSupply'));
    const bodies = [];
    for (const l of chosen) {
      const r = R.receivedBody(l, l.item, supOf(l.item)?.unit, card?.name || '', doc);
      if (r.error) return toast(`${l.text}: ${t(r.error)}`);
      bodies.push(r.body);
    }
    let done = 0;
    try {
      await busy($('#rpGo'), async () => {
        // ONE LINE AT A TIME through the receipt door; a refusal stops here
        // and says how many went in, so a retry does not receive twice.
        for (const b of bodies) { await post('/owner/stock/received', withStore(b)); done++; }
        if (card) await post('/owner/stock/alias', R.aliasBody(card.id, nipt, chosen, known()));
      });
      toast(`${t('rp_wrote')} ${done}`); closeSheet();
    } catch (err) { toast(`${t('rp_wrote')} ${done} / ${bodies.length} · ${String(err.message || err)}`); lines = lines.filter(l => !chosen.slice(0, done).includes(l)); draw(); }
  };
}
