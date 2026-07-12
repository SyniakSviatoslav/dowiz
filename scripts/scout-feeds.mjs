#!/usr/bin/env node
// scout-feeds — the plane-maintainer's SCOUT sensor for UPSTREAM signal (charter:
// docs/governance/plane-maintainer-agent.md, step 4 "Scout"). Polls the GitHub `.releases.atom`
// feed (+ any extra Atom feeds) of the deps/tools dowiz actually depends on, diffs against a
// per-box cursor, and surfaces NEW upstream releases/advisories since the last run.
//
// This fills the SCOUT hole that `scripts/new-dep-scan.mjs` does NOT cover: new-dep-scan watches
// OUR newly-added deps; scout-feeds watches UPSTREAM releases of the deps we already ship. It is
// the zero-dep form of the RSSHub PILOT (docs/security/redteam-toolset-analysis-2026-07-02.md):
// start with keyless GitHub `.releases.atom` polling; self-host RSSHub only once the watchlist
// needs ≥~5 feed-less / non-GitHub sources.
//
// ADVISORY ONLY — never auto-acts. It surfaces signal for the maintainer/human to triage, exactly
// like the inbox pane of plane-telemetry (content_trust: untrusted-remote). No shell exec on
// remote content, all HTTP via fetch + AbortSignal, zero new deps, Node stdlib only.
//
// Run:  node scripts/scout-feeds.mjs                 (human report grouped by repo)
//       node scripts/scout-feeds.mjs --json          (machine form, stable shape)
//       node scripts/scout-feeds.mjs --update-cursor (advance the cursor to newest per feed)
//       node scripts/scout-feeds.mjs --watchlist f.json  (override the seed watchlist)
//
// Env / test seams (LOUD on stderr, never set in production):
//   SCOUT_FIXTURES=<json>  map of feedUrl -> ("ERROR" | file-path | inline-atom-xml). When set,
//                          feeds are read from the map instead of the network (used by tests).
import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { pathToFileURL } from 'node:url';

export const SCHEMA_VERSION = 1;
const ROOT = process.env.SCOUT_ROOT || process.cwd();
const CURSOR_PATH = join(ROOT, 'loops', 'runs', 'scout-cursor.json');
const TITLE_CAP = 200;
const MAX_ENTRIES_PER_FEED = 15; // most-recent slice; feeds are already reverse-chron
const FETCH_TIMEOUT_MS = 8000;

// ---------------------------------------------------------------------------
// Watchlist seed — the deps/tools dowiz ACTUALLY ships, mapped to their GitHub source repo.
// Seeded from: (a) ~load-bearing deps grepped from apps/api + apps/web package.json (runtime,
// security-, or data-critical), (b) the adopted-tool repos in TOOLING-REGISTRY.md. Override the
// whole list with --watchlist <file.json> (same {name,owner,repo,extraFeeds?} shape).
//
// Feed URL derivation: `https://github.com/<owner>/<repo>/releases.atom` (keyless, no API rate
// limit). `extraFeeds` carries any additional Atom URL (e.g. a GHSA/security advisory feed) —
// GitHub exposes no stable per-repo advisory Atom, so advisory sources are added here explicitly
// as they appear, rather than guessed.
// ---------------------------------------------------------------------------
export const DEFAULT_WATCHLIST = [
  // --- load-bearing runtime / security / data deps (apps/api, apps/web) ---
  { name: 'fastify', owner: 'fastify', repo: 'fastify' },
  { name: 'react', owner: 'facebook', repo: 'react' },
  { name: 'zod', owner: 'colinhacks', repo: 'zod' },
  { name: 'argon2', owner: 'ranisalt', repo: 'node-argon2' }, // auth / password hashing (red-line)
  { name: 'jose', owner: 'panva', repo: 'jose' }, // JWT / RS256 (red-line)
  { name: 'pg', owner: 'brianc', repo: 'node-postgres' }, // DB driver
  { name: 'pg-boss', owner: 'timgit', repo: 'pg-boss' }, // job queue
  { name: 'ws', owner: 'websockets', repo: 'ws' }, // realtime / courier
  { name: 'ioredis', owner: 'redis', repo: 'ioredis' },
  { name: 'sharp', owner: 'lovell', repo: 'sharp' }, // image pipeline (native)
  { name: 'pino', owner: 'pinojs', repo: 'pino' }, // logging
  { name: 'react-router', owner: 'remix-run', repo: 'react-router' },
  { name: 'vite', owner: 'vitejs', repo: 'vite' },
  { name: 'playwright', owner: 'microsoft', repo: 'playwright' }, // E2E proof harness
  // --- adopted agentic-plane tools (TOOLING-REGISTRY.md) ---
  { name: 'browser-use', owner: 'browser-use', repo: 'browser-use' },
  { name: 'ollama', owner: 'ollama', repo: 'ollama' },
  { name: 'mem0', owner: 'mem0ai', repo: 'mem0' },
  { name: 'open-deep-research', owner: 'langchain-ai', repo: 'open_deep_research' },
];

// ---------------------------------------------------------------------------
// Sanitize remote-authored text before ANY terminal/--json surface. Remote Atom titles are
// untrusted DATA (content_trust: untrusted-remote), same discipline as plane-telemetry's inbox:
// strip ANSI/OSC + all control chars, flatten whitespace, cap length. Result is inert quoted
// text, never instructions. (No import from plane-telemetry — a parallel lane edits it.)
// ---------------------------------------------------------------------------
export function sanitizeTitle(input, cap = TITLE_CAP) {
  let s = String(input ?? '');
  /* eslint-disable no-control-regex */
  s = s.replace(/\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)?/g, ''); // OSC sequences
  s = s.replace(/\x1b\[[0-9;?]*[ -/]*[@-~]/g, ''); // CSI sequences
  s = s.replace(/\x1b./g, ''); // any other escape
  s = s.replace(/[\x00-\x1f\x7f]+/g, ' '); // C0 controls + newlines/tabs + DEL
  /* eslint-enable no-control-regex */
  s = s.replace(/\s+/g, ' ').trim();
  if (s.length > cap) s = `${s.slice(0, cap)}…[capped]`;
  return s;
}

function decodeEntities(s) {
  return String(s ?? '')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"')
    .replace(/&#0*39;/g, "'")
    .replace(/&#x0*27;/gi, "'")
    .replace(/&apos;/g, "'")
    .replace(/&amp;/g, '&'); // last — so "&amp;lt;" doesn't become "<"
}

// ---------------------------------------------------------------------------
// Small tolerant Atom parser (regex/string only — no XML dep). Extracts per <entry>: title,
// updated (ISO), link href, id. Never throws on malformed input — returns what it can.
// ---------------------------------------------------------------------------
export function parseAtom(xml) {
  const text = String(xml ?? '');
  const entries = [];
  const entryRe = /<entry\b[^>]*>([\s\S]*?)<\/entry>/g;
  let m;
  while ((m = entryRe.exec(text)) !== null) {
    const body = m[1];
    const title = pick(body, /<title\b[^>]*>([\s\S]*?)<\/title>/);
    const updated = pick(body, /<updated\b[^>]*>([\s\S]*?)<\/updated>/)
      || pick(body, /<published\b[^>]*>([\s\S]*?)<\/published>/);
    const linkHref = (body.match(/<link\b[^>]*\bhref="([^"]*)"/i) || [])[1] || '';
    const id = pick(body, /<id\b[^>]*>([\s\S]*?)<\/id>/);
    entries.push({
      title: decodeEntities(title).trim(),
      updated: decodeEntities(updated).trim(),
      link: decodeEntities(linkHref).trim(),
      id: decodeEntities(id).trim(),
    });
  }
  return entries;
}

function pick(body, re) {
  const mm = body.match(re);
  return mm ? mm[1] : '';
}

// Stable key for an entry: prefer id (Atom guid), fall back to link, then title.
function entryKey(e) {
  return e.id || e.link || e.title || '';
}

// ---------------------------------------------------------------------------
// New-entry detection vs the cursor. An entry is NEW when its `updated` timestamp is strictly
// later than the cursor's lastSeen for that feed AND its key hasn't been seen. No cursor for a
// feed → every entry is new (first-run baseline). Newest-first order preserved.
// ---------------------------------------------------------------------------
export function diffNewEntries(entries, feedCursor) {
  const lastSeen = feedCursor?.lastSeen || '';
  const seenKeys = new Set(feedCursor?.seenKeys || []);
  const fresh = [];
  for (const e of entries) {
    const newerByTime = lastSeen ? String(e.updated) > String(lastSeen) : true;
    const unseen = !seenKeys.has(entryKey(e));
    if (newerByTime && unseen) fresh.push(e);
  }
  // sort newest-first by updated (lexicographic on ISO-8601 is chronological)
  return fresh.sort((a, b) => String(b.updated).localeCompare(String(a.updated)));
}

// Newest `updated` across a set of entries (for cursor advance).
function newestUpdated(entries) {
  let max = '';
  for (const e of entries) if (String(e.updated) > max) max = String(e.updated);
  return max;
}

// ---------------------------------------------------------------------------
// Feed IO — network via fetch+AbortSignal, or fixtures in test mode. Degrades cleanly: any
// failure returns { ok:false, error } so ONE dead feed never fails the whole run.
// ---------------------------------------------------------------------------
function readFixtures() {
  if (!process.env.SCOUT_FIXTURES) return null;
  try {
    const map = JSON.parse(process.env.SCOUT_FIXTURES);
    console.error('[scout-feeds] ⚠️ TEST HOOK ACTIVE — SCOUT_FIXTURES set, reading feeds from fixtures (never set in production)');
    return map;
  } catch {
    console.error('[scout-feeds] ⚠️ SCOUT_FIXTURES set but not valid JSON — ignoring');
    return null;
  }
}

async function fetchFeed(url, fixtures) {
  if (fixtures) {
    if (!(url in fixtures)) return { ok: false, error: 'no fixture for url' };
    const v = fixtures[url];
    if (v === 'ERROR') return { ok: false, error: 'fixture: simulated feed error' };
    try {
      const body = existsSync(v) ? readFileSync(v, 'utf8') : v; // path or inline XML
      return { ok: true, body };
    } catch (e) {
      return { ok: false, error: `fixture read failed: ${e.message}` };
    }
  }
  try {
    const res = await fetch(url, {
      headers: { accept: 'application/atom+xml, application/xml;q=0.9, */*;q=0.5', 'user-agent': 'dowiz-scout-feeds' },
      redirect: 'follow',
      signal: AbortSignal.timeout(FETCH_TIMEOUT_MS),
    });
    if (!res.ok) return { ok: false, error: `HTTP ${res.status}` };
    return { ok: true, body: await res.text() };
  } catch (e) {
    return { ok: false, error: e?.name === 'TimeoutError' ? 'timeout' : String(e?.message || e) };
  }
}

function feedUrlFor(entry) {
  return `https://github.com/${entry.owner}/${entry.repo}/releases.atom`;
}

// All feeds for a watchlist entry: the derived releases.atom + any extraFeeds.
function feedsFor(entry) {
  const feeds = [{ kind: 'releases', url: feedUrlFor(entry) }];
  for (const u of entry.extraFeeds || []) feeds.push({ kind: 'advisory', url: String(u) });
  return feeds;
}

// ---------------------------------------------------------------------------
// Cursor IO — per-box scratch, never the record (like new-dep-scan's baseline).
// ---------------------------------------------------------------------------
function readCursor() {
  try {
    const c = JSON.parse(readFileSync(CURSOR_PATH, 'utf8'));
    if (c?.schema_version === SCHEMA_VERSION && c.feeds) return c;
  } catch { /* missing/corrupt → full rescan */ }
  return { schema_version: SCHEMA_VERSION, feeds: {} };
}

function writeCursor(cursor) {
  mkdirSync(dirname(CURSOR_PATH), { recursive: true });
  writeFileSync(CURSOR_PATH, JSON.stringify(cursor, null, 2));
}

// ---------------------------------------------------------------------------
// Core scan — pure-ish: given a watchlist + cursor, returns the report object. Does NOT write the
// cursor (the caller decides, gated by --update-cursor).
// ---------------------------------------------------------------------------
export async function scan(watchlist, cursor, { fixtures = null } = {}) {
  const results = [];
  for (const entry of watchlist) {
    const repo = `${entry.owner}/${entry.repo}`;
    for (const feed of feedsFor(entry)) {
      const feedCursor = cursor.feeds?.[feed.url];
      const r = await fetchFeed(feed.url, fixtures);
      if (!r.ok) {
        console.error(`[scout-feeds] warn: ${repo} (${feed.kind}) feed failed — ${r.error} (skipping this source, others continue)`);
        results.push({ name: entry.name, repo, kind: feed.kind, url: feed.url, error: r.error, new: [] });
        continue;
      }
      const parsed = parseAtom(r.body).slice(0, MAX_ENTRIES_PER_FEED);
      const fresh = diffNewEntries(parsed, feedCursor).map((e) => ({
        title: sanitizeTitle(e.title),
        updated: e.updated,
        link: e.link,
      }));
      results.push({
        name: entry.name, repo, kind: feed.kind, url: feed.url,
        error: null, new: fresh,
        newestUpdated: newestUpdated(parsed),
        seenKeys: parsed.map(entryKey).filter(Boolean).slice(0, MAX_ENTRIES_PER_FEED),
      });
    }
  }
  return {
    schema_version: SCHEMA_VERSION,
    generated_ts: new Date().toISOString(),
    content_trust: 'untrusted-remote',
    advisory: true, // NEVER authority — scout surfaces signal, it never auto-acts
    feeds_checked: results.length,
    feeds_failed: results.filter((r) => r.error).length,
    total_new: results.reduce((n, r) => n + r.new.length, 0),
    results,
  };
}

// Advance cursor from a scan report (only for feeds that fetched OK).
function advanceCursor(cursor, report) {
  const next = { schema_version: SCHEMA_VERSION, feeds: { ...(cursor.feeds || {}) } };
  for (const r of report.results) {
    if (r.error) continue; // never advance past a feed we couldn't read
    next.feeds[r.url] = {
      lastSeen: r.newestUpdated || cursor.feeds?.[r.url]?.lastSeen || '',
      seenKeys: r.seenKeys || [],
      updated_ts: report.generated_ts,
    };
  }
  return next;
}

// ---------------------------------------------------------------------------
// Best-effort telemetry emit (kind=scout). Non-fatal if the emitter is absent — do NOT import it
// (a parallel lane edits it); shell out with an arg array, shell:false.
// ---------------------------------------------------------------------------
function emitTelemetry(report) {
  const emitter = join(ROOT, 'scripts', 'plane-telemetry.mjs');
  if (!existsSync(emitter)) return;
  try {
    spawnSync(process.execPath, [emitter, 'emit',
      '--kind', 'scout',
      '--outcome', report.feeds_failed ? 'deferred' : 'pass',
      '--target', 'upstream-feeds',
      '--detail', `checked=${report.feeds_checked} new=${report.total_new} failed=${report.feeds_failed}`,
      '--emitter', 'scout-feeds',
    ], { cwd: ROOT, encoding: 'utf8', shell: false, timeout: 10000 });
  } catch { /* telemetry is best-effort — never blocks scout */ }
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------
function renderHuman(report) {
  const lines = [];
  lines.push(`== scout-feeds (advisory — content_trust: untrusted-remote) ==`);
  lines.push(`checked=${report.feeds_checked} feeds · NEW=${report.total_new} · failed=${report.feeds_failed} · ${report.generated_ts}`);
  const withNew = report.results.filter((r) => r.new.length);
  if (!withNew.length) lines.push('  — nothing new since cursor —');
  for (const r of withNew) {
    lines.push(`\n▸ ${r.repo} (${r.kind}) — ${r.new.length} new`);
    for (const e of r.new) lines.push(`    • ${e.updated}  ${e.title}${e.link ? `  <${e.link}>` : ''}`);
  }
  const failed = report.results.filter((r) => r.error);
  if (failed.length) {
    lines.push(`\n  dead feeds (skipped, non-fatal):`);
    for (const r of failed) lines.push(`    ✗ ${r.repo} (${r.kind}) — ${r.error}`);
  }
  return lines.join('\n');
}

// ---------------------------------------------------------------------------
// Arg parsing (mirrors plane-telemetry's minimal parser)
// ---------------------------------------------------------------------------
function parseArgs(argv) {
  const args = { _: [] };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a.startsWith('--')) {
      const key = a.slice(2);
      const next = argv[i + 1];
      if (next === undefined || next.startsWith('--')) args[key] = true;
      else { args[key] = next; i++; }
    } else args._.push(a);
  }
  return args;
}

function loadWatchlist(path) {
  const raw = JSON.parse(readFileSync(path, 'utf8'));
  const list = Array.isArray(raw) ? raw : raw.watchlist;
  if (!Array.isArray(list)) throw new Error('watchlist file must be a JSON array (or {watchlist:[…]})');
  for (const e of list) if (!e.owner || !e.repo) throw new Error('each watchlist entry needs owner+repo');
  return list;
}

const USAGE = `scout-feeds — SCOUT sensor for upstream releases/advisories (schema v${SCHEMA_VERSION}, advisory)
usage: node scripts/scout-feeds.mjs [--json] [--update-cursor] [--watchlist <file>]
  --json           machine-readable report (stable shape)
  --update-cursor  advance loops/runs/scout-cursor.json to newest per feed
  --watchlist f    override the seeded watchlist ([{name,owner,repo,extraFeeds?}])
ADVISORY ONLY — surfaces signal, never auto-acts. Node stdlib only, no new deps.`;

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help || args._[0] === 'help') { console.log(USAGE); return 0; }
  const watchlist = args.watchlist && args.watchlist !== true ? loadWatchlist(String(args.watchlist)) : DEFAULT_WATCHLIST;
  const fixtures = readFixtures();
  const cursor = readCursor();
  const report = await scan(watchlist, cursor, { fixtures });

  if (args.json) console.log(JSON.stringify(report, null, 2));
  else console.log(renderHuman(report));

  if (args['update-cursor']) {
    writeCursor(advanceCursor(cursor, report));
    console.error(`[scout-feeds] cursor advanced → ${CURSOR_PATH}`);
  }
  emitTelemetry(report);
  return 0;
}

const isMain = process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href;
if (isMain) {
  main().then((code) => { process.exitCode = code ?? 0; })
    .catch((e) => { console.error(`[scout-feeds] fatal: ${e.stack || e}`); process.exitCode = 1; });
}
