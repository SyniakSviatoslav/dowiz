// THE LOOK, THREE MORE WAYS (W-WIRE row 7): a whole preset at once
// (`POST /api/owner/branding/preset`), colours suggested from a photo
// (`POST /api/owner/branding/extract` -- the BROWSER decodes the image and
// sends 64x64 pixels; the hub only suggests, and every suggestion already
// passes contrast), and removing the logo (`POST /api/owner/logo/clear`).
// Nothing here is applied without a tap on the thing that applies it.
//
// ASCII QUOTES ONLY as delimiters (DOWIZ-COMMON-RULES rule 11).
import { $, $$, esc, icon, t, api, post, busy, toast, confirm, hydrate } from '/admin/core.js';
import { btn, input, rowBtn, loading, empty, pill } from '/admin/parts.js';
import { q, fail, open, paint } from '/admin/wire-core.js';
import { pixelsHex } from '/admin/wire-logic.js';

const SIDE = 64;

/// A photo file as the flat hex pixels `branding/extract` reads.
async function pixelsOf(file){
  const bmp = await createImageBitmap(file);
  const c = document.createElement('canvas');
  c.width = SIDE; c.height = SIDE;
  const g = c.getContext('2d');
  g.drawImage(bmp, 0, 0, SIDE, SIDE);
  return pixelsHex(g.getImageData(0, 0, SIDE, SIDE).data);
}

const swatch = hex => `<span class="swatch" data-bg="${esc(hex)}" aria-hidden="true"></span>`;

export async function mount(host){
  if (!host) return;
  host.innerHTML = loading(2);
  let b;
  try { b = await api('/owner/branding' + q()); } catch (e) { return fail(e); }
  const cur = b.brand || {};
  host.innerHTML = `<p class="eyebrow" data-t="w_presets"></p>
    <div class="rows" role="list">${(b.presets || []).map(p => rowBtn({ data: { preset: p.id }, leading: swatch(p.primary), title: p.label || p.id,
      sub: `${swatch(p.paper)} ${esc(p.typePair || '')}` })).join('')}</div>
    <p class="eyebrow mt-3" data-t="w_fromPhoto"></p><p class="muted small" data-t="w_fromPhotoHint"></p>
    ${input({ type: 'file', id: 'w-photo', accept: 'image/*', key: 'uploadPhoto' })}
    <div class="rows" role="list" id="wSw"></div>
    <p class="eyebrow mt-3" data-t="w_logo"></p>
    <div class="btn-row">${btn({ id: 'wLogoClear', variant: 'danger', icon: 'trash', key: 'w_removeLogo' })}</div>`;
  paint(host); hydrate(host);
  for (const r of $$('[data-preset]', host)) r.onclick = async () => {
    const ok = await confirm(t('w_presets'), r.dataset.preset);
    if (!ok) return openBrandExtra();
    try { await post('/owner/branding/preset' + q(), { preset: r.dataset.preset }); toast(t('saved')); } catch (e) { fail(e); }
    openBrandExtra();
  };
  $('#w-photo').onchange = async e => {
    const f = e.target.files && e.target.files[0]; if (!f) return;
    $('#wSw').innerHTML = loading(2);
    let d;
    try { d = await post('/owner/branding/extract' + q(), { pixels: await pixelsOf(f) }); } catch (err) { $('#wSw').innerHTML = ''; return fail(err); }
    const sw = d.swatches || [];
    $('#wSw').innerHTML = sw.length ? sw.map(s => rowBtn({ data: { hex: s.hex }, leading: swatch(s.hex), title: s.hex, sub: `${esc(s.sharePct)}%`,
      trailing: s.passes ? pill('ok', { key: 'w_readable' }) : pill('warn', { key: 'w_lowContrast' }) })).join('') : empty('photo', { title: d.note || t('w_noColours') });
    paint($('#wSw')); hydrate($('#wSw'));
    for (const s of $$('[data-hex]', $('#wSw'))) s.onclick = async () => {
      try {
        await busy(s, () => post('/owner/branding' + q(), { primary: s.dataset.hex, paper: cur.paper, typePair: cur.typePair }));
        toast(t('saved'));
      } catch (err) { fail(err); }
    };
  };
  $('#wLogoClear').onclick = async () => {
    const ok = await confirm(t('w_logo'), t('w_removeLogoQ'), { danger: true });
    if (ok) { try { await post('/owner/logo/clear' + q(), {}); toast(t('saved')); } catch (e) { fail(e); } }
    openBrandExtra();
  };
}

export const openBrandExtra = () => mount(open('w_brandExtra', 'w_brandExtraHint', 'brandExtra'));
