#!/usr/bin/env node
// song-of-singularity — this week's ethical infusion verse (docs/governance/song-of-singularity.md).
// Deterministic: rotates by ISO week number, so a given week always yields the same verse (reproducible,
// re-runnable, no LLM). Parses the verse list from the doc so the song can grow without touching code.
//
// Run:  node scripts/song-of-singularity.mjs            (print this week's verse)
//       node scripts/song-of-singularity.mjs --json     (machine form)
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = process.cwd();
const doc = readFileSync(join(ROOT, 'docs/governance/song-of-singularity.md'), 'utf8');

// Verses are the numbered `N. *italic*` lines under "## The verses".
const verses = [...doc.matchAll(/^\d+\.\s+\*(.+?)\*\s*$/gm)].map((m) => m[1]);
if (!verses.length) { console.error('no verses found in song-of-singularity.md'); process.exit(1); }

// ISO week number (deterministic index; no Math.random / no wall-clock dependence beyond the date).
const d = new Date();
const t = new Date(Date.UTC(d.getUTCFullYear(), d.getUTCMonth(), d.getUTCDate()));
const day = t.getUTCDay() || 7;
t.setUTCDate(t.getUTCDate() + 4 - day);
const yearStart = new Date(Date.UTC(t.getUTCFullYear(), 0, 1));
const week = Math.ceil(((t - yearStart) / 86400000 + 1) / 7);
const idx = (week - 1) % verses.length;
const verse = verses[idx];
const stamp = `${t.getUTCFullYear()}-w${String(week).padStart(2, '0')}`;

if (process.argv.includes('--json')) {
  console.log(JSON.stringify({ week: stamp, index: idx + 1, of: verses.length, verse }, null, 2));
} else {
  console.log(`☼ Song of Singularity — ${stamp} (verse ${idx + 1}/${verses.length})\n\n   "${verse}"\n`);
}
