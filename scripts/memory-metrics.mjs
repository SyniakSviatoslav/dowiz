#!/usr/bin/env node
// memory-metrics — usage-mining over data that already exists (agentfiles/skillkit pattern:
// zero new instrumentation). Cross-references ~/.claude/projects/-root-dowiz/*.jsonl (session
// transcripts) + .claude/logs/harness-events.jsonl against memory/*.md and docs/lessons/INDEX.md
// to surface per-memory-file and per-lesson USAGE counts (never-used / rarely-used candidates).
// Read-only. Node stdlib only. No new deps.
//
// Usage: node scripts/memory-metrics.mjs [--json] [--since 30d]
//   --json   emit machine-readable { memories: [...], lessons: [...] } instead of the text report
//   --since  only count session transcripts modified in the last N days (default: all)

import { readFileSync, readdirSync, statSync } from 'node:fs';
import { homedir } from 'node:os';
import path from 'node:path';

const REPO_ROOT = path.resolve(new URL('.', import.meta.url).pathname, '..');
const MEMORY_DIR = path.join(homedir(), '.claude/projects/-root-dowiz/memory');
const SESSIONS_DIR = path.join(homedir(), '.claude/projects/-root-dowiz');
const HARNESS_EVENTS = path.join(REPO_ROOT, '.claude/logs/harness-events.jsonl');
const LESSONS_INDEX = path.join(REPO_ROOT, 'docs/lessons/INDEX.md');

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

function parseSinceDays(s) {
  if (!s) return null;
  const m = /^(\d+)d$/.exec(String(s));
  if (!m) return null;
  return Date.now() - Number(m[1]) * 86400_000;
}

/** List memory/*.md slugs (filename without extension), excluding MEMORY.md itself. */
function listMemorySlugs() {
  let files = [];
  try {
    files = readdirSync(MEMORY_DIR).filter((f) => f.endsWith('.md') && f !== 'MEMORY.md');
  } catch {
    return [];
  }
  return files.map((f) => f.replace(/\.md$/, ''));
}

/** Parse docs/lessons/INDEX.md's `| TRIGGER | file |` table (do not reorder columns — hook contract). */
function listLessonFiles() {
  let text = '';
  try {
    text = readFileSync(LESSONS_INDEX, 'utf8');
  } catch {
    return [];
  }
  const rows = [];
  for (const line of text.split('\n')) {
    const m = /^\|\s*(.+?)\s*\|\s*(docs\/lessons\/[^\s|]+\.md)\s*\|$/.exec(line.trim());
    if (m) rows.push({ trigger: m[1], file: m[2] });
  }
  return rows;
}

/** Tally pre-edit-lessons "inject" events by the lesson file logged in the `detail` field. */
function tallyLessonInjections() {
  const counts = new Map(); // lessonFile -> count
  let text = '';
  try {
    text = readFileSync(HARNESS_EVENTS, 'utf8');
  } catch {
    return counts;
  }
  for (const line of text.split('\n')) {
    if (!line.includes('"pre-edit-lessons"') || !line.includes('"inject"')) continue;
    try {
      const d = JSON.parse(line);
      if (d.hook === 'pre-edit-lessons' && d.event === 'inject' && d.detail) {
        counts.set(d.detail, (counts.get(d.detail) ?? 0) + 1);
      }
    } catch { /* skip malformed line */ }
  }
  return counts;
}

/** Scan every session transcript's assistant-role text for a slug or [[wikilink]] mention. */
function tallyMemoryCitations(slugs, sinceMs) {
  const counts = new Map(slugs.map((s) => [s, { mentions: 0, sessions: 0 }]));
  let files = [];
  try {
    files = readdirSync(SESSIONS_DIR).filter((f) => f.endsWith('.jsonl'));
  } catch {
    return counts;
  }
  for (const f of files) {
    const full = path.join(SESSIONS_DIR, f);
    if (sinceMs) {
      try {
        if (statSync(full).mtimeMs < sinceMs) continue;
      } catch { continue; }
    }
    let text;
    try {
      text = readFileSync(full, 'utf8');
    } catch { continue; }
    const seenThisSession = new Set();
    for (const slug of slugs) {
      // Cheap substring match (slug name or its [[wikilink]] form) — no JSON parse needed per
      // line since we only need presence, not structure; a false-positive substring hit inside
      // an unrelated word is unlikely given these slugs are hyphenated multi-word identifiers.
      const needle = slug;
      let idx = -1;
      let n = 0;
      while ((idx = text.indexOf(needle, idx + 1)) !== -1) n++;
      if (n > 0) {
        const c = counts.get(slug);
        c.mentions += n;
        if (!seenThisSession.has(slug)) { c.sessions += 1; seenThisSession.add(slug); }
        seenThisSession.add(slug);
      }
    }
  }
  return counts;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const sinceMs = parseSinceDays(args.since);

  const slugs = listMemorySlugs();
  const memoryCounts = tallyMemoryCitations(slugs, sinceMs);
  const memories = slugs
    .map((slug) => ({ slug, ...memoryCounts.get(slug) }))
    .sort((a, b) => a.mentions - b.mentions);

  const lessonRows = listLessonFiles();
  const injectionCounts = tallyLessonInjections();
  const lessons = lessonRows
    .map((row) => ({ ...row, injections: injectionCounts.get(row.file) ?? 0 }))
    .sort((a, b) => b.injections - a.injections);

  if (args.json) {
    console.log(JSON.stringify({ memories, lessons }, null, 2));
    return;
  }

  const neverCited = memories.filter((m) => m.mentions === 0);
  console.log(`== memory-metrics == ${memories.length} memory files, ${lessons.length} lessons`);
  console.log(`-- never cited in any session transcript (${neverCited.length}) --`);
  for (const m of neverCited.slice(0, 30)) console.log(` - ${m.slug}`);
  if (neverCited.length > 30) console.log(`   ... and ${neverCited.length - 30} more`);
  console.log('-- top 10 most-cited memory files --');
  for (const m of memories.slice(-10).reverse()) {
    console.log(` ${m.slug}: ${m.mentions} mentions across ${m.sessions} sessions`);
  }
  console.log('-- lesson injections (pre-edit-lessons hook fires), by lesson file --');
  for (const l of lessons) console.log(` ${l.injections}x  ${l.file}  (trigger: ${l.trigger})`);
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main();
}
