// Pure function — no side effects, fully testable without DB.

export interface MenuLineState {
  productId: string;
  quantity: number;
  modifierIds: string[];
  /** null if product not in menu; false if in stop-list */
  productAvailable: boolean | null;
  /** modifierId → availability. null=not in menu, false=stop-list */
  modifierAvailability: Record<string, boolean | null>;
}

export interface SignalState {
  /** Velocity phone count in current window */
  velocityPhoneCount: number;
  /** Velocity IP count in current window */
  velocityIpCount: number;
  /** No-show count (raw counter) */
  noShowCount: number;
  /** Days since last no-show. null = never */
  noShowAgeDays: number | null;
  /** Completed order count */
  completedCount: number;
  /** Whether OTP is required for this location */
  otpRequired: boolean;
  /** Whether OTP has been verified server-side */
  otpVerified: boolean;
}

export interface PreflightInput {
  lines: MenuLineState[];
  signals: SignalState;
  /** Codes the client already acknowledged in this submit */
  acknowledgedCodes: string[];
}

export interface PreflightReason {
  code: string;
  severity: 'objective' | 'soft';
  message: string;
  itemId?: string;
}

export interface PreflightResult {
  outcome: 'clean' | 'soft_confirm' | 'hard_block';
  reasons: PreflightReason[];
  requiresConfirmation?: boolean;
  requiresOtp?: boolean;
  confirmedReasons?: string[];
}

const NO_SHOW_DECAY_WINDOW_DAYS = 90;

function calcNoShowStrength(noShowCount: number, ageDays: number | null, completedCount: number): number {
  if (ageDays === null) return 0;
  const decayFactor = Math.exp(-ageDays / 30);
  return noShowCount * decayFactor / Math.max(1, completedCount);
}

const VELOCITY_PHONE_THRESHOLD = 3;
const VELOCITY_IP_THRESHOLD = 3;
const NO_SHOW_STRENGTH_THRESHOLD = 0.5;

export function evaluatePreflight(input: PreflightInput): PreflightResult {
  const reasons: PreflightReason[] = [];

  // 1. HARD_BLOCK: item/modifier unavailable
  for (const line of input.lines) {
    if (line.productAvailable === null) {
      reasons.push({
        code: 'item_unavailable',
        severity: 'objective',
        message: 'Item is not in the menu.',
        itemId: line.productId,
      });
    } else if (line.productAvailable === false) {
      reasons.push({
        code: 'item_unavailable',
        severity: 'objective',
        message: 'Item is currently unavailable (stop-list).',
        itemId: line.productId,
      });
    }

    for (const [modId, avail] of Object.entries(line.modifierAvailability)) {
      if (avail === null) {
        reasons.push({
          code: 'item_unavailable',
          severity: 'objective',
          message: 'Modifier is not in the menu.',
          itemId: modId,
        });
      } else if (avail === false) {
        reasons.push({
          code: 'item_unavailable',
          severity: 'objective',
          message: 'Modifier is currently unavailable (stop-list).',
          itemId: modId,
        });
      }
    }
  }

  if (reasons.length > 0) {
    return { outcome: 'hard_block', reasons };
  }

  // 2. Collect soft reasons
  const { signals } = input;

  if (signals.velocityPhoneCount > VELOCITY_PHONE_THRESHOLD) {
    reasons.push({
      code: 'velocity',
      severity: 'soft',
      message: 'Unusually many orders from this number in a short time — please confirm.',
    });
  }

  if (signals.velocityIpCount > VELOCITY_IP_THRESHOLD) {
    reasons.push({
      code: 'velocity',
      severity: 'soft',
      message: 'Unusually many orders from this device in a short time — please confirm.',
    });
  }

  const noShowStrength = calcNoShowStrength(signals.noShowCount, signals.noShowAgeDays, signals.completedCount);
  if (noShowStrength > NO_SHOW_STRENGTH_THRESHOLD && signals.noShowAgeDays !== null && signals.noShowAgeDays <= NO_SHOW_DECAY_WINDOW_DAYS) {
    reasons.push({
      code: 'no_show_history',
      severity: 'soft',
      message: `There were ${signals.noShowCount} no-show(s) in the past. Please confirm the order.`,
    });
  }

  if (signals.otpRequired && !signals.otpVerified) {
    reasons.push({
      code: 'otp_required',
      severity: 'soft',
      message: 'Please verify your phone number with a code.',
    });
  }

  // 3. Determine outcome
  if (reasons.length === 0) {
    return { outcome: 'clean', reasons };
  }

  // Check if all soft reasons have been acknowledged by client
  const requiresOtp = signals.otpRequired && !signals.otpVerified;
  const allAcknowledged = reasons.every(r => input.acknowledgedCodes.includes(r.code));
  const otpSatisfied = !requiresOtp;

  if (allAcknowledged && otpSatisfied) {
    return {
      outcome: 'clean',
      reasons,
      confirmedReasons: reasons.map(r => r.code),
    };
  }

  return {
    outcome: 'soft_confirm',
    reasons,
    requiresConfirmation: true,
    requiresOtp,
  };
}
