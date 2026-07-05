// EvoMap-insight A — the proven-upgrade ASSET REGISTRY ("genes").
//
// When the §2 oracle KEEPS a Class-A candidate (green + no-security-regression +
// ≥5% benchmark speedup + reversible), the proven change is persisted here as a
// versioned, append-only, auditable + replayable asset — NOT re-discovered each
// run. Mirrors proposals.ts: durable, deduped-by-id upsert, frequency-weighted
// (count++), never auto-deleted, timestamp supplied by the caller (determinism —
// this module never calls Date.now()).
//
// This is a LEDGER, not an apply path: writing a gene records that a change was
// already proven+kept by the oracle. It mutates no product code and never
// auto-applies anything (auto-apply stays disabled/report-only by default).

import fs from 'node:fs';
import path from 'node:path';

export interface ProvenUpgrade {
  id: string;
  /** Bumped each time the same id is re-proven (a newer measurement supersedes). */
  version: number;
  /** Reference to the mechanical patch/diff that was applied (the candidate's action). */
  patch_ref: string;
  /** Oracle benchmark metric BEFORE the change (lower = better); null if unmeasured. */
  metric_before: number | null;
  /** Oracle benchmark metric AFTER the change. */
  metric_after: number | null;
  /** Proven speedup the oracle measured (≥ threshold, else it would have rolled back). */
  speedup_pct: number | null;
  /** The recorded, tested revert (§2.4) — how this change is atomically undone. */
  revert: string;
  /** Where the proof came from: trusted detector source + the lenses it passed. */
  provenance: string;
  /** Caller-supplied timestamps (no Date.now() here — determinism). */
  first_proven: string;
  last_proven: string;
  /** How many runs re-proved it (frequency-weighting, like proposals.count). */
  count: number;
}

export type ProvenUpgradeInput = Pick<
  ProvenUpgrade,
  'id' | 'patch_ref' | 'metric_before' | 'metric_after' | 'speedup_pct' | 'revert' | 'provenance'
>;

function registryPath(baseDir: string): string {
  return path.join(baseDir, 'proven-upgrades.json');
}

export function readProvenUpgrades(baseDir: string): ProvenUpgrade[] {
  const p = registryPath(baseDir);
  if (!fs.existsSync(p)) return [];
  try { const j = JSON.parse(fs.readFileSync(p, 'utf8')); return Array.isArray(j) ? j : []; }
  catch { return []; }
}

function writeRegistry(baseDir: string, all: ProvenUpgrade[]): void {
  fs.mkdirSync(baseDir, { recursive: true });
  fs.writeFileSync(registryPath(baseDir), JSON.stringify(all, null, 2) + '\n');
}

/**
 * Upsert a proven upgrade by id (append-only ledger). New → version 1, count 1.
 * Re-proven → version++ + count++ + refresh metrics/last_proven, PRESERVING
 * first_proven. Never deletes. Returns the resulting gene.
 */
export function recordProvenUpgrade(baseDir: string, input: ProvenUpgradeInput, ts: string): ProvenUpgrade {
  fs.mkdirSync(baseDir, { recursive: true });
  const all = readProvenUpgrades(baseDir);
  const idx = all.findIndex((x) => x.id === input.id);
  let result: ProvenUpgrade;
  if (idx >= 0) {
    const prev = all[idx]!;
    result = {
      ...prev, ...input,
      version: prev.version + 1,
      first_proven: prev.first_proven,
      last_proven: ts,
      count: prev.count + 1,
    };
    all[idx] = result;
  } else {
    result = { ...input, version: 1, first_proven: ts, last_proven: ts, count: 1 };
    all.push(result);
  }
  writeRegistry(baseDir, all);
  return result;
}
