// Loop Selection Router (Router spec) — the mandatory intake gate above the whole
// loop system. Runs FIRST on every command and decides: DIRECT (no loop — the
// common case), RUN an existing loop, BUILD a new one, or BOUNCE (no admissible
// metric). It ROUTES ONLY; it never overrides Class A/B, oracle-admissibility, or
// the security carve-out (§5). Cheap hot path: deterministic tag/scope match over
// the registry; LLM-classify is a deferred fallback for genuine ambiguity (§3).
//
// CORE GUARD (§0): mandatory SELECTION ≠ mandatory LOOP. DIRECT is the DEFAULT —
// anything that isn't an iterative task toward a measurable goal executes directly.

import { assessAdmissibility } from './loop-builder.js';
import type { RegistryLoop } from './registry.js';

export type RouteOutcome = 'DIRECT' | 'RUN' | 'BUILD' | 'BOUNCE';

export interface RouteDecision {
  outcome: RouteOutcome;
  loopId?: string;
  goal?: string;
  reason: string;
  method: 'deterministic' | 'llm';
  confidence: number;
  announce: string; // the one-line decision printed to the terminal (§1 transparency)
}

// One-shot / non-iterative commands (query · single action · explanation). DIRECT.
const ONE_SHOT_RE = /^\s*(git\s+(status|log|diff|show|branch)|ls|cat|pwd|head|tail|grep|rg|find|echo)\b|^\s*(explain|show|what|whats|why|how|list|describe|read|get|where|which|status|tell|print)\b/i;
// Iterative-toward-a-measurable-goal phrasing. Only these are LOOP-WORTHY.
const LOOP_WORTHY_RE = /\b(fix\s+all|converge|make\s+.*\bgreen\b|to\s+green|polish(ing)?|harden(ing)?|de-?slop|\bqa\b|coverage|\bloop\b|all\s+.*tests?\s+(pass|green)|iterate|perf(ormance)?\s+loop|i18n\s+coverage|untranslated)\b/i;

function words(s: string): Set<string> {
  return new Set((s.toLowerCase().match(/[a-z0-9]+/g) ?? []));
}

export interface LoopMatch { loop: RegistryLoop; score: number; hits: string[] }

// Ultra-generic tags shared by many loops — they must NOT match a loop on their
// own (e.g. "green" shouldn't route "BE polishing to get backend green" to the
// convergence loop). They only add weight when a SPECIFIC tag also hits.
const GENERIC_TAGS = new Set(['green', 'loop', 'wiring', 'flow', 'test', 'tests']);

/** Tag/scope match of a command against active registry loops (deterministic, cheap). */
export function scoreMatches(command: string, registry: RegistryLoop[]): LoopMatch[] {
  const w = words(command);
  return registry
    .filter((l) => l.status === 'active')
    .map((l) => {
      const hits = l.trigger_tags.filter((t) => w.has(t.toLowerCase()));
      const specific = hits.filter((t) => !GENERIC_TAGS.has(t.toLowerCase()));
      if (specific.length === 0) return { loop: l, score: 0, hits: [] }; // generic-only → not a real match
      let score = specific.length + 0.5 * (hits.length - specific.length);
      for (const gw of words(l.goal)) if (w.has(gw) && gw.length > 3 && !GENERIC_TAGS.has(gw)) score += 0.25;
      return { loop: l, score, hits };
    })
    .filter((m) => m.score > 0)
    .sort((a, b) => b.score - a.score);
}

const A = (s: string): string => `[router] ${s}`;

export function route(command: string, registry: RegistryLoop[]): RouteDecision {
  // §0/§7 — DIRECT is the default: only iterative-toward-a-measurable-goal is loop-worthy.
  if (!LOOP_WORTHY_RE.test(command)) {
    return {
      outcome: 'DIRECT', method: 'deterministic', confidence: ONE_SHOT_RE.test(command) ? 0.95 : 0.8,
      reason: 'not an iterative task toward a measurable goal — execute directly (DIRECT is the default; forcing a loop here would make the terminal unusable).',
      announce: A('→ DIRECT (one-shot / non-iterative)'),
    };
  }

  // 2a — match a registered loop (best by tag score; flag a tie as ambiguous).
  const matches = scoreMatches(command, registry);
  if (matches.length && matches[0]!.score >= 1) {
    const best = matches[0]!;
    const tie = !!matches[1] && matches[1]!.score === best.score;
    return {
      outcome: 'RUN', loopId: best.loop.id, method: 'deterministic',
      confidence: tie ? 0.55 : Math.min(0.95, 0.6 + best.score * 0.1),
      reason: `matched registered loop "${best.loop.id}" (tags: ${best.hits.join(', ')})${tie ? ` — AMBIGUOUS tie with "${matches[1]!.loop.id}"; clarify if high blast-radius` : ''}`,
      announce: A(`→ RUN ${best.loop.id} (matched: ${best.hits.join(' + ')})${tie ? ' [ambiguous]' : ''}`),
    };
  }

  // 2b — no match: admissible → BUILD; else BOUNCE (never auto-build a churn loop).
  const adm = assessAdmissibility(command);
  if (adm.admissible) {
    return {
      outcome: 'BUILD', goal: command, method: 'deterministic', confidence: 0.7,
      reason: 'no registered loop matches, but the goal is oracle-admissible → hand to the loop builder (it designs + smoke-gates + may auto-register).',
      announce: A(`→ BUILD loop for "${command}" (no match, admissible)`),
    };
  }
  return {
    outcome: 'BOUNCE', method: 'deterministic', confidence: 0.8,
    reason: `loop-worthy but no admissible metric: ${adm.reason}`,
    announce: A('→ BOUNCE: define measurable success criteria (no falsifiable metric → no loop built/run)'),
  };
}

export interface RoutingRecord {
  ts: string; command: string; outcome: RouteOutcome; loopId?: string; goal?: string;
  method: 'deterministic' | 'llm'; confidence: number;
}

/** §6 — the lightweight routing record (intent only; truncated; for audit + recall). */
export function toRoutingRecord(command: string, d: RouteDecision, ts: string): RoutingRecord {
  return {
    ts, command: command.slice(0, 200), outcome: d.outcome,
    loopId: d.loopId, goal: d.goal, method: d.method, confidence: d.confidence,
  };
}

// ─── CLI (hook-usable) — route a command, announce, log. Advisory: exit 0. ───
// Usage (e.g. from a userPromptSubmitted hook):
//   echo "$COMMAND" | npx tsx tools/loop-harness/src/router.ts [baseDir]

import fs from 'node:fs';
import path from 'node:path';
import { readRegistry } from './registry.js';

async function readStdin(): Promise<string> {
  if (process.stdin.isTTY) return '';
  const chunks: Buffer[] = [];
  for await (const c of process.stdin) chunks.push(c as Buffer);
  return Buffer.concat(chunks).toString('utf8').trim();
}

async function main(): Promise<void> {
  const baseDir = process.argv[2] ?? 'loops/runs';
  const command = (process.argv.slice(3).filter((a) => !a.startsWith('-')).join(' ') || (await readStdin())).trim();
  if (!command) { console.error('usage: router.ts [baseDir] "<command>"  (or pipe the command on stdin)'); process.exit(2); }
  const decision = route(command, readRegistry(baseDir));
  console.log(decision.announce);
  // §6 routing telemetry (append-only)
  try {
    fs.mkdirSync(baseDir, { recursive: true });
    fs.appendFileSync(path.join(baseDir, 'routing.jsonl'), JSON.stringify(toRoutingRecord(command, decision, new Date(Date.now()).toISOString())) + '\n');
  } catch { /* advisory — never block on telemetry */ }
}

if (import.meta.url === `file://${process.argv[1]}`) main().catch((e) => { console.error(e); process.exit(1); });
