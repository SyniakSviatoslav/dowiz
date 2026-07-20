// @ts-nocheck
import type { Pool } from 'pg';
import crypto from 'node:crypto';

export type SignalKind =
  | 'no_show_recent'
  | 'velocity_rapid'
  | 'velocity_high_volume'
  | 'ip_velocity_rapid'
  | 'ip_velocity_high_volume'
  | 'manual_flag';

export interface SignalEvidence {
  count?: number;
  ageDays?: number;
  decayFactor?: number;
  windowSize?: string;
  threshold?: number;
  lastNoShowAt?: string;
  completedCount?: number;
  ratio?: number;
}

export interface Signal {
  id?: string;
  kind: SignalKind;
  severity: 'low' | 'medium' | 'high';
  reason: string;
  evidence: SignalEvidence;
}

export interface ComputeSignalsParams {
  customerId?: string;
  locationId: string;
  phoneHash?: string;
  clientIpHash?: string;
}

const VELOCITY_WINDOWS: { kind: SignalKind; seconds: number; threshold: number; label: string }[] = [
  { kind: 'velocity_rapid', seconds: 3600, threshold: 3, label: '1h' },
  { kind: 'velocity_high_volume', seconds: 86400, threshold: 10, label: '24h' },
];

const IP_VELOCITY_WINDOWS: { kind: SignalKind; seconds: number; threshold: number; label: string }[] = [
  { kind: 'ip_velocity_rapid', seconds: 3600, threshold: 3, label: '1h' },
  { kind: 'ip_velocity_high_volume', seconds: 86400, threshold: 10, label: '24h' },
];

function computeNoShowStrength(
  noShowCount: number,
  completedCount: number,
  lastNoShowAt: Date | null,
): { strength: number; ageDays: number; decayFactor: number; ratio: number } | null {
  if (!lastNoShowAt) return null;

  const ageMs = Date.now() - new Date(lastNoShowAt).getTime();
  const ageDays = Math.max(0, ageMs / 86400000);
  const decayFactor = Math.exp(-ageDays / 30);
  const ratio = noShowCount / Math.max(1, completedCount);
  const strength = noShowCount * decayFactor / Math.max(1, completedCount);

  return { strength, ageDays, decayFactor, ratio };
}

function noShowSeverity(strength: number): 'low' | 'medium' | 'high' {
  if (strength > 2.0) return 'high';
  if (strength > 1.0) return 'medium';
  return 'low';
}

function velocitySeverity(count: number, threshold: number): 'low' | 'medium' | 'high' {
  if (count >= threshold * 3) return 'high';
  if (count >= threshold * 2) return 'medium';
  return 'low';
}

export async function computeSignals(
  pool: Pool,
  params: ComputeSignalsParams,
): Promise<Signal[]> {
  const signals: Signal[] = [];

  // 1. No-show reputation
  if (params.customerId) {
    const custRes = await pool.query(
      `SELECT no_show_count, completed_count, last_no_show_at FROM customers WHERE id = $1`,
      [params.customerId],
    );

    if (custRes.rowCount > 0) {
      const { no_show_count, completed_count, last_no_show_at } = custRes.rows[0];
      const result = computeNoShowStrength(no_show_count, completed_count, last_no_show_at);

      if (result && result.strength > 0.5) {
        signals.push({
          kind: 'no_show_recent',
          severity: noShowSeverity(result.strength),
          reason: `Recent no-show (${Math.round(result.ageDays)}d ago, count=${no_show_count}, ratio=${result.ratio.toFixed(2)})`,
          evidence: {
            count: no_show_count,
            ageDays: Math.round(result.ageDays * 10) / 10,
            decayFactor: Math.round(result.decayFactor * 100) / 100,
            threshold: 0.5,
            lastNoShowAt: last_no_show_at?.toISOString(),
            completedCount: completed_count,
            ratio: Math.round(result.ratio * 100) / 100,
          },
        });
      }
    }
  }

  // 2. Phone velocity
  if (params.phoneHash) {
    for (const w of VELOCITY_WINDOWS) {
      const velRes = await pool.query(
        `SELECT COUNT(*)::int AS cnt FROM velocity_events
         WHERE location_id = $1 AND phone_hash = $2
           AND kind = 'order_placed'
           AND window_started_at > now() - ($3 || ' seconds')::interval`,
        [params.locationId, params.phoneHash, String(w.seconds)],
      );
      const count = velRes.rows[0]?.cnt ?? 0;

      if (count > w.threshold) {
        signals.push({
          kind: w.kind,
          severity: velocitySeverity(count, w.threshold),
          reason: `${count} orders in ${w.label} (threshold=${w.threshold})`,
          evidence: { count, windowSize: w.label, threshold: w.threshold },
        });
      }
    }
  }

  // 3. IP velocity
  if (params.clientIpHash) {
    for (const w of IP_VELOCITY_WINDOWS) {
      const velRes = await pool.query(
        `SELECT COUNT(*)::int AS cnt FROM velocity_events
         WHERE location_id = $1 AND client_ip_hash = $2
           AND kind = 'order_placed'
           AND window_started_at > now() - ($3 || ' seconds')::interval`,
        [params.locationId, params.clientIpHash, String(w.seconds)],
      );
      const count = velRes.rows[0]?.cnt ?? 0;

      if (count > w.threshold) {
        signals.push({
          kind: w.kind,
          severity: velocitySeverity(count, w.threshold),
          reason: `${count} orders from same IP in ${w.label} (threshold=${w.threshold})`,
          evidence: { count, windowSize: w.label, threshold: w.threshold },
        });
      }
    }
  }

  return signals;
}

export function computeNoShowStrengthSync(
  noShowCount: number,
  completedCount: number,
  lastNoShowAt: Date | null,
): { strength: number; ageDays: number; decayFactor: number; ratio: number } | null {
  return computeNoShowStrength(noShowCount, completedCount, lastNoShowAt);
}
