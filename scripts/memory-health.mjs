#!/usr/bin/env node
// memory-health — corpus health check (skillkit `health --json` pattern, ported zero-ML):
// staleness (never_used/unused_30d — uses memory-metrics.mjs's usage JSON if piped in via
// --usage <file>, else falls back to mtime), oversized files, long frontmatter descriptions,
// and Jaccard word-shingle duplicate-pairs (catches FUTURE drift; this is the re-runnable
// check the 2026-07-04 harness-token-audit's one-off clustering should have been all along).
// Candidates only — this script NEVER deletes, merges, or archives anything; it only emits
// JSON + a human summary for a human (or the librarian, at its existing trigger points) to act
// on. Read-only. Node stdlib only. No new deps.
//
// Usage: node scripts/memory-health.mjs [--json] [--usage <memory-metrics.json>]
//                                       [--oversized-tokens 2000] [--long-description-chars 400]
//                                       [--jaccard-threshold 0.25] [--stale-days 30]

import { readFileSync, readdirSync, statSync } from 'node:fs';
import { homedir } from 'node:os';
import path from 'node:path';

const MEMORY_DIR = path.join(homedir(), '.claude/projects/-root-dowiz/memory');
const CHARS_PER_TOKEN = 4;

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

function listMemoryFiles() {
  let files = [];
  try {
    files = readdirSync(MEMORY_DIR).filter((f) => f.endsWith('.md') && f !== 'MEMORY.md');
  } catch {
    return [];
  }
  return files;
}

function parseFrontmatter(raw) {
  const m = /^---\n([\s\S]*?)\n---/.exec(raw);
  if (!m) return { description: '' };
  const dm = /description:\s*"?(.*?)"?\s*$/m.exec(m[1]);
  return { description: dm ? dm[1] : '' };
}

function bodyOf(raw) {
  return raw.replace(/^---[\s\S]*?---/, '');
}

function tokenSet(text) {
  return new Set(
    text
      .toLowerCase()
      .replace(/\[\[.*?\]\]/g, '')
      .replace(/[^a-z0-9\s]/g, ' ')
      .split(/\s+/)
      .filter((w) => w.length > 2)
  );
}

function jaccard(a, b) {
  let inter = 0;
  for (const x of a) if (b.has(x)) inter++;
  const union = a.size + b.size - inter;
  return union === 0 ? 0 : inter / union;
}

function loadUsage(usagePath) {
  if (!usagePath) return null;
  try {
    const j = JSON.parse(readFileSync(usagePath, 'utf8'));
    const map = new Map();
    for (const m of j.memories ?? []) map.set(m.slug, m);
    return map;
  } catch {
    return null;
  }
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const oversizedTokens = args['oversized-tokens'] ? Number(args['oversized-tokens']) : 2000;
  const longDescChars = args['long-description-chars'] ? Number(args['long-description-chars']) : 400;
  const jaccardThreshold = args['jaccard-threshold'] ? Number(args['jaccard-threshold']) : 0.25;
  const staleDays = args['stale-days'] ? Number(args['stale-days']) : 30;
  const usageMap = loadUsage(args.usage);

  const files = listMemoryFiles();
  const now = Date.now();
  const entries = [];
  for (const f of files) {
    const slug = f.replace(/\.md$/, '');
    const full = path.join(MEMORY_DIR, f);
    let raw = '';
    let mtimeMs = null;
    try {
      raw = readFileSync(full, 'utf8');
      mtimeMs = statSync(full).mtimeMs;
    } catch {
      continue;
    }
    const bytes = Buffer.byteLength(raw);
    const tokens = Math.ceil(bytes / CHARS_PER_TOKEN);
    const { description } = parseFrontmatter(raw);
    const ageDays = mtimeMs === null ? null : Math.round((now - mtimeMs) / 86400_000);
    entries.push({ slug, file: f, bytes, tokens, description_chars: description.length, age_days: ageDays, tokenSet: tokenSet(bodyOf(raw)) });
  }

  // Staleness: prefer usage data (job-1 output) when available; else mtime-only proxy.
  const staleness = entries.map((e) => {
    const usage = usageMap?.get(e.slug);
    if (usage) {
      return { slug: e.slug, never_used: usage.mentions === 0, unused_recent: usage.mentions === 0, basis: 'usage-data' };
    }
    return { slug: e.slug, never_used: null, unused_recent: e.age_days !== null && e.age_days > staleDays, basis: 'mtime-proxy' };
  });

  const oversized = entries.filter((e) => e.tokens > oversizedTokens).map((e) => ({ slug: e.slug, tokens: e.tokens }));
  const longDescriptions = entries.filter((e) => e.description_chars > longDescChars).map((e) => ({ slug: e.slug, chars: e.description_chars }));

  // Pairwise Jaccard over word-shingles — O(n^2) but n ~ 120 so ~7200 pairs, cheap.
  const dupPairs = [];
  for (let i = 0; i < entries.length; i++) {
    for (let j = i + 1; j < entries.length; j++) {
      const sim = jaccard(entries[i].tokenSet, entries[j].tokenSet);
      if (sim >= jaccardThreshold) {
        dupPairs.push({ a: entries[i].slug, b: entries[j].slug, similarity: Math.round(sim * 1000) / 1000 });
      }
    }
  }
  dupPairs.sort((a, b) => b.similarity - a.similarity);

  const report = {
    total_files: entries.length,
    usage_data_used: !!usageMap,
    staleness: {
      basis: usageMap ? 'usage-data' : 'mtime-proxy',
      never_used: staleness.filter((s) => s.never_used === true).map((s) => s.slug),
      unused_recent: staleness.filter((s) => s.unused_recent === true).map((s) => s.slug),
    },
    warnings: {
      oversized: oversized.sort((a, b) => b.tokens - a.tokens),
      long_descriptions: longDescriptions.sort((a, b) => b.chars - a.chars),
    },
    duplicate_candidates: dupPairs,
  };

  if (args.json) {
    console.log(JSON.stringify(report, null, 2));
    return;
  }

  console.log(`== memory-health == ${report.total_files} files, staleness basis: ${report.staleness.basis}`);
  console.log(`-- unused (>${staleDays}d or never-cited): ${report.staleness.unused_recent.length} --`);
  for (const s of report.staleness.unused_recent.slice(0, 20)) console.log(` - ${s}`);
  console.log(`-- oversized (> ${oversizedTokens} tok): ${report.warnings.oversized.length} --`);
  for (const o of report.warnings.oversized) console.log(` - ${o.slug}: ${o.tokens} tok`);
  console.log(`-- long descriptions (> ${longDescChars} chars): ${report.warnings.long_descriptions.length} --`);
  for (const d of report.warnings.long_descriptions) console.log(` - ${d.slug}: ${d.chars} chars`);
  console.log(`-- duplicate-candidate pairs (Jaccard >= ${jaccardThreshold}): ${dupPairs.length} --`);
  for (const p of dupPairs.slice(0, 20)) console.log(` - ${p.a} <-> ${p.b}: ${p.similarity}`);
  console.log('\n(candidates only — no file was merged, archived, or deleted by this script)');
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main();
}
