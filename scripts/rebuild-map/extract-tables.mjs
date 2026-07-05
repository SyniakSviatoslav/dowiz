#!/usr/bin/env node
// scripts/rebuild-map/extract-tables.mjs
//
// Namespace: tables
// Mirrors inventory/12-data-layer.md §0/§1 table census: replay every `packages/db/migrations/`
// file (node-pg-migrate, forward-only) in chronological (filename-timestamp) order, `up()`
// section only (`down()` never runs — forward-only discipline, inventory/12 §0/§method),
// tracking CREATE TABLE / DROP TABLE as a last-write-wins state machine so a
// created-then-dropped-without-recreate table correctly disappears from the live census.
// Doc count: 84 migrated (+2 out-of-band, applied by hand — not in the migrations/ tree,
// so structurally invisible to this extractor; see README) = 86 total live.
//
// Scope: `public` schema only (bare `CREATE TABLE name` or `public.name`) — `pgboss.*` job
// tables are a separate schema per inventory/12 §0 and excluded here by design.
//
// RED-LINE NOTE: this extractor only READS packages/db/migrations/ (protected path per
// Ship Discipline) — it never writes there.

import { readRepoFile, idSafe, isMain, printRecords, walkFiles } from './lib/common.mjs';

const MIGRATIONS_DIR = 'packages/db/migrations';
const DOWN_FN_RE = /export\s+async\s+function\s+down/;

const CREATE_RE = /CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?"?(?:public\.)?([a-zA-Z_][a-zA-Z0-9_]*)"?/gi;
const CREATE_PGM_RE = /pgm\.createTable\(\s*['"](?:public\.)?([a-zA-Z_][a-zA-Z0-9_]*)['"]/g;
const DROP_RE = /DROP\s+TABLE\s+(?:IF\s+EXISTS\s+)?"?(?:public\.)?([a-zA-Z_][a-zA-Z0-9_]*)"?/gi;
const DROP_PGM_RE = /pgm\.dropTable\(\s*['"](?:public\.)?([a-zA-Z_][a-zA-Z0-9_]*)['"]/g;

function lineAt(text, index) {
  return text.slice(0, index).split('\n').length;
}

/**
 * Pure/testable: given one migration file's full text, return the up()-section CREATE/DROP
 * table events in file order: [{type:'create'|'drop', table, line}].
 */
export function parseMigrationTableEvents(content) {
  const downMatch = DOWN_FN_RE.exec(content);
  const upSection = downMatch ? content.slice(0, downMatch.index) : content;

  const events = [];
  const collect = (re, type) => {
    re.lastIndex = 0;
    let m;
    while ((m = re.exec(upSection))) {
      events.push({ type, table: m[1], index: m.index, line: lineAt(upSection, m.index) });
    }
  };
  collect(CREATE_RE, 'create');
  collect(CREATE_PGM_RE, 'create');
  collect(DROP_RE, 'drop');
  collect(DROP_PGM_RE, 'drop');
  events.sort((a, b) => a.index - b.index);
  return events.map(({ type, table, line }) => ({ type, table, line }));
}

export async function extract() {
  // filenames are timestamp-prefixed -> lexicographic sort == chronological order
  const files = walkFiles(MIGRATIONS_DIR, ['.ts']).sort();

  const state = new Map(); // table -> { file, line, exists }
  for (const f of files) {
    const events = parseMigrationTableEvents(readRepoFile(f));
    for (const ev of events) {
      if (ev.type === 'create') {
        state.set(ev.table, { file: f, line: ev.line, exists: true });
      } else if (ev.type === 'drop') {
        const prev = state.get(ev.table);
        state.set(ev.table, { file: prev ? prev.file : f, line: prev ? prev.line : ev.line, exists: false });
      }
    }
  }

  const records = [];
  for (const [table, info] of state.entries()) {
    if (!info.exists) continue;
    records.push({ ns: 'tables', id: `TBL-${idSafe(table)}`, file: info.file, line: info.line });
  }
  return records.sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
}

if (isMain(import.meta.url)) {
  const records = await extract();
  printRecords(records);
}
