// The loop registry (Router spec §2) — the connective tissue. `runs/registry.json`
// is the manifest the ROUTER selects from and the loop BUILDER writes to on
// auto-register. One place that answers "what loops exist and what do they match?".
// Append/upsert by id; the autoupgrade loop may retire stale entries (Class A).

import fs from 'node:fs';
import path from 'node:path';

export interface RegistryLoop {
  id: string;
  goal: string;
  trigger_tags: string[];
  scope_class: 'A' | 'B';
  security_carveout: string[];
  last_success?: string;
  registered_at?: string;
  status: 'active' | 'retired';
}

function registryPath(baseDir: string): string {
  return path.join(baseDir, 'registry.json');
}

export function readRegistry(baseDir: string): RegistryLoop[] {
  const p = registryPath(baseDir);
  if (!fs.existsSync(p)) return [];
  try { const j = JSON.parse(fs.readFileSync(p, 'utf8')); return Array.isArray(j?.loops) ? j.loops : []; }
  catch { return []; }
}

/** Upsert a loop by id (preserve last_success + registered_at on update). */
export function registerLoop(baseDir: string, loop: RegistryLoop): RegistryLoop {
  fs.mkdirSync(baseDir, { recursive: true });
  const all = readRegistry(baseDir);
  const idx = all.findIndex((l) => l.id === loop.id);
  const merged: RegistryLoop = idx >= 0 ? { ...all[idx], ...loop, registered_at: all[idx]!.registered_at ?? loop.registered_at } : loop;
  if (idx >= 0) all[idx] = merged; else all.push(merged);
  fs.writeFileSync(registryPath(baseDir), JSON.stringify({ loops: all }, null, 2) + '\n');
  return merged;
}
