// The Published-menu card, drawn (W-PUBUI, over BN2). PURE: the answer of
// GET /api/owner/publish in, escaped HTML out -- no DOM, no clock, no network
// -- so it renders in node (publish-view.test.mjs).
//
// WHAT IT SHOWS, and nothing else: whether publishing is on, the generation
// last published (menu / translations / settings), how many objects and
// photos the record holds, the manifest's address on the CDN, and one button.
// When publishing is off the card says so in one plain sentence (pub_offHint)
// and the button is disabled: a tap that can only answer 503 is not offered.
//
// ASCII QUOTES ONLY in this file.
import * as ui from '../lib/ui/index.js';
import { btn, pill, rowDiv, k } from './parts.js';

/// The CDN origin for the console's host: `https://cdn.<platform>` for a venue
/// at `<slug>.<platform>` -- the rule store/shell.js reads with. A workers.dev
/// host, localhost or a bare IP has none, and the link is not drawn.
export function cdnOrigin(loc){
  const host = String((loc && loc.hostname) || '').toLowerCase();
  if (!host || host.endsWith('.workers.dev') || host === 'localhost' || /^[\d.]+$/.test(host)) return '';
  const labels = host.split('.');
  if (labels.length < 3 || labels[0] === 'www') return '';
  return `https://cdn.${labels.slice(1).join('.')}`;
}

/// The manifest's full address, or '' when the object has no venue record yet or the host has no CDN.
export function manifestUrl(d, cdn){
  return d && d.manifest && cdn ? `${cdn}/${d.manifest}` : '';
}

/// The three generations as `menu 12 · translations 3 · settings 5`, each word a data-t span.
export function generationLine(gen){
  const g = Array.isArray(gen) ? gen : [0, 0, 0];
  const part = (key, n) => `<span data-t="${key}">${ui.esc(ui.tr(key))}</span> ${Number(n) | 0}`;
  return `${part('pub_genCatalog', g[0])} · ${part('pub_genWords', g[1])} · ${part('pub_genSettings', g[2])}`;
}

/// The whole card.
export function page(d, cdn){
  const on = !!(d && d.enabled);
  const p = (d && d.published) || {};
  const objects = Object.keys(p.objects || {}).length;
  const photos = Array.isArray(p.media) ? p.media.length : 0;
  const hint = on ? 'pub_onHint' : 'pub_offHint';
  const url = manifestUrl(d, cdn);
  const state = rowDiv({ leading: ui.icon('cloud-upload'), title: k('pub_state'),
    sub: `<span data-t="${hint}">${ui.esc(ui.tr(hint))}</span>`, trailing: pill(on ? 'ok' : 'warn', { key: on ? 'on' : 'off' }) });
  const last = objects
    ? `<p class="small"><span data-t="pub_last">${ui.esc(ui.tr('pub_last'))}</span>: ${generationLine(p.generation)}</p>
       <p class="muted small">${objects} <span data-t="pub_objects">${ui.esc(ui.tr('pub_objects'))}</span> · ${photos} <span data-t="pub_photos">${ui.esc(ui.tr('pub_photos'))}</span></p>`
    : `<p class="muted small" data-t="pub_never">${ui.esc(ui.tr('pub_never'))}</p>`;
  const link = url ? `<p class="small"><a class="mono" id="pubManifest" href="${ui.esc(url)}" target="_blank" rel="noopener">${ui.esc(url)}</a></p>` : '';
  return `<p class="eyebrow" data-t="settings"></p><h2 data-t="pubMenu"></h2><p class="muted small" data-t="pub_hint"></p>
    <section class="group">${state}${last}${link}</section>
    <div class="btn-row">${btn({ id: 'pubNow', icon: 'upload', key: 'pub_now', variant: 'primary', disabled: !on })}</div>`;
}
