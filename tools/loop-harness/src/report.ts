// §5 — render the end-of-loop report FROM the canonical run-record. Pure: the
// prose is a regenerable view, never the unit of storage. §6 (VS HISTORY) is
// computed from the metrics index. The harness always prints this in full.

import type { RunRecord, MetricsLine, HistoryComparison } from './types.js';

const BAR = '════════════════════════════════════════════════════════════════';
const OUTCOME_BADGE: Record<RunRecord['outcome'], string> = {
  green: 'GREEN ✓',
  stall: 'STALL ✗',
  abort: 'ABORT ✗',
  natural_stop: 'STOPPED',
};

function num(n: number): string {
  return n.toLocaleString('en-US');
}
function arrow(thisV: number, avg: number, lowerBetter = true): string {
  if (avg === 0 || !isFinite(avg)) return '';
  const pct = Math.round(((thisV - avg) / avg) * 100);
  if (pct === 0) return ' (≈avg)';
  const better = lowerBetter ? pct < 0 : pct > 0;
  return ` (avg ${avg % 1 ? avg.toFixed(2) : avg} ${better ? '↓' : '↑'}${Math.abs(pct)}%)`;
}

/** §6 — compute the run-over-run comparison from the metrics index (all prior
 *  runs of this loop; nothing downsampled). */
export function computeHistory(priorMetrics: MetricsLine[], current: RunRecord): HistoryComparison {
  const greens = priorMetrics.filter((m) => m.outcome === 'green');
  const itersList = greens.map((m) => m.iters);
  const avg = (xs: number[]) => (xs.length ? xs.reduce((a, b) => a + b, 0) / xs.length : 0);
  const perResolvedList = priorMetrics.map((m) => m.per_resolved).filter((x): x is number => x != null);
  const costList = priorMetrics.map((m) => m.cost_usd);

  const recurringCounts = new Map<string, number>();
  for (const m of priorMetrics) for (const f of m.recurring_flags || []) {
    recurringCounts.set(f, (recurringCounts.get(f) || 0) + 1);
  }

  return {
    prior_runs: priorMetrics.length,
    iters_to_green: {
      this: current.iter_to - current.iter_from + 1,
      avg: Math.round(avg(itersList) * 10) / 10,
      best: itersList.length ? Math.min(...itersList) : current.iter_to - current.iter_from + 1,
    },
    per_resolved: {
      this: current.telemetry.per_resolved,
      avg: perResolvedList.length ? Math.round(avg(perResolvedList)) : null,
    },
    cost_usd: { this: current.telemetry.cost_usd, avg: Math.round(avg(costList) * 100) / 100 },
    recurring: [...recurringCounts.entries()].map(([tag, count]) => ({ tag, count })).sort((a, b) => b.count - a.count),
  };
}

export function renderReport(r: RunRecord): string {
  const t = r.telemetry;
  const L: string[] = [];
  const wall = `${Math.floor(r.wall_s / 60)}m${Math.round(r.wall_s % 60)}s`;

  L.push(BAR);
  L.push(` LOOP REPORT · ${r.loop} · run #${r.run_index} · iter ${r.iter_from}–${r.iter_to} · ${OUTCOME_BADGE[r.outcome]}`);
  L.push(` start ${r.t_start} · end ${r.t_end} · wall ${wall}` + (r.breaker_reason ? ` · breaker: ${r.breaker_reason}` : ''));
  L.push(BAR);
  L.push('');
  L.push('1. INITIAL GOAL');
  L.push('   ' + r.goal);
  L.push('');
  L.push('2. WHAT WAS DONE');
  L.push('   ' + r.what_done);
  L.push('');
  L.push('3. ISSUES (unresolved / surfaced)');
  if (r.issues.length) for (const i of r.issues) L.push('   • ' + i);
  else L.push('   (none)');
  L.push('');
  L.push('4. PATTERNS · INSIGHTS · LEARNINGS');
  if (r.patterns.length) for (const p of r.patterns) L.push('   • ' + p);
  else L.push('   (none)');
  L.push('');
  L.push('5. TELEMETRY');
  L.push(`   CODE      tests ${t.tests_fail_start}→${t.tests_fail_end} · ${t.edits} edits · +${t.loc_add}/−${t.loc_del} · slop ${t.slop_min ?? 'n/a'} · fake-green ×${t.fake_green_caught}`);
  L.push(`   GIT/MEM   ${t.commits} commits · ${t.prs} PRs · ${t.conflicts} conflicts · RSS peak ${(t.rss_peak_mb / 1024).toFixed(1)}GB`);
  L.push(`   AGENTS    ${Object.entries(t.agents).map(([k, v]) => `${k} ×${v}`).join(' · ') || '(none)'}`);
  L.push(`   SKILLS    ${Object.entries(t.skills_used).map(([k, v]) => `${k} ×${v}`).join(', ') || '(none)'}` + (t.skills_ghost.length ? ` · ghost: ${t.skills_ghost.join(', ')}` : ''));
  L.push(`   TOKENS    in ${num(t.tokens_in)} · out ${num(t.tokens_out)} · cache-r ${num(t.cache_read)} · cache-w ${num(t.cache_write ?? 0)} · cost $${t.cost_usd.toFixed(2)}` + (t.per_resolved ? ` · per-resolved ${num(t.per_resolved)}` : ''));
  L.push(`   ECO       ${t.eco.kwh ?? 0} kWh · ${t.eco.gco2 ?? 0} gCO₂ · ${t.eco.water_ml ?? 0} ml water  (${t.eco.estimate ? 'estimate' : 'measured'}${t.eco.method ? ', ' + t.eco.method : ''})`);
  L.push('');
  L.push('6. VS HISTORY (' + r.loop + ', all prior runs)');
  if (r.history && r.history.prior_runs > 0) {
    const h = r.history;
    L.push(`   iters-to-green  ${h.iters_to_green.this}${arrow(h.iters_to_green.this, h.iters_to_green.avg)} (best ${h.iters_to_green.best})`);
    if (h.per_resolved.this != null && h.per_resolved.avg != null)
      L.push(`   tokens/resolved ${num(h.per_resolved.this)}${arrow(h.per_resolved.this, h.per_resolved.avg)}`);
    L.push(`   cost            $${h.cost_usd.this.toFixed(2)}${arrow(h.cost_usd.this, h.cost_usd.avg)}`);
    if (h.recurring.length) L.push(`   recurring: ${h.recurring.map((x) => `${x.tag} ×${x.count}`).join(' · ')}`);
  } else {
    L.push('   (first run — no prior history)');
  }
  L.push('');
  L.push('7. CARRY FORWARD → run #' + (r.run_index + 1));
  L.push(`   guards: ${r.carry_forward.guards.join(' · ') || '(none)'}`);
  L.push(`   watch:  ${r.carry_forward.watch.join(' · ') || '(none)'}`);
  L.push(BAR);
  return L.join('\n');
}
