// §1 Global Governor — a DETERMINISTIC resource/safety watchdog above ALL loops.
// NOT the multi-agent supervisor (rejected): zero intelligence, no routing, no
// coordination. It only watches aggregate numbers and pulls a brake. Per-loop
// breakers stop ONE loop; nothing else caps the SUM or offers one halt — this does.
//
// The safety asymmetry: a ceiling breach AUTO-HALTS, but clearing a halt is MANUAL
// (a human). A runaway can never auto-resume itself.

import fs from 'node:fs';
import path from 'node:path';
import { readMetrics } from './storage.js';

export interface GovernorConfig {
  costPerDayUsd: number;
  costPerHourUsd: number;
  minFreeRamMb: number;
  churnPerDay: number;        // total files changed across loops/day
  maxConcurrentLoops: number;
}
export const DEFAULT_GOVERNOR: GovernorConfig = {
  costPerDayUsd: 200, costPerHourUsd: 50, minFreeRamMb: 1024, churnPerDay: 500, maxConcurrentLoops: 2,
};

const DAY = 86_400_000;
const HOUR = 3_600_000;

export function haltFlagPath(baseDir: string): string {
  return path.join(baseDir, 'HALT');
}
export function isHalted(baseDir: string): { halted: boolean; reason: string | null } {
  const p = haltFlagPath(baseDir);
  if (!fs.existsSync(p)) return { halted: false, reason: null };
  return { halted: true, reason: fs.readFileSync(p, 'utf8').trim() || 'halted' };
}
/** Master halt — immediately blocks new spawns + autonomous applies. Manual or auto. */
export function masterHalt(baseDir: string, reason: string): void {
  fs.mkdirSync(baseDir, { recursive: true });
  fs.writeFileSync(haltFlagPath(baseDir), reason + '\n');
}
/** Clearing a halt is a deliberate HUMAN act — never automatic (the safety asymmetry). */
export function clearHalt(baseDir: string): void {
  const p = haltFlagPath(baseDir);
  if (fs.existsSync(p)) fs.rmSync(p);
}

export interface Aggregate { costDay: number; costHour: number; churnDay: number; runsDay: number }
export function aggregate(baseDir: string, nowMs: number): Aggregate {
  const agg: Aggregate = { costDay: 0, costHour: 0, churnDay: 0, runsDay: 0 };
  for (const m of readMetrics(baseDir)) {
    const t = Date.parse(m.ts);
    if (Number.isNaN(t)) continue;
    const age = nowMs - t;
    if (age <= DAY) { agg.costDay += m.cost_usd; agg.churnDay += m.edits ?? 0; agg.runsDay += 1; }
    if (age <= HOUR) agg.costHour += m.cost_usd;
  }
  return agg;
}

export interface GovernorCheck { allowed: boolean; reason: string; breached: string[]; agg: Aggregate }

/**
 * The gate every autonomous spawn/apply calls. Returns allowed=false (and AUTO-HALTS)
 * if halted or any aggregate ceiling is breached. Reads runs/metrics.jsonl (the one
 * whole-system view). `freeRamMb`/`concurrentLoops` are passed by the caller.
 */
export function checkGovernor(
  baseDir: string,
  opts: { nowMs: number; freeRamMb?: number; concurrentLoops?: number },
  cfg: GovernorConfig = DEFAULT_GOVERNOR,
): GovernorCheck {
  const agg = aggregate(baseDir, opts.nowMs);
  const halted = isHalted(baseDir);
  if (halted.halted) {
    return { allowed: false, reason: `MASTER HALT: ${halted.reason} (resume is MANUAL — a human clears it)`, breached: ['halt'], agg };
  }
  const breached: string[] = [];
  if (agg.costDay >= cfg.costPerDayUsd) breached.push(`cost/day $${agg.costDay.toFixed(2)} ≥ $${cfg.costPerDayUsd}`);
  if (agg.costHour >= cfg.costPerHourUsd) breached.push(`cost/hour $${agg.costHour.toFixed(2)} ≥ $${cfg.costPerHourUsd}`);
  if (agg.churnDay >= cfg.churnPerDay) breached.push(`churn/day ${agg.churnDay} ≥ ${cfg.churnPerDay} files`);
  if (opts.freeRamMb != null && opts.freeRamMb < cfg.minFreeRamMb) breached.push(`free RAM ${opts.freeRamMb}MB < ${cfg.minFreeRamMb}MB`);
  if (opts.concurrentLoops != null && opts.concurrentLoops >= cfg.maxConcurrentLoops) breached.push(`concurrent loops ${opts.concurrentLoops} ≥ ${cfg.maxConcurrentLoops}`);

  if (breached.length) {
    masterHalt(baseDir, `AUTO-HALT: ${breached.join('; ')}`); // breach → auto-halt; resume is manual
    return { allowed: false, reason: `aggregate ceiling breached → AUTO-HALTED: ${breached.join('; ')}`, breached, agg };
  }
  return { allowed: true, reason: 'within all aggregate ceilings', breached: [], agg };
}

/** §1 liveness reaper — loops silent (no heartbeat) beyond maxSilentMs are stale (OOM/hung). */
export function staleLoops(heartbeats: Record<string, number>, nowMs: number, maxSilentMs: number): string[] {
  return Object.entries(heartbeats).filter(([, t]) => nowMs - t > maxSilentMs).map(([id]) => id);
}
