// THE FLOOR PLAN EDITOR: zones, and the tables standing in them.
//
// No canvas library. The plan is an SVG in the hub's own coordinates (the
// storefront draws the same numbers), and a table moves on a grid: drag it,
// or select it and nudge with the arrow pad or the arrow keys. The rules that
// are not visual are `lib/floorplan.js` (node-tested); the hub parses the plan
// back on save and refuses it whole, naming the zone and the table.
//
// A ZONE'S ID NEVER CHANGES once made: it is in every booking that names one
// of its tables. Renaming a zone changes what the guest reads, nothing else.
//
// ASCII QUOTES ONLY in this file.

import { $, $$, esc, icon, t, api, post, toast, sheet, confirm, busy } from '/admin/core.js';
import { T, LANGS } from '/admin/i18n.js';
import { ui, btn, iconBtn, field, chips, empty } from '/admin/parts.js';
import { GRID, zoneId, nudge, placeAt, newTable, withShape, check, toWire } from '/lib/floorplan.js';

const WORDS = {
  sq: { fpHint: 'Vizatoni sallat dhe tavolinat. Tërhiqni një tavolinë, ose zgjidheni dhe lëvizeni me shigjeta.',
    fpZone: 'Salla', fpAddZone: 'Sallë e re', fpZoneName: 'Emri i sallës', fpAddTable: 'Tavolinë e re', fpNumber: 'Numri',
    fpSeats: 'Vende', fpShape: 'Forma', fpRect: 'Katrore', fpCircle: 'E rrumbullakët', fpDelete: 'Hiq tavolinën', fpDeleteZone: 'Hiq sallën',
    fpSave: 'Ruaj planin', fpEmpty: 'Asnjë sallë ende. Shtoni të parën.', fpPick: 'Zgjidhni një tavolinë për ta ndryshuar.',
    fpNoName: 'Salla pa emër', fpZoneTwice: 'Dy salla me të njëjtin id', fpBadNumber: 'Numër tavoline i pavlefshëm',
    fpNumberTwice: 'Dy tavolina me të njëjtin numër', fpOffPlan: 'Tavolina del jashtë planit', fpOverlap: 'Dy tavolina mbivendosen', fpNewZone: 'Salla' },
  en: { fpHint: 'Draw the rooms and their tables. Drag a table, or select it and move it with the arrows.',
    fpZone: 'Room', fpAddZone: 'New room', fpZoneName: 'Room name', fpAddTable: 'New table', fpNumber: 'Number',
    fpSeats: 'Seats', fpShape: 'Shape', fpRect: 'Square', fpCircle: 'Round', fpDelete: 'Remove table', fpDeleteZone: 'Remove room',
    fpSave: 'Save the plan', fpEmpty: 'No rooms yet. Add the first.', fpPick: 'Select a table to change it.',
    fpNoName: 'A room has no name', fpZoneTwice: 'Two rooms share an id', fpBadNumber: 'A table number is not valid',
    fpNumberTwice: 'Two tables share a number', fpOffPlan: 'A table is off the plan', fpOverlap: 'Two tables overlap', fpNewZone: 'Room' },
  uk: { fpHint: 'Намалюйте зали та столи. Перетягніть стіл або виберіть його і рухайте стрілками.',
    fpZone: 'Зала', fpAddZone: 'Нова зала', fpZoneName: 'Назва зали', fpAddTable: 'Новий стіл', fpNumber: 'Номер',
    fpSeats: 'Місць', fpShape: 'Форма', fpRect: 'Квадратний', fpCircle: 'Круглий', fpDelete: 'Прибрати стіл', fpDeleteZone: 'Прибрати залу',
    fpSave: 'Зберегти план', fpEmpty: 'Ще немає зал. Додайте першу.', fpPick: 'Виберіть стіл, щоб змінити його.',
    fpNoName: 'Зала без назви', fpZoneTwice: 'Дві зали з одним id', fpBadNumber: 'Недійсний номер столу',
    fpNumberTwice: 'Два столи з одним номером', fpOffPlan: 'Стіл виходить за межі плану', fpOverlap: 'Два столи накладаються', fpNewZone: 'Зала' },
};
for (const l of LANGS) Object.assign(T[l], WORDS[l]);

const fp = { zones: [], zi: 0, sel: null, lim: null, drag: false };
const fail = e => toast(String(e.message || e));
const zone = () => fp.zones[fp.zi] || null;
const table = () => zone()?.tables.find(x => x.n === fp.sel) || null;

function ensureCss(){
  if (document.getElementById('rsCss')) return;
  const l = document.createElement('link');
  l.id = 'rsCss'; l.rel = 'stylesheet'; l.href = '/admin/bookings.css';
  document.head.appendChild(l);
}

function tablesSvg(){
  const z = zone(); if (!z) return '';
  const { planW: W, planH: H } = fp.lim;
  let grid = '';
  for (let x = GRID * 4; x < W; x += GRID * 4) grid += `<line class="fp-grid" x1="${x}" y1="0" x2="${x}" y2="${H}"/>`;
  for (let y = GRID * 4; y < H; y += GRID * 4) grid += `<line class="fp-grid" x1="0" y1="${y}" x2="${W}" y2="${y}"/>`;
  return grid + z.tables.map(tb => {
    const body = tb.shape === 'circle'
      ? `<circle class="fp-top" cx="${tb.x}" cy="${tb.y}" r="${tb.w / 2}"/>`
      : `<rect class="fp-top" x="${tb.x - tb.w / 2}" y="${tb.y - tb.h / 2}" width="${tb.w}" height="${tb.h}" rx="6"/>`;
    return `<g class="fp-t ${tb.n === fp.sel ? 'sel' : ''}" data-n="${tb.n}">${body}
      <text x="${tb.x}" y="${tb.y}" text-anchor="middle">${tb.n}</text>
      <text class="fp-s" x="${tb.x}" y="${tb.y + 13}" text-anchor="middle">${tb.seats}</text></g>`;
  }).join('');
}

function controls(){
  const tb = table();
  if (!tb) return `<p class="muted small" data-t="fpPick"></p>`;
  return `<div class="fp-fields">
      ${field({ id: 'fp-n', key: 'fpNumber', type: 'number', value: tb.n, attrs: { min: 1 }, tour: 'floor.number' })}
      ${field({ id: 'fp-seats', key: 'fpSeats', type: 'number', value: tb.seats, attrs: { min: 1, max: fp.lim.maxSeats }, tour: 'floor.seats' })}</div>
    ${chips({ values: [{ value: 'rect', key: 'fpRect' }, { value: 'circle', key: 'fpCircle' }], value: tb.shape, attr: 'shape', labelKey: 'fpShape', tour: 'floor.shape' })}
    <div class="fp-pad" data-tour="floor.nudge">${[['', ''], ['0,-1', 'chevron-up'], ['', ''], ['-1,0', 'chevron-left'], ['', ''], ['1,0', 'chevron-right'], ['', ''], ['0,1', 'chevron-down'], ['', '']]
      .map(([d, ic]) => d ? iconBtn({ icon: ic, ariaLabel: ic, data: { nudge: d } }) : '<span></span>').join('')}</div>
    <div class="btn-row">${btn({ id: 'fpDel', variant: 'danger', icon: 'trash', key: 'fpDelete', tour: 'floor.deleteTable' })}</div>`;
}

function draw(){
  const z = zone();
  const issues = check(fp.zones, fp.lim);
  sheet(`<p class="eyebrow" data-t="roomGroup"></p><h2 data-t="floorPlan"></h2><p class="muted small" data-t="fpHint"></p>
    <div class="chips" data-tour="floor.zones">${fp.zones.map((x, i) => ui.chip({ as: 'button', selected: i === fp.zi, label: x.name || x.id, attrs: { data: { zi: i } } })).join('')}
      ${ui.chip({ as: 'button', id: 'fpAddZone', icon: 'plus', label: { t: 'fpAddZone' }, attrs: { data: { tour: 'floor.addZone' } } })}</div>
    ${z ? `${field({ id: 'fp-zname', key: 'fpZoneName', maxlength: 40, value: z.name, tour: 'floor.zoneName' })}
    <svg class="fp-plan" id="fpSvg" data-tour="floor.plan" viewBox="0 0 ${fp.lim.planW} ${fp.lim.planH}" tabindex="0" role="application" aria-label="${esc(z.name)}">${tablesSvg()}</svg>
    <div class="btn-row">${btn({ id: 'fpAddT', icon: 'plus', key: 'fpAddTable', tour: 'floor.addTable' })}
      ${btn({ id: 'fpDelZ', variant: 'ghost', icon: 'trash', key: 'fpDeleteZone', tour: 'floor.deleteZone' })}</div>
    <div id="fpCtl">${controls()}</div>` : empty('category', { key: 'fpEmpty' })}
    ${issues.length ? `<p class="fp-issues">${issues.map(e => `${esc(t(e.key))}${e.zone ? ` (${esc(e.zone)}${e.n != null ? ` #${esc(e.n)}` : ''})` : ''}`).join('<br>')}</p>` : ''}
    <div class="btn-row">${btn({ id: 'fpSave', variant: 'primary', icon: 'check', key: 'fpSave', disabled: issues.length > 0, tour: 'floor.save' })}</div>`,
    { name: 'floorplan', keepScroll: true });
  bind();
}

/// Only the drawing, during a drag: rewriting the sheet would drop the pointer.
const redrawSvg = () => { const s = $('#fpSvg'); if (s) s.innerHTML = tablesSvg(); };

function svgPoint(svg, e){
  const p = svg.createSVGPoint(); p.x = e.clientX; p.y = e.clientY;
  return p.matrixTransform(svg.getScreenCTM().inverse());
}

function edit(fn){ const z = zone(), tb = table(); if (!z || !tb) return; z.tables[z.tables.indexOf(tb)] = fn(tb); }

function bind(){
  const root = $('#sheetIn');
  for (const b of $$('[data-zi]', root)) b.onclick = () => { fp.zi = +b.dataset.zi; fp.sel = null; draw(); };
  $('#fpAddZone', root).onclick = () => {
    const name = `${t('fpNewZone')} ${fp.zones.length + 1}`;
    fp.zones.push({ id: zoneId(name, fp.zones.map(z => z.id)), name, tables: [] });
    fp.zi = fp.zones.length - 1; fp.sel = null; draw();
  };
  $('#fpSave', root).onclick = () => save($('#fpSave', root));
  const z = zone(); if (!z) return;
  $('#fp-zname', root).onchange = e => { z.name = e.target.value.trim(); draw(); };
  $('#fpAddT', root).onclick = () => { const tb = newTable(z, fp.lim); z.tables.push(tb); fp.sel = tb.n; draw(); };
  $('#fpDelZ', root).onclick = async () => {
    const ok = await confirm(t('floorPlan'), `${t('fpDeleteZone')}: ${z.name}`, { danger: true });
    if (ok) { fp.zones.splice(fp.zi, 1); fp.zi = 0; fp.sel = null; }
    draw();
  };
  const svg = $('#fpSvg', root);
  svg.addEventListener('pointerdown', e => {
    const g = e.target.closest('[data-n]'); if (!g) return;
    fp.sel = +g.dataset.n; fp.drag = true; svg.setPointerCapture(e.pointerId); redrawSvg();
  });
  svg.addEventListener('pointermove', e => {
    if (!fp.drag) return;
    const p = svgPoint(svg, e); edit(tb => placeAt(tb, p.x, p.y, fp.lim)); redrawSvg();
  });
  const end = () => { if (fp.drag) { fp.drag = false; draw(); } };
  svg.addEventListener('pointerup', end); svg.addEventListener('pointercancel', end);
  svg.addEventListener('keydown', e => {
    const d = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] }[e.key];
    if (!d || !table()) return;
    e.preventDefault(); edit(tb => nudge(tb, d[0], d[1], fp.lim)); redrawSvg();
  });
  for (const b of $$('[data-nudge]', root)) b.onclick = () => { const [dx, dy] = b.dataset.nudge.split(',').map(Number); edit(tb => nudge(tb, dx, dy, fp.lim)); redrawSvg(); };
  for (const b of $$('[data-shape]', root)) b.onclick = () => { edit(tb => withShape(tb, b.dataset.shape)); draw(); };
  const n = $('#fp-n', root); if (n) n.onchange = () => { const v = Number(n.value) | 0; edit(tb => ({ ...tb, n: v })); fp.sel = v; draw(); };
  const s = $('#fp-seats', root); if (s) s.onchange = () => { edit(tb => ({ ...tb, seats: Number(s.value) | 0 })); draw(); };
  const del = $('#fpDel', root); if (del) del.onclick = () => { z.tables = z.tables.filter(x => x.n !== fp.sel); fp.sel = null; draw(); };
}

/// The hub's answer is the truth: its refusal names the zone and the table.
async function save(btn){
  await busy(btn, async () => {
    try {
      const d = await post('/owner/floorplan', { zones: toWire(fp.zones) });
      toast(`${t('saved')} · ${d.zones} / ${d.tables}`);
    } catch (e) { fail(e); }
  });
}

export async function open(){
  ensureCss();
  let d;
  try { d = await api('/owner/floorplan'); } catch (e) { return fail(e); }
  fp.lim = { planW: d.planW, planH: d.planH, maxSeats: d.maxSeats };
  fp.zones = (d.zones || []).map(z => ({ id: z.id, name: z.name || '', tables: (z.tables || []).map(x => ({ shape: 'rect', ...x })) }));
  fp.zi = 0; fp.sel = null;
  draw();
}
