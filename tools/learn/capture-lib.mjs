// The pure half of the lesson recorder (tools/learn/capture.mjs): arguments, the host
// refusal, which lesson and app, the write guard, local assets, redaction. No browser
// and no network here, so every rule is proved by capture.test.mjs without a live venue.
import { readFileSync, existsSync, readdirSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { join, dirname, normalize as normPath, extname, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseYaml } from './yaml-lite.mjs';
import { normalize, LANGS, ROLES } from './build-lessons.mjs';

export const REPO = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
/// The operator's venue for recordings (2026-09-24). Any other host needs --host.
export const DEFAULT_HOST = 'https://dubin-sushi.dowiz.org';
export const LOCATION = { 'https://dubin-sushi.dowiz.org': 'dubin-durres' };

/// Each role's app: where it lives, the key its language is read from, the viewport.
export const APPS = {
  owner:   { path: '/admin/',   langKey: 'dw_admin_lang', ready: '#nav:not([hidden])' },
  waiter:  { path: '/room/',    langKey: 'dw_room_lang',  ready: '[data-act="open"], .bar' },
  courier: { path: '/courier/', langKey: 'dw_c_lang',     ready: 'body' },
  guest:   { path: '/',         langKey: 'dw_lang',       ready: '.card' },
};
export const PHONE = { width: 390, height: 844 };
export const DESKTOP = { width: 1280, height: 800 };

/// argv -> options, or { error }. Nothing is read from the environment: a stray HOST=
/// in a shell must not point a recording that writes at another venue.
export function parseArgs(argv) {
  const o = { id: null, langs: [...LANGS], host: DEFAULT_HOST, hostGiven: false, ui: 'local',
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

/// The refusal: the default venue always; anything else only when --host named it
/// explicitly, and only https (or a local stand on http://127.0.0.1 / localhost).
export function checkHost(host, given) {
  let u;
  try { u = new URL(host); } catch { return `not a URL: ${host}`; }
  if (u.origin !== host) return `give the host as an origin (scheme://name), not ${host}`;
  if (host === DEFAULT_HOST) return null;
  if (!given) return `refusing ${host}: recordings run on ${DEFAULT_HOST} unless --host names another`;
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

/// sha256 of a lesson's YAML bytes: recorded beside a capture, compared by the stale-video gate.
export const sourceSha = file => createHash('sha256').update(readFileSync(file)).digest('hex');

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
