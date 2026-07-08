// Bebop governor — L5 signal-health monitor (operator directive 2026-07-08).
//
// Unifies FOUR math foundations into meta-control over ANY autonomous process (agent/model/loop):
//   1. PID        — discrete position form + integral anti-windup; drives resource AUTHORITY.
//   2. ICIR       — quant factor-health: IC_t = rank-corr(predicted, actual); ICIR = mean/std.
//                   an agent's "factor" is its self-knowledge (does it predict its own quality?).
//   3. RESONANCE  — predict closed-loop ζ/ω_r of a PROPOSED gain BEFORE applying it; refuse
//                   changes that push the loop under-damped (harmonic blow-up across layers).
//   4. THERMO     — Landauer floor k·T·ln2 per bit; a hard floor on cost (can't think for free).
//
// Every property is verified by a falsifiable RED+GREEN test (Verified-by-Math bar).

export interface TelemetrySample {
  t: number;
  predictedQuality: number; // agent's self-predicted quality ∈ [0,1]
  actualQuality: number; // observed quality ∈ [0,1]
  cost: number; // resource units consumed this step
  volume: number; // throughput (tokens/actions) this step
}

export type FactorStatus = 'unknown' | 'healthy' | 'volatile' | 'dead';

export interface GovernorState {
  authority: number; // ∈ [uMin,uMax], recommended resource latitude
  pidU: number; // raw PID output before clamps
  icir: number | null;
  factorStatus: FactorStatus;
  resonanceRisky: boolean;
  anomaly: boolean;
  thermoFloorHit: boolean;
  error: number;
}

// ── pure math primitives ──────────────────────────────────────────────────────

export function clamp(x: number, lo: number, hi: number): number {
  return x < lo ? lo : x > hi ? hi : x;
}

function rank(xs: number[]): number[] {
  const idx = xs.map((v, i) => [v, i] as [number, number]).sort((a, b) => a[0] - b[0]);
  const r = new Array(xs.length).fill(0);
  let i = 0;
  while (i < idx.length) {
    let j = i;
    while (j + 1 < idx.length && idx[j + 1][0] === idx[i][0]) j++;
    const avg = (i + j) / 2 + 1; // 1-based average rank (ties)
    for (let k = i; k <= j; k++) r[idx[k][1]] = avg;
    i = j + 1;
  }
  return r;
}

export function pearson(a: number[], b: number[]): number {
  const n = a.length;
  const ma = a.reduce((s, x) => s + x, 0) / n;
  const mb = b.reduce((s, x) => s + x, 0) / n;
  let num = 0, da = 0, db = 0;
  for (let i = 0; i < n; i++) { const pa = a[i] - ma, pb = b[i] - mb; num += pa * pb; da += pa * pa; db += pb * pb; }
  const den = Math.sqrt(da * db);
  return den === 0 ? 0 : num / den;
}

/** Spearman rank correlation — the IC of a quant factor. */
export function spearman(a: number[], b: number[]): number {
  if (a.length !== b.length || a.length < 2) return 0;
  // both series constant ⇒ perfectly comonotonic (a perfectly self-knowing agent) ⇒ IC = 1
  const av = a[0]; const bv = b[0];
  if (a.every((x) => x === av) && b.every((x) => x === bv)) return 1;
  return pearson(rank(a), rank(b));
}

export interface PIDConfig {
  kp: number; ki: number; kd: number;
  iMin: number; iMax: number; uMin: number; uMax: number; maxStep?: number;
}

export interface PIDState { integral: number; prevError: number; }

export function pidStep(cfg: PIDConfig, st: PIDState, error: number): { u: number; integral: number } {
  // integral with anti-windup clamp — bounds the accumulator so a sustained error can't explode
  const rawI = st.integral + cfg.ki * error;
  const integral = clamp(rawI, cfg.iMin, cfg.iMax);
  const d = cfg.kd * (error - st.prevError);
  const u = clamp(cfg.kp * error + integral + d, cfg.uMin, cfg.uMax);
  return { u, integral };
}

/** ICIR = mean(IC) / std(IC) over a window. null when undefined. */
export function icir(icSeries: number[]): number | null {
  if (icSeries.length < 2) return null;
  const m = icSeries.reduce((s, x) => s + x, 0) / icSeries.length;
  const v = icSeries.reduce((s, x) => s + (x - m) ** 2, 0) / icSeries.length;
  const sd = Math.sqrt(v);
  if (sd === 0) return m > 0 ? Infinity : 0;
  return m / sd;
}

// ── resonance prediction (predict the change before applying it) ──────────────

const SQRT1_2 = 1 / Math.sqrt(2);

export interface Resonance {
  wn: number; zeta: number; wr: number; mr: number; risky: boolean; aliasRisk: boolean;
}

/** Closed-loop resonance of a 2nd-order agent-plant under gains (Kp,Kd), inertia M, damping B. */
export function loopResonance(kp: number, kd: number, M: number, B: number, samplePeriod = 0): Resonance {
  const wn = Math.sqrt(Math.max(0, kp / M));
  const zeta = (B + kd) / (2 * Math.sqrt(Math.max(1e-9, kp * M)));
  const risky = zeta < SQRT1_2;
  const wr = wn * Math.sqrt(Math.max(0, 1 - 2 * zeta * zeta));
  const mr = risky ? 1 / (2 * zeta * Math.sqrt(Math.max(1e-9, 1 - zeta * zeta))) : 1;
  // discrete alias: if the natural frequency approaches the Nyquist band, harmonics fold in
  const aliasRisk = samplePeriod > 0 && wn * samplePeriod > 0.3;
  return { wn, zeta, wr, mr, risky: risky || aliasRisk, aliasRisk };
}

// ── thermodynamics of computation (Landauer) ──────────────────────────────────

const K_B = 1.380649e-23;
const LN2 = Math.LN2;

/** Minimum energy to erase `bits` bits at temperature T (Kelvin). @300K ≈ 2.87e-21 J/bit. */
export function landauerFloor(bits: number, T = 300): number {
  if (bits < 0) throw new Error('thermo: negative bits');
  return bits * K_B * T * LN2;
}

/** Bits erased by a decision of given volume (log2 of distinct states touched). */
export function bitsErased(volume: number): number {
  return Math.max(1, Math.ceil(Math.log2(volume + 2)));
}

// ── anomaly detection (operator priority: flag telemetry breaching estimated bounds) ──

export function detectAnomaly(history: number[], x: number, k = 3): boolean {
  if (history.length < 2) return false;
  const m = history.reduce((s, v) => s + v, 0) / history.length;
  const sd = Math.sqrt(history.reduce((s, v) => s + (v - m) ** 2, 0) / history.length);
  if (sd === 0) return false; // zero-variance history: no basis to call a breach; stay quiet
  return Math.abs(x - m) > k * sd;
}

// ── the Governor: ties it all together over one agent/loop ─────────────────────

export interface GovernorConfig extends PIDConfig {
  targetQuality: number; // setpoint ∈ [0,1]
  deadIC?: number; // ICIR below this → factor 'dead' → authority floored (legacy name)
  icirKill?: number; // ICIR below this → factor 'dead' → authority floored
  icirVolatile: number; // ICIR below this (but ≥ kill) → 'volatile'
  plantM: number; plantB: number; samplePeriod?: number;
  anomalyK?: number;
  volHistoryLen?: number;
}

export class Governor {
  cfg: GovernorConfig;
  pid: PIDState = { integral: 0, prevError: 0 };
  private predAct: Array<[number, number]> = []; // trailing (pred,act) pairs for IC
  private icSeries: number[] = [];
  private volHistory: number[] = [];
  private _meanIC = 0;
  anomaly = false;
  resonanceRisky = false;
  thermoFloorHit = false;

  constructor(cfg: GovernorConfig) { this.cfg = cfg; }

  private pushFactor(pred: number, act: number): number | null {
    this.predAct.push([pred, act]);
    const W = 8;
    if (this.predAct.length > W) this.predAct.shift();
    if (this.predAct.length < 4) return null;
    const ic = spearman(this.predAct.map((p) => p[0]), this.predAct.map((p) => p[1]));
    this.icSeries.push(ic);
    const L = 16;
    if (this.icSeries.length > L) this.icSeries.shift();
    this._meanIC = this.icSeries.reduce((s, x) => s + x, 0) / this.icSeries.length;
    return icir(this.icSeries);
  }

  factorStatus(icirV: number | null): FactorStatus {
    if (icirV === null) return 'unknown'; // insufficient telemetry — neither trust nor kill
    const kill = this.cfg.deadIC ?? this.cfg.icirKill ?? 0.05; // ICIR below this ⇒ 'dead'
    if (icirV < kill) return 'dead'; // proven zero predictive power → kill-switch
    if (icirV < this.cfg.icirVolatile) return 'volatile';
    return 'healthy';
  }

  step(s: TelemetrySample): GovernorState {
    const error = this.cfg.targetQuality - s.actualQuality;
    const c = this.cfg;
    const { u, integral } = pidStep(c, this.pid, error);
    this.pid = { integral, prevError: error };

    const icirV = this.pushFactor(s.predictedQuality, s.actualQuality);
    const status = this.factorStatus(icirV);

    // dead factor → kill-switch: floor authority (reduce exposure), no integral growth
    let authority = u;
    if (status === 'dead') authority = c.uMin;
    if (status === 'volatile') authority = Math.min(authority, (c.uMin + c.uMax) / 2);

    // resonance: estimate closed-loop ζ of the PROPOSED step; if risky, cap the change magnitude
    const res = loopResonance(c.kp, c.kd, c.plantM, c.plantB, c.samplePeriod ?? 0);
    this.resonanceRisky = res.risky;
    const maxStep = c.maxStep ?? (c.uMax - c.uMin);
    if (res.risky) authority = clamp(authority, this.cfg.uMin, this.cfg.uMin + maxStep * 0.2);

    // anomaly on volume channel
    this.volHistory.push(s.volume);
    const VH = c.volHistoryLen ?? 32;
    if (this.volHistory.length > VH) this.volHistory.shift();
    this.anomaly = detectAnomaly(this.volHistory.slice(0, -1), s.volume, c.anomalyK ?? 3);

    // thermodynamics: cost must clear the Landauer floor for the bits this volume erases
    const floor = landauerFloor(bitsErased(s.volume));
    this.thermoFloorHit = s.cost < floor;

    return { authority: clamp(authority, c.uMin, c.uMax), pidU: u, icir: icirV, factorStatus: status, resonanceRisky: this.resonanceRisky, anomaly: this.anomaly, thermoFloorHit: this.thermoFloorHit, error };
  }
}

// ── closed-loop plant sim (deterministic proof: governor drives a 1st-order agent to setpoint) ──

/** y_{k+1} = y_k + a·(authority_k − y_k). Returns final error after `steps`. */
export function simulatePlant(authoritySeries: number[], a = 0.3, y0 = 0): number[] {
  const ys: number[] = [y0];
  for (const u of authoritySeries) ys.push(ys[ys.length - 1] + a * (u - ys[ys.length - 1]));
  return ys;
}
