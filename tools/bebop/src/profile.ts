// Bebop profile — the single source of truth for the 5-axis personalization.
//
// Governing principle (RESEARCH §1.6): every cross-cutting layer is driven by THIS object. The 5 axes
// are data, not branching code. The "bebop" native preset is the recommended default:
//   1. origin   = claude-opencode hybrid (best of both, no lock-in)
//   2. class    = multi (one agent, many tasks)
//   3. narration= bebop (dry, truthful, slightly offensive cosmo-noir)
//   4. patrons  = hybrid (cold logic + authentic freestyling)
//   5. looks    = bebop (cosmo-gothic-noir jazz, teal signal)

import type { Backend } from './backend.ts';

export type Origin = 'claude' | 'opencode' | 'codex' | 'hybrid';
export type ClassKind = 'multi' | 'marketing' | 'sales' | 'automation' | 'research' | 'osint';
export type Narration = 'bebop' | 'plain' | 'sarcastic' | 'corporate-killer';
export type Patrons = 'rock' | 'garden' | 'hybrid';
export type Looks = 'bebop' | 'claude' | 'opencode' | 'codex' | 'custom';

export interface Profile {
  version: 1;
  origin: Origin;
  classKind: ClassKind;
  narration: Narration;
  patrons: Patrons;
  looks: Looks;
  /** Backend rotation order (driven by `origin`). `native` is always appended as the fallback. */
  backendOrder: Backend[];
  /** When true, backends that need auto-approve (aider/openhands-style) may run unattended. */
  yolo: boolean;
}

/** The recommended native preset. */
export const BEBOP_PRESET: Profile = {
  version: 1,
  origin: 'hybrid',
  classKind: 'multi',
  narration: 'bebop',
  patrons: 'hybrid',
  looks: 'bebop',
  // hybrid origin → prefer claude+opencode, then the rest, native last.
  backendOrder: ['opencode', 'claude', 'codex', 'hermes', 'goose', 'aider', 'native'],
  yolo: false,
};

/** Per-origin default backend rotation (the "origin" axis → which backends, in what order). */
const ORIGIN_ORDER: Record<Origin, Backend[]> = {
  claude: ['claude', 'opencode', 'codex', 'hermes', 'goose', 'aider', 'native'],
  opencode: ['opencode', 'claude', 'codex', 'hermes', 'goose', 'aider', 'native'],
  codex: ['codex', 'opencode', 'claude', 'hermes', 'goose', 'aider', 'native'],
  hybrid: ['opencode', 'claude', 'codex', 'hermes', 'goose', 'aider', 'native'],
};

/** Build a Profile from the 5 axes. Origin sets the backend rotation; everything else is data. */
export function makeProfile(axes: Partial<Omit<Profile, 'version' | 'backendOrder'>>): Profile {
  const origin = axes.origin ?? BEBOP_PRESET.origin;
  return {
    version: 1,
    origin,
    classKind: axes.classKind ?? BEBOP_PRESET.classKind,
    narration: axes.narration ?? BEBOP_PRESET.narration,
    patrons: axes.patrons ?? BEBOP_PRESET.patrons,
    looks: axes.looks ?? BEBOP_PRESET.looks,
    backendOrder: ORIGIN_ORDER[origin],
    yolo: axes.yolo ?? BEBOP_PRESET.yolo,
  };
}

/** Validate a loaded profile. Throws on corruption (no silent bad default). */
export function validateProfile(p: any): Profile {
  const need: (keyof Profile)[] = ['version', 'origin', 'classKind', 'narration', 'patrons', 'looks', 'backendOrder'];
  for (const k of need) {
    if (!(k in p)) throw new Error(`profile missing field: ${k}`);
  }
  if (p.version !== 1) throw new Error(`unsupported profile version: ${p.version}`);
  const validOrigins: Origin[] = ['claude', 'opencode', 'codex', 'hybrid'];
  if (!validOrigins.includes(p.origin)) throw new Error(`invalid origin: ${p.origin}`);
  if (!Array.isArray(p.backendOrder) || p.backendOrder.length === 0) {
    throw new Error('backendOrder must be a non-empty array');
  }
  // native must remain reachable (last) so Bebop never hard-fails with zero backends.
  if (!p.backendOrder.includes('native')) p.backendOrder = [...p.backendOrder, 'native'];
  return p as Profile;
}
