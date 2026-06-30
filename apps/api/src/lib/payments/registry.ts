// Payments provider registry + feature flags (ADR-0017). DARK by default: PAYMENTS_PREPAID_ENABLED +
// PAYMENTS_CRYPTO_ENABLED both default off. Reads env directly (like other dark flags) — no config-schema churn.
import { createPlisioAdapter } from './plisio.js';
import { cashProvider, type PaymentProvider } from './provider.js';

export const isPrepaidEnabled = (): boolean => process.env.PAYMENTS_PREPAID_ENABLED === 'true';
export const isCryptoEnabled = (): boolean => process.env.PAYMENTS_CRYPTO_ENABLED === 'true';

let cached: PaymentProvider | null = null;

/** The active provider. `cash` (no-op) unless PAYMENTS_PROVIDER=plisio AND a secret key is configured. */
export function getPaymentProvider(): PaymentProvider {
  if (cached) return cached;
  const provider = process.env.PAYMENTS_PROVIDER || 'cash';
  if (provider === 'plisio') {
    const secretKey = process.env.PLISIO_SECRET_KEY;
    if (!secretKey) return cashProvider; // misconfigured → safe no-op (never a half-wired money path)
    const base = process.env.PUBLIC_API_BASE_URL || process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';
    cached = createPlisioAdapter({ secretKey, callbackUrl: `${base}/webhook/payments/plisio` });
    return cached;
  }
  return cashProvider;
}

/** Reset the memoized provider (tests / config reload). */
export function _resetPaymentProvider(): void { cached = null; }
