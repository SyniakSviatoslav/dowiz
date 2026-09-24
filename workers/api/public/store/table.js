// ORDERING AT THE TABLE, from the QR code on it (A9).
//
// The code opens `/store/?t=<zone>.<n>.<sig>`. The signature is the Worker's
// (`services/orders/room/table_link.rs`); this file never checks it and never
// trusts its own reading of the zone and number for anything but the words on
// screen. The basket carries the code back as `table_link`, and the server
// decides the table and the sitting from it.
//
// REMEMBERED FOR THE VISIT, not for ever: sessionStorage, so the next time the
// same phone opens the storefront at home it is a delivery again. Every read
// and write is wrapped -- private mode throws, and a table that cannot be
// remembered is still the table in the URL.

import { state, API, moneyEl } from '/store/state.js';
import { t, retranslate } from '/store/i18n.js';
import { esc, icon } from '/store/ui.js';

const KEY = 'dw_table';
const CODE = /^([a-z0-9_-]+)\.(\d+)\.([0-9a-f]{16})$/;

/// `{code, zone, n}` or null. The shape only; the server checks the rest.
export function parseTable(code){
  const m = CODE.exec(String(code || '').trim());
  return m ? { code: m[0], zone: m[1], n: Number(m[2]) } : null;
}

function read(){
  let fromUrl = null;
  try { fromUrl = parseTable(new URLSearchParams(location.search).get('t')); } catch {}
  if (fromUrl) {
    try { sessionStorage.setItem(KEY, fromUrl.code); } catch {}
    return fromUrl;
  }
  try { return parseTable(sessionStorage.getItem(KEY)); } catch { return null; }
}

/// The table this visit orders to, or null.
export const TABLE = read();
if (TABLE) state.how = 'table';

/// "Table salla 4" -- the zone id is the venue's own word, never translated.
export const tableLabel = tb => `${t('tblTable')} ${tb.zone} ${tb.n}`;

/// The line the cart and the checkout show while ordering to a table.
export function tableBanner(){
  if (!TABLE) return '';
  return `<p class="geo ok" id="tableLine">${icon('map-pin')}<span><span data-t="tblAt"></span>: <b>${esc(tableLabel(TABLE))}</b></span></p>`;
}

/// The body fields a table round sends. The server overwrites `table` with
/// the one the signature names; it is sent so an older server still reads it.
export function tableBody(note){
  return {
    fulfilment: { kind: 'dine_in', table: `${TABLE.zone}:${TABLE.n}`, note: note || null },
    table_link: TABLE.code,
    payment: 'cash',
  };
}

// ── the table's bill, read-only, on the tracking sheet ─────────────────────

/// A placeholder the tracking sheet draws for a guest's round.
export const billMarkup = order => (order?.sitting_id && order?.placed_by === 'guest')
  ? `<section class="credits-wrap" id="tableBill"><p class="eyebrow" data-t="tblBill"></p><p class="muted" data-t="tblWait"></p><div id="tableBillIn"></div></section>` : '';

/// Fill it from `GET /api/order/:id/sitting` with the order's own token.
export async function mountBill(order, tok){
  const box = document.getElementById('tableBillIn');
  if (!box || !tok) return;
  try {
    const r = await fetch(`${API}/order/${encodeURIComponent(order.id)}/sitting`, { headers: { authorization: 'Bearer ' + tok } });
    if (!r.ok) return;
    const s = await r.json();
    const rows = (s.rounds || []).map((x, i) => `<div class="row"><span>#${i + 1} · <span data-t-st="${esc(x.status)}">${esc(x.status)}</span></span>${moneyEl(x.total || 0)}</div>`).join('');
    const el = document.getElementById('tableBillIn');
    if (!el) return;
    el.innerHTML = `<div class="totals">${rows}<div class="row grand"><span data-t="total"></span>${moneyEl(s.bill || 0)}</div></div>`;
    retranslate(el);
  } catch { /* the order itself is still on screen */ }
}
