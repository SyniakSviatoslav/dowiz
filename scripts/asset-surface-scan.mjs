#!/usr/bin/env node
// asset-surface-scan — READ-ONLY Certificate-Transparency asset-surface scout (ADOPT:
// docs/security/redteam-toolset-analysis-2026-07-02.md — crt.sh, highest value-to-effort).
//
// Queries crt.sh's public JSON API for the operator's OWN domains, extracts the unique set of
// (sub)domains that have ever been issued a public certificate, and diffs that set against a
// per-box baseline. The actionable signal is a NEW subdomain since the baseline — a forgotten
// staging/preview host quietly exposed by a cert. This is benign, public, read-only recon on
// domains the operator controls: NO offensive action, NO scanning of the hosts themselves.
//
// It is ADVISORY: it never changes anything except (on --update-baseline) the local baseline
// file. It emits one plane-telemetry `scout` event when that emitter is present (best-effort).
//
// Subcommands / flags:
//   (default)            scan the domain allowlist, diff against baseline, print human report
//   --json               emit the machine-readable report instead of the human one
//   --update-baseline    accept the current surface as the new baseline (merges: errored
//                        domains keep their prior baseline entry, never wiped by a bad fetch)
//   --domains a,b,c      override the default domain allowlist (comma-separated)
//   --timeout MS         per-request crt.sh timeout (default 15000)
//   --test-fixture PATH  test seam: read crt.sh response bodies from a JSON map { domain: body }
//                        instead of the network (body is the RAW text crt.sh would return, so
//                        the same parse path is exercised). Also honoured via ASSET_SCAN_FIXTURE.
//   help | --help        usage
//
// Exit codes: 0 = scanned (report printed, NEW subdomains are a signal not an error);
//             1 = every domain failed to fetch (crt.sh down / unreachable) — degrade, never crash;
//             2 = bad usage.
//
// Node stdlib only. No new deps. No shell. All HTTP via global fetch + AbortSignal timeout.
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { pathToFileURL } from 'node:url';

export const SCHEMA_VERSION = 1;

// The operator's OWN domains (verified against docs/deploy, docs/compliance, .env.example).
// Overridable via --domains. Add here as new tenant/preview domains come online.
export const DEFAULT_DOMAINS = [
  'dowiz.org',
  'dowiz.com',
  'dowiz.fly.dev',
  'dowiz-staging.fly.dev',
];

const ROOT = process.env.ASSET_SCAN_ROOT || process.cwd();
const BASELINE_PATH = join(ROOT, 'loops', 'runs', 'asset-surface-baseline.json');
const DEFAULT_TIMEOUT = 15000;

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------
const nowIso = () => new Date().toISOString();

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

/** Normalize a raw cert name to a comparable host: trim, lowercase, strip a leading wildcard. */
function normalizeName(raw) {
  return String(raw ?? '').trim().toLowerCase().replace(/^\*\./, '');
}

// ---------------------------------------------------------------------------
// Fetch + parse (crt.sh JSON). Fetch returns RAW text so the parse path is identical
// whether the bytes came from the network or from a test fixture.
// ---------------------------------------------------------------------------
async function fetchCertText(domain, { fixture, timeoutMs }) {
  if (fixture) {
    // Fixture is the RAW body crt.sh would return; a domain absent from the map == empty result.
    return Object.prototype.hasOwnProperty.call(fixture, domain) ? String(fixture[domain]) : '[]';
  }
  const url = `https://crt.sh/?q=%25.${encodeURIComponent(domain)}&output=json`;
  const res = await fetch(url, {
    headers: { accept: 'application/json', 'user-agent': 'dowiz-asset-surface-scan (read-only CT scout)' },
    signal: AbortSignal.timeout(timeoutMs),
  });
  if (!res.ok) throw new Error(`crt.sh HTTP ${res.status}`);
  return await res.text();
}

/**
 * Extract the sorted unique set of (sub)domains under `domain` from a crt.sh JSON body.
 * Malformed / non-array bodies degrade to { names: [], parseError: true } — never throw.
 */
export function extractSubdomains(text, domain) {
  let data;
  try { data = JSON.parse(String(text ?? '')); } catch { return { names: [], parseError: true }; }
  if (!Array.isArray(data)) return { names: [], parseError: true };
  const set = new Set();
  for (const cert of data) {
    if (!cert || typeof cert !== 'object') continue;
    // name_value can carry several \n-separated SANs; common_name is a single host.
    const candidates = [cert.common_name, ...String(cert.name_value ?? '').split('\n')];
    for (const raw of candidates) {
      const n = normalizeName(raw);
      if (!n) continue;
      if (n === domain || n.endsWith(`.${domain}`)) set.add(n);
    }
  }
  return { names: [...set].sort(), parseError: false };
}

// ---------------------------------------------------------------------------
// Baseline I/O (per-box scratch under loops/runs/ — gitignored)
// ---------------------------------------------------------------------------
export function readBaseline() {
  try {
    const b = JSON.parse(readFileSync(BASELINE_PATH, 'utf8'));
    if (b && typeof b === 'object' && b.domains) return b;
  } catch { /* missing / corrupt → treated as no baseline */ }
  return null;
}

function writeBaseline(baseline) {
  mkdirSync(dirname(BASELINE_PATH), { recursive: true });
  writeFileSync(BASELINE_PATH, `${JSON.stringify(baseline, null, 2)}\n`);
}

// ---------------------------------------------------------------------------
// Core: scan every domain, then diff against baseline
// ---------------------------------------------------------------------------
export async function scanDomains(domains, opts = {}) {
  const timeoutMs = opts.timeoutMs || DEFAULT_TIMEOUT;
  const fixture = opts.fixture;
  const results = {};
  for (const domain of domains) {
    try {
      const text = await fetchCertText(domain, { fixture, timeoutMs });
      const { names, parseError } = extractSubdomains(text, domain);
      results[domain] = parseError
        ? { names: [], error: 'malformed response (not a JSON array)' }
        : { names };
    } catch (e) {
      results[domain] = { names: [], error: String(e?.message ?? e) };
    }
  }
  return results;
}

/** Build the report object: per-domain totals + NEW (vs baseline) + summary. Pure, testable. */
export function buildReport(domains, results, baseline) {
  const base = baseline?.domains ?? {};
  const report = {
    schema_version: SCHEMA_VERSION,
    generated_ts: nowIso(),
    has_baseline: Boolean(baseline),
    baseline_ts: baseline?.updated_ts ?? null,
    domains: {},
    summary: { total_subdomains: 0, total_new: 0, errored_domains: [], scanned_domains: 0 },
  };
  for (const domain of domains) {
    const r = results[domain] ?? { names: [], error: 'not scanned' };
    if (r.error) {
      report.domains[domain] = { total: 0, subdomains: [], new: [], error: r.error };
      report.summary.errored_domains.push(domain);
      continue;
    }
    const known = new Set(base[domain] ?? []);
    // With no baseline at all, nothing is "new" yet — establishing the baseline is the next step.
    const fresh = baseline ? r.names.filter((n) => !known.has(n)) : [];
    report.domains[domain] = { total: r.names.length, subdomains: r.names, new: fresh };
    report.summary.total_subdomains += r.names.length;
    report.summary.total_new += fresh.length;
    report.summary.scanned_domains += 1;
  }
  return report;
}

// ---------------------------------------------------------------------------
// plane-telemetry emit (best-effort; a parallel lane may be editing it — spawn, never import)
// ---------------------------------------------------------------------------
function emitTelemetry(outcome, detail) {
  const tp = join(ROOT, 'scripts', 'plane-telemetry.mjs');
  if (!existsSync(tp)) return;
  try {
    spawnSync(process.execPath, [
      tp, 'emit', '--kind', 'scout', '--outcome', outcome,
      '--emitter', 'asset-scan', '--detail', detail.slice(0, 240),
    ], { cwd: ROOT, stdio: 'ignore', timeout: 12000 });
  } catch { /* telemetry is advisory — never fatal */ }
}

// ---------------------------------------------------------------------------
// Human report
// ---------------------------------------------------------------------------
function printHuman(report) {
  const lines = [];
  lines.push(`== asset-surface scan (crt.sh, read-only CT) — ${report.generated_ts} ==`);
  lines.push(report.has_baseline
    ? `baseline: ${report.baseline_ts} · ${BASELINE_PATH}`
    : `baseline: NONE — run with --update-baseline to establish one (NEW-detection starts next run)`);
  lines.push('');
  for (const [domain, d] of Object.entries(report.domains)) {
    if (d.error) { lines.push(`  ✗ ${domain} — ERROR: ${d.error}`); continue; }
    if (d.total === 0) { lines.push(`  · ${domain} — no certs found (check domain)`); continue; }
    const flag = d.new.length ? ` · ${d.new.length} NEW` : '';
    lines.push(`  ${d.new.length ? '▲' : '·'} ${domain} — ${d.total} subdomain(s)${flag}`);
    for (const n of d.new) lines.push(`      NEW → ${n}`);
  }
  lines.push('');
  const s = report.summary;
  lines.push(`summary: ${s.total_subdomains} subdomain(s) across ${s.scanned_domains} domain(s); ${s.total_new} NEW; ${s.errored_domains.length} errored`);
  if (s.total_new) lines.push(`⚠ ${s.total_new} NEW subdomain(s) since baseline — investigate forgotten/preview hosts (advisory).`);
  console.log(lines.join('\n'));
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------
const USAGE = `asset-surface-scan — read-only crt.sh Certificate-Transparency asset scout (schema v${SCHEMA_VERSION})
usage: node scripts/asset-surface-scan.mjs [--json] [--update-baseline] [--domains a,b,c] [--timeout MS]
default domains: ${DEFAULT_DOMAINS.join(', ')}
NEW subdomains since baseline are the actionable signal (forgotten staging/preview hosts).
Advisory only: nothing is changed except the local baseline on --update-baseline.`;

function loadFixture(args) {
  const path = typeof args['test-fixture'] === 'string' ? args['test-fixture'] : process.env.ASSET_SCAN_FIXTURE;
  if (!path) return undefined;
  console.error(`[asset-surface-scan] ⚠️ TEST FIXTURE ACTIVE — reading crt.sh bodies from ${path} (never set in production)`);
  return JSON.parse(readFileSync(path, 'utf8'));
}

export async function main(argv) {
  const args = parseArgs(argv);
  if (args.help || args._[0] === 'help') { console.log(USAGE); return 0; }

  const domains = typeof args.domains === 'string'
    ? args.domains.split(',').map((s) => s.trim()).filter(Boolean)
    : DEFAULT_DOMAINS;
  if (!domains.length) { console.error('[asset-surface-scan] error: empty domain list'); return 2; }

  const timeoutMs = args.timeout ? Number(args.timeout) : DEFAULT_TIMEOUT;
  let fixture;
  try { fixture = loadFixture(args); }
  catch (e) { console.error(`[asset-surface-scan] error: could not read fixture: ${e.message}`); return 2; }

  const baseline = readBaseline();
  const results = await scanDomains(domains, { fixture, timeoutMs });
  const report = buildReport(domains, results, baseline);

  // Degrade cleanly: every domain failed → crt.sh is down/unreachable → non-zero, clear message.
  const allErrored = report.summary.errored_domains.length === domains.length;

  if (args['update-baseline']) {
    // Merge: keep prior baseline for domains that errored this run (never wipe on a bad fetch).
    const priorDomains = baseline?.domains ?? {};
    const merged = { ...priorDomains };
    for (const [domain, d] of Object.entries(report.domains)) {
      if (!d.error) merged[domain] = d.subdomains;
    }
    writeBaseline({ schema_version: SCHEMA_VERSION, updated_ts: nowIso(), domains: merged });
    console.error(`[asset-surface-scan] baseline updated → ${BASELINE_PATH}`);
  }

  if (args.json) console.log(JSON.stringify(report, null, 2));
  else printHuman(report);

  if (allErrored) {
    console.error('[asset-surface-scan] all domains failed to fetch — crt.sh unreachable/down (degraded)');
    emitTelemetry('fail', `asset-scan: all ${domains.length} domain(s) failed (crt.sh unreachable)`);
    return 1;
  }
  emitTelemetry(report.summary.errored_domains.length ? 'deferred' : 'pass',
    `asset-scan: ${report.summary.total_subdomains} subdomain(s), ${report.summary.total_new} NEW, ${report.summary.errored_domains.length} errored`);
  return 0;
}

const isMain = process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href;
if (isMain) {
  main(process.argv.slice(2))
    .then((code) => { process.exitCode = code ?? 0; })
    .catch((e) => { console.error(`[asset-surface-scan] fatal: ${e.stack || e}`); process.exitCode = 1; });
}
