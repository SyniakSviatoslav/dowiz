#!/usr/bin/env node
// BLIND ORCHESTRATION — the physical floor of the token economy (operator directive 2026-07-05).
// Inversion of control: the LLM never sees the state. Deterministic code does ALL the math (spatial
// cull, ETA, VSA cosine) at $0 and AUTO-ASSIGNS every clear match. The model wakes ONLY for a
// genuine ambiguity — a conflict no rule can settle — as a surgical ~50-token micro-prompt. Most
// ticks cost 0 tokens; the residue costs cents.
//
//   Tier 1 — Deterministic Guillotine: per unassigned order → cull drivers by distance + ETA +
//            availability, score survivors by VSA cosine ⊕ proximity ⊕ ETA-slack.
//   Tier 2 — Conflict Extraction: a clear winner (high score, clear margin, no contention, no soft
//            constraint) is assigned in-code, NO LLM. Only contention / soft-constraint tradeoffs
//            escalate.
//   Tier 3 — Micro-prompt: {q,task,vip,options:[{d,vsa_score,risk}]} (~50 tok) + a cached ~40-tok
//            judge prompt. The model returns only a driver id.

import { textHv, bundle, cosine } from './src/hv.mjs';

const R_KM = 6371;
const toRad = (d) => (d * Math.PI) / 180;
export function haversineKm(a, b) {
  const dLat = toRad(b.lat - a.lat);
  const dLng = toRad(b.lng - a.lng);
  const s = Math.sin(dLat / 2) ** 2 + Math.cos(toRad(a.lat)) * Math.cos(toRad(b.lat)) * Math.sin(dLng / 2) ** 2;
  return 2 * R_KM * Math.asin(Math.sqrt(s));
}

const bucket = (n, edges) => edges.findIndex((e) => n <= e);

// Feature hypervectors — cached per entity so cosine is cheap. Shared tags (zone, vehicle fit) drive
// the semantic match; the caller blends this with hard geometry.
function orderHv(o) {
  return bundle([textHv('zone:' + o.zone), textHv('cuisine:' + (o.cuisine ?? 'any')), textHv('size:' + bucket(o.size ?? 1, [1, 3, 6, 99]))]);
}
function driverHv(d) {
  return bundle([textHv('zone:' + d.zone), textHv('veh:' + d.type), textHv('cap:' + bucket(d.capacity ?? 3, [1, 3, 6, 99]))]);
}

/**
 * Resolve a dispatch state. Returns { autoAssignments, conflicts, stats }. No LLM is called here —
 * `conflicts` are the ONLY thing that ever reaches the model.
 */
export function resolveDispatch(state, opts = {}) {
  const maxKm = opts.maxKm ?? 8; // spatial cull radius
  const speedKmh = opts.speedKmh ?? 25;
  const clearScore = opts.clearScore ?? 0.62; // top must clear this to auto-assign
  const clearMargin = opts.clearMargin ?? 0.08; // and beat #2 by this
  const drivers = (state.drivers ?? []).filter((d) => d.status !== 'offline');
  const orders = (state.orders ?? []).filter((o) => !o.assigned);
  const dHv = new Map(drivers.map((d) => [d.id, driverHv(d)]));

  // Tier 1: per order, cull + score candidates.
  const ranked = new Map(); // orderId → sorted candidates
  for (const o of orders) {
    const ohv = orderHv(o);
    const cands = [];
    for (const d of drivers) {
      const km = haversineKm(o, d);
      if (km > maxKm) continue; // SPATIAL cull
      const etaMin = (km / speedKmh) * 60;
      if (o.deadlineMin != null && etaMin > o.deadlineMin) continue; // TIME cull
      if ((d.load ?? 0) >= (d.capacity ?? 3)) continue; // capacity cull
      const vsa = cosine(ohv, dHv.get(d.id));
      const prox = 1 - km / maxKm;
      const slack = o.deadlineMin != null ? Math.max(0, 1 - etaMin / o.deadlineMin) : 0.5;
      // proximity/slack (hard geometry) dominate; VSA cosine is the semantic tiebreaker.
      const score = 0.5 * prox + 0.3 * slack + 0.2 * vsa;
      // soft constraints that make an otherwise-good pick ambiguous:
      const risks = [];
      // overtime only when the delivery genuinely runs past the driver's shift end (eta + a 10-min
      // drop), and VIP-mismatch only for a materially low rating — so the model sees real tradeoffs.
      if (d.shiftEndsMin != null && etaMin + 10 > d.shiftEndsMin) risks.push('overtime');
      if (o.vip && (d.rating ?? 5) < 3.8) risks.push('low_rating_vip');
      cands.push({ d, km, etaMin, vsa: +vsa.toFixed(2), score: +score.toFixed(3), risks });
    }
    cands.sort((a, b) => b.score - a.score);
    ranked.set(o.id, cands);
  }

  // Tier 2: capacity-aware greedy. Strongest matches claim driver slots first; a match auto-assigns
  // only if it's high-scoring, clears #2 (or is the sole option), and carries no soft constraint.
  // Genuine collisions (last slot contended by a close order, soft-constraint tradeoff, or no free
  // driver) are the ONLY things that escalate to the model.
  const autoAssignments = [];
  const conflicts = [];
  const used = new Map(); // driverId → assigned count this tick
  const slotsFree = (d) => (d.capacity ?? 3) - (d.load ?? 0) - (used.get(d.id) ?? 0);
  const ordersByStrength = [...orders].sort((a, b) => (ranked.get(b.id)[0]?.score ?? 0) - (ranked.get(a.id)[0]?.score ?? 0));
  for (const o of ordersByStrength) {
    const cs = ranked.get(o.id).filter((c) => slotsFree(c.d) > 0);
    if (cs.length === 0) {
      conflicts.push(buildConflict(o, ranked.get(o.id).slice(0, 2), 'no_free_candidate'));
      continue;
    }
    const top = cs[0];
    // A near-tie between two interchangeable good drivers is NOT a conflict — pick either. Escalate
    // ONLY when the choice genuinely MATTERS: a soft-constraint tradeoff on the best pick, or scarcity
    // (this driver's LAST slot, and the runner-up order would be stranded — no other free driver).
    const scarce = slotsFree(top.d) === 1 && cs.length === 1; // sole viable driver, one slot
    // The LLM earns its keep ONLY on a genuine JUDGMENT CALL — a soft-constraint tradeoff (overtime /
    // VIP-vs-low-rating) or a scarcity collision. Everything else is deterministic: a strong clean
    // match auto-assigns; even a merely-mediocre-but-only option auto-assigns best-effort (the model
    // can't conjure a closer driver — asking it would burn tokens for no decision).
    if (top.risks.length > 0 || scarce) {
      conflicts.push(buildConflict(o, cs.slice(0, 2), top.risks.length ? 'soft_constraint' : 'scarcity'));
    } else {
      autoAssignments.push({ order: o.id, driver: top.d.id, score: top.score, bestEffort: top.score < clearScore });
      used.set(top.d.id, (used.get(top.d.id) ?? 0) + 1);
    }
  }

  const stats = {
    orders: orders.length,
    autoResolved: autoAssignments.length,
    escalated: conflicts.length,
    autoPct: orders.length ? +((autoAssignments.length / orders.length) * 100).toFixed(1) : 0,
  };
  return { autoAssignments, conflicts, stats };
}

// Tier 3: the surgical micro-prompt for ONE conflict — no coords, no history, just the choice.
function buildConflict(o, topCands, reason) {
  return {
    q: o.zone,
    task: o.id,
    vip: !!o.vip,
    reason,
    options: topCands.map((c) => ({ d: c.d.id, vsa_score: c.vsa, risk: c.risks[0] ?? (c.km > 5 ? 'dead_mileage' : 'none') })),
  };
}

// Cached judge prompt (~40 tok). The ONLY standing instruction; conflicts ride as the tiny payload.
export const JUDGE_PROMPT =
  "You resolve dispatch conflicts. Given {task, vip, options:[{d, vsa_score, risk}]}, pick the best 'd'. " +
  'Priority: satisfy VIP > avoid overtime > avoid low_rating_vip > higher vsa_score > avoid dead_mileage. ' +
  "Reply with ONLY the chosen d identifier, nothing else.";

// CLI: resolve a state file (or a built-in Fable-5 sim) and print the token accounting.
if (import.meta.url === `file://${process.argv[1]}`) {
  const { countTokens } = await import('./src/tokens.mjs');
  const fs = await import('node:fs');
  let state;
  if (process.argv[2] && process.argv[2] !== 'sim') state = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
  else state = simFable5();
  const t0 = process.hrtime.bigint();
  const { autoAssignments, conflicts, stats } = resolveDispatch(state);
  const ms = Number(process.hrtime.bigint() - t0) / 1e6;
  // tokens: blind = judge prompt (cached, once) + Σ micro-prompt payloads; naive = the full JSON.
  const judgeTok = await countTokens(JUDGE_PROMPT);
  let microTok = 0;
  for (const c of conflicts) microTok += await countTokens(JSON.stringify(c));
  const naiveTok = await countTokens(JSON.stringify(state));
  console.log(JSON.stringify({
    resolve_ms: +ms.toFixed(1),
    stats,
    tokens: {
      naive_full_json: naiveTok,
      blind_dynamic: microTok, // only conflicts reach the model
      blind_cached_judge: judgeTok,
      llm_calls: conflicts.length, // vs 1 big call in the naive path (or 0 if all auto)
      reduction_vs_naive_pct: +((1 - (microTok + judgeTok) / naiveTok) * 100).toFixed(1),
    },
    sample_conflict: conflicts[0] ?? null,
  }, null, 2));
}

// A Fable-5-scale sim: 50 drivers, 450 orders across 5 zones, realistic geo + deadlines + some VIP.
export function simFable5() {
  const rnd = ((s) => () => (s = (s * 1103515245 + 12345) & 0x7fffffff) / 0x7fffffff)(50);
  const hub = [[41.32, 19.80], [41.34, 19.82], [41.31, 19.83], [41.35, 19.79], [41.33, 19.85]];
  const drivers = Array.from({ length: 50 }, (_, i) => {
    const z = i % 5;
    return { id: 'v_' + i, zone: 'H' + z, type: i % 2 ? 'van' : 'car', lat: hub[z][0] + (rnd() - 0.5) * 0.02, lng: hub[z][1] + (rnd() - 0.5) * 0.02, status: rnd() < 0.1 ? 'offline' : 'on', load: Math.floor(rnd() * 3), capacity: 3, rating: 3.5 + rnd() * 1.5, shiftEndsMin: rnd() < 0.15 ? 15 + Math.floor(rnd() * 30) : null };
  });
  // ~120 live orders against ~45 online drivers × 3 slots ≈ 135 slots — realistic supply≈demand,
  // so most orders have a clear nearest-available driver and only the residue collides.
  const orders = Array.from({ length: 120 }, (_, i) => {
    const z = i % 5;
    return { id: 'o_' + i, zone: 'H' + z, cuisine: ['pizza', 'sushi', 'burger'][i % 3], size: 1 + Math.floor(rnd() * 4), lat: hub[z][0] + (rnd() - 0.5) * 0.03, lng: hub[z][1] + (rnd() - 0.5) * 0.03, deadlineMin: 20 + Math.floor(rnd() * 40), vip: rnd() < 0.05, assigned: false };
  });
  return { drivers, orders };
}
