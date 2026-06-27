// §6 — eco computation. Pure: tokens × per-model factors → energy / CO₂ / water.
// EcoLogits can't intercept Claude Code's internal calls, so (per §6) we compute
// from session token totals × per-model factors offline.
//
// HONESTY (§6): absolute numbers are ESTIMATES — providers disclose little and
// methodologies diverge. The factors below are constant across runs, so the
// run-over-run TREND is the reliable signal (that's what the self-comparison
// uses). Treat the trend as real; don't quote the absolute as audited. Update
// the factors here if better EcoLogits data lands — one place, lossless.

import type { EcoBlock } from './types.js';

/** Per-model energy intensity (kWh per 1k total tokens), tiered by model size.
 *  Order-of-magnitude EcoLogits-style estimates; bigger model = more energy. */
const KWH_PER_KTOK: Record<string, number> = {
  'claude-opus-4-8': 0.00060,
  'claude-opus': 0.00060,
  'claude-sonnet-4-6': 0.00022,
  'claude-sonnet': 0.00022,
  'claude-haiku-4-5': 0.00008,
  'claude-haiku': 0.00008,
};
const DEFAULT_KWH_PER_KTOK = 0.0003;

/** Grid carbon intensity (gCO₂eq/kWh). Hetzner DE ≈ 380; tune to your region. */
const GCO2_PER_KWH = 380;
/** Datacenter water intensity incl. cooling (ml/kWh). ~1.8 L/kWh global avg. */
const WATER_ML_PER_KWH = 1800;

function factorFor(model: string): number {
  if (KWH_PER_KTOK[model] != null) return KWH_PER_KTOK[model]!;
  const key = Object.keys(KWH_PER_KTOK).find((k) => model.startsWith(k));
  return key ? KWH_PER_KTOK[key]! : DEFAULT_KWH_PER_KTOK;
}

/**
 * Compute the §6 eco block from per-model total token counts.
 * @param tokensByModel  model id → total tokens attributed to it this run.
 */
export function computeEco(tokensByModel: Record<string, number>): EcoBlock {
  let kwh = 0;
  for (const [model, total] of Object.entries(tokensByModel)) {
    kwh += (total / 1000) * factorFor(model);
  }
  const round = (n: number, dp: number) => Math.round(n * 10 ** dp) / 10 ** dp;
  return {
    kwh: round(kwh, 4),
    gco2: round(kwh * GCO2_PER_KWH, 1),
    water_ml: round(kwh * WATER_ML_PER_KWH, 0),
    method: 'token×model-factor (EcoLogits-style estimate)',
    estimate: true,
  };
}
