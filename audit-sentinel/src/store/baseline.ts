import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Finding } from './findings.js';

const dirName = path.dirname(fileURLToPath(import.meta.url));
const BASELINE_PATH = path.join(dirName, '..', '..', 'findings', 'baseline.json');

export interface Baseline {
  updated_at: string;
  env: string;
  base_url: string;
  health_ok: boolean;
  known_findings: Record<string, Finding>;
  green_checks: string[];
  red_checks: string[];
}

export function loadBaseline(): Baseline | null {
  try {
    if (fs.existsSync(BASELINE_PATH)) {
      return JSON.parse(fs.readFileSync(BASELINE_PATH, 'utf-8'));
    }
  } catch {
    console.warn('Failed to load baseline, starting fresh');
  }
  return null;
}

export function saveBaseline(baseline: Baseline): void {
  fs.mkdirSync(path.dirname(BASELINE_PATH), { recursive: true });
  fs.writeFileSync(BASELINE_PATH, JSON.stringify(baseline, null, 2), 'utf-8');
}

export function diffFindings(
  current: Finding[],
  baseline: Baseline | null,
): { new_findings: Finding[]; regressed: Finding[]; resolved: string[] } {
  if (!baseline) {
    return { new_findings: current.map((f) => ({ ...f, status: 'NEW' as const })), regressed: [], resolved: [] };
  }

  const new_findings: Finding[] = [];
  const regressed: Finding[] = [];
  const currentIds = new Set(current.map((f) => f.id));

  for (const f of current) {
    const known = baseline.known_findings[f.id];
    if (!known) {
      new_findings.push({ ...f, status: 'NEW' });
    } else if (known.status === 'RESOLVED') {
      regressed.push({ ...f, status: 'REGRESSED' });
    }
  }

  const resolved = Object.keys(baseline.known_findings).filter(
    (id) =>
      !currentIds.has(id) && baseline.known_findings[id].status !== 'RESOLVED',
  );

  return { new_findings, regressed, resolved };
}
