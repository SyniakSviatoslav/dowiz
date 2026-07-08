// Bebop init — the 5-axis personalization wizard.
//
// Writes ~/.bebop/settings.json (a typed Profile). Interactive in a TTY; scriptable via
// --preset bebop | --json '{"origin":...}'. The 5 axes are data consumed by every other module.

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { BEBOP_PRESET, makeProfile, validateProfile, type Profile } from './profile.ts';
import { ADAPTERS, type Backend } from './backend.ts';
import { makePaint } from './theme.ts';

const SETTINGS_DIR = path.join(os.homedir(), '.bebop');
const SETTINGS_PATH = path.join(SETTINGS_DIR, 'settings.json');

const AXES: { key: string; label: string; values: string[] }[] = [
  { key: 'origin', label: 'Choose your origin (behavior baseline)', values: ['claude', 'opencode', 'codex', 'hybrid'] },
  { key: 'classKind', label: 'Choose your class (focus)', values: ['multi', 'marketing', 'sales', 'automation', 'research', 'osint'] },
  { key: 'narration', label: 'Choose your narration (voice)', values: ['bebop', 'plain', 'sarcastic', 'corporate-killer'] },
  { key: 'patrons', label: 'Choose your patrons (reasoning)', values: ['rock', 'garden', 'hybrid'] },
  { key: 'looks', label: 'Choose your looks (theme + mascot)', values: ['bebop', 'claude', 'opencode', 'codex', 'custom'] },
];

function readLine(prompt: string): Promise<string> {
  return new Promise((resolve) => {
    process.stdout.write(prompt);
    process.stdin.once('data', (d) => resolve(d.toString().trim()));
  });
}

async function interactive(): Promise<Profile> {
  const axes: Record<string, string> = {};
  for (const a of AXES) {
    console.log(`\n${makePaint().teal('◈')} ${a.label}:`);
    a.values.forEach((v, i) => console.log(`  ${i + 1}. ${v}`));
    const ans = await readLine(`  pick (1-${a.values.length}): `);
    const idx = Number(ans) - 1;
    axes[a.key] = a.values[idx] ?? a.values[0];
  }
  const yolo = (await readLine('\nAllow unattended auto-approve backends? (y/N): ')).toLowerCase() === 'y';
  return makeProfile({
    origin: axes.origin as any,
    classKind: axes.classKind as any,
    narration: axes.narration as any,
    patrons: axes.patrons as any,
    looks: axes.looks as any,
    yolo,
  });
}

export function loadProfile(): Profile | null {
  try {
    const raw = JSON.parse(fs.readFileSync(SETTINGS_PATH, 'utf8'));
    return validateProfile(raw);
  } catch {
    return null;
  }
}

export function writeProfile(p: Profile): string {
  fs.mkdirSync(SETTINGS_DIR, { recursive: true });
  fs.writeFileSync(SETTINGS_PATH, JSON.stringify(p, null, 2));
  return SETTINGS_PATH;
}

/** Show the connected/available backends (the conductor's view). */
export function statusLine(p: Profile): string {
  const cells = p.backendOrder.map((b: Backend) => {
    const a = ADAPTERS[b];
    const ok = a.detect() && (a.requiredEnv.length === 0 || a.requiredEnv.some((v) => !!process.env[v]));
    return `${b}${ok ? '' : '*'}`;
  });
  return cells.join(' → ');
}

export async function init(opts: { preset?: string; json?: string; force?: boolean }): Promise<Profile> {
  let profile: Profile;
  if (opts.preset === 'bebop') {
    profile = { ...BEBOP_PRESET };
  } else if (opts.json) {
    const parsed = JSON.parse(opts.json);
    profile = validateProfile({ ...BEBOP_PRESET, ...parsed });
  } else if (process.stdin.isTTY) {
    profile = await interactive();
  } else {
    profile = { ...BEBOP_PRESET };
  }
  if (!opts.force && loadProfile()) {
    // don't clobber an existing profile unless forced
    return loadProfile()!;
  }
  writeProfile(profile);
  return profile;
}
