// WHAT A MEMBER OF STAFF SEES IN THE HUB -- the PURE half of the console's
// filtering (docs/design/KITCHEN-ACCESS-2026-09-27.md). Operator 2026-09-26:
// the kitchen reaches most of the hub, except money, the venue's revenue,
// customers' personal data, staff admin, legal, data safety, keys and tax.
//
// THE HUB DECIDES, NOT THIS FILE. Every route asks its own door and narrows
// its own answer (`access.rs`); this file only avoids drawing a row that would
// answer 403. An owner (no caps list) sees everything, exactly as before.
//
// Every Venue row (`more.js` GROUPS) and every W-WIRE tile (`wire.js` TILES)
// has a line here; `access.test.mjs` fails on a new row without one.
//
// ASCII QUOTES ONLY in this file (DOWIZ-COMMON-RULES rule 11).
import { TAB_CAPS } from './kitchen-logic.js';

/// The kitchen's three words: any of them opens a row marked KITCHEN.
const KITCHEN = ['advance', 'catalog', 'stock'];
const PASS = ['advance'];
const MENU = ['catalog'];
/// The owner's alone.
const OWNER = [];

/// Venue rows: the words that open each to a member of staff.
export const SECTION_CAPS = {
  inbox: OWNER, bookings: PASS, floorPlan: PASS, tableQr: OWNER,
  promos: OWNER, posts: OWNER, campaigns: OWNER, social: OWNER,
  analytics: OWNER, customers: OWNER, staff: OWNER, exceptions: OWNER,
  learn: KITCHEN,
  venue: OWNER, hours: KITCHEN, deliveryTerms: OWNER, deliveryArea: OWNER, payments: OWNER,
  branding: OWNER, features: OWNER, preview: KITCHEN,
  integrations: OWNER, notifications: OWNER, channels: OWNER, ebills: OWNER,
  printer: PASS, assistant: OWNER, mcp: OWNER, apiKeys: OWNER, cloud: OWNER,
  activation: OWNER, health: OWNER, dpa: OWNER,
};

/// The rows a member of staff may open but not change: the hub refuses the
/// write, so the sheet draws no save.
export const READ_ONLY = ['bookings', 'floorPlan', 'hours'];

/// W-WIRE tiles: the words that open each to a member of staff.
export const TILE_CAPS = {
  messages: OWNER, wallets: OWNER, history: OWNER, tax: OWNER,
  catWords: MENU, brand: OWNER, safety: OWNER, graph: OWNER,
};

const opens = (p, words) => !p.staff || (words || OWNER).some(w => p.caps.has(w));

/// May this principal open the Venue row `key`?
export const canOpen = (p, key) => opens(p, SECTION_CAPS[key]);
/// May this principal open the tile `id`?
export const canTile = (p, id) => opens(p, TILE_CAPS[id]);
/// Does the row open without its save (staff reading an owner's screen)?
export const readOnly = (p, key) => !!p.staff && READ_ONLY.includes(key);

/// `[group, rows]` pairs narrowed to the rows this principal opens; a group
/// left empty is dropped. Rows are `[key, ...]` (more.js GROUPS / SECTIONS).
export function sectionsFor(p, groups){
  return groups.map(([g, rows]) => [g, rows.filter(r => canOpen(p, r[0]))]).filter(([, rows]) => rows.length);
}

/// The tiles this principal opens (`wire.js` TILES rows carry `id`).
export const tilesFor = (p, tiles) => tiles.filter(x => canTile(p, x.id));

/// The console's tabs for this principal. The Venue tab (`more`) is shown
/// when at least one of its rows or tiles opens; the others follow the
/// kitchen's own table (`kitchen-logic.js` TAB_CAPS).
export function tabsFor(p, tabs){
  const venue = Object.keys(SECTION_CAPS).some(k => canOpen(p, k)) || Object.keys(TILE_CAPS).some(k => canTile(p, k));
  return tabs.filter(id => !p.staff || (id === 'more' ? venue : (TAB_CAPS[id] || []).some(w => p.caps.has(w))));
}

/// The lessons worth offering: all of the owner's to an owner; to a member of
/// staff, those whose module is a screen they can open AND change -- a tour
/// through a save button the person does not have would stop half way.
export function lessonsFor(p, lessons, tabs){
  if (!p.staff) return lessons;
  const mine = new Set(tabsFor(p, tabs));
  return lessons.filter(l => mine.has(l.module) || (l.module in SECTION_CAPS && canOpen(p, l.module) && !readOnly(p, l.module)));
}
