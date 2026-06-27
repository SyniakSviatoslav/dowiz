import crypto from 'crypto';
import type { Signal } from './signals/compute.js';
import type { SignalState } from './preflight.js';

// Pure canonicalization helpers extracted from POST /orders. NO database, NO
// reply. Two concerns:
//   1. buildRequestHash — the idempotency request fingerprint (section 3).
//   2. buildSignalState — reduce computeSignals() output into the preflight
//      SignalState (section 4d).
// Both are byte-/value-stable: the request hash in particular MUST stay
// identical to the inline version or in-flight idempotency keys would mismatch
// on retry (cached order → spurious IDEMPOTENCY_KEY_REUSED). Hence the JSON key
// order and 5-dp pin rounding below mirror the original exactly.

export interface CanonicalRequestInput {
  locationId: string;
  type: string;
  items: Array<{ product_id: string; quantity: number; modifier_ids?: string[] }>;
  /** Delivery pin (null for pickup). Rounded to 5 decimal places for stability. */
  pin: { lat: number; lng: number } | null;
  addressText: string | null;
  cashPayWith: number | null | undefined;
  currencyCode: string;
  menuVersion: string;
  /** Resolved customer id, or 'anonymous'. */
  customerId: string;
}

export function buildRequestHash(input: CanonicalRequestInput): string {
  const { locationId, type, items, pin, addressText, cashPayWith, currencyCode, menuVersion, customerId } = input;

  const canonicalItems = items.map((i) => ({
    product_id: i.product_id,
    quantity: i.quantity,
    modifier_ids: [...(i.modifier_ids || [])].sort(),
  }));
  const latRounded5 = pin ? Math.round(pin.lat * 100000) / 100000 : null;
  const lngRounded5 = pin ? Math.round(pin.lng * 100000) / 100000 : null;

  const canonicalBody = JSON.stringify({
    locationId,
    type,
    items: canonicalItems,
    pin: pin ? { lat: latRounded5, lng: lngRounded5 } : null,
    address_text: addressText,
    cash_pay_with: cashPayWith || null,
    currency_code: currencyCode,
    menu_version: menuVersion,
    customer_id: customerId,
  });
  return crypto.createHash('sha256').update(canonicalBody).digest('hex');
}

export interface BuildSignalStateInput {
  signals: Signal[];
  otpRequired: boolean;
  otpVerified: boolean;
}

/**
 * Reduce the raw signals list into the preflight SignalState. Velocity counts
 * take the max across matching windows; no-show pulls count/age/completed from
 * its evidence. Mirrors section 4d exactly.
 */
export function buildSignalState(input: BuildSignalStateInput): SignalState {
  const { signals, otpRequired, otpVerified } = input;

  const sigState: SignalState = {
    velocityPhoneCount: 0,
    velocityIpCount: 0,
    noShowCount: 0,
    noShowAgeDays: null,
    completedCount: 0,
    otpRequired,
    otpVerified,
  };

  for (const s of signals) {
    if (s.kind === 'velocity_rapid' || s.kind === 'velocity_high_volume') {
      sigState.velocityPhoneCount = Math.max(sigState.velocityPhoneCount, s.evidence.count || 0);
    }
    if (s.kind === 'ip_velocity_rapid' || s.kind === 'ip_velocity_high_volume') {
      sigState.velocityIpCount = Math.max(sigState.velocityIpCount, s.evidence.count || 0);
    }
    if (s.kind === 'no_show_recent') {
      sigState.noShowCount = s.evidence.count || 0;
      sigState.noShowAgeDays = s.evidence.ageDays ?? null;
      sigState.completedCount = s.evidence.completedCount || 0;
    }
  }

  return sigState;
}
