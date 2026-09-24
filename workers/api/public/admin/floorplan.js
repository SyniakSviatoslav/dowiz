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
import { GRID, zoneId, nudge, placeAt, newTable, withShape, check, toWire } from '/lib/floorplan.js';

const WORDS = {
  sq: { fpHint: 'Vizatoni sallat dhe tavolinat. Tërhiqni një tavolinë, ose zgjidheni dhe lëvizeni me shigjeta.',
    fpZone: 'Salla', fpAddZone: 'Sallë e re', fpZoneName: 'Emri i sallës', fpAddTable: 'Tavolinë e re', fpNumber: 'Numri',
    fpSeats: 'Vende', fpShape: 'Forma', fpRect: 'Katrore', fpCircle: 'E rrumbullakët', fpDelete: 'Hiq tavolinën', fpDeleteZone: 'Hiq sallën',
    fpSave: 'Ruaj planin', fpEmpty: 'Asnjë sallë ende. Shtoni të parën.', fpPick: 'Zgjidhni një tavolinë për ta ndryshuar.',
    fpNoName: 'Salla pa emër', fpZoneTwice: 'Dy salla me të njëjtin id', fpBadNumber: 'Numër tavoline i pavlefshëm',
    fpNumberTwice: 'Dy tavolina me të njëjtin numër', fpOffPlan: 'Tavolina del jashtë planit', fpNewZone: 'Salla' },
  en: { fpHint: 'Draw the rooms and their tables. Drag a table, or select it and move it with the arrows.',
    fpZone: 'Room', fpAddZone: 'New room', fpZoneName: 'Room name', fpAddTable: 'New table', fpNumber: 'Number',
    fpSeats: 'Seats', fpShape: 'Shape', fpRect: 'Square', fpCircle: 'Round', fpDelete: 'Remove table', fpDeleteZone: 'Remove room',
    fpSave: 'Save the plan', fpEmpty: 'No rooms yet. Add the first.', fpPick: 'Select a table to change it.',
    fpNoName: 'A room has no name', fpZoneTwice: 'Two rooms share an id', fpBadNumber: 'A table number is not valid',
    fpNumberTwice: 'Two tables share a number', fpOffPlan: 'A table is off the plan', fpNewZone: 'Room' },
  uk: { fpHint: 'Намалюйте зали та столи. Перетягніть стіл або виберіть його і рухайте стрілками.',
    fpZone: 'Зала', fpAddZone: 'Нова зала', fpZoneName: 'Назва зали', fpAddTable: 'Новий стіл', fpNumber: 'Номер',
    fpSeats: 'Місць', fpShape: 'Форма', fpRect: 'Квадратний', fpCircle: 'Круглий', fpDelete: 'Прибрати стіл', fpDeleteZone: 'Прибрати залу',
    fpSave: 'Зберегти план', fpEmpty: 'Ще немає зал. Додайте першу.', fpPick: 'Виберіть стіл, щоб змінити його.',
    fpNoName: 'Зала без назви', fpZoneTwice: 'Дві зали з одним id', fpBadNumber: 'Недійсний номер столу',
    fpNumberTwice: 'Два столи з одним номером', fpOffPlan: 'Стіл виходить за межі плану', fpNewZone: 'Зала' },
};
for (const l of LANGS) Object.assign(T[l], WORDS[l]);

const ui = { zones: [], zi: 0, sel: null, lim: null, drag: false };
const fail = e => toast(String(e.message || e));
const zone = () => ui.zones[ui.zi] || null;
const table = () => zone()?.tables.find(x => x.n === ui.sel) || null;

function ensureCss(){
  if (document.getElementById('rsCss')) return;
  const l = document.createElement('link');
  l.id = 'rsCss'; l.rel = 'stylesheet'; l.href = '/admin/bookings.css';
  document.head.appendChild(l);
}

function tablesSvg(){
  const z = zone(); if (!z) return '';
  const { planW: W, planH: H } = ui.lim;
  let grid = '';
  for (let x = GRID * 4; x < W; x += GRID * 4) grid += `<line class="fp-grid" x1="${x}" y1="0" x2="${x}" y2="${H}"/>`;
  for (let y = GRID * 4; y < H; y += GRID * 4) grid += `<line class="fp-grid" x1="0" y1="${y}" x2="${W}" y2="${y}"/>`;
  return grid + z.tables.map(tb => {
    const body = tb.shape === 'circle'
      ? `<circle class="fp-top" cx="${tb.x}" cy="${tb.y}" r="${tb.w / 2}"/>`
      : `<rect class="fp-top" x="${tb.x - tb.w / 2}" y="${tb.y - tb.h / 2}" width="${tb.w}" height="${tb.h}" rx="6"/>`;
    return `<g class="fp-t ${tb.n === ui.sel ? 'sel' : ''}" data-n="${tb.n}">${body}
      <text x="${tb.x}" y="${tb.y}" text-anchor="middle">${tb.n}</text>
      <text class="fp-s" x="${tb.x}" y="${tb.y + 13}" text-anchor="middle">${tb.seats}</text></g>`;
  }).join('');
}

function controls(){
  const tb = table();
  if (!tb) return `<p class="muted small" data-t="fpPick"></p>`;
  return `<div class="fp-fields">
      <div><label for="fp-n" data-t="fpNumber"></label><input id="fp-n" type="number" min="1" value="${tb.n}"></div>
      <div><label for="fp-seats" data-t="fpSeats"></label><input id="fp-seats" type="number" min="1" max="${ui.lim.maxSeats}" value="${tb.seats}"></div></div>
    <div class="seg" role="radiogroup" aria-label="${esc(t('fpShape'))}">
      ${['rect', 'circle'].map(s => `<button type="button" class="seg-b ${tb.shape === s ? 'on' : ''}" data-shape="${s}" aria-pressed="${tb.shape === s}" data-t="${s === 'rect' ? 'fpRect' : 'fpCircle'}"></button>`).join('')}</div>
    <div class="fp-pad">${[['', ''], ['0,-1', 'chevron-up'], ['', ''], ['-1,0', 'chevron-left'], ['', ''], ['1,0', 'chevron-right'], ['', ''], ['0,1', 'chevron-down'], ['', '']]
      .map(([d, ic]) => d ? `<button type="button" class="btn ghost" data-nudge="${d}" aria-label="${ic}">${icon(ic)}</button>` : '<span></span>').join('')}</div>
    <div class="btn-row"><button type="button" class="btn ghost" id="fpDel">${icon('trash')}<span data-t="fpDelete"></span></button></div>`;
}

function draw(){
  const z = zone();
  const issues = check(ui.zones, ui.lim);
  sheet(`<p class="eyebrow" data-t="roomGroup"></p><h2 data-t="floorPlan"></h2><p class="muted small" data-t="fpHint"></p>
    <div class="chips">${ui.zones.map((x, i) => `<button type="button" class="chip ${i === ui.zi ? 'on' : ''}" data-zi="${i}">${esc(x.name || x.id)}</button>`).join('')}
      <button type="button" class="chip" id="fpAddZone">${icon('plus')}<span data-t="fpAddZone"></span></button></div>
    ${z ? `<label for="fp-zname" data-t="fpZoneName"></label><input id="fp-zname" maxlength="40" value="${esc(z.name)}">
    <svg class="fp-plan" id="fpSvg" viewBox="0 0 ${ui.lim.planW} ${ui.lim.planH}" tabindex="0" role="application" aria-label="${esc(z.name)}">${tablesSvg()}</svg>
    <div class="btn-row"><button type="button" class="btn ghost" id="fpAddT">${icon('plus')}<span data-t="fpAddTable"></span></button>
      <button type="button" class="btn ghost" id="fpDelZ">${icon('trash')}<span data-t="fpDeleteZone"></span></button></div>
    <div id="fpCtl">${controls()}</div>` : `<div class="empty">${icon('category')}<b data-t="fpEmpty"></b></div>`}
    ${issues.length ? `<p class="fp-issues">${issues.map(e => `${esc(t(e.key))}${e.zone ? ` (${esc(e.zone)}${e.n != null ? ` #${esc(e.n)}` : ''})` : ''}`).join('<br>')}</p>` : ''}
    <div class="btn-row"><button class="btn" id="fpSave"${issues.length ? ' disabled' : ''}><span data-t="fpSave"></span></button></div>`,
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
  for (const b of $$('[data-zi]', root)) b.onclick = () => { ui.zi = +b.dataset.zi; ui.sel = null; draw(); };
  $('#fpAddZone', root).onclick = () => {
    const name = `${t('fpNewZone')} ${ui.zones.length + 1}`;
    ui.zones.push({ id: zoneId(name, ui.zones.map(z => z.id)), name, tables: [] });
    ui.zi = ui.zones.length - 1; ui.sel = null; draw();
  };
  $('#fpSave', root).onclick = () => save($('#fpSave', root));
  const z = zone(); if (!z) return;
  $('#fp-zname', root).onchange = e => { z.name = e.target.value.trim(); draw(); };
  $('#fpAddT', root).onclick = () => { const tb = newTable(z, ui.lim); z.tables.push(tb); ui.sel = tb.n; draw(); };
  $('#fpDelZ', root).onclick = async () => {
    const ok = await confirm(t('floorPlan'), `${t('fpDeleteZone')}: ${z.name}`, { danger: true });
    if (ok) { ui.zones.splice(ui.zi, 1); ui.zi = 0; ui.sel = null; }
    draw();
  };
  const svg = $('#fpSvg', root);
  svg.addEventListener('pointerdown', e => {
    const g = e.target.closest('[data-n]'); if (!g) return;
    ui.sel = +g.dataset.n; ui.drag = true; svg.setPointerCapture(e.pointerId); redrawSvg();
  });
  svg.addEventListener('pointermove', e => {
    if (!ui.drag) return;
    const p = svgPoint(svg, e); edit(tb => placeAt(tb, p.x, p.y, ui.lim)); redrawSvg();
  });
  const end = () => { if (ui.drag) { ui.drag = false; draw(); } };
  svg.addEventListener('pointerup', end); svg.addEventListener('pointercancel', end);
  svg.addEventListener('keydown', e => {
    const d = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] }[e.key];
    if (!d || !table()) return;
    e.preventDefault(); edit(tb => nudge(tb, d[0], d[1], ui.lim)); redrawSvg();
  });
  for (const b of $$('[data-nudge]', root)) b.onclick = () => { const [dx, dy] = b.dataset.nudge.split(',').map(Number); edit(tb => nudge(tb, dx, dy, ui.lim)); redrawSvg(); };
  for (const b of $$('[data-shape]', root)) b.onclick = () => { edit(tb => withShape(tb, b.dataset.shape)); draw(); };
  const n = $('#fp-n', root); if (n) n.onchange = () => { const v = Number(n.value) | 0; edit(tb => ({ ...tb, n: v })); ui.sel = v; draw(); };
  const s = $('#fp-seats', root); if (s) s.onchange = () => { edit(tb => ({ ...tb, seats: Number(s.value) | 0 })); draw(); };
  const del = $('#fpDel', root); if (del) del.onclick = () => { z.tables = z.tables.filter(x => x.n !== ui.sel); ui.sel = null; draw(); };
}

/// The hub's answer is the truth: its refusal names the zone and the table.
async function save(btn){
  await busy(btn, async () => {
    try {
      const d = await post('/owner/floorplan', { zones: toWire(ui.zones) });
      toast(`${t('saved')} · ${d.zones} / ${d.tables}`);
    } catch (e) { fail(e); }
  });
}

export async function open(){
  ensureCss();
  let d;
  try { d = await api('/owner/floorplan'); } catch (e) { return fail(e); }
  ui.lim = { planW: d.planW, planH: d.planH, maxSeats: d.maxSeats };
  ui.zones = (d.zones || []).map(z => ({ id: z.id, name: z.name || '', tables: (z.tables || []).map(x => ({ shape: 'rect', ...x })) }));
  ui.zi = 0; ui.sel = null;
  draw();
}
