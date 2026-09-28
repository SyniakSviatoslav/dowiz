// The pure half of the lesson recorder (tools/learn/capture.mjs): arguments, the host
// refusal, which lesson and app, the write guard, local assets, redaction. No browser
// and no network here, so every rule is proved by capture.test.mjs without a live venue.
import { readFileSync, existsSync, readdirSync } from 'node:fs';
import { join, dirname, normalize as normPath, extname, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseYaml } from './yaml-lite.mjs';
import { normalize, ROLES } from './build-lessons.mjs';
// The recorder films in the films' languages only (no Russian cut; lib/langs.js MEDIA_LANGS).
import { MEDIA_LANGS as LANGS } from '../../workers/api/public/lib/langs.js';

export const REPO = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
/// The venues a recording may use without --host. The QA hub is the default (operator
/// 2026-09-27: record on qa-durres; dubin-sushi only with every artefact TEST-marked and closed).
export const QA_HOST = 'https://qa-durres.dowiz.org';
export const DUBIN_HOST = 'https://dubin-sushi.dowiz.org';
export const DEFAULT_HOST = QA_HOST;
export const VENUES = [QA_HOST, DUBIN_HOST];
export const LOCATION = { [QA_HOST]: 'qa-durres', [DUBIN_HOST]: 'dubin-durres' };
/// The venue a role records on when --host is not given: the QA hub, for every role.
export const hostFor = () => QA_HOST;
/// Which credentials sign the courier in, per venue (names in /root/.dowiz_owner): the QA hub
/// has its own TEST courier (created 2026-09-27 by lane W-VIDEO); anywhere else COURIER_*.
export const courierKeys = host => host === QA_HOST ? ['QA_HUB_COURIER_PHONE', 'QA_HUB_COURIER_PASSWORD'] : ['COURIER_PHONE', 'COURIER_PASSWORD'];
/// May a lesson's `writes: yes` steps go out without --allow-writes? Only a courier lesson on the
/// QA hub: a shift opened, an order taken and delivered there is what the film is about, and the
/// QA hub is the venue walks may write to (operator 2026-09-26). The owner's writing steps stay
/// blocked even there -- a delete, a logout, a forget or a campaign send would break the lessons
/// filmed after it.
export const autoWrites = (role, host) => role === 'courier' && host === QA_HOST;

/// Each role's app: where it lives, the key its language is read from, the viewport.
export const APPS = {
  owner:   { path: '/admin/',   langKey: 'dw_admin_lang', ready: '#nav:not([hidden]), [data-tour="login.go"]' },
  waiter:  { path: '/room/',    langKey: 'dw_room_lang',  ready: '[data-act="open"], .bar, form[data-form="login"]' },
  courier: { path: '/courier/', langKey: 'dw_c_lang',     ready: 'body' },
  guest:   { path: '/',         langKey: 'dw_lang',       ready: '.card' },
};
export const PHONE = { width: 390, height: 844 };
export const DESKTOP = { width: 1280, height: 800 };

/// argv -> options, or { error }. Nothing is read from the environment: a stray HOST=
/// in a shell must not point a recording that writes at another venue.
export function parseArgs(argv) {
  const o = { id: null, langs: [...LANGS], host: null, hostGiven: false, ui: 'local',
    out: null, desktop: false, lesson: null, dryRun: false, allowWrites: false };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i], v = argv[i + 1];
    if (a === '--host') { o.host = String(v || '').replace(/\/+$/, ''); o.hostGiven = true; i++; }
    else if (a === '--lang') { o.langs = String(v || '').split(',').filter(Boolean); i++; }
    else if (a === '--ui') { o.ui = v; i++; }
    else if (a === '--out') { o.out = v; i++; }
    else if (a === '--lesson') { o.lesson = v; i++; }
    else if (a === '--desktop') o.desktop = true;
    else if (a === '--dry-run') o.dryRun = true;
    else if (a === '--allow-writes') o.allowWrites = true;
    else if (a.startsWith('--')) return { error: `unknown flag ${a}` };
    else if (!o.id) o.id = a;
    else return { error: `unexpected argument ${a}` };
  }
  if (!o.id && !o.lesson) return { error: 'usage: capture.mjs <lesson id> [--lang sq,en,uk] [--out DIR] [--ui local|live] [--host URL] [--lesson FILE] [--desktop]' };
  if (!o.out) return { error: '--out DIR is required' };
  if (!['local', 'live'].includes(o.ui)) return { error: `--ui must be local or live, not ${o.ui}` };
  const bad = o.langs.filter(l => !LANGS.includes(l));
  if (!o.langs.length || bad.length) return { error: `--lang must name ${LANGS.join('/')}, got ${bad.join(',') || 'nothing'}` };
  return o;
}

/// The refusal: the recording venues always; anything else only when --host named it
/// explicitly, and only https (or a local stand on http://127.0.0.1 / localhost).
export function checkHost(host, given) {
  let u;
  try { u = new URL(host); } catch { return `not a URL: ${host}`; }
  if (u.origin !== host) return `give the host as an origin (scheme://name), not ${host}`;
  if (VENUES.includes(host)) return null;
  if (!given) return `refusing ${host}: recordings run on ${VENUES.join(' or ')} unless --host names another`;
  const local = u.protocol === 'http:' && (u.hostname === '127.0.0.1' || u.hostname === 'localhost');
  if (u.protocol !== 'https:' && !local) return `refusing ${host}: https only`;
  return null;
}

/// The lesson file for an id: docs/learn/lessons/<role>/<id>.yaml.
export function lessonFile(id, root = REPO) {
  for (const role of ROLES) {
    const f = join(root, 'docs/learn/lessons', role, `${id}.yaml`);
    if (existsSync(f)) return f;
  }
  return null;
}

/// How long a step stays on screen: long enough to read ITS caption in the film's language
/// (45 ms a character), never under 2.8 s or over 6.5 s.
export const dwell = (s, lang) => Math.min(6500, Math.max(2800, 45 * String(s.caption[lang] ?? '').length));

/// Parse and validate one lesson with the SAME rules the in-app build applies.
export function loadLesson(file, root = REPO) {
  const rel = file.startsWith(root + sep) ? file.slice(root.length + 1) : file;
  const parts = rel.split(sep).join('/').split('/');
  const errs = [];
  let doc;
  try { doc = parseYaml(readFileSync(file, 'utf8')); } catch (e) { return { errors: [`${rel}: ${e.message}`] }; }
  const lesson = normalize(doc, parts.slice(-2).join('/'), errs);
  return errs.length ? { errors: errs } : { lesson };
}

const READS = new Set(['GET', 'HEAD', 'OPTIONS']);
/// Sign-in and token refresh are the only writes a read-only step may make.
const AUTH = /^\/api\/(auth\/(login|refresh)|staff\/login|courier\/(auth\/)?login)$/;

/// May this request go out during this step? By default NOTHING the recording does may change
/// the venue: every mutating /api/ call is aborted and listed in the step's mark, including a
/// `writes: yes` step's (a campaign send, a fiscal arm, a floor-plan save cannot be taken back on
/// the real venue). The video still shows the control and the tap. With --allow-writes a
/// `writes: yes` step's calls go out and are recorded for the closing sweep.
export function guard(step, method, url, allowWrites = false) {
  let path;
  try { path = new URL(url).pathname; } catch { return 'allow'; }
  if (!path.startsWith('/api/') || READS.has(method) || AUTH.test(path)) return 'allow';
  return allowWrites && step && step.writes ? 'record' : 'block';
}

const TYPES = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8', '.css': 'text/css; charset=utf-8', '.json': 'application/json',
  '.svg': 'image/svg+xml', '.png': 'image/png', '.webmanifest': 'application/manifest+json',
  '.woff2': 'font/woff2', '.jpg': 'image/jpeg', '.webp': 'image/webp', '.ico': 'image/x-icon' };
export const typeOf = f => TYPES[extname(f).toLowerCase()] || 'application/octet-stream';

/// `--ui local`: a static path answered from the working tree's public/ (the anchored
/// markup is not deployed yet), the API untouched. Answers a file path or null.
export function localAsset(urlPath, publicDir) {
  if (urlPath.startsWith('/api/')) return null;
  let p = decodeURIComponent(urlPath.split('?')[0]);
  if (p.endsWith('/')) p += 'index.html';
  if (p === '/index.html') p = '/store/index.html';
  const f = normPath(join(publicDir, p));
  if (!f.startsWith(publicDir + sep)) return null;
  return existsSync(f) && !readdirSafe(f) ? f : null;
}
function readdirSafe(f) { try { readdirSync(f); return true; } catch { return false; } }

/// Tokens never reach a log: JWTs, Bearer values, and the known password fields.
export function redact(s) {
  return String(s)
    .replace(/eyJ[\w-]+\.[\w-]+\.[\w-]+/g, '<jwt>')
    .replace(/(Bearer\s+)\S+/gi, '$1<token>')
    .replace(/"(password|jwt|token|access_token|refresh_token)"\s*:\s*"[^"]*"/g, '"$1":"<redacted>"');
}

/// The plan the driver follows: which steps it drives, which it skips and why.
export function plan(lesson) {
  return lesson.steps.map(s => ({ n: s.n, key: s.key, anchor: s.anchor, writes: s.writes,
    do: s.pending ? 'skip' : s.action.do, selector: s.action.selector, value: s.action.value ?? null,
    why: s.pending ? 'pending: the markup does not carry this anchor yet' : null }));
}

/// How to REACH a control that is not on screen. The in-app tour starts where the reader already
/// is (the lessons list sits in the owner's Venue tab), so a lesson's YAML names only the control
/// it teaches; the recorder starts on the app's first screen and, when a step's anchor is absent,
/// taps these anchors first (the tab, then the row or card that opens the sheet). Keyed by the
/// anchor or its leading parts (the longest key wins); the taps are filmed -- they are the way there.
export const REACH = {
  owner: {
    'more.tile': ['nav.more'],
    orders: ['nav.orders'], order: ['nav.orders', 'orders.row'], state: ['header.state'],
    // the menu's categories start folded: a category row opens (the dishes show), a dish row opens its sheet
    menu: ['nav.menu'], 'menu.dish': ['nav.menu', 'menu.category'], category: ['nav.menu', 'menu.category'], newDish: ['nav.menu', 'menu.addDish'],
    dish: ['nav.menu', 'menu.category', 'menu.dish'], recipe: ['nav.menu', 'menu.category', 'menu.dish', 'dish.ingredients'],
    taste: ['nav.menu', 'menu.category', 'menu.dish', 'dish.ingredients'],
    import: ['nav.menu', 'menu.import'],
    stock: ['nav.stock'], supply: ['nav.stock', 'stock.addSupply'], bulk: ['nav.stock', 'stock.import'],
    // an ingredient's card, and the movement form its Delivery button opens
    card: ['nav.stock', 'stock.supply'], move: ['nav.stock', 'stock.supply', 'card.received'],
    'stock.received': ['nav.stock', 'stock.supply', 'card.received'], 'stock.wasted': ['nav.stock', 'stock.supply', 'card.received'],
    'stock.stocktake': ['nav.stock', 'stock.supply', 'card.received'],
    couriers: ['nav.couriers'], kitchen: ['nav.kitchen'],
    // the e-bills sheet is a Venue tile; the fiscal sheet opens from a row in it
    // customers: the list is a Venue tile, a row opens the reveal sheet, its Card button the card
    customers: ['nav.more', 'more.tile.customers'], 'customers.revealRow': ['nav.more', 'more.tile.customers', 'customers.revealLog'],
    'customers.card': ['nav.more', 'more.tile.customers', 'customers.row'], 'customers.reveal': ['nav.more', 'more.tile.customers', 'customers.row'],
    'customers.revealReason': ['nav.more', 'more.tile.customers', 'customers.row'],
    ...Object.fromEntries(['note', 'allergens', 'tags', 'table', 'lang', 'birthday', 'save', 'link', 'linkPick', 'linkReason', 'unlink', 'withdraw', 'forget']
      .map(x => ['customers.' + x, ['nav.more', 'more.tile.customers', 'customers.row', 'customers.card']])),
    exceptions: ['nav.more', 'more.tile.exceptions'],
    // marketing: each list is a Venue tile; the stamp card sits under the promo codes
    promos: ['nav.more', 'more.tile.promos'], campaigns: ['nav.more', 'more.tile.campaigns'], posts: ['nav.more', 'more.tile.posts'], social: ['nav.more', 'more.tile.social'],
    stamps: ['nav.more', 'more.tile.promos'],
    // settings: the WhatsApp half of the bell is its sheet; Telegram is a screen one row further in
    notify: ['nav.more', 'more.tile.notifications'],
    ...Object.fromEntries(['telegramState', 'tgToken', 'tgChat', 'test'].map(x => ['notify.' + x, ['nav.more', 'more.tile.notifications', 'notify.telegram']])),
    printer: ['nav.more', 'more.tile.printer'], channels: ['nav.more', 'more.tile.channels'], keys: ['nav.more', 'more.tile.apiKeys'],
    mcp: ['nav.more', 'more.tile.mcp'], inbox: ['nav.more', 'more.tile.inbox'], dpa: ['nav.more', 'more.tile.dpa'],
    ebills: ['nav.more', 'more.tile.ebills'], fiscal: ['nav.more', 'more.tile.ebills', 'ebills.fiscal'],
  },
  // an open panel (earnings, history, lessons) covers the waiting screen: its back button returns there
  courier: { 'panel.earnings': ['panel.back'], 'panel.history': ['panel.back'], 'panel.learn': ['panel.back'],
    'panel.agent': ['panel.back'], 'shift.end': ['panel.back'], ask: ['panel.back'] },
  // the room opens on the list of tables: a table opens its sitting, a round row its round sheet;
  // every inner screen's Back returns to the room
  waiter: { room: ['nav.back'], sitting: ['nav.back', 'room.table'], round: ['room.table', 'sitting.round'], guest: ['room.table', 'sitting.round'],
    pay: ['room.table', 'sitting.round', 'round.pay'], transfer: ['room.table', 'sitting.round', 'round.transfer'],
    move: ['room.table', 'sitting.round', 'round.moveSitting'], till: ['room.till'], floor: ['room.floor'], pass: ['room.pass'],
    open: ['room.open'], agent: ['hud.agent'] },
  guest: { menu: ['nav.menu'], dish: ['nav.menu', 'menu.dish'], orders: ['nav.orders'], booking: ['nav.book'],
    // the basket and the checkout need a dish in the basket: one quick add (the basket lives in the page, nothing is sent)
    cart: ['nav.menu', 'menu.quickAdd', 'cart.open'], 'checkout.open': ['nav.menu', 'menu.quickAdd', 'cart.open'],
    checkout: ['nav.menu', 'menu.quickAdd', 'cart.open', 'checkout.open'], map: ['nav.menu', 'menu.quickAdd', 'cart.open', 'checkout.open', 'checkout.map'] },
};
export function reachFor(role, anchor) {
  if (!anchor) return [];
  const map = REACH[role] || {};
  const parts = anchor.split('.');
  for (let i = parts.length; i >= 1; i--) { const k = parts.slice(0, i).join('.'); if (map[k]) return map[k]; }
  return [];
}

/// Was a step's anchor on screen? A card-only step (no anchor, `wait` over the whole screen)
/// has nothing to find and counts as found; a pending (skipped) step never is; an anchored
/// step is found when its element was.
export function stepFound(st, el) {
  if (st.do === 'skip') return false;
  return st.selector ? !!el : true;
}

/// A step's mark: when it started, when the next began, and what the page did.
export function mark(n, key, startMs, found, blocked = []) {
  return { n, key, startMs: Math.max(0, Math.round(startMs)), found, blocked };
}

/// Close each mark at the next one's start, and the last at `endMs`.
export function closeMarks(marks, endMs) {
  return marks.map((m, i) => ({ ...m, endMs: Math.max(m.startMs + 1, Math.round(i + 1 < marks.length ? marks[i + 1].startMs : endMs)) }));
}
