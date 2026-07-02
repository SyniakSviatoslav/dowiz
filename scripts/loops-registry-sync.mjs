#!/usr/bin/env node
// loops-registry-sync — loops/registry.md is the single source of truth for loop membership;
// loops/runs/registry.json (the router's machine registry) is DERIVED from it by this script.
//
// Regression (meta-loop P2, 2026-07-02): three registries disagreed — registry.md listed 16
// loops, docs/agents/loops/REGISTRY.md 11, runs/registry.json only 2 — so the Loop Selection
// Router could RUN-match 2 of 16 loops. Sync semantics: UNION-merge. Existing rich entries in
// registry.json are preserved (they carry hand-authored trigger_tags/scope_class the md table
// doesn't have); loops present in the md table but missing from json are added with basic
// fields; nothing is deleted. Drift is printed either way.
//
// Run: node scripts/loops-registry-sync.mjs [--check]   (--check: exit 1 on drift, write nothing)
import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = process.cwd();
const MD = join(ROOT, 'loops/registry.md');
const JSON_PATH = join(ROOT, 'loops/runs/registry.json');
const CHECK = process.argv.includes('--check');

const md = readFileSync(MD, 'utf8');
const rows = [];
for (const line of md.split('\n')) {
  const m = line.match(/^\|\s*([a-z0-9-]+)\s*\|(.+)\|$/);
  if (!m || m[1] === 'id') continue;
  const cells = line.split('|').map((c) => c.trim());
  // | id | intent | version | status | card | report | memory | trigger |
  if (cells.length < 9) continue;
  rows.push({ id: cells[1], intent: cells[2], version: cells[3], status: cells[4], trigger: cells[8] });
}
if (rows.length === 0) {
  console.error('✗ loops-registry-sync: parsed 0 rows from loops/registry.md — table format changed?');
  process.exit(1);
}

const current = existsSync(JSON_PATH) ? JSON.parse(readFileSync(JSON_PATH, 'utf8')) : { loops: [] };
const byId = new Map((current.loops || []).map((l) => [l.id, l]));

let added = 0;
for (const r of rows) {
  const existing = byId.get(r.id);
  if (existing) {
    // refresh status/version from the SoT, keep the rich hand-authored fields
    existing.status = r.status;
    existing.version = r.version;
    continue;
  }
  byId.set(r.id, {
    id: r.id,
    goal: r.intent,
    version: r.version,
    status: r.status,
    trigger: r.trigger,
    trigger_tags: [],
    source: 'registry.md (synced — enrich trigger_tags/scope_class by hand or via loop-architect)',
  });
  added++;
}

const mdIds = new Set(rows.map((r) => r.id));
const jsonOnly = [...byId.keys()].filter((id) => !mdIds.has(id));
if (jsonOnly.length) console.log(`  note: in registry.json but not the md table (kept): ${jsonOnly.join(', ')}`);

const out = { generated_by: 'scripts/loops-registry-sync.mjs', source_of_truth: 'loops/registry.md', loops: [...byId.values()].sort((a, b) => a.id.localeCompare(b.id)) };

if (CHECK) {
  if (added > 0) {
    console.error(`✗ loops-registry-sync --check: ${added} loop(s) in registry.md missing from runs/registry.json — run: node scripts/loops-registry-sync.mjs`);
    process.exit(1);
  }
  console.log(`✓ loops-registry-sync: registry.json covers all ${rows.length} loops in registry.md.`);
  process.exit(0);
}

writeFileSync(JSON_PATH, JSON.stringify(out, null, 2) + '\n');
console.log(`✓ loops-registry-sync: ${rows.length} md rows → registry.json now has ${out.loops.length} loops (${added} added).`);
