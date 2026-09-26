// JSON → MARKDOWN (§B.3): the breach list first (empty = green), then what could not be
// measured and why, then every indicator by group with value, baseline and delta.
//
//   node tools/evals/report.mjs docs/measurements/evals/<file>.json   (prints the markdown)
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { groupOf } from './rules.mjs';

const esc = s => String(s ?? '').replace(/\|/g, '\\|').replace(/\n/g, ' ');

/** 1234567 → 1,234,567; null → '—'. */
export function fmt(v) {
  if (v === null || v === undefined) return '—';
  return typeof v === 'number' ? v.toLocaleString('en-US') : String(v);
}

export function delta(r) {
  if (r.baseline === null || r.baseline === undefined || r.value === null || r.value === undefined) return '';
  const d = r.value - r.baseline;
  return d === 0 ? '0' : (d > 0 ? '+' : '') + d.toLocaleString('en-US');
}

/** The indicator rows of a run document (everything but `_run`). */
export const rowsOf = doc => Object.entries(doc).filter(([k]) => k !== '_run');

export function render(doc) {
  const m = doc._run || {};
  const rows = rowsOf(doc);
  const breach = rows.filter(([, r]) => r.status === 'breach');
  const unv = rows.filter(([, r]) => r.status === 'unverified');
  const L = [
    `# Evals · ${m.suite} · ${m.date} · ${m.commit}`,
    '',
    `Deployed version: ${m.deployed_version}. Hosts: ${(m.hosts || []).join(', ') || 'none (no network)'}. `
      + `${rows.length} indicators, ${breach.length} breaches, ${unv.length} unverified. `
      + `Run ${Math.round(((m.finished_at || 0) - (m.started_at || 0)) / 1000)} s on node ${m.node}.`,
    '',
    '## Breaches',
    '',
  ];
  if (!breach.length) L.push('None — every judged indicator is within its rule.');
  else {
    L.push('| Indicator | Value | Rule | Why |', '|---|---|---|---|');
    for (const [id, r] of breach) L.push(`| \`${id}\` | ${fmt(r.value)} ${esc(r.unit)} | ${r.rule} | ${esc(r.why)}${r.note ? ` — ${esc(r.note)}` : ''} |`);
  }
  L.push('', '## Unverified', '');
  if (!unv.length) L.push('None.');
  else for (const [id, r] of unv) L.push(`- \`${id}\` — ${esc(r.unverified)} (source: ${esc(r.source)})`);
  const groups = {};
  for (const [id, r] of rows) (groups[groupOf(id)] ||= []).push([id, r]);
  for (const g of Object.keys(groups).sort()) {
    L.push('', `## ${g}`, '', '| Indicator | Value | Unit | Baseline | Δ | Rule | Status | Note |', '|---|---:|---|---:|---:|---|---|---|');
    for (const [id, r] of groups[g]) {
      const rule = r.limit !== undefined ? `${r.rule} ${fmt(r.limit)}` : r.rule;
      const status = r.status === 'ok' ? 'ok' : `**${r.status}**`;
      L.push(`| \`${id}\` | ${fmt(r.value)} | ${esc(r.unit)} | ${fmt(r.baseline)} | ${delta(r)} | ${rule} | ${status} | ${esc(r.why || r.note || '')} |`);
    }
  }
  return L.join('\n') + '\n';
}

/* node:coverage disable */
if (process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1])) {
  process.stdout.write(render(JSON.parse(fs.readFileSync(process.argv[2], 'utf8'))));
}
/* node:coverage enable */
